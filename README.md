# Lantern

A small, readable 3D engine prototype in Rust, and a third-person platformer built on it. The
game's rules run in Nyx through its C ABI. Lantern follows the domain split of Umbra (the C++
engine in the snake repository) and is meant as a reference for a later rewrite.

- `docs/brief.md` — what the prototype is for and its milestones.
- `docs/architecture.md` — the tour: crates, per-frame sequence, ownership and shutdown, the
  script API, a rewrite checklist and the known omissions.
- `docs/decisions.md` — every decision made along the way, with reasons (D-001 onwards).
- `docs/editor.md` — the world editor: architecture, build steps and controls.
- `docs/dependencies.md` — each third-party crate and why; `docs/future-work.md` — later ideas.

## Prerequisites

| | Version | Why |
|---|---|---|
| Rust | stable 1.87 or later | naga needs 1.87 |
| CMake | 3.30 or later | builds Nyx and SDL |
| C++ compiler | GCC 14 or Clang 19 | Nyx uses C++26 features |
| libclang | any recent | bindgen, for the Nyx bindings |
| Nyx | `nyx-scripting-language` on a branch with the C API, checked out **beside this repository** | the scripting layer |

SDL 3 is built from source by `sdl3-sys` and linked statically, so no SDL install is needed.
On Linux, SDL only builds the window and audio backends whose development headers it finds.

```text
~/code/lantern
~/code/nyx-scripting-language   (git checkout claude/project-thread-qf7t4j)
```

The C API (`include/nyx/nyx.h`) is not on Nyx's `proto` branch yet. It is on
`claude/project-thread-qf7t4j`, nyx PR #7 (see D-002). Once that is merged, check out `proto`.
Run `git submodule update --init --recursive` in the Nyx checkout after switching branches.

Set `NYX_SOURCE_DIR` to use a checkout somewhere else, or `NYX_CAPI_PREFIX` to link a prefix
made by `cmake --install build --component nyx-capi --prefix <dir>`.

### Arch Linux

```sh
sudo pacman -S --needed rustup base-devel cmake clang llvm \
    wayland wayland-protocols libxkbcommon libdecor \
    libx11 libxext libxrandr libxcursor libxi libxss \
    alsa-lib libpulse pipewire vulkan-icd-loader mesa
# plus the Vulkan driver for your GPU: vulkan-radeon, vulkan-intel or nvidia-utils
rustup default stable
```

Arch's GCC is 14 or newer, so the default compiler works. On Wayland, SDL picks the Wayland
backend by itself; `SDL_VIDEO_DRIVER=x11` forces XWayland for comparison.

### macOS

```sh
xcode-select --install
brew install rustup cmake llvm@19
rustup default stable
export CC="$(brew --prefix llvm@19)/bin/clang" CXX="$(brew --prefix llvm@19)/bin/clang++"
export LIBCLANG_PATH="$(brew --prefix llvm@19)/lib"
```

Apple's clang is too old for Nyx, which is why `CC`/`CXX` point at Homebrew's Clang 19.
Rendering uses Metal through SDL GPU (shaders are translated to MSL at build time).

### Ubuntu / Debian (other Linux)

```sh
sudo apt install build-essential cmake g++-14 libclang-dev \
    libwayland-dev wayland-protocols libxkbcommon-dev libdecor-0-dev \
    libx11-dev libxext-dev libxrandr-dev libxcursor-dev libxi-dev libxss-dev \
    libasound2-dev libpulse-dev libpipewire-0.3-dev libudev-dev libvulkan1 mesa-vulkan-drivers
export CC=gcc-14 CXX=g++-14
```

### Windows

Install Rust, CMake 3.30+, LLVM 19 (for libclang and clang-cl) and Visual Studio 2022's C++
tools. SDL GPU uses Vulkan there (no D3D12 shaders yet), so a Vulkan driver is needed. Windows
has not been built yet; see "What has been tested".

