//! Input routing: platform events in, the per-frame action snapshot out.
//!
//! Platform events update device state as they arrive. Once per display frame `end_frame_events`
//! evaluates the bindings into action states. Two consumers read them:
//!
//! - the **frame snapshot**: edges since the previous display frame, used by menus and the camera;
//! - the **fixed-step snapshot**: edges *latched* until a fixed step takes them. A press in a
//!   display frame that runs no fixed step is delivered to the next step; when a frame runs several
//!   steps only the first sees the press.

use std::collections::{HashMap, HashSet};

use glam::Vec2;
use lantern_core::input::{
    ActionKind, ActionSet, ActionState, Binding, Bindings, GamepadAxis, GamepadButton, GamepadStick, InputSnapshot,
    KeyCode, MouseButton, apply_radial_dead_zone,
};
use lantern_platform::PlatformEvent;

/// Stick drift below this length reads as zero.
pub const STICK_DEAD_ZONE: f32 = 0.18;
/// A single gamepad axis below this magnitude reads as zero.
pub const AXIS_DEAD_ZONE: f32 = 0.15;

/// Raw device state. "Tapped" sets remember a press that was released again before the frame
/// was evaluated, so a very quick tap still counts as one held frame.
#[derive(Default, Debug)]
struct DeviceState {
    keys_down: HashSet<KeyCode>,
    keys_tapped: HashSet<KeyCode>,
    mouse_down: HashSet<MouseButton>,
    mouse_tapped: HashSet<MouseButton>,
    gamepad_down: HashSet<GamepadButton>,
    gamepad_tapped: HashSet<GamepadButton>,
    gamepad_axes: HashMap<GamepadAxis, f32>,
    mouse_delta: Vec2,
}

impl DeviceState {
    fn key(&self, key: KeyCode) -> bool {
        self.keys_down.contains(&key) || self.keys_tapped.contains(&key)
    }

    fn axis(&self, axis: GamepadAxis) -> f32 {
        self.gamepad_axes.get(&axis).copied().unwrap_or(0.0)
    }

    fn release_everything(&mut self) {
        self.keys_down.clear();
        self.mouse_down.clear();
        self.gamepad_down.clear();
        self.gamepad_axes.clear();
    }

    fn clear_frame_data(&mut self) {
        self.keys_tapped.clear();
        self.mouse_tapped.clear();
        self.gamepad_tapped.clear();
        self.mouse_delta = Vec2::ZERO;
    }
}

pub struct InputRouter {
    actions: ActionSet,
    bindings: Bindings,
    devices: DeviceState,
    held_last_frame: Vec<bool>,
    frame_snapshot: InputSnapshot,
    latched_pressed: Vec<bool>,
    latched_released: Vec<bool>,
}

impl InputRouter {
    pub fn new(actions: ActionSet, bindings: Bindings) -> InputRouter {
        let count = actions.len();
        InputRouter {
            actions,
            bindings,
            devices: DeviceState::default(),
            held_last_frame: vec![false; count],
            frame_snapshot: InputSnapshot { states: vec![ActionState::default(); count] },
            latched_pressed: vec![false; count],
            latched_released: vec![false; count],
        }
    }

    pub fn actions(&self) -> &ActionSet {
        &self.actions
    }

    /// Replaces the actions and bindings (a game script defined new ones, or was reloaded).
    /// Device state is kept, so a key held across the change stays held; edges start fresh.
    pub fn reconfigure(&mut self, actions: ActionSet, bindings: Bindings) {
        let devices = std::mem::take(&mut self.devices);
        *self = InputRouter::new(actions, bindings);
        self.devices = devices;
    }

