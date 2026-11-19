//! Honeydew's native instrument adapter for Potion.
//! The initial factory patch uses a sine table; user wavetable loading and the editor are separate follow-up work.

use std::sync::Arc;

use crate::ProcessContext;
use crate::instrument::Instrument;
use crate::potion::{PotionEngine, PotionParams};
use crate::wavetable::Wavetable;
/// Potion as a first-party Honeydew rack instrument.
pub struct PotionInstrument {
    engine: PotionEngine,
    params: PotionParams,
}

impl PotionInstrument {
    /// Builds its factory sine table on the app thread, never in the audio callback.
    pub fn new(sample_rate: f32) -> Self {
        let size = 2048usize;
        let sine = (0..size)
            .map(|i| (std::f32::consts::TAU * i as f32 / size as f32).sin())
            .collect::<Vec<_>>();
        let triangle = (0..size)
            .map(|i| {
                let phase = std::f32::consts::TAU * i as f32 / size as f32;
                (0..32)
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
                (1..128)
                    .map(|h| (phase * h as f32).sin() / h as f32)
                    .sum::<f32>()
                    * (2.0 / std::f32::consts::PI)
            })
            .collect::<Vec<_>>();
        let square = (0..size)
            .map(|i| {
                let phase = std::f32::consts::TAU * i as f32 / size as f32;
                (0..64)
                    .map(|h| (phase * (2 * h + 1) as f32).sin() / (2 * h + 1) as f32)
                    .sum::<f32>()
                    * (4.0 / std::f32::consts::PI)
            })
            .collect::<Vec<_>>();
        let table = Arc::new(
            Wavetable::new(vec![sine, triangle, saw, square])
                .expect("built-in Potion factory tables are valid"),
        );
        Self {
            engine: PotionEngine::new(sample_rate, std::array::from_fn(|_| table.clone())),
            params: PotionParams::default(),
        }
    }

    fn seconds(value: f32, maximum: f32) -> f32 {
        0.001 * (maximum / 0.001).powf(value.clamp(0.0, 1.0))
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
            _ => {}
        }
    }

    fn note_on(&mut self, key: u8, velocity: f32, pan: f32) {
        self.engine.note_on_with_pan(key, velocity, pan);
    }

    fn note_off(&mut self, key: u8) {
        self.engine.note_off(key);
    }

    fn all_notes_off(&mut self) {
        self.engine.all_notes_off();
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

    #[test]
    fn factory_instrument_renders_and_obeys_notes() {
        let mut instrument = PotionInstrument::new(48_000.0);
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
        let mut instrument = PotionInstrument::new(48_000.0);
        instrument.set_parameter(1, 0.2);
        instrument.set_parameter(2, 0.8);
        assert!((instrument.params.level[0] - 0.2).abs() < 1.0e-6);
        assert!((instrument.params.level[1] - 0.8).abs() < 1.0e-6);
        assert_eq!(crate::potion::OSCILLATORS, 3);
    }
}
