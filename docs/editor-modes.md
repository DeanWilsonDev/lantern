# Editor modes

The world editor works on the level in one mode at a time, as Blender's Object and Sculpt modes and Unreal's editor
modes do. A mode is what is being done to the level, and it decides which menus, toolbar buttons, panels and drawer
pages are shown, so the screen holds only what the job in hand needs. Undo and redo are shared by every mode: a sculpt
stroke and a move are steps in the same history.

| Mode | Shortcut | Icon | What it is for |
| --- | --- | --- | --- |
| Select | Shift+1 | cursor | Picking, placing (the Silhouettes panel) and transforming entities |
| Terrain | Shift+2 (or B) | chart-line | Sculpting and generating terrain |

There are no mode buttons: the mode follows what is being worked on. Selecting a terrain adds a Terrain group of brushes
to the tool palette, and picking one of them enters Terrain mode; Done at the foot of the palette, or Escape, goes back
to Select with the terrain still selected. Shift+1 and Shift+2 still switch, and the Window menu lists the modes, so the
command palette finds them ("Window: Terrain Mode"). The status bar's first chip names the mode.

Placing anything is part of Select mode. Everything that can be placed is a Silhouette (a block, a light, a camera
volume and a terrain are each an entity with its own properties), so they all come from the one Silhouettes panel.

## Silhouette and graph tabs

Editing a Silhouette, and the graph, aren't modes: each is a document in a tab of its own. While one of those tabs is up,
the level's menus and panels (the Silhouettes panel, Selection, Entity, View, Terrain, the outliner and the details panel) are hidden and
only the tab's own panels show; the status bar says "Silhouette tab" or "Graph tab". The level's mode comes back with
the Level tab. Below, "Silhouette" and "Graph" in the Modes columns mean those tabs.

## How Terrain mode works

Terrain mode is the terrain brush. Entering it turns the brush on and leaving it turns the brush off, and B, which turns
the brush on and off, moves between Select and Terrain mode. Escape while sculpting puts the brush down and keeps the
selection; a second Escape clears the selection, as it does in Select mode. Entering Terrain mode also opens the drawer on its Terrain
page, since the brush settings live there. Asking for a page another mode owns (Alt+6 for Terrain while selecting, say)
switches to that mode.

The editor starts in Select mode on the Level tab, because the terrain brush always starts off (D-275), so opening the
editor never leaves a left click sculpting. The panel choices that are remembered (which panels are shown, and the
drawer's page) are kept as before.

## What each mode shows

"All" means both modes and both tabs.

### Menu bar

| Menu | Modes |
| --- | --- |
| File | All |
| Edit | All. Rename, Duplicate, Delete and the Snap, Local Space and Pivot toggles only in Select |
| Selection | Select |
| View | Select, Terrain (it changes the level viewport) |
| Entity | Select |
| Terrain | Terrain |
| Window | All. The Outliner and Details toggles only in Select; the drawer pages only those of the mode |
| Help | All |

### Toolbar

The toolbar keeps only what is used all the time: Play in the middle, and the command palette and settings on the
right. It is the same in every mode.

Play is white while nothing runs and green while the game runs, when it reads Stop. Beside it while the game runs is
Pause, which turns orange while the game is paused and resumes it when pressed again. A paused game keeps its window
and keeps drawing, but its world holds still.

Everything else lives where it belongs: New, Open and Save in the File menu; Undo, Redo, Duplicate and Delete in Edit;
Frame in View; Group in Entity. The tools are in the tool palette (below), and Snap, World or Local space and the pivot
are named toggles in the viewport's header in Select mode.

### Tool palette

The tool palette is a narrow column of the mode's tools, like Photoshop's tools panel. It sits docked at the viewport's
left, or floats over the window. The grip at its top shows while the mouse is over the palette: drag it out by the grip,
and drop it near the window's left edge to dock it again. Whether it floats, and where, is remembered.

| Tools | Modes |
| --- | --- |
| Move, Rotate, Scale | Select |
| A Terrain group with the seven brushes | Select, while a terrain is selected |
| The seven brushes, New Terrain, Done | Terrain |

The Silhouette and graph tabs have no palette.

### Panels

| Panel | Modes |
| --- | --- |
| Outliner | Select |
| Side rail (the Silhouettes button) | Select |
| Details | Select |
| Viewport header (Camera and Show menus) | Select, Terrain (it belongs to the Level tab) |
| Snap, World or Local, Pivot toggles in the viewport header | Select |
| Right-click menu of an entity | Select (it opens from the outliner) |
| Document tabs | All |
| Status bar | All |

The outliner, the details panel, the Silhouettes panel, the asset preview and the content drawer can each be moved,
like the tool palette. A grip shows along a
panel's top edge while the mouse is over it; drag the grip and let go near the window's left or right edge to dock the
panel in a column on that side, near the bottom to dock it under the viewport, or anywhere else to float it there. While
dragging, a blue rectangle shows where the panel will go. Panels in the same place share it, with handles between them
to resize, and a side with no panel takes no room. A floating panel moves with its grip. Where each panel is, and where
a floating one sits, is remembered. Each panel fills the room it is given: its rows, fields and grids stretch when it is
made wider. A card can be dragged out of any panel, floating or docked, and dropped on the viewport.

### Drawer pages

| Page | Modes |
| --- | --- |
| Assets | Select |
| Silhouettes | Silhouette (on the level it is a panel of its own) |
| Terrain | Terrain |
| Console | All |
| History | All |

When the page the drawer was last on doesn't belong to the mode, the drawer shows the mode's first page, without
forgetting the remembered page. The graph tab has its own node list and preview, so it shows only Console and History in
the drawer.

### Command palette

The palette lists the commands of the menus the mode shows, plus the two modes, and the tools in Select mode. A command the mode hides
can't be found there either.

## The Silhouettes panel

The Silhouettes panel places something on the level. It is one of the movable panels: it starts docked on the left,
hidden, and its grip docks it on either side or under the viewport, or floats it. Its button on the side rail at the
viewport's right edge in Select mode shows and hides it, as do Shift+A, Alt+5 and Window > Silhouettes; a floating one
comes out at the mouse. It stays out while things are placed, so several can go down in a row.

The panel is like a small file browser: a search box over a column of categories and a grid of cards, each with a live
thumbnail. The cards wrap into as many columns as the panel's width holds and scroll up and down. A click on a category
shows its cards, and the search keeps the cards whose name or label holds the text. Pressing a card shows it in the asset
preview; dragging a card into the viewport places it where the mouse lets go.

## The asset preview

The template picked last, in the asset browser, the Silhouettes panel or the Silhouettes drawer page, turns in the asset
preview, with its name and an Edit button that opens its Silhouette tab. The preview is a movable panel of its own and
starts beside the drawer; Window > Asset Preview shows and hides it.

| Category | What is in it |
| --- | --- |
| All | Everything below |
| Shapes | Blocks, ramps and platforms |
| Lights | Point, spot and sun lights |
| Cameras | Camera volumes and camera shots |
| Gameplay | Player starts, spawns and kill planes |
| Terrain | New Terrain (with the generator settings on the Terrain page), and terrain templates |
| Props | Silhouettes made of several parts, and any kind none of the others name (a gem) |

A template goes in the category of its kind, with the number of each beside its name. A category with nothing in it is
left out, except All and Terrain, which always offer New Terrain.
