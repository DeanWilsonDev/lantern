# Testing in a cloud session

A cloud session can build `nox` and `mist` and run the whole suite with `mist run test`, so a game change can be
tested before it reaches a Mac. A clean bootstrap takes about 15 minutes on 4 cores (Nox about 6, Mist about 2 and
`mist install` about 4); a session that has already built them does it again in seconds.

## Bootstrap

```sh
engine/tools/cloud-bootstrap.sh
export PATH="$HOME/.local/bin:$PATH"
mist run test
```

The script is safe to run again. It:

1. Points `https://github.com/fearlessgamedev/` at `fearlessgamelabs`, because submodule URLs still use the old
   organisation name.
2. Takes `nox-systems-language` (branch `proto`) and `mist-package-manager` (branch `main`) from beside the Lantern
   checkout, cloning them if missing, and initialises their submodules.
3. Builds and installs `nox` and `nox-lsp` (GCC 14, Release, unity build) and `mist` into `~/.local`.
4. Runs `mist install` in Lantern, which builds SDL3, SDL3_image, SDL3_ttf, cgltf, stb_image and `lantern-native`
   (which builds Nyx and Chaos).

Settings: `LANTERN_WORKSPACE` (where the sibling checkouts live), `LANTERN_PREFIX` (default `~/.local`),
`LANTERN_JOBS`, and `CC`/`CXX` (default `gcc-14` and `g++-14`).

## What the session needs

- GCC 14, CMake 3.30 or later and Ninja. The cloud image has them.
- The `noctis-runtime` repository attached to the project. Nyx has it as a submodule, so `mist install` fails with
  "could not read Username" when the session cannot reach it.
- No window or GPU. `engine/tools/run-tests.sh` sets `SDL_VIDEO_DRIVER=offscreen` when there is no display, the
  committed `engine/shaders/compiled/` files are used as they are, and the GPU suite passed without `mesa-vulkan-drivers`.
  Install it (with `openbox`) only when a screenshot or a real window is needed.

## Running tests

- `mist run test` runs every suite under `engine/tests/` in about 12 seconds once built.
- `mist run test --filter TEXT` printed nothing in a trial run, so don't rely on it.
- `engine/tools/run-one-test.sh TEST-NAME` builds and runs one test program (for example `the-watcher-room`) directly,
  and prints the failing assertion's file, line and message, which `nox test` hides behind "it exited abort".

## Known limits

- Metal output and real GPU frame times can't be checked here.
- Windows and Wayland can't be checked here.
- A change to `mist.json` or `mist.lock` needs `mist install` again.
