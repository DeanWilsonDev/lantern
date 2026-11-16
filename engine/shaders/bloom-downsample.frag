// Bloom, first half: shrink the picture to half size, again and again, so each smaller picture holds a blurrier, wider
// view of the bright parts.
//
// Bloom is the glow around bright things. A camera lens and an eye both scatter a little of a bright light's energy over
// its surroundings, and a renderer can imitate that by blurring the bright parts of the frame and adding the blur back on
// top. A wide blur is expensive at full size, so the renderer blurs by shrinking: each pass here reads one image and writes
// one half as wide and half as tall, and a later pass (bloom-upsample.frag) grows them back, adding as it goes.
//
// The filter is the 13-tap downsample from the Call of Duty: Advanced Warfare talk: thirteen samples arranged so each
// output pixel averages a 4x4 area with soft overlapping weights, which avoids the flicker a plain 2x2 average shows when
// a small bright thing moves across the image.
//
// The first pass (mode 1) also picks out the bright parts. It subtracts a threshold from the pixel's brightness with a
// soft knee, so a pixel just above the threshold glows a little and one far above glows a lot, rather than a hard cut. It
// weights each of its four sample groups by 1 / (1 + brightness), the "Karis average", so a single very bright pixel
// ("a firefly") can't dominate its neighbours and make the glow sparkle.
#version 450

layout(set = 2, binding = 0) uniform sampler2D source_image;

// Slot 0: x, y: the size of one source texel in uv units, z: the brightness threshold, w: 1 for the first pass.
layout(set = 3, binding = 0) uniform DownsampleUniforms {
    vec4 texel_threshold_mode;
} settings;

layout(location = 0) in vec2 source_uv;
layout(location = 0) out vec4 output_color;

float Brightness(vec3 color) {
    return max(max(color.r, color.g), color.b);
}

// Keeps only the part of a colour above the threshold. The knee is a quadratic ramp over the last tenth of the threshold
// below it, so the cut-off has no visible edge.
vec3 KeepBrightPart(vec3 color, float threshold) {
    float brightness = Brightness(color);
    float knee = max(threshold * 0.1, 0.0001);
    float soft = clamp(brightness - threshold + knee, 0.0, 2.0 * knee);
    soft = soft * soft / (4.0 * knee);
    float contribution = max(brightness - threshold, soft) / max(brightness, 0.0001);
    return color * contribution;
}

vec3 Sample(vec2 offset_in_texels) {
    return texture(source_image, source_uv + offset_in_texels * settings.texel_threshold_mode.xy).rgb;
}

void main() {
    vec3 a = Sample(vec2(-2.0, -2.0));
    vec3 b = Sample(vec2( 0.0, -2.0));
    vec3 c = Sample(vec2( 2.0, -2.0));
    vec3 d = Sample(vec2(-2.0,  0.0));
    vec3 e = Sample(vec2( 0.0,  0.0));
    vec3 f = Sample(vec2( 2.0,  0.0));
    vec3 g = Sample(vec2(-2.0,  2.0));
    vec3 h = Sample(vec2( 0.0,  2.0));
    vec3 i = Sample(vec2( 2.0,  2.0));
    vec3 j = Sample(vec2(-1.0, -1.0));
    vec3 k = Sample(vec2( 1.0, -1.0));
    vec3 l = Sample(vec2(-1.0,  1.0));
    vec3 m = Sample(vec2( 1.0,  1.0));

    vec3 result;
    if (settings.texel_threshold_mode.w > 0.5) {
        float threshold = settings.texel_threshold_mode.z;
        vec3 centre = (j + k + l + m) * 0.25;
        vec3 top_left = (a + b + d + e) * 0.25;
        vec3 top_right = (b + c + e + f) * 0.25;
        vec3 bottom_left = (d + e + g + h) * 0.25;
        vec3 bottom_right = (e + f + h + i) * 0.25;
        centre = KeepBrightPart(centre, threshold);
        top_left = KeepBrightPart(top_left, threshold);
        top_right = KeepBrightPart(top_right, threshold);
        bottom_left = KeepBrightPart(bottom_left, threshold);
        bottom_right = KeepBrightPart(bottom_right, threshold);
        float centre_weight = 0.5 / (1.0 + Brightness(centre));
        float top_left_weight = 0.125 / (1.0 + Brightness(top_left));
        float top_right_weight = 0.125 / (1.0 + Brightness(top_right));
        float bottom_left_weight = 0.125 / (1.0 + Brightness(bottom_left));
        float bottom_right_weight = 0.125 / (1.0 + Brightness(bottom_right));
        result = (centre * centre_weight + top_left * top_left_weight + top_right * top_right_weight
            + bottom_left * bottom_left_weight + bottom_right * bottom_right_weight)
            / (centre_weight + top_left_weight + top_right_weight + bottom_left_weight + bottom_right_weight);
    } else {
        result = (j + k + l + m) * 0.125
            + (a + c + g + i) * 0.03125
            + (b + d + f + h) * 0.0625
            + e * 0.125;
    }
    output_color = vec4(result, 1.0);
}
