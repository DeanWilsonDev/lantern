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
| Ability 2, the Guide Light: her glow reveals hidden platforms; solo, a smaller, cooler lantern | Slice 2 | The gameplay is Nyx. Drawing the glow as a light depends on the renderer's point lights, which are being added separately. |
| Chapter looks: palette, fog, tone from neutral to saturated to cold (Game Design, Narrative Structure) | Slice 2 | Clear colour, fog and gem tint are already game-settable; lighting colour follows the renderer work. |
| Thorns from Chapter 3 | Slice 2 | A hazard that moves the player back, never one that ends the run. |
| Ability 3, the Anchor | Later | The 3D form is open in the design (physics rope, set arc or something else). Waiting on Dean (question on the card). |
| Authored cameras: static shots, player-locked, player-controlled (Camera) | Later | The third-person camera is the only mode today. A shot system needs its own design. |
| Chapters 1 to 5 as a whole, recurring and unique levels | Later | The slice builds one recurring level (the streets) in two chapter looks. More levels come after the mechanics feel right. |
| Post-completion changes (the blue and red doors, stripped levels) | Later | Needs saving and a finished first playthrough. |
| Companion-driven audio (music layers, her distance as a mix input) | Later | The engine plays one-shots and one music track. Layered music is an audio feature. |
| Save and menu flow, train transitions, documentary | Not for the demo | Product concerns, not engine demos. |

## Slice 1: the streets

Playable end to end with what the engine has today:

1. **Checkpoints.** A `checkpoint` world kind. Touching one stores the respawn point; falling returns the player there and
   keeps the gems already collected.
2. **The companion.** A `companion_stop` world kind with `order` and `wait_seconds`. She stands at a stop, walks to the
   next when the player comes within a few metres of her, and waits less in later chapters.
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
| `companion_stop` | `order`, `wait_seconds`, `boost` (optional), `speed` (optional) | One stop of the companion's scripted route. She walks to the next stop when the player is within 4 m of her and she has waited; at a `boost` stop she crouches and waits until the player has jumped off her back. |

The route through the level:

1. Walk to the crate and jump onto it, then jump from the crate onto the upper street. The street is 2.2 m up, one
   normal jump (1.5 m) too high, so this is the solo form of the Boost.
2. Walk up the street. The companion, waiting at the first stop, follows once you are close, and crouches against the
   wall at the end. The rooftop is 3.2 m above the street.
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

- The Guide Light and the lantern. Hidden `glow_platform` pieces become solid and visible while the companion's glow, or
  the carried lantern's smaller one, reaches them.
- Chapter looks and thorns, so one level reads differently in Chapter 1 and Chapter 3.

## How it fits the engine

The game keeps to the seam in [`game-api.md`](game-api.md). New world kinds are only labels the editor stores
and the game gives meaning to, as `gem` and `moving_platform` already are. Where the game needs something the API doesn't
offer, the gap is a card first and the workaround is named in the code:

| Gap | Card |
|---|---|
| Switching a world piece's collision on and off | `engine-follow-ups/game-controlled-collision-for-world-entities` |
| Point lights the game can place (the glow) | The renderer lighting work |
| The Anchor's traversal form | `platformer-game/crimson-and-thorn-slice/the-anchor-in-3d` |

## Open questions

- Boost jump height and trigger (a button press on the spot, or automatic when she crouches), to settle by playing it.
- Guide Light radius against the lantern's, to settle once lights exist.
- The Anchor's form in 3D, for Dean.
