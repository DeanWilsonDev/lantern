# Umbra audit

Umbra is the C++20 / CMake / Raylib 5.5 engine inside the Snake prototype
(`DeanWilsonDev/snake`, branch `game-with-engine-architecture`, 50 commits at the time of
this audit). It is reference material only. Nothing in that repository was changed.

Each conclusion is labelled:

- **current**: what the source on that branch does today;
- **proposed**: what a design document in `design-docs/` describes but the source does not do yet;
- **discarded**: a trait we deliberately do not carry into Lantern.

## 1. Folder layout and domain split (current)

Everything is one CMake executable target, `Snake`. The folders under `src/` are a logical
split, not separate CMake targets, so nothing enforces dependency direction.

| Folder | What lives there |
|---|---|
| `core/` | Interfaces (`I*`) and plain value types: application, colour, components, debug (`DebugNode`, `DebugValue`, `IDebugHUD`), dependency injection, entities, events (`IEventBus`, input and window events), geometry, input (actions, `ActionSet`, `ActionRouter`, `KeyMap`, bindings), life-cycle hooks, logging, math, rendering, scenes, spatial, state, systems, user interface, window. |
| `engine/` | Implementations and orchestration: `Application`, `DependencyInjector`, `EntityManager` with component/activation pipelines and registrars, `EventBus`, `InputSystem` + `InputDispatcher` + keyboard source, `SceneManager` + `Scene`, `StateMachine`, `SystemManager` + `GameSystemManager`, UI managers, config structs, `entry-point.hpp`. |
| `platform/` | `WindowManager` and `Screen`. |
| `raylib-facade/` | Raylib adapters for window, input backend, renderer, UI text and the debug HUD. |
| `renderer-2d/` | `RenderComponent2DManager` and 2D render components. |
| `physics/` | `RectangleCollider2D` and a collider component (axis-aligned overlap only). |
| `debug/` | `DebugHUD` (a path-keyed tree of values, `"Player/Position/x"`) and the debug UI manager. |
| `snake-game/` | The client: `SnakeApplication`, settings, one scene (`GameplayScene`), game states (main menu, gameplay, game over), entities, components, game systems, events and UI. |
| `main.cpp` | Defines `CreateApplication()`; `engine/entry-point.hpp` owns `main()`. |

## 2. Application lifecycle and main loop (current)

`main()` → `CreateApplication()` (game-supplied) → `Application::Run()`:

1. set debug mode, initialise the logger (`firefly`);
2. `Configure(config)`: the game registers named input actions, builds a key map and fills
   window, input and debug config;
3. `RegisterDependencies()`: binds Raylib facades to core interfaces in the injector;
4. `Initialize()`: resolves services, creates engine systems in order (input, game system
   manager, entity manager, scene manager), opens the window; the game then registers scenes
   and calls `SwitchTo("Gameplay")`;
5. loop until the window should close: measure a **variable** `deltaTime`, `OnUpdate(dt)`
   (system manager then scene manager), `OnDebugUpdate()`, `OnRender()` (begin drawing, clear,
   render component manager), `OnDebugRender()` (systems, scene, debug HUD), `EndDrawing()`;
6. `Shutdown()` closes the window, then the injector tears down.

The fixed-timestep accumulator with interpolation in
`design-docs/2026-08-16_105934_game-loop-architecture.md` is **proposed**, not implemented.
Lantern implements that proposal (fixed 60 Hz, 8-step catch-up cap, input sampled once per
display frame, interpolation alpha passed to rendering).

## 3. Boundaries (current)

- **Platform / window**: `Core::Window::IWindow`, implemented by `RaylibWindowFacade`.
  `platform/window` holds screen helpers. Raylib owns the actual window and GL context.
- **Input**: named actions (`ActionSet::RegisterAction("Up")`), digital key bindings built with
  `Bind("Up").Digital({KeyCode::W, KeyCode::Up})`, a `KeyboardInputSource` that polls the
  backend, an `InputDispatcher` that computes pressed/released edges and publishes
  `InputActionPressedEvent` / `InputActionReleasedEvent` on the event bus, and an
  `ActionRouter` that calls bound callbacks and returns RAII `Handle`s that unbind on drop.
  Gameplay never reads key codes directly. Action values are `bool`, `float` or `Vector2D`.
  Gamepad support is a noted "side quest".
