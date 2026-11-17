# Wet surfaces and puddles

This guide explains the wetness a material can have: a wet surface is darker and glossier, and wet ground facing up
breaks into puddles that act as mirrors. For where lighting happens in a frame, read `renderer.md` and
`lighting-and-shadows.md`.

## What it is

Rain changes how a surface looks in three ways, and the lit shader (`engine/shaders/lit.frag`) copies each:

1. **Water in the pores soaks up light.** A wet road or wall is darker. The diffuse colour is multiplied by 0.55 at full
   wetness. A metal has no diffuse colour, so it does not darken.
2. **A film of water is smooth.** Roughness falls most of the way towards a mirror's (0.1 at full wetness), so the sky
   shows in the surface and each lamp leaves a long, sharp streak on the ground below it.
3. **Standing water is flat.** Where water has collected into a puddle, the surface ignores its own normal and normal map,
   turns dark underneath (diffuse multiplied by 0.3) and becomes a near-perfect mirror (roughness 0.04).

Puddles only form on surfaces facing nearly straight up, and where a noise pattern over the ground plane (two octaves of
value noise, blobs a few metres across) rises above a cut-off. `puddles` moves the cut-off, so 0.25 is a few scattered
pools and 1 is most of the ground.

## Using it

Per entity, from Nyx:

```
lantern:::world:::SetWetness(entity, 1.0f, 0.5f);
```

- The first number is the wetness, 0 to 1. The second is how much of the entity's upward-facing surface is puddles,
  0 to 1. Both are clamped.

For the whole scene, the way rain works:

```
lantern:::graphics:::SetWetWorld(0.8f, 0.55f);
```

- Every draw uses the larger of its own wetness and the scene's, so `SetWetWorld(0, 0)` makes the scene dry again.
- The platformer calls it from `settings/chapter-looks.nyx`: the streets (chapters 1 and 2) are wet at night, and the
  rooftops and the course are dry. `LANTERN_WET=0` makes the city dry and `LANTERN_WET=1` rains everywhere.

In engine code, set `Material.wetness` and `Material.puddles` (`Material:::Wet(puddles)` is a soaked material with that
much puddle), and `RenderWorld.worldWetness` and `RenderWorld.worldPuddles` for the scene.

## How it works

Nothing new is sent to the GPU. The material's uniform block already had two spare floats, so wetness travels in
`surface.w` and the puddle amount in `emissive.w` (`GpuUniforms.nox`, `MaterialUniforms`). The shader's wetness block
runs before the lighting, so every light sees the darker, smoother, flatter surface, and the sky's image-based reflection
(see `sky-and-image-based-lighting.md`) shows in it with no extra work.

## Cost and limits

- A few dozen ALU operations and four hash lookups per pixel, only for draws with wetness. It is not changed by the
  quality level.
- **A puddle reflects the sky and the lights' highlights, not the scene.** The buildings and the player do not appear in
  it. At night the sky is dark, so a puddle reads as a dark patch with a bright streak under each lamp. Reflecting the
  scene needs screen-space reflections or a reflection probe, which are not built (`next-renderer-ideas` on the board).
- The puddle pattern is fixed in world space, so it does not move or ripple, and a moving platform carries no puddles
  of its own: they appear where the noise is high wherever the surface is.
- The editor's material inspector has no wetness field yet.
