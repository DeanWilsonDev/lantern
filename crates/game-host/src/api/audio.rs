//! `Audio`: sounds by name. A sound that is missing or broken is logged and stays silent; a
//! game never fails to start over a sound effect.

use nyx::Runtime;

use super::{SharedState, f32_argument, string_argument};

struct AudioApi;

pub fn register(runtime: &Runtime, state: &SharedState) -> nyx::Result<()> {
    let load = state.clone();
    let play = state.clone();
    let music = state.clone();
    let stop = state.clone();
    runtime
        .register_type::<AudioApi>("Audio")
        .static_method("Load", "(name: string, path: string) -> void", move |_context, _, arguments| {
            let name = string_argument(arguments, 0)?;
            let relative = string_argument(arguments, 1)?;
            let mut state = load.borrow_mut();
            let path = state.asset_path(&relative);
            state.audio.load_sound(&name, &path);
            Ok(None)
        })
        .static_method("Play", "(name: string, volume: f32) -> void", move |_context, _, arguments| {
            let name = string_argument(arguments, 0)?;
            let volume = f32_argument(arguments, 1)?;
            play.borrow_mut().audio.play(&name, volume);
            Ok(None)
        })
        // Loops on the music bus, replacing any music already playing.
        .static_method("PlayMusic", "(name: string) -> void", move |_context, _, arguments| {
            let name = string_argument(arguments, 0)?;
            music.borrow_mut().audio.play_music(&name);
            Ok(None)
        })
        .static_method("StopMusic", "() -> void", move |_context, _, _| {
            stop.borrow_mut().audio.stop_music();
            Ok(None)
        })
        .register()?;
    Ok(())
}
