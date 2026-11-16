# Lantern documentation

Every document in this folder, grouped by what you want to do. Lantern is a tutorial repository, so the guides are written
to be read in order: the first three under "Start here" give you the whole picture.

## Start here

| Document | What it tells you |
| --- | --- |
| [`../README.md`](../README.md) | What Lantern is, how to run it, and what's where. |
| [`running.md`](running.md) | Installing `nox` and `mist`, then building, playing, testing and editing on macOS, Arch Linux, other Linux and Windows, with every environment switch. |
| [`engine-structure.md`](engine-structure.md) | How `engine/src/` is laid out, one folder per domain, with SDL behind a facade folder, and why. |
| [`application-and-game-loop.md`](application-and-game-loop.md) | Start-up, the fixed-step game loop, input, and shutdown in order. |
| [`renderer.md`](renderer.md) | The renderer one frame at a time, from the game's `RenderWorld` to the screen. |

## Writing a game

| Document | What it tells you |
| --- | --- |
| [`game-api.md`](game-api.md) | Every host function and type a Nyx game can call with its signature, examples from the platformer, and the hooks the engine calls. |
| [`adding-a-level-and-a-mechanic.md`](adding-a-level-and-a-mechanic.md) | A tutorial: make a level, put it on the menu, then add a mechanic through an entity, a system, an event and a rule, and test it. |
| [`crimson-and-thorn-slice.md`](crimson-and-thorn-slice.md) | Which design mechanics the platformer takes on, in what order, and where each lives in `games/platformer/`. |
| [`gameplay-prototypes.md`](gameplay-prototypes.md) | Twelve gameplay ideas, the companion's command model, the six prototype rooms, the crossing level and every tunable. |
| [`grounded-adventure.md`](grounded-adventure.md) | The grounded feel, player-only climbing, the word and food minigames, the station and its trains, and every tunable. |
| [`grounded-adventure-ideas.md`](grounded-adventure-ideas.md) | Five more ideas to test for the action-adventure direction, with how each would be fun and what it costs. |
| [`playtest-checklist.md`](playtest-checklist.md) | What to try in each prototype room, with and without her, and what to note. |
| [`night-look.md`](night-look.md) | Why the platformer is played at night, the moon, sky and lamps that make it readable, and the day look. |
| [`photo-mode.md`](photo-mode.md) | The photo collectable: raising the camera, what counts as a photo, and how sights are authored. |

## The renderer's effects

[`graphics-effects.md`](graphics-effects.md) is the map: which effects exist, what each quality level turns on and off,
and what each costs. [`quality-settings.md`](quality-settings.md) is the guide to choosing a level, tuning one effect and
finding what to turn down when a frame is slow. Each effect has its own guide.

| Document | Effect |
| --- | --- |
| [`lighting-and-shadows.md`](lighting-and-shadows.md) | The sun, point and spot lights, materials, and shadow maps. |
| [`shadow-softness-and-cascade-blending.md`](shadow-softness-and-cascade-blending.md) | The shadow filter's width and sample count, and the fade between cascades. |
| [`bloom-and-tone-mapping.md`](bloom-and-tone-mapping.md) | The high dynamic range scene, exposure, tone mapping curves and bloom. |
| [`sky-and-image-based-lighting.md`](sky-and-image-based-lighting.md) | The procedural sky, and the ambient light and reflections it gives. |
| [`anti-aliasing.md`](anti-aliasing.md) | FXAA. |
| [`ambient-occlusion.md`](ambient-occlusion.md) | Screen-space ambient occlusion over the depth buffer. |
| [`fog-and-light-shafts.md`](fog-and-light-shafts.md) | Height fog and sun light shafts. |
| [`blob-shadows.md`](blob-shadows.md) | Soft dark patches under characters. |
| [`colour-grade.md`](colour-grade.md) | Saturation, contrast, tint and vignette after tone mapping. |
| [`particles.md`](particles.md) | Dust, sparks and trails. |
| [`normal-maps.md`](normal-maps.md) | Fine surface bumps from a normal map. |

## The world editor

| Document | What it tells you |
| --- | --- |
| [`editor.md`](editor.md) | The editor's architecture, its controls, play in editor, the C API rules and its gaps. |
| [`../apps/world-editor/nox/README.md`](../apps/world-editor/nox/README.md) | The editor's Nox host: its files and how to run it. |

## Decisions, status and history

| Document | What it tells you |
| --- | --- |
| [`decisions.md`](decisions.md) | Every decision made along the way with reasons and how to undo it (D-001 to D-073, in order). |
| [`known-gaps.md`](known-gaps.md) | What hasn't been checked, what isn't built and what is half-way. |
| [`future-work.md`](future-work.md) | Ideas out of scope for the prototype, and the experiments to try next. |
| [`dependencies.md`](dependencies.md) | Each third-party library and why, for the Nox engine and for the Rust reference. |
| [`kanban/`](kanban/) | The board that tracks the work, in the format the `cairn` tool reads (`cairn --root docs/kanban tree`). |

## The Rust reference engine

Lantern began as a Rust engine (`crates/`) and was rewritten in Nox. These documents describe the original and are kept
as history and for comparison.

| Document | What it tells you |
| --- | --- |
| [`brief.md`](brief.md) | The original brief: what the prototype was for and its milestones. |
| [`architecture.md`](architecture.md) | A tour of the Rust engine's crates, the per-frame sequence, ownership and shutdown. |
| [`renderer_illustrated.md`](renderer_illustrated.md), `renderer_illustrated.pdf` | An illustrated, printable edition of the renderer walkthrough, from before the post-processing effects. |

## Screenshots

`game-ui-main-menu.png`, `game-ui-pause-menu.png` and `game-ui-hud.png` show the game's Chaos UI.
`editor-screenshot.png` (used by `editor.md`), `editor-nox-screenshot.png` and `editor-cpp-screenshot.png` show the world
editor. Before and after pictures for each renderer effect are attached to the pull request that added it.

## Conventions

- Lantern's names are `lantern:::<lowercase-namespace>:::Name` (types are PascalCase), and no document names another
  engine.
- Documents are plain Markdown.
- Every `.nox` and `.nyx` file explains itself in comments written for someone learning the technique; keep that up when you
  change one.
- A change that adds a decision adds the next `D-` number to `decisions.md`, and work is tracked as a card on the board
  before it starts.
