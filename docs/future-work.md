# Future work

Ideas that are out of scope for the prototype, kept here so they don't creep in.

- D3D12 backend: emit HLSL from naga and compile with DXC at build time.
- Deferred GPU destruction with per-frame fences instead of waiting for idle on release.
- SDF or TTF font atlas, UTF-8 text beyond ASCII.
- Compressed audio (Ogg Vorbis) for music.
- Audio on its own callback thread with a lock-free command queue.
- Shadow mapping for the directional light.
- A BVH instead of the uniform grid when levels get large.
- Hot reload for Nyx scripts.
- Input rebinding UI and saved settings.
