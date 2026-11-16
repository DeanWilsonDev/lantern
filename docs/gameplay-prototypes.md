# Gameplay prototypes

Dean's worry (2026-11-16): the Crimson and Thorn loop of walking, jumping and being helped by a companion may not be fun
enough by itself, and if it isn't, the game has to lean hard on environment and story. This page is the menu of
ideas that might make the loop engaging, which of them are built as playable rooms, and how to read the results.
Playing and judging each room is the point; [`playtest-checklist.md`](playtest-checklist.md) says what to try and note.

Everything here is Nyx game code over the API in [`game-api.md`](game-api.md). Where the engine can't do something,
the workaround is named in the code and the gap is a card on `engine-follow-ups`.

> After the first play-test the direction moved toward an action adventure with grounded movement; the newer prototypes
> (the climb, the word market and the station) are in [`grounded-adventure.md`](grounded-adventure.md).

## The rules every idea follows

1. **Solvable alone.** The companion makes a puzzle shorter, safer or more interesting; she never gates progress. A solo
   route is longer, riskier or needs more platforming skill.
2. **The player commands her with a few keys, not menus.** The command model is below.
3. **Light and shadow carry meaning.** Ideas prefer the engine's real lights, shadows and raycasts to invented flags.
4. **Tunables are named constants**, listed under each prototype, so feel can be changed in one place.

## The companion's command model

Until now she walked a scripted route (`companion_stop` markers). The new model, used by the prototype rooms, makes her a
character with her own physics body who obeys three commands:

| Command | Keys | What she does |
| --- | --- | --- |
| **Call** | Q, gamepad D-pad left | Comes to you and follows, keeping about 3 m behind. |
| **Wait** | E, gamepad D-pad right | Stays exactly where she is until called. |
| **Send** | F, gamepad West | Walks to the nearest `command_point` marker in front of you and does what the marker says there: stand (hold a plate), crouch (offer the boost), pull (a lever), shine (hold a light), or lure (be seen). She stays there until called. |

States: `Follow`, `Wait`, `GoTo(point)`, `Interact(point)`. She never fights the player: a command is a request, and a
blocked or too-far request is answered by a short head-shake (a status message) rather than silently ignored.
The scripted route (`Guide`) stays for the streets, rooftops and streets-later, where she leads.

## The menu: twelve ideas

Cost is for the engine as it is today: **S** is a day or less of Nyx, **M** a few days, **L** needs engine work.

| # | Idea | Companion use | Solo | Cost | Fun hypothesis | Built |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | **Hold the door**: plates open doors only while something stands on them | Sent to stand on the plate | A timed plate: step on it and run through before the door shuts | S | Splitting up and sending someone ahead is a small, readable act of trust, and the timed version is a good skill test | Yes |
| 2 | **The narrow way**: a slot only she fits through | She crawls through and pulls a lever beyond it | A longer route round with thorns and a moving platform | S | A place only she can go makes her feel like a partner, not a follower | Yes |
| 3 | **Light gate**: a crystal opens when a light has line of sight to it, so a pillar's shadow is the puzzle | Sent to a spot where her light reaches the crystal | Carry the lantern into view of the crystal and run to the gate before it shuts | M | Using real light and shadow as the puzzle is the game's best fit with the engine | Yes |
| 4 | **Hand-off**: boost up on command, then you open the way for her | Crouches where sent, then follows through the door you open | A stair of crates and a hard jump | S | Each of you needs the other in turn, which is the story's core | Yes |
| 5 | **The watcher**: a sweeping light that sends you back | Sent to hold its attention while you cross | Time the sweep and hide behind pillars | M | Tension and a different pace, and the one idea where stopping is the skill | Yes |
| 6 | **Flow and fragile**: bounce pads and crumbling platforms | She leads the line and waits at the end | Same, without hints | S | Variation in what the character is doing; a run that wants momentum | Yes |
| 7 | **Updraft**: wind columns that lift a glide-less jump | She stands in a vent to switch it on | A timed vent | M | Vertical traversal for a change | No |
| 8 | **Throw and catch**: throw her the lantern to light a far crystal | Catches and holds it | Climb with it instead | L | Throwing is a strong verb, but the engine has no carryable bodies | No |
| 9 | **Rope chain**: anchors in a row to swing between | She holds the far end taut so the swing is longer | The short swing of today | M | Builds on the Anchor; needs a feel pass first | No |
| 10 | **Whistle and scout**: she runs ahead and reveals a trapped route by standing in it | Sent along a path of hidden platforms | Lantern radius reveals them one at a time | M | Trust: you cross where she stood | Part of 3 |
| 11 | **Carry across**: she carries you over a gap too wide to jump | Hold interact to be carried | A long way round | L | A big emotional beat, but needs her to move the player | No |
| 12 | **Rhythm route**: platforms and lights pulse in time and the route is a beat pattern | Counts the beat aloud with a light pulse | Read the pulse yourself | M | Flow, and good with audio, but audio layers are not built | No |

## What was built and why

Prototypes 1 to 5 each use a different verb (stand, reach, aim a light, hand over, sneak) and reuse something the engine
already proves: the kinematic collision trick the glow platforms use, the raycast, the thorns, the boost and the spot
light. Prototype 6 is variety for the character rather than a companion mechanic. Ideas 7 to 12 need engine work (carry,
throw, push) or audio that isn't there; they stay on the menu for a second round.

## The prototype rooms

The main menu has a **Prototypes** button listing one room per idea, plus **The crossing**, a full level that uses them
all. Each room is one world file in `assets/worlds/prototypes/`, small and isolated, with signs that say what to try.
Every room has a **solo route** and a **companion route**; [`playtest-checklist.md`](playtest-checklist.md) asks you to
try both. Every room has a host test in `engine/tests/host/` that plays it with scripted keys.

