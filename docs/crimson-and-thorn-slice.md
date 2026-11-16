# The Crimson and Thorn slice

The platformer in `games/platformer/` is the demo that shows what the Lantern engine can do, and Dean wants it to grow
towards a full 3D platformer. *Crimson and Thorn* (the design documents in fearless-hq, `projects/crimson-and-thorn`) is
the design source for that growth. It is a design source only: the demo borrows its mechanics and its pacing, not its
story, art or audio. Everything here is Nyx game code over the API in [`game-api.md`](game-api.md); anything the engine
can't do yet is a card on the `engine-follow-ups` epic.

The work is tracked on the board under `platformer-game/crimson-and-thorn-slice`.

## What the design asks for

The design's central idea is that **the companion is a capability the player stops using**. Each of three abilities is
first something the companion does for the player and then something the player learns to do alone, and a level should
teach both forms of the same puzzle. The game is gentle: nothing punishes the player, difficulty is not the point, and
the camera and the level layout explain a puzzle without text.

## Mechanics: now, later, not for the demo

| Mechanic (design source) | Demo | Why |
|---|---|---|
| Core platforming: run, free jump, variable jump height (`mechanics-design`, Core Platforming) | Done | Already the platformer's base (`PlayerTuning`, D-015). |
| Never punish the player (Design Philosophy) | Slice 1 | Checkpoints and a respawn that keeps collected gems; no health. |
| Ability 1, the Boost: companion crouches and the player jumps off her back; solo, a crate does the same | Slice 1 | Needs only a scripted entity, a trigger and a bigger jump. The most readable teaching puzzle. |
| Companion behaviour: scripted, waits for the player, leaves ahead (Companion Behaviour) | Slice 1 | A list of stops from the world file; no pathfinding. |
| Teaching structure: solo first, then cooperative (The Companion Teaching Structure) | Slice 1 | A "streets" level that has the crate route first and the companion route second. |
| Puzzle communication without text (Puzzle Communication) | Slice 1 | Level layout and an authored camera glance at the ledge; no tutorial text. |
| Ability 2, the Guide Light: her glow reveals hidden platforms; solo, a smaller, cooler lantern | Slice 2 | Done. The gameplay is Nyx; the glow is a real point light (`lantern:::lights`). |
| Chapter looks: palette, fog, tone from neutral to saturated to cold (Game Design, Narrative Structure) | Slice 2 | Done: sky, fog and sun per chapter. |
| Thorns from Chapter 3 | Slice 2 | Done: a hazard that moves the player back, never one that ends the run. |
| Ability 3, the Anchor | Slice 3 | Done as a set swing arc between two posts (Dean's choice). The companion's form first, then the solo one. |
| A goal, a win state and a chapter progression | Slice 4 | Done: a gold goal at the end of the streets ends the level and loads the next one. |
| A second chapter-2 level of moving platforms | Slice 5 | Done: `the rooftops`, solo, between the streets and `streets-later`. |
| Authored cameras: static shots, player-locked, player-controlled (Camera) | Later | The third-person camera is the only mode today. A shot system needs its own design. |
| Chapters 1 to 5 as a whole, recurring and unique levels | Later | The slice builds one recurring level (the streets) in two chapter looks. More levels come after the mechanics feel right. |
| Post-completion changes (the blue and red doors, stripped levels) | Later | Needs saving and a finished first playthrough. |
| Companion-driven audio (music layers, her distance as a mix input) | Later | The engine plays one-shots and one music track. Layered music is an audio feature. |
| Save and menu flow, train transitions, documentary | Not for the demo | Product concerns, not engine demos. |

## Slice 1: the streets

Playable end to end with what the engine has today:

1. **Checkpoints.** A `checkpoint` world kind. Touching one stores the respawn point; falling returns the player there and
   keeps the gems already collected.
2. **The companion.** A `companion_stop` world kind with `order` and `wait_seconds`. She leads: she stands at a stop, walks
   to the next when the player is within 10 m of her or has run past her, and waits less in later chapters. If the player
   falls far behind she waits, faces them and hops every few seconds.
3. **The Boost.** A `companion_stop` with `boost` set marks where she crouches. Standing beside her and pressing jump gives a higher
   jump than any normal jump (the exact height is a tuning question the design leaves to prototyping). A first ledge is
   one normal jump too high and has a crate stack beside it (the solo form); a second ledge is only reachable through
   the companion (the cooperative form).
4. **A short level** in `assets/worlds/streets.world.json` that teaches these in order, ending at a goal with gems on the
   way.

### What slice 1 is, in the game

Pick **The streets** on the main menu (or run with `LANTERN_LEVEL=worlds/streets.world.json`). The level is
`assets/worlds/streets.world.json`, authored like the course: plain blocks plus game kinds.

| World kind | Properties | Meaning |
|---|---|---|
| `checkpoint` | `companion_stop` (optional) | A grey post that turns warm when reached. A fall returns the player here with their gems; the companion goes back to the named stop. |
| `companion_stop` | `order`, `wait_seconds`, `boost`, `speed`, `swing`, `near` (all optional but the first two) | One stop of the companion's scripted route. She walks to the next stop when she has waited and the player is within 10 m of her (`near` replaces the 10) or is nearer the next stop than she is, in which case she hurries; at a `boost` stop she crouches and waits until the player has jumped off her back; at a `swing` stop she crosses by swinging the `swing_arc` whose start post is within 2.5 m, then walks on from its far post. |

The route through the level:

1. Walk to the crate and jump onto it, then jump from the crate onto the upper street. The street is 2.2 m up, one
   normal jump (1.5 m) too high, so this is the solo form of the Boost.
2. Walk up the street. The companion, waiting at the first stop, walks ahead once you are within 10 m and crouches against
   the wall at the end, waiting for you there (if you hang back she stops, turns and hops to call you on). The rooftop is 3.2 m above the street.
3. Stand beside her and jump: the jump takes you 3.6 m up (`PlayerTuning:::BOOST_JUMP_HEIGHT`), enough for the roof. She
   then climbs up after you.

Where the code is, one folder per kind of module as in the rest of the game:

| File | Job |
|---|---|
| `game-entities/checkpoint.nyx` | the post and its reached state |
| `game-entities/companion.nyx`, `companion-stop.nyx` | the companion and her route |
| `game-systems/boost-system.nyx` | whether the player may boost now |
| `game-systems/checkpoint-system.nyx`, `trigger-test.nyx` | reaching a checkpoint; the capsule-against-sphere test |
| `game-systems/streets-autopilot.nyx` | `LANTERN_AUTOPLAY=1` plays the streets, for smoke runs and screenshots |
| `game-rules/game-rules.nyx` | what a checkpoint, a fall and a boost mean |

`engine/tests/host/the-streets-teach-the-boost` plays the level with the keyboard through the headless host: crate, upper
street, boost, rooftop and checkpoint.

## Slice 2: light, thorns, chapters

Pick **The streets** to play the level with the Guide Light, or **The streets, later** for the same level in Chapter 3's
look with thorns (`LANTERN_LEVEL=worlds/streets-later.world.json`). Both levels continue past the rooftop of slice 1: two
gaps with no floor, a lantern between them and a second checkpoint at the end. `engine/tests/host/the-guide-light-shows-the-way`
walks the whole way with the keyboard, and `thorns-nudge-the-player-back` walks into the thorns.

| World kind | Properties | Meaning |
|---|---|---|
| `glow_platform` | none; a `block`-like piece with a `kinematic` collider | Hidden and not solid until a Guide Light reaches it, and for 4 s after the light leaves. |
| `lantern` | none | A small box the player walks into. It disappears and the player carries a cooler, smaller Guide Light from then on. |
| `thorns` | `half_extents` (vector), `min_chapter` (optional, 3) | A patch that appears from its chapter. Touching it sends the player back to the last place they stood on the ground away from thorns. Nothing is lost. |
| `spawn` | `chapter` (optional, 0) | Which look the level wears (`settings/chapter-looks.nyx`). The course is chapter 0, the streets chapter 1, the rooftops chapter 2, `streets-later` chapter 3. |

How it plays, in order:

1. The companion walks to the roof's edge and waits. Her warm light (reach 8 m) shows the first plates of the gap; the
   rest appear as she walks on and the player follows. A plate stays for 4 s after her light leaves, so a player a few
   metres behind her still has floor.
2. On the far block she stops, and the lantern is there to pick up. Its light (reach 5 m, cooler) goes with the player.
3. The second gap is beyond her last stop. Only the lantern's light shows its plates, which is the solo form of the same
   puzzle.
4. In the later level the rooftop and the far block have thorns beside a free lane. Stepping into them puts the player
   back on the safe lane.

| File | Job |
|---|---|
| `game-entities/guide-light.nyx` | one light that follows its carrier and has a reveal radius |
| `game-entities/glow-platform.nyx`, `game-systems/glow-system.nyx` | a piece that hides itself, and the rule that lights it |
| `game-entities/lantern-pickup.nyx`, `game-systems/lantern-system.nyx` | the lantern in the world and its pickup event |
| `game-entities/thorns.nyx`, `game-systems/thorn-system.nyx` | the patches and the touch tests |
| `settings/chapter-looks.nyx` | sky (zenith, horizon, ground), fog and sun for a chapter; the horizon colour also clears the screen and colours the fog so the distance blends into the sky |

The glow platforms use the workaround the engine card `engine-follow-ups/game-controlled-collision-for-world-entities`
names: a hidden piece is moved 1000 m down, and because a `static` collider doesn't move, these pieces are `kinematic` in
the world file.

## Slice 3: the Anchor

The design leaves the Anchor's 3D form open. Dean chose a **set swing arc**: two posts and a fixed pendulum arc between
them. It is deterministic, easy to author in the editor and easy to read without text. The streets continue past the
second checkpoint with two 12 m chasms, one swing across each. `engine/tests/host/the-anchor-swings-across-the-gap` walks
the whole way with the keyboard.

| World kind | Properties | Meaning |
|---|---|---|
| `swing_arc` | `end` (vector), `sag`, `seconds`, `needs_companion` (all optional but `end`) | The entity is the start post, `end` the far post. A jump on the ground beside the start post swings the player to the far post along an arc that dips `sag` metres (2) below the posts and takes `seconds` (1.8). With `needs_companion` set to 1 it only works while the companion stands within 3.5 m of either post. |

How it plays:

1. The first arc is the companion's form. She walks across the gap to a stop beside the start post (she is scripted, so
   she crosses the floorless gap above the plates her own light shows) and, once the player is within 16 m, swings the arc
   first so the player can watch. She then stands at the far post until the player has swung over, which is what the arc
   asks of her, and the start post's cap turns warm. The jump swings the player across.
