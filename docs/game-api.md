# Game API

Everything a Nyx game can call in the engine. This is the whole seam: game code (Nyx, in
`games/<name>/`) reaches engine code (Nox) only through these host types, and the engine reaches
the game only through the hooks below. The registrations are in `engine/src/host/`, one `*Api.nox`
file per group (`MathApi`, `EngineApi`, `InputApi`, `WorldApi`, `AssetsApi`, `PhysicsApi`, `LightsApi`,
`GraphicsApi`, `SkyApi`, `AtmosphereApi`, `ParticlesApi`, `GameUiApi`, `DebugApi`).

The complete list, with every signature, is the "Signature index" at the end of this page. It is
written from `games/platformer/engine-host-surface.json`, the manifest `nyx check` and the language
server read, and a test (`engine/tests/host/the-engine-api-matches-the-committed-host-surface`) fails
when the registrations and the manifest differ. After adding or changing a call, run the tests with
`LANTERN_UPDATE_HOST_SURFACE=1` to rewrite the manifest, then update this page. The tutorial for using
these calls is `adding-a-level-and-a-mechanic.md`.

Every name the engine gives a script is under `lantern:::`, in a lowercase namespace per group:
`lantern:::input:::IsDown(...)`, `lantern:::world:::Spawn(...)`, `lantern:::math:::Vec3`. Types
keep PascalCase. Tables below write the full path the first time a group's call appears.

Rules that hold for every call:

- **Handles are `i64`.** Entities, meshes and textures are generational handles packed into 64
  bits. A script can't hold a dangling pointer: a stale handle throws `Error:::NotFound`.
- **Vectors are `lantern:::math:::Vec3` and `lantern:::math:::Vec2` values.** Build one with
  braces where the type is known (`spot: lantern:::math:::Vec3 = { x: 1.0f, y: 2.0f };`, or
  `{ y: 1.0f }` as an argument or a return value; missing fields are 0). Writing the path before
  the braces (`lantern:::math:::Vec3 { … }`) isn't supported by Nyx yet. Read and write `x`, `y` and `z` as fields, and get a copy at every
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

`alpha` blends the last two fixed steps (0..1): use `lantern:::world:::InterpolatedPosition` with it.

The engine keeps two keys for itself: **F5** reloads the game from disk, and **F1** (gamepad
Back) toggles the debug HUD.

## Engine

| Call | |
|---|---|
| `lantern:::engine:::Log(text: string) -> void` | info log, target `game` |
| `lantern:::engine:::Warn(text: string) -> void` | warning log |
| `lantern:::engine:::Quit() -> void` | quit after this frame |
| `lantern:::engine:::Flag(name: string) -> bool` | an environment variable that is set, not empty and not `"0"` (tests override it) |
| `lantern:::engine:::Setting(name: string) -> string` | an environment variable's text, `""` when unset (`LANTERN_LEVEL` is the level the world editor's Play button launched) |

## Input

The game names its actions, binds devices to them, then asks about actions. Edges are latched
for fixed steps: a press is seen by exactly one fixed step even if a frame runs none.

| Call | |
|---|---|
| `lantern:::input:::DefineButton(name) / DefineAxis(name) / DefineVector(name)` | declare an action |
| `lantern:::input:::BindKey(action, key)` | a key to a button |
| `lantern:::input:::BindMouseButton(action, button)` | `Left`, `Right`, `Middle` |
| `lantern:::input:::BindGamepadButton(action, button)` | `South`, `East`, `West`, `North`, `Back`, `Start`, `LeftStick`, `RightStick`, `LeftShoulder`, `RightShoulder`, `DPadUp`, ... |
| `lantern:::input:::BindGamepadAxisButton(action, axis, threshold)` | an axis past a threshold as a button |
| `lantern:::input:::BindKeyAxis(action, negative, positive)` | two keys to an axis |
| `lantern:::input:::BindGamepadAxis(action, axis, scale)` | `LeftX`, `LeftY`, `RightX`, `RightY`, `LeftTrigger`, `RightTrigger` |
| `lantern:::input:::BindKeys(action, up, down, left, right)` | four keys to a vector |
| `lantern:::input:::BindStick(action, stick, scale)` | `Left` or `Right` stick to a vector |
| `lantern:::input:::BindMouseMotion(action, scale)` | mouse motion to a vector |
| `lantern:::input:::IsPressed(action) / IsReleased(action) / IsHeld(action) -> bool` | button state |
| `lantern:::input:::Axis(action) -> f32`, `lantern:::input:::Vector(action) -> Vec2` | analogue state |

Defining an action twice is a script error (`the action 'jump' is defined twice`), and so is a key, button or axis name the engine doesn't know. Keys are `A`..`Z`, `Digit0`..`Digit9`, `Up`, `Down`, `Left`, `Right`, `Space`, `Enter`,
`Escape`, `Tab`, `Backspace`, `LeftShift`, `LeftControl`, `F1`..`F12` and so on: the names
`engine/src/host/DeviceNames.nox` turns into `lantern:::core:::KeyCode`s. An unknown name is a script error.

## World

