// The lit mesh pipeline's fragment shader. Runs once per pixel each mesh triangle covers (that passes the depth test).
//
// The lighting model is deliberately simple:
//
//   colour = albedo * (ambient * hemisphere + sun * diffuse), then faded into the fog colour with distance
//
// - albedo: the surface's own colour, the texture times the vertex colour times the draw's tint.
// - diffuse: Lambert's law, how squarely the surface faces the sun. A surface facing it gets all of its light, one
//   at a right angle or facing away gets none.
// - hemisphere: a cheap stand-in for sky light, so upward faces get a little more ambient than downward ones.
// - fog: linear between a start and end distance from the camera, fading to the clear colour so the far plane's cut
//   is hidden.
//
// All of this is in linear colour, where adding and multiplying light is physically meaningful. Textures are sRGB
// formats, so sampling decodes them to linear. Output is encoded back to sRGB either by the swapchain (SDR_LINEAR)
// or, when the target can't, by EncodeSrgb below.
//
// SDL GPU fixes where resources live in SPIR-V for a fragment shader: texture-sampler pairs are set 2, uniform blocks
// are set 3, binding n for push slot n.
#version 450

// The draw's texture, or the 1x1 white texture when it has none, with the sampler matching its filter and wrap.
layout(set = 2, binding = 0) uniform sampler2D albedo_texture;

// Slot 0, pushed once per frame: LightUniforms in GpuUniforms.nox (96 bytes).
layout(set = 3, binding = 0) uniform LightUniforms {
    // Unit vector from the surface towards the sun.
    vec4 direction_to_light;
    vec4 light_color;
    vec4 ambient_color;
    vec4 fog_color;
    // x: fog start distance, y: fog end distance, w: 1 when this shader must sRGB-encode its output.
    vec4 fog_and_output;
    vec4 camera_position;
} light;

// Slot 1, pushed once per draw: the DrawItem's tint (VectorUniforms in GpuUniforms.nox).
layout(set = 3, binding = 1) uniform MaterialUniforms {
    vec4 color;
} material;

// Interpolated from lit.vert's outputs.
layout(location = 0) in vec3 world_position;
layout(location = 1) in vec3 world_normal;
layout(location = 2) in vec2 surface_uv;
layout(location = 3) in vec4 surface_color;

// The colour written to the colour target.
layout(location = 0) out vec4 output_color;

// The exact sRGB transfer function: linear light to the encoded values screens expect. A straight line near black,
// a 1/2.4 power curve above.
vec3 EncodeSrgb(vec3 linear_color) {
    vec3 low = linear_color * 12.92;
    vec3 high = 1.055 * pow(linear_color, vec3(1.0 / 2.4)) - 0.055;
    return mix(high, low, lessThanEqual(linear_color, vec3(0.0031308)));
}

void main() {
    vec4 albedo = texture(albedo_texture, surface_uv) * surface_color * material.color;
    // Interpolating unit normals across a triangle shortens them, so normalise again per pixel.
    vec3 normal = normalize(world_normal);
    float diffuse = max(dot(normal, light.direction_to_light.xyz), 0.0);
    float hemisphere = 0.65 + 0.35 * normal.y;
    vec3 lit = albedo.rgb * (light.ambient_color.rgb * hemisphere + light.light_color.rgb * diffuse);

    float distance_to_camera = distance(world_position, light.camera_position.xyz);
    float fog_range = max(light.fog_and_output.y - light.fog_and_output.x, 0.001);
    float fog = clamp((distance_to_camera - light.fog_and_output.x) / fog_range, 0.0, 1.0);
    lit = mix(lit, light.fog_color.rgb, fog);

    if (light.fog_and_output.w > 0.5) {
        lit = EncodeSrgb(lit);
    }
    // Meshes are opaque: the lit pipeline doesn't blend, so alpha is written as 1.
    output_color = vec4(lit, 1.0);
}
