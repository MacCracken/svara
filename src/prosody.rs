//! Prosody: pitch contours, intonation patterns, and stress.
//!
//! Controls the suprasegmental features of speech: fundamental frequency
//! contour, timing, and amplitude variations that convey meaning beyond
//! individual phonemes.

use alloc::{vec, vec::Vec};
use serde::{Deserialize, Serialize};
use tracing::trace;

/// A prosodic contour specifying f0 trajectory, duration scaling, and amplitude.
///
/// The `f0_points` field contains time-value pairs where time is normalized
/// to [0.0, 1.0] and value is a multiplier of the base f0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProsodyContour {
    /// Time-value pairs for f0 contour. Time in `[0,1]`, value is f0 multiplier.
    pub f0_points: Vec<(f32, f32)>,
    /// Duration scale factor (1.0 = normal).
    pub duration_scale: f32,
    /// Amplitude scale factor (1.0 = normal).
    pub amplitude_scale: f32,
}

impl ProsodyContour {
    /// Creates a flat prosody contour (constant f0, no scaling).
    #[must_use]
    pub fn flat() -> Self {
        Self {
            f0_points: vec![(0.0, 1.0), (1.0, 1.0)],
            duration_scale: 1.0,
            amplitude_scale: 1.0,
        }
    }

    /// Creates a prosody contour from an intonation pattern.
    ///
    /// The contour values are multipliers of the given `base_f0`.
    #[must_use]
    pub fn from_pattern(pattern: IntonationPattern, _base_f0: f32) -> Self {
        trace!(?pattern, "creating prosody contour from pattern");

        match pattern {
            IntonationPattern::Declarative => Self {
                // Falling: starts slightly above base, falls to ~80%
                f0_points: vec![
                    (0.0, 1.05),
                    (0.3, 1.02),
                    (0.6, 0.95),
                    (0.8, 0.88),
                    (1.0, 0.80),
                ],
                duration_scale: 1.0,
                amplitude_scale: 1.0,
            },
            IntonationPattern::Interrogative => Self {
                // Rising: starts at base, rises to ~130%
                f0_points: vec![
                    (0.0, 1.0),
                    (0.4, 0.98),
                    (0.6, 1.05),
                    (0.8, 1.15),
                    (1.0, 1.30),
                ],
                duration_scale: 1.1,
                amplitude_scale: 1.0,
            },
            IntonationPattern::Continuation => Self {
                // Rise-fall: rises then partial fall (implies more to come)
                f0_points: vec![
                    (0.0, 1.0),
                    (0.3, 1.08),
                    (0.5, 1.12),
                    (0.7, 1.06),
                    (1.0, 1.0),
                ],
                duration_scale: 1.05,
                amplitude_scale: 1.0,
            },
            IntonationPattern::Exclamatory => Self {
                // High start, dramatic fall
                f0_points: vec![(0.0, 1.3), (0.2, 1.25), (0.5, 1.1), (0.8, 0.9), (1.0, 0.75)],
                duration_scale: 0.9,
                amplitude_scale: 1.2,
            },
        }
    }

    /// Applies stress modification at the given normalized position.
    ///
    /// Primary stress boosts f0, duration, and amplitude.
    /// Secondary stress gives a smaller boost.
    /// Unstressed slightly reduces values.
    pub fn apply_stress(&mut self, stress: Stress, position: f32) {
        let position = position.clamp(0.0, 1.0);

        match stress {
            Stress::Primary => {
                // Boost f0 by ~10% at stress point, extend duration, increase amplitude
                self.insert_f0_boost(position, 1.10, 0.15);
                self.duration_scale *= 1.15;
                self.amplitude_scale *= 1.1;
            }
            Stress::Secondary => {
                self.insert_f0_boost(position, 1.05, 0.10);
                self.duration_scale *= 1.05;
                self.amplitude_scale *= 1.05;
            }
            Stress::Unstressed => {
                // Slight reduction
                self.duration_scale *= 0.9;
                self.amplitude_scale *= 0.95;
            }
        }
    }

    /// Inserts an f0 boost at the given position with specified width.
    fn insert_f0_boost(&mut self, position: f32, boost: f32, width: f32) {
        let start = (position - width).max(0.0);
        let end = (position + width).min(1.0);

        // Modify existing points in the region
        for point in &mut self.f0_points {
            let dist = (point.0 - position).abs();
            if dist < width {
                let influence = 1.0 - (dist / width);
                point.1 *= 1.0 + (boost - 1.0) * influence;
            }
        }

        // Add peak point if not near an existing point
        let has_nearby = self.f0_points.iter().any(|p| (p.0 - position).abs() < 0.02);
        if !has_nearby {
            // Find the interpolated base value at this position
            let base = self.f0_at_internal(position);
            self.f0_points.push((position, base * boost));
            self.f0_points
                .sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(core::cmp::Ordering::Equal));
        }

