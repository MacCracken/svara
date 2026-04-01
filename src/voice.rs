//! Voice profiles defining speaker characteristics.
//!
//! A `VoiceProfile` captures the acoustic parameters that distinguish one
//! speaker from another: fundamental frequency, formant scaling, breathiness,
//! vibrato, and micro-perturbations (jitter/shimmer).

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::formant::VowelTarget;
use crate::glottal::GlottalSource;

/// A speaker's voice characteristics.
///
/// Use the preset constructors (`new_male`, `new_female`, `new_child`) or the
/// builder methods to create custom voices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceProfile {
    /// Base fundamental frequency in Hz.
    pub base_f0: f32,
    /// F0 range (maximum deviation from base_f0) in Hz.
    pub f0_range: f32,
    /// Formant frequency scaling factor (1.0 = adult male reference).
    pub formant_scale: f32,
    /// Breathiness amount (0.0 = clear, 1.0 = very breathy).
    pub breathiness: f32,
    /// Vibrato rate in Hz (typically ~5 Hz).
    pub vibrato_rate: f32,
    /// Vibrato depth as fraction of f0 (typically ~0.05 = 5%).
    pub vibrato_depth: f32,
    /// Jitter: cycle-to-cycle f0 perturbation (fraction, typically 0.01-0.02).
    pub jitter: f32,
    /// Shimmer: cycle-to-cycle amplitude perturbation (fraction, typically 0.02-0.04).
    pub shimmer: f32,
    /// Extra bandwidth widening factor for singing at high f0.
    ///
    /// Applied multiplicatively when `base_f0 > 300 Hz`. At 0.0 only the
    /// standard `sqrt(f0/120)` scaling is used. At 1.0, an additional
    /// `1 + 0.3 * ((f0 - 300) / 500)` factor is applied. Default: 0.0.
    #[serde(default)]
    pub bandwidth_widening: f32,
}

impl VoiceProfile {
    /// Creates a typical adult male voice profile.
    ///
    /// f0 = 120 Hz, formant_scale = 1.0 (reference).
    #[must_use]
    pub fn new_male() -> Self {
        Self {
            base_f0: 120.0,
            f0_range: 40.0,
            formant_scale: 1.0,
            breathiness: 0.02,
            vibrato_rate: 5.0,
            vibrato_depth: 0.04,
            jitter: 0.01,
            shimmer: 0.02,
            bandwidth_widening: 0.0,
        }
    }

    /// Creates a typical adult female voice profile.
    ///
    /// f0 = 220 Hz, formant_scale = 1.17 (shorter vocal tract).
    #[must_use]
    pub fn new_female() -> Self {
        Self {
            base_f0: 220.0,
            f0_range: 50.0,
            formant_scale: 1.17,
            breathiness: 0.05,
            vibrato_rate: 5.5,
            vibrato_depth: 0.05,
            jitter: 0.008,
            shimmer: 0.018,
            bandwidth_widening: 0.0,
        }
    }

    /// Creates a typical child voice profile.
    ///
    /// f0 = 300 Hz, formant_scale = 1.3 (even shorter vocal tract).
    #[must_use]
    pub fn new_child() -> Self {
        Self {
            base_f0: 300.0,
            f0_range: 60.0,
            formant_scale: 1.3,
            breathiness: 0.03,
            vibrato_rate: 6.0,
            vibrato_depth: 0.03,
            jitter: 0.012,
            shimmer: 0.025,
            bandwidth_widening: 0.0,
        }
    }

    /// Sets the base fundamental frequency (builder pattern).
    #[must_use]
    pub fn with_f0(mut self, f0: f32) -> Self {
        self.base_f0 = f0;
        self
    }

    /// Sets the breathiness amount (builder pattern).
    #[must_use]
    pub fn with_breathiness(mut self, b: f32) -> Self {
        self.breathiness = b.clamp(0.0, 1.0);
        self
    }

    /// Sets the vibrato rate in Hz (builder pattern).
    #[must_use]
    pub fn with_vibrato_rate(mut self, rate: f32) -> Self {
        self.vibrato_rate = rate.max(0.0);
        self
    }

    /// Sets the vibrato depth as fraction of f0 (builder pattern).
    #[must_use]
    pub fn with_vibrato_depth(mut self, depth: f32) -> Self {
        self.vibrato_depth = depth.clamp(0.0, 0.5);
        self
    }

    /// Sets the jitter amount (builder pattern).
    #[must_use]
    pub fn with_jitter(mut self, j: f32) -> Self {
        self.jitter = j.clamp(0.0, 0.05);
        self
    }