2. The second arc is the solo form. It has no `needs_companion`, so it works with or without her; she swings it first too
   and walks on to the goal.
3. During a swing the character controller isn't asked to move the player: the game sets their feet along the arc
   each step (`Player.SwingTo`) and there is nothing to steer or miss. Gems can hang on the arc, and checkpoints sit on
   the landings, so a fall can't be a swing's fault.
4. Beads along the arc and a cap on each post say where the swing goes before anyone tries it.

| File | Job |
|---|---|
| `game-entities/anchor.nyx` | the two posts, the beads and the arc maths (`PointAt`) |
| `game-systems/anchor-system.nyx` | whether a jump would swing, starting it, and advancing it |

The arc is a circle hanging from a point above the middle, so its angle follows `-cos(pi t)`: the swing starts and ends
slowly and is fastest at the bottom. Posts stand 0.4 m inside each ledge, and a 2 m sag dips the feet about 0.3 m below
the ledge as the player passes its corner.

## Slice 4: a goal and the next level

Until now the streets had no end. Slice 4 gives them one, and gives the game a loop: reach the goal, see the gem tally,
and the next level loads. `engine/tests/host/the-anchor-swings-across-the-gap` carries on past the second swing, walks
into the goal and checks the hand-over.

| World kind | Properties | Meaning |
|---|---|---|
| `goal` | `next_level` (text, optional) | A gold plate and post. Touching it completes the level. `next_level` is a world file path; without it the game returns to the main menu. |

