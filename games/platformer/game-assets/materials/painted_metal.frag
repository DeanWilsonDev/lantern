// Generated from the material graph 'painted_metal' by lantern:::materialgraph. Edit the graph, not this file.
#version 450
#extension GL_GOOGLE_include_directive : require
#include "sky-common.glsl"
#include "surface-common.glsl"
#include "lit-common.glsl"

// wall
layout(set = 2, binding = 4) uniform sampler2D material_texture_0;

// parameter 0: metallic
// parameter 1: wear

MaterialSurface SurfaceOf(vec2 uv) {
    vec3 node_paint = vec3(0.30980393, 0.47843137, 0.6039216);
    vec2 node_coordinate = uv;
    vec2 node_tiled = node_coordinate * 2.0;
    vec3 node_grain = texture(material_texture_0, node_tiled).rgb;
    vec3 node_painted = node_paint * node_grain;
    float node_metal = material.parameters[0].x;
    float node_wear = material.parameters[0].y;
    float node_rough = mix(0.35, 0.85, clamp(node_wear, 0.0, 1.0));
    MaterialSurface surface = DefaultSurface(uv);
    surface.base_color = node_painted;
    surface.metallic = node_metal;
    surface.roughness = node_rough;
    surface.emissive = vec3(0.0);
    surface.tangent_normal = vec3(0.0, 0.0, 1.0);
    surface.opacity_mask = 1.0;
    surface.opacity = 1.0;
    surface.normal_strength = 0.0;
    return surface;
}

#include "lit-main.glsl"
// Compiled against the engine's lit shader 17199922695573393515.
