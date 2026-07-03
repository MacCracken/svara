//! Error types for the svara crate.

use alloc::string::String;
use serde::{Deserialize, Serialize};

/// Errors that can occur during vocal synthesis operations.
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[non_exhaustive]
pub enum SvaraError {
    /// A formant frequency or bandwidth is out of valid range.
    #[error("invalid formant: {0}")]
    InvalidFormant(String),

    /// An unrecognized or unsupported phoneme was specified.
    #[error("invalid phoneme: {0}")]
    InvalidPhoneme(String),

    /// The pitch (f0) value is out of valid range.
    #[error("invalid pitch: {0}")]
    InvalidPitch(String),

    /// The duration value is out of valid range.
    #[error("invalid duration: {0}")]
    InvalidDuration(String),

    /// An articulation step failed during synthesis.
    #[error("articulation failed: {0}")]
    ArticulationFailed(String),

    /// A numeric computation produced an invalid result.
    #[error("computation error: {0}")]
    ComputationError(String),
}

/// Convenience type alias for svara results.
pub type Result<T> = core::result::Result<T, SvaraError>;
