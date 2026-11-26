# Editor modes

The world editor works on the level in one mode at a time, as Blender's Object and Sculpt modes and Unreal's editor
modes do. A mode is what is being done to the level, and it decides which menus, toolbar buttons, panels and drawer
pages are shown, so the screen holds only what the job in hand needs. Undo and redo are shared by every mode: a sculpt
stroke and a move are steps in the same history.

| Mode | Shortcut | Icon | What it is for |
| --- | --- | --- | --- |
| Select | Shift+1 | cursor | Picking, placing (the Add menu) and transforming entities |
| Terrain | Shift+2 (or B) | chart-line | Sculpting and generating terrain |

The mode switcher is the pair of buttons after Undo and Redo on the toolbar. The Window menu lists the modes too, so the
command palette finds them ("Window: Terrain Mode"). The status bar's first chip names the mode.

Placing anything is part of Select mode: basic shapes, lights, cameras, spawn points and Silhouettes all come from the one
Add menu, so placing a Silhouette is the same job as placing a block.

## Silhouette and graph tabs

Editing a Silhouette, and the graph, aren't modes: each is a document in a tab of its own. While one of those tabs is up,
the level's menus and panels (Add, Selection, Entity, View, Terrain, the outliner and the details panel) are hidden and
only the tab's own panels show; the status bar says "Silhouette tab" or "Graph tab". The level's mode comes back with
the Level tab. Below, "Silhouette" and "Graph" in the Modes columns mean those tabs.

## How Terrain mode works

Terrain mode is the terrain brush. Entering it turns the brush on and leaving it turns the brush off, and B, which turns
the brush on and off, moves between Select and Terrain mode. Entering Terrain mode also opens the drawer on its Terrain
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
| Add | Select |
| Selection | Select |
| View | Select, Terrain (it changes the level viewport) |
| Entity | Select |
| Terrain | Terrain |
| Window | All. The Outliner and Details toggles only in Select; the drawer pages only those of the mode |
| Help | All |

### Toolbar

The toolbar keeps only what is used all the time: the mode switcher (Select and Terrain, each named) on the left, Play
in the middle, and the command palette and settings on the right. It is the same in every mode.

Play is white while nothing runs and green while the game runs, when it reads Stop. Beside it while the game runs is
Pause, which turns orange while the game is paused and resumes it when pressed again. A paused game keeps its window
and keeps drawing, but its world holds still.

Everything else lives where it belongs: New, Open and Save in the File menu; Undo, Redo, Duplicate and Delete in Edit;
Frame in View; Group in Entity. The tools are in the tool palette (below), and Snap, World or Local space and the pivot
are named toggles in the viewport's header in Select mode.

### Tool palette

The tool palette is a narrow column of the mode's tools, like Photoshop's tools panel. It sits docked at the viewport's
left, or floats over the window: drag it out by the grip at its top, and drop it near the window's left edge to dock it
again. The pin button at its foot docks or floats it at once. Whether it floats, and where, is remembered.

| Tools | Modes |
| --- | --- |
| Add, Move, Rotate, Scale | Select |
| The seven brushes, New Terrain | Terrain |

The Silhouette and graph tabs have no palette.

### Panels

| Panel | Modes |
| --- | --- |
| Outliner | Select |
| Details | Select |
| Viewport header (Camera and Show menus) | Select, Terrain (it belongs to the Level tab) |
| Snap, World or Local, Pivot toggles in the viewport header | Select |
| Right-click menu of an entity | Select (it opens from the outliner) |
| Document tabs | All |
| Status bar | All |

### Drawer pages

| Page | Modes |
| --- | --- |
| Assets | Select |
| Silhouettes | Select, Silhouette |
| Terrain | Terrain |
| Console | All |
| History | All |

When the page the drawer was last on doesn't belong to the mode, the drawer shows the mode's first page, without
forgetting the remembered page. The graph tab has its own node list and preview, so it shows only Console and History in
the drawer.

### Command palette

The palette lists the commands of the menus the mode shows, plus the two modes, and the tools in Select mode. A command the mode hides
can't be found there either.

## The Add menu

The Add menu places something on the level. It opens from the Add button on the tool palette, from Shift+A over the
viewport, and from the Add title in the menu bar, all in Select mode. It has a column of categories and a column of the
things in the hovered category:

| Category | What is in it |
| --- | --- |
| Shapes | Blocks, ramps and platforms |
| Lights | Point, spot and sun lights |
| Cameras | Camera volumes and camera shots |
| Gameplay | Player starts, spawns and kill planes |
| Terrain | New Terrain (with the generator settings on the Terrain page) |
| Silhouettes | The project's own Silhouettes |

A template goes in the category of its kind; one made from a Silhouette that isn't one of those kinds goes in
Silhouettes. A new entity is placed in front of the camera, as Add to World in the drawer does, and becomes the
selection.

The Silhouettes category is a searchable grid of thumbnails rather than a list, like a small file browser, since a
project can gather hundreds of Silhouettes. It holds every Silhouette in the project, those also listed under Shapes or
Lights included. Typing in its search box keeps the ones whose name or label holds the text, and pressing a card places
that Silhouette.
