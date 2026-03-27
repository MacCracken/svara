//! Glottal source model using the Rosenberg glottal pulse.
//!
//! The glottal source generates the excitation signal that drives the vocal
//! tract. It models the opening and closing of the vocal folds, with parameters
//! for jitter (frequency perturbation), shimmer (amplitude perturbation), and
//! breathiness (noise mixing).

use serde::{Deserialize, Serialize};
use tracing::trace;

use crate::error::{Result, SvaraError};

/// Default PRNG seed for deterministic noise generation.
const DEFAULT_RNG_SEED: u64 = 42;
/// Default open quotient (fraction of glottal cycle where folds are open).
const DEFAULT_OPEN_QUOTIENT: f32 = 0.6;
/// Default jitter (cycle-to-cycle f0 perturbation fraction).
const DEFAULT_JITTER: f32 = 0.01;
/// Default shimmer (cycle-to-cycle amplitude perturbation fraction).
const DEFAULT_SHIMMER: f32 = 0.02;

/// A simple linear-congruential PRNG for noise generation when naad is not available.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_add(1),
        }
    }

    /// Returns a value in [-1.0, 1.0].
    #[inline]
    fn next_f32(&mut self) -> f32 {
        // LCG parameters from Numerical Recipes
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        // Convert upper bits to float in [-1, 1]
        let bits = (self.state >> 33) as i32;
        bits as f32 / (i32::MAX as f32)
    }

    /// Returns a value in [0.0, 1.0].
    #[inline]
    #[allow(dead_code)]
    fn next_f32_unsigned(&mut self) -> f32 {
        (self.next_f32() + 1.0) * 0.5
    }
}

/// Glottal source generator using the Rosenberg pulse model.
///
/// Produces a periodic glottal waveform with configurable voice quality
/// parameters including jitter, shimmer, and breathiness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlottalSource {
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
    /// Vibrato phase accumulator in radians.
    vibrato_phase: f32,
    /// One-pole spectral tilt filter state (y[n-1]).
    tilt_state: f32,
    /// Current phase within the glottal period [0, period_samples).
    phase: f32,
    /// Current period length in samples (may vary due to jitter).
    current_period: f32,
    /// Amplitude for the current period (may vary due to shimmer).
    current_amplitude: f32,
    /// PRNG for jitter, shimmer, and noise.
    rng: SimpleRng,
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
        if sample_rate <= 0.0 {
            return Err(SvaraError::InvalidFormant(
                "sample_rate must be positive".to_string(),
            ));
        }

        let period = sample_rate / f0;
        trace!(f0, sample_rate, period, "created glottal source");

        Ok(Self {
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
            rng: SimpleRng::new(DEFAULT_RNG_SEED),
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

    /// Sets the vibrato rate in Hz (typically 4-7 Hz for natural singing/speaking).
    pub fn set_vibrato(&mut self, rate: f32, depth: f32) {
        self.vibrato_rate = rate.max(0.0);
        self.vibrato_depth = depth.clamp(0.0, 0.5);
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
    /// The output is a mix of the Rosenberg glottal pulse and noise,
    /// controlled by the breathiness parameter. Spectral tilt is applied
    /// via a one-pole low-pass filter. Vibrato modulates f0 sinusoidally.
    #[inline]
    pub fn next_sample(&mut self) -> f32 {
        let t = self.phase / self.current_period;

        // Rosenberg pulse model
        let pulse = self.rosenberg_pulse(t);

        // Apply spectral tilt via one-pole low-pass: y[n] = (1-α)*x[n] + α*y[n-1]
        // α derived from tilt: higher tilt = more low-pass = breathier voice
        let pulse = if self.spectral_tilt > 0.0 {
            let alpha = (-std::f32::consts::TAU * self.spectral_tilt / self.sample_rate).exp();
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
        if self.vibrato_rate > 0.0 {
            self.vibrato_phase += std::f32::consts::TAU * self.vibrato_rate / self.sample_rate;
            if self.vibrato_phase >= std::f32::consts::TAU {
                self.vibrato_phase -= std::f32::consts::TAU;
            }
        }

        sample
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

    /// Begins a new glottal period, applying jitter, shimmer, and vibrato.
    fn new_period(&mut self) {
        // Apply vibrato: sinusoidal modulation of f0
        let vibrato_mod = if self.vibrato_rate > 0.0 {
            1.0 + self.vibrato_depth * self.vibrato_phase.sin()
        } else {
            1.0
        };
        let effective_f0 = self.f0 * vibrato_mod;

        // Apply jitter to period length
        let base_period = self.sample_rate / effective_f0;
        let jitter_offset = self.rng.next_f32() * self.jitter * base_period;
        self.current_period = (base_period + jitter_offset).max(1.0);

        // Apply shimmer to amplitude
        let shimmer_offset = self.rng.next_f32() * self.shimmer;
        self.current_amplitude = (1.0 + shimmer_offset).max(0.01);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_serde_roundtrip() {
        let gs = GlottalSource::new(150.0, 44100.0).unwrap();
        let json = serde_json::to_string(&gs).unwrap();
        let gs2: GlottalSource = serde_json::from_str(&json).unwrap();
        assert!((gs2.f0() - 150.0).abs() < f32::EPSILON);
    }
}
