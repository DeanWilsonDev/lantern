# HDR, tone mapping and bloom

This guide explains how Lantern draws light that is brighter than white, how that light becomes a picture, and how to
make a gem glow. For where the passes sit in a frame, read `renderer.md`. For the exact function signatures, read
`game-api.md`.

## The problem: a screen can't show "brighter than white"

A display shows brightness from 0 (black) to 1 (white). A lit scene doesn't stay in that range. A white wall in full sun
is about 1. A lamp is 5. A gem's glow can be 20. If the renderer wrote those numbers straight to an ordinary 8-bit
image, everything above 1 would be cut off, and the wall, the lamp and the gem would all be the same flat white.

So the renderer draws in **high dynamic range** (HDR): the scene goes into a 16-bit float texture (RGBA16F), which keeps
the real values. Three more steps then decide how that light becomes a picture. They all happen after the scene is drawn,
in `GpuPost.nox`.

```text
scene pass ──▶ scene texture (HDR) ──▶ bloom ──▶ glow texture
                      │                              │
                      └────────────▶ composite ◀─────┘ ──▶ screen
                                       add glow, exposure, tone map, sRGB
```

The debug lines and the 2D overlay (HUD, menus) are drawn afterwards, straight onto the screen image. They are not
tone mapped, so a menu colour is the colour you wrote.

## Exposure

Exposure is a brightness knob applied to the light before the curve below. `SetExposure(2.0f)` shows the scene twice as
bright, `0.5f` half as bright. A camera does the same with its shutter and aperture. Raise it for a dark cave and lower
it for a sunny field.

```nyx
lantern:::graphics:::SetExposure(1.0f);
```

## Tone mapping

Tone mapping squeezes any brightness into the 0 to 1 a display shows. A good curve keeps dark and middle colours close to
how they were and rolls bright ones off smoothly towards white, instead of clipping them.

| Name | What it does | When to use it |
| --- | --- | --- |
| `"neutral"` (default) | Leaves every colour below about 0.8 as it is and only rolls off brighter light, losing a little saturation as it goes. This is the Khronos PBR Neutral curve. | Pastel, stylised scenes like the platformer: the colours you painted are the colours you get. |
| `"aces"` | A filmic S-curve (an ACES fit): a lifted toe, strong contrast in the middle and a long shoulder. | A cinematic or night scene. It brightens and desaturates pastels, so lights and colours that were tuned without it need re-balancing. |
| `"none"` | No curve: values above 1 are cut off flat. | Comparing with the old look, or a game that stays inside 0 to 1 on purpose. |

```nyx
lantern:::graphics:::SetToneMapper("aces");
```

The default is the neutral curve because the platformer was lit for a clamped target: its colours barely change, and
only the light that was being clipped now rolls off. ACES is one call away.

## Bloom

Bloom is the glow around bright things. A lens and an eye both scatter a little of a bright light's energy over its
surroundings, and bloom imitates that: blur the bright parts of the picture and add the blur back.

The renderer does it in two halves (`bloom-downsample.frag`, `bloom-upsample.frag`):

1. **Shrink.** Five full-screen passes, each reading a texture and writing one half as wide and half as tall. The first
   also keeps only the brightness above the **threshold**. Each smaller texture is a wider, blurrier view of the bright parts.
2. **Grow.** Four passes run back up the chain. Each reads a small texture, blurs it a little and *adds* it to the next
   larger one. The glow of every width ends up in the largest.

The composite pass then adds that glow, times the **strength**, to the scene before the curve.

```nyx
lantern:::graphics:::SetBloom(true, 0.1f, 1.5f);
```

- **Threshold** is where glow starts, in scene light units (1 is a white wall in full sun). The default 1.5 means a
  sunlit wall doesn't glow but a gem does. Lower it and ordinary bright surfaces start to haze.
- **Strength** is how much glow is added back. 0.05 is a faint halo, 0.2 a dreamy one.

### Making something glow

Bloom only picks out light above the threshold, so a glowing thing has to be brighter than 1. The material's emissive
colour is written as `"RRGGBB"` and can't exceed 1, so `SetGlow` multiplies it:

```nyx
lantern:::world:::SetSurface(entity, 0.0f, 0.2f, "3FD0FF");
lantern:::world:::SetGlow(entity, "3FD0FF", 4.0f);
```

The platformer's gems do exactly this (`game-entities/gem.nyx`, `GLOW_STRENGTH`). Note that emissive light doesn't light
its neighbours: bloom is a halo in the picture, not a light in the world. For that, add a lamp.

## Quality

Bloom costs nine small full-screen passes. The cloud has no real GPU, so the frame-time cost on hardware is still to be
measured. `SetQuality` is the one switch for what the renderer spends on looks:

| Level | Bloom |
| --- | --- |
| `"low"` | off |
| `"medium"` | on |
| `"high"` | on |

Later effects join the same levels, so a game that calls `SetQuality` once keeps working as they arrive. Individual calls
such as `SetBloom` may follow it and win. With bloom off, the renderer still draws in HDR and tone maps; only the
glow passes are skipped.

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| The whole picture has a haze | Bloom's threshold is below the brightness of ordinary surfaces. Raise it, or lower the sun's colour. |
| Gems don't glow | The glow strength is 1 or less, so it never passes the threshold. Raise it with `SetGlow`. |
| The picture is too washed out or too contrasty | Try the other tone mapper, then adjust the exposure. |
| A colour looks different from what was written in a menu | It shouldn't: the overlay isn't tone mapped. Check the colour of the world behind it instead. |

## Where the code lives

| File | What it holds |
| --- | --- |
| `engine/shaders/fullscreen.vert` | The one triangle every post pass draws |
| `engine/shaders/bloom-downsample.frag`, `bloom-upsample.frag` | The two halves of bloom |
| `engine/shaders/composite.frag` | Glow, exposure, tone mapping and sRGB encoding |
| `engine/src/sdl-facade/renderer/GpuPost.nox` | The textures and the passes |
| `engine/src/renderer/RenderWorld.nox` | `PostSettings`, `ToneMapper`, `QualityLevel` |
| `engine/src/host/GraphicsApi.nox` | `lantern:::graphics` for Nyx |
