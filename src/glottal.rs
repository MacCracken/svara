//! Glottal source models for vocal synthesis.
//!
//! The glottal source generates the excitation signal that drives the vocal
//! tract. Two models are available:
//!
//! - **Rosenberg B**: Simple polynomial pulse (`3t² - 2t³`). Fast and adequate.
//! - **LF (Liljencrants-Fant)**: The standard in speech science. Models the
//!   derivative of glottal flow with an abrupt closure, parameterized by Rd
//!   (a single voice quality dimension from pressed to breathy).
//!
//! Both models support jitter, shimmer, breathiness, and vibrato.

use alloc::format;
use alloc::string::ToString;
use serde::{Deserialize, Serialize};
use tracing::trace;

use crate::error::{Result, SvaraError};
use crate::rng::Rng;
/// Default open quotient (fraction of glottal cycle where folds are open).
const DEFAULT_OPEN_QUOTIENT: f32 = 0.6;
/// Default jitter (cycle-to-cycle f0 perturbation fraction).
const DEFAULT_JITTER: f32 = 0.01;
/// Default shimmer (cycle-to-cycle amplitude perturbation fraction).
const DEFAULT_SHIMMER: f32 = 0.02;

/// Glottal pulse model selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum GlottalModel {
    /// Rosenberg B polynomial pulse: `3t² - 2t³`. Simple, fast.
    Rosenberg,
    /// Liljencrants-Fant model: derivative of glottal flow with abrupt closure.
    /// Parameterized by Rd for voice quality control.
    LF,
    /// Whisper: noise-only excitation with no periodic voicing.
    /// Models breathy, aperiodic airflow through a partially open glottis.
    Whisper,
    /// Creaky voice (vocal fry): irregular glottal pulses with subharmonic
    /// patterns. Very low Rd (pressed), doubled/skipped periods, high shimmer.
    Creaky,
}

/// LF model parameters derived from the Rd voice quality parameter.
///
/// Rd is a single dimension capturing voice quality:
/// - Rd ≈ 0.3: pressed voice (tense, high effort)
/// - Rd ≈ 1.0: modal voice (normal phonation)
/// - Rd ≈ 2.7: breathy voice (relaxed, aspirated)
///
/// Reference: Fant, G. et al. (1985). "A four-parameter model of glottal flow."
/// STL-QPSR 4/1985, 1-13.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct LfParams {
    /// Return phase time constant (controls abruptness of closure).
    ta: f32,
    /// Open quotient equivalent for LF.
    te: f32,
    /// Excitation amplitude (peak negative derivative).
    ee: f32,
}

impl LfParams {
    /// Derives LF parameters from the Rd voice quality parameter.
    ///
    /// Rd mapping follows Fant (1995) simplified parameterization.
    fn from_rd(rd: f32) -> Self {
        let rd = rd.clamp(0.3, 2.7);

        // Empirical mapping from Rd to LF timing parameters
        // te/T0: increases with Rd (more open phase for breathy)
        let te = 0.4 + 0.2 * (rd - 0.3) / 2.4;
        // ta/T0: return phase duration, increases with Rd
        let ta = 0.003 + 0.012 * (rd - 0.3) / 2.4;
        // Excitation strength: decreases with Rd (breathy = weaker closure)
        let ee = 1.0 - 0.3 * (rd - 0.3) / 2.4;

        Self { ta, te, ee }
    }
}