How it plays:

1. The goal stands at the far end of the streets (z -145.5). It is a trigger like a checkpoint: the player's capsule
   overlapping a 1.3 m sphere above the plate reaches it (`GoalSystem.DetectReached`).
2. `GameRules.Finish` marks the level complete once, tints the goal pale gold, plays the pickup sound and shows
   `Level complete! Gems N / M` for 6 s. Falling or restarting afterwards resets the goal (`Goal.Restore`).
3. After 4 s the game loads `next_level` and keeps playing. `streets-later` has no `next_level`, so finishing it
   returns to the main menu. The chain is authored in the world files: the streets lead to the rooftops (slice 5) and
   the rooftops to `streets-later`.
4. A `swing_arc` whose `end` is missing or less than half a metre away along the ground is skipped with a log line
   naming the entity, instead of breaking the level (`Anchor.isValid`).

| File | Job |
|---|---|
| `game-entities/goal.nyx` | the plate and post, the reached tint, the spin |
| `game-systems/goal-system.nyx` | the overlap test that raises `GameEvent.GoalReached` |

The debug overlay has `Game/Complete` and `Game/Level` for tests and tuning.

## Slice 5: the rooftops

A third level, `worlds/rooftops.world.json`, gives the first playthrough a chapter 2 between the streets (chapter 1) and
`streets-later` (chapter 3). It is a short, bright, **solo** level with no companion: four rooftops over a void, joined
by two moving platforms and one swing arc, six gems, three checkpoints and a goal. It uses only world kinds the game
already had (`moving_platform`, `swing_arc`, `checkpoint`, `gem`, `goal`), so the slice is a world file, a menu button and
a test. The main menu has a "The rooftops" button, and the streets' goal now leads here.

