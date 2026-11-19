# Wayland and Windows: what has been checked

Lantern's primary platforms are macOS and Arch Linux on Wayland, and Windows should build. A Mac run is the only one Dean
has done by hand. This page records what a cloud session could check without a Mac, an Arch machine or a Windows
machine, what it found, and what is left for a real machine. [`running.md`](running.md) has the install and run steps, and
[`known-gaps.md`](known-gaps.md) the short list of what is still open.

## Wayland

### How it was checked

A cloud session has no display, so it runs a Wayland compositor with no screen: Weston's headless backend, with Mesa's
lavapipe (a software Vulkan driver) drawing into it. The game and the editor then run exactly as they would on a desktop
with `SDL_VIDEO_DRIVER=wayland`, with SDL talking to a real Wayland compositor and Vulkan presenting to a Wayland surface.

```sh
sudo apt install weston mesa-vulkan-drivers
engine/tools/check-wayland.sh
```

The script builds the release game, starts Weston on a private socket, plays the platformer for 60 frames under the
autopilot and runs the stub scene's window script, then checks the logs. It stops Weston when it ends. It takes about
15 seconds.

### What worked

| Check | Result |
| --- | --- |
| SDL picks the Wayland video driver | `SDL 3.4.20 initialised, video driver: wayland` |
| Vulkan swapchain on a Wayland surface | `GPU driver: vulkan (SPIR-V shaders)`, sRGB swapchain, frames render |
| The platformer, 240 frames under the autopilot, with a capture | Ran and drew the scene, HUD text and character correctly |
| The world editor, 90 frames | Started, ran and exited 0 |
| Resize | 1280x720 to 800x450 points, the renderer followed |
| Minimise | The loop stopped rendering and simulating, and quit cleanly afterwards (no frames rendered while minimised) |
| No audio device | `No available audio device`, then the game carried on without sound |

### What differs from X11

- **A client can't restore a window it minimised.** Wayland has no request for it, so the stub window script's restore
  step does nothing there. `check-window.sh` (the X11 check, which starts Xvfb) still checks the restore, and
  `check-wayland.sh` leaves it out. On a real Wayland desktop the person restores the window from the task bar.
- **The window can't place itself.** Lantern never sets a window position, so this changes nothing.
- **Window decorations** come from the compositor when it offers them (KDE, Sway) and from libdecor when it doesn't
  (GNOME). libdecor is in the Arch package list in `running.md` for that reason.

### One change made

`SdlContext` now sets `SDL_HINT_APP_ID` to `lantern`. A Wayland compositor matches a window to a desktop entry by its app
id, so without one the window has no icon or name in the task bar. On X11 the same hint is the window class, and on
Windows the taskbar group.

### What only a real machine can show

Mark these off on an Arch machine (and Hyprland, Sway, KDE or GNOME, whichever Dean uses):

- [ ] `mist install` and `mist run game` build and open a window with `pacman` packages only (the list in `running.md`).
- [ ] The log says `video driver: wayland` and `GPU driver: vulkan`, with your GPU's driver and not lavapipe
  (`vulkaninfo --summary` names it).
