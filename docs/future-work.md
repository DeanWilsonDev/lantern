# Future work

Ideas that are out of scope for the prototype, kept here so they don't creep in.

- D3D12 backend: emit HLSL from naga and compile with DXC at build time.
- Deferred GPU destruction with per-frame fences instead of waiting for idle on release.
- SDF or TTF font atlas, UTF-8 text beyond ASCII.
- Compressed audio (Ogg Vorbis) for music.
- Audio on its own callback thread with a lock-free command queue.
- Shadow mapping for the directional light.
- A BVH instead of the uniform grid when levels get large.
- Automatic Nyx hot reload on file change (F5 reloads by hand today), keeping script state
  across the reload with the prototype's `PatchClass`.
- File and line numbers for run-time errors raised inside Nyx hooks (the prototype's call API
  only returns the error variant and message today).
- Hook discovery through Nyx class reflection instead of scanning the source text.
- Input rebinding UI and saved settings.
