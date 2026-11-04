#version 450

layout(set = 2, binding = 0) uniform sampler2D atlas_texture;

layout(location = 0) in vec2 atlas_uv;
layout(location = 1) in vec4 glyph_color;
layout(location = 2) flat in float encode_srgb;

layout(location = 0) out vec4 output_color;

void main() {
    vec4 texel = texture(atlas_texture, atlas_uv);
    vec3 color = texel.rgb * glyph_color.rgb;
    if (encode_srgb > 0.5) {
        color = pow(color, vec3(1.0 / 2.2));
    }
    output_color = vec4(color, texel.a * glyph_color.a);
}
