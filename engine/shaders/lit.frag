// The lit mesh pipeline's fragment shader. Runs once per pixel each mesh triangle covers (that passes the depth test).
//
// The lighting model is "PBR-lite": physically based rendering's two surface parameters, metallic and roughness, without
// the image based lighting and extra layers of a full renderer. For each pixel it adds up
//
//   colour = ambient + sun + every local light + emissive, then faded into the fog colour with distance
//
// - albedo: the surface's own colour, the texture times the vertex colour times the draw's tint.
// - ambient: sky light from above blended with ground light from below by which way the surface faces, standing in for
//   all the light that bounces around the scene.
// - the sun and each local light go through one function, EvaluateLight, with a direction to the light and the light's
//   colour at this pixel. A point or spot light first loses brightness with distance (its attenuation) and a spot light
//   loses more outside its cone.
// - EvaluateLight is the Cook-Torrance model: a diffuse part (the albedo, for non-metals) plus a specular highlight from
//   a GGX microfacet distribution (how many tiny mirrors face the half vector), a Smith visibility term (how many of
//   them are blocked by their neighbours) and Schlick's Fresnel (surfaces reflect more at grazing angles).
// - emissive: light the surface gives off itself, added after the lighting.
// - fog: linear between a start and end distance from the camera, fading to the clear colour so the far plane's cut
//   is hidden.
//
// Light strengths are defined so that a light of strength 1 facing a white matte surface gives colour 1. Physically
// based formulas divide diffuse light by pi and the specular lobe is normalised against it; EvaluateLight multiplies
// everything by pi to cancel that, so the numbers an artist writes mean what they look like.
//
// All of this is in linear colour, where adding and multiplying light is physically meaningful. Textures are sRGB
// formats, so sampling decodes them to linear. Output is encoded back to sRGB either by the swapchain (SDR_LINEAR)
// or, when the target can't, by EncodeSrgb below.
//
// SDL GPU fixes where resources live in SPIR-V for a fragment shader: texture-sampler pairs are set 2, uniform blocks
// are set 3, binding n for push slot n.
#version 450

const float PI = 3.14159265359;
// Must match MAX_LOCAL_LIGHTS in RenderWorld.nox.
const int MAX_LOCAL_LIGHTS = 8;

// The draw's texture, or the 1x1 white texture when it has none, with the sampler matching its filter and wrap.
layout(set = 2, binding = 0) uniform sampler2D albedo_texture;

// Slot 0, pushed once per frame: LightUniforms in GpuUniforms.nox (112 bytes).
layout(set = 3, binding = 0) uniform LightUniforms {
    // Unit vector from the surface towards the sun.
    vec4 direction_to_light;
    vec4 light_color;
    // Sky light, for surfaces facing up.
    vec4 ambient_color;
    // Ground light, for surfaces facing down.
    vec4 ground_ambient_color;
    vec4 fog_color;
    // x: fog start distance, y: fog end distance, w: 1 when this shader must sRGB-encode its output.
    vec4 fog_and_output;
    vec4 camera_position;
} light;

// Slot 1, pushed once per draw: the draw's tint and material (MaterialUniforms in GpuUniforms.nox, 48 bytes).
layout(set = 3, binding = 1) uniform MaterialUniforms {
    vec4 color;
    // x: metallic, y: roughness.
    vec4 surface;
    // Light the surface gives off.
    vec4 emissive;
} material;

// One local light, four vec4s (GpuUniforms.nox, `LocalLightUniforms`).
struct LocalLight {
    // xyz: position, w: range.
    vec4 position_and_range;
    // rgb: colour times strength, w: 0 for a point light and 1 for a spot light.
    vec4 color_and_kind;
    // xyz: the way a spot light points, w: cosine of its outer cone angle.
    vec4 direction_and_outer_cosine;
    // x: cosine of the inner cone angle.
    vec4 cone;
};

// Slot 2, pushed once per frame: the local lights.
layout(set = 3, binding = 2) uniform LocalLightUniforms {
    // x: how many of the lights below are used.
    vec4 header;
    LocalLight lights[MAX_LOCAL_LIGHTS];
} locals;

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

// The GGX normal distribution: how much of the surface's microscopic mirrors face along the half vector. Rougher
// surfaces (larger alpha) spread it out, giving a broad dim highlight.
float DistributionGgx(float normal_dot_half, float alpha) {
    float alpha_squared = alpha * alpha;
    float denominator = normal_dot_half * normal_dot_half * (alpha_squared - 1.0) + 1.0;
    return alpha_squared / (PI * denominator * denominator);
}

// Smith's visibility term, the height-correlated form: the fraction of the microfacets that neither shadow nor mask
// each other, already divided by the 4 * dot(n, l) * dot(n, v) the specular formula needs.
float VisibilitySmith(float normal_dot_light, float normal_dot_view, float alpha) {
    float smith_view = normal_dot_light * mix(normal_dot_view, 1.0, alpha);
    float smith_light = normal_dot_view * mix(normal_dot_light, 1.0, alpha);
    return 0.5 / max(smith_view + smith_light, 0.0001);
}

