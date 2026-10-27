# Lantern world editor

The world editor opens a world file, shows it in a 3D viewport and lets you select, move, rotate,
scale, add, duplicate, delete and re-parent entities, edit their fields, undo and redo, and save.
It is isolated from the game: it never links the game, and the game never links it. The two
share only the world file format (`crates/world-format`) and the game's assets.

![The editor with a block selected](editor-screenshot.png)

## Architecture

```text
apps/world-editor (C++26, CMake)                      crates/ (Rust)
┌──────────────────────────────────────────┐          ┌──────────────────────────────────┐
│ src/ui/*.chaos *.cosmos *.nyx            │          │ lantern-editor-backend           │
│   toolbar, outliner, inspector, status   │          │   EditorBackend                  │
│   EditorApplication (Nyx Application)    │          │   ├── EditorSession ─────────────┼──► lantern-editor-core
│        │ calls editor*() host functions  │          │   └── GpuRenderer (off-screen) ──┼──► lantern-sdl-gpu
│        ▼                                 │  C API   │   ffi.rs                         │
│ src/EditorHost.cpp ──────────────────────┼─────────►│   include/lantern_editor.h       │
│ src/native/LanternViewportWidget.cpp ────┼─────────►│                                  │
│   streams RGBA pixels into an SDL        │          └──────────────────────────────────┘
│   texture; forwards mouse, wheel, keys   │                       │
│                                          │                       ▼
│ Penumbra · Chaos · Cosmos · nyx-proto    │          lantern-world-format ◄── the game host
└──────────────────────────────────────────┘          (WorldFile, build_pieces)   loads the same file
```

- **`lantern-world-format`** is the seam. It defines the JSON world file (`WorldFile`,
  `WorldEntity`, shapes, materials, tints, colliders and free-form properties) and turns a world
  into meshes and collision triangles (`build_pieces`). The game host and the editor both use it.
- **`lantern-editor-core`** is all the editor's logic, in plain Rust with no window or GPU:
  the document and its undo history, selection, the orbit camera, picking, gizmos with snapping,
  the inspector's fields, the outline, and the input state machine. It is unit tested.
- **`lantern-editor-backend`** owns one `EditorSession` and an off-screen `GpuRenderer`. It
  draws the viewport into memory and exposes everything through a C API
  (`crates/editor-backend/include/lantern_editor.h`). It is built as a static library linked
  against the system SDL3, the same SDL Penumbra uses.
- **`apps/world-editor`** is the front end, built the same way as Pharos. `main.cpp` opens the
  backend and hands control to `ChaosApplicationHost`. `EditorHost.cpp` registers one Nyx host
  function per C API call (`editorRevision()`, `editorSelect(id, additive)` and so on) and the
  `LanternViewport` native widget. The panels are Chaos components styled by Cosmos.
  `EditorApplication.nyx` polls `editorRevision()` each frame and refreshes its signals when
  the revision changes.

### What the editor knows about the game

Nothing by name. The game ships `assets/editor/palette.json`, which lists:

- **templates** for the Add buttons (a label plus a partial entity: kind, shape, material, tint,
  collider, properties, and how far to lift it above the surface it lands on);
- **markers**: how to draw an entity that has no shape (a gem mesh, a capsule, a plane outline),
  by kind;
- **materials**: the names the inspector offers, each a texture in `assets/textures/`.

Without a palette the editor still works, with generic blocks, ramps and markers.

## Build and run

You need everything the game needs, plus the C++ UI stack:

- GCC 14 or Clang 19, CMake 3.30 or later and Ninja.
- SDL3, SDL3_ttf and SDL3_image installed where CMake and pkg-config find them. On Arch:
  `pacman -S sdl3 sdl3_ttf sdl3_image`. On macOS: `brew install sdl3 sdl3_ttf sdl3_image`.
- Network access on the first configure, when CMake fetches Penumbra, Chaos, Cosmos,
  penumbra-ui-backend and Cimmerian (and Chaos fetches nyx-proto).

