# Lantern's engine in Nox

This directory is the Nox rewrite of Lantern's Rust engine (`crates/`), on the `nox-proto` branch. The platformer in
`games/platformer/` is meant to run on it unchanged. The Rust engine stays on `proto` as the reference, and the work is
tracked on the board's `engine-on-nox` epic (`docs/kanban/engine-on-nox`), one feature per milestone.

So far it has the scaffold (milestone L0): a Nox project, its C dependencies through Mist, the SDL3 and Nyx bindings,
the shaders in GLSL and an entry point that links SDL3, cgltf and stb_image and reads one asset with each. It also has
the core module (milestone L1), described under "Layout".

## What you need

- `nox` on `PATH`, from [nox-systems-language](https://github.com/fearlessgamedev/nox-systems-language) on `proto`:
  `cmake -B build -G Ninja && cmake --build build && cmake --install build --component nox --prefix ~/.local`.
- `mist` on `PATH`, from [mist-package-manager](https://github.com/fearlessgamedev/mist-package-manager):
  `cmake -B build -G Ninja && cmake --build build && cmake --install build --component mist --prefix ~/.local`.
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
./tools/install-dependencies.sh
LIBRARY_PATH=generated/mist:/opt/homebrew/lib nox run
```

On Arch, leave `/opt/homebrew/lib` out of `LIBRARY_PATH`. `nox run` builds `build/lantern` and runs it, and it prints
the SDL version, the size of `assets/textures/ground.png` and the number of meshes in `assets/models/gem.glb`, read
through stb_image and cgltf. Run it from this directory, since the asset paths are relative to it.

- `bind-sdl.sh` runs `nox bind-c` over SDL3's headers and writes `generated/sdl.nox`. It looks for `SDL3/SDL.h` under
  `/opt/homebrew/include`, `/usr/local/include` and `/usr/include`; pass another include directory as its argument.
- `bind-nyx.sh` does the same for `include/nyx/nyx.h` and writes `generated/nyx.nox`, which links `libnyx-capi`.
  The game host (milestone L6) is the first code that calls it.
- `install-dependencies.sh` runs `mist install`, which fetches cgltf and stb_image at the commits in `mist.lock` and
  builds each into a static library with its Nox binding, then links those into `generated/mist/`. It's a stand-in
  until `mist resolve` tells `nox build` where Mist's builds are.

Each binding records the host's struct layouts, so `generated/` is written on each machine and isn't committed.

## Layout

- `src/core/` is the Rust `core` crate: `Conventions.nox` (axes, yaw, perspective, `MoveTowards`, `WrapAngle`, `Aabb`)
  over `VectorMath.nox`, `Handle.nox` (`Handle<T>` and `Pool<T>`), `Time.nox` (`FixedStepClock`, `FrameStats`),
  `Input.nox` (keys, buttons, actions, bindings, `InputSnapshot`, the radial dead zone), `Color.nox`, `Debug.nox`
  (`DebugValues`), `Logging.nox` (the stderr logger, filtered by `LANTERN_LOG`), `Text.nox` (padding and fixed
  decimals) and `LibM.nox` (the libm functions they call).
- `src/platform/Platform.nox` holds the platform vocabulary: `WindowConfig`, `WindowMetrics`, `PlatformEvent`, and the
  `PlatformBackend` and `AudioSink` interfaces a backend implements.
- `tests/<module>/<test>/` is one test program each: a `main.nox`, a `nox.json` naming the engine sources it needs,
  and an `expected.txt` with the exit code and output it must produce. They're the Rust tests, one program per test.

## Testing

```sh
./tools/run-tests.sh
```

runs `nox test` over every suite under `tests/`, and passes its arguments through (`--backend c`, for one). It needs a
`nox` that builds a test program from the `nox.json` beside it, which nox-systems-language's `proto` has from #4 on.

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
