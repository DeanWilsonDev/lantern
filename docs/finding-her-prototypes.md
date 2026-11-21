# Finding Her prototypes

What is built for the direction in [`finding-her.md`](finding-her.md), how to play it and what to note. Everything here is
Nyx game code over [`game-api.md`](game-api.md); there is no engine change. Decisions are D-237 to D-247 in
[`decisions.md`](decisions.md). Nothing here was played on a Mac: the host tests ran in the cloud and the screenshots were
drawn headless on a software GPU.

## How to play

From the main menu choose Prototypes, then **13 Finding her**:

| Room | File | What it is |
| --- | --- | --- |
| 1 The festival, chapter 2 | `prototypes/finding-her-festival-chapter-2` | Walk in beside her, hold her hand, the crowd pulls you apart, find her |
| 2 The festival, chapter 5 | `prototypes/finding-her-festival-chapter-5` | The same space; nobody answers; it ends when you stop |
| 3 The rose garden | `prototypes/finding-her-rose-garden` | Chapter 4: thorns between you, the first call nobody answers |
| 4 The cable car | `prototypes/finding-her-cable-car` | Chapter 3: hold her hand while the car climbs; it slips away |
| 5 The lantern walk | `prototypes/finding-her-lantern-walk` | Chapter 5: the lantern is the only light, calling does nothing |

| Input | Key | Gamepad | What it does |
| --- | --- | --- | --- |
| Walk | WASD / arrows | left stick | As everywhere; the pace is slower in later chapters |
| Call | Q | East | A short sound; she answers after the chapter's delay, from where she is, with a pulse of light |
| Look | hold E | right trigger | You stop, the lens narrows, the crowd's sound ducks, her light reads stronger |
| Hold hands | hold G | left trigger | She walks beside you at your pace; a thin glowing thread joins you; the crowd can break it |
| Do (the meal) | hold F | West | As in the word market: bite by bite |
| Photo | C | left shoulder | As before; the festival's sight counts only with her in frame |

Step onto a crate (hop beside it) and the camera lifts and widens; every few seconds a line says how far away she is and
which side.

## The festival in chapter 2

1. **Together.** You start at the south end with her beside you. Passing the takoyaki stall, a line says she slows by it:
   that is the place to remember. Hold G and she keeps your pace; let go and she keeps about two metres off.
2. **Separated.** Reaching the marker in the middle of the plaza, the crowd surges (it runs 2.2 times faster for 3 seconds),
   her hand is gone and she is carried off, laughing.
3. **Searching.** She waits behind the stall you passed. Call, look, climb a crate for a view over the crowd, follow the
   petals she dropped (pink particles where she walked), or just remember the stall.
4. **Found.** Within 3.2 m of her and she runs to you; when she is beside you the level is complete.

The meal (taiyaki, three bites, she steals the last piece) and the photo of the two of you under the lantern strings are in
the same plaza and are optional.

## The festival in chapter 5

Same plaza and crowd, colder and darker, and nobody. The call has no answer. The places she liked (the stall, the cat, the
bench) show a line when you reach them, now as places. The photo sight has no one under it and the meal is one bowl. The level
ends once you have called at least three times, visited a place she liked, and then stand still without calling for six
seconds. The exit marker at the north end is the other ending (see the switches).

## Where the code is

| Piece | File |
| --- | --- |
| The four beats, call, look, hand-holding, slip, endings | `games/platformer/game-systems/search-system.nyx` |
| The crowd, vantage spots, hiding places, memory spots, per-level switches | `game-entities/search-site.nyx`, `crowd-figure.nyx`, `memory-spot.nyx` |
| Every number, and the chapter mood | `settings/finding-her-tuning.nyx` |
| Walking her where the search says | `Partner.Direct` in `game-entities/partner.nyx` |
| Camera look and vantage | `CameraDirector.SetSearchLens` |
| Walk pace and posture by chapter | `Player.SetMood`, `CharacterAnimator.SetBodyScale` |

World kinds the search reads: `crowd_lane` (`end`, `count`), `vantage_spot`, `her_spot` (`order`), `memory_spot` (`label`,
`note_together`, `note_alone`, `is_exit`), `separation`, `hand_slip` (`radius`, `seconds`), and the spawn's `search_phase`,
`her_answers`, `her_glow`, `ends_on`, `ending_line`, `ending_calls`. Every room has a host test in `engine/tests/host/`.

## The doc's open questions

Each is taken at the doc's leaning and is a single switch in `settings/finding-her-tuning.nyx` (D-245). They are for Dean:

1. The call: a wordless low tone for now (`CALL_SOUND`, `CALL_PITCH`).
2. Chapter 5's ending: stop calling and stand still (`CHAPTER_FIVE_ENDS_ON_STILLNESS`); the exit is the alternative.
3. How literal the crowd is: abstract faceless figures (`FIGURE_TINT`, `FIGURE_WIDTH`).
4. Both chapter 2 vignettes kept; the meal goes first if the budget needs cutting.

## Not built, and known gaps

- Crowd instancing: 68 figures are 68 entities. Fine on the software renderer's test; a bigger crowd needs engine instancing.
- An additive pose layer for posture: the chapter slump is a lower body height for now.
- A jog while she is lost (the constant exists, nothing reads it), traces other than petals (glow on walls, heads turning),
  an echo of the call in fog, and the blue-door ghost answer.
- Seated clips for the cable car and the meal: the two of you stand.
- A real call sound and positioned sound loops (Cinder); the answer is a generated placeholder at two pitches.
- Her hiding place is fixed; she does not move on while you search, and the crowd does not turn its head toward her.
- Not played on a Mac or on Wayland and Windows; none of the numbers are tuned by feel.
