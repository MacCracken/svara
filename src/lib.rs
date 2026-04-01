//! # svara — Formant and Vocal Synthesis
//!
//! **svara** (Sanskrit: voice/tone/musical note) provides a complete formant-based
//! vocal synthesis pipeline: glottal source generation, vocal tract modeling,
//! phoneme-level synthesis, prosodic control, and sequenced speech rendering.
//!
//! ## Architecture
//!
//! The synthesis pipeline flows:
//!
//! ```text
//! GlottalSource → VocalTract (FormantFilter + NasalCoupling + LipRadiation) → Output
//! ```
//!
//! Higher-level constructs build on this:
//!
//! - **Phonemes** define articulatory targets and synthesis strategies per sound class
//! - **Prosody** controls f0 contour, stress, and intonation patterns
//! - **Voice profiles** parameterize speaker characteristics
//! - **Sequences** combine phonemes with coarticulation and crossfading
//!
//! ## Quick Start
//!
//! ```rust
//! use svara::prelude::*;
//!
//! // Create a male voice and synthesize a vowel
//! let voice = VoiceProfile::new_male();
//! let samples = synthesize_phoneme(
//!     &Phoneme::VowelA,
//!     &voice,
//!     44100.0,
//!     0.5,
//! ).expect("synthesis should succeed");
//!
//! // Or build a phoneme sequence
//! let mut seq = PhonemeSequence::new();
//! seq.push(PhonemeEvent::new(Phoneme::VowelA, 0.15, Stress::Primary));
//! seq.push(PhonemeEvent::new(Phoneme::NasalN, 0.08, Stress::Unstressed));
//! seq.push(PhonemeEvent::new(Phoneme::VowelI, 0.15, Stress::Secondary));
//! let audio = seq.render(&voice, 44100.0).expect("render should succeed");
//! ```
//!
//! ## Feature Flags
//!
//! - **`naad-backend`** (default): Use naad crate for oscillators, filters, and noise.
//!   Without this, svara uses internal minimal implementations.
//! - **`std`** (default): Enable standard library. Disable for `no_std` environments
//!   (requires `alloc`).
//! - **`logging`**: Enable tracing-subscriber for structured log output.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod bridge;
mod dsp;
pub mod error;
pub mod formant;
pub mod glottal;
pub mod lod;
mod math;
pub mod phoneme;
pub mod prosody;
pub(crate) mod rng;
pub mod sequence;
pub(crate) mod smooth;
pub mod spectral;
pub mod tract;
pub mod voice;

/// Convenience re-exports for common usage.
pub mod prelude {
    pub use crate::error::{Result, SvaraError};
    pub use crate::formant::{Formant, FormantFilter, Vowel, VowelTarget};
    pub use crate::glottal::{GlottalModel, GlottalSource};
    pub use crate::lod::Quality;
    pub use crate::phoneme::{
        Nasalization, Phoneme, PhonemeClass, SynthesisContext, f2_locus_equation, phoneme_duration,
        phoneme_formants, synthesize_phoneme, synthesize_phoneme_nasalized,
    };
    pub use crate::prosody::{IntonationPattern, ProsodyContour, Stress};
    pub use crate::sequence::{PhonemeEvent, PhonemeSequence};
    pub use crate::spectral::{Spectrum, analyze as analyze_spectrum, rms_level};
    pub use crate::tract::{NasalPlace, VocalTract};
    pub use crate::voice::{EffortParams, VocalEffort, VoiceProfile};
}

// Compile-time trait assertions: all public types must be Send + Sync
// for safe multi-voice parallel rendering.
#[cfg(test)]
mod assert_traits {
    fn _assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn public_types_are_send_sync() {
        _assert_send_sync::<crate::error::SvaraError>();
        _assert_send_sync::<crate::formant::Formant>();
        _assert_send_sync::<crate::formant::FormantFilter>();
        _assert_send_sync::<crate::formant::Vowel>();
        _assert_send_sync::<crate::formant::VowelTarget>();
        _assert_send_sync::<crate::glottal::GlottalSource>();
        _assert_send_sync::<crate::glottal::GlottalModel>();
        _assert_send_sync::<crate::lod::Quality>();
        _assert_send_sync::<crate::phoneme::Phoneme>();
        _assert_send_sync::<crate::phoneme::PhonemeClass>();
        _assert_send_sync::<crate::phoneme::Nasalization>();
        _assert_send_sync::<crate::phoneme::SynthesisContext>();
        _assert_send_sync::<crate::prosody::ProsodyContour>();
        _assert_send_sync::<crate::prosody::IntonationPattern>();
        _assert_send_sync::<crate::prosody::Stress>();
        _assert_send_sync::<crate::sequence::PhonemeEvent>();
        _assert_send_sync::<crate::sequence::PhonemeSequence>();
        _assert_send_sync::<crate::tract::VocalTract>();
        _assert_send_sync::<crate::tract::NasalPlace>();
        _assert_send_sync::<crate::voice::VoiceProfile>();
        _assert_send_sync::<crate::voice::VocalEffort>();
        _assert_send_sync::<crate::voice::EffortParams>();
        _assert_send_sync::<crate::spectral::Spectrum>();
    }
}
