//! Formant trajectory planning across multi-phoneme windows.
//!
//! Instead of synthesizing phonemes independently and crossfading, the
//! [`TrajectoryPlanner`] computes a continuous formant trajectory across
//! an entire utterance, with each phoneme's formant targets serving as
//! control points. Neighboring phonemes influence each other's transitions
//! based on coarticulation resistance.

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::formant::VowelTarget;
use crate::phoneme::{Phoneme, phoneme_formants};
use crate::voice::VoiceProfile;

/// A formant keypoint: the target formants at a specific time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormantKeypoint {
    /// Time in samples from the start of the utterance.
    pub time: usize,
    /// Formant target at this time.
    pub target: VowelTarget,
    /// Coarticulation resistance (0.0-1.0). Higher = less influenced by neighbors.
    pub resistance: f32,
}

/// Plans formant trajectories across a sequence of phonemes.
///
/// Given a list of phoneme events with durations, the planner computes
/// formant targets at each phoneme midpoint and boundary, then provides
/// interpolated formant values at any sample position.
///
/// The trajectory considers 3-phoneme windows: the formant transition at
/// a boundary is influenced not just by the two adjacent phonemes, but
/// also by the phoneme beyond (if its resistance is low enough to
/// propagate influence).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryPlanner {
    /// Formant keypoints in temporal order.
    keypoints: Vec<FormantKeypoint>,
    /// Total duration in samples.
    total_samples: usize,
}

impl TrajectoryPlanner {
    /// Creates a trajectory plan from phoneme events.
    ///
    /// `durations` are effective durations in seconds (after stress/cluster scaling).
    /// `voice` is used to apply formant scaling.
    #[must_use]
    pub fn plan(
        phonemes: &[Phoneme],
        durations: &[f32],
        voice: &VoiceProfile,
        sample_rate: f32,
    ) -> Self {
        assert_eq!(phonemes.len(), durations.len());

        if phonemes.is_empty() {
            return Self {
                keypoints: Vec::new(),
                total_samples: 0,
            };
        }

        // Compute sample offsets for each phoneme boundary
        let mut boundaries = Vec::with_capacity(phonemes.len() + 1);
        let mut offset = 0usize;
        boundaries.push(0);
        for &dur in durations {
            offset += (dur * sample_rate) as usize;
            boundaries.push(offset);
        }
        let total_samples = offset;

        // Create keypoints at phoneme midpoints
        let mut keypoints = Vec::with_capacity(phonemes.len() + 2);

        // Leading keypoint at time=0 (first phoneme's target)
        let first_target = voice.apply_formant_scale(&phoneme_formants(&phonemes[0]));
        keypoints.push(FormantKeypoint {
            time: 0,
            target: first_target,
            resistance: phonemes[0].coarticulation_resistance(),
        });

        // Midpoint keypoints for each phoneme
        for (i, phoneme) in phonemes.iter().enumerate() {
            let mid = (boundaries[i] + boundaries[i + 1]) / 2;
            let target = voice.apply_formant_scale(&phoneme_formants(phoneme));
            keypoints.push(FormantKeypoint {
                time: mid,
                target,
                resistance: phoneme.coarticulation_resistance(),
            });
        }

        // Trailing keypoint at end (last phoneme's target)
        let last_target = voice.apply_formant_scale(&phoneme_formants(phonemes.last().unwrap()));
        keypoints.push(FormantKeypoint {
            time: total_samples,
            target: last_target,
            resistance: phonemes.last().unwrap().coarticulation_resistance(),
        });

        Self {
            keypoints,
            total_samples,
        }
    }

