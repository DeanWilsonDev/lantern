# Silhouettes

A Silhouette is a named, reusable entity: a type plus preset values, and optionally children. A scene places instances of Silhouettes and stores only what differs from them. Silhouettes are stored as Calamus files, Amanuensis's human-editable entity format, so a change to a Silhouette reaches every instance that does not override that field.

This page describes what is built. The editor does not use Silhouettes yet; it still uses the palette templates (`docs/editor.md`).

## Layout on disk

A game keeps its Silhouettes under a library folder, one folder per Silhouette:

```
silhouettes/
  platform_stone/root.calamus
  platform_ice/root.calamus          # @base "silhouettes/platform_stone"
  lamp_post/
    root.calamus                     # the post
    light.calamus                    # a child
  lamp_pair/
    root.calamus
    left.calamus                     # @base "silhouettes/lamp_post": a nested Silhouette
    right.calamus
    overrides/left.light.calamus     # a patch on a descendant
```

- `root.calamus` is the root entity. Every other `.calamus` file in the folder is a child of the root, named by its local id (`light`, `left`).
- A folder named like a child (`light/`) holds that child's own children, so the hierarchy follows the folders.
- `@base "silhouettes/name"` makes the entity an instance of another Silhouette. On a root it makes a variant (`platform_ice` from `platform_stone`). On a child it nests that Silhouette, with all of its children, under the child.
- `overrides/` holds patches. A patch file is named by the dotted path of local ids from the root (`left.light.calamus`) and changes that entity wherever it came from. A patch for a path that does not exist is an error.
- The body of an entity file uses the world file's entity shape (`name`, `transform`, `shape`, `material`, `tint`, `collider`, `properties`), with vectors as `[x, y, z]` arrays. `@type` is the entity's `kind`. The keys `id`, `parent` and `kind` may not appear in a body, because the scene decides them.
- Marked lines (`* key : value | replaced`) override the base. Unmarked lines that differ from an unchanged base are hand edits and also count as overrides; see `calamus-spec.md` in Amanuensis for the rules. Calamus writes `@id` in hexadecimal.

## Resolving

Resolution runs from the inside out: the type's values (none yet, so the empty object), then the `@base` chain, then the file's own marked fields, then the folder's patches. `lantern:::silhouette:::SilhouetteLibrary` resolves a name to a `ResolvedSilhouette`: entity 0 is the root, the rest are its descendants with their dotted paths, parents first. A missing Silhouette (`missing silhouette 'silhouettes/x'`) and a cycle (`silhouette cycle: a -> b -> a`) are errors that name the Silhouettes; nothing falls back to the raw type.

## Scenes

