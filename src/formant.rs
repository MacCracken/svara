//! Formant filtering and vowel target definitions.
//!
//! Formants are resonant frequencies of the vocal tract. This module provides
//! parallel formant filter banks (using biquad resonators), vowel targets based on
//! Peterson & Barney (1952), and smooth interpolation between vowel shapes.

use alloc::format;
use alloc::string::ToString;
use serde::{Deserialize, Serialize};
use tracing::trace;

use crate::error::{Result, SvaraError};

/// A single formant resonance specification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Formant {
    /// Center frequency in Hz.
    pub frequency: f32,
    /// Bandwidth in Hz.
    pub bandwidth: f32,
    /// Relative amplitude (linear scale, typically 0.0-1.0).
    pub amplitude: f32,
}

impl Formant {
    /// Creates a new formant specification.
    #[must_use]
    pub fn new(frequency: f32, bandwidth: f32, amplitude: f32) -> Self {
        Self {
            frequency,
            bandwidth,
            amplitude,
        }
    }
}

/// Vowel categories for formant target lookup.
///
/// Based on IPA vowel classifications covering the primary vowel space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Vowel {
    /// /a/ — open front unrounded
    A,
    /// /e/ — close-mid front unrounded
    E,
    /// /i/ — close front unrounded
    I,
    /// /o/ — close-mid back rounded
    O,
    /// /u/ — close back rounded
    U,
    /// Schwa /ə/ — mid central
    Schwa,
    /// Open-O /ɔ/ — open-mid back rounded
    OpenO,
    /// Near-open front /æ/
    Ash,
    /// Near-close near-front /ɪ/
    NearI,
    /// Near-close near-back /ʊ/
    NearU,
}

/// Formant frequency and bandwidth targets for a vowel (F1 through F5).
///
/// Frequencies are based on Hillenbrand et al. (1995) for adult male speakers.
/// Bandwidths vary per vowel and formant, reflecting measured acoustic data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VowelTarget {
    /// First formant frequency (Hz).
    pub f1: f32,
    /// Second formant frequency (Hz).
    pub f2: f32,
    /// Third formant frequency (Hz).
    pub f3: f32,
    /// Fourth formant frequency (Hz).
    pub f4: f32,
    /// Fifth formant frequency (Hz).
    pub f5: f32,
    /// First formant bandwidth (Hz).
    pub b1: f32,
    /// Second formant bandwidth (Hz).
    pub b2: f32,
    /// Third formant bandwidth (Hz).
    pub b3: f32,
    /// Fourth formant bandwidth (Hz).
    pub b4: f32,
    /// Fifth formant bandwidth (Hz).
    pub b5: f32,
}

/// Default bandwidths (Hz) for formants F1-F5, used when specific values are not available.
const DEFAULT_BANDWIDTHS: [f32; 5] = [60.0, 80.0, 100.0, 120.0, 140.0];

/// Default amplitudes (linear) for formants F1-F5 in the parallel filter bank.
pub const DEFAULT_AMPLITUDES: [f32; 5] = [1.0, 0.8, 0.5, 0.3, 0.2];

impl VowelTarget {
    /// Creates a new vowel target with specified formant frequencies and default bandwidths.
    #[must_use]
    pub fn new(f1: f32, f2: f32, f3: f32, f4: f32, f5: f32) -> Self {
        Self {
            f1,
            f2,
            f3,
            f4,
            f5,
            b1: DEFAULT_BANDWIDTHS[0],
            b2: DEFAULT_BANDWIDTHS[1],
            b3: DEFAULT_BANDWIDTHS[2],
            b4: DEFAULT_BANDWIDTHS[3],
            b5: DEFAULT_BANDWIDTHS[4],
        }
    }

    /// Creates a vowel target with specified frequencies and bandwidths.
    ///
    /// `freqs` and `bws` are `[F1, F2, F3, F4, F5]` in Hz.
    #[must_use]
    pub fn with_bandwidths(freqs: [f32; 5], bws: [f32; 5]) -> Self {
        Self {
            f1: freqs[0],
            f2: freqs[1],
            f3: freqs[2],
            f4: freqs[3],
            f5: freqs[4],
            b1: bws[0],
            b2: bws[1],
            b3: bws[2],
            b4: bws[3],
            b5: bws[4],
        }
    }

