//! Dependency-free bridge functions for ecosystem integration.
//!
//! These functions convert outputs from upstream AGNOS science crates into
//! svara synthesis parameters, without depending on those crates directly.
//! Consumers call the upstream crate, then pass values through these bridges.
//!
//! # Bridges
//!
//! | Upstream Crate | Domain | Bridge Target |
//! |---------------|--------|---------------|
//! | **bhava** | Emotion/affect | Prosody, voice quality (Rd), breathiness |
//! | **vansh** | TTS text processing | Phoneme durations, stress |
//! | **prani** | Creature/character | Voice profile parameters |
//! | **goonj** | Acoustics/propagation | Distance attenuation, room effects |
//! | **badal** | Weather/environment | Lombard effect (effort in noise) |

use crate::glottal::GlottalModel;
use crate::prosody::{IntonationPattern, Stress};

// ---------------------------------------------------------------------------
// bhava (emotion/affect) bridges
// ---------------------------------------------------------------------------

/// Maps an arousal level (0.0 = calm, 1.0 = excited) to an Rd voice quality parameter.
///
/// Low arousal → breathy/relaxed (Rd ≈ 2.0), high arousal → pressed/tense (Rd ≈ 0.5).
/// Based on the observation that vocal effort correlates inversely with Rd.
///
/// Returns Rd in [0.3, 2.7].
#[must_use]
#[inline]
pub fn rd_from_arousal(arousal: f32) -> f32 {
    let arousal = arousal.clamp(0.0, 1.0);
    // Linear map: arousal 0 → Rd 2.2, arousal 1 → Rd 0.4
    2.2 - 1.8 * arousal
}

/// Maps a valence level (-1.0 = negative, 1.0 = positive) to a vibrato depth fraction.
///
/// Positive valence increases vibrato slightly (warmth), negative decreases it
/// (tension). Returns depth as fraction of f0, in [0.0, 0.06].
#[must_use]
#[inline]
pub fn vibrato_depth_from_valence(valence: f32) -> f32 {
    let valence = valence.clamp(-1.0, 1.0);
    // Centered at 0.03, ±0.03
    (0.03 + 0.03 * valence).clamp(0.0, 0.06)
}

/// Maps arousal to breathiness amount (0.0-1.0).
///
/// Low arousal → more breathiness (relaxed phonation), high arousal → less.
#[must_use]
#[inline]
pub fn breathiness_from_arousal(arousal: f32) -> f32 {
    let arousal = arousal.clamp(0.0, 1.0);
    // Low arousal → 0.3 breathiness, high arousal → 0.02
    0.3 - 0.28 * arousal
}

/// Maps arousal to jitter amount.
///
/// Moderate arousal is most stable; extremes (very calm or very tense) increase
/// micro-perturbation. Returns jitter fraction in [0.005, 0.03].
#[must_use]
#[inline]
pub fn jitter_from_arousal(arousal: f32) -> f32 {
    let arousal = arousal.clamp(0.0, 1.0);
    // U-shaped: minimum at 0.5 arousal
    let distance = (arousal - 0.5).abs();
    0.005 + 0.025 * distance * 2.0
}

/// Selects an intonation pattern from a categorical emotion label.
///
/// Maps basic emotion categories to prosodic patterns:
/// - 0 = neutral (declarative)
/// - 1 = happy (exclamatory)
/// - 2 = sad (declarative, low energy)
/// - 3 = angry (exclamatory)
/// - 4 = surprised (interrogative)
/// - 5 = fearful (continuation/rising)
///
/// Returns `None` for unrecognized categories.
#[must_use]
pub fn intonation_from_emotion(category: u8) -> Option<IntonationPattern> {
    match category {
        0 => Some(IntonationPattern::Declarative),
        1 | 3 => Some(IntonationPattern::Exclamatory),
        2 => Some(IntonationPattern::Declarative),
        4 => Some(IntonationPattern::Interrogative),
        5 => Some(IntonationPattern::Continuation),
        _ => None,
    }
}

