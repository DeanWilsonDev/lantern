# Grounded adventure prototypes

Dean's play-test (2026-11-16): the six rooms and the crossing showed that the game has more action-adventure in it than
puzzle-platformer. Think The Last of Us without combat. Movement should be fairly grounded and realistic, and the
challenge should come from places and small games, not from gaps and timing. This page says what was built to test that,
how each piece works, where its numbers live and what has not been run.

Everything here is Nyx game code over the API in [`game-api.md`](game-api.md). The engine changed in no way: the climb is
done by the game placing the player (as the Anchor's swing does), and the grounded feel is a different set of numbers
handed to the same character controller.

**Nothing here has been compiled or run.** The cloud session that wrote it has no `nox` or `mist`, so the code was read
against the existing patterns and the three host tests were written without being run. The first run is Dean's: see
[`playtest-checklist.md`](playtest-checklist.md), "Grounded adventure".

## What changed in the rules

1. **The companion is an equal, not a unit.** The Q, E and F commands are going away in the companion's own rework (another
   thread). These prototypes never ask the player to order her. The places she matters are marked by what a team does:
   she opens a route the player can't use, and the player opens one for her.
2. **A level completes with her beside the player.** The same rework adds this to the goal check; nothing here touches it.
3. **Climbing is the player's alone.** She walks and never climbs or jumps. Every climb has a ground route she can take,
   and a lever on a channel links the two.
4. **Small games are rests, not gates.** Nothing in a level needs a word or a meal.

## Grounded movement

A level asks for it with its spawn's `movement` property set to `"grounded"`. Every other level keeps the platformer feel,
because its gaps were built for a 1.5 m jump.

| What | Platformer | Grounded | Why |
| --- | --- | --- | --- |
| Walk speed | 6.0 m/s | 4.4 m/s | A brisk walk, not a sprint |
| Jump height | 1.5 m | 0.8 m | A person hopping, which clears a crate and not a wall |
| Gravity | 28 m/s² | 22 m/s² | Weighty, and still quick enough that a short hop isn't slow |
| Speed up and slow down | 0.12 s | about 0.4 s on the ground, 1.6 s in the air | Momentum: you can't stop dead or change course in the air |
| After a hard landing | none | 0.35 s at a third of the speed, from a fall faster than 6 m/s | A drop has weight; a hop doesn't |

The momentum is the game easing the direction it hands the controller toward the stick (`Player.GroundedDirection`). The
body still turns toward the stick at once, so the character answers, and then the speed builds.

## Climbing

World kind `climb_surface`: a thin box standing against a wall, with a ledge at the top. Properties are in
`game-entities/climb-surface.nyx`.

| Kind | Speed | Sideways | Used for |
| --- | --- | --- | --- |
| `ladder` | 1.8 m/s | no | The fast way up |
| `pipe` | 1.1 m/s | yes, along its width | The slow way up |
| `ledge` | 1.1 m/s | yes | A wide wall of handholds |

Controls: **F** beside a surface grabs it (a lever within reach still takes F first), **W** and **S** move up and down, **A**
and **D** shimmy on a pipe or ledge, **Space** lets go and drops you a short way out, and reaching the top climbs you up
and over onto the ledge. `from_top` lets you grab from the ledge and climb down over the edge, as at a station platform.

While the player holds a surface the controller isn't stepped at all: `ClimbSystem` says where the feet are and
`Player.SwingTo` puts them there.

**She can't climb.** Her follow behaviour is being reworked; until it lands, a climb is the one place where her trail and
the player's can part. The seam is: put her `team_beat` markers on the ground route and link the routes with a lever and a
gate on one channel. The climb room does exactly that with plain levers and gates today, and its signs say so.

## The prototype rooms and the station

### 8 The climb

One wall, 6.5 m high, three ways up.