    /// Returns the formant targets for a given vowel.
    ///
    /// Frequencies from Hillenbrand et al. (1995) for adult male speakers.
    /// Bandwidths are per-vowel estimates based on Hillenbrand and Hawks & Miller (1995).
    /// F4 and F5 frequencies are estimated from typical male vocal tract resonances.
    #[must_use]
    pub fn from_vowel(vowel: Vowel) -> Self {
        // Hillenbrand et al. (1995) male averages for F1-F3, with per-vowel bandwidths.
        // B1 ranges ~40-90 Hz depending on vowel openness (wider for open vowels).
        // B2 ranges ~60-120 Hz. B3 ranges ~80-150 Hz.
        // F4/F5 and B4/B5 are speaker-dependent estimates.
        match vowel {
            // Hillenbrand et al. (1995) male averages: [F1, F2, F3, F4, F5], [B1, B2, B3, B4, B5]
            Vowel::A => Self::with_bandwidths(
                [768.0, 1333.0, 2522.0, 3300.0, 3750.0],
                [90.0, 100.0, 120.0, 140.0, 160.0],
            ),
            Vowel::E => Self::with_bandwidths(
                [476.0, 2089.0, 2691.0, 3300.0, 3750.0],
                [55.0, 80.0, 100.0, 120.0, 140.0],
            ),
            Vowel::I => Self::with_bandwidths(
                [342.0, 2322.0, 3000.0, 3657.0, 3750.0],
                [40.0, 70.0, 90.0, 120.0, 140.0],
            ),
            Vowel::O => Self::with_bandwidths(
                [497.0, 910.0, 2459.0, 3300.0, 3750.0],
                [65.0, 70.0, 100.0, 120.0, 140.0],
            ),
            Vowel::U => Self::with_bandwidths(
                [378.0, 997.0, 2343.0, 3300.0, 3750.0],
                [45.0, 65.0, 90.0, 120.0, 140.0],
            ),
            Vowel::Schwa => Self::with_bandwidths(
                [523.0, 1588.0, 2469.0, 3300.0, 3750.0],
                [60.0, 80.0, 100.0, 120.0, 140.0],
            ),
            Vowel::OpenO => Self::with_bandwidths(
                [652.0, 997.0, 2538.0, 3300.0, 3750.0],
                [80.0, 75.0, 110.0, 130.0, 150.0],
            ),
            Vowel::Ash => Self::with_bandwidths(
                [669.0, 1880.0, 2489.0, 3300.0, 3750.0],
                [80.0, 90.0, 110.0, 130.0, 150.0],
            ),
            Vowel::NearI => Self::with_bandwidths(
                [427.0, 2034.0, 2684.0, 3300.0, 3750.0],
                [50.0, 75.0, 95.0, 120.0, 140.0],
            ),
            Vowel::NearU => Self::with_bandwidths(
                [469.0, 1122.0, 2434.0, 3300.0, 3750.0],
                [55.0, 70.0, 95.0, 120.0, 140.0],
            ),
        }
    }

    /// Converts vowel target to a fixed-size array of Formant specifications.
    #[must_use]
    pub fn to_formants(&self) -> [Formant; 5] {
        [
            Formant::new(self.f1, self.b1, DEFAULT_AMPLITUDES[0]),
            Formant::new(self.f2, self.b2, DEFAULT_AMPLITUDES[1]),
            Formant::new(self.f3, self.b3, DEFAULT_AMPLITUDES[2]),
            Formant::new(self.f4, self.b4, DEFAULT_AMPLITUDES[3]),
            Formant::new(self.f5, self.b5, DEFAULT_AMPLITUDES[4]),
        ]
    }

    /// Linearly interpolates between two vowel targets (frequencies and bandwidths).
    ///
    /// `t` is clamped to [0.0, 1.0]. At t=0.0, returns `from`; at t=1.0, returns `to`.
    #[must_use]
    pub fn interpolate(from: &VowelTarget, to: &VowelTarget, t: f32) -> VowelTarget {
        let t = t.clamp(0.0, 1.0);
        let lerp = |a: f32, b: f32| a + (b - a) * t;
        VowelTarget {
            f1: lerp(from.f1, to.f1),
            f2: lerp(from.f2, to.f2),
            f3: lerp(from.f3, to.f3),
            f4: lerp(from.f4, to.f4),
            f5: lerp(from.f5, to.f5),
            b1: lerp(from.b1, to.b1),
            b2: lerp(from.b2, to.b2),
            b3: lerp(from.b3, to.b3),
            b4: lerp(from.b4, to.b4),
            b5: lerp(from.b5, to.b5),
        }
    }
}

/// Maximum number of formant resonators in the parallel bank.
///
/// Fixed at 8 to align with f32x8 SIMD width. Unused slots are zeroed.
pub const MAX_FORMANTS: usize = 8;

