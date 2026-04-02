// ZapEngine — Visibility Mask Post-Process Shader
// Fullscreen pass that multiplies scene color by a visibility mask texture.
// Mask is R8Unorm: 0.0 = hidden (black), 1.0 = fully visible.
// Applied after world content (layers 0-4) but before UI (layer 5).

@group(0) @binding(0) var scene_tex: texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;
@group(0) @binding(2) var vis_tex: texture_2d<f32>;
@group(0) @binding(3) var vis_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_visibility(@builtin(vertex_index) vi: u32) -> VertexOutput {
    // Fullscreen triangle: vertices at (-1,-1), (3,-1), (-1,3)
    let x = f32(i32(vi & 1u)) * 4.0 - 1.0;
    let y = f32(i32(vi >> 1u)) * 4.0 - 1.0;
    var out: VertexOutput;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    // UV: map clip [-1,1] → [0,1], flip Y for texture coordinates
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

@fragment
fn fs_visibility(in: VertexOutput) -> @location(0) vec4<f32> {
    let scene = textureSample(scene_tex, scene_sampler, in.uv);
    let vis = textureSample(vis_tex, vis_sampler, in.uv).r;
    return vec4<f32>(scene.rgb * vis, scene.a);
}
