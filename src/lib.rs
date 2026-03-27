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
//! - **`logging`**: Enable tracing-subscriber for structured log output.

pub mod error;
pub mod formant;
pub mod glottal;
pub mod phoneme;
pub mod prosody;
pub mod sequence;
pub mod tract;
pub mod voice;

/// Convenience re-exports for common usage.
pub mod prelude {
    pub use crate::error::{Result, SvaraError};
    pub use crate::formant::{Formant, FormantFilter, Vowel, VowelTarget};
    pub use crate::glottal::GlottalSource;
    pub use crate::phoneme::{
        Phoneme, PhonemeClass, phoneme_duration, phoneme_formants, synthesize_phoneme,
    };
    pub use crate::prosody::{IntonationPattern, ProsodyContour, Stress};
    pub use crate::sequence::{PhonemeEvent, PhonemeSequence};
    pub use crate::tract::VocalTract;
    pub use crate::voice::VoiceProfile;
}
