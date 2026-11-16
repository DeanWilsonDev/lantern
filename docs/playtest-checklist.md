# Play-test checklist

What to try in each room, and what to write down. The point is to find out whether the loop is fun on its own, so note
how it felt before you note whether it worked. Start the game (`mist run game`), pick **Prototypes** and a room.

Partner keys: **Q** call her, **E** wait, **F** pull a lever beside you or send her to the glowing spot you face
(gamepad: D-pad left, D-pad right, West). Dying or getting seen returns you to the last checkpoint; **R** restarts.

For every room, play it **twice**: once with her, once alone. Note the time each way if you can.

| Room | With her | Alone | Questions |
| --- | --- | --- | --- |
| 1 Hold the door | Face the left plate and press F; run through the door | Run the right plate, then the door within 4 s | Did sending her feel like trust or like a chore? Was 4 s fair? |
| 2 The narrow way | Face the slot and press F | Climb the ledges on the left | Did she feel like a partner? Is the ledge climb a good skill test or a slog? |
| 3 Light gate | F toward the shadow spot and the lit spot; compare | Take the lantern, find the spot, run | Could you read why it opened or not from the light and shadows? |
| 4 Hand-off | F to the spot by the ledge, jump from her back; pull the lever; send her to the plate | The steps on the right | Did needing each other in turn land? Was anything unclear? |
| 5 The watcher | F to the open spot, then walk the hall | Move pillar to pillar between sweeps | Is waiting tense or boring? Is being sent back too harsh? |
| 6 Flow and fragile | She follows the line | Same | Does momentum feel good? Do crumbling slabs feel fair? |
| 7 The crossing | Use her wherever a sign suggests | Do it all alone | Which beat did you enjoy most, and which would you cut? Where did you want to stop? |

## The streets companion

In **The streets** she walks the route ahead of you (the commanded partner above is only in the prototype rooms).

- Does she feel like she is leading? Run past her and hang back: does she hurry on, or stop and hop to call you?
- The first swing: does she swing first, so you can watch, and then wait for you on the far post? Does your own swing
  work the moment you are at the start post?
- Is 10 m (16 m at a swing) a good distance for her to set off? Did she ever leave you behind or wait for nothing?

## Grounded adventure

Rooms 8 to 10 of the Prototypes list (the climb, the word market, the station) use the grounded feel: a lower jump,
a slower walk, speed that builds and fades, and a weighty landing. Details in [`grounded-adventure.md`](grounded-adventure.md).
None of this has been run before; the first play is the test.

| Room | Try | Questions |
| --- | --- | --- |
| 8 The climb | Climb the pipe (F, W). Hop on the crate, jump and press F in the air to take the ladder. Let go with Space. Then find the slot and the roof lever | Does the grounded jump feel weighty or just low? Is climbing fun for six seconds or a wait? Did the second route feel like her gift to you, or yours to her? |
| 9 The word market | Walk into every tag. Eat at both stalls: hold F and keep still, let go and hold again | Is collecting a rest or a chore? Is holding F to eat charming for ten seconds, or too long? Did you want the kana to show? |
| 10 The station | Buy a ticket, cross by the footbridge, then again by the ladder shortcut. Watch the trains from the island | Do trains feel like a place with life in it? Is the shortcut worth the risk? Did you ever not know where to go? |

Also note: how long to full speed feels right (0.4 s), whether a hard landing's pause is felt or annoying, and whether
the jump is too low for the steps and crates.

## Things to feel for

- Boost height 3.6 m, pad height 5 m and 6 m, the four-second plates: too much, too little, right?
- When you send her, did you know where she would go? Did "She can't get there" ever appear unfairly?
- Did she ever get in the way, block a jump, or stand somewhere silly?
- Did anything take over a minute without you knowing what to do?
- Was there a point where you stopped caring? Say where.

## Known gaps

Solo routes for the ledge climbs and the step stairs were checked by arithmetic only; the door, lever, lure, pad and slab
routes are covered by scripted tests. Wayland, Windows and real GPUs were not run. All times are from a cloud
renderer, so frame rate figures mean nothing yet.
