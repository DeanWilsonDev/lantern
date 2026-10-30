# Game API

Everything a Nyx game can call in the engine. This is the whole seam: game code (Nyx, in
`games/<name>/`) reaches engine code (Rust) only through these host types, and the engine reaches
the game only through the hooks below. The registrations are in `crates/game-host/src/api/`, one
module per group.

Rules that hold for every call:

- **Handles are `i64`.** Entities, meshes and textures are generational handles packed into 64
  bits. A script can't hold a dangling pointer: a stale handle throws `Error:::NotFound`.
- **Vectors are `Vec3` and `Vec2` values.** Build one with braces (`Vec3 { x: 1.0f, y: 2.0f }`,
  missing fields are 0), read and write `x`, `y` and `z` as fields, and get a copy at every
  binding, argument and return. Arithmetic is operators with the vector on the left:
  `a + b`, `a - b`, `v * s`, `v / s` and `-v`.
- **Colours are strings**: `"RRGGBB"` or `"RRGGBBAA"`, with or without `#`.
- **Paths are relative to the assets directory** (`game.json`'s `assets`, or `LANTERN_ASSETS`).
- **Everything a game makes is released on reload and on exit**: entities, meshes, textures,
  materials, music, input definitions and the game's debug values.
- A call that can't do what it was asked throws `Error:::NotFound` (a missing file, entity or
  property) or `Error:::Unknown` (anything else). The hook fails, the engine shows the error with
  the Nyx file and line it came from and the calls that led there, keeps running, and F5 reloads
  the game.

## Hooks

The game's entry file (`game.json`'s `entry`) declares a class deriving `Application` (named by
`application_class`) with `Run :: () -> void { }`. The engine calls these methods on it, and
skips any the class doesn't define.

| Hook | When | Typical work |
|---|---|---|
| `OnStart~: () -> void` | once, after loading | define input, load assets, load the world |
| `OnFixedUpdate~: (dt: f32) -> void` | every fixed step (`dt` = 1/60 s) | the simulation: controllers, platforms, rules |
| `OnUpdate~: (frameSeconds: f32) -> void` | once per display frame | menu state, camera look, timers |
| `OnRender~: (alpha: f32, frameSeconds: f32) -> void` | once per drawn frame | camera, menus, HUD text, debug drawing |
| `OnExit~: () -> void` | before unloading (quit or F5) | save, stop music |

`alpha` blends the last two fixed steps (0..1): use `World:::InterpolatedPosition` with it.

The engine keeps two keys for itself: **F5** reloads the game from disk, and **F1** (gamepad
Back) toggles the debug HUD.

## Engine

| Call | |
|---|---|
| `Engine:::Log(text: string) -> void` | info log, target `game` |
| `Engine:::Warn(text: string) -> void` | warning log |
| `Engine:::Quit() -> void` | quit after this frame |
| `Engine:::Flag(name: string) -> bool` | an environment variable that is set, not empty and not `"0"` (tests override it) |
| `Engine:::Setting(name: string) -> string` | an environment variable's text, `""` when unset (`LANTERN_LEVEL` is the level the world editor's Play button launched) |

## Input

The game names its actions, binds devices to them, then asks about actions. Edges are latched
for fixed steps: a press is seen by exactly one fixed step even if a frame runs none.

| Call | |
|---|---|
| `Input:::DefineButton(name) / DefineAxis(name) / DefineVector(name)` | declare an action |
| `Input:::BindKey(action, key)` | a key to a button |
| `Input:::BindMouseButton(action, button)` | `Left`, `Right`, `Middle` |
| `Input:::BindGamepadButton(action, button)` | `South`, `East`, `West`, `North`, `Back`, `Start`, `LeftStick`, `RightStick`, `LeftShoulder`, `RightShoulder`, `DPadUp`, ... |
| `Input:::BindGamepadAxisButton(action, axis, threshold)` | an axis past a threshold as a button |
| `Input:::BindKeyAxis(action, negative, positive)` | two keys to an axis |
| `Input:::BindGamepadAxis(action, axis, scale)` | `LeftX`, `LeftY`, `RightX`, `RightY`, `LeftTrigger`, `RightTrigger` |
| `Input:::BindKeys(action, up, down, left, right)` | four keys to a vector |
| `Input:::BindStick(action, stick, scale)` | `Left` or `Right` stick to a vector |
| `Input:::BindMouseMotion(action, scale)` | mouse motion to a vector |
| `Input:::IsPressed(action) / IsReleased(action) / IsHeld(action) -> bool` | button state |
| `Input:::Axis(action) -> f32`, `Input:::Vector(action) -> Vec2` | analogue state |

Keys are `A`..`Z`, `Digit0`..`Digit9`, `Up`, `Down`, `Left`, `Right`, `Space`, `Enter`,
`Escape`, `Tab`, `Backspace`, `LeftShift`, `LeftControl`, `F1`..`F12` and so on: the names of
`lantern_core::input::KeyCode`.

