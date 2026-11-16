// The vertex shader every post-processing pass shares: it draws one triangle that covers the whole target, with no vertex
// buffer at all.
//
// A full-screen effect (bloom, tone mapping) is a fragment shader that runs once for every pixel of the target. To make the
// GPU run it for every pixel, draw a shape that covers the screen. The usual shape is a quad of two triangles, but one
// oversized triangle does the same job without the diagonal seam: its corners sit at (-1, -1), (3, -1) and (-1, 3) in clip
// space, so the part inside the screen's square from -1 to 1 is covered entirely and the rest is clipped away.
//
// The three positions come from gl_VertexIndex (0, 1, 2), so the draw call is just "3 vertices" with nothing bound.
#version 450

// Where to read the source image: (0, 0) is its top-left corner and (1, 1) its bottom-right.
layout(location = 0) out vec2 source_uv;

void main() {
    // (0,0), (2,0), (0,2) for vertices 0, 1, 2.
    vec2 corner = vec2(float((gl_VertexIndex << 1) & 2), float(gl_VertexIndex & 2));
    gl_Position = vec4(corner * 2.0 - 1.0, 0.0, 1.0);
    // Clip space has +y up and image space has +y down, so the v coordinate flips.
    source_uv = vec2(corner.x, 1.0 - corner.y);
}
