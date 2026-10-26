//! The engine's audio system: a `Mixer` feeding an `AudioSink`, with named sounds.
//!
//! Mixing happens on the main thread once per frame (decision D-006). Each frame tops the
//! device queue up to `TARGET_QUEUED_SECONDS`; at 60 fps that is about 3.6 frames of headroom,
//! so a single slow frame does not run the device dry. Sound latency is at most that much.

use std::path::Path;

use lantern_audio::mixer::{Mixer, SoundBank, SoundHandle, Volumes};
use lantern_audio::wav::load_wav;
use lantern_platform::AudioSink;

/// How much audio we keep queued ahead of the device.
pub const TARGET_QUEUED_SECONDS: f32 = 0.06;

/// The rate used when there is no device, so sounds still load and the code paths still run.
const FALLBACK_SAMPLE_RATE: u32 = 48_000;

pub struct AudioSystem {
    mixer: Mixer,
    sink: Option<Box<dyn AudioSink>>,
    sounds: SoundBank,
    mix_buffer: Vec<f32>,
    underrun_count: u32,
    has_pushed_audio: bool,
}

/// Numbers for the debug HUD.
#[derive(Clone, Copy, Debug, Default)]
pub struct AudioStats {
    pub has_device: bool,
    pub active_voices: usize,
    pub queued_milliseconds: f32,
    pub underrun_count: u32,
}

impl AudioSystem {
    /// `sink` is `None` when no device could be opened: everything still works, silently.
    pub fn new(sink: Option<Box<dyn AudioSink>>) -> AudioSystem {
        let sample_rate = sink.as_ref().map_or(FALLBACK_SAMPLE_RATE, |sink| sink.sample_rate());
        AudioSystem {
            mixer: Mixer::new(sample_rate),
            sink,
            sounds: SoundBank::default(),
            mix_buffer: Vec::new(),
            underrun_count: 0,
            has_pushed_audio: false,
        }
    }

    /// Loads a WAV under `name`. A missing or broken file is logged and the name stays silent;
    /// a game should never fail to start over a sound effect.
    pub fn load_sound(&mut self, name: &str, path: &Path) -> Option<SoundHandle> {
        match load_wav(path) {
            Ok(sound) => {
                let handle = self.mixer.add_sound(sound);
                self.sounds.insert(name, handle);
                Some(handle)
            }
            Err(error) => {
                log::warn!(target: "audio", "sound '{name}' unavailable: {error}");
                None
            }
        }
    }

    pub fn has_sound(&self, name: &str) -> bool {
        self.sounds.get(name).is_some()
    }

    /// Plays a one-shot effect by name. Unknown names are ignored (they were logged at load).
    pub fn play(&mut self, name: &str, volume: f32) {
        if let Some(sound) = self.sounds.get(name) {
            self.mixer.play(sound, volume);
        }
    }

    pub fn play_music(&mut self, name: &str) {
        if let Some(sound) = self.sounds.get(name) {
            self.mixer.play_music(sound);
        }
    }

    pub fn stop_music(&mut self) {
        self.mixer.stop_music();
    }

    pub fn volumes_mut(&mut self) -> &mut Volumes {
        &mut self.mixer.volumes
    }

    /// Call once per frame: mixes enough audio to refill the device queue.
    pub fn update(&mut self) {
        let Some(sink) = self.sink.as_mut() else { return };
        let target_frames = (TARGET_QUEUED_SECONDS * self.mixer.sample_rate() as f32) as usize;
        let queued_frames = sink.queued_frames();
        if queued_frames == 0 && self.has_pushed_audio {
            self.underrun_count += 1;
        }
        let frames_to_mix = target_frames.saturating_sub(queued_frames);
        if frames_to_mix == 0 {
            return;
        }
        self.mix_buffer.resize(frames_to_mix * 2, 0.0);
        self.mixer.mix(&mut self.mix_buffer);
        sink.push_frames(&self.mix_buffer);
        self.has_pushed_audio = true;
    }

    pub fn stats(&self) -> AudioStats {
        let queued_frames = self.sink.as_ref().map_or(0, |sink| sink.queued_frames());
        AudioStats {
            has_device: self.sink.is_some(),
            active_voices: self.mixer.active_voice_count(),
            queued_milliseconds: queued_frames as f32 * 1000.0 / self.mixer.sample_rate() as f32,
            underrun_count: self.underrun_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// A fake device that plays `frames_played_per_update` frames between updates.
    struct FakeSink {
        queued_frames: Rc<RefCell<usize>>,
        pushed_samples: Rc<RefCell<Vec<f32>>>,
    }

    impl AudioSink for FakeSink {
        fn sample_rate(&self) -> u32 {
            1000
        }
        fn queued_frames(&self) -> usize {
            *self.queued_frames.borrow()
        }
        fn push_frames(&mut self, interleaved_stereo: &[f32]) {
            *self.queued_frames.borrow_mut() += interleaved_stereo.len() / 2;
            self.pushed_samples.borrow_mut().extend_from_slice(interleaved_stereo);
        }
    }

    #[test]
    fn update_tops_the_queue_up_to_the_target() {
        let queued_frames = Rc::new(RefCell::new(0));
        let pushed_samples = Rc::new(RefCell::new(Vec::new()));
        let sink = FakeSink { queued_frames: queued_frames.clone(), pushed_samples: pushed_samples.clone() };
        let mut audio = AudioSystem::new(Some(Box::new(sink)));
        audio.update();
        assert_eq!(*queued_frames.borrow(), 60, "60 ms at 1 kHz");
        *queued_frames.borrow_mut() = 45; // the device played 15 frames
        audio.update();
        assert_eq!(*queued_frames.borrow(), 60);
        assert_eq!(pushed_samples.borrow().len(), 75 * 2);
        assert_eq!(audio.stats().underrun_count, 0);
        *queued_frames.borrow_mut() = 0;
        audio.update();
        assert_eq!(audio.stats().underrun_count, 1);
    }

    #[test]
    fn missing_device_and_missing_sounds_are_not_errors() {
        let mut audio = AudioSystem::new(None);
        assert!(audio.load_sound("jump", Path::new("does/not/exist.wav")).is_none());
        audio.play("jump", 1.0);
        audio.play_music("theme");
        audio.update();
        assert!(!audio.stats().has_device);
    }
}
