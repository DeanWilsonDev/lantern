# Adding a level and a mechanic

A walk through the two things you'll do most in the platformer: make a new level, and give it something to do. It uses
the code that is already there (the gem, the lantern and the checkpoint are the models), so you can read the real files
beside it. The calls are in `game-api.md`, and `crimson-and-thorn-slice.md` shows two finished mechanics.

The mechanic built here, **speed shoes**, is an example of the pattern and isn't in the repository. Its snippets are
adapted from the lantern pickup and use the same calls, but they haven't been run, so expect to fix a typo if you type
them in. Lantern is a tutorial repository: write comments in your files that teach the technique (`decisions.md` D-042).

## How the game is put together

Every piece of the platformer sits in one folder per kind of module, and `main.nyx` owns one object of each and passes
them to each other:

| Folder | Holds | An example |
| --- | --- | --- |
| `game-assets/` | loading textures, meshes and sounds | `game-assets.nyx` |
| `game-scenes/` | a level: the world file plus what the game reads from it | `level-scene.nyx` |
| `game-entities/` | one thing in the level, wrapping a world entity | `gem.nyx`, `lantern-pickup.nyx` |
| `game-systems/` | work on many entities each step, reporting what it found as events | `gem-system.nyx` |
| `game-events/` | what can happen in a step | `game-event.nyx` |
| `game-rules/` | what each event means | `game-rules.nyx` |
| `settings/` | numbers: tuning, input, lighting | `player-tuning.nyx` |
| `ui/` | Chaos components and Cosmos styles | `main-menu.chaos` |

The engine knows none of this. It loads a world file into entities with a `kind` label and properties, and the game decides
what each kind means. A new mechanic is therefore usually a new kind plus a file or two in each of entities, systems and
rules.

## Before you start

File a card on the board first (`cairn --root docs/kanban new task <parent> "Name"`), put a question on it if a design
choice is open, and add a decision to `decisions.md` when you settle one. If the engine can't do what you need, that's a card
on `engine-follow-ups`, and the workaround is named in the code.

## Part 1: a new level

### The world file

A level is a JSON world file in `assets/worlds/`. The editor writes it; you can also write one by hand. The shape
(`engine/src/world/WorldFile.nox` is the authority):

```json
{
  "format_version": 1,
  "name": "playground",
  "entities": [
    {
      "id": 1,
      "name": "floor",
      "kind": "block",
      "transform": { "position": [0, -0.5, 0], "rotation_degrees": [0, 0, 0], "scale": [1, 1, 1] },
      "shape": { "type": "Box", "half_extents": [8, 0.5, 8] },
      "material": "ground",
      "collider": "static"
    },
    {
      "id": 2,
      "name": "player_spawn",
      "kind": "spawn",
      "transform": { "position": [0, 0, 0], "rotation_degrees": [0, 0, 0], "scale": [1, 1, 1] },
      "properties": { "chapter": 0 }
    },
    {
      "id": 3,
      "name": "kill_plane",
      "kind": "kill_plane",
      "transform": { "position": [0, -8, 0], "rotation_degrees": [0, 0, 0], "scale": [1, 1, 1] }
    }
  ]
}
```

- A **piece** has a `shape` (`Box` or `Ramp`), a `material` (`ground`, `wall` or `platform`: the names `SetMaterial` gives a
  texture in `game-assets.nyx`) and a `collider`: `static` for fixed geometry, `kinematic` for something the game moves
  (a moving platform), and the engine builds its mesh and collision.
- A **marker** has only a transform and a `kind`. The game gives it meaning: `spawn` is where the player starts (its
  `chapter` property picks the look from `settings/chapter-looks.nyx`), `kill_plane` is a height below which the player has
  fallen, `gem` is a collectable. Any other kind is a free label.
