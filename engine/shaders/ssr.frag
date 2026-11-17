// Screen-space reflections: let glossy surfaces (wet streets, puddles, polished metal) show what is on the screen.
//
// The lit shader already gives every surface a reflection of the sky. That is right for open ground, but a puddle under a
// lamp should show the lamp, and a wet street should show the buildings beside it. The sky is all the lit shader has, because
// at that point the rest of the scene is not drawn yet. This pass runs afterwards, when the whole picture exists, and asks
// for each glossy pixel: "if I bounce my line of sight off this surface, which pixel of the picture do I hit?"
//
//   1. Read the pixel's surface (surface-common.glsl: its normal, roughness and reflectance) and its depth. Pixels that are
//      not glossy, and the sky, are copied through untouched: that early exit is what makes the pass cheap, since only the
//      wet ground and shiny things go on.
//   2. Rebuild the pixel's position in front of the camera (depth-common.glsl) and reflect the line of sight off the surface
//      normal, which gives the direction of the mirror ray.
//   3. March the ray across the screen. The ray is a segment in space; projected onto the picture it is a segment of the
//      screen too, so the pass walks along it in equal screen steps. At each step it compares the ray's distance from the
//      camera with the depth buffer's. While the ray is nearer, it is in the air in front of everything. The first step where it
//      is farther, the ray has gone behind a surface: a hit. A ray that goes behind a surface by more than a "thickness" has
//      passed behind a pole or a wall's edge and is let through.
//   4. Narrow the hit down: a few halvings between the last step in front and the first behind find the surface to within a
//      pixel or so.
//   5. Take the picture's colour at the hit. It becomes the reflection, scaled by how reflective the surface is for this
//      view angle (EnvironmentBrdf, which also makes a puddle a stronger mirror at a grazing angle), and faded where the
//      answer is not trustworthy: near the picture's edge, far along the ray, or where the hit surface faces away from the
//      ray (we can only see the front of things, so a ray that arrives behind one is wrong).
//
// Where there is no hit (the ray leaves the picture, or flies off into the sky) the result is the sky reflection the lit shader
// already added, which is why the answer is written as a change to the picture and not as a replacement:
//
//   picture + confidence * brdf * (what the ray hit - the sky reflection the lit shader added)
//
// so zero confidence leaves the picture as it was and full confidence swaps the sky for the scene. The change is faded out by
// the pixel's own fog, since the lit shader already fogged the pixel as a whole.
//
// The output is the whole picture, reflections included, in a second high dynamic range texture, and that is what bloom and the
// composite read: a reflected lamp is as bright as the lamp, so it glows.
//
// Not handled, as with every screen-space method: things off the picture, things hidden behind other things as seen from the
// camera (the back of a pole, the underside of a character), and tinted metal (the buffer holds one reflectance per pixel).
//
// SDL GPU fixes where resources live in SPIR-V for a fragment shader: texture-sampler pairs are set 2, uniform blocks are set
// 3, binding n for push slot n.
#version 450
#extension GL_GOOGLE_include_directive : require
#include "sky-common.glsl"
#include "surface-common.glsl"

// The scene before reflections, the scene's depth and the surface buffer. All three are read with `textureLod` at level 0:
// the lookups sit after early exits and inside loops, where the screen-space derivatives an ordinary lookup needs are undefined.
layout(set = 2, binding = 0) uniform sampler2D scene_image;
layout(set = 2, binding = 1) uniform sampler2D depth_image;
layout(set = 2, binding = 2) uniform sampler2D surface_image;

