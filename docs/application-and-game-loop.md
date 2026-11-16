# The application and the game loop

How `lantern` starts, runs a frame and shuts down. The files are `engine/src/main.nox`, `engine/src/application/`
and `engine/src/engine/loop/`.

## From main to the first frame

```text
main.nox
  └─ application/Programs.nox      RunGame(gameDirectory)            RunStubGame()
       │                             read game.json                   (no game: a lit test scene)
       ▼
  application/Application.nox
       1. Create        SDL, the window, the platform backend
       2. Initialise    CreateRenderer(), OpenAudioOutput(); the program builds its Game around them
       3. Run           a GameLoop runs the game until a quit (Run, or RunObserved to act between frames)
       4. Shut down     delete the Game (renderer, audio), then the Application (platform, window, SDL)
```

`Application` is the only place that creates platform objects. It is a composition root: it makes each object and
passes it to whatever needs it, so following a constructor shows every dependency. Nothing reaches for a global.

### Why the shutdown order matters

Objects are destroyed in the reverse of the order they depend on each other:

1. The **game loop** and its input router, when `Run` returns.
2. The **game**, and with it the renderer and the audio output it was given. The renderer's GPU resources belong to
   a device that is attached to the window, so they must go while the window still exists.
3. The **platform**, which owns the **window**.
4. **SDL** itself, last, since everything above was made through it.

`Programs.nox` deletes the game before the application, and `Application`'s destructor deletes the rest in that
order.

## ApplicationConfig

`ApplicationConfig.nox` gathers what a run reads from outside: the window settings (the game's title and the
`WindowConfig` defaults: 1280x720, resizable, vertical sync) and the environment variables `LANTERN_MAX_FRAMES`,
`LANTERN_DEBUG_HUD`, `LANTERN_WINDOW_SCRIPT` and the GPU options (`LANTERN_GPU_DEBUG`, `LANTERN_VSYNC`). They're read
once, at start-up, in one place. `engine/README.md` lists every variable.

## The Game interface

The loop doesn't know what it runs. It calls six hooks on a `Game` (`engine/loop/Game.nox`), in this order each
frame:

| Hook | When | For |
| --- | --- | --- |
| `UpdateInputConfiguration` | first | changing key bindings before input is read |
| `HandleEvent` | once per platform event | window, keyboard, mouse and gamepad events |
| `FixedUpdate` | zero or more times | the simulation, in steps of exactly 1/60 s |
| `FrameUpdate` | once per displayed frame | the camera, the UI, anything tied to real time |
| `Render` | once per displayed frame, unless minimised | filling the `RenderWorld` and calling the renderer |
| `WantsToQuit` | last | ending the loop |

`GameHost` (`host/GameHost.nox`) is the `Game` for Nyx games: each hook calls the game script's matching hook
(`OnFixedUpdate`, `OnUpdate`, `OnRender`) and does the engine's own work around it. `StubGame` and the tests' scripted
games are others.

## One frame of the game loop

`GameLoop.RunFrame` (`engine/loop/GameLoop.nox`):

```text
1. game.UpdateInputConfiguration(input)
2. events = platform.PumpEvents()
   for each event: note a quit request, feed the input router, game.HandleEvent(event)
3. frameSeconds = now - previous frame time          (0 on the first frame and after a minimise)
4. plan = clock.Advance(frameSeconds)                 how many fixed steps, and alpha
5. for each of plan.stepCount steps:
       game.FixedUpdate(step's input snapshot, 1/60 s, step number)
6. game.FrameUpdate(frame)
7. if the window has pixels: game.Render(frame)
   else: sleep 16 ms                                  nothing to draw while minimised
8. if a quit was requested: Finished, else Running
```

### Why fixed steps

If the simulation advanced by the real frame time, its results would depend on frame rate: a character would jump a
little differently at 30, 60 and 144 frames a second, and one long frame (a shader compile, a file load) could move a
body clean through a wall. So the simulation only ever advances in steps of exactly 1/60 s, and the loop runs as many
steps as real time has covered. That makes the physics deterministic: the tests replay the platformer at several
display rates and get the same path every time.

### The fixed-step clock

`FixedStepClock.Advance` (`core/Time.nox`) turns a frame's real duration into a number of steps:

```text
frame = clamp(measured frame time, 0, 0.25 s)          one long stall can't demand dozens of steps
if |frame - 1/60| < 0.0002 s: frame = 1/60              snap: under vsync, frames are 1/60 give or take timer
                                                        jitter, and snapping stops that jitter piling up into
                                                        occasional double steps
accumulator += frame
while accumulator >= 1/60 and steps < 8:                at most 8 steps a frame; if still behind, the extra time
    accumulator -= 1/60; steps += 1                     is dropped rather than carried (no "spiral of death")
alpha = accumulator / (1/60)                            the leftover fraction of a step, 0 to 1
```

### Interpolation with alpha

With a 60 Hz simulation on a 144 Hz display, most frames run no step at all, and the newest state would be shown two
or three times in a row: motion would judder. Instead each entity keeps its transform from the previous step and the
current one, and rendering draws it `alpha` of the way between them (`World.InterpolatedWorld`). The picture is up to
one step behind the simulation, in return for smooth motion at any display rate.

### Input across steps

The platform reports input once per frame, but steps happen zero to eight times per frame. The `InputRouter`
(`engine/input/InputRouter.nox`) keeps two views:

- each fixed step takes a snapshot in which a press or release since the last step shows exactly once, so a jump
  tapped between two steps is never lost or seen twice;
- each frame's snapshot shows what is held and what changed this frame, for the camera and the UI.

### Minimised windows

While the window is minimised, `Render` isn't called and the clock isn't advanced, and the time spent minimised isn't
counted when it comes back, so the simulation doesn't race to catch up.