/// Maps arousal to f0 range scaling factor.
///
/// Higher arousal → wider pitch range (more expressive).
/// Returns a multiplier for `VoiceProfile::f0_range`.
#[must_use]
#[inline]
pub fn f0_range_scale_from_arousal(arousal: f32) -> f32 {
    let arousal = arousal.clamp(0.0, 1.0);
    // Low arousal → 0.6x range, high arousal → 1.8x range
    0.6 + 1.2 * arousal
}

// ---------------------------------------------------------------------------
// vansh (TTS text processing) bridges
// ---------------------------------------------------------------------------

/// Maps a normalized speech rate (1.0 = normal) to a duration scale factor.
///
/// Faster speech compresses durations; slower speech expands them.
/// Returns a multiplier for phoneme durations, clamped to [0.3, 3.0].
#[must_use]
#[inline]
pub fn duration_scale_from_speech_rate(rate: f32) -> f32 {
    if rate <= 0.0 {
        return 1.0;
    }
    (1.0 / rate).clamp(0.3, 3.0)
}

/// Maps a TOBI-style pitch accent level (0-4) to a stress level.
///
/// - 0: unstressed
/// - 1: secondary stress
/// - 2+: primary stress
#[must_use]
pub fn stress_from_tobi_accent(level: u8) -> Stress {
    match level {
        0 => Stress::Unstressed,
        1 => Stress::Secondary,
        _ => Stress::Primary,
    }
}

/// Maps a prominence score (0.0 = background, 1.0 = focus) to an f0 multiplier
/// for the syllable peak.
///
/// Focus prominence raises the pitch peak on a syllable. Returns multiplier
/// in [1.0, 1.3].
#[must_use]
#[inline]
pub fn f0_peak_from_prominence(prominence: f32) -> f32 {
    let prominence = prominence.clamp(0.0, 1.0);
    1.0 + 0.3 * prominence
}

// ---------------------------------------------------------------------------
// prani (creature/character) bridges
// ---------------------------------------------------------------------------

/// Maps a body size factor (0.0 = tiny, 1.0 = large) to a formant scale.
///
/// Larger bodies have longer vocal tracts → lower formant frequencies.
/// Returns a formant frequency multiplier for `VoiceProfile::formant_scale`.
#[must_use]
#[inline]
pub fn formant_scale_from_body_size(size: f32) -> f32 {
    let size = size.clamp(0.0, 1.0);
    // Small body → 1.4 (child-like), large body → 0.85 (deep)
    1.4 - 0.55 * size
}

/// Maps a body size factor to a base f0 in Hz.
///
/// Larger bodies → lower pitch. Returns f0 in [70, 400] Hz.
#[must_use]
#[inline]
pub fn f0_from_body_size(size: f32) -> f32 {
    let size = size.clamp(0.0, 1.0);
    // Small → 400Hz (child), large → 70Hz (deep male)
    400.0 - 330.0 * size
}

/// Maps an age factor (0.0 = infant, 1.0 = elderly) to a jitter amount.
///
/// Very young and elderly voices have more jitter (less stable phonation).
/// Returns jitter fraction in [0.005, 0.04].
#[must_use]
#[inline]
pub fn jitter_from_age(age: f32) -> f32 {
    let age = age.clamp(0.0, 1.0);
    // U-shaped: minimum at age ≈ 0.4 (adult prime)
    let distance = (age - 0.4).abs();
    0.005 + 0.035 * distance / 0.6
}

/// Selects a glottal model based on vocal effort level.
///
/// Low effort (breathy/whisper) works better with LF model which supports Rd.
/// High effort (shout) also benefits from LF for pressed voice. Rosenberg is
/// adequate for normal modal phonation.
///
/// Returns `(model, rd)` tuple.
#[must_use]
pub fn glottal_model_from_effort(effort: f32) -> (GlottalModel, f32) {
    let effort = effort.clamp(0.0, 1.0);
    if effort < 0.3 {
        // Breathy/whisper: LF with high Rd
        (GlottalModel::LF, 2.0 + (0.3 - effort) * 2.3)
    } else if effort > 0.7 {
        // Pressed/shout: LF with low Rd
        (GlottalModel::LF, 0.8 - (effort - 0.7) * 1.5)
    } else {
        // Modal voice: Rosenberg is sufficient
        (GlottalModel::Rosenberg, 1.0)
    }
}

