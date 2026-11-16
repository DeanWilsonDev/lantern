// The shadow pass's vertex shader. Runs once per vertex of every mesh that casts a shadow, once for each shadow tile the
// mesh lands in.
//
// A shadow map is a picture of the scene from the light's point of view, where each pixel holds not a colour but how far
// away the nearest surface is. This shader draws that picture's geometry: it only needs to place each vertex on screen,
// since the depth the GPU computes while rasterising is the whole output. So it is the lit vertex shader minus
// everything about lighting: the model matrix, then the light's view-projection matrix.
//
// SDL GPU fixes where resources live in SPIR-V: a vertex shader's uniform blocks are set 1, binding n for push slot n.
// GpuRenderer.RecordShadowTile pushes slot 0 once per tile and slot 1 once per draw.
#version 450

// Slot 0, pushed once per tile: the tile's light view-projection matrix.
layout(set = 1, binding = 0) uniform TileUniforms {
    mat4 light_view_projection;
} tile;

// Slot 1, pushed once per draw: the draw's model matrix.
layout(set = 1, binding = 1) uniform ObjectUniforms {
    mat4 model;
} object_uniforms;

// The mesh's vertex, as the lit pipeline reads it. Only the position is used; the pipeline declares the whole vertex so
// the same mesh buffers bind to both.
layout(location = 0) in vec3 input_position;
layout(location = 1) in vec3 input_normal;
layout(location = 2) in vec2 input_uv;
layout(location = 3) in vec4 input_color;

void main() {
    gl_Position = tile.light_view_projection * (object_uniforms.model * vec4(input_position, 1.0));
}