/// Glottal source generator with selectable pulse model.
///
/// Produces a periodic glottal waveform with configurable voice quality
/// parameters including jitter, shimmer, and breathiness. Supports both
/// Rosenberg B and LF (Liljencrants-Fant) pulse models.
///
/// When the `naad-backend` feature is enabled, aspiration noise uses
/// [`naad::noise::NoiseGenerator`] and vibrato uses [`naad::modulation::Lfo`]
/// for higher-quality output. Without the feature, internal PCG32 PRNG and
/// manual sine are used as fallback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlottalSource {
    /// Active glottal model.
    model: GlottalModel,
    /// Rd voice quality parameter for LF model (0.3=pressed, 1.0=modal, 2.7=breathy).
    rd: f32,
    /// Cached LF parameters derived from Rd.
    lf_params: LfParams,
    /// Fundamental frequency in Hz.
    f0: f32,
    /// Audio sample rate in Hz.
    sample_rate: f32,
    /// Open quotient (fraction of cycle where glottis is open). Range: 0.4-0.7.
    open_quotient: f32,
    /// Spectral tilt in dB/octave. Higher values produce breathier voice.
    spectral_tilt: f32,
    /// Jitter: random perturbation of f0, as a fraction (0.0-0.02 typical).
    jitter: f32,
    /// Shimmer: random perturbation of amplitude, as a fraction (0.0-0.04 typical).
    shimmer: f32,
    /// Breathiness: mix ratio of noise (0.0 = pure pulse, 1.0 = pure noise).
    breathiness: f32,
    /// Vibrato rate in Hz (typically 4-7 Hz).
    vibrato_rate: f32,
    /// Vibrato depth as fraction of f0 (0.0-0.1 typical).
    vibrato_depth: f32,
    /// Vibrato phase accumulator in radians (fallback path only).
    vibrato_phase: f32,
    /// One-pole spectral tilt filter state (y[n-1]).
    tilt_state: f32,
    /// Current phase within the glottal period [0, period_samples).
    phase: f32,
    /// Current period length in samples (may vary due to jitter).
    current_period: f32,
    /// Amplitude for the current period (may vary due to shimmer).
    current_amplitude: f32,
    /// PRNG for jitter, shimmer, and aspiration noise (fallback when naad unavailable).
    rng: Rng,
    /// naad white noise generator for aspiration noise.
    #[cfg(feature = "naad-backend")]
    aspiration_noise: naad::noise::NoiseGenerator,
    /// naad LFO for vibrato modulation.
    #[cfg(feature = "naad-backend")]
    vibrato_lfo: naad::modulation::Lfo,
}

impl GlottalSource {
    /// Creates a new glottal source with the given fundamental frequency and sample rate.
    ///
    /// # Errors
    ///
    /// Returns `SvaraError::InvalidPitch` if `f0` is not in the range [20, 2000] Hz.
    /// Returns `SvaraError::InvalidFormant` if `sample_rate` is not positive.
    pub fn new(f0: f32, sample_rate: f32) -> Result<Self> {
        if !(20.0..=2000.0).contains(&f0) {
            return Err(SvaraError::InvalidPitch(format!(
                "f0 must be in [20, 2000] Hz, got {f0}"
            )));
        }
        if sample_rate <= 0.0 || !sample_rate.is_finite() {
            return Err(SvaraError::InvalidFormant(
                "sample_rate must be positive and finite".to_string(),
            ));
        }

        let period = sample_rate / f0;
        trace!(f0, sample_rate, period, "created glottal source");

        Ok(Self {
            model: GlottalModel::Rosenberg,
            rd: 1.0,
            lf_params: LfParams::from_rd(1.0),
            f0,
            sample_rate,
            open_quotient: DEFAULT_OPEN_QUOTIENT,
            spectral_tilt: 0.0,
            jitter: DEFAULT_JITTER,
            shimmer: DEFAULT_SHIMMER,
            breathiness: 0.0,
            vibrato_rate: 0.0,
            vibrato_depth: 0.0,
            vibrato_phase: 0.0,
            tilt_state: 0.0,
            phase: 0.0,
            current_period: period,
            current_amplitude: 1.0,
            rng: Rng::new(crate::rng::DEFAULT_SEED),
            #[cfg(feature = "naad-backend")]
            aspiration_noise: naad::noise::NoiseGenerator::new(naad::noise::NoiseType::White, 42),
            #[cfg(feature = "naad-backend")]
            vibrato_lfo: naad::modulation::Lfo::new(
                naad::modulation::LfoShape::Sine,
                0.001, // Will be updated on first set_vibrato call
                sample_rate,
            )
            .unwrap_or_else(|_| {
                // Fallback: create with minimal valid frequency
                naad::modulation::Lfo::new(naad::modulation::LfoShape::Sine, 0.001, sample_rate)
                    .expect("fallback LFO must succeed")
            }),
        })
    }

