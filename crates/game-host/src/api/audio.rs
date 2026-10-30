//! `Audio`: sounds by name. A sound that is missing or broken is logged and stays silent; a
//! game never fails to start over a sound effect.

use nyx::Runtime;

use super::{HostNamespace, SharedState, f32_argument, string_argument};

pub fn register(runtime: &Runtime, state: &SharedState) -> nyx::Result<()> {
    let load = state.clone();
    let play = state.clone();
    let music = state.clone();
    let stop = state.clone();
    HostNamespace::new(runtime, "lantern::audio")
        .function("Load", "(name: string, path: string) -> void", move |_context, arguments| {
            let name = string_argument(arguments, 0)?;
            let relative = string_argument(arguments, 1)?;
            let mut state = load.borrow_mut();
            let path = state.asset_path(&relative);
            state.audio.load_sound(&name, &path);
            Ok(None)
        })
        .function("Play", "(name: string, volume: f32) -> void", move |_context, arguments| {
            let name = string_argument(arguments, 0)?;
            let volume = f32_argument(arguments, 1)?;
            play.borrow_mut().audio.play(&name, volume);
            Ok(None)
        })
        // Loops on the music bus, replacing any music already playing.
        .function("PlayMusic", "(name: string) -> void", move |_context, arguments| {
            let name = string_argument(arguments, 0)?;
            music.borrow_mut().audio.play_music(&name);
            Ok(None)
        })
        .function("StopMusic", "() -> void", move |_context, _| {
            stop.borrow_mut().audio.stop_music();
            Ok(None)
        })
        .register()?;
    Ok(())
}
