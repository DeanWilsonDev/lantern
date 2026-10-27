# Lantern architecture

Lantern is a small, readable 3D engine prototype written to answer one question: *what must a
small 3D platformer engine actually do each frame?* It follows Umbra's domain split (see
`umbra-audit.md`) but replaces Umbra's interfaces, injector and managers with concrete Rust types
owned by one composition root. Decisions and their reasons are in `decisions.md` (D-001 to
D-017); this document describes the result.

Start here, then read `apps/platformer-game/src/main.rs` (construction and shutdown) and
`apps/platformer-game/src/game.rs` (`impl Game for PlatformerGame`: everything that happens in a
frame). Every other file is reached from those two.

## Workspace layout

```text
Cargo.toml                  workspace root (default members: everything except the Nyx example
                            and asset generator)
crates/
  core/          lantern-core        math conventions, generational handles and pools, fixed-step
                                     clock, input actions and bindings, colours, logging, debug
                                     values (Umbra src/core)
  platform/      lantern-platform    backend-neutral vocabulary: PlatformEvent, WindowMetrics, the
                                     PlatformBackend and AudioSink traits (Umbra src/platform)
  sdl-facade/    lantern-sdl-facade  SDL3 context, window, event pump, gamepad, audio stream
                                     (Umbra src/raylib-facade)
  renderer-3d/   lantern-renderer-3d CPU-side render data: MeshData, TextureData, primitives,
                                     glTF/PNG loading, Camera, the RenderWorld packet, bitmap font,
                                     immediate-mode Ui (Umbra src/renderer-2d + user-interface)
  sdl-gpu/       lantern-sdl-gpu     the SDL GPU backend: GpuRenderer owns the device, pipelines,
                                     meshes and textures, and draws a RenderWorld
  physics/       lantern-physics     triangles, capsule casts, CollisionWorld, kinematic bodies,
                                     CharacterController (Umbra src/physics)
  audio/         lantern-audio       WAV decoding, SoundBank, software Mixer (new)
  scripting/     lantern-scripting   Nyx host: ScriptHost, the engine API types, ScriptCommand (new)
  debug/         lantern-debug       DebugHud panel and debug_draw helpers (Umbra src/debug)
  engine/        lantern-engine      FrameLoop and the Game trait, InputRouter, World (entities,
                                     transforms, hierarchy, scenes), extract_draws, AudioSystem
                                     (Umbra src/engine)
  nyx-sys/       nyx-sys             raw Nyx C ABI bindings (bindgen; builds Nyx with CMake)
  nyx/           nyx                 safe Nyx wrapper
  world-format/  lantern-world-format the world file (JSON) and world-to-mesh/collision pieces,
                                     shared by the game host and the editor (new)
  editor-core/   lantern-editor-core the world editor's logic: document, undo, selection,
                                     gizmos, inspector, camera, input (new)
  editor-backend/ lantern-editor-backend EditorSession + off-screen viewport behind a C API
                                     (include/lantern_editor.h), built as a static library (new)
apps/
  platformer-game/                   the demo game and composition root (Umbra src/snake-game)
    scripts/platformer.nyx           the game rules in Nyx
  world-editor/                      the world editor: Penumbra shell (C++, CMake) with panels
                                     in Nyx, Chaos and Cosmos; see docs/editor.md
assets/                              textures, model, sounds, music (made by tools/asset-gen)
shaders/                             lit.wgsl, lines.wgsl, overlay.wgsl (compiled at build time)
docs/                                brief, decisions, this file, dependencies, future work
examples/frame-loop/                 the original Nyx hosting example
tools/asset-gen/                     regenerates assets/ deterministically; `--check` verifies
```

### Dependency direction

```text
lantern-core ◄── platform ◄── sdl-facade ◄── sdl-gpu
     ▲   ▲          ▲                           │
     │   └── renderer-3d ◄──────────────────────┘
     │          ▲    ▲
     │          │    └── debug
     │          │
     ├── audio  │
     │    ▲     │
     │    └── engine (core, platform, renderer-3d, audio)
     ├── physics
     └── (nothing) ◄── scripting ──► nyx ──► nyx-sys

apps/platformer-game depends on all of them and is the only place they meet.
```

