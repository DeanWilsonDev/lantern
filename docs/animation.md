# Skeletal animation

Characters move their bodies with a skeleton: a tree of joints, clips that pose it over time, and an animator that picks and
blends clips from a few facts the game reports. The player, the companion and the team room's partner wear the mannequin, a 17-joint humanoid built in
code, so nothing needs importing to see it work.

## The pieces

- **Skeleton** (`engine/src/engine/animation/Skeleton.nox`): joints with a parent, a bind pose and an inverse bind matrix.
  `MAX_JOINTS` is 32.
- **Clip** (`Clip.nox`): keyframed translation, rotation and scale per joint. A clip samples at any time, looping or not.
- **Rig** (`Rig.nox`): a skeleton and its clips. Rigs are shared; each character has its own animator.
- **Animator** (`Animator.nox`): named states (one clip, or a *blend state* that mixes clips by a parameter), parameters, and
  transition rules. It crossfades with a smoothstep; a transition that starts mid-fade freezes the pose it was showing and fades
  from that, so a changed mind never pops. It also holds the look-at layer below.
- **Mannequin** (`Mannequin.nox`, `animation-mesh/MannequinMesh.nox`): the humanoid, eight clips (idle, walk, run, jump, fall,
  crouch, wave, reach) and its skinned mesh. It faces -Z, stands 1.8 m tall and has its origin at the feet.
- **AnimationSystem** (`entities/AnimationSystem.nox`): updates every animator each frame and adds its skinned draw, with the
  joint palette, to the render world.

## In the renderer

A skinned mesh has a second vertex buffer: four joint indices and four weights per vertex (`SkinVertex`, 8 bytes). A draw with
`jointOffset >= 0` uses `lit-skinned.vert`, and in the shadow pass `shadow-skinned.vert`, with the draw's 32 palette matrices in a
uniform block. The fragment shaders are the ordinary ones, so skinned characters get lighting, shadows, fog and every effect
the rest of the scene has. Skinned meshes' bounds are grown by 1 m so culling does not clip a stretched arm.

## From Nyx

```nyx
rig := lantern:::animation:::MannequinRig();
animator := lantern:::animation:::Spawn(entity, rig, texture, "E8A33D");
lantern:::animation:::AddBlendState(animator, "locomotion", "speed");
lantern:::animation:::AddBlendClip(animator, "locomotion", "idle", 0.0f);
lantern:::animation:::AddBlendClip(animator, "locomotion", "run", 5.0f);
lantern:::animation:::AddTransition(animator, "locomotion", "jump", "jump", "trigger", 0.0f, 0.08f);
lantern:::animation:::SetFloat(animator, "speed", currentSpeed);
```

The game wraps this in `games/platformer/game-entities/character-animator.nyx` (`CharacterAnimator`): create it once, then report
`Observe(position, isGrounded, dt)` each step, `Jump()`, `Wave()`, `SetCrouching(bool)` and `LookAt(target, weight)`. The game
never plays a clip itself. `docs/game-api.md` lists every function.

## The look-at layer

`SetLookTarget(animator, target, weight)` turns the chest, neck and head toward a world point. The arc is clamped to the
animator's limit (75 degrees) and shared 0.2, 0.3 and 0.5 between chest, neck and head, and the weight and target are smoothed so
glances ease in and out. The companion looks at the player while they are within 12 m.

## Foot placement

On uneven ground the clips alone would put feet in the air or through the step. After an animator has made its pose, the
feet are planted on the ground they are really over (`FootPlacer`, `engine/src/engine/animation/FootPlacement.nox`):

1. Each frame the host casts a ray straight down at each foot, from 0.5 m above the character's own ground, and tells the
   animation system what it hit (`PlaceAnimatedFeet`, `engine/src/host/FootPlacementHost.nox`).
2. The height of each surface, relative to the ground under the character's origin, is smoothed (14 per second).
3. The hips drop by the deepest drop under a planted foot. Each planted ankle is moved to its surface with two-bone IK
   (`SolveTwoBoneLeg`: the law of cosines for the knee, then a swing of the thigh; the knee bends toward the character's front)
   and the foot is tilted to the surface's slope.

A foot that is in the air in the animation (the swing of a stride, a jump or a fall) is left alone: a foot's weight is 1 at
6 cm above its resting height and 0 at 22 cm. A game turns the whole thing on with `SetFootPlacement(animator, true)`;
`CharacterAnimator` does, and switches it off while the character is in the air. A skeleton needs joints named `leftThigh`,
`leftShin`, `leftFoot` and the right ones (as the mannequin's are); any other rig is left as it is.

## Footsliding

A clip knows how fast its feet carry a character when played as authored (`AnimationClip.groundSpeed`, in metres a second;
0 means unknown). A blend state mixes two neighbouring clips by the character's speed, and plays at that speed divided by the
mixed ground speed, between 0.6 and 1.6, so a run played at 5 m/s with a 3.7 m/s run clip plays 1.34 times as fast and the
planted foot keeps up with the ground. Below 0.2 m/s, or when a clip does not say, it plays as authored.

## Importing a character from glTF

```nyx
rig := lantern:::animation:::LoadRig("characters/runner.glb", 1.0f, 180.0f);
animator := lantern:::animation:::Spawn(entity, rig, texture, "FFFFFF");
lantern:::animation:::AddState(animator, "idle", "Idle", true);
```

`LoadRig(path, scale, yawDegrees)` reads the first skin of a `.glb` or `.gltf` file under the assets folder and returns a rig
like `MannequinRig()` does. A model facing +z, as glTF models usually do, wants a yaw of 180 to face Lantern's -z; a model
modelled in centimetres wants a scale of 0.01. The model is put with its feet on the ground. In Nox,
`lantern:::animation:::LoadGltfRig(path, options)` and `GltfRigFromBytes(bytes, directory, options)` return the skeleton, clips
and mesh to place yourself.

What comes across:

- **Joints:** the first skin's joints become the skeleton, parents first, named from their nodes, at most 32 (a larger skin is
  an error that says so). The armature node above the root joint is folded into the root.
- **The mesh:** every triangle primitive of every mesh node that uses the skin, with positions, normals, the first texture
  coordinates, vertex colours times the material's base colour, and four joints and weights a vertex.
- **Clips:** every animation becomes a clip named for it, from translation and rotation channels with linear or step
  interpolation. Cubic spline channels are read at their keys without tangents.

What does not: scale channels, morph targets, textures (a game gives the texture), and non-joint nodes between two joints.

## Not built yet

- Locking a planted foot to the world during its stance (foot placement matches the ground's height and slope, and stride
  matching keeps the feet near the ground's speed, but a foot can still drift a little on a turn).
- Moving platforms: the ray sees the platform, but the platform's own motion is not carried into the feet.
- More than 32 joints, additive layers and masks (a wave over a run), and animation events (footstep sounds).
