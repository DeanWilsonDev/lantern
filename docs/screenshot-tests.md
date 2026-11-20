# Screenshot comparison tests

A screenshot test renders a fixed scene, then checks the picture against a reference PNG kept in the repository. It
catches the changes a pixel probe can't: a lamp that stops glowing, a shadow that goes missing, text that shifts, a
colour grade that drifts. The other GPU tests in `engine/tests/gpu` read back a few pixels; these compare the whole frame.

## Running them

```sh
mist run test            # everything, then the screenshot tests when lavapipe is installed
mist run screenshots     # only the screenshot tests
mist run screenshots -- --update    # record the scenes as the new references
mist run screenshots -- --require   # fail, instead of skipping, when lavapipe isn't installed
```

`engine/tools/run-screenshot-tests.sh` is the script behind the last three. A failure prints how many pixels differ and how
far, and writes two images into `engine/build/screenshot-failures/`:

- `NAME.actual.png`, what the renderer drew;
- `NAME.diff.png`, the capture dimmed to grey with every differing pixel painted red (brighter means further off).

## Why the software renderer

The references belong to one renderer: Mesa's lavapipe, the software Vulkan driver. A real GPU rounds colours differently,
a different driver differently again, and Metal differently from Vulkan, so a picture that is right on a Mac would "fail"
against a reference recorded on Linux. The script points Vulkan at lavapipe alone (`VK_DRIVER_FILES`), so a Linux machine
with a GPU still renders the references' way. When lavapipe isn't installed (`apt-get install mesa-vulkan-drivers`, which
is also what `docs/cloud-testing.md` installs) the script says so and skips. On a Mac the screenshot tests skip, and
`mist run test` stays green; use a Linux machine or a cloud session to check or update references.

The tests only compare when `LANTERN_SCREENSHOT_SOFTWARE=1`, which only the script sets. Run by hand without it, a
screenshot test prints that it skipped and passes.

## The scenes

Each is a test program in `engine/tests/screenshots/`, built from `engine/tests/support/screenshot-scenes/ScreenshotScenes.nox`.
Every number is written out, nothing depends on time or randomness, and each capture is 640 by 360.

| Test and reference | What it covers |
| --- | --- |
| `lit-street` | A sunlit street: the sun with its cascaded shadows, the procedural sky, height fog, lamp posts with emissive globes, a metal ball. |
| `night-look` | The same street set up like the platformer's first chapter: moon, dark sky and fog, lamps that light a wet road with puddles, bloom, exposure, grade and vignette. |
| `shadows` | A low sun, a shadowed point light and a shadowed spot light over a plain floor, with a column, a crate and a ball for each to shadow. |
| `overlay-ui` | Overlay text, a menu panel with a highlighted button, translucent HUD boxes, a gradient triangle and debug lines over a small scene. |

References live in `engine/tests/screenshots/references/NAME.png`. The renderer writes uncompressed PNGs, so each is about
900 KB; `optipng` or similar shrinks one without changing a pixel.

## When a comparison fails

1. Open `NAME.actual.png` and `NAME.diff.png`. The red marks say where.
2. If the change is a bug, fix it.
3. If the picture changed on purpose (a new default, a retuned light), record it with `mist run screenshots -- --update`,
   look at every changed reference, and commit them with the change that caused them. A reference that changes is a
   changed look, so don't update blindly.

## How a pixel matches

`lantern:::screenshots:::Check(name, image)` (in `engine/tests/support/screenshots/ScreenshotCompare.nox`) compares red,
green and blue, and ignores alpha. A pixel counts as different when any channel is more than 6 steps (out of 255) off. The
capture matches when at most 0.2% of its pixels differ. The first number hides rounding between Mesa versions; the second
lets a few edge pixels move without hiding a missing lamp or shadow. A scene that needs other numbers calls
`CheckWith(name, image, channelTolerance, differingShare)`. The comparison itself is tested without a GPU in
`engine/tests/renderer/the-screenshot-compare-accepts-small-differences-and-flags-large-ones`.

## Adding a scene

1. Write a function in `ScreenshotScenes.nox` that returns a `RenderWorld`, using `SceneAssets`' meshes (or add a mesh to
   `LoadAssets`).
2. Copy one of the test folders in `engine/tests/screenshots/` (`nox.json`, `main.nox`, `expected.txt`), and change the scene
   function and the reference name in `main.nox`.
3. `mist run screenshots -- --update`, look at the new reference, and commit it.

## Settings

| Variable | Meaning |
| --- | --- |
| `LANTERN_SCREENSHOT_SOFTWARE=1` | Compare (or record). Set by the script. |
| `LANTERN_UPDATE_SCREENSHOTS=1` | Write the references instead of checking them. Set by `--update`. |
| `LANTERN_SCREENSHOT_REFERENCES` | The folder of reference PNGs. |
| `LANTERN_SCREENSHOT_OUTPUT` | The folder failed comparisons write their images into. |
