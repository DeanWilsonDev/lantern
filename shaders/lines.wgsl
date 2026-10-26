// Debug lines: world-space positions with a colour, depth-tested against the scene but not
// writing depth.

struct FrameUniforms {
    view_projection: mat4x4<f32>,
    // w: 1 when the shader must sRGB-encode its output.
    output_flags: vec4<f32>,
};

@group(1) @binding(0) var<uniform> frame: FrameUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) @interpolate(flat) encode_srgb: f32,
};

@vertex
fn vertex_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = frame.view_projection * vec4<f32>(input.position, 1.0);
    output.color = input.color;
    output.encode_srgb = frame.output_flags.w;
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    var color = input.color.rgb;
    if (input.encode_srgb > 0.5) {
        color = pow(color, vec3<f32>(1.0 / 2.2));
    }
    return vec4<f32>(color, input.color.a);
}
