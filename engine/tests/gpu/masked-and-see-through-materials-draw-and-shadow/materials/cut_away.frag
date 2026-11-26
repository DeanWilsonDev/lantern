#version 450
#extension GL_GOOGLE_include_directive : require
#include "sky-common.glsl"
#include "surface-common.glsl"
#include "lit-common.glsl"

MaterialSurface SurfaceOf(vec2 uv) {
    MaterialSurface surface = DefaultSurface(uv);
    surface.opacity_mask = 0.0;
    return surface;
}

#include "lit-main.glsl"
