# Testing in a cloud session

A cloud session can run the whole suite with `mist run test`, so a game change can be tested before it reaches a Mac.

| Path | Time on 4 cores |
| --- | --- |
| Building from source (Nox 2m17, Mist 0m41, `mist install` 5m23) | about 8m25 |
| Nox and Mist release downloads, Mist cache seeded from the Lantern bundle | about 1m30 |
| Nox and Mist release downloads, no cache | about 5m |
| `mist run test` once set up (suites, then a build of the world editor) | about 15s |

Threads should use the default path. The source build is the fallback when a download or checksum fails, or when the
session isn't Linux x86-64.

## Bootstrap

```sh
engine/tools/cloud-bootstrap.sh
export PATH="$HOME/.local/bin:$PATH"
mist run test
```

The script is safe to run again. It does three things:

1. Downloads `nox-linux-x86_64.tar.gz` from the `proto-latest` release of `nox-systems-language` and
   `mist-linux-x86_64.tar.gz` from the `main-latest` release of `mist-package-manager`, checks each `.sha256` and
   unpacks them into `~/.local`. A merge of `staging` into the release branch (or Run workflow on `staging`) publishes
   those releases, so a Nox or Mist change reaches cloud sessions without anyone repacking. When a download, a checksum
   or the platform fails, it builds from source instead (below).
2. Seeds Mist's cache from the Lantern bundle (below), unless `~/.mist/cache/sources` already exists.
3. Runs `mist install` in Lantern, which builds SDL3, SDL3_image, SDL3_ttf, cgltf, stb_image and `lantern-native`
   (which builds Nyx and Chaos), using whatever the cache already holds.

The source build:

1. Points `https://github.com/fearlessgamedev/` at `fearlessgamelabs`, because submodule URLs still use the old
   organisation name.
2. Takes `nox-systems-language` (branch `proto`) and `mist-package-manager` (branch `main`) from beside the Lantern
   checkout, cloning them if missing, and initialises their submodules.
3. Builds and installs `nox` and `nox-lsp` (GCC 14, Release, unity build) and `mist` into `~/.local`.

Settings: `LANTERN_WORKSPACE` (where the sibling checkouts live), `LANTERN_PREFIX` (default `~/.local`),
`LANTERN_JOBS`, `CC`/`CXX` (default `gcc-14` and `g++-14`), `LANTERN_BUILD_FROM_SOURCE=1` (skip the downloads),
`LANTERN_NOX_RELEASE_TAG` and `LANTERN_MIST_RELEASE_TAG` (use another release, for example `v0.1.0`).

## The Mist cache bundle

The compiled SDL, cgltf and `lantern-native` builds are most of the time a cold `mist install` takes, and they don't
come with the Nox and Mist releases. `lantern-cloud-toolchain.tar.gz` (and its `.sha256`), the release tagged
`lantern-cloud-toolchain` in `fearless-hq`, holds Mist's `builds/` and `sources/` caches as they were after a
bootstrap. The bootstrap takes only the caches from it: the git cache (about 0.8 GB) is left out on purpose, because
`lantern-native` is a path package that CMake rebuilds from the files' modification times, so `sources/` has to come
back with its times intact.

The bundle records the home, Lantern and Mist cache directories it was built in. The whole cache is used only when the
cache directory matches this session's; otherwise only `sources/` is taken. Builds keyed to another Nox or Mist
version are rebuilt (the bind-c step of `stb_image`, for one), the rest are reused. A failed download only costs time:
`mist install` fetches and builds whatever is missing.

Refresh it after the mist dependencies or `mist.lock` change:

```sh
LANTERN_BUILD_FROM_SOURCE=1 engine/tools/cloud-bootstrap.sh
engine/tools/cloud-toolchain-pack.sh OUTPUT-DIRECTORY
```

then upload the two files it writes as the release's assets. Settings: `LANTERN_BUNDLE_REPOSITORY` (default
`fearless-hq`) and `LANTERN_BUNDLE_TAG`.

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
