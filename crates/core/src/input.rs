//! Input vocabulary: devices, named actions, bindings and the per-frame snapshot.
//!
//! Gameplay never reads keys. It asks about *actions* ("jump", "move") through an
//! `InputSnapshot`, which `lantern_engine::input::InputRouter` builds from device state and the
//! `Bindings` once per display frame.

use glam::Vec2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum KeyCode {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    Up,
    Down,
    Left,
    Right,
    Space,
    Enter,
    Escape,
    Tab,
    Backspace,
    Backquote,
    LeftShift,
    RightShift,
    LeftControl,
    RightControl,
    LeftAlt,
    RightAlt,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Gamepad buttons by position, so the same binding works on Xbox, PlayStation and Switch pads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GamepadButton {
    South,
    East,
    West,
    North,
    Back,
    Start,
    LeftStick,
    RightStick,
    LeftShoulder,
    RightShoulder,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}

/// Sticks are in -1..1 with +X right and +Y **up** (pushed away from the player).
/// Triggers are in 0..1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GamepadAxis {
    LeftX,
    LeftY,
    RightX,
    RightY,
    LeftTrigger,
    RightTrigger,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GamepadStick {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionKind {
    /// On/off. `value.x` is 0 or 1.
    Button,
    /// One axis in -1..1, in `value.x`.
    Axis1D,
    /// Two axes. Stick-like bindings are clamped to the unit circle; mouse motion is not.
    Axis2D,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ActionId(pub u16);

#[derive(Clone, Debug)]
pub struct ActionDefinition {
    pub id: ActionId,
    pub name: String,
    pub kind: ActionKind,
}

/// The game's named actions.
#[derive(Clone, Debug, Default)]
pub struct ActionSet {
    actions: Vec<ActionDefinition>,
}

impl ActionSet {
    pub fn register(&mut self, name: &str, kind: ActionKind) -> ActionId {
        assert!(self.find(name).is_none(), "action {name:?} is registered twice");
        let id = ActionId(u16::try_from(self.actions.len()).expect("fewer than 65536 actions"));
        self.actions.push(ActionDefinition { id, name: name.to_string(), kind });
        id
    }

    pub fn find(&self, name: &str) -> Option<ActionId> {
        self.actions.iter().find(|action| action.name == name).map(|action| action.id)
    }

    pub fn definition(&self, id: ActionId) -> &ActionDefinition {
        &self.actions[usize::from(id.0)]
    }

    pub fn all(&self) -> &[ActionDefinition] {
        &self.actions
    }

    pub fn len(&self) -> usize {
        self.actions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

/// One way of driving an action from a device.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Binding {
    Key(KeyCode),
    Mouse(MouseButton),
    Gamepad(GamepadButton),
    /// A trigger or stick direction used as a button once it passes `threshold`.
    GamepadAxisButton {
        axis: GamepadAxis,
        threshold: f32,
    },
    /// Two keys as a 1D axis.
    KeyAxis {
        negative: KeyCode,
        positive: KeyCode,
    },
    /// A gamepad axis as a 1D axis.
    GamepadAxis {
        axis: GamepadAxis,
        scale: f32,
    },
    /// Four keys as a 2D vector (+Y is `up`).
    KeyVector {
        up: KeyCode,
        down: KeyCode,
        left: KeyCode,
        right: KeyCode,
    },
    /// A gamepad stick as a 2D vector.
    Stick {
        stick: GamepadStick,
        scale: f32,
    },
    /// Relative mouse motion this frame in pixels, times `scale` (+Y is up).
    MouseMotion {
        scale: f32,
    },
}

/// Action → bindings.
#[derive(Clone, Debug, Default)]
pub struct Bindings {
    entries: Vec<(ActionId, Binding)>,
}

impl Bindings {
    pub fn bind(&mut self, action: ActionId, binding: Binding) -> &mut Bindings {
        self.entries.push((action, binding));
        self
    }

    pub fn for_action(&self, action: ActionId) -> impl Iterator<Item = &Binding> {
        self.entries.iter().filter(move |(id, _)| *id == action).map(|(_, binding)| binding)
    }
}

/// One action's state for one consumer (a display frame or a fixed step).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ActionState {
    pub held: bool,
    /// Became held since the consumer last looked.
    pub pressed: bool,
    /// Stopped being held since the consumer last looked.
    pub released: bool,
    pub value: Vec2,
}

/// An immutable view of every action, handed to whoever consumes input.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InputSnapshot {
    pub states: Vec<ActionState>,
}

impl InputSnapshot {
    pub fn state(&self, action: ActionId) -> ActionState {
        self.states.get(usize::from(action.0)).copied().unwrap_or_default()
    }

    pub fn held(&self, action: ActionId) -> bool {
        self.state(action).held
    }

    pub fn pressed(&self, action: ActionId) -> bool {
        self.state(action).pressed
    }

    pub fn released(&self, action: ActionId) -> bool {
        self.state(action).released
    }

    pub fn axis(&self, action: ActionId) -> f32 {
        self.state(action).value.x
    }

    pub fn vector(&self, action: ActionId) -> Vec2 {
        self.state(action).value
    }
}

/// Applies a radial dead zone and rescales the rest to 0..1, so small stick drift reads as zero
/// and full deflection still reads as one.
pub fn apply_radial_dead_zone(stick: Vec2, dead_zone: f32) -> Vec2 {
    let length = stick.length();
    if length <= dead_zone {
        return Vec2::ZERO;
    }
    let scaled = ((length - dead_zone) / (1.0 - dead_zone)).min(1.0);
    stick / length * scaled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actions_are_found_by_name() {
        let mut actions = ActionSet::default();
        let jump = actions.register("jump", ActionKind::Button);
        let movement = actions.register("move", ActionKind::Axis2D);
        assert_eq!(actions.find("jump"), Some(jump));
        assert_eq!(actions.find("move"), Some(movement));
        assert_eq!(actions.find("fly"), None);
        assert_eq!(actions.definition(movement).kind, ActionKind::Axis2D);
    }

    #[test]
    #[should_panic(expected = "registered twice")]
    fn duplicate_actions_are_a_bug() {
        let mut actions = ActionSet::default();
        actions.register("jump", ActionKind::Button);
        actions.register("jump", ActionKind::Button);
    }

    #[test]
    fn dead_zone_zeroes_drift_and_keeps_full_deflection() {
        assert_eq!(apply_radial_dead_zone(Vec2::new(0.1, 0.05), 0.2), Vec2::ZERO);
        let full = apply_radial_dead_zone(Vec2::new(1.0, 0.0), 0.2);
        assert!((full.length() - 1.0).abs() < 1e-6);
        let half = apply_radial_dead_zone(Vec2::new(0.0, 0.6), 0.2);
        assert!((half.y - 0.5).abs() < 1e-6);
    }

    #[test]
    fn unknown_actions_read_as_idle() {
        let snapshot = InputSnapshot::default();
        assert!(!snapshot.held(ActionId(3)));
        assert_eq!(snapshot.vector(ActionId(3)), Vec2::ZERO);
    }
}