`lantern:::silhouette:::LoadScene(library, sceneFolder, name)` reads a folder of instance files. Each file needs an `@id` (the world id of the instance's root) and an `@base` (the Silhouette). An instance's descendants take the ids after the largest `@id`, in file order. A folder `<file>.overrides/` beside an instance file patches that instance's entities by dotted path. The result is an ordinary `WorldFile`, validated like any other.

## The platformer's Silhouettes

`games/platformer/assets/silhouettes/` holds the platformer's pieces: block, ramp, moving platform (and `moving_platform_wide`, a variant of it), gem, spawn, kill plane, the point light, spot light and sun, street lamp, lit window, photo sight, camera volume and camera shot, and `lamp_post`, a post with a point light as its child. Each root has an `editor` object (`label`, `lift`) that the editor reads for its menu entry; the `editor` values are not part of the entity.

## In the editor

When the editor starts it reads `games/platformer/assets/editor/palette.json`, then adds one template for every folder in `<assets>/silhouettes`. A Silhouette with the same name as a palette entry replaces it, so the Silhouette is the source of truth once it exists; the palette entries stay as the fallback when there is no Silhouettes folder. Placing a Silhouette from the asset browser or Entity menu expands it into the open world file: the root and all its descendants are inserted with new ids, parent links and unique names, as one undo step. This is a copy, not a link: later edits to the Silhouette do not reach entities already placed. A Silhouette that fails to resolve is logged and left out of the menu.

`SilhouetteWorld(library, name)` turns one Silhouette into a world file (entity 1 is the root, the rest are its descendants), which is also how the platformer Silhouettes are tested.

## The Silhouette tab

Double-click a Silhouette's card in the asset browser, the Silhouettes panel or the Silhouettes drawer page (or select it and press **Edit** in the asset preview) to open it as a document in a tab of its own, beside Level. Any number of Silhouettes can be open at once, each in its own tab named after it; opening one that is already open shows its tab. A renamed Silhouette's tab follows the new name. The document is laid out like a Blueprint editor. A header names the Silhouette (and its base, for a variant) and holds what is done to the whole Silhouette: New variant, New Silhouette, Rename, Set base, Refresh placed and Delete, all reading the name box beside them. Under it are three areas:

- **Hierarchy** (left): what the Silhouette is made of, the root first and its children indented below it, each with an icon for its kind. A child that is another Silhouette (a child file with an `@base`) shows that Silhouette's name and a layers icon, and the components it brings with it are greyed and marked `inherited`, since they have no file in this Silhouette. **Add** opens a list of game components to hang under the selected row, the bin deletes the selected row, a double-click renames a row (Enter commits), and **Move under...** and **Move to root** re-parent it.
- **Viewport** (centre): the Silhouette turning, drawn by the same renderer as the asset preview.
- **Inspector** (right): every value of the selected row, grouped by the first part of its key (`transform`, `shape`, `properties`, `editor`, and `general` for keys with no dot), each in a field of its type. For a variant, a value the Silhouette sets itself is marked `*` and has a revert button that puts it back to its base's value. At the foot, a key box and a JSON value box set any property, new or not, and the behaviour graph buttons set a `scripted` Silhouette's `graph`.

Above the three areas, two tabs switch the document between **Viewport** (the three areas above) and **Event Graph**. Event Graph shows the Silhouette's own graph on the same canvas as the Graph tab, without the list of graphs. The graph is `graph.json` in the Silhouette's folder, beside `root.calamus`, and the graph code opens it as `silhouette:<name>` (`lantern:::editorbackend:::SilhouetteGraphFile`). A Silhouette with no graph yet shows a **Create graph** button, which writes a graph whose `EventGraph` function holds the **BeginPlay**, **Update** and **FixedUpdate** events. Each event gives the placed `entity` and its time (`frameTime` for Update, `fixedStepTime` for FixedUpdate) on pins of its own, and the graph's variables keep their values from one frame to the next, since the graph becomes a class (`LampPostBehaviour`) with a method per event. The game doesn't run these graphs yet, and saving one writes only its JSON; see `docs/silhouette-editor.md` for the slices still to come.

Ctrl+W closes the tab being shown, and the tab to its left takes over. It reads the same templates the Silhouettes panel uses, so what the tab shows is what placing it would make. `lantern:::editorbackend:::ComponentOrigin` tells the hierarchy where each component comes from.

### Editing in the tab

The tab edits the Silhouette's files directly, so there is no separate save: each change is written when you press the button, and the tab and the Silhouettes panel read the folder again straight away.

- **Add a child.** Select the component the child should hang from and press one of `block`, `point_light`, `spot_light`, `gem` or `spawn`. A file is written in the parent's folder (`lamp.calamus` for a child of the root, `pole/lamp.calamus` for a child of `pole`) named after the kind, or `block2`, `block3` when that name is taken. It starts with its kind, a name and a position at its parent's origin.
- **Rename.** Type a name and press **Rename**. Only the `name` line of that component's file is rewritten; the file keeps its place, so nothing that points at it breaks. A Silhouette with an `@base` can't be renamed this way yet.
- **Delete.** **Delete** removes the selected component's file and everything under it. The root can't be deleted.
- **Rename Silhouette.** Type a new name in the name box beside New variant and press **Rename Silhouette**. The folder moves and every Silhouette made from this one gets its `@base` rewritten; a taken or unusable name changes nothing. A Silhouette that replaced a built-in palette template of the same id leaves that id out of the palette until the editor restarts.
- **Typed fields.** Each value row is a field of its type: a checkbox for a bool, an input for a number, a text input (shown without quotes) for a string, one input per axis for a vector of two to four numbers, and a raw JSON input for anything else. Enter commits the field; the label still fills the key and JSON boxes for Set value and Revert to base. `own` values are marked with `*` as before.
- A component the Silhouette gets from its base has no file of its own, so it can't be renamed or deleted, and it can't take new children, in the variant. Change it in the base.

- **New Silhouette and New variant.** Type a name (lowercase letters, digits and underscores) and press **New Silhouette** for an empty block, or **New variant** for a variant of the Silhouette open in the tab. A variant's root has the open Silhouette as its `@base` and sets only its name and menu label, marked `*`, so it follows every later change to the base. The new Silhouette opens in the tab and appears in the asset browser. A name that is taken or not allowed is refused and the reason is shown under the buttons.

- **Set a value and revert it.** Type a dotted key (`transform.position`, `material`) and its value as JSON (`"wall"`, `[1, 0.5, 1]`, `true`), then press **Set value**; text needs quotes. Clicking a value in the list fills both boxes with it. For a variant, **Revert to base** puts the key back to what the base has (and removes it when the base has none), so it goes back to being inherited. The file is rewritten through Calamus, so a variant still marks only what differs from its base; numbers are compared as numbers, so `0` and `0.0` are the same.

- **Move a component.** Select it and press **Move under...**, then click the component it should hang from; **Move to root** does the same for the root. Its file and the folder of its children move together and it keeps its name. A component can't go under itself or its own children, the root can't move, and a name already used by a child of the new parent is refused. Components inherited from a base can't be moved. Patches in `overrides/` are named by dotted path, so a moved component's patch no longer finds it.
- **Set base.** Type a Silhouette's name in the name box and press **Set base** to point the open variant at it. If the result doesn't resolve (a missing Silhouette, or a cycle) the file is put back and the reason is shown. Only a variant has a base to change.
- **Delete Silhouette.** Removes the whole folder, unless another Silhouette is made from this one. It also leaves the tab and the asset browser.

Changing a Silhouette doesn't change the entities already placed in a world on its own; it changes what is placed from then on. A placed root carries a `silhouette` property naming where it came from, and **Refresh placed instances** (beside Rename Silhouette) rebuilds every such instance in the open level from the Silhouettes as they are now. Each root keeps its id, name, parent and transform and takes everything else from the Silhouette; its children are rebuilt, keeping their names in order, so anything changed on the instance or its children is lost. It is one undo step. A root placed before this property existed has none and isn't refreshed.

## Calamus from Nox

`lantern:::calamus` wraps Amanuensis's C API (`capi/` in the amanuensis repository), which `lantern-native` links: `ParseCalamus(text)` gives a `CalamusDocument` (header, `Resolve(baseJson)`), `WriteCalamus` writes an entity as Calamus text with `*` marks against a base, and `StableHash` is the hash Calamus records for a base. Values cross the boundary as JSON text.

## Known gaps

- The editor places Silhouettes (see below) but still saves world files as JSON. Writing Silhouette files from the editor, linked instances in a Calamus scene folder, and the inspector's inherited and overridden states are not built.
- The Silhouette editor tab shows the components tree and each component's resolved values, and adds, renames and deletes components. A value is edited in a field of its own type (below), or by typing its key and JSON. Visual scripting comes after them.
- Types have no schema, so a Silhouette's values are only checked when a scene is loaded into a world file.
- Removing an inherited child in an instance is not supported; disable or hide it with a patch.