## World

| Call | |
|---|---|
| `World:::Load(path) -> i32` | spawn a world file's entities, meshes and collision; returns the entity count |
| `World:::Clear() -> void` | remove every game entity, the world meshes and all collision |
| `World:::Spawn(name, position: Vec3) -> i64` | an empty entity |
| `World:::Despawn(entity)` | |
| `World:::Find(name) -> i64` | the first entity with that name; throws `NotFound` |
| `World:::FindByKind(kind) -> Array<i64>` | world-file entities of a kind (`gem`, `spawn`, ...) |
| `World:::Kind(entity) -> string` | |
| `World:::HasProperty(entity, key) -> bool` | |
| `World:::NumberProperty / TextProperty / VectorProperty(entity, key)` | world-file properties |
| `World:::SetMaterial(material, texture: i64)` | the texture world-file pieces with that material use (set before `Load`) |
| `World:::SetMesh(entity, mesh, tint)`, `SetTexturedMesh(entity, mesh, texture, tint)` | |
| `World:::SetTint(entity, tint)`, `SetVisible(entity, visible)`, `IsVisible(entity) -> bool` | |
| `World:::SetParent(child, parent)` | transforms become relative to the parent |
| `World:::Position(entity) -> Vec3` | local position |
| `World:::WorldPosition(entity) -> Vec3` | |
| `World:::InterpolatedPosition(entity, alpha) -> Vec3` | for `OnRender` |
| `World:::SetPosition(entity, position)` | also moves the entity's collision body, carrying riders |
| `World:::Teleport(entity, position)` | moves without interpolating from the old place |
| `World:::SetYaw(entity, radians)`, `SetScale(entity, scale: Vec3)` | |

World files are made by the editor (`crates/world-format`). The engine builds what's in them;
the game decides what a kind means.

## Assets

| Call | |
|---|---|
| `Textures:::Load(path) -> i64` | a PNG; throws `NotFound` |
| `Textures:::Checker(size, cells, first, second) -> i64`, `Textures:::Solid(color) -> i64` | generated |
| `Meshes:::Load(path) -> i64` | a glTF/GLB mesh; throws `NotFound` |
| `Meshes:::Capsule(radius, halfHeight, segments, rings)`, `Sphere(radius, segments, rings)`, `Cube(size)`, `Gem(radius, height)` | generated, `-> i64` |

## Physics

| Call | |
|---|---|
| `Physics:::Raycast(origin, direction, maxDistance) -> RayHit?` | `hit.Distance()`, `hit.Point()`, `hit.Normal()` |
| `Physics:::SphereCast(radius, origin, motion) -> f32?` | the fraction of `motion` before contact, or null |
| `new CharacterController(radius, height, feet: Vec3)` | a kinematic capsule: walks, jumps, climbs steps, rides platforms |
| `controller.Step(moveDirection: Vec3, jumpPressed, jumpHeld, dt) -> ControllerStep` | one fixed step; `Jumped()`, `Landed()`, `LandingSpeed()`, `HitCeiling()` |
| `controller.Teleport(feet)`, `FeetPosition()`, `CapsuleCenter()`, `Velocity()`, `GroundNormal()`, `IsGrounded()` | |
| `controller.Radius()`, `Height()`, `HalfSegment()` | the capsule's axis is `CapsuleCenter() ± HalfSegment()` in y |
| `controller.SetWalkSpeed(m/s)`, `SetJumpHeight(m)`, `SetGravity(m/s²)` | feel; the rest of `ControllerSettings` is engine tuning |

## Rendering and screen

| Call | |
|---|---|
| `Camera:::LookAt(eye, target)` | this frame's camera |
| `Renderer:::SetClearColor(color)`, `Renderer:::SetFog(start, end)` | |
| `Screen:::Width() / Height() -> f32` | pixels |
| `Screen:::UiScale() -> f32` | whole-pixel text scale for the display density |
| `Screen:::SetMouseCaptured(captured)` | applied after this frame's `OnUpdate` |

## GameUi (Chaos and Cosmos)

The game's menus and HUD are Chaos components (`.chaos`) styled by Cosmos stylesheets
(`.cosmos`), all in the game's `ui/` folder (D-043). Every `.cosmos` file there is loaded at start:
`global.cosmos` as the global sheet, and all the others together as one shared component sheet.
So a rule in `hud.cosmos` can match an element in a menu, `:root` variables from every file share
one namespace, and a class defined in two files isn't reported (the later file, by name, wins).
Keep class names unique across the folder.

| Call | |
|---|---|
| `GameUi:::Mount(file, component) -> i32` | mounts `component` from `file` (relative to the game directory), passing the game's `Application` object as its one argument: `GameScreen :: (game~ Platformer) -> Lantern:::Component`. Call it from `OnStart` or later |
| `GameUi:::Unmount(mount)` | |
| `GameUi:::SetNavigation(previous, next, confirm)` | the input actions (by name) that move focus and press the focused button |

