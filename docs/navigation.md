# Navigation

How the partner finds her way. The engine builds a graph of the level's floors (`engine/src/navigation/NavigationMesh.nox`,
`lantern:::navigation`), finds routes over it with A*, and Nyx walks them (`games/platformer/game-entities/partner.nyx`).
The Nyx calls are in [`game-api.md`](game-api.md); this page is how it works and where it stops.

## The graph

`Build` samples the **static** collision world on a grid of columns, 0.4 m apart. In each column it collects every
triangle a vertical line crosses; an upward-facing one (a slope no steeper than the controller's limit) is a floor, and
it becomes a **node** when:

- nothing crosses the column within the body's height above it (headroom), and
- a body-shaped capsule standing on it touches no wall or ceiling. The capsule starts a step height above the feet, so
  stairs and kerbs don't count as walls, as in the character controller.

A column can hold several floors (a street, a bridge over it). Then it joins nodes with **links**:

| Link | When | Cost |
| --- | --- | --- |
| walk | a neighbouring node (eight directions) differs by at most a step, or by a slope the ground really follows, and a capsule can move between them | the distance, three times that next to a wall |
| drop | at an edge, ground at most `maxDrop` lower within 1.5 m, with air to fall through | distance + 1 + half the drop |
| jump | at an edge, ground up to `maxJumpUp` higher (within 1.5 m) or lower, up to `maxJumpAcross` away, with room to go up, across and down | distance + 4 + the height change |

A node nearer a wall than the body's whole radius is **tight**. Nodes keep a clear distance of only a quarter of the
radius, so a slot a little wider than the body still has nodes in it, and tight ones cost three times as much so routes
keep to the middle where there is room.

There is no climb link: a way that needs climbing (a pipe, a ladder, a block higher than a jump) has no path. That is the
partner's rule in this game, so it is the graph's too.

## Paths

`FindPath(from, to)` snaps both ends to the nearest node (within four cells and 1.6 m of height), runs A* with a straight
line estimate, and returns the nodes as points with the kind of link into each one. Then it **smooths** the route: a run of
walk points becomes the longest straight lines a full-radius capsule can walk with the ground never changing by more than a
step between samples. Drop and jump points stay, together with the point before them, so the walker knows where to leave
from.

When `to` can't be reached the path is the way to the reachable node nearest it, and `IsComplete()` is false.

## How the partner uses it

- **Leading.** At the start of each leg she plans a path to the beat. If the graph has one, she walks it and ignores the
  beat's `via` waypoints. If it has none (a beat on a moving slab, behind a lift), she walks the `via` route as before.
- **Following.** After her last beat she plans a path to the player once a second while they stand on the ground, and
  falls back to their breadcrumb trail when there is none.
- **Points.** A point counts as reached within 0.6 m (0.3 m before a jump, so she leaves from the edge, and 0.35 m at the
  end). On a jump link she presses jump while she is grounded and keeps walking at the landing; on a drop she walks off the
  edge, so her edge check is off for that point.
- **Stuck.** With no progress for two seconds she plans the path again; the second time she gives up the path and walks
  the `via` route; after that the older safety nets apply (back to the beat she last stood at, then straight to the beat).
  A fall puts her back on the last safe spot and plans again.
- **Debug.** With the HUD on (F1), the path she is walking is drawn: green for walking, orange for a drop, red for a jump.
  The HUD shows `Partner/Navigation`, the node count and which point she is on.

## Where it stops

- Moving bodies aren't in the graph (they aren't where they were when it was built). Paths run through a closed door and
  ignore a crumbling slab. The rooms that use them still work because a path that is blocked is replaced by the `via`
  route after two stuck checks; the follow-up card is `navigation-over-moving-platforms-and-gates`.
- The graph is built once, when the partner spawns. Static geometry that changes later isn't seen.
- The streets and rooftops' scripted `Companion` (`companion.nyx`) still walks straight lines between her stops.
- Link checks use sweeps of a capsule a quarter of her radius, so a path can squeeze closer to a wall than she really
  fits; she slides along the wall and the stuck check catches the rest.
