//! Phoneme sequencing with coarticulation and crossfading.
//!
//! Combines individual phonemes into continuous speech, applying prosodic
//! contours, coarticulatory formant blending, and smooth crossfades at
//! phoneme boundaries.

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use tracing::trace;

use crate::error::Result;
use crate::phoneme::{self, Phoneme};
use crate::prosody::Stress;
use crate::voice::VoiceProfile;

use crate::phoneme::PhonemeClass;

/// Minimum crossfade fraction for high-resistance phonemes.
const MIN_CROSSFADE_FRACTION: f32 = 0.15;
/// Maximum crossfade fraction for low-resistance phonemes.
const MAX_CROSSFADE_FRACTION: f32 = 0.45;
/// Duration compression factor for consonants within a cluster.
const CLUSTER_COMPRESSION: f32 = 0.7;

/// A timed phoneme event within a sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhonemeEvent {
    /// The phoneme to synthesize.
    pub phoneme: Phoneme,
    /// Duration in seconds.
    pub duration: f32,
    /// Stress level for this phoneme.
    pub stress: Stress,
}

impl PhonemeEvent {
    /// Creates a new phoneme event.
    #[must_use]
    pub fn new(phoneme: Phoneme, duration: f32, stress: Stress) -> Self {
        Self {
            phoneme,
            duration,
            stress,
        }
    }
}

/// An ordered sequence of phoneme events for rendering continuous speech.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhonemeSequence {
    /// The phoneme events in temporal order.
    events: Vec<PhonemeEvent>,
    /// Coarticulation transition window in seconds.
    transition_window: f32,
    /// Look-ahead onset: fraction of segment where transition to next phoneme
    /// begins (0.0 = start, 1.0 = end). Default 0.6 means the last 40% of each
    /// segment transitions toward the next phoneme's formant targets.
    lookahead_onset: f32,
}

impl PhonemeSequence {
    /// Creates a new empty phoneme sequence.
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            transition_window: 0.05, // 50ms default
            lookahead_onset: 0.6,    // transition starts at 60% of segment
        }
    }

    /// Sets the coarticulation transition window in seconds.
    pub fn set_transition_window(&mut self, window: f32) {
        self.transition_window = window.max(0.001);
    }

    /// Sets the look-ahead onset (0.0-1.0). Default 0.6 means transition starts
    /// at 60% of each segment.
    pub fn set_lookahead_onset(&mut self, onset: f32) {
        self.lookahead_onset = onset.clamp(0.0, 1.0);
    }

    /// Returns the transition window in seconds.
    #[must_use]
    pub fn transition_window(&self) -> f32 {
        self.transition_window
    }

    /// Appends a phoneme event to the sequence.
    pub fn push(&mut self, event: PhonemeEvent) {
        self.events.push(event);
    }

    /// Returns the number of events in the sequence.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Returns whether the sequence is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Returns a reference to the events.
    #[must_use]
    pub fn events(&self) -> &[PhonemeEvent] {
        &self.events
    }

    /// Returns the total duration of the sequence in seconds.
    #[must_use]
    pub fn total_duration(&self) -> f32 {
        self.events.iter().map(|e| e.duration).sum()
    }

    /// Renders the entire phoneme sequence to audio samples.
    ///
    /// Applies stress-based duration scaling, synthesizes each phoneme, and
    /// crossfades at boundaries for smooth coarticulation.
    ///
    /// # Errors
    ///
    /// Returns `SvaraError` if any phoneme fails to synthesize.
    pub fn render(&self, voice: &VoiceProfile, sample_rate: f32) -> Result<Vec<f32>> {
        if self.events.is_empty() {
            return Ok(Vec::new());
        }

        trace!(
            num_events = self.events.len(),
            sample_rate, "rendering phoneme sequence"
        );

        // Detect consonant clusters and apply duration compression.
        // A cluster is 2+ adjacent consonants (no vowels/diphthongs/silence between).
        let in_cluster = detect_consonant_clusters(&self.events);

        // Calculate effective durations with stress scaling and cluster compression
        let durations: Vec<f32> = self
            .events
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let stress_scale = match e.stress {
                    Stress::Primary => 1.15,
                    Stress::Secondary => 1.05,
                    Stress::Unstressed => 0.9,
                };
                let cluster_scale = if in_cluster[i] {
                    CLUSTER_COMPRESSION
                } else {
                    1.0
                };
                e.duration * stress_scale * cluster_scale
            })
            .collect();

        // Detect anticipatory nasalization: vowels/diphthongs before nasals
        let phoneme_list: Vec<Phoneme> = self.events.iter().map(|e| e.phoneme).collect();
        let nasalizations = phoneme::detect_nasalization(&phoneme_list);

        // Synthesize each phoneme with anticipatory nasalization
        let mut segments: Vec<Vec<f32>> = Vec::with_capacity(self.events.len());
        for (i, (event, &dur)) in self.events.iter().zip(durations.iter()).enumerate() {
            // Apply stress-based voice modification
            let mut event_voice = voice.clone();
            match event.stress {
                Stress::Primary => {
                    event_voice.base_f0 *= 1.10;
                }
                Stress::Secondary => {
                    event_voice.base_f0 *= 1.05;
                }
                Stress::Unstressed => {}
            }

            let segment = phoneme::synthesize_phoneme_nasalized(
                &event.phoneme,
                &event_voice,
                sample_rate,
                dur,
                nasalizations[i].as_ref(),
            )?;
            segments.push(segment);
        }

        // Compute per-boundary crossfade lengths based on coarticulation resistance.
        // Low-resistance phonemes (schwa, /h/) get longer crossfades, high-resistance
        // (/i/, /s/) get shorter. This models natural coarticulatory dynamics.
        let mut crossfade_lengths: Vec<usize> =
            Vec::with_capacity(segments.len().saturating_sub(1));
        for i in 0..segments.len().saturating_sub(1) {
            let r_left = self.events[i].phoneme.coarticulation_resistance();
            let r_right = self.events[i + 1].phoneme.coarticulation_resistance();
            // Average resistance determines crossfade: lower resistance = longer crossfade
            let avg_resistance = (r_left + r_right) * 0.5;
            let frac = MAX_CROSSFADE_FRACTION
                - avg_resistance * (MAX_CROSSFADE_FRACTION - MIN_CROSSFADE_FRACTION);
            let shorter_len = segments[i].len().min(segments[i + 1].len());
            let cf_len = (frac * shorter_len as f32) as usize;
            // Floor at the transition window
            let min_cf = (self.transition_window * sample_rate) as usize;
            crossfade_lengths.push(cf_len.max(min_cf));
        }

        let output = crossfade_segments_variable(&segments, &crossfade_lengths);

        Ok(output)
    }
}