    /// Sets the shimmer amount (builder pattern).
    #[must_use]
    pub fn with_shimmer(mut self, s: f32) -> Self {
        self.shimmer = s.clamp(0.0, 0.1);
        self
    }

    /// Sets the formant scaling factor (builder pattern).
    #[must_use]
    pub fn with_formant_scale(mut self, scale: f32) -> Self {
        self.formant_scale = scale.max(0.1);
        self
    }

    /// Sets the f0 range (builder pattern).
    #[must_use]
    pub fn with_f0_range(mut self, range: f32) -> Self {
        self.f0_range = range.max(0.0);
        self
    }

    /// Sets the extra bandwidth widening factor for singing (builder pattern).
    ///
    /// At 0.0, only the standard f0-based bandwidth scaling is applied.
    /// At 1.0, formant bandwidths are additionally widened at high f0 (>300 Hz)
    /// to model increased source-tract coupling in singing.
    #[must_use]
    pub fn with_bandwidth_widening(mut self, factor: f32) -> Self {
        self.bandwidth_widening = factor.clamp(0.0, 2.0);
        self
    }

    /// Creates a [`GlottalSource`] configured with this voice profile's parameters.
    ///
    /// Sets f0, breathiness, jitter, shimmer, and vibrato from the profile.
    ///
    /// # Errors
    ///
    /// Returns an error if `base_f0` is outside the valid range.
    pub fn create_glottal_source(&self, sample_rate: f32) -> Result<GlottalSource> {
        let mut gs = GlottalSource::new(self.base_f0, sample_rate)?;
        gs.set_breathiness(self.breathiness);
        gs.set_jitter(self.jitter);
        gs.set_shimmer(self.shimmer);
        gs.set_vibrato(self.vibrato_rate, self.vibrato_depth);
        Ok(gs)
    }

    /// Applies formant frequency and bandwidth scaling to a vowel target.
    ///
    /// Frequencies are scaled by `formant_scale` (modeling vocal tract length).
    /// Bandwidths are scaled by `sqrt(base_f0 / 120.0)` — higher f0 voices
    /// (female, child) have wider bandwidths due to increased source-tract coupling.
    ///
    /// When `bandwidth_widening > 0` and `base_f0 > 300 Hz`, an additional
    /// widening factor is applied to model the increased source-tract coupling
    /// in singing at high pitches.
    #[must_use]
    pub fn apply_formant_scale(&self, target: &VowelTarget) -> VowelTarget {
        // Bandwidth scaling: sqrt(f0 / male_reference_f0)
        let mut bw_scale = crate::math::f32::sqrt(self.base_f0 / 120.0);

        // Additional singing bandwidth widening at high f0
        if self.bandwidth_widening > 0.0 && self.base_f0 > 300.0 {
            let extra = 1.0 + 0.3 * ((self.base_f0 - 300.0) / 500.0);
            // Blend between 1.0 (no extra) and `extra` by the widening factor
            bw_scale *= 1.0 + self.bandwidth_widening * (extra - 1.0);
        }

        VowelTarget::with_bandwidths(
            [
                target.f1 * self.formant_scale,
                target.f2 * self.formant_scale,
                target.f3 * self.formant_scale,
                target.f4 * self.formant_scale,
                target.f5 * self.formant_scale,
            ],
            [
                target.b1 * bw_scale,
                target.b2 * bw_scale,
                target.b3 * bw_scale,
                target.b4 * bw_scale,
                target.b5 * bw_scale,
            ],
        )
    }
}

