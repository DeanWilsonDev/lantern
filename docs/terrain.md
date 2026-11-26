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

## Meshes and collision

`BuildTerrainMesh` makes the mesh of the whole terrain: two triangles per cell, smooth normals from the neighbouring
heights, texture coordinates in metres. `BuildChunkMesh` makes one 32 by 32 cell chunk with the same normals, so chunks
shade without a seam. `BuildPiece` uses the whole mesh for a terrain entity, so the game spawns it and adds its static
triangles to the collision world with no terrain-specific code. The collision grid sorts the triangles, so a 97 by 97
terrain costs no more per query than any other floor.

## Known gaps

- One mesh per terrain, so a big terrain is always drawn whole (the chunk builders are ready).
- Erosion is thermal only.
- No painted layers yet: a terrain has the one material of its entity.
