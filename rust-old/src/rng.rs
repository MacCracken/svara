//! Deterministic PRNG for synthesis noise generation.
//!
//! Provides a shared PCG32 implementation used by both glottal source (aspiration)
//! and phoneme synthesis (plosive/fricative noise bursts). Serializable for state
//! persistence and deterministic replay.

use serde::{Deserialize, Serialize};

/// Default PRNG seed for deterministic noise generation.
pub(crate) const DEFAULT_SEED: u64 = 42;

/// PCG32-based PRNG for noise generation (jitter, shimmer, breathiness).
///
/// Uses the PCG (Permuted Congruential Generator) algorithm for
/// high-quality, deterministic random numbers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Rng {
    state: u64,
    inc: u64,
}

impl Rng {
    /// Creates a new PRNG with the given seed.
    pub fn new(seed: u64) -> Self {
        let inc = (seed << 1) | 1;
        let mut rng = Self { state: 0, inc };
        rng.next_u32();
        rng.state = rng.state.wrapping_add(seed);
        rng.next_u32();
        rng
    }

    /// Generates the next u32 using PCG32 algorithm.
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        let old_state = self.state;
        self.state = old_state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(self.inc);
        let xor_shifted = (((old_state >> 18) ^ old_state) >> 27) as u32;
        let rot = (old_state >> 59) as u32;
        (xor_shifted >> rot) | (xor_shifted << (rot.wrapping_neg() & 31))
    }

    /// Returns a value in [-1.0, 1.0].
    #[inline]
    pub fn next_f32(&mut self) -> f32 {
        let bits = (self.next_u32() >> 1) as i32;
        bits as f32 * (1.0 / i32::MAX as f32)
    }

    /// Returns a value in [0.0, 1.0].
    #[inline]
    #[allow(dead_code)]
    pub fn next_f32_unsigned(&mut self) -> f32 {
        self.next_u32() as f32 * (1.0 / u32::MAX as f32)
    }
}
