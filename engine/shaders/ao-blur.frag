// Cleans up the ambient-occlusion picture: a 4x4 average that ignores neighbours at a different depth.
//
// `ao.frag` turns its sample pattern by a different amount in each pixel of a 4x4 block, so every pixel is a little
// wrong, but the 16 together are right. Averaging the block gives a smooth result. A plain average would also smear the
// darkness of a nearby object onto the far wall behind it, so a neighbour counts only as much as its depth matches the
// centre's: a neighbour 10% farther or nearer counts for nothing.
#version 450
#extension GL_GOOGLE_include_directive : require

layout(set = 2, binding = 0) uniform sampler2D occlusion_image;
layout(set = 2, binding = 1) uniform sampler2D depth_image;

#include "ao-common.glsl"

layout(location = 0) in vec2 source_uv;
layout(location = 0) out vec4 output_occlusion;

void main() {
    float center_depth = texture(depth_image, source_uv).r;
    if (IsBackground(center_depth)) {
        output_occlusion = vec4(1.0);
        return;
    }
    float center_distance = LinearDistance(center_depth);
    float total = 0.0;
    float total_weight = 0.0;
    for (int row = -2; row < 2; row++) {
        for (int column = -2; column < 2; column++) {
            vec2 tap_uv = source_uv + (vec2(float(column), float(row)) + 0.5) * settings.texel.xy;
            float tap_depth = texture(depth_image, tap_uv).r;
            if (IsBackground(tap_depth)) {
                continue;
            }
            float difference = abs(LinearDistance(tap_depth) - center_distance) / center_distance;
            float weight = max(0.0, 1.0 - difference * 10.0);
            total += texture(occlusion_image, tap_uv).r * weight;
            total_weight += weight;
        }
    }
    output_occlusion = vec4(total_weight > 0.0 ? total / total_weight : 1.0);
}