    /// Returns the interpolated formant target at a given sample position.
    ///
    /// Uses Catmull-Rom spline interpolation when 4 keypoints are available
    /// (the current segment plus one neighbor on each side), falling back
    /// to linear interpolation at the edges.
    #[must_use]
    pub fn formants_at(&self, sample: usize) -> VowelTarget {
        if self.keypoints.is_empty() {
            return VowelTarget::new(500.0, 1500.0, 2500.0, 3300.0, 3750.0);
        }
        if self.keypoints.len() == 1 {
            return self.keypoints[0].target.clone();
        }

        // Find the segment: keypoints[seg] <= sample < keypoints[seg+1]
        let seg = self.find_segment(sample);
        let k0 = &self.keypoints[seg];
        let k1 = &self.keypoints[(seg + 1).min(self.keypoints.len() - 1)];

        // Local interpolation parameter
        let span = (k1.time.saturating_sub(k0.time)).max(1);
        let t = (sample.saturating_sub(k0.time)) as f32 / span as f32;
        let t = t.clamp(0.0, 1.0);

        // Try Catmull-Rom if we have neighbors on both sides
        if seg > 0 && seg + 2 < self.keypoints.len() {
            let km1 = &self.keypoints[seg - 1];
            let k2 = &self.keypoints[seg + 2];

            // Weight the Catmull-Rom influence by the average resistance of
            // the outer keypoints. High resistance = more linear (less
            // influence from distant phonemes).
            let outer_resistance = (km1.resistance + k2.resistance) * 0.5;
            let catmull_weight = 1.0 - outer_resistance;

            if catmull_weight > 0.05 {
                let linear = VowelTarget::interpolate(&k0.target, &k1.target, t);
                let catmull = catmull_rom_vowel(&km1.target, &k0.target, &k1.target, &k2.target, t);
                return blend_targets(&linear, &catmull, catmull_weight);
            }
        }

        // Fallback: sigmoid interpolation (same as current crossfade behavior)
        let t_smooth = hisab::calc::ease_in_out_smooth(t);
        VowelTarget::interpolate(&k0.target, &k1.target, t_smooth)
    }

    /// Returns the total number of samples in the planned trajectory.
    #[must_use]
    pub fn total_samples(&self) -> usize {
        self.total_samples
    }

    /// Returns the number of keypoints.
    #[must_use]
    pub fn num_keypoints(&self) -> usize {
        self.keypoints.len()
    }

    /// Returns the keypoints.
    #[must_use]
    pub fn keypoints(&self) -> &[FormantKeypoint] {
        &self.keypoints
    }

