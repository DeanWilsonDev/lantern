#version 450

layout(set = 1, binding = 0) uniform FrameUniforms {
    mat4 view_projection;
    vec4 camera_position;
} frame;

layout(set = 1, binding = 1) uniform ObjectUniforms {
    mat4 model;
    mat4 normal_matrix;
} object_uniforms;

layout(location = 0) in vec3 input_position;
layout(location = 1) in vec3 input_normal;
layout(location = 2) in vec2 input_uv;
layout(location = 3) in vec4 input_color;

layout(location = 0) out vec3 world_position;
layout(location = 1) out vec3 world_normal;
layout(location = 2) out vec2 surface_uv;
layout(location = 3) out vec4 surface_color;

void main() {
    vec4 world = object_uniforms.model * vec4(input_position, 1.0);
    gl_Position = frame.view_projection * world;
    world_position = world.xyz;
    world_normal = (object_uniforms.normal_matrix * vec4(input_normal, 0.0)).xyz;
    surface_uv = input_uv;
    surface_color = input_color;
}
