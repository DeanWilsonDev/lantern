# Fog and light shafts

This guide explains the air Lantern puts between the camera and the world: fog that is thicker low down, and streaks of
sunlight through it. For where the work sits in a frame, read `renderer.md`.

## Why a second kind of fog

The renderer already has distance fog: past `fogStart` things fade to the horizon colour until `fogEnd`, so the far plane's
cut-off is hidden. That fog only knows how far away something is. Real haze also depends on height: it pools in valleys and
thins out above the rooftops. A road at the bottom of a canyon fades faster than a tower top the same distance away.

## Height fog

The air's density falls off exponentially with height above a base:

```text
density(y) = base_density * exp(-falloff * (y - base))
```

A ray from the camera to a surface loses light in proportion to the density added up along it, its *optical depth*. For an
exponential density that sum has a closed form, so the shader needs no marching:

```text
optical = base_density * exp(-falloff * (camera_y - base)) * (1 - exp(-falloff * ray_y * length)) / (falloff * ray_y)
fog     = 1 - exp(-optical)
```

`ray_y` is how much the ray climbs, and `length` is how far it travels. A level ray reduces to `density * length`. The
result is the fraction of the fog colour to mix in. Looking towards the sun the fog is brighter (`fogSunScattering`), as
real haze glows around the sun.

The composite pass does this, not the lit shader. It reads the depth buffer to get each pixel's distance and rebuilds the
ray from the camera's right, up and forward vectors (the same basis the sky uses). Sky pixels are left alone: the sky
already blends into the horizon colour.

## Light shafts

When the sun is low behind trees, buildings or hills, the haze lights up in streaks that fan out from the sun. Lantern fakes
this on the screen:

1. Find where the sun is on the screen (not drawn if it is behind the camera, with a smooth fade).
2. For each pixel, walk 24 steps towards that point.
3. At each step, read the depth buffer. If it is the far plane, the step saw open sky, which means sunlight reaches the
   camera along this line. Each step counts a little less than the last (`lightShaftDecay`).
4. The share of steps that saw sky, times the strength, times how directly the pixel looks at the sun, is sunlight added to
   the pixel.

A pixel whose line to the sun is blocked by a pillar sees no sky on the way and gets nothing, which is the dark gap between
shafts. Only the depth buffer is read, so it needs no extra shadow work, and the sun's disc doesn't have to be on screen.

## Using it from Nyx

```nyx
lantern:::atmosphere:::SetHeightFog(true, 0.008f, 0.1f, 0.0f);
lantern:::atmosphere:::SetFogColor("9EB8D9", 0.5f);
lantern:::atmosphere:::SetLightShafts(true, 0.35f, 0.95f);
```

- `SetHeightFog(enabled, density, heightFalloff, baseHeight)`: density is the fraction of light lost per metre at the base
  height (0.01 is a light haze), falloff is per metre going up, and the base is in metres.
- `SetFogColor(color, sunScattering)`: an `"RRGGBB"` string, and how much brighter fog is towards the sun.
- `SetLightShafts(enabled, strength, decay)`: decay near 1 gives longer shafts.

`SetQuality("low")` turns the shafts off. Height fog is arithmetic only and stays on at every level. Neither is used for an
orthographic camera.

## Cost

Height fog is a handful of arithmetic per pixel plus one depth read. Shafts are 24 more depth reads per pixel, so Low turns
them off. They only run when the sun is in front of the camera.

## Where the code is

| Piece | File |
| --- | --- |
| The fog and shafts | `engine/shaders/composite.frag` |
| Packing the camera and settings | `PackCompositeUniforms` in `engine/src/sdl-facade/renderer/GpuSky.nox` |
| The settings | `AtmosphereSettings` in `engine/src/renderer/RenderWorld.nox` |
| The Nyx functions | `engine/src/host/AtmosphereApi.nox` |
