// Generated from the material graph 'lava_flow' by lantern:::materialgraph. Edit the graph, not this file.
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

MaterialSurface SurfaceOf(vec2 uv) {
    vec3 node_crust = vec3(0.101960786, 0.05490196, 0.039215688);
    float node_rough = material.parameters[0].x;
    vec2 node_coordinate = uv;
    vec2 node_tiled = node_coordinate * 2.0;
    vec2 node_flow = node_tiled + vec2(0.04, 0.15) * MaterialTime();
    vec3 node_stone = texture(material_texture_0, node_flow).rgb;
    vec3 node_heat = vec3(1.0, 0.41568628, 0.101960786);
    vec3 node_molten = node_stone * node_heat;
    vec3 node_glow = node_molten * 2.5;
    vec3 node_bumps = ScaleTangentNormal(LinearToSrgbExact(texture(material_texture_1, node_flow).rgb) * 2.0 - 1.0, 1.0);
    MaterialSurface surface = DefaultSurface(uv);
    surface.base_color = node_crust;
    surface.metallic = 0.0;
    surface.roughness = node_rough;
    surface.emissive = node_glow;
    surface.tangent_normal = node_bumps;
    surface.opacity_mask = 1.0;
    surface.normal_strength = 1.0;
    surface.normal_uv = node_flow;
    return surface;
}

#include "lit-main.glsl"
// Compiled against the engine's lit shader 13426740955923470436.
