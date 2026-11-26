// Generated from the material graph 'gold' by lantern:::materialgraph. Edit the graph, not this file.
#version 450
#extension GL_GOOGLE_include_directive : require
#include "sky-common.glsl"
#include "surface-common.glsl"
#include "lit-common.glsl"


// parameter 0: roughness

MaterialSurface SurfaceOf(vec2 uv) {
    vec3 node_gold_colour = vec3(1.0, 0.7647059, 0.3529412);
    float node_full_metal = 1.0;
    float node_polish = material.parameters[0].x;
    MaterialSurface surface = DefaultSurface(uv);
    surface.base_color = node_gold_colour;
    surface.metallic = node_full_metal;
    surface.roughness = node_polish;
    surface.emissive = vec3(0.0);
    surface.tangent_normal = vec3(0.0, 0.0, 1.0);
    surface.opacity_mask = 1.0;
    surface.normal_strength = 0.0;
    return surface;
}

#include "lit-main.glsl"
// Compiled against the engine's lit shader 13426740955923470436.
