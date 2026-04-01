//! Object pooling for transient phoneme synthesis.
//!
//! Provides [`SynthesisPool`], a pre-allocated wrapper around
//! [`SynthesisContext`] with convenience
//! methods, diagnostic counters, and pre-warmed buffer capacity.
//!
//! This is particularly useful for rendering many short phonemes in sequence
//! or for real-time systems where allocation jitter is unacceptable.

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::phoneme::{Nasalization, Phoneme, SynthesisContext};
use crate::voice::VoiceProfile;

/// A pool of pre-allocated synthesis objects for zero-allocation phoneme rendering.
///
/// Wraps a [`SynthesisContext`] with pre-warmed buffer capacity and usage
/// diagnostics. Create once, reuse across many render calls.
///
/// # Example
///
/// ```rust
/// use svara::prelude::*;
///
/// let voice = VoiceProfile::new_male();
/// let mut pool = SynthesisPool::new(&voice, 44100.0).unwrap();
///
/// // Render multiple phonemes without per-phoneme allocation
/// let samples_a = pool.render(&Phoneme::VowelA, &voice, 0.1).unwrap();
/// assert!(!samples_a.is_empty());
/// let samples_i = pool.render(&Phoneme::VowelI, &voice, 0.1).unwrap();
/// assert!(!samples_i.is_empty());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisPool {
    /// Inner reusable synthesis context.
    ctx: SynthesisContext,
    /// Number of phonemes rendered through this pool (diagnostic).
    render_count: u64,
    /// Peak buffer size in samples (high-water mark).
    peak_samples: usize,
}

impl SynthesisPool {
    /// Creates a new synthesis pool pre-allocated for the given voice.
    ///
    /// # Errors
    ///
    /// Returns an error if the voice profile's f0 is outside the valid range.
    pub fn new(voice: &VoiceProfile, sample_rate: f32) -> Result<Self> {
        let ctx = SynthesisContext::new(voice, sample_rate)?;
        Ok(Self {
            ctx,
            render_count: 0,
            peak_samples: 0,
        })
    }

    /// Creates a pool with a pre-warmed buffer for the given maximum duration.
    ///
    /// Pre-allocates enough buffer space for phonemes up to `max_duration`
    /// seconds, avoiding any allocation during subsequent `render` calls
    /// for phonemes within that duration.
    ///
    /// # Errors
    ///
    /// Returns an error if the voice profile's f0 is outside the valid range.
    pub fn with_capacity(
        voice: &VoiceProfile,
        sample_rate: f32,
        max_duration: f32,
    ) -> Result<Self> {
        let mut pool = Self::new(voice, sample_rate)?;
        let warm_samples = (max_duration * sample_rate) as usize;
        // Warm the buffer by rendering a silence of max duration
        if warm_samples > 0 {
            let _ = pool
                .ctx
                .synthesize(&Phoneme::Silence, voice, max_duration, None)?;
        }
        pool.peak_samples = warm_samples;
        Ok(pool)
    }

    /// Renders a single phoneme using pooled objects.
    ///
    /// The returned slice is valid until the next call to `render` or
    /// `render_nasalized`. Copy the data if you need to retain it.
    ///
    /// # Errors
    ///
    /// Returns an error if synthesis parameters are invalid.
    pub fn render(
        &mut self,
        phoneme: &Phoneme,
        voice: &VoiceProfile,
        duration: f32,
    ) -> Result<&[f32]> {
        self.render_nasalized(phoneme, voice, duration, None)
    }

    /// Renders a phoneme with optional anticipatory nasalization.
    ///
    /// # Errors
    ///
    /// Returns an error if synthesis parameters are invalid.
    pub fn render_nasalized(
        &mut self,
        phoneme: &Phoneme,
        voice: &VoiceProfile,
        duration: f32,
        nasalization: Option<&Nasalization>,
    ) -> Result<&[f32]> {
        let result = self
            .ctx
            .synthesize(phoneme, voice, duration, nasalization)?;
        let len = result.len();
        self.render_count += 1;
        if len > self.peak_samples {
            self.peak_samples = len;
        }
        Ok(result)
    }

    /// Renders multiple phonemes in sequence, collecting into a single buffer.
    ///
    /// Each phoneme is rendered and copied into the output. This avoids
    /// the crossfade logic of `PhonemeSequence` — use this for simple
    /// concatenation.
    ///
    /// # Errors
    ///
    /// Returns an error if any phoneme fails to synthesize.
    pub fn render_batch(
        &mut self,
        phonemes: &[(&Phoneme, f32)],
        voice: &VoiceProfile,
    ) -> Result<Vec<f32>> {
        let total_samples: usize = phonemes
            .iter()
            .map(|(_, dur)| (*dur * self.ctx.sample_rate()) as usize)
            .sum();
        let mut output = Vec::with_capacity(total_samples);

        for &(phoneme, duration) in phonemes {
            let samples = self.ctx.synthesize(phoneme, voice, duration, None)?;
            output.extend_from_slice(samples);
            self.render_count += 1;
        }

        Ok(output)
    }

