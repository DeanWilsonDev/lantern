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
| `mat_scalar` | `a`: a number typed in, or a wire from any number node (a parameter, a sine) | a number |
| `mat_color` | `text`: `#RRGGBB` | a colour |
| `mat_param` | `name`: a number in the Silhouette's `properties`; `a`: the default | a number that starts at the Silhouette's value and that a game can change while it runs (at most eight per material) |
| `mat_uv` | | the surface's texture coordinate |
| `mat_time` | `a`: speed (1) | seconds since the game (or editor) started, times the speed; wraps every hour |
| `mat_panner` | `a`: the coordinate (the surface's own); `b`, `value`: speed along x and y | the coordinate sliding over time |
| `mat_world_position` | `name`: `xyz` (a colour, the default), `xz` `xy` `zy` (a coordinate) or `x` `y` `z` (a number) | where the pixel is in the world, for world-aligned textures or height fades |
| `mat_fresnel` | `a`: power (5) | 0 where the surface faces the camera, rising to 1 at its edge |
| `mat_sine` | `a` | the sine of `a`, per channel; with `mat_time`, a pulse |
| `mat_texture` | `name`: a texture in `textures/`; `a`: the coordinate (the surface's own when unwired) | a colour |
| `mat_normal_map` | `name`: a normal-map texture; `a`: the coordinate; `b`: its strength, a number or any number node (1) | a tangent-space normal, its tilt scaled by the strength |
| `mat_add` `mat_sub` `mat_mul` | `a`, `b` | per channel; a number joins any other value |
| `mat_lerp` | `a`, `b`, `value` | `a` towards `b` by `value` (0 to 1) |
| `mat_one_minus` `mat_clamp` | `a` | `1 - a`, or `a` held between 0 and 1 |
| `mat_output` | `args`: base colour, metallic, roughness, emissive, normal, opacity mask, opacity | the material |

Values have one of four shapes: a number, a coordinate (`vec2`), a colour (`vec3`) or a normal. A number combines with any
of them; a coordinate and a colour don't, and the generator says which node tried. An output a graph leaves unwired takes
the default a material Silhouette with no properties has: white, not metallic, roughness 0.8, no light of its own, flat,
solid and opaque. Where the opacity mask is below 0.5 the pixel is cut away, for grates, leaves and chain-link fences,
and its shadow is cut the same way. A graph that wires its opacity is see-through: the lit colour is blended over what
is behind it by that amount (0 shows only what is behind, 1 hides it), for glass and water.
A material samples at most four textures.

## The shader

`lantern:::materialgraph:::GenerateMaterialShader(graph, parameterNames, parameterValues)` (in
`engine/src/materialgraph/`) writes a `SurfaceOf(vec2 uv)` function with one local per node, in the order they are read,
between the lit shader's two halves:

- `engine/shaders/lit-common.glsl`: every declaration and lighting function the lit shader has. It includes
  `material-surface.glsl`: `MaterialSurface`, `DefaultSurface(uv)` (every field set to the defaults above, which a
  generated `SurfaceOf` starts from), `MaterialTime()` and `Fresnel`.
- `engine/shaders/lit-main.glsl`: `main`, which calls `SurfaceOf`, multiplies its base colour by the vertex colour and the
  draw's tint, bends the normal, applies wetness and lights the result. A graph's emissive is added to the draw's own.

A graph that wires its opacity mask also gets `maskSource`: the same `SurfaceOf` between `mask-common.glsl` (only what
a surface reads: the time, the parameters and its textures from binding 0) and `mask-main.glsl`, which discards where the
mask is below 0.5. The shadow pass draws the material's pieces with it. A graph that wires its opacity is marked
`isTranslucent`.

`lit.frag` is the same two halves around the default `SurfaceOf`: the draw's texture, its uniform metallic and roughness,
and its normal map. Splitting it changed no picture (the screenshot tests pass unchanged).

A graph's textures are bound after the normal map, from binding 4. Its parameters are eight floats at the end of the
per-draw material uniforms (`material.parameters`, two vec4s); each parameter takes the next slot in the order the
graph reads them. The surface records the coordinate its
normal map was sampled at (`normal_uv`), and the lit shader builds the tangent frame from it, so a normal map follows a
scaled, panned, flipped or world-aligned coordinate. Time is the camera position uniform's fourth component, which the
lit shader didn't use. A broken graph is refused with the node at fault:
`node 'sum' can't combine a vec2 with a vec3`.

Test: `engine/tests/materialgraph/a-material-graph-becomes-a-lit-shader`.

## In the renderer

`RenderBackend.CreateMaterialShader(source)` takes a `MaterialShaderSource` (the compiled shader's `name` and `directory`,
and up to four texture handles in the order the graph named them) and returns a `Handle<MaterialShaderTag>`. The GPU
renderer builds two pipelines from it, the lit one and the skinned one, both with the material's fragment shader and
`4 + textureCount` samplers. A `DrawItem` with `materialShader` set is drawn with them; one without, or with a removed
handle, uses the ordinary lit pipeline. Visible draws sort by material shader first, so each pipeline is bound once a
frame, then by texture and mesh as before. On a switch the renderer binds the shadow maps again and the material's
textures from slot 4, and each draw's material uniforms carry that shader's parameters (SDL's GPU API allows only
four uniform blocks per stage, and the lit shader uses all four).

A source marked `masked` also gets two shadow pipelines from `<name>.mask.frag` (with the `shadow-masked` and
`shadow-masked-skinned` vertex shaders, which pass the world position, normal and coordinate on), so the shadow is cut by
the mask: a grate's shadow has its slats. A source marked `translucent` gets lit pipelines that blend by the surface's
opacity, test depth without writing it, and leave the second colour target (the surface buffer reflections read) alone.
Its draws are sorted after every opaque draw, farthest first, and drawn after the sky, so each one blends over what is
behind it. A see-through piece casts no shadow.

Test: `engine/tests/gpu/masked-and-see-through-materials-draw-and-shadow` (a cube whose mask cuts it all away casts no
shadow with its mask shader and its whole shadow without; half see-through glass shows the red wall behind it, solid
glass hides it, and a draw with its own opacity of 0 shows only the wall).

`MaterialShaderSource.parameters` holds the values the parameters start at, and
`RenderBackend.SetMaterialShaderParameter(shader, slot, value)` changes one for every draw from then on. A `DrawItem`
with `overridesParameters` set carries its own eight values in `parameters` instead, in the lit pass and the shadow pass.
`RenderWorld.timeSeconds` is the clock `mat_time` reads: the game moves it on each frame (`AdvanceTime`), and the
editor sets it from SDL's clock, so a moving material moves in the viewport and the preview too.

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
lantern:::world:::SetMaterialShader("painted_metal", "../game-assets/materials", "painted_metal", Texture("textures/wall.png"), 0, 0, 0, "metallic=0.6;wear=0.3");
```

The last argument names the shader's parameters in slot order with the values they start at.

The engine's shader folder is found from the assets folder (the checkout they sit in), or from `LANTERN_ENGINE_SHADERS`.
A graph that can't be a material stops the bake with the node at fault, and the previous files stay. The package keeps
the compiled `.spv` and `.msl` and drops the graph, the Silhouette and the `.frag`.

`SetMaterialShader` asks the renderer for the material shader and gives it to the material's look, so every piece placed
with that material draws with it. A graph material's look still takes the Silhouette's tint (keep it `#FFFFFF` to let the
graph decide the colour). A shader that fails to load logs a warning and the material keeps its plain look.

A game changes a parameter while it runs by the name its graph gave it:

```nyx
lantern:::world:::SetMaterialParameter("painted_metal", "wear", 0.8f);
```

It returns false, and changes nothing, when the material has no shader or no parameter by that name. The change applies
to every piece with that material. One piece can have a value of its own instead:

```nyx
lantern:::world:::SetEntityMaterialParameter(door, "wear", 0.1f);
```

The first value a piece gets copies the material's parameters as they stand, with that one changed; from then on the
piece keeps its own, and a later `SetMaterialParameter` no longer reaches it. It returns false when the entity has no
graph material or no parameter by that name. Test:
`engine/tests/host/a-game-changes-a-material-parameter-while-it-runs`.

`SetMaterialShader`'s last argument names what else the shader does: `masked` when it has a shadow shader beside it and
`translucent` when it is see-through, separated by a space, or empty.

The platformer's sample is `assets/silhouettes/materials/painted_metal`: blue paint over the tiled wall texture, with
its roughness from the `wear` property. Test: `engine/tests/editorbackend/the-game-materials-are-built-and-current`
checks the baked files are current and what the bake writes for it.

## In the editor

Window > Graph Editor opens the Graph tab, which lists every material Silhouette under **Materials** beside the script
graphs. Clicking one opens its graph as `material:<name>` on the same canvas script graphs use. A material that has no
graph yet is shown as "(new graph)", and opening it writes a starter graph that looks like the Silhouette already does:
its texture times its tint as the base colour, and its metallic and roughness as parameters.

In a material graph the right-click menu offers the material kinds (inputs first, then maths) and a comment; the output
is made with the graph and a second one is refused. Each node shows its pins and the value or link each one holds, and
the side panel edits a node's fields, with the output's seven ports labelled base colour, metallic, roughness, emissive,
normal, opacity mask and opacity. A colour node's `text` is typed as `#RRGGBB`. The text under the canvas is the generated GLSL, or the line
saying which node stops it.

**Save and compile** writes `material.graph.json` beside the Silhouette and runs the bake, so the game's compiled shader
is current at once. The preview panel shows the material as last saved on a turning sphere in a studio: a dark backdrop,
a warm key light, a cool rim light and a bright band of sky light around the horizon that metals reflect, so metallic,
roughness and normal maps all read at a glance. A see-through material gets a checkered wall behind the sphere, so what shows through it reads too. The level viewport and the content browser draw every graph material
with its own shader too. **Hide shader** folds the generated GLSL away to give the canvas more room, and the panels only
script graphs use (functions and variables) are hidden while a material is open.

A material's own variables are its parameters, so a material graph has a **Parameters** panel on the left in their place.
It lists each `mat_param` in the graph as `name = default`, with a **Get** button that puts another node for it on the
canvas, and a name and default box with **Add parameter** to make a new one. Any input pin that shows a typed value, such
as a scalar's `value` or a multiply's `b`, takes a wire instead: drag a parameter's output onto it and the value is
replaced by the wire, so one parameter can drive several nodes and a game can change it while it runs.

Two sample materials show the range: `gold` (a colour, full metal and a polished roughness parameter) and `brick` (a
tiled brick texture, its normal map with a `bumpiness` parameter, and a rough non-metal). Their textures,
`textures/brick.png` and `textures/brick_normal.png`, are made by `mist run assets` (`engine/tools/generate-assets/Bricks.nox`).
Three more show the moving and world nodes: `neon_pulse` (a pink glow that pulses with `mat_time` and `mat_sine` and
brightens at the edges with `mat_fresnel`), `lava_flow` (the ground texture sliding under a `mat_panner`, glowing orange)
and `vent_grille` (steel slats cut by an opacity mask from the sine of the world height, whose shadow has its slats).
`sea_glass` is see-through: a pale teal whose opacity is a `thickness` parameter plus a `mat_fresnel`, so its edges are
more solid than its face, as glass is.

Tests:
`engine/tests/graphs/a-material-graph-offers-material-nodes` and
`engine/tests/materialgraph/a-material-graph-opens-in-the-editor`.

## Still to come

Tracked on the board under `material-graphs-an-unreal-style-material-editor-that-generates-a-shader-per-material`:

- Per-piece values set in the editor and saved with the level; today a game sets them while it runs.
- See-through pieces are sorted by their origin, so two that overlap can blend in the wrong order, and they cast no
  shadow (a coloured shadow would need its own pass).
- Refraction: glass and water blend over what is behind them but don't bend it.
