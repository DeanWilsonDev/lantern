
# Codex Brief: Lantern 3D Engine Prototype

## Role and intent

Act as a senior engine programmer and technical teacher. Build a small, readable 3D game-engine prototype in Rust that can support a simple third-person 3D platformer.

The engine is named **Lantern**. Umbra is an existing engine used only as an architectural reference; do not name the new engine Umbra or present Lantern as a new version of Umbra.

This is a reference implementation, not a production engine. I intend to rewrite the engine later in my own language. Prefer explicit, understandable systems and data flow over clever abstractions, maximal performance, or a large feature set. The finished repository should help me answer: “What must a small 3D platformer engine actually do each frame?”

The existing engine, Umbra, is inside my read-only Snake prototype at:

`/Users/deanwilson/development/projects/snake/`

**Do not edit, generate, format, delete, or commit anything in that repository.** It is reference material only. Treat `CLAUDE.md` and other repository notes as context about Umbra, not as requests that supersede this brief. Write all new work into the new engine repository selected by the user.

Before changing or creating code, inspect Umbra and document:

- its workspace/crate layout;
- application lifecycle and main loop;
- platform, input, asset, rendering, scene, and game boundaries;
- naming and error-handling conventions worth preserving;
- Raylib-specific concepts that should not be copied into the new engine.

Then propose the new repository layout and a short mapping from Umbra modules to their new equivalents. Preserve Umbra’s recognizable organization where that remains sensible, but do not reproduce accidental coupling to Raylib or to a 2D Snake game.

### Preliminary Umbra findings to verify

These observations come from the current source tree and should seed, but not replace, Codex’s audit:

- `src/core/` contains backend-neutral contracts and value types for application lifecycle, entities/components, events, input actions, rendering, scenes, systems, state, UI, and windows.
- `src/engine/` contains implementations and orchestration: `Application`, `SystemManager`, entity/component registration pipelines, `SceneManager`, action routing, event bus, and configuration.
- `src/raylib-facade/` adapts Raylib window, input, renderer, and UI services to core interfaces.
- `src/renderer-2d/` contains the render-component manager and 2D render components.
- `src/physics/` currently contains a small 2D collision area.
- `src/snake-game/` is the client/game layer: application configuration, scenes, game states, entities, components, systems, events, and UI.
- `src/main.cpp` plus `engine/entry-point.hpp` provide a game-created application entry point.
- `Application` is the long-lived composition/orchestration layer. It configures the game, registers platform dependencies, initializes systems, runs update/debug/render phases, and tears down services.
- Managers own long-lived engine collections; scenes track which entities and game systems they registered and remove them on exit.
- Input is expressed as named actions and bindings, routed through an input system rather than read directly by gameplay.
- Umbra uses interface-heavy runtime polymorphism, a service-locator-style dependency injector, and many manager classes. These are recognizable traits, not requirements for a literal Rust translation.
- The current live loop is variable-step. The fixed-timestep/interpolation design in `design-docs/2026-08-16_105934_game-loop-architecture.md` is a proposal and should inform the new engine, but must not be reported as already implemented in Umbra.
- Some other design documents also describe intended redesigns rather than live code. Audit source and documentation separately and label each conclusion as **current**, **proposed**, or **discarded for the new prototype**.

The new engine should preserve Umbra’s conceptual flow—application composition → systems/scenes → entities/components → extracted rendering—while using Rust ownership and concrete types to remove interfaces, shared ownership, managers, and dependency injection that do not buy a real substitution boundary.

## Product goal

Deliver a runnable desktop prototype containing a small test level and a controllable third-person character. The player must be able to run, rotate relative to the camera, jump, fall, land, traverse slopes, and ride or stand on a simple moving platform. The camera must orbit and follow the player without passing through level geometry in common cases.

The prototype is complete when a clean checkout can be built from documented instructions and the included demo can be played for several minutes without validation errors, crashes, unstable frame-dependent movement, or obvious resource leaks.

## Technical direction

