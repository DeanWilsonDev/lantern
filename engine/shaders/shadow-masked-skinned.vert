// shadow-masked.vert for skinned meshes: the joint palette blends the vertex first, as shadow-skinned.vert does.
#version 450

layout(set = 1, binding = 0) uniform TileUniforms {
    mat4 light_view_projection;
} tile;

layout(set = 1, binding = 1) uniform ObjectUniforms {
    mat4 model;
} object_uniforms;

layout(set = 1, binding = 2) uniform JointUniforms {
    mat4 joints[32];
} palette;

layout(location = 0) in vec3 input_position;
layout(location = 1) in vec3 input_normal;
layout(location = 2) in vec2 input_uv;
layout(location = 3) in vec4 input_color;
layout(location = 4) in uvec4 input_joints;
layout(location = 5) in vec4 input_weights;

layout(location = 0) out vec3 world_position;
layout(location = 1) out vec3 world_normal;
layout(location = 2) out vec2 surface_uv;

void main() {
    mat4 skin = palette.joints[input_joints.x] * input_weights.x
              + palette.joints[input_joints.y] * input_weights.y
              + palette.joints[input_joints.z] * input_weights.z
              + palette.joints[input_joints.w] * input_weights.w;
    vec4 world = object_uniforms.model * (skin * vec4(input_position, 1.0));
    world_position = world.xyz;
    world_normal = mat3(object_uniforms.model) * mat3(skin) * input_normal;
    surface_uv = input_uv;
    gl_Position = tile.light_view_projection * world;
}