| Route | Who | How |
| --- | --- | --- |
| The pipe | The player | Walk up, F, W for about six seconds |
| The ladder | The player | Its bottom rungs are missing: hop onto the crate, jump, and press F in the air |
| The service stair | Both | A locked door at its foot; the lever works from the slot or from the roof |

The slot (0.66 m wide, ten metres long) is too narrow for the player and hides the lever, so only she can open the door
from below. If the player climbs first, a second lever on the roof opens the same door for her.

### 9 The word market

A plaza of eight tags and two stalls. **Words:** walk into a glowing tag and the romaji and its meaning show for four
seconds, and the status line counts them. **Food:** stand beside a stall, hold **F** and keep the stick still; a bar of
bites fills, the meal shrinks with each bite, and letting go pauses it. The first bite says "Itadakimasu" and the last
"Gochisousama deshita".

The engine font is an 8x8 bitmap of ASCII, so kana and kanji can't be drawn yet. The words are in romaji (a card on the
Lantern board, `non-ascii-text-in-game-ui`).

### 10 The station (a full level, about 90 m)

| Beat | Where | What the player does |
| --- | --- | --- |
| Ticket hall | z +12 to -24 | Buy a ticket (F at the machine) and the three gates open; words, a stall, a checkpoint |
| Platform 1 | z -24 to -32 | Choose: the footbridge, or the ladder down to the tracks |
| The tracks | z -32 to -44 | Two trains cross at different times; an island between them is safe; a gem sits on it |
| The footbridge | the left side, y 3 m | Stairs up, a deck over the tracks, stairs down: her way, and the safe way |
| Platform 2 and the exit hall | z -44 to -80 | A ladder up from the tracks, a second stall, the goal |

A train is a `moving_platform` with `train` set. Standing on the track (feet below the platform height) near a train sends
the player back to the last safe spot, like thorns: a nudge, not a death. The ladder shortcut is faster and risks that
nudge; the footbridge is longer and safe.

## Tunables

All are named constants in `games/platformer/settings/adventure-tuning.nyx` (`AdventureTuning`).

| Group | Constants | Feel it changes |
| --- | --- | --- |
| Grounded feel | `GROUNDED_WALK_SPEED`, `GROUNDED_JUMP_HEIGHT`, `GROUNDED_GRAVITY` | Pace and jump |
| Momentum | `GROUNDED_GROUND_RAMP_SECONDS`, `GROUNDED_AIR_RAMP_SECONDS` | How long to reach full speed, and how committed a jump is |
| Landing | `LANDING_RECOVERY_SPEED`, `LANDING_RECOVERY_SECONDS`, `LANDING_RECOVERY_SCALE` | How much a drop costs |
| Climbing | `CLIMB_LADDER_SPEED`, `CLIMB_PIPE_SPEED`, `CLIMB_SHIMMY_SPEED`, `CLIMB_GRAB_REACH`, `CLIMB_TOP_MARGIN`, `CLIMB_LET_GO_PUSH` | How fast and how forgiving a grab is |
| Food | `FOOD_BITE_SECONDS`, `FOOD_DEFAULT_BITES`, `FOOD_REACH` | How long a meal takes |
| Words | `WORD_PICKUP_RADIUS`, `WORD_SHOW_SECONDS` | How easy a tag is to collect |
| Trains | `TRAIN_LANE_MARGIN`, `TRAIN_TRACK_DEPTH` | How close a train counts as in the way |

## Not run, and what to look at first

- The grounded jump: can the player get onto the 0.6 m crate and then grab the ladder in the air? The numbers say yes
  (apex 0.6 + 0.8 = 1.4 m, the ladder's lowest rung is at 1.2 m), but the feel decides.
- Climbing the pipe end to end, and the mount at the top. The mount point lands 0.78 m into the wall on the roof.
- Whether the station's trains pass without trapping the player on the track (a ladder is always within about eight metres).
- Whether eleven prototype entries fit the menu on a small window.
- Her behaviour on the footbridge and in the climb room, which waits for the companion rework.