// ---------------------------------------------------------------------------
// goonj (acoustics/propagation) bridges
// ---------------------------------------------------------------------------

/// Computes gain attenuation from source-listener distance using inverse-square law.
///
/// `ref_distance` is the distance at which gain = 1.0 (no attenuation).
/// Returns gain multiplier in [0.0, 1.0].
#[must_use]
#[inline]
pub fn gain_from_distance(ref_distance: f32, distance: f32) -> f32 {
    if distance <= ref_distance || ref_distance <= 0.0 {
        return 1.0;
    }
    (ref_distance / distance).clamp(0.0, 1.0)
}

/// Estimates formant bandwidth broadening from room reverberation.
///
/// Reverberant environments smear formant peaks. `rt60` is the reverberation
/// time in seconds. Returns a bandwidth multiplier (1.0 = anechoic, up to 2.0).
#[must_use]
#[inline]
pub fn bandwidth_scale_from_reverb(rt60: f32) -> f32 {
    let rt60 = rt60.max(0.0);
    // Light reverb (0.3s) → 1.1x, heavy reverb (2.0s) → 1.8x
    (1.0 + 0.4 * rt60).min(2.0)
}

/// Computes a high-frequency roll-off factor from air absorption over distance.
///
/// At long distances, high frequencies are attenuated more than low frequencies.
/// Returns a spectral tilt addition in dB/octave.
#[must_use]
#[inline]
pub fn spectral_tilt_from_distance(distance_m: f32) -> f32 {
    // Air absorption at 20°C ≈ 0.005 dB/m/kHz
    // Approximate as additional tilt
    (distance_m * 0.005).min(6.0)
}

// ---------------------------------------------------------------------------
// badal (weather/environment) bridges
// ---------------------------------------------------------------------------

/// Computes Lombard effect vocal effort increase from ambient noise level.
///
/// The Lombard effect causes speakers to increase vocal effort in noisy
/// environments. `ambient_db_spl` is the background noise level.
///
/// Returns an effort multiplier (1.0 = quiet environment, up to 1.8).
///
/// Reference: Lombard, E. (1911). "Le signe de l'élévation de la voix."
#[must_use]
#[inline]
pub fn lombard_effort_from_noise(ambient_db_spl: f32) -> f32 {
    // Lombard effect onset ~45 dB SPL, saturates ~85 dB SPL
    if ambient_db_spl < 45.0 {
        1.0
    } else if ambient_db_spl > 85.0 {
        1.8
    } else {
        1.0 + 0.8 * (ambient_db_spl - 45.0) / 40.0
    }
}

/// Maps Lombard effort to f0 shift (Hz above base).
///
/// Speakers raise pitch in noise. Returns f0 addition in Hz.
#[must_use]
#[inline]
pub fn lombard_f0_shift(ambient_db_spl: f32) -> f32 {
    // ~0.3 semitones per 10 dB above 50 dB SPL
    let excess = (ambient_db_spl - 50.0).max(0.0);
    // Approximate: 2 Hz per dB above threshold, max 30 Hz
    (excess * 0.75).min(30.0)
}

