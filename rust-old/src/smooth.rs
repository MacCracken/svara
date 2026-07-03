//! One-pole parameter smoother for click-free real-time parameter changes.
//!
//! When a synthesis parameter (e.g., nasal coupling, gain) changes abruptly,
//! the resulting discontinuity can produce audible clicks. `SmoothedParam`
//! applies exponential smoothing (one-pole low-pass) so the parameter glides
//! smoothly to its target value.
//!
//! The smoothing time constant determines how quickly the parameter reaches
//! its target: ~63% after one time constant, ~95% after three.

use serde::{Deserialize, Serialize};

/// Default smoothing time constant in seconds (~5ms, fast enough for speech).
const DEFAULT_SMOOTH_TIME: f32 = 0.005;

/// One-pole exponential parameter smoother.
///
/// Tracks a target value, smoothly approaching it over a configurable
/// time constant. Zero-cost when the value has already converged.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SmoothedParam {
    /// Current smoothed value.
    current: f32,
    /// Target value to approach.
    target: f32,
    /// Smoothing coefficient (0.0 = instant, 1.0 = never moves).
    alpha: f32,
}

impl SmoothedParam {
    /// Creates a new smoother at the given initial value.
    pub fn new(initial: f32, sample_rate: f32) -> Self {
        Self {
            current: initial,
            target: initial,
            alpha: Self::alpha_from_time(DEFAULT_SMOOTH_TIME, sample_rate),
        }
    }

    /// Creates a smoother with a custom time constant.
    #[allow(dead_code)]
    pub fn with_time(initial: f32, smooth_time_secs: f32, sample_rate: f32) -> Self {
        Self {
            current: initial,
            target: initial,
            alpha: Self::alpha_from_time(smooth_time_secs, sample_rate),
        }
    }

    /// Sets a new target value. The smoother will glide toward it.
    #[inline]
    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    /// Returns the current smoothed value and advances one sample.
    #[inline]
    pub fn next(&mut self) -> f32 {
        // Skip smoothing when converged (saves a multiply)
        if (self.current - self.target).abs() < 1e-8 {
            self.current = self.target;
        } else {
            self.current = self.alpha * self.current + (1.0 - self.alpha) * self.target;
        }
        self.current
    }

    /// Returns the current value without advancing.
    #[inline]
    #[allow(dead_code)]
    pub fn value(&self) -> f32 {
        self.current
    }

    /// Instantly sets both current and target (no smoothing).
    #[inline]
    #[allow(dead_code)]
    pub fn set_immediate(&mut self, value: f32) {
        self.current = value;
        self.target = value;
    }

    /// Computes alpha from a time constant in seconds.
    fn alpha_from_time(time_secs: f32, sample_rate: f32) -> f32 {
        if time_secs <= 0.0 || sample_rate <= 0.0 {
            return 0.0; // Instant
        }
        let samples = time_secs * sample_rate;
        crate::math::f32::exp(-1.0 / samples)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_immediate_convergence() {
        let mut p = SmoothedParam::new(0.0, 44100.0);
        p.set_immediate(1.0);
        assert!((p.next() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_smooth_approach() {
        let mut p = SmoothedParam::new(0.0, 44100.0);
        p.set_target(1.0);
        // After ~5x the time constant (5ms * 5 = 25ms = 1102 samples), should be >99%
        for _ in 0..2000 {
            p.next();
        }
        assert!(
            (p.next() - 1.0).abs() < 0.001,
            "should converge: got {}",
            p.value()
        );
    }

    #[test]
    fn test_zero_time_is_instant() {
        let mut p = SmoothedParam::with_time(0.0, 0.0, 44100.0);
        p.set_target(1.0);
        assert!((p.next() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_serde_roundtrip() {
        let p = SmoothedParam::new(0.5, 44100.0);
        let json = serde_json::to_string(&p).unwrap();
        let p2: SmoothedParam = serde_json::from_str(&json).unwrap();
        assert!((p2.value() - 0.5).abs() < f32::EPSILON);
    }
}
