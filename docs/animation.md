# Skeletal animation

Characters move their bodies with a skeleton: a tree of joints, clips that pose it over time, and an animator that picks and
blends clips from a few facts the game reports. The player and the companion wear the mannequin, a 17-joint humanoid built in
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

## Not built yet

- Foot placement on slopes and stairs (a two-bone IK pass over the finished pose).
- Importing skins and animations from glTF; the mannequin's clips come from formulas.
- The partner in the team room still wears a capsule.
- More than 32 joints, additive layers and masks (a wave over a run), and animation events (footstep sounds).
