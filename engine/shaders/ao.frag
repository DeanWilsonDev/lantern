// Ambient occlusion: darken the creases where surfaces meet and the ground under things that sit on it.
//
// Ambient light comes from the whole sky, but a corner can see less of the sky than an open field can: the walls block
// some of it. This pass estimates, for every pixel, how much of the hemisphere above the surface is blocked by nearby
// geometry, using only the depth buffer (so it is "screen-space"):
//
//   1. Rebuild the pixel's position and its surface normal from the depth buffer (the normal from the position of the
//      neighbours, taking the nearer neighbour on each axis so the normal doesn't bend over a silhouette).
//   2. Scatter a dozen sample points inside the hemisphere above the surface, within a radius, in a pattern turned by an
//      amount that repeats every 4x4 pixels.
//   3. Project each sample point back to the screen and read the depth stored there. If the depth buffer says something is
//      in front of the sample point, the point is inside geometry: it counts as blocked. Blocking only counts when the
//      blocker is close in depth (a range check), so a far wall behind a nearby pole doesn't darken the pole.
//   4. The fraction blocked, scaled by the intensity, is the pixel's darkness. Samples close to the surface are rarely
//      blocked even in a corner, so the fraction is multiplied by 1.8 first to make a right-angle crease about half dark.
//
// The 4x4 turning pattern trades banding for a regular noise that `ao-blur.frag` removes with a depth-aware 4x4 average.
#version 450
#extension GL_GOOGLE_include_directive : require

layout(set = 2, binding = 0) uniform sampler2D depth_image;

#include "ao-common.glsl"

layout(location = 0) in vec2 source_uv;
layout(location = 0) out vec4 output_occlusion;

const int SAMPLE_COUNT = 12;

// The 4x4 Bayer matrix, 0 to 15: a turning amount that is spread evenly over any 4x4 block of pixels.
float TurnAmount(ivec2 pixel) {
    const float bayer[16] = float[16](0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0);
    return bayer[(pixel.y & 3) * 4 + (pixel.x & 3)] / 16.0;
}

void main() {
    float center_depth = texture(depth_image, source_uv).r;
    if (IsBackground(center_depth)) {
        output_occlusion = vec4(1.0);
        return;
    }
    vec2 texel = settings.texel.xy;
    vec3 center = ViewPosition(source_uv, center_depth);

    vec2 right_uv = source_uv + vec2(texel.x, 0.0);
    vec2 left_uv = source_uv - vec2(texel.x, 0.0);
    vec2 down_uv = source_uv + vec2(0.0, texel.y);
    vec2 up_uv = source_uv - vec2(0.0, texel.y);
    vec3 right = ViewPosition(right_uv, texture(depth_image, right_uv).r);
    vec3 left = ViewPosition(left_uv, texture(depth_image, left_uv).r);
    vec3 down = ViewPosition(down_uv, texture(depth_image, down_uv).r);
    vec3 up = ViewPosition(up_uv, texture(depth_image, up_uv).r);
    vec3 across = abs(right.z - center.z) < abs(left.z - center.z) ? right - center : center - left;
    vec3 along = abs(down.z - center.z) < abs(up.z - center.z) ? center - down : up - center;
    vec3 normal = normalize(cross(across, along));
    if (dot(normal, center) > 0.0) {
        normal = -normal;
    }

    float radius = settings.look.x;
    float bias = settings.look.z;
    float turn = TurnAmount(ivec2(gl_FragCoord.xy)) * 6.2831853;
    vec3 turned = vec3(cos(turn), sin(turn), 0.0);
    vec3 tangent = normalize(turned - normal * dot(turned, normal));
    vec3 bitangent = cross(normal, tangent);

    float blocked = 0.0;
    for (int sample_index = 0; sample_index < SAMPLE_COUNT; sample_index++) {
        float fraction = (float(sample_index) + 0.5) / float(SAMPLE_COUNT);
        float ring = sqrt(fraction);
        float angle = float(sample_index) * 2.3999632;
        vec3 direction = vec3(ring * cos(angle), ring * sin(angle), sqrt(1.0 - fraction));
        float reach = mix(0.1, 1.0, fraction * fraction) * radius;
        vec3 sample_position = center + (tangent * direction.x + bitangent * direction.y + normal * direction.z) * reach;
        vec2 sample_uv = ProjectToUv(sample_position);
        if (sample_uv.x < 0.0 || sample_uv.x > 1.0 || sample_uv.y < 0.0 || sample_uv.y > 1.0) {
            continue;
        }
        float stored_depth = texture(depth_image, sample_uv).r;
        float stored_distance = IsBackground(stored_depth) ? 1.0e6 : LinearDistance(stored_depth);
        float sample_distance = -sample_position.z;
        float range_check = smoothstep(0.0, 1.0, radius / max(abs(-center.z - stored_distance), 0.0001));
        if (stored_distance < sample_distance - bias) {
            blocked += range_check;
        }
    }
    float occlusion = clamp(blocked / float(SAMPLE_COUNT) * 1.8, 0.0, 1.0);
    output_occlusion = vec4(1.0 - occlusion * settings.look.y);
}
