// The 2D overlay pipeline's vertex shader: HUD, menus and text, positioned in screen pixels.
//
// No matrices: each vertex's pixel position is turned straight into normalized device coordinates, where the
// screen runs from -1 to 1 on both axes with +Y up. Pixels run from the top-left corner downwards, so Y flips.
#version 450

// Slot 0: VectorUniforms in GpuUniforms.nox. xy: the target's size in pixels, w: the sRGB flag.
layout(set = 1, binding = 0) uniform ScreenUniforms {
    vec4 size_and_flags;
} screen;

// The vertex, as RenderWorld.nox's OverlayVertex lays it out (32 bytes).
layout(location = 0) in vec2 input_position;
layout(location = 1) in vec2 input_uv;
layout(location = 2) in vec4 input_color;

layout(location = 0) out vec2 atlas_uv;
layout(location = 1) out vec4 glyph_color;
layout(location = 2) flat out float encode_srgb;

void main() {
    vec2 normalized_device_position = vec2(
        input_position.x / screen.size_and_flags.x * 2.0 - 1.0,
        1.0 - input_position.y / screen.size_and_flags.y * 2.0);
    // Depth 0 and w 1: no perspective, and the overlay pipeline doesn't depth test, so draw order alone decides
    // what's on top.
    gl_Position = vec4(normalized_device_position, 0.0, 1.0);
    atlas_uv = input_uv;
    glyph_color = input_color;
    encode_srgb = screen.size_and_flags.w;
}
