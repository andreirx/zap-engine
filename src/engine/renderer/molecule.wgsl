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

// Central-difference normal estimation for non-analytic shapes
fn estimate_normal(p: vec2<f32>, shape_type: f32, half_h: f32, corner_r: f32) -> vec3<f32> {
    let eps = 0.01;
    let dx = vec2(eps, 0.0);
    let dy = vec2(0.0, eps);

    var d_px: f32;
    var d_nx: f32;
    var d_py: f32;
    var d_ny: f32;

    if (shape_type < 1.5) {
        // Capsule
        d_px = sdf_capsule(p + dx, half_h);
        d_nx = sdf_capsule(p - dx, half_h);
        d_py = sdf_capsule(p + dy, half_h);
        d_ny = sdf_capsule(p - dy, half_h);
    } else {
        // RoundedBox
        d_px = sdf_rounded_box(p + dx, half_h, corner_r);
        d_nx = sdf_rounded_box(p - dx, half_h, corner_r);
        d_py = sdf_rounded_box(p + dy, half_h, corner_r);
        d_ny = sdf_rounded_box(p - dy, half_h, corner_r);
    }

    let grad = vec2(d_px - d_nx, d_py - d_ny);
    let grad_len = length(grad);
    if (grad_len < 0.0001) {
        return vec3(0.0, 0.0, 1.0);
    }
    let n2d = grad / grad_len;
    // Fake Z component from distance to surface
    let z = sqrt(max(1.0 - dot(n2d, n2d) * 0.5, 0.1));
    return normalize(vec3(n2d, z));
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
        dist = sdf_capsule(p, in.half_height_norm);
        if (dist > 0.02) {
            discard;
        }
        normal = estimate_normal(p, in.shape_type, in.half_height_norm, 0.0);
    } else {
        // ---- RoundedBox ----
        dist = sdf_rounded_box(p, in.half_height_norm, in.extra_norm);
        if (dist > 0.02) {
            discard;
        }
        normal = estimate_normal(p, in.shape_type, in.half_height_norm, in.extra_norm);
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
