// ZapEngine — SDF Molecule Shader (Raymarched Shapes)
// Renders instanced quads with per-fragment SDF raymarching.
// Supports Sphere (atoms), Capsule (bonds), and RoundedBox (labels).
// Phong shading + Fresnel rim glow + HDR emissive.

// ---- Camera Uniform (shared with sprite pipeline) ----

struct Camera {
    projection: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> camera: Camera;

// ---- SDF Instance Storage Buffer ----
// Matches SDFInstance layout: 12 floats = 48 bytes per instance.
// [x, y, radius, rotation, r, g, b, shininess, emissive, shape_type, half_height, extra]

struct SDFInstance {
    position: vec2<f32>,
    radius: f32,
    rotation: f32,
    color: vec3<f32>,
    shininess: f32,
    emissive: f32,
    shape_type: f32,
    half_height: f32,
    extra: f32,
};

@group(1) @binding(0) var<storage, read> sdf_instances: array<SDFInstance>;

// ---- Dynamic Light Data (shared with lighting pass) ----

struct PointLight {
    x: f32,
    y: f32,
    r: f32,
    g: f32,
    b: f32,
    intensity: f32,
    radius: f32,
    layer_mask: f32,
};

struct LightUniforms {
    // xyz = ambient RGB, w = light_count as f32
    ambient_and_count: vec4<f32>,
    // xy = projWidth, projHeight (unused here but matches lighting.wgsl layout)
    proj_size: vec4<f32>,
};

@group(2) @binding(0) var<uniform> light_uniforms: LightUniforms;
@group(2) @binding(1) var<storage, read> lights: array<PointLight>;

// HDR emissive multiplier — set per render tier at pipeline creation.
// hdr-edr: 5.4, hdr-srgb: 2.5, sdr: 0.5
override SDF_EMISSIVE_MULT: f32 = 5.4;

// ---- Vertex I/O ----

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local_uv: vec2<f32>,
    @location(1) base_color: vec3<f32>,
    @location(2) shininess: f32,
    @location(3) emissive: f32,
    @location(4) shape_type: f32,
    @location(5) half_height_norm: f32,
    @location(6) extra_norm: f32,
    @location(7) world_center: vec2<f32>,  // Instance center in world space
    @location(8) world_radius: f32,        // Instance radius in world units
    @location(9) rotation: f32,            // Entity rotation for local→world normal transform
};

// Fullscreen quad — two triangles, 6 vertices
const QUAD_POS = array<vec2<f32>, 4>(
    vec2(-0.5, -0.5),
    vec2( 0.5, -0.5),
    vec2(-0.5,  0.5),
    vec2( 0.5,  0.5),
);
const QUAD_UV = array<vec2<f32>, 4>(
    vec2(-1.0, -1.0),
    vec2( 1.0, -1.0),
    vec2(-1.0,  1.0),
    vec2( 1.0,  1.0),
);
const QUAD_IDX = array<u32, 6>(0u, 1u, 2u, 2u, 1u, 3u);

@vertex
fn vs_sdf(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let inst = sdf_instances[input.instance_index];
    let tri_idx = QUAD_IDX[input.vertex_index];
    let pos = QUAD_POS[tri_idx];
    let uv = QUAD_UV[tri_idx];

    // Overscan factor for anti-aliasing at edges
    let overscan = 2.2;

    // Determine quad extent based on shape:
    // Capsule/RoundedBox need elongated quads to cover the full shape.
    var quad_w = inst.radius * overscan;
    var quad_h = inst.radius * overscan;

    // For non-sphere shapes, extend the quad to cover full geometry
    let is_capsule = inst.shape_type > 0.5 && inst.shape_type < 1.5;
    let is_box = inst.shape_type > 1.5;
    if (is_capsule || is_box) {
        // half_height is in world units — add it to the quad extent
        quad_w = (inst.radius + inst.half_height) * overscan;
        quad_h = (inst.radius + inst.half_height) * overscan;
    }

    // Apply entity rotation to the quad corners
    let cos_r = cos(inst.rotation);
    let sin_r = sin(inst.rotation);
    let scaled = vec2(pos.x * quad_w * 2.0, pos.y * quad_h * 2.0);
    let rotated = vec2(
        scaled.x * cos_r - scaled.y * sin_r,
        scaled.x * sin_r + scaled.y * cos_r,
    );
    let world_pos = rotated + inst.position;
    out.clip_position = camera.projection * vec4<f32>(world_pos, 0.0, 1.0);

    // Pass UV and normalize shape params relative to radius.
    // Multiply by overscan so |local_uv|=1.0 maps to exactly inst.radius
    // in world space (the extra quad area beyond 1.0 is discarded by the SDF).
    out.local_uv = uv * overscan;
    if (is_capsule || is_box) {
        // Scale UV to cover the full elongated extent
        let extent = (inst.radius + inst.half_height) / inst.radius;
        out.local_uv = uv * extent * overscan;
    }
    out.base_color = inst.color;
    out.shininess = inst.shininess;
    out.emissive = inst.emissive;
    out.shape_type = inst.shape_type;
    out.half_height_norm = inst.half_height / max(inst.radius, 0.001);
    out.extra_norm = inst.extra / max(inst.radius, 0.001);
    out.world_center = inst.position;
    out.world_radius = inst.radius;
    out.rotation = inst.rotation;

    return out;
}

