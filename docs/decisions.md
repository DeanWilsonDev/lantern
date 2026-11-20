# Decisions log

Dean handed over the decisions for the overnight build on 2026-10-26. Each entry says what was
decided, why, and how to undo it. Newest entries go at the bottom.

## Index

Decisions are numbered in the order they were made, without gaps. A new decision takes the next number, and a pull
request that adds one to a branch that has since gained another renumbers its own. This table lists D-001 to D-073; a
later decision is in the log below and is added here when someone next tidies it. `grep '^## D-' docs/decisions.md` lists
every entry.

| Number | Decision |
| --- | --- |
| D-001 | Work on a branch based on `proto` |
| D-002 | Nyx C ABI comes from the `nyx-capi` pull request branch |
| D-003 | SDL3 through `sdl3-sys`, bundled by default |
| D-004 | Shaders: WGSL compiled to SPIR-V and MSL by naga at build time (superseded: shaders are GLSL, built with `glslangValidator` and `spirv-cross`; see `running.md`) |
| D-005 | Text: embedded 8×8 bitmap font |
| D-006 | Audio: WAV only, mixed on the main thread |
| D-007 | Collision: our own kinematic collision, no physics crate |
| D-008 | Assets are generated and committed |
| D-009 | Windows renders through Vulkan |
| D-010 | One rustfmt configuration for the whole workspace |
| D-011 | The frame loop is a struct driven one frame at a time |
| D-012 | Minimised windows pause the simulation |
| D-013 | Rendering details chosen for M2 |
| D-014 | Scene, assets and audio wiring (M3) |
| D-015 | Collision and character controller (M4) |
| D-016 | Platformer slice details (M5) |
| D-017 | Nyx scripting layer (M6) |
| D-018 | The world file is the seam between the game and the editor |
| D-019 | The game describes itself to the editor with a palette |
| D-020 | The editor's split: Rust logic and viewport, Nyx/Chaos/Cosmos panels |
| D-021 | The viewport renders off-screen and is copied into a Penumbra texture |
| D-022 | The editor links the system SDL3, and the C API's rules |
| D-023 | All game code is Nyx; engine code is Rust (2026-10-27) |
| D-024 | Direct calls replace the snapshot and command queue |
| D-025 | One generic runner: `lantern` and `game.json` |
| D-026 | The level is the editor's world file |
| D-027 | The engine keeps F5 and F1 |
| D-028 | Reload restarts the whole game |
| D-029 | Vectors are host value types |
| D-030 | Nyx gaps go on the Game Engine epic |
| D-031 | The viewport is a Chaos primitive, not a native widget |
| D-032 | Editor UI state lives in one Nyx `Application` |
| D-033 | Menus, popups, tooltips and dialogs are portals built from frames |
| D-034 | Shortcuts are split between Nyx and the viewport |
| D-035 | Preferences are a flat JSON file |
| D-036 | Play in editor runs the real game on a scratch level |
| D-037 | The stack is pinned by branch, overridable per checkout |
| D-038 | Editing operations are editor-core methods with one undo entry each |
| D-039 | Thumbnails and previews are rendered by the backend |
| D-040 | Penumbra's closed dropdown draws its label |
| D-041 | The hosts describe their names to nyx-lsp |
| D-042 | The game code reads as a tutorial |
| D-043 | The game's UI is Chaos and Cosmos |
| D-044 | Mouse in the in-game UI; Ui::: removed; elements are lantern:::ui:::Element |
| D-045 | Every engine name is under lantern::: |
| D-046 | The Nox engine links Nyx and the Chaos bridge as one library |
| D-047 | The Nox game host keeps the game's UI in a GameUiState the session owns |
| D-048 | The Nox editor model keeps the Rust shapes, with sorted arrays for maps |
| D-049 | Nox gizmo handles are a struct, and the viewport build waits for the session |
| D-050 | The Nox editor session takes its drag out of the session while it works on it |
| D-051 | The Nox editor backend is a static library behind the same C header |
| D-052 | The view cube is drawn by the Nox editor into the viewport's overlay |
| D-053 | Clicking the same spot again selects the next entity behind |
| D-054 | Outliner rows reorder by dropping on the line between them |
| D-055 | EditorApplication only sets up the application; behaviour lives in services and components |
| D-056 | The Nox host is the world editor's only host |
| D-057 | The engine is laid out one folder per domain, with SDL behind a facade folder |
| D-058 | Lights are plain data in RenderWorld: one sun, up to eight local lights, a PBR-lite material |
| D-059 | Shadows come from depth maps in two atlases, filtered with PCF |
| D-060 | The platformer grows by slices of the Crimson and Thorn mechanics |
| D-061 | Slice 1 of the Crimson and Thorn mechanics: the streets |
| D-062 | Lights in a world file are ordinary entities, found by kind |
| D-063 | Slice 2 of the Crimson and Thorn mechanics: light, thorns, chapters |
| D-064 | The scene is drawn in high dynamic range, then tone mapped, with bloom as an optional pass |
| D-065 | A procedural sky that lights the scene |
| D-066 | FXAA, not MSAA or TAA |
| D-067 | Screen-space ambient occlusion over the depth buffer |
| D-068 | Height fog and light shafts in the composite pass |
| D-069 | A spiral shadow filter and cascade blending, not contact-hardening shadows |
| D-070 | Blob shadows darken surfaces by distance to a point, in the composite pass |
| D-071 | The colour grade is applied after tone mapping, in the composite pass |
| D-072 | Particles are simulated in the engine and drawn as fading line streaks |
| D-073 | Normal maps use a cotangent frame from screen-space derivatives |

## D-001 Work on a branch based on `proto`

All work is on `claude/project-thread-mxbapl`, branched from `proto` and opened as a pull request
into `proto`. Nothing is merged into `main`. Merge or rebase it into `proto` when happy.

## D-002 Nyx C ABI comes from the `nyx-capi` pull request branch