- **Assets**: there is no asset system. Snake draws rectangles.
- **Rendering**: `IRenderer` has `BeginDrawing`, `EndDrawing`, `ClearBackground` and
  `DrawRectangle`. Render components register with a `RenderComponent2DManager`, which draws them.
  The renderer is called with gameplay-owned components rather than an extracted packet.
- **Scenes**: `SceneManager` holds a name → factory registry with `Persistent` or `Transient`
  lifetime, an active scene and an overlay stack (`Push` / `Pop`). `Scene` records the entities and
  game systems it added and removes them on exit, so scene scope is explicit.
- **Game**: `snake-game/` derives from `Engine::Application`, configures actions, registers
  scenes, and owns states, entities, systems and UI.
- **Debug HUD**: `DebugHUD::Set("Snake/Head/x", DebugValue::FromNumber(...))` builds a tree;
  the Raylib debug facade visits it and draws `key: value` lines over a translucent panel.
- **UI**: `IUserInterface::DrawTextCentered`. `MainMenuUI` draws a title and "Press 'Enter' to
  start". The main-menu and gameplay *states* exist but their bodies are commented out and the
  application goes straight to the gameplay scene, so the main menu is **not live** today.

## 4. Conventions worth preserving

- kebab-case folder and file names; one concept per file;
- core holds vocabulary, engine holds behaviour, facade holds the backend;
- named actions and bindings rather than raw key reads;
- scene-scoped registration: a scene removes what it added;
- explicit update / debug-update / render / debug-render phases;
- the debug HUD as a tree of named values that any system can publish into;
- `[Subsystem]` prefixes on log lines (`[Application] Initializing`);
- `MAIN QUEST` / `SIDE QUEST` / `1UP` comments that mark known follow-ups. Lantern keeps a
  similar habit through `docs/future-work.md` instead of inline tags.

Error handling is mostly `assert` plus logging, and `SceneManager::ResolveScene` throws on an
unknown scene. Lantern uses `Result` at subsystem boundaries instead (see `architecture.md`).

## 5. Things not to copy

**discarded for the new prototype**

- Raylib's immediate global state (`BeginDrawing`, global window, `DrawText` with an implicit
  default font, implicit GL context). Lantern owns a GPU device and records explicit command
  buffers.
- Interfaces for everything (`IWindow`, `IRenderer`, `IEventBus`, one interface per life-cycle
  hook). Lantern uses concrete types and keeps traits only where a substitution really happens.
- The service-locator `DependencyInjector` and `shared_ptr` everywhere. Lantern's executable
  constructs subsystems explicitly and owns them by value, so destruction order is the reverse of
  field order and can be read from one struct.
- `typeid`-keyed event bus with `std::function` listeners. Lantern passes data explicitly between
  phases and uses small typed command queues (for example scene and script commands).
- Component storage as `unordered_map<type_index, unique_ptr<IComponent>>` per entity, and integer
  entity IDs from a global counter. Lantern uses generational IDs and typed stores.
- Variable `deltaTime` in simulation (replaced by the fixed step, see §2).
- 2D-only types (`Vector2D`, `Rectangle`, `TransformComponent2D`).

## 6. Bugs and smells noticed on the way (current)

These are notes for Dean; they did not change Lantern's design except where stated.

- `Scene::OnExit` loops over `ownedEntities` while `RemoveEntity` erases from that same vector,
  so iterators are invalidated and some entities can be skipped. Lantern's scene exit drains its
  list first and then despawns.
- `InputDispatcher::GetActionValue` returns from inside the loop on the first source, so a
  second source (gamepad) would never be read.
- `DebugHUD` uses `unordered_map`, so line order can change between runs. Lantern's HUD keeps
  insertion order.
- `Application::Initialize` logs addresses of the `shared_ptr` members, not the services.
