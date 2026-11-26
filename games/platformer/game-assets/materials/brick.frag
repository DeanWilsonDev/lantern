// Generated from the material graph 'brick' by lantern:::materialgraph. Edit the graph, not this file.
#version 450
#extension GL_GOOGLE_include_directive : require
#include "sky-common.glsl"
#include "surface-common.glsl"
#include "lit-common.glsl"

// brick
layout(set = 2, binding = 4) uniform sampler2D material_texture_0;
// brick_normal
layout(set = 2, binding = 5) uniform sampler2D material_texture_1;

// parameter 0: roughness
// parameter 1: bumpiness

MaterialSurface SurfaceOf(vec2 uv) {
    vec2 node_coordinate = uv;
    vec2 node_tiled = node_coordinate * 3.0;
    vec3 node_bricks = texture(material_texture_0, node_tiled).rgb;
    float node_no_metal = 0.0;
    float node_rough = material.parameters[0].x;
    float node_bumpiness = material.parameters[0].y;
    vec3 node_bumps = ScaleTangentNormal(LinearToSrgbExact(texture(material_texture_1, node_tiled).rgb) * 2.0 - 1.0, node_bumpiness);
    MaterialSurface surface = DefaultSurface(uv);
    surface.base_color = node_bricks;
    surface.metallic = node_no_metal;
    surface.roughness = node_rough;
    surface.emissive = vec3(0.0);
    surface.tangent_normal = node_bumps;
    surface.opacity_mask = 1.0;
    surface.opacity = 1.0;
    surface.normal_strength = 1.0;
    surface.normal_uv = node_tiled;
    return surface;
}

#include "lit-main.glsl"
// Compiled against the engine's lit shader 17199922695573393515.
