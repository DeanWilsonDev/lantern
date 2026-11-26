// Generated from the material graph 'green_glow' by lantern:::materialgraph. Edit the graph, not this file.
#version 450
#extension GL_GOOGLE_include_directive : require
#include "sky-common.glsl"
#include "surface-common.glsl"
#include "lit-common.glsl"

// blue
layout(set = 2, binding = 4) uniform sampler2D material_texture_0;

MaterialSurface SurfaceOf(vec2 uv) {
    vec3 node_paint = texture(material_texture_0, uv).rgb;
    vec3 node_glow = vec3(0.0, 1.0, 0.0);
    MaterialSurface surface;
    surface.base_color = node_paint;
    surface.metallic = 0.0;
    surface.roughness = 0.9;
    surface.emissive = node_glow;
    surface.tangent_normal = vec3(0.0, 0.0, 1.0);
    surface.normal_strength = 0.0;
    return surface;
}

#include "lit-main.glsl"

