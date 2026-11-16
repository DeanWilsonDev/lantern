# The sky and image-based lighting

This guide explains how Lantern draws a sky, and how that sky lights everything under it, so a gold ring looks gold
because of the blue above it. For where the sky pass sits in a frame, read `renderer.md`. For the exact function
signatures, read `game-api.md`.

## The problem: a flat ambient colour is lifeless

Before the sky, the renderer lit shadowed surfaces with two flat colours: one for surfaces facing up and one for surfaces
facing down. That is cheap, and it reads as a world lit by a studio. It has two weaknesses:

- A metal surface has almost no diffuse light, so it reflected nothing and looked like dark plastic.
- The backdrop was a single clear colour, so the horizon was a hard line and fog had nothing to fade into but a flat
  colour.

## The idea: one sky, used twice

The sky is a function: give it a direction and it returns a colour. Lantern's is three colours (zenith, horizon, ground)
blended by how far up or down the direction points, plus a small bright disc where the sun is.

```text
SkyGradient(direction) = blend of zenith, horizon and ground by direction.y
```

The function is used in two places:

1. **As a backdrop.** `sky.frag` runs a fullscreen triangle for each pixel and asks for the colour in that pixel's view
   direction. It is drawn after the meshes at the far depth, so only pixels no mesh covered pick it up.
2. **As light.** `lit.frag` asks the same function two questions for every lit pixel:
   - *Diffuse:* what is the average sky colour over the half-sphere the surface faces? The function answers it with a
     fully blurred lookup (`blur = 1`), which is nearly a blend of the three colours by the surface normal.
   - *Specular:* what colour does the sky have in the direction a mirror would reflect the view? The lookup is blurred by
     the surface's roughness, so a rough metal shows a soft smear of sky and a polished one shows a sharp horizon.

This is **image-based lighting**, with the "image" computed on the fly instead of stored as a cubemap. A cubemap would
need a bake step and a texture to load; the formula needs neither, and the tutorial reader can read the whole thing in
one file (`engine/shaders/sky-common.glsl`).

A metal surface also needs to know how strongly it reflects, which depends on the angle you look at it. `lit.frag` uses
an approximation of the standard lookup table (`EnvironmentBrdf`) so grazing angles reflect more.

## Using it from Nyx

```nyx
lantern:::sky:::SetColors("4285E0", "B3CCEB", "57524D");
lantern:::sky:::SetSunDisc(1.5f, 12.0f);
lantern:::sky:::SetLighting(1.0f, 0.6f);
lantern:::sky:::SetEnabled(true);
```

- `SetColors(zenith, horizon, ground)` takes `"RRGGBB"` strings.
- `SetSunDisc(radiusDegrees, strength)` sets the disc. A strength well above 1 makes it bloom.
- `SetLighting(strength, intensity)`: strength 0 keeps the old flat ambient pair and 1 replaces it with the sky;
  intensity scales how bright the sky's light is.
- `SetEnabled(false)` brings back the clear-colour backdrop and the flat pair.

The sky follows the sun: the disc sits in the direction of the world's directional light.

## Things worth knowing

- The sky is skipped for an orthographic camera (the editor's top and side views), because there is no view direction.
- The sky's three colours and the lighting numbers live in the frame's `RenderWorld.sky`, like the lights do.
- The sky costs one fullscreen pass and a handful of arithmetic per lit pixel, so it is on at every quality level.
- Fog fades towards the clear colour as before; set the clear colour near the horizon colour for a seamless distance.
- Not checked in the cloud: Metal output and frame time on a real GPU.

## Where the code is

| Piece | File |
| --- | --- |
| The sky function | `engine/shaders/sky-common.glsl` |
| The backdrop pass | `engine/shaders/sky.vert`, `sky.frag`, `engine/src/sdl-facade/renderer/GpuSky.nox` |
| The lighting | `engine/shaders/lit.frag` |
| The settings | `SkySettings` in `engine/src/renderer/RenderWorld.nox` |
| The Nyx functions | `engine/src/host/SkyApi.nox` |
