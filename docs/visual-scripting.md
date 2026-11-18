# Visual scripting

A graph is a way to write a piece of game logic by wiring nodes together instead of typing it. Lantern's graphs sit **beside Nyx, not instead of it**: a graph is turned into ordinary Nyx source, and the game runs that Nyx on the same interpreter, through the same host API, as everything else. Nox (the engine) never learns that graphs exist.

## The idea in one picture

```
 .graph.json  ──generate──▶  .nyx text  ──mount──▶  Nyx runtime  ──host calls──▶  Nox engine
   (nodes)                  (a namespace of            (as today)                  (lantern:::world:::…)
        ▲                     functions)
        └── a node can hold a Nyx snippet, or call any Nyx function
```

- A graph is **a namespace of functions**. `graphs/bounce.graph.json` becomes `Bounce :: namespace { … }` in generated Nyx, one function per graph.
- Hand-written Nyx **calls a graph like any function** (`Bounce:::Update(entity, dt)`).
- A graph **calls Nyx like any function**: a *Call* node names a Nyx function by path and gets one pin per parameter. Engine calls (`lantern:::world:::Position`) are Nyx calls, so a graph reaches the engine only through the host API, the same seam game code has (`docs/game-api.md`).
- A graph can hold **a bit of Nyx**: a *Nyx* node is a short block of text with named inputs and one output. That is how you "do a bit of Nyx and a bit of visual scripting" in one function without leaving the graph.
- Because the output is Nyx, the existing tools work on it unchanged: `nyx check`, the language server, error lines, the debugger.

## Why generate Nyx instead of interpreting graphs

| Option | Cost | Verdict |
|---|---|---|
| Interpret the graph in Nox | a second language with its own types, errors, debugging and hot reload | no |
| Compile the graph to Nox | the game would need a native rebuild per edit; game code is Nyx by rule (D-023) | no |
| **Generate Nyx text** | one small compiler; every Nyx feature, tool and hot reload applies | yes |

A graph therefore cannot do anything Nyx can't, and cannot reach anything the host API doesn't offer. It sits on top of Nox the same way the rest of game code does.

## File format

A graph is a JSON file (`*.graph.json`) written by the editor and readable in a diff. Calamus was considered; graphs are not entities with inherited values, so strict JSON through the existing `lantern:::json` is simpler.

```json
{
  "graph": "bounce",
  "functions": [
    {
      "name": "Update",
      "inputs": [ { "name": "height", "type": "f32" } ],
      "returns": "f32",
      "nodes": [
        { "id": "start",  "kind": "entry",  "next": "check" },
        { "id": "check",  "kind": "branch", "condition": "low", "then": "up", "else": "keep" },
        { "id": "low",    "kind": "less",   "a": "height", "b": { "f32": 1.0 } },
        { "id": "up",     "kind": "return", "value": { "node": "sum" } },
        { "id": "keep",   "kind": "return", "value": "height" },
        { "id": "sum",    "kind": "add",    "a": "height", "b": { "f32": 0.5 } }
      ]
    }
  ]
}
```

Rules:

- **Execution** nodes (`entry`, `branch`, `set`, `call`, `nyx` with a statement body, `return`) are chained by `next` (and `then` / `else` for a branch). Execution starts at the function's single `entry`.
- **Data** nodes (`add`, `less`, `get`, `literal`, `call` marked pure, `nyx` with an expression body) have no `next`. They are referenced by id wherever a value is wanted, and are generated inline.
- A **value reference** is a string (an input name or a node id) or an object literal (`{ "f32": 1.0 }`, `{ "i32": 3 }`, `{ "bool": true }`, `{ "string": "hi" }`).
- Each function declares its inputs and return type, so the generated Nyx is fully typed.

## Node kinds (first set)

| Kind | Does | Generated Nyx |
|---|---|---|
| `entry` | starts a function | the function header |
| `branch` | if/else on a bool | `if (c) { … } else { … }` |
| `set` | assign a local | `name = value;` (declared on first use) |
| `call` | call any Nyx function | `Path:::Name(args);` |
| `nyx` | a snippet of Nyx | its text, inputs substituted by name |
| `return` | leave the function | `return value;` |
| `add` `sub` `mul` `div` | arithmetic | `(a + b)` |
| `less` `greater` `equal` | comparison | `(a < b)` |
| `and` `or` `not` | logic | `(a && b)` |
| `literal` `get` | a constant, an input or local | the value |

Loops, `when`, and event entry points come after the first slices (see Slices).

## The editor

A **Graph** document tab, beside Level and Silhouette. A node canvas: nodes are boxes with execution pins (white) and data pins (typed colours), wires between pins, drag to move, click a pin and click another to wire, a palette of node kinds, a details panel for the selected node's values, and a read-only **Nyx** pane showing the generated text, so the graph is never a black box. Saving writes the `.graph.json` and the generated `.nyx` side by side.

## Where graphs live and how a game uses one

- The editor keeps graphs in `<assets>/graphs/`; a game keeps its own in `games/<name>/game-graphs/`. A build step (`lantern:::graphs:::GenerateAll`) writes `game-graphs/generated/<Name>.nyx`, which is imported like any `.nyx` file. The generated file starts with a line saying it is generated and is never edited by hand.
- A **Silhouette** component can name a graph in its values (`behaviour.graph`, `behaviour.function`); the game's existing entity systems call it. The first game use is one small graph in the platformer replacing a hand-written behaviour.

