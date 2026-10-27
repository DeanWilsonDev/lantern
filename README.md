# Lantern

A small, readable 3D engine prototype in Rust, and a third-person platformer built on it. All
game code is Nyx (`games/platformer/`), hosted through Nyx's C ABI; all engine code is Rust
(`crates/`), and `crates/game-host` is the seam between them. Lantern follows the domain split of Umbra (the C++
engine in the snake repository) and is meant as a reference for a later rewrite.

- `docs/brief.md` — what the prototype is for and its milestones.
- `docs/architecture.md` — the tour: crates, per-frame sequence, ownership and shutdown, the
  game seam, a rewrite checklist and the known omissions.
- `docs/game-api.md` — every engine call a Nyx game can make, and the hooks the engine calls.
- `docs/decisions.md` — every decision made along the way, with reasons (D-001 onwards).
- `docs/dependencies.md` — each third-party crate and why; `docs/future-work.md` — later ideas.

## Prerequisites

| | Version | Why |
|---|---|---|
| Rust | stable 1.87 or later | naga needs 1.87 |
| CMake | 3.30 or later | builds Nyx and SDL |
| C++ compiler | GCC 14 or Clang 19 | Nyx uses C++26 features |
| libclang | any recent | bindgen, for the Nyx bindings |
| Nyx | `nyx-scripting-language` on `proto`, checked out **beside this repository** | the game runs in it |

SDL 3 is built from source by `sdl3-sys` and linked statically, so no SDL install is needed.
On Linux, SDL only builds the window and audio backends whose development headers it finds.

```text
~/code/lantern
~/code/nyx-scripting-language   (git checkout proto)
```

The C API (`include/nyx/nyx.h`) is on Nyx's `proto` branch since nyx PR #7 merged (D-002).
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
cargo build                          # debug build of the engine and the runner
cargo test --workspace               # unit and integration tests (no window needed)
cargo run -p lantern                 # play games/platformer (debug build)
cargo run -p lantern --release       # play at full speed; use this for the 60 fps check
cargo run -p lantern -- path/to/game # run another game directory (it needs a game.json)
```

The game's `.nyx` files are read when the game starts, so editing them needs no rebuild: save
and press F5 in the running game.

The checks that must stay green:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -p asset-gen -- --check    # assets/ matches the generator
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
| Reload the game's Nyx code | F5 | |

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
| `LANTERN_DEBUG_HUD=1`, `LANTERN_COLLISION_VIEW=1` | start with the debug views on |
| `LANTERN_MAX_FRAMES=n` | quit after n frames |
| `LANTERN_CAPTURE_PATH=out.png` | save the last frame as a PNG |
| `LANTERN_AUTOPLAY=1` | skip the menu; a scripted pilot runs and jumps (for smoke tests) |

A headless smoke run (how the game was checked without a display):

```sh
LANTERN_AUTOPLAY=1 LANTERN_DEBUG_HUD=1 LANTERN_MAX_FRAMES=600 \
LANTERN_CAPTURE_PATH=smoke.png SDL_AUDIO_DRIVER=dummy cargo run -p lantern
```

## What has been tested

Everything was built and checked on Linux in a cloud container without a GPU or display:
Vulkan through Mesa's software rasteriser (lavapipe) under Xvfb, with validation on, and audio
through SDL's dummy driver. The game reached the goal in automated runs at 30, 60, 144 and
240 Hz display rates. Since the move to Nyx (D-023), headless tests run the Nyx game through
the real frame loop (`crates/game-host/tests/headless_game.rs`), and a 400-frame run under Xvfb
showed about 1 ms of Nyx per frame. Not yet tested: macOS/Metal, Wayland, a real GPU, real audio output,
Windows, and the release-build 60 fps check.

## Other contents

- `crates/nyx-sys`, `crates/nyx`: raw and safe bindings to `nyx-capi`, the Nyx prototype's C ABI.
- `examples/frame-loop`: the original Nyx hosting example (`cargo run -p frame-loop`).
- `tools/asset-gen`: regenerates the textures, gem model, sounds and music in `assets/`
  (`cargo run -p asset-gen`).