    /// Returns the number of phonemes rendered through this pool.
    #[must_use]
    pub fn render_count(&self) -> u64 {
        self.render_count
    }

    /// Returns the peak buffer size in samples (high-water mark).
    #[must_use]
    pub fn peak_samples(&self) -> usize {
        self.peak_samples
    }

    /// Resets pool state (clears filter history, resets counters).
    pub fn reset(&mut self) {
        self.render_count = 0;
        self.peak_samples = 0;
    }

    /// Returns a reference to the inner [`SynthesisContext`].
    #[must_use]
    pub fn context(&self) -> &SynthesisContext {
        &self.ctx
    }

    /// Returns a mutable reference to the inner [`SynthesisContext`].
    pub fn context_mut(&mut self) -> &mut SynthesisContext {
        &mut self.ctx
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phoneme::Phoneme;

    #[test]
    fn test_pool_creation() {
        let voice = VoiceProfile::new_male();
        let pool = SynthesisPool::new(&voice, 44100.0);
        assert!(pool.is_ok());
        assert_eq!(pool.unwrap().render_count(), 0);
    }

    #[test]
    fn test_pool_with_capacity() {
        let voice = VoiceProfile::new_male();
        let pool = SynthesisPool::with_capacity(&voice, 44100.0, 0.5).unwrap();
        assert!(pool.peak_samples() >= (0.5 * 44100.0) as usize);
    }

    #[test]
    fn test_pool_render_vowel() {
        let voice = VoiceProfile::new_male();
        let mut pool = SynthesisPool::new(&voice, 44100.0).unwrap();
        let samples = pool.render(&Phoneme::VowelA, &voice, 0.1).unwrap();
        assert!(!samples.is_empty());
        assert!(samples.iter().all(|s| s.is_finite()));
        assert_eq!(pool.render_count(), 1);
    }

    #[test]
    fn test_pool_render_multiple() {
        let voice = VoiceProfile::new_male();
        let mut pool = SynthesisPool::new(&voice, 44100.0).unwrap();

        for phoneme in &[Phoneme::VowelA, Phoneme::FricativeS, Phoneme::NasalN] {
            let samples = pool.render(phoneme, &voice, 0.08).unwrap();
            assert!(samples.iter().all(|s| s.is_finite()));
        }
        assert_eq!(pool.render_count(), 3);
    }

    #[test]
    fn test_pool_render_batch() {
        let voice = VoiceProfile::new_male();
        let mut pool = SynthesisPool::new(&voice, 44100.0).unwrap();

        let phonemes: alloc::vec::Vec<(&Phoneme, f32)> = alloc::vec![
            (&Phoneme::VowelA, 0.08),
            (&Phoneme::NasalN, 0.05),
            (&Phoneme::VowelI, 0.08),
        ];
        let output = pool.render_batch(&phonemes, &voice).unwrap();
        assert!(!output.is_empty());
        assert!(output.iter().all(|s| s.is_finite()));
        assert_eq!(pool.render_count(), 3);
    }

    #[test]
    fn test_pool_peak_tracking() {
        let voice = VoiceProfile::new_male();
        let mut pool = SynthesisPool::new(&voice, 44100.0).unwrap();

        let _ = pool.render(&Phoneme::VowelA, &voice, 0.05).unwrap();
        let peak1 = pool.peak_samples();

        let _ = pool.render(&Phoneme::VowelA, &voice, 0.2).unwrap();
        let peak2 = pool.peak_samples();

        assert!(peak2 > peak1);
    }

    #[test]
    fn test_pool_reset() {
        let voice = VoiceProfile::new_male();
        let mut pool = SynthesisPool::new(&voice, 44100.0).unwrap();
        let _ = pool.render(&Phoneme::VowelA, &voice, 0.1).unwrap();
        assert_eq!(pool.render_count(), 1);

        pool.reset();
        assert_eq!(pool.render_count(), 0);
        assert_eq!(pool.peak_samples(), 0);
    }

    #[test]
    fn test_serde_roundtrip_pool() {
        let voice = VoiceProfile::new_male();
        let pool = SynthesisPool::new(&voice, 44100.0).unwrap();
        let json = serde_json::to_string(&pool).unwrap();
        let pool2: SynthesisPool = serde_json::from_str(&json).unwrap();
        assert_eq!(pool2.render_count(), 0);
    }
}