(Arrows point at the dependency.) Only `sdl-facade` and `sdl-gpu` see SDL types, and only
`nyx`/`nyx-sys` see the Nyx C ABI. Engine, physics, audio, scripting and game code never touch
raw pointers; the game calls the GPU backend with `renderer-3d` types only (`RenderWorld`,
`MeshData`, `MeshHandle`).

## Umbra → Lantern mapping

| Umbra | Lantern | Notes |
|---|---|---|
| `Core::IApplication` + `Engine::Application` | `lantern_engine::frame_loop::{FrameLoop, Game}`, implemented by `PlatformerGame` | The loop calls four hooks: `handle_event`, `fixed_update`, `frame_update`, `render`. |
| `DependencyInjector` registrations | plain construction in `apps/platformer-game/src/main.rs` (`run`) | Construction order is dependency order; see "Ownership and shutdown". |
| `IWindow`, `IInputBackend` + Raylib facades | `lantern_platform::PlatformBackend`, implemented by `lantern_sdl_facade::SdlPlatform` | Tests drive the loop with a scripted fake platform. |
| `SystemManager` / `GameSystemManager` | the fixed call order in `PlatformerGame::fixed_update` | Order is code you can read, not registration order. |
| `SceneManager` + `Scene` | `apps/platformer-game/src/scene.rs` (`LevelScene::load` / `unload`) + `World::despawn_scene` | Every entity carries the `SceneId` that spawned it, so unloading removes exactly those. There is one level, so no scene stack (D-016). |
| `EntityManager` + component pipelines | `lantern_engine::world::World` with generational `EntityId` and `ComponentStore<T>` | Iteration is in slot order, so it is deterministic. |
| `IGameState` + `StateMachine` | `GameState` enum (`MainMenu`, `Playing`, `Paused`) in `game.rs` | Three states with no shared behaviour do not need a framework (D-016). |
| `ActionSet`, `KeyMap`, `InputDispatcher`, `ActionRouter` | `lantern_core::input::{ActionSet, Bindings, InputSnapshot}` + `lantern_engine::input::InputRouter` | Named actions with latched edges, read from a snapshot instead of event-bus callbacks. |
| `IRenderComponentManager` | `lantern_engine::extract::extract_draws` | Fills a `RenderWorld`; the renderer never sees entities. |
| `IRenderer` + Raylib renderer facade | `lantern_sdl_gpu::GpuRenderer` | One concrete backend; a trait would be added with a second one. |
| `renderer-2d` | `lantern-renderer-3d` | Three pipelines: lit, lines, overlay. |
| `physics/collision` | `lantern-physics` | Capsule casts against triangles and a kinematic controller; no rigid bodies. |
| `debug` + `core/debug` | `lantern_core::debug::DebugValues` + `lantern-debug` | Same path-keyed tree (`"Player/Velocity"`). |
| `core/user-interface` + `snake-game/ui` | `lantern_renderer_3d::ui::Ui` + `apps/platformer-game/src/menu.rs` | Immediate-mode labels and buttons with focus navigation. |
| (none) | `lantern-audio` + `lantern_engine::audio::AudioSystem` | Pure-Rust mixer; the SDL audio stream lives in the facade. |
| (none) | `lantern-scripting` + `apps/platformer-game/src/rules.rs` | Rules (Nyx or Rust) decide what happens; Rust moves things. |
| `snake-game` | `apps/platformer-game` | Game rules, content, scripts and composition. |

## Conventions (stated once, tested in `lantern-core`)

| Topic | Convention |
|---|---|
| Handedness | Right-handed. |
| Axes | +Y is up. -Z is forward. +X is right. |
| Units | 1 world unit = 1 metre. Time in seconds. |
| Angles | Radians in code. Anything in degrees is named `_degrees`. Yaw rotates about +Y, positive is counter-clockwise seen from above. |
| Vectors and matrices | `glam` column vectors and column-major matrices. A point is transformed as `matrix * point`. |
| Transform composition | `local = translation * rotation * scale`. `world = parent_world * local`. |
| Clip space | SDL GPU's clip space on every backend: NDC y up, depth 0 (near) to 1 (far). Projection is `Mat4::perspective_rh` (0..1 depth). Depth test is `LESS`. |
| Texture UV | (0, 0) is the top-left texel; v grows downward. Row 0 of uploaded pixels is the top row. |
| Winding | Counter-clockwise triangles are front faces. Back faces are culled. |
| Colours | Linear `f32` RGBA in code. The swapchain is presented as it is configured by SDL (sRGB not forced). |

