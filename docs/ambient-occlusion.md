# Ambient occlusion

This guide explains why corners should be darker, how Lantern finds them from the depth buffer, and the settings. For where
the passes sit in a frame, read `renderer.md`.

## The problem: light that doesn't notice corners

Ambient light is the light that arrives from everywhere: the sky. An object in an open field sees all of the sky. The same
object pushed into a corner sees less, because the walls block some of it, so the corner should be darker. A renderer that
gives every surface the same ambient light makes objects look as if they float a little above the ground.

*Ambient occlusion* (AO) is "how much of the sky does this point miss?".

## The idea: ask the depth buffer

The renderer already keeps a depth buffer: the distance of the nearest surface at every pixel. That is enough to ask, for a
point on a surface, whether nearby space is filled:

1. **Rebuild the point.** From the pixel and its stored depth, work out the point's position in front of the camera
   (`ao-common.glsl` does the arithmetic). The same for its four neighbours gives the surface's normal.
2. **Scatter samples.** Pick twelve points in the half-sphere above the surface, within a radius.
3. **Test each one.** Find the pixel each sample lands on and read the depth stored there. If the stored surface is nearer
   than the sample, the sample is inside geometry, so that direction is blocked.
4. **Count.** The fraction blocked is the darkness.

Only the depth buffer is read, which is why it is called *screen-space* AO. A point hidden behind something on screen is
invisible to it, and that is its main limit.

Two fixes keep it clean:

- **A range check.** A blocker far in front of the sample (a pole in front of a far wall) shouldn't darken the wall, so a
  blocker counts less the farther it is from the point in depth.
- **A blur.** Each pixel turns its sample pattern by a different amount across a 4x4 block, so the result is noisy.
  `ao-blur.frag` averages the block, ignoring neighbours at a different depth so darkness doesn't smear over an edge.

## How the picture uses it

The composite pass multiplies the scene by the AO picture (1 where nothing is blocked, less in creases) before adding bloom,
so a glowing gem's halo isn't darkened. Lantern lights in a single pass, so the AO darkens direct sunlight in creases as
well as the ambient light. A renderer with a depth pre-pass would darken ambient light alone; that is a possible later
change.

AO isn't used for an orthographic camera (the editor's straight-on views).

## Using it from Nyx

```nyx
lantern::graphics::SetAmbientOcclusion(true, 0.6, 0.8);
```

The arguments are on or off, the radius in metres (how far a blocker can be and still darken a point), and the intensity
from 0 to 1. `SetQuality("low")` turns it off; `"medium"` and `"high"` turn it on.

| Setting | Default | Effect |
| --- | --- | --- |
| radius | 0.6 m | Bigger radius darkens wider areas but costs detail |
| intensity | 0.8 | How dark a fully blocked point gets |
| bias | 0.04 m | How far in front a surface must be to block; stops flat walls shading themselves |

## Cost

Two full-screen passes on frame-sized one-channel textures: 12 depth reads per pixel to estimate, 16 to blur. This is the
most expensive effect so far, so Low turns it off.

## Where the code is

| Piece | File |
| --- | --- |
| The estimate | `engine/shaders/ao.frag`, `ao-common.glsl` |
| The blur | `engine/shaders/ao-blur.frag` |
| The passes | `RecordAmbientOcclusion` in `engine/src/sdl-facade/renderer/GpuPost.nox` |
| The settings | `PostSettings.ambientOcclusion*` in `engine/src/renderer/RenderWorld.nox` |
| The Nyx function | `SetAmbientOcclusion` in `engine/src/host/GraphicsApi.nox` |
