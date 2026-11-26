// The lit mesh pipeline's fragment shader. Runs once per pixel each mesh triangle covers (that passes the depth test).
//
// The lighting model is "PBR-lite": physically based rendering's two surface parameters, metallic and roughness, without
// the image based lighting and extra layers of a full renderer. For each pixel it adds up
//
//   colour = ambient + sun + every local light + emissive, then faded into the fog colour with distance
//
// The surface buffer (surface-common.glsl) gets this pixel's final normal (a puddle's is straight up), roughness and f0, so the
// screen-space reflection pass can trace a mirror ray from it.
//
// A wet material (material.surface.w) is darker and glossier, and on wet ground facing up some of the surface is a puddle
// (material.emissive.w): flat, dark and almost a mirror. See WetnessMask and the wetness block in main.
//
// where the ambient comes from the sky (see below), or from the flat sky and ground colours when the sky's lighting is off
//
// - albedo: the surface's own colour, the texture times the vertex colour times the draw's tint.
// - ambient: sky light from above blended with ground light from below by which way the surface faces, standing in for
//   all the light that bounces around the scene. With the procedural sky on, the ambient is image-based instead: the
//   sky's colour averaged around the surface's normal lights its diffuse colour, and the sky seen in the mirror direction
//   (blurred by roughness) is its reflection, scaled by an approximation of how the surface responds (EnvironmentBrdf).
//   The sky is the function in sky-common.glsl, the same one sky.frag draws.
// - the sun and each local light go through one function, EvaluateLight, with a direction to the light and the light's
//   colour at this pixel. A point or spot light first loses brightness with distance (its attenuation) and a spot light
//   loses more outside its cone.
// - EvaluateLight is the Cook-Torrance model: a diffuse part (the albedo, for non-metals) plus a specular highlight from
//   a GGX microfacet distribution (how many tiny mirrors face the half vector), a Smith visibility term (how many of
//   them are blocked by their neighbours) and Schlick's Fresnel (surfaces reflect more at grazing angles).
// - shadows: before the sun and each local light add their colour, a lookup in a shadow map says how much of that light
//   reaches this pixel (1 fully lit, 0 fully in shadow). The renderer drew the maps from the lights' points of view in an
//   earlier pass; see SunShadow and LocalShadow below.
// - emissive: light the surface gives off itself, added after the lighting.
// - fog: linear between a start and end distance from the camera, fading to the clear colour so the far plane's cut
//   is hidden.
//
// Light strengths are defined so that a light of strength 1 facing a white matte surface gives colour 1. Physically
// based formulas divide diffuse light by pi and the specular lobe is normalised against it; EvaluateLight multiplies
// everything by pi to cancel that, so the numbers an artist writes mean what they look like.
//
// All of this is in linear colour, where adding and multiplying light is physically meaningful. Textures are sRGB
// formats, so sampling decodes them to linear. The output stays linear and unclamped, in a high dynamic range target;
// composite.frag tone maps it and encodes it to sRGB later (see GpuPost.nox).
//
// SDL GPU fixes where resources live in SPIR-V for a fragment shader: texture-sampler pairs are set 2, uniform blocks
// are set 3, binding n for push slot n.
// The declarations and lighting live in lit-common.glsl and the main in lit-main.glsl, so a material graph's generated shader
// can put its own SurfaceOf between them (docs/material-graphs.md).
#version 450
#extension GL_GOOGLE_include_directive : require
#include "sky-common.glsl"
#include "surface-common.glsl"
#include "lit-common.glsl"

// The surface of a draw with no material graph: the draw's texture, and its uniform metallic, roughness and normal map.
MaterialSurface SurfaceOf(vec2 uv) {
    MaterialSurface surface;
    surface.base_color = texture(albedo_texture, uv).rgb;
    surface.metallic = material.surface.x;
    surface.roughness = material.surface.y;
    surface.emissive = vec3(0.0);
    surface.normal_strength = material.surface.z;
    surface.tangent_normal = vec3(0.0, 0.0, 1.0);
    if (surface.normal_strength > 0.0) {
        surface.tangent_normal = LinearToSrgbExact(texture(normal_texture, uv).rgb) * 2.0 - 1.0;
    }
    return surface;
}

#include "lit-main.glsl"
