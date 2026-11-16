# Colour grade

This guide explains the colour grade, the hook a game uses to give a chapter or a place its own look. For where it sits in
a frame, read `renderer.md`.

## What it is

Film and games get their mood in a final step: the finished picture is bent towards a look (washed out and cold for a bleak
chapter, warm and contrasty for a sunset) without touching any lighting. That step is the *colour grade*. Lantern's has four
controls, applied in this order to the picture after tone mapping:

| Control | What it does | Neutral value |
| --- | --- | --- |
| saturation | Blends between the pixel's grey and its colour. 0 is black and white, above 1 is vivid. | 1 |
| contrast | Scales how far each channel is from middle grey. Above 1 deepens darks and brightens lights. | 1 |
| tint | Multiplies the picture by a colour: a warm one warms everything, a cool one cools it. | white |
| vignette | Darkens towards the corners, drawing the eye to the middle. | 0 |

The defaults change nothing, so a game that never calls it looks exactly as it did.

## Where it happens

The composite pass tone-maps the HDR scene to 0 to 1, and then grades it, then sRGB-encodes it. Grading after tone mapping
means the numbers act on what the viewer sees: contrast 1.2 does 20% more than the picture shows, and not a different amount
in a bright scene and a dark one. The HUD, menus and debug lines are drawn after the composite, so a grade never tints them.

```text
HDR scene ─▶ occlusion, blobs, fog, shafts, glow ─▶ exposure ─▶ tone map ─▶ GRADE ─▶ sRGB ─▶ screen
```

The grade is a few multiplications per pixel and costs nothing noticeable, so quality levels don't touch it.

## Using it from Nyx

```nyx
lantern::graphics::SetColorGrade(0.75, 1.15, "FFEBD1", 0.4);
```

The arguments are saturation (0 to 3), contrast (0.2 to 3), the tint as an `"RRGGBB"` string, and the vignette (0 to 1).
A game can call it at a chapter change and the next frame has the new look; to ease between two looks, call it each frame
with values blended by hand. Calling it with `1, 1, "FFFFFF", 0` clears it.

## Where the code is

| Piece | File |
| --- | --- |
| The grade | `ColorGrade` in `engine/shaders/composite.frag` |
| The settings | `ColorGrade` and `PostSettings.grade` in `engine/src/renderer/RenderWorld.nox` |
| The Nyx function | `SetColorGrade` in `engine/src/host/GraphicsApi.nox` |
