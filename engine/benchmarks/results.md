# Benchmark results

## Headless platformer, 2026-11-05

`games/platformer` loads under the autopilot and then runs 600 frames at 60 Hz with the headless
renderer, the same setup as the trace test. Each build ran it five times. The numbers are medians.

| Build | Load | 600 frames | Per frame |
| --- | --- | --- | --- |
| Rust, `cargo test --release` | 38.9 ms | 292.7 ms | 0.488 ms |
| Nox, `--release`, x86-64 backend | 42.2 ms | 362.1 ms | 0.604 ms |
| Nox, `--release --backend c` | 37.9 ms | 310.0 ms | 0.517 ms |

- **Machine:** a 4-core cloud VM (Intel Xeon at 2.8 GHz), so run to run the times vary by up to 20%.
- **Versions:** lantern `nox-proto` at the L7 merge, and nox-systems-language `proto` at `102cd7d`.
- **Where the time goes:** most of each frame is the game's Nyx code, which runs in the same Nyx
  interpreter (`liblantern-native`, built Release) in all three builds.
- **Against 60 fps:** every build stays far inside the 16.7 ms a frame that 60 fps allows.

### How to run it

- **Rust:** from the repository root, run
  `cargo test --release -p lantern-game-host --test headless_game the_platformer_autopilot_timing -- --ignored --nocapture`.
- **Nox:** from `engine/benchmarks/headless-platformer`, build with `nox build --release -o /tmp/bench`, adding
  `--backend c` for the C backend. Then run `/tmp/bench` from `engine/`, with `LIBRARY_PATH` and `LD_LIBRARY_PATH` set
  as `tools/run-tests.sh` sets them.
