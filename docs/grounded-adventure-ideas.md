# More ideas to test for the grounded adventure

Dean's direction (2026-11-16): an action adventure without combat, grounded in movement, with challenge from places and small
games. The climb, the word market and the station ([`grounded-adventure.md`](grounded-adventure.md)) cover climbing, rests
and a place with life in it. These five are the next things worth testing, each with how it is meant to be fun, what the
companion does as an equal, and what it costs on today's engine. **S** is a day or less of Nyx, **M** a few days, **L** needs
engine work.

## 1. Crowd flow (M)

A busy platform of walking figures (blocks with a heading and a speed) that the player has to move through without
bumping. Bumping isn't punished; it slows you and a figure says "sumimasen".

- **Why it is fun:** reading a crowd and finding a gap is a real skill, and it is the opposite of a platforming jump:
  patience and anticipation. It also makes a station feel alive for the cost of a handful of moving boxes.
- **Her part:** she is small and slips through gaps you can't. She waits in a clear spot and you go to her, so the way
  through the crowd is a place you choose together.
- **Needs:** figures on timed paths (a moving platform without a rider); a "bumped" slow-down rule. No engine work.

## 2. Hold on: stamina on the long climbs (S)

A grip meter that falls while you hang on a pipe or shimmy, and refills on a ledge. At zero you slip back down to the last
ledge (never to the ground).

- **Why it is fun:** it turns a climb from a wait into a decision about resting spots, and it is what makes a pipe worse than
  a ladder for a reason you feel.
- **Her part:** she can't climb, so she is the rest at the bottom: a ledge she holds with a rope or a lantern gives you
  somewhere to reset. A long climb with a hanging rope she lets down is the equal-partners version.
- **Needs:** one number and a rule in `ClimbSystem`; a HUD bar.

## 3. Shop and street stalls with a small purchase (S)

At a stall, choose from two dishes with a short phrase to say ("kore wo kudasai": this one, please). Choosing the right
phrase from three, from a list you collected, buys the dish and gives a small boost, such as a brighter lantern for a while.

- **Why it is fun:** the words you collected become a verb. It makes the language minigame matter without making it a quiz,
  because a wrong phrase costs nothing but a retry and a smile.
- **Her part:** she can order for both of you, which is the game's small joke about her being better at it than you.
- **Needs:** a choice UI (a few buttons, which the UI already has) and the word list in the model. Kana wants the engine card.

## 4. The last train: a timed run with a station in the way (M)

A short level where a train is leaving and you have to get both of you on it. The route is the station, with the gates,
the bridge and the crowd, and the time is generous: it is about flow, not failure.

- **Why it is fun:** every skill from the station (bridge or ladder, ticket, crowd) returns under a soft clock, and the
  ending, getting on together, is a clear emotional beat. It is the natural first test of "completes only together".
- **Her part:** she has to be on the platform with you when the train goes; if she is behind, the door waits for her.
- **Needs:** a moving train with a door that opens by a rule, a goal that is a place on a platform. No engine work.

## 5. Light a quiet corner: small lantern puzzles with a point (S)

A dark underpass or storeroom where lighting lamps in a sequence (by walking up and pressing F) opens a door, with a
lamp the player can reach and one only she can.

- **Why it is fun:** it reuses the lantern and lighting the engine already does best, and it gives the night look something
  to do besides look good. It is a quiet beat between the busy ones.
- **Her part:** her light reaches where yours can't; she lights the lamp in the vent while you light the one on the stair.
- **Needs:** a `lamp_switch` kind that turns a real point light on (`lantern:::lights`) and a channel. No engine work.

## Which to try first

Crowd flow and stamina are the two that most change how the game feels, and both are cheap. The station is the place to try
the first, and the climb room the second. The last train is the best second-round level once the crowd exists.
