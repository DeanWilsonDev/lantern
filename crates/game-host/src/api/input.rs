//! `Input`: the game defines its named actions and their bindings, then asks about actions.
//! Scripts never see keys: `Input:::IsPressed("jump")` reads the snapshot the engine built for
//! the current fixed step or display frame (edges are latched for fixed steps, see
//! `docs/architecture.md`).
//!
//! Device names are the engine's enum names: keys `A`..`Z`, `Digit0`..`Digit9`, `Up`, `Space`,
//! `Escape`, `F1`..; mouse buttons `Left`, `Right`, `Middle`; gamepad buttons by position
//! (`South`, `East`, `West`, `North`, `Back`, `Start`, `LeftStick`, `RightStick`,
//! `LeftShoulder`, `RightShoulder`, `DPadUp`, ...); axes `LeftX`, `LeftY`, `RightX`, `RightY`,
//! `LeftTrigger`, `RightTrigger`; sticks `Left`, `Right`.

use lantern_core::input::{
    ActionId, ActionKind, Binding, GamepadAxis, GamepadButton, GamepadStick, KeyCode, MouseButton,
};
use nyx::{CallContext, CallbackError, Runtime, Value};

use super::{
    SharedState, SharedTypes, f32_argument, invalid, not_found, some_bool, some_f32, string_argument, wrap_vec2,
};

struct InputApi;

const KEYS: [KeyCode; 64] = {
    use KeyCode::*;
    [
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
    ]
};

const GAMEPAD_BUTTONS: [GamepadButton; 14] = {
    use GamepadButton::*;
    [
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
    ]
};

const GAMEPAD_AXES: [GamepadAxis; 6] = {
    use GamepadAxis::*;
    [LeftX, LeftY, RightX, RightY, LeftTrigger, RightTrigger]
};

/// Finds the variant whose `Debug` name is `name`.
fn by_name<T: Copy + std::fmt::Debug>(candidates: &[T], name: &str) -> Option<T> {
    candidates.iter().copied().find(|candidate| format!("{candidate:?}") == name)
}

fn device_name<T: Copy + std::fmt::Debug>(
    context: &CallContext,
    candidates: &[T],
    what: &str,
    arguments: &[Value],
    index: usize,
) -> Result<T, CallbackError> {
    let name = string_argument(arguments, index)?;
    by_name(candidates, &name).ok_or_else(|| invalid(context, format!("'{name}' is not a {what} name")))
}

fn key(context: &CallContext, arguments: &[Value], index: usize) -> Result<KeyCode, CallbackError> {
    device_name(context, &KEYS, "key", arguments, index)
}

fn mouse_button(context: &CallContext, arguments: &[Value], index: usize) -> Result<MouseButton, CallbackError> {
    device_name(
        context,
        &[MouseButton::Left, MouseButton::Right, MouseButton::Middle],
        "mouse button",
        arguments,
        index,
    )
}

fn gamepad_button(context: &CallContext, arguments: &[Value], index: usize) -> Result<GamepadButton, CallbackError> {
    device_name(context, &GAMEPAD_BUTTONS, "gamepad button", arguments, index)
}

fn gamepad_axis(context: &CallContext, arguments: &[Value], index: usize) -> Result<GamepadAxis, CallbackError> {
    device_name(context, &GAMEPAD_AXES, "gamepad axis", arguments, index)
}

fn gamepad_stick(context: &CallContext, arguments: &[Value], index: usize) -> Result<GamepadStick, CallbackError> {
    device_name(context, &[GamepadStick::Left, GamepadStick::Right], "gamepad stick", arguments, index)
}

/// Registers `Input:::Define<kind>(name)`.
fn define(
    state: &SharedState,
    kind: ActionKind,
) -> impl Fn(&CallContext, &nyx::HostType<InputApi>, &[Value]) -> nyx::CallbackResult + 'static {
    let state = state.clone();
    move |context, _, arguments| {
        let name = string_argument(arguments, 0)?;
        let mut state = state.borrow_mut();
        if state.input.actions.find(&name).is_some() {
            return Err(invalid(context, format!("the action '{name}' is defined twice")));
        }
        state.input.actions.register(&name, kind);
        state.input.configuration_changed = true;
        Ok(None)
    }
}

/// Registers `Input:::Bind<device>(action, ...)`: `make_binding` reads the device arguments.
fn bind(
    state: &SharedState,
    make_binding: impl Fn(&CallContext, &[Value]) -> Result<Binding, CallbackError> + 'static,
) -> impl Fn(&CallContext, &nyx::HostType<InputApi>, &[Value]) -> nyx::CallbackResult + 'static {
    let state = state.clone();
    move |context, _, arguments| {
        let name = string_argument(arguments, 0)?;
        let binding = make_binding(context, arguments)?;
        let mut state = state.borrow_mut();
        let Some(action) = state.input.actions.find(&name) else {
            return Err(not_found(context, format!("no action named '{name}'; define it first")));
        };
        if action == crate::engine_state::ENGINE_RELOAD_ACTION
            || action == crate::engine_state::ENGINE_TOGGLE_HUD_ACTION
        {
            return Err(invalid(context, format!("'{name}' is an engine action")));
        }
        state.input.bindings.bind(action, binding);
        state.input.configuration_changed = true;
        Ok(None)
    }
}

