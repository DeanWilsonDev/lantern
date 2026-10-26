// Lit, textured opaque meshes: one directional light, a hemispheric ambient term and linear
// distance fog. Binding groups follow SDL GPU's layout (see crates/sdl-gpu/build.rs):
//   group 1 = vertex uniform buffers, group 2 = fragment texture/sampler pairs,
//   group 3 = fragment uniform buffers.

struct FrameUniforms {
    view_projection: mat4x4<f32>,
    camera_position: vec4<f32>,
};

struct ObjectUniforms {
    model: mat4x4<f32>,
    // Inverse-transpose of the model matrix's upper 3x3, stored as a mat4 for simple alignment.
    normal_matrix: mat4x4<f32>,
};

@group(1) @binding(0) var<uniform> frame: FrameUniforms;
@group(1) @binding(1) var<uniform> object: ObjectUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
};

@vertex
fn vertex_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let world_position = object.model * vec4<f32>(input.position, 1.0);
    output.clip_position = frame.view_projection * world_position;
    output.world_position = world_position.xyz;
    output.world_normal = (object.normal_matrix * vec4<f32>(input.normal, 0.0)).xyz;
    output.uv = input.uv;
    output.color = input.color;
    return output;
}

struct LightUniforms {
    // xyz: unit direction from the surface towards the light.
    direction_to_light: vec4<f32>,
    light_color: vec4<f32>,
    ambient_color: vec4<f32>,
    fog_color: vec4<f32>,
    // x: fog start, y: fog end, z: unused, w: 1 when the shader must sRGB-encode its output.
    fog_and_output: vec4<f32>,
    camera_position: vec4<f32>,
};

struct MaterialUniforms {
    color: vec4<f32>,
};

@group(2) @binding(0) var albedo_texture: texture_2d<f32>;
@group(2) @binding(1) var albedo_sampler: sampler;
@group(3) @binding(0) var<uniform> light: LightUniforms;
@group(3) @binding(1) var<uniform> material: MaterialUniforms;

fn encode_srgb(linear: vec3<f32>) -> vec3<f32> {
    let low = linear * 12.92;
    let high = 1.055 * pow(linear, vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(high, low, linear <= vec3<f32>(0.0031308));
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let albedo = textureSample(albedo_texture, albedo_sampler, input.uv) * input.color * material.color;
    let normal = normalize(input.world_normal);
    let diffuse = max(dot(normal, light.direction_to_light.xyz), 0.0);
    // Sky-ish ambient from above, darker from below, so shapes read even in shadow.
    let hemisphere = 0.65 + 0.35 * normal.y;
    var lit = albedo.rgb * (light.ambient_color.rgb * hemisphere + light.light_color.rgb * diffuse);

    let distance_to_camera = distance(input.world_position, light.camera_position.xyz);
    let fog = clamp((distance_to_camera - light.fog_and_output.x) / max(light.fog_and_output.y - light.fog_and_output.x, 0.001), 0.0, 1.0);
    lit = mix(lit, light.fog_color.rgb, fog);

    if (light.fog_and_output.w > 0.5) {
        lit = encode_srgb(lit);
    }
    return vec4<f32>(lit, 1.0);
}
