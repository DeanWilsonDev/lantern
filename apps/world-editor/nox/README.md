# The world editor's Nox host

`apps/world-editor/nox` is the world editor's host, on Nox Penumbra. It mounts the editor's Chaos UI
(`apps/world-editor/src/ui`) through penumbra-bridge's `BridgeHost`, answers its `editor*` host
functions from `lantern:::editorbackend`, and draws into an SDL3 window.

## Building and running

From the lantern checkout's root:

```sh
mist install
mist run editor
```

`mist install` fetches and builds what the root `mist.json` names (see `engine/README.md`), and `mist run editor` runs
`nox run --release --project apps/world-editor/nox` with Mist's library directories on the search paths. Arguments after
`editor` go to the editor. It builds the game (`engine/build/lantern`) first, since that's what the editor's Play button
runs; when the game is up to date that step does nothing. It opens `assets/worlds/course.world.json` unless given another world.

- `nox.json` takes Penumbra's Nox sources from `@penumbra-ui-engine` (on `nox-proto`) and penumbra-bridge's Nox side
  from `@penumbra-bridge` (on `main`). `foreignLibraries` links lantern-native wherever penumbra-bridge's foreign blocks
  name `penumbra-bridge-capi`, since lantern-native carries that API.
- `src/HostPaths.nox` names the UI, font, assets and shader directories relative to the repository root, which is where
  `mist run` starts the editor. To work on penumbra-ui-engine or penumbra-bridge beside lantern, `mist link` your
  checkout, as `engine/README.md` describes.

`mist run editor --host-surface apps/world-editor/src/ui/editor-host-surface.json` writes the host
functions it registers as a nyx-proto host surface manifest instead of opening the window.
`src/ui/.nyx.json` names that committed file for `nyx check` and nyx-lsp, so rewrite it and commit it
after adding or changing a host function.

`LANTERN_MAX_FRAMES` stops the editor after that many frames.

The host answers the editor's title, size, text input and file dialog requests. It leaves out font
reloads: the editor never asks for one, and the fonts take the window's DPI scale every frame.

## What's here

| File | What it does |
| --- | --- |
| `src/HostPaths.nox` | the UI, font, assets and shader directories, relative to the repository root |
| `src/main.nox` | the window, fonts, backends and frame loop; the window requests (title, size, text input, file dialogs) the editor makes |
| `src/EditorFunctions.nox` | every `editor*` host function, each calling `lantern:::editorbackend` |
| `src/HostFunctions.nox` | helpers the editor functions share |
| `src/ViewportRenderers.nox` | the level view, the asset preview and each template's thumbnail, drawn by the editor backend into viewport frames |
| `src/FileDialogs.nox` | SDL3's open and save dialogs, answered back to the editor through `FinishFileDialog` |

A new `editor*` function goes in `src/EditorFunctions.nox` by hand, registered with the same Nyx
signature the UI calls it with.