    pub fn handle_event(&mut self, event: &PlatformEvent) {
        let devices = &mut self.devices;
        match *event {
            PlatformEvent::KeyDown { key, .. } => {
                devices.keys_down.insert(key);
                devices.keys_tapped.insert(key);
            }
            PlatformEvent::KeyUp { key } => {
                devices.keys_down.remove(&key);
            }
            PlatformEvent::MouseButtonDown { button, .. } => {
                devices.mouse_down.insert(button);
                devices.mouse_tapped.insert(button);
            }
            PlatformEvent::MouseButtonUp { button, .. } => {
                devices.mouse_down.remove(&button);
            }
            PlatformEvent::MouseMoved { delta_x, delta_y, .. } => {
                devices.mouse_delta += Vec2::new(delta_x, delta_y);
            }
            PlatformEvent::GamepadButtonDown { button } => {
                devices.gamepad_down.insert(button);
                devices.gamepad_tapped.insert(button);
            }
            PlatformEvent::GamepadButtonUp { button } => {
                devices.gamepad_down.remove(&button);
            }
            PlatformEvent::GamepadAxisMoved { axis, value } => {
                devices.gamepad_axes.insert(axis, value);
            }
            // Losing focus or the pad means we will never see the matching "up" events.
            PlatformEvent::FocusLost | PlatformEvent::GamepadDisconnected => devices.release_everything(),
            _ => {}
        }
    }

    /// Evaluates bindings once all of this frame's events are in. Call once per display frame.
    pub fn end_frame_events(&mut self) {
        for index in 0..self.actions.len() {
            let definition = &self.actions.all()[index];
            let (held, value) =
                evaluate_action(definition.kind, self.bindings.for_action(definition.id), &self.devices);
            let was_held = self.held_last_frame[index];
            let pressed = held && !was_held;
            let released = !held && was_held;
            self.frame_snapshot.states[index] = ActionState { held, pressed, released, value };
            self.latched_pressed[index] |= pressed;
            self.latched_released[index] |= released;
            self.held_last_frame[index] = held;
        }
        self.devices.clear_frame_data();
    }

    /// Edges since the previous display frame.
    pub fn frame_snapshot(&self) -> &InputSnapshot {
        &self.frame_snapshot
    }

    /// The snapshot for one fixed step. Takes (clears) the latched edges.
    pub fn take_fixed_step_snapshot(&mut self) -> InputSnapshot {
        let states = self
            .frame_snapshot
            .states
            .iter()
            .enumerate()
            .map(|(index, frame_state)| ActionState {
                held: frame_state.held,
                value: frame_state.value,
                pressed: std::mem::take(&mut self.latched_pressed[index]),
                released: std::mem::take(&mut self.latched_released[index]),
            })
            .collect();
        InputSnapshot { states }
    }
}

fn evaluate_action<'a>(
    kind: ActionKind,
    bindings: impl Iterator<Item = &'a Binding>,
    devices: &DeviceState,
) -> (bool, Vec2) {
    let mut held = false;
    let mut value = Vec2::ZERO;
    let mut unbounded = false;
    for binding in bindings {
        let (binding_held, binding_value) = evaluate_binding(binding, devices);
        held |= binding_held;
        if matches!(binding, Binding::MouseMotion { .. }) {
            unbounded = true;
        }
        value = match kind {
            ActionKind::Button => value.max(binding_value),
            ActionKind::Axis1D | ActionKind::Axis2D => value + binding_value,
        };
    }
    match kind {
        ActionKind::Axis1D => value.x = value.x.clamp(-1.0, 1.0),
        ActionKind::Axis2D if !unbounded => value = value.clamp_length_max(1.0),
        _ => {}
    }
    if kind != ActionKind::Button {
        held = value != Vec2::ZERO;
    }
    (held, value)
}

fn button_value(held: bool) -> (bool, Vec2) {
    (held, if held { Vec2::X } else { Vec2::ZERO })
}

fn key_value(devices: &DeviceState, key: KeyCode) -> f32 {
    if devices.key(key) { 1.0 } else { 0.0 }
}