- **State**: a field marked `@signal` on a game object re-renders every `<slot>` that read it
  when the game writes it. Write only when the value changes, or the slot re-renders every
  frame.
- **Buttons** are elements with `onPress` or `onRelease`. Focus starts on the first one and moves in the order
  they appear; moving calls the new button's `onFocus`, confirming calls `onPress` then
  `onRelease`. Handlers run just before `OnUpdate`, so a handler should record a choice for
  `OnUpdate` to act on.
- **Drawing**: the engine lays the UI out and draws it after `OnRender`, on top of `Overlay:::`
  drawing. A Cosmos `px` is one window point (`Screen:::UiScale() / 2` pixels). `font-size`
  snaps to multiples of 8, the bitmap font's size. The focused button and its contents use
  their `:hover` style (write `.label:hover`, not a rule nested in `:hover { }`), and `:active`
  while confirmed.
- **Supported elements**: `<frame>` and `<text>`. Supported properties: `background-color`,
  `background-gradient-start` and `background-gradient-end` (top to bottom), `border-color`,
  `border-width`, `border-radius`, `box-shadow` (colour and blur), `padding`, `margin`,
  `color`, `font-size`, `opacity`, `display: stack` with `flex-direction`, `gap`,
  `align-items`, `justify-content`, `width`, `height`, `min-width`, `max-width`, `flex-grow`,
  `position: absolute | fixed` with `top`, `right`, `bottom`, `left`, and `z-index`. Lengths are
  px, %, vw or vh. A frame stacks its children in a column unless it says otherwise.
- Errors in a component or a handler are reported as `UI: ...` like a failed hook. Stylesheet
  problems are logged as warnings.

## UI and overlay

Call these from `OnRender`. The overlay is cleared every frame. The platformer uses `GameUi`
instead of `Ui:::`; `Ui:::` is the small immediate-mode alternative.

| Call | |
|---|---|
| `Ui:::Begin(top, up, down, confirm)` | start a centred menu column at `top` pixels; the game passes its own navigation edges |
| `Ui:::Title(text)`, `Label(text)`, `Spacing(rows)` | |
| `Ui:::Button(text) -> bool` | true on the frame the focused button is confirmed |
| `Ui:::End()` | (an unfinished column is ended by the engine) |
| `Ui:::Focused() -> i32`, `Ui:::ResetFocus()` | the focused button's index |
| `Overlay:::Rect(x, y, width, height, color)` | screen pixels |
| `Overlay:::Text(text, x, y, scale, color)` | the engine's 8×8 bitmap font |
| `Overlay:::TextWidth(text, scale) / TextHeight(text, scale) -> f32`, `Overlay:::GlyphSize() -> f32` | |

## Audio

| Call | |
|---|---|
| `Audio:::Load(name, path)` | a WAV; a missing sound is logged and stays silent |
| `Audio:::Play(name, volume)` | a one-shot voice |
| `Audio:::PlayMusic(name)`, `Audio:::StopMusic()` | one looping track |

## EngineDebug

| Call | |
|---|---|
| `EngineDebug:::SetValue(path, value: string)` | a HUD line, such as `"Player/Speed"` |
| `EngineDebug:::IsHudVisible() -> bool`, `EngineDebug:::SetHudVisible(visible)` | |
| `EngineDebug:::Capsule(center, radius, halfSegment, color)`, `Sphere(center, radius, color)` | wireframes |
| `EngineDebug:::Arrow(start, vector, color)`, `Line(start, end, color)` | |
| `EngineDebug:::CollisionNear(center, halfExtent, color)` | the collision triangles in a box |

The engine fills the `Frame`, `Script`, `Renderer`, `World`, `Audio` and `Errors` groups itself.

## Math

| Call | |
|---|---|
| `Vec3 { x, y, z }`, `Vec3:::Zero()`, `Up()`, `Down()`, `FromYaw(radians)` | fields `x`, `y`, `z` |
| `a + b`, `a - b`, `v * s`, `v / s`, `-v` | operators, the vector on the left |
| `v.Cross(other)`, `Dot(other)`, `Normalized()` | |
| `v.Horizontal()`, `WithX / WithY / WithZ`, `ClampLength(max)`, `Lerp(other, t)`, `Clamp(min, max)` | |
| `v.Length()`, `LengthSquared()`, `DistanceTo(other)`, `Yaw()` | |
| `Vec2 { x, y }`, `Vec2:::Zero()`; the same operators; `Length`, `ClampLength`, `Normalized` | fields `x`, `y` |
| `Angle:::Wrap(radians)` | to -π..π |
| `Angle:::ToDegrees(radians)` | |

Nyx's own `Math` (`sin`, `cos`, `tan`, `atan2`, `asin`, `acos`, `sqrt`, `min`, `max`, `clamp`,
`TAU`, ...) needs no host.
