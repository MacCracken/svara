//! Voice profiles defining speaker characteristics.
//!
//! A `VoiceProfile` captures the acoustic parameters that distinguish one
//! speaker from another: fundamental frequency, formant scaling, breathiness,
//! vibrato, and micro-perturbations (jitter/shimmer).

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::formant::VowelTarget;
use crate::glottal::{GlottalModel, GlottalSource};

/// Vocal effort level controlling coordinated voice quality parameters.
///
/// Maps to a consistent set of glottal model, Rd, breathiness, spectral tilt,
/// and f0 range settings that model the physical changes in phonation from
/// whisper through shouting.
///
/// Reference: Traunmüller & Eriksson (2000), "Acoustic effects of variation
/// in vocal effort by men, women, and children."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum VocalEffort {
    /// Whisper: noise-only excitation, no voicing.
    Whisper,
    /// Soft/breathy: voiced with high Rd, moderate breathiness.
    Soft,
    /// Normal modal phonation.
    Normal,
    /// Loud: pressed voice with low Rd, wider f0 range.
    Loud,
    /// Shout: very pressed, maximum f0 range and spectral energy.
    Shout,
}

/// Parameters derived from a [`VocalEffort`] level.
///
/// These are the coordinated acoustic changes that happen together when
/// a speaker changes vocal effort.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EffortParams {
    /// Glottal model to use.
    pub model: GlottalModel,
    /// Rd voice quality parameter (only meaningful for LF/Creaky models).
    pub rd: f32,
    /// Breathiness amount (0.0-1.0).
    pub breathiness: f32,
    /// Spectral tilt in dB/octave.
    pub spectral_tilt: f32,
    /// F0 range multiplier (1.0 = unchanged).
    pub f0_range_scale: f32,
    /// Formant bandwidth multiplier (>1.0 = wider).
    pub bandwidth_scale: f32,
    /// Jitter multiplier (relative to profile's base jitter).
    pub jitter_scale: f32,
    /// Shimmer multiplier (relative to profile's base shimmer).
    pub shimmer_scale: f32,
}

