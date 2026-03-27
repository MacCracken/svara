//! Vocal tract model connecting glottal source to output.
//!
//! The vocal tract shapes the glottal excitation through formant filtering,
//! nasal coupling, and lip radiation to produce the final speech signal.

use alloc::{vec, vec::Vec};
use serde::{Deserialize, Serialize};
use tracing::trace;

use crate::error::Result;
use crate::formant::{Formant, FormantFilter, Vowel, VowelTarget};

/// Nasal anti-formant center frequency (Hz) — models the nasal sinus zero.
const NASAL_ANTIFORMANT_FREQ: f32 = 250.0;
/// Nasal anti-formant bandwidth (Hz).
const NASAL_ANTIFORMANT_BW: f32 = 100.0;
/// Lip radiation coefficient (first-order high-pass approximation).
const DEFAULT_LIP_RADIATION: f32 = 0.97;
use crate::glottal::GlottalSource;

/// Place of articulation for nasal consonants, affecting antiformant frequency.
///
/// Different nasal consonants produce zeros (anti-formants) at different
/// frequencies depending on where the oral cavity is closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum NasalPlace {
    /// Bilabial (/m/): anti-formant ~750 Hz, long oral cavity.
    Bilabial,
    /// Alveolar (/n/): anti-formant ~1450 Hz, medium oral cavity.
    Alveolar,
    /// Velar (/ŋ/): anti-formant ~3000 Hz, short oral cavity.
    Velar,
    /// Default/neutral position (~250 Hz).
    Neutral,
}

impl NasalPlace {
    /// Returns the anti-formant frequency for this place of articulation.
    #[must_use]
    fn antiformant_frequency(self) -> f32 {
        match self {
            Self::Neutral => NASAL_ANTIFORMANT_FREQ,
            Self::Bilabial => 750.0,
            Self::Alveolar => 1450.0,
            Self::Velar => 3000.0,
        }
    }
}

/// A vocal tract model that processes glottal excitation into speech.
///
/// Includes formant filtering, nasal coupling (anti-formant), and lip
/// radiation (first-order high-pass differentiation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VocalTract {
    /// Parallel formant filter bank.
    filter: FormantFilter,
    /// Nasal coupling coefficient (0.0 = oral, 1.0 = fully nasal).
    nasal_coupling: f32,
    /// Anti-formant filter for nasal coupling (zero at ~250Hz).
    nasal_antiformant: NasalAntiformant,
    /// Lip radiation: previous sample for first-order difference filter.
    lip_prev: f32,
    /// Lip radiation coefficient (0.0-1.0).
    lip_radiation: f32,
    /// Source-filter interaction: feedback from tract output to modify excitation.
    /// Models the effect of vocal tract impedance on glottal flow.
    /// Range: 0.0 (no coupling) to 0.3 (strong coupling).
    interaction_strength: f32,
    /// Previous tract output for source-filter interaction feedback.
    interaction_feedback: f32,
    /// Subglottal resonance coupling strength (0.0 = off, typically 0.05-0.1).
    /// Adds a resonance at ~600Hz that interacts with F1, modeling tracheal effects.
    subglottal_coupling: f32,
    /// Subglottal resonance state: [x1, x2, y1, y2] for a 600Hz bandpass biquad.
    sg_state: [f32; 4],
    /// Subglottal biquad coefficients: [b0, b2, a1, a2].
    sg_coeff: [f32; 4],
    /// Output gain normalization factor.
    gain: f32,
    /// Sample rate in Hz.
    sample_rate: f32,
}

/// Simple anti-formant (notch) filter for nasal coupling.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NasalAntiformant {
    /// Center frequency of the nasal zero.
    frequency: f32,
    /// Bandwidth of the nasal zero.
    bandwidth: f32,
    // Biquad notch coefficients
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

