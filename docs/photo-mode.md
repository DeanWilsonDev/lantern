# Photo mode

Photo mode is a second kind of collectable. Raise the camera, frame a sight, take the photo. Nothing is locked behind it:
the level can be finished without ever lifting the camera, and a photo counts once per sight per run of the level.

## Playing it

| Input | Does |
| --- | --- |
| `C` or left shoulder | Raises the camera to the player's eyes, or lowers it. |
| Mouse or right stick | Turns the camera (a little slower than the ordinary camera, to line a shot up). |
| `Enter`, `Space`, a click or right shoulder | Takes the photo. |
| `Esc` | Lowers the camera and pauses. |

While the camera is raised the world keeps running (the companion walks on) but the player stands still. A small glowing
diamond hangs at each sight so you can find them with the camera down; they hide while the camera is up, so they are never
in the photo.

The status line shows `Photos 1 / 5`, and the level complete screen lists the title of each photo taken.

## What counts as a photo

A photo counts a sight when all of these are true:

1. **In frame.** The sight is inside the bordered rectangle of the viewfinder.
2. **Close enough.** The lens is no farther from it than the sight's `max_distance`.
3. **In the clear.** A ray from the lens reaches the sight without meeting anything solid first (the last 0.75 m in
   front of a sight don't count, so a sight on a wall isn't blocked by the wall).
4. **Together, when it asks.** A sight with `together` also needs the companion: her chest inside the frame, in the
   clear, and standing within 7 m of the sight, so the photo is of the two of you *there*. A level with no companion
   ignores `together`.

If two sights are in frame, the one nearest the middle is the one judged. The ring in the middle turns green when a photo
taken now would count, and the line under the frame says what is missing (`too far`, `something is in the way`, `she has
to be in the shot too`).

## How it is built

| Piece | Where |
| --- | --- |
| The sight entity: reads `title`, `together`, `max_distance`; owns the marker | `game-entities/photo-sight.nyx` |
| The verdict the camera gives | `game-entities/photo-verdict.nyx` |
| The camera, the framing maths and the counting | `game-systems/photo-mode.nyx` |
| Blending the lens in and out | `game-systems/camera-director.nyx` (see [`camera.md`](camera.md)) |
| The viewfinder, the flash and the album line | `ui/photo-hud.chaos`, `ui/photo-hud.cosmos` |
| What the UI reads | `ui/game-ui-model.nyx` (`isPhotoMode`, `isPhotoReady`, `photoHint`, `isPhotoFlash`, `album`) |
| The actions | `settings/input-settings.nyx` (`PHOTO`, `SHUTTER`) |

The framing test is a pinhole camera. A point's offset from the lens is split along the camera's forward, right and up;
dividing right and up by the forward distance, and then by the field of view's tangent (and the aspect ratio for
right), gives a position from -1 to 1 across the screen. The viewfinder frame is 8% in from every edge, so the limit is
0.84. The field of view (60 degrees) is the renderer's `Camera` default; the game does not read it back, so it is written
in `photo-mode.nyx` too.

The engine needs nothing new for photo mode. Raycasts, the camera, the HUD and the world's properties already existed; photo mode is
game code, apart from one function. Every press of the shutter, counted or not, calls
`lantern:::screen:::SavePhoto("photos", "photo")`. After the frame is rendered the host clears the overlay (so the viewfinder,
flash and HUD are not in the picture), draws the frame again and writes it as `photos/photo-0001.png`, taking the first
number no file has. The folder is relative to where the game was started.

## Adding a sight

In the world editor, place a **Photo sight** where the camera should be aimed and give it properties:

| Property | Meaning | Default |
| --- | --- | --- |
| `title` | The name in the album | `A sight` |
| `together` | 1 when the companion has to be in the shot | 0 |
| `max_distance` | How close the camera has to be, in metres | 30 |

Put it in the open (more than 0.75 m from any wall) and where the player can get the view: a sight no one can see is a
sight no one will photograph. For a `together` sight, put it where she waits (a boost stop holds her until the boost is
used, so it is a reliable place).

## Taking screenshots of it

The game can't send itself key presses, so for a repeatable capture run it with `LANTERN_PHOTO_DEMO=1` (and
`LANTERN_AUTOPLAY=1` to start playing): the camera is raised on the third frame of play and the shutter pressed on the
twelfth. Add `LANTERN_LEVEL=worlds/streets.world.json`, `LANTERN_MAX_FRAMES=N` and `LANTERN_CAPTURE_PATH=file.png` to stop on
frame N and save it.

## Tested

`engine/tests/host/a-photo-of-a-sight-counts-once` plays the streets from the menu, raises the camera, checks the verdict is
`ready` for the first street lamp, takes the photo, and checks it counts once and the camera lowers again. The debug HUD
(F1) shows `Photo/Camera raised`, `Photo/Verdict` and `Photo/Photos`.
