//! Async rendering API for non-real-time batch synthesis.
//!
//! Provides [`BatchRenderer`] for rendering phoneme sequences asynchronously,
//! yielding between phonemes to avoid blocking. This is useful for offline
//! TTS, audio file generation, or any context where synthesis runs alongside
//! other async tasks.
//!
//! Requires the `std` feature (async traits use `Box`/`Pin`).

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::phoneme::{Phoneme, SynthesisContext};
use crate::prosody::Stress;
use crate::sequence::PhonemeEvent;
use crate::voice::VoiceProfile;

/// Progress information emitted during batch rendering.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RenderProgress {
    /// Index of the phoneme just completed (0-based).
    pub phoneme_index: usize,
    /// Total number of phonemes in the batch.
    pub total_phonemes: usize,
    /// Samples rendered so far.
    pub samples_rendered: usize,
}

impl RenderProgress {
    /// Returns the fraction complete (0.0 to 1.0).
    #[must_use]
    pub fn fraction(&self) -> f32 {
        if self.total_phonemes == 0 {
            1.0
        } else {
            self.phoneme_index as f32 / self.total_phonemes as f32
        }
    }
}

/// Non-real-time batch renderer for phoneme sequences.
///
/// Renders phonemes one at a time, collecting results into a single output
/// buffer. Designed for offline synthesis where latency is not critical
/// but throughput and integration with other work matters.
///
/// # Example
///
/// ```rust
/// use svara::prelude::*;
/// use svara::render::BatchRenderer;
///
/// let voice = VoiceProfile::new_male();
/// let mut renderer = BatchRenderer::new(&voice, 44100.0).unwrap();
///
/// renderer.push(Phoneme::VowelA, 0.1, Stress::Primary);
/// renderer.push(Phoneme::NasalN, 0.06, Stress::Unstressed);
/// renderer.push(Phoneme::VowelI, 0.1, Stress::Primary);
///
/// let output = renderer.render_all().unwrap();
/// assert!(!output.samples.is_empty());
/// assert_eq!(output.progress.phoneme_index, 3);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRenderer {
    /// Phoneme events to render.
    events: Vec<PhonemeEvent>,
    /// Reusable synthesis context.
    ctx: SynthesisContext,
    /// Voice profile for rendering.
    voice: VoiceProfile,
    /// Sample rate in Hz.
    sample_rate: f32,
}

/// The result of a batch render operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderOutput {
    /// The rendered audio samples.
    pub samples: Vec<f32>,
    /// Final progress state.
    pub progress: RenderProgress,
}

impl BatchRenderer {
    /// Creates a new batch renderer for the given voice and sample rate.
    ///
    /// # Errors
    ///
    /// Returns an error if the voice profile's f0 is outside the valid range.
    pub fn new(voice: &VoiceProfile, sample_rate: f32) -> Result<Self> {
        let ctx = SynthesisContext::new(voice, sample_rate)?;
        Ok(Self {
            events: Vec::new(),
            ctx,
            voice: voice.clone(),
            sample_rate,
        })
    }

    /// Adds a phoneme to the render queue.
    pub fn push(&mut self, phoneme: Phoneme, duration: f32, stress: Stress) {
        self.events
            .push(PhonemeEvent::new(phoneme, duration, stress));
    }

    /// Adds multiple phoneme events to the render queue.
    pub fn extend(&mut self, events: &[PhonemeEvent]) {
        self.events.extend_from_slice(events);
    }

    /// Clears the render queue.
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Returns the number of queued phonemes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Returns whether the queue is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Renders all queued phonemes synchronously, returning the concatenated audio.
    ///
    /// Applies stress-based voice modification and anticipatory nasalization
    /// (same as `PhonemeSequence::render` but without crossfading).
    ///
    /// # Errors
    ///
    /// Returns an error if any phoneme fails to synthesize.
    pub fn render_all(&mut self) -> Result<RenderOutput> {
        self.render_with_progress(|_| {})
    }

