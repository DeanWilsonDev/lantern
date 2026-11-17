# Graphics effects and the quality setting

This page is the map of the renderer's visual effects and how `lantern:::graphics:::SetQuality(level)` turns them up and
down. Each effect has its own guide; this page says which exist and what each quality level does. `quality-settings.md` is the
guide to choosing a level, tuning one effect and finding what to turn down; `renderer.md` shows where each effect runs in the
frame.

## The effects

| Effect | Guide | Cost |
| --- | --- | --- |
| HDR scene, bloom, tone mapping | `bloom-and-tone-mapping.md` | a few full-screen passes |
| Procedural sky and sky lighting | `sky-and-image-based-lighting.md` | one draw and a small lookup in the lit shader |
| Anti-aliasing (FXAA) | `anti-aliasing.md` | one full-screen pass |
| Ambient occlusion | `ambient-occlusion.md` | two full-screen passes; the costliest effect |
| Height fog and light shafts | `fog-and-light-shafts.md` | a few ALU ops; shafts march the sun's direction |
| Soft shadows and cascade blending | `shadow-softness-and-cascade-blending.md` | more shadow map taps per pixel |
| Blob shadows | `blob-shadows.md` | a short loop in the composite pass |
| Colour grade | `colour-grade.md` | a few ALU ops |
| Particles | `particles.md` | one line per live particle |
| Normal maps | `normal-maps.md` | one sample for draws that have a map |
| Wet surfaces and puddles | `wet-surfaces.md` | a few ALU ops and a noise lookup for wet draws |

## What each quality level sets

| | low | medium | high |
| --- | --- | --- | --- |
| Bloom | off | on | on |
| Anti-aliasing | off | on | on |
| Ambient occlusion | off | on | on |
| Light shafts | off | on | on |
| Shadow filter samples | 4 | 12 | 20 |
| Particle budget | 64 | 256 | 512 |

Height fog, blob shadows, the colour grade, the sky, wet surfaces and normal maps are cheap and are not changed by the level. The default
is medium. Each effect's own call, such as `SetBloom` or `SetAntiAliasing`, tunes or overrides one effect and may be called
before or after `SetQuality`; a later `SetQuality` sets every row above again.

The world editor's Graphics settings tab sets the same effects for its viewport; see `docs/editor.md`.

## Requirements

The scene is drawn into a 16-bit float colour target so light brighter than 1 survives to the tone map, so the renderer
needs a GPU that can render to RGBA16F. Every current desktop GPU can; one that can't makes the renderer refuse to start
instead of drawing wrongly.

## Not measured in the cloud

The cloud machine has only a software Vulkan driver (lavapipe), so Metal output and frame times on a real GPU have not been
checked. The levels are chosen by what each effect costs, not by measurement; check 60 fps on a real GPU before settling on
a default for a game.
