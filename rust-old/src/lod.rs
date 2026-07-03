//! Level-of-detail quality control for multi-voice scenarios.
//!
//! When rendering many simultaneous voices (e.g., crowd scenes, chorus),
//! lower-quality settings reduce CPU cost per voice by simplifying the
//! synthesis pipeline.

use serde::{Deserialize, Serialize};

/// Synthesis quality level controlling pipeline complexity.
///
/// Higher quality uses the full pipeline; lower quality skips expensive
/// processing stages to reduce CPU cost per voice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Quality {
    /// Full pipeline: all formants, nasal coupling, subglottal resonance,
    /// source-filter interaction, lip radiation. Best for solo/foreground voices.
    Full,
    /// Reduced pipeline: 3 formants instead of 5, no subglottal resonance,
    /// no source-filter interaction. Suitable for mid-distance voices.
    Reduced,
    /// Minimal pipeline: 2 formants, no nasal coupling, no subglottal resonance,
    /// no source-filter interaction, no lip radiation. Suitable for background/crowd.
    Minimal,
}

impl Quality {
    /// Returns the maximum number of formants for this quality level.
    #[must_use]
    #[inline]
    pub fn max_formants(self) -> usize {
        match self {
            Self::Full => 5,
            Self::Reduced => 3,
            Self::Minimal => 2,
        }
    }

    /// Whether nasal coupling should be applied at this quality level.
    #[must_use]
    #[inline]
    pub fn use_nasal_coupling(self) -> bool {
        match self {
            Self::Full | Self::Reduced => true,
            Self::Minimal => false,
        }
    }

    /// Whether subglottal resonance should be applied.
    #[must_use]
    #[inline]
    pub fn use_subglottal(self) -> bool {
        matches!(self, Self::Full)
    }

    /// Whether source-filter interaction feedback should be applied.
    #[must_use]
    #[inline]
    pub fn use_interaction(self) -> bool {
        matches!(self, Self::Full)
    }

    /// Whether lip radiation filter should be applied.
    #[must_use]
    #[inline]
    pub fn use_lip_radiation(self) -> bool {
        match self {
            Self::Full | Self::Reduced => true,
            Self::Minimal => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_formant_counts() {
        assert_eq!(Quality::Full.max_formants(), 5);
        assert_eq!(Quality::Reduced.max_formants(), 3);
        assert_eq!(Quality::Minimal.max_formants(), 2);
    }

    #[test]
    fn test_quality_feature_flags() {
        // Full has everything
        assert!(Quality::Full.use_nasal_coupling());
        assert!(Quality::Full.use_subglottal());
        assert!(Quality::Full.use_interaction());
        assert!(Quality::Full.use_lip_radiation());

        // Reduced drops subglottal and interaction
        assert!(Quality::Reduced.use_nasal_coupling());
        assert!(!Quality::Reduced.use_subglottal());
        assert!(!Quality::Reduced.use_interaction());
        assert!(Quality::Reduced.use_lip_radiation());

        // Minimal drops everything except basic formants
        assert!(!Quality::Minimal.use_nasal_coupling());
        assert!(!Quality::Minimal.use_subglottal());
        assert!(!Quality::Minimal.use_interaction());
        assert!(!Quality::Minimal.use_lip_radiation());
    }

    #[test]
    fn test_serde_roundtrip() {
        for quality in [Quality::Full, Quality::Reduced, Quality::Minimal] {
            let json = serde_json::to_string(&quality).unwrap();
            let q2: Quality = serde_json::from_str(&json).unwrap();
            assert_eq!(q2, quality);
        }
    }
}