| Gap | Crossed by |
|---|---|
| 14 m, first | A 3 m platform that eases 6 m along the street every 6 s. Step on as it arrives, ride, step off. |
| 16 m, second | The same platform, longer: 12 m every 8 s. |
| 12 m, third | The solo form of the Anchor: jump beside the post. |

`engine/tests/host/the-rooftops-ride-the-platforms` plays the whole level from the menu with the keyboard: it waits for
each platform by its position (the debug overlay's `Platform/N` values, the depth of each moving platform in the order
the level lists them), rides it, swings and finishes. A platform's position is a function of the simulation clock, so
the waits are the same every run.

## How it fits the engine

The game keeps to the seam in [`game-api.md`](game-api.md). New world kinds are only labels the editor stores
and the game gives meaning to, as `gem` and `moving_platform` already are. Where the game needs something the API doesn't
offer, the gap is a card first and the workaround is named in the code:

| Gap | Card |
|---|---|
| Switching a world piece's collision on and off (slice 2 moves hidden pieces away instead) | `engine-follow-ups/game-controlled-collision-for-world-entities` |

## Open questions

- Boost jump height and trigger (a button press on the spot, or automatic when she crouches), to settle by playing it.
- Guide Light radii (8 m and 5 m), how long a plate stays (4 s) and the glow's strength, to settle by playing it.
- The Anchor's sag, duration and reach, and whether a swing should carry momentum out of the far post, to settle by playing it.

## Night and photographs

Dean's feedback after slice 5: play at night so the lights show what the mechanics do, and add a photograph collectable
for variety. Both are in `night-look.md` and `photo-mode.md` (decisions D-079 and D-080). The streets, the rooftops and the
streets, later each gained towers with lit windows, street lamps and photo sights; the towers sit outside the walls, so
the routes are unchanged.
