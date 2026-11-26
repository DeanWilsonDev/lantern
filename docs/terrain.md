# Terrain

Terrain is ground you can shape: hills made from a seed, then raised, lowered, smoothed and flattened by hand. It is a
square grid of heights drawn as one smooth mesh, and the player and the companion walk on it like any other floor.

The code is `engine/src/world/terrain/`, the tests `engine/tests/terrain/`, and `games/platformer/assets/worlds/terrain-demo.world.json`
is a level to walk on (`LANTERN_LEVEL=worlds/terrain-demo.world.json`).

## How a terrain is stored

A terrain is a world entity whose `kind` is `terrain`. It has no shape; its data is in properties, so the world file
format is unchanged.

| Property | Meaning |
| --- | --- |
| `terrain_columns` | Vertices along one side, 2 to 257. The grid is square. |
| `terrain_cell_size` | Metres between neighbouring vertices. |
| `terrain_heights` | One height per vertex, row by row: four hex digits each, the height in whole centimetres plus 32768. |
| `terrain_seed_low`, `terrain_seed_high` (the 32-bit seed in two halves), `terrain_feature_size`, `terrain_octaves`, `terrain_persistence`, `terrain_lacunarity`, `terrain_amplitude`, `terrain_base_height` | The generator settings the terrain was made with. They are kept so the editor can show them; changing them does nothing until you generate again. |

The terrain is centred on the entity's origin: the first vertex is at `(-half, -half)` and the last at `(half, half)`
with `half = (columns - 1) * cell_size / 2`. The entity's transform moves, turns and scales it, and a `material` and
`tint` on the entity colour it like a block's. A terrain with `"collider": "static"` blocks the player, one with none is
only drawn.

## Generating

`GenerateTerrain(field, settings, offsetX, offsetZ)` fills a `Heightfield` with layered gradient noise.

| Setting | Effect |
| --- | --- |
| `seed` | Picks the terrain. The same seed and settings always give the same ground. |
| `featureSize` | Width in metres of the biggest hills. |
| `octaves` | Layers of detail, 1 to 8. Each layer is `lacunarity` times finer and `persistence` times as tall as the one before. |
| `amplitude` | How tall the hills are, in metres. Heights stay within `baseHeight` plus or minus `amplitude`. |
| `baseHeight` | The level the hills stand on. |

Noise is read at each vertex's position in metres plus the offsets, so terrains placed side by side join if their
offsets differ by their width. `ErodeTerrain(field, passes, talusSlope, strength)` then lets material slide down any slope
steeper than `talusSlope` (rise over run), which rounds peaks and fills hollows.

## Brushes

`ApplyBrush(field, kind, centerX, centerZ, radius, strength, flattenHeight)` is one dab, centred at a point in the
terrain's own space. The weight is 1 in the middle and eases to 0 at `radius`.

| Brush | What it does |
| --- | --- |
| `Raise`, `Lower` | Move the ground up or down by `strength` metres at the centre. |
| `Smooth` | Moves each vertex toward the average of its neighbours; `strength` 0 to 1. |
| `Flatten` | Moves each vertex toward `flattenHeight`; `strength` 0 to 1. |

It returns how many vertices changed.

## Sculpting in the editor

The Terrain tab sits in the bottom panel beside Silhouettes (Alt+6, or Window > Terrain). It has three columns.

- **Sculpt.** Tick "Brush on", pick a brush, a radius and a strength, then hold the left button on a terrain. A ring shows where the brush
  works. While the brush is on, a left press on a terrain starts a stroke instead of selecting or box-selecting; anything that is not a
  terrain is looked through, and the gizmo, the view cube, the right button and the alt-drag still work as usual. Raise and Lower move the ground by
  `strength` metres a second at the centre, Smooth and Flatten close that share of the gap, times six, a second. Flatten levels to the height under
  the press. Releasing the button ends the stroke as one undo step, "Sculpt terrain".
- **Generate.** The seed (New seed picks the next one), feature size, octaves, height and base height a terrain is made from.
- **Make.** The size (columns of vertices and cell size) and the actions. New terrain adds one centred where the view looks and selects it.
  Regenerate selected rebuilds the selected terrain from the Generate settings. Erode selected runs the erosion passes on it. Each is one undo step.

The same options are on the session as `SetTerrainOption(name, value)` and `TerrainOption(name)`, and the actions as `TerrainAction(name)`
(`generate`, `regenerate`, `erode`, `reseed`).

## Meshes and collision

`BuildTerrainMesh` makes the mesh of the whole terrain: two triangles per cell, smooth normals from the neighbouring
heights, texture coordinates in metres. `BuildChunkMesh` makes one 32 by 32 cell chunk with the same normals, so chunks
shade without a seam. `BuildPiece` uses the whole mesh for a terrain entity, so the game spawns it and adds its static
triangles to the collision world with no terrain-specific code. The collision grid sorts the triangles, so a 97 by 97
terrain costs no more per query than any other floor.

## Known gaps

- One mesh per terrain, so a big terrain is always drawn whole (the chunk builders are ready). Each sculpt frame rebuilds the mesh of the terrain it changes, which is quick for the default 65 by 65 and noticeable at 257 by 257.
- The Generate settings are the tools', not the selected terrain's: selecting a terrain doesn't load its seed into the tab.
- The brush radius is in metres of the terrain's own space, so a scaled terrain gets a proportionally bigger or smaller brush.
- Erosion is thermal only.
- No painted layers yet: a terrain has the one material of its entity.