    /// Finds the segment index for a given sample position.
    fn find_segment(&self, sample: usize) -> usize {
        // Binary search for the segment containing this sample
        let mut lo = 0;
        let mut hi = self.keypoints.len().saturating_sub(1);
        while lo < hi {
            let mid = (lo + hi).div_ceil(2);
            if self.keypoints[mid].time <= sample {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        lo.min(self.keypoints.len().saturating_sub(2))
    }
}

/// Catmull-Rom spline interpolation for VowelTarget.
///
/// Given 4 control points (p0, p1, p2, p3) and parameter t in [0,1],
/// interpolates between p1 and p2 with smooth curvature influenced by
/// the outer points.
fn catmull_rom_vowel(
    p0: &VowelTarget,
    p1: &VowelTarget,
    p2: &VowelTarget,
    p3: &VowelTarget,
    t: f32,
) -> VowelTarget {
    let cr = |a: f32, b: f32, c: f32, d: f32| -> f32 {
        let t2 = t * t;
        let t3 = t2 * t;
        0.5 * ((2.0 * b)
            + (-a + c) * t
            + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2
            + (-a + 3.0 * b - 3.0 * c + d) * t3)
    };

    VowelTarget::with_bandwidths(
        [
            cr(p0.f1, p1.f1, p2.f1, p3.f1),
            cr(p0.f2, p1.f2, p2.f2, p3.f2),
            cr(p0.f3, p1.f3, p2.f3, p3.f3),
            cr(p0.f4, p1.f4, p2.f4, p3.f4),
            cr(p0.f5, p1.f5, p2.f5, p3.f5),
        ],
        [
            cr(p0.b1, p1.b1, p2.b1, p3.b1),
            cr(p0.b2, p1.b2, p2.b2, p3.b2),
            cr(p0.b3, p1.b3, p2.b3, p3.b3),
            cr(p0.b4, p1.b4, p2.b4, p3.b4),
            cr(p0.b5, p1.b5, p2.b5, p3.b5),
        ],
    )
}

/// Blends two VowelTargets by weight (0.0 = all a, 1.0 = all b).
fn blend_targets(a: &VowelTarget, b: &VowelTarget, weight: f32) -> VowelTarget {
    VowelTarget::interpolate(a, b, weight)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phoneme::Phoneme;

    #[test]
    fn test_empty_plan() {
        let plan = TrajectoryPlanner::plan(&[], &[], &VoiceProfile::new_male(), 44100.0);
        assert_eq!(plan.total_samples(), 0);
        assert_eq!(plan.num_keypoints(), 0);
    }

    #[test]
    fn test_single_phoneme_plan() {
        let voice = VoiceProfile::new_male();
        let plan = TrajectoryPlanner::plan(&[Phoneme::VowelA], &[0.1], &voice, 44100.0);
        assert_eq!(plan.total_samples(), 4410);
        // Leading + midpoint + trailing = 3 keypoints
        assert_eq!(plan.num_keypoints(), 3);
    }

    #[test]
    fn test_three_phoneme_plan() {
        let voice = VoiceProfile::new_male();
        let phonemes = [Phoneme::VowelA, Phoneme::NasalN, Phoneme::VowelI];
        let durations = [0.1, 0.06, 0.1];
        let plan = TrajectoryPlanner::plan(&phonemes, &durations, &voice, 44100.0);
        // Leading + 3 midpoints + trailing = 5 keypoints
        assert_eq!(plan.num_keypoints(), 5);
        assert_eq!(plan.total_samples(), (0.26 * 44100.0) as usize);
    }

    #[test]
    fn test_formants_at_endpoints() {
        let voice = VoiceProfile::new_male();
        let phonemes = [Phoneme::VowelA, Phoneme::VowelI];
        let durations = [0.1, 0.1];
        let plan = TrajectoryPlanner::plan(&phonemes, &durations, &voice, 44100.0);

        let target_a = voice.apply_formant_scale(&phoneme_formants(&Phoneme::VowelA));
        let target_i = voice.apply_formant_scale(&phoneme_formants(&Phoneme::VowelI));

        // At time=0, should be close to /a/ target
        let at_start = plan.formants_at(0);
        assert!((at_start.f1 - target_a.f1).abs() < 1.0);

        // At the end, should be close to /i/ target
        let at_end = plan.formants_at(plan.total_samples());
        assert!((at_end.f1 - target_i.f1).abs() < 1.0);
    }

    #[test]
    fn test_formants_at_midpoint_blends() {
        let voice = VoiceProfile::new_male();
        let phonemes = [Phoneme::VowelA, Phoneme::VowelI];
        let durations = [0.1, 0.1];
        let plan = TrajectoryPlanner::plan(&phonemes, &durations, &voice, 44100.0);

        let target_a = voice.apply_formant_scale(&phoneme_formants(&Phoneme::VowelA));
        let target_i = voice.apply_formant_scale(&phoneme_formants(&Phoneme::VowelI));

        // At the boundary between /a/ and /i/, F1 should be between the two
        let boundary = plan.total_samples() / 2;
        let at_boundary = plan.formants_at(boundary);
        let f1_a = target_a.f1;
        let f1_i = target_i.f1;
        let f1_mid = at_boundary.f1;
        assert!(
            (f1_mid > f1_i.min(f1_a) - 10.0) && (f1_mid < f1_a.max(f1_i) + 10.0),
            "boundary F1 should be between /a/ and /i/: got {f1_mid}, range [{f1_i}, {f1_a}]"
        );
    }

    #[test]
    fn test_catmull_rom_influence() {
        // With 3+ phonemes, Catmull-Rom should produce a different trajectory
        // than pure linear interpolation for low-resistance phonemes
        let voice = VoiceProfile::new_male();
        let phonemes = [
            Phoneme::VowelSchwa, // low resistance
            Phoneme::VowelA,
            Phoneme::VowelSchwa, // low resistance
        ];
        let durations = [0.1, 0.1, 0.1];
        let plan = TrajectoryPlanner::plan(&phonemes, &durations, &voice, 44100.0);

        // The trajectory should have smooth curvature, not just piecewise linear
        // Verify it produces finite values throughout
        for sample in (0..plan.total_samples()).step_by(100) {
            let target = plan.formants_at(sample);
            assert!(target.f1.is_finite());
            assert!(target.f2.is_finite());
            assert!(target.f1 > 0.0);
        }
    }

    #[test]
    fn test_serde_roundtrip_planner() {
        let voice = VoiceProfile::new_male();
        let plan = TrajectoryPlanner::plan(
            &[Phoneme::VowelA, Phoneme::VowelI],
            &[0.1, 0.1],
            &voice,
            44100.0,
        );
        let json = serde_json::to_string(&plan).unwrap();
        let plan2: TrajectoryPlanner = serde_json::from_str(&json).unwrap();
        assert_eq!(plan2.num_keypoints(), plan.num_keypoints());
        assert_eq!(plan2.total_samples(), plan.total_samples());
    }
}
