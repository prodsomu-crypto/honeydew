//! Potion's polyphonic wavetable voice engine.
//! Tables are immutable and prepared off the audio thread. Rendering performs no allocation, locking or I/O.

use std::sync::Arc;

use crate::wavetable::{Wavetable, WavetableOscillator};

pub const MAX_VOICES: usize = 32;
pub const OSCILLATORS: usize = 3;
pub const MAX_UNISON: usize = 7;

#[derive(Clone, Copy, Debug)]
pub struct PotionParams {
    pub enabled: [bool; OSCILLATORS],
    pub level: [f32; OSCILLATORS],
    pub pan: [f32; OSCILLATORS],
    pub semitones: [f32; OSCILLATORS],
    pub table_position: [f32; OSCILLATORS],
    pub attack_seconds: f32,
    pub decay_seconds: f32,
    pub release_seconds: f32,
    pub master: f32,
    pub sustain: f32,
    pub filter_cutoff: f32,
    pub filter_resonance: f32,
    /// 0 = low-pass, 1 = band-pass, 2 = high-pass.
    pub filter_mode: u8,
    pub unison: usize,
    pub detune_cents: f32,
    pub stereo_spread: f32,
    pub glide_seconds: f32,
    /// 0 = poly, 1 = mono, 2 = legato.
    pub voice_mode: u8,
    pub lfo_rate: [f32; 2],
    pub lfo_amount: [f32; 2],
    /// 0 = pitch, 1 = filter, 2 = pan.
    pub lfo_destination: [u8; 2],
    /// 0 = sine, 1 = triangle, 2 = square.
    pub lfo_wave: [u8; 2],
    pub fm_amount: f32,
    pub ring_amount: f32,
    pub warp_amount: f32,
    /// 0 = off, 1 = bend, 2 = fold, 3 = hard-sync-like phase remap.
    pub warp_mode: u8,
    pub sub_level: f32,
    pub noise_level: f32,
    pub velocity_sensitivity: f32,
    pub filter_env_amount: f32,
    pub polyphony: usize,
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
            decay_seconds: 0.25,
            release_seconds: 0.18,
            master: 0.7,
            sustain: 0.8,
            filter_cutoff: 1.0,
            filter_resonance: 0.1,
            filter_mode: 0,
            unison: 1,
            detune_cents: 8.0,
            stereo_spread: 0.7,
            glide_seconds: 0.0,
            voice_mode: 0,
            lfo_rate: [1.0, 0.23],
            lfo_amount: [0.0; 2],
            lfo_destination: [0, 1],
            lfo_wave: [0, 1],
            fm_amount: 0.0,
            ring_amount: 0.0,
            warp_amount: 0.0,
            warp_mode: 0,
            sub_level: 0.0,
            noise_level: 0.0,
            velocity_sensitivity: 0.7,
            filter_env_amount: 0.0,
            polyphony: MAX_VOICES,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Voice {
    active: bool,
    note: u8,
    target_pitch: f32,
    pitch: f32,
    velocity: f32,
    pan: f32,
    envelope: f32,
    /// 0 attack, 1 decay, 2 sustain, 3 release.
    amp_stage: u8,
    filter_ic1_left: f32,
    filter_ic2_left: f32,
    filter_ic1_right: f32,
    filter_ic2_right: f32,
    oscillators: [[WavetableOscillator; MAX_UNISON]; OSCILLATORS],
    sub_phase: f64,
    noise_seed: u32,
    lfo_phase: [f64; 2],
}

pub struct PotionEngine {
    voices: Box<[Voice]>,
    tables: [Arc<Wavetable>; OSCILLATORS],
    sample_rate: f32,
    last_pitch: f32,
}

impl PotionEngine {
    pub fn new(sample_rate: f32, tables: [Arc<Wavetable>; OSCILLATORS]) -> Self {
        Self {
            voices: vec![Voice::default(); MAX_VOICES].into_boxed_slice(),
            tables,
            sample_rate: sample_rate.max(1.0),
            last_pitch: 60.0,
        }
    }

    pub fn reset(&mut self) {
        self.voices.fill(Voice::default());
        self.last_pitch = 60.0;
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
        self.note_on_with_params(note, velocity, pan, &PotionParams::default());
    }