fn find_action(context: &CallContext, state: &SharedState, arguments: &[Value]) -> Result<ActionId, CallbackError> {
    let name = string_argument(arguments, 0)?;
    state.borrow().input.actions.find(&name).ok_or_else(|| not_found(context, format!("no action named '{name}'")))
}

pub fn register(runtime: &Runtime, state: &SharedState, types: &SharedTypes) -> nyx::Result<()> {
    let pressed_state = state.clone();
    let released_state = state.clone();
    let held_state = state.clone();
    let axis_state = state.clone();
    let vector_state = state.clone();
    let vector_types = types.clone();
    runtime
        .register_type::<InputApi>("Input")
        .static_method("DefineButton", "(name: string) -> void", define(state, ActionKind::Button))
        .static_method("DefineAxis", "(name: string) -> void", define(state, ActionKind::Axis1D))
        .static_method("DefineVector", "(name: string) -> void", define(state, ActionKind::Axis2D))
        .static_method(
            "BindKey",
            "(action: string, key: string) -> void",
            bind(state, |context, arguments| Ok(Binding::Key(key(context, arguments, 1)?))),
        )
        .static_method(
            "BindMouseButton",
            "(action: string, button: string) -> void",
            bind(state, |context, arguments| Ok(Binding::Mouse(mouse_button(context, arguments, 1)?))),
        )
        .static_method(
            "BindGamepadButton",
            "(action: string, button: string) -> void",
            bind(state, |context, arguments| Ok(Binding::Gamepad(gamepad_button(context, arguments, 1)?))),
        )
        .static_method(
            "BindGamepadAxisButton",
            "(action: string, axis: string, threshold: f32) -> void",
            bind(state, |context, arguments| {
                Ok(Binding::GamepadAxisButton {
                    axis: gamepad_axis(context, arguments, 1)?,
                    threshold: f32_argument(arguments, 2)?,
                })
            }),
        )
        .static_method(
            "BindKeyAxis",
            "(action: string, negative: string, positive: string) -> void",
            bind(state, |context, arguments| {
                Ok(Binding::KeyAxis { negative: key(context, arguments, 1)?, positive: key(context, arguments, 2)? })
            }),
        )
        .static_method(
            "BindGamepadAxis",
            "(action: string, axis: string, scale: f32) -> void",
            bind(state, |context, arguments| {
                Ok(Binding::GamepadAxis {
                    axis: gamepad_axis(context, arguments, 1)?,
                    scale: f32_argument(arguments, 2)?,
                })
            }),
        )
        .static_method(
            "BindKeys",
            "(action: string, up: string, down: string, left: string, right: string) -> void",
            bind(state, |context, arguments| {
                Ok(Binding::KeyVector {
                    up: key(context, arguments, 1)?,
                    down: key(context, arguments, 2)?,
                    left: key(context, arguments, 3)?,
                    right: key(context, arguments, 4)?,
                })
            }),
        )
        .static_method(
            "BindStick",
            "(action: string, stick: string, scale: f32) -> void",
            bind(state, |context, arguments| {
                Ok(Binding::Stick { stick: gamepad_stick(context, arguments, 1)?, scale: f32_argument(arguments, 2)? })
            }),
        )
        .static_method(
            "BindMouseMotion",
            "(action: string, scale: f32) -> void",
            bind(state, |_context, arguments| Ok(Binding::MouseMotion { scale: f32_argument(arguments, 1)? })),
        )
        .static_method("IsPressed", "(action: string) -> bool", move |context, _, arguments| {
            let action = find_action(context, &pressed_state, arguments)?;
            some_bool(context, pressed_state.borrow().input.snapshot.pressed(action))
        })
        .static_method("IsReleased", "(action: string) -> bool", move |context, _, arguments| {
            let action = find_action(context, &released_state, arguments)?;
            some_bool(context, released_state.borrow().input.snapshot.released(action))
        })
        .static_method("IsHeld", "(action: string) -> bool", move |context, _, arguments| {
            let action = find_action(context, &held_state, arguments)?;
            some_bool(context, held_state.borrow().input.snapshot.held(action))
        })
        .static_method("Axis", "(action: string) -> f32", move |context, _, arguments| {
            let action = find_action(context, &axis_state, arguments)?;
            some_f32(context, axis_state.borrow().input.snapshot.axis(action))
        })
        .static_method("Vector", "(action: string) -> Vec2", move |context, _, arguments| {
            let action = find_action(context, &vector_state, arguments)?;
            let value = vector_state.borrow().input.snapshot.vector(action);
            wrap_vec2(&vector_types, value)
        })
        .register()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_names_are_the_enum_names() {
        assert_eq!(by_name(&KEYS, "Space"), Some(KeyCode::Space));
        assert_eq!(by_name(&KEYS, "F5"), Some(KeyCode::F5));
        assert_eq!(by_name(&GAMEPAD_BUTTONS, "DPadUp"), Some(GamepadButton::DPadUp));
        assert_eq!(by_name(&KEYS, "space"), None);
    }
}