| Call | |
|---|---|
| `lantern:::world:::Load(path) -> i32` | spawn a world file's entities, meshes and collision; returns the entity count |
| `lantern:::world:::Clear() -> void` | remove every game entity, the world meshes and all collision |
| `lantern:::world:::Spawn(name, position: Vec3) -> i64` | an empty entity |
| `lantern:::world:::Despawn(entity)` | |
| `lantern:::world:::Find(name) -> i64` | the first entity with that name; throws `NotFound` |
| `lantern:::world:::FindByKind(kind) -> Array<i64>` | world-file entities of a kind (`gem`, `spawn`, ...) |
| `lantern:::world:::Kind(entity) -> string` | |
| `lantern:::world:::HasProperty(entity, key) -> bool` | |
| `lantern:::world:::NumberProperty / TextProperty / VectorProperty(entity, key)` | world-file properties |
| `lantern:::world:::SetMaterial(material, texture: i64)` | the texture world-file pieces with that material use (set before `Load`) |
| `lantern:::world:::SetMesh(entity, mesh, tint)`, `SetTexturedMesh(entity, mesh, texture, tint)` | |
| `lantern:::world:::SetTint(entity, tint)`, `SetVisible(entity, visible)`, `IsVisible(entity) -> bool` | |
| `lantern:::world:::SetNormalMap(entity, texture, strength)` | fine surface bumps from a normal map, strength 0 to 4; see `normal-maps.md` |
| `lantern:::world:::SetSurface(entity, metallic, roughness, emissive)` | the entity's material: 0 to 1 metallic and roughness, an "RRGGBB" emissive glow; see `lighting-and-shadows.md` |
| `lantern:::world:::SetWetness(entity, wetness, puddles)` | 0 to 1 each: a wet entity is darker and glossier, and the puddles fraction of its upward-facing surface becomes mirrors; see `wet-surfaces.md` |
| `lantern:::world:::SetGlow(entity, color, strength)` | the entity's emissive light as `color` times `strength`; a strength above 1 is brighter than white, so it blooms. Keeps metallic and roughness |
| `lantern:::world:::SetCastsShadows(entity, castsShadows)` | whether the entity blocks light |
| `lantern:::world:::SetParent(child, parent)` | transforms become relative to the parent |
| `lantern:::world:::Position(entity) -> Vec3` | local position |
| `lantern:::world:::WorldPosition(entity) -> Vec3` | |
| `lantern:::world:::InterpolatedPosition(entity, alpha) -> Vec3` | for `OnRender` |
| `lantern:::world:::SetPosition(entity, position)` | also moves the entity's collision body, carrying riders |
| `lantern:::world:::Teleport(entity, position)` | moves without interpolating from the old place |
| `lantern:::world:::SetYaw(entity, radians)`, `SetScale(entity, scale: Vec3)` | |

World files are made by the editor (the format is `engine/src/world/WorldFile.nox`). The engine builds what's in them;
the game decides what a kind means.

## Assets

| Call | |
|---|---|
| `lantern:::textures:::Load(path) -> i64` | a PNG; throws `NotFound` |
| `lantern:::textures:::Checker(size, cells, first, second) -> i64`, `lantern:::textures:::Solid(color) -> i64` | generated |
| `lantern:::textures:::Bumps(size, cells, steepness) -> i64` | a generated normal map of round bumps; see `normal-maps.md` |
| `lantern:::meshes:::Load(path) -> i64` | a glTF/GLB mesh; throws `NotFound` |
| `lantern:::meshes:::Capsule(radius, halfHeight, segments, rings)`, `Sphere(radius, segments, rings)`, `Cube(size)`, `Gem(radius, height)` | generated, `-> i64` |

## Physics

| Call | |
|---|---|
| `lantern:::physics:::Raycast(origin, direction, maxDistance) -> lantern:::physics:::RayHit?` | `hit.Distance()`, `hit.Point()`, `hit.Normal()` |
| `lantern:::physics:::SphereCast(radius, origin, motion) -> f32?` | the fraction of `motion` before contact, or null |
| `new lantern:::physics:::CharacterController(radius, height, feet: Vec3)` | a kinematic capsule: walks, jumps, climbs steps, rides platforms |
| `controller.Step(moveDirection: Vec3, jumpPressed, jumpHeld, dt) -> lantern:::physics:::ControllerStep` | one fixed step; `Jumped()`, `Landed()`, `LandingSpeed()`, `HitCeiling()` |
| `controller.Teleport(feet)`, `FeetPosition()`, `CapsuleCenter()`, `Velocity()`, `GroundNormal()`, `IsGrounded()` | |
| `controller.Radius()`, `Height()`, `HalfSegment()` | the capsule's axis is `CapsuleCenter() ± HalfSegment()` in y |
| `controller.SetWalkSpeed(m/s)`, `SetJumpHeight(m)`, `SetGravity(m/s²)` | feel; the rest of `ControllerSettings` is engine tuning |

## Navigation

The engine samples the level's static collision into a graph of standable floors, joined by walk links (steps, stairs and
slopes), drop links (walking off a ledge) and jump links (a gap, or a ledge too high to step). There are no climb links,
so a way that needs climbing has no path. Moving bodies (doors, lifts, crumbling slabs) are not in the graph. Build it once
the level is loaded; the partner does it when she spawns. `docs/navigation.md` explains how it works.

| Call | |
|---|---|
| `lantern:::navigation:::Build(radius, height, stepHeight, maxDrop, maxJumpUp, maxJumpAcross) -> i32` | samples the loaded level for a body of that size and returns the node count; 0 when there is no level |
| `lantern:::navigation:::FindPath(from, to) -> lantern:::navigation:::Path` | a route; `from` and `to` snap to the nearest floor |
| `lantern:::navigation:::IsReachable(from, to) -> bool`, `NodeCount() -> i32`, `Clear()` | |
| `path.IsComplete() -> bool` | false when `to` can't be reached: the path then ends at the reachable place nearest it |
| `path.Count() -> i32`, `path.Point(index) -> Vec3`, `path.Link(index) -> i32` | `Link` is how point `index` is reached: 0 walk, 1 drop, 2 jump. Call them through a `~` binding |

A jump point is reached by leaving the point before it with the jump button down. An empty path means `from` is not near
any floor.

## Rendering and screen