    pub fn note_on_with_params(
        &mut self,
        note: u8,
        velocity: f32,
        pan: f32,
        params: &PotionParams,
    ) {
        let target = f32::from(note.min(127));
        let mono = params.voice_mode != 0;
        let legato = params.voice_mode >= 2;
        let existing = if mono {
            self.voices.iter().position(|voice| voice.active)
        } else {
            None
        };
        let slot = existing
            .or_else(|| self.voices.iter().position(|voice| !voice.active))
            .or_else(|| self.voices.iter().position(|voice| voice.amp_stage == 3))
            .unwrap_or(0);
        let voice = &mut self.voices[slot];
        let continuing = existing.is_some() && legato;
        let start_pitch = if existing.is_some() {
            voice.pitch
        } else if mono && params.glide_seconds > 0.0 {
            self.last_pitch
        } else {
            target
        };
        if continuing {
            voice.note = note.min(127);
            voice.target_pitch = target;
            voice.velocity = velocity.clamp(0.0, 1.0);
            voice.pan = (pan.clamp(0.0, 1.0) - 0.5) * 2.0;
        } else {
            *voice = Voice {
                active: true,
                note: note.min(127),
                target_pitch: target,
                pitch: start_pitch,
                velocity: velocity.clamp(0.0, 1.0),
                pan: (pan.clamp(0.0, 1.0) - 0.5) * 2.0,
                amp_stage: 0,
                noise_seed: (u32::from(note).wrapping_mul(0x9e37_79b9) ^ 0xa341_316c).max(1),
                ..Voice::default()
            };
            for osc in 0..OSCILLATORS {
                for unison in 0..MAX_UNISON {
                    voice.oscillators[osc][unison]
                        .set_phase(unison as f64 / MAX_UNISON as f64);
                }
            }
        }
        self.last_pitch = target;
    }

    pub fn note_off(&mut self, note: u8) {
        if let Some(voice) = self
            .voices
            .iter_mut()
            .find(|voice| voice.active && voice.note == note)
        {
            voice.amp_stage = 3;
        }
    }

    pub fn all_notes_off(&mut self) {
        for voice in self.voices.iter_mut().filter(|voice| voice.active) {
            voice.amp_stage = 3;
        }
    }

