// ZapEngine — WGSL Shaders
// Reads per-instance data from a storage buffer (SharedArrayBuffer-backed).
// Two fragment entry points: standard alpha blend and additive HDR glow.

// ---- Uniforms ----

struct Camera {
    projection: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> camera: Camera;

// ---- Textures ----

@group(1) @binding(0) var t_atlas: texture_2d<f32>;
@group(1) @binding(1) var s_atlas: sampler;

// ---- Instance data from storage buffer ----
// Matches RenderInstance layout: 8 floats = 32 bytes per instance.
// [x, y, rotation, scale, sprite_col, alpha, cell_span, atlas_row]

struct Instance {
    position: vec2<f32>,
    rotation: f32,
    scale: f32,
    sprite_col: f32,
    alpha: f32,
    cell_span: f32,
    atlas_row: f32,
};

@group(2) @binding(0) var<storage, read> instances: array<Instance>;

// ---- Segment colors UBO (effects pipeline only, group 3) ----

struct SegmentColors {
    values: array<vec4<f32>, 13>,
};
@group(3) @binding(0) var<uniform> segment_colors: SegmentColors;

fn segment_color(idx: f32) -> vec3<f32> {
    let i = min(u32(idx + 0.5), 12u);
    return segment_colors.values[i].xyz;
}

// ---- Vertex I/O ----

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) alpha: f32,
    @location(2) color_idx: f32,
};

const QUAD_POS = array<vec2<f32>, 4>(
    vec2(-0.5, -0.5),
    vec2( 0.5, -0.5),
    vec2(-0.5,  0.5),
    vec2( 0.5,  0.5),
);
const QUAD_UV = array<vec2<f32>, 4>(
    vec2(0.0, 0.0),
    vec2(1.0, 0.0),
    vec2(0.0, 1.0),
    vec2(1.0, 1.0),
);
const QUAD_IDX = array<u32, 6>(0u, 1u, 2u, 2u, 1u, 3u);

// Texture atlas layout — overridable per pipeline.
override ATLAS_COLS: f32 = 16.0;
override ATLAS_ROWS: f32 = 8.0;

// HDR glow multiplier — set per render tier at pipeline creation.
// hdr-edr: 6.4, hdr-srgb: 3.0, sdr: 1.0
override EFFECTS_HDR_MULT: f32 = 6.4;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let inst = instances[input.instance_index];
    let tri_idx = QUAD_IDX[input.vertex_index];
    let pos = QUAD_POS[tri_idx];
    let uv = QUAD_UV[tri_idx];

    // Scale is now world-space size directly (no hardcoded tile_size multiplier).
    // Games write the actual rendered size into inst.scale.
    let tile_size = inst.scale;

    // Apply rotation
    let cos_r = cos(inst.rotation);
    let sin_r = sin(inst.rotation);
    let rotated = vec2<f32>(
        pos.x * cos_r - pos.y * sin_r,
        pos.x * sin_r + pos.y * cos_r,
    );

    // Scale and translate to world position
    let world_pos = rotated * tile_size + inst.position;
    out.clip_position = camera.projection * vec4<f32>(world_pos, 0.0, 1.0);

    // Map sprite_col to atlas UV.
    let col = inst.sprite_col % ATLAS_COLS;
    let row = inst.atlas_row;

    // cell_span encodes UV cell count: 1.0 = single cell, 2.0 = 2×2 block
    let cell_size = max(inst.cell_span, 1.0);
    let uv_origin = vec2<f32>(col / ATLAS_COLS, row / ATLAS_ROWS);
    let uv_size = vec2<f32>(cell_size / ATLAS_COLS, cell_size / ATLAS_ROWS);
    out.tex_coord = uv_origin + uv * uv_size;

    out.alpha = inst.alpha;
    out.color_idx = 0.0;

    return out;
}

// Standard alpha-blended fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_atlas, s_atlas, in.tex_coord);
    return color * in.alpha;
}

// ---- Effects vertex shader (raw triangle list, non-instanced) ----

struct EffectsVertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
};

@vertex
fn vs_effects(input: EffectsVertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.projection * vec4<f32>(input.position.xy, 0.0, 1.0);
    out.tex_coord = input.tex_coord;
    out.alpha = 1.0;
    out.color_idx = input.position.z;
    return out;
}

// Additive fragment shader for HDR glow effects (electric arcs).
// Procedural lightsaber profile: white-hot core with colored glow halo.
// Multiplies by 6.4 to push into EDR range on supported displays.
@fragment
fn fs_additive(in: VertexOutput) -> @location(0) vec4<f32> {
    let d = abs(in.tex_coord.x * 2.0 - 1.0);

    let core = exp(-d * d * 16.0);
    let halo = exp(-d * d * 3.0);

    let tip = in.tex_coord.y;

    let base = segment_color(in.color_idx);
    let rgb = (vec3<f32>(1.0, 1.0, 1.0) * core * 0.6 + base * halo) * EFFECTS_HDR_MULT * tip;
    let a = halo * tip;
    return vec4<f32>(rgb, a);
}
