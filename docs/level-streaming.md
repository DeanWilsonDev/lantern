# Level streaming

A level is one world file, and until now `lantern:::world:::Load` spawned every entity in it at once: a mesh and a collider for each block, a light for each lamp, a tag record, a slot in the entity world that every fixed step walks. A level ten times longer cost ten times as much to hold, to walk each step, and to load. Streaming splits a level into **rooms** and keeps only the rooms near the player and the partner alive, so what a level costs follows the part you stand in and not its length.

This page is the first slice. It says what streams, how a room is picked, how a room arrives without a hitch, how the partner's routes and the lights stay right across a room boundary, and what is not done.

## What streams, and what doesn't

Only entities the engine alone gives meaning to stream: a `block`, a `ramp`, a `point_light` and a `spot_light`. They are plain scenery. Nothing in the game holds their ids, so despawning one and spawning it again later loses nothing.

Everything the game wraps in a class at load stays resident for the whole level: gems, plates, levers, gates, lamps, checkpoints, team beats, signs, camera shots and the rest. Those carry state (a plate that is held, a gem that is taken, a gate that is open) and the game's own lists point at them. Letting them live in a room is a separate piece of work (see "Not done").

An entity streams when all of these hold. If any fails it stays resident and the loader logs a warning naming it, so a mistake in a file costs memory and not a missing floor.

- It has a `room` property: text, the room's name.
- Its kind is `block`, `ramp`, `point_light` or `spot_light`.
- It has no parent and nothing names it as a parent (a parent's transform composes with its children's, so the pair has to live and die together; that is not built).
- Its collider is none or `static` (a `kinematic` body is moved by game code and carries a body handle).

## Rooms are authored in the world file

A room is every streaming entity that shares a `room` name. Its bounds are the box around its members' world-space boxes (a block's box is its half extents scaled, so a rotated block is measured by the box of its rotated corners). Nothing is added to the file format: `room` is an ordinary property, so the editor round-trips it, the Inspector shows it, and a level that has none loads and plays exactly as before.

The build never reads a Calamus file for this. Rooms live in the world file, which is already a plain data file the build copies into the package (D-217, D-218).

## Choosing rooms

`lantern:::world:::StreamRooms(focus, loadDistance, unloadDistance, urgentDistance, budget)` runs once a fixed step. `focus` is a list of points: the game passes the player, the partner and the companion, so a room the partner is standing in stays alive when the player has walked on, and a room she is about to walk into is there before she is.

For each room the distance is the **flat distance from the nearest focus point to the room's bounds** (zero inside it; height is ignored, because a level that stacks floors should load the whole stack). Then:

