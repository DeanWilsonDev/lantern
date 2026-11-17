// Shared by the shaders that rebuild a position from the depth buffer (ambient occlusion and screen-space reflections).
//
// The depth buffer holds clip-space depth, a number from 0 (the near plane) to 1 (the far plane) that is not linear: it is
// packed tightly near the camera. The perspective matrix (see `Perspective` in Conventions.nox) makes
//
//   depth = D * (near - d) / d      where d is the distance in front of the camera and D = far / (near - far)
//
// so solving for d gives d = D * near / (depth + D), which this file does. With d known, the pixel's position in view space
// (x right, y up, the camera looking down -z) follows from the field of view: a pixel at the edge of the picture is
// `tan(half the field of view)` times d away from the centre line.
//
// The including shader declares a uniform block named `settings` whose member `projection_terms` is x: tan(half the horizontal
// field of view), y: tan(half the vertical), z: the near plane, w: D above.
float LinearDistance(float depth) {
    float depth_range = settings.projection_terms.w;
    return depth_range * settings.projection_terms.z / (depth + depth_range);
}

// A depth of (nearly) 1 is the far plane, where nothing was drawn.
bool IsBackground(float depth) {
    return depth > 0.99999;
}

// The view-space position of the pixel at `uv` whose depth is `depth`.
vec3 ViewPosition(vec2 uv, float depth) {
    float distance_ahead = LinearDistance(depth);
    vec2 ndc = vec2(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    return vec3(ndc.x * settings.projection_terms.x * distance_ahead, ndc.y * settings.projection_terms.y * distance_ahead, -distance_ahead);
}

// The uv a view-space position lands on, the reverse of `ViewPosition`.
vec2 ProjectToUv(vec3 view_position) {
    float distance_ahead = -view_position.z;
    vec2 ndc = vec2(view_position.x / (settings.projection_terms.x * distance_ahead), view_position.y / (settings.projection_terms.y * distance_ahead));
    return vec2(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
}
