// ZapEngine — Fullscreen Composite Shader
// Blits a cached layer texture onto the screen with alpha blending.
// Uses a fullscreen triangle (3 vertices, no vertex buffer).

@group(0) @binding(0) var layer_tex: texture_2d<f32>;
@group(0) @binding(1) var layer_sampler: sampler;

struct CompositeVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_composite(@builtin(vertex_index) vi: u32) -> CompositeVertexOutput {
    // Fullscreen triangle: vertices at (-1,-1), (3,-1), (-1,3)
    let x = f32(i32(vi & 1u)) * 4.0 - 1.0;
    let y = f32(i32(vi >> 1u)) * 4.0 - 1.0;
    var out: CompositeVertexOutput;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    // UV: map clip [-1,1] → [0,1], flip Y for texture coordinates
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

@fragment
fn fs_composite(in: CompositeVertexOutput) -> @location(0) vec4<f32> {
    return textureSample(layer_tex, layer_sampler, in.uv);
}
