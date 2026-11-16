# Testing in a cloud session

A cloud session can run the whole suite with `mist run test`, so a game change can be tested before it reaches a Mac.

| Path | Time on 4 cores |
| --- | --- |
| Building from source (Nox 2m17, Mist 0m41, `mist install` 5m23) | about 8m25 |
| Restoring the prebuilt toolchain | about 13s, plus the download of about 150 MB |
| `mist run test` once set up | about 16s |

Threads should use the prebuilt toolchain. The source build is the fallback when no prebuilt one exists or it does
not fit the checkout's layout.

## Bootstrap

```sh
engine/tools/cloud-bootstrap.sh
export PATH="$HOME/.local/bin:$PATH"
mist run test
```

The script is safe to run again. It first tries the prebuilt toolchain (below), then falls back to building from
source. The source build:

1. Points `https://github.com/fearlessgamedev/` at `fearlessgamelabs`, because submodule URLs still use the old
   organisation name.
2. Takes `nox-systems-language` (branch `proto`) and `mist-package-manager` (branch `main`) from beside the Lantern
   checkout, cloning them if missing, and initialises their submodules.
3. Builds and installs `nox` and `nox-lsp` (GCC 14, Release, unity build) and `mist` into `~/.local`.
4. Runs `mist install` in Lantern, which builds SDL3, SDL3_image, SDL3_ttf, cgltf, stb_image and `lantern-native`
   (which builds Nyx and Chaos).

Settings: `LANTERN_WORKSPACE` (where the sibling checkouts live), `LANTERN_PREFIX` (default `~/.local`),
`LANTERN_JOBS`, and `CC`/`CXX` (default `gcc-14` and `g++-14`).

## The prebuilt toolchain

`cloud-bootstrap.sh` downloads `lantern-cloud-toolchain.tar.gz` (and its `.sha256`) from the release tagged
`lantern-cloud-toolchain` in `LANTERN_TOOLCHAIN_REPOSITORY`, checks the checksum and unpacks it over `/`. It holds
`nox`, `nox-lsp` and `mist`, plus Mist's `builds/` and `sources/` caches, so `mist install` finds the SDL, cgltf and
`lantern-native` builds already done and finishes in seconds. The Mist git cache (about 0.8 GB) is left out on purpose:
`lantern-native` is a path package that CMake rebuilds from the files' modification times, so the cache's `sources/`
has to come back with its times intact, and the git cache is only needed when `mist.lock` changes.

The bundle records the Nox and Mist commits it was built from, the hash of `mist.lock`, and the home, Lantern and
install directories. The bootstrap uses it only when the three directories match this session, and warns when
`nox-systems-language` `proto` or `mist-package-manager` `main` has moved on. A newer `mist.lock` still works: `mist
install` fetches and builds what changed. When a test needs a newer compiler, update the sibling checkout (`git -C ../nox-systems-language pull --ff-only`) and run with `LANTERN_BUILD_FROM_SOURCE=1`, which rebuilds the checkout as it stands.

Refresh it after Nox, Mist or the mist dependencies change:

```sh
LANTERN_BUILD_FROM_SOURCE=1 engine/tools/cloud-bootstrap.sh
engine/tools/cloud-toolchain-pack.sh OUTPUT-DIRECTORY
```

then upload the two files it writes as the release's assets. Settings: `LANTERN_TOOLCHAIN_REPOSITORY`,
`LANTERN_TOOLCHAIN_TAG`, `LANTERN_TOOLCHAIN_FILE` (use a local archive instead of downloading) and
`LANTERN_BUILD_FROM_SOURCE=1`.

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
