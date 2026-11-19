//! Band-limited wavetable primitives for Honeydew's native wavetable instrument.
//! Wavetables are prepared off the audio thread. Playback performs no allocation, locking or I/O.

use std::f64::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WavetableError {
    NoFrames,
    TooManyFrames,
    InvalidLength,
    MismatchedFrameLength,
    NonFiniteSample,
}
pub const MIN_TABLE_SIZE: usize = 32;
pub const MAX_TABLE_SIZE: usize = 4096;
pub const MAX_FRAMES: usize = 256;

struct Frame {
    raw: Box<[f32]>,
    mips: Vec<Box<[f32]>>,
}

/// Immutable wavetable. Construct on a worker/app thread, then share across voices.
pub struct Wavetable {
    frames: Box<[Frame]>,
    table_size: usize,
}

impl Wavetable {
    /// Creates a wavetable from equally sized periodic frames.
    pub fn new(frames: Vec<Vec<f32>>) -> Result<Self, WavetableError> {
        if frames.is_empty() {
            return Err(WavetableError::NoFrames);
        }
        if frames.len() > MAX_FRAMES {
            return Err(WavetableError::TooManyFrames);
        }
        let n = frames[0].len();
        if !(MIN_TABLE_SIZE..=MAX_TABLE_SIZE).contains(&n) || !n.is_power_of_two() {
            return Err(WavetableError::InvalidLength);
        }
        if frames.iter().any(|f| f.len() != n) {
            return Err(WavetableError::MismatchedFrameLength);
        }
        if frames.iter().flatten().any(|x| !x.is_finite()) {
            return Err(WavetableError::NonFiniteSample);
        }
        let frames = frames
            .into_iter()
            .map(|samples| {
                let mips = build_mips(&samples);
                Frame {
                    raw: samples.into_boxed_slice(),
                    mips,
                }
            })
            .collect::<Vec<_>>();
        Ok(Self {
            frames: frames.into_boxed_slice(),
            table_size: n,
        })
    }