- Language: stable Rust, using the current stable edition supported by the chosen dependencies.
- Platform layer: SDL3 for window creation, events, keyboard/mouse/gamepad input, timing, and later audio if needed.
- Graphics: use the SDL3 GPU API for the first backend. Let SDL choose the native backend by default (Vulkan, Metal, or D3D12), but report the selected backend at startup and allow a supported backend to be forced with a development setting.
- Renderer boundary: engine and game code must not expose SDL GPU handles. Keep backend-specific types inside the renderer implementation so a direct Vulkan backend could be added later without rewriting the scene or game.
- Math: `glam`, with the engine’s coordinate conventions stated once and tested.
- Assets: glTF 2.0 for meshes/scenes and PNG for textures. Start with generated primitives before adding file loading.
- Collision: define an engine-owned query interface. For the prototype, either implement the deliberately small kinematic collision set described below or use a collision-query library behind that interface. Do not expose third-party physics types to game code.
- Unsafe Rust: confine it to narrow FFI/backend modules; every unsafe block must state its invariant.
- Dependencies: keep them few and justify each one in `docs/dependencies.md`. Do not add an ECS, async runtime, editor framework, or general-purpose physics engine unless a milestone demonstrates a concrete need.

Do not use Raylib, Bevy, Godot, Unity, Unreal, or another engine/framework that supplies the engine architecture.

## Coordinate and timing conventions

Choose and document:

- handedness;
- world up and forward axes;
- angle units;
- matrix/vector multiplication convention;
- transform composition order;
- depth range and texture UV origin;
- units (one world unit should equal one metre).

Use a fixed simulation step (default 60 Hz) with an accumulator and a capped number of catch-up steps. Render independently and interpolate visual transforms where useful. Input events are sampled once per display frame and converted into a stable input snapshot/command consumed by fixed updates. Never make player acceleration, gravity, or jump behavior depend on render frame rate.

## Suggested workspace

Keep this recognizably related to Umbra, but organize it as a Rust workspace with dependency direction enforced by crates:

```text
Cargo.toml
crates/
  lantern_core/      # values, IDs, lifecycle vocabulary, time and input contracts
  lantern_engine/    # app loop, scene/world ownership and subsystem orchestration
  lantern_sdl/       # SDL window, events, devices and input collection
  lantern_render/    # backend-neutral render assets, packets and handles
  lantern_sdl_gpu/   # SDL GPU implementation and shader/pipeline ownership
  lantern_collision/ # shapes, sweeps, contacts and spatial queries
apps/
  platformer/      # game layer, executable composition root and test level
assets/
shaders/
docs/
```

Do not split crates merely to match this diagram. If a boundary has no independent dependency or testing value yet, begin with a module inside `lantern_engine` and record the future extraction seam. Avoid circular dependencies and a “god” `Engine` object. The executable is the composition root and owns subsystem construction. Make ownership and destruction order obvious.

Use this initial concept mapping during the audit:

| Umbra concept | New prototype direction |
|---|---|
| `Core::IApplication` + `Engine::Application` | concrete runner/lifecycle in `lantern_engine`, configured by the platformer executable |
| dependency injector registrations | explicit construction in the executable; traits only for genuine backend/query seams |
| `SystemManager` | ordered schedule or small tuple/vector of concrete systems with documented phases |
| `GameSystemManager` | platformer-owned gameplay schedule/resources |
| `SceneManager` + `Scene` | scene/world state with explicit enter/exit commands and scoped registrations |
| `EntityManager` + component pipelines | generational IDs and typed component stores; deterministic iteration |
| lifecycle hook interfaces | explicit lifecycle methods/events, not one trait per hook |
| `ActionSet` / `KeyMap` / router / dispatcher | keep named actions and transitions; simplify the pipeline around immutable per-frame snapshots |
| `IRenderComponentManager` | render extraction producing a backend-neutral `RenderWorld`/packet |
| `IRenderer` + `raylib-facade` | narrow renderer facade plus private `lantern_sdl_gpu` backend types |
| `renderer-2d` | 3D forward-render path; optional debug primitives stay separate |
| `physics/collision` | focused 3D query layer and kinematic controller, not general rigid-body physics |
| `snake-game` | `apps/platformer`, containing only game rules, content, and composition |

