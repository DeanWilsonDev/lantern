// Shared by the two ambient-occlusion shaders: their uniform block. The depth helpers live in depth-common.glsl.

// Slot 0 of both shaders: x: tan(half the horizontal field of view), y: tan(half the vertical), z: near plane, w: D above.
layout(set = 3, binding = 0) uniform AmbientOcclusionUniforms {
    vec4 projection_terms;
    // x: the sampling radius in metres, y: how dark a fully occluded pixel gets (0 to 1), z: bias in metres, w: unused.
    vec4 look;
    // xy: the size of one pixel in uv units.
    vec4 texel;
} settings;

#include "depth-common.glsl"
