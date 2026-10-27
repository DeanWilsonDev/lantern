# Lantern architecture

Lantern is a small, readable 3D engine prototype written to answer one question: *what must a
small 3D platformer engine actually do each frame?* It follows Umbra's domain split (see
`umbra-audit.md`) but replaces Umbra's interfaces, injector and managers with concrete Rust types
owned by one composition root. Decisions and their reasons are in `decisions.md`; this document
describes the result.

**All game code is Nyx and all engine code is Rust** (D-023). The game lives in
`games/platformer/`; the engine in `crates/`. They meet in one place, `crates/game-host`, which
registers the engine API for Nyx (`docs/game-api.md`) and calls the game's hooks.

Start here, then read `crates/game-host/src/lib.rs` (`run`: construction and shutdown),
`crates/game-host/src/host.rs` (`impl Game for GameHost`: the engine's work around each hook)
and `games/platformer/main.nyx` (the game). Every other file is reached from those three.

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
  debug/         lantern-debug       DebugHud panel and debug_draw helpers (Umbra src/debug)
  engine/        lantern-engine      FrameLoop and the Game trait, InputRouter, World (entities,
                                     transforms, hierarchy, scenes), extract_draws, AudioSystem
                                     (Umbra src/engine)
  nyx-sys/       nyx-sys             raw Nyx C ABI bindings (bindgen; builds Nyx with CMake)
  nyx/           nyx                 safe Nyx wrapper
  world-format/  lantern-world-format the world file shared with the editor: entities, shapes,
                                     colliders, properties; build_pieces (new)
  game-host/     lantern-game-host   the seam: EngineState, GameHost (hooks), the engine API for
                                     Nyx (api/), RenderBackend + HeadlessRenderer, run() (new)
apps/
  lantern/                           the runner: `lantern [game-directory]` (composition root)
games/
  platformer/                        the demo game, all Nyx (Umbra src/snake-game)
    game.json                        title, entry file, Application class, assets directory
    main.nyx                         Platformer :: class : Application, the hooks
    settings/                        input map, player tuning
    game-assets/  game-entities/     assets; Player, Gem, MovingPlatform
    game-scenes/  game-systems/      LevelScene; camera, gem touches, autopilot
    game-rules/   game-events/       GameRules; GameEvent and its queue
    game-state/   ui/                GameState; main menu, pause menu, HUD, collision view
assets/                              textures, model, sounds, music (tools/asset-gen), worlds/
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
     └── world-format (core, renderer-3d, physics)

game-host depends on all of them and on nyx ──► nyx-sys; it is the only place they meet.
apps/lantern depends on game-host. games/*/ are Nyx and depend only on the API game-host
registers.
```

(Arrows point at the dependency.) Only `sdl-facade` and `sdl-gpu` see SDL types, and only
`nyx`/`nyx-sys` see the Nyx C ABI. Engine, physics, audio and the game host never touch raw
pointers, and the game can't: it holds `i64` handles and host objects.

## Umbra → Lantern mapping

| Umbra | Lantern | Notes |
|---|---|---|
| `Core::IApplication` + `Engine::Application` | `lantern_engine::frame_loop::{FrameLoop, Game}`, implemented by `GameHost`, which calls the Nyx `Application` class's hooks | The loop calls `fixed_update`, `frame_update`, `render`; the host turns them into `OnFixedUpdate`, `OnUpdate`, `OnRender`. |
| `SnakeApplication` | `games/platformer/main.nyx` (`Platformer :: class : Application`) | The game's own composition: input, assets, level, states. |
| `DependencyInjector` registrations | plain construction in `lantern_game_host::run` | Construction order is dependency order; see "Ownership and shutdown". |
| `IWindow`, `IInputBackend` + Raylib facades | `lantern_platform::PlatformBackend`, implemented by `lantern_sdl_facade::SdlPlatform` | Tests drive the loop with a scripted fake platform. |
| `SystemManager` / `GameSystemManager` | the fixed call order in `Platformer.Simulate` (`main.nyx`) and `games/platformer/game-systems/` | Order is code you can read, not registration order. |
| `SceneManager` + `Scene` | `games/platformer/game-scenes/level-scene.nyx` over `World:::Load` / `World:::Clear` | Everything the game spawns carries the game's `SceneId`, so a clear or reload removes exactly those. There is one level, so no scene stack (D-016). |
| `EntityManager` + component pipelines | `lantern_engine::world::World` with generational `EntityId` and `ComponentStore<T>` | Iteration is in slot order, so it is deterministic. |
| `IGameState` + `StateMachine` | `GameState` enum (`MainMenu`, `Playing`, `Paused`) in `game-state/game-state.nyx`, matched with `when` | Three states with no shared behaviour do not need a framework (D-016). |
| `snake-game/game-events` | `GameEvent` enum and `GameEvents` queue (`game-events/game-event.nyx`) | Events from a step, handled by `GameRules` in order. |
| `ActionSet`, `KeyMap`, `InputDispatcher`, `ActionRouter` | `lantern_core::input::{ActionSet, Bindings, InputSnapshot}` + `lantern_engine::input::InputRouter` | Named actions with latched edges, read from a snapshot instead of event-bus callbacks. |
| `IRenderComponentManager` | `lantern_engine::extract::extract_draws` | Fills a `RenderWorld`; the renderer never sees entities. |
| `IRenderer` + Raylib renderer facade | `lantern_sdl_gpu::GpuRenderer` | One concrete backend; a trait would be added with a second one. |
| `renderer-2d` | `lantern-renderer-3d` | Three pipelines: lit, lines, overlay. |
| `physics/collision` | `lantern-physics` | Capsule casts against triangles and a kinematic controller; no rigid bodies. |
| `debug` + `core/debug` | `lantern_core::debug::DebugValues` + `lantern-debug` | Same path-keyed tree (`"Player/Velocity"`). |
| `core/user-interface` + `snake-game/ui` | `lantern_renderer_3d::ui::Ui` (engine, `Ui:::` in Nyx) + `games/platformer/ui/` | Immediate-mode labels and buttons with focus navigation. |
| (none) | `lantern-audio` + `lantern_engine::audio::AudioSystem` | Pure-Rust mixer; the SDL audio stream lives in the facade. |
| (none) | `lantern-game-host` | The seam between Rust engine and Nyx game. |
| `snake-game` | `games/platformer` | The whole game, in Nyx. |

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

This is `FrameLoop::run_frame` (`crates/engine/src/frame_loop.rs`) calling into `GameHost`
(`crates/game-host/src/host.rs`), which calls the Nyx hooks in `games/platformer/main.nyx`.
Indentation is call depth; `[Nyx]` marks game code.

```text
FrameLoop::run_frame
├─ 0. host.take_input_configuration()        the game's actions, if Input::: changed them
├─ 1. events
│   ├─ platform.pump_events()                SDL_PollEvent → PlatformEvent
│   └─ input_router.handle_event(event)      device state: keys, buttons, axes, mouse delta
├─ 2. time
│   └─ clock.advance(frame_seconds)          → step_count N, alpha, dropped backlog
├─ 3. N × host.fixed_update(step)
│   ├─ world.store_previous_transforms()     interpolation start point
│   ├─ collision.begin_step()                kinematic bodies remember last transform
│   ├─ [Nyx] OnFixedUpdate(dt)               only while Playing:
│   │   ├─ MovingPlatform.Move(time)           World:::SetPosition moves entity and body
│   │   ├─ Player.Move(stick, ...)             CharacterController.Step: ride platform,
│   │   │                                        accelerate, gravity, jump, move-and-slide,
│   │   │                                        step up, ground snap; then shadow raycast
│   │   ├─ GemSystem.DetectTouches()           capsule axis vs gem centres
│   │   ├─ events: jumped, landed, gem, fell, reset → GameRules.Handle, in order
│   │   └─ GemSystem.Animate(time)             spin and bob
│   └─ world.update_world_transforms()       hierarchy, parent before child
├─ 4. host.frame_update(frame)
│   ├─ engine keys: F1 HUD, F5 reload
│   ├─ [Nyx] OnUpdate(frameSeconds)          F2, pause, camera look, menu orbit, HUD values
│   ├─ apply the game's requests             mouse capture, quit
│   ├─ engine HUD values                     frame, script ms, renderer, audio, errors
│   └─ audio.update()                        mix enough to keep ~60 ms queued in SDL
└─ 5. host.render(frame)                     skipped while minimised
    ├─ render_world.clear()
    ├─ [Nyx] OnRender(alpha, frameSeconds)
    │   ├─ ThirdPersonCamera.Follow()          interpolated player, sphere-cast so it never
    │   │                                        clips; Camera:::LookAt
    │   ├─ CollisionView.Draw() (F2)           Debug::: lines
    │   └─ menus (Ui:::) or HUD text (Overlay:::)
    ├─ end an unfinished Ui column
    ├─ extract_draws(world, alpha)           visible MeshRenderers → DrawItems
    ├─ script error banner, debug HUD (F1)
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
2. `InputRouter` marks the `jump` action (defined by `settings/input-settings.nyx`) pressed and
   latches the edge until a fixed step takes it.
3. `GameHost::fixed_update` copies the step's snapshot into the engine state and calls
   `OnFixedUpdate`. `Platformer.Simulate` reads `Input:::IsPressed("jump")` and passes it to
   `Player.Move`, which calls `controller.Step(...)`: the host runs
   `CharacterController::step` against the collision world. The controller buffers the press
   for 0.12 s, so a press just before landing still jumps.
4. `Step` returns a `ControllerStep` whose `Jumped()` is true; the game pushes
   `GameEvent:::PlayerJumped`, and `GameRules.Handle` calls `Audio:::Play("jump", 0.8f)`, which
   starts a voice in the engine's `AudioSystem`.
5. `Player.Move` set the player entity's position (`World:::SetPosition`); `extract_draws`
   interpolates it with `alpha` and the GPU backend draws it.

## Ownership and shutdown

Construction in `lantern_game_host::run`, in dependency order:

1. `GameManifest::load` — `game.json`.
2. `SdlContext::init()` and `SdlWindow::create` — SDL video, events, gamepad; the window.
3. `GpuRenderer::new(&window)` — device, claims the window, pipelines, depth texture.
4. `AudioSystem::new(SdlAudioOutput::open(..))` — no device is not an error.
5. `EngineState::new(renderer, audio, assets)` — world, collision, UI, HUD, the font atlas.
6. `GameHost::new(state, script_source)` — creates the Nyx runtime, registers the engine API,
   mounts the entry file and calls `OnStart`, which loads the game's assets and its world.
7. `SdlPlatform::new(window)` and `FrameLoop::new(InputRouter)`; `frame_loop.run(..)`.

Shutdown is the reverse and is mostly done by `Drop`:

- `host.shutdown()` calls `OnExit`, drops the script session (the Nyx runtime) and releases
  everything the game made: entities, world meshes, its meshes and textures, music.
- `GameHost`'s fields drop the script session before the engine state, because the runtime's
  host closures share that state. The engine state then drops the `GpuRenderer` (waits for the
  GPU, releases every mesh and texture, releases the window claim, destroys the device) and the
  `AudioSystem` (closes the SDL audio stream).
- SDL objects share an `Rc` to the SDL init guard, and the GPU renderer holds a clone of the
  window handle. So the window outlives the renderer even though the platform drops first, and
  `SDL_Quit` runs only after the last SDL object is gone.

Resource rules:

- GPU meshes and textures live in the renderer's `Pool`s and are addressed by generational
  `MeshHandle` / `TextureHandle`. A stale handle is skipped, never dereferenced. SDL GPU defers
  the real destruction of a released buffer or texture until command buffers using it finish,
  so removing a mesh mid-game is safe.
- The engine state records every mesh, texture and entity the game asks for
  (`script_meshes`, `script_textures`, `world_meshes`, the game's `SceneId`), so a reload
  releases exactly those (`EngineState::reset_for_new_session`, D-028).
- Scripts hold only `i64` handles and immutable host objects, so nothing in Nyx can dangle when
  an entity is despawned or the world is cleared.

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

- The level is triangles: every world-file piece with a collider adds the same triangles it
  draws to the `CollisionWorld`, so what you see is what you collide with
  (`lantern_world_format::build_pieces`, loaded by `World:::Load`).
- Static triangles live in a uniform 4 m grid. Moving platforms are kinematic bodies with local
  triangles; `World:::SetPosition` on the entity moves the body too.
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
display density. `Ui` is immediate mode: each frame the game's menus (`games/platformer/ui/`)
call `Ui:::Title`, `Label`, `Button` and `Spacing` between `Ui:::Begin` and `Ui:::End`; keyboard or gamepad up/down moves focus and confirm
activates. Everything becomes overlay quads in the `RenderWorld`. The debug HUD is the same
text path, drawing `DebugValues` grouped by their first path segment.

## The game seam

`crates/game-host` is the only crate that knows about both the engine and the game.

1. `ScriptSession::load` creates a Nyx runtime, registers the engine API (`api/`: `World`,
   `Physics`, `CharacterController`, `Input`, `Ui`, `Overlay`, `Audio`, `Camera`, `Debug`,
   `Vec3`, ...), adds the game directory as the import search path and mounts the entry file.
   Analysis errors come back rendered with file, line and column, and nothing runs.
2. `GameHost` implements the frame loop's `Game` trait. Around each hook it does the engine's
   part (remember transforms, begin the collision step, recompute world transforms, extract
   draws, draw the HUD, render), so the game only says what happens.
3. Host functions act on `EngineState` directly, through an `Rc<RefCell<..>>` each closure
   shares (D-024). A call borrows it only for its own length and never calls into Nyx.

`docs/game-api.md` lists every call and the hooks. A failing hook is logged once, counted and
shown on screen (and in full in the F1 HUD); the game keeps running, and F5 reloads it (D-028).
A Nyx feature the game needs and can't have yet is a card on the Nyx kanban's Game Engine epic
(D-030), with a host call as the stopgap.

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
9. **Game seam**: a game is a class with five hooks and a flat API of engine calls taking
   handles and values. It works the same whether the game is Nyx or the new language.
10. **Audio**: a mixer that fills a queue each frame is enough at this scale; no callback thread.

Things that are Rust-specific and need an explicit replacement:

- **`Drop`**: GPU resources, SDL objects and the Nyx runtime free themselves. Without
  destructors, call `destroy` explicitly in the order in "Ownership and shutdown".
- **`Rc` guards**: `SdlInitGuard` and the shared window handle encode "SDL_Quit after the last
  SDL object". Replace with explicit shutdown order.
- **Enums with data**: `PlatformEvent` and `GameHook` are tagged unions; Nyx enums with
  payloads map directly (the game's `GameEvent` already is one).
- **Traits**: `PlatformBackend`, `AudioSink`, `Game` and `RenderBackend` are the only traits.
  Each can be an interface or a struct of function pointers.
- **`Result`**: errors are values everywhere; in Nyx, `throw` an error-enum variant.

## Dependency boundary inventory

Where Lantern meets code it did not write, and what crosses each boundary:

| Boundary | Crate | What crosses | `unsafe` |
|---|---|---|---|
| SDL3 (window, events, gamepad, audio) | `lantern-sdl-facade` | raw `SDL_*` calls; out: `PlatformEvent`, `WindowMetrics`, `AudioSink` | yes, each call has a `SAFETY` note |
| SDL3 GPU | `lantern-sdl-gpu` | raw `SDL_*GPU*` calls; in: `RenderWorld`, `MeshData`, `TextureData`; out: handles, `FrameReport` | yes, each call has a `SAFETY` note |
| Nyx C ABI | `nyx-sys`, `nyx` | bindgen bindings; `nyx` wraps them safely | yes, inside `nyx` only |
| Nyx game | `lantern-game-host` | host calls (`docs/game-api.md`), `i64` handles, `Vec3`/`Vec2` host objects, five hooks | none (`forbid(unsafe_code)`) |
| WGSL → SPIR-V/MSL | `lantern-sdl-gpu` build script | naga at build time; shader bytes embedded | none |
| PNG, glTF | `lantern-renderer-3d` | file bytes in, `TextureData` / `MeshData` out | none |
| glam | everywhere | `Vec3`, `Quat`, `Mat4` values | none |

`lantern-core`, `-platform`, `-engine`, `-renderer-3d`, `-audio`, `-physics`, `-debug` and
`-game-host` all `forbid(unsafe_code)`. See `dependencies.md` for versions and why each crate
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
- One level (`assets/worlds/course.world.json`); the game loads it by path.
- No shadows besides the blob shadow, no skinned animation, no transparency sorting.
- Nyx hook run-time errors report the error but no line number (D-029, GAME-F01-T05).
- F5 restarts the game rather than keeping its state (D-028).