        // Ensure we have boundary points
        if self.f0_points.first().is_some_and(|p| p.0 > start) {
            let val = self.f0_at_internal(start);
            self.f0_points.insert(0, (start, val));
        }
        if self.f0_points.last().is_some_and(|p| p.0 < end) {
            let val = self.f0_at_internal(end);
            self.f0_points.push((end, val));
        }
    }

    /// Internal f0 lookup without borrow issues.
    fn f0_at_internal(&self, t: f32) -> f32 {
        if self.f0_points.is_empty() {
            return 1.0;
        }
        if self.f0_points.len() == 1 {
            return self.f0_points[0].1;
        }

        let t = t.clamp(0.0, 1.0) as f64;

        // Use hisab's monotone cubic interpolation (Fritsch-Carlson).
        // Guarantees no overshoot — critical for f0 contours where
        // overshooting pitch targets produces unnatural artifacts.
        let xs: Vec<f64> = self.f0_points.iter().map(|p| p.0 as f64).collect();
        let ys: Vec<f64> = self.f0_points.iter().map(|p| p.1 as f64).collect();

        hisab::calc::monotone_cubic(&xs, &ys, t).unwrap_or(1.0) as f32
    }

    /// Returns the interpolated f0 multiplier at normalized time `t`.
    ///
    /// Uses Catmull-Rom spline interpolation for smooth, natural pitch curves.
    /// Time is clamped to `[0.0, 1.0]`.
    #[must_use]
    #[inline]
    pub fn f0_at(&self, t: f32) -> f32 {
        self.f0_at_internal(t)
    }
}

/// Intonation patterns for different utterance types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum IntonationPattern {
    /// Falling pitch (statements).
    Declarative,
    /// Rising pitch (yes/no questions).
    Interrogative,
    /// Rise-fall (continuation, more to follow).
    Continuation,
    /// High-start dramatic fall (exclamations).
    Exclamatory,
}

/// Lexical tone for tone languages (Mandarin, Thai, Yoruba, etc.).
///
/// Tones modify the f0 contour of a syllable to distinguish meaning.
/// The variants model the five Mandarin tones plus common cross-language
/// tone patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Tone {
    /// High level tone (Mandarin tone 1, Chao 55). Flat, high pitch.
    High,
    /// Rising tone (Mandarin tone 2, Chao 35). Low-to-high.
    Rising,
    /// Dipping tone (Mandarin tone 3, Chao 214). Fall then rise.
    Dipping,
    /// Falling tone (Mandarin tone 4, Chao 51). High-to-low.
    Falling,
    /// Neutral/light tone (Mandarin tone 5). Short, pitch depends on context.
    Neutral,
    /// Low level tone (common in Thai, Yoruba). Flat, low pitch.
    Low,
    /// Mid level tone (common in many African languages). Flat, mid pitch.
    Mid,
    /// Low rising (Thai tone 5). Starts low, rises moderately.
    LowRising,
    /// High falling (common in Vietnamese, Cantonese). Starts very high, drops.
    HighFalling,
}

impl Tone {
    /// Returns the f0 contour for this tone as a [`ProsodyContour`].
    ///
    /// The contour values are f0 multipliers relative to the speaker's base f0.
    /// A value of 1.0 is the base pitch, 1.3 is 30% higher, 0.7 is 30% lower.
    #[must_use]
    pub fn to_contour(self) -> ProsodyContour {
        match self {
            Self::High => ProsodyContour {
                f0_points: vec![(0.0, 1.2), (0.5, 1.2), (1.0, 1.2)],
                duration_scale: 1.0,
                amplitude_scale: 1.0,
            },
            Self::Rising => ProsodyContour {
                f0_points: vec![(0.0, 0.85), (0.3, 0.85), (0.7, 1.1), (1.0, 1.25)],
                duration_scale: 1.0,
                amplitude_scale: 1.0,
            },
            Self::Dipping => ProsodyContour {
                f0_points: vec![
                    (0.0, 1.0),
                    (0.2, 0.85),
                    (0.5, 0.75),
                    (0.8, 0.9),
                    (1.0, 1.05),
                ],
                duration_scale: 1.15, // dipping tone is typically longer
                amplitude_scale: 0.9,
            },
            Self::Falling => ProsodyContour {
                f0_points: vec![(0.0, 1.3), (0.3, 1.15), (0.7, 0.9), (1.0, 0.75)],
                duration_scale: 0.9,
                amplitude_scale: 1.1,
            },
            Self::Neutral => ProsodyContour {
                f0_points: vec![(0.0, 1.0), (1.0, 0.95)],
                duration_scale: 0.7, // neutral tone is short
                amplitude_scale: 0.85,
            },
            Self::Low => ProsodyContour {
                f0_points: vec![(0.0, 0.75), (0.5, 0.75), (1.0, 0.75)],
                duration_scale: 1.0,
                amplitude_scale: 0.95,
            },
            Self::Mid => ProsodyContour {
                f0_points: vec![(0.0, 1.0), (0.5, 1.0), (1.0, 1.0)],
                duration_scale: 1.0,
                amplitude_scale: 1.0,
            },
            Self::LowRising => ProsodyContour {
                f0_points: vec![(0.0, 0.7), (0.4, 0.75), (0.7, 0.95), (1.0, 1.1)],
                duration_scale: 1.0,
                amplitude_scale: 1.0,
            },
            Self::HighFalling => ProsodyContour {
                f0_points: vec![(0.0, 1.35), (0.2, 1.3), (0.6, 1.0), (1.0, 0.7)],
                duration_scale: 0.95,
                amplitude_scale: 1.05,
            },
        }
    }
}

