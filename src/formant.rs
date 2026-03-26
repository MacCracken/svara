//! Formant filtering and vowel target definitions.
//!
//! Formants are resonant frequencies of the vocal tract. This module provides
//! formant filter cascades (using biquad resonators), vowel targets based on
//! Peterson & Barney (1952), and smooth interpolation between vowel shapes.

use serde::{Deserialize, Serialize};
use tracing::trace;

use crate::error::{Result, SvaraError};

/// A single formant resonance specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Formant frequency targets for a vowel (F1 through F5).
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

impl VowelTarget {
    /// Creates a new vowel target with specified formant frequencies.
    #[must_use]
    pub fn new(f1: f32, f2: f32, f3: f32, f4: f32, f5: f32) -> Self {
        Self { f1, f2, f3, f4, f5 }
    }

    /// Returns the formant targets for a given vowel.
    ///
    /// Frequencies from Peterson & Barney (1952) for adult male speakers,
    /// with F4 and F5 estimated.
    #[must_use]
    pub fn from_vowel(vowel: Vowel) -> Self {
        match vowel {
            Vowel::A => Self::new(730.0, 1090.0, 2440.0, 3300.0, 3750.0),
            Vowel::E => Self::new(530.0, 1840.0, 2480.0, 3300.0, 3750.0),
            Vowel::I => Self::new(270.0, 2290.0, 3010.0, 3300.0, 3750.0),
            Vowel::O => Self::new(570.0, 840.0, 2410.0, 3300.0, 3750.0),
            Vowel::U => Self::new(300.0, 870.0, 2240.0, 3300.0, 3750.0),
            Vowel::Schwa => Self::new(500.0, 1500.0, 2500.0, 3300.0, 3750.0),
            Vowel::OpenO => Self::new(590.0, 880.0, 2540.0, 3300.0, 3750.0),
            Vowel::Ash => Self::new(660.0, 1720.0, 2410.0, 3300.0, 3750.0),
            Vowel::NearI => Self::new(390.0, 1990.0, 2550.0, 3300.0, 3750.0),
            Vowel::NearU => Self::new(440.0, 1020.0, 2240.0, 3300.0, 3750.0),
        }
    }

    /// Converts vowel target to a Vec of Formant specifications with default bandwidths.
    #[must_use]
    pub fn to_formants(&self) -> Vec<Formant> {
        vec![
            Formant::new(self.f1, 60.0, 1.0),
            Formant::new(self.f2, 80.0, 0.8),
            Formant::new(self.f3, 100.0, 0.5),
            Formant::new(self.f4, 120.0, 0.3),
            Formant::new(self.f5, 140.0, 0.2),
        ]
    }

    /// Linearly interpolates between two vowel targets.
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
        }
    }
}

/// A second-order biquad resonator filter (internal implementation).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BiquadResonator {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl BiquadResonator {
    /// Creates a bandpass resonator tuned to the given frequency and bandwidth.
    fn new(frequency: f32, bandwidth: f32, sample_rate: f32) -> Self {
        let omega = 2.0 * std::f32::consts::PI * frequency / sample_rate;
        let cos_omega = omega.cos();
        let sin_omega = omega.sin();

        // Bandwidth as Q factor
        let bw_omega = 2.0 * std::f32::consts::PI * bandwidth / sample_rate;
        let alpha = sin_omega * (bw_omega / 2.0).sinh();

        // Bandpass filter (constant peak gain)
        let b0 = alpha;
        let b1 = 0.0;
        let b2 = -alpha;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    /// Updates the filter coefficients for new frequency and bandwidth.
    fn update(&mut self, frequency: f32, bandwidth: f32, sample_rate: f32) {
        let new = Self::new(frequency, bandwidth, sample_rate);
        self.b0 = new.b0;
        self.b1 = new.b1;
        self.b2 = new.b2;
        self.a1 = new.a1;
        self.a2 = new.a2;
        // Keep state to avoid clicks
    }

    /// Processes a single sample through the biquad filter.
    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let output =
            self.b0 * input + self.b1 * self.x1 + self.b2 * self.x2 - self.a1 * self.y1
                - self.a2 * self.y2;

        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;

        output
    }

    /// Resets the filter state.
    fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

/// A cascade of biquad filters tuned to formant frequencies.
///
/// Processes an input signal (typically from [`GlottalSource`](crate::glottal::GlottalSource))
/// through parallel formant resonators and sums the outputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormantFilter {
    filters: Vec<BiquadResonator>,
    amplitudes: Vec<f32>,
    sample_rate: f32,
}

impl FormantFilter {
    /// Creates a new formant filter cascade from the given formant specifications.
    ///
    /// # Errors
    ///
    /// Returns `SvaraError::InvalidFormant` if any formant frequency is out of range
    /// or if the formant list is empty.
    pub fn new(formants: &[Formant], sample_rate: f32) -> Result<Self> {
        if formants.is_empty() {
            return Err(SvaraError::InvalidFormant(
                "at least one formant is required".to_string(),
            ));
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

        let filters: Vec<BiquadResonator> = formants
            .iter()
            .map(|f| BiquadResonator::new(f.frequency, f.bandwidth, sample_rate))
            .collect();
        let amplitudes: Vec<f32> = formants.iter().map(|f| f.amplitude).collect();

        trace!(
            num_formants = formants.len(),
            sample_rate,
            "created formant filter"
        );

        Ok(Self {
            filters,
            amplitudes,
            sample_rate,
        })
    }

    /// Updates the formant filter targets (for smooth transitions).
    ///
    /// # Errors
    ///
    /// Returns `SvaraError::InvalidFormant` if formant count doesn't match.
    pub fn update_formants(&mut self, formants: &[Formant]) -> Result<()> {
        if formants.len() != self.filters.len() {
            return Err(SvaraError::InvalidFormant(format!(
                "expected {} formants, got {}",
                self.filters.len(),
                formants.len()
            )));
        }
        for (i, f) in formants.iter().enumerate() {
            self.filters[i].update(f.frequency, f.bandwidth, self.sample_rate);
            self.amplitudes[i] = f.amplitude;
        }
        Ok(())
    }

    /// Processes a single input sample through the formant filter cascade.
    ///
    /// Runs the input through all formant resonators in parallel and sums the
    /// weighted outputs.
    #[inline]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        let mut output = 0.0;
        for (filter, &amp) in self.filters.iter_mut().zip(self.amplitudes.iter()) {
            output += filter.process(input) * amp;
        }
        output
    }

    /// Resets all filter states.
    pub fn reset(&mut self) {
        for filter in &mut self.filters {
            filter.reset();
        }
    }

    /// Returns the number of formant resonators.
    #[must_use]
    pub fn num_formants(&self) -> usize {
        self.filters.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vowel_targets() {
        let target = VowelTarget::from_vowel(Vowel::A);
        assert!((target.f1 - 730.0).abs() < f32::EPSILON);
        assert!((target.f2 - 1090.0).abs() < f32::EPSILON);
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
