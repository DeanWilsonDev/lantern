# The debug window

F1 shows and hides the debug window: the engine's debug values as pages you can filter, scroll and click through. It
replaces the single panel the HUD used to be, which ran off the bottom of the screen and sat over the pause menu.

## Using it

| Input | Effect |
| --- | --- |
| F1, Backquote, gamepad Back | Show or hide the window. `LANTERN_DEBUG_HUD=1` starts with it showing. |
| F3 | Focus the window (or give focus back). Focused, the game gets no input and the mouse is free. |
| Click on the window | Focus it. A click on a tab picks that page. |
| Tab, Right / Shift+Tab, Left | Next page / previous page, wrapping round. |
| Up, Down / Shift+Up, Shift+Down | Scroll a line / a page. The mouse wheel scrolls three lines. |
| Letters, digits, space | Type into the filter. Backspace removes a letter. |
| Enter | Clear the filter. |
| Escape | Clear the filter, or leave the window when there is none. |

The tab row has **All** and a page for each area that has values: Errors (only while there is one), Frame, Budget, Audio,
Renderer, Camera, Player, Companion, World, Mechanics, then any group the game publishes that has no page yet. Each tab
shows how many values it holds. The row scrolls along to keep the chosen tab visible.

The **filter** looks at every value's path (`Audio/Underruns`) and at what it shows, in any case, across all pages. Pages with
no match disappear from the tab row, the counts show the matches, and a page the filter empties falls back to All.

The window takes at most 84 characters of width and 31% of the screen, so it stays clear of the game's own HUD, and up to 24
rows of height. A longer page scrolls, with a scrollbar and a "first-last of total" count in the footer. The text scale
follows the display density (8 pixel glyphs at 1x, 16 at 2x). It is drawn before the game's UI, so the pause menu and its
backdrop are on top of it, not under it.

The **Audio** page shows whether a device opened, the voices playing, the milliseconds queued, the underrun count and each
bus (`Audio/Bus Music` and the others) as `volume gain gain`, or `muted`.

## The Budget page

The Budget page shows where each frame's time goes against the 60 fps budget of 1000/60 = 16.7 ms. Open the window with F1
and pick **Budget** (or start on it with `LANTERN_DEBUG_PAGE=Budget`).

- The heading says how long the latest frame took, and turns red with **OVER BUDGET** when it ran more than a millisecond past
  the budget (a 60 Hz display measures frames of 16.6 or 16.7 ms, so the millisecond keeps normal frames green). It also counts
  the over-budget frames among the last 120.
- The graph is one bar per frame for the last 120 frames, newest on the right. A bar stacks the phases and, in grey on
  top, the time no phase accounts for (events and the loop itself). A line marks the budget; a frame that ran over has red caps
  at the top of its bar and of the graph. The graph is 33 ms tall, or as tall as the worst frame when that is more.
- The values under it give each phase as `this frame  avg  worst` in milliseconds, with the phase's colour beside it, plus the
  frame, its average and worst, and the over-budget counts. They are `Budget/` values, so the filter finds them.

| Phase | What it times |
| --- | --- |
| Update | Engine work around the steps and the frame update: previous transforms, collision step start, input snapshots, world transforms, the UI's input and publishing these values. |
| Scripting | The game's `OnFixedUpdate`, `OnUpdate`, `OnRender` and its Chaos UI update, without the physics queries inside them. |
| Physics | The collision work scripts ask for: controller steps, ledge searches, ray casts and sphere casts. |
| Animation | Particles, the animators, foot placement and building each entity's draw. |
| Overlay | Drawing the debug window and the game's UI. |
| Render | The render backend's frame. With vsync on, the wait for the next swapchain image is in here, so this phase fills the time the others leave; turn vsync off (`LANTERN_VSYNC=0`) to see the real cost. |
| Audio | Topping up the audio queue. |

Nothing is measured while the window is hidden: the engine only reads the clock between phases while it is showing, and the
`Budget/` values are removed when it hides. The first frame after showing it starts the graph again.

## Publishing values

A value is `Group/Name`, published as text with `lantern:::debug:::SetValue("Player/Speed", "6.00")` from a script, or with
`SetText`, `SetNumber`, `SetInteger`, `SetFlag` and `SetVector` on the engine's `DebugValues` from Nox. The group decides
the page:

| Page | Groups |
| --- | --- |
| Errors | Errors |
| Frame | Frame, Script |
| Audio | Audio |
| Renderer | Renderer |
| Camera | Camera |
| Player | Player, Climb, Mantle |
| Companion | Companion, Anchor, Partner, Team |
| World | World, Game, Platform, Photo, Minigames |
| Mechanics | Gate, Switch, Crumble, Channel, Glow, Watcher |

A group in none of those is a page of its own with the group's name, so a new subsystem appears by publishing a value and
needs no registration. To put it on a page that exists, or to make a new shared one, call
`lantern:::debug:::SetPage("Rope", "Mechanics")` once; from Nox it is `PageAssignments.Assign`.

## In the code

- `engine/src/debug/DebugWindow.nox`: the window, its keys, clicks, layout and drawing.
- `engine/src/core/FrameBudget.nox`: the phase timer and the ring of the last 120 frames. `GameHost.nox` and `PhysicsApi.nox` bracket
  the phases with `Start` and `Stop`, and `GameHost.PublishBudgetValues` publishes them.
- `engine/src/debug/DebugPages.nox`: `PageAssignments` (group to page), `PageSummary` (the tabs) and the filter.
- `engine/src/debug/TypedCharacter.nox`: the keys that type.
- `engine/src/host/GameHost.nox`: routes key, wheel and click events to the window, blocks the game's input while it is
  focused, and publishes the engine's values.
- Tests: `engine/tests/debug/`.
