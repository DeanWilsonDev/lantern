#version 450
#extension GL_GOOGLE_include_directive : require
#include "mask-common.glsl"

MaterialSurface SurfaceOf(vec2 uv) {
    MaterialSurface surface = DefaultSurface(uv);
    surface.opacity_mask = 0.0;
    return surface;
}

#include "mask-main.glsl"
