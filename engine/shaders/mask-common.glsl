// The declarations a masked material's shadow shader needs: what material-surface.glsl and a generated SurfaceOf read, and
// nothing a shadow pass can't give. A material graph with an opacity mask gets a second shader, `<name>.mask.frag`: this,
// its SurfaceOf, then mask-main.glsl, which cuts the shadow where the mask does.

// Slot 0, pushed once per shadow tile: x, y, z the camera's position, w the material clock (MaskLightUniforms, 16 bytes).
layout(set = 3, binding = 0) uniform LightUniforms {
    vec4 camera_position;
} light;

// Slot 1, pushed once per draw: the material's parameters (MaskMaterialUniforms, 32 bytes).
layout(set = 3, binding = 1) uniform MaterialUniforms {
    vec4 parameters[2];
} material;

layout(location = 0) in vec3 world_position;
layout(location = 1) in vec3 world_normal;
layout(location = 2) in vec2 surface_uv;

#include "material-surface.glsl"