| Call | |
|---|---|
| `lantern:::camera:::LookAt(eye, target)` | this frame's camera |
| `lantern:::camera:::SetFieldOfView(degrees)` | the lens, 10 to 120 degrees top to bottom (60 by default); `LookAt` keeps it |
| `lantern:::renderer:::SetClearColor(color)`, `lantern:::renderer:::SetFog(start, end)` | |
| `lantern:::graphics:::SetQuality(level)` | `"low"`, `"medium"` or `"high"`: how much the renderer spends on looks. Low turns bloom, anti-aliasing, ambient occlusion and light shafts off and uses fewer shadow lookups; see `bloom-and-tone-mapping.md` |
| `lantern:::graphics:::SetAntiAliasing(enabled)` | smooth jagged edges with FXAA (on by default); see `anti-aliasing.md` |
| `lantern:::graphics:::SetAmbientOcclusion(enabled, radius, intensity)` | darken creases and contact points from the depth buffer (radius in metres, intensity 0 to 1); see `ambient-occlusion.md` |
| `lantern:::graphics:::SetScreenSpaceReflections(enabled, intensity, maxDistance)` | let glossy and wet surfaces reflect what is on screen, falling back to the sky; see `screen-space-reflections.md` |
| `lantern:::graphics:::AddBlobShadow(center, radius, strength)` | this frame only: a soft dark patch on the surface around `center` (up to eight); see `blob-shadows.md` |
| `lantern:::graphics:::SetBlobShadows(enabled)` | switch the frame's blob shadows on or off |
| `lantern:::graphics:::SetColorGrade(saturation, contrast, tint, vignette)` | the look of the finished picture: saturation 0 to 3, contrast 0.2 to 3, an `"RRGGBB"` tint and a vignette 0 to 1; `1, 1, "FFFFFF", 0` is neutral; see `colour-grade.md` |
| `lantern:::graphics:::SetWetWorld(wetness, puddles)` | rain on the whole scene: the least wetness every surface has and how much wet ground is puddles, 0 to 1 each; see `wet-surfaces.md` |
| `lantern:::graphics:::SetBloom(enabled, strength, threshold)` | the glow around bright things: how much is added back and the brightness where it starts (1 is a white wall in full sun) |
| `lantern:::graphics:::SetExposure(exposure)`, `lantern:::graphics:::SetToneMapper(name)` | a brightness knob applied before the curve, and the curve itself: `"none"`, `"neutral"` or `"aces"` |
| `lantern:::lights:::SetSun(direction, color, skyAmbient, groundAmbient)` | the sun and the ambient light; colours are `"RRGGBB"` |
| `lantern:::lights:::SetSunShadows(enabled)`, `lantern:::lights:::SetShadowQuality(enabled, sunMapSize, localTileSize, distance)` | whether the sun casts shadows; the shadow master switch, map sizes in pixels (256 to 4096) and how far the sun's shadows reach |
| `lantern:::lights:::SetShadowSoftness(softness, samples, blendCascades)` | how soft shadow edges are (filter radius in map texels, 0.5 to 6), how many lookups the filter makes (1 to 32) and whether cascades fade into each other; see `shadow-softness-and-cascade-blending.md` |
| `lantern:::lights:::AddPoint(position, color, strength, range) -> i64` | a point light; the id names it from now on |
| `lantern:::lights:::AddSpot(position, target, color, strength, range, innerDegrees, outerDegrees) -> i64` | a spot light |
| `lantern:::lights:::Move(light, position)`, `Aim(light, target)`, `SetColor(light, color, strength)`, `SetShadows(light, enabled)`, `Remove(light)`, `Count() -> i64` | change a light by id; lights stay until removed or the game reloads |
| `lantern:::screen:::Width() / Height() -> f32` | pixels |
| `lantern:::screen:::UiScale() -> f32` | whole-pixel text scale for the display density |
| `lantern:::screen:::SetMouseCaptured(captured)` | applied after this frame's `OnUpdate` |

### Sky, `lantern:::sky`

The sky behind the world and the light it gives (guide: `sky-and-image-based-lighting.md`).

- `SetColors(zenith, horizon, ground)`: three `"RRGGBB"` strings.
- `SetSunDisc(radiusDegrees, strength)`: the sun's disc; a strength above 1 blooms.
- `SetLighting(strength, intensity)`: how much of the ambient light comes from the sky, and how bright it is.
- `SetEnabled(enabled)`: off brings back the clear colour and the flat ambient pair.

### Atmosphere, `lantern:::atmosphere`

Height fog and light shafts (guide: `fog-and-light-shafts.md`).

- `SetHeightFog(enabled, density, heightFalloff, baseHeight)`: fog that thins with height; density is per metre at the base.
- `SetFogColor(color, sunScattering)`: an `"RRGGBB"` string, and how much brighter fog is looking towards the sun.
- `SetLightShafts(enabled, strength, decay)`: streaks of sunlight away from the sun where objects block it.

### Particles, `lantern:::particles`

Dust, sparks and trails (guide: `particles.md`).

- `Emit(position, velocity, color, life, length, gravity)`: one particle; `color` is an `"RRGGBB"` string, `life` is seconds, `length` is the streak's length in seconds of travel and `gravity` is metres per second squared. Dropped when the quality budget is full.
- `Clear()`: removes every particle.
- `Count()`: how many are alive, as an i64.

## UI (Chaos and Cosmos), `lantern:::ui`

The game's menus and HUD are Chaos components (`.chaos`) styled by Cosmos stylesheets
(`.cosmos`), all in the game's `ui/` folder (D-043). Every `.cosmos` file there is loaded at start:
`global.cosmos` as the global sheet, and all the others together as one shared component sheet.
So a rule in `hud.cosmos` can match an element in a menu, `:root` variables from every file share
one namespace, and a class defined in two files isn't reported (the later file, by name, wins).
Keep class names unique across the folder.

