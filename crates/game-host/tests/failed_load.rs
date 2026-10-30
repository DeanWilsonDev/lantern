//! A game that fails to load releases the Chaos UI it started, so the next load (F5) works.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use lantern_engine::audio::AudioSystem;
use lantern_game_host::api::SharedState;
use lantern_game_host::{EngineState, HeadlessRenderer, ScriptSession};

fn platformer_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../games/platformer")
}

fn headless_state() -> SharedState {
    let state = EngineState::new(Box::new(HeadlessRenderer::default()), AudioSystem::new(None), None)
        .expect("a headless engine state starts");
    Rc::new(RefCell::new(state))
}

#[test]
fn a_failed_load_leaves_the_next_load_working() {
    let state = headless_state();
    let missing_class = ScriptSession::load(&platformer_directory(), "main.nyx", "NoSuchApplication", &state);
    assert!(missing_class.is_err(), "loading a class the game doesn't define fails");
    let missing_file = ScriptSession::load(&platformer_directory(), "no-such-file.nyx", "Platformer", &state);
    assert!(missing_file.is_err(), "loading a file that doesn't exist fails");
    let session = ScriptSession::load(&platformer_directory(), "main.nyx", "Platformer", &state);
    assert!(session.is_ok(), "the game loads after failed loads: {:?}", session.err());
}