impl NasalAntiformant {
    fn new(frequency: f32, bandwidth: f32, sample_rate: f32) -> Self {
        let omega = 2.0 * core::f32::consts::PI * frequency / sample_rate;
        let cos_omega = crate::math::f32::cos(omega);
        let bw_omega = 2.0 * core::f32::consts::PI * bandwidth / sample_rate;
        let alpha = crate::math::f32::sinh(bw_omega / 2.0) * crate::math::f32::sin(omega);

        // Notch filter coefficients
        let a0 = 1.0 + alpha;
        let b0 = 1.0 / a0;
        let b1 = (-2.0 * cos_omega) / a0;
        let b2 = 1.0 / a0;
        let a1 = (-2.0 * cos_omega) / a0;
        let a2 = (1.0 - alpha) / a0;

        Self {
            frequency,
            bandwidth,
            b0,
            b1,
            b2,
            a1,
            a2,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let output = self.b0 * input + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;
        output
    }

    /// Updates the nasal antiformant frequency and bandwidth, preserving state.
    fn update(&mut self, frequency: f32, bandwidth: f32, sample_rate: f32) {
        let new = Self::new(frequency, bandwidth, sample_rate);
        self.frequency = new.frequency;
        self.bandwidth = new.bandwidth;
        self.b0 = new.b0;
        self.b1 = new.b1;
        self.b2 = new.b2;
        self.a1 = new.a1;
        self.a2 = new.a2;
        // Keep state to avoid clicks
    }

    fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

impl VocalTract {
    /// Creates a new vocal tract with default neutral vowel (schwa) formants.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let target = VowelTarget::from_vowel(Vowel::Schwa);
        let formants = target.to_formants();
        // unwrap safety: VowelTarget always produces valid formants and sample_rate
        // is used as-is. We use a fallback if somehow creation fails.
        let filter = FormantFilter::new(&formants, sample_rate).unwrap_or_else(|_| {
            // Absolute fallback: single formant at 500Hz
            FormantFilter::new(&[Formant::new(500.0, 100.0, 1.0)], sample_rate)
                .expect("fallback formant filter must succeed")
        });

        let nasal_antiformant =
            NasalAntiformant::new(NASAL_ANTIFORMANT_FREQ, NASAL_ANTIFORMANT_BW, sample_rate);

        trace!(sample_rate, "created vocal tract with neutral vowel");

        Self {
            filter,
            nasal_coupling: 0.0,
            nasal_antiformant,
            lip_prev: 0.0,
            lip_radiation: DEFAULT_LIP_RADIATION,
            interaction_strength: 0.05,
            interaction_feedback: 0.0,
            subglottal_coupling: 0.05,
            sg_state: [0.0; 4],
            sg_coeff: {
                let (b0, b2, a1, a2) =
                    crate::formant::biquad_coefficients(600.0, 80.0, sample_rate);
                [b0, b2, a1, a2]
            },
            gain: 1.0,
            sample_rate,
        }
    }

    /// Configures the vocal tract for a specific vowel.
    ///
    /// # Errors
    ///
    /// Returns `SvaraError::InvalidFormant` if the formants are invalid for the sample rate.
    pub fn set_vowel(&mut self, vowel: Vowel) -> Result<()> {
        let target = VowelTarget::from_vowel(vowel);
        self.set_formants_from_target(&target)
    }

    /// Directly sets formant targets on the tract.
    ///
    /// # Errors
    ///
    /// Returns `SvaraError::InvalidFormant` if the formants are invalid.
    pub fn set_formants(&mut self, formants: &[Formant]) -> Result<()> {
        self.filter = FormantFilter::new(formants, self.sample_rate)?;
        Ok(())
    }

    /// Sets formants from a VowelTarget.
    ///
    /// # Errors
    ///
    /// Returns `SvaraError::InvalidFormant` if the formants are invalid.
    pub fn set_formants_from_target(&mut self, target: &VowelTarget) -> Result<()> {
        let formants = target.to_formants();
        self.set_formants(&formants)
    }

    /// Sets the nasal coupling coefficient (0.0 = oral, 1.0 = fully nasal).
    pub fn set_nasal_coupling(&mut self, coupling: f32) {
        self.nasal_coupling = coupling.clamp(0.0, 1.0);
    }

    /// Sets the lip radiation coefficient (0.0-1.0).
    pub fn set_lip_radiation(&mut self, coefficient: f32) {
        self.lip_radiation = coefficient.clamp(0.0, 1.0);
    }

    /// Sets the nasal resonance characteristics by place of articulation.
    ///
    /// Different nasal consonants (/m/, /n/, /ŋ/) produce anti-formants at
    /// different frequencies. Call this before synthesizing nasal segments.
    pub fn set_nasal_place(&mut self, place: NasalPlace) {
        self.nasal_antiformant.update(
            place.antiformant_frequency(),
            NASAL_ANTIFORMANT_BW,
            self.sample_rate,
        );
    }

    /// Sets the source-filter interaction strength (0.0-0.3).
    ///
    /// Models the effect of vocal tract impedance loading on the glottal source.
    /// Higher values produce more natural-sounding vowels at the cost of
    /// slight spectral modification.
    pub fn set_interaction_strength(&mut self, strength: f32) {
        self.interaction_strength = strength.clamp(0.0, 0.3);
    }

    /// Sets the subglottal resonance coupling strength (0.0-0.2).
    pub fn set_subglottal_coupling(&mut self, strength: f32) {
        self.subglottal_coupling = strength.clamp(0.0, 0.2);
    }

    /// Sets the output gain normalization factor.
    ///
    /// Use this to normalize output levels across different vowel configurations.
    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain.max(0.0);
    }

