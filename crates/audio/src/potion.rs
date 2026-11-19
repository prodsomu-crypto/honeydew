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
    pub filter_notch: bool,
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
    /// Tempo-locked LFOs follow the host beat while the transport is running.
    pub lfo_sync: [bool; 2],
    /// Quarter-note cycle divisions: 0=1/8 through 6=8/1.
    pub lfo_sync_division: [u8; 2],
    pub fm_amount: f32,
    pub ring_amount: f32,
    pub warp_amount: f32,
    /// 0 = off, 1 = bend, 2 = fold, 3 = hard-sync-like phase remap.
    pub warp_mode: u8,
    pub sub_level: f32,
    pub noise_level: f32,
    pub velocity_sensitivity: f32,
    pub filter_env_amount: f32,
    /// Signed keyboard tracking and positive velocity tracking, in normalized filter-octave units.
    pub filter_key_track: f32,
    pub filter_velocity: f32,
    pub filter_attack_seconds: f32,
    pub filter_decay_seconds: f32,
    pub filter_sustain: f32,
    pub filter_release_seconds: f32,
    pub pitch_attack_seconds: f32,
    pub pitch_decay_seconds: f32,
    pub pitch_sustain: f32,
    pub pitch_release_seconds: f32,
    /// Bipolar pitch-envelope amount in semitones.
    pub pitch_env_amount: f32,
    pub polyphony: usize,
    pub pitch_bend: f32,
    pub pitch_bend_range: f32,
    pub mod_wheel: f32,
    pub aftertouch: f32,
    /// Four simple modulation routes: source, destination and signed depth.
    /// Sources: LFO 1, LFO 2, amp envelope, velocity, mod wheel, aftertouch. Destinations: pitch, filter, pan, osc 1/2/3 level.
    pub mod_source: [u8; 4],
    pub mod_destination: [u8; 4],
    pub mod_amount: [f32; 4],
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
            filter_notch: false,
            unison: 1,
            detune_cents: 8.0,
            stereo_spread: 0.7,
            glide_seconds: 0.0,
            voice_mode: 0,
            lfo_rate: [1.0, 0.23],
            lfo_amount: [0.0; 2],
            lfo_destination: [0, 1],
            lfo_wave: [0, 1],
            lfo_sync: [false; 2],
            lfo_sync_division: [3; 2],
            fm_amount: 0.0,
            ring_amount: 0.0,
            warp_amount: 0.0,
            warp_mode: 0,
            sub_level: 0.0,
            noise_level: 0.0,
            velocity_sensitivity: 0.7,
            filter_env_amount: 0.0,
            filter_key_track: 0.12,
            filter_velocity: 0.0,
            filter_attack_seconds: 0.01,
            filter_decay_seconds: 0.25,
            filter_sustain: 0.7,
            filter_release_seconds: 0.18,
            pitch_attack_seconds: 0.01,
            pitch_decay_seconds: 0.25,
            pitch_sustain: 0.0,
            pitch_release_seconds: 0.18,
            pitch_env_amount: 0.0,
            polyphony: MAX_VOICES,
            pitch_bend: 0.0,
            pitch_bend_range: 2.0,
            mod_wheel: 0.0,
            aftertouch: 0.0,
            mod_source: [0, 1, 2, 3],
            mod_destination: [0, 1, 2, 3],
            mod_amount: [0.0; 4],
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
    slide_pitch: f32,
    slide_pan: f32,
    slide_velocity: f32,
    envelope: f32,
    /// 0 attack, 1 decay, 2 sustain, 3 release.
    amp_stage: u8,
    filter_envelope: f32,
    /// Independent filter ADSR stage: 0 attack, 1 decay, 2 sustain, 3 release.
    filter_stage: u8,
    pitch_envelope: f32,
    /// 0 attack, 1 decay, 2 sustain, 3 release.
    pitch_stage: u8,
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
        let voice_limit = params.polyphony.clamp(1, MAX_VOICES);
        let voices = &mut self.voices[..voice_limit];
        let existing = if mono {
            voices.iter().position(|voice| voice.active)
        } else {
            None
        };
        let slot = existing
            .or_else(|| voices.iter().position(|voice| !voice.active))
            .or_else(|| voices.iter().position(|voice| voice.amp_stage == 3))
            .unwrap_or(0);
        let voice = &mut voices[slot];
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
            let unison_count = params.unison.clamp(1, MAX_UNISON);
            for osc in 0..OSCILLATORS {
                for unison in 0..MAX_UNISON {
                    voice.oscillators[osc][unison]
                        .set_phase((unison % unison_count) as f64 / unison_count as f64);
                }
            }
        }
        self.last_pitch = target;
    }

    pub fn note_off(&mut self, note: u8) {
        for voice in self
            .voices
            .iter_mut()
            .filter(|voice| voice.active && voice.note == note)
        {
            voice.amp_stage = 3;
            voice.filter_stage = 3;
            voice.pitch_stage = 3;
        }
    }

    pub fn all_notes_off(&mut self) {
        for voice in self.voices.iter_mut().filter(|voice| voice.active) {
            voice.amp_stage = 3;
            voice.filter_stage = 3;
            voice.pitch_stage = 3;
        }
    }

    /// Applies the host's current slide offsets to every sounding voice.
    pub fn set_slide(&mut self, pitch: f32, pan: f32, velocity: f32) {
        for voice in self.voices.iter_mut().filter(|voice| voice.active) {
            voice.slide_pitch = pitch.clamp(-48.0, 48.0);
            voice.slide_pan = pan.clamp(-1.0, 1.0);
            voice.slide_velocity = velocity.clamp(-1.0, 1.0);
        }
    }

    /// Adds one rendered block to the supplied stereo buffers. Buffers must have equal lengths.
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32], params: &PotionParams) {
        self.render_with_transport(left, right, params, 120.0, 0.0, false);
    }

    pub fn render_with_transport(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        params: &PotionParams,
        tempo_bpm: f32,
        start_beat: f64,
        playing: bool,
    ) {
        let len = left.len().min(right.len());
        let sr = self.sample_rate;
        let tempo_bpm = if tempo_bpm.is_finite() && tempo_bpm > 0.0 {
            tempo_bpm
        } else {
            120.0
        };
        let beats_per_frame = f64::from(tempo_bpm) / (60.0 * f64::from(sr));
        let attack_step = (params.attack_seconds.max(0.001) * sr).recip();
        let decay_step = (params.decay_seconds.max(0.001) * sr).recip();
        let release_step = (params.release_seconds.max(0.001) * sr).recip();
        let sustain = params.sustain.clamp(0.0, 1.0);
        let filter_attack_step = (params.filter_attack_seconds.max(0.001) * sr).recip();
        let filter_decay_step = (params.filter_decay_seconds.max(0.001) * sr).recip();
        let filter_release_step = (params.filter_release_seconds.max(0.001) * sr).recip();
        let filter_sustain = params.filter_sustain.clamp(0.0, 1.0);
        let pitch_attack_step = (params.pitch_attack_seconds.max(0.001) * sr).recip();
        let pitch_decay_step = (params.pitch_decay_seconds.max(0.001) * sr).recip();
        let pitch_release_step = (params.pitch_release_seconds.max(0.001) * sr).recip();
        let pitch_sustain = params.pitch_sustain.clamp(0.0, 1.0);
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
        let polyphony = params.polyphony.clamp(1, MAX_VOICES);
        // Shrinking the voice limit cuts voices outside the active range instead of leaving stale tails behind.
        for voice in self.voices.iter_mut().skip(polyphony) {
            voice.active = false;
        }

        for voice in self.voices[..polyphony]
            .iter_mut()
            .filter(|voice| voice.active)
        {
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
                match voice.filter_stage {
                    0 => {
                        voice.filter_envelope =
                            (voice.filter_envelope + filter_attack_step).min(1.0);
                        if voice.filter_envelope >= 1.0 {
                            voice.filter_stage = 1;
                        }
                    }
                    1 => {
                        voice.filter_envelope = (voice.filter_envelope
                            - filter_decay_step * (1.0 - filter_sustain))
                            .max(filter_sustain);
                        if voice.filter_envelope <= filter_sustain + 1.0e-6 {
                            voice.filter_envelope = filter_sustain;
                            voice.filter_stage = 2;
                        }
                    }
                    2 => voice.filter_envelope = filter_sustain,
                    3 => {
                        voice.filter_envelope =
                            (voice.filter_envelope - filter_release_step).max(0.0);
                    }
                    _ => voice.filter_stage = 3,
                }

                match voice.pitch_stage {
                    0 => {
                        voice.pitch_envelope = (voice.pitch_envelope + pitch_attack_step).min(1.0);
                        if voice.pitch_envelope >= 1.0 {
                            voice.pitch_stage = 1;
                        }
                    }
                    1 => {
                        voice.pitch_envelope = (voice.pitch_envelope
                            - pitch_decay_step * (1.0 - pitch_sustain))
                            .max(pitch_sustain);
                        if voice.pitch_envelope <= pitch_sustain + 1.0e-6 {
                            voice.pitch_envelope = pitch_sustain;
                            voice.pitch_stage = 2;
                        }
                    }
                    2 => voice.pitch_envelope = pitch_sustain,
                    3 => {
                        voice.pitch_envelope = (voice.pitch_envelope - pitch_release_step).max(0.0);
                    }
                    _ => voice.pitch_stage = 3,
                }
                voice.pitch += (voice.target_pitch - voice.pitch) * glide_step;
                let base_hz = 440.0
                    * 2.0f32
                        .powf((voice.pitch + voice.slide_pitch + params.pitch_bend - 69.0) / 12.0);
                let beat = start_beat + i as f64 * beats_per_frame;
                let phase_a = if playing && params.lfo_sync[0] {
                    (beat / sync_cycle_beats(params.lfo_sync_division[0])).rem_euclid(1.0)
                } else {
                    voice.lfo_phase[0]
                };
                let phase_b = if playing && params.lfo_sync[1] {
                    (beat / sync_cycle_beats(params.lfo_sync_division[1])).rem_euclid(1.0)
                } else {
                    voice.lfo_phase[1]
                };
                let lfo_a = lfo_value(phase_a, params.lfo_wave[0]);
                let lfo_b = lfo_value(phase_b, params.lfo_wave[1]);
                for lfo in 0..2 {
                    if !playing || !params.lfo_sync[lfo] {
                        voice.lfo_phase[lfo] = (voice.lfo_phase[lfo]
                            + f64::from(params.lfo_rate[lfo].clamp(0.01, 30.0) / sr))
                        .fract();
                    }
                }
                let mut pitch_mod =
                    voice.pitch_envelope * params.pitch_env_amount.clamp(-24.0, 24.0);
                let mut filter_mod = 0.0;
                let mut pan_mod = 0.0;
                let mut oscillator_mod = [0.0f32; OSCILLATORS];
                for (value, amount, destination) in [
                    (lfo_a, params.lfo_amount[0], params.lfo_destination[0]),
                    (lfo_b, params.lfo_amount[1], params.lfo_destination[1]),
                ] {
                    let depth = amount.clamp(0.0, 1.0);
                    match destination.min(2) {
                        0 => pitch_mod += value * depth * 12.0,
                        1 => filter_mod += value * depth * 0.45,
                        _ => pan_mod += value * depth * 0.6,
                    }
                }
                for route in 0..4 {
                    let source = match params.mod_source[route].min(5) {
                        0 => lfo_a,
                        1 => lfo_b,
                        2 => voice.envelope,
                        3 => voice.velocity * 2.0 - 1.0,
                        4 => params.mod_wheel * 2.0 - 1.0,
                        _ => params.aftertouch * 2.0 - 1.0,
                    };
                    let depth = source * params.mod_amount[route].clamp(-1.0, 1.0);
                    match params.mod_destination[route].min(5) {
                        0 => pitch_mod += depth * 12.0,
                        1 => filter_mod += depth * 0.45,
                        2 => pan_mod += depth * 0.6,
                        3..=5 => {
                            oscillator_mod[usize::from(params.mod_destination[route] - 3)] += depth
                        }
                        _ => {}
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
                                (params.semitones[osc].clamp(-48.0, 48.0)
                                    + pitch_mod
                                    + cents / 100.0)
                                    / 12.0,
                            );
                        if osc == 0 {
                            hz *= 2.0f32.powf(osc_out[1] * fm_amount * 2.0);
                        }
                        let mut sample = voice.oscillators[osc][unison_index].next_sample_warped(
                            &self.tables[osc],
                            hz.clamp(1.0, sr * 0.45),
                            sr,
                            params.table_position[osc],
                            params.warp_mode,
                            warp_amount,
                        );
                        if params.warp_mode == 2 && warp_amount > 0.0 {
                            let drive = 1.0 + warp_amount * 7.0;
                            sample = (sample * drive).sin() / drive.sqrt();
                        }
                        let pan = (params.pan[osc]
                            + voice.pan
                            + voice.slide_pan
                            + (position * 2.0 - 1.0) * spread * 0.75
                            + pan_mod)
                            .clamp(-1.0, 1.0);
                        let left_gain = (1.0 - pan).sqrt() * 0.70710677;
                        let right_gain = (1.0 + pan).sqrt() * 0.70710677;
                        let level = (params.level[osc].clamp(0.0, 1.0)
                            * (1.0 + oscillator_mod[osc]).clamp(0.0, 2.0))
                        .clamp(0.0, 1.0);
                        let gain = level / unison as f32;
                        out_l += sample * gain * left_gain;
                        out_r += sample * gain * right_gain;
                        mono += sample * gain;
                    }
                    osc_out[osc] = mono;
                }
                if ring_amount > 0.0 {
                    // The dry signal is already present in the stereo sum; add the exact ring-modulation difference.
                    let delta = osc_out[0] * ring_amount * (osc_out[1] - 1.0);
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

                let key_track = ((voice.pitch - 60.0) / 60.0).clamp(-1.0, 1.0)
                    * params.filter_key_track.clamp(-1.0, 1.0);
                let velocity_track =
                    (voice.velocity - 0.5) * params.filter_velocity.clamp(0.0, 1.0) * 0.3;
                let cutoff_norm = (params.filter_cutoff
                    + voice.filter_envelope * params.filter_env_amount.clamp(0.0, 1.0)
                    + filter_mod
                    + key_track
                    + velocity_track)
                    .clamp(0.0, 1.0);
                let cutoff_hz = (35.0 * (18_000.0f32 / 35.0).powf(cutoff_norm)).min(sr * 0.45);
                let g = (std::f32::consts::PI * cutoff_hz / sr)
                    .tan()
                    .clamp(0.0001, 10.0);
                let k = 2.0 - params.filter_resonance.clamp(0.0, 1.0) * 1.9;
                let a1 = (1.0 + g * (g + k)).recip();
                let a2 = g * a1;
                let a3 = g * a2;
                let filter_mode = if params.filter_notch {
                    3
                } else {
                    params.filter_mode
                };
                let (filtered_l, filtered_r) = (
                    svf(
                        dry_l,
                        &mut voice.filter_ic1_left,
                        &mut voice.filter_ic2_left,
                        a1,
                        a2,
                        a3,
                        k,
                        filter_mode,
                    ),
                    svf(
                        dry_r,
                        &mut voice.filter_ic1_right,
                        &mut voice.filter_ic2_right,
                        a1,
                        a2,
                        a3,
                        k,
                        filter_mode,
                    ),
                );
                let velocity = (voice.velocity + voice.slide_velocity).clamp(0.0, 1.0);
                let velocity_gain = (1.0 - velocity_sensitivity) + velocity * velocity_sensitivity;
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
fn sync_cycle_beats(division: u8) -> f64 {
    [0.5, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0][usize::from(division.min(6))]
}

fn lfo_value(phase: f64, shape: u8) -> f32 {
    match shape.min(2) {
        1 => {
            let p = phase as f32;
            if p < 0.5 {
                4.0 * p - 1.0
            } else {
                3.0 - 4.0 * p
            }
        }
        2 => {
            if phase < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
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
    match mode.min(3) {
        1 => v1,
        2 => input - k * v1 - v2,
        3 => input - k * v1,
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
    fn tempo_sync_divisions_are_quarter_note_based() {
        assert_eq!(sync_cycle_beats(0), 0.5);
        assert_eq!(sync_cycle_beats(3), 4.0);
        assert_eq!(sync_cycle_beats(6), 32.0);
        assert_eq!((4.0f64 / sync_cycle_beats(3)).fract(), 0.0);
        assert_eq!((2.0f64 / sync_cycle_beats(3)).fract(), 0.5);
    }

    #[test]
    fn tempo_synced_lfo_tracks_host_and_free_lfo_keeps_running() {
        let t = table();
        let mut engine = PotionEngine::new(48_000.0, [t.clone(), t.clone(), t]);
        let mut params = PotionParams::default();
        params.lfo_sync = [true, false];
        engine.note_on(69, 1.0);
        let mut left = vec![0.0; 256];
        let mut right = left.clone();
        engine.render_with_transport(&mut left, &mut right, &params, 120.0, 3.0, true);
        assert_eq!(engine.voices[0].lfo_phase[0], 0.0);
        assert!(engine.voices[0].lfo_phase[1] > 0.0);
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
    fn pitch_envelope_has_its_own_attack_and_decay() {
        let t = table();
        let mut engine = PotionEngine::new(48_000.0, [t.clone(), t.clone(), t]);
        let mut params = PotionParams::default();
        params.attack_seconds = 0.001;
        params.sustain = 1.0;
        params.pitch_attack_seconds = 0.1;
        params.pitch_decay_seconds = 0.1;
        params.pitch_sustain = 0.0;
        params.pitch_env_amount = 12.0;
        engine.note_on_with_params(69, 1.0, 0.5, &params);
        let mut left = vec![0.0; 48];
        let mut right = left.clone();
        engine.render(&mut left, &mut right, &params);
        assert!(engine.voices[0].envelope > engine.voices[0].pitch_envelope);
        assert!(engine.voices[0].pitch_envelope > 0.0);
    }

    #[test]
    fn filter_envelope_has_its_own_attack_time() {
        let t = table();
        let mut engine = PotionEngine::new(48_000.0, [t.clone(), t.clone(), t]);
        let mut params = PotionParams::default();
        params.attack_seconds = 0.001;
        params.sustain = 1.0;
        params.filter_attack_seconds = 0.1;
        params.filter_sustain = 0.0;
        engine.note_on_with_params(69, 1.0, 0.5, &params);
        let mut left = vec![0.0; 48];
        let mut right = left.clone();
        engine.render(&mut left, &mut right, &params);
        assert!(engine.voices[0].envelope > engine.voices[0].filter_envelope);
        assert!(engine.voices[0].filter_envelope > 0.0);
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
        let mut params = PotionParams::default();
        params.filter_notch = true;
        params.filter_resonance = 0.95;
        let mut left = vec![0.0; 512];
        let mut right = left.clone();
        engine.render(&mut left, &mut right, &params);
        assert!(left.iter().chain(&right).all(|v| v.is_finite()));
    }
}
