# The night look

The platformer is played at night. This guide explains why, which pieces make a night readable, and where each number lives.
For how the renderer lights a frame, read `lighting-and-shadows.md`; for the sky and fog, `sky-and-image-based-lighting.md`
and `fog-and-light-shafts.md`.

## Why night

Lantern's mechanics are about light: the companion's glow and the lantern reveal hidden platforms where they reach. In
full daylight that is invisible. The platform appears, but nothing shows *why*, because everything was already lit. At night
the moon is dim, so the light you carry is the light you see by, and a platform appearing at the edge of a glow reads as
the glow reaching it.

## The pieces

| Piece | What it does | Where |
| --- | --- | --- |
| The moon | The one light that shines everywhere, low in the sky ahead of the player. Its colour is how bright the night is. It casts the long shadows. | `settings/chapter-looks.nyx`, direction in `settings/lighting-settings.nyx` |
| Sky and fog | A dark sky whose horizon colour also clears the screen and colours the fog, so the far distance fades into the sky. The sky is also the ambient light. | `settings/chapter-looks.nyx` |
| The picture | Exposure, bloom (lower threshold, so lamps glow), and a colour grade with a vignette. | `settings/chapter-looks.nyx` |
| Guide lights | The companion's warm light and the player's cooler lantern. | `game-entities/guide-light.nyx` |
| Night lights | Street lamps, checkpoint posts, the goal, the lantern pickup and lit windows. | `game-systems/night-lights.nyx` |
| Gems and anchor caps | Emissive, so they stay bright in shadow and bloom. | `game-entities/gem.nyx`, `game-entities/anchor.nyx` |

## Chapters are different nights

A level says which chapter it is on its spawn entity, and `ChapterLooks` turns the number into colours, as it did for the
day:

- Chapter 1: a quiet blue night.
- Chapter 2: a warmer, violet night with more colour (saturation 1.25).
- Chapter 3: desaturated, the fog closer.
- Chapter 5: the darkest and coldest, the fog nearly on top of you.

## Keeping it readable

A night level must still let the player see the next jump. The numbers are chosen so that:

- the moon and the sky's ambient light keep platform tops and edges visible without any lamp,
- thorns are a dark red against dark ground, so they are the one place a lamp's glow matters most,
- a lamp, a post or the goal is never more than a few jumps from the next, so there is always a bright thing to head for,
- every glowing thing stays under the engine's limit of eight nearby local lights (`renderer.md`): the nearest eight are
  used, so a lamp far away costs nothing.

## The day look

Run the game with `LANTERN_LOOK=day` to see every chapter as it was in daylight. It is a quick way to check that a level's
geometry reads without the lights.

```sh
LANTERN_LOOK=day mist run game
```

## Adding lights to a level

The lights are world entities, so a level author adds them in the editor:

| Kind | What you get | Properties |
| --- | --- | --- |
| `street_lamp` | A post with a lit head and a warm point light. Place it on the ground beside the street. | none |
| `lit_window` | A glowing pane, thin in x, to put on the face of a tower. No light of its own. | none |
| `point_light` | The engine's own light, for anything else (see `lighting-and-shadows.md`). | `color`, `strength`, `range` |

Checkpoint posts, the goal and the lantern pickup get their lights from the game, because their brightness follows their
state.
