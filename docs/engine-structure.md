# Engine structure

How `engine/src/` is organised, and why. This is the map to read before the code. The renderer has its own
walkthrough in `renderer.md`, and the application and game loop in `application-and-game-loop.md`.

## The rules

1. **One folder per domain.** A folder holds one part of the engine (rendering, platform, physics, audio...), and its
   namespace is `lantern:::<folder>`. Folder and namespace names are lowercase; types are PascalCase.
2. **Domain code never names a library.** `renderer/`, `platform/`, `engine/` and the rest describe *what* the engine
   does in its own types. They don't import SDL.
3. **Each library gets a facade folder.** Everything that talks to SDL lives in `sdl-facade/`, split by the domain it
   serves (`window/`, `platform/`, `audio/`, `renderer/`). A facade implements an interface the domain declares:
   `SdlPlatform` is a `lantern:::platform:::PlatformBackend`, `GpuRenderer` is a `lantern:::renderer:::RenderBackend`.
   Swapping SDL for another library means writing a new facade folder, not touching the domains.
4. **Interfaces live with the domain that needs them**, beside the vocabulary types they use: `RenderBackend` sits in
   `renderer/` next to `RenderWorld`, `PlatformBackend` in `platform/` next to `PlatformEvent`, `Game` in
   `engine/loop/` next to the game loop.
5. **One composition root.** `application/Application.nox` creates the SDL context, the window, the platform,
   the renderer and the audio output, runs the game loop and tears everything down in the right order. Nothing else
   creates them. There's no dependency-injection container: the application passes each object to whatever needs it.

## Layout

```text
engine/src/
  main.nox                     entry point: `lantern [game-directory]`
  application/                 the composition root: Application (create, run, shut down), ApplicationConfig, and
                               Programs.nox with the two programs, a Nyx game and the stub game
  core/                        vocabulary every domain shares: maths, handles and pools, the fixed-step clock,
                               input actions and bindings, colours, logging, JSON
  platform/                    the platform vocabulary: WindowConfig, WindowMetrics, PlatformEvent, and the
                               PlatformBackend and AudioSink interfaces
  renderer/                    the backend-neutral renderer: the RenderBackend interface, RenderWorld (one frame's
                               draw list), Camera, Frustum, meshes, textures, primitives, glTF and PNG loading,
                               the bitmap font, and HeadlessRenderer for tests
  sdl-facade/                  everything SDL
    core/                      SdlContext: SDL's start and shutdown
    window/                    SdlWindow
    platform/                  SdlPlatform and SdlEvents: events, the clock, gamepad, mouse capture
    audio/                     SdlAudioOutput, an AudioSink
    renderer/                  GpuRenderer, the RenderBackend over SDL GPU, and its device, pipeline, shader,
                               resource, uniform and draw-list files
  engine/
    loop/                      the Game interface and the GameLoop (events, fixed steps, frame update, render)
    input/                     InputRouter: platform events to each step's action snapshot
    entities/                  World: entities, transforms, hierarchy, scenes, mesh renderers, ExtractDraws
    audio/                     AudioSystem: named sounds and the music track on an AudioSink
  audio/                       WAV decoding and the software mixer
  physics/                     collision world and the character controller
  debug/                       the debug HUD and debug-line shapes
  world/                       the .world.json format
  nyx/                         the Nyx runtime wrapper
  chaosui/                     the game's Chaos and Cosmos UI
  host/                        the game host: EngineState, the engine API for Nyx, GameHost (a Game)
  editor/                      the world editor's logic
  editorbackend/               the editor's off-screen viewport and session
  stub/                        the stub game `lantern` runs with no game directory
```

## Who depends on whom

```text
core  <-  platform  <-  renderer  <-  engine  <-  host  <-  application  <-  main
                 ^            ^                                  |
                 +---- sdl-facade  <-----------------------------+   (the editor backend names it too)
```

Arrows point at what a folder uses. `sdl-facade` depends on the domain interfaces; only the composition root
(`application/`) and the editor backend, which owns an off-screen GPU renderer, name a facade type. A test that
doesn't need a GPU never builds `sdl-facade/renderer/`: `HeadlessRenderer` stands in for it.

## What moved

| Before | After |
| --- | --- |
| `runner/RunGame.nox`, `main.nox`'s window and loop setup | `application/Application.nox`, `ApplicationConfig.nox` and `Programs.nox` |
| `main.nox`'s asset report | `stub/AssetReport.nox` |
| `engine/FrameLoop.nox` (`FrameLoop`, `Game`) | `engine/loop/GameLoop.nox` (`GameLoop`) and `engine/loop/Game.nox` |
| `engine/InputRouter.nox`, `World.nox`, `Audio.nox` | `engine/input/`, `engine/entities/`, `engine/audio/` |
| `host/RenderBackend.nox` (`lantern:::host`) | `renderer/RenderBackend.nox` and `renderer/HeadlessRenderer.nox` (`lantern:::renderer`) |
| `runner/GpuRenderBackend.nox` (an adapter) | gone: `GpuRenderer` implements `RenderBackend` itself |
| `gpu/` (`lantern:::gpu`) | `sdl-facade/renderer/` (still `lantern:::gpu`), with `GpuRenderer.nox` split by job |
| `gpu/Frustum.nox` | `renderer/Frustum.nox` (`lantern:::renderer`): culling isn't SDL's |
| `sdl/` | `sdl-facade/core/`, `window/`, `platform/`, `audio/` |

Behaviour doesn't change: the same tests pass, and the game, the stub and the editor run as before.

## Why the application isn't inside engine/

Each test builds only the folders it names in its `nox.json`, and many build `engine/` without a GPU, the game host
or SDL. The application needs all of those, so it sits above them in its own folder, and `engine/` stays buildable on
its own.

## What wasn't copied

- **No dependency-injection container or managers.** The application creates each object and passes it on, so a
  constructor shows every dependency. The folder pattern carries over without the container.
- **File names stay PascalCase** (`GameLoop.nox`), as the rest of Lantern's Nox is. Folders follow the pattern above.
- **Namespaces kept their names where only the folder moved** (`lantern:::sdl`, `lantern:::gpu`, `lantern:::engine`),
  so most of the change is a move. `RenderBackend`, `RenderReport`, `HeadlessRenderer` and `Frustum` changed namespace,
  to `lantern:::renderer`, because they moved domain.
