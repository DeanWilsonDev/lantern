# The world editor's Nox host

`apps/world-editor/nox` is the world editor's host, on Nox Penumbra. It mounts the editor's Chaos UI
(`apps/world-editor/src/ui`) through penumbra-bridge's `BridgeHost`, answers its `editor*` host
functions from `lantern:::editorbackend`, and draws into an SDL3 window.

## Building and running

The steps below run from the lantern checkout's root, on a branch that has `apps/world-editor/nox`.
penumbra-ui-engine (on `nox-proto`), penumbra-bridge (on `main`), nyx-scripting-language,
chaos-ui-framework and cosmos-styling-language (on `proto`, with their submodules) are checked out
beside lantern.

```sh
cd engine
tools/bind-sdl.sh
tools/bind-nyx.sh
tools/install-dependencies.sh
tools/build-shaders.sh
tools/build-native.sh
cd ../apps/world-editor/nox
tools/setup.sh
LIBRARY_PATH=../../../engine/generated/mist:../../../engine/generated/nyx nox build --release
LD_LIBRARY_PATH=../../../engine/generated/nyx ./build/lantern-world-editor
```

On macOS, add `/opt/homebrew/lib` to `LIBRARY_PATH` and run with `DYLD_LIBRARY_PATH` in place of
`LD_LIBRARY_PATH`. The editor opens `assets/worlds/course.world.json` unless given another world.

- `engine/tools/*` prepare the engine as `engine/README.md` describes: SDL3 and Nyx bindings,
  cgltf and stb_image, the compiled shaders, and lantern-native, which now carries
  penumbra-bridge's C API beside Nyx's and the Chaos UI bridge.
- `apps/world-editor/nox/tools/setup.sh` links `external/penumbra-nox` to the penumbra-ui-engine
  checkout (`PENUMBRA_NOX_DIR`, default `../penumbra-ui-engine` beside lantern), copies
  penumbra-bridge's Nox sources (`PENUMBRA_BRIDGE_SOURCE_DIR`) into `generated/penumbra-bridge` so
  they call lantern-native, and writes `generated/HostPaths.nox` with the UI, font, assets and
  shader directories.

`./build/lantern-world-editor --host-surface OUTPUT` writes the host functions it registers as a
nyx-proto host surface manifest instead of opening the window. `src/ui/.nyx.json` names the
committed one, `../src/ui/editor-host-surface.json`, for `nyx check` and nyx-lsp. After adding or
changing a host function, rewrite it and commit it:

```sh
LD_LIBRARY_PATH=../../../engine/generated/nyx ./build/lantern-world-editor --host-surface ../src/ui/editor-host-surface.json
```

`LANTERN_MAX_FRAMES` stops the editor after that many frames.

The host answers the editor's title, size, text input and file dialog requests. It leaves out font
reloads: the editor never asks for one, and the fonts take the window's DPI scale every frame.

## What's here

| File | What it does |
| --- | --- |
| `src/main.nox` | the window, fonts, backends and frame loop; the window requests (title, size, text input, file dialogs) the editor makes |
| `src/EditorFunctions.nox` | every `editor*` host function, each calling `lantern:::editorbackend` |
| `src/HostFunctions.nox` | helpers the editor functions share |
| `src/ViewportRenderers.nox` | the level view, the asset preview and each template's thumbnail, drawn by the editor backend into viewport frames |
| `src/FileDialogs.nox` | SDL3's open and save dialogs, answered back to the editor through `FinishFileDialog` |

A new `editor*` function goes in `src/EditorFunctions.nox` by hand, registered with the same Nyx
signature the UI calls it with.