    /// Builds a harmonic-scan bank from one periodic cycle. Frames progress from a fundamental-only sine
    /// towards the complete source spectrum, so imported single-cycle WAVs can be scanned like a wavetable.
    /// Preparation is intended for the app/worker thread, never the render callback.
    pub fn spectral_frames(cycle: &[f32], frame_count: usize) -> Result<Vec<Vec<f32>>, WavetableError> {
        if frame_count < 2 {
            return Err(WavetableError::InvalidLength);
        }
        if frame_count > MAX_FRAMES {
            return Err(WavetableError::TooManyFrames);
        }
        let n = cycle.len();
        if !(MIN_TABLE_SIZE..=MAX_TABLE_SIZE).contains(&n) || !n.is_power_of_two() {
            return Err(WavetableError::InvalidLength);
        }
        if cycle.iter().any(|sample| !sample.is_finite()) {
            return Err(WavetableError::NonFiniteSample);
        }

        let mut spectrum_re: Vec<f64> = cycle.iter().map(|&sample| f64::from(sample)).collect();
        let mut spectrum_im = vec![0.0; n];
        fft(&mut spectrum_re, &mut spectrum_im, false);
        // A wavetable is periodic and zero-centred; remove DC so a biased source does not cause clicks.
        spectrum_re[0] = 0.0;
        spectrum_im[0] = 0.0;
        let source_peak = cycle.iter().fold(0.0f32, |peak, sample| peak.max(sample.abs()));
        let max_harmonic = n / 2;
        let log_max = (max_harmonic as f64).log2();
        let mut result = Vec::with_capacity(frame_count);

        for frame in 0..frame_count {
            let progress = frame as f64 / (frame_count - 1) as f64;
            let cutoff = 2.0f64.powf(progress * log_max).round() as usize;
            let mut real = spectrum_re.clone();
            let mut imag = spectrum_im.clone();
            for bin in 1..n {
                let harmonic = bin.min(n - bin);
                if harmonic > cutoff {
                    real[bin] = 0.0;
                    imag[bin] = 0.0;
                }
            }
            fft(&mut real, &mut imag, true);
            let mut samples: Vec<f32> = real.into_iter().map(|sample| sample as f32).collect();
            let peak = samples.iter().fold(0.0f32, |peak, sample| peak.max(sample.abs()));
            if peak > 1.0e-8 && source_peak > 1.0e-8 {
                let gain = (source_peak / peak).min(4.0);
                samples.iter_mut().for_each(|sample| *sample *= gain);
            }
            result.push(samples);
        }
        Ok(result)
    }

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }
    pub fn table_size(&self) -> usize {
        self.table_size
    }

    /// Reads one sample with frame morphing and frequency-selected anti-aliasing mip levels.
    #[inline]
    pub fn sample(&self, phase: f64, position: f32, frequency_hz: f32, sample_rate: f32) -> f32 {
        let pos = position.clamp(0.0, 1.0) * (self.frames.len() - 1) as f32;
        let frame_a = pos.floor() as usize;
        let frame_b = (frame_a + 1).min(self.frames.len() - 1);
        let frame_mix = pos - frame_a as f32;
        let (level_a, level_b, level_mix) = self.mip_selection(frequency_hz, sample_rate);
        let a = self.read_frame(&self.frames[frame_a], phase, level_a, level_b, level_mix);
        if frame_a == frame_b {
            return a;
        }
        let b = self.read_frame(&self.frames[frame_b], phase, level_a, level_b, level_mix);
        a + (b - a) * frame_mix
    }

    #[inline]
    fn read_frame(&self, frame: &Frame, phase: f64, a: usize, b: usize, mix: f32) -> f32 {
        let x = wrap_phase(phase) * self.table_size as f64;
        let i = x.floor() as usize;
        let frac = (x - i as f64) as f32;
        let ta: &[f32] = if a == 0 {
            &frame.raw
        } else {
            &frame.mips[a - 1]
        };
        let va = read_linear(ta, i, frac);
        if a == b {
            return va;
        }
        let tb: &[f32] = if b == 0 {
            &frame.raw
        } else {
            &frame.mips[b - 1]
        };
        let vb = read_linear(tb, i, frac);
        va + (vb - va) * mix
    }

    #[inline]
    fn mip_selection(&self, frequency: f32, sample_rate: f32) -> (usize, usize, f32) {
        let max_level = self.frames[0].mips.len();
        if !frequency.is_finite()
            || !sample_rate.is_finite()
            || frequency <= 0.0
            || sample_rate <= 0.0
        {
            return (0, 0, 0.0);
        }
        let allowed = (sample_rate * 0.5 / frequency).max(1.0);
        let needed = (((self.table_size as f32 * 0.5) / allowed).max(1.0))
            .log2()
            .max(0.0);
        let lo = (needed.floor() as usize).min(max_level);
        let hi = (lo + 1).min(max_level);
        (lo, hi, (needed - lo as f32).clamp(0.0, 1.0))
    }
}

/// A phase-continuous single wavetable oscillator.
#[derive(Clone, Copy, Debug, Default)]
pub struct WavetableOscillator {
    phase: f64,
}

impl WavetableOscillator {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn set_phase(&mut self, phase: f64) {
        self.phase = wrap_phase(phase);
    }
    pub fn phase(&self) -> f64 {
        self.phase
    }
    pub fn reset(&mut self) {
        self.phase = 0.0;
    }

    #[inline]
    pub fn next_sample(
        &mut self,
        table: &Wavetable,
        frequency_hz: f32,
        sample_rate: f32,
        position: f32,
    ) -> f32 {
        if !frequency_hz.is_finite()
            || !sample_rate.is_finite()
            || frequency_hz <= 0.0
            || sample_rate <= 0.0
        {
            return 0.0;
        }
        let out = table.sample(self.phase, position, frequency_hz, sample_rate);
        self.phase = wrap_phase(self.phase + f64::from(frequency_hz / sample_rate));
        out
    }
}

#[inline]
fn wrap_phase(phase: f64) -> f64 {
    if phase.is_finite() {
        phase.rem_euclid(1.0)
    } else {
        0.0
    }
}
#[inline]
fn read_linear(table: &[f32], i: usize, frac: f32) -> f32 {
    let mask = table.len() - 1;
    let a = table[i & mask];
    a + (table[(i + 1) & mask] - a) * frac
}

