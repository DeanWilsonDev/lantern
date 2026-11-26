# Material graphs

A material graph describes how a surface looks by wiring nodes together, the way a material editor in a big engine does:
a colour, a texture, some maths, into one **Material Output** node with base colour, metallic, roughness, emissive and
normal. Each graph becomes a **lit fragment shader of its own**. The lighting code stays in one place; the graph only
decides what the surface is at each pixel before it is lit.

```
 material.graph.json  ──generate──▶  <name>.frag (GLSL)  ──compile──▶  .spv / .msl  ──▶  a lit pipeline for this material
   (beside root.calamus)            SurfaceOf() between                (at bake)          (the renderer, per material)
                                    lit-common and lit-main
```

## Where a graph lives

A material is a material Silhouette (D-212): a folder under `assets/silhouettes/` (or `assets/silhouettes/materials/`)
whose `root.calamus` says `@type material`. Its graph is `material.graph.json` in the same folder. Both are project files:
the editor reads and writes them, the bake turns them into shaders, and the built game never reads either (D-213, D-217).

A material Silhouette without a graph keeps working exactly as before, through the uniform values `texture`, `tint`,
`metallic`, `roughness` and `emissive`.

## The file

A material graph is an ordinary graph file (`docs/visual-scripting.md`), so the Graph tab's canvas, wiring, comments,
layout and file format all apply. It has one function called `material`, and its nodes are the `mat_` kinds below.

```json
{
  "graph": "rusty_metal",
  "functions": [
    { "name": "material", "nodes": [
      { "id": "coordinate", "kind": "mat_uv" },
      { "id": "tiled", "kind": "mat_mul", "a": "coordinate", "b": { "f32": 4.0 } },
      { "id": "rust", "kind": "mat_texture", "name": "rust", "a": "tiled" },
      { "id": "paint", "kind": "mat_color", "text": "#804020" },
      { "id": "base", "kind": "mat_mul", "a": "paint", "b": "rust" },
      { "id": "wear", "kind": "mat_param", "name": "wear", "a": { "f32": 0.2 } },
      { "id": "bare", "kind": "mat_one_minus", "a": "wear" },
      { "id": "out", "kind": "mat_output", "args": ["base", "bare", { "f32": 0.6 }, "", ""] }
    ] }
  ]
}
```

## Nodes

| Kind | Ports | Gives |
| --- | --- | --- |
| `mat_scalar` | `a`: a number | a number |
| `mat_color` | `text`: `#RRGGBB` | a colour |
| `mat_param` | `name`: a number in the Silhouette's `properties`; `a`: the default | a number, baked in as a constant |
| `mat_uv` | | the surface's texture coordinate |
| `mat_texture` | `name`: a texture in `textures/`; `a`: the coordinate (the surface's own when unwired) | a colour |
| `mat_normal_map` | `name`: a normal-map texture; `a`: the coordinate; `b`: its strength (1) | a tangent-space normal |
| `mat_add` `mat_sub` `mat_mul` | `a`, `b` | per channel; a number joins any other value |
| `mat_lerp` | `a`, `b`, `value` | `a` towards `b` by `value` (0 to 1) |
| `mat_one_minus` `mat_clamp` | `a` | `1 - a`, or `a` held between 0 and 1 |
| `mat_output` | `args`: base colour, metallic, roughness, emissive, normal | the material |

Values have one of four shapes: a number, a coordinate (`vec2`), a colour (`vec3`) or a normal. A number combines with any
of them; a coordinate and a colour don't, and the generator says which node tried. An output a graph leaves unwired takes
the default a material Silhouette with no properties has: white, not metallic, roughness 0.8, no light of its own, flat.
A material samples at most four textures.

## The shader

`lantern:::materialgraph:::GenerateMaterialShader(graph, parameterNames, parameterValues)` (in
`engine/src/materialgraph/`) writes a `SurfaceOf(vec2 uv)` function with one local per node, in the order they are read,
between the lit shader's two halves:

- `engine/shaders/lit-common.glsl`: every declaration and lighting function the lit shader has, and `MaterialSurface`.
- `engine/shaders/lit-main.glsl`: `main`, which calls `SurfaceOf`, multiplies its base colour by the vertex colour and the
  draw's tint, bends the normal, applies wetness and lights the result. A graph's emissive is added to the draw's own.

`lit.frag` is the same two halves around the default `SurfaceOf`: the draw's texture, its uniform metallic and roughness,
and its normal map. Splitting it changed no picture (the screenshot tests pass unchanged).

A graph's textures are bound after the normal map, from binding 4. A broken graph is refused with the node at fault:
`node 'sum' can't combine a vec2 with a vec3`.

Test: `engine/tests/materialgraph/a-material-graph-becomes-a-lit-shader`.

## In the renderer

`RenderBackend.CreateMaterialShader(source)` takes a `MaterialShaderSource` (the compiled shader's `name` and `directory`,
and up to four texture handles in the order the graph named them) and returns a `Handle<MaterialShaderTag>`. The GPU
renderer builds two pipelines from it, the lit one and the skinned one, both with the material's fragment shader and
`4 + textureCount` samplers. A `DrawItem` with `materialShader` set is drawn with them; one without, or with a removed
handle, uses the ordinary lit pipeline. Visible draws sort by material shader first, so each pipeline is bound once a
frame, then by texture and mesh as before. On a switch the renderer binds the shadow maps again and the material's
textures from slot 4. Shadow maps don't change: a material decides how a surface looks, not its shape.

A graph material ignores the draw's own `texture` and `normalTexture` and its uniform metallic and roughness; the tint,
the draw's emissive and wetness still apply. `RemoveMaterialShader` frees the pipelines, not the textures.

Test: `engine/tests/gpu/a-material-shader-draws-its-own-surface` (a red-tinted sphere turns green and blue under a graph
that samples a blue texture and glows green, and goes back to red when the shader is removed). Its compiled shader is
checked in beside it in `materials/`, generated from `green_glow.graph.json`.

## Still to come

Tracked on the board under `material-graphs-an-unreal-style-material-editor-that-generates-a-shader-per-material`:

1. **The bake.** `mist run materials` writes each graph's `.frag` and its compiled `.spv` and `.msl` into `game-assets/`,
   and `BakedMaterials.nyx` registers them.
2. **The editor.** A Material Graph tab reusing the graph canvas, with live preview.
3. **Later.** Time and panner nodes, world position and fresnel, parameters a game changes while it runs.
