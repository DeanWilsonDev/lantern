# Lighting and shadows

This guide shows how to light a Lantern scene from game code: what each kind of light does, which numbers to
change, and how to keep a scene at 60 fps. For how the renderer draws the result, read `renderer.md`. For the
exact function signatures, read `game-api.md`.

Everything here is Nyx. The engine owns the lights; the game gets an `i64` id for each one it adds and passes the
id back to change it.

## The ingredients

| Ingredient | What it is | Nyx |
| --- | --- | --- |
| Sun | One light, everywhere, in one direction. The only light that shades the whole scene evenly. | `lantern:::lights:::SetSun` |
| Ambient | Light from the sky and the ground. Faces pointing up get the sky colour, faces pointing down get the ground colour. | the last two arguments of `SetSun` |
| Point light | A lamp: shines in every direction from a position, fading to nothing at its range. | `lantern:::lights:::AddPoint` |
| Spot light | A lamp with a cone, like a torch or a stage light. | `lantern:::lights:::AddSpot` |
| Material | How a surface answers light: metallic, roughness and an emissive glow. | `lantern:::world:::SetSurface` |

Colours are `"RRGGBB"` strings. A light's *strength* is a plain number where 1 means a white matte surface facing
the light and close to it shows as white. A strength of 6 on a lamp with a range of 12 metres lights a small
courtyard.

## A first scene

```nyx
sunDirection: lantern:::math:::Vec3 = { x: -0.4f, y: -1.0f, z: -0.5f };
lantern:::lights:::SetSun(sunDirection, "FFF1D6", "5F7FA8", "3A3328");
lantern:::lights:::SetSunShadows(true);
```

`direction` is the way the light travels, so a negative `y` shines downward. A sun low in the sky (a small `y`
next to `x` and `z`) throws long shadows; a sun straight overhead (`x` and `z` near zero) throws short ones.

The ambient pair keeps shaded sides readable. A cool sky colour above and a warm, dark ground colour below look more
natural than one grey. If shadows are too black, raise the ambient colours, not the sun.

`games/platformer/settings/lighting-settings.nyx` is a working example with a sun and two lamps.

## Lamps

```nyx
lampPosition: lantern:::math:::Vec3 = { x: 0.0f, y: 4.0f, z: 0.0f };
lamp := lantern:::lights:::AddPoint(lampPosition, "FFB866", 6.0f, 12.0f);
lantern:::lights:::SetShadows(lamp, true);
```

- `range` is a hard limit as well as a fade: past it the light is exactly zero, which is what lets the renderer skip
  the lamp for pixels it can't reach. Keep it as small as the look allows.
- `Move`, `Aim`, `SetColor` and `Remove` change a lamp by its id. A flickering fire is `SetColor` each frame with a
  strength that wobbles.
- A spot light takes a target point and two angles. Inside `innerDegrees` it is full strength, and it fades to
  nothing by `outerDegrees`.

## One set of lights per level

Lights stay in the engine until they are removed, so a game with more than one level must take the old level's lamps
down when it loads the next. The platformer does this in `game-systems/level-lighting.nyx`: `LevelLighting.Apply`
removes the lamps it added last time, sets the sun and adds the lamps `LightingSettings:::LampPositions` lists for
the level being loaded. A level with no entry there is lit by the sun alone. Keep the ids `AddPoint` and `AddSpot`
return so the lamps can be removed later.

## Lights in a world file

The world editor places lights, so a level can carry its own. A light is an entity in the world file, and its numbers
are the entity's properties, edited in the inspector like a moving platform's period.

| Kind | Properties (default) | Shines |
| --- | --- | --- |
| `point_light` | `color` (`"#FFFFFF"`), `strength` (8), `range` (12), `casts_shadows` (false) | in every direction from the entity |
| `spot_light` | the point light's, plus `inner_degrees` (20) and `outer_degrees` (35) | the way the entity points |
| `sun` | `color`, `sky_ambient`, `ground_ambient`, `casts_shadows` (true) | the way the entity points |