impl VocalEffort {
    /// Returns the coordinated acoustic parameters for this effort level.
    #[must_use]
    pub fn params(self) -> EffortParams {
        match self {
            Self::Whisper => EffortParams {
                model: GlottalModel::Whisper,
                rd: 2.7,
                breathiness: 1.0,
                spectral_tilt: 12.0,
                f0_range_scale: 0.5,
                bandwidth_scale: 1.3,
                jitter_scale: 0.0, // no voicing → no jitter
                shimmer_scale: 0.0,
            },
            Self::Soft => EffortParams {
                model: GlottalModel::LF,
                rd: 2.0,
                breathiness: 0.15,
                spectral_tilt: 6.0,
                f0_range_scale: 0.8,
                bandwidth_scale: 1.1,
                jitter_scale: 1.2,
                shimmer_scale: 1.2,
            },
            Self::Normal => EffortParams {
                model: GlottalModel::LF,
                rd: 1.0,
                breathiness: 0.02,
                spectral_tilt: 2.0,
                f0_range_scale: 1.0,
                bandwidth_scale: 1.0,
                jitter_scale: 1.0,
                shimmer_scale: 1.0,
            },
            Self::Loud => EffortParams {
                model: GlottalModel::LF,
                rd: 0.6,
                breathiness: 0.0,
                spectral_tilt: 0.0,
                f0_range_scale: 1.3,
                bandwidth_scale: 0.9,
                jitter_scale: 0.8,
                shimmer_scale: 0.8,
            },
            Self::Shout => EffortParams {
                model: GlottalModel::LF,
                rd: 0.3,
                breathiness: 0.0,
                // Negative tilt = spectral emphasis. Currently the one-pole
                // filter in GlottalSource only applies positive tilt; the
                // pressed LF pulse (Rd=0.3) naturally has strong HF content.
                spectral_tilt: 0.0,
                f0_range_scale: 1.6,
                bandwidth_scale: 0.8,
                jitter_scale: 0.5,
                shimmer_scale: 0.5,
            },
        }
    }
}

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

    /// Applies a [`VocalEffort`] level, overriding breathiness, jitter, shimmer,
    /// and f0 range to match the effort level while preserving the speaker's
    /// base f0 and formant scale (builder pattern).
    ///
    /// This modifies the profile in place. The resulting profile will produce
    /// the correct glottal model and parameters when passed to
    /// [`create_glottal_source_with_effort`](Self::create_glottal_source_with_effort).
    #[must_use]
    pub fn with_effort(mut self, effort: VocalEffort) -> Self {
        let p = effort.params();
        self.breathiness = p.breathiness;
        self.jitter *= p.jitter_scale;
        self.shimmer *= p.shimmer_scale;
        self.f0_range *= p.f0_range_scale;
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

    /// Creates a [`GlottalSource`] configured for a specific vocal effort level.
    ///
    /// Applies the effort's glottal model, Rd, spectral tilt, and scaled
    /// jitter/shimmer/breathiness on top of this profile's base parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if `base_f0` is outside the valid range.
    pub fn create_glottal_source_with_effort(
        &self,
        effort: VocalEffort,
        sample_rate: f32,
    ) -> Result<GlottalSource> {
        let p = effort.params();
        let mut gs = GlottalSource::new(self.base_f0, sample_rate)?;
        gs.set_breathiness(p.breathiness);
        gs.set_jitter(self.jitter * p.jitter_scale);
        gs.set_shimmer(self.shimmer * p.shimmer_scale);
        gs.set_spectral_tilt(p.spectral_tilt);
        gs.set_vibrato(self.vibrato_rate, self.vibrato_depth);

        match p.model {
            GlottalModel::Whisper => gs.set_whisper(),
            GlottalModel::Creaky => gs.set_creaky(p.rd),
            GlottalModel::LF => gs.set_rd(p.rd),
            GlottalModel::Rosenberg => gs.set_model(GlottalModel::Rosenberg),
        }

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

    /// Applies formant scaling with an additional effort-dependent bandwidth factor.
    ///
    /// Loud/shout effort narrows bandwidths (more precise articulation), while
    /// whisper/soft widens them (more diffuse resonances).
    #[must_use]
    pub fn apply_formant_scale_with_effort(
        &self,
        target: &VowelTarget,
        effort: VocalEffort,
    ) -> VowelTarget {
        let base = self.apply_formant_scale(target);
        let effort_bw = effort.params().bandwidth_scale;
        VowelTarget::with_bandwidths(
            [base.f1, base.f2, base.f3, base.f4, base.f5],
            [
                base.b1 * effort_bw,
                base.b2 * effort_bw,
                base.b3 * effort_bw,
                base.b4 * effort_bw,
                base.b5 * effort_bw,
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

    // --- VocalEffort tests ---

    #[test]
    fn test_effort_params_whisper() {
        let p = VocalEffort::Whisper.params();
        assert_eq!(p.model, crate::glottal::GlottalModel::Whisper);
        assert!((p.breathiness - 1.0).abs() < f32::EPSILON);
        assert!(p.spectral_tilt > 10.0);
    }

    #[test]
    fn test_effort_params_shout() {
        let p = VocalEffort::Shout.params();
        assert_eq!(p.model, crate::glottal::GlottalModel::LF);
        assert!((p.rd - 0.3).abs() < f32::EPSILON);
        assert!(p.breathiness < f32::EPSILON);
        assert!(p.spectral_tilt < f32::EPSILON);
    }

    #[test]
    fn test_effort_energy_extremes() {
        // Whisper should have less energy than Shout. The intermediate levels
        // may not be strictly monotonic (Soft can exceed Normal due to noise).
        let voice = VoiceProfile::new_male();

        let energy_for = |effort: VocalEffort| -> f32 {
            let mut gs = voice
                .create_glottal_source_with_effort(effort, 44100.0)
                .unwrap();
            let samples: alloc::vec::Vec<f32> = (0..4410).map(|_| gs.next_sample()).collect();
            samples.iter().map(|s| s * s).sum()
        };

        let e_whisper = energy_for(VocalEffort::Whisper);
        let e_normal = energy_for(VocalEffort::Normal);
        let e_shout = energy_for(VocalEffort::Shout);

        assert!(
            e_whisper < e_normal,
            "whisper should have less energy than normal: {e_whisper} < {e_normal}"
        );
        assert!(
            e_normal < e_shout,
            "normal should have less energy than shout: {e_normal} < {e_shout}"
        );
    }

    #[test]
    fn test_effort_all_produce_finite_output() {
        let voice = VoiceProfile::new_male();
        for effort in [
            VocalEffort::Whisper,
            VocalEffort::Soft,
            VocalEffort::Normal,
            VocalEffort::Loud,
            VocalEffort::Shout,
        ] {
            let mut gs = voice
                .create_glottal_source_with_effort(effort, 44100.0)
                .unwrap();
            let samples: alloc::vec::Vec<f32> = (0..1024).map(|_| gs.next_sample()).collect();
            assert!(
                samples.iter().all(|s| s.is_finite()),
                "{effort:?} produced non-finite samples"
            );
        }
    }

    #[test]
    fn test_with_effort_builder() {
        let v = VoiceProfile::new_male().with_effort(VocalEffort::Whisper);
        assert!((v.breathiness - 1.0).abs() < f32::EPSILON);
        // f0_range should be scaled down for whisper (0.5x)
        let base_range = VoiceProfile::new_male().f0_range;
        assert!((v.f0_range - base_range * 0.5).abs() < 0.01);
    }

    #[test]
    fn test_effort_bandwidth_scaling() {
        let target = crate::formant::VowelTarget::from_vowel(crate::formant::Vowel::A);
        let voice = VoiceProfile::new_male();

        let whisper_scaled = voice.apply_formant_scale_with_effort(&target, VocalEffort::Whisper);
        let normal_scaled = voice.apply_formant_scale_with_effort(&target, VocalEffort::Normal);
        let shout_scaled = voice.apply_formant_scale_with_effort(&target, VocalEffort::Shout);

        // Whisper should have wider bandwidths than normal
        assert!(
            whisper_scaled.b1 > normal_scaled.b1,
            "whisper BW should be wider: {}  > {}",
            whisper_scaled.b1,
            normal_scaled.b1
        );
        // Shout should have narrower bandwidths than normal
        assert!(
            shout_scaled.b1 < normal_scaled.b1,
            "shout BW should be narrower: {} < {}",
            shout_scaled.b1,
            normal_scaled.b1
        );
    }

    #[test]
    fn test_serde_roundtrip_vocal_effort() {
        for effort in [
            VocalEffort::Whisper,
            VocalEffort::Soft,
            VocalEffort::Normal,
            VocalEffort::Loud,
            VocalEffort::Shout,
        ] {
            let json = serde_json::to_string(&effort).unwrap();
            let e2: VocalEffort = serde_json::from_str(&json).unwrap();
            assert_eq!(effort, e2);
        }
    }

    #[test]
    fn test_serde_roundtrip_effort_params() {
        let params = VocalEffort::Normal.params();
        let json = serde_json::to_string(&params).unwrap();
        let p2: EffortParams = serde_json::from_str(&json).unwrap();
        assert!((p2.rd - params.rd).abs() < f32::EPSILON);
        assert!((p2.breathiness - params.breathiness).abs() < f32::EPSILON);
    }
}
