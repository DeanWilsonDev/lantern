//! SDL event → `PlatformEvent` translation.

use lantern_core::input::{GamepadAxis, GamepadButton, KeyCode, MouseButton};
use lantern_platform::PlatformEvent;
use sdl3_sys::everything as sdl;

pub(crate) enum Translated {
    Event(PlatformEvent),
    GamepadAdded(sdl::SDL_JoystickID),
    GamepadRemoved(sdl::SDL_JoystickID),
    Ignored,
}

pub(crate) fn translate(event: &sdl::SDL_Event) -> Translated {
    // SAFETY: every SDL_Event variant starts with the `type` field, so reading it is always valid.
    let event_type = sdl::SDL_EventType(unsafe { event.r#type });
    // SAFETY (for each union read below): SDL guarantees the union member that matches
    // `event_type` is the one it filled in.
    let translated = match event_type {
        sdl::SDL_EVENT_QUIT => PlatformEvent::QuitRequested,
        sdl::SDL_EVENT_WINDOW_PIXEL_SIZE_CHANGED => PlatformEvent::WindowResized,
        sdl::SDL_EVENT_WINDOW_MINIMIZED => PlatformEvent::WindowMinimized,
        sdl::SDL_EVENT_WINDOW_RESTORED => PlatformEvent::WindowRestored,
        sdl::SDL_EVENT_WINDOW_FOCUS_GAINED => PlatformEvent::FocusGained,
        sdl::SDL_EVENT_WINDOW_FOCUS_LOST => PlatformEvent::FocusLost,
        sdl::SDL_EVENT_KEY_DOWN | sdl::SDL_EVENT_KEY_UP => {
            let key_event = unsafe { event.key };
            let Some(key) = key_from_scancode(key_event.scancode) else { return Translated::Ignored };
            if key_event.down {
                PlatformEvent::KeyDown { key, repeat: key_event.repeat }
            } else {
                PlatformEvent::KeyUp { key }
            }
        }
        sdl::SDL_EVENT_MOUSE_MOTION => {
            let motion = unsafe { event.motion };
            PlatformEvent::MouseMoved { delta_x: motion.xrel, delta_y: motion.yrel }
        }
        sdl::SDL_EVENT_MOUSE_BUTTON_DOWN | sdl::SDL_EVENT_MOUSE_BUTTON_UP => {
            let button_event = unsafe { event.button };
            let button = match i32::from(button_event.button) {
                sdl::SDL_BUTTON_LEFT => MouseButton::Left,
                sdl::SDL_BUTTON_RIGHT => MouseButton::Right,
                sdl::SDL_BUTTON_MIDDLE => MouseButton::Middle,
                _ => return Translated::Ignored,
            };
            if button_event.down {
                PlatformEvent::MouseButtonDown { button }
            } else {
                PlatformEvent::MouseButtonUp { button }
            }
        }
        sdl::SDL_EVENT_MOUSE_WHEEL => PlatformEvent::MouseWheel { delta_y: unsafe { event.wheel }.y },
        sdl::SDL_EVENT_GAMEPAD_ADDED => return Translated::GamepadAdded(unsafe { event.gdevice }.which),
        sdl::SDL_EVENT_GAMEPAD_REMOVED => return Translated::GamepadRemoved(unsafe { event.gdevice }.which),
        sdl::SDL_EVENT_GAMEPAD_BUTTON_DOWN | sdl::SDL_EVENT_GAMEPAD_BUTTON_UP => {
            let button_event = unsafe { event.gbutton };
            let Some(button) = gamepad_button(sdl::SDL_GamepadButton(i32::from(button_event.button))) else {
                return Translated::Ignored;
            };
            if button_event.down {
                PlatformEvent::GamepadButtonDown { button }
            } else {
                PlatformEvent::GamepadButtonUp { button }
            }
        }
        sdl::SDL_EVENT_GAMEPAD_AXIS_MOTION => {
            let axis_event = unsafe { event.gaxis };
            let raw = f32::from(axis_event.value) / 32767.0;
            let (axis, value) = match sdl::SDL_GamepadAxis(i32::from(axis_event.axis)) {
                sdl::SDL_GAMEPAD_AXIS_LEFTX => (GamepadAxis::LeftX, raw),
                // SDL's stick Y is +down; Lantern's is +up.
                sdl::SDL_GAMEPAD_AXIS_LEFTY => (GamepadAxis::LeftY, -raw),
                sdl::SDL_GAMEPAD_AXIS_RIGHTX => (GamepadAxis::RightX, raw),
                sdl::SDL_GAMEPAD_AXIS_RIGHTY => (GamepadAxis::RightY, -raw),
                sdl::SDL_GAMEPAD_AXIS_LEFT_TRIGGER => (GamepadAxis::LeftTrigger, raw),
                sdl::SDL_GAMEPAD_AXIS_RIGHT_TRIGGER => (GamepadAxis::RightTrigger, raw),
                _ => return Translated::Ignored,
            };
            PlatformEvent::GamepadAxisMoved { axis, value: value.clamp(-1.0, 1.0) }
        }
        _ => return Translated::Ignored,
    };
    Translated::Event(translated)
}

fn gamepad_button(button: sdl::SDL_GamepadButton) -> Option<GamepadButton> {
    Some(match button {
        sdl::SDL_GAMEPAD_BUTTON_SOUTH => GamepadButton::South,
        sdl::SDL_GAMEPAD_BUTTON_EAST => GamepadButton::East,
        sdl::SDL_GAMEPAD_BUTTON_WEST => GamepadButton::West,
        sdl::SDL_GAMEPAD_BUTTON_NORTH => GamepadButton::North,
        sdl::SDL_GAMEPAD_BUTTON_BACK => GamepadButton::Back,
        sdl::SDL_GAMEPAD_BUTTON_START => GamepadButton::Start,
        sdl::SDL_GAMEPAD_BUTTON_LEFT_STICK => GamepadButton::LeftStick,
        sdl::SDL_GAMEPAD_BUTTON_RIGHT_STICK => GamepadButton::RightStick,
        sdl::SDL_GAMEPAD_BUTTON_LEFT_SHOULDER => GamepadButton::LeftShoulder,
        sdl::SDL_GAMEPAD_BUTTON_RIGHT_SHOULDER => GamepadButton::RightShoulder,
        sdl::SDL_GAMEPAD_BUTTON_DPAD_UP => GamepadButton::DPadUp,
        sdl::SDL_GAMEPAD_BUTTON_DPAD_DOWN => GamepadButton::DPadDown,
        sdl::SDL_GAMEPAD_BUTTON_DPAD_LEFT => GamepadButton::DPadLeft,
        sdl::SDL_GAMEPAD_BUTTON_DPAD_RIGHT => GamepadButton::DPadRight,
        _ => return None,
    })
}

/// Physical key positions (scancodes), so WASD stays WASD on AZERTY and Dvorak layouts.
fn key_from_scancode(scancode: sdl::SDL_Scancode) -> Option<KeyCode> {
    use sdl::SDL_Scancode as S;
    Some(match scancode {
        S::A => KeyCode::A,
        S::B => KeyCode::B,
        S::C => KeyCode::C,
        S::D => KeyCode::D,
        S::E => KeyCode::E,
        S::F => KeyCode::F,
        S::G => KeyCode::G,
        S::H => KeyCode::H,
        S::I => KeyCode::I,
        S::J => KeyCode::J,
        S::K => KeyCode::K,
        S::L => KeyCode::L,
        S::M => KeyCode::M,
        S::N => KeyCode::N,
        S::O => KeyCode::O,
        S::P => KeyCode::P,
        S::Q => KeyCode::Q,
        S::R => KeyCode::R,
        S::S => KeyCode::S,
        S::T => KeyCode::T,
        S::U => KeyCode::U,
        S::V => KeyCode::V,
        S::W => KeyCode::W,
        S::X => KeyCode::X,
        S::Y => KeyCode::Y,
        S::Z => KeyCode::Z,
        S::_0 => KeyCode::Digit0,
        S::_1 => KeyCode::Digit1,
        S::_2 => KeyCode::Digit2,
        S::_3 => KeyCode::Digit3,
        S::_4 => KeyCode::Digit4,
        S::_5 => KeyCode::Digit5,
        S::_6 => KeyCode::Digit6,
        S::_7 => KeyCode::Digit7,
        S::_8 => KeyCode::Digit8,
        S::_9 => KeyCode::Digit9,
        S::UP => KeyCode::Up,
        S::DOWN => KeyCode::Down,
        S::LEFT => KeyCode::Left,
        S::RIGHT => KeyCode::Right,
        S::SPACE => KeyCode::Space,
        S::RETURN => KeyCode::Enter,
        S::ESCAPE => KeyCode::Escape,
        S::TAB => KeyCode::Tab,
        S::BACKSPACE => KeyCode::Backspace,
        S::GRAVE => KeyCode::Backquote,
        S::LSHIFT => KeyCode::LeftShift,
        S::RSHIFT => KeyCode::RightShift,
        S::LCTRL => KeyCode::LeftControl,
        S::RCTRL => KeyCode::RightControl,
        S::LALT => KeyCode::LeftAlt,
        S::RALT => KeyCode::RightAlt,
        S::F1 => KeyCode::F1,
        S::F2 => KeyCode::F2,
        S::F3 => KeyCode::F3,
        S::F4 => KeyCode::F4,
        S::F5 => KeyCode::F5,
        S::F6 => KeyCode::F6,
        S::F7 => KeyCode::F7,
        S::F8 => KeyCode::F8,
        S::F9 => KeyCode::F9,
        S::F10 => KeyCode::F10,
        S::F11 => KeyCode::F11,
        S::F12 => KeyCode::F12,
        _ => return None,
    })
}