/// Computes biquad bandpass filter coefficients for a given frequency and bandwidth.
///
/// Returns `(b0, b2, a1, a2)`. Note: `b1` is always 0 for bandpass, and `b2 = -b0`.
#[inline]
/// Computes biquad bandpass filter coefficients.
///
/// Computation is performed in f64 for precision (pole radius near 1.0 for
/// narrow bandwidths at high sample rates), then truncated to f32 for
/// per-sample processing.
pub(crate) fn biquad_coefficients(
    frequency: f32,
    bandwidth: f32,
    sample_rate: f32,
) -> (f32, f32, f32, f32) {
    // f64 computation prevents coefficient quantization errors
    let freq = frequency as f64;
    let bw = bandwidth as f64;
    let sr = sample_rate as f64;

    let omega = core::f64::consts::TAU * freq / sr;
    let cos_omega = crate::math::f64::cos(omega);
    let sin_omega = crate::math::f64::sin(omega);
    let bw_omega = core::f64::consts::TAU * bw / sr;
    let alpha = sin_omega * crate::math::f64::sinh(bw_omega / 2.0);

    let a0 = 1.0 + alpha;
    let b0 = (alpha / a0) as f32;
    let b2 = (-alpha / a0) as f32;
    let a1 = (-2.0 * cos_omega / a0) as f32;
    let a2 = ((1.0 - alpha) / a0) as f32;
    (b0, b2, a1, a2)
}

/// Structure-of-arrays biquad resonator bank for SIMD-friendly processing.
///
/// All arrays are fixed at [`MAX_FORMANTS`] width. Active formants are in
/// slots `0..count`, unused slots have zeroed coefficients (producing zero output).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BiquadBankSoa {
    /// Feedforward coefficient b0 (bandpass: b0 = alpha/a0).
    b0: [f32; MAX_FORMANTS],
    /// Feedforward coefficient b2 (bandpass: b2 = -b0).
    b2: [f32; MAX_FORMANTS],
    /// Feedback coefficient a1.
    a1: [f32; MAX_FORMANTS],
    /// Feedback coefficient a2.
    a2: [f32; MAX_FORMANTS],
    /// Amplitude weights per formant.
    amp: [f32; MAX_FORMANTS],
    /// Input delay line x[n-1].
    x1: [f32; MAX_FORMANTS],
    /// Input delay line x[n-2].
    x2: [f32; MAX_FORMANTS],
    /// Output delay line y[n-1].
    y1: [f32; MAX_FORMANTS],
    /// Output delay line y[n-2].
    y2: [f32; MAX_FORMANTS],
    /// Number of active formants.
    count: usize,
}

impl BiquadBankSoa {
    /// Creates a new SOA biquad bank from formant specifications.
    fn new(formants: &[Formant], sample_rate: f32) -> Self {
        let mut bank = Self {
            b0: [0.0; MAX_FORMANTS],
            b2: [0.0; MAX_FORMANTS],
            a1: [0.0; MAX_FORMANTS],
            a2: [0.0; MAX_FORMANTS],
            amp: [0.0; MAX_FORMANTS],
            x1: [0.0; MAX_FORMANTS],
            x2: [0.0; MAX_FORMANTS],
            y1: [0.0; MAX_FORMANTS],
            y2: [0.0; MAX_FORMANTS],
            count: formants.len().min(MAX_FORMANTS),
        };
        for (i, f) in formants.iter().take(MAX_FORMANTS).enumerate() {
            let (b0, b2, a1, a2) = biquad_coefficients(f.frequency, f.bandwidth, sample_rate);
            bank.b0[i] = b0;
            bank.b2[i] = b2;
            bank.a1[i] = a1;
            bank.a2[i] = a2;
            bank.amp[i] = f.amplitude;
        }
        bank
    }

    /// Updates coefficients for formant `i` without resetting state.
    fn update(
        &mut self,
        i: usize,
        frequency: f32,
        bandwidth: f32,
        amplitude: f32,
        sample_rate: f32,
    ) {
        let (b0, b2, a1, a2) = biquad_coefficients(frequency, bandwidth, sample_rate);
        self.b0[i] = b0;
        self.b2[i] = b2;
        self.a1[i] = a1;
        self.a2[i] = a2;
        self.amp[i] = amplitude;
    }

    /// Processes a single input sample through all formant slots, returns weighted sum.
    ///
    /// Processes all [`MAX_FORMANTS`] slots unconditionally — unused slots have
    /// zeroed coefficients and produce zero output. The fixed loop bound enables
    /// compiler auto-vectorization:
    /// - SSE2 (default x86_64): 128-bit, 2 iterations of 4 floats
    /// - AVX2 (`-C target-cpu=native`): 256-bit, 1 iteration of 8 floats, ~21% faster
    ///
    /// Manual SIMD intrinsics were benchmarked but the `#[target_feature]` call
    /// boundary prevents inlining, making runtime-detected intrinsics slower than
    /// the auto-vectorized loop. Build with `RUSTFLAGS="-C target-cpu=native"` for
    /// optimal performance on the build machine.
    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let mut sum = 0.0f32;