`nyx-sys` builds `nyx-capi-static` from a sibling `nyx-scripting-language` checkout. The C API
(`include/nyx/nyx.h`) is not on that repository's `proto` branch yet; it lives on
`claude/project-thread-qf7t4j` (PR #7). Lantern was built and tested against that branch at
`f5531fc`, with submodules initialised. Once PR #7 merges into `proto`, nothing changes here.

## D-003 SDL3 through `sdl3-sys`, bundled by default

Raw bindings plus our own thin wrappers keep every `unsafe` block in two crates
(`sdl-facade`, `sdl-gpu`). SDL is built from source and statically linked by default (feature
`bundled-sdl`), which makes a clean checkout build the same way on macOS, Arch and Windows. To use
a system SDL3 instead, build with `--no-default-features --features system-sdl` on the game.

## D-004 Shaders: WGSL compiled to SPIR-V and MSL by naga at build time

**Superseded.** This is the Rust engine's shader workflow. The Nox engine writes shaders in GLSL and builds them with `glslangValidator` and `spirv-cross` (`mist run build-shaders`; see `running.md`). The reasoning below is kept as history.

- One source per shader, readable, and no external SDK needed.
- SPIR-V is used by the Vulkan backend (Linux, Windows) and MSL by Metal (macOS).
- SDL GPU's D3D12 backend needs DXIL/DXBC, which naga cannot produce. Windows therefore runs on
  Vulkan. Adding D3D12 later means adding HLSL output and a DXC step; see `future-work.md`.
- SDL expects one combined image-sampler per texture slot on Vulkan. WGSL has separate textures
  and samplers, so the build script maps both to the same descriptor binding, which Vulkan allows
  for a combined image-sampler descriptor. This was checked on Mesa lavapipe with validation
  layers on and no errors.
- `ADJUST_COORDINATE_SPACE` is turned off because SDL GPU already gives the same clip space on
  every backend (checked: positive NDC y lands at the top of the image).

## D-005 Text: embedded 8×8 bitmap font

The brief allowed bitmap or SDF. A bitmap font needs no font file, no licence question (the
glyphs are the public-domain `font8x8_basic` set) and no rasteriser dependency. Text is drawn at a
whole-number scale chosen from the window's pixel density, so it stays sharp on high-DPI screens.
An SDF or TTF atlas can replace `BitmapFont` without touching the UI code.

## D-006 Audio: WAV only, mixed on the main thread

The mixer is pure Rust and testable without a device. Each display frame the game tops up the
SDL audio stream so about 60 ms of audio is queued. That means no locks and no audio callback.
The trade-off is that a very long frame (over ~60 ms) can underrun; the HUD counts underruns.

## D-007 Collision: our own kinematic collision, no physics crate

Capsule casts use conservative advancement against triangles (the closest distance between the
capsule's segment and a triangle bounds how far it can move safely). It is exact enough for a
platformer, needs no special case per shape pair, and handles boxes (as triangles) and triangle
meshes with the same code. Static geometry is bucketed in a uniform grid.

## D-008 Assets are generated and committed

Sounds (jump, land, pickup, menu move/select, a short looping music track), a checker texture and
a small glTF collectible are generated by `tools/asset-gen` and committed under `assets/`, so the
demo works offline and the files are reproducible. Missing files fall back to generated
primitives and silence, with a logged warning.

## D-009 Windows renders through Vulkan

See D-004. SDL is asked for SPIR-V or MSL shader formats, so it picks Vulkan on Windows and
Linux, and Metal on macOS. `LANTERN_GPU_DRIVER=vulkan|metal` forces a driver for testing.

## D-010 One rustfmt configuration for the whole workspace

`rustfmt.toml` sets `max_width = 120` and `use_small_heuristics = "Max"`, close to the style
the existing nyx crates were written in. `cargo fmt --all` was run once over everything,
including `nyx`, `nyx-sys` and `frame-loop`; those files changed formatting only.

## D-011 The frame loop is a struct driven one frame at a time

`FrameLoop::run_frame` does one display frame. `run` just loops it. Tests drive `run_frame`
with a scripted platform to check quit handling, frame-rate independence, latched jump presses
and minimise behaviour without a window.

## D-012 Minimised windows pause the simulation

While minimised the loop runs no fixed steps, skips rendering and sleeps 16 ms per frame. The
first frame after restoring measures zero elapsed time, so the game does not lurch forward.

## D-013 Rendering details chosen for M2

- One render pass per frame: lit meshes, then debug lines (depth-tested, no depth write), then
  the overlay (no depth test, alpha-blended). Every pipeline declares the depth target so they
  can share the pass.
- The swapchain asks for `SDR_LINEAR` (an sRGB format), so shaders write linear colour and the
  hardware encodes it. Where that composition is unavailable the shaders encode instead (a
  uniform flag), so colours look the same either way.
- Textures upload as `R8G8B8A8_UNORM_SRGB`; linear-filtered textures get a full mip chain from
  `SDL_GenerateMipmapsForGPUTexture`.
- Per-frame geometry (lines, overlay) goes through one reusable buffer each, uploaded with
  `cycle = true` so the CPU never waits on the GPU reading last frame's data.
- Deferred destruction is SDL's: `SDL_Release*` waits until command buffers that use the
  resource have finished. On shutdown the renderer waits for idle first.
- Frustum culling uses each mesh's local bounds transformed by the draw's matrix. Opaque draws
  are sorted by texture, then mesh.
- `GpuRenderer::capture` renders off-screen and reads pixels back; the `render_capture`
  integration test uses it to check a cube, a line and overlay text land where expected. It ran
  on Mesa lavapipe (software Vulkan) under Xvfb with Vulkan validation enabled and reported no
  validation errors.

## D-014 Scene, assets and audio wiring (M3)

- **World and stores.** `lantern_engine::world::World` holds names, transforms, a parent and an
  optional `MeshRenderer`. Game-specific data (player state, gems) lives in the game's own
  structs or `ComponentStore<T>`s keyed by `EntityId`. No ECS: the demo has a handful of entity
  kinds, and plain structs keep every piece of state visible in one place.
- **Scenes are tagged entities.** Each entity records the `SceneId` that spawned it;
  `World::despawn_scene` removes exactly those. The app's `LevelScene` also owns the GPU meshes
  it created and releases them in `unload(self, ...)`, which takes the scene by value so it
  cannot be used afterwards. There is no scene stack because the demo has one
  level; game states (menu, playing, paused) live in the app.
- **The level is code** (`apps/platformer-game/src/level.rs`): boxes and ramps only. The same
  data builds render meshes and, from M4, collision, so the two cannot disagree. A scene file
  format is out of scope per the brief.
- **Asset lookup:** `LANTERN_ASSETS`, then `assets/` beside the executable, then the
  repository's `assets/`, then `./assets`. Every file has a generated fallback; a failure is
  logged once with the path and reason, and the game carries on.
- **Generated assets** come from `tools/asset-gen` and are committed (D-008).
  `cargo run -p asset-gen -- --check` compares the files with fresh output. The synthesiser
  uses `f32` maths (`sin`, `powf`), so the check is exact on one machine but may differ by a
  bit on another platform's libm; that is harmless (re-run the generator) and is why the check
  is not part of the test suite.
- **Audio device behind a trait.** `lantern_platform::AudioSink` (sample rate, queued frames,
  push) is implemented by `lantern_sdl_facade::SdlAudioOutput` (an `SDL_AudioStream` on the
  default playback device, `f32` stereo at 48 kHz, no callback). `lantern_engine::audio::
  AudioSystem` mixes on the main thread each frame and tops the queue up to 60 ms. If no device
  opens, the system runs without a sink and every call is a silent no-op.
- **Headless capture.** `LANTERN_CAPTURE_PATH=out.png LANTERN_MAX_FRAMES=n` saves the last
  frame. This is how the scene was checked in the sandbox (Xvfb + lavapipe).

## D-015 Collision and character controller (M4)

- **Casting** is conservative advancement per triangle, using the fact that the distance
  between a translating convex shape and a triangle is convex in time: Newton steps from the
  start never pass the first contact, and a non-decreasing distance means no contact at all.
  Fast moves cannot tunnel (tested at 1000 m per cast through a 2 cm wall).
- **Skin.** The controller casts a capsule inflated by `skin_width` (1 cm), so stopping at a hit
  leaves exactly that gap and sliding never re-hits the surface it slides on. Earlier versions
  pushed out along the normal after each hit; that jittered in corners and was removed.
- **Ledges and stairs.** A capsule's rounded bottom touching an edge reports a tilted normal
  that looks like a steep slope. For "can I stand here?" the controller also ray-checks the
  face just beside the contact (PhysX's controller uses the touched triangle's normal for the
  same reason). Steps up to `step_height` (0.3 m) are climbed with an up / forward / down probe.
- **Ties.** When a cast touches several surfaces at the same fraction (a capsule resting where
  floor meets wall), the one most opposed to the motion wins, so a downward probe reports the
  floor rather than the wall.
- **Moving platforms** are kinematic bodies: triangles in local space plus a transform the
  game sets once per fixed step after `CollisionWorld::begin_step`. A grounded player is
  carried by the body's step motion before moving itself.
- **Feel numbers** (in `ControllerSettings::default`): 6 m/s walk, 50 m/s² ground and 20 m/s²
  air acceleration, 28 m/s² gravity, 1.5 m jump apex with 2.5× gravity after an early release,
  0.1 s coyote time, 0.12 s jump buffer, 46° walkable limit, 0.3 m step, 0.3 m ground snap.
  These were chosen to make the demo course comfortable and are measured by tests rather than
  tuned by feel; Dean should adjust them after playing.
- **The course test** (`apps/platformer-game/src/course_test.rs`) drives a pilot through the
  real level at 30, 60, 144 and 240 Hz display rates and requires it to reach the goal at each.
  This is the M4 gate.
- **Rules seam.** Game rules (gems, platform path, falling) live behind the `GameRules` trait:
  they read a `RulesView`, return `RuleCommand`s and receive `RuleEvent`s. `NativeRules` is the
  Rust version; M6 adds the Nyx version with identical behaviour.

## D-016 Platformer slice details (M5)

The behaviour below still holds; since D-023 it is written in Nyx (`games/platformer/`), and F1 is
the engine's (D-027).

- **Game states** are an enum in the app (`MainMenu`, `Playing`, `Paused`) with the
  transitions written out in `game.rs`, rather than state objects and a state
  machine. Three states with no shared behaviour do not need a framework. The simulation only
  advances while `Playing`; the menu shows the level behind it with a slowly orbiting camera.
- **Menu input** uses the per-frame snapshot (menus are presentation); gameplay uses the
  per-fixed-step snapshot. A menu choice made while drawing is applied at the start of the next
  frame, where the platform is available for mouse capture.
- **Blob shadow.** The player gets a dark disc on the surface below (one ray per step), which
  shrinks with height. Without real shadows it is the cheapest way to judge jumps, and it is a
  well-known platformer technique. Real shadow mapping stays out of scope (brief).
- **Sounds** are triggered by rules (jump, land above 4 m/s, pickup, reset) and by the menus
  (move, select). The music loop starts at launch.
- **Debug views:** F1 toggles the HUD (frame timing, player state, camera, renderer counters,
  audio queue, rules status and errors), F2 the collision view (nearby triangles, player
  capsule, ground normal, velocity, gem trigger spheres). `LANTERN_DEBUG_HUD=1` and
  `LANTERN_COLLISION_VIEW=1` start with them on.
- **Autoplay** (`LANTERN_AUTOPLAY=1`) skips the menu and runs forward with periodic jumps. It
  exists for headless smoke runs together with `LANTERN_MAX_FRAMES` and `LANTERN_CAPTURE_PATH`.

## D-017 Nyx scripting layer (M6)

Superseded on 2026-10-27 by D-023 to D-030: the snapshot, the commands, the native rules and the
`nyx-scripting` feature are gone, and hook errors now carry their Nyx location (D-029).

- **Snapshot in, commands out.** A script never holds a reference into the engine. Before each
  hook, `ScriptedRules` builds a `ScriptSnapshot` (time, pressed and held actions, entity ids
  by name, entity positions and visibility). The host types the script sees (`Game`, `Input`,
  `Entity`, `Sound`, `Vector`) read from that snapshot and queue `ScriptCommand`s for writes.
  The game applies the commands in order after the hook returns, the same way it applies
  `NativeRules`' commands. This keeps the C ABI boundary free of borrowed engine state and makes
  a script error unable to leave the world half-changed mid-step.
- **The API table** is documented at the top of `crates/scripting/src/lib.rs`. Entity ids cross
  the boundary as `i64` (the generational handle's bits). An unknown or stale id throws
  `Error:::NotFound` in the script instead of being ignored, and stale ids in commands are
  dropped on the Rust side.
- **Hooks** are methods on the script's `Application` class (`OnStart`, `OnFixedUpdate`,
  `OnTriggerEnter`, `OnPlayerFell`, `OnJump`, `OnLand`, `OnResetPressed`, `OnExit`). A hook is
  optional: the host scans the source for `Name~:` or `Name::` to decide whether to call it.
  This is a stopgap until the prototype exposes class reflection through its C ABI.
- **Feature gate and fallback.** Scripting is the `nyx-scripting` feature of the game (on by
  default). Without it, with `LANTERN_RULES=native`, or when the script fails to load, the game
  uses `NativeRules`, so it stays playable. A load error is shown on screen (a two-line banner
  plus the full text in the F1 HUD) and logged. The test `nyx_rules_match_the_native_rules`
  runs both rule sets over the same events and requires identical commands.
- **F5 reloads** the script from disk. If the new source fails analysis, the old rules stay in
  place and the error is shown; otherwise `OnStart` runs again.
- **Known gap: hook errors have no line numbers.** Load-time diagnostics carry file and line
  (for example `NYX0341 --> platformer.nyx:78:9`), but a run-time failure from a hook call only
  reports the error variant and message, because that is all the prototype's call API returns.
  Listed in `future-work.md`.
- **Verified** by the host API tests (`crates/scripting/tests/host_api.rs`), the equivalence
  test above, a headless run with the HUD showing `Nyx: platformer.nyx` and zero errors, and a
  deliberately broken copy of the script (an `i32 += 1.0f`) that shows the diagnostic on screen
  and falls back to the native rules without crashing.

## D-018 The world file is the seam between the game and the editor

- **One JSON file per world** (`assets/worlds/*.world.json`), read and written only through
  `lantern-world-format`. An entity has a stable `u64` id, a name, an optional parent, a `kind`
  string, a transform (position, Euler degrees applied Y then X then Z, scale), an optional
  shape (`Box` or `Ramp`), an optional material, tint and collider, and free-form `properties`
  (bool, number, text or 3-vector).
- **The format doesn't know the game.** `kind` and `properties` are plain strings and values.
  The game decides what `gem` or `period_seconds` means. Nyx scripts read them through the game
  host. The editor only displays and edits them.
- **Saved files are diffable.** Keys keep their order, numeric arrays stay on one line, floats
  print in their shortest form, and saving is atomic (write a temporary file, then rename).
  `cargo run -p lantern-world-format --example format-world` rewrites a file in that form.
- **The demo course was exported** from the old `level.rs` into `course.world.json` (29
  entities), so the game and the editor start from the same data. Switching the game to load it
  belongs to the "move all game code into Nyx" work, not the editor.
- **Additive changes only** while the game host is being moved over: new optional fields get
  defaults, and `format_version` changes for anything else.

## D-019 The game describes itself to the editor with a palette

- `assets/editor/palette.json` lists templates (for the Add buttons), marker styles (how to draw
  a shapeless entity of a given kind: cube, sphere, gem, capsule or plane, with a colour) and
  material names. The editor has no game kinds compiled in.
- A missing or broken palette falls back to generic blocks, ramps and markers, so the editor
  can open any world.
- **Why:** this keeps the editor isolated. A different
  game ships a different palette and uses the same editor binary.

## D-020 The editor's split: Rust logic and viewport, Nyx/Chaos/Cosmos panels

- **`lantern-editor-core`** holds every editing rule (undo history with coalescing and preview,
  selection, picking, gizmos, snapping, inspector fields, outline, input state machine, camera)
  with no window or GPU. It has 25 tests.
- **`lantern-editor-backend`** adds the off-screen renderer and a C API, built as a static
  library.
- **`apps/world-editor`** is a Penumbra application built like Pharos: a short C++ `main`, Nyx
  host functions that wrap the C API one to one, one native widget for the viewport, and the
  toolbar, outliner, inspector and status bar in `.chaos`, `.cosmos` and `.nyx`.
- **Why C++ in the middle:** Penumbra, Chaos's runtime and penumbra-ui-backend are C++, and
  Dean asked for the editor front end in Nyx, Chaos and Cosmos with a Rust backend. A C API is
  the narrowest seam between the two languages, and Lantern already uses one for Nyx (D-017).
- **Polling, not callbacks.** The front end reads `lantern_editor_revision()` every frame and
  refreshes its signals only when it changes. No callback crosses the C boundary, so there is
  no re-entrancy between Rust and Nyx.
- **Isolation:** the editor doesn't link the game, and the game doesn't link the editor. Both
  depend on `lantern-world-format` and the shared assets only.

## D-021 The viewport renders off-screen and is copied into a Penumbra texture

- `GpuRenderer::new_offscreen` opens an SDL GPU device without a window. `render_offscreen`
  draws the `RenderWorld` into a colour and depth target, copies the colour into a transfer
  buffer, waits on a fence and returns RGBA8 sRGB pixels. Targets are reused until the size
  changes.
- `LanternViewportWidget` calls `lantern_editor_render` at the widget's physical size, uploads
  the pixels into a streaming `SDL_Texture` and draws it. Input in logical pixels is scaled to
  physical pixels before it goes to Rust.
- **Why a read-back instead of sharing a GPU texture:** Penumbra draws with `SDL_Renderer`, and
  the editor's 3D view uses SDL GPU. SDL 3 has no supported way to share a texture between the
  two. The copy is about 6 MB a frame for a 1600×900 viewport and needs no changes to Penumbra. Sharing
  the texture is listed as future work if profiling shows the copy matters.
- The renderer is opened lazily on the first frame. If no GPU is available, the panels still
  work and the status bar says why the viewport is blank.

## D-022 The editor links the system SDL3, and the C API's rules

- The backend's default feature is `system-sdl` (pkg-config), unlike the game's bundled SDL
  (D-003). Penumbra links the system SDL3, and two SDL copies in one process would each have
  their own global state.
- The C API's rules are in `ffi.rs` and `docs/editor.md`: one thread, borrowed input strings,
  returned strings valid until the next string call, `int64_t` ids with 0 for none, and no panic
  crosses the boundary (it is caught and shown in the status line).
- **Keyboard:** Penumbra's `InputState` names only a few keys, so the viewport widget reads
  `SDL_GetKeyboardState` while the pointer is over it (or a drag is under way) and sends key
  transitions to Rust. Letters go by physical position, so WASD flying works on any layout.

## D-023 All game code is Nyx; engine code is Rust (2026-10-27)

Dean asked for every line of game code to be Nyx, with a clear seam where engine code (Rust)
begins. Before this, only the rules were Nyx and the game itself (`apps/platformer-game`: the
player, camera, menus, states, level, assets, input map) was Rust.

- **Engine** (Rust, `crates/`): the window, input devices, GPU, collision world, character
  controller, audio mixer, entity store, immediate-mode UI and text, debug HUD, frame loop, and
  `crates/game-host`, which exposes them to Nyx. Nothing in `crates/` names a game concept
  (gem, player, level, menu).
- **Game** (Nyx, `games/platformer/`): what the game is. Its player, camera, rules, menus, HUD,
  states, tuning, input map and the choice of assets.
- **The seam** is `crates/game-host`: host types registered through the `nyx` crate
  (`docs/game-api.md` lists every call) and five hooks on the game's `Application` class
  (`OnStart`, `OnFixedUpdate`, `OnUpdate`, `OnRender`, `OnExit`).
- **The rule for new code**: if a second game would want it unchanged, it is engine; otherwise
  it is game. The character controller is engine (any 3D platformer walks and jumps); its feel
  numbers are game (`settings/player-tuning.nyx` calls `SetWalkSpeed`, `SetJumpHeight`,
  `SetGravity`).
- **Undo**: none expected. A game feature Nyx can't express goes on the Nyx kanban's Game Engine
  epic (D-030), with a host call as the stopgap.

## D-024 Direct calls replace the snapshot and command queue

D-017's snapshot-in, commands-out design suited a script that only made decisions. A script that
is the whole game reads and writes the world many times per step, so host calls now act on the
engine state directly: `World:::SetPosition` moves the entity and its collision body at once.

- The engine state lives in an `Rc<RefCell<EngineState>>` shared by the host closures. Each call
  borrows it only for its own length and never calls back into Nyx, so borrows can't overlap.
- Handles still cross as `i64`, and a stale one still throws `Error:::NotFound`, so a script
  can't reach freed memory.
- A hook that fails part-way leaves its earlier writes in place. That is how any game code
  behaves, and the error is shown on screen.
- `NativeRules`, the Rust copy of the rules, is deleted: there is one implementation of the game.

## D-025 One generic runner: `lantern` and `game.json`

`apps/lantern` runs any game directory (`cargo run -p lantern -- games/platformer`, the default).
`game.json` names the window title, the entry file, the `Application` class and the assets
directory. The runner is the composition root (`lantern_game_host::run`); there is no
game-specific Rust binary. Tests use `GameHost` with a `HeadlessRenderer` and a scripted
platform instead (`crates/game-host/tests/headless_game.rs`).

## D-026 The level is the editor's world file

The course is `assets/worlds/course.world.json` (format: `crates/world-format`, shared with the
world editor). `World:::Load` builds its meshes, static collision and kinematic bodies; the game
finds entities by kind (`spawn`, `kill_plane`, `gem`, `moving_platform`) and reads their
properties (`period_seconds`, `travel`). So the engine knows shapes and colliders, and the game
alone knows what a gem is. Agreed with the editor thread on 2026-10-26.

## D-027 The engine keeps F5 and F1

F5 (reload the game from disk) and F1 or gamepad Back (debug HUD) are engine actions, defined
before the game's own and always working, even when the game failed to load. Everything else,
including F2's collision view, is the game's input map (`settings/input-settings.nyx`).

## D-028 Reload restarts the whole game

F5 calls `OnExit`, drops the Nyx runtime, releases everything the game made (entities, meshes,
textures, materials, music, input definitions, its debug values) and loads the game again from
`OnStart`. A game that fails to load shows the error, and F5 tries again. State is not kept
across a reload: the C ABI has no hot reload yet (Nyx kanban `CABI-F05-T05`). The headless test
`reloading_the_game_releases_what_it_made` reloads three times and checks that nothing leaks.

## D-029 Vectors are host value types

`Vec3` and `Vec2` are value types registered through the C ABI's value layouts
(`register_value_type` in the `nyx` crate): scripts build them with braces, read and write
`x`, `y` and `z` as fields, and get a copy at every boundary. Arithmetic is operators the host
declares (`+`, `-`, `*`, `/` and unary `-`, with the vector on the left), the answer Dean chose
for `GAME-F01-T02`: host value types may declare operators, Nyx types may not (yet). Angles use
Nyx's own `Math:::atan2`. Run-time hook errors carry the Nyx file, line and calling functions,
and optional hooks are found with `Program::defines_method`.

## D-030 Nyx gaps go on the Game Engine epic

The gaps found while porting are cards under the Nyx kanban's `[proto] Game Engine` epic in
fearless-hq (`game-engine`, feature `GAME-F01`), each naming the workaround Lantern uses today.
Two existing C ABI cards that Lantern also needs (`CABI-F05-T03` callables, `CABI-F05-T05` hot
reload) have a comment saying so. Measured script cost (`GAME-F01-T06`,
`cargo run --release -p lantern-game-host --example script_cost`): 0.24 ms of Nyx per frame on
average for the platformer, headless in the cloud container on 2026-10-27, with `OnFixedUpdate`
about 150 µs, `OnUpdate` 40 µs and `OnRender` 50 µs per call.

## D-031 The viewport is a Chaos primitive, not a native widget

The 3D view, the template thumbnails and the turntable preview are all `<viewport renderer="…">`
elements in `.chaos` files. The primitive is new in Penumbra and penumbra-ui-backend; a renderer
is anything that implements the interface library's `IViewportRenderer`: it fills an RGBA
buffer each frame and gets pointer, wheel, key and hover events in its own coordinates. The
editor registers three renderers by name (`level`, `preview`, `thumbnail:<template>`) from
`EditorHost::RegisterViewportRenderers` before the UI runs, and the Chaos tree picks them by
name. The old `LanternViewport` native widget is deleted. Each view is a renderer, named in
the UI, with no widget code per view.

## D-032 Editor UI state lives in one Nyx `Application`

Superseded by D-055.

`EditorApplication.nyx` holds every piece of UI state as `@signal` fields (open menu, context
menu, tooltip, panels, filters, dialogs) and the mirrored editor state (outline rows, inspector
groups, toolbar state, status). `OnUpdate` polls `editorRevision()` and refreshes the mirrors
when it changes, so the Rust session stays the single source of truth and the UI never caches a
stale entity. Every Chaos component reads the application through `EditorUi:::App()` and calls
its methods; no component keeps editing state of its own. The one exception is per-component
hover, which is a Penumbra event.

## D-033 Menus, popups, tooltips and dialogs are portals built from frames

Penumbra has no menu bar, context menu or dialog widget. Rather than add five widgets, the
backend gained one `<portal>` primitive (an absolutely positioned overlay that can dismiss on an
outside click) and the editor builds the menu bar, dropdown menus, the right-click menu,
tooltips, the command palette, the settings dialog, the confirm dialog and the help pages from
frames inside portals. Menus are data (`editor-menus.nyx` returns `MenuEntry` lists with a
command name, a shortcut label, an enabled flag and a check mark), so one `MenuPopup` component
renders them all and one `RunCommand` dispatches every command, whether it came from a menu,
the toolbar, the palette or a shortcut.

## D-034 Shortcuts are split between Nyx and the viewport

Keys reach the editor twice: Penumbra sends them to the root `onKeyDown`, and the viewport
renderer gets them while the pointer is over it. The viewport (Rust, `editor-core`) owns the
editing keys (tools, framing, delete, undo, duplicate, bookmarks, hide, fly). The Nyx
application owns application-level shortcuts (new, open, save, quit, play, palette, settings,
lock, invert selection) and forwards everything else to the viewport through `editorKey`, so a
shortcut works wherever the pointer is. Modifier state is tracked in Nyx from the `Shift`,
`Control` and `Meta` key events; the platform's "shortcut" key is `Shortcut+` in the key names
Penumbra delivers (Cmd on macOS, Ctrl elsewhere).

## D-035 Preferences are a flat JSON file

`~/.config/lantern/world-editor.json` (or `XDG_CONFIG_HOME`, `APPDATA`) holds string values
keyed by name: window size, snapping steps, view options, the camera bookmarks, the bottom
panel that was open and the recent files list. The Rust session reads and writes it
(`EditorPreferences`), the UI reads and sets keys through `editorPreference` and
`editorSetPreference`, and the file is saved on quit and from the settings dialog. A missing
or unreadable file is ignored.

## D-036 Play in editor runs the real game on a scratch level

The Play button saves the level to `.editor-play.world.json` beside the game's worlds and
launches the `lantern` runner on it with `LANTERN_LEVEL` set, so unsaved edits are what plays
and the game's own loader is what loads them. The game reads `LANTERN_LEVEL` through
`Engine:::Setting` and falls back to its configured level. Stop kills the child process. The
editor never links the game.

## D-037 The stack is pinned by branch, overridable per checkout

`apps/world-editor/cmake/Dependencies.cmake` fetches Penumbra, Chaos and Cosmos from `proto`
and penumbra-ui-backend and Cimmerian from `main`. `LANTERN_STACK_BRANCH` overrides all of
them at once and `LANTERN_<DEP>_BRANCH` one at a time, and CMake's own
`FETCHCONTENT_SOURCE_DIR_<DEP>` points one at a local checkout. That is how the editor is
built against unmerged stack branches while their pull requests are open.

## D-038 Editing operations are editor-core methods with one undo entry each

Every operation the UI offers (duplicate along an axis, mirror, align, distribute, group, snap
to ground, revert to or apply to a template, re-parent, rename a property, hide and lock) is a
method on `EditorSession` that records one labelled history entry, and the C API exposes each
as one function. The history panel lists those labels and jumps to any point. Nothing in C++
or Nyx composes edits, so the undo history is the same no matter which panel issued the
command, and a headless test can drive every operation.

## D-039 Thumbnails and previews are rendered by the backend

The asset browser's thumbnails and the turntable preview are real renders of each template
through the same `GpuRenderer` as the viewport (`lantern_editor_thumbnail`,
`lantern_editor_render_preview`), not icons. A thumbnail is rendered once per template at the
size the card asks for; the preview is rendered every frame it is visible, turned by the
elapsed time. Each is its own `IViewportRenderer`, so the UI treats them as viewports.

## D-040 Penumbra's closed dropdown draws its label

`Penumbra::Widgets::Dropdown` drew only an item's leading visual when closed, which left a
text-only dropdown blank. It now draws the selected label and a chevron, and measures its
width from the widest option. This changed Penumbra rather than the backend because every
Penumbra app wants a readable dropdown. Checkbox glyph size comes from the class's
`font-size` for the same reason.

## D-041 The hosts describe their names to nyx-lsp

nyx-lsp and `nyx check` can't load a host, so in an editor every import across the game's folders
and every engine name showed as an error. Each Nyx tree now has a `.nyx.json` (the game's at
`games/platformer/`, the editor's at `apps/world-editor/src/ui/`) naming a host surface manifest
written from the real registrations: the game host's through Nyx's C API
(`nyx_runtime_write_host_surface`, checked by `crates/game-host/tests/host_surface.rs`), the
editor's by `lantern-world-editor --host-surface` after every build. Both are committed, so a fresh
clone's editor is right before anything is built. The engine's debug type is `EngineDebug`, since
Nyx's own built-in `Debug` (`Debug:::describe`) took that name and the host can't register it.

## D-042 The game code reads as a tutorial

Dean asked for the game code to explain itself, above all its maths. Every file in
`games/platformer/` now opens with what it is for and where it sits in the frame (fixed step,
display frame or render), and every calculation says what it computes and why: the two clocks
and `alpha`, the camera's spherical coordinates and forward/right vectors, mouse versus stick
look, turn clamping with angle wrapping, the cosine ease of the platforms, the gem bob, the
capsule-versus-sphere touch test and the blob shadow's sizing. Maths the engine does on the
game's behalf (take-off speed v = √(2gh), acceleration, variable jump height, coyote time and
the jump buffer) is explained from the game's side in `settings/player-tuning.nyx`, with a
pointer to `crates/physics/src/controller.rs`, rather than by adding comments to the Rust.
Comments are plain `//` lines, as before, and no code changed: the files are identical once
comments are stripped. Card: `platformer-game/tutorial-comments-in-the-game-code` on the
lantern board.

## D-043 The game's UI is Chaos and Cosmos

Dean asked whether Chaos and Cosmos, so far run only on Penumbra, work for the game's own UI.
They do: the main menu, pause menu and HUD are now `.chaos` components and `.cosmos`
stylesheets in `games/platformer/ui/`, and the old `Ui:::` menus are gone from the game.
Cards: `platformer-game/in-game-ui-with-chaos-and-cosmos` on the lantern board, gaps on the Nyx
(GAME-F04), Chaos and Cosmos boards (epic `lantern-in-game-ui-gaps`).

- **Backend shape.** Chaos and Cosmos are C++, so a small C++ bridge (`crates/chaos-ui/native`)
  is the Chaos backend: each element is a plain node implementing the interface library's `IWidget`,
  the contract Chaos reconciles against, and `Cosmos::IStyleTarget`. The bridge
  hands Rust a flat snapshot of nodes with resolved styles over a C API; layout, drawing and
  focus are Rust (`lantern-chaos-ui`). That keeps the engine's drawing in one language and the
  C++ surface small. Penumbra isn't used: it is a desktop widget toolkit with its own renderer.
- **One Nyx.** Chaos must run on the game's own runtime so components can read and write game
  objects. One CMake superproject builds Nyx, Chaos, Cosmos and the bridge together (nyx-sys's
  `chaos-ui` feature), and the bridge reaches the C++ runtime behind the C API's handles with
  Nyx's new `nyx-host-interop.hpp` (nyx PR). `NYX_CAPI_PREFIX` can't be used with it.
- **Mounting.** `GameUi:::Mount(file, component)` passes the `Application` object as the
  component's argument, because a host signature can't name a game class or a generic object.
  State is `@signal` fields on a game object (`GameUiModel`); writing one re-renders the
  `<slot>`s that read it.
- **Element type.** Elements are `lantern:::ui:::Element` (`RegisterElementType`), as
  Penumbra's are `Penumbra:::Component`. Dean chose the name (D-044).
- **Layout defaults.** A frame with no `display` stacks its children in a column, like a menu;
  `display: stack` with no `flex-direction` is a row, as in Cosmos. Children stretch across by
  default. A Cosmos `px` is one window point (half of `Screen:::UiScale()` pixels), so a
  1280×720 window and a 2560×1440 window lay out alike. `font-size` snaps to multiples of 8
  because the font is an 8×8 bitmap scaled by whole pixels.
- **Focus.** Elements with `onPress` or `onRelease` are buttons, focused in document order with
  the game's own navigation actions (`GameUi:::SetNavigation`). The focused button and its
  descendants draw with their `:hover` style, and `:active` while confirmed, so Cosmos needs no
  focus state yet (a card asks for `:focus`). The mouse came later (D-044).
- **Frame order.** Navigation and its handlers run just before `OnUpdate`, outside any host call,
  so a handler can call the engine; the UI ticks and is read back after `OnStart` and after
  `OnRender`, then drawn on top of the game's overlay. A choice made in a handler is acted on in
  the same frame's `OnUpdate`.
- **One shared component sheet.** Every `ui/*.cosmos` except `global.cosmos` is merged into
  Cosmos's component layer, so the files act as one sheet rather than each belonging to its own
  component. It is enough while class names are unique; a sheet per component (matched by file
  name) is a card on the lantern board.
- **One UI per process.** Chaos's reactive runtime is a process-wide singleton, so
  `lantern-chaos-ui` allows one `ChaosUi` at a time (a lock, which keeps parallel tests apart).
  A Chaos card asks for a runtime per instance.
- **Gaps found and filed:** a stray `}` hung the Cosmos parser (fixed in cosmos), a rule nested
  in a `:hover { }` block applies always (cosmos card; use `.label:hover`), no `:focus`, no
  text shadow (cosmos cards), the singleton runtime (chaos card), and the C API interop (Nyx
  GAME-F04-T01, done).

## D-044 Mouse in the in-game UI; Ui::: removed; elements are lantern:::ui:::Element

Dean answered the three questions D-043 left open.

- **Mouse: yes.** The platform's mouse events now carry the pointer's window position (from
  SDL's event `x`/`y`), and the game host tracks the left button from them
  (`GameHost::handle_event`). Each frame it hands the UI the pointer in pixels with the
  navigation edges, or none while the game has captured the mouse, since a captured mouse
  steers the camera. `lantern-chaos-ui` hit-tests the boxes it drew last frame, the last
  button in drawing order winning. Moving over a button focuses it (`onFocus`). Pressing on a
  button calls `onPress` and draws it `:active` while the pointer stays on it, releasing on the
  same button calls `onRelease`, and releasing elsewhere cancels, as on the desktop. A pointer
  that hasn't moved never takes focus, so keyboard and gamepad navigation aren't fought by a
  resting mouse. Buttons are hit in node order, not by `z-index`; nothing in the game overlaps.
- **`Ui:::` removed.** The immediate-mode menus (`lantern_renderer_3d::ui` and the `Ui` host
  type) had no users left, so there is one way to build UI. `Overlay:::` stays for quick text
  and rectangles; its registration moved to `api/overlay.rs`.
- **Element type `lantern:::ui:::Element`.** Components return `lantern:::ui:::Element`. The
  name says where an element comes from: a game element is the engine's, and an application
  element is `Penumbra:::Component`, so the two never read alike in one codebase. Engine names
  start with `lantern:::`, and namespaces are lowercase; `Element` is PascalCase as a type. Nyx takes a three-part name as an element type as it is.

Cards: `mouse-hover-and-click-in-the-in-game-ui`, `remove-the-immediate-mode-ui-api` and
`rename-the-element-type-to-lantern-ui-element` under `platformer-game/in-game-ui-with-chaos-and-cosmos`.

## D-045 Every engine name is under lantern:::

Dean asked that every name a script gets from the engine start with `lantern:::`, with lowercase
namespaces and PascalCase types, and that Lantern never name another engine.

- **Functions.** Each API group registers its calls as host functions under one namespace
  (`HostNamespace` in `api/mod.rs`): `Input:::IsDown` is now `lantern:::input:::IsDown`,
  `GameUi:::Mount` is `lantern:::ui:::Mount`, `EngineDebug:::` is `lantern:::debug:::`, and so on,
  one lowercase namespace per old group. They were static methods of empty marker types before,
  which made `Input` a type a script could name.
- **Types.** `Vec3` and `Vec2` are `lantern:::math:::Vec3` and `lantern:::math:::Vec2`;
  `RayHit`, `ControllerStep` and `CharacterController` are under `lantern:::physics:::`; the UI
  element is `lantern:::ui:::Element`.
- **Nyx.** Namespaced host functions and types needed Nyx changes (nyx PR #19, Nyx board
  GAME-F05-T01). Nyx can't yet build a namespaced value type from braces with the path in front
  (`lantern:::math:::Vec3 { y: 1.0f }`, GAME-F05-T02), so the game builds vectors from braces
  where the type is known: a typed local, an argument or a return value.

Card: `naming/lantern-names/move-the-engine-api-under-lantern`.


## D-046 The Nox engine links Nyx and the Chaos bridge as one library

The Nox engine (`engine/`) needs the Chaos UI bridge that `crates/chaos-ui` builds for the Rust
engine. The bridge reaches Nyx's internals through `nyx-host-interop.hpp`, so it has to share one
copy of Nyx with the C API the engine calls.

- **One library.** `engine/native/` builds `liblantern-native`: Nyx's C API (linked whole), Chaos,
  Cosmos and the bridge, exporting only the `nyx_` and `lantern_chaos_ui_` functions. Both
  `generated/nyx.nox` and `generated/chaos-ui.nox` link it, and `libnyx-capi` isn't used any more.
- **Current upstreams.** It builds from nyx-scripting-language, chaos-ui-framework and
  cosmos-styling-language on `proto` beside lantern, so the bridge moved to Chaos's current names
  (`Equinox::IWidget`, `include/chaos`, `include/cosmos`). `crates/chaos-ui` keeps its pinned
  commits and its own copy of the bridge.
- **One call added.** `lantern_chaos_ui_snapshot_node` copies one node of the last snapshot, since
  Nox can't index a C array through a raw pointer.
- **Chaos's target.** The bridge sets `ChaosBuildTarget::GameEngine`, the name Chaos gives a game
  engine's target. `UmbraEngine` stays in Chaos as an alias of it.

Story: `engine-on-nox/l7-game-ui/port-chaos-ui-the-native-library-its-binding-nodes-styles-layout-and-drawing`.

## D-047 The Nox game host keeps the game's UI in a GameUiState the session owns

The Rust host shares its Chaos UI, the game's application value and the navigation actions with the
`lantern:::ui` functions through an `Rc<RefCell<GameUiState>>`. Nox has no shared ownership, so the
Nox `ScriptSession` owns one `GameUiState` and the `lantern:::ui` functions reach it through a raw
pointer captured when they're registered, the way the other APIs reach `EngineState`.

- **Order.** The session deletes the `GameUiState` first (the Chaos UI, then the application value),
  then the program, then the runtime, as the Rust `Drop` does. Every failed load deletes it before
  the runtime too, since Chaos allows one UI per process and a leaked one would stop every reload.
- **Host surface.** `EngineHostSurface` starts Chaos as the Rust one does, so the Nox manifest
  matches all 153 rows of the committed one.
- **Tests.** The Rust `headless_game` tests that play the platformer are `tests/host` programs over
  `tests/support/platformer/`. The scripted platform gives one event per frame, so a click is three
  frames (move, press, release) where Rust queued the press and release together.

Story: `engine-on-nox/l7-game-ui/wire-the-game-ui-into-the-game-host`.

## D-048 The Nox editor model keeps the Rust shapes, with sorted arrays for maps

`engine/src/editor/` ports `editor-core`'s document, palette, preferences, camera and scene geometry
one type for one type. Where the Rust leans on something Nox doesn't have, it does this instead:

- **Maps.** The palette's markers and the preferences are `BTreeMap`s in Rust. In Nox they're two
  arrays kept sorted by key, so they save in the same order.
- **Templates.** An `EntityTemplate` holds its partial entity as a `WorldEntity` with id 0 (its
  `prototype`), so a new entity starts as a copy of it. Reading one reuses the world file's
  `WorldReader`, and writing one reuses the world file's shape and property writers.
- **Equality.** `WorldEntity` gets `Copy` and `SameAs` in place of Rust's derived `Clone` and
  `PartialEq`, which a preview commit uses to leave out entities that didn't change.
- **Tests.** The camera, scene geometry and preferences tests are the Rust ones. `document.rs` and
  `palette.rs` had none, so `the-document-undoes-redoes-coalesces-and-previews` and
  `the-palette-loads-saves-and-falls-back` are new.
- **Saved palettes.** A saved palette holds the same JSON as Rust's, but it's laid out as the world
  file is (`1` and `[1, 0.5, 1]`), where serde's pretty printer writes `1.0` and one array element
  per line. Saved preferences match Rust byte for byte.

Story: `engine-on-nox/l8-editor/port-editor-core-s-model-document-palette-preferences-camera-and-scene-geometry`.

## D-049 Nox gizmo handles are a struct, and the viewport build waits for the session

`engine/src/editor/Gizmo.nox`, `Inspector.nox` and `Viewport.nox` port `gizmo.rs`, `inspector.rs`
and the drawing half of `viewport.rs`.

- **Handles.** Rust's `Handle` enum carries an axis in two of its variants. In Nox it's a `Handle`
  struct with a `HandleKind` and an axis, compared with `Same`, since optionals have no `==`.
- **Gizmo frame.** `GizmoFrame` keeps its three axes as `xAxis`, `yAxis` and `zAxis` fields.
  `EulerDegrees` ports glam's YXZ path: it builds the rotation matrix's columns and takes
  `atan2`s, and when the pitch is within glam's tolerance of ±90° it puts the whole turn into yaw
  and sets roll to 0, as glam does. Degrees use Rust's `to_degrees` constant, `57.29578`.
- **Inspector.** Choices are an array on the `InspectorField`, not a payload on `FieldKind`.
  Numbers parse with Rust's `f32` grammar (sign, digits, an optional point and exponent, or `inf`
  and `nan`) checked before `strtod` reads them, and they're shown as Rust shows them.
  Two known gaps: `strtod` reads to f64 and then rounds to f32, where Rust rounds once, so an
  input very close to an f32 halfway point can land one step away; and above about 10⁶,
  `FormatNumber` can print a different last digit from Rust's shortest form (`1585384.2` against
  `1585384.3`), the same shortest-digits gap the L4 float formatting has.
- **Viewport.** `build_viewport` reads the session's selection, hover, view options and gizmo, so
  it moves with the session in the next story, along with its two tests. This story ports what
  it draws with: `ViewportResources`, marker matrices and bounds, oriented outlines, the grid, the
  plane marker, arrows and the rubber band.

Story: `engine-on-nox/l8-editor/port-editor-core-s-interaction-viewport-picking-gizmos-and-the-inspector`.

## D-050 The Nox editor session takes its drag out of the session while it works on it

`engine/src/editor/SessionTypes.nox` and `Session.nox` port `session.rs`, and `Viewport.nox`
gains `BuildViewport`, the half of `viewport.rs` that reads the session.

- **Drags.** Rust's `Drag` enum carries different fields in each variant. In Nox it's one `Drag`
  class with a `DragKind`, owned by the session as `drag`. While a pointer move or release works on
  a drag, the session moves it out of `drag` and puts it back afterwards, so previewing an edit on
  the document doesn't borrow the session twice. A drag's `originals` (each entity's copy, parent
  matrix and world position) are owned by the drag and freed with it.
- **Keys and buttons.** `EditorKey` is a struct with a kind, a letter and a digit, since Nox enums
  carry no payload. Held letters are a small array of bytes. The pointer button method is
  `SetPointerButton`, because a method can't share its name with the `PointerButton` enum.
- **Scaling.** Rust passes a closure that changes the scale array. Nox passes the factor for each
  component (1 for the axes left alone), which gives the same values, since multiplying by 1 is
  exact.
- **Viewport.** `BuildViewport` takes the session, a `ViewportResources` and a `RenderWorld`, and
  draws the same things in the same order as `build_viewport`.
- **Tests.** `tests/editor/the-session-drives-clicks-drags-keys-and-edits` ports the 19 tests in
  `editor-core/tests/session.rs`, and `the-viewport-draws-the-course-and-follows-view-options`
  ports `viewport.rs`'s two. Both pass on the native and C backends, and the session test runs
  under valgrind with no leaks.

Story: `engine-on-nox/l8-editor/port-editor-core-s-session-and-its-tests`.

## D-051 The Nox editor backend is a static library behind the same C header

`engine/src/editorbackend/` ports `crates/editor-backend` (`lib.rs`, `resources.rs`, `preview.rs`
and `play.rs`), and `engine/editor-backend/` ports its C API (`ffi.rs` and `ffi_tools.rs`).

- **The C header is unchanged.** `engine/editor-backend/include/lantern_editor.h` is the Rust
  crate's header, so `apps/world-editor` needs no change beyond which library it links. Nox exports
  each function from `lantern:::editorapi` (`EditorApi.nox`) as `nox_lantern__editorapi__<Name>`,
  and a small C file (`shim/lantern_editor.c`) implements the header over them. It keeps the last
  returned string NUL-terminated until the next call, and copies pixels into buffers it owns,
  because Nox hands strings over as pointer and length and its arrays can't be lent to C past a
  call. `nox build --static-library` writes the library and its header from
  `engine/editor-backend/nox.json`, which names only the engine directories the editor needs.
- **Rendering off-screen.** `GpuRenderer:::CreateOffscreen` opens the GPU device without a window
  (it initialises SDL's video subsystem itself and quits it when the renderer is destroyed), and
  `RenderOffscreen` draws a `RenderWorld` into a colour target of the requested size, kept per
  size, and reads it back as RGBA rows, top row first, into an array the caller lends it. The
  backend keeps one array for the viewport and one for previews, so a frame of the same size
  reuses its memory, and the report says the frame was presented, as Rust's did.
- **Ownership.** The backend's GPU resources, play session and render worlds are owned by the
  backend. A method that needs two of them at once (the session and the render world, say) moves
  one out of its field while it works and puts it back, as the session does with its drag (D-050),
  because the borrow checker counts every field reached through `this` as one borrow.
- **Shaders.** The game finds its shaders beside its executable. The editor's executable lives
  in a CMake build directory, so the C layer sets `LANTERN_SHADER_DIRECTORY` to
  `engine/shaders/compiled/` when the variable isn't set already.
- **Play in editor** runs the Nox game runner, chosen as before (`LANTERN_RUNNER`, then `lantern`
  beside the editor, then `engine/build/lantern` in the workspace), as
  `lantern games/platformer` through SDL's process API. `LANTERN_LEVEL` and `LANTERN_ASSETS` are
  set on a copy of the editor's environment that only the child gets, so the editor's own
  environment never changes.
- **Matching Rust's edges.** A preview of an unknown template fails with Rust's message before
  the GPU is touched, a preview's size is recorded even when it fails, a preferences path that is a
  directory is ignored, and the viewport's shape cache compares floats by their bits, as Rust's
  derived equality on `f32::to_bits` did.
- **Errors.** Rust caught panics at the C boundary and showed them in the status line. A Nox trap
  ends the process, so the C header's rule now says only that errors show in
  `lantern_editor_status`.
- **Tests.** `engine/editor-backend/tests/lantern_editor_test.c` ports `backend.rs`'s three
  tests through the C API, plus previews and thumbnails, and passes with a real GPU (lavapipe).
  `engine/tests/editorbackend/` ports the preview and workspace tests, and
  `engine/tests/gpu/an-off-screen-renderer-draws-without-a-window` covers the off-screen renderer.
  `-DLANTERN_EDITOR_BACKEND=rust` links the Rust backend instead, for comparison.

Story: `engine-on-nox/l8-editor/port-editor-backend-behind-lantern-editor-h-as-a-nox-static-library`.

## D-052 The view cube is drawn by the Nox editor into the viewport's overlay

The world editor's viewport shows a cube in its top-right corner that turns with the camera.
Clicking a face applies the camera preset that puts the camera on that face's side.

- **In the backend, not the shell.** `engine/src/editor/ViewCube.nox` lays the cube out and hit
  tests it from the orbit camera and the viewport size alone. `BuildViewport` draws it into the
  render world's 2D overlay, and the session handles hover and clicks. So `apps/world-editor` and
  `lantern_editor.h` don't change, and the cube shows in the off-screen frame like everything else.
  The Rust backend, kept only for comparison, has no cube.
- **Rotation only.** Each face's corners are projected with the camera's right and up vectors,
  without perspective, so the cube keeps its size whatever the zoom. A face is drawn when it points
  at the camera. The visible faces of a cube never overlap in this projection, so the hit test takes
  the first visible face that contains the point.
- **Faces are named after their presets.** +Z is front, -Z back, +X right, -X left, +Y top and
  -Y bottom.
- **The left and right presets swap sides.** The front view puts the camera on +Z with +X on the
  screen's right, so the right view now looks from +X and the left view from -X, as Blender, Maya
  and Unity do. Before, as in the Rust editor, "left" looked from +X, which the cube made plain: in
  the default view its right-hand face read LEFT.
- **Labels** use the font atlas, which `ViewportResources` now provides as `FontTexture`. A label is
  shown only on a face turned well towards the camera, so it stays readable.
- **Input.** A left click on a face applies its preset and starts no drag or selection. Hovering
  the cube highlights the face and hovers nothing behind it.

Card: `world-editor/follow-ups-after-the-first-full-pass/clickable-view-cube-in-the-viewport-corner`.

## D-053 Clicking the same spot again selects the next entity behind

A plain click selects the nearest entity under the cursor, as before. Clicking again within the
click slop, while that click's entity is still the whole selection, selects the nearest entity
under the cursor that this run of clicks hasn't reached yet. Past the back of the stack it wraps
to the front.

- **On release, not press.** The press still picks the front entity, so pressing and dragging
  moves what's under the cursor, whatever an earlier click reached.
- **What starts over:** a click elsewhere, a Shift- or Command-click, a click on empty space, any
  change to the camera (orbit, pan, zoom, fly or a preset, since the same pixel then casts another
  ray), a drag, or a selection made any other way (the next click no longer finds its entity selected
  alone). A press on the selection's gizmo is a gizmo drag, as before, and also starts over.
- **Status line.** When more than one entity lies under the cursor, it names the selection and
  its place in the stack: "Selected low_ceiling (1 of 2 under the cursor)". Hidden and locked
  entities, and markers while they're hidden from view, are skipped as they are for a plain click.

Card: `world-editor/follow-ups-after-the-first-full-pass/selection-cycling-through-overlapping-entities-on-repeated-clicks`.

## D-054 Outliner rows reorder by dropping on the line between them

Dragging a row in the outliner now does something: Penumbra gained a drag session (penumbra-ui-engine's
`DragDispatch`) and penumbra-bridge maps Chaos's `dragPayload`, `onDragStart`, `onDragEnd`, `onDragEnter`,
`onDragLeave` and `onDrop` onto it. Before that the rows' drag props were inert.

- **Three drop places.** A row's middle parents the dragged entity under that row, as before. While a drag is
  going, each row shows a thin drop line across its top edge (an absolutely positioned 8px strip, so the rows
  don't move); dropping there moves the entity just before that row, under the row's parent. The empty tail
  under the list moves it to the end of the root.
- **One edit.** `EditorSession::MoveBefore(child, sibling)` removes the entity and inserts it at its new place in
  the world's entity array, so one undo puts it back. A move to another parent keeps the entity where it is in
  the world, as `SetParent` does, and a move in among its own descendants is refused.
- **Order is the array order.** The outline lists siblings in the order the world file holds them, so moving
  in the outline saves as a new order in the file.
- **Highlights.** The application keeps the drag (`outlineDragging`) and the hovered drop place
  (`outlineDropTarget`) as signals. The hovered row gets an accent border and the hovered line turns accent.
- **C++ Penumbra only.** The Nox Penumbra port isn't changed; it's replaced as a later, separate pass.

Card: `world-editor/follow-ups-after-the-first-full-pass/reorder-siblings-by-drag-and-drop-in-the-outliner`.

## D-055 EditorApplication only sets up the application; behaviour lives in services and components

Supersedes D-032. Dean (2026-11-05): `EditorApplication` is the application layer, and component logic in it
gives it more than one responsibility.

- **EditorApplication** sets the window title and size, loads the font for the DPI scale, mounts `App.chaos`,
  calls `TickChaos` each frame and saves preferences on shutdown. Nothing else.
- **Services.** `Editor.nyx` builds one of each and wires them together: `EditorDocument` (polls
  `editorRevision()` and holds what the panels mirror: toolbar, status, history, log, recent files, inspector
  groups, the window title), `EditorCommands` (every command, the save-first confirm and the file dialogs),
  `EditorOverlays` (menus, context menu, palette, settings, help, confirm, and what Escape closes),
  `EditorKeyboard` (shortcuts, modifiers, forwarding keys to the viewport), `EditorSelection` (click-to-select
  with modifiers, the hovered entity, dropping a template on the viewport) and `EditorLayout` (the bottom panel).
  Their state is `@signal` fields, so a slot that reads one re-renders when it changes.
- **How components reach them.** `App.chaos` declares `@provide editor~ Editor = new Editor();` and its
  `OnTick` local calls `editor.Tick`. Every component that needs a service reads `@context editor~ Editor;`,
  Chaos's required context: it takes App's `Editor`, and Chaos reports CHA0620 if a component renders where
  nothing provides one. An earlier version gave each `@context` a default (`= new Editor()`), which isn't
  harmless: a default is evaluated on every render before the provided value replaces it, so every component
  render built a whole `Editor` (about 25 host calls). That needed Nyx to let a decorator assign a deferred
  local (nyx-scripting-language #4) and Chaos's required `@context` (chaos-ui-framework #4). `OutlinerRow` and
  `ConsolePanel` read their panel's `OutlinerState` and `ConsoleFilters` the same way.
  Measured with penumbra-bridge's `nox/tools/measure-editor.sh` (headless, 600 frames, a change every 30):
  host function calls were 9,484 / 36,844 on nox-proto at 29 / 200 entities, 31,665 / 150,168 with the
  defaults, and 9,690 / 38,418 with the required context; a frame after a change took 28 / 101 ms, 34 / 126 ms
  and 29 / 86 ms (means, one run each, noisy).
- **Panel state stays in the panel**, or in a small object it provides to its rows: `OutlinerState`
  (filter, collapsed rows, the drag and its drop target), `ConsoleFilters` (provided by `BottomPanel`, so the
  filters survive switching tabs), the inspector's collapsed groups and new-property name, the asset preview,
  the palette query and the settings tab. The settings dialog now opens on its first tab each time.
- **The window.** Services call penumbra-bridge's `penumbra:::` functions (`mouseX`, `windowWidth`,
  `setWindowTitle`, `requestQuit`, the file dialogs) instead of `Application` methods, so nothing but the
  application holds the application. The C++ host and the C API the Nox editor uses both register them.

Card: `world-editor/follow-ups-after-the-first-full-pass/components-own-their-state-not-editorapplication`.

## D-056 The Nox host is the world editor's only host

The C++ host (`apps/world-editor/src/main.cpp`, `EditorHost.cpp`, `src/native/` and the CMake project with its
`LANTERN_EDITOR_BACKEND` option) is gone. Dean ran the Nox host on his Mac on 2026-11-05, and it does everything the
C++ one did. The editor runs only from `apps/world-editor/nox`.

- **What stays.** `src/ui` (the Chaos, Cosmos and Nyx UI) and `data/fonts` (Inter), which the Nox host loads.
- **The host functions.** `nox/src/EditorFunctions.nox` was generated from `EditorHost.cpp`. It is now the source,
  edited by hand, and its generator is gone.
- **The host surface.** `src/ui/editor-host-surface.json` is written by `lantern-world-editor --host-surface` from
  the Nox host, by hand after a host function changes, rather than after every CMake build. Its function rows now
  name their parameters. It no longer lists C++ Penumbra's own types (`PenumbraWidget`, `WidgetRef`,
  `PenumbraFontBackend`, `PenumbraLifecycleRegistry` and their methods), which nothing in `src/ui` uses.
- **The C API.** `engine/editor-backend`'s `lantern_editor.h`, its C shim and C test stay. Nothing in the editor uses
  them now, since the Nox host imports `lantern:::editorapi` directly, but they're how the Nox and Rust backends are
  compared.

Card: `world-editor-on-nox-penumbra/mac-run-and-retire-the-c-editor-host`.

## D-057 The engine is laid out one folder per domain, with SDL behind a facade folder

Dean asked for the engine to be a clear reference for writing a renderer, with a clean application and game loop,
laid out by domain. `engine/src/` now has a folder per domain (`core`, `platform`, `renderer`, `engine/loop`,
`engine/input`, `engine/entities`, `engine/audio`, ...), everything that calls SDL in `sdl-facade/` split by the domain
it serves, and one composition root in `application/`. `engine-structure.md` has the map; `renderer.md` and
`application-and-game-loop.md` are walkthroughs. Behaviour didn't change: 189 of 189 tests pass and the game, the stub
and the editor run as before.

- **RenderBackend lives with the renderer.** It moved from `lantern:::host` to `lantern:::renderer`, with
  `RenderReport`, `HeadlessRenderer` and `Frustum`. `GpuRenderer` implements it directly, so the `GpuRenderBackend`
  adapter and the duplicate `FrameReport` are gone.
- **GpuRenderer is split by job**: `GpuDevice.nox` (device and swapchain), `GpuUniforms.nox` (the uniform blocks),
  `GpuDrawList.nox` (draw order), and the frame into `DrawMeshes`, `DrawLines` and `DrawOverlay`.
- **One Application.** `Application` starts SDL, opens the window and the platform, hands out the renderer and the
  audio output, runs a `GameLoop` (formerly `FrameLoop`) and shuts down in order. `runner/` and `main.nox`'s copy of
  the same set-up are gone.
- **The application sits above engine/**, because tests build `engine/` without a GPU, the host or SDL.
- **No dependency-injection container**: the application passes each object on, as before. File names stay
  PascalCase, and namespaces kept their names where only the folder moved.
- **Tutorial comments** in the renderer, the application and the loop, as the game code has (D-042).

Card: `engine-follow-ups/engine-structure-and-a-renderer-to-learn-from`.

## D-058 Lights are plain data in RenderWorld: one sun, up to eight local lights, a PBR-lite material

Dean asked for proper lighting and shadows so a full 3D platformer can be built on the engine. The light model stays
data in the `RenderWorld`, like everything else the renderer reads, and nothing above `renderer/` touches the GPU.
(Provisional, 2026-11-16, not yet reviewed by Dean.)

- **One sun plus local lights.** `DirectionalLight` stays the sun and the ambient term. A new `Light` struct covers
  point and spot lights (position, direction, colour times intensity, range, cone angles, a shadow flag).
  `RenderWorld.lights` holds them; the shader takes the eight that matter most to the camera
  (`MAX_LOCAL_LIGHTS`). Many lights would need clustered or deferred shading, which this tutorial renderer leaves out.
- **Forward shading, PBR-lite.** Cook-Torrance with GGX, Smith visibility and Schlick Fresnel, driven by a metallic
  and a roughness value, plus an emissive colour. Chosen over Blinn-Phong because it is a few more lines and
  behaves under any light, and over full PBR because there are no image-based lights, normal maps or clear coat.
- **Material on the draw.** `DrawItem` gets a `Material` (metallic, roughness, emissive) with defaults that look
  like the old matte surfaces, so existing games and tests don't change. The tint stays on the draw.
- **Ambient is a sky and ground colour pair**, blended by the normal's height, instead of one colour times a fixed
  hemisphere factor. The old single `ambient` stays valid and is used for both.
- **Nyx sees it as `lantern:::lights`**, with handles for local lights, so game code sets the sun, adds, moves and
  removes lights and sets a material; the engine remains Nox.

Card: `lighting-and-shadows/light-model-and-materials`.

## D-059 Shadows come from depth maps in two atlases, filtered with PCF

(Provisional, 2026-11-16, not yet reviewed by Dean.)

- **A depth-only pass before the main pass.** Casting draws are rendered from the light into a depth texture that the
  lit shader then samples with a comparison sampler.
- **Atlases, not texture arrays.** Cascades, spot lights and point-light faces are tiles of one depth texture each,
  drawn by setting the viewport per tile inside a single render pass. One texture and one pass per atlas keeps the
  code short and the resource count low.
- **Sun: three cascades** fitted to slices of the camera's view, with texel snapping so edges don't shimmer.
- **Local lights:** the shadowed spot lights use one tile each; a shadowed point light uses six (a cube laid out as
  tiles, face picked by the larger axis of the light-to-pixel vector). The local atlas has 12 tiles (`MAX_LOCAL_SHADOW_TILES`), so a frame shadows at most twelve spot lights, or two point lights.
- **Filtering and bias.** Hardware 2x2 comparison filtering plus a 3x3 PCF loop. Acne is handled by a rasteriser
  constant and slope bias in the shadow pipeline, plus a normal-offset push and a small fixed depth bias in the shader.
- **Opt-out per draw.** `DrawItem.castsShadows` and `ShadowSettings` on the `RenderWorld` (map size, distance, on or off).
- **Left out:** soft shadows from area lights (PCSS), shadow caching, translucent shadows and cascade blending.

Card: `lighting-and-shadows/shadow-maps`.

## D-060 The platformer grows by slices of the Crimson and Thorn mechanics

Dean (2026-11-16) wants the platformer to become a full 3D platformer, starting with mechanics from the Crimson and
Thorn design documents. `crimson-and-thorn-slice.md` maps every mechanic to now, later or not for the demo.

- **Design source only.** The demo takes mechanics and pacing, not story, art or audio, and Lantern's code and docs name
  nothing but Lantern.
- **Slice 1 is the streets**: checkpoints, a scripted companion, the Boost in its solo and cooperative forms, and one
  short level that teaches them in that order. Slice 2 adds the Guide Light, chapter looks and thorns.
- **Never punish.** The design's gentleness is a rule for the demo: no health and no lives. A fall returns the player to
  the latest checkpoint with their gems.
- **The Anchor waits.** Its 3D form is open in the design, so it is a question for Dean on its card, not a guess.
- **The game stays on the seam.** New world kinds are labels the game gives meaning to. A missing engine call is a card
  on `engine-follow-ups` before it is worked around.

Cards: `platformer-game/crimson-and-thorn-slice`.

## D-061 Slice 1 of the Crimson and Thorn mechanics: the streets

The first slice (`crimson-and-thorn-slice.md`) is playable from the main menu's "The streets" button.

- **The Boost is a taller take-off, not a new move.** The controller works the take-off speed out from its jump height
  when the jump starts, so `Player.Move` sets `BOOST_JUMP_HEIGHT` (3.6 m) for the one step that starts a boosted jump and
  puts the normal 1.5 m back. Gravity and the variable-height rule are untouched. 3.6 m is a first guess for Dean to feel:
  the level's ledges (2.2 m with a crate, 3.2 m with the boost) leave 0.4 m of margin.
- **The companion is data and a few lines.** Her route is a list of `companion_stop` entities sorted by `order`. She walks
  when the player is within 4 m along the ground, waits at a `boost` stop until the boost is used, and climbs a stop that is
  straight above the last one at the stop's `speed`. There is no pathfinding, as the design says.
- **A fall costs a walk.** Checkpoints keep gems and send the companion back to the stop the checkpoint names. `R` still
  restarts the whole level.
- **No `boost_spot` kind.** The boost stop already says where she crouches, so a second marker would only repeat it.
- **The autopilot has a script per level.** `LANTERN_AUTOPLAY=1` on the streets plays the level through (it is open-loop,
  a function of the simulation clock), which gives smoke runs and screenshots. The engine test drives the same route
  with the keyboard.
- **The menu picks the level.** `Play` loads the course (or the editor's level), `The streets` loads the streets;
  choosing the one already loaded just resumes.

Cards: `platformer-game/crimson-and-thorn-slice` (checkpoints, companion and the Boost complete).

## D-062 Lights in a world file are ordinary entities, found by kind

(Provisional, 2026-11-16, not yet reviewed by Dean.)

Dean asked for lights that can be placed in the world editor. A light is a world entity of kind `point_light`,
`spot_light` or `sun`, and its colour, strength, range, cone and shadow switch are the entity's properties, so the
file format, the editor's placement, move, rotate, hide and undo tools and its property inspector all work on lights
unchanged.

- **No new format.** The world file already carries a kind and typed properties per entity. A new top-level `lights`
  list would have needed a format version, a second editing path and its own selection rules.
- **Direction is rotation.** A spot light and the sun point straight down with no rotation, and rotating the entity
  turns them, which the editor's rotate gizmo already does. Scale is ignored.
- **One conversion, two users.** `lantern:::world:::LocalLightFromEntity` and `SunFromEntity` turn an entity into the
  renderer's `Light` or `DirectionalLight`. The game's world load and the editor's viewport both call them, so the
  editor shows what the game draws.
- **Load adds, clear removes.** `lantern:::world:::Load` adds the file's lights to the render world and remembers
  their ids, and `lantern:::world:::Clear` removes exactly those, so lights a game adds itself stay. A `sun` entity
  replaces the sun and ambient light; the last one in the file wins. Clearing the world does not put the old sun back, so a game that loads more than one level sets its own sun before each load, as the platformer does.
- **A parent's transform is not applied in the game.** The editor places a light through its parents, the game load
  by the light's own transform. Keep lights unparented until the game load composes parents.
- **Left out:** a light that follows another entity, lights switched on and off by game events from the file alone
  (a script can `FindByKind` the entity and use the `lights` API), and a viewport toggle that turns the editor's
  light preview off.

Card: `lighting-and-shadows/lights-in-the-world-editor`.

## D-063 Slice 2 of the Crimson and Thorn mechanics: light, thorns, chapters

The second slice (`crimson-and-thorn-slice.md`) adds the Guide Light, chapter looks and thorns to the streets.

- **The Guide Light is a point light with a reveal radius.** `GuideLight` adds a light (`lantern:::lights:::AddPoint`),
  moves it with its carrier each step and removes it on reload. Hidden `glow_platform` pieces are lit when they are
  within any light's radius (8 m for the companion, 5 m for the lantern) and stay for 4 s, which covers a player a few
  metres behind the light. The radius is the rule and the light is only how it looks, so the two can be tuned apart.
- **Hiding a platform moves it.** There is no engine call to switch collision off, so a hidden piece is teleported
  1000 m down and its mesh hidden, as the engine card says. A static collider doesn't follow a teleport, so these pieces
  are `kinematic` in the world file. `SetSolid` on the card replaces this.
- **The lantern is what the second gap is for.** The companion's last stop is before it, so only the lantern's light
  shows those plates: her form of the puzzle first, then the player's own.
- **The level says its chapter.** The spawn's `chapter` property picks the sky, fog and sun in `chapter-looks.nyx`.
  Level lamps stay with `LightingSettings`. The streets are chapter 1; `streets-later` is the same geometry as chapter 3,
  which is also when thorns appear (`min_chapter`, default 3).
- **Thorns nudge.** The game remembers the last place the player stood on the ground at least 1.6 m from any thorn and a
  touch sends them there. Gems, checkpoints and the companion are untouched, so it can't end a run.
- **The streets autopilot is companion-aware.** It still is a function of the clock, but it waits where the companion
  needs the player close and walks at 70% speed past the rooftop.

Cards: `platformer-game/crimson-and-thorn-slice` (Guide Light, chapter looks and thorns complete; the Anchor waits for Dean).


## D-064 The scene is drawn in high dynamic range, then tone mapped, with bloom as an optional pass

(Provisional, 2026-11-16, not yet reviewed by Dean.)

Dean asked for the renderer to be improved step by step, starting with emissive gems that glow. A glow needs light
brighter than white, and an 8-bit target throws that away, so the lit pass now draws into a 16-bit float texture and
three more steps turn it into the picture. `bloom-and-tone-mapping.md` is the guide.

- **Pass order.** Shadows, the scene pass (lit meshes into the HDR texture), bloom (nine small full-screen passes),
  the composite pass (glow, exposure, tone map, sRGB into the swapchain image), then a last pass for debug lines and
  the overlay. Lines and overlay stay out of the HDR path, so a HUD colour reaches the screen as written and the
  editor's gizmo colours don't change.
- **The default tone mapper is Khronos PBR Neutral, not ACES.** ACES was tried first and washed the platformer out: its
  curve brightens middle values and desaturates pastels, and the game's colours were tuned for a clamped target. Neutral
  leaves everything below about 0.8 alone. ACES stays one call away (`SetToneMapper("aces")`), and so does "none".
- **Bloom is 13-tap shrink and tent-filter grow**, five levels, with a soft-knee threshold and Karis averaging on the
  first shrink so a single bright pixel can't sparkle. Defaults: threshold 1.5, strength 0.1, so a sunlit wall doesn't
  glow and a gem does.
- **`SetGlow(entity, color, strength)`** lets a material's emissive light exceed 1 (the `"RRGGBB"` string can't). The
  gems glow at 4 times their colour.
- **`lantern:::graphics`** holds the settings and `SetQuality("low" | "medium" | "high")` is the one switch effects join
  as they are added. Low turns bloom off. HDR and tone mapping stay on at every level.
- **A GPU that can't draw into and sample RGBA16F is refused at start-up** with a clear error. SDL GPU's Vulkan and Metal
  devices all support it, so there is no 8-bit fallback to maintain.
- **Not checked in the cloud:** Metal output, and frame times on a real GPU. The cloud renders with lavapipe, a software
  Vulkan driver, so the 60 fps goal needs Dean's Mac.

Card: `renderer-visuals/hdr-rendering-tone-mapping-and-bloom`.

## D-065: A procedural sky that lights the scene

- **The sky is a formula, not a cubemap.** `SkyGradient(direction, zenith, horizon, ground, blur)` in
  `engine/shaders/sky-common.glsl` is shared by the backdrop shader and the lit shader, so what you see behind the world
  is exactly what lights it. No bake step, no texture to load.
- **Image-based lighting is analytic.** Diffuse irradiance is the sky's blurred lookup along the normal; specular is the
  roughness-blurred lookup along the mirror direction times an approximation of the environment BRDF table.
- **The sky colours ride in the existing light uniform block** (now 176 bytes). SDL GPU allows four uniform buffers per
  stage, and a fifth crashed lavapipe, so no new buffer was added.
- **The sky is drawn after the meshes** at the far depth with a less-or-equal test, so it only shades uncovered pixels.
  It is skipped for orthographic cameras.
- **On at every quality level.** It is one fullscreen pass and arithmetic per lit pixel.
- **`lantern:::sky`** sets colours, the sun disc, the lighting strength and intensity, and turns the sky off.
- **Not checked in the cloud:** Metal output, and frame times on a real GPU.

Card: `renderer-visuals/procedural-sky-and-image-based-lighting`.

## D-066: FXAA, not MSAA or TAA

- **FXAA runs on the tone-mapped picture** as one extra full-screen pass (`fxaa.frag`). The composite writes a texture of
  the screen's size and format and FXAA reads it into the screen image.
- **Not MSAA.** The lines and overlay draw after the composite onto a single-sample image and the lines depth-test against
  the scene's depth, so MSAA would need a depth resolve or lines in the HDR pass (changing their exact colours). It would
  also complicate ambient occlusion, which reads depth next.
- **Not TAA.** It needs motion vectors and history, and ghosts on fast movement.
- **It softens fine texture detail slightly.** Accepted for a game of big shapes. MSAA can be revisited as its own card.
- **Quality:** low turns it off, medium and high turn it on. `SetAntiAliasing(enabled)` overrides it.
- **Not checked in the cloud:** Metal output, and frame times on a real GPU.

Card: `renderer-visuals/anti-aliasing`.

## D-067: Screen-space ambient occlusion over the depth buffer

- **Twelve hemisphere samples per pixel** read the scene's depth, with a range check, then a depth-aware 4x4 average
  removes the 4x4 sample-turning pattern. Two full-screen passes on one-channel textures.
- **The depth buffer gains the sampler usage.** It was a depth target only; every depth format the renderer picks already
  supports sampling (the shadow maps use them).
- **The composite multiplies the scene by it, before bloom.** A single-pass lit shader can't apply it to ambient light
  alone, so direct light in creases darkens a little too. A depth pre-pass would let AO touch only ambient; not done, to keep
  the renderer small. The multiplier of 1.8 on the blocked fraction is because samples near the surface rarely count.
- **Perspective cameras only.** The editor's orthographic views skip it.
- **Quality:** low off, medium and high on. Defaults: radius 0.6 m, intensity 0.8, bias 0.04 m.
- **Not checked in the cloud:** Metal output, and frame times on a real GPU. This is the most expensive effect so far.

Card: `renderer-visuals/ambient-occlusion`.

## D-068: Height fog and light shafts in the composite pass

- **Both are done in `composite.frag`**, which already reads the scene, so it also reads the depth buffer. Each pixel's
  distance comes from its stored depth and its ray from the camera basis, as the sky does it.
- **Height fog is analytic.** Exponential density with height integrates in closed form along a ray, so there is no
  marching. It adds to the old distance fog, which still hides the far plane; it doesn't replace it.
- **Light shafts are a screen-space radial walk** towards the sun, counting steps that see the far plane. No shadow-map
  marching: it needs only the depth buffer and no extra shadow work, but it can't shaft through geometry the depth buffer
  doesn't contain (so the sun behind the camera, or off one side, gives none).
- **Quality:** Low turns shafts off; height fog stays (it is arithmetic only). Orthographic cameras skip both.
- **Defaults are on, and subtle** (density 0.008, shaft strength 0.35), so the platformer gains some haze without nyx edits.
  A game can set them with `lantern:::atmosphere`; the Crimson and Thorn chapter looks can set their own.
- **Not checked in the cloud:** Metal output, and frame times on a real GPU.

Card: `renderer-visuals/fog-height-fog-and-sun-light-shafts`.

## D-069: A spiral shadow filter and cascade blending, not contact-hardening shadows

- **The 3x3 filter becomes a spiral of 4, 12 or 20 lookups** over a disc of `softness` texels (default 2), turned by a
  different angle per pixel, so the edge is softer and the banding becomes fine noise. Quality sets the count.
- **Cascades fade into each other** over the last 12% of each one, and the last fades to "no shadow", so the seams between
  cascades and the hard cut at the shadow distance are gone. Snapping the cascades to whole texels was already there.
- **Not done: a penumbra that widens with distance from the caster.** That needs a blocker search over raw depths, and the
  renderer reads its shadow maps through a comparison sampler. It would need a second, plain sampler on the same texture
  and a blocker-search loop; left for a card of its own.
- **The effect is modest.** Edges are softer and smoother, which is clearest on long shadows; contact points stay tight.
- **Not checked in the cloud:** Metal output, and frame times on a real GPU.

Card: `renderer-visuals/softer-shadows-and-stable-cascades`.

## D-070: Blob shadows darken surfaces by distance to a point, in the composite pass

- **A blob is a point, a radius and a darkness**, and the composite pass darkens each pixel by how near its world position
  (rebuilt from the depth buffer) is to the point, as an ellipse that reaches only a short way above and below it. No mesh,
  no decal projection, and it lies correctly over slopes, steps and walls.
- **They last one frame**, like draws, so a game adds them every frame in `OnRender`. A frame holds eight; a ninth is ignored.
- **The platformer's disc is gone.** Its player keeps the floor point and radius from the per-step raycast it already did and
  hands them to the engine in `OnRender`; the disc entity, its lift off the floor and its hide-and-show are removed.
  `unitSphereMesh` stays in the assets for other uses. The recorded traces in
  `tests/host/the-platformer-matches-the-rust-trace` keep the draw count in their last column, which fell by one wherever the
  disc used to draw, so those two files were re-recorded; every other column is unchanged, so the simulation still matches.
- **Not decals.** Textured decals (scorch marks, paint) need a texture bound to the pass; blobs use none. Decals are left for
  a card of their own.
- **Not checked in the cloud:** Metal output, and frame times on a real GPU.

Card: `renderer-visuals/surface-detail-normal-maps-decals-and-contact-shadows`.

## D-071: The colour grade is applied after tone mapping, in the composite pass

- **Four controls:** saturation, contrast, tint and vignette, in that order, on the 0 to 1 picture after tone mapping and
  before the sRGB encode, so each number does the same to what the viewer sees whatever the scene's brightness.
- **Neutral by default.** Nothing changes for a game that doesn't call it.
- **The HUD isn't graded.** It is drawn after the composite, as before.
- **No lookup-table grade.** A 3D colour lookup texture is the usual tool for authored looks, but it needs an asset
  pipeline for the tables; four numbers are enough for the chapter looks Crimson and Thorn needs and can be blended by hand
  between chapters. A table is a possible later step.
- **A game calls it when a chapter changes** (or every frame while easing between looks). The platformer's chapter looks are
  the other thread's to choose; this adds the hook only.
- **Not checked in the cloud:** Metal output, and frame times on a real GPU.

Card: `renderer-visuals/particles-and-a-colour-grade-hook`.

## D-072: Particles are simulated in the engine and drawn as fading line streaks

Date: 2026-11-16.

- **The engine moves them, the game emits them.** `lantern:::particles:::Emit` hands over a start, a velocity, a colour, a
  life, a streak length and a gravity; `RenderWorld.StepParticles` steps and draws them once per frame after `OnRender`. Doing
  it in Nyx would cost an interpreted loop per particle per frame, which the 60 fps target can't spare.
- **Drawn with the existing line pipeline.** It needs no new shader, pipeline or vertex layout, and a streak is the right look
  for sparks and thrown dust. The cost is that particles skip bloom, grading and anti-aliasing and can't be soft or textured.
  A billboard quad pipeline is the later step and is listed in `particles.md`.
- **The quality setting is a budget**, 64, 256 or 512 live particles, rather than a switch: the effect thins out instead of
  disappearing. A hard ceiling of 512 keeps the per-frame line count bounded.
- **Stepped by display time, not fixed steps**, so they are smooth at any refresh rate and can't affect gameplay.
- **Platformer:** dust on every jump and landing only (`game-systems/dust.nyx`), kept small. Trails and sparks are available
  to the game but unused.
- **Not checked in the cloud:** Metal output, and frame times on a real GPU.

Card: `renderer-visuals/particles-and-a-colour-grade-hook`.

## D-073: Normal maps use a cotangent frame from screen-space derivatives

Date: 2026-11-16.

- **No tangent vertex attribute.** The mesh vertex stays 48 bytes with no tangent; the lit shader derives the tangent frame
  from the change of world position and uv between pixels. This works on every existing mesh, including glTF ones, and
  needs no change to the vertex layout, shadow pipeline or tests. The cost is a frame that is constant per triangle and a
  faint seam where uvs jump; stored tangents are the later step if a game needs perfect maps.
- **A fourth sampler on the lit pipeline** (set 2, binding 3), bound for every draw (the white texture when there is no map),
  with the strength in `material.surface.z`: 0 means "never read it", so a draw without a map pays nothing.
- **One strength control, 0 to 4,** instead of separate scale per axis. 1 is as authored.
- **OpenGL convention** (green up the image), since that is what the generated maps and most authoring tools produce.
- **sRGB re-encode.** Textures upload as sRGB, so the shader encodes the sample back to its stored bytes. A second linear
  texture format would avoid the maths but needs a second texture path; not worth it yet.
- **Normal-mapped draws bind their own sampler** per draw, which the sort by texture doesn't group; fine for the handful of
  detailed surfaces a scene has.
- **Decals are not built.** A projected decal needs the depth buffer read in the lit pass or a separate pass; a flat quad
  drawn just above the surface covers the platformer's ground cases. Contact shadows are the blob shadows of D-070.
- **Not checked in the cloud:** Metal output, and frame times on a real GPU.

Card: `renderer-visuals/surface-detail-normal-maps-decals-and-contact-shadows`.

## D-074 Slice 3 of the Crimson and Thorn mechanics: the Anchor as a set swing arc

Dean chose a set swing arc for the Anchor, over a physics rope or a grapple (`crimson-and-thorn-slice.md`, slice 3).

- **A fixed arc, driven by the game.** Two posts and a circular arc between them; the player's feet are set along it each
  step while it runs, with no controller movement and no steering. It is deterministic, so it is easy to author in the
  editor and to test, and it can't strand the player. A physics rope can come later as a different kind.
- **A pendulum's timing.** The angle follows `-cos(pi t)` over `seconds`, which gives a slow start, a fast bottom and a
  soft arrival with no velocity to hand back to the controller.
- **Cooperative first, then solo.** The first arc works only with the companion beside its start post
  (`needs_companion`), as the Boost and the Guide Light did, and the second is the player's own.
- **Authored in the world file.** One `swing_arc` entity per swing: its position is the start post and `end` the far one,
  so the editor places the arc with its ordinary tools and no new engine call is needed.
- **Slice 2's review nits.** The guide lights are drawn at the blended positions in `OnRender` while the reveal rule keeps
  the fixed-step ones, a safe spot is never recorded on a glow plate, and the strength comment is corrected.

Cards: `platformer-game/crimson-and-thorn-slice` (the Anchor complete).


## D-075: The editor's quality settings are view options on the session

Date: 2026-11-16.

- **View options, not a new host function.** `quality`, `shadow_quality`, `bloom`, `ambient_occlusion`,
  `anti_aliasing` and `light_shafts` go through `editorSetViewOption` and `editorViewOption` like the grid does, so the
  committed host surface is unchanged and the choices are saved and restored with the other view options.
- **A level is a preset, the checkboxes are overrides.** Setting `quality` sets the four effect toggles and the shadow
  quality the way `SetQuality` does (low turns the four effects off, medium and high turn them on); the toggles and
  `shadow_quality` can then differ from it. Names are listed with `quality` first, so loading preferences applies the
  level before the individual choices.
- **Default medium,** the settings the viewport drew with before, so nothing changes for anyone who doesn't open the tab.
- **Applied every frame in `BuildViewport`** to the viewport's `RenderWorld`, so a change shows on the next frame.
- **A Graphics tab,** not more rows on Viewport: the Viewport tab is about what is drawn, the new one about how well.
- **Not checked in the cloud:** Metal output, and frame times on a real GPU.

Card: `renderer-visuals/editor-quality-settings`.

## D-076: Slice 4 of the Crimson and Thorn mechanics: a goal ends the level and chains to the next

The streets needed an end, and the game needed a loop (`crimson-and-thorn-slice.md`, slice 4).

- **The next level is data.** A `goal` entity's optional `next_level` text property names the world file to load, so a
  chapter progression is authored in the editor and needs no engine call. Without it the game returns to the main menu.
- **A pause, then the hand-over.** Reaching the goal shows the gem tally for 4 s while the player can still move, then
  the level changes. Nothing is saved yet (saving is a later feature), so the gem count starts again with the new level.
- **Tolerate bad authoring.** A `swing_arc` with no usable `end` is skipped and logged, because a world file edited by
  hand should not stop the level loading. This has no host test (the host tests cannot choose a level); the log line
  is the check.
- **Slice 3's review nits.** The `SwingTo` comment sits above the right method, and the missing `end` is the guard above.

Cards: `platformer-game/crimson-and-thorn-slice` (the goal and the swing arc diagnostics complete).

## D-077: Slice 5 of the Crimson and Thorn mechanics: the rooftops, a solo level of moving platforms

The chapter progression had one real level played twice. A second level makes it a progression
(`crimson-and-thorn-slice.md`, slice 5).

- **Solo, with no companion.** The level lists no companion stops, so no companion appears. The design teaches solo
  first, and a level without her shows the other kinds working alone; the Anchor's solo arc is used for the same reason.
- **Moving platforms, not a new kind.** The platforms are the engine's existing `moving_platform`, whose position is a
  function of time. Timing a crossing needs no new input: the player waits for the platform, which the test does
  from the debug overlay's `Platform/N` values.
- **World files are the chain.** streets -> rooftops -> streets-later is two `next_level` properties, so the order is
  edited in the world files and the menu buttons are shortcuts.
- **No autopilot for it.** The scripted pilot plays only the streets; with `LANTERN_AUTOPLAY` the rooftops run the
  course's pilot. The keyboard test is its check.
- **Moving platform order is the level's.** `Platform/0` is the last platform listed in the world file, found by
  `FindByKind`; the test relies on that order.

Cards: `platformer-game/crimson-and-thorn-slice` (the rooftops complete).

## D-078: The editor measures its own frames and honours LANTERN_VSYNC

The editor's status bar read a constant 60 fps on a 120 Hz display with `LANTERN_VSYNC=0`, while the game's HUD read
185 to 234. Two things caused it. The editor window's SDL renderer was always opened with vertical sync on
(`SdlWindow.Open`), and `LANTERN_VSYNC` only reached the off-screen GPU renderer that draws the viewport, which has no
swapchain to sync. And the figure was the time between viewport renders, which in a loop that waits for the display is
the display's rate whatever the work costs.

- The window's loop times itself, from the start of one frame to the start of the next (wait for the display
  included), into the same `FrameStats` ring the game's debug HUD uses, and the status bar shows its average and worst
  milliseconds over 120 frames, the rate the average gives, and whether the window really has vertical sync on. The
  viewport's own render time and draw count sit in a second chip.
- `SdlWindow.SetVerticalSync` and `VerticalSyncEnabled` (penumbra-ui-engine) let the editor set vertical sync at
  start from `LANTERN_VSYNC` and change it from a Vertical sync checkbox in the Graphics tab. The interface asks through
  `editorSetVerticalSync`; the loop makes the window match once a frame and reports what the window says it has, so a
  driver that refuses shows as still on.
- The editor already redraws every frame, so there is no idle mode and no separate redraw toggle: vertical sync is the
  only pacing. If it becomes event-driven later, the status bar should say "idle" between redraws.
- The setting is not saved with the preferences. It is a measuring switch, and starting a benchmark with it left off
  from a previous session would be a surprise. Provisional (2026-11-16): not reviewed by Dean.

Cards: `world-editor/settings-status-bar-and-visual-polish/editor-reports-real-frame-times-and-has-a-vsync-toggle`.
## D-079: The platformer is played at night, and small lights carry the mechanics

Dean's feedback (2026-11-16): with everything fully lit, the companion's glow and the lantern did nothing you could see.
See `night-look.md`.

- **Night is the default look, day is an option.** `ChapterLooks.Apply` reads `LANTERN_LOOK`; `day` gives the old
  daytime table, anything else the night. The day look is a table of a few numbers, so keeping it is cheap, and it
  checks that a level's geometry reads without its lights.
- **The moon is the sun.** The engine has one directional light, so the moon is that light, with a cool dim colour, a
  lower direction (`LightingSettings.MoonDirection`) and a pale, bigger sun disc. Sun shadows stay on, so the moon casts
  the long shadows. No second engine light was added.
- **Ambient comes from the sky.** The sky colours also light the scene (`sky-and-image-based-lighting.md`), so the
  night's sky colours are chosen as a light source, not only as a backdrop: a night too dark to read platform edges is
  fixed by lifting the horizon and zenith, not by adding a flat fill light.
- **Small lights belong to the game, by state.** `NightLights` adds a point light for each street lamp, checkpoint
  post, goal and lantern pickup, and `Update` recolours them from the objects' own state each step, so a restart or a
  respawn needs no extra code. They cast no shadows: a shadowed point light costs six extra shadow renders, and the
  engine has twelve local shadow tiles in all (`ShadowPlan`).
- **Street lamps and lit windows are new world kinds.** They are authored in the editor like any entity (palette
  entries `street_lamp`, `lit_window`), and the game gives them their meshes, so a world file stays a list of
  positions. The towers beside the street are plain `block` entities.
- **Bloom is retuned for night.** Threshold 1.0 and strength 0.35, instead of 1.5 and 0.1, so lamps and windows glow.
  Quality `low` still turns bloom off.
- **Not measured on a real GPU here.** The cloud session renders with a software Vulkan driver, so frame times in
  `night-look.md` are lavapipe's, not a GPU's. Dean's Mac numbers (185 to 234 fps) are the check that matters.

Cards: `platformer-game/night-and-photo-mode` (`night-chapter-looks`).

## D-080: Photo mode is a collectable built from the camera, a raycast and a counter

Dean's feedback (2026-11-16): a photo-taking collectable, of sights together, for more variety in what the character does.
See `photo-mode.md`.

- **Game code only.** The engine already offers the camera, raycasts, the screen size and world properties, so photo mode
  is Nyx in `games/platformer/`. No host function was added.
- **A counted photo, not a file.** A photo counts a sight and the album names it. Saving the picture needs one new engine
  function (a screen capture the game can call); the engine already writes a PNG at the end of a run, so the work is small
  but it is an engine change, left as a follow-up card.
- **First person, body hidden.** The lens is at the player's eyes. Visibility belongs to a mesh in this engine, so
  `Player.SetBodyVisible` hides the body and visor while the camera is up.
- **The framing test is maths in the game.** A pinhole projection of the sight's position decides "in frame", so a sight
  needs a position and nothing else, and no engine support for picking is needed. The field of view (60 degrees) is
  duplicated from the renderer's `Camera`, because the game can't read it back.
- **"Together" means the companion is in the shot and near the sight.** Her chest has to be in the frame and in the
  clear, and within 7 m of the sight, so a photo of her anywhere else doesn't count. A level with no companion ignores the
  property.
- **Optional by construction.** The world keeps running, no door waits on a photo, the shutter always flashes, and a
  photo that doesn't count says why on the line under the frame.
- **The shutter shares Space with the jump.** With the camera raised the player stands still (`Simulate` zeroes the
  move and jump input), so the key is free.
- **Sights hang a marker in the world.** A small glowing diamond shows where a sight is while the camera is down, so
  players find them by looking around; a hidden one is a later option (`photo_sight` with `hidden`).

Cards: `platformer-game/night-and-photo-mode` (`photo-sights-in-world-files`, `photo-mode-camera-and-framing`,
`photo-viewfinder-hud-and-album`).

## Gameplay prototypes and the crossing

- **D-081: she is a character with her own body, not a route.** In the prototype rooms and the crossing she has a
  character controller of her own (radius 0.24 m, so she fits the 0.66 m slot the player cannot), three modes
  (follow, wait, walk to a point) and acts at command points. The scripted route (`companion_stop`) stays for the
  streets, rooftops and `streets-later`, where she leads.
- **D-082: three commands on four keys.** Q calls, E waits, F pulls a lever beside you or else sends her to the nearest
  `command_point` within 24 m in front of the camera. A request she cannot meet is answered with a status line, never
  ignored. On a gamepad the D-pad left and right call and wait, and West is use; the shoulders belong to photo mode.
- **D-083: she follows your trail, not a straight line.** She retraces a breadcrumb of your route (a point every
  metre), so she goes through doors, slots and bridges you used. If she is stuck 2.5 s while following she is put behind
  you; while walking to a point she says "She can't get there". Provisional: a real nav mesh is not worth it yet.
- **D-084: channels, not scripts.** Plates, levers and crystals switch a numbered channel; gates read a channel.
  Plates are momentary or timed (`hold_seconds`), levers latch or time out, a crystal is lit while a light has a
  raycast line of sight to it. A gate never closes on someone standing in it. The same pieces build every puzzle, so a
  new puzzle is a world file.
- **D-085: every puzzle has a solo route.** A timed plate, the lantern, ledges, steps or cover. The solo route is longer,
  riskier or needs more platforming skill, never impossible. Rooms 1, 3 and 5 prove theirs end to end in tests; the
  ledge and step climbs are reasoned from the geometry only.
- **D-086: moving a collider away is a teleport.** The engine cannot switch a collider off, so doors slide (and carry
  anyone riding them, which is wanted) and crumbled slabs are teleported 1000 m down, which drops whoever stood on them.
- **D-087: a bounce launch holds the jump.** A pad asks the controller for a taller jump for one step, and the game
  keeps the jump held until the next landing, because a released jump is cut short.
- **D-088: the watcher resets, it doesn't kill.** Being seen sends the player to the last checkpoint after a 1.5 s
  grace; the lure keeps the beam on her for 6 s.
- **D-089: a full level beside the existing ones.** The crossing is a new chapter 2 level between the rooftops and
  `streets-later`, instead of lengthening three files another thread was editing. It is also entry 7 of the Prototypes
  list. The rooftops' goal now points at it.

Cards: `platformer-game/gameplay-prototypes`.

## Companion leads and swings first

Dean's play-test (2026-11-16): she can't swing the Anchor, and it is easy to get ahead of her because she only walks on
when you are close. Details in `docs/crimson-and-thorn-slice.md`; the route-following companion is the one in the streets
and `streets-later` (the commanded partner of the prototype rooms is unchanged, D-081).

- **D-090: she leads, she doesn't follow.** She walks on when you are within 10 m (was 4) or nearer the next stop than she
  is, in which case she hurries (1.8 times her speed) back in front; a player more than 30 m away counts as elsewhere, not
  ahead. Mid-walk she stops when you fall 16 m behind and goes on again at 11 m, so she can't flicker at the edge. A stop's
  `near` replaces the range and turns hurrying off: the gap-crossing stop uses 4 m so her light never leaves you.
- **D-091: the wave is a hop.** Waiting for a player who is far off, she faces them and hops every 3 s. Walking back to
  fetch them was rejected: her route is straight lines between stops, so going back off-route could cross walls.
  Provisional: not reviewed by Dean.
- **D-092: a swing stop is a first-class leg.** A `companion_stop` with `swing` set makes the leg to the next stop a swing
  along the `swing_arc` whose start post is within 2.5 m, at the arc's own pace; she then walks from the far post to the
  next stop. She sets off when you are within `near` (16 m in the streets), so you see it.
- **D-093: a cooperative arc works with her at either post.** The old rule (beside the start post) can't hold once she swings
  first, so `needs_companion` now means beside either post. After swinging a cooperative arc she stays at the far post
  until you are within 4 m of her, which is also where your own swing lands. Solo arcs are unchanged and never need her,
  so the level stays solvable alone wherever she is. Streets and `streets-later` got stops 7 and 8 for the second arc and
  the goal, and the checkpoints now send her to the stop on their own side of each gap.

Cards: `platformer-game/crimson-and-thorn-slice/the-companion-swings-first` and `.../the-companion-leads-the-way`.

## Gem shadows and the watcher's light

Dean's play-test (2026-11-16): the gems left odd dark specks on the floor, and the watcher's light was barely visible.
Card: `platformer-game/gameplay-prototypes/fix-gem-shadows-and-watcher-light`. Not run on a real GPU or `nox` from the
cloud, so Dean's Mac check is the first look.

- **D-094: gems don't cast shadows.** A gem is a small mesh that spins and bobs, so as a shadow caster it drew a speck
  that shimmered every frame, beside the player's blob shadow. `Gem.Glow` now turns `casts_shadows` off for it. Pillars,
  the player and platforms still shadow as before.
- **D-095: the watcher is readable at night.** The beam is amber (`FFA23C`, strength 140; it was pale yellow at 40),
  the lamp has a glowing housing that blooms, and a glowing floor patch follows the beam, as wide as the catching cone
  at that distance (it hides when the beam hits a wall or pillar). The patch is the danger zone, so what you see is the
  rule. Provisional: the colour, strength and patch glow are first guesses, not reviewed by Dean.

## Grounded adventure prototypes

Dean's play-test (2026-11-16): the game has more action-adventure than puzzle-platformer in it, wants grounded movement and
challenge from places and small games. Details in `docs/grounded-adventure.md`. Numbers D-096 to D-102 were free when this
was written; another thread's decisions may have used some, in which case the reviewer renumbers.

- **D-096: the feel is chosen per level.** A spawn's `movement` property set to `"grounded"` asks for a lower jump, a slower
  walk and momentum; every other level keeps the platformer feel because its gaps need a 1.5 m jump. A global change would
  have made every existing level impossible. Provisional: not reviewed by Dean.
- **D-097: momentum is the game's, not the controller's.** The engine controller reaches speed in 0.12 s and offers no way to
  change its acceleration, so the game eases the direction it hands over (0.4 s on the ground, 1.6 s in the air). This needs no
  engine change and no new host function. If a later controller exposes accelerations, this moves there.
- **D-098: climbing is the game placing the player.** While a surface is held, the controller isn't stepped; the game puts
  the feet where the climb says (`Player.SwingTo`, as the Anchor does). The alternative, a climbing mode in the controller,
  was left until the feel is proven. Provisional.
- **D-099: she never climbs, and every climb has a ground route.** The ground route is linked to the climb by a lever and a
  gate on one channel, so each of them can open the way for the other. Her `team_beat` markers go on the ground route.
- **D-100: a train's danger is a rule, not physics.** A moving platform marked `train` sends a player on the track back to the
  last safe spot, as thorns do. A kinematic block shoving a character controller isn't something the engine promises to
  resolve safely, and a nudge fits "never punish the player". Track positions are never remembered as safe spots.
- **D-101: the small games are rests.** Words and meals are counted in the status line and cost nothing if skipped. Eating
  asks for F held with the stick still, so it is an act of stopping, which is the point of a rest.
- **D-102: romaji until the engine has a font.** The 8x8 bitmap font is ASCII, so the Japanese is written in Latin letters.
  Kana and kanji are an engine card, not a Nyx gap.

## The companion as an equal partner

Dean's play-test (2026-11-16): the player and the companion are equals and a team, so the command model is gone. Nothing in
this section has been compiled or run (the cloud has no `nox` or `mist`); all of it is provisional and not reviewed by Dean.

- **D-103: she works from `team_beat` markers, not commands.** Beats sorted by `order` say where she goes, what she does
  there (walk, stand, crouch, pull, lure) and when she moves on. Q, E and F-to-send were removed; F still pulls levers.
- **D-104: a beat is released by a rule.** `near` the player, `at` a point, a switch `channel` or `seconds`. `at` and
  `channel` make holds mutual: she holds a door until the player reaches the far side, and the player holds a plate for her.
- **D-105: she is safe by construction.** A down ray ahead of each step stops her at edges, a stuck timer retries then
  skips a beat then teleports her to the player's trail, and a fall puts her back. Waypoints (`via`) keep her off pillars.
- **D-106: she hurries and pauses to stay a team.** She speeds up when the player is far behind, waits when too far
  ahead, and follows the player's trail after the last beat.
- **D-107: a goal needs her.** The goal counts only with her within 6 m and 3 m height; alone, the player gets a hint.
  Provisional: both distances are guesses.
- **D-108: crumble slabs take their own timing.** `delay_seconds` and `return_seconds` (negative: never returns) so one
  kind can be a one-way trap she springs for the player.
- **D-109: rooms were reworked, not patched.** Hold the door has a far plate for her; the narrow way and hand-off keep
  their solo routes, harder; hand-off's stairs have low risers she can climb; flow and fragile has no bounce pads and
  crumbling slabs break behind her while the player uses stepping stones; the crossing is rebuilt on the same pieces.
- **D-110: the light gate is a lit-or-dark gem.** The pillar became a wall of shade and the crystal has a smaller range, so
  the gem counts only in her light or the lantern's. If this is still confusing the room is a candidate to drop.
- **D-111: the watcher stays a prototype.** She lures it unprompted; it doesn't fit the game's tone and is kept for the feel
  of tension only.
- **D-112: the climb and station have markers on their ground routes.** Her service stair route and the footbridge, as D-099
  promised.
- **D-113: she pauses further back.** `PAUSE_DISTANCE` 24 m and `RESUME_DISTANCE` 18 m (were 16 and 11). A player holding a plate for her
  stands up to 18 m from the far plate she has to reach, and at 16 m she stopped short and waited for them. Provisional: not reviewed by Dean.
- **D-114: failing room tests print the debug values.** Six host tests use `ExpectWithState`, which prints every debug value before it asserts, so a
  failure shows where she stood and what mode she was in. It prints only on failure.

## Wet surfaces and puddles

Dean (2026-11-17): more renderer work, shaders and materials. Nothing in this section is reviewed by Dean; the streets' wet look is provisional.

- **D-115: wetness is a material property, with a scene-wide minimum.** `Material.wetness` darkens the diffuse colour and pulls roughness
  towards a mirror's; `Material.puddles` says how much of the wet, upward-facing surface is flat standing water. `RenderWorld.worldWetness`
  and `worldPuddles` are a floor every draw shares (the larger value wins), which is how rain works without touching every entity.
- **D-116: no new uniforms.** Wetness rides in the spare `surface.w` and the puddle amount in `emissive.w` of the 48-byte material block, so
  the C-side layout, the Metal translation and every other shader stay as they were.
- **D-117: puddles are a world-space noise mask, not authored decals.** Two octaves of value noise over the ground plane, cut off at a level
  `puddles` sets, only on surfaces whose normal is within about 25 degrees of up. Cheap, texture-free and the same everywhere, but static
  and unrelated to the level's geometry (a puddle can cross the edge of a slab). Authored puddles would be a decal, which is not built.
- **D-118: a puddle mirrors the sky and the lights, not the scene.** Real reflections need screen-space reflections or probes; this
  step shows the sky through the existing image-based lighting and a sharp lamp streak from the lowered roughness. SSR is the next card.
- **D-119: the streets are wet by default.** Chapters 1 and 2 call `SetWetWorld(0.8, 0.55)` after the night look; the rooftops and course
  stay dry, `LANTERN_WET=0` turns it off and `LANTERN_WET=1` rains everywhere. Provisional: Dean picks whether the night streets stay wet.
- **D-120: the navigation graph is a sampled grid of floors, not a triangle navmesh.** The level is boxes and ramps, a grid handles stacked floors (a street under a bridge) as spans, and the first goal is behaviour that is easy to predict and test. 0.4 m cells give about 23,000 nodes for The crossing, built in about 0.4 s when the partner spawns. A triangle mesh with portals is the upgrade if the node count or the zig-zag of grid paths becomes a problem.
- **D-121: the graph holds the static level only.** A door, a lift or a crumbling slab is a moving body and isn't where it was at build time, so paths ignore doors and slabs. The partner falls back to a beat's `via` route when there is no path, and after two stuck checks on a path, which keeps every room test passing. Follow-up card: `navigation-over-moving-platforms-and-gates`.
- **D-122: there is no climb link.** She can't climb in this game (Dean, 2026-11-16), so a way that needs climbing has no path; she helps by taking another route. She can step up 0.3 m, jump up 1.0 m or across 2.2 m (her jump rises 1.2 m) and drop 3 m.
- **D-123: nodes keep a quarter of her radius from walls, and nodes nearer than the whole radius cost three times as much.** With the whole radius, a grid cell can miss a slot only 0.18 m wider than her (the narrow way had no nodes in its slot). Smoothing only cuts straight lines a full-radius capsule can walk. Provisional: she may still scrape a corner; the stuck check catches it.
- **D-124: stuck on a path means plan again, then the `via` route, then the older nets.** The first stuck check is usually a path that was good when planned (the player moved a platform); the second means the graph is wrong here (a closed door).
- **D-125: a path is drawn when the debug HUD is on.** Green walk, orange drop, red jump, from her to the end, so a thread or Dean can see why she went where she did without a screenshot tool.

Dean (2026-11-17): skeletal animation for the player and companion. Nothing in this section is reviewed by Dean; the numbers are provisional.

- **D-126: animation is engine data driven by game facts.** The engine owns skeletons, clips, animators and the blend maths
  (`lantern:::animation`); the game describes states and rules once and afterwards only sets parameters (`SetFloat`, `SetBool`,
  `Trigger`). A game never plays a clip by name from its own code, so a state change can't skip a fade.
- **D-127: skinning on the GPU, in two extra shaders.** `lit-skinned.vert` and `shadow-skinned.vert` take joints and weights from a
  second vertex buffer and a 32-matrix uniform; the fragment stage is shared, so skinned meshes get shadows and every effect for free.
  The alternative, skinning on the CPU and re-uploading vertices every frame, was left out for the cost at 60 fps.
- **D-128: 32 joints, four influences per vertex.** The palette is a fixed uniform block (2 KB) and weights are bytes. A richer
  rig needs a storage buffer, which is not worth it for a humanoid with 17 joints.
- **D-129: the mannequin is built in code.** Importing glTF skins is its own card; a generated humanoid keeps the checkout
  self-contained, and its clips come from formulas (`StrideCycle`) so the tests can check them.
- **D-130: animators work from measured speed.** `CharacterAnimator.Observe` takes a position and works out horizontal speed from the
  last one, so physics, a scripted walk and a swing animate the same way. A step over 20 m/s is a teleport and reads as standing.
  Footsliding is not corrected; the walk and run clips' strides are matched to the blend thresholds by eye (1.8 and 5 m/s).
- **D-131: look-at is a layer on top of the clips.** After the pose is sampled, a smoothed turn toward the target is spread over chest,
  neck and head; a weight of 0 changes nothing, so a character that never looks costs nothing.
- **D-132: the Rust trace test ignores the entity count.** The characters used to be a capsule and a visor (3 entities for the
  player); an animated body is one entity, so the count moved by one while every simulated column still matches.


## The camera: follow, volumes, shots and blends

Dean (2026-11-17): the game is more action-adventure than puzzle platformer, so the camera should follow smoothly, lead,
sit over a shoulder, never clip walls, and be authored by level designers. All of this section is provisional and not
reviewed by Dean; the numbers want playing. Cards: `engine-follow-ups/grounded-camera-and-authored-shots`.

- **D-133: one director owns the camera.** Only `CameraDirector` calls `lantern:::camera`. The follow rig, a shot and the
  photo lens each produce a `CameraPose` (a `data` value) and the director blends and shows one. Photo mode no longer calls
  `LookAt` itself, so raising the camera blends in like any other change.
- **D-134: the follow rig smooths, leads and sits over a shoulder.** The aim point follows the head with half-lives of 0.06 s
  sideways and 0.22 s up and down (a jump lifts the view less than the player), leads the measured speed by 0.45 s up to
  1.2 m (eased with a 0.3 s half-life), and sits 0.55 m over the right shoulder. All easing is by half-life, so it is the same
  at 60 and 144 fps.
- **D-135: collision covers the pivot as well as the boom.** A 0.25 m sphere is swept from the player's real head to the shifted
  pivot, stopping 0.1 m short, then the boom is swept back from the pivot as before. The gap is the fix for a bug the
  camera room's per-frame test found: a pivot left exactly touching a wall made the boom sweep start in contact, report no
  room, and throw the lens in to the 0.6 m minimum in one frame. When a wall is within 55% of the shoulder shift and the
  other side is clear (98%), the camera crosses to the other shoulder, and goes back after the first side has been clear for
  0.8 s so a corner doesn't flicker it.
- **D-136: volumes are axis-aligned boxes and change framing by easing.** Highest `priority` wins, then nearest the centre.
  The settings ease with a half-life of a quarter of `blend`, so the change is done in about `blend` seconds. Boxes are not
  turned by the entity's rotation, because the world API can't read an entity's rotation.
- **D-137: a shot is a tripod, linked to a volume by number.** A `camera_shot` sits at its entity's position and looks at
  `target` or, with `track`, at the follow pivot (already smoothed, so it doesn't twitch with footsteps). The volume's `shot`
  names the shot's `number`, because the world API can't look an entity up by name. Shots are not collision checked.
- **D-138: a change of what is on screen blends the pose, not the settings.** Follow to shot, shot to shot, shot to follow and
  photo up or down blend from the pose on screen to the live new pose over `blend` seconds (`exit_blend` for leaving, which
  defaults to `blend`) with a smooth-step curve. 0 is a cut. A volume can cut in and glide out.
- **D-139: controls hold steady through a cut.** When the shown camera changes while the stick is pushed, the movement
  heading stays what it was until the stick is let go, then follows the shot. Look input is ignored while a shot is showing.
  Dean should judge whether fixed-camera controls (relative to the shot) feel right once the player lets go.
- **D-140: the engine gains `SetFieldOfView`.** `LookAt` builds a new camera each call, so it now keeps the lens last set;
  `SetFieldOfView(degrees)` is clamped to 10 to 120. The photo framing test still assumes 60 degrees and the photo lens sets
  it, so the verdict is the same.
- **D-141: a photo is judged against the lens at rest.** The lens blends in over 0.2 s; the verdict, and so a photo taken in
  that fifth of a second, is judged against where it ends up.
- **D-152: screen-space reflections replace the sky reflection by a difference, so a miss changes nothing.** The lit shader
  still adds the sky reflection. The reflection pass adds `reflected - sky` weighted by a confidence, so a ray that leaves
  the screen or finds nothing leaves the sky in place and the picture is never worse than before.
- **D-153: the surface buffer is a second colour target, not a G-buffer.** Lighting stays forward. Lit, skinned lit and sky
  write octahedral normal, roughness and reflectance; skinned characters therefore reflect and are reflected with no
  extra shader. With reflections off the target's store is DONT_CARE.
- **D-154: low quality turns reflections off; medium is 24 steps and high 48.** `LANTERN_SSR=1` forces them on at medium
  cost, `0` forces them off. The editor Graphics tab has a checkbox and the quality levels set it. Not timed on a real
  GPU: the development machine has only a software driver.
- **D-155: the underside of things is a known limit.** The depth buffer has only the nearest surface, so floating objects
  reflect their visible front only. Hit thickness grows with distance to make thin walls catch, at the price of some
  jitter noise. Dean should look at it on a real GPU before more is spent on it.

- **D-142: the survey found most of the controller already there, so this pass adds only what was missing.** Slopes with
  a walkable limit, step-up, ground snap, coyote time, the jump buffer, platform carry and the grounded feel's momentum
  all existed and have tests. The gaps were what happens when the player leaves a moving platform, and a ledge too high to
  step but within reach of a jump.
- **D-143: leaving a moving platform keeps its sideways speed, behind `keepsPlatformSpeedInAir`.** The platform's
  horizontal speed is added to the player's movement until they land, separately from the stick-driven velocity, so letting
  go of the stick in the air doesn't cancel it. It is off by default because the Rust-trace test pins the old behaviour, and
  the game turns it on for the player in the grounded feel only. The companion's navigation doesn't plan for it.
- **D-144: the engine finds ledges, the game plays the haul.** `CharacterController.FindLedge(direction)` wants a wall
  within `mantleReach` (0.5 m), a walkable top more than a step and at most `mantleMaxHeight` (1.9 m) above the feet, and
  room for the whole body on top; `LedgeStandPoint()` is where the feet end up. Nothing moves the body in the engine.
- **D-145: mantling is the player's, in the grounded feel, in the air.** `MantleSystem` rises beside the wall for 60% of
  the haul and then moves onto the top, with the controller not asked to move the player, as for climbing and the Anchor.
  It starts only when the player pushes toward the ledge and their jump has reached its top (upward speed 1.5 m/s or less).
  The companion never mantles. Platformer levels keep their old feel, so their jump gaps are unchanged.
- **D-146: foot placement is a pass over the finished pose, with the ground read by the host.** After an animator has
  sampled and blended its clips, `FootPlacer` (`FootPlacement.nox`) lowers the hips by the deepest drop under a planted foot,
  moves each planted ankle with two-bone IK (law of cosines, the knee bending toward the character's forward) and tilts the
  foot to the slope. The animation system does not know about physics: `PlaceAnimatedFeet` (`host/FootPlacementHost.nox`)
  casts one ray down from 0.5 m above the character's own ground at each foot's `x` and `z` (reach 0.5 m up and 0.5 m down)
  and hands the hits over. Offsets are relative to the ground under the character's origin, so a flat floor changes nothing.
- **D-147: only planted feet are placed.** A foot's weight fades from 1 at 6 cm above its resting height to 0 at 22 cm, so the
  swing of a stride keeps its arc and a jump pose is left alone. The whole effect has a weight of its own that the game sets
  with `SetFootPlacement`: `CharacterAnimator.Observe` switches it off in the air and on when grounded. Offsets and tilt follow
  the ground at 14 per second, so a step glides instead of popping. Rays hit static geometry and moving bodies alike.
  Provisional (2026-11-17): a moving platform's own motion is not compensated, and a foot with no ground in reach is left as
  the animation has it.
- **D-148: footsliding is corrected by stride matching, not foot locking.** A clip says how fast its feet carry the character
  at playback speed 1 (`AnimationClip.groundSpeed`; the mannequin's walk and run get it from their leg swing and cycle
  time, 1.66 and 3.74 m/s). A blend state then plays at the character's speed over the mixed ground speed of the two clips
  it is mixing, between 0.6 and 1.6, and not at all below 0.2 m/s. Locking feet to the world during stance (IK anchoring)
  would fix more cases but needs a stance detector per clip; it stays a follow-up.
- **D-149: glTF import builds a `Rig` and a skinned mesh from the first skin.** `LoadGltfRig` and `GltfRigFromBytes`
  (`animation-mesh/GltfRig.nox`) read joints, `JOINTS_0` and `WEIGHTS_0`, and translation and rotation channels (linear, step;
  cubic spline is read at its keys without tangents) into a skeleton, a skinned `MeshData` and one `AnimationClip` per animation,
  with translation and rotation keys resampled onto the union of each joint's key times, since `Keyframe` holds both. Scale
  channels, morph targets, textures and non-joint nodes between joints are left out (a gap, stated in `docs/animation.md`).
  More than `MAX_JOINTS` (32) joints is an error that says so, not a truncation.
- **D-150: an imported character is stood up by options, not by the file.** `GltfRigOptions` has a scale, a yaw and a
  feet-on-the-ground shift. They are folded into the root joint (its translation, rotation and scale, and the same on every
  root key) and into the vertices, so the skeleton, the clips and the mesh stay consistent and no entity scale is needed. The
  armature node above the root joints is folded into the root the same way, so a file exported with a rotated or scaled
  armature loads upright. `LoadRig(path, scale, yawDegrees)` in Nyx always puts the feet on the ground. Provisional
  (2026-11-17): a model facing +z, the glTF convention, wants a yaw of 180 to face Lantern's -z; not reviewed by Dean.
- **D-151: the partner wears the animated body.** She is a `CharacterAnimator` on the mannequin rig like the player and the
  companion: walking and running follow her measured speed, she crouches for the boost, waves for her nudge instead of
  hopping, jumps on jump links, and looks at the player within 12 m. The squashed capsule, `HOP_SECONDS`, `HOP_HEIGHT` and
  `CROUCH_SCALE` are gone. Her physics body, her height and her feet are unchanged.

## Doors, lifts and slabs in navigation

- **D-156: the graph's moving parts are features, added after the build.** `Build` still samples only the static world. A game then adds gates (`AddGate`), surfaces (`AddSurface`) and rides (`AddRide`) and opens or shuts each with `SetFeatureAvailable`; `Revision()` counts the changes. `Build` forgets them, so the game registers them again after each build (`game-systems/navigation-dynamics.nyx`). Replaces D-121's "static level only".
- **D-157: a door blocks the nodes it stands on while it is shut.** The static floor under a door is in the graph already (the door is a kinematic body), so a gate only marks those nodes, within the body's radius of the box. A route avoids them; the point of a route inside a shut gate carries the feature, so the walker can wait.
- **D-158: when nothing goes round a shut door the route goes through it and she waits.** `FindRoute` plans with shut gates out; with no complete route it plans again with the waitable shut ones in, at a cost per crossing. A bridge or a stair that a switch raises is a surface and waits the same way; a slab that has broken is not waited for.
- **D-159: slabs and bridges are patches of nodes on the top of the box.** They overlap by 0.1 m at a seam so two slabs share a column of nodes, join the floor next to them by walking, and the edges drop and jump like any edge. The path cost of fragile ground is charged once when a route steps onto it, not per node: a per-node charge made jumping over the slabs cheaper than walking them.
- **D-160: she crosses fragile slabs only as the leader.** `FindRoute(..., allowFragile)`: leading is true, following is false. They break behind her, so a route home over them would strand her. Not modelled: leading across slabs she would then need to come back over.
- **D-161: a ride is a link between the two landings, and she boards by watching the platform.** `AddRide` joins the floor nearest each end of the platform's trip. She waits at the landing until the platform's edge is within 1 m and its top level with the landing, steps on, stands still, and steps off at the far landing once the platform is beside it. Trains are not rides.
- **D-162: a path is kept while the level changes.** A door she waits for opens, a slab breaks behind her. She plans again when she is stuck (D-124) or has no path and something has changed since she looked, not on every change, which restarted her path each time a slab began to shiver.
- **D-163: the streets companion walks a graph route between stops.** `Companion.Spawn` builds the graph; each leg asks `FindRoute` and falls back to the straight line when it doesn't reach the stop. She is still scripted, not a physics body. The rooftops level has no companion.
- **D-164: a twelfth prototype, the lift.** A chasm too wide to jump with one moving platform across it, so the ride has a room to be tested in; no existing level had a lift the partner needed.
- **D-165: five buses, with master over four.** `music`, `ambience`, `sfx` and `ui` each have a volume and a mute, and `master` scales all of them. This replaces the three knobs (master, music, effects). A sound never plays on master. Defaults: master 0.8, music 0.5, ambience 0.6, sfx 0.9, ui 0.9.
- **D-166: a voice's gain ramps across each block.** The mixer works out the gain a voice wants at the end of a block (volume, level, bus, distance, pan) and ramps from where the last block ended, so changing a bus volume, moving an emitter or fading a layer never clicks. A new voice starts at its first gain with no ramp.
- **D-167: positional sources are mono, attenuated by a squared falloff and panned equal-power.** A positional voice mixes its two channels to one. Gain is 1 inside `minDistance`, 0 beyond `maxDistance` and the square of a straight line between; pan is the sideways part of the direction to the source against the listener's right-hand vector, with equal-power gains. Not modelled: height, a darker sound behind the listener, Doppler, occlusion by walls. The defaults are 2 m and 30 m.
- **D-168: a loop has a fading level; layers and emitters are the same thing.** A named loop (`StartLayer` flat, `StartEmitter` at a point) fades its level toward a target at a fixed rate. An ambience crossfade is two `FadeLayer` calls, music stems are layers on the music bus, and stopping fades out and then frees the voice. Stems of one piece are all 8 seconds long and start in the same frame, so they stay in step; nothing re-syncs them.
- **D-169: no recorded audio; every placeholder is generated, and a file replaces it.** `lantern:::synth` builds footsteps for five surfaces, five ambience beds, three music stems and five menu sounds from tones and filtered noise, the same samples every time. `Load(name, path)` reads the WAV, and only when the file is missing falls back to the generated sound of that name, so a real recording is added by putting a file at the path.
- **D-170: audio is tested without a device.** `AudioSystem.Update` with no sink returns before mixing, so tests advance voices by calling `Mixer.Mix` themselves; `Mixer.Mix` is called directly by tests; the SDL output is opened with `SDL_AUDIODRIVER=dummy` in its test. A cloud session has no sound card and none is needed.
- **D-171: one `AudioDirector` decides the ambience and music layers each frame.** `game-systems/audio-director.nyx` sets the listener from the camera, crossfades the ambience beds over 3 s when a level starts, and eases four music stems (`theme` in menus, `calm` in play, `tension` while a watcher beam is on the player, `companion` while the partner is within 6 to 14 m) at a rate per stem. Pausing ducks the music to 0.35. The stems are the same length and start together, so they stay in step.
- **D-172: footsteps are driven by distance walked, not by animation.** The animator has no events, so `FootstepSystem` adds up the distance each walker moves and steps once per stride (1.1 to 1.9 m, longer when faster). A jump of more than 1.5 m in a frame counts as a teleport and adds nothing. The player, the partner and the companion each have a walker.
- **D-173: surfaces are authored, because collision gives none.** A level's spawn takes `surface` and `ambience` properties; a block may take `surface` and `surface_radius` to patch the ground near it. With none, the chapter picks (chapters 2 to 4 are metal, the rest stone), and every fourth step is wet in chapters 1 and 2.
- **D-174: menu and photo sounds are on the `ui` bus, and the generated names replace `menu_move` and `menu_select`.** `ui_move`, `ui_select`, `ui_back`, `ui_error` and `ui_shutter` are loaded from `sounds/<name>.wav` when the file exists and generated otherwise. The old `menu_move` and `menu_select` files are no longer loaded.
- **D-175: a new game session stops every named loop.** `ResetForNewSession` fades none: it stops layers and emitters at once, so a reloaded game doesn't inherit the previous level's beds.
- **D-176: beds and stems are set by loudness, and the limiter leaves quiet mixes alone.** The static was the generated ambience: a bed was scaled so its loudest sample was 0.8, but noise has a low peak for its loudness, so rain sat at an average level (rms) of 0.24 and the streets mix at 0.11, a constant hiss about 19 dB under full scale. Beds now have rms 0.05 and stems 0.07 (`NormaliseLoudness`, with a peak ceiling of 0.5), and the rain bed is darker (500 to 3500 Hz) with more swell. The mixer's cubic `SoftClip` bent everything above 0.5; it now passes anything under 0.8 unchanged and bends only the excess toward 1. The streets mix peaks at 0.16 and sits at rms 0.035. The tuning is by measurement, not by ear; not reviewed by Dean.
- **D-177: a forearm's x angle is positive, a shin's negative.** Facing -z, a positive angle about x swings a hanging limb forwards, so a knee bends with a negative shin angle and an elbow with a positive forearm angle. Every forearm in `MannequinPose` had been written negative, which folded the elbows behind the upper arms and made the arms look on backwards; they are all positive now. `the-mannequin-elbows-bend-forwards-and-knees-backwards` fails if any motion folds a forearm behind its upper arm or a shin in front of its thigh.
- **D-178: the editor is dark neutral grey, in the Unreal style.** The panels are #242424 on a #141414 window with #0E0E0E borders and 3 px corners; the accent stays blue (#0F7BDC). The viewport and the template previews use a flat grey sky (zenith, horizon and ground set in `BuildViewport` and `BuildTemplatePreview`, no sun disc) and neutral grey grid lines, so the colours of the level are the only colour in the picture. Not reviewed by Dean.
- **D-179: noise beds are quieter than hum beds.** After D-176 the ambience was still the audible static (Dean muted each bus in turn with `LANTERN_AUDIO_MUTE`). Noise has no pitch to sit under, so it is heard as hiss at a level where a hum isn't. Rain, wind and market now have rms 0.02 (hums stay at 0.05), and the rain bed is a softer 250 to 1800 Hz rumble with a deeper swell. Tuned by measurement, not by ear; not reviewed by Dean.

## The debug window

- **D-180: the debug HUD is a paged, filterable window with its own focus.** `DebugWindow` replaces `DebugHud`. Values are still `Group/Name`; a group maps to a page (`PageAssignments`), and an unlisted group is a page of its own. F1 shows it, F3 focuses it (F2 is the collision view). Focused, the engine hands the script an empty input snapshot, skips the UI's navigation and frees the mouse, so typing a filter does not move the player; unfocusing restores the capture the game asked for. It is drawn before the game's UI, so the pause menu covers it (the old overlap), and it is capped at 31% of the screen's width to clear the in-game HUD. No new key codes: the filter takes letters, digits and space; Shift+Up/Down is a page. Paging, filtering and scrolling are tested in `engine/tests/debug/`.
- **D-181: the audio page shows the buses.** `Audio/Bus Music`, `Ambience`, `Effects`, `Interface` and `Master` show volume and gain, or `muted`, beside the underrun count, so an audio problem can be bisected by looking.
- **D-182: renaming is an input in the outliner row, opened by F2, the Rename command or a double click.** The row swaps its name for an `<input>` that takes focus with its text selected (`autoFocus`); Enter (`onSubmit`) writes the name with `editorSetField(id, "name", text)` and an empty name keeps the old one; Escape or losing the row cancels. The state is `OutlinerState.renaming`, one entity at a time. It needs the `onDoubleClick`, `onSubmit` and `autoFocus` props, which Chaos, Equinox and penumbra-bridge gained for it. Not reviewed by Dean.
- **D-183: the editor's panels dock like Unreal's.** The outliner and the details panel stack on the right, the content drawer sits under the viewport, and each has its own shown or hidden flag in `EditorLayout`, saved in the preferences. Alt+1, 2 and 3 toggle them, Ctrl+Space, backtick and Alt+4 open the drawer on assets, console and history (the key for the tab already showing hides it), and F11 hides everything and restores what was shown. The editor session ignores Alt with a digit key so the panel shortcuts don't also go to a camera bookmark. Not reviewed by Dean.
- **D-184: the viewport's own settings live in two drop-down menus in its header, not in the toolbar.** The header shows `Perspective` (or `Orthographic`) and `Show`. Camera holds the presets, orthographic and the frame and align commands; Show holds grid, wireframe, colliders, markers, level bounds, hidden entities, statistics, snapping and the grid size. Alt+C and Alt+V open them (`viewportMenu:camera` and `viewportMenu:show`, as commands, so the palette can run them). The main toolbar keeps file, history, tools, snapping, space, pivot, selection actions and play, and the controls hint is one short line pointing at Help. They are `MenuPopup`s driven by `EditorOverlays.viewportMenu`. Not reviewed by Dean.
- **D-185: entities open in document tabs above the viewport.** `EditorTabs` keeps the open entity ids and the active tab (0 is the level). F4, Entity > Edit in Tab and the context menu open a tab and select the entity, Ctrl+W closes the active tab, Ctrl+Tab cycles, and a tab whose entity was deleted closes itself. The outliner and details panel stay docked for every tab and follow the selection, so the tab's page lists the entity's components and the values are edited in Details; the level viewport is rebuilt when its tab is shown again (the camera lives in the engine, so the view is unchanged). Not reviewed by Dean.
- **D-186: an entity tab edits one component at a time.** The page lists the entity's inspector groups as components with their property counts, and the chosen one's fields are shown beside the list with the same `InspectorField` rows the details panel uses, so both edit through the same path and stay in step. A picture of the entity on the page needs a new editor API call and is its own card (`entity-tab-picture-render-one-entity-in-the-entity-page`). Not reviewed by Dean.

## Silhouettes

- **D-187: Silhouettes are folders of Calamus files, resolved by a Nox library over Amanuensis's Calamus C API.** A Silhouette is a folder with `root.calamus`, child files, same-named child folders and an `overrides/` folder of patches named by dotted path. `@base` makes a variant on a root and a nested instance on a child, so a Silhouette can hold Silhouettes; cycles and missing Silhouettes are errors that name them. Scenes are folders of instance files (`@id`, `@base`) that load into an ordinary `WorldFile`. Values cross the C boundary as JSON text, so the C API stays small (parse, load against a base, write, hash). `lantern-native` compiles Amanuensis's current sources into its own static library, because the Chaos and Nyx builds still use the old `Amanuensis::` names and cannot share the `amanuensis` target. Child ids are assigned when a scene loads, after the largest `@id`. See `docs/silhouettes.md`.
- **D-188: The editor places Silhouettes as copies, and a Silhouette replaces a palette template of the same name.** The editor adds a template for every folder under `assets/silhouettes` after reading `palette.json`; the same id replaces the palette entry, and Silhouette templates are never written back to `palette.json`. Placing one inserts the root and its descendants with new ids, parents and unique names as one undo step. `editor.label` and `editor.lift` in a root's values feed the menu and are ignored by the world file. Linked instances (a Calamus scene folder with override states in the inspector) and the blueprint-style Silhouette editor tab are separate, later slices. Provisional: not reviewed by Dean.
- **D-189: the Silhouette tab edits files as text, one change per button, with no save step.** Add, rename and delete write the component's `.calamus` file (or remove it and its same-named folder) the moment the button is pressed, then the Silhouette's palette template is rebuilt, so the tab, the preview and the Add menu show the files as they are. A new child is a minimal file (`@type`, `name`, `transform.position`); rename rewrites only the `name` line; a component with no file of its own (inherited from a base) and the root can't be deleted, and a variant's own names aren't edited yet. This keeps files as the one source of truth and avoids a second in-memory copy with a save command, at the price of no undo for these edits (version control is the undo). Selection in the tab is by position, because two components can share a name. Provisional: not reviewed by Dean.
- **D-190: a new Silhouette is a folder with one root file, and a variant sets only its name and label.** The tab's New Silhouette writes `silhouettes/<name>/root.calamus` holding a block (name, position, `editor.label`); New variant writes a root with the open Silhouette as `@base` and two marked values, `name` and `editor.label`, each with the base's value beside it. Names are lowercase letters, digits and underscores because the name is the folder, the template id and the `@base` path; an existing folder is never overwritten. Provisional: not reviewed by Dean.
- **D-191: a Silhouette value is edited by key and JSON text, and numbers are compared as numbers.** `EditSilhouetteValue` resolves the component's file over its base, replaces (or adds, or for a revert restores or removes) the value at a dotted key, and writes the file through Calamus, so marks stay minimal. The base's resolved values and the edited value are both written the way the editor's JSON writer writes them before they are compared, because Calamus compares numbers by text and `0` against `0.0` would mark every number as the variant's own. Typing JSON is the first slice; typed fields in the inspector's own widgets are a later one. Provisional: not reviewed by Dean.
- **D-192: moving, rebasing and deleting are file moves, with the same checks the library makes.** Moving a component renames its file and its same-named folder into the new parent's folder, so what hangs from it comes along; nothing is moved if the name is taken. Changing a base rewrites the root's `@base` line (and drops the stale `@base_hash`) and puts the old text back if the Silhouette no longer resolves. Deleting a Silhouette scans every other Silhouette's files for its `"silhouettes/<name>"` path and refuses while one names it. Renaming a whole Silhouette is left out because every `@base` that names it would need rewriting. Provisional: not reviewed by Dean.
- **D-193: visual scripting is a graph file that generates Nyx, not a second runtime.** A graph (`*.graph.json`) is a set of typed functions made of execution and data nodes. A generator turns it into ordinary Nyx source, one namespace per graph, which the game mounts like any `.nyx` file. Nyx calls a graph function like any function, and a graph's *Call* and *Nyx* nodes call any Nyx function or hold a snippet of Nyx text, so one function can be part graph and part text. A graph can reach only what Nyx can, which means only the host API, so it sits on Nox exactly as game code does and the engine never learns graphs exist. Rejected: interpreting graphs in Nox (a second language to debug) and compiling them to Nox (game code is Nyx, D-023). The file is strict JSON through `lantern:::json` rather than Calamus because a graph has no inheritance to record. Design: `docs/visual-scripting.md`. Provisional: not reviewed by Dean.

## D-194: generated graph Nyx is committed

The Nyx a game graph generates (`games/<name>/game-graphs/generated/<Name>.nyx`) is committed beside the graph. A game then runs from a plain checkout with no build step, and a diff shows what a graph change did to the Nyx. `mist run graphs` regenerates it, and a test fails when a committed file is stale. Rejected: build-only output, which would make every cloud thread and Dean's Mac run a generator before the game would load.

## D-195: a scripted entity names its graph in two properties

An entity of kind `scripted` carries `graph` and `function` text properties (`function` is `Update` when missing), and the game runs `GraphBehaviours:::Run(graph, function, entity, time)` for it each step. `GraphBehaviours.nyx` is generated beside the graphs because Nyx can't call a function by name. A behaviour function has the fixed shape `(entity: i64, time: f32) -> void`; it keeps no state of its own, so anything that must persist lives in the world through the host API. Rejected: a dotted `behaviour.graph` property (flat properties are what every other entity kind reads), and calling arbitrary graph signatures (the engine can't know the arguments).

## D-196: the editor saves graphs where the game reads them

When `games/platformer/game-graphs/` exists beside the assets, the Graph tab lists, opens and saves graphs there, and Save regenerates `generated/` so the committed Nyx stays current (D-194). Without that folder it falls back to `<assets>/graphs/`. If the Nyx build fails the graph file is still saved and the message names the problem. Rejected: two graph folders, which left editor-made graphs the game never saw.

## D-197: a `when` node is a statement `when` over literal cases

A `when` node has a subject (`a`), any number of cases (a match value and a wire each) and an optional `else`. It generates a statement `when` with block arms, so every arm may run a whole chain, and the arms are literal or computed values, never enum patterns. With no `else` it ends in `_`, which does nothing, so a value that matches no case just continues. Cases are stored as `cases: [{ "match": ..., "do": id }]` rather than parallel fields, so a file reads as what it means. Rejected: enum patterns and bindings (a graph has no types to name them with yet), and value-producing `when` (a data node, later if wanted).

## D-198: a graph error is located by the generated file's own header

When a script error's stack has a frame in a generated graph file, the game's error text adds `graph <name>, node <id>`. The graph is found from the first line of the generated file (`// Generated from <name>.graph.json`) and its folder (the file's `generated` folder's parent), and the node from `NodeAtLine`, so the runtime needs nothing new and no node ids are written into the Nyx. Rejected: writing `//@id` markers into the committed Nyx (noisy diffs) and a runtime hook for the node. Highlighting the node on the editor canvas is left until the editor can show a game's errors.

## D-199: renaming a Silhouette rewrites every `@base` that names it

`RenameSilhouette` moves the folder and rewrites the `"silhouettes/<old>"` path in every file of every other Silhouette (as a base, or as the base of a child), dropping the `@base_hash` of each rewritten file, then resolves each of them and reports the first that no longer loads. A taken or unusable name changes nothing. This closes the gap D-192 left. The editor's palette drops the old template and loads the new one and the variants again. Known gap: a Silhouette that replaced a built-in palette template of the same id (D-188) leaves that id missing from the palette until the editor restarts. Rejected: keeping the old name as an alias (two names for one folder).

## D-200: Silhouette values are edited in typed fields chosen from the value's JSON text

Each resolved value row becomes a field by its shape: `true` or `false` a checkbox, a number an input, a quoted string a text input (quotes added and escaped when written), two to four numbers one input per axis, anything else a raw JSON input. The helpers are plain Nyx (`SilhouetteValues.nyx`) so a Nox test can mount and call them; `SilhouetteValueField.chaos` only lays them out. A field commits on Enter through the same `editorSilhouetteEditValue` path as the typed key and JSON boxes, so marks, hashes and refresh are unchanged. A vector commits one axis at a time. Rejected: a per-key schema (the Silhouette format has none yet), so a number stored as text or an enum looks like a plain text field.

## D-201: The Graph tab reads the graph's revision from the editor's own tick

The Graph page's `OnTick` never ran when the page was mounted inside the workspace's slot, so selecting a node on the canvas, or any change made on it, left the details panel on what it showed before. `EditorDocument.Tick` now reads `editorGraphRevision()` into a `graphRevision` signal, which the page reads when it renders, and selecting or deselecting a node on the canvas bumps the graph's `generation`. Penumbra's `TextInput` and `TextArea` also stopped reporting the pointer as consumed when it is not over them (penumbra-ui-engine #31), which had kept the field rows before the focused one from receiving clicks. Provisional: not reviewed by Dean.

## D-202: A placed Silhouette root remembers its Silhouette, and one command rebuilds every placed instance

The editor still places Silhouettes as copies (D-188). The root's prototype now carries a text property `silhouette` with the Silhouette's name, which the game ignores like any free property. `Session.RefreshSilhouetteInstances` finds every root whose property names a Silhouette in the palette and, as one undo step, replaces the root (keeping id, name, parent and transform), removes its descendants and inserts the Silhouette's descendants with fresh ids, reusing the old descendants' names in order. The Silhouette tab's **Refresh placed instances** button runs it for all of them; a per-Silhouette or per-selection refresh and keeping instance-level overrides are later work. Rejected: refreshing on every Silhouette edit (it would silently discard changes made on instances). Provisional: not reviewed by Dean.

## D-203: `mist run test` also builds the world editor

`nox test` compiles only what each test names, so `engine/editor-backend` and the editor app had no compile check: a syntax error in `EditorApi.nox` (D-202) was merged and only failed when the editor started. `engine/tools/run-tests.sh` now ends by building `apps/world-editor/nox` when it runs the whole suite, so the same command catches it. A single suite named on the command line skips the build. Provisional: not reviewed by Dean.

## D-204: A graph is laid out in columns by execution and value flow, on opening and on Format graph

`ArrangeFunction` gives every node a column from the longest chain of links into it (execution links run from a node to its targets, value links from the node a reference names to the node using it), pulls a value-only node with no inputs next to its first user, orders each column with a few barycentre sweeps, and stacks each column top to bottom. It runs when a function's nodes all sit at one place, and from **Format graph**; saved positions are never moved otherwise. Rejected for now: fractional zoom (the canvas draws with whole-number pixel scales) and routing wires around nodes. Provisional: not reviewed by Dean.

## D-205: Adding a node is a right-click menu, and a wire dropped on empty canvas offers likely nodes first

The "Add a node" palette in the Graph tab's side panel is gone. A right-click on empty canvas, or letting go of a dragged wire on empty canvas, asks the interface for a menu through a counter (`menuRequest`) that the editor tick polls, the same way it polls the graph revision (D-201); the interface opens a popup at the pointer and the pick comes back as `editorGraphAddNodeFromMenu(kind)`, which adds the node at the grid-snapped click position and wires it to the pin it came from. The suggestion lists are fixed per source (execution pin, comparison output, other value output) and not learned. Also fixed: the canvas treated button 1 (middle) as the right button, so right-click on a pin never disconnected in the real editor; the platform numbers buttons 0 left, 1 middle, 2 right. Provisional: not reviewed by Dean.

## D-206: The Graph canvas is anti-aliased and uses a baked Actor, not the 8 by 8 bitmap font

The canvas was blocky next to the editor's own UI. It still draws into a pixel array (no new dependency, same viewport path), but shapes are drawn with coverage: `FillRounded`, `Disc`, `Ring`, `Arrow` and wires collected into a per-wire coverage buffer. Text is Actor baked by `engine/tools/bake-graph-font.py` at 10, 14, 20 and 26 pixels (one per zoom level, 1 to 4) as hex coverage in a generated Nox file of about 100 KB; glyphs are placed at sixteenth-pixel advances, with no kerning or hinting. Rejected: linking SDL_ttf into the canvas path (a new native dependency for one tab) and drawing the nodes as Penumbra widgets (a rewrite of hit testing and wires). The look keeps Lantern's warm selection colour and the per-kind header hues. Provisional: not reviewed by Dean.

## D-207: The editor learns of a game script error through a file the game writes, and marks the graph node

Play starts the game with `LANTERN_ERROR_FILE` naming `worlds/.editor-play.error.txt`. `GameHost.ReportError` writes the error text there (replacing the file; the game already skips a repeated message). The editor reads it while the game runs and once more before noting that it ended, reads the graph and node out of the text (`FailureFromText`), notes the error in the Console and bumps the graph revision, so the Graph tab redraws; `GraphRender` hands the node to the canvas, which draws a red frame and badge. The mark clears when Play starts again. Rejected: reading the game's stderr through a pipe (the game inherits the editor's output so the log stays in one place) and a socket (more than one line of text needs). Also fixed `DescribeHookError`, which built its extra lines with `\n` inside a template string, which Nox doesn't escape, so the "in graph ..., node ..." lines carried a literal backslash-n. Provisional: not reviewed by Dean.

## D-208: Graph zoom is a percentage, anchored at the pointer, and every text size is baked

Zoom was whole steps 1 to 4, so a trackpad (many small wheel events) jumped between the ends. It is now a percentage (`zoom`, 35 to 400, 100 is one pixel per graph unit); every length goes through `Scaled`. A wheel notch changes it by 12 percent and the point under the pointer stays put. `bake-graph-font.py` now bakes Actor at every size from 9 to 28 pixels (about 500 KB of generated source) so text always matches the zoom. Text is dropped below 70 percent, as Blueprint does when zoomed far out. Also: the asset card's thumbnail viewport is `pointer-events: none`, so a click on the picture reaches the card (the viewport used to swallow it). Provisional: not reviewed by Dean.

## D-209: Graph variables are typed locals declared at the top of the function

A variable is `{ name, type, default }` on its function, not a node, so the Variables list is the one place it lives and a `get` or `set` node only names it. The generator emits `name~ type = default;` before the first node and treats the name as declared, so `get` reads it, `set` assigns it (no `type` needed) and Nyx text sees it. Removing a variable also removes its nodes, because a dangling `get` could not generate. Only primitive types have a default (`bool`, `string`, `f32`, `f64`, integers); any other type needs a start value typed in. Pin and wire colours come from `ValueTypeOf`, which follows a reference to its source (literal, input, variable, comparison, operator operands, declared local) up to a depth of 8. Rejected: variables as a node in the graph (Blueprint keeps them in a panel) and per-instance variables shared between functions (needs class-level state, which a graph does not have yet). Provisional: not reviewed by Dean.

## D-210: The Assets panel is a folder browser with search over virtual roots

The panel lists roots (Scripts, Graphs, Silhouettes, Textures, Models, Sounds, Music, Worlds) on the left and the open folder's contents as cards. A folder with a `root.calamus` is a Silhouette and keeps its rendered thumbnail, drag-to-place and Edit; other files get an icon card by extension (script, graph, texture, mesh, sound, world, material), and a graph opens in the Graph tab. The search box searches every root by name, ignoring case, to a depth of six. Roots map to real folders in `engine/src/editorbackend/AssetEntries.nox`, so the browser needs no file watching or new index; material Silhouettes, a separate Silhouettes tab and drag for non-Silhouette assets are still to come. Provisional: not reviewed by Dean.

## D-211: The viewport's Game Lighting option copies the game's chapter one night

Show > Game Lighting (also in Settings > Graphics) gives the editor viewport the sky, fog, moon, ambient light, bloom, exposure and colour grade the game uses for chapter 1 (`ApplyGameLook` in `engine/src/editor/GameLook.nox`). Lights in the level still light it, since they are added after. The values are copied from `games/platformer/settings/chapter-looks.nyx` because the editor backend can't run game Nyx; the test `the-viewport-can-wear-the-games-night-look` reads that file and fails if its chapter one line changes, so the copy can't drift quietly. Rejected: running the game's Nyx look code inside the editor (a large host surface for one function) and a per-level chapter choice for now (the engine doesn't read a level's `chapter` yet). Not yet there: the day look, other chapters, wet streets and the game's authored lights. Provisional: not reviewed by Dean.

## D-212: A material is a Silhouette of type `material`

A folder under `assets/silhouettes` whose `root.calamus` says `@type material` is a material Silhouette, with the properties `texture` (a name in `textures/`, or "" for none), `tint`, `metallic`, `roughness` and `emissive`. An entity's `material` name finds it at `silhouettes/NAME`, then `silhouettes/materials/NAME`; a name with no material Silhouette still means `textures/NAME.png`, so every existing world draws as before. The editor viewport and the template preview draw such an entity through `DrawWithMaterial` with the Silhouette's renderer material and its tint multiplied into the entity's own. The content browser shows these folders as materials, and the Silhouettes tab leaves them out because they aren't placeable. Four starters live in `assets/silhouettes/materials/`: `brushed_steel`, `warm_plaster`, `wet_asphalt`, `neon_sign`. Not done: the game's world loader still resolves a material name as a texture only, so these materials show in the editor now and in the game once that follows. Test: `a-material-silhouette-sets-how-a-surface-looks`.

## D-213: The game gets material Silhouettes as generated Nyx, not as Calamus files

Silhouette files belong to the game project and the editor, not to the built game. `lantern:::silhouette:::GenerateMaterialsNyx` turns every material Silhouette into `games/platformer/game-assets/BakedMaterials.nyx`, a committed generated file in the same way as the graphs' `generated/` Nyx: `BakedMaterials:::Register()` calls the new host function `lantern:::world:::SetMaterialLook(material, texture, tint, metallic, roughness, emissive)` for each. The game's `game-assets.nyx` calls it after its own materials, and `SpawnPiece` multiplies the look's tint into the piece's and gives the renderer its material. The editor rewrites the file when a Silhouette value changes and on level save; `mist run materials` writes it by hand and `the-game-materials-are-built-and-current` fails when it is stale. The game build and its tests need no Calamus. Not done: Silhouettes and assets still live in the repository's `assets/` folder; moving them into the game project folder, and compiling placed Silhouettes into objects with the game's binary, is on the engine follow-ups board.

## D-214: Photos are saved by `lantern:::screen:::Capture(path)`, taken on the next frame without the UI

A shutter press asks the engine to save the next drawn frame as a PNG. The engine leaves the game UI and debug HUD out of that frame, so the viewfinder, flash and counter are not in the picture, creates the folder, and writes the file. The game saves every press, counted or not, as `photos/photo-N.png` relative to the folder it runs in (git-ignored). Provisional: a per-user pictures folder and an album screen are not decided.

## D-215: The Rust workspace is removed; Lantern is Nox, Nyx, Mist and the C libraries they build

Nothing built or ran the Rust engine after the port: `mist run game|editor|test|build-shaders|graphs`, the shell scripts under `engine/tools`, the editor's and `lantern-native`'s CMake builds and the cloud bootstrap never invoked Cargo, and there is no CI. `crates/`, `apps/lantern`, `examples/frame-loop`, `tools/asset-gen`, `Cargo.toml`, `Cargo.lock`, `rustfmt.toml` and the root `shaders/*.wgsl` (superseded by the GLSL in `engine/shaders`) are deleted. The old crates' C++ copies (`crates/chaos-ui/native`, `crates/editor-backend/include`) were older versions of `engine/native` and `engine/editor-backend`, which stay. `docs/architecture.md`, a tour of the Rust crates, is deleted with them; `docs/engine-structure.md` is its replacement.

`tools/asset-gen` was the only thing that regenerated the placeholder textures, sounds and `gem.glb` (D-008). The committed files in `assets/` are now the source of truth; to change one, edit it or write a Nox generator. This is provisional: if reproducible assets matter again, port the generator to Nox. The `rust-*.trace` goldens under `engine/tests` stay as frozen fixtures, and the Rust code is still in git history. To undo: `git revert` the commit that removed them.

## D-216: A game's assets live in its project folder

`assets/` moved to `games/platformer/assets/`, so the Silhouettes, material Silhouettes, textures, models, sounds, music, worlds and editor palette are part of the game project, which is what the editor opens. `game.json`'s `assets` is now `assets`, and the editor finds the checkout from the assets folder by going three folders up to the one with `engine/nox.json`. Nothing about the on-disk formats changed. What the game needs from the Calamus files is still compiled in as generated Nyx (D-213: `BakedMaterials.nyx`); the Calamus files themselves stay with the project. Still to do: compile placed Silhouettes and scenes the same way, so a built game carries objects and not the project's source files.

## D-217: A built game is a package without the project's Calamus files

`mist run package` (`engine/tools/package-game.sh`) builds the release engine and copies the game into `engine/build/package` beside the binary and the compiled shaders, leaving out `assets/silhouettes` (Silhouettes and material Silhouettes), `assets/editor`, every `.calamus` and `.graph.json` file, and the editor's scratch files; it fails if a Calamus file reaches the package. This is possible because a placed Silhouette is already a plain entity in the world file, and the values of the game's material Silhouettes and graphs are generated Nyx (D-213), so the running game never reads a Calamus file. Provisional: the data is still loose files beside the binary, not embedded in it. Packing worlds, textures, sounds and models into the executable itself is not decided.

## D-218: A packaged game is one file: the data is appended to the engine and unpacked on first run

`mist run package` writes `engine/build/package/lantern`. `engine/tools/pack-game` appends an archive of the game folder (without Calamus, graph JSON or editor files) and `shaders/compiled` to the release binary: for each file its path and bytes, then the archive's length and the text `LANTERNPAK1`. At start `lantern:::application:::RunEmbeddedGame` reads its own program (`/proc/self/exe`, else its first argument); with an archive it unpacks into `$LANTERN_CACHE_DIRECTORY`, else `$TMPDIR`, else `/tmp`, in `lantern-<FNV-1a of the archive>`, writes a `.complete` marker last, sets `LANTERN_SHADER_DIRECTORY` and runs the unpacked `game` folder like any game. Provisional: this is a self-extracting file, not a virtual file system. Nyx's file mounts, SDL_image, cgltf and the audio loader all take paths, so serving reads from memory is a larger change; it is not decided. Reading the whole program at every start costs a few milliseconds per megabyte. Not done: Windows (no `/proc`, and `TMPDIR` is unset).

## D-219: The placeholder assets are generated by Nox again

`engine/tools/generate-assets` (`mist run assets`, `--check`) replaces the deleted Rust `asset-gen` (D-008): the checker textures, the six sound effects and the music loop, and `gem.glb`. The sounds are sample for sample what the Rust tool wrote and the textures are pixel for pixel; the PNG is written as stored zlib blocks, so the files are larger than the encoder's (64 KB each, they were small). `gem.glb` differs in the last bit of a few normals and in the generator name. The committed files are the Nox tool's output, and `mist run test` fails when they drift. Only the textures the game ships are generated: `ground` and `platform` (`wall` and `player` are no longer in the project).

## D-220: A game run from its project folder reloads when a Silhouette or graph changes

`lantern:::host:::ContentWatcher` hashes the project's `assets/silhouettes`, `game-graphs` (graph JSON and its generated Nyx) and `game-assets/BakedMaterials.nyx` every half second. On a change it runs `BuildGameMaterials` and `BuildGameGraphs`, which write only files whose text changed, and the game host reloads its scripts as F5 does. A failed generation is logged and the previous Nyx stays. No watcher exists when the game has neither folder, which is every packaged game, and `LANTERN_HOT_RELOAD=0` turns it off. A Silhouette already placed in a world file is a copy (D-217), so it isn't touched; a reload restarts the game, as F5 does. Edits to `.nyx`, `.chaos` and `.cosmos` are not watched.

## D-221: The crossing's cameras and photo sights are authored in the world file

Five photo sights and four camera volumes (three with shots, one a framing change for the watcher hall) were added to `the-crossing.world.json` with the existing `photo_sight`, `camera_volume` and `camera_shot` entities, so no engine or game code changed. Shots sit inside the floor's outline so nothing solid is between them and the player; volumes carry a higher `priority` than the follow framing. The crossing's host test now checks it loads with five sights. Provisional: the shot positions are judged from lavapipe screenshots with the pilot walking, not from play on a Mac. Not reviewed by Dean.

## D-222: Wayland and Windows are checked from the cloud as far as they can be

Provisional (2026-11-19): not reviewed by Dean. `engine/tools/check-wayland.sh` runs the game and the stub window script on Weston's headless backend with lavapipe, and `docs/platform-checks.md` records what that showed, the Windows cross-build result and a checklist for a real machine. `SdlContext` sets `SDL_HINT_APP_ID` to `lantern` so a Wayland compositor can match the window to a desktop entry, and `engine/native/CMakeLists.txt` exports every symbol on Windows. Mist's lack of Windows script support is MIST-29.

## D-223: The help line is a wrapped box in the bottom right corner, and the status line sits at the top right

Provisional (2026-11-19): not reviewed by Dean. The station level showed the status line centred under the debug window, which takes the top left, and a 32 px hint that ran off the screen. The status line is now 16 px at the top right, and the hint (`HintText` in `main.nyx`) goes to its own `GameUiModel.hintRows` signal and draws as one 16 px row per wrapped line, at most 30 characters each, in a small bottom right box (`hud.chaos`, `hud.cosmos`). Rule messages such as "Level complete" keep the large centred message. Chaos UI has no text wrapping, so `HintWrap:::Lines` in `ui/hint-wrap.nyx` wraps in the game; wrapping to the box's width inside the engine is a later card. Checked with a lavapipe screenshot of the station at 1280 by 720, not on a Mac.

## D-224: A level streams in rooms named by an entity property, and only engine-only scenery streams

Provisional (2026-11-20): not reviewed by Dean. A world entity with a `room` text property belongs to that room, and a room's bounds are the box around its members. Only a `block`, `ramp`, `point_light` or `spot_light` with no parent, no children and a static or absent collider streams; every kind the game wraps in a class stays resident, because those carry state and the game's lists point at them. The alternatives were a separate rooms file (a second thing to keep in step with the world, and the editor would not know it), a `room` entity that draws a volume (membership by containment fails for a block that straddles two), and streaming the game's kinds as well (a plate or gem would need its state saved and restored). `room` needs no format change, so the editor and every existing level are untouched. See `docs/level-streaming.md`.

## D-225: Rooms load by the flat distance to the nearest of the player, partner and companion, with an urgent ring that ignores the budget

Provisional (2026-11-20): not reviewed by Dean. Load at 70 m, unload at 95 m, load at once inside 24 m, at most 12 entities a step otherwise (`streaming-settings.nyx`). Distance ignores height, so a level that stacks floors loads the stack. The partner and the companion are focus points as well as the player, so a floor is there before the partner steps onto it and stays while she is on it. The urgent ring is what makes teleports (respawn, restart, checkpoint) safe without the game saying so. The numbers were chosen by reasoning and lavapipe screenshots, not play on a Mac.

## D-226: Static collision is removed by group with tombstones and compacted when the dead outnumber the live

Provisional (2026-11-20): not reviewed by Dean. `BeginStaticGroup`, `EndStaticGroup` and `RemoveStaticGroup` on `CollisionWorld`; a removed group's triangles are marked dead and skipped by `NearTriangles`, and the arrays and grid are rebuilt once when dead triangles pass 4096 and outnumber the live. The alternative, rebuilding the grid on every unload, makes each unload cost the level; a free list reusing dead slots needs same-size groups. `StaticTriangleCount` now counts live triangles.

## D-227: The partner's graph is still built once, from the whole level, before streaming starts

Provisional (2026-11-20): not reviewed by Dean. `LevelScene.Load` calls `lantern:::world:::LoadAllRooms()` so the graph build sees every floor, then streaming starts and unloads what is far. The graph keeps its nodes for unloaded rooms, so routes cross the whole level and the partner plans long leads as before. This keeps the partner right across boundaries for free and leaves her code alone, at the price that load time still grows with the level. Building the graph per room as rooms load is the follow-up card.

## D-228: A composed scene is a world file placed by an instance entity, and a placed scene is a room

Provisional (2026-11-20): not reviewed by Dean. Dean asked for scene objects that hold objects, can be placed into the world and stitched together, with a low level of detail peek into the neighbouring scene. Instead of a second container concept, a scene is an ordinary world file, a `scene_instance` entity places it, and the loader expands the instance into one room, so room streaming loads and unloads scenes. Openings are `scene_socket` entities, and stitching snaps two sockets together. The peek is a baked low-detail proxy of the neighbour's near geometry drawn through the socket's rectangle, with no collision, lights or entities. Alternatives: scenes as a runtime object tree the game walks (every consumer, the editor included, would need to understand it, and the baked game would read scene sources), and a portal system with full rendering of the neighbour (the cost of the neighbour returns, which is what streaming removes). Split into four cards and PRs: room streaming, instances and sockets, the peek proxy, editor placement. Not built yet: only the design and the cards. See `docs/level-streaming.md`.