- `properties` is free-form data per entity, read from Nyx with `lantern:::world:::NumberProperty`, `TextProperty` and
  `VectorProperty` (and `HasProperty` first when it's optional).

The fastest way to make one is the editor: `mist run editor`, then File, New, place blocks and markers from the Entity menu
(`editor.md`), and Save into `assets/worlds/`. Copy a shipped level (`course.world.json`, `streets.world.json`) when you
want a model.

### Try it without touching the game

```sh
LANTERN_LEVEL=worlds/playground.world.json mist run game ../games/platformer
```

`LANTERN_LEVEL` replaces the level the menu's **Play** button starts. The editor's Play button sets it the same way.

### Put it on the menu

Three small edits, each following **The streets**:

1. `ui/menu-choice.nyx`: add `PlayPlayground` to the `MenuChoice` enum.
2. `ui/main-menu.chaos`: add `<MenuButton label="The playground" ui={ui} choice={MenuChoice:::PlayPlayground} />`.
3. `main.nyx`: add `static PLAYGROUND_PATH :: "worlds/playground.world.json";` beside `STREETS_PATH`, and a case in
   `HandleMenuChoice`: `PlayPlayground -> { PlayLevel(PLAYGROUND_PATH); },`.

Save and press F5 in the running game to see it. If the level has its own look, add a chapter to
`settings/chapter-looks.nyx` and set `chapter` on the spawn.

## Part 2: a new mechanic, speed shoes

The idea: a pair of shoes lies in the level; touching them makes the player run faster for six seconds. It exercises every
layer: a world kind, an entity, a system, an event, a rule and a debug value.

### 1. The world kind

Place a marker with `"kind": "speed_shoes"` in the world file (by hand, or add a template to `assets/editor/palette.json`
so the editor's Entity menu offers it, with a `markers` entry saying how to draw it: `{ "mesh": "gem", "color": "#FF9A3D" }`).
The engine needs nothing: it already loads an entity with that kind.

### 2. The entity, `game-entities/speed-shoes.nyx`

An entity class wraps one world entity. This is the lantern pickup with the name changed:

```nyx
Math :: import standard:::Math;

SpeedShoes :: class {
public:
    entity: i64;
    home: lantern:::math:::Vec3;

    SpeedShoes~: (entity: i64) : entity(entity), home(lantern:::world:::Position(entity)) { }

    IsTaken :: () -> bool {
        return !lantern:::world:::IsVisible(entity);
    }

    Take :: () -> void {
        lantern:::world:::SetVisible(entity, false);
    }

    Restore :: () -> void {
        lantern:::world:::SetVisible(entity, true);
    }

    Animate :: (time: f32) -> void {
        bob: lantern:::math:::Vec3 = { y: Math:::sin(time * 1.5f) * 0.08f };
        lantern:::world:::SetPosition(entity, home + bob);
        lantern:::world:::SetYaw(entity, time);
    }
}
```

A taken pickup is hidden, not despawned, so restarting the level can show it again without reloading the file.

### 3. The event, `game-events/game-event.nyx`

Add a case to the `GameEvent` enum:

```nyx
SpeedShoesTaken(shoes: i64),
```

Events are what systems say and rules listen to. Keep the list alphabetical, as it is.

### 4. The system, `game-systems/speed-shoes-system.nyx`

A system looks at many entities each fixed step and pushes an event for what it finds. It decides nothing. This is
`lantern-system.nyx` with the names changed, and it reuses `TriggerTest`, the capsule-against-sphere test:

```nyx
GameEvent :: import GameEvent;
GameEvents :: import GameEvents;
SpeedShoes :: import SpeedShoes;
TriggerTest :: import TriggerTest;

SpeedShoesSystem :: class {
public:
    static TRIGGER_RADIUS :: 0.7f;

    DetectTaken~: (shoes: Array<SpeedShoes>, capsuleCenter: lantern:::math:::Vec3, halfSegment: f32, capsuleRadius: f32, events~ GameEvents) -> void {
        for pair: SpeedShoes in shoes {
            guard !pair.IsTaken() continue;
            center := lantern:::world:::Position(pair.entity);
            if TriggerTest:::Overlaps(center, TRIGGER_RADIUS, capsuleCenter, halfSegment, capsuleRadius) {
                events.Push(GameEvent:::SpeedShoesTaken(pair.entity));
            }
        }
    }
}
```

### 5. The scene, `game-scenes/level-scene.nyx`

The scene finds the entities by kind and keeps them in lists. Three additions, each next to what `lanterns` does:

```nyx
speedShoes~ Array<SpeedShoes> = [];
```

in the fields; in `Load`, after the lanterns loop:

```nyx
for entity: i64 in lantern:::world:::FindByKind("speed_shoes") {
    speedShoes.add(new SpeedShoes(entity));
}
```

and `speedShoes = [];` in `Unload`. Add `SpeedShoes :: import SpeedShoes;` at the top: imports aren't transitive, so every
class a file names is imported there.

### 6. The rule, `game-rules/game-rules.nyx`

The rules say what the event means. Add a field (rules keep their state in `~` fields that `main.nyx` can read, as it reads
`rules.isCarryingLantern`), a case, a handler beside `TakeLantern`, and the imports `PlayerTuning :: import PlayerTuning;` and
`SpeedShoes :: import SpeedShoes;` at the top:

```nyx
shoesSecondsLeft~ f32 = 0.0f;

// in Handle's `when event <GameEvent>`:
SpeedShoesTaken(entity) -> { TakeSpeedShoes(entity, level, player); },

TakeSpeedShoes~: (entity: i64, level: LevelScene, player~ Player) -> void {
    for pair in level.speedShoes {
        guard pair.entity == entity continue;
        pair.Take();
        player.controller.SetWalkSpeed(PlayerTuning:::WALK_SPEED * 1.6f);
        shoesSecondsLeft = 6.0f;
        lantern:::audio:::Play("pickup", 0.6f);
        ShowMessage("Speed shoes!", 2.0f);
    }
}
```

The message the handler shows is counted down by `Tick` once per display frame. A power-up that ends belongs on the simulation clock, so give
the rules a step method, called from `Simulate` in `main.nyx` with the fixed step's `dt`, that counts `shoesSecondsLeft` down
and calls `player.controller.SetWalkSpeed(PlayerTuning:::WALK_SPEED)` when it reaches zero. Using the fixed step keeps the
effect paused while the game is paused and the same on every run, which is why the gems animate from `simulationSeconds`
too. Reset `shoesSecondsLeft` and the walk speed in `RestartLevel`, and `Restore()` each pair there.

### 7. Wire it up, `main.nyx`

Three lines, each beside its lantern counterpart: a field `speedShoesSystem~ SpeedShoesSystem = new SpeedShoesSystem();`, a
call in `Simulate` after the other detections,

```nyx
speedShoesSystem.DetectTaken(level.speedShoes, controller.CapsuleCenter(), controller.HalfSegment(), controller.Radius(), events);
```

and `pair.Animate(simulationSeconds)` for each pair in `UpdateGlows`'s loop over pickups. Add
`SpeedShoes :: import SpeedShoes;` and `SpeedShoesSystem :: import SpeedShoesSystem;` at the top.

Publish a value so you can see it work: in the debug section next to `Game/Gems`,

```nyx
lantern:::debug:::SetValue("Game/Shoes", `${rules.shoesSecondsLeft:.1f}`);
```

Run the game with the debug HUD (F1, or `LANTERN_DEBUG_HUD=1`), walk into the shoes and watch it count down from 6.

## Part 3: test it

The engine's tests run the real platformer headless: `engine/tests/host/walking-into-a-gem-collects-it` is the model. A test
is a folder with a `main.nox`, a `nox.json` naming the sources it needs (copy the gem test's) and an `expected.txt`:

```nox
host := lantern:::testing:::StartPlatformer(manifest, false);
platform := lantern:::testing:::NewPlatformerPlatform();
frameLoop := lantern:::testing:::NewFrameLoop();
lantern:::testing:::PressKey(platform, lantern:::core:::KeyCode:::Enter);
lantern:::testing:::RunQueued(frameLoop, platform, host);
lantern:::testing:::HoldKey(platform, lantern:::core:::KeyCode:::W, 40);
lantern:::testing:::RunQueued(frameLoop, platform, host);
lantern:::testing:::AssertNoScriptError(host, "after walking");
assert(lantern:::testing:::DebugText(host, "Game/Shoes") != "0.0", "the shoes are on");
```

Put the shoes in a level the test can reach: the streets test chooses its level from the menu with the Down and Enter keys, so
a test for a new level adds a menu button as in Part 1 and presses to it. The values the HUD shows
(`DebugText(host, "Game/Shoes")`) are how tests read the game's state. Run it with `mist run test`.

## Checklist

- A card on the board before you start, a decision recorded when you settle one.
- A comment in each new `.nyx` and `.nox` file that teaches the technique, not just what the line does.
- A new engine call needs the host registration, the manifest (`engine-host-surface.json`, see `game-api.md`) and a test, in
  that order; you don't need one for a game-only mechanic like this.
- `docs/README.md` links the document that explains it, and `known-gaps.md` lists what you left out.