    /// Renders phonemes one at a time, calling `on_progress` after each.
    ///
    /// This allows callers to report progress, update UI, or yield to other
    /// work between phonemes. For true async, wrap this in a `spawn_blocking`
    /// or equivalent.
    ///
    /// # Errors
    ///
    /// Returns an error if any phoneme fails to synthesize.
    pub fn render_with_progress<F>(&mut self, mut on_progress: F) -> Result<RenderOutput>
    where
        F: FnMut(&RenderProgress),
    {
        let mut output = Vec::new();
        let total = self.events.len();

        let phoneme_list: Vec<Phoneme> = self.events.iter().map(|e| e.phoneme).collect();
        let nasalizations = crate::phoneme::detect_nasalization(&phoneme_list);

        for (i, event) in self.events.iter().enumerate() {
            let mut event_voice = self.voice.clone();
            let stress_scale = match event.stress {
                Stress::Primary => {
                    event_voice.base_f0 *= 1.10;
                    1.15
                }
                Stress::Secondary => {
                    event_voice.base_f0 *= 1.05;
                    1.05
                }
                Stress::Unstressed => 0.9,
            };
            let dur = event.duration * stress_scale;

            let samples = self.ctx.synthesize(
                &event.phoneme,
                &event_voice,
                dur,
                nasalizations[i].as_ref(),
            )?;
            output.extend_from_slice(samples);

            on_progress(&RenderProgress {
                phoneme_index: i + 1,
                total_phonemes: total,
                samples_rendered: output.len(),
            });
        }

        let final_len = output.len();
        Ok(RenderOutput {
            samples: output,
            progress: RenderProgress {
                phoneme_index: total,
                total_phonemes: total,
                samples_rendered: final_len,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_renderer_creation() {
        let voice = VoiceProfile::new_male();
        let renderer = BatchRenderer::new(&voice, 44100.0);
        assert!(renderer.is_ok());
        assert!(renderer.unwrap().is_empty());
    }

    #[test]
    fn test_batch_render_single() {
        let voice = VoiceProfile::new_male();
        let mut renderer = BatchRenderer::new(&voice, 44100.0).unwrap();
        renderer.push(Phoneme::VowelA, 0.1, Stress::Primary);

        let output = renderer.render_all().unwrap();
        assert!(!output.samples.is_empty());
        assert!(output.samples.iter().all(|s| s.is_finite()));
        assert_eq!(output.progress.phoneme_index, 1);
        assert_eq!(output.progress.total_phonemes, 1);
    }

    #[test]
    fn test_batch_render_sequence() {
        let voice = VoiceProfile::new_male();
        let mut renderer = BatchRenderer::new(&voice, 44100.0).unwrap();
        renderer.push(Phoneme::VowelA, 0.1, Stress::Primary);
        renderer.push(Phoneme::NasalN, 0.06, Stress::Unstressed);
        renderer.push(Phoneme::VowelI, 0.1, Stress::Primary);

        let output = renderer.render_all().unwrap();
        assert!(!output.samples.is_empty());
        assert_eq!(output.progress.total_phonemes, 3);
    }

    #[test]
    fn test_batch_render_with_progress() {
        let voice = VoiceProfile::new_male();
        let mut renderer = BatchRenderer::new(&voice, 44100.0).unwrap();
        renderer.push(Phoneme::VowelA, 0.08, Stress::Primary);
        renderer.push(Phoneme::FricativeS, 0.06, Stress::Unstressed);
        renderer.push(Phoneme::VowelE, 0.08, Stress::Primary);

        let mut progress_calls = 0u32;
        let output = renderer
            .render_with_progress(|p| {
                progress_calls += 1;
                assert!(p.fraction() <= 1.0);
                assert!(p.samples_rendered > 0);
            })
            .unwrap();

        assert_eq!(progress_calls, 3);
        assert!(!output.samples.is_empty());
    }

    #[test]
    fn test_batch_render_empty() {
        let voice = VoiceProfile::new_male();
        let mut renderer = BatchRenderer::new(&voice, 44100.0).unwrap();
        let output = renderer.render_all().unwrap();
        assert!(output.samples.is_empty());
        assert_eq!(output.progress.total_phonemes, 0);
    }

    #[test]
    fn test_render_progress_fraction() {
        let p = RenderProgress {
            phoneme_index: 5,
            total_phonemes: 10,
            samples_rendered: 22050,
        };
        assert!((p.fraction() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_batch_renderer_clear() {
        let voice = VoiceProfile::new_male();
        let mut renderer = BatchRenderer::new(&voice, 44100.0).unwrap();
        renderer.push(Phoneme::VowelA, 0.1, Stress::Primary);
        assert_eq!(renderer.len(), 1);
        renderer.clear();
        assert!(renderer.is_empty());
    }

    #[test]
    fn test_serde_roundtrip_batch_renderer() {
        let voice = VoiceProfile::new_male();
        let mut renderer = BatchRenderer::new(&voice, 44100.0).unwrap();
        renderer.push(Phoneme::VowelA, 0.1, Stress::Primary);
        let json = serde_json::to_string(&renderer).unwrap();
        let r2: BatchRenderer = serde_json::from_str(&json).unwrap();
        assert_eq!(r2.len(), 1);
    }

    #[test]
    fn test_serde_roundtrip_render_output() {
        let voice = VoiceProfile::new_male();
        let mut renderer = BatchRenderer::new(&voice, 44100.0).unwrap();
        renderer.push(Phoneme::VowelA, 0.05, Stress::Primary);
        let output = renderer.render_all().unwrap();
        let json = serde_json::to_string(&output).unwrap();
        let o2: RenderOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(o2.samples.len(), output.samples.len());
        assert_eq!(o2.progress.total_phonemes, 1);
    }

    #[test]
    fn test_serde_roundtrip_render_progress() {
        let p = RenderProgress {
            phoneme_index: 3,
            total_phonemes: 10,
            samples_rendered: 44100,
        };
        let json = serde_json::to_string(&p).unwrap();
        let p2: RenderProgress = serde_json::from_str(&json).unwrap();
        assert_eq!(p2.phoneme_index, 3);
    }
}
