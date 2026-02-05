// ZapEngine — SDF Molecule Shader (Raymarched Spheres)
// Renders instanced quads with per-fragment sphere raymarching.
// Phong shading + Fresnel rim glow + HDR emissive.

// ---- Camera Uniform (shared with sprite pipeline) ----

struct Camera {
    projection: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> camera: Camera;

// ---- SDF Instance Storage Buffer ----
// Matches SDFInstance layout: 12 floats = 48 bytes per instance.
// [x, y, radius, rotation, r, g, b, shininess, emissive, pad, pad, pad]

struct SDFInstance {
    position: vec2<f32>,
    radius: f32,
    rotation: f32,
    color: vec3<f32>,
    shininess: f32,
    emissive: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
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
    let quad_size = inst.radius * overscan;

    // Scale and translate to world position (no rotation on the quad itself)
    let world_pos = pos * quad_size + inst.position;
    out.clip_position = camera.projection * vec4<f32>(world_pos, 0.0, 1.0);

    out.local_uv = uv;
    out.base_color = inst.color;
    out.shininess = inst.shininess;
    out.emissive = inst.emissive;

    return out;
}

// ---- Fragment Shader: Raymarched Sphere ----

// Light direction (top-left in Y-down coordinate system)
const LIGHT_DIR = vec3<f32>(-0.4, -0.6, 0.7);

@fragment
fn fs_sdf(in: VertexOutput) -> @location(0) vec4<f32> {
    // Distance from center in UV space ([-1, 1])
    let d2 = dot(in.local_uv, in.local_uv);

    // Discard outside sphere (radius = 1.0 in UV space)
    if (d2 > 1.0) {
        discard;
    }

    // Reconstruct sphere normal from UV
    let z = sqrt(1.0 - d2);
    let normal = vec3<f32>(in.local_uv.x, in.local_uv.y, z);

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

    // Fresnel rim glow
    let fresnel = pow(1.0 - z, 3.0);
    let rim = fresnel * 0.4;

    // Combine lighting
    let lit = in.base_color * (ambient + diffuse * 0.7) + vec3<f32>(1.0) * specular * 0.5 + in.base_color * rim;

    // HDR emissive multiplier
    let hdr_mult = 1.0 + in.emissive * SDF_EMISSIVE_MULT;
    let final_color = lit * hdr_mult;

    // Edge anti-aliasing
    let edge_dist = 1.0 - d2;
    let aa = smoothstep(0.0, 0.05, edge_dist);

    return vec4<f32>(final_color, aa);
}
