// Generated from the material graph 'neon_pulse' by lantern:::materialgraph. Edit the graph, not this file.
#version 450
#extension GL_GOOGLE_include_directive : require
#include "sky-common.glsl"
#include "surface-common.glsl"
#include "lit-common.glsl"


// parameter 0: roughness

MaterialSurface SurfaceOf(vec2 uv) {
    vec3 node_glass = vec3(0.16470589, 0.039215688, 0.10980392);
    float node_rough = material.parameters[0].x;
    vec3 node_tube = vec3(1.0, 0.24705882, 0.6431373);
    float node_clock = MaterialTime() * 3.0;
    float node_wave = sin(node_clock);
    float node_swing = node_wave * 0.2;
    float node_level = node_swing + 0.25;
    vec3 node_glow = node_tube * node_level;
    float node_rim = Fresnel(world_position, world_normal, 3.0);
    float node_sheen = node_rim * 0.5;
    vec3 node_light = node_glow + node_sheen;
    MaterialSurface surface = DefaultSurface(uv);
    surface.base_color = node_glass;
    surface.metallic = 0.0;
    surface.roughness = node_rough;
    surface.emissive = node_light;
    surface.tangent_normal = vec3(0.0, 0.0, 1.0);
    surface.opacity_mask = 1.0;
    surface.opacity = 1.0;
    surface.normal_strength = 0.0;
    return surface;
}

#include "lit-main.glsl"
// Compiled against the engine's lit shader 17199922695573393515.
