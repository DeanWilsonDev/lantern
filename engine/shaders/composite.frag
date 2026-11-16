// The last step before the screen: turn the scene's high dynamic range image into the 0 to 1 range a display shows.
//
// The lit pass writes light, not colour: a white wall in full sun is 1.0 and a lamp or a gem can be 5 or 20. The target
// it writes into is a 16-bit float texture, so those values survive instead of being cut off at 1. This pass reads that
// texture and does three things to each pixel, in this order:
//
//   0. multiply by the ambient-occlusion picture, when there is one (1 where nothing is blocked, darker in creases), then
//      darken under blob shadows, add height fog and light shafts, which need each pixel's distance and so read the depth
//      buffer too;
//   1. add the bloom picture (the blurred bright parts) scaled by a strength;
//   2. multiply by an exposure, the camera's brightness knob: 2 makes the whole picture twice as bright before step 3;
//   3. tone map: squeeze 0..infinity into 0..1 with a curve that keeps dark and middle values nearly as they were and
//      rolls bright ones off smoothly towards white, instead of clipping them flat.
//
// The occlusion multiplies the lit scene as a whole, before bloom is added, so a glowing gem's halo is not darkened by it.
// (A renderer that lit in several passes would apply it to the ambient light alone; this one lights in a single pass, so
// it takes the simpler route and darkens direct light in creases a little too.)
//
// Three curves are available, picked by a number in the uniforms:
//   0  none: clamp to 1, the look of a renderer without high dynamic range;
//   1  Khronos PBR Neutral: leaves everything below about 0.8 exactly as it was and only rolls off the brighter light, so a
//      scene lit for a clamped target keeps its colours. The default;
//   2  an ACES fit by Krzysztof Narkowicz: f(x) = (x (2.51 x + 0.03)) / (x (2.43 x + 0.59) + 0.14). An "S": a lifted toe, a
//      contrasty middle and a long shoulder. More cinematic, but it brightens and desaturates pastel colours.
//
// After the tone mapping comes the colour grade (saturation, contrast, tint, vignette), a game's hook for a chapter's look.
//
// The output is encoded to sRGB here when the target can't do it itself (see GpuDevice.nox), so the lit pass can stay in
// linear light.
#version 450

layout(set = 2, binding = 0) uniform sampler2D scene_image;
layout(set = 2, binding = 1) uniform sampler2D bloom_image;
layout(set = 2, binding = 2) uniform sampler2D occlusion_image;
layout(set = 2, binding = 3) uniform sampler2D depth_image;

// Slot 0: x: exposure, y: bloom strength (0 when bloom is off), z: the tone mapper (0, 1 or 2 above), w: 1 to sRGB-encode.
layout(set = 3, binding = 0) uniform CompositeUniforms {
    vec4 exposure_bloom_tonemap_encode;
    // x: 1 when the ambient-occlusion picture is drawn this frame (it is the scene itself otherwise, and unread),
    // y: 1 when height fog is on, z: 1 when light shafts are on, w: how many blob shadows there are (0 to 8).
    vec4 flags;
    // x: tan(half the horizontal field of view), y: tan(half the vertical), z: the near plane, w: the projection's depth range.
    vec4 projection_terms;
    // The camera's unit right, up and forward vectors in world space.
    vec4 camera_right;
    vec4 camera_up;
    vec4 camera_forward;
    // xyz: the camera's world position, w: how fast fog thins with height (per metre).
    vec4 camera_position_and_falloff;
    // rgb: the fog colour, w: the fog density at the base height (per metre).
    vec4 fog_color_and_density;
    // xyz: the unit direction towards the sun, w: how much the sun brightens fog that is looked at towards it.
    vec4 sun_direction_and_scattering;
    // rgb: the sun's colour, w: the light shafts' strength.
    vec4 sun_color_and_shaft_strength;
    // x: the height where fog has its full density, y: how much of the shaft's light survives each step towards the sun.
    vec4 fog_base_and_shaft_decay;
    // The colour grade: x: saturation (1 leaves colours as they are), y: contrast (1 leaves it), z: vignette strength, w: unused.
    vec4 grade_saturation_contrast_vignette;
    // rgb: the colour the picture is multiplied by after grading; white leaves it as it is.
    vec4 grade_tint;
    // Blob shadows, two vec4s each: xyz the point on the ground it sits on and w its radius in metres, then x its darkness (0
    // to 1) and y how far above and below that point, in metres, it still darkens. flags.w says how many are used.
    vec4 blobs[16];
} settings;

layout(location = 0) in vec2 source_uv;
layout(location = 0) out vec4 output_color;

vec3 EncodeSrgb(vec3 linear_color) {
    vec3 low = linear_color * 12.92;
    vec3 high = 1.055 * pow(linear_color, vec3(1.0 / 2.4)) - 0.055;
    return mix(high, low, lessThanEqual(linear_color, vec3(0.0031308)));
}

vec3 AcesFilmic(vec3 x) {
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
}