/// FFT-derived mip levels with progressively fewer harmonics; construction is never called by render().
fn build_mips(samples: &[f32]) -> Vec<Box<[f32]>> {
    let n = samples.len();
    let mut re: Vec<f64> = samples.iter().map(|&v| f64::from(v)).collect();
    let mut im = vec![0.0; n];
    fft(&mut re, &mut im, false);
    let mut result = Vec::new();
    let mut harmonics = n / 2;
    while harmonics > 1 {
        harmonics = (harmonics / 2).max(1);
        let mut rr = re.clone();
        let mut ii = im.clone();
        for h in (harmonics + 1)..n.saturating_sub(harmonics) {
            rr[h] = 0.0;
            ii[h] = 0.0;
        }
        fft(&mut rr, &mut ii, true);
        result.push(
            rr.into_iter()
                .map(|v| v as f32)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        if harmonics == 1 {
            break;
        }
    }
    result
}

fn fft(re: &mut [f64], im: &mut [f64], inverse: bool) {
    let n = re.len();
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let sign = if inverse { 1.0 } else { -1.0 };
    let mut len = 2;
    while len <= n {
        let half = len / 2;
        for k in 0..half {
            let (wi, wr) = (sign * 2.0 * PI * k as f64 / len as f64).sin_cos();
            for start in (0..n).step_by(len) {
                let (a, b) = (start + k, start + k + half);
                let tr = re[b] * wr - im[b] * wi;
                let ti = re[b] * wi + im[b] * wr;
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
            }
        }
        len <<= 1;
    }
    if inverse {
        let scale = 1.0 / n as f64;
        for v in re.iter_mut() {
            *v *= scale;
        }
        for v in im.iter_mut() {
            *v *= scale;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sine(n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| (2.0 * PI * i as f64 / n as f64).sin() as f32)
            .collect()
    }
    #[test]
    fn builds_and_reads_single_frame() {
        let table = Wavetable::new(vec![sine(256)]).unwrap();
        assert_eq!(table.frame_count(), 1);
        assert_eq!(table.table_size(), 256);
        assert!(table.sample(0.25, 0.0, 440.0, 48_000.0).abs() > 0.9);
    }
    #[test]
    fn interpolates_between_frames() {
        let table = Wavetable::new(vec![vec![0.25; 256], vec![-0.25; 256]]).unwrap();
        assert!(table.sample(0.1, 0.5, 220.0, 48_000.0).abs() < 1.0e-5);
    }
    #[test]
    fn rejects_invalid_tables() {
        assert_eq!(Wavetable::new(vec![]).err(), Some(WavetableError::NoFrames));
        assert_eq!(
            Wavetable::new(vec![vec![0.0; 100]]).err(),
            Some(WavetableError::InvalidLength)
        );
        assert_eq!(
            Wavetable::new(vec![vec![0.0; 64], vec![0.0; 128]]).err(),
            Some(WavetableError::MismatchedFrameLength)
        );
    }
    #[test]
    fn spectral_frames_form_a_bounded_harmonic_scan_bank() {
        let cycle: Vec<f32> = (0..256)
            .map(|i| {
                let phase = std::f32::consts::TAU * i as f32 / 256.0;
                phase.sin() + 0.35 * (phase * 7.0).sin()
            })
            .collect();
        let frames = Wavetable::spectral_frames(&cycle, 16).unwrap();
        assert_eq!(frames.len(), 16);
        assert!(frames.iter().all(|frame| frame.len() == 256));
        assert!(frames.iter().flatten().all(|sample| sample.is_finite()));
        let harmonic_energy = |frame: &[f32], harmonic: usize| {
            let mut re: Vec<f64> = frame.iter().map(|&sample| f64::from(sample)).collect();
            let mut im = vec![0.0; frame.len()];
            fft(&mut re, &mut im, false);
            re[harmonic].hypot(im[harmonic])
        };
        assert!(harmonic_energy(&frames[15], 7) > harmonic_energy(&frames[0], 7));
        assert_eq!(
            Wavetable::spectral_frames(&cycle, MAX_FRAMES + 1).err(),
            Some(WavetableError::TooManyFrames)
        );
    }

    #[test]
    fn oscillator_phase_wraps() {
        let table = Wavetable::new(vec![sine(256)]).unwrap();
        let mut osc = WavetableOscillator::new();
        for _ in 0..48_000 {
            let _ = osc.next_sample(&table, 440.0, 48_000.0, 0.0);
        }
        assert!(osc.phase() < 1.0 && osc.phase() >= 0.0);
    }
}