| Distance | What happens to the room |
| --- | --- |
| at most `urgentDistance` | loaded at once, whatever the budget. A floor must exist before anyone can stand on it. |
| at most `loadDistance` | queued to load, nearest first, spending at most `budget` entities per step |
| past `unloadDistance` | unloaded: its entities, meshes, colliders and lights are removed |
| between the two | left as it is (the gap is the hysteresis, so standing on a boundary doesn't load and unload every step) |

`unloadDistance` has to be larger than `loadDistance`; the engine raises it to one metre past if it isn't.

A room that loads over several steps is **partial** until its last entity arrives. A partial room inside the urgent distance is finished at once, so nobody walks onto half a floor.

The platformer's numbers are in `games/platformer/settings/streaming-settings.nyx`: load 70 m, unload 95 m, urgent 24 m, 12 entities per step. They are provisional: the night fog hides the edge at 70 m on lavapipe, and nobody has judged it on a Mac.

## Loading without a hitch

Making an entity costs most in the mesh: `CreateMesh` allocates buffers. So the load queue spends a budget of entities per step, nearest room first, and a room that doesn't fit carries on next step. Only the urgent ring ignores the budget, and a player walking at the game's speed meets the urgent ring about three seconds after the load ring, so in play it has finished already; it only fires for a teleport (a respawn, a restart, a checkpoint) or a first frame.

Unloading is not spread: a room despawns in one step. A block is cheap to remove (a despawn, one `RemoveMesh`, a dead mark on its triangles), but a very large room would cost more; "Not done" has it.

## Collision

The static collision world is a hash grid of triangles. A streaming entity's triangles are added as a **group**: `BeginStaticGroup` returns an id, every `AddStaticTriangle` until `EndStaticGroup` belongs to it, and `RemoveStaticGroup(id)` marks them dead. A query skips dead triangles. The grid keeps their entries until the dead outnumber the live (and are over 4096), then `CompactStatic` rebuilds the arrays and the grid once and keeps every live group's range. That pass is the one place unloading can cost more than a frame's worth; the next slice would spread it.

## The partner's routes

The partner walks a graph of the level's floors that is built once from the static collision world (`docs/navigation.md`). Streaming doesn't touch it, and this slice relies on that on purpose:

1. `LevelScene.Load` loads the world file. With rooms in it, the rooms are registered and none is spawned.
2. Before the graph is built, `lantern:::world:::LoadAllRooms()` spawns every room, because the build samples every floor.
3. The game builds the graph (`companion.Spawn`, `partner.Spawn`), then starts streaming. The first `StreamRooms` unloads everything but what is near.

The graph keeps its nodes for rooms that are no longer there, so a route is planned across the whole level and a long lead (a beat in a far room) works. She can only walk a route whose floor is loaded, and the focus list contains her, so the floor under her and the rooms either side of her are always alive. The urgent ring is the guard: she never steps off a loaded room.

The cost of this is that **load time still grows with the level**: step 2 spawns everything and the graph samples everything. A graph built per room, as each room loads, is the next piece ("Not done").

## Lights

A `point_light` or `spot_light` in a room is added to the render world when the room loads and removed when it unloads, keeping its id in the room. The renderer already picks the eight lights nearest the camera each frame (`MAX_LOCAL_LIGHTS`), so lights across a boundary stay right whether or not their room is alive: a lamp in a neighbouring room is alive whenever it is within the load distance, which is further than a lamp's range (14 m for a street lamp). What streaming saves is the per-frame cost of choosing among every light in the level, and a long list of lights to keep.

The game's own lamps (`street_lamp`, a game kind) stay resident, so the night look needs no change.

## Checking it

- `engine/tests/physics/removed-static-groups-stop-colliding-and-compact` adds groups, removes some, checks queries and the ray cast, compacts and checks again.
- `engine/tests/world/rooms-are-chosen-by-distance-with-hysteresis` tests the pure planning functions.
- `engine/tests/host/rooms-stream-in-and-out-as-the-player-walks` loads a synthetic level hundreds of metres long in rooms, walks along it and checks the live entity and triangle counts stay bounded, no step spawns more than the budget outside the urgent ring, and the player is never without a floor.
- `engine/tests/host/the-crossing-streams-its-scenery` loads the crossing, checks it streams, and plays its opening.

## Composed scenes (instances and sockets built, peek and editor not)

Dean's second idea builds on rooms: a **scene** is an object that holds other objects, and scenes are placed into the world and stitched together. The plan keeps one concept: a scene is a world file, a placed scene is a room.

- **Instances.** A `scene_instance` entity has a `scene` property (the scene's world file) and a transform. The loader expands the file's entities into the parent world with the instance's transform composed in, and puts them all in one room named for the instance, so streaming loads and unloads scenes with no new machinery. Entity ids inside an instance are namespaced by the instance, so two instances of one scene don't collide. Scenes nest, to a depth limit. The expansion happens when the world file is loaded or baked, so the running game reads plain entities and never a scene's source.
- **Sockets.** A `scene_socket` entity inside a scene marks an opening: a position, a facing (its yaw), a width and a height. Stitching places an instance so one of its sockets lies on a socket of its neighbour, facing the other way; the loader keeps each joined pair as an adjacency.
- **Peeking.** A neighbour that streaming hasn't loaded is drawn only through its opening: a cheap proxy of its near geometry (boxes merged and simplified, no collision, no lights, no entities), shown while the opening is on screen and clipped to the socket's rectangle. Only what is immediately visible, at a lower level of detail, is the point: the proxy is a few hundred triangles and costs one draw. The proxy is baked with the scene, so the game binary still carries no Calamus data.
- **Navigation and lights across the seam.** The adjacency tells the partner's graph which sockets to join when each side's nodes exist. Streaming needs nothing extra for a seam: a room's bounds reach the socket, so the neighbour is as near as the opening is.
- **The editor** places an instance, shows its sockets and snaps the dragged instance to the nearest compatible socket.

Four cards track it under `level-streaming`. Room streaming was the first PR. Instances and sockets are the second and are built: `engine/src/world/Scenes.nox` (`ExpandScenes`, `StitchTransform`, `PlacedPoint`, `SceneLink`), called by the host's Nyx `lantern:::world:::Load` after it reads the file (the world editor's own loader doesn't expand), so a game loads a level made of scenes with no code of its own. Scene paths are relative to the file that places them. An instance may only turn about the up axis (its `rotation_degrees.y`), because a yaw adds exactly to an entity's own rotation; its scale is ignored. A scene's ids become `(instance id << 32) | scene id`. Entities that can't stream (gems, plates, sockets, the instance markers) are copied in resident, so a scene placed twice has two gems. The streamer keeps the joined openings (`RoomStreamer.links`) for the peek (built, below); the editor is the fourth PR. Its snapping is built as a pure function first: `lantern:::world:::NearestSocketSnap` takes the dragged scene's position, yaw and sockets and the world's sockets and returns the placement that lays the nearest pair together (`engine/tests/world/sockets-snap-a-dragged-scene-to-the-nearest-pair`). `EditorSession` calls it while a translate drag previews a parentless `scene_instance` (`SnapPrimaryToSockets`, sockets from `engine/src/editor/SceneSockets.nox`), so a release is one undo step; `SetSocketSnapEnabled` turns it off. Checked by `engine/tests/editor/a-dragged-scene-instance-snaps-onto-its-neighbours-socket`. The Assets tab's Add scene button (`editorAddSceneInstance`, `EditorSession.AddSceneInstance`) places the chosen world file as a `scene_instance` named relative to the saved level (`RelativePath`).

Checked by `engine/tests/world/composed-scenes-place-stitch-and-link` (placement, ids, rooms, stitching with a turned scene, links) and `engine/tests/host/scenes-in-a-world-file-become-streaming-rooms` (four halls loaded through `world:::Load` become four rooms with three links, streaming loads the one the player stands in).

### The peek, as built

`UpdatePeeks` (in `RoomStreamer.nox`) runs at the end of each `StreamRooms`. A room that isn't present and joins a complete room gets up to 16 boxes from `PeekBoxes` (the pieces' boxes nearest the opening, within 40 m), made from one shared cube mesh in each piece's colour, with no collision, tags, lights or shadows. Each step `BoxSeenThroughOpening` hides the boxes the camera can't see through the opening. The boxes are removed when the room loads or has no complete neighbour. Checked by `engine/tests/world/the-peek-chooses-near-boxes-and-culls-what-the-opening-hides` and the peek assertions in `scenes-in-a-world-file-become-streaming-rooms`. Merged and simplified meshes per scene are not built.

## Not done

- **Per-room navigation.** Load time is O(level). Build each room's nodes when it loads and join across the boundary, spread over frames.
- **The game's own kinds in rooms.** A room could own gems, plates and lamps if the game got room loaded and unloaded events and kept their state in a record the room carries.
- **Spreading unload and compaction** over frames, once a Mac shows a hitch.
- **Streaming meshes and textures from disk**: a room's block meshes are built from shapes in the file, and its materials are already loaded textures, so nothing waits on a file read yet. A room of imported models would.
- **Parents.** A streaming entity can't have a parent or a child.
- **The editor.** It loads a world file whole and shows every room, and does not expand scene instances. Drawing sockets, a toggle button for socket snapping, a room box, a "show only this room" toggle and an inspector field for `room` are not built.