    /// Returns the sample rate.
    #[must_use]
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Processes a single sample through the vocal tract.
    ///
    /// Includes source-filter interaction: a fraction of the tract output
    /// is fed back to modify the input excitation, modeling the effect of
    /// vocal tract impedance on glottal flow.
    #[inline]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        // Source-filter interaction: modify excitation with tract feedback
        let excitation = input - self.interaction_strength * self.interaction_feedback;

        // Formant filtering
        let formant_out = self.filter.process_sample(excitation);

        // Nasal coupling: blend between oral and nasalized signal
        let output = if self.nasal_coupling > 0.0 {
            let nasal = self.nasal_antiformant.process(formant_out);
            formant_out * (1.0 - self.nasal_coupling) + nasal * self.nasal_coupling
        } else {
            formant_out
        };

        // Subglottal resonance coupling: weak resonance at ~600Hz
        let output = if self.subglottal_coupling > 0.0 {
            let c = &self.sg_coeff;
            let s = &mut self.sg_state;
            let sg_out = c[0] * output + c[1] * s[1] - c[2] * s[2] - c[3] * s[3];
            s[1] = s[0];
            s[0] = output;
            s[3] = s[2];
            s[2] = sg_out;
            output + sg_out * self.subglottal_coupling
        } else {
            output
        };

        // Lip radiation: first-order high-pass (difference filter)
        let radiated = output - self.lip_radiation * self.lip_prev;
        self.lip_prev = output;

        // Store feedback for next sample's source-filter interaction
        self.interaction_feedback = radiated;

        // Apply gain normalization
        radiated * self.gain
    }

    /// Synthesizes a block of samples by piping glottal source through the vocal tract.
    pub fn synthesize(&mut self, glottal: &mut GlottalSource, num_samples: usize) -> Vec<f32> {
        let mut output = vec![0.0; num_samples];
        self.synthesize_into(glottal, &mut output);
        output
    }

    /// Synthesizes into a pre-allocated buffer, avoiding allocation.
    ///
    /// Fills the entire buffer with samples from the glottal source piped
    /// through the vocal tract.
    pub fn synthesize_into(&mut self, glottal: &mut GlottalSource, output: &mut [f32]) {
        for sample in output.iter_mut() {
            let excitation = glottal.next_sample();
            *sample = self.process_sample(excitation);
        }
    }

    /// Resets the vocal tract state (clears filter history).
    pub fn reset(&mut self) {
        self.filter.reset();
        self.nasal_antiformant.reset();
        self.lip_prev = 0.0;
        self.interaction_feedback = 0.0;
        self.sg_state = [0.0; 4];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vocal_tract_creation() {
        let vt = VocalTract::new(44100.0);
        assert!((vt.sample_rate() - 44100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_synthesize() {
        let mut vt = VocalTract::new(44100.0);
        vt.set_vowel(Vowel::A).unwrap();
        let mut glottal = GlottalSource::new(120.0, 44100.0).unwrap();
        let samples = vt.synthesize(&mut glottal, 1024);
        assert_eq!(samples.len(), 1024);
        assert!(samples.iter().all(|s| s.is_finite()));
        // Should produce non-silent output
        assert!(samples.iter().any(|&s| s.abs() > 1e-6));
    }

    #[test]
    fn test_nasal_coupling() {
        let mut vt = VocalTract::new(44100.0);
        vt.set_vowel(Vowel::A).unwrap();
        vt.set_nasal_coupling(0.5);
        let mut glottal = GlottalSource::new(120.0, 44100.0).unwrap();
        let samples = vt.synthesize(&mut glottal, 512);
        assert!(samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn test_reset() {
        let mut vt = VocalTract::new(44100.0);
        let mut glottal = GlottalSource::new(120.0, 44100.0).unwrap();
        let _ = vt.synthesize(&mut glottal, 100);
        vt.reset();
        // After reset, processing silence should produce near-zero output
        let out = vt.process_sample(0.0);
        assert!(out.abs() < 1e-6);
    }
}
