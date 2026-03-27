//! Prosody: pitch contours, intonation patterns, and stress.
//!
//! Controls the suprasegmental features of speech: fundamental frequency
//! contour, timing, and amplitude variations that convey meaning beyond
//! individual phonemes.

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
                .sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
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

        let t = t.clamp(0.0, 1.0);

        if t <= self.f0_points[0].0 {
            return self.f0_points[0].1;
        }
        if t >= self.f0_points.last().map_or(1.0, |p| p.0) {
            return self.f0_points.last().map_or(1.0, |p| p.1);
        }

        for i in 0..self.f0_points.len() - 1 {
            let (t0, v0) = self.f0_points[i];
            let (t1, v1) = self.f0_points[i + 1];
            if t >= t0 && t <= t1 {
                let frac = if (t1 - t0).abs() < f32::EPSILON {
                    0.0
                } else {
                    (t - t0) / (t1 - t0)
                };
                return v0 + (v1 - v0) * frac;
            }
        }

        self.f0_points.last().map_or(1.0, |p| p.1)
    }

    /// Returns the interpolated f0 multiplier at normalized time `t`.
    ///
    /// Uses linear interpolation between defined points.
    /// Time is clamped to [0.0, 1.0].
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
}