- [ ] **Mouse look.** Click Play, the pointer disappears, and moving the mouse turns the camera at the same speed as on
  macOS. Escape gives the pointer back. (Weston headless has no pointer, so relative mouse mode
  `SDL_SetWindowRelativeMouseMode` hasn't run.)
- [ ] **Gamepad.** Plug one in. Move, look, jump (South) and pause (Start) work. If it isn't seen, the udev rules for
  the controller are the usual cause.
- [ ] **Audio.** Music and effects play through PipeWire or PulseAudio, with no crackle and no underruns (F1, Audio
  page). The mixer runs on the main thread, so a slow frame is audible.
- [ ] **Scale.** On a fractional-scaling display the log's `density` is above 1 and the picture is sharp, not blurred. The
  HUD and the editor are sized for it.
- [ ] **Frame rate.** About 60 fps with vertical sync, and the F1 window's number with `LANTERN_VSYNC=0`.
- [ ] **Minimise and workspace switch.** Minimise the window, or move to another workspace, and come back. The game
  should pause and carry on. A compositor that stops sending frame callbacks to a hidden window can make the swapchain
  wait block the whole loop (and the audio with it), which would show as sound that stops while the window is hidden.
- [ ] **The editor.** `mist run editor` opens, the menus, dock splits and file dialogs work, and the Play button starts
  the game in its own window.
- [ ] `SDL_VIDEO_DRIVER=x11 mist run game` (XWayland) behaves the same, for comparison.

## Windows

### What was checked

| Check | Result |
| --- | --- |
| `nox check --target-platform windows` on the engine | Passes: every `TARGET_PLATFORM` branch type-checks |
| `nox build --target-platform windows --release` | All of Lantern's Nox compiles to C and through MinGW GCC 13 to object code, then stops at the link: there are no Windows builds of `SDL3`, `cgltf`, `stb_image` or `lantern-native` |
| Windows-specific code in the engine and editor | Read through: see below |

### What the code already does for Windows

- `editorbackend/Play.nox` asks SDL for the platform and starts `lantern.exe` there, splits paths at `/` and `\`, and finds
  the runner beside the editor with `SDL_GetBasePath`.
- `editor/Preferences.nox` keeps its file in `APPDATA` (or `USERPROFILE`) when `XDG_CONFIG_HOME` and `HOME` are missing.
- `editor/SessionTypes.nox` maps the command key to Ctrl.
- The renderer asks for SPIR-V and Metal shaders only, so SDL GPU picks Vulkan on Windows (D-009), and it copes with the
  blue-green-red swapchain order most Windows Vulkan drivers use (`GpuDevice.nox`).
- The Nox library has Windows paths for files, time and threads (`nox-systems-language`, `docs/implementation/library.md`).

### What is missing

| Gap | Where it stands |
| --- | --- |
| **Windows builds of the four libraries.** SDL3, cgltf and stb_image build with MinGW easily. `lantern-native` builds Nyx, Chaos and Cosmos, which need C++26 (GCC 14, Clang 19), and Ubuntu's MinGW is GCC 13. A MinGW with GCC 14 (MSYS2) or `llvm-mingw` would do. | Open. Card `engine-follow-ups/wayland-and-windows-checks`. |
| **Mist can't run scripts on Windows.** It starts each one with `/bin/sh -c`. | Card MIST-29 on the Mist board. Until then, run the commands the scripts hold by hand. |
| **`lantern-native` exports.** On Windows a DLL exports only what it marks, and the C functions it carries (`nyx_*`, `penumbra_bridge_*`, `lantern_chaos_ui_*`, `amanuensis_calamus_*`) aren't marked. `engine/native/CMakeLists.txt` now asks CMake to export every symbol on Windows. | Written, never built on Windows. |
| **Packaging.** `mist run package` copies only `bin/lantern`. On Windows the package also needs `lantern.exe` and the DLLs beside it, since Windows has no rpath. | Open, with the other packaging work (D-217). |
| **Shell scripts.** `engine/tools/*.sh` are bash, so they need Git Bash or WSL. | Open. |

### The route to try

Nox cross-compiles to Windows through the C backend (`nox build --target-platform windows`, using
`x86_64-w64-mingw32-gcc` or `NOX_WINDOWS_CC`) and tests under Wine, so the first Windows build is a cross-build from Linux
or macOS, copied to a Windows machine. In order:

1. Build `SDL3`, the only SDL library the engine links today. Configure it with a MinGW toolchain file and
   install the import library and `SDL3.dll`.
2. Build `cgltf` and `stb_image` as static libraries with `x86_64-w64-mingw32-gcc -c` and `ar`.
3. Build `lantern-native` as a DLL with a MinGW (or clang) toolchain that has C++26. Check that `nyx_runtime_create` and
   the other exports are in it (`llvm-objdump -p lantern-native.dll`).
4. `nox build --target-platform windows --release`, with the library folders on `LIBRARY_PATH`.
5. Copy `lantern.exe`, the DLLs, `engine/shaders/compiled/` and a game folder to a Windows machine and run it with a
   Vulkan driver installed.

## Where this page's claims come from

- Wayland: `engine/tools/check-wayland.sh`, Weston (headless backend) and Mesa lavapipe in a Linux x86-64 cloud session,
  2026-11-19. A real compositor and a real GPU may differ.
- Windows: `nox check` and `nox build` with `--target-platform windows` on Linux x86-64, 2026-11-19, with MinGW-w64
  GCC 13 and no Windows machine.
