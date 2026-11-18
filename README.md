# Lantern

A small, readable 3D engine prototype, and a third-person platformer built on it. Lantern is a tutorial repository:
it exists to answer one question, *what must a small 3D platformer engine actually do each frame?*, and every file
explains itself.

- **The engine is written in Nox** (`engine/`), a natively compiled systems language, over SDL3 and SDL GPU.
- **All game code is Nyx** (`games/platformer/`), a scripting language the engine hosts through Nyx's C API. The engine
  and the game meet in one place, the host API in `engine/src/host/` (`docs/game-api.md`).
- **The game's menus and HUD are Chaos components** styled by Cosmos (`games/platformer/ui/`), and so is the world
  editor (`apps/world-editor/`), which runs on Penumbra, a UI engine written in Nox.
- **`crates/` is the Rust engine** the Nox engine was ported from. It stays as a reference to compare against and isn't
  part of the build below.

Start with [`docs/README.md`](docs/README.md): it links every document and says which to read first.

## Run it

Lantern's setup is managed by [Mist](https://github.com/fearlessgamelabs/mist-package-manager), the package manager
for Nox. With `nox` and `mist` installed (see [`docs/running.md`](docs/running.md) for the full steps on macOS, Arch
Linux, other Linux and Windows), from the repository root:

```sh
mist install            # fetch and build SDL3, cgltf, stb_image, Penumbra, penumbra-bridge and lantern-native
mist run game                      # play the platformer (`mist run game --stub` runs the engine's stub scene)
mist run editor         # open the world editor on the course
mist run test           # run the engine's tests
mist run build-shaders  # recompile the shaders after editing engine/shaders (needs glslangValidator and spirv-cross)
```

The scripts are defined in [`mist.json`](mist.json). The compiled shaders are committed, so building the game needs
neither shader tool.

## What's where

```text
engine/              the Nox engine: engine/src (one folder per domain), tests, shaders, native bridge, tools
games/platformer/    the demo game, all Nyx: entities, systems, rules, settings and Chaos UI
apps/world-editor/   the world editor: Chaos UI in src/ui and its Nox host in nox/
games/platformer/assets/   the game project's textures, models, sounds, music, Silhouettes, worlds and editor palette
docs/                the guides, the decision log and the kanban board that tracks the work
crates/              the Rust engine the Nox engine was ported from (reference only)
```

`docs/engine-structure.md` explains the engine's folders and why they're split that way.

## What has been tested

- The engine's tests pass headless on Linux, with SDL's offscreen driver and a software Vulkan driver (lavapipe).
- The platformer and the world editor have been run on macOS by hand.
- **Not yet checked:** frame rate on a real GPU (the 60 fps target), Metal output of the later renderer effects,
  Wayland on Arch Linux, and Windows. `docs/known-gaps.md` lists these and the features not built.

## Credits

The 8x8 bitmap font is Daniel Hepper's public-domain `font8x8_basic`. The world editor's UI font is Inter, under the
licence in `apps/world-editor/data/fonts/Inter-LICENSE.txt`. Actor (Thomas Junold, SIL Open Font License 1.1, in
`apps/world-editor/data/fonts/Actor-LICENSE.txt`) is kept beside it for the Graph tab's look.
