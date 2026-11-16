# Soft shadow edges and cascade blending

This guide explains how Lantern softens the edge of a shadow and hides the seams between the sun's shadow cascades. For how
shadow maps work at all, read `lighting-and-shadows.md` first.

## Hard edges and aliasing

A shadow map is a picture of the scene's depth from the light. To ask "is this pixel in shadow?", the shader compares the
pixel's depth, seen from the light, with the depth stored in the map. The answer is yes or no, so a shadow's edge follows
the map's square texels and looks like a staircase. Making the answer a blend of many nearby answers fixes it. That blend is
called *percentage-closer filtering*.

## The filter

`FilteredShadow` in `lit.frag` takes a number of lookups (4, 12 or 20 by quality level) spread over a disc, averages them,
and returns how much light gets through. Two details matter:

- **A sunflower spiral.** Each lookup is turned by the golden angle (about 137.5 degrees) from the last, and its distance
  from the centre grows with the square root of its number. That covers a disc evenly with any number of points.
- **A turn per pixel.** The whole spiral is rotated by a different angle in each pixel (interleaved gradient noise). A fixed
  pattern leaves visible bands where the lookups line up; a rotation turns the bands into fine noise that the eye ignores.

`softness` is the disc's radius in shadow-map texels. 1 is nearly as sharp as the map allows. 2, the default, gives a soft
edge. 5 is very soft, and contact points blur with it.

The filter's width is fixed in texels, not in metres, so a soft edge looks the same width on a near and a far shadow only if
the map covers the same world per texel. A true penumbra (wider the farther the shadow is from what casts it) needs the
distance to the blocker, which needs to read raw depths, which Lantern's comparison sampler can't. That is a possible later
step.

## Cascades and their seams

The sun's shadow is split into three cascades: near, middle and far slices of the view, each with a map of the same size, so
nearer slices have sharper shadows. At the boundary between two slices the shadow's detail changes suddenly, and where the
filter sees a different texel size, a thin line appears.

With `blendCascades` on, the last 12% of each cascade also looks the pixel up in the next one, and the two answers are mixed
with a smooth curve. The last cascade fades out into "no shadow" the same way instead of stopping with an edge.

Stability (the shadow edges not crawling as the player moves) was already handled: each cascade's map is snapped to whole
texels, so the same world position always lands on the same texel.

## Using it from Nyx

```nyx
lantern:::lights:::SetShadowSoftness(2.0f, 12, true);
lantern:::graphics:::SetQuality("high");
```

`SetShadowSoftness(softness, samples, blendCascades)`: softness 0.5 to 6 texels, 1 to 32 samples. `SetQuality` sets the sample
count: low 4, medium 12, high 20; softness and blending stay as the game set them.

## Cost

Each shadowed pixel reads the sun's map `samples` times, plus the same again in the blend zone, and does the same for each
shadowed local light. Twelve is a good balance; Low uses 4.

## Where the code is

| Piece | File |
| --- | --- |
| The filter and the blend | `FilteredShadow`, `CascadeShadow`, `SunShadow` in `engine/shaders/lit.frag` |
| The settings | `ShadowSettings` in `engine/src/renderer/RenderWorld.nox` |
| Packing | `PackShadowUniforms` in `engine/src/sdl-facade/renderer/GpuShadows.nox` |
| The Nyx function | `SetShadowSoftness` in `engine/src/host/LightsApi.nox` |
