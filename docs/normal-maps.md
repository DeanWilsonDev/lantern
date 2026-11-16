# Normal maps

This guide explains normal maps: textures that make a flat surface light as if it were bumpy. For where lighting happens in
a frame, read `renderer.md` and `lighting-and-shadows.md`.

## What it is

A mesh's triangles give a surface one normal (the direction it faces) per vertex, smoothed across the triangle. Fine detail
such as cobbles, rivets or brick mortar would need a huge number of triangles to light correctly. A *normal map* stores a
direction per texel instead, as a colour: red is the sideways lean, green the up-down lean and blue the lean away from the
surface, so flat is (0.5, 0.5, 1), a light lavender-blue. The lit shader bends the surface normal by the map before it
adds up the lights, so every light, the sun and local ones, sees the bumps while the geometry stays flat.

## Using it

```
bumps = lantern:::textures:::Bumps(128, 4, 0.9f);
lantern:::world:::SetNormalMap(entity, bumps, 1.0f);
```

- `lantern:::textures:::Bumps(size, cells, steepness)` makes a map of round bumps, `cells` across, for tests and rough
  surfaces. `steepness` is 0 (flat) to 1.
- A normal map from a file is loaded with `lantern:::textures:::Load(path)` like any texture, then given to an entity with
  `SetNormalMap`. The map's green channel points up the image (the OpenGL convention); a map authored the other way round
  lights the bumps as dents.
- `SetNormalMap(entity, texture, strength)` takes a strength: 0 ignores the map, 1 follows it as authored, up to 4
  exaggerates it. The entity needs a mesh already.
- In engine code, set `DrawItem.normalTexture` and `Material.normalStrength`.

## How it works

The mesh vertex has no tangent, so the shader builds the tangent frame from how the world position and the uv change from
pixel to pixel (screen-space derivatives), the usual "cotangent frame". That needs no extra vertex data and works on every
mesh, at the price of a frame that is flat across a triangle's pixels; a mesh with sharp uv seams can show a faint seam in
the bumps. Textures are uploaded as sRGB colour, so the shader encodes the sampled value back to the stored bytes before
using it as a direction.

## Cost and limits

- One extra texture sample and a few dozen ALU operations per pixel, and only for draws that have a map. The draw's
  texture and normal map are bound separately, so a normal-mapped draw adds one sampler bind. It is not tied to the quality
  setting.
- No parallax or height: the bumps change lighting only, so the silhouette and the shadows stay flat.
- The normal map shares the albedo's uv coordinates; it can't tile differently.
- Decals (texture quads projected on a surface) are not built. A quad drawn a few millimetres above the surface with the
  existing `Draw` gets most of the way for flat ground, and a projected decal volume needs a depth-reading pass of its own.