impl Default for PhonemeSequence {
    fn default() -> Self {
        Self::new()
    }
}

/// Detects consonant clusters: runs of 2+ adjacent consonants.
///
/// Returns a boolean vec where `true` means the phoneme at that index is
/// part of a consonant cluster and should receive duration compression.
fn detect_consonant_clusters(events: &[PhonemeEvent]) -> Vec<bool> {
    let n = events.len();
    let mut in_cluster = alloc::vec![false; n];

    let is_consonant = |p: &Phoneme| {
        matches!(
            p.class(),
            PhonemeClass::Plosive
                | PhonemeClass::Fricative
                | PhonemeClass::Nasal
                | PhonemeClass::Affricate
                | PhonemeClass::Approximant
                | PhonemeClass::Lateral
        )
    };

    // Find runs of consonants
    let mut run_start = None;
    for i in 0..=n {
        let is_cons = i < n && is_consonant(&events[i].phoneme);
        if is_cons && run_start.is_none() {
            run_start = Some(i);
        } else if !is_cons && let Some(start) = run_start {
            let run_len = i - start;
            if run_len >= 2 {
                for flag in &mut in_cluster[start..i] {
                    *flag = true;
                }
            }
            run_start = None;
        }
    }

    in_cluster
}

/// Crossfade easing using hisab's smootherstep (Ken Perlin's improved curve).
///
/// Maps `t` in `[0, 1]` to a smooth S-curve with zero first AND second derivatives
/// at endpoints — smoother than Hermite smoothstep for coarticulation blending.
#[inline]
fn sigmoid_fade(t: f32) -> f32 {
    hisab::calc::ease_in_out_smooth(t.clamp(0.0, 1.0))
}