    /// Adds one rendered block to the supplied stereo buffers. Buffers must have equal lengths.
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32], params: &PotionParams) {
        let len = left.len().min(right.len());
        let sr = self.sample_rate;
        let attack_step = (params.attack_seconds.max(0.001) * sr).recip();
        let decay_step = (params.decay_seconds.max(0.001) * sr).recip();
        let release_step = (params.release_seconds.max(0.001) * sr).recip();
        let sustain = params.sustain.clamp(0.0, 1.0);
        let master = params.master.clamp(0.0, 1.0) * 0.18;
        let unison = params.unison.clamp(1, MAX_UNISON);
        let detune = params.detune_cents.clamp(0.0, 50.0);
        let spread = params.stereo_spread.clamp(0.0, 1.0);
        let glide_step = if params.glide_seconds > 0.001 {
            (sr * params.glide_seconds).recip().clamp(0.00001, 1.0)
        } else {
            1.0
        };
        let fm_amount = params.fm_amount.clamp(0.0, 1.0);
        let ring_amount = params.ring_amount.clamp(0.0, 1.0);
        let warp_amount = params.warp_amount.clamp(0.0, 1.0);
        let velocity_sensitivity = params.velocity_sensitivity.clamp(0.0, 1.0);

        for voice in self.voices.iter_mut().filter(|voice| voice.active) {
            for i in 0..len {
                match voice.amp_stage {
                    0 => {
                        voice.envelope = (voice.envelope + attack_step).min(1.0);
                        if voice.envelope >= 1.0 {
                            voice.amp_stage = 1;
                        }
                    }
                    1 => {
                        voice.envelope =
                            (voice.envelope - decay_step * (1.0 - sustain)).max(sustain);
                        if voice.envelope <= sustain + 1.0e-6 {
                            voice.envelope = sustain;
                            voice.amp_stage = 2;
                        }
                    }
                    2 => voice.envelope = sustain,
                    3 => {
                        voice.envelope = (voice.envelope - release_step).max(0.0);
                        if voice.envelope <= 0.0 {
                            voice.active = false;
                            break;
                        }
                    }
                    _ => voice.amp_stage = 3,
                }

                voice.pitch += (voice.target_pitch - voice.pitch) * glide_step;
                let base_hz = 440.0 * 2.0f32.powf((voice.pitch - 69.0) / 12.0);
                let lfo_a = lfo_value(voice.lfo_phase[0], params.lfo_wave[0]);
                let lfo_b = lfo_value(voice.lfo_phase[1], params.lfo_wave[1]);
                for lfo in 0..2 {
                    voice.lfo_phase[lfo] = (voice.lfo_phase[lfo]
                        + f64::from(params.lfo_rate[lfo].clamp(0.01, 30.0) / sr))
                    .fract();
                }
                let mut pitch_mod = 0.0;
                let mut filter_mod = 0.0;
                let mut pan_mod = 0.0;
                for (value, amount, destination) in [
                    (lfo_a, params.lfo_amount[0], params.lfo_destination[0]),
                    (lfo_b, params.lfo_amount[1], params.lfo_destination[1]),
                ] {
                    let depth = amount.clamp(0.0, 1.0);
                    match destination {
                        0 => pitch_mod += value * depth * 12.0,
                        1 => filter_mod += value * depth * 0.45,
                        _ => pan_mod += value * depth * 0.6,
                    }
                }

                let mut osc_out = [0.0f32; OSCILLATORS];
                let mut out_l = 0.0f32;
                let mut out_r = 0.0f32;
                // Oscillator 2 is evaluated first so it can modulate oscillator 1's frequency.
                for osc in [1usize, 0, 2] {
                    if !params.enabled[osc] {
                        continue;
                    }
                    let mut mono = 0.0;
                    for unison_index in 0..unison {
                        let position = if unison > 1 {
                            unison_index as f32 / (unison - 1) as f32
                        } else {
                            0.5
                        };
                        let cents = (position * 2.0 - 1.0) * detune;
                        let mut hz = base_hz
                            * 2.0f32.powf(
                                (params.semitones[osc].clamp(-48.0, 48.0) + pitch_mod + cents / 100.0)
                                    / 12.0,
                            );
                        if osc == 0 {
                            hz *= 2.0f32.powf(osc_out[1] * fm_amount * 2.0);
                        }
                        let phase = voice.oscillators[osc][unison_index].phase();
                        if warp_amount > 0.0 {
                            let warped = match params.warp_mode.min(3) {
                                1 => phase + (std::f64::consts::TAU * phase).sin()
                                    * f64::from(warp_amount) * 0.12,
                                3 => (phase * f64::from(1.0 + warp_amount * 7.0)).fract(),
                                _ => phase,
                            };
                            voice.oscillators[osc][unison_index].set_phase(warped);
                        }
                        let mut sample = voice.oscillators[osc][unison_index].next_sample(
                            &self.tables[osc],
                            hz.clamp(1.0, sr * 0.45),
                            sr,
                            params.table_position[osc],
                        );
                        if params.warp_mode == 2 && warp_amount > 0.0 {
                            let drive = 1.0 + warp_amount * 7.0;
                            sample = (sample * drive).sin() / drive.sqrt();
                        }
                        let pan = (params.pan[osc]
                            + voice.pan
                            + (position * 2.0 - 1.0) * spread * 0.75
                            + pan_mod)
                            .clamp(-1.0, 1.0);
                        let left_gain = (1.0 - pan).sqrt() * 0.70710677;
                        let right_gain = (1.0 + pan).sqrt() * 0.70710677;
                        let gain = params.level[osc].clamp(0.0, 1.0) / unison as f32;
                        out_l += sample * gain * left_gain;
                        out_r += sample * gain * right_gain;
                        mono += sample * gain;
                    }
                    osc_out[osc] = mono;
                }
                let ring = (1.0 - ring_amount) + ring_amount * osc_out[1];
                out_l += osc_out[0] * 0.0; // stereo contribution is already accumulated above.
                out_r += osc_out[0] * 0.0;
                if ring_amount > 0.0 {
                    // Blend a ring-modulated first oscillator into the dry signal without another oscillator pass.
                    let dry = osc_out[0];
                    let wet = dry * ring;
                    let delta = (wet - dry) * ring_amount;
                    out_l += delta * 0.70710677;
                    out_r += delta * 0.70710677;
                }

                voice.sub_phase =
                    (voice.sub_phase + f64::from((base_hz * 0.5 / sr).clamp(0.0, 0.49))).fract();
                let sub = (std::f64::consts::TAU * voice.sub_phase).sin() as f32
                    * params.sub_level.clamp(0.0, 1.0);
                voice.noise_seed ^= voice.noise_seed << 13;
                voice.noise_seed ^= voice.noise_seed >> 17;
                voice.noise_seed ^= voice.noise_seed << 5;
                let noise = (voice.noise_seed as f32 / u32::MAX as f32 * 2.0 - 1.0)
                    * params.noise_level.clamp(0.0, 1.0);
                let dry_l = out_l + (sub + noise) * 0.5;
                let dry_r = out_r + (sub + noise) * 0.5;

                let key_track = ((voice.pitch - 60.0) / 60.0).clamp(-1.0, 1.0) * 0.12;
                let cutoff_norm = (params.filter_cutoff
                    + voice.envelope * params.filter_env_amount.clamp(0.0, 1.0)
                    + filter_mod
                    + key_track)
                    .clamp(0.0, 1.0);
                let cutoff_hz = 35.0 * (18_000.0f32 / 35.0).powf(cutoff_norm);
                let g = (std::f32::consts::PI * cutoff_hz / sr).tan().clamp(0.0001, 10.0);
                let k = 2.0 - params.filter_resonance.clamp(0.0, 1.0) * 1.9;
                let a1 = (1.0 + g * (g + k)).recip();
                let a2 = g * a1;
                let a3 = g * a2;
                let (filtered_l, filtered_r) = (
                    svf(
                        dry_l,
                        &mut voice.filter_ic1_left,
                        &mut voice.filter_ic2_left,
                        a1,
                        a2,
                        a3,
                        k,
                        params.filter_mode,
                    ),
                    svf(
                        dry_r,
                        &mut voice.filter_ic1_right,
                        &mut voice.filter_ic2_right,
                        a1,
                        a2,
                        a3,
                        k,
                        params.filter_mode,
                    ),
                );
                let velocity_gain =
                    (1.0 - velocity_sensitivity) + voice.velocity * velocity_sensitivity;
                let gain = voice.envelope * velocity_gain * master;
                left[i] += filtered_l * gain;
                right[i] += filtered_r * gain;
            }
        }
    }

    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|voice| voice.active).count()
    }
}