/// Maps wind speed (m/s) to a breathiness increase factor.
///
/// Wind noise near a microphone or listener causes the speaker to adopt
/// a less breathy, more pressed voice to cut through. Returns a breathiness
/// *reduction* factor (multiply with current breathiness).
#[must_use]
#[inline]
pub fn breathiness_reduction_from_wind(wind_speed_ms: f32) -> f32 {
    let wind = wind_speed_ms.max(0.0);
    // Above 5 m/s, speakers reduce breathiness; at 15 m/s, minimal breathiness
    if wind < 5.0 {
        1.0
    } else {
        (1.0 - (wind - 5.0) / 10.0).clamp(0.2, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rd_from_arousal_range() {
        let calm = rd_from_arousal(0.0);
        let excited = rd_from_arousal(1.0);
        assert!(calm > excited, "calm should have higher Rd (breathier)");
        assert!((0.3..=2.7).contains(&calm));
        assert!((0.3..=2.7).contains(&excited));
    }

    #[test]
    fn test_rd_from_arousal_clamps() {
        assert!((rd_from_arousal(-1.0) - rd_from_arousal(0.0)).abs() < f32::EPSILON);
        assert!((rd_from_arousal(2.0) - rd_from_arousal(1.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_breathiness_from_arousal() {
        let calm = breathiness_from_arousal(0.0);
        let excited = breathiness_from_arousal(1.0);
        assert!(calm > excited);
        assert!(calm <= 1.0 && excited >= 0.0);
    }

    #[test]
    fn test_jitter_from_arousal_u_shape() {
        let low = jitter_from_arousal(0.0);
        let mid = jitter_from_arousal(0.5);
        let high = jitter_from_arousal(1.0);
        assert!(mid < low, "mid-arousal should be most stable");
        assert!(mid < high, "mid-arousal should be most stable");
    }

    #[test]
    fn test_duration_scale_from_speech_rate() {
        assert!((duration_scale_from_speech_rate(1.0) - 1.0).abs() < f32::EPSILON);
        assert!(duration_scale_from_speech_rate(2.0) < 1.0);
        assert!(duration_scale_from_speech_rate(0.5) > 1.0);
        // Zero/negative rates don't panic
        assert!((duration_scale_from_speech_rate(0.0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_stress_from_tobi_accent() {
        assert_eq!(stress_from_tobi_accent(0), Stress::Unstressed);
        assert_eq!(stress_from_tobi_accent(1), Stress::Secondary);
        assert_eq!(stress_from_tobi_accent(2), Stress::Primary);
        assert_eq!(stress_from_tobi_accent(4), Stress::Primary);
    }

    #[test]
    fn test_formant_scale_from_body_size() {
        let small = formant_scale_from_body_size(0.0);
        let large = formant_scale_from_body_size(1.0);
        assert!(small > large, "small body should have higher formant scale");
    }

    #[test]
    fn test_f0_from_body_size() {
        let small = f0_from_body_size(0.0);
        let large = f0_from_body_size(1.0);
        assert!(small > large, "small body should have higher f0");
        assert!((20.0..=2000.0).contains(&small));
        assert!((20.0..=2000.0).contains(&large));
    }

    #[test]
    fn test_gain_from_distance() {
        assert!((gain_from_distance(1.0, 1.0) - 1.0).abs() < f32::EPSILON);
        assert!((gain_from_distance(1.0, 2.0) - 0.5).abs() < f32::EPSILON);
        assert!((gain_from_distance(1.0, 0.5) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_lombard_effort_from_noise() {
        assert!((lombard_effort_from_noise(30.0) - 1.0).abs() < f32::EPSILON);
        assert!(lombard_effort_from_noise(65.0) > 1.0);
        assert!((lombard_effort_from_noise(90.0) - 1.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_glottal_model_from_effort() {
        let (model_low, rd_low) = glottal_model_from_effort(0.1);
        assert_eq!(model_low, GlottalModel::LF);
        assert!(rd_low > 1.5, "low effort should be breathy (high Rd)");

        let (model_mid, _) = glottal_model_from_effort(0.5);
        assert_eq!(model_mid, GlottalModel::Rosenberg);

        let (model_high, rd_high) = glottal_model_from_effort(0.9);
        assert_eq!(model_high, GlottalModel::LF);
        assert!(rd_high < 1.0, "high effort should be pressed (low Rd)");
    }

    #[test]
    fn test_bandwidth_scale_from_reverb() {
        let dry = bandwidth_scale_from_reverb(0.0);
        let wet = bandwidth_scale_from_reverb(2.0);
        assert!((dry - 1.0).abs() < f32::EPSILON);
        assert!(wet > dry);
        assert!(wet <= 2.0);
    }

    #[test]
    fn test_breathiness_reduction_from_wind() {
        assert!((breathiness_reduction_from_wind(0.0) - 1.0).abs() < f32::EPSILON);
        assert!(breathiness_reduction_from_wind(10.0) < 1.0);
        assert!(breathiness_reduction_from_wind(20.0) >= 0.2);
    }
}
