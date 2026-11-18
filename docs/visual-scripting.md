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

- A game keeps graphs in `games/<name>/game-graphs/`. A build step (`lantern:::graphs:::GenerateAll`) writes `game-graphs/generated/<Name>.nyx`, which is imported like any `.nyx` file. The generated file starts with a line saying it is generated and is never edited by hand.
- A **Silhouette** component can name a graph in its values (`behaviour.graph`, `behaviour.function`); the game's existing entity systems call it. The first game use is one small graph in the platformer replacing a hand-written behaviour.

## Slices

1. **Model, format and generator.** `lantern:::graphs`: load and save `.graph.json`, check it (one entry, no cycles in data, every reference resolves, types agree), generate Nyx. Tested by mounting the generated Nyx on a headless runtime and calling it.
2. **Mixing with Nyx.** `call` and `nyx` nodes, with typed pins taken from the host surface manifest.
3. **The editor tab.** Canvas, palette, wiring, details, Nyx pane.
4. **In a game.** The build step, the `behaviour.graph` property on Silhouettes, and one platformer behaviour as a graph.
5. **Later.** Loops and `when`, event entries (`OnTick`, `OnTouch`), per-node debugging (highlight the node that threw), comment boxes, reroutes, copy and paste.

## Known open questions

- Whether the generated `.nyx` is committed or only built; slice 4 starts with built-only and revisits.
- How a graph names a class's methods (a graph function is free, not a method) before event entries exist.
