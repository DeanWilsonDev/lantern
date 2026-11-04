#version 450

layout(set = 1, binding = 0) uniform FrameUniforms {
    mat4 view_projection;
    vec4 output_flags;
} frame;

layout(location = 0) in vec3 input_position;
layout(location = 1) in vec4 input_color;

layout(location = 0) out vec4 line_color;
layout(location = 1) flat out float encode_srgb;

void main() {
    gl_Position = frame.view_projection * vec4(input_position, 1.0);
    line_color = input_color;
    encode_srgb = frame.output_flags.w;
}