        // Fixed iteration count enables SIMD auto-vectorization.
        // Unused slots have b0=b2=a1=a2=amp=0, so they contribute nothing.
        for i in 0..MAX_FORMANTS {
            let y = self.b0[i] * input + self.b2[i] * self.x2[i]
                - self.a1[i] * self.y1[i]
                - self.a2[i] * self.y2[i];

            self.x2[i] = self.x1[i];
            self.x1[i] = input;
            self.y2[i] = self.y1[i];
            self.y1[i] = y;

            sum += y * self.amp[i];
        }
        sum
    }

    /// Processes a block of samples, writing results into `output`.
    ///
    /// More efficient than per-sample calls: the fixed inner loop bound
    /// and tight outer loop enable better register allocation and
    /// auto-vectorization.
    #[inline]
    fn process_block(&mut self, input: &[f32], output: &mut [f32]) {
        for (out, &inp) in output.iter_mut().zip(input.iter()) {
            let mut sum = 0.0f32;
            for i in 0..MAX_FORMANTS {
                let y = self.b0[i] * inp + self.b2[i] * self.x2[i]
                    - self.a1[i] * self.y1[i]
                    - self.a2[i] * self.y2[i];

                self.x2[i] = self.x1[i];
                self.x1[i] = inp;
                self.y2[i] = self.y1[i];
                self.y1[i] = y;

                sum += y * self.amp[i];
            }
            *out = sum;
        }
    }

    /// Resets all filter state.
    fn reset(&mut self) {
        self.x1 = [0.0; MAX_FORMANTS];
        self.x2 = [0.0; MAX_FORMANTS];
        self.y1 = [0.0; MAX_FORMANTS];
        self.y2 = [0.0; MAX_FORMANTS];
    }
}

/// One-pole DC-blocking high-pass filter.
///
/// Removes DC offset that accumulates from numerical drift in cascaded/parallel
/// biquad filters. Implements: `y[n] = x[n] - x[n-1] + α * y[n-1]`
/// with α chosen for a ~20 Hz cutoff.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DcBlocker {
    alpha: f32,
    x_prev: f32,
    y_prev: f32,
}

impl DcBlocker {
    fn new(sample_rate: f32) -> Self {
        // α = 1 - (2π * fc / fs), fc ≈ 20 Hz
        let alpha = 1.0 - (core::f32::consts::TAU * 20.0 / sample_rate);
        Self {
            alpha,
            x_prev: 0.0,
            y_prev: 0.0,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let output = input - self.x_prev + self.alpha * self.y_prev;
        self.x_prev = input;
        self.y_prev = output;
        output
    }

    fn reset(&mut self) {
        self.x_prev = 0.0;
        self.y_prev = 0.0;
    }
}

/// A parallel bank of biquad filters tuned to formant frequencies.
///
/// Processes an input signal (typically from [`GlottalSource`](crate::glottal::GlottalSource))
/// through parallel formant resonators, sums the weighted outputs, and applies
/// a DC-blocking filter to prevent numerical drift.
///
/// Internally uses a structure-of-arrays (SOA) layout for SIMD-friendly processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormantFilter {
    bank: BiquadBankSoa,
    dc_blocker: DcBlocker,
    sample_rate: f32,
}

impl FormantFilter {
    /// Creates a new parallel formant filter bank from the given formant specifications.
    ///
    /// # Errors
    ///
    /// Returns `SvaraError::InvalidFormant` if any formant frequency is out of range
    /// or if the formant list is empty.
    pub fn new(formants: &[Formant], sample_rate: f32) -> Result<Self> {
        if sample_rate <= 0.0 || !sample_rate.is_finite() {
            return Err(SvaraError::InvalidFormant(format!(
                "sample_rate must be positive and finite, got {sample_rate}"
            )));
        }
        if formants.is_empty() {
            return Err(SvaraError::InvalidFormant(
                "at least one formant is required".to_string(),
            ));
        }
        if formants.len() > MAX_FORMANTS {
            return Err(SvaraError::InvalidFormant(format!(
                "at most {} formants supported, got {}",
                MAX_FORMANTS,
                formants.len()
            )));
        }
        let nyquist = sample_rate / 2.0;
        for f in formants {
            if f.frequency <= 0.0 || f.frequency >= nyquist {
                return Err(SvaraError::InvalidFormant(format!(
                    "formant frequency {} must be in (0, {}) Hz",
                    f.frequency, nyquist
                )));
            }
            if f.bandwidth <= 0.0 {
                return Err(SvaraError::InvalidFormant(format!(
                    "bandwidth must be positive, got {}",
                    f.bandwidth
                )));
            }
        }

        let bank = BiquadBankSoa::new(formants, sample_rate);

        trace!(
            num_formants = formants.len(),
            sample_rate, "created formant filter"
        );

        Ok(Self {
            bank,
            dc_blocker: DcBlocker::new(sample_rate),
            sample_rate,
        })
    }

