// The 2D overlay (HUD, menus): pixel-space triangles, alpha-blended, sampling the font atlas.

struct ScreenUniforms {
    // xy: drawable size in pixels, w: 1 when the shader must sRGB-encode its output.
    size_and_flags: vec4<f32>,
};

@group(1) @binding(0) var<uniform> screen: ScreenUniforms;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) encode_srgb: f32,
};

@vertex
fn vertex_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    // Pixels (origin top-left, +y down) to clip space (+y up).
    let ndc = vec2<f32>(
        input.position.x / screen.size_and_flags.x * 2.0 - 1.0,
        1.0 - input.position.y / screen.size_and_flags.y * 2.0,
    );
    output.clip_position = vec4<f32>(ndc, 0.0, 1.0);
    output.uv = input.uv;
    output.color = input.color;
    output.encode_srgb = screen.size_and_flags.w;
    return output;
}

@group(2) @binding(0) var atlas_texture: texture_2d<f32>;
@group(2) @binding(1) var atlas_sampler: sampler;

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(atlas_texture, atlas_sampler, input.uv);
    var color = texel.rgb * input.color.rgb;
    if (input.encode_srgb > 0.5) {
        color = pow(color, vec3<f32>(1.0 / 2.2));
    }
    return vec4<f32>(color, texel.a * input.color.a);
}
