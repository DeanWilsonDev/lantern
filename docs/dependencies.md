# Dependencies

## The Nox engine (`engine/`, the current build)

Policy: as few libraries as possible, each with a reason. [Mist](https://github.com/fearlessgamelabs/mist-package-manager)
fetches and builds them from the root `mist.json`, and `mist.lock` pins the versions that were tested. Updating one is a
deliberate commit that re-runs `mist run test`.

| Package | Version or branch | Used by | Why |
| --- | --- | --- | --- |
| SDL3 (`sdl3`) | 3.4 | `engine/src/sdl-facade/`, the editor host | The window, events, gamepad, audio device and the SDL GPU API that draws everything. Behind the facade folder, so the rest of the engine never names it (`engine-structure.md`). |
| SDL3_image, SDL3_ttf | 3.4, 3.2 | the world editor | Images and TrueType fonts for the editor's UI. The game doesn't use them. |
| cgltf (`cgltf`) | 1.15 | `renderer/GltfLoader.nox` | Reads `.glb` and `.gltf` meshes. |
| stb_image (`stb_image`) | 2 | `renderer/Texture.nox` | Reads PNG textures. |
| Nyx (`nyx-scripting-language`, `proto`) | through `lantern-native` | `engine/src/nyx/` | The scripting language the game is written in, hosted through its C API. |
| Chaos and Cosmos (`chaos-ui-framework`, `cosmos-styling-language`, `proto`) | through `lantern-native` | `engine/src/chaosui/`, the editor | The UI language and its stylesheets, for the game's menus and HUD and for the editor. |
| `penumbra-ui-engine` (`nox-proto`) and `penumbra-bridge` (`main`) | direct | `apps/world-editor/nox` | The UI engine and its C bridge the editor runs on. The game doesn't use them. |
| `lantern-native` | path (`engine/native`) | everything that calls Nyx or Chaos | One library holding Nyx's C API, the Chaos UI bridge and penumbra-bridge's C API, so there's exactly one copy of Nyx in a process. Built by CMake from `engine/native/mist.json`. |

Not used, on purpose: no ECS (the world is a handful of typed stores), no physics engine (the character controller needs
capsule casts against triangles, which `engine/src/physics/` implements in a few readable files), no font library (the
HUD uses an embedded public-domain 8x8 bitmap font), no audio decoding library (`audio/Wav.nox` reads 16-bit PCM WAV) and
no threads besides SDL's own audio thread.

Shaders are GLSL in `engine/shaders/`, compiled by `glslangValidator` to SPIR-V and by `spirv-cross` to Metal Shading
Language (`mist run build-shaders`); the compiled files are committed.

## Toolchain

- `nox` and `mist`, the Nox compiler and its package manager (`docs/running.md` has the install steps).
- CMake 3.30 or later and a C and C++ compiler (GCC 14 or Clang 19 or later): Mist uses them to build SDL3 and
  `lantern-native`, whose Nyx, Chaos and Cosmos sources are C++26.
- Linux: X11 or Wayland and ALSA, PulseAudio or PipeWire development headers so SDL3 can build those backends.
- Only to recompile shaders: `glslangValidator` and `spirv-cross`.
- No Rust toolchain is needed. Lantern had a Rust engine before the port to Nox; it was removed from the repository on
  2026-11-18 (`D-208`).
