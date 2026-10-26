//! Minimal RIFF/WAVE reader and writer: 16-bit PCM, mono or stereo, any sample rate.

use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct WavError(pub String);

impl std::fmt::Display for WavError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for WavError {}

/// Decoded audio: interleaved stereo `f32` in -1..1 at `sample_rate`.
#[derive(Clone, Debug, PartialEq)]
pub struct SoundData {
    pub sample_rate: u32,
    /// Interleaved left/right.
    pub samples: Vec<f32>,
}

impl SoundData {
    pub fn frame_count(&self) -> usize {
        self.samples.len() / 2
    }

    pub fn duration_seconds(&self) -> f32 {
        self.frame_count() as f32 / self.sample_rate as f32
    }

    /// Linear-interpolation resampling. Good enough for effects and a chiptune loop; a real
    /// engine would use a windowed-sinc filter.
    pub fn resampled(&self, target_rate: u32) -> SoundData {
        if target_rate == self.sample_rate || self.frame_count() == 0 {
            return SoundData { sample_rate: target_rate, samples: self.samples.clone() };
        }
        let source_frames = self.frame_count();
        let target_frames = (source_frames as u64 * u64::from(target_rate) / u64::from(self.sample_rate)) as usize;
        let ratio = self.sample_rate as f64 / target_rate as f64;
        let mut samples = Vec::with_capacity(target_frames * 2);
        for frame in 0..target_frames {
            let position = frame as f64 * ratio;
            let index = position as usize;
            let next = (index + 1).min(source_frames - 1);
            let t = (position - index as f64) as f32;
            for channel in 0..2 {
                let a = self.samples[index * 2 + channel];
                let b = self.samples[next * 2 + channel];
                samples.push(a + (b - a) * t);
            }
        }
        SoundData { sample_rate: target_rate, samples }
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(offset..offset + 2)?.try_into().ok()?))
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(offset..offset + 4)?.try_into().ok()?))
}

pub fn decode_wav(bytes: &[u8]) -> Result<SoundData, WavError> {
    let error = |message: &str| WavError(message.to_string());
    if bytes.get(0..4) != Some(b"RIFF") || bytes.get(8..12) != Some(b"WAVE") {
        return Err(error("not a RIFF/WAVE file"));
    }
    let mut offset = 12;
    let mut format: Option<(u16, u16, u32, u16)> = None; // (format tag, channels, rate, bits)
    while offset + 8 <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let size = read_u32(bytes, offset + 4).ok_or_else(|| error("truncated chunk header"))? as usize;
        let body_start = offset + 8;
        let body = bytes.get(body_start..body_start + size).ok_or_else(|| error("truncated chunk"))?;
        match id {
            b"fmt " => {
                format = Some((
                    read_u16(body, 0).ok_or_else(|| error("short fmt chunk"))?,
                    read_u16(body, 2).ok_or_else(|| error("short fmt chunk"))?,
                    read_u32(body, 4).ok_or_else(|| error("short fmt chunk"))?,
                    read_u16(body, 14).ok_or_else(|| error("short fmt chunk"))?,
                ));
            }
            b"data" => {
                let (tag, channels, sample_rate, bits) = format.ok_or_else(|| error("data before fmt"))?;
                if tag != 1 || bits != 16 {
                    return Err(WavError(format!("only 16-bit PCM is supported (format {tag}, {bits} bits)")));
                }
                if channels != 1 && channels != 2 {
                    return Err(WavError(format!("{channels} channels; only mono and stereo are supported")));
                }
                let values: Vec<f32> = body
                    .chunks_exact(2)
                    .map(|pair| f32::from(i16::from_le_bytes([pair[0], pair[1]])) / 32768.0)
                    .collect();
                let samples =
                    if channels == 1 { values.iter().flat_map(|&value| [value, value]).collect() } else { values };
                return Ok(SoundData { sample_rate, samples });
            }
            _ => {}
        }
        offset = body_start + size + (size & 1); // chunks are word-aligned
    }
    Err(error("no data chunk"))
}

pub fn load_wav(path: &Path) -> Result<SoundData, WavError> {
    let bytes = std::fs::read(path).map_err(|io| WavError(format!("{}: {io}", path.display())))?;
    decode_wav(&bytes).map_err(|WavError(message)| WavError(format!("{}: {message}", path.display())))
}

/// Encodes mono samples (-1..1) as 16-bit PCM. Used by `tools/asset-gen`.
pub fn encode_wav_mono(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let data_size = (samples.len() * 2) as u32;
    let mut bytes = Vec::with_capacity(44 + data_size as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&1u16.to_le_bytes()); // mono
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());
    for sample in samples {
        bytes.extend_from_slice(&((sample.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mono_round_trips_as_stereo() {
        let samples = [0.0, 0.5, -0.5, 1.0];
        let sound = decode_wav(&encode_wav_mono(&samples, 22050)).unwrap();
        assert_eq!(sound.sample_rate, 22050);
        assert_eq!(sound.frame_count(), 4);
        assert!((sound.samples[2] - 0.5).abs() < 1e-3 && (sound.samples[3] - 0.5).abs() < 1e-3);
    }

    #[test]
    fn resampling_keeps_duration() {
        let sound = decode_wav(&encode_wav_mono(&vec![0.25; 22050], 22050)).unwrap();
        let resampled = sound.resampled(48000);
        assert!((resampled.duration_seconds() - 1.0).abs() < 1e-3);
        assert!((resampled.samples[100] - 0.25).abs() < 1e-3);
    }

    #[test]
    fn bad_files_say_why() {
        assert!(decode_wav(b"nope").unwrap_err().0.contains("RIFF"));
        let mut eight_bit = encode_wav_mono(&[0.0], 8000);
        eight_bit[34] = 8;
        assert!(decode_wav(&eight_bit).unwrap_err().0.contains("16-bit"));
    }
}
