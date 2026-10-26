//! The main menu and pause menu (Umbra `MainMenuUI` / `GameplayUI`), drawn with the
//! immediate-mode `Ui`. Each function draws one frame of its menu and returns what was chosen.

use glam::Vec2;
use lantern_renderer_3d::render_world::Overlay;
use lantern_renderer_3d::ui::{Ui, UiInput};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MainMenuChoice {
    Play,
    Quit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PauseMenuChoice {
    Resume,
    Restart,
    ToggleDebugHud,
    MainMenu,
    Quit,
}

const CONTROLS: [&str; 6] = [
    "Move: WASD / arrows / left stick",
    "Look: mouse / right stick      Jump: Space / A",
    "Reset: R / Y      Pause: Esc / Start",
    "Debug HUD: F1 / Back      Collision view: F2 / R3",
    "Reload scripts: F5",
    "Click to capture the mouse",
];

pub fn main_menu(
    ui: &mut Ui,
    overlay: &mut Overlay,
    input: UiInput,
    screen: Vec2,
    scale: f32,
) -> Option<MainMenuChoice> {
    overlay.rect(Vec2::ZERO, screen, lantern_core::color::Color::rgba(0.0, 0.02, 0.06, 0.55));
    let mut column = ui.begin(overlay, input, screen, screen.y * 0.22, scale);
    column.title("LANTERN");
    column.label("a small 3D platformer");
    column.spacing(2.0);
    let mut choice = None;
    if column.button("Play") {
        choice = Some(MainMenuChoice::Play);
    }
    column.spacing(1.0);
    if column.button("Quit") {
        choice = Some(MainMenuChoice::Quit);
    }
    column.spacing(3.0);
    for line in CONTROLS {
        column.label(line);
    }
    choice
}

pub fn pause_menu(
    ui: &mut Ui,
    overlay: &mut Overlay,
    input: UiInput,
    screen: Vec2,
    scale: f32,
    debug_hud_visible: bool,
) -> Option<PauseMenuChoice> {
    overlay.rect(Vec2::ZERO, screen, lantern_core::color::Color::rgba(0.0, 0.0, 0.05, 0.45));
    let mut column = ui.begin(overlay, input, screen, screen.y * 0.25, scale);
    column.title("PAUSED");
    column.spacing(1.0);
    let entries = [
        ("Resume", PauseMenuChoice::Resume),
        ("Restart level", PauseMenuChoice::Restart),
        (if debug_hud_visible { "Debug HUD: on" } else { "Debug HUD: off" }, PauseMenuChoice::ToggleDebugHud),
        ("Main menu", PauseMenuChoice::MainMenu),
        ("Quit", PauseMenuChoice::Quit),
    ];
    let mut choice = None;
    for (label, value) in entries {
        if column.button(label) {
            choice = Some(value);
        }
        column.spacing(0.5);
    }
    choice
}