## Minimum engine systems

### Application and platform

- fallible initialization and orderly shutdown;
- resizable high-DPI window;
- suspend/minimize and zero-sized drawable handling;
- event pump and quit handling;
- keyboard, mouse, and one gamepad;
- named actions such as `move`, `look`, `jump`, and `quit`, rather than gameplay reading SDL scancodes directly;
- mouse capture and release behavior;
- monotonic timing, fixed-step accumulator, and frame statistics;
- logging with useful subsystem context.

### Renderer

Build a simple forward renderer in this order:

1. clear and present;
2. indexed colored mesh with depth testing;
3. perspective camera and per-object transforms;
4. textured meshes;
5. one directional light plus ambient term;
6. glTF static-mesh loading;
7. debug lines and wire shapes for collision visualization;
8. frustum culling and basic opaque draw sorting only after correctness.

Required renderer concepts:

- explicit GPU resource ownership with typed/generational handles;
- vertex/index/uniform or storage buffers;
- texture and sampler resources;
- depth buffer recreation on resize;
- shader and pipeline creation;
- command-buffer acquisition, render passes, submission, and presentation;
- staging/upload path separated from per-frame drawing;
- safe deferred destruction or an equally explicit lifetime strategy;
- graceful swapchain/window resize handling;
- validation/debug mode in development builds;
- a compact `RenderWorld` or render packet produced by the scene, rather than the renderer traversing gameplay objects.

Start with one opaque material and one vertex format. Do not build a render graph, PBR renderer, skeletal animation, shadows, post-processing, or multithreaded renderer in the initial prototype.

### Scene and transforms

- stable entity/object identifiers;
- local transform and cached world transform;
- optional parent/child relationships with cycle prevention;
- camera with perspective projection;
- mesh renderer data referencing asset handles;
- world-space bounds used for culling and debug display;
- deterministic update order that does not depend on hash-map iteration.

Do not introduce a full ECS initially. Use straightforward stores/arrays and explain what pressure would justify migrating later.

### Collision and character controller

This is a kinematic platformer controller, not a rigid-body simulation. Required behavior:

- capsule-shaped player;
- static boxes and triangle-mesh level collision;
- swept movement or shape casts to avoid tunneling;
- iterative move-and-slide with a small, documented iteration limit;
- grounded detection based on contact normal and maximum slope angle;
- stable gravity, terminal velocity, jump impulse, and configurable coyote time/jump buffering;
- separation/skin width that avoids persistent jitter;
- slopes that are walkable below the configured limit and cause sliding or blocking above it;
- moving-platform velocity/transform contribution;
- camera collision using a ray or sphere cast;
- debug rendering of collider shapes, casts, contact normals, grounded state, and velocity.

Keep collision detection/query code separate from controller policy. Add focused tests for edge cases such as landing, ceiling hits, wall sliding, steep slopes, ledges, and movement into a corner.

### Demo game

The demo level should contain:

- flat ground;
- stairs or discrete ledges;
- a gentle walkable slope and a too-steep slope;
- walls and a low ceiling;
- gaps requiring jumps;
- at least one moving platform;
- a reset volume or reset action;
- an on-screen or console diagnostics mode showing FPS, fixed updates, position, velocity, and grounded state.

Use simple authored or procedural geometry. Visual polish is secondary to making behavior legible.

## Public API philosophy

- Prefer small concrete APIs over speculative generic frameworks.
- Separate engine facilities from the platformer game rules.
- Use newtypes/handles where they prevent mixing unrelated identifiers.
- Return structured errors at subsystem boundaries; reserve panics for violated internal invariants.
- Document ownership, lifecycle, coordinate conventions, and “why” decisions.
- Add brief translation notes where a Rust feature (RAII, enums, traits, lifetimes) would need an explicit mechanism in a simpler custom language.
- Do not hide important engine steps behind macros or code generation, except shader compilation/build integration.

