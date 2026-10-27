//! Play in editor: launches the game on the level being edited and watches it until it quits.
//!
//! The level is saved to a scratch file beside the game's worlds (`.editor-play.world.json`),
//! so unsaved edits are what plays, and the game is told about it through `LANTERN_LEVEL`
//! (read by the platformer's `LevelPath`). The runner is, in order: the executable named by
//! `LANTERN_RUNNER`, a `lantern` beside the editor's own executable, or `cargo run -p lantern`
//! from the workspace the assets belong to.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Instant;

use lantern_editor_core::LogLevel;

use crate::EditorBackend;

const SCRATCH_LEVEL_FILE: &str = ".editor-play.world.json";
const GAME_DIRECTORY: &str = "games/platformer";

pub struct PlaySession {
    child: Child,
    level_path: PathBuf,
    started: Instant,
}

/// What `start_play` decided to run, before it runs: visible to tests.
#[derive(Debug, PartialEq, Eq)]
pub enum RunnerChoice {
    Executable(PathBuf),
    Cargo { workspace: PathBuf },
}

/// Finds the game runner for the assets directory's workspace.
pub fn choose_runner(assets_directory: &Path) -> Option<RunnerChoice> {
    if let Some(explicit) = std::env::var_os("LANTERN_RUNNER") {
        return Some(RunnerChoice::Executable(PathBuf::from(explicit)));
    }
    if let Ok(current) = std::env::current_exe() {
        let sibling = current.with_file_name(if cfg!(windows) { "lantern.exe" } else { "lantern" });
        if sibling.is_file() {
            return Some(RunnerChoice::Executable(sibling));
        }
    }
    let workspace = workspace_root(assets_directory)?;
    Some(RunnerChoice::Cargo { workspace })
}

/// The workspace holding `assets/`: the directory with the `Cargo.toml` that names it.
pub fn workspace_root(assets_directory: &Path) -> Option<PathBuf> {
    let parent = assets_directory.parent()?;
    parent.join("Cargo.toml").is_file().then(|| parent.to_path_buf())
}

impl EditorBackend {
    pub fn is_playing(&mut self) -> bool {
        self.poll_play();
        self.play.is_some()
    }

    /// Seconds since the game was launched, 0 when it isn't running.
    pub fn play_seconds(&self) -> f32 {
        self.play.as_ref().map_or(0.0, |play| play.started.elapsed().as_secs_f32())
    }

    /// Saves the level to its scratch file and launches the game on it.
    pub fn start_play(&mut self) -> bool {
        if self.is_playing() {
            self.session.note(LogLevel::Warning, "The game is already running");
            return false;
        }
        let Some(runner) = choose_runner(&self.assets_directory) else {
            self.session.note(
                LogLevel::Error,
                "No game runner found: set LANTERN_RUNNER to the lantern executable, or run from the workspace",
            );
            return false;
        };
        let game_directory = workspace_root(&self.assets_directory)
            .map_or_else(|| self.assets_directory.join("..").join(GAME_DIRECTORY), |root| root.join(GAME_DIRECTORY));
        let level_path = self.assets_directory.join("worlds").join(SCRATCH_LEVEL_FILE);
        if let Err(error) = self.session.world().save(&level_path) {
            self.session.note(LogLevel::Error, format!("Couldn't write the play level: {error}"));
            return false;
        }
        let mut command = match &runner {
            RunnerChoice::Executable(path) => Command::new(path),
            RunnerChoice::Cargo { workspace } => {
                let mut command = Command::new("cargo");
                command.args(["run", "-p", "lantern", "--"]).current_dir(workspace);
                command
            }
        };
        command
            .arg(&game_directory)
            .env("LANTERN_LEVEL", &level_path)
            .env("LANTERN_ASSETS", &self.assets_directory)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        match command.spawn() {
            Ok(child) => {
                let how = match &runner {
                    RunnerChoice::Executable(path) => path.display().to_string(),
                    RunnerChoice::Cargo { .. } => "cargo run -p lantern".to_string(),
                };
                self.session.note(LogLevel::Info, format!("Playing {} with {how}", level_path.display()));
                self.play = Some(PlaySession { child, level_path, started: Instant::now() });
                true
            }
            Err(error) => {
                self.session.note(LogLevel::Error, format!("Couldn't start the game: {error}"));
                let _ = std::fs::remove_file(&level_path);
                false
            }
        }
    }

    /// Ends the game if it is running.
    pub fn stop_play(&mut self) -> bool {
        let Some(mut play) = self.play.take() else { return false };
        let _ = play.child.kill();
        let _ = play.child.wait();
        let _ = std::fs::remove_file(&play.level_path);
        self.session.note(LogLevel::Info, "Stopped the game");
        true
    }

    /// Notices a game that quit on its own. Call once per frame.
    pub fn poll_play(&mut self) {
        let Some(play) = &mut self.play else { return };
        match play.child.try_wait() {
            Ok(Some(status)) => {
                let seconds = play.started.elapsed().as_secs_f32();
                let _ = std::fs::remove_file(&play.level_path);
                self.play = None;
                let text = if status.success() {
                    format!("The game quit after {seconds:.0} s")
                } else {
                    format!("The game exited with {status} after {seconds:.0} s")
                };
                self.session.note(if status.success() { LogLevel::Info } else { LogLevel::Warning }, text);
            }
            Ok(None) => {}
            Err(error) => {
                self.play = None;
                self.session.note(LogLevel::Error, format!("Lost track of the game: {error}"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_workspace_is_found_from_its_assets() {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let root = workspace_root(&assets).unwrap();
        assert!(root.join("Cargo.toml").is_file());
        assert!(workspace_root(Path::new("/nowhere/assets")).is_none());
    }
}
