//! `Engine`: logging, quitting and run-time flags.

use nyx::Runtime;

use super::{SharedState, some_bool, some_string, string_argument};

struct EngineApi;

pub fn register(runtime: &Runtime, state: &SharedState) -> nyx::Result<()> {
    let quit_state = state.clone();
    let flag_state = state.clone();
    runtime
        .register_type::<EngineApi>("Engine")
        .static_method("Log", "(text: string) -> void", |_context, _, arguments| {
            log::info!(target: "game", "{}", string_argument(arguments, 0)?);
            Ok(None)
        })
        .static_method("Warn", "(text: string) -> void", |_context, _, arguments| {
            log::warn!(target: "game", "{}", string_argument(arguments, 0)?);
            Ok(None)
        })
        .static_method("Quit", "() -> void", move |_context, _, _| {
            quit_state.borrow_mut().requests.quit = true;
            Ok(None)
        })
        // A run-time switch: a test override, else an environment variable that is set and
        // not "0" (LANTERN_AUTOPLAY, ...).
        .static_method("Flag", "(name: string) -> bool", move |context, _, arguments| {
            let name = string_argument(arguments, 0)?;
            some_bool(context, flag_state.borrow().flag(&name))
        })
        // A run-time text setting from the environment, "" when unset (LANTERN_LEVEL: the
        // level the world editor's Play button launched).
        .static_method("Setting", "(name: string) -> string", move |context, _, arguments| {
            let name = string_argument(arguments, 0)?;
            let value = std::env::var(&name).unwrap_or_default();
            some_string(context, &value)
        })
        .register()?;
    Ok(())
}