| Call | |
|---|---|
| `lantern:::ui:::Mount(file, component) -> i32` | mounts `component` from `file` (relative to the game directory), passing the game's `Application` object as its one argument: `GameScreen :: (game~ Platformer) -> lantern:::ui:::Element`. Call it from `OnStart` or later |
| `lantern:::ui:::Unmount(mount)` | |
| `lantern:::ui:::SetNavigation(previous, next, confirm)` | the input actions (by name) that move focus and press the focused button |

- **State**: a field marked `@signal` on a game object re-renders every `<slot>` that read it
  when the game writes it. Write only when the value changes, or the slot re-renders every
  frame.
- **Buttons** are elements with `onPress` or `onRelease`. Focus starts on the first one and
  moves in the order they appear; moving calls the new button's `onFocus`, confirming calls
  `onPress` then `onRelease`. Handlers run just before `OnUpdate`, so a handler should record a
  choice for `OnUpdate` to act on.
- **Mouse**: while the mouse isn't captured (`lantern:::screen:::SetMouseCaptured`), moving it over a
  button focuses that button (`onFocus`), pressing the left button on it calls `onPress` and
  draws it `:active`, and releasing on the same button calls `onRelease`; releasing anywhere
  else cancels. A mouse that doesn't move never takes focus from the keyboard.
- **Drawing**: the engine lays the UI out and draws it after `OnRender`, on top of `lantern:::overlay:::`
  drawing. A Cosmos `px` is one window point (`lantern:::screen:::UiScale() / 2` pixels). `font-size`
  snaps to multiples of 8, the bitmap font's size. The focused button and its contents use
  their `:hover` style (write `.label:hover`, not a rule nested in `:hover { }`), and `:active`
  while confirmed or held down by the mouse.
- **Supported elements**: `<frame>` and `<text>`. Supported properties: `background-color`,
  `background-gradient-start` and `background-gradient-end` (top to bottom), `border-color`,
  `border-width`, `border-radius`, `box-shadow` (colour and blur), `padding`, `margin`,
  `color`, `font-size`, `opacity`, `display: stack` with `flex-direction`, `gap`,
  `align-items`, `justify-content`, `width`, `height`, `min-width`, `max-width`, `flex-grow`,
  `position: absolute | fixed` with `top`, `right`, `bottom`, `left`, and `z-index`. Lengths are
  px, %, vw or vh. A frame stacks its children in a column unless it says otherwise.
- Errors in a component or a handler are reported as `UI: ...` like a failed hook. Stylesheet
  problems are logged as warnings.

## Overlay

Call these from `OnRender`. The overlay is cleared every frame. Menus and the HUD are Chaos
components (`lantern:::ui`); `lantern:::overlay:::` is for quick text and rectangles.

| Call | |
|---|---|
| `lantern:::overlay:::Rect(x, y, width, height, color)` | screen pixels |
| `lantern:::overlay:::Text(text, x, y, scale, color)` | the engine's 8×8 bitmap font |
| `lantern:::overlay:::TextWidth(text, scale) / TextHeight(text, scale) -> f32`, `lantern:::overlay:::GlyphSize() -> f32` | |

## Audio

| Call | |
|---|---|
| `lantern:::audio:::Load(name, path)` | a WAV; a missing sound is logged and stays silent |
| `lantern:::audio:::Play(name, volume)` | a one-shot voice |
| `lantern:::audio:::PlayMusic(name)`, `lantern:::audio:::StopMusic()` | one looping track |

## Debug

| Call | |
|---|---|
| `lantern:::debug:::SetValue(path, value: string)` | a HUD line, such as `"Player/Speed"` |
| `lantern:::debug:::IsHudVisible() -> bool`, `lantern:::debug:::SetHudVisible(visible)` | |
| `lantern:::debug:::Capsule(center, radius, halfSegment, color)`, `Sphere(center, radius, color)` | wireframes |
| `lantern:::debug:::Arrow(start, vector, color)`, `Line(start, end, color)` | |
| `lantern:::debug:::CollisionNear(center, halfExtent, color)` | the collision triangles in a box |

The engine fills the `Frame`, `Script`, `Renderer`, `World`, `Audio` and `Errors` groups itself.

## Math

| Call | |
|---|---|
| `lantern:::math:::Vec3` `{ x, y, z }`, `lantern:::math:::Vec3:::Zero()`, `Up()`, `Down()`, `FromYaw(radians)` | fields `x`, `y`, `z` |
| `a + b`, `a - b`, `v * s`, `v / s`, `-v` | operators, the vector on the left |
| `v.Cross(other)`, `Dot(other)`, `Normalized()` | |
| `v.Horizontal()`, `WithX / WithY / WithZ`, `ClampLength(max)`, `Lerp(other, t)`, `Clamp(min, max)` | |
| `v.Length()`, `LengthSquared()`, `DistanceTo(other)`, `Yaw()` | |
| `lantern:::math:::Vec2` `{ x, y }`, `lantern:::math:::Vec2:::Zero()`; the same operators; `Length`, `ClampLength`, `Normalized` | fields `x`, `y` |
| `lantern:::angle:::Wrap(radians)` | to -π..π |
| `lantern:::angle:::ToDegrees(radians)` | |

Nyx's own `Math` (`sin`, `cos`, `tan`, `atan2`, `asin`, `acos`, `sqrt`, `min`, `max`, `clamp`,
`TAU`, ...) needs no host.

## Animation, `lantern:::animation`

Skeletal animation (see `docs/animation.md`). Ids are small integers valid until the level's entities are cleared.

