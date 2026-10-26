//! Tiny deterministic synthesiser for the placeholder sound effects and music loop.
//!
//! Every note is shaped by an envelope that reaches zero before the note ends, so sounds start
//! and stop without clicks and the music loop is seamless.

use std::f32::consts::TAU;

/// 22.05 kHz mono keeps the committed files small; the mixer resamples to the device rate
/// when a sound is loaded.
pub const SAMPLE_RATE: u32 = 22_050;

const SAMPLE_PERIOD: f32 = 1.0 / SAMPLE_RATE as f32;

#[derive(Clone, Copy)]
enum Waveform {
    Sine,
    Triangle,
    /// A square wave with its harmonics softened, so it is less harsh than a pure square.
    SoftSquare,
    Noise,
}

/// A deterministic linear congruential generator for the noise waveform.
struct NoiseSource(u32);

impl NoiseSource {
    fn next_sample(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 8) as f32 / (1u32 << 23) as f32 - 1.0
    }
}

fn oscillator(waveform: Waveform, phase: f32, noise: &mut NoiseSource) -> f32 {
    let wrapped = phase.fract();
    match waveform {
        Waveform::Sine => (wrapped * TAU).sin(),
        Waveform::Triangle => 1.0 - 4.0 * (wrapped - 0.5).abs(),
        Waveform::SoftSquare => ((wrapped * TAU).sin() * 3.0).tanh(),
        Waveform::Noise => noise.next_sample(),
    }
}

/// One synthesised note: a pitch that glides from `start_hz` to `end_hz` (exponentially, as
/// pitch is heard), with a linear attack and an exponential decay that is forced to zero at the
/// end.
struct Note {
    waveform: Waveform,
    start_hz: f32,
    end_hz: f32,
    duration_seconds: f32,
    attack_seconds: f32,
    /// How fast the decay falls: larger is more percussive.
    decay_rate: f32,
    volume: f32,
}

impl Note {
    fn tone(waveform: Waveform, hz: f32, duration_seconds: f32, volume: f32) -> Note {
        Note {
            waveform,
            start_hz: hz,
            end_hz: hz,
            duration_seconds,
            attack_seconds: 0.005,
            decay_rate: 3.0 / duration_seconds,
            volume,
        }
    }

    fn glide(mut self, end_hz: f32) -> Note {
        self.end_hz = end_hz;
        self
    }

    fn decay(mut self, decay_rate: f32) -> Note {
        self.decay_rate = decay_rate;
        self
    }

    /// Adds the note into `buffer` starting at `start_seconds`.
    fn render_into(&self, buffer: &mut [f32], start_seconds: f32) {
        let first_sample = (start_seconds * SAMPLE_RATE as f32) as usize;
        let sample_count = (self.duration_seconds * SAMPLE_RATE as f32) as usize;
        let mut noise = NoiseSource(0x1234_5678 ^ first_sample as u32);
        let mut phase = 0.0f32;
        // The last 4 ms fade linearly to zero whatever the decay, to avoid a click.
        let release_samples = (0.004 * SAMPLE_RATE as f32) as usize;
        for index in 0..sample_count {
            let Some(slot) = buffer.get_mut(first_sample + index) else { break };
            let time = index as f32 * SAMPLE_PERIOD;
            let progress = time / self.duration_seconds;
            let frequency = self.start_hz * (self.end_hz / self.start_hz).powf(progress);
            phase += frequency * SAMPLE_PERIOD;
            let attack = (time / self.attack_seconds).min(1.0);
            let release = ((sample_count - index) as f32 / release_samples as f32).min(1.0);
            let envelope = attack * (-self.decay_rate * time).exp() * release;
            *slot += oscillator(self.waveform, phase, &mut noise) * envelope * self.volume;
        }
    }
}

fn silence(duration_seconds: f32) -> Vec<f32> {
    vec![0.0; (duration_seconds * SAMPLE_RATE as f32) as usize]
}

/// Scales the buffer so its loudest sample is `peak`.
fn normalized(mut buffer: Vec<f32>, peak: f32) -> Vec<f32> {
    let loudest = buffer.iter().fold(0.0f32, |loudest, sample| loudest.max(sample.abs()));
    if loudest > 0.0 {
        buffer.iter_mut().for_each(|sample| *sample *= peak / loudest);
    }
    buffer
}