## Slices

1. **Model, format and generator.** `lantern:::graphs`: load and save `.graph.json`, check it (one entry, no cycles in data, every reference resolves, types agree), generate Nyx. Tested by mounting the generated Nyx on a headless runtime and calling it.
2. **Mixing with Nyx.** `call` and `nyx` nodes, with typed pins taken from the host surface manifest.
3. **The editor tab.** Canvas, palette, wiring, details, Nyx pane.
4. **In a game.** The build step, the `behaviour.graph` property on Silhouettes, and one platformer behaviour as a graph.
5. **Later.** Loops and `when`, event entries (`OnTick`, `OnTouch`), per-node debugging (highlight the node that threw), comment boxes, reroutes, copy and paste.

## What is built

- Slice 1: `engine/src/graphs/` (`Graph.nox` the model and `LoadGraph`, `GraphNyx.nox` the generator). `lantern:::graphs:::GenerateNyxFromText(jsonText)` returns the Nyx source or the first problem, named by function and node. Node kinds so far: `entry`, `branch` (with an optional `else`, then on to its own `next`), `set` (a `type` declares the local), `return`, the data nodes `add` `sub` `mul` `div` `less` `greater` `equal` `and` `or` `not`, and literals written `{ "f32": 1.0 }`, `{ "i32": 3 }`, `{ "bool": true }`, `{ "string": "hi" }`. Tests: `engine/tests/graphs/` (a graph's Nyx is mounted on a Nyx runtime and called, and eight broken graphs are refused with their messages).

- Slice 2: a `call` node (`function`: any Nyx path, `args`: value references, and `name` plus `type` to keep the result in a local; as a data reference it is the call inline), a `nyx` node (`text`: statements written as typed, in the function's scope, so they see its inputs and locals; `declares`: names it adds for later nodes) and a `nyx_value` node (`text`: one Nyx expression). Brackets in a snippet must balance so it can't close the function around it. Test: `a-graph-mixes-nodes-with-nyx-text` has a graph call a hand-written Nyx function, run a Nyx `for` loop and be called back from Nyx.

- Slice 3a: `GraphFile.nox` (`WriteGraph`, one node per line, which `LoadGraph` reads back to the same text) and `GraphEditing.nox` (`AddNode`, `RemoveNode`, `MoveNode`, `SetPort`, `AddFunction`, `NewGraph`). A wire is a port set to another node's id (`next`, `then`, `else` for execution; `a`, `b`, `value`, `condition`, `arg0`... for values); a literal is `#` and its Nyx text. A refused edit returns a message and changes nothing. Nodes carry an `x` and `y` for the tab. Test: `a-graph-is-edited-and-written-back`.

- Slice 3b: the canvas, in `GraphPins.nox` (where a node's pins are), `GraphEditor.nox` (the open graph and a `PixelCanvas` of RGBA bytes with rectangles, lines, curved wires and text in the engine's 8 by 8 font), `GraphEditorDraw.nox` and `GraphEditorInput.nox`. Nodes are a coloured header (execution in on the left, value out on the right) over rows of pins. Press a node to select and drag it (it snaps to a 20 unit grid on release), press the background and drag to pan, press an output pin and let go over a node or an input pin to wire, right-click a pin to disconnect it, turn the wheel to zoom a whole step, press Delete to remove the selected node. Test: `the-graph-canvas-draws-and-wires-by-pointer`.

- Slice 3c: the Graph tab in the world editor (Window > Graph Editor). `engine/src/editorbackend/GraphFiles.nox` lists, opens, creates and saves graphs in `<assets>/graphs/`, writing `<name>.graph.json` and `<name>.generated.nyx` side by side. `EditorApi.nox` exports the `GraphOpen`, `GraphRender`, `GraphPointer*`, `GraphField`, `GraphSave` family, the app registers them as `editorGraph*` host functions (`apps/world-editor/nox/src/GraphFunctions.nox`), and `GraphPage.chaos` lays out the graph list, functions, the canvas as a Penumbra viewport (`graph` renderer), the node palette, the selected node's fields and the generated Nyx pane.

- Slice 4: `lantern:::editorbackend:::BuildGameGraphs(graphsFolder, outputFolder, writeFiles)` (`GameGraphs.nox`) turns every `<name>.graph.json` into `<Name>.nyx`; `mist run graphs` writes `games/platformer/game-graphs/generated/`, and the test `the-game-graphs-are-built-and-current` fails, naming the file, when a committed file is stale. The platformer's first graph is `crumble_phase`: `CrumblePhase:::Next(phase, isStoodOn, phaseSeconds, delaySeconds, returnSeconds)` picks a crumbling slab's next phase (solid, shivering, gone), and `CrumblePlatform.Update` calls it and only does what each phase looks like. The generated file is committed (D-194). Not done yet: the `behaviour.graph` Silhouette property, because no entity system reads a behaviour from a Silhouette yet; it needs that hook first.

## Known open questions

- How a graph names a class's methods (a graph function is free, not a method) before event entries exist.
