// Generated from the material graph 'vent_grille' by lantern:::materialgraph. Edit the graph, not this file.
#version 450
#extension GL_GOOGLE_include_directive : require
#include "mask-common.glsl"


// parameter 0: metallic
// parameter 1: roughness

MaterialSurface SurfaceOf(vec2 uv) {
    vec3 node_steel = vec3(0.54901963, 0.5764706, 0.6117647);
    float node_metal = material.parameters[0].x;
    float node_rough = material.parameters[0].y;
    float node_height = world_position.y;
    float node_slats = node_height * 40.0;
    float node_wave = sin(node_slats);
    float node_mask = node_wave + 0.6;
    MaterialSurface surface = DefaultSurface(uv);
    surface.base_color = node_steel;
    surface.metallic = node_metal;
    surface.roughness = node_rough;
    surface.emissive = vec3(0.0);
    surface.tangent_normal = vec3(0.0, 0.0, 1.0);
    surface.opacity_mask = node_mask;
    surface.opacity = 1.0;
    surface.normal_strength = 0.0;
    return surface;
}

#include "mask-main.glsl"
// Compiled against the engine's lit shader 17199922695573393515.
