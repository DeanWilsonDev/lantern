// The main of a masked material's shadow shader: no colour, only a cut where the material's mask is below one half.
void main() {
    if (SurfaceOf(surface_uv).opacity_mask < 0.5) {
        discard;
    }
}