// ---- SDF Primitives ----
// All operate in normalized space where sphere radius = 1.0.

fn sdf_sphere(p: vec2<f32>) -> f32 {
    return length(p) - 1.0;
}

fn sdf_capsule(p: vec2<f32>, half_h: f32) -> f32 {
    // Capsule along local Y axis: clamp Y to [-half_h, half_h], measure distance to tube
    var q = p;
    q.y = q.y - clamp(q.y, -half_h, half_h);
    return length(q) - 1.0;
}

fn sdf_rounded_box(p: vec2<f32>, half_h: f32, corner_r: f32) -> f32 {
    // 2D rounded box: half-extents are (1.0, half_h), corner rounding = corner_r
    let half_extents = vec2(1.0, half_h);
    let d = abs(p) - half_extents + vec2(corner_r);
    return length(max(d, vec2(0.0))) + min(max(d.x, d.y), 0.0) - corner_r;
}



// ---- Fragment Shader: Raymarched SDF ----

// Fallback light direction when no scene lights (top-left in Y-down coordinate system)
const FALLBACK_LIGHT_DIR = vec3<f32>(-0.4, -0.6, 0.7);

@fragment
fn fs_sdf(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = in.local_uv;
    var dist: f32;
    var normal: vec3<f32>;

    // Track if this is a striped pool ball (extra_norm > 0.5)
    var is_striped_ball = false;

    if (in.shape_type < 0.5) {
        // ---- Sphere ----
        let d2 = dot(p, p);
        if (d2 > 1.0) {
            discard;
        }
        dist = sqrt(d2) - 1.0;
        let z = sqrt(1.0 - d2);
        normal = vec3(p.x, p.y, z);

        // Check for striped ball (pool balls 9-15)
        // Note: extra_norm = extra / radius, so 1.0/12.0 ≈ 0.08 for typical balls
        is_striped_ball = in.extra_norm > 0.01;
    } else if (in.shape_type < 1.5) {
        // ---- Capsule ----
        // Analytic normal: vector from clamped axis point to p
        let half_h = in.half_height_norm;
        let q_y_clamped = clamp(p.y, -half_h, half_h);
        let closest = vec2(0.0, q_y_clamped);
        let diff = p - closest;
        let d2 = dot(diff, diff);
        
        // Distance check (radius = 1.0)
        dist = sqrt(d2) - 1.0;
        if (dist > 0.02) {
            discard;
        }
        
        // Normal is direction of diff + derived Z
        // If p is exactly on axis (diff ~ 0), default to Z up
        if (d2 < 0.0001) {
            normal = vec3(0.0, 0.0, 1.0);
        } else {
            let n2d = normalize(diff);
            // Fake Z component from distance to surface (hemisphere profile)
            // For capsule body (d=1.0 at surface), z should be 0.
            // But we want 3D volume look. 
            // Correct way for capsule: 
            // Surface is section of cylinder or sphere.
            // On cylinder part: normal Z is sqrt(1 - y^2) ... no wait.
            // The capsule radius is 1.0.
            // At surface (dist=0), the 2D projected point 'p' matches the 3D surface point (x,y,z).
            // Actually, for a capsule along Y, the cross section is a circle.
            // The distance from axis 'diff' IS the 2D normal.
            // The Z height is sqrt(1.0 - |diff|^2).
            // But 'diff' is (p.x, p.y - q_y_clamped).
            // |diff| is distance from axis.
            // If |diff| > 1.0, we are outside.
            // So Z = sqrt(1.0 - d2).
            let z = sqrt(max(1.0 - d2, 0.0));
            normal = vec3(n2d, z);
        }
    } else {
        // ---- RoundedBox ----
        // Analytic gradient for rounded box
        // d = length(max(q, 0)) + min(max(q.x, q.y), 0) - r
        // where q = abs(p) - b
        // Gradient of box(p, b) is:
        // if outside: normalize(max(q, 0)) * sign(p)
        // if inside: (0, 1) or (1, 0) based on closest edge * sign(p)
        
        let half_h = in.half_height_norm;
        let corner_r = in.extra_norm;
        let half_extents = vec2(1.0, half_h);
        
        // Symmetry
        let p_abs = abs(p);
        let sign_p = sign(p);
        let q = p_abs - half_extents + vec2(corner_r);
        
        // Distance
        let dist_vec = max(q, vec2(0.0));
        let outside_dist = length(dist_vec);
        let inside_dist = min(max(q.x, q.y), 0.0);
        dist = outside_dist + inside_dist - corner_r;
        
        if (dist > 0.02) {
            discard;
        }
        
        // Gradient
        var grad: vec2<f32>;
        if (inside_dist < 0.0) {
            // Inside box (not corner area yet)
            // Gradient is along axis of max component
            if (q.x > q.y) {
                grad = vec2(1.0, 0.0);
            } else {
                grad = vec2(0.0, 1.0);
            }
        } else {
            // Outside or on corner
            // If outside straight edge, max(q, 0) picks that axis
            // If outside corner, max(q, 0) checks both
            // Normalize handles the direction
            grad = normalize(dist_vec);
            // If exactly 0,0 (inside "inner" box), degenerate?
            // But q includes corner_r. 
            // Wait, q = abs(p) - (size - r).
            // If we are deep inside, q is negative.
            // If we are in the rounding zone, q is complex.
        }
        
        // Apply sign to restore quadrant
        let n2d = grad * sign_p;
        
        // Fake Z: approximations for box are hard?
        // Let's use simple heuristic: edge falloff.
        // Or just keep it flat-ish with rim?
        // Actually, let's stick to a Z-up normal for flat face, and curve at edges.
        // Distance field gives us distance to edge.
        // Use generalized Z derivation
        let n_len_sq = dot(n2d, n2d);
        let z = sqrt(max(1.0 - n_len_sq * 0.5, 0.1)); // Fallback heuristic
        normal = normalize(vec3(n2d, z));
    }

    // Determine final base color (stripe detection for pool balls)
    // Stripes use LOCAL p.y so they rotate with the ball (correct!)
    var final_base_color = in.base_color;
    if (is_striped_ball) {
        // Striped ball: colored band in middle, white outside
        // p.y is in normalized local space, stripe band covers |p.y| < 0.35
        let stripe_width = 0.35;
        let in_stripe = abs(p.y) < stripe_width;
        if (!in_stripe) {
            final_base_color = vec3<f32>(1.0, 1.0, 1.0);  // White
        }
    }

    // Transform normal from local space to world space for lighting
    // The normal's XY components need to be rotated by the entity's rotation
    // so that light reflections stay fixed in world space as the ball spins
    let cos_r = cos(in.rotation);
    let sin_r = sin(in.rotation);
    let world_normal = vec3<f32>(
        normal.x * cos_r - normal.y * sin_r,
        normal.x * sin_r + normal.y * cos_r,
        normal.z
    );

    // View direction (orthographic, looking down -Z)
    let view_dir = vec3<f32>(0.0, 0.0, 1.0);

    // Fresnel rim glow (use world_normal.z as facing ratio)
    let fresnel = pow(1.0 - max(world_normal.z, 0.0), 3.0);
    let rim = fresnel * 0.4;

    // Get light count and ambient from uniforms
    let ambient = light_uniforms.ambient_and_count.xyz;
    let light_count = u32(light_uniforms.ambient_and_count.w);

    // Compute world position of this fragment
    // p is in normalized local space [-1, 1], need to rotate to world space first
    let p_world = vec2<f32>(
        p.x * cos_r - p.y * sin_r,
        p.x * sin_r + p.y * cos_r
    );
    let frag_world_pos = in.world_center + p_world * in.world_radius;

    // Accumulate lighting from scene lights
    var diffuse_accum = vec3<f32>(0.0);
    var specular_accum = vec3<f32>(0.0);

    if (light_count > 0u) {
        // Dynamic scene lights
        for (var i = 0u; i < light_count; i = i + 1u) {
            let light = lights[i];
            let light_pos = vec2<f32>(light.x, light.y);
            let delta = light_pos - frag_world_pos;
            let d = length(delta);

            // Smooth quadratic falloff: (1 - d/r)^2
            let norm_dist = saturate(1.0 - d / light.radius);
            let attenuation = norm_dist * norm_dist;

            // Light direction in 3D: (dx, dy, height_above_surface)
            let light_height = light.radius * 0.3;
            let light_dir = normalize(vec3<f32>(delta, light_height));

            // Diffuse (N·L) - scaled down to avoid over-saturation with multiple lights
            let n_dot_l = max(dot(world_normal, light_dir), 0.0);
            let light_color = vec3<f32>(light.r, light.g, light.b) * light.intensity;
            diffuse_accum = diffuse_accum + light_color * n_dot_l * attenuation * 0.5;

            // Specular (Blinn-Phong) - toned down significantly for pool balls
            let half_dir = normalize(light_dir + view_dir);
            let spec_angle = max(dot(world_normal, half_dir), 0.0);
            let specular = pow(spec_angle, in.shininess);
            specular_accum = specular_accum + light_color * specular * attenuation * 0.35;
        }
    } else {
        // Fallback: use hardcoded directional light when no scene lights
        let light_dir = normalize(FALLBACK_LIGHT_DIR);
        let n_dot_l = max(dot(world_normal, light_dir), 0.0);
        diffuse_accum = vec3<f32>(0.7) * n_dot_l;

        let half_dir = normalize(light_dir + view_dir);
        let spec_angle = max(dot(world_normal, half_dir), 0.0);
        specular_accum = vec3<f32>(0.5) * pow(spec_angle, in.shininess);
    }

    // Combine lighting: ambient + diffuse + specular + rim
    // When using dynamic lights, reduce rim effect to avoid over-brightness
    let rim_factor = select(rim, rim * 0.3, light_count > 0u);
    let lit = final_base_color * (ambient + diffuse_accum) + specular_accum + final_base_color * rim_factor;

    // HDR emissive multiplier (only for emissive shapes, not for lit pool balls)
    let hdr_mult = 1.0 + in.emissive * SDF_EMISSIVE_MULT;
    let final_color = lit * hdr_mult;

    // Edge anti-aliasing
    let aa = smoothstep(0.0, 0.02, -dist);

    return vec4<f32>(final_color, aa);
}