fn midi_to_hz(note: i32) -> f32 {
    440.0 * 2f32.powf((note - 69) as f32 / 12.0)
}

pub fn jump() -> Vec<f32> {
    let mut buffer = silence(0.18);
    Note::tone(Waveform::SoftSquare, 280.0, 0.18, 1.0).glide(720.0).decay(12.0).render_into(&mut buffer, 0.0);
    normalized(buffer, 0.7)
}

pub fn land() -> Vec<f32> {
    let mut buffer = silence(0.14);
    Note::tone(Waveform::Sine, 140.0, 0.14, 1.0).glide(55.0).decay(25.0).render_into(&mut buffer, 0.0);
    Note::tone(Waveform::Noise, 1.0, 0.06, 0.35).decay(60.0).render_into(&mut buffer, 0.0);
    normalized(buffer, 0.8)
}

pub fn pickup() -> Vec<f32> {
    let mut buffer = silence(0.32);
    Note::tone(Waveform::Triangle, midi_to_hz(83), 0.09, 1.0).decay(10.0).render_into(&mut buffer, 0.0);
    Note::tone(Waveform::Triangle, midi_to_hz(88), 0.23, 1.0).decay(12.0).render_into(&mut buffer, 0.08);
    normalized(buffer, 0.7)
}

pub fn menu_move() -> Vec<f32> {
    let mut buffer = silence(0.05);
    Note::tone(Waveform::Triangle, midi_to_hz(76), 0.05, 1.0).decay(40.0).render_into(&mut buffer, 0.0);
    normalized(buffer, 0.5)
}

pub fn menu_select() -> Vec<f32> {
    let mut buffer = silence(0.22);
    Note::tone(Waveform::Triangle, midi_to_hz(72), 0.08, 1.0).decay(15.0).render_into(&mut buffer, 0.0);
    Note::tone(Waveform::Triangle, midi_to_hz(79), 0.16, 1.0).decay(15.0).render_into(&mut buffer, 0.06);
    normalized(buffer, 0.6)
}

pub fn reset() -> Vec<f32> {
    let mut buffer = silence(0.35);
    Note::tone(Waveform::SoftSquare, 620.0, 0.35, 1.0).glide(150.0).decay(6.0).render_into(&mut buffer, 0.0);
    normalized(buffer, 0.6)
}

/// An 8-second loop at 120 bpm: four bars of Am, F, C, G with a triangle bass on the eighth
/// notes and a sine arpeggio on the sixteenths.
pub fn theme_loop() -> Vec<f32> {
    const BEAT_SECONDS: f32 = 0.5;
    const BARS: [[i32; 3]; 4] = [[57, 60, 64], [53, 57, 60], [48, 52, 55], [55, 59, 62]];
    let mut buffer = silence(BEAT_SECONDS * 4.0 * BARS.len() as f32);
    for (bar_index, chord) in BARS.iter().enumerate() {
        let bar_start = bar_index as f32 * BEAT_SECONDS * 4.0;
        for eighth in 0..8 {
            let start = bar_start + eighth as f32 * BEAT_SECONDS / 2.0;
            let bass_note = chord[0] - 12 + if eighth % 4 == 3 { 7 } else { 0 };
            Note::tone(Waveform::Triangle, midi_to_hz(bass_note), BEAT_SECONDS / 2.0, 0.5)
                .decay(5.0)
                .render_into(&mut buffer, start);
        }
        // Arpeggio pattern over the chord, one octave up, rising then falling.
        const PATTERN: [usize; 8] = [0, 1, 2, 1, 0, 2, 1, 2];
        for sixteenth in 0..16 {
            let start = bar_start + sixteenth as f32 * BEAT_SECONDS / 4.0;
            let note = chord[PATTERN[sixteenth % PATTERN.len()]] + 12;
            let accent = if sixteenth % 4 == 0 { 0.32 } else { 0.2 };
            Note::tone(Waveform::Sine, midi_to_hz(note), BEAT_SECONDS / 4.0, accent)
                .decay(14.0)
                .render_into(&mut buffer, start);
        }
    }
    normalized(buffer, 0.55)
}
