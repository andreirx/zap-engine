// ZapEngine — Vector/Polygon Shader
// Renders Lyon-tessellated geometry with per-vertex color.
// Uses same camera uniform as other pipelines.

// ---- Uniforms ----

struct Camera {
    projection: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> camera: Camera;

// ---- HDR multiplier ----
// Can be overridden per render tier at pipeline creation.
// hdr-edr: higher values for extended dynamic range, sdr: 1.0
override VECTOR_HDR_MULT: f32 = 1.0;

// ---- Vertex I/O ----

struct VectorVertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct VectorVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_vector(input: VectorVertexInput) -> VectorVertexOutput {
    var out: VectorVertexOutput;
    out.clip_position = camera.projection * vec4<f32>(input.position, 0.0, 1.0);
    out.color = input.color;
    return out;
}

@fragment
fn fs_vector(in: VectorVertexOutput) -> @location(0) vec4<f32> {
    // Apply HDR multiplier to RGB, preserve alpha
    return vec4<f32>(in.color.rgb * VECTOR_HDR_MULT, in.color.a);
}