    /// Sets the fundamental frequency.
    ///
    /// # Errors
    ///
    /// Returns `SvaraError::InvalidPitch` if `f0` is not in [20, 2000] Hz.
    pub fn set_f0(&mut self, f0: f32) -> Result<()> {
        if !(20.0..=2000.0).contains(&f0) {
            return Err(SvaraError::InvalidPitch(format!(
                "f0 must be in [20, 2000] Hz, got {f0}"
            )));
        }
        self.f0 = f0;
        Ok(())
    }

    /// Sets the breathiness amount (0.0 = pure pulse, 1.0 = pure noise).
    pub fn set_breathiness(&mut self, amount: f32) {
        self.breathiness = amount.clamp(0.0, 1.0);
    }

    /// Sets the open quotient (0.4-0.7).
    pub fn set_open_quotient(&mut self, oq: f32) {
        self.open_quotient = oq.clamp(0.4, 0.7);
    }

    /// Sets the jitter amount (0.0-0.05).
    pub fn set_jitter(&mut self, j: f32) {
        self.jitter = j.clamp(0.0, 0.05);
    }

    /// Sets the shimmer amount (0.0-0.1).
    pub fn set_shimmer(&mut self, s: f32) {
        self.shimmer = s.clamp(0.0, 0.1);
    }

    /// Sets the spectral tilt in dB/octave.
    pub fn set_spectral_tilt(&mut self, tilt: f32) {
        self.spectral_tilt = tilt;
    }

    /// Sets the glottal pulse model.
    pub fn set_model(&mut self, model: GlottalModel) {
        self.model = model;
    }

    /// Returns the current glottal model.
    #[must_use]
    pub fn model(&self) -> GlottalModel {
        self.model
    }

    /// Sets the Rd voice quality parameter for the LF model.
    ///
    /// - Rd ≈ 0.3: pressed voice (tense, high effort)
    /// - Rd ≈ 1.0: modal voice (normal phonation)
    /// - Rd ≈ 2.7: breathy voice (relaxed, aspirated)
    ///
    /// Also switches the model to LF automatically.
    pub fn set_rd(&mut self, rd: f32) {
        self.rd = rd.clamp(0.3, 2.7);
        self.lf_params = LfParams::from_rd(self.rd);
        self.model = GlottalModel::LF;
    }

    /// Returns the current Rd parameter.
    #[must_use]
    pub fn rd(&self) -> f32 {
        self.rd
    }

    /// Switches to whisper mode (noise-only excitation, no periodic voicing).
    ///
    /// In whisper mode, the glottal source produces only shaped noise,
    /// modeling turbulent airflow through a partially open glottis.
    pub fn set_whisper(&mut self) {
        self.model = GlottalModel::Whisper;
    }

    /// Switches to creaky voice (vocal fry) mode.
    ///
    /// Uses the LF model with very low Rd (pressed voice) and irregular
    /// period timing to create subharmonic patterns. Shimmer is amplified
    /// for characteristic amplitude irregularity.
    ///
    /// `rd` is clamped to [0.3, 0.8] — creaky voice is always pressed.
    pub fn set_creaky(&mut self, rd: f32) {
        self.rd = rd.clamp(0.3, 0.8);
        self.lf_params = LfParams::from_rd(self.rd);
        self.model = GlottalModel::Creaky;
    }

    /// Sets the vibrato rate in Hz (typically 4-7 Hz for natural singing/speaking).
    pub fn set_vibrato(&mut self, rate: f32, depth: f32) {
        self.vibrato_rate = rate.max(0.0);
        self.vibrato_depth = depth.clamp(0.0, 0.5);
        #[cfg(feature = "naad-backend")]
        if self.vibrato_rate > 0.0 {
            let _ = self.vibrato_lfo.set_frequency(self.vibrato_rate);
            self.vibrato_lfo.depth = self.vibrato_depth;
        }
    }

    /// Returns the current fundamental frequency.
    #[must_use]
    #[inline]
    pub fn f0(&self) -> f32 {
        self.f0
    }

    /// Returns the sample rate.
    #[must_use]
    #[inline]
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Returns the current period in samples.
    #[must_use]
    #[inline]
    pub fn period_samples(&self) -> f32 {
        self.current_period
    }

