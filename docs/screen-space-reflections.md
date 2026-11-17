# Screen-space reflections

Wet ground, puddles and glossy materials reflect what is on screen: buildings, lamps, the player and the companion. Where
the reflection cannot find anything on screen it falls back to the sky and the lamp highlights, which is what the lit
shader already drew before this existed, so a miss never leaves a hole.

## How it works

1. **A surface buffer.** The lit, skinned lit and sky shaders write a second colour target as well as the HDR picture:
   the surface normal (octahedral, in red and green), the roughness (blue) and the reflectance at normal incidence
   (alpha). It is cleared to a flat, non-reflective value. Skinned characters share the lit fragment shader, so they
   write it too and appear in reflections like any other mesh.
2. **A reflection pass** (`ssr.frag`, run from `GpuReflections.nox` after the scene pass). Only pixels that are glossy
   enough (roughness under about 0.55, with some reflectance) do any work; every other pixel copies the scene and
   returns. A glossy pixel reflects its view ray about its normal and marches the ray across the screen in steps, reading
   the depth buffer, with perspective-correct interpolation and a thickness that grows with distance. When the ray passes
   behind the depth buffer it takes a few binary-search steps to land on the surface.
3. **The result.** The pass writes the scene plus the difference between the reflected colour and the sky reflection the
   lit shader had already added, weighted by a confidence. Bloom and the composite read that picture.

The confidence fades to zero with roughness (0.25 to 0.55), near the screen edges, as the ray travels towards the
maximum distance, when the hit faces away from the mirror, and with fog. Zero confidence is exactly the old picture.

## Controls

| Control | Effect |
| --- | --- |
| `lantern:::graphics:::SetQuality("low" / "medium" / "high")` | low turns reflections off, medium marches 24 steps with 4 refinements, high 48 steps with 6 |
| `lantern:::graphics:::SetScreenSpaceReflections(enabled, intensity, maxDistance)` | switch on or off, scale the strength (1 is physical, 0 shows the sky only) and cap the ray length in metres (default 40) |
| `LANTERN_SSR=0` or `1` | force reflections off, or on at medium cost even at low quality, for comparisons |
| Editor, Settings, Graphics, Screen-space reflections | the same switch in the viewport; the quality levels set it |

With reflections off the surface target is not stored, so the scene pass costs no extra bandwidth.

## Cost

One full-screen pass at the frame's size and one extra RGBA8 target written by the scene pass. Non-glossy pixels return
at once. The work is counted, not timed: the development machine has only a software Vulkan driver, so no real-GPU frame
time exists yet. Lower quality or `maxDistance` first if a machine is slow.

## Limits

- Only what is on screen can be reflected. A thing behind the camera or off the edge falls back to the sky.
- The underside of an object is not in the depth buffer, so a floating object reflects only its visible front, and its
  reflection can end short of where a mirror would show it.
- A reflection fades to nothing where the ray leaves the screen, so a bright strip can meet the sky reflection at an edge.
- Rays that hit a thin thing show some jitter noise.
- There is no tint for metal yet, and reflections do not reflect other reflections.
- The pass runs only for perspective cameras.

The Metal shader is generated from the same SPIR-V (`mist run build-shaders`) and was checked to be well formed output,
but has not been compiled with Apple's compiler.