fn evaluate_binding(binding: &Binding, devices: &DeviceState) -> (bool, Vec2) {
    match *binding {
        Binding::Key(key) => button_value(devices.key(key)),
        Binding::Mouse(button) => {
            button_value(devices.mouse_down.contains(&button) || devices.mouse_tapped.contains(&button))
        }
        Binding::Gamepad(button) => {
            button_value(devices.gamepad_down.contains(&button) || devices.gamepad_tapped.contains(&button))
        }
        Binding::GamepadAxisButton { axis, threshold } => {
            let value = devices.axis(axis);
            button_value(if threshold >= 0.0 { value >= threshold } else { value <= threshold })
        }
        Binding::KeyAxis { negative, positive } => {
            let value = key_value(devices, positive) - key_value(devices, negative);
            (value != 0.0, Vec2::new(value, 0.0))
        }
        Binding::GamepadAxis { axis, scale } => {
            let raw = devices.axis(axis);
            let value = if raw.abs() < AXIS_DEAD_ZONE { 0.0 } else { raw * scale };
            (value != 0.0, Vec2::new(value, 0.0))
        }
        Binding::KeyVector { up, down, left, right } => {
            let value = Vec2::new(
                key_value(devices, right) - key_value(devices, left),
                key_value(devices, up) - key_value(devices, down),
            )
            .normalize_or_zero();
            (value != Vec2::ZERO, value)
        }
        Binding::Stick { stick, scale } => {
            let (x_axis, y_axis) = match stick {
                GamepadStick::Left => (GamepadAxis::LeftX, GamepadAxis::LeftY),
                GamepadStick::Right => (GamepadAxis::RightX, GamepadAxis::RightY),
            };
            let raw = Vec2::new(devices.axis(x_axis), devices.axis(y_axis));
            let value = apply_radial_dead_zone(raw, STICK_DEAD_ZONE) * scale;
            (value != Vec2::ZERO, value)
        }
        Binding::MouseMotion { scale } => {
            // Screen +Y is down; action +Y is up.
            let value = Vec2::new(devices.mouse_delta.x, -devices.mouse_delta.y) * scale;
            (value != Vec2::ZERO, value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lantern_core::input::ActionId;

    struct Fixture {
        router: InputRouter,
        jump: ActionId,
        movement: ActionId,
        look: ActionId,
    }

    fn fixture() -> Fixture {
        let mut actions = ActionSet::default();
        let jump = actions.register("jump", ActionKind::Button);
        let movement = actions.register("move", ActionKind::Axis2D);
        let look = actions.register("look", ActionKind::Axis2D);
        let mut bindings = Bindings::default();
        bindings
            .bind(jump, Binding::Key(KeyCode::Space))
            .bind(jump, Binding::Gamepad(GamepadButton::South))
            .bind(
                movement,
                Binding::KeyVector { up: KeyCode::W, down: KeyCode::S, left: KeyCode::A, right: KeyCode::D },
            )
            .bind(movement, Binding::Stick { stick: GamepadStick::Left, scale: 1.0 })
            .bind(look, Binding::MouseMotion { scale: 0.5 });
        Fixture { router: InputRouter::new(actions, bindings), jump, movement, look }
    }

    fn frame(router: &mut InputRouter, events: &[PlatformEvent]) {
        for event in events {
            router.handle_event(event);
        }
        router.end_frame_events();
    }

    const SPACE_DOWN: PlatformEvent = PlatformEvent::KeyDown { key: KeyCode::Space, repeat: false };
    const SPACE_UP: PlatformEvent = PlatformEvent::KeyUp { key: KeyCode::Space };

    #[test]
    fn press_hold_release_edges_per_frame() {
        let Fixture { mut router, jump, .. } = fixture();
        frame(&mut router, &[SPACE_DOWN]);
        assert_eq!(
            router.frame_snapshot().state(jump),
            ActionState { held: true, pressed: true, released: false, value: Vec2::X }
        );
        frame(&mut router, &[]);
        let held = router.frame_snapshot().state(jump);
        assert!(held.held && !held.pressed && !held.released);
        frame(&mut router, &[SPACE_UP]);
        let released = router.frame_snapshot().state(jump);
        assert!(!released.held && !released.pressed && released.released);
    }

    #[test]
    fn a_tap_inside_one_frame_still_counts_as_one_press() {
        let Fixture { mut router, jump, .. } = fixture();
        frame(&mut router, &[SPACE_DOWN, SPACE_UP]);
        assert!(router.frame_snapshot().pressed(jump));
        frame(&mut router, &[]);
        assert!(router.frame_snapshot().released(jump));
    }

    #[test]
    fn a_press_waits_for_the_next_fixed_step() {
        let Fixture { mut router, jump, .. } = fixture();
        frame(&mut router, &[SPACE_DOWN]); // this display frame runs no fixed step
        frame(&mut router, &[]); // the next one does
        let step = router.take_fixed_step_snapshot();
        assert!(step.pressed(jump) && step.held(jump));
    }

    #[test]
    fn only_the_first_fixed_step_of_a_frame_sees_the_press() {
        let Fixture { mut router, jump, .. } = fixture();
        frame(&mut router, &[SPACE_DOWN]);
        let first = router.take_fixed_step_snapshot();
        let second = router.take_fixed_step_snapshot();
        assert!(first.pressed(jump));
        assert!(!second.pressed(jump) && second.held(jump));
    }

    #[test]
    fn a_press_and_release_between_steps_are_both_delivered() {
        let Fixture { mut router, jump, .. } = fixture();
        frame(&mut router, &[SPACE_DOWN]);
        frame(&mut router, &[SPACE_UP]);
        let step = router.take_fixed_step_snapshot();
        assert!(step.pressed(jump) && step.released(jump) && !step.held(jump));
    }

    #[test]
    fn keyboard_and_stick_combine_and_diagonals_are_normalised() {
        let Fixture { mut router, movement, .. } = fixture();
        frame(
            &mut router,
            &[
                PlatformEvent::KeyDown { key: KeyCode::W, repeat: false },
                PlatformEvent::KeyDown { key: KeyCode::D, repeat: false },
            ],
        );
        let value = router.frame_snapshot().vector(movement);
        assert!((value.length() - 1.0).abs() < 1e-6);
        frame(&mut router, &[PlatformEvent::GamepadAxisMoved { axis: GamepadAxis::LeftX, value: 1.0 }]);
        assert!(router.frame_snapshot().vector(movement).length() <= 1.0 + 1e-6, "combined input is clamped");
    }

    #[test]
    fn stick_drift_is_ignored() {
        let Fixture { mut router, movement, .. } = fixture();
        frame(&mut router, &[PlatformEvent::GamepadAxisMoved { axis: GamepadAxis::LeftY, value: 0.1 }]);
        assert!(!router.frame_snapshot().held(movement));
    }

    #[test]
    fn mouse_motion_is_per_frame_and_y_up() {
        let Fixture { mut router, look, .. } = fixture();
        frame(
            &mut router,
            &[
                PlatformEvent::MouseMoved { delta_x: 4.0, delta_y: 2.0, x: 104.0, y: 102.0 },
                PlatformEvent::MouseMoved { delta_x: 6.0, delta_y: 0.0, x: 110.0, y: 102.0 },
            ],
        );
        assert_eq!(router.frame_snapshot().vector(look), Vec2::new(5.0, -1.0));
        frame(&mut router, &[]);
        assert_eq!(router.frame_snapshot().vector(look), Vec2::ZERO);
    }

    #[test]
    fn losing_focus_releases_held_keys() {
        let Fixture { mut router, jump, .. } = fixture();
        frame(&mut router, &[SPACE_DOWN]);
        frame(&mut router, &[PlatformEvent::FocusLost]);
        assert!(router.frame_snapshot().released(jump));
    }
}
