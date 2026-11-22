use std::fs::{self, File};
use std::io::{BufReader, Write};
use std::path::Path;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

pub struct AudioManager {
    stream_handle: Option<OutputStreamHandle>,
    music_sink: Option<Sink>,
    pub available: bool,
}

impl AudioManager {
    pub fn init() -> (Option<OutputStream>, Self) {
        match OutputStream::try_default() {
            Ok((stream, handle)) => {
                (
                    Some(stream),
                    Self {
                        stream_handle: Some(handle),
                        music_sink: None,
                        available: true,
                    },
                )
            }
            Err(e) => {
                eprintln!(
                    "[Audio] Warning: No audio output device available ({:?}). Running in no-op audio mode.",
                    e
                );
                (
                    None,
                    Self {
                        stream_handle: None,
                        music_sink: None,
                        available: false,
                    },
                )
            }
        }
    }

    pub fn play_sound(&self, path: &str, volume: f32) -> Result<(), String> {
        if !self.available {
            return Ok(());
        }
        let handle = match &self.stream_handle {
            Some(h) => h,
            None => return Ok(()),
        };

        let file_path = Path::new(path);
        if !file_path.exists() {
            return Err(format!("Audio file '{}' not found", path));
        }

        let file = File::open(file_path).map_err(|e| format!("Failed to open '{}': {}", path, e))?;
        let reader = BufReader::new(file);
        let source = Decoder::new(reader).map_err(|e| format!("Failed to decode '{}': {}", path, e))?;

        let sink = Sink::try_new(handle).map_err(|e| format!("Failed to create audio sink: {}", e))?;
        sink.set_volume(volume.clamp(0.0, 2.0));
        sink.append(source);
        sink.detach();

        Ok(())
    }

    pub fn play_music(&mut self, path: &str, volume: f32) -> Result<(), String> {
        if !self.available {
            return Ok(());
        }
        let handle = match &self.stream_handle {
            Some(h) => h,
            None => return Ok(()),
        };

        let file_path = Path::new(path);
        if !file_path.exists() {
            return Err(format!("Music file '{}' not found", path));
        }

        let file = File::open(file_path).map_err(|e| format!("Failed to open '{}': {}", path, e))?;
        let reader = BufReader::new(file);
        let source = Decoder::new(reader).map_err(|e| format!("Failed to decode '{}': {}", path, e))?;

        // Stop existing music sink if playing
        if let Some(existing) = self.music_sink.take() {
            existing.stop();
        }

        let sink = Sink::try_new(handle).map_err(|e| format!("Failed to create music sink: {}", e))?;
        sink.set_volume(volume.clamp(0.0, 2.0));
        sink.append(source.repeat_infinite());

        self.music_sink = Some(sink);
        Ok(())
    }

    pub fn stop_music(&mut self) {
        if let Some(sink) = self.music_sink.take() {
            sink.stop();
        }
    }

    pub fn is_available(&self) -> bool {
        self.available
    }
}

/// Generates a procedural impact/bounce WAV audio file
pub fn generate_bounce_wav(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let sample_rate: u32 = 44100;
    let duration_secs: f32 = 0.22;
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut pcm_data = Vec::with_capacity(num_samples);

    let mut phase: f32 = 0.0;
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let progress = t / duration_secs;

        // Pitch drops from 480 Hz to 90 Hz
        let freq = 480.0 * (1.0 - progress * 0.8).max(0.1);
        phase += 2.0 * std::f32::consts::PI * freq / sample_rate as f32;

        // Exponential decay envelope
        let env = (-16.0 * progress).exp();
        let sample = (phase.sin() * env).clamp(-1.0, 1.0);
        let sample_i16 = (sample * 30000.0) as i16;
        pcm_data.push(sample_i16);
    }

    let data_len = (pcm_data.len() * 2) as u32;
    let total_len = 36 + data_len;

    let mut file = File::create(path)?;
    // RIFF header
    file.write_all(b"RIFF")?;
    file.write_all(&total_len.to_le_bytes())?;
    file.write_all(b"WAVE")?;

    // fmt subchunk
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?; // Subchunk1Size (16 for PCM)
    file.write_all(&1u16.to_le_bytes())?;  // AudioFormat (1 = PCM)
    file.write_all(&1u16.to_le_bytes())?;  // NumChannels (1 = Mono)
    file.write_all(&sample_rate.to_le_bytes())?;
    let byte_rate = sample_rate * 2;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&2u16.to_le_bytes())?;  // BlockAlign (NumChannels * BitsPerSample/8)
    file.write_all(&16u16.to_le_bytes())?; // BitsPerSample (16)

    // data subchunk
    file.write_all(b"data")?;
    file.write_all(&data_len.to_le_bytes())?;
    for sample in pcm_data {
        file.write_all(&sample.to_le_bytes())?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_bounce_wav() {
        let path = Path::new("assets/sfx/bounce.wav");
        generate_bounce_wav(path).expect("Failed to generate bounce.wav");
        assert!(path.exists());
    }
}

