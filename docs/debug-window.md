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

The tab row has **All** and a page for each area that has values: Errors (only while there is one), Frame, Audio,
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
- `engine/src/debug/DebugPages.nox`: `PageAssignments` (group to page), `PageSummary` (the tabs) and the filter.
- `engine/src/debug/TypedCharacter.nox`: the keys that type.
- `engine/src/host/GameHost.nox`: routes key, wheel and click events to the window, blocks the game's input while it is
  focused, and publishes the engine's values.
- Tests: `engine/tests/debug/`.
