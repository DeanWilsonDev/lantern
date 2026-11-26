// The lit fragment shader's declarations and lighting functions: everything lit.frag had before its main, shared by lit.frag
// and every material graph's generated shader. A fragment shader includes this, then defines SurfaceOf, then includes
// lit-main.glsl. See lit.frag's header for how the lighting works.
const float PI = 3.14159265359;
// Must match MAX_LOCAL_LIGHTS in RenderWorld.nox.
const int MAX_LOCAL_LIGHTS = 8;

// The draw's texture, or the 1x1 white texture when it has none, with the sampler matching its filter and wrap.
layout(set = 2, binding = 0) uniform sampler2D albedo_texture;
// The sun's shadow map: three cascades side by side. A shadow sampler compares the reference depth it is given with the
// stored depth and returns 1 where the reference is nearer (lit) and 0 where it is farther (in shadow), blending the
// four nearest texels' answers, which smooths the edge by up to one texel.
layout(set = 2, binding = 1) uniform sampler2DShadow sun_shadow_map;
// The local lights' shadow map: tiles in a grid, one per spot light and six per point light.
layout(set = 2, binding = 2) uniform sampler2DShadow local_shadow_map;
// The draw's normal map: tangent-space normals stored as colours (0.5, 0.5, 1 is flat). Bound for every draw; the white
// texture stands in when there is none, and material.surface.z (the strength) is 0 so it is never read.
layout(set = 2, binding = 3) uniform sampler2D normal_texture;

// Slot 0, pushed once per frame: LightUniforms in GpuUniforms.nox (176 bytes).
layout(set = 3, binding = 0) uniform LightUniforms {
    // Unit vector from the surface towards the sun.
    vec4 direction_to_light;
    vec4 light_color;
    // Sky light, for surfaces facing up.
    vec4 ambient_color;
    // Ground light, for surfaces facing down.
    vec4 ground_ambient_color;
    vec4 fog_color;
    // x: fog start distance, y: fog end distance.
    vec4 fog_and_output;
    // The sky that lights the scene: its colour straight up, at the horizon and straight down.
    vec4 sky_zenith;
    vec4 sky_horizon;
    vec4 sky_ground;
    // x: how much of the ambient light comes from the sky instead of the flat sky and ground pair (0 to 1), y: how
    // bright that sky light is.
    vec4 sky_strength_and_intensity;
    // xyz: the camera's position, w: seconds since the game or editor started, wrapped every hour (a material's mat_time).
    vec4 camera_position;
} light;

// Slot 1, pushed once per draw: the draw's tint and material (MaterialUniforms in GpuUniforms.nox, 80 bytes).
layout(set = 3, binding = 1) uniform MaterialUniforms {
    vec4 color;
    // x: metallic, y: roughness, z: normal map strength (0 when the draw has no normal map), w: wetness (0 dry, 1 soaked).
    vec4 surface;
    // rgb: light the surface gives off, w: how much of a wet upward-facing surface is puddles (0 to 1).
    vec4 emissive;
    // A material graph's parameters (its mat_param nodes), four to a vec4 in slot order; zero for a draw without a graph.
    vec4 parameters[2];
} material;

// One local light, four vec4s (GpuUniforms.nox, `LocalLightUniforms`).
struct LocalLight {
    // xyz: position, w: range.
    vec4 position_and_range;
    // rgb: colour times strength, w: 0 for a point light and 1 for a spot light.
    vec4 color_and_kind;
    // xyz: the way a spot light points, w: cosine of its outer cone angle.
    vec4 direction_and_outer_cosine;
    // x: cosine of the inner cone angle, y: the index of its first shadow tile or -1 when it casts no shadow, z: the
    // world size of one texel of its shadow map one unit away.
    vec4 cone;
};

// Slot 2, pushed once per frame: the local lights.
layout(set = 3, binding = 2) uniform LocalLightUniforms {
    // x: how many of the lights below are used.
    vec4 header;
    LocalLight lights[MAX_LOCAL_LIGHTS];
} locals;

