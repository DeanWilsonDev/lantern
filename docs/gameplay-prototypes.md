# Gameplay prototypes

Dean's worry (2026-11-16): the Crimson and Thorn loop of walking, jumping and being helped by a companion may not be fun
enough by itself, and if it isn't, the game has to lean hard on environment and story. This page is the menu of
ideas that might make the loop engaging, which of them are built as playable rooms, and how to read the results.
Playing and judging each room is the point; [`playtest-checklist.md`](playtest-checklist.md) says what to try and note.

Everything here is Nyx game code over the API in [`game-api.md`](game-api.md). Where the engine can't do something,
the workaround is named in the code and the gap is a card on `engine-follow-ups`.

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
| **Call** | Q, gamepad West | Comes to you and follows, keeping about 3 m behind. |
| **Wait** | E, gamepad East | Stays exactly where she is until called. |
| **Send** | F, gamepad North... see `input-settings.nyx` | Walks to the nearest `command_point` marker in front of you and does what the marker says there: stand (hold a plate), crouch (offer the boost), pull (a lever), shine (hold a light), or lure (be seen). She stays there until called. |

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
| 6 | **Flow and fragile**: bounce pads and crumbling platforms | She leads the line and waits at the end | Same, without hints | S | Variation in what the character is doing; a run that wants momentum | Yes (stretch) |
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

The main menu has a **Prototypes** button listing one room per idea. Each room is one world file in
`assets/worlds/prototypes/`, small and isolated, with a start area that says (by layout, with a short on-screen line) what to
try. Every room has a **solo route** and a **companion route**; the checklist asks you to try both.

## Tunables

Filled in per prototype as each is built (constants live in `games/platformer/settings/`):
see `playtest-checklist.md` for the list with values.
