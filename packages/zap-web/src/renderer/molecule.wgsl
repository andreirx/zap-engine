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

// Light direction (top-left in Y-down coordinate system)
const LIGHT_DIR = vec3<f32>(-0.4, -0.6, 0.7);

@fragment
fn fs_sdf(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = in.local_uv;
    var dist: f32;
    var normal: vec3<f32>;

    if (in.shape_type < 0.5) {
        // ---- Sphere ----
        let d2 = dot(p, p);
        if (d2 > 1.0) {
            discard;
        }
        dist = sqrt(d2) - 1.0;
        let z = sqrt(1.0 - d2);
        normal = vec3(p.x, p.y, z);
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

    // Normalize light direction
    let light = normalize(LIGHT_DIR);

    // Phong shading
    let ambient = 0.15;
    let n_dot_l = max(dot(normal, light), 0.0);
    let diffuse = n_dot_l;

    // Specular (Blinn-Phong)
    let view_dir = vec3<f32>(0.0, 0.0, 1.0);
    let half_dir = normalize(light + view_dir);
    let spec_angle = max(dot(normal, half_dir), 0.0);
    let specular = pow(spec_angle, in.shininess);

    // Fresnel rim glow (use normal.z as facing ratio)
    let fresnel = pow(1.0 - max(normal.z, 0.0), 3.0);
    let rim = fresnel * 0.4;

    // Combine lighting
    let lit = in.base_color * (ambient + diffuse * 0.7) + vec3<f32>(1.0) * specular * 0.5 + in.base_color * rim;

    // HDR emissive multiplier
    let hdr_mult = 1.0 + in.emissive * SDF_EMISSIVE_MULT;
    let final_color = lit * hdr_mult;

    // Edge anti-aliasing
    let aa = smoothstep(0.0, 0.02, -dist);

    return vec4<f32>(final_color, aa);
}
