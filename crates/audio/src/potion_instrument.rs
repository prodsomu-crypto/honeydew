//! Honeydew's native instrument adapter for Potion.

use std::sync::Arc;

use crate::ProcessContext;
use crate::instrument::Instrument;
use crate::potion::{PotionEngine, PotionParams};
use crate::three_osc::osc::Wave;
use crate::wavetable::Wavetable;

/// Potion as a first-party Honeydew rack instrument.
pub struct PotionInstrument {
    engine: PotionEngine,
    params: PotionParams,
}

impl PotionInstrument {
    /// Builds factory and optional imported tables on the app thread, never in the audio callback.
    pub fn new(sample_rate: f32, custom_wave: Option<Arc<Wave>>) -> Self {
        let size = 4096usize;
        let sine = (0..size)
            .map(|i| (std::f32::consts::TAU * i as f32 / size as f32).sin())
            .collect::<Vec<_>>();
        let triangle = (0..size)
            .map(|i| {
                let phase = std::f32::consts::TAU * i as f32 / size as f32;
                (0..64)
                    .map(|h| {
                        let n = (2 * h + 1) as f32;
                        let sign = if h % 2 == 0 { 1.0 } else { -1.0 };
                        sign * (phase * n).sin() / (n * n)
                    })
                    .sum::<f32>()
                    * (8.0 / std::f32::consts::PI.powi(2))
            })
            .collect::<Vec<_>>();
        let saw = (0..size)
            .map(|i| {
                let phase = std::f32::consts::TAU * i as f32 / size as f32;
                (1..256)
                    .map(|h| (phase * h as f32).sin() / h as f32)
                    .sum::<f32>()
                    * (2.0 / std::f32::consts::PI)
            })
            .collect::<Vec<_>>();
        let square = (0..size)
            .map(|i| {
                let phase = std::f32::consts::TAU * i as f32 / size as f32;
                (0..128)
                    .map(|h| (phase * (2 * h + 1) as f32).sin() / (2 * h + 1) as f32)
                    .sum::<f32>()
                    * (4.0 / std::f32::consts::PI)
            })
            .collect::<Vec<_>>();
        let mut frames = vec![sine, triangle, saw, square];
        if let Some(wave) = custom_wave {
            let raw = wave.raw();
            if raw.len() >= size {
                // A 3x Osc custom WAV is already resampled to a 16384-point cycle off the audio thread.
                // Decimation to Potion's 4096-point table keeps the imported cycle periodic and bounded.
                let mut custom: Vec<f32> = raw.iter().step_by(raw.len() / size).take(size).copied().collect();
                if custom.len() == size {
                    let mean = custom.iter().sum::<f32>() / custom.len() as f32;
                    for sample in &mut custom {
                        *sample -= mean;
                    }
                    let peak = custom.iter().fold(0.0f32, |peak, sample| peak.max(sample.abs()));
                    if peak > 1.0e-6 {
                        custom.iter_mut().for_each(|sample| *sample /= peak);
                    }
                    frames.push(custom);
                }
            }
        }
        let table = Arc::new(
            Wavetable::new(frames).expect("built-in Potion factory tables are valid"),
        );
        Self {
            engine: PotionEngine::new(sample_rate, std::array::from_fn(|_| table.clone())),
            params: PotionParams::default(),
        }
    }

    fn seconds(value: f32, maximum: f32) -> f32 {
        0.001 * (maximum / 0.001).powf(value.clamp(0.0, 1.0))
    }

    fn rate(value: f32) -> f32 {
        0.03 * (20.0f32 / 0.03).powf(value.clamp(0.0, 1.0))
    }
}