| Function | Does |
| --- | --- |
| `MannequinRig() -> i64` | Builds the humanoid rig and its mesh. Make it once, share it. |
| `Spawn(entity, rig, texture, tint) -> i64` | Makes the entity an animated, skinned body and returns its animator. |
| `AddState(animator, state, clip, loops)` | A state that plays one clip. |
| `AddBlendState(animator, state, parameter)` | A state that mixes clips by a float parameter. |
| `AddBlendClip(animator, state, clip, threshold)` | Adds a clip to a blend state at a parameter value. |
| `SetStateSpeed(animator, state, speed)` | Playback speed of a state (1 is normal). |
| `AddTransition(animator, from, to, parameter, condition, threshold, fadeSeconds)` | A rule: `condition` is `greater`, `less`, `finished` or `trigger`; an empty `from` means any state. The first rule that holds wins. |
| `SetFloat(animator, parameter, value)`, `SetBool(...)`, `Trigger(animator, parameter)` | Report facts. A trigger fires one transition. |
| `Play(animator, state, fadeSeconds)` | Force a crossfade into a state. |
| `CurrentState(animator) -> string` | The state playing. |
| `SetLookTarget(animator, target, weight)` | Turn chest, neck and head toward a world point. |

## Examples

Each snippet is how the platformer (`games/platformer/`) uses the group; the file is named so you can read it in context.
Names such as `spawnPosition` stand for values the surrounding code has.

**Input** (`settings/input-settings.nyx`, `main.nyx`): name an action, bind devices to it, ask about it each frame.

```nyx
lantern:::input:::DefineVector("move");
lantern:::input:::BindKeys("move", "W", "S", "A", "D");
lantern:::input:::BindStick("move", "Left", 1.0f);

lantern:::input:::DefineButton("jump");
lantern:::input:::BindKey("jump", "Space");
lantern:::input:::BindGamepadButton("jump", "South");

stick: lantern:::math:::Vec2 = lantern:::input:::Vector("move");
jumpPressed: bool = lantern:::input:::IsPressed("jump");
```

**Assets and world** (`game-assets/game-assets.nyx`, `game-scenes/level-scene.nyx`): load assets, tell the world which
texture each material uses, load the level, then find the entities by kind.

```nyx
lantern:::world:::SetMaterial("ground", lantern:::textures:::Load("textures/ground.png"));
gemMesh := lantern:::meshes:::Load("models/gem.glb");
count := lantern:::world:::Load("worlds/course.world.json");
for entity: i64 in lantern:::world:::FindByKind("gem") {
    lantern:::world:::SetMesh(entity, gemMesh, "FFFFFF");
}
```

(`textures:::Load` and `meshes:::Load` throw `Error:::NotFound` when a file is missing; the platformer catches that with
`when` and falls back to a generated texture or mesh, `Solid`, `Checker`, `Gem`.)

**Physics** (`game-entities/player.nyx`, `game-systems/third-person-camera.nyx`): a character controller, and a cast that
keeps the camera out of walls.

```nyx
controller := new lantern:::physics:::CharacterController(radius, height, spawnPosition);
step := controller.Step(moveDirection, jumpPressed, jumpHeld, dt);
if step.Jumped() {
    lantern:::audio:::Play("jump", 1.0f);
}
allowedDistance := when lantern:::physics:::SphereCast(0.2f, pivot, backward * distance) {
    fraction: f32 -> Math:::max(distance * fraction, 0.5f),
    null -> distance
};
```

**Camera and screen** (`game-systems/camera-director.nyx`, `main.nyx`):

```nyx
lantern:::camera:::SetFieldOfView(shown.fieldOfViewDegrees);
lantern:::camera:::LookAt(shown.eye, shown.target);
lantern:::screen:::SetMouseCaptured(true);
```

`third-person-camera.nyx` works out a pose and `camera-director.nyx` is the one place that places the camera; see
[`camera.md`](camera.md).

**Lights** (`game-systems/level-lighting.nyx`): the sun once, lamps as handles you can remove.

```nyx
lantern:::lights:::SetSun(LightingSettings:::SunDirection(), sun, skyAmbient, groundAmbient);
lampId := lantern:::lights:::AddPoint(lampPosition, LightingSettings:::LAMP_COLOR, LightingSettings:::LAMP_STRENGTH, LightingSettings:::LAMP_RANGE);
lantern:::lights:::SetShadows(lampId, true);
lantern:::lights:::Remove(lampId);
```

**Sky, fog and particles** (`settings/chapter-looks.nyx`, `game-systems/dust.nyx`):

```nyx
lantern:::sky:::SetColors(zenith, horizon, ground);
lantern:::renderer:::SetClearColor(horizon);
lantern:::atmosphere:::SetFogColor(horizon, 0.5f);
lantern:::renderer:::SetFog(fogStart, fogEnd);
lantern:::particles:::Emit(at, velocity, "C8B89A", 0.4f, 0.05f, 6.0f);
```

**Graphics** (`game-entities/player.nyx`): a blob shadow under the player each frame. The other graphics calls are in
`quality-settings.md`.

```nyx
lantern:::graphics:::AddBlobShadow(blobCenter, blobRadius, 0.55f);
```

**Audio, UI and debug** (`main.nyx`, `game-state/`, `game-systems/`):

```nyx
lantern:::audio:::Load("jump", "sounds/jump.wav");
lantern:::audio:::PlayMusic("theme");
lantern:::ui:::SetNavigation("menu_up", "menu_down", "menu_confirm");
lantern:::ui:::Mount("ui/game-screen.chaos", "GameScreen");
lantern:::debug:::SetValue("Player/Speed", `${velocity.Horizontal().Length():.2f}`);
```

