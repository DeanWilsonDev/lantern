# Future work

Ideas that are out of scope for the prototype, kept here so they don't creep in.

- D3D12 backend: emit HLSL from naga and compile with DXC at build time.
- Deferred GPU destruction with per-frame fences instead of waiting for idle on release.
- SDF or TTF font atlas, UTF-8 text beyond ASCII.
- Compressed audio (Ogg Vorbis) for music.
- Audio on its own callback thread with a lock-free command queue.
- Shadow mapping for the directional light.
- A BVH instead of the uniform grid when levels get large.
- Automatic Nyx hot reload on file change (F5 restarts the game by hand today), keeping the
  game's state across the reload (Nyx kanban `CABI-F05-T05`).
- Input rebinding UI and saved settings.

## Recommended next experiments

In the order I would try them after playing the prototype on real hardware:

1. **Tune the feel.** Play on macOS and Arch and adjust `games/platformer/settings/player-tuning.nyx`
   (F5 reloads it), the camera distances in `game-systems/third-person-camera.nyx`, and the
   engine-side `ControllerSettings::default` (`crates/physics/src/controller.rs`). The controller
   tests will flag a change that breaks a rule (stairs, slopes, coyote time).
2. **Release-build frame time.** `LANTERN_VSYNC=0 LANTERN_DEBUG_HUD=1 cargo run -p
   lantern --release` and read "Frame/Average ms" and "Worst ms". The scene is tiny, so
   anything near 16 ms points at a backend problem rather than content.
3. **A second game** in `games/`, to check that nothing in `crates/` is platformer-specific
   (D-023's rule).
4. **A second level** made in the editor and switched through `LevelScene`, to test scene
   teardown under real switching rather than restart only.
5. **Windows CI build** (GitHub Actions) for Windows and an X11 Linux setup, as the brief asks.
