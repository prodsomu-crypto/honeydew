//! Potion's first playable wavetable voice engine.
//! Honeydew's native rack-instrument adapter is in `potion_instrument`; the custom editor is still to come.
//! All voice storage is allocated in `PotionEngine::new`; rendering does not allocate or lock.

use std::sync::Arc;

use crate::wavetable::{Wavetable, WavetableOscillator};

pub const MAX_VOICES: usize = 32;
pub const OSCILLATORS: usize = 3;

#[derive(Clone, Copy, Debug)]
pub struct PotionParams {
    pub enabled: [bool; OSCILLATORS],
    pub level: [f32; OSCILLATORS],
    pub pan: [f32; OSCILLATORS],
    pub semitones: [f32; OSCILLATORS],
    pub table_position: [f32; OSCILLATORS],
    pub attack_seconds: f32,
    pub release_seconds: f32,
    pub master: f32,
}

impl Default for PotionParams {
    fn default() -> Self {
        Self {
            enabled: [true; OSCILLATORS],
            level: [0.7, 0.35, 0.2],
            pan: [-0.35, 0.35, 0.0],
            semitones: [0.0, -12.0, -24.0],
            table_position: [0.0; OSCILLATORS],
            attack_seconds: 0.01,
            release_seconds: 0.18,
            master: 0.7,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Oscillator {
    phase: WavetableOscillator,
}

#[derive(Clone, Copy, Debug, Default)]
struct Voice {
    active: bool,
    releasing: bool,
    note: u8,
    velocity: f32,
    pan: f32,
    envelope: f32,
    oscillators: [Oscillator; OSCILLATORS],
}

pub struct PotionEngine {
    voices: Box<[Voice]>,
    tables: [Arc<Wavetable>; OSCILLATORS],
    sample_rate: f32,
}

impl PotionEngine {
    pub fn new(sample_rate: f32, tables: [Arc<Wavetable>; OSCILLATORS]) -> Self {
        Self {
            voices: vec![Voice::default(); MAX_VOICES].into_boxed_slice(),
            tables,
            sample_rate: sample_rate.max(1.0),
        }
    }

    pub fn reset(&mut self) {
        self.voices.fill(Voice::default());
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        if sample_rate.is_finite() && sample_rate > 1.0 {
            self.sample_rate = sample_rate;
        }
    }

    pub fn note_on(&mut self, note: u8, velocity: f32) {
        self.note_on_with_pan(note, velocity, 0.5);
    }

    pub fn note_on_with_pan(&mut self, note: u8, velocity: f32, pan: f32) {
        let slot = self
            .voices
            .iter()
            .position(|v| !v.active)
            .or_else(|| self.voices.iter().position(|v| v.releasing))
            .unwrap_or(0);
        let voice = &mut self.voices[slot];
        *voice = Voice {
            active: true,
            releasing: false,
            note: note.min(127),
            velocity: velocity.clamp(0.0, 1.0),
            pan: (pan.clamp(0.0, 1.0) - 0.5) * 2.0,
            envelope: 0.0,
            oscillators: [Oscillator::default(); OSCILLATORS],
        };
    }

    pub fn note_off(&mut self, note: u8) {
        for voice in self
            .voices
            .iter_mut()
            .filter(|v| v.active && v.note == note)
        {
            voice.releasing = true;
        }
    }

    pub fn all_notes_off(&mut self) {
        for voice in self.voices.iter_mut().filter(|v| v.active) {
            voice.releasing = true;
        }
    }

    /// Adds one rendered block to the supplied stereo buffers. Buffers must have equal lengths.
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32], params: &PotionParams) {
        let len = left.len().min(right.len());
        let sr = self.sample_rate;
        let attack = (params.attack_seconds.max(0.001) * sr).recip();
        let release = (params.release_seconds.max(0.001) * sr).recip();
        let master = params.master.clamp(0.0, 1.0) * 0.22;
        for voice in self.voices.iter_mut().filter(|v| v.active) {
            let base_hz = 440.0 * 2.0f32.powf((f32::from(voice.note) - 69.0) / 12.0);
            for i in 0..len {
                if voice.releasing {
                    voice.envelope = (voice.envelope - release).max(0.0);
                    if voice.envelope <= 0.0 {
                        voice.active = false;
                        break;
                    }
                } else {
                    voice.envelope = (voice.envelope + attack).min(1.0);
                }
                let mut l = 0.0;
                let mut r = 0.0;
                for osc in 0..OSCILLATORS {
                    if !params.enabled[osc] {
                        continue;
                    }
                    let hz = base_hz * 2.0f32.powf(params.semitones[osc].clamp(-48.0, 48.0) / 12.0);
                    let sample = voice.oscillators[osc].phase.next_sample(
                        &self.tables[osc],
                        hz,
                        sr,
                        params.table_position[osc],
                    ) * params.level[osc].clamp(0.0, 1.0);
                    let pan = (params.pan[osc] + voice.pan).clamp(-1.0, 1.0);
                    let lg = (1.0 - pan).sqrt() * 0.70710677;
                    let rg = (1.0 + pan).sqrt() * 0.70710677;
                    l += sample * lg;
                    r += sample * rg;
                }
                let gain = voice.envelope * voice.velocity * master;
                left[i] += l * gain;
                right[i] += r * gain;
            }
        }
    }

    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> Arc<Wavetable> {
        let frame = (0..256)
            .map(|i| (std::f32::consts::TAU * i as f32 / 256.0).sin())
            .collect();
        Arc::new(Wavetable::new(vec![frame]).unwrap())
    }

    #[test]
    fn renders_a_note_and_releases_it() {
        let t = table();
        let mut engine = PotionEngine::new(48_000.0, [t.clone(), t.clone(), t]);
        engine.note_on(69, 1.0);
        let mut left = vec![0.0; 512];
        let mut right = vec![0.0; 512];
        engine.render(&mut left, &mut right, &PotionParams::default());
        assert!(left.iter().chain(&right).any(|v| v.abs() > 1.0e-5));
        assert_eq!(engine.active_voices(), 1);
        engine.note_off(69);
        let mut tail = vec![0.0; 48_000];
        let mut tail_r = tail.clone();
        let mut params = PotionParams::default();
        params.release_seconds = 0.05;
        engine.render(&mut tail, &mut tail_r, &params);
        assert_eq!(engine.active_voices(), 0);
    }

    #[test]
    fn polyphony_is_bounded() {
        let t = table();
        let mut engine = PotionEngine::new(48_000.0, [t.clone(), t.clone(), t]);
        for note in 0..(MAX_VOICES as u8 + 4) {
            engine.note_on(note, 1.0);
        }
        assert_eq!(engine.active_voices(), MAX_VOICES);
    }
}