## Timing

`lantern_core::time::FixedStepClock` implements the proposal from Umbra's
`game-loop-architecture.md`:

- fixed step: 1/60 s;
- each display frame adds the measured time (clamped to `MAX_FRAME_TIME` = 0.25 s) to an
  accumulator;
- times within 0.2 ms of the step are snapped to the step (vsync jitter);
- at most `MAX_STEPS_PER_FRAME` = 8 fixed steps per frame, after which the remaining backlog is
  dropped and counted in the HUD;
- the remainder gives `alpha = accumulator / step` for render interpolation.

Simulation (controller, gravity, jumping, moving platforms, scripts) only runs in fixed steps
and only sees `FIXED_STEP`. Nothing in the simulation reads the frame delta.

## Input

1. The platform pump turns SDL events into `PlatformEvent`s (key down/up by `KeyCode`, mouse
   motion and buttons, gamepad buttons and axes, window events).
2. `InputRouter` keeps device state (keys held, mouse delta, gamepad axes) and, once per display
   frame, evaluates `Bindings` into an `InputSnapshot`: for each action, whether it is held, was
   pressed or released, and its axis value.
3. Pressed and released edges are *latched* until a fixed step consumes them. If a display frame
   runs zero fixed steps, a jump press is not lost; if it runs three, only the first sees the
   press. Mouse look is accumulated and consumed by the frame update, since the camera is a
   presentation concern. The loop hands each fixed step `InputRouter::take_fixed_step_snapshot()`
   and the frame update `InputRouter::frame_snapshot()`.

## Per-frame sequence

This is `FrameLoop::run_frame` (`crates/engine/src/frame_loop.rs`) calling into
`PlatformerGame` (`apps/platformer-game/src/game.rs`). Indentation is call depth.

```text
FrameLoop::run_frame
├─ 1. events
│   ├─ platform.pump_events()                SDL_PollEvent → PlatformEvent
│   ├─ input_router.handle_event(event)      device state: keys, buttons, axes, mouse delta
│   └─ game.handle_event(event)              (unused by the platformer)
├─ 2. time
│   └─ clock.advance(frame_seconds)          → step_count N, alpha, dropped backlog
├─ 3. N × game.fixed_update(step)            only while GameState::Playing
│   ├─ world.store_previous_transforms()     interpolation start point
│   ├─ collision.begin_step()                kinematic bodies remember last transform
│   ├─ rules.on_fixed_update(view, dt)       Nyx OnFixedUpdate or NativeRules
│   ├─ apply RuleCommands                    platform position → set_body_transform, ...
│   ├─ player.fixed_update(...)              CharacterController::step:
│   │                                          ride platform, accelerate, gravity, jump,
│   │                                          move-and-slide (capsule casts), step up,
│   │                                          ground probe and snap
│   ├─ player.sync_entity(), shadow ray      controller → entity transform
│   ├─ collect RuleEvents                    jumped, landed, trigger entered, fell, reset
│   ├─ rules.on_event(view, event) for each  → more RuleCommands, applied in order
│   ├─ scene.animate_gems()                  presentation-only spin and bob
│   └─ world.update_world_transforms()       hierarchy, parent before child
├─ 4. game.frame_update(frame)
│   ├─ apply pending state change            (mouse capture needs the platform)
│   ├─ F1 / F2 / F5, pause, camera look      from the per-frame input snapshot
│   ├─ update_debug_values()                 HUD data
│   └─ audio.update()                        mix enough to keep ~60 ms queued in SDL
└─ 5. game.render(frame)                     skipped while minimised
    ├─ camera.update()                       follow the interpolated player, sphere-cast
    │                                          against the level so it never clips
    ├─ extract_draws(world, alpha)           visible MeshRenderers → DrawItems
    ├─ collision view lines (F2)
    ├─ menus (Ui) and overlay text, HUD (F1)
    └─ renderer.render(&render_world)
        ├─ acquire command buffer + swapchain texture
        ├─ copy pass: upload this frame's line and overlay vertices
        ├─ render pass: lit opaque draws (frustum-culled, sorted by texture then mesh),
        │                 debug lines, overlay quads
        └─ submit and present
```

