# Lantern's engine in Nox

This directory is the Nox rewrite of Lantern's Rust engine (`crates/`), on the `nox-proto` branch. The platformer in
`games/platformer/` is meant to run on it unchanged. The Rust engine stays on `proto` as the reference, and the work is
tracked on the board's `engine-on-nox` epic (`docs/kanban/engine-on-nox`), one feature per milestone.

So far it has the scaffold (milestone L0): a Nox project, its C dependencies through Mist, the SDL3 and Nyx bindings,
the shaders in GLSL and an entry point that links SDL3, cgltf and stb_image and reads one asset with each. It also has
the core module (milestone L1), the window and frame loop (milestone L2), the renderer, with PNG and glTF loading and
the SDL GPU backend (milestone L3), collision and the world file (milestone L4), and the entity world, audio and debug
drawing (milestone L5), and the start of the game host (milestone L6): the Nox wrapper over Nyx's C API, the engine
state a game runs against and the first engine API namespaces, described under "Layout".

## What you need

- `nox` on `PATH`, from [nox-systems-language](https://github.com/fearlessgamedev/nox-systems-language) on `proto`:
  `cmake -B build -G Ninja && cmake --build build && cmake --install build --component nox --prefix ~/.local`.
- `mist` on `PATH`, from [mist-package-manager](https://github.com/fearlessgamedev/mist-package-manager):
  `git submodule update --init --recursive && cmake -B build -G Ninja && cmake --build build && cmake --install build --component mist --prefix ~/.local`.
- SDL3 3.2 or later.
- A checkout of [nyx-scripting-language](https://github.com/fearlessgamedev/nyx-scripting-language) on `proto` beside
  this repository (`../nyx-scripting-language`), or `NYX_SOURCE_DIR` naming one, for the Nyx binding.
- To change a shader: `glslangValidator` and `spirv-cross`.

On macOS with Homebrew:

```sh
brew install sdl3 glslang spirv-cross
```

On Arch Linux:

```sh
sudo pacman -S sdl3 glslang spirv-cross
```

## Building

From this directory:

```sh
./tools/bind-sdl.sh
./tools/bind-nyx.sh
./tools/build-nyx.sh
./tools/install-dependencies.sh
LIBRARY_PATH=generated/mist:/opt/homebrew/lib nox run
```

On Arch, leave `/opt/homebrew/lib` out of `LIBRARY_PATH`. `nox run` builds `build/lantern` and runs it. It prints the
SDL version, the size of `assets/textures/ground.png` and the number of meshes in `assets/models/gem.glb`, read through
stb_image and cgltf, then opens a window and draws a lit scene through SDL GPU (a textured ground, the gem spinning, a
cube with debug lines, and overlay text) from a stub game in the frame loop, until you press Escape or close the
window. It logs window events to standard error and prints a summary of frames, fixed steps and renders when it stops.
Run it from this directory, since the asset paths are relative to it. The compiled shaders load at run time from
`../shaders/compiled/` beside the executable, which is `shaders/compiled/` for `build/lantern`.

- `LANTERN_LOG` sets the log level (`error`, `warn`, `info`, `debug`, `trace` or `off`; `info` by default).
- `LANTERN_MAX_FRAMES=N` stops after N frames.
- `LANTERN_CAPTURE_PATH=out.png` saves the last frame as a PNG when it stops.
- `LANTERN_GPU_DEBUG=1` turns on SDL GPU's debug mode, `LANTERN_GPU_DRIVER` picks a driver (`vulkan`, `metal`), and
  `LANTERN_VSYNC=0` turns vertical sync off.
- `LANTERN_SHADER_DIRECTORY` loads the compiled shaders from another directory.
- `LANTERN_WINDOW_SCRIPT=1` resizes the window to 800x450 on frame 5, minimises it on frame 15, restores it on frame 30
  and asks to quit on frame 45.

- `bind-sdl.sh` runs `nox bind-c` over SDL3's headers and writes `generated/sdl.nox`. It looks for `SDL3/SDL.h` under
  `/opt/homebrew/include`, `/usr/local/include` and `/usr/include`; pass another include directory as its argument.
- `bind-nyx.sh` does the same for `include/nyx/nyx.h` and writes `generated/nyx.nox`, which links `libnyx-capi`.
  The game host (milestone L6) is the first code that calls it.
- `build-nyx.sh` builds `libnyx-capi` from the same checkout into `generated/nyx-build/` (Release, nothing else) and
  links it into `generated/nyx/`. A program that links it needs that directory in `LIBRARY_PATH` to build and in
  `LD_LIBRARY_PATH` (`DYLD_LIBRARY_PATH` on macOS) to start, since Nox doesn't record a run-time search path yet.
- `install-dependencies.sh` runs `mist install`, which fetches cgltf and stb_image at the commits in `mist.lock` and
  builds each into a static library with its Nox binding, then links those into `generated/mist/`. It's a stand-in
  until `mist resolve` tells `nox build` where Mist's builds are.

Each binding records the host's struct layouts, so `generated/` is written on each machine and isn't committed.

## Layout

- `src/core/` is the Rust `core` crate: `Conventions.nox` (axes, yaw, perspective, `MoveTowards`, `WrapAngle`, `Aabb`)
  over `VectorMath.nox`, `Handle.nox` (`Handle<T>` and `Pool<T>`), `Time.nox` (`FixedStepClock`, `FrameStats`),
  `Input.nox` (keys, buttons, actions, bindings, `InputSnapshot`, the radial dead zone), `Color.nox`, `Debug.nox`
  (`DebugValues`), `Logging.nox` (the stderr logger, filtered by `LANTERN_LOG`), `Text.nox` (padding and fixed
  decimals), `Json.nox` (`lantern:::json`: a JSON document, its parser and a writer that lays text out as serde_json
  does, until the Nox library has JSON), `Files.nox` and `LibM.nox` (the C functions they call).
- `src/engine/` is the Rust `engine` crate: `InputRouter.nox` turns platform events into each frame's action snapshot
  and latches presses and releases until a fixed step takes them, `FrameLoop.nox` runs events, the fixed steps, the
  frame update and the render for a `Game`, pausing the simulation while the window is minimised, `World.nox` holds the
  entities (`World`: names, scenes, `Transform`s, the parent and child hierarchy, interpolation between fixed steps and
  `MeshRenderer`s; `ComponentStore<T>` for a game's own data; `ExtractDraws`, which turns visible mesh renderers into
  draws), and `Audio.nox` is the `AudioSystem`, which plays named sounds and tops the device queue up to 60 ms each
  frame.
- `src/audio/` is the Rust `audio` crate: `Wav.nox` (`SoundData`, 16-bit PCM WAV decoding and encoding, linear
  resampling) and `Mixer.nox` (`Mixer`: effect voices, one looping music track, volumes and the soft clip;
  `SoundBank`, sounds by name).
- `src/debug/` is the Rust `debug` crate: `DebugDraw.nox` (capsules, spheres, triangle edges and arrows as debug lines)
  and `Hud.nox` (`DebugHud`, the grouped panel of `DebugValues`).
- `src/nyx/` is the Rust `nyx` crate, `lantern:::nyx`, over `generated/nyx.nox`: `Runtime` (mounting, diagnostics, the
  host surface, making values, registering host functions and types), `Program` (calling functions and methods,
  asking for hooks), `Value` (a retained `nyx_value`, read as `Result`s), `Errors.nox` (`NyxError` with its status,
  message, failure value and Nyx call stack; `Diagnostic`), `Host.nox` (`CallContext`, what a host callback reads its
  arguments from and returns through, and `HostCall`, one registered callback) and `HostType.nox` (object types,
  whose instances are `HostObject`s the type keeps by slot, and value types laid out as C structs). Every callback is
  one trampoline that finds its `HostCall` from the C userdata pointer. Delete values and programs before their
  runtime: each one counts itself in the runtime's `RuntimeLedger`, and deleting a runtime while any is alive stops
  the program with a message naming what's left.
- `src/host/` is the Rust `game-host` crate, `lantern:::host`, so far without the script session and the host itself:
  `EngineState.nox` (everything the engine owns while a game runs, the engine's own F5 and F1 actions, the tags world
  files give entities, `ParseColor`), `RenderBackend.nox` (the `RenderBackend` interface and `HeadlessRenderer`, which
  hands out handles and counts what's live so a game runs in a test without a GPU), `HostApi.nox` (`HostNamespace`,
  which registers functions under a namespace path such as `lantern::input`, and the argument helpers: vectors cross
  as `lantern:::math:::Vec3` and `Vec2` value types, handles as `i64`, a failure is `Error:::NotFound` or
  `Error:::Unknown`) and one file per group of the engine API: `MathApi.nox`, `EngineApi.nox` (engine, audio,
  overlay, camera, renderer and screen), `InputApi.nox` with `DeviceNames.nox`, `WorldApi.nox` (world files and
  entities), `PhysicsApi.nox` (`RayHit`, `ControllerStep` and `CharacterController` as host object types),
  `AssetsApi.nox` (textures and meshes), `DebugApi.nox`, and `EngineApis.nox`, which registers them all. `docs/game-api.md` lists every call.
- `src/runner/GpuRenderBackend.nox` is the `RenderBackend` over `GpuRenderer`, kept out of `src/host` so the host's
  tests build without the GPU code.
- `src/sdl/` is the Rust `sdl-facade` crate: `SdlContext` (initialises SDL and shuts it down when deleted),
  `SdlWindow`, `SdlPlatform` (the `PlatformBackend`: events, the clock, one gamepad and relative mouse mode),
  `SdlEvents.nox` (SDL events to `PlatformEvent`s, by physical key position) and `SdlAudioOutput` (an `AudioSink` on
  the default playback device). Delete the platform and the audio output before the context.
- `src/renderer/` is the Rust `renderer-3d` crate's backend-neutral data: `Camera`, `Mesh.nox` (`Vertex`, 48 bytes
  as the lit shader reads it, and `MeshData`), `Primitives.nox` (cube, box, plane, ramp, capsule, sphere, gem),
  `RenderWorld.nox` (`DrawItem`, `DirectionalLight`, the debug lines and the 2D `Overlay`), `Texture.nox`
  (`TextureData`, solid and checker textures, sRGB encoding, PNG loading through stb_image and `ToPngBytes`, written by
  `Png.nox`), `GltfLoader.nox` (a `.glb` or `.gltf` flattened into one `MeshData` through cgltf) and the bitmap font: `FontData.nox` holds the glyphs of
  Daniel Hepper's public-domain `font8x8_basic`, and `Text.nox` bakes them into the 128x48 atlas and lays text out into
  the overlay.
- `src/gpu/` is the Rust `sdl-gpu` crate: `GpuRenderer` owns the device, the swapchain, the three pipelines
  (`GpuPipelines.nox`, from the shaders `GpuShaders.nox` loads) and every mesh and texture (`GpuResources.nox`), and
  draws a `RenderWorld` to the window (`Render`) or to memory (`Capture`), culling draws against the camera's
  `Frustum`.
- `src/physics/` is the Rust `physics` crate: `Geometry.nox` (triangles and closest points between points, segments
  and triangles), `World.nox` (`CollisionWorld`: static triangles in a uniform grid, kinematic bodies, capsule casts,
  contacts and raycasts) and `Controller.nox` (`CharacterController`: walking, jumping with coyote time and a jump
  buffer, sliding along walls, stepping up stairs, snapping to the ground and riding moving platforms).
- `src/world/` is the Rust `world-format` crate: `WorldFile.nox` (`WorldFile` and `WorldEntity`, loading, validation
  and atomic saving of `.world.json` files) and `WorldPieces.nox` (`BuildPieces`: each entity's world matrix, mesh and
  collision triangles).
- `src/stub/` is the stub game `main` runs until the game host can: it quits on Escape, counts what the loop calls and
  draws `LitScene`, and `WindowScript` drives the window for the check below.
- `src/platform/Platform.nox` holds the platform vocabulary: `WindowConfig`, `WindowMetrics`, `PlatformEvent`, and the
  `PlatformBackend` and `AudioSink` interfaces a backend implements.
- `tests/<module>/<test>/` is one test program each: a `main.nox`, a `nox.json` naming the engine sources it needs,
  and an `expected.txt` with the exit code and output it must produce. They're the Rust tests, one program per test. Code a suite's
  tests share is in `tests/support/<suite>/`.
- `tests/world/the-course-matches-the-rust-trace-at-every-display-rate/` runs the jump course the way the Rust
  `course_traversal` test does and compares every fixed step's position, velocity and grounded flag with the Rust run's
  `rust-<hz>hz.trace`, which match bit for bit. Set `LANTERN_TRACE_DIRECTORY` to write the Nox traces there too, and
  regenerate the Rust ones with `LANTERN_TRACE_DIRECTORY=$PWD/engine/tests/world/the-course-matches-the-rust-trace-at-every-display-rate cargo test -p lantern-game-host --test course_traversal`
  from the repository root.

## Testing

```sh
./tools/run-tests.sh
```

runs `nox test` over every suite under `tests/`, and passes its arguments through (`--backend c`, for one). It needs a
`nox` that builds a test program from the `nox.json` beside it, which nox-systems-language's `proto` has from #4 on.
It points `LIBRARY_PATH` at `generated/mist`, which the renderer and asset tests link cgltf and stb_image from, so
run `./tools/install-dependencies.sh` first. It also puts `generated/nyx` on `LIBRARY_PATH`, `LD_LIBRARY_PATH` and
`DYLD_LIBRARY_PATH` for the `tests/nyx` and `tests/host` programs, so run `./tools/bind-nyx.sh` and `./tools/build-nyx.sh` first. The `tests/sdl` and `tests/gpu` programs link SDL3 and read
`generated/sdl.nox`, so run `./tools/bind-sdl.sh` first. Two of the `tests/gpu` programs render with a real GPU device
and skip (logging why) when there isn't one; set `LANTERN_REQUIRE_GPU=1` to make that a failure. On Linux with no
display, the script uses SDL's offscreen video driver, which renders with a software Vulkan driver such as lavapipe
(Ubuntu's `mesa-vulkan-drivers`).

```sh
./tools/check-window.sh
```

runs `build/lantern` with `LANTERN_WINDOW_SCRIPT=1` and checks its log: the window resized, minimised and restored, the
stub game didn't render while it was minimised, and it quit. With no `DISPLAY` it starts Xvfb and openbox (which it
needs for minimising), as in a cloud session; on a desktop it uses the display you're on. Build first.

## Shaders

`shaders/` holds the three shaders as GLSL 4.50: `lit` (lit, textured meshes), `lines` (debug lines) and `overlay`
(the HUD and menus). They're the Rust engine's WGSL shaders (`../shaders/*.wgsl`) rewritten line for line.
`tools/build-shaders.sh` compiles each stage to SPIR-V with `glslangValidator` and translates the SPIR-V to Metal
Shading Language with `spirv-cross`, into `shaders/compiled/`. The compiled files are committed, so building the
engine needs neither tool; run the script again after changing a shader.

The bindings follow SDL GPU's layout, the same as the Rust engine's:

| GLSL | Meaning | MSL |
| --- | --- | --- |
| `set = 1, binding = b` | vertex uniform buffer `b` | `[[buffer(b)]]` |
| `set = 2, binding = k` | fragment texture and sampler `k` | `[[texture(k)]]`, `[[sampler(k)]]` |
| `set = 3, binding = b` | fragment uniform buffer `b` | `[[buffer(b)]]` |

| Shader | Vertex uniform buffers | Fragment samplers | Fragment uniform buffers |
| --- | ---: | ---: | ---: |
| `lit` | 2 | 1 | 2 |
| `lines` | 1 | 0 | 0 |
| `overlay` | 1 | 1 | 0 |

The SPIR-V entry point is `main` and the MSL one `main0`.