/// Stress level for a phoneme or syllable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Stress {
    /// Primary lexical stress: f0 boost + duration stretch + amplitude increase.
    Primary,
    /// Secondary stress: smaller boost.
    Secondary,
    /// No stress: slight reduction.
    Unstressed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_contour() {
        let c = ProsodyContour::flat();
        assert!((c.f0_at(0.0) - 1.0).abs() < f32::EPSILON);
        assert!((c.f0_at(0.5) - 1.0).abs() < f32::EPSILON);
        assert!((c.f0_at(1.0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_declarative_falls() {
        let c = ProsodyContour::from_pattern(IntonationPattern::Declarative, 120.0);
        let start = c.f0_at(0.0);
        let end = c.f0_at(1.0);
        assert!(start > end, "declarative should fall: {start} > {end}");
    }

    #[test]
    fn test_interrogative_rises() {
        let c = ProsodyContour::from_pattern(IntonationPattern::Interrogative, 120.0);
        let start = c.f0_at(0.0);
        let end = c.f0_at(1.0);
        assert!(end > start, "interrogative should rise: {end} > {start}");
    }

    #[test]
    fn test_stress_application() {
        let mut c = ProsodyContour::flat();
        let before_dur = c.duration_scale;
        c.apply_stress(Stress::Primary, 0.5);
        assert!(c.duration_scale > before_dur);
        assert!(c.amplitude_scale > 1.0);
    }

    #[test]
    fn test_interpolation_monotonic() {
        let c = ProsodyContour::from_pattern(IntonationPattern::Declarative, 120.0);
        // Values should be finite throughout
        for i in 0..100 {
            let t = i as f32 / 100.0;
            let v = c.f0_at(t);
            assert!(v.is_finite(), "f0 at t={t} is not finite: {v}");
            assert!(v > 0.0, "f0 at t={t} must be positive: {v}");
        }
    }

    #[test]
    fn test_serde_roundtrip() {
        let c = ProsodyContour::from_pattern(IntonationPattern::Interrogative, 200.0);
        let json = serde_json::to_string(&c).unwrap();
        let c2: ProsodyContour = serde_json::from_str(&json).unwrap();
        assert!((c2.f0_at(0.5) - c.f0_at(0.5)).abs() < f32::EPSILON);
    }

    // --- Tone tests ---

    #[test]
    fn test_tone_high_is_flat() {
        let c = Tone::High.to_contour();
        let start = c.f0_at(0.0);
        let end = c.f0_at(1.0);
        assert!((start - end).abs() < 0.05, "high tone should be flat");
        assert!(start > 1.1, "high tone should be above base: {start}");
    }

    #[test]
    fn test_tone_rising() {
        let c = Tone::Rising.to_contour();
        assert!(c.f0_at(1.0) > c.f0_at(0.0), "rising tone should rise");
    }

    #[test]
    fn test_tone_falling() {
        let c = Tone::Falling.to_contour();
        assert!(c.f0_at(0.0) > c.f0_at(1.0), "falling tone should fall");
    }

    #[test]
    fn test_tone_dipping() {
        let c = Tone::Dipping.to_contour();
        let mid = c.f0_at(0.5);
        let start = c.f0_at(0.0);
        let end = c.f0_at(1.0);
        assert!(mid < start, "dipping should dip below start");
        assert!(end > mid, "dipping should rise back");
    }

    #[test]
    fn test_tone_neutral_short() {
        let c = Tone::Neutral.to_contour();
        assert!(c.duration_scale < 0.8, "neutral tone should be short");
    }

    #[test]
    fn test_all_tones_produce_valid_contours() {
        for tone in [
            Tone::High,
            Tone::Rising,
            Tone::Dipping,
            Tone::Falling,
            Tone::Neutral,
            Tone::Low,
            Tone::Mid,
            Tone::LowRising,
            Tone::HighFalling,
        ] {
            let c = tone.to_contour();
            for i in 0..10 {
                let t = i as f32 / 10.0;
                let v = c.f0_at(t);
                assert!(v.is_finite(), "{tone:?} produced non-finite f0 at t={t}");
                assert!(v > 0.0, "{tone:?} produced non-positive f0 at t={t}");
            }
        }
    }

    #[test]
    fn test_serde_roundtrip_tone() {
        for tone in [
            Tone::High,
            Tone::Rising,
            Tone::Dipping,
            Tone::Falling,
            Tone::Neutral,
        ] {
            let json = serde_json::to_string(&tone).unwrap();
            let t2: Tone = serde_json::from_str(&json).unwrap();
            assert_eq!(tone, t2);
        }
    }
}