    /// Updates the formant filter targets (for smooth transitions).
    ///
    /// # Errors
    ///
    /// Returns `SvaraError::InvalidFormant` if formant count doesn't match.
    pub fn update_formants(&mut self, formants: &[Formant]) -> Result<()> {
        if formants.len() != self.bank.count {
            return Err(SvaraError::InvalidFormant(format!(
                "expected {} formants, got {}",
                self.bank.count,
                formants.len()
            )));
        }
        for (i, f) in formants.iter().enumerate() {
            self.bank
                .update(i, f.frequency, f.bandwidth, f.amplitude, self.sample_rate);
        }
        Ok(())
    }

    /// Processes a single input sample through the parallel formant filter bank.
    ///
    /// Runs the input through all formant resonators in parallel (SOA layout),
    /// sums the weighted outputs, and applies DC blocking.
    #[inline]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        let raw = self.bank.process(input);
        self.dc_blocker.process(raw)
    }

    /// Processes a block of input samples, writing results into `output`.
    ///
    /// More efficient than per-sample calls due to better auto-vectorization
    /// of the inner loop.
    pub fn process_block(&mut self, input: &[f32], output: &mut [f32]) {
        self.bank.process_block(input, output);
        // Apply DC blocker to the block
        for sample in output.iter_mut() {
            *sample = self.dc_blocker.process(*sample);
        }
    }

    /// Resets all filter states including the DC blocker.
    pub fn reset(&mut self) {
        self.bank.reset();
        self.dc_blocker.reset();
    }

    /// Returns the number of formant resonators.
    #[must_use]
    pub fn num_formants(&self) -> usize {
        self.bank.count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vowel_targets() {
        // Hillenbrand et al. (1995) male averages
        let target = VowelTarget::from_vowel(Vowel::A);
        assert!((target.f1 - 768.0).abs() < f32::EPSILON);
        assert!((target.f2 - 1333.0).abs() < f32::EPSILON);
        // Should have per-vowel bandwidths
        assert!((target.b1 - 90.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_interpolation_endpoints() {
        let from = VowelTarget::from_vowel(Vowel::A);
        let to = VowelTarget::from_vowel(Vowel::I);

        let at0 = VowelTarget::interpolate(&from, &to, 0.0);
        assert!((at0.f1 - from.f1).abs() < f32::EPSILON);
        assert!((at0.f2 - from.f2).abs() < f32::EPSILON);

        let at1 = VowelTarget::interpolate(&from, &to, 1.0);
        assert!((at1.f1 - to.f1).abs() < f32::EPSILON);
        assert!((at1.f2 - to.f2).abs() < f32::EPSILON);
    }

    #[test]
    fn test_formant_filter_creation() {
        let formants = VowelTarget::from_vowel(Vowel::A).to_formants();
        let ff = FormantFilter::new(&formants, 44100.0);
        assert!(ff.is_ok());
        assert_eq!(ff.unwrap().num_formants(), 5);
    }

    #[test]
    fn test_formant_filter_empty() {
        let ff = FormantFilter::new(&[], 44100.0);
        assert!(ff.is_err());
    }

    #[test]
    fn test_filter_processes_signal() {
        let formants = VowelTarget::from_vowel(Vowel::A).to_formants();
        let mut ff = FormantFilter::new(&formants, 44100.0).unwrap();
        // Feed impulse, check output is finite
        let out = ff.process_sample(1.0);
        assert!(out.is_finite());
        for _ in 0..100 {
            let o = ff.process_sample(0.0);
            assert!(o.is_finite());
        }
    }

    #[test]
    fn test_serde_roundtrip() {
        let target = VowelTarget::from_vowel(Vowel::E);
        let json = serde_json::to_string(&target).unwrap();
        let target2: VowelTarget = serde_json::from_str(&json).unwrap();
        assert!((target2.f1 - target.f1).abs() < f32::EPSILON);
    }
}