## Build, test, run

```sh
cargo build                          # debug build of the engine and the game
cargo test --workspace               # unit and integration tests (no window needed)
cargo run -p platformer-game         # play (debug build)
cargo run -p platformer-game --release   # play at full speed; use this for the 60 fps check
```

Without the Nyx toolchain you can still build and play with the Rust rules:

```sh
cargo run -p platformer-game --no-default-features --features bundled-sdl
```

The checks that must stay green:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo clippy -p platformer-game --no-default-features --features bundled-sdl --all-targets -- -D warnings
cargo test --workspace
cargo run -p asset-gen -- --check    # assets/ matches the generator
```

### World editor

The editor (`apps/world-editor`) is a CMake project, because its UI runs on Penumbra, Chaos and
Nyx. It needs GCC 14 or Clang 19, CMake 3.30, Ninja, and SDL3, SDL3_ttf and SDL3_image
installed on the system. See `docs/editor.md`.

```sh
cmake -S apps/world-editor -B apps/world-editor/build -G Ninja -DCMAKE_BUILD_TYPE=Release
cmake --build apps/world-editor/build
apps/world-editor/build/lantern-world-editor
```

## Controls

| Action | Keyboard and mouse | Gamepad |
|---|---|---|
| Move | WASD / arrow keys | left stick |
| Look | mouse (click to capture) | right stick |
| Jump (hold for higher) | Space | A |
| Reset to the start | R | Y |
| Pause menu | Esc | Start |
| Debug HUD | F1 | Back |
| Collision view | F2 | right stick click |
| Reload the Nyx script | F5 | |

Collect the six gems. Falling off the level puts you back at the start.

## Environment variables

All optional.

| Variable | Effect |
|---|---|
| `LANTERN_LOG=debug` | more logging |
| `LANTERN_GPU_DEBUG=1` | GPU validation layers |
| `LANTERN_GPU_DRIVER=vulkan\|metal` | force a GPU driver |
| `LANTERN_VSYNC=0` | turn vsync off (to measure frame time) |
| `LANTERN_ASSETS=dir` | where `assets/` is |
| `LANTERN_RULES=native` | use the Rust rules instead of the Nyx script |
| `LANTERN_SCRIPTS=dir` | where `platformer.nyx` is (default `apps/platformer-game/scripts`) |
| `LANTERN_DEBUG_HUD=1`, `LANTERN_COLLISION_VIEW=1` | start with the debug views on |
| `LANTERN_MAX_FRAMES=n` | quit after n frames |
| `LANTERN_CAPTURE_PATH=out.png` | save the last frame as a PNG |
| `LANTERN_AUTOPLAY=1` | skip the menu; a scripted pilot runs and jumps (for smoke tests) |

A headless smoke run (how the game was checked without a display):

```sh
LANTERN_AUTOPLAY=1 LANTERN_DEBUG_HUD=1 LANTERN_MAX_FRAMES=600 \
LANTERN_CAPTURE_PATH=smoke.png SDL_AUDIO_DRIVER=dummy cargo run -p platformer-game
```

## What has been tested

Everything was built and checked on Linux in a cloud container without a GPU or display:
Vulkan through Mesa's software rasteriser (lavapipe) under Xvfb, with validation on, and audio
through SDL's dummy driver. The game reached the goal in automated runs at 30, 60, 144 and
240 Hz display rates. Not yet tested: macOS/Metal, Wayland, a real GPU, real audio output,
Windows, and the release-build 60 fps check.

## Other contents

- `crates/nyx-sys`, `crates/nyx`: raw and safe bindings to `nyx-capi`, the Nyx prototype's C ABI.
- `examples/frame-loop`: the original Nyx hosting example (`cargo run -p frame-loop`).
- `tools/asset-gen`: regenerates the textures, gem model, sounds and music in `assets/`
  (`cargo run -p asset-gen`).
