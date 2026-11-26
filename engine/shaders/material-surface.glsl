// What a material graph's SurfaceOf can use, shared by the lit shaders (lit-common.glsl) and the mask shaders a masked
// material's shadow is drawn with (mask-common.glsl). The including file declares the `light` block (with camera_position)
// and the `material` block (with parameters), and the inputs world_position and world_normal, first.

// A texture holds its normals as plain bytes, but the albedo-style upload decodes them as sRGB. Encoding again returns the
// bytes the file held.
vec3 LinearToSrgbExact(vec3 linear) {
    vec3 low = linear * 12.92;
    vec3 high = 1.055 * pow(max(linear, vec3(0.0)), vec3(1.0 / 2.4)) - 0.055;
    return mix(high, low, lessThanEqual(linear, vec3(0.0031308)));
}

// A tangent-space normal with its tilt scaled by `strength`: 0 is flat, 1 is the map as drawn. A material graph's normal map
// applies its own strength with this, so the normal output can take any graph value.
vec3 ScaleTangentNormal(vec3 tangent_space, float strength) {
    return vec3(tangent_space.xy * strength, tangent_space.z);
}

// What a material says about one pixel of its surface, before lighting. lit.frag's SurfaceOf reads the draw's texture and
// uniform material; a material graph's generated shader (lantern:::materialgraph, docs/material-graphs.md) writes its own
// SurfaceOf from the graph's nodes. lit-main.glsl multiplies the base colour by the vertex colour and the draw's tint, so a
// tint still works on a graph material.
struct MaterialSurface {
    vec3 base_color;
    float metallic;
    float roughness;
    // Light the surface gives off by itself, added to the draw's own emissive.
    vec3 emissive;
    // A tangent-space normal (0, 0, 1 is flat) and how strongly it bends the surface; 0 leaves the normal alone.
    vec3 tangent_normal;
    float normal_strength;
    // The coordinate the normal map was sampled at, so the tangent frame follows a scaled, panned or flipped coordinate.
    vec2 normal_uv;
    // Below 0.5 the pixel is cut away (a masked material: leaves, grates, chain-link). 1 for every solid surface.
    float opacity_mask;
    // How much of the surface is there, 0 to 1: below 1 on a translucent material (glass, water), which blends over what is
    // behind it. An opaque material ignores it.
    float opacity;
};

// A surface with every field set: white, not metallic, roughness 0.8, no light, flat and solid. A generated SurfaceOf starts
// from this and overwrites what its graph wires.
MaterialSurface DefaultSurface(vec2 uv) {
    MaterialSurface surface;
    surface.base_color = vec3(1.0);
    surface.metallic = 0.0;
    surface.roughness = 0.8;
    surface.emissive = vec3(0.0);
    surface.tangent_normal = vec3(0.0, 0.0, 1.0);
    surface.normal_strength = 0.0;
    surface.normal_uv = uv;
    surface.opacity_mask = 1.0;
    surface.opacity = 1.0;
    return surface;
}

// Seconds since the game or editor started, wrapped every hour so it keeps its precision.
float MaterialTime() {
    return light.camera_position.w;
}

// How much a surface faces away from the camera: 0 looking straight at it, 1 at its silhouette, sharpened by `power`.
float Fresnel(vec3 position, vec3 surface_normal, float power) {
    vec3 to_camera = normalize(light.camera_position.xyz - position);
    float facing = clamp(dot(normalize(surface_normal), to_camera), 0.0, 1.0);
    return pow(1.0 - facing, max(power, 0.0001));
}