`lantern:::overlay:::Text` and `Rect` draw quick text and boxes from `OnRender`; the platformer uses Chaos components for
all its text instead.

## Signature index

Every function the engine registers, as `games/platformer/engine-host-surface.json` lists it, in the order they are registered. `lantern:::ui:::Element` is the type a Chaos component returns; `Component` is the base class a game application derives from.


### `lantern:::angle`

| Function | Signature |
| --- | --- |
| `Wrap` | `(radians: f32) -> f32` |
| `ToDegrees` | `(radians: f32) -> f32` |

### `lantern:::engine`

| Function | Signature |
| --- | --- |
| `Log` | `(text: string) -> void` |
| `Warn` | `(text: string) -> void` |
| `Quit` | `() -> void` |
| `Flag` | `(name: string) -> bool` |
| `Setting` | `(name: string) -> string` |

### `lantern:::input`

| Function | Signature |
| --- | --- |
| `DefineButton` | `(name: string) -> void` |
| `DefineAxis` | `(name: string) -> void` |
| `DefineVector` | `(name: string) -> void` |
| `BindKey` | `(action: string, key: string) -> void` |
| `BindMouseButton` | `(action: string, button: string) -> void` |
| `BindGamepadButton` | `(action: string, button: string) -> void` |
| `BindGamepadAxisButton` | `(action: string, axis: string, threshold: f32) -> void` |
| `BindKeyAxis` | `(action: string, negative: string, positive: string) -> void` |
| `BindGamepadAxis` | `(action: string, axis: string, scale: f32) -> void` |
| `BindKeys` | `(action: string, up: string, down: string, left: string, right: string) -> void` |
| `BindStick` | `(action: string, stick: string, scale: f32) -> void` |
| `BindMouseMotion` | `(action: string, scale: f32) -> void` |
| `IsPressed` | `(action: string) -> bool` |
| `IsReleased` | `(action: string) -> bool` |
| `IsHeld` | `(action: string) -> bool` |
| `Axis` | `(action: string) -> f32` |
| `Vector` | `(action: string) -> lantern:::math:::Vec2` |

### `lantern:::world`

| Function | Signature |
| --- | --- |
| `Load` | `(path: string) -> i32` |
| `Clear` | `() -> void` |
| `Spawn` | `(name: string, position: lantern:::math:::Vec3) -> i64` |
| `Despawn` | `(entity: i64) -> void` |
| `Find` | `(name: string) -> i64` |
| `FindByKind` | `(kind: string) -> Array<i64>` |
| `Kind` | `(entity: i64) -> string` |
| `HasProperty` | `(entity: i64, key: string) -> bool` |
| `NumberProperty` | `(entity: i64, key: string) -> f32` |
| `TextProperty` | `(entity: i64, key: string) -> string` |
| `VectorProperty` | `(entity: i64, key: string) -> lantern:::math:::Vec3` |
| `SetMaterial` | `(material: string, texture: i64) -> void` |
| `SetMesh` | `(entity: i64, mesh: i64, tint: string) -> void` |
| `SetTexturedMesh` | `(entity: i64, mesh: i64, texture: i64, tint: string) -> void` |
| `SetNormalMap` | `(entity: i64, texture: i64, strength: f32) -> void` |
| `SetTint` | `(entity: i64, tint: string) -> void` |
| `SetSurface` | `(entity: i64, metallic: f32, roughness: f32, emissive: string) -> void` |
| `SetWetness` | `(entity: i64, wetness: f32, puddles: f32) -> void` |
| `SetGlow` | `(entity: i64, color: string, strength: f32) -> void` |
| `SetCastsShadows` | `(entity: i64, castsShadows: bool) -> void` |
| `SetParent` | `(child: i64, parent: i64) -> void` |
| `Position` | `(entity: i64) -> lantern:::math:::Vec3` |
| `WorldPosition` | `(entity: i64) -> lantern:::math:::Vec3` |
| `InterpolatedPosition` | `(entity: i64, alpha: f32) -> lantern:::math:::Vec3` |
| `SetPosition` | `(entity: i64, position: lantern:::math:::Vec3) -> void` |
| `Teleport` | `(entity: i64, position: lantern:::math:::Vec3) -> void` |
| `SetYaw` | `(entity: i64, yaw: f32) -> void` |
| `SetScale` | `(entity: i64, scale: lantern:::math:::Vec3) -> void` |
| `IsVisible` | `(entity: i64) -> bool` |
| `SetVisible` | `(entity: i64, visible: bool) -> void` |

### `lantern:::textures`

| Function | Signature |
| --- | --- |
| `Load` | `(path: string) -> i64` |
| `Checker` | `(size: i32, cells: i32, first: string, second: string) -> i64` |
| `Bumps` | `(size: i32, cells: i32, steepness: f32) -> i64` |
| `Solid` | `(color: string) -> i64` |

### `lantern:::meshes`

| Function | Signature |
| --- | --- |
| `Load` | `(path: string) -> i64` |
| `Capsule` | `(radius: f32, halfHeight: f32, segments: i32, rings: i32) -> i64` |
| `Sphere` | `(radius: f32, segments: i32, rings: i32) -> i64` |
| `Cube` | `(size: f32) -> i64` |
| `Gem` | `(radius: f32, height: f32) -> i64` |

### `lantern:::physics`

| Function | Signature |
| --- | --- |
| `Raycast` | `(origin: lantern:::math:::Vec3, direction: lantern:::math:::Vec3, maxDistance: f32) -> lantern:::physics:::RayHit?` |
| `SphereCast` | `(radius: f32, origin: lantern:::math:::Vec3, motion: lantern:::math:::Vec3) -> f32?` |

### `lantern:::navigation`

