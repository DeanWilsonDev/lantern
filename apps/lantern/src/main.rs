//! `lantern [game-directory]`: runs a Nyx game. Without an argument it runs the platformer in
//! `games/platformer`. All game code is in the game directory; this binary only starts the
//! engine (`lantern_game_host::run`).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// The platformer, found from the working directory or from this crate's checkout.
fn default_game_directory() -> PathBuf {
    let relative = Path::new("games/platformer");
    if relative.join("game.json").is_file() {
        return relative.to_path_buf();
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/platformer")
}

fn main() -> ExitCode {
    lantern_core::logging::init(log::LevelFilter::Info);
    let game_directory = std::env::args_os().nth(1).map(PathBuf::from).unwrap_or_else(default_game_directory);
    match lantern_game_host::run(&game_directory) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            log::error!(target: "game", "{error}");
            ExitCode::FAILURE
        }
    }
}
