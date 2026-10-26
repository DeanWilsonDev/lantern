//! The game's named actions and default bindings (Umbra `SnakeApplication::Configure`).

use lantern_core::input::{
    ActionId, ActionKind, ActionSet, Binding, Bindings, GamepadAxis, GamepadButton, GamepadStick, KeyCode, MouseButton,
};

/// Mouse look: radians of camera turn per point of mouse motion.
const MOUSE_LOOK_SCALE: f32 = 0.0035;
/// Stick look: radians per second at full deflection (applied by the camera).
const STICK_LOOK_SCALE: f32 = 1.0;

#[derive(Clone, Copy, Debug)]
pub struct Actions {
    pub movement: ActionId,
    pub look_mouse: ActionId,
    pub look_stick: ActionId,
    pub jump: ActionId,
    pub reset: ActionId,
    pub toggle_debug_hud: ActionId,
    pub toggle_collision_view: ActionId,
    pub pause: ActionId,
    pub menu_up: ActionId,
    pub menu_down: ActionId,
    pub menu_confirm: ActionId,
    pub menu_back: ActionId,
    pub capture_mouse: ActionId,
}

pub fn build() -> (ActionSet, Bindings, Actions) {
    let mut set = ActionSet::default();
    let actions = Actions {
        movement: set.register("move", ActionKind::Axis2D),
        look_mouse: set.register("look_mouse", ActionKind::Axis2D),
        look_stick: set.register("look_stick", ActionKind::Axis2D),
        jump: set.register("jump", ActionKind::Button),
        reset: set.register("reset", ActionKind::Button),
        toggle_debug_hud: set.register("toggle_debug_hud", ActionKind::Button),
        toggle_collision_view: set.register("toggle_collision_view", ActionKind::Button),
        pause: set.register("pause", ActionKind::Button),
        menu_up: set.register("menu_up", ActionKind::Button),
        menu_down: set.register("menu_down", ActionKind::Button),
        menu_confirm: set.register("menu_confirm", ActionKind::Button),
        menu_back: set.register("menu_back", ActionKind::Button),
        capture_mouse: set.register("capture_mouse", ActionKind::Button),
    };

    let mut bindings = Bindings::default();
    bindings
        .bind(
            actions.movement,
            Binding::KeyVector { up: KeyCode::W, down: KeyCode::S, left: KeyCode::A, right: KeyCode::D },
        )
        .bind(
            actions.movement,
            Binding::KeyVector { up: KeyCode::Up, down: KeyCode::Down, left: KeyCode::Left, right: KeyCode::Right },
        )
        .bind(actions.movement, Binding::Stick { stick: GamepadStick::Left, scale: 1.0 })
        .bind(actions.look_mouse, Binding::MouseMotion { scale: MOUSE_LOOK_SCALE })
        .bind(actions.look_stick, Binding::Stick { stick: GamepadStick::Right, scale: STICK_LOOK_SCALE })
        .bind(actions.jump, Binding::Key(KeyCode::Space))
        .bind(actions.jump, Binding::Gamepad(GamepadButton::South))
        .bind(actions.reset, Binding::Key(KeyCode::R))
        .bind(actions.reset, Binding::Gamepad(GamepadButton::North))
        .bind(actions.toggle_debug_hud, Binding::Key(KeyCode::F1))
        .bind(actions.toggle_debug_hud, Binding::Key(KeyCode::Backquote))
        .bind(actions.toggle_debug_hud, Binding::Gamepad(GamepadButton::Back))
        .bind(actions.toggle_collision_view, Binding::Key(KeyCode::F2))
        .bind(actions.toggle_collision_view, Binding::Gamepad(GamepadButton::RightStick))
        .bind(actions.pause, Binding::Key(KeyCode::Escape))
        .bind(actions.pause, Binding::Gamepad(GamepadButton::Start))
        .bind(actions.menu_up, Binding::Key(KeyCode::Up))
        .bind(actions.menu_up, Binding::Key(KeyCode::W))
        .bind(actions.menu_up, Binding::Gamepad(GamepadButton::DPadUp))
        .bind(actions.menu_up, Binding::GamepadAxisButton { axis: GamepadAxis::LeftY, threshold: 0.6 })
        .bind(actions.menu_down, Binding::Key(KeyCode::Down))
        .bind(actions.menu_down, Binding::Key(KeyCode::S))
        .bind(actions.menu_down, Binding::Gamepad(GamepadButton::DPadDown))
        .bind(actions.menu_down, Binding::GamepadAxisButton { axis: GamepadAxis::LeftY, threshold: -0.6 })
        .bind(actions.menu_confirm, Binding::Key(KeyCode::Enter))
        .bind(actions.menu_confirm, Binding::Key(KeyCode::Space))
        .bind(actions.menu_confirm, Binding::Gamepad(GamepadButton::South))
        .bind(actions.menu_back, Binding::Key(KeyCode::Escape))
        .bind(actions.menu_back, Binding::Gamepad(GamepadButton::East))
        .bind(actions.capture_mouse, Binding::Mouse(MouseButton::Left));
    (set, bindings, actions)
}
