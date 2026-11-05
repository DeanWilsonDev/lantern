# The world editor's Nox host

`apps/world-editor/nox` runs the world editor on Nox Penumbra. It mounts the same Chaos UI as the C++
host (`apps/world-editor/src/ui`) through penumbra-bridge's `BridgeHost`, answers its `editor*` host
functions from `lantern:::editorbackend`, and draws into an SDL3 window.

## Building and running

1. Build lantern-native, which now carries penumbra-bridge's C API beside Nyx's and the Chaos UI
   bridge: `engine/tools/build-native.sh` (needs nyx-scripting-language, chaos-ui-framework and
   cosmos-styling-language on `proto` and penumbra-bridge on `main` beside lantern).
2. `tools/setup.sh` links `external/penumbra-nox` to a penumbra-ui-engine checkout on `nox-proto`
   (`PENUMBRA_NOX_DIR`, default `../penumbra-ui-engine` beside lantern), copies penumbra-bridge's Nox
   sources into `generated/penumbra-bridge` so they call lantern-native, and writes
   `generated/HostPaths.nox` with the UI, font, assets and shader directories.
3. Build: `LIBRARY_PATH=../../../engine/generated/mist:../../../engine/generated/nyx nox build --release`
   (on macOS add `/opt/homebrew/lib`).
4. Run: `LD_LIBRARY_PATH=../../../engine/generated/nyx ./build/lantern-world-editor [world.json]`
   (`DYLD_LIBRARY_PATH` on macOS).

`./build/lantern-world-editor --host-surface OUTPUT` writes the host functions it registers as a
nyx-proto host surface manifest instead of opening the window. `LANTERN_MAX_FRAMES` stops it after
that many frames.

## What's here

| File | What it does |
| --- | --- |
| `src/main.nox` | the window, fonts, backends and frame loop; the window requests (title, size, text input, file dialogs) the editor makes |
| `src/EditorFunctions.nox` | every `editor*` host function, generated from the C++ host's `EditorHost.cpp` by `tools/generate-host-functions.py` |
| `src/HostFunctions.nox` | helpers the generated functions share |
| `src/ViewportRenderers.nox` | the level view, the asset preview and each template's thumbnail, drawn by the editor backend into viewport frames |
| `src/FileDialogs.nox` | SDL3's open and save dialogs, answered back to the editor through `FinishFileDialog` |

After a change to `apps/world-editor/src/EditorHost.cpp`'s functions, run
`tools/generate-host-functions.py ../src/EditorHost.cpp src/EditorFunctions.nox`.