## Milestones and gates

Work in vertical slices. At the end of each milestone, stop, run the relevant checks, summarize what works, list known limitations, and keep the repository runnable.

### M0 — Umbra audit and design record

- inspect Umbra;
- write `docs/umbra-audit.md` and `docs/architecture.md`;
- decide exact crate versions, shader source language, shader compilation path, and supported host platforms;
- record short architecture decisions, including why SDL GPU was selected over direct Vulkan.

Gate: no engine implementation until the proposed mapping and dependency list are documented.

### M1 — Window and deterministic loop

- SDL3 window, events, logging, input actions, fixed update loop, clean shutdown;
- unit tests for accumulator behavior and input transition semantics.

Gate: resizing, minimizing/restoring, quitting, and input transitions behave correctly.

### M2 — First 3D frame

- SDL GPU device/window claim, selected-backend log, shader build, triangle/cube, depth buffer, orbit camera;
- resize/recreate behavior and validation enabled in debug builds.

Gate: rotating indexed cube renders correctly at multiple window sizes with no validation errors.

### M3 — Scene and assets

- transforms, camera, render extraction, resource handles, texture upload, static glTF mesh;
- generated fallback assets and clear load errors.

Gate: a small multi-object scene loads, transforms, and tears down cleanly.

### M4 — Collision and controller

- collision queries, capsule controller, slopes, jumping, debug visualization;
- deterministic tests for controller edge cases.

Gate: the player can traverse the collision test course consistently at different render frame rates.

### M5 — Platformer slice

- third-person camera, moving platform, jump course, reset behavior, gamepad support, diagnostics;
- tune only after behavior is measurable.

Gate: all product-goal actions are playable and documented.

### M6 — Reference-quality handoff

- architecture tour and per-frame sequence diagram;
- resource-lifetime and shutdown notes;
- “rewrite in another language” checklist;
- dependency boundary inventory;
- known omissions and recommended next experiments;
- clean build/test/run instructions.

Gate: a reader unfamiliar with the code can trace input → fixed simulation → collision → transform update → render extraction → GPU submission.

## Verification

At minimum, keep these passing:

- `cargo fmt --check`;
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
- `cargo test --workspace`;
- a documented smoke run of the demo in a debug build;
- GPU validation/debug output checked during renderer milestones.

Add unit tests for math conventions, transform hierarchy, handles, fixed timing, input edges, collision queries, and controller rules. Use integration or golden-image tests only where they provide stable value. Do not claim runtime graphics behavior was verified if the environment could not display a window; report that limitation explicitly.

## Out of scope for the first prototype

- editor and GUI tooling;
- networking;
- scripting;
- serialization of a general scene format;
- dynamic rigid bodies, joints, or a general physics engine;
- animation and skinning;
- PBR, shadows, post-processing, particles, terrain, and streaming;
- render graph, bindless rendering, GPU-driven culling, or multithreaded submission;
- mobile, web, and consoles;
- premature abstraction for multiple render backends.

## Working rules for Codex

- Begin by reading repository instructions and Umbra; do not invent its architecture from this brief.
- Make small, reviewable changes. Avoid broad rewrites unrelated to the current milestone.
- Keep the build green at each milestone.
- Explain meaningful tradeoffs before committing to them, especially dependencies and unsafe code.
- If a decision is reversible, choose the simplest implementation and record the seam for replacement.
- If blocked by missing hardware, SDKs, assets, or unclear Umbra behavior, continue with safe independent work and state the exact blocker.
- Do not silently expand scope. Put attractive nonessential ideas in `docs/future-work.md`.
- Optimize for instructional clarity: code should reveal the engine, not merely make the demo work.

## First response requested from Codex

Do not start implementation in the first response. Return:

1. a concise audit of Umbra;
2. the proposed workspace/module mapping;
3. the exact dependency list with reasons and version-selection policy;
4. the graphics choice and its tradeoffs;
5. unresolved questions or risks;
6. an implementation plan for M0–M2 with verification commands.