// Schlick's approximation of Fresnel: reflectance grows from f0 (looking straight at the surface) to 1 (grazing).
vec3 FresnelSchlick(float view_dot_half, vec3 f0) {
    return f0 + (1.0 - f0) * pow(clamp(1.0 - view_dot_half, 0.0, 1.0), 5.0);
}

// The colour one light adds at this pixel: its radiance times the surface's response, times cos(angle of incidence).
vec3 EvaluateLight(vec3 normal, vec3 view, vec3 to_light, vec3 radiance, vec3 diffuse_color, vec3 f0, float alpha) {
    float normal_dot_light = max(dot(normal, to_light), 0.0);
    if (normal_dot_light <= 0.0) {
        return vec3(0.0);
    }
    vec3 half_vector = normalize(view + to_light);
    float normal_dot_view = max(dot(normal, view), 0.0001);
    float normal_dot_half = max(dot(normal, half_vector), 0.0);
    float view_dot_half = max(dot(view, half_vector), 0.0);
    vec3 fresnel = FresnelSchlick(view_dot_half, f0);
    vec3 specular = DistributionGgx(normal_dot_half, alpha) * VisibilitySmith(normal_dot_light, normal_dot_view, alpha) * fresnel;
    // What the surface reflects as a highlight it doesn't also scatter as diffuse light.
    vec3 diffuse = (1.0 - fresnel) * diffuse_color / PI;
    // See the file's header for the pi.
    return (diffuse + specular) * PI * radiance * normal_dot_light;
}

// How much of a local light's strength reaches a point `distance_to_light` away: the inverse-square law, with a smooth
// window that takes it to exactly zero at the light's range so a light can be skipped beyond it. The +1 keeps the
// value finite right at the light.
float DistanceAttenuation(float distance_to_light, float range) {
    float ratio = distance_to_light / max(range, 0.0001);
    float window = clamp(1.0 - ratio * ratio * ratio * ratio, 0.0, 1.0);
    return window * window / (distance_to_light * distance_to_light + 1.0);
}

void main() {
    vec4 albedo = texture(albedo_texture, surface_uv) * surface_color * material.color;
    // Interpolating unit normals across a triangle shortens them, so normalise again per pixel.
    vec3 normal = normalize(world_normal);
    vec3 view = normalize(light.camera_position.xyz - world_position);

    float metallic = clamp(material.surface.x, 0.0, 1.0);
    // Below about 0.04 the highlight shrinks to a few pixels and flickers, so roughness stops there.
    float roughness = clamp(material.surface.y, 0.04, 1.0);
    // The GGX alpha is the square of roughness, which spreads the artist's slider more evenly.
    float alpha = roughness * roughness;
    // A non-metal shows its colour as diffuse and reflects about 4% of light as a white highlight; a metal has no
    // diffuse and tints its highlight with its colour.
    vec3 diffuse_color = albedo.rgb * (1.0 - metallic);
    vec3 f0 = mix(vec3(0.04), albedo.rgb, metallic);

    float up_amount = 0.5 + 0.5 * normal.y;
    vec3 ambient = mix(light.ground_ambient_color.rgb, light.ambient_color.rgb, up_amount);
    vec3 lit = ambient * (diffuse_color + f0 * (1.0 - roughness));

    lit += EvaluateLight(normal, view, light.direction_to_light.xyz, light.light_color.rgb, diffuse_color, f0, alpha);

    int local_count = min(int(locals.header.x), MAX_LOCAL_LIGHTS);
    for (int index = 0; index < local_count; index++) {
        LocalLight local = locals.lights[index];
        vec3 to_light = local.position_and_range.xyz - world_position;
        float distance_to_light = length(to_light);
        if (distance_to_light >= local.position_and_range.w) {
            continue;
        }
        to_light /= max(distance_to_light, 0.0001);
        float strength = DistanceAttenuation(distance_to_light, local.position_and_range.w);
        if (local.color_and_kind.w > 0.5) {
            // A spot light fades from full strength at the inner cone to nothing at the outer one.
            float cosine_to_axis = dot(-to_light, local.direction_and_outer_cosine.xyz);
            float outer_cosine = local.direction_and_outer_cosine.w;
            float inner_cosine = local.cone.x;
            // smoothstep is undefined when its two edges are equal, so a cone with no soft edge cuts off sharply.
            float cone_fade = inner_cosine - outer_cosine > 0.0001
                ? smoothstep(outer_cosine, inner_cosine, cosine_to_axis)
                : step(outer_cosine, cosine_to_axis);
            strength *= cone_fade;
        }
        lit += EvaluateLight(normal, view, to_light, local.color_and_kind.rgb * strength, diffuse_color, f0, alpha);
    }

    lit += material.emissive.rgb;

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