### Following one jump through the code

1. `SdlPlatform::pump_events` turns `SDL_EVENT_KEY_DOWN` (Space) into
   `PlatformEvent::KeyDown { key: KeyCode::Space, repeat: false }`.
2. `InputRouter` marks the `jump` action pressed and latches the edge until a fixed step takes it.
3. `PlatformerGame::fixed_update` reads `pressed(jump)` (through `pilot_input`) and passes it to
   `Player::fixed_update`, which calls `CharacterController::step`. The controller buffers the
   press for 0.12 s, so a press just before landing still jumps.
4. The controller returns `ControllerEvents { jumped: true, .. }`; the game queues
   `RuleEvent::PlayerJumped`.
5. `ScriptedRules::on_event` calls the script's `OnJump`; `Sound:::Play("jump", 0.8f)` queues
   `ScriptCommand::PlaySound`, translated to `RuleCommand::PlaySound`.
6. `apply_rule_command` calls `AudioSystem::play`; the next `audio.update()` mixes it into the
   SDL stream.
7. The player entity's transform was set by `sync_entity`; `extract_draws` interpolates it
   with `alpha` and the GPU backend draws it.

## Ownership and shutdown

Construction in `main.rs::run`, in dependency order:

1. `SdlContext::init()` — SDL video, events, gamepad.
2. `SdlWindow::create` — the window.
3. `GpuRenderer::new(&window)` — device, claims the window, pipelines, depth texture.
4. `GameAssets::load` — textures and meshes uploaded to the renderer; the game keeps handles.
5. `AudioSystem::new(open_audio(&sdl))` and `load_sounds` — no device is not an error.
6. `choose_rules()` — Nyx script, or `NativeRules` on failure.
7. `PlatformerGame::new(GameParts { .. })` — takes ownership of all of the above and loads the
   level (`LevelScene::load`).
8. `SdlPlatform::new(window)` and `FrameLoop::new(InputRouter)`; `frame_loop.run(..)`.

Shutdown is the reverse and is mostly done by `Drop`:

- `game.shutdown_level()` unloads the level explicitly first: `rules.on_exit`, then
  `LevelScene::unload`, which despawns every entity with the level's `SceneId` and removes the
  level's meshes from the renderer.
- When `run` returns, locals drop in reverse order: the frame loop, the platform (closes the
  gamepad), then the game. `PlatformerGame`'s fields drop in declaration order: rules (the Nyx
  runtime), world, assets (plain handles), `GpuRenderer` (waits for the GPU, releases every
  mesh and texture, releases the window claim, destroys the device), then `AudioSystem` (closes
  the SDL audio stream).
- SDL objects share an `Rc` to the SDL init guard, and the GPU renderer holds a clone of the
  window handle. So the window outlives the renderer even though the platform drops first, and
  `SDL_Quit` runs only after the last SDL object is gone.

Resource rules:

- GPU meshes and textures live in the renderer's `Pool`s and are addressed by generational
  `MeshHandle` / `TextureHandle`. A stale handle is skipped, never dereferenced. SDL GPU defers
  the real destruction of a released buffer or texture until command buffers using it finish,
  so removing a mesh mid-game is safe.
- The level scene owns the meshes it created and returns them in `unload(self)`; taking `self`
  by value means an unloaded scene cannot be used.
- The Nyx runtime lives inside `ScriptHost`; scripts hold only `i64` ids, so nothing in Nyx can
  dangle when entities are despawned.

## Rendering

- One mesh vertex format: position, normal, uv, colour. Lines and overlay quads have their own
  small formats (`LineVertex`, `OverlayVertex`).
- Pipelines: lit opaque (texture × vertex colour × tint, one directional light plus ambient and
  a little fog), debug lines (depth-tested, no depth write), overlay (alpha-blended screen-space
  quads sampling the font atlas or a white texel).
- WGSL in `shaders/` is translated by naga at build time to SPIR-V (Vulkan) and MSL (Metal).
- The depth texture is recreated when the swapchain size changes. A minimised window renders
  nothing.
- `LANTERN_GPU_DEBUG=1` creates the device with validation.

## Physics and the controller

