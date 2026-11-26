#version 450
#extension GL_GOOGLE_include_directive : require
#include "sky-common.glsl"
#include "surface-common.glsl"
#include "lit-common.glsl"

MaterialSurface SurfaceOf(vec2 uv) {
    MaterialSurface surface = DefaultSurface(uv);
    surface.base_color = vec3(0.0);
    surface.emissive = vec3(0.0, 1.0, 0.0);
    surface.opacity = material.parameters[0].x;
    return surface;
}

#include "lit-main.glsl"
