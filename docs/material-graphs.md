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
| `mat_normal_map` | `name`: a normal-map texture; `a`: the coordinate; `b`: its strength, a number or any number node (1) | a tangent-space normal, its tilt scaled by the strength |
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

## The bake

`mist run materials` (`engine/tools/build-materials`, also run by the content watcher and the editor after a Silhouette
save, and by `mist run package`) finds every material Silhouette with a `material.graph.json` beside its `root.calamus`,
in `silhouettes/` and then `silhouettes/materials/`. For each one it:

1. generates the GLSL, with the Silhouette's numeric `properties` as the values `mat_param` nodes read;
2. writes it to `game-assets/materials/<name>.frag`, ending with a hash of the engine's lit shader code it includes, and
   when that text changed (so a graph change or a lit shader change), or the compiled files are missing,
   compiles it beside itself to `<name>.frag.spv` (`glslangValidator -I engine/shaders -V`) and `<name>.frag.msl`
   (`spirv-cross --msl --msl-version 20100 --msl-decoration-binding`), the same tools `engine/tools/build-shaders.sh`
   uses;
3. adds a line to `BakedMaterials:::Register()`, after every material's `SetMaterialLook`:

```nyx
lantern:::world:::SetMaterialShader("painted_metal", "../game-assets/materials", "painted_metal", Texture("textures/wall.png"), 0, 0, 0);
```

The engine's shader folder is found from the assets folder (the checkout they sit in), or from `LANTERN_ENGINE_SHADERS`.
A graph that can't be a material stops the bake with the node at fault, and the previous files stay. The package keeps
the compiled `.spv` and `.msl` and drops the graph, the Silhouette and the `.frag`.

`SetMaterialShader` asks the renderer for the material shader and gives it to the material's look, so every piece placed
with that material draws with it. A graph material's look still takes the Silhouette's tint (keep it `#FFFFFF` to let the
graph decide the colour). A shader that fails to load logs a warning and the material keeps its plain look.

The platformer's sample is `assets/silhouettes/materials/painted_metal`: blue paint over the tiled wall texture, with
its roughness from the `wear` property. Test: `engine/tests/editorbackend/the-game-materials-are-built-and-current`
checks the baked files are current and what the bake writes for it.

## Still to come

Tracked on the board under `material-graphs-an-unreal-style-material-editor-that-generates-a-shader-per-material`:

1. **The editor.** A Material Graph tab reusing the graph canvas, with live preview.
2. **Later.** A normal map sampled at a coordinate other than the mesh's own (flipped or rotated) still tilts along
   the mesh's UV directions, because the tangent frame is built from the mesh's UVs; tiling and offsets are fine. Time and
   panner nodes, world position and fresnel, parameters a game changes while it runs.