- The level is triangles: every block and ramp mesh that is drawn is also added to the
  `CollisionWorld`, so what you see is what you collide with (`scene.rs::level_pieces`).
- Static triangles live in a uniform 4 m grid. Moving platforms are kinematic bodies with local
  triangles and a transform the rules set each step.
- `CharacterController::step` does move-and-slide with capsule casts (conservative advancement
  against triangles), climbs steps up to 0.3 m, refuses slopes over 46°, snaps to the ground
  within 0.3 m, and is carried by moving platforms. D-015 lists the feel numbers and reasons.

## Audio

`AudioSystem` (engine) owns a `Mixer` and `SoundBank` (lantern-audio) and an optional
`Box<dyn AudioSink>` (the SDL stream). `play(name, volume)` starts a voice; `update()` once per
frame mixes just enough stereo `f32` to keep about 60 ms queued. There is no audio thread of our
own; SDL's device thread drains the queue. Music is a looping voice on its own volume bus.

## UI and text

`lantern_renderer_3d::text` lays out an embedded 8×8 bitmap font, scaled by whole pixels for the
display density. `Ui` is immediate mode: each frame the menu functions in `menu.rs` call
`title`, `label`, `button` and `spacing`; keyboard or gamepad up/down moves focus and confirm
activates. Everything becomes overlay quads in the `RenderWorld`. The debug HUD is the same
text path, drawing `DebugValues` grouped by their first path segment.

## Scripting

The game's rules sit behind `GameRules` (`apps/platformer-game/src/rules.rs`), with two
implementations: `ScriptedRules` (Nyx, default) and `NativeRules` (Rust).

1. Before a hook, `ScriptedRules` copies what scripts may read into a `ScriptSnapshot`: time,
   pressed and held actions, entity ids by name, entity positions and visibility.
2. `ScriptHost::call(hook, &snapshot, args)` calls the method on the script's `Application`
   class through the `nyx` crate. The host types read the snapshot and record writes as
   `ScriptCommand`s.
3. `ScriptedRules` translates `ScriptCommand`s to `RuleCommand`s (dropping stale ids) and the
   game applies them in order.

The script API, as Nyx sees it:

| Nyx | What it does |
|---|---|
| `Game:::Log(text)`, `Game:::Time()`, `Game:::ResetPlayer()` | logging, simulation time, respawn |
| `Game:::SetStatus(text)`, `Game:::ShowMessage(text, seconds)` | on-screen text |
| `Input:::IsPressed(action)`, `Input:::IsHeld(action)` | named actions |
| `Entity:::Find(name)` | an `i64` id; throws `Error:::NotFound` |
| `Entity:::Position(id)`, `Entity:::SetPosition(id, v)` | local position |
| `Entity:::IsVisible(id)`, `Entity:::SetVisible(id, visible)` | mesh visibility |
| `Sound:::Play(name, volume)` | a one-shot sound |
| `Vector:::New(x, y, z)`, `.X()`, `.Y()`, `.Z()` | a 3D vector |

Hooks (all optional): `OnStart`, `OnFixedUpdate(dt)`, `OnTriggerEnter(id)`, `OnPlayerFell`,
`OnJump`, `OnLand(speed)`, `OnResetPressed`, `OnExit`. A failing hook is logged, counted and
shown in the HUD; the game keeps running. A script that fails to load is reported on screen and
the native rules take over. F5 reloads the script. See D-017.

## World editor

The editor is a separate program that edits `assets/worlds/*.world.json`. It shares only
`lantern-world-format` and the assets with the game. Its logic is `lantern-editor-core`, its
viewport is drawn off-screen by `lantern-editor-backend`, and its panels are Nyx, Chaos and
Cosmos on Penumbra (`apps/world-editor`). The game's content reaches it as data through
`assets/editor/palette.json`. See `docs/editor.md` and D-018 to D-022.

## Rewrite-in-another-language checklist

Lantern is a reference for a later rewrite in Dean's own language. The parts to carry over, in
the order they were built and tested:

1. **Conventions first.** Right-handed, +Y up, -Z forward, metres, radians, column vectors, 0..1
   depth. Port `crates/core/src/math.rs` and its tests before anything that draws.
2. **Generational handles.** An `(index, generation)` pair and a pool that checks generation on
   lookup (`crates/core/src/handle.rs`). Two integers and a comparison in any language.
