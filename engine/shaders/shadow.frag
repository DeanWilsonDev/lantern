// The shadow pass's fragment shader: nothing to do. The pass draws no colour, and the depth the GPU computes while
// rasterising is what the shadow map keeps. It exists because every pipeline needs a fragment shader.
#version 450

void main() {
}
