// Fast approximate anti-aliasing (FXAA), the last full-screen pass before the overlay.
//
// Jagged edges happen because each pixel is either inside a triangle or outside it: a pixel the edge cuts in half is drawn
// as one colour or the other, and the staircase is the result. Multisampling fixes that while drawing the triangles. This
// pass fixes it afterwards, from the finished picture alone, in three steps:
//
//   1. Look at the luma (brightness) of the pixel and its four diagonal neighbours. If they are all about the same there
//      is no edge, so copy the pixel and stop. Most pixels stop here, which is why the pass is cheap.
//   2. Otherwise the way luma changes across the four neighbours says which direction the edge runs. Step along that
//      direction, a third and two thirds of a span each way.
//   3. Average the samples taken along the edge. Take the short average (two samples) when the long one (four) strays
//      outside the neighbourhood's luma range, because that means the long one reached something that isn't this edge.
//
// It runs on the tone-mapped picture, after the composite pass, so the edges it sees are the ones the viewer sees (a
// bright emissive edge looks different after tone mapping than before). It blurs fine texture detail a little, which is the
// price of needing no extra geometry samples. The method is Timothy Lottes' FXAA, in the compact form that fits one page.
#version 450

layout(set = 2, binding = 0) uniform sampler2D scene_image;

// Slot 0: xy: the size of one pixel in uv units (1 / width, 1 / height). zw unused.
layout(set = 3, binding = 0) uniform FxaaUniforms {
    vec4 texel_size;
} settings;

layout(location = 0) in vec2 source_uv;
layout(location = 0) out vec4 output_color;

float Luma(vec3 color) {
    return dot(color, vec3(0.299, 0.587, 0.114));
}

void main() {
    vec2 texel = settings.texel_size.xy;
    vec3 middle = texture(scene_image, source_uv).rgb;
    vec3 north_west = texture(scene_image, source_uv + vec2(-1.0, -1.0) * texel).rgb;
    vec3 north_east = texture(scene_image, source_uv + vec2(1.0, -1.0) * texel).rgb;
    vec3 south_west = texture(scene_image, source_uv + vec2(-1.0, 1.0) * texel).rgb;
    vec3 south_east = texture(scene_image, source_uv + vec2(1.0, 1.0) * texel).rgb;

    float luma_middle = Luma(middle);
    float luma_north_west = Luma(north_west);
    float luma_north_east = Luma(north_east);
    float luma_south_west = Luma(south_west);
    float luma_south_east = Luma(south_east);

    float luma_lowest = min(luma_middle, min(min(luma_north_west, luma_north_east), min(luma_south_west, luma_south_east)));
    float luma_highest = max(luma_middle, max(max(luma_north_west, luma_north_east), max(luma_south_west, luma_south_east)));
    float contrast = luma_highest - luma_lowest;
    if (contrast < max(0.0312, luma_highest * 0.125)) {
        output_color = vec4(middle, 1.0);
        return;
    }

    vec2 edge_direction;
    edge_direction.x = -((luma_north_west + luma_north_east) - (luma_south_west + luma_south_east));
    edge_direction.y = (luma_north_west + luma_south_west) - (luma_north_east + luma_south_east);

    float direction_reduce = max((luma_north_west + luma_north_east + luma_south_west + luma_south_east) * 0.25 * 0.125, 1.0 / 128.0);
    float direction_scale = 1.0 / (min(abs(edge_direction.x), abs(edge_direction.y)) + direction_reduce);
    edge_direction = clamp(edge_direction * direction_scale, vec2(-8.0), vec2(8.0)) * texel;

    vec3 short_average = 0.5 * (
        texture(scene_image, source_uv + edge_direction * (1.0 / 3.0 - 0.5)).rgb +
        texture(scene_image, source_uv + edge_direction * (2.0 / 3.0 - 0.5)).rgb);
    vec3 long_average = short_average * 0.5 + 0.25 * (
        texture(scene_image, source_uv + edge_direction * -0.5).rgb +
        texture(scene_image, source_uv + edge_direction * 0.5).rgb);

    float luma_long = Luma(long_average);
    if (luma_long < luma_lowest || luma_long > luma_highest) {
        output_color = vec4(short_average, 1.0);
    } else {
        output_color = vec4(long_average, 1.0);
    }
}