// Slot 3, pushed once per frame: where the shadow maps came from (ShadowUniforms in GpuShadows.nox, 1024 bytes).
layout(set = 3, binding = 3) uniform ShadowUniforms {
    // The light view-projection matrix of each tile of the local shadow map.
    mat4 local_tile_matrices[12];
    // The same for each of the sun's three cascades.
    mat4 cascade_matrices[3];
    // x, y, z: where each cascade ends, as a distance along the camera's forward vector. w: 1 when the sun casts shadows.
    vec4 cascade_ends;
    // x, y, z: the world size of one texel of each cascade. w: how many texels to push a surface along its normal.
    vec4 cascade_texels;
    // xyz: the way the camera looks. w: the depth bias, in the light's depth units.
    vec4 camera_forward_and_bias;
    // x: the filter's radius in texels, y: how many lookups it makes, z: the fraction of a cascade, from its end, over which it
    // blends into the next (0 for a hard switch).
    vec4 filter_radius_samples_blend;
} shadows;

// Interpolated from lit.vert's outputs.
layout(location = 0) in vec3 world_position;
layout(location = 1) in vec3 world_normal;
layout(location = 2) in vec2 surface_uv;
layout(location = 3) in vec4 surface_color;

// The colour written to the colour target.
layout(location = 0) out vec4 output_color;
// The surface buffer's pixel: normal, roughness and reflectance, for the screen-space reflection pass (surface-common.glsl).
layout(location = 1) out vec4 output_surface;

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

// The sun's tiles sit side by side: 3 across.
const float SUN_CASCADE_COUNT = 3.0;
// The local tiles form a grid 6 across and 2 down. Must match LOCAL_SHADOW_COLUMNS and LOCAL_SHADOW_ROWS.
const float LOCAL_COLUMNS = 6.0;
const float LOCAL_ROWS = 2.0;

// Reads a shadow map around `uv` with a soft filter. The shadow sampler already smooths a lookup over four texels, so one
// lookup is a little soft. This takes many, spread over a disc `filter_radius` texels wide, and averages them: the edge of
// a shadow fades across the disc, which is what a penumbra is. The lookups are placed on a sunflower spiral (each one
// turned by the golden angle from the last, so they cover the disc evenly), and the whole spiral is turned by a different
// angle in every pixel. That trades the regular banding a fixed pattern leaves for a fine noise the eye ignores.
// `tile_minimum` and `tile_maximum` fence the lookups inside the pixel's own tile, so they never read a neighbouring
// tile's depths.
float InterleavedGradientNoise(vec2 pixel) {
    return fract(52.9829189 * fract(dot(pixel, vec2(0.06711056, 0.00583715))));
}

float FilteredShadow(sampler2DShadow shadow_map, vec2 uv, float reference_depth, vec2 texel, vec2 tile_minimum, vec2 tile_maximum) {
    float radius = shadows.filter_radius_samples_blend.x;
    int sample_count = int(shadows.filter_radius_samples_blend.y);
    float turn = InterleavedGradientNoise(gl_FragCoord.xy) * 6.2831853;
    float lit_amount = 0.0;
    for (int sample_index = 0; sample_index < sample_count; sample_index++) {
        float ring = sqrt((float(sample_index) + 0.5) / float(sample_count)) * radius;
        float angle = float(sample_index) * 2.3999632 + turn;
        vec2 sample_uv = clamp(uv + vec2(cos(angle), sin(angle)) * ring * texel, tile_minimum + texel * 0.5, tile_maximum - texel * 0.5);
        lit_amount += texture(shadow_map, vec3(sample_uv, reference_depth));
    }
    return lit_amount / float(sample_count);
}