impl Default for VoiceProfile {
    fn default() -> Self {
        Self::new_male()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_male_preset() {
        let v = VoiceProfile::new_male();
        assert!((v.base_f0 - 120.0).abs() < f32::EPSILON);
        assert!((v.formant_scale - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_female_preset() {
        let v = VoiceProfile::new_female();
        assert!((v.base_f0 - 220.0).abs() < f32::EPSILON);
        assert!((v.formant_scale - 1.17).abs() < f32::EPSILON);
    }

    #[test]
    fn test_child_preset() {
        let v = VoiceProfile::new_child();
        assert!((v.base_f0 - 300.0).abs() < f32::EPSILON);
        assert!((v.formant_scale - 1.3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_builder_pattern() {
        let v = VoiceProfile::new_male()
            .with_f0(150.0)
            .with_breathiness(0.3)
            .with_vibrato_rate(6.0);
        assert!((v.base_f0 - 150.0).abs() < f32::EPSILON);
        assert!((v.breathiness - 0.3).abs() < f32::EPSILON);
        assert!((v.vibrato_rate - 6.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_formant_scaling() {
        let v = VoiceProfile::new_female();
        let target = crate::formant::VowelTarget::from_vowel(crate::formant::Vowel::A);
        let scaled = v.apply_formant_scale(&target);
        assert!((scaled.f1 - target.f1 * 1.17).abs() < 0.01);
        assert!((scaled.f2 - target.f2 * 1.17).abs() < 0.01);
    }

    #[test]
    fn test_clamping() {
        let v = VoiceProfile::new_male().with_breathiness(5.0);
        assert!((v.breathiness - 1.0).abs() < f32::EPSILON);
        let v = VoiceProfile::new_male().with_breathiness(-1.0);
        assert!(v.breathiness.abs() < f32::EPSILON);
    }

    #[test]
    fn test_serde_roundtrip() {
        let v = VoiceProfile::new_female().with_f0(210.0);
        let json = serde_json::to_string(&v).unwrap();
        let v2: VoiceProfile = serde_json::from_str(&json).unwrap();
        assert!((v2.base_f0 - 210.0).abs() < f32::EPSILON);
        assert!((v2.formant_scale - 1.17).abs() < f32::EPSILON);
    }

    #[test]
    fn test_default() {
        let v = VoiceProfile::default();
        assert!((v.base_f0 - 120.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_bandwidth_widening_default_zero() {
        let v = VoiceProfile::new_male();
        assert!(v.bandwidth_widening.abs() < f32::EPSILON);
    }

    #[test]
    fn test_bandwidth_widening_no_effect_below_300hz() {
        let target = crate::formant::VowelTarget::from_vowel(crate::formant::Vowel::A);

        let v_no_widen = VoiceProfile::new_male()
            .with_f0(200.0)
            .with_bandwidth_widening(1.0);
        let v_normal = VoiceProfile::new_male().with_f0(200.0);

        let scaled_widen = v_no_widen.apply_formant_scale(&target);
        let scaled_normal = v_normal.apply_formant_scale(&target);

        // Below 300Hz, widening should have no effect
        assert!((scaled_widen.b1 - scaled_normal.b1).abs() < f32::EPSILON);
    }

    #[test]
    fn test_bandwidth_widening_increases_above_300hz() {
        let target = crate::formant::VowelTarget::from_vowel(crate::formant::Vowel::A);

        let v_no_widen = VoiceProfile::new_female()
            .with_f0(500.0)
            .with_bandwidth_widening(0.0);
        let v_widen = VoiceProfile::new_female()
            .with_f0(500.0)
            .with_bandwidth_widening(1.0);

        let scaled_normal = v_no_widen.apply_formant_scale(&target);
        let scaled_wide = v_widen.apply_formant_scale(&target);

        // With widening enabled at high f0, bandwidths should be wider
        assert!(
            scaled_wide.b1 > scaled_normal.b1,
            "bandwidth widening should increase B1: normal={}, wide={}",
            scaled_normal.b1,
            scaled_wide.b1
        );
    }

    #[test]
    fn test_bandwidth_widening_scales_with_f0() {
        let target = crate::formant::VowelTarget::from_vowel(crate::formant::Vowel::A);

        let v_400 = VoiceProfile::new_female()
            .with_f0(400.0)
            .with_bandwidth_widening(1.0);
        let v_800 = VoiceProfile::new_female()
            .with_f0(800.0)
            .with_bandwidth_widening(1.0);

        let scaled_400 = v_400.apply_formant_scale(&target);
        let scaled_800 = v_800.apply_formant_scale(&target);

        // Higher f0 should produce even wider bandwidths
        assert!(
            scaled_800.b1 > scaled_400.b1,
            "higher f0 should widen more: 400Hz={}, 800Hz={}",
            scaled_400.b1,
            scaled_800.b1
        );
    }

    #[test]
    fn test_bandwidth_widening_clamped() {
        let v = VoiceProfile::new_male().with_bandwidth_widening(5.0);
        assert!((v.bandwidth_widening - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_serde_roundtrip_with_bandwidth_widening() {
        let v = VoiceProfile::new_female().with_bandwidth_widening(0.8);
        let json = serde_json::to_string(&v).unwrap();
        let v2: VoiceProfile = serde_json::from_str(&json).unwrap();
        assert!((v2.bandwidth_widening - 0.8).abs() < f32::EPSILON);
    }
}
