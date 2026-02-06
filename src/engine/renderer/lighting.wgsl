// ZapEngine — Dynamic Point Light Shader (2D)
// Fullscreen post-process: samples scene color, accumulates point light contributions.
// Uses smooth quadratic falloff: attenuation = (1 - dist/radius)^2.
//
// When ambient is (1,1,1) and no lights, output equals input (no visual change).

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
    // xy = projWidth, projHeight (world-space visible area)
    proj_size: vec4<f32>,
};

@group(0) @binding(0) var scene_tex: texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;
@group(0) @binding(2) var<uniform> uniforms: LightUniforms;
@group(0) @binding(3) var<storage, read> lights: array<PointLight>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_lighting(@builtin(vertex_index) vi: u32) -> VertexOutput {
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
fn fs_lighting(in: VertexOutput) -> @location(0) vec4<f32> {
    let scene_color = textureSample(scene_tex, scene_sampler, in.uv);

    let ambient = uniforms.ambient_and_count.xyz;
    let light_count = u32(uniforms.ambient_and_count.w);

    // Convert UV to world position (orthographic projection, Y-down)
    let world_pos = in.uv * uniforms.proj_size.xy;

    // Accumulate light contributions
    var total_light = ambient;

    for (var i = 0u; i < light_count; i = i + 1u) {
        let light = lights[i];
        let light_pos = vec2<f32>(light.x, light.y);
        let d = distance(world_pos, light_pos);

        // Smooth quadratic falloff: (1 - d/r)^2
        let norm_dist = saturate(1.0 - d / light.radius);
        let attenuation = norm_dist * norm_dist;

        let contribution = vec3<f32>(light.r, light.g, light.b) * light.intensity * attenuation;
        total_light = total_light + contribution;
    }

    // Multiply scene color by accumulated lighting
    return vec4<f32>(scene_color.rgb * total_light, scene_color.a);
}
