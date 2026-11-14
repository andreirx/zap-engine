// ZapEngine — Visibility Mask Post-Process Shader
// Fullscreen pass that multiplies scene color by a visibility mask texture.
// Mask is R8Unorm: 0.0 = hidden (black), 1.0 = fully visible.
// Applied after world content (layers 0-4) but before UI (layer 5).
//
// Rule R-VISIBILITY-MASK (ADR-040), transcribed from camera.ts maskUv:
// - no camera (view.z == 0): the mask is sampled at the screen uv, as before;
// - camera: the mask covers the world rectangle 0..gameW x 0..gameH; a pixel
//   whose world point lies outside it is not darkened.

struct VisibilityUniforms {
    // left, top, width, height of the world-space visible area; all 0 with no camera
    view: vec4<f32>,
    // xy = gameWidth, gameHeight; zw unused
    game: vec4<f32>,
};

@group(0) @binding(0) var scene_tex: texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;
@group(0) @binding(2) var vis_tex: texture_2d<f32>;
@group(0) @binding(3) var vis_sampler: sampler;
@group(0) @binding(4) var<uniform> vis_uniforms: VisibilityUniforms;

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
    let has_camera = vis_uniforms.view.z != 0.0;
    let world_pos = vis_uniforms.view.xy + in.uv * vis_uniforms.view.zw;
    let world_uv = world_pos / vis_uniforms.game.xy;
    // Sampled unconditionally: textureSample must stay in uniform control flow.
    let mask = textureSample(vis_tex, vis_sampler, select(in.uv, world_uv, has_camera)).r;
    let inside = all(world_uv >= vec2<f32>(0.0)) && all(world_uv <= vec2<f32>(1.0));
    let vis = select(mask, 1.0, has_camera && !inside);
    return vec4<f32>(scene.rgb * vis, scene.a);
}
