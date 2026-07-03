//! Shared DSP utilities for svara synthesis modules.

use alloc::format;

/// Validates that a sample rate is positive and finite.
#[inline]
#[allow(dead_code)]
pub(crate) fn validate_sample_rate(sample_rate: f32) -> crate::error::Result<()> {
    if sample_rate <= 0.0 || !sample_rate.is_finite() {
        return Err(crate::error::SvaraError::InvalidFormant(format!(
            "sample_rate must be positive and finite, got {sample_rate}"
        )));
    }
    Ok(())
}

/// Validates that a duration is positive and finite.
#[inline]
#[allow(dead_code)]
pub(crate) fn validate_duration(duration: f32) -> crate::error::Result<()> {
    if duration <= 0.0 || !duration.is_finite() {
        return Err(crate::error::SvaraError::InvalidDuration(format!(
            "duration must be positive and finite, got {duration}"
        )));
    }
    Ok(())
}

/// Maps a naad backend error to a `SvaraError` with context.
#[cfg(feature = "naad-backend")]
#[allow(dead_code)]
pub(crate) fn map_naad_error(component: &str, err: naad::NaadError) -> crate::error::SvaraError {
    let msg = format!("{component} init failed: {err}");
    #[cfg(feature = "logging")]
    tracing::error!(component, %err, "naad backend error");
    crate::error::SvaraError::ComputationError(msg)
}