// Slot 0, pushed once per frame: ReflectionUniforms in GpuReflections.nox (176 bytes).
layout(set = 3, binding = 0) uniform ReflectionUniforms {
    // x: tan(half the horizontal field of view), y: tan(half the vertical), z: the near plane, w: the projection's depth range.
    vec4 projection_terms;
    // The camera's unit right, up and forward vectors in world space.
    vec4 camera_right;
    vec4 camera_up;
    vec4 camera_forward;
    // The sky the lit shader reflected, as it lit the scene.
    vec4 sky_zenith;
    vec4 sky_horizon;
    vec4 sky_ground;
    // x: how much of the lit shader's ambient light came from the sky (0 to 1), y: how bright that sky light is.
    vec4 sky_strength_and_intensity;
    // x: where fog starts, in metres from the camera, y: where it is complete.
    vec4 fog;
    // x: how many steps along the ray, y: how many halvings to narrow a hit, z: how far behind a surface a ray may go and
    // still hit it, in metres at ten metres from the camera (it grows with distance), w: the longest ray, in metres.
    vec4 march;
    // x: the reflection's strength (1 is physically scaled), yzw: unused.
    vec4 look;
} settings;

layout(location = 0) in vec2 source_uv;
layout(location = 0) out vec4 output_color;

// The most steps and halvings the loops below can take; the uniforms choose how many of them run.
const int MAX_STEPS = 64;
const int MAX_REFINEMENTS = 8;

#include "depth-common.glsl"

// A pseudo-random number between 0 and 1 that differs from pixel to pixel without a pattern the eye picks out. The ray's
// start is moved by it, which trades the banding that equal steps leave for fine noise.
float InterleavedGradientNoise(vec2 pixel) {
    return fract(52.9829189 * fract(dot(pixel, vec2(0.06711056, 0.00583715))));
}

// How far in front of the camera the ray is at `t`, 0 at its start and 1 at its end. Screen position is linear in t, and so is
// 1 / distance (the perspective divide), which is why the interpolation is done on that.
float RayDistance(float start_inverse, float end_inverse, float t) {
    return 1.0 / mix(start_inverse, end_inverse, t);
}

// How far the ray is behind what the depth buffer shows at `uv`: positive once the ray has gone behind a surface. The sky is
// infinitely far, so a ray over it is never behind anything.
float DepthDifference(vec2 uv, float ray_distance) {
    float stored_depth = textureLod(depth_image, uv, 0.0).r;
    float scene_distance = IsBackground(stored_depth) ? 1.0e6 : LinearDistance(stored_depth);
    return ray_distance - scene_distance;
}