#[inline]
fn lfo_value(phase: f64, shape: u8) -> f32 {
    match shape.min(2) {
        1 => {
            let p = phase as f32;
            if p < 0.5 { 4.0 * p - 1.0 } else { 3.0 - 4.0 * p }
        }
        2 => if phase < 0.5 { 1.0 } else { -1.0 },
        _ => (std::f64::consts::TAU * phase).sin() as f32,
    }
}

#[inline]
fn svf(
    input: f32,
    ic1: &mut f32,
    ic2: &mut f32,
    a1: f32,
    a2: f32,
    a3: f32,
    k: f32,
    mode: u8,
) -> f32 {
    let v3 = input - *ic2;
    let v1 = a1 * *ic1 + a2 * v3;
    let v2 = *ic2 + a2 * *ic1 + a3 * v3;
    *ic1 = 2.0 * v1 - *ic1;
    *ic2 = 2.0 * v2 - *ic2;
    match mode.min(2) {
        1 => v1,
        2 => input - k * v1 - v2,
        _ => v2,
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

    #[test]
    fn mono_legato_reuses_voice_and_tracks_pitch() {
        let t = table();
        let mut engine = PotionEngine::new(48_000.0, [t.clone(), t.clone(), t]);
        let mut params = PotionParams::default();
        params.voice_mode = 2;
        params.glide_seconds = 0.2;
        engine.note_on_with_params(60, 1.0, 0.5, &params);
        let mut left = vec![0.0; 128];
        let mut right = left.clone();
        engine.render(&mut left, &mut right, &params);
        engine.note_on_with_params(67, 1.0, 0.5, &params);
        assert_eq!(engine.active_voices(), 1);
        engine.render(&mut left, &mut right, &params);
        assert!(engine.voices[0].target_pitch > engine.voices[0].pitch);
    }

    #[test]
    fn filter_modes_produce_finite_output() {
        let t = table();
        let mut engine = PotionEngine::new(48_000.0, [t.clone(), t.clone(), t]);
        engine.note_on(69, 1.0);
        for mode in 0..=2 {
            let mut params = PotionParams::default();
            params.filter_mode = mode;
            params.filter_resonance = 0.95;
            let mut left = vec![0.0; 512];
            let mut right = left.clone();
            engine.render(&mut left, &mut right, &params);
            assert!(left.iter().chain(&right).all(|v| v.is_finite()));
        }
    }
}