// How much of the sun reaches this pixel through one cascade: 1 fully lit, 0 fully shadowed. The pixel is moved a little way
// along its normal first. Without that push a surface can shadow itself, speckling it with "acne", because the map's
// texels are coarser than the surface. A pixel outside the cascade's picture is lit.
float CascadeShadow(int cascade, vec3 position, vec3 normal) {
    vec3 pushed = position + normal * (shadows.cascade_texels[cascade] * shadows.cascade_texels.w);
    vec4 clip = shadows.cascade_matrices[cascade] * vec4(pushed, 1.0);
    // Clip space has +y up and texture space has +y down.
    vec2 tile_uv = vec2(clip.x * 0.5 + 0.5, 0.5 - clip.y * 0.5);
    if (clip.z > 1.0 || tile_uv.x < 0.0 || tile_uv.x > 1.0 || tile_uv.y < 0.0 || tile_uv.y > 1.0) {
        return 1.0;
    }
    vec2 atlas_uv = vec2((float(cascade) + tile_uv.x) / SUN_CASCADE_COUNT, tile_uv.y);
    vec2 texel = 1.0 / vec2(textureSize(sun_shadow_map, 0));
    vec2 tile_minimum = vec2(float(cascade) / SUN_CASCADE_COUNT, 0.0);
    vec2 tile_maximum = vec2(float(cascade + 1) / SUN_CASCADE_COUNT, 1.0);
    return FilteredShadow(sun_shadow_map, atlas_uv, clip.z - shadows.camera_forward_and_bias.w, texel, tile_minimum, tile_maximum);
}

// How much of the sun reaches this pixel: 1 fully lit, 0 fully shadowed. Picks the cascade by how far the pixel is from the
// camera. Near the end of a cascade it also looks the pixel up in the next one and blends the two, so the switch isn't a
// visible line; the last cascade fades out to "no shadow" the same way instead of stopping at its edge.
float SunShadow(vec3 position, vec3 normal) {
    if (shadows.cascade_ends.w < 0.5) {
        return 1.0;
    }
    float view_depth = dot(position - light.camera_position.xyz, shadows.camera_forward_and_bias.xyz);
    int cascade = 3;
    if (view_depth < shadows.cascade_ends.x) {
        cascade = 0;
    } else if (view_depth < shadows.cascade_ends.y) {
        cascade = 1;
    } else if (view_depth < shadows.cascade_ends.z) {
        cascade = 2;
    }
    if (cascade > 2) {
        return 1.0;
    }
    float shadow = CascadeShadow(cascade, position, normal);
    float blend_fraction = shadows.filter_radius_samples_blend.z;
    if (blend_fraction > 0.0) {
        float cascade_end = shadows.cascade_ends[cascade];
        float blend = smoothstep(cascade_end * (1.0 - blend_fraction), cascade_end, view_depth);
        if (blend > 0.0) {
            float neighbour = cascade < 2 ? CascadeShadow(cascade + 1, position, normal) : 1.0;
            shadow = mix(shadow, neighbour, blend);
        }
    }
    return shadow;
}

// How much of a local light reaches this pixel. A spot light has one tile. A point light has six, one per face of a cube
// around it, and the face is the axis along which the pixel is farthest from the light: +x, -x, +y, -y, +z, -z in tile
// order. A perspective shadow map's texels cover more world the farther they are, so the normal push grows with distance.
float LocalShadow(LocalLight local, vec3 position, vec3 normal, vec3 light_to_position, float distance_to_light) {
    int tile = int(local.cone.y);
    if (tile < 0) {
        return 1.0;
    }
    if (local.color_and_kind.w < 0.5) {
        vec3 magnitude = abs(light_to_position);
        int face = 5;
        if (magnitude.x >= magnitude.y && magnitude.x >= magnitude.z) {
            face = light_to_position.x > 0.0 ? 0 : 1;
        } else if (magnitude.y >= magnitude.z) {
            face = light_to_position.y > 0.0 ? 2 : 3;
        } else {
            face = light_to_position.z > 0.0 ? 4 : 5;
        }
        tile += face;
    }
    vec3 pushed = position + normal * (distance_to_light * local.cone.z * 2.0);
    vec4 clip = shadows.local_tile_matrices[tile] * vec4(pushed, 1.0);
    float reference_depth = clip.z / clip.w;
    vec2 tile_uv = vec2(clip.x / clip.w * 0.5 + 0.5, 0.5 - clip.y / clip.w * 0.5);
    if (clip.w <= 0.0 || reference_depth > 1.0 || tile_uv.x < 0.0 || tile_uv.x > 1.0 || tile_uv.y < 0.0 || tile_uv.y > 1.0) {
        return 1.0;
    }
    float column = float(tile % 6);
    float row = float(tile / 6);
    vec2 atlas_uv = vec2((column + tile_uv.x) / LOCAL_COLUMNS, (row + tile_uv.y) / LOCAL_ROWS);
    vec2 texel = 1.0 / vec2(textureSize(local_shadow_map, 0));
    vec2 tile_minimum = vec2(column / LOCAL_COLUMNS, row / LOCAL_ROWS);
    vec2 tile_maximum = vec2((column + 1.0) / LOCAL_COLUMNS, (row + 1.0) / LOCAL_ROWS);
    return FilteredShadow(local_shadow_map, atlas_uv, reference_depth - 0.0001, texel, tile_minimum, tile_maximum);
}

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