| Function | Signature |
| --- | --- |
| `Build` | `(radius: f32, height: f32, stepHeight: f32, maxDrop: f32, maxJumpUp: f32, maxJumpAcross: f32) -> i32` |
| `Clear` | `() -> void` |
| `NodeCount` | `() -> i32` |
| `FindPath` | `(from: lantern:::math:::Vec3, to: lantern:::math:::Vec3) -> lantern:::navigation:::Path` |
| `IsReachable` | `(from: lantern:::math:::Vec3, to: lantern:::math:::Vec3) -> bool` |

### `lantern:::camera`

| Function | Signature |
| --- | --- |
| `LookAt` | `(eye: lantern:::math:::Vec3, target: lantern:::math:::Vec3) -> void` |
| `SetFieldOfView` | `(degrees: f32) -> void` |

### `lantern:::renderer`

| Function | Signature |
| --- | --- |
| `SetClearColor` | `(color: string) -> void` |
| `SetFog` | `(start: f32, end: f32) -> void` |

### `lantern:::screen`

| Function | Signature |
| --- | --- |
| `Width` | `() -> f32` |
| `Height` | `() -> f32` |
| `UiScale` | `() -> f32` |
| `SetMouseCaptured` | `(captured: bool) -> void` |

### `lantern:::lights`

| Function | Signature |
| --- | --- |
| `SetSun` | `(direction: lantern:::math:::Vec3, color: string, skyAmbient: string, groundAmbient: string) -> void` |
| `SetSunShadows` | `(enabled: bool) -> void` |
| `SetShadowQuality` | `(enabled: bool, sunMapSize: i64, localTileSize: i64, distance: f32) -> void` |
| `SetShadowSoftness` | `(softness: f32, samples: i64, blendCascades: bool) -> void` |
| `AddPoint` | `(position: lantern:::math:::Vec3, color: string, strength: f32, range: f32) -> i64` |
| `AddSpot` | `(position: lantern:::math:::Vec3, target: lantern:::math:::Vec3, color: string, strength: f32, range: f32, innerDegrees: f32, outerDegrees: f32) -> i64` |
| `Move` | `(light: i64, position: lantern:::math:::Vec3) -> void` |
| `Aim` | `(light: i64, target: lantern:::math:::Vec3) -> void` |
| `SetColor` | `(light: i64, color: string, strength: f32) -> void` |
| `SetShadows` | `(light: i64, enabled: bool) -> void` |
| `Remove` | `(light: i64) -> void` |
| `Count` | `() -> i64` |

### `lantern:::graphics`

| Function | Signature |
| --- | --- |
| `SetQuality` | `(level: string) -> void` |
| `SetBloom` | `(enabled: bool, strength: f32, threshold: f32) -> void` |
| `SetAntiAliasing` | `(enabled: bool) -> void` |
| `SetAmbientOcclusion` | `(enabled: bool, radius: f32, intensity: f32) -> void` |
| `SetScreenSpaceReflections` | `(enabled: bool, intensity: f32, maxDistance: f32) -> void` |
| `AddBlobShadow` | `(center: lantern:::math:::Vec3, radius: f32, strength: f32) -> void` |
| `SetBlobShadows` | `(enabled: bool) -> void` |
| `SetColorGrade` | `(saturation: f32, contrast: f32, tint: string, vignette: f32) -> void` |
| `SetWetWorld` | `(wetness: f32, puddles: f32) -> void` |
| `SetExposure` | `(exposure: f32) -> void` |
| `SetToneMapper` | `(name: string) -> void` |

### `lantern:::sky`

| Function | Signature |
| --- | --- |
| `SetColors` | `(zenith: string, horizon: string, ground: string) -> void` |
| `SetSunDisc` | `(radiusDegrees: f32, strength: f32) -> void` |
| `SetLighting` | `(strength: f32, intensity: f32) -> void` |
| `SetEnabled` | `(enabled: bool) -> void` |

### `lantern:::atmosphere`

| Function | Signature |
| --- | --- |
| `SetHeightFog` | `(enabled: bool, density: f32, heightFalloff: f32, baseHeight: f32) -> void` |
| `SetFogColor` | `(color: string, sunScattering: f32) -> void` |
| `SetLightShafts` | `(enabled: bool, strength: f32, decay: f32) -> void` |

### `lantern:::particles`

| Function | Signature |
| --- | --- |
| `Emit` | `(position: lantern:::math:::Vec3, velocity: lantern:::math:::Vec3, color: string, life: f32, length: f32, gravity: f32) -> void` |
| `Clear` | `() -> void` |
| `Count` | `() -> i64` |

### `lantern:::audio`

| Function | Signature |
| --- | --- |
| `Load` | `(name: string, path: string) -> void` |
| `Play` | `(name: string, volume: f32) -> void` |
| `PlayMusic` | `(name: string) -> void` |
| `StopMusic` | `() -> void` |

### `lantern:::overlay`

| Function | Signature |
| --- | --- |
| `Rect` | `(x: f32, y: f32, width: f32, height: f32, color: string) -> void` |
| `Text` | `(text: string, x: f32, y: f32, scale: f32, color: string) -> void` |
| `TextWidth` | `(text: string, scale: f32) -> f32` |
| `TextHeight` | `(text: string, scale: f32) -> f32` |
| `GlyphSize` | `() -> f32` |

### `lantern:::ui`

| Function | Signature |
| --- | --- |
| `Mount` | `(file: string, component: string) -> i32` |
| `Unmount` | `(mount: i32) -> void` |
| `SetNavigation` | `(previous: string, next: string, confirm: string) -> void` |

### `lantern:::debug`

