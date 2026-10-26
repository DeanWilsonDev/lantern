//! The software mixer: one-shot sound effects plus one looping music track, with master, music
//! and effects volumes.

use std::collections::HashMap;

use lantern_core::handle::{Handle, Pool};

use crate::wav::SoundData;

pub type SoundHandle = Handle<SoundData>;

/// More simultaneous effects than this steal the oldest voice.
pub const MAX_VOICES: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Volumes {
    pub master: f32,
    pub music: f32,
    pub effects: f32,
}

impl Default for Volumes {
    fn default() -> Self {
        Volumes { master: 0.8, music: 0.5, effects: 0.9 }
    }
}

struct Voice {
    sound: SoundHandle,
    frame: usize,
    volume: f32,
}

pub struct Mixer {
    sample_rate: u32,
    sounds: Pool<SoundData>,
    voices: Vec<Voice>,
    music: Option<Voice>,
    pub volumes: Volumes,
    frames_mixed: u64,
}

impl Mixer {
    pub fn new(sample_rate: u32) -> Mixer {
        Mixer {
            sample_rate,
            sounds: Pool::new(),
            voices: Vec::new(),
            music: None,
            volumes: Volumes::default(),
            frames_mixed: 0,
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Takes ownership of a sound, converting it to the mixer's rate.
    pub fn add_sound(&mut self, sound: SoundData) -> SoundHandle {
        self.sounds.insert(sound.resampled(self.sample_rate))
    }

    pub fn remove_sound(&mut self, sound: SoundHandle) {
        self.voices.retain(|voice| voice.sound != sound);
        if self.music.as_ref().is_some_and(|music| music.sound == sound) {
            self.music = None;
        }
        self.sounds.remove(sound);
    }

    pub fn play(&mut self, sound: SoundHandle, volume: f32) {
        if !self.sounds.contains(sound) {
            log::warn!(target: "audio", "play: unknown sound {sound:?}");
            return;
        }
        if self.voices.len() >= MAX_VOICES {
            self.voices.remove(0);
        }
        self.voices.push(Voice { sound, frame: 0, volume });
    }

    /// Starts (or restarts) the looping music track.
    pub fn play_music(&mut self, sound: SoundHandle) {
        if self.music.as_ref().is_some_and(|music| music.sound == sound) {
            return;
        }
        self.music = Some(Voice { sound, frame: 0, volume: 1.0 });
    }

    pub fn stop_music(&mut self) {
        self.music = None;
    }

    pub fn active_voice_count(&self) -> usize {
        self.voices.len() + usize::from(self.music.is_some())
    }

    pub fn frames_mixed(&self) -> u64 {
        self.frames_mixed
    }

    /// Fills `out` (interleaved stereo) with the next `out.len() / 2` frames.
    pub fn mix(&mut self, out: &mut [f32]) {
        out.fill(0.0);
        let frames = out.len() / 2;
        let effects_gain = self.volumes.master * self.volumes.effects;
        let music_gain = self.volumes.master * self.volumes.music;
        let sounds = &self.sounds;

        self.voices.retain_mut(|voice| {
            let Some(sound) = sounds.get(voice.sound) else { return false };
            let available = sound.frame_count().saturating_sub(voice.frame);
            let count = available.min(frames);
            let gain = effects_gain * voice.volume;
            for frame in 0..count {
                let source = (voice.frame + frame) * 2;
                out[frame * 2] += sound.samples[source] * gain;
                out[frame * 2 + 1] += sound.samples[source + 1] * gain;
            }
            voice.frame += count;
            voice.frame < sound.frame_count()
        });

        if let Some(music) = &mut self.music {
            if let Some(sound) = sounds.get(music.sound).filter(|sound| sound.frame_count() > 0) {
                for frame in 0..frames {
                    let source = music.frame * 2;
                    out[frame * 2] += sound.samples[source] * music_gain;
                    out[frame * 2 + 1] += sound.samples[source + 1] * music_gain;
                    music.frame = (music.frame + 1) % sound.frame_count();
                }
            }
        }

        // Soft clip so several loud sounds at once distort gently instead of wrapping.
        for sample in out.iter_mut() {
            *sample = sample.clamp(-1.5, 1.5);
            *sample -= *sample * *sample * *sample / 6.75;
        }
        self.frames_mixed += frames as u64;
    }
}

/// Sounds by name, for scripts and data files ("jump", "pickup").
#[derive(Default)]
pub struct SoundBank {
    by_name: HashMap<String, SoundHandle>,
}

impl SoundBank {
    pub fn insert(&mut self, name: &str, sound: SoundHandle) {
        self.by_name.insert(name.to_string(), sound);
    }

    pub fn get(&self, name: &str) -> Option<SoundHandle> {
        self.by_name.get(name).copied()
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.by_name.keys().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn constant(value: f32, frames: usize) -> SoundData {
        SoundData { sample_rate: 48000, samples: vec![value; frames * 2] }
    }

    fn quiet_mixer() -> Mixer {
        let mut mixer = Mixer::new(48000);
        mixer.volumes = Volumes { master: 1.0, music: 1.0, effects: 1.0 };
        mixer
    }

    #[test]
    fn effects_play_once_then_free_their_voice() {
        let mut mixer = quiet_mixer();
        let beep = mixer.add_sound(constant(0.1, 100));
        mixer.play(beep, 1.0);
        let mut out = vec![0.0; 2 * 64];
        mixer.mix(&mut out);
        assert!(out[0] > 0.09 && out[0] < 0.1, "{}", out[0]);
        assert_eq!(mixer.active_voice_count(), 1);
        mixer.mix(&mut out);
        assert!(out[2 * 35] > 0.0 && out[2 * 36] == 0.0, "the remaining 36 frames, then silence");
        assert_eq!(mixer.active_voice_count(), 0);
    }

    #[test]
    fn music_loops_forever() {
        let mut mixer = quiet_mixer();
        let song = mixer.add_sound(constant(0.2, 10));
        mixer.play_music(song);
        let mut out = vec![0.0; 2 * 1000];
        mixer.mix(&mut out);
        assert!(out.iter().all(|&sample| sample > 0.1));
        assert_eq!(mixer.active_voice_count(), 1);
    }

    #[test]
    fn volumes_scale_and_loud_mixes_stay_bounded() {
        let mut mixer = quiet_mixer();
        let loud = mixer.add_sound(constant(1.0, 10));
        for _ in 0..8 {
            mixer.play(loud, 1.0);
        }
        let mut out = vec![0.0; 20];
        mixer.mix(&mut out);
        assert!(out.iter().all(|sample| sample.abs() <= 1.0));
        mixer.volumes.master = 0.0;
        mixer.play(loud, 1.0);
        mixer.mix(&mut out);
        assert!(out.iter().all(|&sample| sample == 0.0));
    }

    #[test]
    fn voices_are_capped() {
        let mut mixer = quiet_mixer();
        let beep = mixer.add_sound(constant(0.01, 1000));
        for _ in 0..MAX_VOICES + 5 {
            mixer.play(beep, 1.0);
        }
        assert_eq!(mixer.active_voice_count(), MAX_VOICES);
    }

    #[test]
    fn removed_sounds_stop_and_unknown_handles_are_ignored() {
        let mut mixer = quiet_mixer();
        let beep = mixer.add_sound(constant(0.5, 1000));
        mixer.play(beep, 1.0);
        mixer.play_music(beep);
        mixer.remove_sound(beep);
        assert_eq!(mixer.active_voice_count(), 0);
        mixer.play(beep, 1.0);
        assert_eq!(mixer.active_voice_count(), 0);
    }
}
