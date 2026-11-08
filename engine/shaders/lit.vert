// The lit mesh pipeline's vertex shader. Runs once per vertex of every mesh draw.
//
// Its job is to place the vertex on screen and hand the fragment shader what it needs for lighting. Positions go
// through two matrices: the model matrix (object space to world space, different for every draw) and the
// view-projection matrix (world space to clip space, the same for the whole frame). Lighting and fog are computed in
// world space, so the world position and normal are passed on too. The GPU interpolates every `out` value across the
// triangle before the fragment shader reads it.
//
// SDL GPU fixes where resources live in SPIR-V: a vertex shader's uniform blocks are set 1, binding n for push slot
// n. The Nox side (GpuUniforms.nox) pushes slot 0 once per frame and slot 1 once per draw.
#version 450

// Slot 0, pushed once per frame: MatrixAndVectorUniforms in GpuUniforms.nox (80 bytes).
layout(set = 1, binding = 0) uniform FrameUniforms {
    mat4 view_projection;
    vec4 camera_position;
} frame;

// Slot 1, pushed once per draw: ObjectUniforms in GpuUniforms.nox (128 bytes).
layout(set = 1, binding = 1) uniform ObjectUniforms {
    mat4 model;
    // The inverse transpose of the model matrix, so normals stay perpendicular to their surface under
    // non-uniform scale.
    mat4 normal_matrix;
} object_uniforms;

// The vertex, as Mesh.nox's Vertex lays it out (48 bytes): locations match LitPipelineDescription's attributes.
layout(location = 0) in vec3 input_position;
layout(location = 1) in vec3 input_normal;
layout(location = 2) in vec2 input_uv;
layout(location = 3) in vec4 input_color;

// Interpolated across the triangle for lit.frag.
layout(location = 0) out vec3 world_position;
layout(location = 1) out vec3 world_normal;
layout(location = 2) out vec2 surface_uv;
layout(location = 3) out vec4 surface_color;

void main() {
    // w = 1 makes the model matrix's translation apply: this is a point.
    vec4 world = object_uniforms.model * vec4(input_position, 1.0);
    // Clip space: the GPU clips to -w..w, divides by w (perspective) and maps the result to the viewport.
    gl_Position = frame.view_projection * world;
    world_position = world.xyz;
    // w = 0: a direction, so translation must not apply. Not normalised here; interpolation would undo it, so
    // lit.frag normalises per pixel.
    world_normal = (object_uniforms.normal_matrix * vec4(input_normal, 0.0)).xyz;
    surface_uv = input_uv;
    surface_color = input_color;
}
