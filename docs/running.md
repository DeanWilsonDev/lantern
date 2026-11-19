# Running Lantern

How to install the tools, then build, play, test and edit on macOS, Arch Linux, other Linux and Windows. Everything is
driven by [Mist](https://github.com/fearlessgamelabs/mist-package-manager) from the repository root: the scripts are in
[`mist.json`](../mist.json), and the packages it fetches and builds are listed in its `dependencies`.

## The four commands

| Command | What it does |
| --- | --- |
| `mist install` | Fetches and builds every dependency in `mist.json`: SDL3, SDL3_image, SDL3_ttf, cgltf, stb_image, Penumbra (`penumbra-ui-engine`, branch `nox-proto`), `penumbra-bridge` and `lantern-native`. `lantern-native` is the one library holding Nyx's C API, the Chaos UI bridge and penumbra-bridge's C API (`engine/native/`). Run it once, and again after pulling changes to `mist.json` or `mist.lock`. |
| `mist run game [game-directory]` | Builds the engine in release mode and runs it from `engine/`. With no argument it plays the platformer. A game directory is relative to `engine/`: `mist run game ../games/platformer`. `mist run game --stub` runs the engine's stub scene (a lit block and a gem), which is a quick smoke test. |
| `mist run package [game-folder] [output-folder]` | Builds the release engine and writes a runnable copy of the game to `engine/build/package` (`bin/lantern`, `shaders/`, `game/`, and `play.sh` to start it). The copy has the game's scripts, UI and runtime assets (worlds, textures, models, sounds, music) but none of the project's Calamus files: no Silhouettes, no material Silhouettes, no editor palette, no graph JSON. Their values are already in the world files and in the generated Nyx. |
| `mist run editor [world-file]` | Builds the engine (the editor's Play button runs it), then builds and runs the world editor, `apps/world-editor/nox`. It opens `games/platformer/assets/worlds/course.world.json` unless given another world. |
| `mist run test` | Runs `engine/tools/run-tests.sh`: `nox test` over every suite under `engine/tests/`. Arguments pass through, for example `--backend c`. |
| `mist run build-shaders` | Recompiles `engine/shaders/*` to SPIR-V and Metal Shading Language into `engine/shaders/compiled/`. The compiled files are committed, so you only need this after editing a shader, and it needs `glslangValidator` and `spirv-cross`. |

To work on Nyx, Chaos, Cosmos, Penumbra or penumbra-bridge beside Lantern, point Mist at your checkout and install
again: `mist link nyx-scripting-language ../nyx-scripting-language`, then `mist install`. `mist unlink` goes back to the
locked commit. Links live in `mist.local.json`, which isn't committed. `engine/README.md` has the details.

## What you need on every platform

- `nox` on your `PATH`, built from [nox-systems-language](https://github.com/fearlessgamelabs/nox-systems-language)
  on its `proto` branch:
  `git clone -b proto https://github.com/fearlessgamelabs/nox-systems-language && cd nox-systems-language`, then
  `cmake -B build -G Ninja && cmake --build build && cmake --install build --component nox --prefix ~/.local`
- `mist` on your `PATH`, from [mist-package-manager](https://github.com/fearlessgamelabs/mist-package-manager):
  `git clone https://github.com/fearlessgamelabs/mist-package-manager && cd mist-package-manager`, then
  `git submodule update --init --recursive && cmake -B build -G Ninja && cmake --build build && cmake --install build --component mist --prefix ~/.local`
- CMake 3.30 or later, Ninja, and a C++26 compiler (GCC 14, Clang 19 or later, or a recent AppleClang). Mist uses them
  to build SDL3 and `lantern-native`, which builds Nyx.
- Optional, to change a shader: `glslangValidator` and `spirv-cross`.

Make sure `~/.local/bin` is on your `PATH`.

In a cloud session, `engine/tools/cloud-bootstrap.sh` does the install; see [Testing in a cloud session](cloud-testing.md).

## macOS

```sh
xcode-select --install
brew install cmake ninja llvm glslang spirv-cross
export CC="$(brew --prefix llvm)/bin/clang" CXX="$(brew --prefix llvm)/bin/clang++"
```

Use Homebrew's current LLVM (20 or later). LLVM 19's libc++ has no floating-point `std::from_chars`, which Nyx and
Amanuensis use, so `llvm@19` fails with "call to deleted function 'from_chars'". If you switch compilers, delete the old
build directories first, because CMake keeps the first compiler it found.

SDL GPU uses Metal. The shaders are translated to Metal Shading Language ahead of time, which is why `engine/shaders/compiled/`
holds both `.spv` and `.msl` files. `LANTERN_GPU_DRIVER=metal` forces Metal.

## Arch Linux (Wayland or X11)

```sh
sudo pacman -S --needed base-devel cmake ninja clang glslang spirv-cross \
    wayland wayland-protocols libxkbcommon libdecor \
    libx11 libxext libxrandr libxcursor libxi libxss \
    alsa-lib libpulse pipewire vulkan-icd-loader mesa
# plus the Vulkan driver for your GPU: vulkan-radeon, vulkan-intel or nvidia-utils
```

Arch's GCC is 14 or newer, so the default compiler works. SDL builds only the window and audio backends whose
development headers it finds, which is why the Wayland, X11 and audio packages are in the list. On Wayland SDL picks the
Wayland backend by itself; `SDL_VIDEO_DRIVER=x11` forces XWayland for comparison. Rendering goes through Vulkan.

## Other Linux (Ubuntu and Debian)

```sh
sudo apt install build-essential ninja-build g++-14 glslang-tools spirv-cross \
    libwayland-dev wayland-protocols libxkbcommon-dev libdecor-0-dev \
    libx11-dev libxext-dev libxrandr-dev libxcursor-dev libxi-dev libxss-dev libxtst-dev \
    libasound2-dev libpulse-dev libpipewire-0.3-dev libudev-dev libvulkan1 mesa-vulkan-drivers
export CC=gcc-14 CXX=g++-14
```

Ubuntu 24.04's `cmake` package is 3.28, older than the 3.30 Mist and Nox need. Install a newer one with `pip install --user 'cmake>=3.30'` (put `~/.local/bin` first on `PATH`) or from Kitware's apt repository, and check `cmake --version`.

`mesa-vulkan-drivers` includes lavapipe, a software Vulkan driver, so the engine and its GPU tests also run on a
machine with no GPU. Package names vary by release; if SDL's configure step reports a missing library, install its
development package and run `mist install` again.

## Windows

**Not yet tried.** The pieces that should make it work, and the ones that don't exist yet:

- SDL GPU draws through Vulkan on Windows, so a Vulkan driver is needed. Direct3D 12 would need shaders compiled to DXIL,
  which the build doesn't produce (see `known-gaps.md`).
- Nox's own Windows support was still an open card when this was written, and Mist's scripts are shell scripts (`engine/tools/*.sh`),
  so use a POSIX shell such as Git Bash or WSL until that's checked.
- Nox uses the C backend where it has no native one (`nox build --backend c`), which needs a C compiler on the path.

Treat a Windows run as an experiment, and note what breaks in `known-gaps.md`.

## Playing the platformer

```sh
mist run game ../games/platformer
```

| Input | Action |
| --- | --- |
| W A S D or the arrow keys, left stick | Move |
| Mouse (click the window first to capture it), right stick | Look |
| Space, gamepad South | Jump |
| R, gamepad North | Start the level over: back at the spawn point with every gem and checkpoint as new |
| F2, right stick click | Show or hide the collision view |
| Escape, gamepad Start | Pause |
| Up, Down, Enter, Space | Menu: move, confirm |
| F1 | Show or hide the debug window: pages of frame, audio, renderer, camera, player and other values |
| F3 | Focus the debug window to filter, change page and scroll (see `debug-window.md`) |
| F5 | Reload the game's `.nyx`, `.chaos` and `.cosmos` files from disk |

The main menu offers Play (the jump course), The streets and The streets, later (the streets in Chapter 3's look, with thorns). A script edit needs no rebuild: save and
press F5.

### Switches

These environment variables are read by the engine and the game:

| Variable | Effect |
| --- | --- |
| `LANTERN_LOG` | Log level: `error`, `warn`, `info` (default), `debug`, `trace` or `off`. |
| `LANTERN_LEVEL` | A world file inside the assets directory to start on, such as `worlds/streets.world.json`, used when the menu has not picked a level. A level picked from the menu (such as The streets) takes precedence. The editor's Play button sets it. |
| `LANTERN_ASSETS` | The assets directory, overriding `game.json`'s `assets`. |
| `LANTERN_AUTOPLAY=1` | The platformer plays itself (a scripted pilot), for smoke runs and screenshots. |
| `LANTERN_DEBUG_HUD=1` | Start with the debug window showing. |
| `LANTERN_COLLISION_VIEW=1` | Start with the platformer's collision overlay showing (nearby triangles, the player's capsule, ground normal, velocity and gem trigger spheres). F2 toggles it in game. |
| `LANTERN_SSR` | `0` turns screen-space reflections off, `1` turns them on (at medium cost) whatever the quality level; see `docs/screen-space-reflections.md`. |
| `LANTERN_VSYNC=0` | Turn vertical sync off, to see raw frame time. The world editor honours it too: it starts with vertical sync off, and Settings, Graphics, Frame rate turns it on or off while it runs. |
| `LANTERN_GPU_DRIVER` | Force an SDL GPU driver, such as `vulkan` or `metal`. |
| `LANTERN_GPU_DEBUG=1` | SDL GPU's debug mode (validation layers where the driver has them). |
| `LANTERN_MAX_FRAMES=N` | Stop after N frames, so a run ends on its own. |
| `LANTERN_CAPTURE_PATH=out.png` | Save the last frame as a PNG when the run ends. Combine with `LANTERN_MAX_FRAMES` for a screenshot. |
| `LANTERN_SHADER_DIRECTORY` | Load compiled shaders from another directory (`mist run test` sets it to `engine/shaders/compiled`). |
| `LANTERN_RUNNER` | The path of the game executable the editor's Play button starts. |
| `LANTERN_WINDOW_SCRIPT=1` | With `--stub`, the stub scene resizes, minimises and restores its window, then quits (`engine/tools/check-window.sh`). |
| `LANTERN_REQUIRE_GPU=1` | In tests: a GPU test that can't get a device fails instead of skipping. |

A screenshot of the streets level, headless or not:

```sh
LANTERN_LEVEL=worlds/streets.world.json LANTERN_AUTOPLAY=1 LANTERN_MAX_FRAMES=600 \
  LANTERN_CAPTURE_PATH=streets.png mist run game ../games/platformer
```

## The world editor

```sh
mist run editor
mist run editor games/platformer/assets/worlds/streets.world.json
```

`docs/editor.md` has the controls and how the editor is built, and `apps/world-editor/nox/README.md` the host's files.
The Play button writes a separate snapshot, `.editor-play.world.json`, and starts the game on it. It does not save your world file, so save your edits yourself. Build the game first (`mist run editor` does).

## Tests

```sh
mist run test
```

runs every suite in `engine/tests/` with `nox test`: one small program per test, each with an `expected.txt`. Most suites
need no GPU. Two programs in `tests/gpu` render with a real device and skip, logging why, when there isn't one; on a
machine with no display the script uses SDL's offscreen driver and a software Vulkan driver such as lavapipe. Set
`LANTERN_REQUIRE_GPU=1` to make a skip a failure. `engine/tools/check-window.sh` checks the stub scene's window
handling on Linux; it starts Xvfb and openbox when there's no display.

## If something fails

- **`mist: command not found` or `nox: command not found`:** install them (above) and put `~/.local/bin` on `PATH`.
- **A library can't be found at link time:** run `mist install` again, and run the game through `mist run` rather than by
  hand, since `mist run` sets the library search paths.
- **The window opens black or the program exits at start-up on a machine with no GPU:** install a software Vulkan driver
  (lavapipe, in `mesa-vulkan-drivers`), or run with `SDL_VIDEO_DRIVER=offscreen` for a headless capture.
- **"the GPU refused the post-processing textures":** the renderer draws the scene into a 16-bit float target, so it needs a
  GPU that can render to RGBA16F; every current desktop GPU can.
- **A shader looks wrong after an edit:** run `mist run build-shaders`; the engine loads the compiled files, not the GLSL.