// Khronos PBR Neutral. Below the compression start every colour passes through with only a small offset in the very darkest
// values; above it the brightest channel is rolled off towards 1 and the colour drifts towards white, as a real sensor's does.
vec3 PbrNeutral(vec3 color) {
    const float start_compression = 0.8 - 0.04;
    const float desaturation = 0.15;
    float lowest = min(color.r, min(color.g, color.b));
    float offset = lowest < 0.08 ? lowest - 6.25 * lowest * lowest : 0.04;
    color -= offset;
    float peak = max(color.r, max(color.g, color.b));
    if (peak < start_compression) {
        return color;
    }
    float room = 1.0 - start_compression;
    float new_peak = 1.0 - room * room / (peak + room - start_compression);
    color *= new_peak / peak;
    float blend_to_white = 1.0 - 1.0 / (desaturation * (peak - new_peak) + 1.0);
    return mix(color, new_peak * vec3(1.0), blend_to_white);
}

vec3 ToneMap(vec3 exposed, float mapper) {
    if (mapper > 1.5) {
        return AcesFilmic(exposed);
    }
    if (mapper > 0.5) {
        return clamp(PbrNeutral(exposed), 0.0, 1.0);
    }
    return clamp(exposed, 0.0, 1.0);
}

// The distance in front of the camera, from the stored depth. See ao-common.glsl for the algebra.
float LinearDistance(float depth) {
    float depth_range = settings.projection_terms.w;
    return depth_range * settings.projection_terms.z / (depth + depth_range);
}

bool IsBackground(float depth) {
    return depth > 0.99999;
}

// Height fog: the air is thickest near the base height and thins exponentially above it,
//   density(y) = base_density * exp(-falloff * (y - base)).
// How much light a ray loses is the density added up along it (its optical depth), and that has a closed form:
//   optical = base_density * exp(-falloff * (camera_y - base)) * (1 - exp(-falloff * ray_y * length)) / (falloff * ray_y)
// which becomes plain `density * length` when the ray is level. Then fog = 1 - exp(-optical): thin air adds a little and
// a long or low ray soaks up almost all of it. The fog is a colour, and looking towards the sun it is brighter.
vec3 ApplyHeightFog(vec3 scene, vec3 ray, float ray_length) {
    float falloff = settings.camera_position_and_falloff.w;
    float base_density = settings.fog_color_and_density.w;
    float height_above_base = settings.camera_position_and_falloff.y - settings.fog_base_and_shaft_decay.x;
    float slope = falloff * ray.y;
    float exponent = clamp(-slope * ray_length, -30.0, 30.0);
    float length_term = abs(slope) > 0.0001 ? (1.0 - exp(exponent)) / slope : ray_length;
    float optical = base_density * exp(clamp(-falloff * height_above_base, -30.0, 30.0)) * length_term;
    float fog = 1.0 - exp(-max(optical, 0.0));
    float toward_sun = pow(max(dot(ray, settings.sun_direction_and_scattering.xyz), 0.0), 6.0);
    vec3 fog_color = settings.fog_color_and_density.rgb + settings.sun_color_and_shaft_strength.rgb * (settings.sun_direction_and_scattering.w * toward_sun);
    return mix(scene, fog_color, clamp(fog, 0.0, 1.0));
}

// Light shafts (crepuscular rays), done on the screen. Where the sky is visible past an object, sunlight reaches the
// camera through the haze; where the object blocks it, there is a shadow-shaped gap. So: walk from this pixel towards the
// sun's position on the screen and count how many of the steps land on sky (depth at the far plane). The more steps see
// sky, the more sun light is scattered towards the camera along this line of sight. Each step counts a little less than the
// one before, so the shaft fades with distance from the sun, and the result is scaled by how directly the pixel looks at
// the sun.
vec3 LightShafts(vec2 uv, vec3 ray) {
    vec3 sun = settings.sun_direction_and_scattering.xyz;
    float facing = dot(sun, settings.camera_forward.xyz);
    if (facing <= 0.05) {
        return vec3(0.0);
    }
    vec2 sun_ndc = vec2(dot(sun, settings.camera_right.xyz) / (facing * settings.projection_terms.x), dot(sun, settings.camera_up.xyz) / (facing * settings.projection_terms.y));
    vec2 sun_uv = vec2(sun_ndc.x * 0.5 + 0.5, 0.5 - sun_ndc.y * 0.5);
    const int STEPS = 24;
    vec2 step_vector = (sun_uv - uv) / float(STEPS);
    float decay = settings.fog_base_and_shaft_decay.y;
    float weight = 1.0;
    float total = 0.0;
    float total_weight = 0.0;
    vec2 position = uv;
    for (int step_index = 0; step_index < STEPS; step_index++) {
        position += step_vector;
        float sky = IsBackground(texture(depth_image, clamp(position, vec2(0.0), vec2(1.0))).r) ? 1.0 : 0.0;
        total += sky * weight;
        total_weight += weight;
        weight *= decay;
    }
    float toward_sun = pow(max(dot(ray, sun), 0.0), 3.0);
    float fade_in = smoothstep(0.05, 0.4, facing);
    return settings.sun_color_and_shaft_strength.rgb * (settings.sun_color_and_shaft_strength.w * toward_sun * fade_in * total / total_weight);
}

