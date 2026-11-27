# The Silhouette editor as a document

Plan for turning the Silhouette tab into a Silhouette editor in the way a Blueprint editor works: a Silhouette opens as a
document of its own, shows what it is made of, and owns its graph. Started 2026-11-27 from Dean's ask. The decisions
below are provisional until Dean reviews them; they get D-numbers in `decisions.md` as each slice lands.

## What Dean asked for

- Double-clicking a Silhouette in the asset browser opens the Silhouette editor for it, as a document (a tab).
- In the document: the viewport in the centre, an inspector on the right with every property of the Silhouette, and a
  hierarchy on the left with what it is made of, including child Silhouettes and game components.
- A tab inside the document switches to the Silhouette's graph, where pins are pulled off event nodes such as BeginPlay,
  Update and FixedUpdate.
- Graphs and Silhouettes stop being two separate things: the graph belongs to the Silhouette.

## Where things are today

- The Silhouette tab (`SilhouettePage.chaos`) exists, but only one Silhouette can be open, it opens from an Edit button,
  and it has three columns: the components tree and add buttons, a preview with the chosen component's values, and a
  column of buttons. It reads like a form, not a document.
- A behaviour graph is a file in `game-graphs/` with free functions. An entity of kind `scripted` names a graph and a
  function in its `graph` and `function` properties, and the platformer calls it every fixed step with the total time.
  Graph variables are locals, so a behaviour can't keep state between steps.
- A material graph already belongs to its Silhouette: `material.graph.json` sits beside the material's `root.calamus`
  and is opened in the Graph tab as `material:<name>`.
- Placed roots already carry a `silhouette` property naming the Silhouette they came from.

## Decisions

1. **A Silhouette document per Silhouette.** Any number of Silhouettes can be open, each in a tab of its own beside Level
   and the entity tabs, named after the Silhouette. Opening one that is already open shows its tab. Double-clicking a card
   in the asset browser, the Silhouettes panel or the Silhouettes drawer page opens it; Edit in the asset preview stays.
   Ctrl+W closes the tab being shown.
2. **Three areas, Blueprint style.** Left: the Hierarchy. Centre: the document's own tabs, **Viewport** and **Graph**.
   Right: the Inspector. The Silhouette's own actions (new variant, rename, set base, delete, refresh placed instances)
   go in a header row above the three areas, out of the inspector.
3. **The hierarchy shows what it is made of.** The root first, then its children indented, each with an icon for its kind.
   A child that is another Silhouette (a child with an `@base`) is marked as a Silhouette and its own children show under
   it as inherited (greyed, not editable here). Adding a child is an Add button at the top of the hierarchy with a menu of
   game components (block, lights, gem, spawn and the rest of the placeable kinds) and Silhouettes. Rename by
   double-clicking a row, delete with the bin button, re-parent with Move under (dragging a row onto another comes
   later).
4. **The inspector shows every property of the selected row.** The root is selected when the document opens. Values
   use the existing typed fields, grouped by their first key (`transform`, `material`, `properties`...), with the `*`
   mark for values a variant sets itself and a Revert button beside each such value. Adding a property is a key box and
   a value box at the foot.
5. **The graph lives in the Silhouette's folder.** A Silhouette's graph is `graph.json` beside its `root.calamus`, like
   `material.graph.json` for a material. It's opened in the editor's graph code as `silhouette:<name>`. The game never
   reads it: the build turns it into Nyx in `game-graphs/generated/`, as it does for the graphs there now.
6. **Event nodes.** The Silhouette's graph has one **Event Graph** canvas holding event nodes: **BeginPlay** (once, when
   the level starts), **Update** (each frame, with `frameTime`) and **FixedUpdate** (each fixed step, with
   `fixedStepTime`). Each event node gives `entity` (the placed root's id) and its time as value pins. An event appears
   once at most. A new Silhouette graph starts with the three events placed and unwired, as Unreal does.
7. **A Silhouette's graph is a class.** The generated Nyx is a class per Silhouette (`LampPostBehaviour`) with one method
   per event, and the graph's variables become its fields, so a placed instance keeps its state between frames, as
   Blueprint variables do. `GraphBehaviours:::Create(silhouette, entity)` makes the instance for a placed root (Nyx has no
   reflection, so the lookup is generated, as `Run` is now).
8. **The game runs them.** When a level loads, every entity with a `silhouette` property whose Silhouette has a graph gets
   an instance; BeginPlay runs once, then FixedUpdate runs in `OnFixedUpdate` and Update in `OnUpdate`.
9. **Material Silhouettes.** A material Silhouette's document shows its material graph in the Graph tab and a material
   preview in the Viewport tab, so material graphs move into their Silhouette documents too.
10. **Standalone graphs.** Behaviour graphs move into the Silhouette that uses them (`lamp_pulse` becomes the `scripted_lamp`
    Silhouette's Update event), and the `scripted` kind with its `graph` and `function` properties is retired once nothing
    uses it. A graph that is a pure function called from hand-written Nyx (`crumble_phase`) stays in `game-graphs/` as a
    function library, still opened from the Graph tab. Dean chose this on 2026-11-27.

## Slices

Each slice is its own pull request against `nox-proto`, stacked on the one before.

1. **Open as documents.** Several Silhouette tabs, double-click to open from every card, Ctrl+W.
2. **Hierarchy, viewport, inspector.** The three-area layout with the header row, the hierarchy with kind icons and
   nested Silhouettes, the grouped inspector.
3. **The Graph tab inside the document.** The canvas reused inside the Silhouette document, `silhouette:<name>` graphs
   saved as `graph.json` in the Silhouette folder (D-291). The material graph of a material Silhouette came with slice 6.
4. **Event nodes and the generated class.** `event` nodes (BeginPlay, Update, FixedUpdate) in the model, the pins, the
   menu and the generator, variables as fields (D-292).
5. **The game runs Silhouette graphs.** The game's `SilhouetteBehaviour` base class, Silhouette graphs built into
   `game-graphs/generated/` with `GraphBehaviours:::Create`, the level scene making instances and calling the events;
   `lamp_pulse` migrates into its Silhouette; the `scripted` kind is retired (D-293).
6. **Material Silhouettes as documents.** A material card opens on double-click; the document shows the material on
   the preview sphere with its values in the inspector, and its material graph in a Material Graph tab (D-294).
7. **Later.** More events (OnTouch, OnOverlap with the player, OnInteract), a "self" node for the instance's own
   entity and children, per-instance property reads.

## Answered by Dean

- 2026-11-27: standalone graphs migrate as decision 10 says: behaviour graphs move into the Silhouette that uses them,
  pure function graphs stay as function libraries in `game-graphs/`.
