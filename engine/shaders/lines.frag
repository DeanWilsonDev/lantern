// The debug line pipeline's fragment shader: the line's own colour, unlit. The pipeline blends by alpha and tests
// depth without writing it, so lines hide behind walls but not behind each other.
#version 450

layout(location = 0) in vec4 line_color;
layout(location = 1) flat in float encode_srgb;

layout(location = 0) out vec4 output_color;

void main() {
    vec3 color = line_color.rgb;
    // A 2.2 power curve: close enough to sRGB for debug colours, and cheaper than the exact function in lit.frag.
    if (encode_srgb > 0.5) {
        color = pow(color, vec3(1.0 / 2.2));
    }
    output_color = vec4(color, line_color.a);
}
