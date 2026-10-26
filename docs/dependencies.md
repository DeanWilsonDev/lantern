# Dependencies

Policy: as few crates as possible, each with a reason. Versions are pinned to a minor version in
`Cargo.toml` (`"0.7.2"` means `^0.7.2`), and `Cargo.lock` is committed so a clean checkout builds
exactly what was tested. Updating a dependency is a deliberate commit that re-runs every check.

| Crate | Version | Used by | Why |
|---|---|---|---|
| `sdl3-sys` | 0.7.2 (SDL 3.4.18) | sdl-facade, sdl-gpu | Raw SDL3 bindings: window, events, gamepad, audio streams and the SDL GPU API. The brief chose SDL3. We use the raw `-sys` crate rather than the higher-level `sdl3` crate because the GPU API must be wrapped anyway and the safe crate's GPU coverage lags SDL. The `build-from-source-static` feature (on by default through our `bundled-sdl` feature) builds SDL with CMake so macOS, Arch and Windows need no SDL install. |
| `glam` | 0.30 | core and up | Vectors, matrices, quaternions (brief requirement). No `unsafe` in our use. |
| `log` | 0.4 | all | Logging facade. `lantern-core` installs a ~40-line stderr logger, so no `env_logger`. |
| `png` | 0.18 | renderer-3d | PNG decoding for textures. Pure Rust, small. |
| `gltf` | 1.4, `default-features = false`, features `utils`, `names` | renderer-3d | glTF 2.0 parsing. Default features are off to avoid the `image` crate; we resolve `.glb` binary chunks ourselves and only read positions, normals, UVs, colours and indices. |
| `naga` | 30, build-dependency only | sdl-gpu | Translates WGSL shaders to SPIR-V (Vulkan) and MSL (Metal) at build time. Pure Rust, so no Vulkan SDK, glslang or Xcode tools are needed to build. |
| `nyx-sys`, `nyx` | path | scripting | Existing Nyx bindings (brief requirement). Build with CMake 3.30+ and GCC 14 / Clang 19. |
| `bindgen`, `cmake` | 0.72, 0.1 | nyx-sys (build) | Existing. |

Not used, on purpose:

- no ECS (`hecs`, `bevy_ecs`): the world is a handful of typed stores;
- no async runtime, no threads besides SDL's own audio device thread;
- no physics engine (`rapier`, `parry`): the brief's controller needs capsule casts against
  triangles, which `lantern-physics` implements in about a thousand readable lines;
- no font crate (`fontdue`, `ab_glyph`): the HUD and menu use an embedded public-domain 8×8
  bitmap font, scaled by whole pixels for high-DPI displays;
- no audio decoding crate: WAV (PCM 16-bit) is parsed by `lantern-audio` in about 80 lines;
- no `raw-window-handle`, `wgpu`, `ash`: SDL GPU owns the graphics API.

## Toolchain

- Rust stable 1.87 or later (naga needs 1.87). Edition 2021 for the existing nyx crates,
  edition 2024 for new crates.
- CMake 3.30 or later (Nyx; SDL also builds with it).
- A C/C++ compiler: GCC 14 or Clang 19 for Nyx (C++26 features), and any C compiler for SDL.
- Linux: X11 and/or Wayland, ALSA/PulseAudio/PipeWire development headers so SDL can build
  those backends (see `README.md` for the Arch and Ubuntu package lists).
