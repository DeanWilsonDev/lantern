// The lit fragment shader's main: SurfaceOf (defined by the shader that includes this) says what the surface is, and the rest
// lights it. Included last by lit.frag and by every generated material shader.
void main() {
    MaterialSurface surface = SurfaceOf(surface_uv);
    if (surface.opacity_mask < 0.5) {
        discard;
    }
    vec4 albedo = vec4(surface.base_color, 1.0) * surface_color * material.color;
    // Interpolating unit normals across a triangle shortens them, so normalise again per pixel.
    vec3 normal = normalize(world_normal);
    if (surface.normal_strength > 0.0) {
        normal = PerturbNormal(normal, world_position, surface.normal_uv, surface.tangent_normal, surface.normal_strength);
    }
    vec3 view = normalize(light.camera_position.xyz - world_position);

    float metallic = clamp(surface.metallic, 0.0, 1.0);
    // Below about 0.04 the highlight shrinks to a few pixels and flickers, so roughness stops there.
    float roughness = clamp(surface.roughness, 0.04, 1.0);

    // Wetness. Water in a surface's pores darkens its diffuse colour (a metal has no diffuse to darken) and a film of water
    // on top is smooth, so roughness falls most of the way to a mirror's. Puddles go further: they are flat (the surface's
    // normal, bumps and all, is replaced by straight up), nearly black underneath and a full mirror on top.
    float wetness = clamp(material.surface.w, 0.0, 1.0);
    float puddle = PuddleMask(world_position, normalize(world_normal).y, wetness * clamp(material.emissive.w, 0.0, 1.0));
    float darkening = mix(1.0, 0.55, wetness * (1.0 - metallic));
    darkening = mix(darkening, 0.3, puddle);
    albedo.rgb *= darkening;
    roughness = mix(roughness, 0.1, wetness * 0.8);
    roughness = mix(roughness, 0.04, puddle);
    normal = normalize(mix(normal, vec3(0.0, 1.0, 0.0), puddle));
    // The GGX alpha is the square of roughness, which spreads the artist's slider more evenly.
    float alpha = roughness * roughness;
    // A non-metal shows its colour as diffuse and reflects about 4% of light as a white highlight; a metal has no
    // diffuse and tints its highlight with its colour.
    vec3 diffuse_color = albedo.rgb * (1.0 - metallic);
    vec3 f0 = mix(vec3(0.04), albedo.rgb, metallic);

    float up_amount = 0.5 + 0.5 * normal.y;
    vec3 ambient = mix(light.ground_ambient_color.rgb, light.ambient_color.rgb, up_amount);
    vec3 flat_ambient = ambient * (diffuse_color + f0 * (1.0 - roughness));
    // Image-based lighting from the sky: a rough average of the sky around the normal lights diffuse colour, and the sky
    // in the mirror direction, blurred by roughness, shows in the specular. It replaces the flat pair above by the
    // strength the game sets.
    vec3 sky_diffuse = SkyGradient(normal, light.sky_zenith.rgb, light.sky_horizon.rgb, light.sky_ground.rgb, 1.0) * diffuse_color;
    vec3 sky_reflection = SkyGradient(reflect(-view, normal), light.sky_zenith.rgb, light.sky_horizon.rgb, light.sky_ground.rgb, roughness);
    vec3 sky_specular = sky_reflection * EnvironmentBrdf(f0, roughness, max(dot(normal, view), 0.0001));
    vec3 sky_ambient = (sky_diffuse + sky_specular) * light.sky_strength_and_intensity.y;
    vec3 lit = mix(flat_ambient, sky_ambient, light.sky_strength_and_intensity.x);

    float sun_visibility = SunShadow(world_position, normal);
    lit += EvaluateLight(normal, view, light.direction_to_light.xyz, light.light_color.rgb * sun_visibility, diffuse_color, f0, alpha);

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
        float visibility = LocalShadow(local, world_position, normal, -to_light * distance_to_light, distance_to_light);
        lit += EvaluateLight(normal, view, to_light, local.color_and_kind.rgb * strength * visibility, diffuse_color, f0, alpha);
    }

    // The surface's own light, then the draw's (which a game can raise past 1 with SetGlow).
    lit += max(surface.emissive, vec3(0.0)) + material.emissive.rgb;

    float distance_to_camera = distance(world_position, light.camera_position.xyz);
    float fog_range = max(light.fog_and_output.y - light.fog_and_output.x, 0.001);
    float fog = clamp((distance_to_camera - light.fog_and_output.x) / fog_range, 0.0, 1.0);
    lit = mix(lit, light.fog_color.rgb, fog);

    // The value goes out as linear light, which can be far above 1: the target is a 16-bit float texture, and
    // composite.frag tone maps it to the screen's range and encodes it to sRGB.
    // Meshes are opaque: the lit pipeline doesn't blend, so alpha is written as 1.
    output_color = vec4(lit, 1.0);
    output_surface = PackSurface(normal, roughness, f0);
}
