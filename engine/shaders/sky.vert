// The sky's vertex shader: one triangle that covers the screen, drawn at the far plane.
//
// Like fullscreen.vert, but it writes depth 1 (the farthest value) instead of 0. The sky pipeline tests depth with "less or
// equal" and the depth buffer starts at 1, so the sky shows only where no mesh was drawn, and meshes drawn before it are
// never overdrawn. The output is the screen position in -1..1, which the fragment shader turns into a view direction.
#version 450

layout(location = 0) out vec2 screen_position;

void main() {
    vec2 corner = vec2(float((gl_VertexIndex << 1) & 2), float(gl_VertexIndex & 2));
    screen_position = corner * 2.0 - 1.0;
    gl_Position = vec4(screen_position, 1.0, 1.0);
}
