# The camera

The game is a grounded action-adventure in the spirit of The Last of Us, without combat, so the camera is meant to read
like a film camera. It follows smoothly, leads where you are heading, sits over a shoulder, never looks through a wall,
and a level designer can change how it frames a place or hand it to an authored shot.

All of it is game code in Nyx under `games/platformer/`. The engine's part is two functions: `lantern:::camera:::LookAt`
and `lantern:::camera:::SetFieldOfView`.

## What you see

| Situation | The camera |
| --- | --- |
| Walking | Behind you and over your right shoulder, so you stand a little left of the middle and the middle of the screen shows what is ahead. It trails you by a fraction of a second sideways and more lazily up and down, so a jump lifts the view less than it lifts you. |
| Running | It leads: the point it looks at moves ahead of you in the direction you are going, up to 1.2 m, and eases back when you stop or turn. |
| A wall beside you | The camera swaps to the other shoulder, easing across. It goes back after the first side has been clear for 0.8 s. |
| A wall behind you | The lens is pulled in so it stays in front of the wall (it was already so), and eases back out at 5 m/s when the wall goes. |
| A camera volume | The framing eases to the volume's: closer in an alley, further back and wider on open ground. |
| A camera volume with a shot | The camera glides or cuts to a fixed viewpoint a designer set up, and glides back when you leave. |
| `C`, photo mode | The lens glides to your eyes over a fifth of a second, and back when you lower it. |

## The pieces

| Piece | Where |
| --- | --- |
| The follow rig: smoothing, look-ahead, shoulder, collision | `game-systems/third-person-camera.nyx` |
| The framing numbers (distance, shoulder, height, pitch offset, lens, look-ahead) | `game-systems/camera-rig-settings.nyx` |
| A pose: eye, target, lens, and how two poses blend | `game-systems/camera-pose.nyx` |
| The director: picks the volume and shot, blends, is the only caller of `lantern:::camera` | `game-systems/camera-director.nyx` |
| Frame-rate independent easing used by all of them | `game-systems/smoothing.nyx` |
| The `camera_volume` entity | `game-entities/camera-volume.nyx` |
| The `camera_shot` entity | `game-entities/camera-shot.nyx` |
| Placing them in the world editor | `assets/editor/palette.json` |
| A room that shows all of it | `assets/worlds/prototypes/the-camera-room.world.json` (Prototypes, number 11) |

### The frame, step by step

1. The director finds which camera volume the player stands in. The highest `priority` wins; between equals, the nearest
   centre.
2. The follow settings ease toward that volume's, or back to the defaults outside every volume. The easing has a half-life
   of a quarter of the volume's `blend`, so the change is about done after `blend` seconds, at any frame rate.
3. The follow rig works out its pose with those settings. It does this every frame, even while a shot is on screen, so
   its smoothing is warm and the way back is a glide.
4. The pose to show is the follow pose, the volume's shot, or the photo lens.
5. When what is showing has just changed, the shown pose blends from the pose that was on screen to the new one over the
   volume's `blend` seconds with a smooth-step curve. The new pose is live, so a moving target is tracked through the blend
   instead of a frozen copy. A blend of 0 is a cut.
6. The director calls `SetFieldOfView` and `LookAt`.

## Authoring

Place **Camera volume** and **Camera shot** from the world editor's palette. Every property is optional.

### `camera_volume`

An axis-aligned box around the entity's position (turning the entity does not turn the box).

| Property | Meaning | Default |
| --- | --- | --- |
| `half_extents` | Half the box's width, height and depth, in metres | 4, 3, 4 |
| `priority` | The highest wins where volumes overlap | 0 |
| `blend` | Seconds to move to this framing or shot on entering; 0 is a cut | 1.2 |
| `exit_blend` | Seconds to come back to the default on leaving | the same as `blend` |
| `shot` | The `number` of the `camera_shot` to show while inside; 0 shows none | 0 |
| `distance` | Metres from the pivot back to the lens | 6 |
| `shoulder` | Metres the pivot sits to the player's right; negative is left; 0 is dead behind | 0.55 |
| `height` | Metres above the feet the camera aims at | 1.4 |
| `pitch_offset_degrees` | Added to the pitch the player has chosen; negative lifts the camera | 0 |
| `fov` | The lens, vertical field of view in degrees | 60 |
| `look_ahead` | The most, in metres, the camera leads the player; 0 turns it off | 1.2 |

The player's chest (0.9 m above the feet) is the point that has to be inside the box.

### `camera_shot`

A camera on a tripod at the entity's position.

| Property | Meaning | Default |
| --- | --- | --- |
| `number` | Which shot this is; a volume's `shot` names it | 1 |
| `target` | The world point it looks at | 10 m ahead along -Z |
| `track` | 1 turns the camera to follow the player (like a camera on a pole); 0 keeps it on `target` | 0 |
| `fov` | The lens in degrees | 50 |

A volume and a shot are linked by number because an entity can't look another one up by name.

### Tips

- A volume that shows a shot should be large enough that the player is already in it before they could see what the shot
  hides, and its `exit_blend` should be a glide: a cut in and a glide out is the usual shape.
- Put a shot where nothing solid is between it and the player. Shots do not collide: they are where the designer put them.
- Give a shot volume a higher `priority` than any follow-framing volume it overlaps.

## Moving through a cut

The stick means "away from the camera". If the camera cuts to a view from the front, the stick would flip under the
player's thumb. The director avoids that: when what is on screen changes while the stick is pushed, it keeps the heading
the player had until the stick is let go, and only then turns to the new view. While a shot is on screen, look input does
nothing, because it would turn a camera that is not showing. Inside a shot, then, the controls are relative to the shot.

## Photo mode

Raising the camera is one more thing the director shows. `PhotoMode` still works out the lens (the player's eyes, along the
player's own look) and judges the shot against it; the director blends it in over 0.2 s and back out when the camera is
lowered. The verdict is judged against the lens at rest, so a photo taken in the first fifth of a second is judged against
where the lens is about to be.

## API

| Call | |
| --- | --- |
| `lantern:::camera:::LookAt(eye, target)` | Places the camera. Keeps the lens last set. |
| `lantern:::camera:::SetFieldOfView(degrees)` | The lens, 10 to 120 degrees top to bottom (60 by default). |

## Debug HUD (F1)

`Camera/Mode` (`follow`, `shot N` or `photo`), `Camera/Volume` (the index of the volume the player is in, -1 for none),
`Camera/Blending`, `Camera/Shoulder` (positive right, negative left), `Camera/Distance`, `Camera/FieldOfView` and
`Camera/Eye`.

## The camera room

Prototypes, number 11. One straight path with a yard, a narrow alley (close framing, shoulder swap), a plaza where a
tracking shot watches you cross, open ground (pulled back, wide) and a bridge seen from the side by a camera that cuts in.
`engine/tests/host/the-camera-room` walks it and checks each of those, including that no frame moves the lens more than
0.9 m through a blend.

## What it doesn't do

- The editor draws a volume as a sphere and a shot as a capsule. It does not draw the box or preview the view.
- Shots do not move along a rail, shake, or change focus.
- A shot's pose is not collision checked.
- The fixed numbers (half-lives, look-ahead, the shoulder swap thresholds) are guesses from reading other games, and want
  playing.