    /// Generates the next audio sample from the glottal source.
    ///
    /// The output depends on the active model:
    /// - **Rosenberg/LF**: periodic pulse mixed with aspiration noise
    /// - **Whisper**: noise-only excitation with steep spectral tilt
    /// - **Creaky**: periodic pulse with irregular timing (handled in `new_period`)
    ///
    /// Spectral tilt is applied via a one-pole low-pass filter.
    /// Vibrato modulates f0 sinusoidally (not applied in Whisper mode).
    #[inline]
    pub fn next_sample(&mut self) -> f32 {
        // Whisper mode: pure noise excitation, no periodic pulse
        if self.model == GlottalModel::Whisper {
            return self.whisper_sample();
        }

        let t = self.phase / self.current_period;

        // Compute glottal pulse based on active model
        let pulse = match self.model {
            GlottalModel::Rosenberg => self.rosenberg_pulse(t),
            GlottalModel::LF | GlottalModel::Creaky => self.lf_pulse(t),
            GlottalModel::Whisper => unreachable!(),
        };

        // Apply spectral tilt via one-pole low-pass: y[n] = (1-α)*x[n] + α*y[n-1]
        // α derived from tilt: higher tilt = more low-pass = breathier voice
        let pulse = if self.spectral_tilt > 0.0 {
            let alpha = crate::math::f32::exp(
                -core::f32::consts::TAU * self.spectral_tilt / self.sample_rate,
            );
            let filtered = (1.0 - alpha) * pulse + alpha * self.tilt_state;
            self.tilt_state = filtered;
            filtered
        } else {
            self.tilt_state = pulse;
            pulse
        };

        // Apply shimmer (amplitude variation)
        let pulse = pulse * self.current_amplitude;

        // Mix with aspiration noise for breathiness.
        // Noise is gated by the glottal open phase: full noise during the open
        // phase, reduced during closed phase (models turbulent airflow at glottis).
        #[cfg(feature = "naad-backend")]
        let noise = self.aspiration_noise.next_sample();
        #[cfg(not(feature = "naad-backend"))]
        let noise = self.rng.next_f32();
        let noise_gate = if t < self.open_quotient { 1.0 } else { 0.1 };
        let sample = pulse * (1.0 - self.breathiness) + noise * self.breathiness * 0.3 * noise_gate;

        // Advance phase
        self.phase += 1.0;
        if self.phase >= self.current_period {
            self.phase -= self.current_period;
            self.new_period();
        }

        // Advance vibrato phase
        #[cfg(not(feature = "naad-backend"))]
        if self.vibrato_rate > 0.0 {
            self.vibrato_phase += core::f32::consts::TAU * self.vibrato_rate / self.sample_rate;
            if self.vibrato_phase >= core::f32::consts::TAU {
                self.vibrato_phase -= core::f32::consts::TAU;
            }
        }

        sample
    }

    /// Generates a whisper sample: noise-only excitation with steep spectral tilt.
    ///
    /// Whisper is modeled as turbulent airflow through a partially open glottis
    /// with no periodic voicing. Spectral tilt is applied at ~12 dB/octave to
    /// shape the characteristic whisper spectrum (energy concentrated in lower
    /// frequencies).
    #[inline]
    fn whisper_sample(&mut self) -> f32 {
        #[cfg(feature = "naad-backend")]
        let noise = self.aspiration_noise.next_sample();
        #[cfg(not(feature = "naad-backend"))]
        let noise = self.rng.next_f32();

        // Steep spectral tilt (~12 dB/octave) for whisper spectrum shaping.
        // The one-pole filter integrates noise, so input is scaled down to keep
        // whisper energy well below voiced phonation.
        let tilt = self.spectral_tilt.max(12.0);
        let alpha = crate::math::f32::exp(-core::f32::consts::TAU * tilt / self.sample_rate);
        let filtered = (1.0 - alpha) * noise * 0.15 + alpha * self.tilt_state;
        self.tilt_state = filtered;
        filtered
    }

