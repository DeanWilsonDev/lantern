# Anti-aliasing

This guide explains why edges look jagged, how Lantern smooths them, and how to turn the smoothing off. For where the pass
sits in a frame, read `renderer.md`.

## The problem: a pixel is one colour

The renderer decides, for each pixel, whether a triangle covers it. A pixel that an edge cuts in half is drawn as the
triangle's colour or the background's, never as a mix. Along a diagonal edge that makes a staircase.

```text
without                with
██                     ██
  ██                   ▓▒██
    ██                   ▓▒██
```

## The choice: FXAA

There are three common fixes.

| Method | How it works | Why Lantern doesn't use it |
| --- | --- | --- |
| MSAA | Samples coverage several times per pixel while drawing | The lines and overlay are drawn after the composite onto a one-sample image, and the depth buffer they test against would have to be resolved. It also makes every later effect that reads depth (ambient occlusion next) harder. |
| TAA | Jitters the camera and blends frames | Needs motion vectors and history buffers, and ghosts on fast movement, which a platformer has a lot of. |
| **FXAA** | Looks for edges in the finished picture and blurs along them | One pass, no extra targets besides one, no change to how anything is drawn. |

FXAA's weakness is that it only sees the finished pixels, so it cannot recover detail that was never drawn (a thin wire
narrower than a pixel) and it softens fine textures slightly. For a small platformer with big shapes that is a good trade.

## How it works

The pass is `engine/shaders/fxaa.frag`, and it runs on the picture the composite pass made (exposed, tone mapped and
sRGB-encoded):

1. Compare the brightness (luma) of a pixel and its four diagonal neighbours. If they are close, copy the pixel. Most of the
   frame stops here.
2. Otherwise the way brightness changes says which way the edge runs. Take samples along it.
3. Average them. If the wide average strays outside the neighbourhood's brightness, use the narrow one.

Running after tone mapping matters: a bright emissive edge looks different once it is tone mapped, so FXAA smooths the edge
the viewer sees.

The debug lines and the overlay are drawn after FXAA on top of the screen image, so text and menus stay as sharp as written.

## Using it from Nyx

```nyx
lantern::graphics::SetQuality("low");
lantern::graphics::SetAntiAliasing(true);
```

`SetQuality("low")` turns it off, `"medium"` and `"high"` turn it on, and `SetAntiAliasing(enabled)` overrides that for the
one effect. It is on by default.

## Cost

When it is on, the composite writes a texture of its own (the screen's size and format) and FXAA reads it: one extra
full-screen pass and one extra texture. Nine of ten pixels return after five reads.

## Where the code is

| Piece | File |
| --- | --- |
| The filter | `engine/shaders/fxaa.frag` |
| The pass and its texture | `RecordAntiAlias`, `PostTargets.ldrTexture` in `engine/src/sdl-facade/renderer/GpuPost.nox` |
| The setting | `PostSettings.antiAliasingEnabled` in `engine/src/renderer/RenderWorld.nox` |
| The Nyx function | `SetAntiAliasing` in `engine/src/host/GraphicsApi.nox` |
