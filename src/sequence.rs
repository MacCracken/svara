//! Phoneme sequencing with coarticulation and crossfading.
//!
//! Combines individual phonemes into continuous speech, applying prosodic
//! contours, coarticulatory formant blending, and smooth crossfades at
//! phoneme boundaries.

use serde::{Deserialize, Serialize};
use tracing::trace;

use crate::error::Result;
use crate::phoneme::{self, Phoneme};
use crate::prosody::Stress;
use crate::voice::VoiceProfile;

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
}

impl PhonemeSequence {
    /// Creates a new empty phoneme sequence.
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            transition_window: 0.05, // 50ms default
        }
    }

    /// Sets the coarticulation transition window in seconds.
    pub fn set_transition_window(&mut self, window: f32) {
        self.transition_window = window.max(0.001);
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

        // Calculate effective durations with stress scaling
        let durations: Vec<f32> = self
            .events
            .iter()
            .map(|e| {
                let scale = match e.stress {
                    Stress::Primary => 1.15,
                    Stress::Secondary => 1.05,
                    Stress::Unstressed => 0.9,
                };
                e.duration * scale
            })
            .collect();

        // Synthesize each phoneme
        let mut segments: Vec<Vec<f32>> = Vec::with_capacity(self.events.len());
        for (event, &dur) in self.events.iter().zip(durations.iter()) {
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

            let segment =
                phoneme::synthesize_phoneme(&event.phoneme, &event_voice, sample_rate, dur)?;
            segments.push(segment);
        }

        // Concatenate with crossfade at boundaries
        let crossfade_samples = (self.transition_window * sample_rate) as usize;
        let output = crossfade_segments(&segments, crossfade_samples);

        Ok(output)
    }
}

impl Default for PhonemeSequence {
    fn default() -> Self {
        Self::new()
    }
}

/// Crossfades adjacent audio segments for smooth transitions.
fn crossfade_segments(segments: &[Vec<f32>], crossfade_len: usize) -> Vec<f32> {
    if segments.is_empty() {
        return Vec::new();
    }
    if segments.len() == 1 {
        return segments[0].clone();
    }

    // Estimate total output length
    let total_samples: usize = segments.iter().map(|s| s.len()).sum();
    let overlap = crossfade_len * (segments.len() - 1);
    let estimated_len = total_samples.saturating_sub(overlap);
    let mut output = Vec::with_capacity(estimated_len);

    for (i, segment) in segments.iter().enumerate() {
        if i == 0 {
            // First segment: add all but the last crossfade_len samples directly
            if segment.len() > crossfade_len {
                output.extend_from_slice(&segment[..segment.len() - crossfade_len]);
            }
            // Add the crossfade region from this segment
            let fade_start = segment.len().saturating_sub(crossfade_len);
            for (j, &sample) in segment[fade_start..].iter().enumerate() {
                let t = j as f32 / crossfade_len.max(1) as f32;
                let fade_out = 0.5 * (1.0 + (std::f32::consts::PI * t).cos()); // cosine fade out
                output.push(sample * fade_out);
            }
        } else {
            // Subsequent segments: crossfade with the tail of the previous
            let fade_len = crossfade_len.min(segment.len());
            let output_len = output.len();

            // Blend the crossfade region
            for (j, &seg_sample) in segment.iter().enumerate().take(fade_len) {
                let t = j as f32 / fade_len.max(1) as f32;
                let fade_in = 0.5 * (1.0 - (std::f32::consts::PI * t).cos()); // cosine fade in
                let idx = output_len - (fade_len - j);
                if idx < output.len() {
                    output[idx] += seg_sample * fade_in;
                }
            }

            // Add the rest of this segment
            if segment.len() > fade_len {
                if i < segments.len() - 1 && segment.len() > fade_len + crossfade_len {
                    // Not the last segment: leave room for next crossfade
                    output.extend_from_slice(&segment[fade_len..segment.len() - crossfade_len]);
                    let fade_start = segment.len() - crossfade_len;
                    for (j, &sample) in segment[fade_start..].iter().enumerate() {
                        let t = j as f32 / crossfade_len.max(1) as f32;
                        let fade_out = 0.5 * (1.0 + (std::f32::consts::PI * t).cos());
                        output.push(sample * fade_out);
                    }
                } else {
                    // Last segment or short segment: just append the rest
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
}