    /// Computes the Rosenberg B glottal pulse at normalized time t in `[0, 1)`.
    ///
    /// During the open phase (t < open_quotient), the pulse follows the
    /// standard Rosenberg B polynomial: `3t² - 2t³`, which smoothly rises
    /// from 0 to 1 and back to 0. During the closed phase, output is 0.
    ///
    /// Reference: Rosenberg, A.E. (1971). "Effect of Glottal Pulse Shape on
    /// the Quality of Natural Vowels." JASA 49(2B), 583-590.
    #[inline]
    fn rosenberg_pulse(&self, t: f32) -> f32 {
        let oq = self.open_quotient;
        if t < oq {
            // Open phase: Rosenberg B polynomial 3t² - 2t³
            let t_norm = t / oq;
            3.0 * t_norm * t_norm - 2.0 * t_norm * t_norm * t_norm
        } else {
            // Closed phase: glottis is closed, no airflow
            0.0
        }
    }

    /// Computes the LF (Liljencrants-Fant) glottal flow derivative at normalized time t.
    ///
    /// The LF model produces the derivative of glottal airflow, which is the
    /// actual excitation signal seen by the vocal tract. It features:
    /// - A smooth open phase with sinusoidal rise
    /// - An abrupt closure (the main source of acoustic excitation)
    /// - A brief return phase (exponential recovery)
    ///
    /// The Rd parameter controls voice quality by adjusting the relative timing
    /// and amplitude of these phases.
    #[inline]
    fn lf_pulse(&self, t: f32) -> f32 {
        let te = self.lf_params.te;
        let ta = self.lf_params.ta;
        let ee = self.lf_params.ee;

        if t < te {
            // Open phase: sinusoidal rise to peak excitation at t=te
            // E(t) = -Ee * sin(π * t / te)
            let phase = core::f32::consts::PI * t / te;
            -ee * crate::math::f32::sin(phase)
        } else if t < te + ta {
            // Return phase: exponential recovery after closure
            // E(t) = -Ee * exp(-(t - te) / epsilon) where epsilon ≈ ta
            let dt = t - te;
            let epsilon = ta.max(0.001);
            -ee * crate::math::f32::exp(-dt / epsilon)
        } else {
            // Closed phase: no airflow
            0.0
        }
    }

