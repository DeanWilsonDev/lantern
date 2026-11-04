#version 450

layout(set = 2, binding = 0) uniform sampler2D albedo_texture;

layout(set = 3, binding = 0) uniform LightUniforms {
    vec4 direction_to_light;
    vec4 light_color;
    vec4 ambient_color;
    vec4 fog_color;
    vec4 fog_and_output;
    vec4 camera_position;
} light;

layout(set = 3, binding = 1) uniform MaterialUniforms {
    vec4 color;
} material;

layout(location = 0) in vec3 world_position;
layout(location = 1) in vec3 world_normal;
layout(location = 2) in vec2 surface_uv;
layout(location = 3) in vec4 surface_color;

layout(location = 0) out vec4 output_color;

vec3 EncodeSrgb(vec3 linear_color) {
    vec3 low = linear_color * 12.92;
    vec3 high = 1.055 * pow(linear_color, vec3(1.0 / 2.4)) - 0.055;
    return mix(high, low, lessThanEqual(linear_color, vec3(0.0031308)));
}

void main() {
    vec4 albedo = texture(albedo_texture, surface_uv) * surface_color * material.color;
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
    output_color = vec4(lit, 1.0);
}
