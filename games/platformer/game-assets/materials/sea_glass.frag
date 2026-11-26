// Generated from the material graph 'sea_glass' by lantern:::materialgraph. Edit the graph, not this file.
#version 450
#extension GL_GOOGLE_include_directive : require
#include "sky-common.glsl"
#include "surface-common.glsl"
#include "lit-common.glsl"


// parameter 0: roughness
// parameter 1: thickness

MaterialSurface SurfaceOf(vec2 uv) {
    vec3 node_tint = vec3(0.49803922, 0.83137256, 0.78431374);
    float node_smooth = material.parameters[0].x;
    vec3 node_sheenColor = vec3(0.10980392, 0.2901961, 0.27450982);
    float node_rim = Fresnel(world_position, world_normal, 2.5);
    vec3 node_sheen = node_sheenColor * node_rim;
    float node_edge = node_rim * 0.65;
    float node_thickness = material.parameters[0].y;
    float node_seeThrough = node_edge + node_thickness;
    MaterialSurface surface = DefaultSurface(uv);
    surface.base_color = node_tint;
    surface.metallic = 0.0;
    surface.roughness = node_smooth;
    surface.emissive = node_sheen;
    surface.tangent_normal = vec3(0.0, 0.0, 1.0);
    surface.opacity_mask = 1.0;
    surface.opacity = node_seeThrough;
    surface.normal_strength = 0.0;
    return surface;
}

#include "lit-main.glsl"
// Compiled against the engine's lit shader 17199922695573393515.
