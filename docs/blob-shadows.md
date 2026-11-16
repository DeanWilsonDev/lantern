# Blob shadows

This guide explains the soft dark patch under the player, why it is cheap, and how a game adds one. For where it sits in a
frame, read `renderer.md`.

## Why a fake shadow

The sun's shadow map (`lighting-and-shadows.md`) gives a character a real shadow, but it is a long shape thrown across the
ground, often away from where the character will land. In a platformer the most useful shadow is the one straight below:
it says where you will come down. A *blob shadow* is the cheap version: a soft dark ellipse on the ground under the thing,
with no shadow map involved. It also grounds a character who is standing in the shade, where the real shadow vanishes.

## The idea: darken pixels near a point

The game gives the renderer a point on the ground, a radius and a darkness. The composite pass (`composite.frag`) already
reads the depth buffer to know where in the world each pixel is, so for each pixel it asks: how close is this pixel's world
position to the blob's point?

```text
across   = horizontal distance to the point / radius
vertical = vertical distance to the point / reach
edge     = sqrt(across^2 + vertical^2), at most 1
darkness = strength * smoothstep(1 - edge)
pixel   *= 1 - darkness
```

Inside the ellipse the pixel is darkened, most at the centre, falling smoothly to nothing at the rim. `reach` (0.6 of the
radius, at least 0.3 m) limits how far above or below the point a surface still counts, so the blob darkens the floor and
the bottom of the character's feet but not its head.

Because the test is on whatever surface is there, a blob lies correctly over slopes, steps and the foot of a wall. A flat
disc mesh needs lifting off the floor to avoid flicker and cuts through anything that isn't flat; this has neither problem.
It also needs no mesh, so there is nothing to spawn or hide.

## Using it from Nyx

Blob shadows last one frame, like draws, so a game adds them every frame in `OnRender`:

```nyx
lantern:::graphics:::AddBlobShadow(feetOnGround, 0.6f, 0.55f);
```

The arguments are the point on the ground, the radius in metres and the darkness from 0 to 1. A frame holds up to eight; a
ninth is ignored. `lantern:::graphics:::SetBlobShadows(false)` switches them all off.

The platformer's player finds the floor with a downward ray each step, remembers the point, and shrinks the radius with
height so a smaller shadow reads as higher up (`game-entities/player.nyx`); `OnRender` hands it to the engine.

## Cost

One loop over at most eight blobs for each pixel, and only when there is at least one. Quality levels don't change it.

## Where the code is

| Piece | File |
| --- | --- |
| The darkening | `ApplyBlobShadows` in `engine/shaders/composite.frag` |
| The data | `BlobShadow`, `AddBlobShadow` in `engine/src/renderer/RenderWorld.nox` |
| Packing | `PackCompositeUniforms` in `engine/src/sdl-facade/renderer/GpuSky.nox` |
| The Nyx functions | `engine/src/host/GraphicsApi.nox` |