// ---- Normal Buffer Pass: Write flat normal for SDF shapes ----
// This prevents sprite normal maps from bleeding onto SDF shapes.

@fragment
fn fs_sdf_normal(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = in.local_uv;

    // Discard outside shape bounds (same logic as main fragment shader)
    if (in.shape_type < 0.5) {
        // Sphere
        let d2 = dot(p, p);
        if (d2 > 1.0) {
            discard;
        }
    } else if (in.shape_type < 1.5) {
        // Capsule
        let half_h = in.half_height_norm;
        let q_y_clamped = clamp(p.y, -half_h, half_h);
        let closest = vec2(0.0, q_y_clamped);
        let diff = p - closest;
        let dist = length(diff) - 1.0;
        if (dist > 0.02) {
            discard;
        }
    } else {
        // RoundedBox
        let half_h = in.half_height_norm;
        let corner_r = in.extra_norm;
        let half_extents = vec2(1.0, half_h);
        let q = abs(p) - half_extents + vec2(corner_r);
        let dist_vec = max(q, vec2(0.0));
        let outside_dist = length(dist_vec);
        let inside_dist = min(max(q.x, q.y), 0.0);
        let dist = outside_dist + inside_dist - corner_r;
        if (dist > 0.02) {
            discard;
        }
    }

    // Write flat normal (0, 0, 1) encoded as (0.5, 0.5, 1.0)
    return vec4<f32>(0.5, 0.5, 1.0, 1.0);
}