| Room | File | Verbs | World kinds it uses |
| --- | --- | --- | --- |
| 1 Hold the door | `hold-the-door` | send her to a plate; or run a timed plate | `plate`, `gate`, `command_point` |
| 2 The narrow way | `the-narrow-way` | send her through a slot to pull a lever; or climb the ledges | `lever`, `gate`, `command_point` (action `pull`) |
| 3 Light gate | `light-gate` | send her light to a crystal; or carry the lantern | `crystal`, `gate`, `lantern` |
| 4 Hand-off | `hand-off` | she crouches for a boost, you lower a stair for her | `command_point` (`crouch`), `lever`, `plate` |
| 5 The watcher | `the-watcher` | hide behind pillars; or send her to lure the light | `watcher`, `command_point` (`lure`) |
| 6 Flow and fragile | `flow-and-fragile` | bounce pads and crumbling slabs | `bounce_pad`, `crumble` |

## The crossing: a fuller slice

`worlds/the-crossing.world.json` is a chapter 2 level of about 210 m that plays after the rooftops and leads on to
`streets-later` (the rooftops' goal points at it). It is also **7** in the Prototypes list. It keeps the lamps and look of
the night chapters and adds nothing to `settings/chapter-looks.nyx`.

The pacing is one beat of about 25 to 35 m at a time, so the character does something different each time:

| Beat | z (metres) | What the player does | Rest after |
| --- | --- | --- | --- |
| 1 Flow | 0 to -30 | Walk, then a bounce pad to a ledge of gems | Checkpoint on open ground |
| 2 Puzzle | -30 to -66 | Hold the door: send her to the plate, or run the timed one | Checkpoint past the door |
| 3 Hazard | -66 to -118 | Crumbling slabs, a thorn patch, a pad over more slabs | Checkpoints on the landing and the island |
| 4 Companion | -118 to -134 | The narrow way: she pulls the lever behind the slot, or climb the ledges | Checkpoint before the door |
| 5 Stealth | -134 to -190 | The watcher hall: cover behind pillars, or the lure | A checkpoint behind each pillar |
| Vista | -190 to -208 | Four steps up to a terrace with a sign, two gems and the goal | The end |

Every beat has a gem or two off the main line, and the beats are separated by open ground, so a photo sight can go on
any of the rests (the terrace and the pad ledge are the obvious two). The streets, rooftops and `streets-later` are
unchanged apart from the rooftops' next level; `streets-later` still ends the chapter.

## Tunables

All are named constants in `games/platformer/settings/partner-tuning.nyx` (`PartnerTuning`), except the boost height,
which stays in `player-tuning.nyx`.

| Group | Constant | Value | Feel it changes |
| --- | --- | --- | --- |
| Her body | `RADIUS`, `HEIGHT` | 0.24 m, 1.5 m | The slot is 0.66 m wide: she fits, the player (radius 0.35) doesn't |
| Her walk | `WALK_SPEED` | 5.0 m/s | How long a send takes |
| Following | `FOLLOW_DISTANCE`, `CATCH_UP_DISTANCE` | 3.0 m, 28 m | How close she stays; beyond 28 m she is put behind you |
| Trail | `TRAIL_SPACING`, `TRAIL_MAX_POINTS`, `TRAIL_DROP_POINTS`, `TRAIL_REACHED_DISTANCE` | 1.0 m, 240, 80, 0.6 m | How exactly she retraces your route |
| Arriving | `ARRIVE_DISTANCE` | 0.35 m | How near a command point she must stand |
| Sending | `SEND_RANGE`, `SEND_FACING_DOT` | 24 m, 0.2 | How far and how directly you must face a point |
| Stuck | `STUCK_SECONDS`, `STUCK_SPEED` | 2.5 s, 0.4 m/s | When she gives up ("She can't get there") |
| Crouch | `CROUCH_SCALE` | 0.6 | How low her boost pose is |
| Levers | `LEVER_REACH`, `PARTNER_LEVER_REACH` | 2.2 m, 1.6 m | How near you must stand to pull |
| Plates | `PLATE_HEIGHT_BELOW`, `PLATE_HEIGHT_ABOVE` | 0.3 m, 0.7 m | The feet band that counts as standing on one |
| Gates | `GATE_SPEED`, `GATE_BLOCK_MARGIN` | 7.0 m/s, 0.5 m | How fast doors move; a door never shuts on someone within the margin |
| Crystals | `CRYSTAL_DEFAULT_RANGE`, `CRYSTAL_DEFAULT_HOLD_SECONDS`, `LIGHT_RAY_MARGIN` | 10 m, 4 s, 0.4 m | How far a light opens one, and for how long |
| Signs | `SIGN_DEFAULT_RADIUS` | 6 m | When a hint appears |
| Watcher look | `BEAM_COLOR`, `BEAM_STRENGTH`, `PATCH_GLOW` in `watcher.nyx` | amber, 140, 1.6 | Amber beam, glowing lamp housing and a floor patch the width of the catching cone |
| Watcher | `WATCHER_LURE_SECONDS`, `WATCHER_FORGIVE_SECONDS` | 6 s, 1.5 s | How long the lure holds; grace before being seen counts |
| Bounce | `BOUNCE_DEFAULT_HEIGHT` | 5 m (a pad's `height` property overrides) | How high a pad throws |
| Crumble | `CRUMBLE_DELAY_SECONDS`, `CRUMBLE_RETURN_SECONDS` | 0.6 s, 3 s | How long a slab shivers, and when it returns |
| Boost | `BOOST_JUMP_HEIGHT` | 3.6 m | Unchanged |

Timed plates and levers carry their own `hold_seconds` in the world file (4 s in the rooms).