// Blob shadows: soft dark ellipses on the ground under things, so a character looks planted and a jump shows where it will
// land. Each is a point, a radius and a darkness. A pixel's world position is rebuilt from its depth and compared with each
// point: how far away it is across the ground, scaled by the radius, and how far above or below, scaled by the blob's
// vertical reach. Inside the ellipse the pixel is darkened, most at the centre and falling off smoothly to nothing at the
// rim. Because it works on whatever surface is there, a blob lies correctly over slopes, steps and walls without a decal
// mesh to place.
vec3 ApplyBlobShadows(vec3 scene, vec3 world_position) {
    int count = int(settings.flags.w);
    float darkness = 0.0;
    for (int blob_index = 0; blob_index < 8; blob_index++) {
        if (blob_index >= count) {
            break;
        }
        vec4 where_and_radius = settings.blobs[blob_index * 2];
        vec4 strength_and_reach = settings.blobs[blob_index * 2 + 1];
        vec3 offset = world_position - where_and_radius.xyz;
        float across = length(offset.xz) / max(where_and_radius.w, 0.001);
        float vertical = abs(offset.y) / max(strength_and_reach.y, 0.001);
        float edge = clamp(sqrt(across * across + vertical * vertical), 0.0, 1.0);
        float falloff = 1.0 - edge;
        darkness = max(darkness, strength_and_reach.x * falloff * falloff * (3.0 - 2.0 * falloff));
    }
    return scene * (1.0 - darkness);
}

// The colour grade: the look a chapter or a place wants, applied to the finished 0 to 1 picture, after tone mapping and before
// the sRGB encoding, so the numbers act on what the viewer sees. In order:
//   saturation: blend between the pixel's grey (its luma) and its colour. 0 is black and white, 1 is unchanged, above 1 is vivid;
//   contrast:   scale how far each channel is from middle grey, 0.5. Above 1 pushes darks darker and lights lighter;
//   tint:       multiply by a colour, to warm or cool the whole picture;
//   vignette:   darken towards the corners, which draws the eye to the middle.
vec3 ColorGrade(vec3 color, vec2 uv) {
    float saturation = settings.grade_saturation_contrast_vignette.x;
    float contrast = settings.grade_saturation_contrast_vignette.y;
    float vignette = settings.grade_saturation_contrast_vignette.z;
    float luma = dot(color, vec3(0.2126, 0.7152, 0.0722));
    color = mix(vec3(luma), color, saturation);
    color = (color - 0.5) * contrast + 0.5;
    color *= settings.grade_tint.rgb;
    float corner_distance = length(uv - 0.5) * 1.41421356;
    color *= 1.0 - vignette * smoothstep(0.35, 1.0, corner_distance);
    return clamp(color, 0.0, 1.0);
}

void main() {
    vec3 scene = texture(scene_image, source_uv).rgb;
    if (settings.flags.x > 0.5) {
        scene *= texture(occlusion_image, source_uv).r;
    }
    if (settings.flags.y > 0.5 || settings.flags.z > 0.5 || settings.flags.w > 0.5) {
        float depth = texture(depth_image, source_uv).r;
        vec2 ndc = vec2(source_uv.x * 2.0 - 1.0, 1.0 - source_uv.y * 2.0);
        vec3 unnormalized = settings.camera_forward.xyz + settings.camera_right.xyz * (ndc.x * settings.projection_terms.x) + settings.camera_up.xyz * (ndc.y * settings.projection_terms.y);
        vec3 ray = normalize(unnormalized);
        if (settings.flags.w > 0.5 && !IsBackground(depth)) {
            scene = ApplyBlobShadows(scene, settings.camera_position_and_falloff.xyz + unnormalized * LinearDistance(depth));
        }
        if (settings.flags.y > 0.5 && !IsBackground(depth)) {
            scene = ApplyHeightFog(scene, ray, LinearDistance(depth) * length(unnormalized));
        }
        if (settings.flags.z > 0.5) {
            scene += LightShafts(source_uv, ray);
        }
    }
    float bloom_strength = settings.exposure_bloom_tonemap_encode.y;
    if (bloom_strength > 0.0) {
        scene += texture(bloom_image, source_uv).rgb * bloom_strength;
    }
    vec3 exposed = scene * settings.exposure_bloom_tonemap_encode.x;
    vec3 display = ToneMap(exposed, settings.exposure_bloom_tonemap_encode.z);
    display = ColorGrade(display, source_uv);
    if (settings.exposure_bloom_tonemap_encode.w > 0.5) {
        display = EncodeSrgb(display);
    }
    output_color = vec4(display, 1.0);
}
