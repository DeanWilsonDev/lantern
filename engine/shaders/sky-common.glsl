// The sky as a function, shared by sky.frag (which draws it) and lit.frag (which lights surfaces with it).
//
// A procedural sky is a colour you can compute from a direction, with no picture involved. This one is three colours: the
// zenith (straight up), the horizon (all the way round) and the ground (straight down, what the lower half of the view
// shows). Between them it blends by how high the direction points.
//
// The same function lights the scene. Image-based lighting asks, for a surface facing some way, "how much light arrives
// from the sky around that way?". Light arriving from a wide area is the sky blurred, so the `blur` argument widens the
// blends: 0 is the sharp sky you see, 1 is the whole-hemisphere average a rough or diffuse surface sees. A surface facing
// up gets zenith light, one facing sideways gets a horizon and ground mix, one facing down gets ground light. A shiny metal
// looks up the sky in its mirror direction with a blur that grows with its roughness, so it reflects the sky.

vec3 SkyGradient(vec3 direction, vec3 zenith, vec3 horizon, vec3 ground, float blur) {
    float elevation = direction.y;
    float upward = clamp(elevation, 0.0, 1.0);
    vec3 above = mix(horizon, zenith, pow(upward, mix(0.45, 1.0, blur)));
    // How far below the horizon line this direction is, with the line itself softened as blur grows.
    float softness = mix(0.01, 0.5, blur);
    float below_amount = smoothstep(-softness, softness, -elevation);
    return mix(above, ground, below_amount);
}
