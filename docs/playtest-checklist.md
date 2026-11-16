# Play-test checklist

What to try in each room, and what to write down. The point is to find out whether the loop is fun on its own, so note
how it felt before you note whether it worked. Start the game (`mist run game`), pick **Prototypes** and a room.

She has no commands: there is no Q, E or send. She leads, waits and works beside you, and **F** pulls a lever beside you
(gamepad: West). A level only completes when she is with you at the goal. Dying or getting seen returns you to the last
checkpoint; **R** restarts.

For every room, play it **twice**: once with her, once alone (alone is harder, and it should still be possible). Note the
time each way if you can. Everywhere, watch whether she gets lost, stuck on stairs, or walks off an edge.

| Room | With her | Alone | Questions |
| --- | --- | --- | --- |
| 1 Hold the door | She holds the first door; walk through, then she meets you. Hold the far plate for her | Run the timed plate and the door within 4 s | Did she hold and wait for you? Did the far plate feel fair? |
| 2 The narrow way | She goes through the slot and pulls the lever | Climb the ledges on the left | Did she feel like a partner? Is the ledge climb tense and harder, but fair? |
| 3 Light gate | She stands in the lit spot; the gem only counts when her light or the lantern reaches it | Carry the lantern and run | Could you tell why the gem was lit or dark? |
| 4 Hand-off | She crouches; jump from her back; hold the lever and plate for her | The low steps on the right | Did she get up the stairs? Did needing each other in turn land? |
| 5 The watcher | She lures the light on her own; walk the hall | Move pillar to pillar between sweeps | Is it fun but out of place? Is the beam easy to see? |
| 6 Flow and fragile | She leads over the slabs and breaks them behind her; take the stepping stones | The stones are the only way | Could she keep up? Is the last jump fair? |
| 7 The crossing | Follow her lead and hold what she needs | Do it all alone | Which beat did you enjoy most? Did she ever get stuck or fall behind? |

## The streets companion

In **The streets** she walks the route ahead of you (the team partner above is only in the prototype rooms).

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

- Boost height 3.6 m, the four-second plates: too much, too little, right?
- Did you ever feel you were giving her orders, or that she was just following? Did you know where she would go next?
- Did she ever get in the way, block a jump, or stand somewhere silly?
- Did anything take over a minute without you knowing what to do?
- Was there a point where you stopped caring? Say where.

## Known gaps

The team model, its edge and recovery checks, and the reworked rooms were written without a compiler or a game run in the
cloud, so none of it has run; the scripted tests are unrun too. Wayland, Windows and real GPUs were not run. All times are from a cloud
renderer, so frame rate figures mean nothing yet.