| Function | Signature |
| --- | --- |
| `SetValue` | `(path: string, value: string) -> void` |
| `IsHudVisible` | `() -> bool` |
| `SetHudVisible` | `(visible: bool) -> void` |
| `Capsule` | `(center: lantern:::math:::Vec3, radius: f32, halfSegment: f32, color: string) -> void` |
| `Sphere` | `(center: lantern:::math:::Vec3, radius: f32, color: string) -> void` |
| `Arrow` | `(start: lantern:::math:::Vec3, vector: lantern:::math:::Vec3, color: string) -> void` |
| `Line` | `(start: lantern:::math:::Vec3, end: lantern:::math:::Vec3, color: string) -> void` |
| `CollisionNear` | `(center: lantern:::math:::Vec3, halfExtent: f32, color: string) -> void` |

### Types


#### `lantern:::math:::Vec3`

| Member | Signature |
| --- | --- |
| field `x` | `() -> f32` |
| field `y` | `() -> f32` |
| field `z` | `() -> f32` |
| static `Zero` | `() -> lantern:::math:::Vec3` |
| static `Up` | `() -> lantern:::math:::Vec3` |
| static `Down` | `() -> lantern:::math:::Vec3` |
| static `FromYaw` | `(yaw: f32) -> lantern:::math:::Vec3` |
| method `Cross` | `(other: lantern:::math:::Vec3) -> lantern:::math:::Vec3` |
| method `Normalized` | `() -> lantern:::math:::Vec3` |
| method `Horizontal` | `() -> lantern:::math:::Vec3` |
| method `WithX` | `(x: f32) -> lantern:::math:::Vec3` |
| method `WithY` | `(y: f32) -> lantern:::math:::Vec3` |
| method `WithZ` | `(z: f32) -> lantern:::math:::Vec3` |
| method `ClampLength` | `(maximum: f32) -> lantern:::math:::Vec3` |
| method `Lerp` | `(other: lantern:::math:::Vec3, t: f32) -> lantern:::math:::Vec3` |
| method `Clamp` | `(minimum: lantern:::math:::Vec3, maximum: lantern:::math:::Vec3) -> lantern:::math:::Vec3` |
| method `Dot` | `(other: lantern:::math:::Vec3) -> f32` |
| method `Length` | `() -> f32` |
| method `LengthSquared` | `() -> f32` |
| method `DistanceTo` | `(other: lantern:::math:::Vec3) -> f32` |
| method `Yaw` | `() -> f32` |
| operator `+` | `(other: lantern:::math:::Vec3) -> lantern:::math:::Vec3` |
| operator `-` | `(other: lantern:::math:::Vec3) -> lantern:::math:::Vec3` |
| operator `*` | `(scalar: f32) -> lantern:::math:::Vec3` |
| operator `/` | `(scalar: f32) -> lantern:::math:::Vec3` |
| operator `-` | `() -> lantern:::math:::Vec3` |

#### `lantern:::math:::Vec2`

| Member | Signature |
| --- | --- |
| field `x` | `() -> f32` |
| field `y` | `() -> f32` |
| static `Zero` | `() -> lantern:::math:::Vec2` |
| method `Length` | `() -> f32` |
| method `ClampLength` | `(maximum: f32) -> lantern:::math:::Vec2` |
| method `Normalized` | `() -> lantern:::math:::Vec2` |
| operator `-` | `() -> lantern:::math:::Vec2` |
| operator `+` | `(other: lantern:::math:::Vec2) -> lantern:::math:::Vec2` |
| operator `-` | `(other: lantern:::math:::Vec2) -> lantern:::math:::Vec2` |
| operator `*` | `(scalar: f32) -> lantern:::math:::Vec2` |
| operator `/` | `(scalar: f32) -> lantern:::math:::Vec2` |

#### `lantern:::physics:::RayHit`

| Member | Signature |
| --- | --- |
| method `Distance` | `() -> f32` |
| method `Point` | `() -> lantern:::math:::Vec3` |
| method `Normal` | `() -> lantern:::math:::Vec3` |

#### `lantern:::navigation:::Path`

| Member | Signature |
| --- | --- |
| method `Count` | `() -> i32` |
| method `IsComplete` | `() -> bool` |
| method `Point` | `(index: i32) -> lantern:::math:::Vec3` |
| method `Link` | `(index: i32) -> i32` |

#### `lantern:::physics:::ControllerStep`

| Member | Signature |
| --- | --- |
| method `Jumped` | `() -> bool` |
| method `Landed` | `() -> bool` |
| method `LandingSpeed` | `() -> f32` |
| method `HitCeiling` | `() -> bool` |

#### `lantern:::physics:::CharacterController`

| Member | Signature |
| --- | --- |
| constructor `constructor` | `(radius: f32, height: f32, feet: lantern:::math:::Vec3) -> void` |
| method `Step` | `(moveDirection: lantern:::math:::Vec3, jumpPressed: bool, jumpHeld: bool, dt: f32) -> lantern:::physics:::ControllerStep` |
| method `Teleport` | `(feet: lantern:::math:::Vec3) -> void` |
| method `FeetPosition` | `() -> lantern:::math:::Vec3` |
| method `CapsuleCenter` | `() -> lantern:::math:::Vec3` |
| method `Velocity` | `() -> lantern:::math:::Vec3` |
| method `GroundNormal` | `() -> lantern:::math:::Vec3` |
| method `IsGrounded` | `() -> bool` |
| method `Radius` | `() -> f32` |
| method `Height` | `() -> f32` |
| method `HalfSegment` | `() -> f32` |
| method `SetWalkSpeed` | `(metresPerSecond: f32) -> void` |
| method `SetJumpHeight` | `(metres: f32) -> void` |
| method `SetGravity` | `(metresPerSecondSquared: f32) -> void` |

#### `lantern:::ui:::Element`

A handle type with no members of its own; it is only passed around.