3. **Fixed-step clock** (`crates/core/src/time.rs`): accumulator, clamp, snap, step cap, alpha.
   Its tests are plain numbers and port directly.
4. **Input with latched edges** (`InputRouter`): edges survive frames with zero fixed steps and
   are consumed once.
5. **World**: entities with a scene tag, a parent, local and world transforms, and previous
   world transforms for interpolation. Parents are updated before children.
6. **Render packet**: the game fills a plain `RenderWorld` (camera, light, draw items, lines,
   overlay); the backend only reads it. This is the seam to keep when the backend changes.
7. **GPU backend**: the SDL GPU calls in `crates/sdl-gpu` are a C API and port 1:1. Keep the
   resource lifetime rules above.
8. **Physics**: closest-point routines (`geometry.rs`), conservative-advancement capsule cast,
   then the controller. Port the 15 controller tests with it; they encode the feel.
9. **Rules seam**: snapshot in, commands out. It works the same whether the rules are Nyx,
   native code, or the new language.
10. **Audio**: a mixer that fills a queue each frame is enough at this scale; no callback thread.

Things that are Rust-specific and need an explicit replacement:

- **`Drop`**: GPU resources, SDL objects and the Nyx runtime free themselves. Without
  destructors, call `destroy` explicitly in the order in "Ownership and shutdown".
- **`Rc` guards**: `SdlInitGuard` and the shared window handle encode "SDL_Quit after the last
  SDL object". Replace with explicit shutdown order.
- **Enums with data**: `PlatformEvent`, `RuleCommand`, `RuleEvent`, `ScriptCommand` are tagged
  unions; Nyx enums with payloads map directly.
- **Traits**: `PlatformBackend`, `AudioSink`, `Game` and `GameRules` are the only traits. Each
  can be an interface or a struct of function pointers.
- **`Result`**: errors are values everywhere; in Nyx, `throw` an error-enum variant.

## Dependency boundary inventory

Where Lantern meets code it did not write, and what crosses each boundary:

| Boundary | Crate | What crosses | `unsafe` |
|---|---|---|---|
| SDL3 (window, events, gamepad, audio) | `lantern-sdl-facade` | raw `SDL_*` calls; out: `PlatformEvent`, `WindowMetrics`, `AudioSink` | yes, each call has a `SAFETY` note |
| SDL3 GPU | `lantern-sdl-gpu` | raw `SDL_*GPU*` calls; in: `RenderWorld`, `MeshData`, `TextureData`; out: handles, `FrameReport` | yes, each call has a `SAFETY` note |
| Nyx C ABI | `nyx-sys`, `nyx` | bindgen bindings; `nyx` wraps them safely | yes, inside `nyx` only |
| Nyx scripts | `lantern-scripting` | `ScriptSnapshot` in, `ScriptCommand` out, `i64` ids, `Vector` values | none (`forbid(unsafe_code)`) |
| WGSL → SPIR-V/MSL | `lantern-sdl-gpu` build script | naga at build time; shader bytes embedded | none |
| PNG, glTF | `lantern-renderer-3d` | file bytes in, `TextureData` / `MeshData` out | none |
| glam | everywhere | `Vec3`, `Quat`, `Mat4` values | none |

`lantern-core`, `-platform`, `-engine`, `-renderer-3d`, `-audio`, `-physics`, `-debug` and
`-scripting` all `forbid(unsafe_code)`. See `dependencies.md` for versions and why each crate
was chosen.

## Known omissions

What the prototype does not do, on purpose or for lack of time (ideas for later are in
`future-work.md`):

- Metal (macOS), Wayland and real-GPU rendering are untested; development had no display or
  GPU and verified with Xvfb + lavapipe (Vulkan in software) and image captures.
- Audio output was only exercised with SDL's dummy driver.
- No D3D12 shaders: on Windows SDL GPU uses Vulkan. Windows is expected to build but was not
  built.
- Controller feel numbers are measured by tests, not tuned by hand.
- One level, defined in Rust (`level.rs`); there is no level file format (the brief rules out
  general scene serialisation).
- No shadows besides the blob shadow, no skinned animation, no transparency sorting.
- Nyx hook run-time errors report the error but no line number (D-017).
