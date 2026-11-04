#version 450

layout(location = 0) in vec4 line_color;
layout(location = 1) flat in float encode_srgb;

layout(location = 0) out vec4 output_color;

void main() {
    vec3 color = line_color.rgb;
    if (encode_srgb > 0.5) {
        color = pow(color, vec3(1.0 / 2.2));
    }
    output_color = vec4(color, line_color.a);
}
