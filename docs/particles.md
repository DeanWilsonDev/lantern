# Particles

This guide explains the particle system: dust, sparks and trails a game emits from Nyx. For where they are drawn in a frame,
read `renderer.md`.

## What it is

A particle is a short-lived speck with a position, a velocity, a colour, a life in seconds, a gravity and a streak length.
A game calls `lantern:::particles:::Emit(position, velocity, color, life, length, gravity)` once per speck. The engine then
does the rest every frame (`RenderWorld.StepParticles`, called by the frame loop right after the script's `OnRender`):

1. It moves every particle by its velocity and pulls its velocity down by `gravity` (metres per second squared).
2. It drops particles whose age has reached their life.
3. It draws each survivor as a line from its position back along its path by `length` seconds of travel, so a fast spark is
   a long streak and a slow mote a short one. The line's alpha falls from 1 to 0 over the particle's life, so it fades out.

`Clear()` removes every particle and `Count()` says how many are alive.

## Quality

`SetQuality` sets how many particles may be alive at once: 64 at "low", 256 at "medium" and 512 at "high". An `Emit` past
that budget is dropped, so a busy effect gets thinner on a weak machine instead of slower. The budget never goes above the
fixed ceiling of 512.

## Limits

- Particles are drawn with the debug line pipeline, so they are one pixel wide, depth-tested against meshes and drawn after
  the composite pass: they aren't bloomed, graded, fogged or anti-aliased, and stay sharp at any resolution.
- Colours are 0 to 1; a spark can't be brighter than white.
- The simulation is simple (no collisions, no wind) and uses the display frame's time, so it is for looks, never gameplay.
- A billboard quad pipeline with a texture, additive blending and soft edges is the natural next step; it needs its own
  pipeline and is not built.

## In the platformer

`games/platformer/game-systems/dust.nyx` kicks up a ring of grey-brown streaks at the feet on every jump and landing, sized
by how hard the landing was. It is the only game code that uses the system.