// Bends the surface normal by a tangent-space normal from a map. The mesh has no tangents, so the tangent frame comes from
// how position and uv change from pixel to pixel (a "cotangent frame"). The map's green channel points up the image.
vec3 PerturbNormal(vec3 surface_normal, vec3 position, vec2 uv, vec3 tangent_space, float strength) {
    vec3 position_dx = dFdx(position);
    vec3 position_dy = dFdy(position);
    vec2 uv_dx = dFdx(uv);
    vec2 uv_dy = dFdy(uv);
    vec3 perpendicular_y = cross(position_dy, surface_normal);
    vec3 perpendicular_x = cross(surface_normal, position_dx);
    vec3 tangent = perpendicular_y * uv_dx.x + perpendicular_x * uv_dy.x;
    vec3 bitangent = -(perpendicular_y * uv_dx.y + perpendicular_x * uv_dy.y);
    float scale = inversesqrt(max(max(dot(tangent, tangent), dot(bitangent, bitangent)), 1e-12));
    vec3 scaled = vec3(tangent_space.xy * strength, tangent_space.z);
    if (dot(scaled, scaled) < 1e-12) {
        return surface_normal;
    }
    vec3 bent = normalize(scaled);
    return normalize(tangent * scale * bent.x + bitangent * scale * bent.y + surface_normal * bent.z);
}

// A pseudo-random number between 0 and 1 for a point on an integer grid. The fract/dot trick needs no texture and no
// integer maths, and is good enough for noise the eye reads as irregular blobs.
float HashGrid(vec2 cell) {
    return fract(sin(dot(cell, vec2(127.1, 311.7))) * 43758.5453);
}

// Smooth value noise: the hash at the four grid corners around `point`, blended with a smoothstep curve so the result has no
// visible grid lines. Between 0 and 1.
float ValueNoise(vec2 point) {
    vec2 cell = floor(point);
    vec2 within = fract(point);
    vec2 blend = within * within * (3.0 - 2.0 * within);
    float low = mix(HashGrid(cell), HashGrid(cell + vec2(1.0, 0.0)), blend.x);
    float high = mix(HashGrid(cell + vec2(0.0, 1.0)), HashGrid(cell + vec2(1.0, 1.0)), blend.x);
    return mix(low, high, blend.y);
}

// How much of the surface at `position` is puddle, 0 (dry) to 1 (open water). Two octaves of noise over the ground plane
// make blobs a few metres across with a ragged edge; `coverage` (0 to 1) says how much of the ground they fill by moving
// the cut-off, and the short smoothstep keeps the water's edge crisp but not aliased. Only surfaces facing nearly straight up
// can hold water, so walls and slopes get none.
float PuddleMask(vec3 position, float up_facing, float coverage) {
    if (coverage <= 0.0) {
        return 0.0;
    }
    float blobs = ValueNoise(position.xz * 0.45) * 0.65 + ValueNoise(position.xz * 1.3 + 17.0) * 0.35;
    float level = 1.0 - coverage * 0.75;
    return smoothstep(level, level + 0.06, blobs) * smoothstep(0.88, 0.98, up_facing);
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
