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

`assets/silhouettes/` holds the platformer's pieces: block, ramp, moving platform (and `moving_platform_wide`, a variant of it), gem, spawn, kill plane, the point light, spot light and sun, street lamp, lit window, photo sight, camera volume and camera shot, and `lamp_post`, a post with a point light as its child. Each root has an `editor` object (`label`, `lift`) that the editor reads for its menu entry; the `editor` values are not part of the entity.

## In the editor

When the editor starts it reads `assets/editor/palette.json`, then adds one template for every folder in `<assets>/silhouettes`. A Silhouette with the same name as a palette entry replaces it, so the Silhouette is the source of truth once it exists; the palette entries stay as the fallback when there is no Silhouettes folder. Placing a Silhouette from the asset browser or Entity menu expands it into the open world file: the root and all its descendants are inserted with new ids, parent links and unique names, as one undo step. This is a copy, not a link: later edits to the Silhouette do not reach entities already placed. A Silhouette that fails to resolve is logged and left out of the menu.

`SilhouetteWorld(library, name)` turns one Silhouette into a world file (entity 1 is the root, the rest are its descendants), which is also how the platformer Silhouettes are tested.

## Calamus from Nox

`lantern:::calamus` wraps Amanuensis's C API (`capi/` in the amanuensis repository), which `lantern-native` links: `ParseCalamus(text)` gives a `CalamusDocument` (header, `Resolve(baseJson)`), `WriteCalamus` writes an entity as Calamus text with `*` marks against a base, and `StableHash` is the hash Calamus records for a base. Values cross the boundary as JSON text.

## Known gaps

- The editor places Silhouettes (see below) but still saves world files as JSON. Writing Silhouette files from the editor, linked instances in a Calamus scene folder, and the inspector's inherited and overridden states are not built.
- There is no Silhouette editor tab yet (components tree, details, preview). It is the next slice; visual scripting comes after it.
- Types have no schema, so a Silhouette's values are only checked when a scene is loaded into a world file.
- Removing an inherited child in an instance is not supported; disable or hide it with a patch.