```sh
cmake -S apps/world-editor -B apps/world-editor/build -G Ninja -DCMAKE_BUILD_TYPE=Release
cmake --build apps/world-editor/build
apps/world-editor/build/lantern-world-editor                      # opens assets/worlds/course.world.json
apps/world-editor/build/lantern-world-editor path/to/other.world.json
```

CMake runs `cargo build -p lantern-editor-backend` itself on every build. If SDL3 is somewhere
pkg-config doesn't look, set `PKG_CONFIG_PATH` (for example `/usr/local/lib/pkgconfig`).

The Rust parts are tested without the app:

```sh
cargo test -p lantern-world-format -p lantern-editor-core
cargo test -p lantern-editor-backend      # needs a GPU or lavapipe for the render test
```

## Controls

| Input | Action |
|---|---|
| Left click | Select (Shift adds or removes) |
| Left drag on a gizmo handle | Move, rotate or scale along that axis |
| Left drag on a selected object | Move it across the surface under the cursor |
| Right drag, or Alt + left drag | Orbit |
| Shift + right drag, or middle drag | Pan |
| Wheel | Zoom |
| Hold right button + W A S D Q E | Fly |
| W / E / R | Move / rotate / scale tool |
| F | Frame the selection (or the whole world) |
| Delete or Backspace | Delete the selection |
| Ctrl/Cmd + Z, Ctrl/Cmd + Shift + Z or Ctrl + Y | Undo, redo |
| Ctrl/Cmd + D | Duplicate |
| Ctrl/Cmd + S | Save |
| Ctrl/Cmd + A | Select all |
| Esc | Cancel a drag, or clear the selection |

Viewport keys work while the pointer is over the viewport. The toolbar has buttons for the
same actions, plus New, Open and Save As (they use the path field), the Snap toggle and one
Add button per palette template. Snapping steps are 0.25 m, 15° and 0.1 scale.

In the inspector, typing applies as you go: each keystroke that parses is applied, and a run of
typing into one field is one undo step. Invalid text is reported in the status bar and doesn't
change the world. Choice fields (kind, shape, material, collider) open a list.

## C API rules

- One handle, used from one thread (the UI thread). A null handle is ignored.
- Strings passed in are NUL-terminated UTF-8, borrowed for the call. Strings returned stay valid
  until the next string-returning call on the same handle, so copy them straight away.
- Entity ids are the world file's ids as `int64_t`; 0 means none.
- No Rust panic crosses the boundary. A panic is caught, logged and shown in the status line,
  and the function returns its default.
- `lantern_editor_revision` changes whenever anything a panel shows changes (camera moves don't
  count), so the front end polls it once a frame instead of receiving callbacks.

## Known gaps

These are filed under the "Game engine" epic on the Nyx kanban in fearless-hq.

- **A click shorter than one frame is lost.** Penumbra samples mouse buttons once per frame
  rather than queueing press and release events.
- **`<input text>` isn't updated after mount.** penumbra-ui-backend applies a changed `text`
  prop to labels only. The editor works around this by emptying the inspector for one frame
  whenever the selection or an outside edit changes it, so the fields mount fresh.
- **Ctrl/Cmd+A doesn't select a text field's contents**, so you clear a field with Backspace.
- **Non-ASCII text in a `.chaos` element is a lex error** (NYX0001), so labels use ASCII.
- **No viewport primitive.** The 3D view is a C++ native widget. A Chaos primitive for an
  external renderer (or `<model3d>` for Umbra) would let it be written in `.chaos`.
- **Keyboard input for native widgets.** Penumbra's `InputState` has only a few keys, so the
  viewport reads SDL's keyboard state directly while the pointer is over it.
- **No dropdown or checkbox primitive.** Choice fields are a button that opens a list.
- **No modal file dialog.** Open and Save As take a path typed in the toolbar.
- **Single-entity inspector.** With several entities selected, the inspector shows the first;
  gizmos move all of them.
- **No re-parenting in the UI yet.** The backend supports it (keeping world position and
  refusing cycles), but there is no drag-and-drop in the outliner.
