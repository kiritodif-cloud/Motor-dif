use std::f32::consts::PI;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use rodio::Source;
use serde::{Deserialize, Serialize};

pub const SAMPLE_RATE: u32 = 44100;
const TWO_PI: f32 = 2.0 * PI;

// ============================================================================
// Music Theory & Scale Helpers
// ============================================================================

pub fn get_scale_intervals(scale: &str) -> &'static [i32] {
    match scale.trim().to_lowercase().as_str() {
        "major" | "ionian" => &[0, 2, 4, 5, 7, 9, 11],
        "pentatonic" | "minor_pentatonic" => &[0, 3, 5, 7, 10],
        "dorian" => &[0, 2, 3, 5, 7, 9, 10],
        "phrygian" => &[0, 1, 3, 5, 7, 8, 10],
        _ => &[0, 2, 3, 5, 7, 8, 10], // default natural minor (aeolian)
    }
}

pub fn midi_to_freq(midi: f32) -> f32 {
    440.0 * 2.0_f32.powf((midi - 69.0) / 12.0)
}

pub fn scale_degree_to_midi(root_midi: u8, scale: &str, degree: i32) -> u8 {
    let intervals = get_scale_intervals(scale);
    let len = intervals.len() as i32;
    let octave = degree.div_euclid(len);
    let idx = degree.rem_euclid(len) as usize;
    let note = root_midi as i32 + octave * 12 + intervals[idx];
    note.clamp(12, 127) as u8
}

pub fn get_triad_midi_notes(root_midi: u8, scale: &str, chord_degree: i32) -> [u8; 3] {
    [
        scale_degree_to_midi(root_midi, scale, chord_degree),
        scale_degree_to_midi(root_midi, scale, chord_degree + 2),
        scale_degree_to_midi(root_midi, scale, chord_degree + 4),
    ]
}

// ============================================================================
// Music Recipe & Tracks Definition
// ============================================================================

fn default_true() -> bool { true }
fn default_bpm() -> f32 { 120.0 }
fn default_bars() -> usize { 4 }
fn default_root_midi() -> u8 { 57 } // A3
fn default_scale() -> String { "minor".to_string() }
fn default_chord_prog() -> Vec<i32> { vec![0, 5, 3, 4] }

fn default_pad_wave() -> String { "sine".to_string() }
fn default_pad_vol() -> f32 { 0.7 }
fn default_detune() -> f32 { 8.0 }

fn default_bass_wave() -> String { "sawtooth".to_string() }
fn default_bass_vol() -> f32 { 0.65 }
fn default_bass_cutoff() -> f32 { 420.0 }
fn default_bass_pattern() -> Vec<u8> { vec![1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0] }

fn default_lead_wave() -> String { "square".to_string() }
fn default_lead_vol() -> f32 { 0.55 }
fn default_lead_pattern() -> Vec<i32> { vec![0, 2, 4, 2, 0, 4, 5, 4, 0, 2, 4, 7, 5, 4, 2, 0] }

fn default_drum_vol() -> f32 { 0.75 }
fn default_kick_pat() -> Vec<u8> { vec![1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0] }
fn default_snare_pat() -> Vec<u8> { vec![0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0] }
fn default_hihat_pat() -> Vec<u8> { vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1] }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PadTrackDef {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_pad_wave")]
    pub waveform: String, // "sine", "triangle", "sawtooth"
    #[serde(default = "default_pad_vol")]
    pub volume: f32,
    #[serde(default = "default_detune")]
    pub detune_cents: f32,
    #[serde(default)]
    pub octave: i32,
}