void main() {
    vec4 picture = textureLod(scene_image, source_uv, 0.0);
    output_color = picture;

    vec4 surface = textureLod(surface_image, source_uv, 0.0);
    float roughness = surface.b;
    float reflectance = surface.a;
    float glossiness = 1.0 - smoothstep(0.25, 0.55, roughness);
    float depth = textureLod(depth_image, source_uv, 0.0).r;
    if (IsBackground(depth) || reflectance < 0.01 || glossiness <= 0.0) {
        return;
    }

    vec3 right = settings.camera_right.xyz;
    vec3 up = settings.camera_up.xyz;
    vec3 forward = settings.camera_forward.xyz;
    vec3 position = ViewPosition(source_uv, depth);
    vec3 eye_ray_world = normalize(right * position.x + up * position.y + forward * -position.z);
    vec3 normal_world = DecodeNormal(surface.rg);
    vec3 reflected_world = reflect(eye_ray_world, normal_world);
    vec3 reflected = vec3(dot(reflected_world, right), dot(reflected_world, up), -dot(reflected_world, forward));
    vec3 normal_view = vec3(dot(normal_world, right), dot(normal_world, up), -dot(normal_world, forward));

    // Start a little off the surface, so the ray cannot hit the pixel it leaves.
    vec3 start = position + normal_view * (0.02 + 0.004 * -position.z);
    float max_distance = settings.march.w;
    float near_plane = settings.projection_terms.z;
    vec3 end = start + reflected * max_distance;
    if (end.z > -near_plane) {
        float to_near_plane = (-near_plane - start.z) / reflected.z;
        end = start + reflected * clamp(to_near_plane * 0.98, 0.0, max_distance);
    }
    vec2 start_uv = ProjectToUv(start);
    vec2 end_uv = ProjectToUv(end);
    float start_inverse = 1.0 / -start.z;
    float end_inverse = 1.0 / -end.z;

    int step_count = min(int(settings.march.x), MAX_STEPS);
    int refinement_count = min(int(settings.march.y), MAX_REFINEMENTS);
    float thickness_at_ten_metres = settings.march.z;
    float jitter = InterleavedGradientNoise(gl_FragCoord.xy);

    float previous_t = 0.0;
    float hit_t = -1.0;
    for (int step_index = 0; step_index < MAX_STEPS; step_index++) {
        if (step_index >= step_count) {
            break;
        }
        // Steps are half as long at the ray's start as at its end: what is near the reflecting surface is seen closely, and a thin
        // thing there is easy to step over.
        float progress = (float(step_index) + 1.0 - 0.8 * jitter) / float(step_count);
        float t = 0.5 * progress + 0.5 * progress * progress;
        vec2 uv = mix(start_uv, end_uv, t);
        if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
            break;
        }
        float ray_distance = RayDistance(start_inverse, end_inverse, t);
        float difference = DepthDifference(uv, ray_distance);
        // A long step can jump clean over a thin thing, so the allowed depth behind a surface grows with the ground a step covers.
        float step_span = abs(ray_distance - RayDistance(start_inverse, end_inverse, previous_t));
        float thickness = thickness_at_ten_metres * max(ray_distance * 0.1, 0.5) + step_span;
        if (difference > 0.0 && difference < thickness) {
            float behind_t = t;
            float front_t = previous_t;
            for (int refinement_index = 0; refinement_index < MAX_REFINEMENTS; refinement_index++) {
                if (refinement_index >= refinement_count) {
                    break;
                }
                float middle_t = 0.5 * (front_t + behind_t);
                float middle_difference = DepthDifference(mix(start_uv, end_uv, middle_t), RayDistance(start_inverse, end_inverse, middle_t));
                if (middle_difference > 0.0) {
                    behind_t = middle_t;
                } else {
                    front_t = middle_t;
                }
            }
            // The halvings leave the hit somewhere within 1/2^n of the step, so that much error is allowed on top of the thickness. A
            // ray that went behind a much nearer thing (a pole in front of a wall) is still far behind it after the halvings and is
            // let through.
            float final_difference = DepthDifference(mix(start_uv, end_uv, behind_t), RayDistance(start_inverse, end_inverse, behind_t));
            float tolerance = thickness_at_ten_metres * max(ray_distance * 0.1, 0.5) + step_span / exp2(float(refinement_count));
            if (final_difference < tolerance) {
                hit_t = behind_t;
                break;
            }
        }
        previous_t = t;
    }

    vec3 sky_reflection = SkyGradient(reflected_world, settings.sky_zenith.rgb, settings.sky_horizon.rgb, settings.sky_ground.rgb, roughness)
        * settings.sky_strength_and_intensity.y * settings.sky_strength_and_intensity.x;
    if (hit_t < 0.0) {
        return;
    }

    vec2 hit_uv = mix(start_uv, end_uv, hit_t);
    vec3 hit_color = textureLod(scene_image, hit_uv, 0.0).rgb;
    vec3 hit_normal_world = DecodeNormal(textureLod(surface_image, hit_uv, 0.0).rg);

    vec2 border = min(hit_uv, 1.0 - hit_uv);
    float edge_fade = smoothstep(0.0, 0.1, min(border.x, border.y));
    float travel_fade = 1.0 - smoothstep(0.6, 1.0, length(mix(start, end, hit_t) - start) / max(max_distance, 0.001));
    float facing_fade = 1.0 - smoothstep(0.0, 0.4, dot(hit_normal_world, reflected_world));
    float fog = clamp((length(position) - settings.fog.x) / max(settings.fog.y - settings.fog.x, 0.001), 0.0, 1.0);
    float confidence = glossiness * edge_fade * travel_fade * facing_fade * (1.0 - fog) * settings.look.x;

    vec3 brdf = EnvironmentBrdf(vec3(reflectance), roughness, max(dot(normal_world, -eye_ray_world), 0.0001));
    output_color = vec4(max(picture.rgb + confidence * brdf * (hit_color - sky_reflection), vec3(0.0)), picture.a);
}
