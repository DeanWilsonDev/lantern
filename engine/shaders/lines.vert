// The debug line pipeline's vertex shader. Lines are already in world space, so there is no model matrix: one
// view-projection multiply per vertex.
#version 450

// Slot 0, pushed once per frame: MatrixAndVectorUniforms in GpuUniforms.nox, with the sRGB flag in output_flags.w.
layout(set = 1, binding = 0) uniform FrameUniforms {
    mat4 view_projection;
    vec4 output_flags;
} frame;

// The vertex, as RenderWorld.nox's LineVertex lays it out (28 bytes).
layout(location = 0) in vec3 input_position;
layout(location = 1) in vec4 input_color;

layout(location = 0) out vec4 line_color;
// `flat`: the same for every vertex, so not interpolated.
layout(location = 1) flat out float encode_srgb;

void main() {
    gl_Position = frame.view_projection * vec4(input_position, 1.0);
    line_color = input_color;
    encode_srgb = frame.output_flags.w;
}
