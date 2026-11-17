// The surface buffer: a second colour target the lit and sky passes write beside the scene, so the screen-space reflection
// pass knows, for every pixel, which way it faces and how mirror-like it is. Four 8-bit channels:
//
//   r, g   the surface's world-space normal, squeezed into two numbers by an octahedral map (below)
//   b      the roughness after wetness and puddles, 0 (mirror) to 1
//   a      the surface's reflectance at a head-on view (f0), averaged over the colour channels: 0.04 for a plain non-metal,
//          near 1 for a bright metal. 0 means "never reflects".
//
// The buffer is cleared to `SURFACE_NONE` (roughness 1, reflectance 0), which is also what the sky writes, so a pixel that
// no lit mesh covered is skipped by the reflection pass.
//
// An octahedral map folds the sphere of directions onto a square: take the unit vector, divide by |x| + |y| + |z| so it
// lies on an octahedron, keep x and y, and fold the lower half (z < 0) outwards over the edges. Two 8-bit channels then
// give about half a degree of accuracy anywhere on the sphere, enough for a mirror's normal.

const vec4 SURFACE_NONE = vec4(0.5, 0.5, 1.0, 0.0);

vec2 OctahedronWrap(vec2 folded) {
    return (1.0 - abs(folded.yx)) * vec2(folded.x >= 0.0 ? 1.0 : -1.0, folded.y >= 0.0 ? 1.0 : -1.0);
}

// A unit normal as two numbers from 0 to 1.
vec2 EncodeNormal(vec3 normal) {
    vec3 on_octahedron = normal / (abs(normal.x) + abs(normal.y) + abs(normal.z));
    vec2 folded = on_octahedron.z >= 0.0 ? on_octahedron.xy : OctahedronWrap(on_octahedron.xy);
    return folded * 0.5 + 0.5;
}

// The unit normal `EncodeNormal` stored.
vec3 DecodeNormal(vec2 encoded) {
    vec2 folded = encoded * 2.0 - 1.0;
    vec3 on_octahedron = vec3(folded.x, folded.y, 1.0 - abs(folded.x) - abs(folded.y));
    float below = clamp(-on_octahedron.z, 0.0, 1.0);
    on_octahedron.xy += vec2(on_octahedron.x >= 0.0 ? -below : below, on_octahedron.y >= 0.0 ? -below : below);
    return normalize(on_octahedron);
}

// What the lit shader writes for a pixel: its normal, roughness and reflectance.
vec4 PackSurface(vec3 normal, float roughness, vec3 f0) {
    return vec4(EncodeNormal(normal), roughness, dot(f0, vec3(0.3333333)));
}

// The split-sum approximation of a surface's response to light arriving from every direction (Karis, "Physically Based
// Shading on Mobile"): how much of a distant light the specular colour keeps, as a scale and a bias that depend on the
// roughness and the angle to the viewer. A fit to numbers that would otherwise need a lookup texture.
vec3 EnvironmentBrdf(vec3 specular_color, float roughness, float normal_dot_view) {
    const vec4 c0 = vec4(-1.0, -0.0275, -0.572, 0.022);
    const vec4 c1 = vec4(1.0, 0.0425, 1.04, -0.04);
    vec4 r = roughness * c0 + c1;
    float a004 = min(r.x * r.x, exp2(-9.28 * normal_dot_view)) * r.x + r.y;
    vec2 scale_and_bias = vec2(-1.04, 1.04) * a004 + r.zw;
    return specular_color * scale_and_bias.x + scale_and_bias.y;
}
