# Known gaps

What hasn't been checked, what isn't built, and what is half-way. Each entry says where its card or decision is. Nothing
here is hidden: if you hit something that isn't listed, it's a bug, so add a card to the board.

## Not yet checked

The engine's tests run on a Linux machine with a software Vulkan driver and no display, and the platformer and editor
have been run on a Mac, but that was before the renderer effects were added. No one has measured the effects on real
hardware.

| What | Why it matters | Where it's tracked |
| --- | --- | --- |
| **60 frames per second on a real GPU** | The target is 60 fps on the platformer scene. The cloud machine has only lavapipe, a software Vulkan driver, so its frame times say nothing about a GPU. The quality levels (`graphics-effects.md`) are chosen by what each effect costs, not by measurement. Ambient occlusion is the costliest effect and the first to turn down if a machine misses 60. | `renderer-visuals` epic; D-064 to D-073 |
| **Metal output of the renderer effects** | The shaders are translated to Metal Shading Language by `spirv-cross` (`mist run build-shaders`). The HDR scene, bloom, sky, ambient occlusion, FXAA, fog, light shafts, soft shadows, blob shadows, colour grade, particles and normal maps have been checked only through Vulkan. | `renderer-visuals` epic |
| **Wayland on Arch Linux** | SDL picks Wayland by itself, but a Wayland run (window, mouse capture, gamepad, audio) hasn't been done. `SDL_VIDEO_DRIVER=x11` forces XWayland. | `running.md` |
| **Windows** | Never built. SDL GPU would use Vulkan; Nox's Windows support and Mist's shell scripts are open questions (`running.md`). | `running.md` |
| **Feel tuning** | The Boost height (3.6 m) and the Guide Light radii (8 m and 5 m) are first guesses that need a play test; the jump and camera distances can be tuned in `games/platformer/settings/` and `game-systems/third-person-camera.nyx` (F5 reloads them). | `crimson-and-thorn-slice.md`, open questions |
| **Gamepad** | Bindings exist for a standard gamepad, but no run with real hardware is recorded. | `game-api.md`, Input |

## Not built

| Feature | What exists instead | Notes |
| --- | --- | --- |
| **Decals** (a texture projected onto a surface) | A dark patch under things (`blob-shadows.md`), and a flat quad drawn just above a surface. | A projected decal needs the depth buffer read in the lit pass or a pass of its own (`normal-maps.md`, D-073). The `renderer-visuals` card for surface detail is closed with normal maps and blob shadows; decals are not built. |
| **Parallax or height mapping** | Normal maps change lighting only. | `normal-maps.md` |
| **Transparency** | Overlay and lines blend; meshes are opaque. | `renderer.md`, "What this renderer leaves out" |
| **More than eight local lights** | The eight nearest the camera are lit. | `renderer.md`; clustered or deferred shading would go past it |
| **Instancing and a render graph** | One draw per mesh instance; a hand-written `RecordFrame`. | `renderer.md` |
| **Particles as textured quads** | Particles are one-pixel lines drawn after the composite, so they don't bloom or fog (`particles.md`). | A billboard pipeline is the next step |
| **Direct3D 12** | SDL GPU takes Vulkan on Windows and Metal on macOS. | Needs shaders compiled to DXIL. |
| **A Vulkan `RenderBackend`** | `GpuRenderer` over SDL GPU. | Card `engine-follow-ups/a-vulkan-renderbackend-for-full-control-of-the-renderer` (todo) |
| **Text beyond ASCII, compressed audio, audio on its own thread** | An 8x8 bitmap font, WAV files and a mixer on the main thread. | `future-work.md` |
| **Sound behind walls, height, Doppler, a duller sound behind the listener** | Positional sound is distance and left-right pan only; the sound of a thing on a floor above sounds the same as one beside you. | `audio.md` |
| **Automatic hot reload** | Silhouettes and graphs reload by themselves; `.nyx`, `.chaos` and `.cosmos` edits still need F5, and every reload restarts the game. | `future-work.md` |
| **Input rebinding and saved settings** | Bindings are set in `games/platformer/settings/input-settings.nyx`. | `future-work.md` |

## The game

| Gap | Where it stands |
| --- | --- |
| **The Anchor** (Ability 3 of the Crimson and Thorn design) | Open: the 3D form (a physics rope, a set swing arc or something else) is Dean's decision. Card `platformer-game/crimson-and-thorn-slice/the-anchor-in-3d`; the recommendation is a set swing arc. It is being built, so check the board before assuming it's absent. |
| **Slice 3 onwards** | Authored cameras, chapters 1 to 5 as a whole, post-completion changes, layered music and a save flow are listed as later in `crimson-and-thorn-slice.md`. |
| **Collision on and off for a world entity** | A hidden platform is moved 1000 m down instead (card `engine-follow-ups/game-controlled-collision-for-world-entities`). |
| **A Cosmos sheet per component** | Every `.cosmos` in `ui/` is merged into one shared sheet, so class names must be unique across the folder (card `platformer-game/in-game-ui-with-chaos-and-cosmos/a-cosmos-sheet-per-component`). |

## The world editor

The editor's own gap list is at the end of `editor.md`. The cards still open are under `world-editor/follow-ups-after-the-first-full-pass`:
one tooltip at a time through a context service, saving split ratios between runs, headless tests of the UI's behaviour,
and thumbnails for model and texture assets. Also open: warn when a light entity has a parent, and cap the spot light's
cone guide angle (both on `engine-follow-ups`).

## Engine follow-ups

The `engine-follow-ups` epic holds the engine's own leftovers: pool slots that hold `T?` so pools can own meshes and
textures, renaming names that hide the `lantern:::` namespaces, building `lantern-native` against the current Chaos,
and editor select handlers that must not return a value. Read the epic with `cairn --root docs/kanban tree`.

## Documentation

Every `.nox` and `.nyx` file outside `tests/` explains itself in comments. Not covered yet: the `.chaos` and `.cosmos` files
(46 of 54 have no comments, most of them in `apps/world-editor/src/ui/`) and the test programs under `tests/`, which have no
header saying what each one checks. The convention names only `.nox` and `.nyx` files, so this is a choice for later (card
`documentation-sweep/comments-in-chaos-cosmos-and-test-files`).
