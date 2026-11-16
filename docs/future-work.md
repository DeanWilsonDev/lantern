# Future work

Ideas that are out of scope for the prototype, kept here so they don't creep in. What is missing today, and what hasn't
been checked, is in `known-gaps.md`.

- Direct3D 12 backend: compile the shaders to DXIL (HLSL through DXC) beside SPIR-V and Metal Shading Language.
- A hand-written Vulkan `RenderBackend`, for full control of the renderer (board card
  `engine-follow-ups/a-vulkan-renderbackend-for-full-control-of-the-renderer`).
- Deferred GPU destruction with per-frame fences instead of waiting for idle on release.
- SDF or TTF font atlas, and UTF-8 text beyond ASCII.
- Compressed audio (Ogg Vorbis) for music.
- Audio on its own callback thread with a lock-free command queue.
- A BVH instead of the uniform grid when levels get large.
- Automatic Nyx hot reload on file change (F5 reloads by hand today), keeping the game's state across the reload.
- Input rebinding UI and saved settings.
- Transparency, instancing and a render graph (`renderer.md`, "What this renderer leaves out").
- Decals and billboard particles (`normal-maps.md`, `particles.md`).

## Recommended next experiments

In the order to try them after playing the prototype on real hardware:

1. **Check 60 fps.** `LANTERN_VSYNC=0 LANTERN_DEBUG_HUD=1 mist run game ../games/platformer`, then read "Frame/Average ms"
   and "Worst ms" on the debug HUD. Try `lantern:::graphics:::SetQuality("low")` if a machine misses 60; see
   `graphics-effects.md` for what each level turns off.
2. **Tune the feel.** Adjust `games/platformer/settings/player-tuning.nyx` (F5 reloads it) and the camera distances in
   `game-systems/third-person-camera.nyx`. The engine-side defaults are in `engine/src/physics/Controller.nox`
   (`ControllerSettings`); the controller tests in `engine/tests/physics` flag a change that breaks a rule (stairs,
   slopes, coyote time).
3. **A second game** in `games/`, to check that nothing in `engine/` is platformer-specific (D-023's rule).
4. **A second level** made in the editor and switched through `LevelScene`, to test scene teardown under real switching
   rather than restart only (the platformer already switches between the course and the streets from its menu).
5. **A Windows build**, and a Wayland run on Arch (`running.md`).
