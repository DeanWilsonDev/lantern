// The shadow pass's vertex shader for a masked material: shadow.vert, plus the world position, normal and texture
// coordinate its mask shader (mask-common.glsl) reads.
#version 450

layout(set = 1, binding = 0) uniform TileUniforms {
    mat4 light_view_projection;
} tile;

layout(set = 1, binding = 1) uniform ObjectUniforms {
    mat4 model;
} object_uniforms;

layout(location = 0) in vec3 input_position;
layout(location = 1) in vec3 input_normal;
layout(location = 2) in vec2 input_uv;
layout(location = 3) in vec4 input_color;

layout(location = 0) out vec3 world_position;
layout(location = 1) out vec3 world_normal;
layout(location = 2) out vec2 surface_uv;

void main() {
    vec4 world = object_uniforms.model * vec4(input_position, 1.0);
    world_position = world.xyz;
    world_normal = mat3(object_uniforms.model) * input_normal;
    surface_uv = input_uv;
    gl_Position = tile.light_view_projection * world;
}