    /// Begins a new glottal period, applying jitter, shimmer, and vibrato.
    ///
    /// In creaky voice mode, the period is irregularly doubled or tripled to
    /// create subharmonic patterns characteristic of vocal fry.
    fn new_period(&mut self) {
        // Apply vibrato: sinusoidal modulation of f0
        let vibrato_mod = if self.vibrato_rate > 0.0 {
            #[cfg(feature = "naad-backend")]
            {
                // naad LFO outputs 0.0-1.0 (unipolar), center at 0.5
                let lfo_val = self.vibrato_lfo.next_value();
                1.0 + self.vibrato_depth * (lfo_val * 2.0 - 1.0)
            }
            #[cfg(not(feature = "naad-backend"))]
            {
                1.0 + self.vibrato_depth * crate::math::f32::sin(self.vibrato_phase)
            }
        } else {
            1.0
        };
        let effective_f0 = self.f0 * vibrato_mod;

        // Apply jitter to period length
        let base_period = self.sample_rate / effective_f0;
        let jitter_offset = self.rng.next_f32() * self.jitter * base_period;
        let mut period = (base_period + jitter_offset).max(1.0);

        // Creaky voice: irregular period multiplication for subharmonic patterns.
        // ~40% chance of doubled period, ~10% chance of tripled period.
        // This creates the characteristic irregular pulse train of vocal fry.
        if self.model == GlottalModel::Creaky {
            let r = self.rng.next_f32().abs();
            if r < 0.10 {
                period *= 3.0;
            } else if r < 0.40 {
                period *= 2.0;
            }
        }

        self.current_period = period;

        // Apply shimmer to amplitude.
        // Creaky voice uses higher amplitude irregularity (3x shimmer).
        let shimmer_scale = if self.model == GlottalModel::Creaky {
            self.shimmer * 3.0
        } else {
            self.shimmer
        };
        let shimmer_offset = self.rng.next_f32() * shimmer_scale;
        self.current_amplitude = (1.0 + shimmer_offset).max(0.01);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    #[test]
    fn test_glottal_source_creation() {
        let gs = GlottalSource::new(120.0, 44100.0);
        assert!(gs.is_ok());
        let gs = gs.unwrap();
        assert!((gs.f0() - 120.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_invalid_f0() {
        assert!(GlottalSource::new(5.0, 44100.0).is_err());
        assert!(GlottalSource::new(3000.0, 44100.0).is_err());
    }

    #[test]
    fn test_generates_samples() {
        let mut gs = GlottalSource::new(120.0, 44100.0).unwrap();
        let samples: Vec<f32> = (0..1024).map(|_| gs.next_sample()).collect();
        // Should produce non-zero output
        assert!(samples.iter().any(|&s| s.abs() > 0.001));
        // Should not produce NaN or infinity
        assert!(samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn test_period_approximately_correct() {
        let mut gs = GlottalSource::new(120.0, 44100.0).unwrap();
        gs.set_jitter(0.0);
        // Period at 120Hz with 44100 sample rate ≈ 367.5 samples
        let expected_period = 44100.0 / 120.0;
        assert!((gs.period_samples() - expected_period).abs() < 1.0);
    }

    #[test]
    fn test_breathiness() {
        let mut gs = GlottalSource::new(120.0, 44100.0).unwrap();
        gs.set_breathiness(1.0);
        let samples: Vec<f32> = (0..1024).map(|_| gs.next_sample()).collect();
        assert!(samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn test_lf_model_produces_output() {
        let mut gs = GlottalSource::new(120.0, 44100.0).unwrap();
        gs.set_model(GlottalModel::LF);
        let samples: Vec<f32> = (0..1024).map(|_| gs.next_sample()).collect();
        assert!(samples.iter().any(|&s| s.abs() > 0.001));
        assert!(samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn test_rd_parameterization() {
        // Pressed voice (low Rd) should have stronger excitation than breathy (high Rd)
        let mut pressed = GlottalSource::new(120.0, 44100.0).unwrap();
        pressed.set_rd(0.3);
        pressed.set_jitter(0.0);
        pressed.set_shimmer(0.0);
        let pressed_samples: Vec<f32> = (0..4410).map(|_| pressed.next_sample()).collect();
        let pressed_energy: f32 = pressed_samples.iter().map(|s| s * s).sum();

        let mut breathy = GlottalSource::new(120.0, 44100.0).unwrap();
        breathy.set_rd(2.7);
        breathy.set_jitter(0.0);
        breathy.set_shimmer(0.0);
        let breathy_samples: Vec<f32> = (0..4410).map(|_| breathy.next_sample()).collect();
        let breathy_energy: f32 = breathy_samples.iter().map(|s| s * s).sum();

        assert!(
            pressed_energy > breathy_energy,
            "pressed voice should have more energy: pressed={pressed_energy}, breathy={breathy_energy}"
        );
    }

    #[test]
    fn test_rd_auto_switches_to_lf() {
        let mut gs = GlottalSource::new(120.0, 44100.0).unwrap();
        assert_eq!(gs.model(), GlottalModel::Rosenberg);
        gs.set_rd(1.0);
        assert_eq!(gs.model(), GlottalModel::LF);
    }

    #[test]
    fn test_serde_roundtrip() {
        let gs = GlottalSource::new(150.0, 44100.0).unwrap();
        let json = serde_json::to_string(&gs).unwrap();
        let gs2: GlottalSource = serde_json::from_str(&json).unwrap();
        assert!((gs2.f0() - 150.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_whisper_produces_output() {
        let mut gs = GlottalSource::new(120.0, 44100.0).unwrap();
        gs.set_whisper();
        assert_eq!(gs.model(), GlottalModel::Whisper);
        let samples: Vec<f32> = (0..4410).map(|_| gs.next_sample()).collect();
        assert!(samples.iter().all(|s| s.is_finite()));
        assert!(samples.iter().any(|&s| s.abs() > 1e-6));
    }

    #[test]
    fn test_whisper_differs_from_voiced() {
        // Whisper and voiced phonation at the same f0 should produce
        // spectrally distinct signals. We compare RMS amplitude — whisper
        // is shaped noise and should be significantly quieter than voiced.
        let mut voiced = GlottalSource::new(120.0, 44100.0).unwrap();
        let v_rms: f32 = {
            let samples: Vec<f32> = (0..4410).map(|_| voiced.next_sample()).collect();
            let energy: f32 = samples.iter().map(|s| s * s).sum();
            crate::math::f32::sqrt(energy / samples.len() as f32)
        };

        let mut whisper = GlottalSource::new(120.0, 44100.0).unwrap();
        whisper.set_whisper();
        let w_rms: f32 = {
            let samples: Vec<f32> = (0..4410).map(|_| whisper.next_sample()).collect();
            let energy: f32 = samples.iter().map(|s| s * s).sum();
            crate::math::f32::sqrt(energy / samples.len() as f32)
        };

        // Whisper should be substantially quieter
        assert!(
            w_rms < v_rms * 0.8,
            "whisper should be quieter than voiced: whisper_rms={w_rms}, voiced_rms={v_rms}"
        );
    }

    #[test]
    fn test_creaky_produces_output() {
        let mut gs = GlottalSource::new(120.0, 44100.0).unwrap();
        gs.set_creaky(0.5);
        assert_eq!(gs.model(), GlottalModel::Creaky);
        assert!((gs.rd() - 0.5).abs() < f32::EPSILON);
        let samples: Vec<f32> = (0..4410).map(|_| gs.next_sample()).collect();
        assert!(samples.iter().all(|s| s.is_finite()));
        assert!(samples.iter().any(|&s| s.abs() > 0.001));
    }

    #[test]
    fn test_creaky_rd_clamped() {
        let mut gs = GlottalSource::new(120.0, 44100.0).unwrap();
        gs.set_creaky(2.0); // Should clamp to 0.8
        assert!((gs.rd() - 0.8).abs() < f32::EPSILON);
        gs.set_creaky(0.1); // Should clamp to 0.3
        assert!((gs.rd() - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_creaky_has_irregular_periods() {
        // Creaky voice should have more period variation than modal voice
        let mut creaky = GlottalSource::new(80.0, 44100.0).unwrap();
        creaky.set_creaky(0.4);
        creaky.set_jitter(0.01);
        // Run enough samples to trigger multiple periods
        let _: Vec<f32> = (0..44100).map(|_| creaky.next_sample()).collect();
        // The test passes if no panics — period doubling/tripling doesn't destabilize
    }

    #[test]
    fn test_whisper_lower_energy_than_voiced() {
        let mut whisper = GlottalSource::new(120.0, 44100.0).unwrap();
        whisper.set_whisper();
        let w_samples: Vec<f32> = (0..4410).map(|_| whisper.next_sample()).collect();
        let w_energy: f32 = w_samples.iter().map(|s| s * s).sum();

        let mut voiced = GlottalSource::new(120.0, 44100.0).unwrap();
        let v_samples: Vec<f32> = (0..4410).map(|_| voiced.next_sample()).collect();
        let v_energy: f32 = v_samples.iter().map(|s| s * s).sum();

        assert!(
            w_energy < v_energy,
            "whisper should have less energy: whisper={w_energy}, voiced={v_energy}"
        );
    }

    #[test]
    fn test_serde_roundtrip_whisper() {
        let mut gs = GlottalSource::new(120.0, 44100.0).unwrap();
        gs.set_whisper();
        let json = serde_json::to_string(&gs).unwrap();
        let gs2: GlottalSource = serde_json::from_str(&json).unwrap();
        assert_eq!(gs2.model(), GlottalModel::Whisper);
    }

    #[test]
    fn test_serde_roundtrip_creaky() {
        let mut gs = GlottalSource::new(120.0, 44100.0).unwrap();
        gs.set_creaky(0.5);
        let json = serde_json::to_string(&gs).unwrap();
        let gs2: GlottalSource = serde_json::from_str(&json).unwrap();
        assert_eq!(gs2.model(), GlottalModel::Creaky);
        assert!((gs2.rd() - 0.5).abs() < f32::EPSILON);
    }
}
