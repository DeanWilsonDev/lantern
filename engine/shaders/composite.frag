// The last step before the screen: turn the scene's high dynamic range image into the 0 to 1 range a display shows.
//
// The lit pass writes light, not colour: a white wall in full sun is 1.0 and a lamp or a gem can be 5 or 20. The target
// it writes into is a 16-bit float texture, so those values survive instead of being cut off at 1. This pass reads that
// texture and does three things to each pixel, in this order:
//
//   0. multiply by the ambient-occlusion picture, when there is one (1 where nothing is blocked, darker in creases);
//   1. add the bloom picture (the blurred bright parts) scaled by a strength;
//   2. multiply by an exposure, the camera's brightness knob: 2 makes the whole picture twice as bright before step 3;
//   3. tone map: squeeze 0..infinity into 0..1 with a curve that keeps dark and middle values nearly as they were and
//      rolls bright ones off smoothly towards white, instead of clipping them flat.
//
// The occlusion multiplies the lit scene as a whole, before bloom is added, so a glowing gem's halo is not darkened by it.
// (A renderer that lit in several passes would apply it to the ambient light alone; this one lights in a single pass, so
// it takes the simpler route and darkens direct light in creases a little too.)
//
// Three curves are available, picked by a number in the uniforms:
//   0  none: clamp to 1, the look of a renderer without high dynamic range;
//   1  Khronos PBR Neutral: leaves everything below about 0.8 exactly as it was and only rolls off the brighter light, so a
//      scene lit for a clamped target keeps its colours. The default;
//   2  an ACES fit by Krzysztof Narkowicz: f(x) = (x (2.51 x + 0.03)) / (x (2.43 x + 0.59) + 0.14). An "S": a lifted toe, a
//      contrasty middle and a long shoulder. More cinematic, but it brightens and desaturates pastel colours.
//
// The output is encoded to sRGB here when the target can't do it itself (see GpuDevice.nox), so the lit pass can stay in
// linear light.
#version 450

layout(set = 2, binding = 0) uniform sampler2D scene_image;
layout(set = 2, binding = 1) uniform sampler2D bloom_image;
layout(set = 2, binding = 2) uniform sampler2D occlusion_image;

// Slot 0: x: exposure, y: bloom strength (0 when bloom is off), z: the tone mapper (0, 1 or 2 above), w: 1 to sRGB-encode.
layout(set = 3, binding = 0) uniform CompositeUniforms {
    vec4 exposure_bloom_tonemap_encode;
    // x: 1 when the ambient-occlusion picture is drawn this frame (it is the scene itself otherwise, and unread).
    vec4 occlusion_flag;
} settings;

layout(location = 0) in vec2 source_uv;
layout(location = 0) out vec4 output_color;

vec3 EncodeSrgb(vec3 linear_color) {
    vec3 low = linear_color * 12.92;
    vec3 high = 1.055 * pow(linear_color, vec3(1.0 / 2.4)) - 0.055;
    return mix(high, low, lessThanEqual(linear_color, vec3(0.0031308)));
}

vec3 AcesFilmic(vec3 x) {
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
}

// Khronos PBR Neutral. Below the compression start every colour passes through with only a small offset in the very darkest
// values; above it the brightest channel is rolled off towards 1 and the colour drifts towards white, as a real sensor's does.
vec3 PbrNeutral(vec3 color) {
    const float start_compression = 0.8 - 0.04;
    const float desaturation = 0.15;
    float lowest = min(color.r, min(color.g, color.b));
    float offset = lowest < 0.08 ? lowest - 6.25 * lowest * lowest : 0.04;
    color -= offset;
    float peak = max(color.r, max(color.g, color.b));
    if (peak < start_compression) {
        return color;
    }
    float room = 1.0 - start_compression;
    float new_peak = 1.0 - room * room / (peak + room - start_compression);
    color *= new_peak / peak;
    float blend_to_white = 1.0 - 1.0 / (desaturation * (peak - new_peak) + 1.0);
    return mix(color, new_peak * vec3(1.0), blend_to_white);
}

vec3 ToneMap(vec3 exposed, float mapper) {
    if (mapper > 1.5) {
        return AcesFilmic(exposed);
    }
    if (mapper > 0.5) {
        return clamp(PbrNeutral(exposed), 0.0, 1.0);
    }
    return clamp(exposed, 0.0, 1.0);
}

void main() {
    vec3 scene = texture(scene_image, source_uv).rgb;
    if (settings.occlusion_flag.x > 0.5) {
        scene *= texture(occlusion_image, source_uv).r;
    }
    float bloom_strength = settings.exposure_bloom_tonemap_encode.y;
    if (bloom_strength > 0.0) {
        scene += texture(bloom_image, source_uv).rgb * bloom_strength;
    }
    vec3 exposed = scene * settings.exposure_bloom_tonemap_encode.x;
    vec3 display = ToneMap(exposed, settings.exposure_bloom_tonemap_encode.z);
    if (settings.exposure_bloom_tonemap_encode.w > 0.5) {
        display = EncodeSrgb(display);
    }
    output_color = vec4(display, 1.0);
}
