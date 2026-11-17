// The sky's fragment shader: the colour of the sky in the direction each pixel looks.
//
// From the camera's three axes it builds the direction through the pixel: forward plus a step right and a step up that
// scale with the pixel's place on screen (the right and up vectors arrive already scaled by the field of view and aspect
// ratio). It then asks `SkyGradient` for the colour and adds the sun: a disc where the direction is within the sun's angular
// radius of the direction to the light, and a soft glow around it. The sun's strength is far above 1 (this is a high dynamic
// range target), so the bloom pass makes it glow. See bloom-and-tone-mapping.md.
//
// SDL GPU fixes where resources live in SPIR-V for a fragment shader: uniform blocks are set 3, binding n for push slot n.
#version 450
#extension GL_GOOGLE_include_directive : require
#include "sky-common.glsl"
#include "surface-common.glsl"

// Slot 0, pushed once per frame: SkyUniforms in GpuSky.nox (128 bytes).
layout(set = 3, binding = 0) uniform SkyUniforms {
    // xyz: the camera's right vector times tan(half the vertical field of view) times the aspect ratio.
    vec4 right_scaled;
    // xyz: the camera's up vector times tan(half the vertical field of view).
    vec4 up_scaled;
    vec4 forward;
    // xyz: unit vector towards the sun, w: cosine of the sun disc's angular radius.
    vec4 to_sun_and_disc_cosine;
    vec4 zenith;
    vec4 horizon;
    vec4 ground;
    // rgb: the sun's colour times its strength.
    vec4 sun_color;
} sky;

layout(location = 0) in vec2 screen_position;
layout(location = 0) out vec4 output_color;
// The sky is nothing to reflect in, so the surface buffer is left as 'not reflective'.
layout(location = 1) out vec4 output_surface;

void main() {
    vec3 direction = normalize(sky.forward.xyz + sky.right_scaled.xyz * screen_position.x + sky.up_scaled.xyz * screen_position.y);
    // Below the horizon the camera sees mostly the colour of distant haze, not the dark ground the surfaces are lit by, so the
    // ground colour is only a quarter of the way there. A level floating in the sky then has sky below it, not a grey slab.
    vec3 seen_ground = mix(sky.horizon.rgb, sky.ground.rgb, 0.25);
    vec3 color = SkyGradient(direction, sky.zenith.rgb, sky.horizon.rgb, seen_ground, 0.3);
    float cosine_to_sun = dot(direction, sky.to_sun_and_disc_cosine.xyz);
    float disc_cosine = sky.to_sun_and_disc_cosine.w;
    float disc = smoothstep(disc_cosine - 0.0002, disc_cosine + 0.0002, cosine_to_sun);
    float glow = pow(max(cosine_to_sun, 0.0), 128.0) * 0.15;
    color += sky.sun_color.rgb * (disc + glow);
    output_color = vec4(color, 1.0);
    output_surface = SURFACE_NONE;
}
