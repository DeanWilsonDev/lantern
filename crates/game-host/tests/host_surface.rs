//! `games/platformer/engine-host-surface.json` describes the engine API to `nyx check` and
//! nyx-lsp, which can't load the Rust host. It has to match what the host registers:
//! `LANTERN_UPDATE_HOST_SURFACE=1 cargo test -p lantern-game-host --test host_surface` rewrites it.

use std::path::PathBuf;

fn committed_manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../games/platformer/engine-host-surface.json")
}

#[test]
fn the_committed_host_surface_matches_the_engine_api() {
    let written = lantern_game_host::engine_host_surface().expect("the engine API registers");
    let path = committed_manifest_path();
    if std::env::var_os("LANTERN_UPDATE_HOST_SURFACE").is_some() {
        std::fs::write(&path, &written).expect("write the host surface");
        return;
    }
    let committed = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        committed == written,
        "{} is out of date with the engine API; rerun with LANTERN_UPDATE_HOST_SURFACE=1",
        path.display()
    );
}