With no rotation a spot light and the sun point straight down; rotate the entity to aim it. Colours are `"#RRGGBB"`
text, like a tint. `assets/worlds/lamp-yard.world.json` is a small level with a sun, a red lamp and a torch.

In the editor, **Point light**, **Spot light** and **Sun** are in the palette. The viewport lights the scene with
them as you edit, so what you see is what the game draws, hiding a light in the outliner puts it out, and a selected
light draws what it reaches: a point light's range, a spot light's cone, the sun's rays.

When the game loads the file (`lantern:::world:::Load`), the point and spot lights are added to the scene and a `sun`
replaces the sun and ambient light. `lantern:::world:::Clear` removes the point and spot lights again, and lights the game added itself stay. It does not put the old sun back, so set the game's sun before each load.
The light entities exist in the world too, so a script can find them with `lantern:::world:::FindByKind("point_light")`.
Load a level after setting the game's own sun, as the platformer does, because the file's `sun` wins.

Keep light entities unparented: the game places a light by its own transform and ignores its parent's.

## Shadows

The sun casts shadows once `SetSunShadows(true)` is called. Lamps cast shadows only after `SetShadows(light, true)`,
because they cost more:

| Caster | Shadow renders |
| --- | --- |
| the sun | 3 (one per cascade) |
| a spot light | 1 |
| a point light | 6 (one per face of a cube) |

The renderer draws up to 12 local shadow tiles per frame. Two shadowed point lights use all of them, so spend them on
the lamps the player will notice. A lamp without shadows is cheap and still lights the scene.

`SetShadowQuality(enabled, sunMapSize, localTileSize, distance)` trades sharpness for speed. `enabled = false` turns
every shadow off. Map sizes run from 256 to 4096 pixels; the default of 1024 for the sun and 512 for each local tile
suits a small level. `distance` is how far from the camera the sun's shadows reach: a shorter distance packs the
same pixels into less ground, so shadows are sharper.

Mark a mesh that shouldn't cast a shadow, such as a glow or a decal, with `lantern:::world:::SetCastsShadows(entity,
false)`.

## Materials

```nyx
lantern:::world:::SetSurface(entity, 0.0f, 0.2f, "3FD0FF");
```

- *Metallic* from 0 to 1: a metal has no diffuse colour, only tinted reflections.
- *Roughness* from 0 to 1: low is a tight shiny highlight, high is a wide dull one.
- *Emissive* is light the surface gives off by itself, added after lighting, so it stays bright in shadow. It does
  not light its neighbours; for that, add a lamp next to it.

`lantern:::world:::SetGlow(entity, color, strength)` sets the emissive light as a colour times a strength. A strength
above 1 is brighter than a white wall in full sun; the renderer keeps that light unclamped, and bloom turns it into a halo
(see `bloom-and-tone-mapping.md`).

The platformer's gems use it: a smooth non-metal that glows pale blue, at four times its colour.

## Keeping 60 fps

- Prefer few shadowed lamps to many. Start with the sun only, then add a lamp shadow where it makes the scene
  clearer.
- Eight local lights are lit per frame. With more in the scene the nearest eight win, so lights near the player are
  the ones that count. Shadowed lamps are picked from those eight by the same distance, so a far lamp can lose its
  shadow, and which lamps have shadows can change as the camera moves.
- Lower `distance` and the map sizes before removing a light.
- The renderer reports how much shadow work a frame did in `RenderReport.shadowTiles` and `shadowDraws` (see `renderer.md`), which is what the tests check.

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| Striped or speckled surfaces ("shadow acne") | The shadow map is too coarse for the distance: lower `distance` or raise the map size. |
| A shadow floating away from its object ("peter panning") | Same cause, from the other side: raise the map size. |
| Shadows missing | `SetSunShadows(true)`, `SetShadows(light, true)` and `SetShadowQuality(true, ...)` must all be on, and the mesh must cast shadows. |
| A lamp seems to do nothing | Its `range` may be too small, or eight nearer lights took its place. |
| Shadows too dark | Raise the sky and ground ambient. |

## Soft edges and cascade seams

The shadow filter's width, its sample count and the fade between the sun's cascades are explained in `shadow-softness-and-cascade-blending.md`.