/// Crossfades adjacent audio segments with per-boundary crossfade lengths.
///
/// Uses sigmoid interpolation for more natural coarticulatory blending.
/// `crossfade_lengths[i]` is the crossfade length between segment `i` and `i+1`.
fn crossfade_segments_variable(segments: &[Vec<f32>], crossfade_lengths: &[usize]) -> Vec<f32> {
    if segments.is_empty() {
        return Vec::new();
    }
    if segments.len() == 1 {
        return segments[0].clone();
    }

    // Estimate total output length
    let total_samples: usize = segments.iter().map(|s| s.len()).sum();
    let overlap: usize = crossfade_lengths.iter().sum();
    let estimated_len = total_samples.saturating_sub(overlap);
    let mut output = Vec::with_capacity(estimated_len);

    for (i, segment) in segments.iter().enumerate() {
        // Crossfade length to the NEXT segment (if any)
        let cf_next = if i < crossfade_lengths.len() {
            crossfade_lengths[i]
        } else {
            0
        };
        // Crossfade length from the PREVIOUS segment (if any)
        let cf_prev = if i > 0 { crossfade_lengths[i - 1] } else { 0 };

        if i == 0 {
            // First segment: add all but the last cf_next samples directly
            if segment.len() > cf_next {
                output.extend_from_slice(&segment[..segment.len() - cf_next]);
            }
            // Add the fade-out tail
            let fade_start = segment.len().saturating_sub(cf_next);
            for (j, &sample) in segment[fade_start..].iter().enumerate() {
                let t = j as f32 / cf_next.max(1) as f32;
                output.push(sample * (1.0 - sigmoid_fade(t)));
            }
        } else {
            // Blend with the tail of the previous segment
            let fade_len = cf_prev.min(segment.len());
            let output_len = output.len();

            for (j, &seg_sample) in segment.iter().enumerate().take(fade_len) {
                let t = j as f32 / fade_len.max(1) as f32;
                let idx = output_len - (fade_len - j);
                if idx < output.len() {
                    output[idx] += seg_sample * sigmoid_fade(t);
                }
            }

            // Add the rest of this segment
            if segment.len() > fade_len {
                if i < segments.len() - 1 && segment.len() > fade_len + cf_next {
                    // Not the last segment: leave room for next crossfade
                    output.extend_from_slice(&segment[fade_len..segment.len() - cf_next]);
                    let fade_start = segment.len() - cf_next;
                    for (j, &sample) in segment[fade_start..].iter().enumerate() {
                        let t = j as f32 / cf_next.max(1) as f32;
                        output.push(sample * (1.0 - sigmoid_fade(t)));
                    }
                } else {
                    output.extend_from_slice(&segment[fade_len..]);
                }
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phoneme::Phoneme;
    use alloc::vec;

    #[test]
    fn test_empty_sequence() {
        let seq = PhonemeSequence::new();
        assert!(seq.is_empty());
        assert_eq!(seq.len(), 0);
        let voice = VoiceProfile::new_male();
        let result = seq.render(&voice, 44100.0);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_single_phoneme() {
        let mut seq = PhonemeSequence::new();
        seq.push(PhonemeEvent::new(Phoneme::VowelA, 0.1, Stress::Primary));
        assert_eq!(seq.len(), 1);

        let voice = VoiceProfile::new_male();
        let result = seq.render(&voice, 44100.0);
        assert!(result.is_ok());
        let samples = result.unwrap();
        assert!(!samples.is_empty());
        assert!(samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn test_multi_phoneme() {
        let mut seq = PhonemeSequence::new();
        seq.push(PhonemeEvent::new(Phoneme::VowelA, 0.1, Stress::Primary));
        seq.push(PhonemeEvent::new(Phoneme::NasalN, 0.06, Stress::Unstressed));
        seq.push(PhonemeEvent::new(Phoneme::VowelI, 0.1, Stress::Secondary));

        let voice = VoiceProfile::new_male();
        let result = seq.render(&voice, 44100.0);
        assert!(result.is_ok());
        let samples = result.unwrap();
        assert!(!samples.is_empty());
        assert!(samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn test_total_duration() {
        let mut seq = PhonemeSequence::new();
        seq.push(PhonemeEvent::new(Phoneme::VowelA, 0.1, Stress::Unstressed));
        seq.push(PhonemeEvent::new(Phoneme::VowelE, 0.2, Stress::Unstressed));
        assert!((seq.total_duration() - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_crossfade_no_clicks() {
        let mut seq = PhonemeSequence::new();
        seq.push(PhonemeEvent::new(Phoneme::VowelA, 0.1, Stress::Primary));
        seq.push(PhonemeEvent::new(Phoneme::VowelI, 0.1, Stress::Primary));

        let voice = VoiceProfile::new_male();
        let samples = seq.render(&voice, 44100.0).unwrap();

        // Check for discontinuities (no sample-to-sample jumps > threshold)
        let max_jump = samples
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0f32, f32::max);

        // A "click" would be a very large jump relative to signal level
        let max_amp = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

        // Max jump should be small relative to signal amplitude
        if max_amp > 0.001 {
            assert!(
                max_jump < max_amp * 2.0,
                "potential click detected: max_jump={max_jump}, max_amp={max_amp}"
            );
        }
    }

    #[test]
    fn test_serde_roundtrip() {
        let mut seq = PhonemeSequence::new();
        seq.push(PhonemeEvent::new(Phoneme::VowelA, 0.1, Stress::Primary));
        let json = serde_json::to_string(&seq).unwrap();
        let seq2: PhonemeSequence = serde_json::from_str(&json).unwrap();
        assert_eq!(seq2.len(), 1);
    }

    #[test]
    fn test_cluster_detection_no_cluster() {
        let events = vec![
            PhonemeEvent::new(Phoneme::VowelA, 0.1, Stress::Primary),
            PhonemeEvent::new(Phoneme::NasalN, 0.06, Stress::Unstressed),
            PhonemeEvent::new(Phoneme::VowelI, 0.1, Stress::Primary),
        ];
        let clusters = detect_consonant_clusters(&events);
        // Single consonant between vowels is NOT a cluster
        assert!(!clusters[1]);
    }

    #[test]
    fn test_cluster_detection_pair() {
        let events = vec![
            PhonemeEvent::new(Phoneme::VowelA, 0.1, Stress::Primary),
            PhonemeEvent::new(Phoneme::FricativeS, 0.06, Stress::Unstressed),
            PhonemeEvent::new(Phoneme::PlosiveT, 0.06, Stress::Unstressed),
            PhonemeEvent::new(Phoneme::VowelI, 0.1, Stress::Primary),
        ];
        let clusters = detect_consonant_clusters(&events);
        assert!(!clusters[0]); // vowel
        assert!(clusters[1]); // /s/ in /st/ cluster
        assert!(clusters[2]); // /t/ in /st/ cluster
        assert!(!clusters[3]); // vowel
    }

    #[test]
    fn test_cluster_detection_triple() {
        // /str/ cluster
        let events = vec![
            PhonemeEvent::new(Phoneme::FricativeS, 0.06, Stress::Unstressed),
            PhonemeEvent::new(Phoneme::PlosiveT, 0.06, Stress::Unstressed),
            PhonemeEvent::new(Phoneme::ApproximantR, 0.06, Stress::Unstressed),
            PhonemeEvent::new(Phoneme::VowelI, 0.1, Stress::Primary),
        ];
        let clusters = detect_consonant_clusters(&events);
        assert!(clusters[0]); // /s/
        assert!(clusters[1]); // /t/
        assert!(clusters[2]); // /r/
        assert!(!clusters[3]); // vowel
    }

    #[test]
    fn test_cluster_renders_shorter() {
        // Sequence with cluster should be shorter than without
        let voice = VoiceProfile::new_male();

        // Without cluster: V-C-V
        let mut seq_no = PhonemeSequence::new();
        seq_no.push(PhonemeEvent::new(Phoneme::VowelA, 0.1, Stress::Unstressed));
        seq_no.push(PhonemeEvent::new(
            Phoneme::FricativeS,
            0.08,
            Stress::Unstressed,
        ));
        seq_no.push(PhonemeEvent::new(Phoneme::VowelI, 0.1, Stress::Unstressed));
        let out_no = seq_no.render(&voice, 44100.0).unwrap();

        // With cluster: V-CC-V
        let mut seq_cl = PhonemeSequence::new();
        seq_cl.push(PhonemeEvent::new(Phoneme::VowelA, 0.1, Stress::Unstressed));
        seq_cl.push(PhonemeEvent::new(
            Phoneme::FricativeS,
            0.08,
            Stress::Unstressed,
        ));
        seq_cl.push(PhonemeEvent::new(
            Phoneme::PlosiveT,
            0.08,
            Stress::Unstressed,
        ));
        seq_cl.push(PhonemeEvent::new(Phoneme::VowelI, 0.1, Stress::Unstressed));
        let out_cl = seq_cl.render(&voice, 44100.0).unwrap();

        assert!(out_cl.iter().all(|s| s.is_finite()));
        // The cluster version has an extra consonant but cluster compression
        // should make it shorter than naively adding another full-duration consonant
        let naive_extra = (0.08 * 0.9 * 44100.0) as usize; // unstressed full duration
        assert!(
            out_cl.len() < out_no.len() + naive_extra,
            "cluster should be compressed: cluster={}, no_cluster={}, naive_extra={}",
            out_cl.len(),
            out_no.len(),
            naive_extra
        );
    }
}