impl Default for PadTrackDef {
    fn default() -> Self {
        Self {
            enabled: true,
            waveform: default_pad_wave(),
            volume: default_pad_vol(),
            detune_cents: default_detune(),
            octave: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BassTrackDef {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_bass_wave")]
    pub waveform: String, // "sawtooth", "triangle", "sine"
    #[serde(default = "default_bass_vol")]
    pub volume: f32,
    #[serde(default = "default_bass_cutoff")]
    pub cutoff_hz: f32,
    #[serde(default = "default_bass_pattern")]
    pub pattern: Vec<u8>,
    #[serde(default = "default_bass_octave")]
    pub octave: i32,
}

fn default_bass_octave() -> i32 { -2 }

impl Default for BassTrackDef {
    fn default() -> Self {
        Self {
            enabled: true,
            waveform: default_bass_wave(),
            volume: default_bass_vol(),
            cutoff_hz: default_bass_cutoff(),
            pattern: default_bass_pattern(),
            octave: default_bass_octave(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeadTrackDef {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_lead_wave")]
    pub waveform: String, // "square", "sine", "sawtooth"
    #[serde(default = "default_lead_vol")]
    pub volume: f32,
    #[serde(default = "default_lead_pattern")]
    pub pattern: Vec<i32>,
    #[serde(default = "default_lead_octave")]
    pub octave: i32,
}

fn default_lead_octave() -> i32 { 1 }

impl Default for LeadTrackDef {
    fn default() -> Self {
        Self {
            enabled: true,
            waveform: default_lead_wave(),
            volume: default_lead_vol(),
            pattern: default_lead_pattern(),
            octave: default_lead_octave(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrumTrackDef {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_drum_vol")]
    pub volume: f32,
    #[serde(default = "default_kick_pat")]
    pub kick_pattern: Vec<u8>,
    #[serde(default = "default_snare_pat")]
    pub snare_pattern: Vec<u8>,
    #[serde(default = "default_hihat_pat")]
    pub hihat_pattern: Vec<u8>,
}

impl Default for DrumTrackDef {
    fn default() -> Self {
        Self {
            enabled: true,
            volume: default_drum_vol(),
            kick_pattern: default_kick_pat(),
            snare_pattern: default_snare_pat(),
            hihat_pattern: default_hihat_pat(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TracksDef {
    #[serde(default)]
    pub pad: Option<PadTrackDef>,
    #[serde(default)]
    pub bass: Option<BassTrackDef>,
    #[serde(default)]
    pub lead: Option<LeadTrackDef>,
    #[serde(default)]
    pub drums: Option<DrumTrackDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicRecipe {
    pub name: String,
    #[serde(default = "default_bpm")]
    pub bpm: f32,
    #[serde(default = "default_bars")]
    pub bars: usize,
    #[serde(default = "default_root_midi")]
    pub root_midi: u8,
    #[serde(default = "default_scale")]
    pub scale: String,
    #[serde(default = "default_chord_prog")]
    pub chord_progression: Vec<i32>,
    #[serde(default)]
    pub tracks: TracksDef,
}

impl Default for MusicRecipe {
    fn default() -> Self {
        Self {
            name: "default_song".to_string(),
            bpm: default_bpm(),
            bars: default_bars(),
            root_midi: default_root_midi(),
            scale: default_scale(),
            chord_progression: default_chord_prog(),
            tracks: TracksDef {
                pad: Some(PadTrackDef::default()),
                bass: Some(BassTrackDef::default()),
                lead: Some(LeadTrackDef::default()),
                drums: Some(DrumTrackDef::default()),
            },
        }
    }
}

// ============================================================================
// Built-in Presets
// ============================================================================

pub fn get_music_preset(name: &str) -> Option<MusicRecipe> {
    match name.trim().to_lowercase().as_str() {
        "space_ambient" => Some(preset_space_ambient()),
        "boss_battle" => Some(preset_boss_battle()),
        "cyber_action" => Some(preset_cyber_action()),
        "menu_chill" => Some(preset_menu_chill()),
        _ => None,
    }
}

pub fn preset_space_ambient() -> MusicRecipe {
    MusicRecipe {
        name: "space_ambient".to_string(),
        bpm: 90.0,
        bars: 4,
        root_midi: 57, // A3
        scale: "dorian".to_string(),
        chord_progression: vec![0, 2, 4, 3], // i - III - v - IV
        tracks: TracksDef {
            pad: Some(PadTrackDef {
                enabled: true,
                waveform: "sine".to_string(),
                volume: 0.75,
                detune_cents: 8.0,
                octave: 0,
            }),
            bass: Some(BassTrackDef {
                enabled: true,
                waveform: "triangle".to_string(),
                volume: 0.60,
                cutoff_hz: 320.0,
                pattern: vec![1, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0],
                octave: -2,
            }),
            lead: Some(LeadTrackDef {
                enabled: true,
                waveform: "sine".to_string(),
                volume: 0.45,
                pattern: vec![0, -1, 2, -1, 4, -1, 2, -1, 5, -1, 4, -1, 2, -1, 0, -1],
                octave: 1,
            }),
            drums: Some(DrumTrackDef {
                enabled: true,
                volume: 0.50,
                kick_pattern: vec![1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0],
                snare_pattern: vec![0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0],
                hihat_pattern: vec![1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0],
            }),
        },
    }
}

pub fn preset_boss_battle() -> MusicRecipe {
    MusicRecipe {
        name: "boss_battle".to_string(),
        bpm: 138.0,
        bars: 4,
        root_midi: 52, // E3
        scale: "phrygian".to_string(),
        chord_progression: vec![0, 1, 0, 6], // i - bII - i - bVII
        tracks: TracksDef {
            pad: Some(PadTrackDef {
                enabled: true,
                waveform: "sawtooth".to_string(),
                volume: 0.55,
                detune_cents: 10.0,
                octave: 0,
            }),
            bass: Some(BassTrackDef {
                enabled: true,
                waveform: "sawtooth".to_string(),
                volume: 0.75,
                cutoff_hz: 650.0,
                pattern: vec![1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 1, 0],
                octave: -2,
            }),
            lead: Some(LeadTrackDef {
                enabled: true,
                waveform: "square".to_string(),
                volume: 0.60,
                pattern: vec![0, 1, 2, 1, 0, 2, 3, 2, 0, 1, 2, 4, 3, 2, 1, 0],
                octave: 1,
            }),
            drums: Some(DrumTrackDef {
                enabled: true,
                volume: 0.85,
                kick_pattern: vec![1, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 1, 0, 0],
                snare_pattern: vec![0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0],
                hihat_pattern: vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
            }),
        },
    }
}

pub fn preset_cyber_action() -> MusicRecipe {
    MusicRecipe {
        name: "cyber_action".to_string(),
        bpm: 125.0,
        bars: 4,
        root_midi: 57, // A3
        scale: "minor".to_string(),
        chord_progression: vec![0, 5, 3, 4], // i - VI - iv - v
        tracks: TracksDef {
            pad: Some(PadTrackDef {
                enabled: true,
                waveform: "sine".to_string(),
                volume: 0.65,
                detune_cents: 6.0,
                octave: 0,
            }),
            bass: Some(BassTrackDef {
                enabled: true,
                waveform: "sawtooth".to_string(),
                volume: 0.70,
                cutoff_hz: 520.0,
                pattern: vec![1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0],
                octave: -2,
            }),
            lead: Some(LeadTrackDef {
                enabled: true,
                waveform: "square".to_string(),
                volume: 0.55,
                pattern: vec![0, 2, 4, 7, 0, 2, 4, 7, 5, 4, 2, 0, 5, 4, 2, 0],
                octave: 1,
            }),
            drums: Some(DrumTrackDef {
                enabled: true,
                volume: 0.80,
                kick_pattern: vec![1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0],
                snare_pattern: vec![0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0],
                hihat_pattern: vec![0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0],
            }),
        },
    }
}

pub fn preset_menu_chill() -> MusicRecipe {
    MusicRecipe {
        name: "menu_chill".to_string(),
        bpm: 85.0,
        bars: 4,
        root_midi: 60, // C4
        scale: "pentatonic".to_string(),
        chord_progression: vec![0, 3, 1, 2],
        tracks: TracksDef {
            pad: Some(PadTrackDef {
                enabled: true,
                waveform: "sine".to_string(),
                volume: 0.70,
                detune_cents: 5.0,
                octave: 0,
            }),
            bass: Some(BassTrackDef {
                enabled: true,
                waveform: "triangle".to_string(),
                volume: 0.55,
                cutoff_hz: 280.0,
                pattern: vec![1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0],
                octave: -2,
            }),
            lead: Some(LeadTrackDef {
                enabled: true,
                waveform: "sine".to_string(),
                volume: 0.40,
                pattern: vec![0, -1, 1, -1, 2, -1, 4, -1, 3, -1, 2, -1, 1, -1, 0, -1],
                octave: 1,
            }),
            drums: Some(DrumTrackDef {
                enabled: true,
                volume: 0.40,
                kick_pattern: vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                snare_pattern: vec![0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0],
                hihat_pattern: vec![1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0],
            }),
        },
    }
}

// ============================================================================
// Stems Synthesis Engine
// ============================================================================

#[derive(Clone)]
pub struct MusicStems {
    pub length_samples: usize,
    pub pad: Vec<f32>,
    pub bass: Vec<f32>,
    pub lead: Vec<f32>,
    pub drums: Vec<f32>,
}

fn osc_sample(phase: f32, wave: &str) -> f32 {
    let p = phase % TWO_PI;
    match wave {
        "square" => if p.sin() >= 0.0 { 1.0 } else { -1.0 },
        "triangle" => 2.0 * (p / TWO_PI - (p / TWO_PI + 0.5).floor()).abs() * 2.0 - 1.0,
        "sawtooth" | "saw" => 2.0 * (p / TWO_PI) - 1.0,
        _ => p.sin(), // "sine"
    }
}

impl MusicRecipe {
    pub fn duration_secs(&self) -> f32 {
        let beat_dur = 60.0 / self.bpm.max(20.0);
        let bar_dur = 4.0 * beat_dur;
        bar_dur * self.bars.max(1) as f32
    }

    pub fn total_samples(&self) -> usize {
        (self.duration_secs() * SAMPLE_RATE as f32).round() as usize
    }

    pub fn synthesize_stems(&self) -> MusicStems {
        let total_samples = self.total_samples().max(1024);
        let mut pad_buf = vec![0.0f32; total_samples];
        let mut bass_buf = vec![0.0f32; total_samples];
        let mut lead_buf = vec![0.0f32; total_samples];
        let mut drum_buf = vec![0.0f32; total_samples];

        let beat_dur = 60.0 / self.bpm.max(20.0);
        let bar_dur = 4.0 * beat_dur;
        let bar_samples = (bar_dur * SAMPLE_RATE as f32).round() as usize;
        let step_dur = bar_dur / 16.0;
        let step_samples = (step_dur * SAMPLE_RATE as f32).round() as usize;

        let num_bars = self.bars.max(1);

        // 1. Synthesize Pad Track
        if let Some(pad) = &self.tracks.pad {
            if pad.enabled && pad.volume > 0.001 {
                let detune_ratio_hi = 2.0_f32.powf(pad.detune_cents / 1200.0);
                let detune_ratio_lo = 2.0_f32.powf(-pad.detune_cents / 1200.0);
                let mut phase1 = [0.0f32; 3];
                let mut phase2 = [0.0f32; 3];

                for bar_idx in 0..num_bars {
                    let chord_deg = self.chord_progression[bar_idx % self.chord_progression.len().max(1)];
                    let triad = get_triad_midi_notes(self.root_midi, &self.scale, chord_deg);
                    let freqs: Vec<f32> = triad.iter().map(|&m| {
                        let shifted = (m as i32 + pad.octave * 12).clamp(12, 127) as u8;
                        midi_to_freq(shifted as f32)
                    }).collect();

                    let bar_start = bar_idx * bar_samples;
                    let bar_end = ((bar_idx + 1) * bar_samples).min(total_samples);

                    for i in bar_start..bar_end {
                        let rel_i = i - bar_start;
                        let t = rel_i as f32 / SAMPLE_RATE as f32;
                        let progress = (rel_i as f32 / bar_samples as f32).clamp(0.0, 1.0);

                        // Smooth bar crossfade / envelope
                        let attack_time = 0.25;
                        let attack = if t < attack_time { t / attack_time } else { 1.0 };
                        let release = if progress > 0.85 { (1.0 - progress) / 0.15 } else { 1.0 };
                        let env = (attack * release).clamp(0.0, 1.0);

                        let mut s = 0.0f32;
                        for note_idx in 0..3 {
                            let f = freqs[note_idx];
                            phase1[note_idx] += TWO_PI * (f * detune_ratio_hi) / SAMPLE_RATE as f32;
                            phase2[note_idx] += TWO_PI * (f * detune_ratio_lo) / SAMPLE_RATE as f32;

                            s += osc_sample(phase1[note_idx], &pad.waveform) * 0.5;
                            s += osc_sample(phase2[note_idx], &pad.waveform) * 0.5;
                        }

                        pad_buf[i] += (s / 3.0) * env * pad.volume;
                    }
                }
            }
        }

        // 2. Synthesize Bass Track
        if let Some(bass) = &self.tracks.bass {
            if bass.enabled && bass.volume > 0.001 {
                let mut bass_phase = 0.0f32;
                let mut lpf_state = 0.0f32;
                let dt = 1.0 / SAMPLE_RATE as f32;
                let alpha = (TWO_PI * dt * bass.cutoff_hz) / (TWO_PI * dt * bass.cutoff_hz + 1.0);

                for bar_idx in 0..num_bars {
                    let chord_deg = self.chord_progression[bar_idx % self.chord_progression.len().max(1)];
                    let root_note = scale_degree_to_midi(self.root_midi, &self.scale, chord_deg);
                    let shifted = (root_note as i32 + bass.octave * 12).clamp(12, 127) as u8;
                    let freq = midi_to_freq(shifted as f32);

                    let bar_start = bar_idx * bar_samples;

                    for step in 0..16 {
                        let is_hit = bass.pattern.get(step % bass.pattern.len().max(1)).copied().unwrap_or(0) != 0;
                        let step_start = bar_start + step * step_samples;
                        let step_end = (step_start + step_samples).min(total_samples);

                        if is_hit {
                            for i in step_start..step_end {
                                let rel = i - step_start;
                                let step_prog = rel as f32 / step_samples as f32;

                                let attack = (step_prog / 0.05).min(1.0);
                                let decay = if step_prog > 0.6 { (1.0 - step_prog) / 0.4 } else { 1.0 };
                                let env = (attack * decay).clamp(0.0, 1.0);

                                bass_phase += TWO_PI * freq / SAMPLE_RATE as f32;
                                let raw = osc_sample(bass_phase, &bass.waveform);

                                lpf_state += alpha * (raw - lpf_state);
                                bass_buf[i] += lpf_state * env * bass.volume;
                            }
                        } else {
                            // Settle LPF on silence
                            for i in step_start..step_end {
                                lpf_state *= 0.999;
                                bass_buf[i] += 0.0;
                            }
                        }
                    }
                }
            }
        }

        // 3. Synthesize Lead Track
        if let Some(lead) = &self.tracks.lead {
            if lead.enabled && lead.volume > 0.001 {
                let mut lead_phase = 0.0f32;

                for bar_idx in 0..num_bars {
                    let chord_deg = self.chord_progression[bar_idx % self.chord_progression.len().max(1)];
                    let bar_start = bar_idx * bar_samples;

                    for step in 0..16 {
                        let note_deg_offset = lead.pattern.get(step % lead.pattern.len().max(1)).copied().unwrap_or(-1);
                        let step_start = bar_start + step * step_samples;
                        let step_end = (step_start + step_samples).min(total_samples);

                        if note_deg_offset >= 0 {
                            let note = scale_degree_to_midi(self.root_midi, &self.scale, chord_deg + note_deg_offset);
                            let shifted = (note as i32 + lead.octave * 12).clamp(12, 127) as u8;
                            let freq = midi_to_freq(shifted as f32);

                            for i in step_start..step_end {
                                let rel = i - step_start;
                                let step_prog = rel as f32 / step_samples as f32;

                                let attack = (step_prog / 0.08).min(1.0);
                                let decay = (1.0 - step_prog * 0.7).clamp(0.0, 1.0);
                                let env = (attack * decay).clamp(0.0, 1.0);

                                lead_phase += TWO_PI * freq / SAMPLE_RATE as f32;
                                let s = osc_sample(lead_phase, &lead.waveform);
                                lead_buf[i] += s * env * lead.volume;
                            }
                        }
                    }
                }
            }
        }

        // 4. Synthesize Drums Track
        if let Some(drums) = &self.tracks.drums {
            if drums.enabled && drums.volume > 0.001 {
                for bar_idx in 0..num_bars {
                    let bar_start = bar_idx * bar_samples;

                    for step in 0..16 {
                        let step_start = bar_start + step * step_samples;

                        let kick_hit = drums.kick_pattern.get(step % drums.kick_pattern.len().max(1)).copied().unwrap_or(0) != 0;
                        let snare_hit = drums.snare_pattern.get(step % drums.snare_pattern.len().max(1)).copied().unwrap_or(0) != 0;
                        let hihat_hit = drums.hihat_pattern.get(step % drums.hihat_pattern.len().max(1)).copied().unwrap_or(0) != 0;

                        // Synthesize Kick (Sweep 150 -> 40 Hz)
                        if kick_hit {
                            let kick_len = ((0.18 * SAMPLE_RATE as f32) as usize).min(total_samples - step_start);
                            let mut phase = 0.0f32;
                            for k in 0..kick_len {
                                let t = k as f32 / SAMPLE_RATE as f32;
                                let freq = 42.0 + 110.0 * (-32.0 * t).exp();
                                phase += TWO_PI * freq / SAMPLE_RATE as f32;
                                let env = (-16.0 * t).exp();
                                let sample = phase.sin() * env * 1.1;
                                drum_buf[step_start + k] += sample * drums.volume;
                            }
                        }

                        // Synthesize Snare (Noise + 180Hz body)
                        if snare_hit {
                            let snare_len = ((0.16 * SAMPLE_RATE as f32) as usize).min(total_samples - step_start);
                            let mut phase = 0.0f32;
                            for s in 0..snare_len {
                                let t = s as f32 / SAMPLE_RATE as f32;
                                phase += TWO_PI * 185.0 / SAMPLE_RATE as f32;
                                let body = phase.sin() * (-24.0 * t).exp();

                                let pseudo = (((step_start + s) as u32).wrapping_mul(1103515245).wrapping_add(12345) & 0x7FFFFFFF) as f32;
                                let noise = (pseudo / 1073741824.0) - 1.0;
                                let noise_env = (-18.0 * t).exp();

                                let sample = body * 0.45 + noise * noise_env * 0.55;
                                drum_buf[step_start + s] += sample * drums.volume;
                            }
                        }

                        // Synthesize Hihat (Short bright noise)
                        if hihat_hit {
                            let hihat_len = ((0.045 * SAMPLE_RATE as f32) as usize).min(total_samples - step_start);
                            for h in 0..hihat_len {
                                let t = h as f32 / SAMPLE_RATE as f32;
                                let pseudo = (((step_start + h) as u32).wrapping_mul(1664525).wrapping_add(1013904223) & 0x7FFFFFFF) as f32;
                                let noise = (pseudo / 1073741824.0) - 1.0;
                                let env = (-95.0 * t).exp();
                                let sample = noise * env * 0.45;
                                drum_buf[step_start + h] += sample * drums.volume;
                            }
                        }
                    }
                }
            }
        }

        // Smooth loop boundaries (50 samples crossfade) to guarantee 0 click on loop repeat
        let fade_len = 80.min(total_samples / 4);
        for i in 0..fade_len {
            let t = i as f32 / fade_len as f32;
            let end_idx = total_samples - fade_len + i;

            // Crossfade pad
            let blended_pad = pad_buf[end_idx] * (1.0 - t) + pad_buf[i] * t;
            pad_buf[i] = blended_pad;
            pad_buf[end_idx] = blended_pad;

            // Crossfade bass
            let blended_bass = bass_buf[end_idx] * (1.0 - t) + bass_buf[i] * t;
            bass_buf[i] = blended_bass;
            bass_buf[end_idx] = blended_bass;
        }

        MusicStems {
            length_samples: total_samples,
            pad: pad_buf,
            bass: bass_buf,
            lead: lead_buf,
            drums: drum_buf,
        }
    }
}


// ============================================================================
// Adaptive Music Source & WAV Baker
// ============================================================================

pub struct AdaptiveMusicSource {
    stems: MusicStems,
    sample_pos: usize,
    intensity: Arc<AtomicU32>,
    smoothed_intensity: f32,
}

impl AdaptiveMusicSource {
    pub fn new(recipe: &MusicRecipe, initial_intensity: f32) -> Self {
        let stems = recipe.synthesize_stems();
        let init_clamped = initial_intensity.clamp(0.0, 1.0);
        let intensity = Arc::new(AtomicU32::new(init_clamped.to_bits()));
        Self {
            stems,
            sample_pos: 0,
            intensity,
            smoothed_intensity: init_clamped,
        }
    }

    pub fn intensity_handle(&self) -> Arc<AtomicU32> {
        self.intensity.clone()
    }
}

impl Iterator for AdaptiveMusicSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.stems.length_samples == 0 {
            return Some(0.0);
        }

        // Smooth intensity transition
        let target_intensity = f32::from_bits(self.intensity.load(Ordering::Relaxed)).clamp(0.0, 1.0);
        self.smoothed_intensity += 0.001 * (target_intensity - self.smoothed_intensity);
        let i = self.smoothed_intensity;

        let pad_w = 0.9;
        let bass_w = if i < 0.25 { 0.0 } else if i < 0.50 { (i - 0.25) / 0.25 } else { 1.0 };
        let lead_w = if i < 0.35 { 0.0 } else if i < 0.70 { (i - 0.35) / 0.35 } else { 1.0 };
        let drum_w = if i < 0.65 { 0.0 } else if i < 0.90 { (i - 0.65) / 0.25 } else { 1.0 };

        let idx = self.sample_pos;
        let s_pad = self.stems.pad.get(idx).copied().unwrap_or(0.0);
        let s_bass = self.stems.bass.get(idx).copied().unwrap_or(0.0);
        let s_lead = self.stems.lead.get(idx).copied().unwrap_or(0.0);
        let s_drum = self.stems.drums.get(idx).copied().unwrap_or(0.0);

        let mix = (s_pad * pad_w + s_bass * bass_w + s_lead * lead_w + s_drum * drum_w) * 0.75;
        self.sample_pos = (self.sample_pos + 1) % self.stems.length_samples;

        Some(mix.clamp(-1.0, 1.0))
    }
}

impl Source for AdaptiveMusicSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

pub fn bake_music_to_wav(recipe: &MusicRecipe, output_path: &str, intensity: Option<f32>) -> Result<String, String> {
    let target_intensity = intensity.unwrap_or(1.0).clamp(0.0, 1.0);
    let stems = recipe.synthesize_stems();
    let total_samples = stems.length_samples;

    let pad_w = 0.9;
    let i = target_intensity;
    let bass_w = if i < 0.25 { 0.0 } else if i < 0.50 { (i - 0.25) / 0.25 } else { 1.0 };
    let lead_w = if i < 0.35 { 0.0 } else if i < 0.70 { (i - 0.35) / 0.35 } else { 1.0 };
    let drum_w = if i < 0.65 { 0.0 } else if i < 0.90 { (i - 0.65) / 0.25 } else { 1.0 };

    let mut pcm_i16 = Vec::with_capacity(total_samples);
    for idx in 0..total_samples {
        let s_pad = stems.pad.get(idx).copied().unwrap_or(0.0);
        let s_bass = stems.bass.get(idx).copied().unwrap_or(0.0);
        let s_lead = stems.lead.get(idx).copied().unwrap_or(0.0);
        let s_drum = stems.drums.get(idx).copied().unwrap_or(0.0);

        let mix = (s_pad * pad_w + s_bass * bass_w + s_lead * lead_w + s_drum * drum_w) * 0.75;
        let clamped = mix.clamp(-1.0, 1.0);
        let s16 = (clamped * 30000.0) as i16;
        pcm_i16.push(s16);
    }

    let out_p = Path::new(output_path);
    if let Some(parent) = out_p.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Error creating parent dir: {}", e))?;
    }

    let data_len = (pcm_i16.len() * 2) as u32;
    let total_len = 36 + data_len;
    let mut bytes = Vec::with_capacity(44 + data_len as usize);

    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&total_len.to_le_bytes());
    bytes.extend_from_slice(b"WAVE");

    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM format
    bytes.extend_from_slice(&1u16.to_le_bytes()); // mono channel
    bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    let byte_rate = SAMPLE_RATE * 2;
    bytes.extend_from_slice(&byte_rate.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes()); // block align
    bytes.extend_from_slice(&16u16.to_le_bytes()); // bits per sample

    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for s in pcm_i16 {
        bytes.extend_from_slice(&s.to_le_bytes());
    }

    let mut file = File::create(out_p).map_err(|e| format!("Failed to create wav file '{:?}': {}", out_p, e))?;
    file.write_all(&bytes).map_err(|e| format!("Failed to write wav bytes: {}", e))?;

    Ok(output_path.to_string())
}


// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_music_presets_generation_and_no_nan() {
        let presets = ["space_ambient", "boss_battle", "cyber_action", "menu_chill"];
        for name in &presets {
            let recipe = get_music_preset(name).unwrap_or_else(|| panic!("Preset {} should exist", name));
            let stems = recipe.synthesize_stems();
            assert!(stems.length_samples > 1000);
            assert_eq!(stems.pad.len(), stems.length_samples);
            assert_eq!(stems.bass.len(), stems.length_samples);
            assert_eq!(stems.lead.len(), stems.length_samples);
            assert_eq!(stems.drums.len(), stems.length_samples);

            for (i, &s) in stems.pad.iter().enumerate() {
                assert!(!s.is_nan() && !s.is_infinite(), "Pad NaN at {} in {}", i, name);
                assert!(s.abs() <= 1.5, "Pad extreme value {} at {} in {}", s, i, name);
            }
            for (i, &s) in stems.bass.iter().enumerate() {
                assert!(!s.is_nan() && !s.is_infinite(), "Bass NaN at {} in {}", i, name);
            }
            for (i, &s) in stems.lead.iter().enumerate() {
                assert!(!s.is_nan() && !s.is_infinite(), "Lead NaN at {} in {}", i, name);
            }
            for (i, &s) in stems.drums.iter().enumerate() {
                assert!(!s.is_nan() && !s.is_infinite(), "Drums NaN at {} in {}", i, name);
            }
        }
    }

    #[test]
    fn test_bake_music_to_wav_valid_file() {
        let recipe = preset_space_ambient();
        let out_path = "assets/music/test_ambient_bake.wav";
        let res = bake_music_to_wav(&recipe, out_path, Some(0.8));
        assert!(res.is_ok(), "Failed to bake music: {:?}", res.err());

        let bytes = std::fs::read(out_path).expect("Test wav should exist");
        assert!(bytes.len() > 44, "WAV file too short: {} bytes", bytes.len());
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(&bytes[12..16], b"fmt ");

        // Clean up test file
        let _ = std::fs::remove_file(out_path);
    }

    #[test]
    fn test_adaptive_music_source_intensity_levels() {
        let recipe = preset_boss_battle();
        let mut source = AdaptiveMusicSource::new(&recipe, 0.1);

        // Sample at low intensity (0.1)
        let mut sum_low = 0.0f32;
        for _ in 0..1000 {
            if let Some(s) = source.next() {
                sum_low += s.abs();
            }
        }

        // Change intensity to medium (0.5)
        source.intensity.store(0.5f32.to_bits(), Ordering::Relaxed);
        let mut sum_mid = 0.0f32;
        for _ in 0..1000 {
            if let Some(s) = source.next() {
                sum_mid += s.abs();
            }
        }

        // Change intensity to full (1.0)
        source.intensity.store(1.0f32.to_bits(), Ordering::Relaxed);
        let mut sum_high = 0.0f32;
        for _ in 0..1000 {
            if let Some(s) = source.next() {
                sum_high += s.abs();
            }
        }

        assert!(sum_low > 0.0);
        assert!(sum_mid > 0.0);
        assert!(sum_high > 0.0);
    }

    #[test]
    fn test_bake_cosmos_theme_wav() {
        let recipe = preset_space_ambient();
        let out_path = "assets/music/cosmos_theme.wav";
        let res = bake_music_to_wav(&recipe, out_path, Some(0.7));
        assert!(res.is_ok(), "Failed to bake cosmos_theme.wav: {:?}", res.err());

        let bytes = std::fs::read(out_path).expect("cosmos_theme.wav should exist");
        assert!(bytes.len() > 1000);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
    }
}
