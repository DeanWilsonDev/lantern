// Bloom, second half: grow a small blurred picture back up one step and add it to the next larger one.
//
// bloom-downsample.frag made a chain of ever smaller, ever blurrier pictures. This pass runs from the smallest back up:
// it reads picture n + 1, blurs it with a 3x3 tent filter (corners weigh 1, edges 2, the middle 4, out of 16) while
// stretching it to picture n's size, and the pipeline's additive blending adds the result onto picture n. After the last
// step, picture 0 holds the glow of every width at once: narrow from the large pictures and wide from the small ones.
//
// There is no maths about adding here: the pipeline blends with "source * 1 + destination * 1", so this shader only has to
// write the blurred colour.
#version 450

layout(set = 2, binding = 0) uniform sampler2D smaller_image;

// Slot 0: x, y: the size of one texel of the smaller picture in uv units.
layout(set = 3, binding = 0) uniform UpsampleUniforms {
    vec4 texel;
} settings;

layout(location = 0) in vec2 source_uv;
layout(location = 0) out vec4 output_color;

void main() {
    vec2 step_size = settings.texel.xy;
    vec3 total = texture(smaller_image, source_uv + vec2(-1.0, -1.0) * step_size).rgb
        + texture(smaller_image, source_uv + vec2( 0.0, -1.0) * step_size).rgb * 2.0
        + texture(smaller_image, source_uv + vec2( 1.0, -1.0) * step_size).rgb
        + texture(smaller_image, source_uv + vec2(-1.0,  0.0) * step_size).rgb * 2.0
        + texture(smaller_image, source_uv).rgb * 4.0
        + texture(smaller_image, source_uv + vec2( 1.0,  0.0) * step_size).rgb * 2.0
        + texture(smaller_image, source_uv + vec2(-1.0,  1.0) * step_size).rgb
        + texture(smaller_image, source_uv + vec2( 0.0,  1.0) * step_size).rgb * 2.0
        + texture(smaller_image, source_uv + vec2( 1.0,  1.0) * step_size).rgb;
    output_color = vec4(total / 16.0, 1.0);
}