impl Instrument for PotionInstrument {
    fn set_parameter(&mut self, index: usize, value: f32) {
        if !value.is_finite() {
            return;
        }
        let value = value.clamp(0.0, 1.0);
        match index {
            1..=3 => self.params.level[index - 1] = value,
            4..=6 => self.params.pan[index - 4] = value * 2.0 - 1.0,
            7..=9 => self.params.semitones[index - 7] = value * 96.0 - 48.0,
            10..=12 => self.params.table_position[index - 10] = value,
            13..=15 => self.params.enabled[index - 13] = value >= 0.5,
            16 => self.params.attack_seconds = Self::seconds(value, 8.0),
            17 => self.params.release_seconds = Self::seconds(value, 12.0),
            18 => self.params.master = value,
            19 => self.params.filter_cutoff = value,
            20 => self.params.sustain = value,
            21 => self.params.decay_seconds = Self::seconds(value, 8.0),
            22 => self.params.filter_resonance = value,
            23 => self.params.filter_mode = (value * 2.0).round() as u8,
            24 => self.params.unison = 1 + (value * 6.0).round() as usize,
            25 => self.params.detune_cents = value * 50.0,
            26 => self.params.stereo_spread = value,
            27 => self.params.glide_seconds = Self::seconds(value, 2.0),
            28 => self.params.voice_mode = (value * 2.0).round() as u8,
            29 => self.params.lfo_rate[0] = Self::rate(value),
            30 => self.params.lfo_amount[0] = value,
            31 => self.params.lfo_destination[0] = (value * 2.0).round() as u8,
            32 => self.params.lfo_wave[0] = (value * 2.0).round() as u8,
            33 => self.params.lfo_rate[1] = Self::rate(value),
            34 => self.params.lfo_amount[1] = value,
            35 => self.params.lfo_destination[1] = (value * 2.0).round() as u8,
            36 => self.params.lfo_wave[1] = (value * 2.0).round() as u8,
            37 => self.params.fm_amount = value,
            38 => self.params.ring_amount = value,
            39 => self.params.warp_amount = value,
            40 => self.params.warp_mode = (value * 3.0).round() as u8,
            41 => self.params.sub_level = value,
            42 => self.params.noise_level = value,
            43 => self.params.velocity_sensitivity = value,
            44 => self.params.filter_env_amount = value,
            45 => self.params.polyphony = 1 + (value * 31.0).round() as usize,
            47..=58 => {
                let route = (index - 47) / 3;
                match (index - 47) % 3 {
                    0 => self.params.mod_source[route] = (value * 5.0).round() as u8,
                    1 => self.params.mod_destination[route] = (value * 5.0).round() as u8,
                    _ => self.params.mod_amount[route] = value * 2.0 - 1.0,
                }
            }
            _ => {}
        }
    }

    fn note_on(&mut self, key: u8, velocity: f32, pan: f32) {
        self.engine
            .note_on_with_params(key, velocity, pan, &self.params);
    }

    fn note_off(&mut self, key: u8) {
        self.engine.note_off(key);
    }

    fn all_notes_off(&mut self) {
        self.engine.all_notes_off();
    }

    fn set_controller(&mut self, index: usize, value: f32) {
        if !value.is_finite() {
            return;
        }
        let value = value.clamp(0.0, 1.0);
        match index {
            4 => self.params.pitch_bend = value * 4.0 - 2.0,
            5 => self.params.aftertouch = value,
            8 => self.params.mod_wheel = value,
            _ => {}
        }
    }

    fn set_slide(&mut self, pitch: f32, pan: f32, velocity: f32) {
        self.engine.set_slide(pitch, pan, velocity);
    }

    fn reset(&mut self) {
        self.engine.reset();
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32], context: ProcessContext) {
        self.engine.set_sample_rate(context.sample_rate);
        self.engine.render(left, right, &self.params);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make() -> PotionInstrument {
        PotionInstrument::new(48_000.0, None)
    }

    #[test]
    fn factory_instrument_renders_and_obeys_notes() {
        let mut instrument = make();
        let (mut left, mut right) = (vec![0.0; 1024], vec![0.0; 1024]);
        instrument.note_on(69, 1.0, 0.5);
        instrument.process(
            &mut left,
            &mut right,
            ProcessContext {
                sample_rate: 48_000.0,
                tempo_bpm: 120.0,
                start_beat: 0.0,
                playing: false,
            },
        );
        assert!(
            left.iter()
                .chain(&right)
                .any(|sample| sample.abs() > 1.0e-5)
        );
        instrument.note_off(69);
        instrument.reset();
    }

    #[test]
    fn oscillator_parameter_groups_are_independent() {
        let mut instrument = make();
        instrument.set_parameter(1, 0.2);
        instrument.set_parameter(2, 0.8);
        assert!((instrument.params.level[0] - 0.2).abs() < 1.0e-6);
        assert!((instrument.params.level[1] - 0.8).abs() < 1.0e-6);
        instrument.set_parameter(24, 1.0);
        instrument.set_parameter(40, 1.0);
        assert_eq!(instrument.params.unison, 7);
        assert_eq!(instrument.params.warp_mode, 3);
        instrument.set_controller(4, 1.0);
        instrument.set_controller(5, 0.75);
        instrument.set_controller(8, 0.25);
        assert_eq!(instrument.params.pitch_bend, 2.0);
        assert_eq!(instrument.params.aftertouch, 0.75);
        assert_eq!(instrument.params.mod_wheel, 0.25);
    }
}
