# Quality settings guide

How a game chooses how much the renderer spends on looks, and how to find the setting to turn down when a frame is too
slow. `graphics-effects.md` is the map of the effects, and `renderer.md` explains where each runs in the frame. This page is
for choosing.

## The one switch

```nyx
lantern:::graphics:::SetQuality("medium");
```

`"low"`, `"medium"` or `"high"`. Anything else is a script error naming the three. The default, with no call at all, is
medium, which is what the platformer runs. A game calls it once from `OnStart`, or again when the player changes a setting.

| | low | medium (default) | high |
| --- | --- | --- | --- |
| Bloom | off | on | on |
| Anti-aliasing (FXAA) | off | on | on |
| Ambient occlusion | off | on | on |
| Light shafts | off | on | on |
| Shadow filter lookups per pixel | 4 | 12 | 20 |
| Particle budget (live particles) | 64 | 256 | 512 |

Medium and high differ only in the shadow filter and the particle budget today; the levels are separate so that a later
effect can join high alone.

**Not changed by the level**, because they are cheap or are a look and not a cost: height fog, blob shadows, the sky and
its lighting, the colour grade, normal maps, exposure and the tone mapper, and each effect's own numbers (bloom strength,
occlusion radius, shadow map sizes and distance, softness).

## Tuning one effect

Every effect also has its own call. Calls like these tune or override one effect, and they may come before or after
`SetQuality`. Order matters in one way: **`SetQuality` sets every row of the table above again**, so a game that wants
"high, but no occlusion" calls `SetQuality("high")` first and `SetAmbientOcclusion(false, ...)` after it.

| Call | Arguments | Notes |
| --- | --- | --- |
| `lantern:::graphics:::SetBloom(enabled, strength, threshold)` | strength 0 and up (0.1 default, 0.04 faint, 0.2 hazy); threshold 0 and up in scene units (1.5 default) | 9 passes when on |
| `lantern:::graphics:::SetAntiAliasing(enabled)` | on by default | 1 pass and one texture |
| `lantern:::graphics:::SetAmbientOcclusion(enabled, radius, intensity)` | radius in metres (0.6), intensity 0 to 1 (0.8) | the costliest effect; ignored for an orthographic camera |
| `lantern:::atmosphere:::SetLightShafts(enabled, strength, decay)` | strength 0.35, decay 0.95 | 24 depth reads per pixel; only when the sun is in front of the camera |
| `lantern:::lights:::SetShadowQuality(enabled, sunMapSize, localTileSize, distance)` | sizes 256 to 4096 pixels (1024 and 512), distance in metres (60) | `enabled = false` turns every shadow off |
| `lantern:::lights:::SetShadowSoftness(softness, samples, blendCascades)` | softness 0.5 to 6 texels (2), samples 1 to 32 (12), blend on | `SetQuality` overwrites `samples` only |
| `lantern:::lights:::SetSunShadows(enabled)`, `lantern:::lights:::SetShadows(light, enabled)` | | a light with shadows off costs no tile |
| `lantern:::graphics:::SetBlobShadows(enabled)` | | cheap; a loop of at most 8 per pixel |
| `lantern:::graphics:::SetExposure(exposure)`, `lantern:::graphics:::SetToneMapper(name)` | exposure 1 is neutral; `"none"`, `"neutral"` (default) or `"aces"` | a look, not a cost |
| `lantern:::graphics:::SetColorGrade(saturation, contrast, tint, vignette)` | saturation 0 to 3, contrast 0.2 to 3, `"RRGGBB"`, vignette 0 to 1; `1, 1, "FFFFFF", 0` is neutral | a look, not a cost |

Values outside a range are clamped, not rejected.

### Examples

A game that always wants the best picture it can get, with ambient occlusion kept light:

```nyx
lantern:::graphics:::SetQuality("high");
lantern:::graphics:::SetAmbientOcclusion(true, 0.4f, 0.5f);
```

A cheap mode for a laptop, which keeps the sky, fog and grade (the look) and drops the passes:

```nyx
lantern:::graphics:::SetQuality("low");
lantern:::lights:::SetShadowQuality(true, 512, 256, 40.0f);
```

## Finding what to turn down

Turn on the debug HUD (F1, or `LANTERN_DEBUG_HUD=1`) and turn off vertical sync (`LANTERN_VSYNC=0`) so the frame time isn't
rounded to the display. `Frame/Average ms` and `Frame/Worst ms` are the numbers to watch; 60 fps is 16.7 ms. Then change one
thing at a time in this order, which is roughly what each costs, highest first:

1. **Ambient occlusion** off. Two full-screen passes with 12 and 16 depth reads per pixel.
2. **Bloom** off. Nine passes, but at half the frame's size and smaller, so cheaper than its count suggests.
3. **Shadows**: a smaller `distance` first (fewer casters per cascade), then smaller map sizes, then fewer filter samples,
   then fewer shadowed lamps. Each tile redraws the casters inside it.
4. **Light shafts** off. 24 depth reads per pixel.
5. **Anti-aliasing** off. One pass.
6. **Particle budget**: `SetQuality("low")` caps it at 64.

`SetQuality("low")` does 1, 2, 4 and 5 in one call and cuts the shadow filter to 4 lookups.

These are counts of work. They haven't been timed on a real GPU: the cloud machine used for development has only a software
Vulkan driver. If the order is wrong on your hardware, change it and tell the board (`known-gaps.md`).

## Hardware the renderer needs

The scene is drawn into a 16-bit float colour texture (RGBA16F) so light brighter than 1 survives to the tone map. Every
current desktop GPU can render to it; one that can't makes the renderer refuse to start, with a message, instead of drawing
wrongly. The default shadow maps (1024 by 3 for the sun, 6 by 2 tiles of 512 for lamps) are small enough for any GPU.

## Where the code is

| Piece | File |
| --- | --- |
| The levels and what each sets | `QualityLevel` and each settings struct's `WithQuality` in `engine/src/renderer/RenderWorld.nox` |
| The Nyx calls | `engine/src/host/GraphicsApi.nox`, `LightsApi.nox`, `AtmosphereApi.nox` |
| The particle budget | `GraphicsApi.nox` (`SetQuality`) and `engine/src/renderer/Particles.nox` |
