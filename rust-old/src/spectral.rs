//! Spectral analysis tools for vocal synthesis.
//!
//! Provides FFT-based spectral analysis using hisab's FFT implementation,
//! enabling formant tracking, spectral envelope extraction, and frequency-domain
//! operations on synthesized speech.

use alloc::vec::Vec;

use hisab::num::Complex;

use crate::error::{Result, SvaraError};

/// Spectral analysis result for a frame of audio.
#[derive(Debug, Clone)]
pub struct Spectrum {
    /// Magnitude spectrum (linear scale), length = fft_size / 2 + 1.
    pub magnitudes: Vec<f32>,
    /// Frequency resolution in Hz per bin.
    pub freq_resolution: f32,
    /// Sample rate used for analysis.
    pub sample_rate: f32,
}

impl Spectrum {
    /// Returns the frequency in Hz for a given bin index.
    #[must_use]
    #[inline]
    pub fn bin_frequency(&self, bin: usize) -> f32 {
        bin as f32 * self.freq_resolution
    }

    /// Returns the bin index closest to a given frequency.
    #[must_use]
    #[inline]
    pub fn frequency_bin(&self, freq: f32) -> usize {
        ((freq / self.freq_resolution) + 0.5) as usize
    }

    /// Returns the magnitude at the given frequency (nearest bin).
    #[must_use]
    pub fn magnitude_at(&self, freq: f32) -> f32 {
        let bin = self.frequency_bin(freq);
        if bin < self.magnitudes.len() {
            self.magnitudes[bin]
        } else {
            0.0
        }
    }

    /// Finds the peak frequency in a given range.
    #[must_use]
    pub fn peak_in_range(&self, lo_hz: f32, hi_hz: f32) -> Option<(f32, f32)> {
        let lo_bin = self.frequency_bin(lo_hz);
        let hi_bin = self.frequency_bin(hi_hz).min(self.magnitudes.len());
        if lo_bin >= hi_bin {
            return None;
        }

        let mut max_mag = 0.0f32;
        let mut max_bin = lo_bin;
        for bin in lo_bin..hi_bin {
            if self.magnitudes[bin] > max_mag {
                max_mag = self.magnitudes[bin];
                max_bin = bin;
            }
        }
        Some((self.bin_frequency(max_bin), max_mag))
    }

    /// Returns the total spectral energy (sum of squared magnitudes).
    ///
    /// Uses Neumaier compensated summation for numerical accuracy across
    /// potentially thousands of frequency bins.
    #[must_use]
    pub fn total_energy(&self) -> f64 {
        let squared: Vec<f64> = self
            .magnitudes
            .iter()
            .map(|&m| (m as f64) * (m as f64))
            .collect();
        hisab::num::neumaier_sum(&squared)
    }

    /// Returns the energy in a frequency band (sum of squared magnitudes in range).
    #[must_use]
    pub fn band_energy(&self, lo_hz: f32, hi_hz: f32) -> f64 {
        let lo_bin = self.frequency_bin(lo_hz);
        let hi_bin = self.frequency_bin(hi_hz).min(self.magnitudes.len());
        if lo_bin >= hi_bin {
            return 0.0;
        }
        let squared: Vec<f64> = self.magnitudes[lo_bin..hi_bin]
            .iter()
            .map(|&m| (m as f64) * (m as f64))
            .collect();
        hisab::num::neumaier_sum(&squared)
    }

    /// Estimates formant frequencies by finding peaks in the spectral envelope.
    ///
    /// Returns up to `max_formants` peak frequencies sorted ascending.
    #[must_use]
    pub fn estimate_formants(&self, max_formants: usize) -> Vec<f32> {
        let mut peaks: Vec<(f32, f32)> = Vec::new();
        let len = self.magnitudes.len();
        if len < 3 {
            return Vec::new();
        }

        // Find local maxima above a threshold
        let max_mag = self.magnitudes.iter().copied().fold(0.0f32, f32::max);
        let threshold = max_mag * 0.1;

        for i in 1..len - 1 {
            if self.magnitudes[i] > self.magnitudes[i - 1]
                && self.magnitudes[i] > self.magnitudes[i + 1]
                && self.magnitudes[i] > threshold
            {
                peaks.push((self.bin_frequency(i), self.magnitudes[i]));
            }
        }

        // Sort by magnitude descending, take top N, then sort by frequency
        peaks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        peaks.truncate(max_formants);
        peaks.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(core::cmp::Ordering::Equal));

        peaks.into_iter().map(|(f, _)| f).collect()
    }
}

/// Computes the magnitude spectrum of an audio frame using FFT.
///
/// The input is zero-padded to the next power of 2 if needed.
/// Returns a [`Spectrum`] with magnitude data for the positive frequencies.
///
/// # Errors
///
/// Returns `SvaraError::ComputationError` if the input is empty.
pub fn analyze(samples: &[f32], sample_rate: f32) -> Result<Spectrum> {
    if samples.is_empty() {
        return Err(SvaraError::ComputationError(
            "cannot analyze empty signal".into(),
        ));
    }

    // Pad to next power of 2
    let fft_size = samples.len().next_power_of_two();
    let mut input: Vec<Complex> = samples
        .iter()
        .map(|&s| Complex {
            re: s as f64,
            im: 0.0,
        })
        .collect();
    input.resize(fft_size, Complex { re: 0.0, im: 0.0 });

    // Apply Hann window to reduce spectral leakage
    let n = samples.len() as f64;
    for (i, c) in input.iter_mut().enumerate().take(samples.len()) {
        let window = 0.5 * (1.0 - crate::math::f64::cos(core::f64::consts::TAU * i as f64 / n));
        c.re *= window;
    }

    hisab::num::fft(&mut input)
        .map_err(|e| SvaraError::ComputationError(alloc::format!("FFT failed: {e}")))?;

    // Extract magnitude of positive frequencies
    let num_bins = fft_size / 2 + 1;
    let magnitudes: Vec<f32> = input
        .iter()
        .take(num_bins)
        .map(|c| crate::math::f32::sqrt((c.re * c.re + c.im * c.im) as f32))
        .collect();

    let freq_resolution = sample_rate / fft_size as f32;

    Ok(Spectrum {
        magnitudes,
        freq_resolution,
        sample_rate,
    })
}

/// Computes the RMS (root mean square) level of an audio buffer.
///
/// Uses Neumaier compensated summation for accuracy over long buffers
/// (minutes of audio at 44.1kHz = millions of samples).
#[must_use]
pub fn rms_level(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let squared: Vec<f64> = samples.iter().map(|&s| (s as f64) * (s as f64)).collect();
    let mean = hisab::num::neumaier_sum(&squared) / samples.len() as f64;
    crate::math::f32::sqrt(mean as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_sine() {
        // Generate a 440Hz sine wave at 44100Hz for 1024 samples
        let sample_rate = 44100.0;
        let freq = 440.0;
        let samples: Vec<f32> = (0..1024)
            .map(|i| {
                let t = i as f32 / sample_rate;
                crate::math::f32::sin(core::f32::consts::TAU * freq * t)
            })
            .collect();

        let spectrum = analyze(&samples, sample_rate).unwrap();
        let (peak_freq, peak_mag) = spectrum.peak_in_range(200.0, 800.0).unwrap();

        // Peak should be near 440Hz (within one bin width)
        assert!(
            (peak_freq - 440.0).abs() < spectrum.freq_resolution * 2.0,
            "peak at {peak_freq}Hz, expected ~440Hz"
        );
        assert!(peak_mag > 0.0);
    }

    #[test]
    fn test_estimate_formants() {
        // Two sine components at 500Hz and 1500Hz
        let sample_rate = 44100.0;
        let samples: Vec<f32> = (0..2048)
            .map(|i| {
                let t = i as f32 / sample_rate;
                crate::math::f32::sin(core::f32::consts::TAU * 500.0 * t)
                    + 0.5 * crate::math::f32::sin(core::f32::consts::TAU * 1500.0 * t)
            })
            .collect();

        let spectrum = analyze(&samples, sample_rate).unwrap();
        let formants = spectrum.estimate_formants(3);

        assert!(
            formants.len() >= 2,
            "should find at least 2 peaks, found {}",
            formants.len()
        );
        // First formant near 500Hz
        assert!(
            (formants[0] - 500.0).abs() < 50.0,
            "F1 at {}Hz, expected ~500Hz",
            formants[0]
        );
    }

    #[test]
    fn test_analyze_empty() {
        assert!(analyze(&[], 44100.0).is_err());
    }

    #[test]
    fn test_spectrum_bin_frequency() {
        let spectrum = Spectrum {
            magnitudes: alloc::vec![0.0; 513],
            freq_resolution: 43.066,
            sample_rate: 44100.0,
        };
        assert!((spectrum.bin_frequency(10) - 430.66).abs() < 0.1);
    }

    #[test]
    fn test_total_energy() {
        let sample_rate = 44100.0;
        let samples: Vec<f32> = (0..1024)
            .map(|i| {
                let t = i as f32 / sample_rate;
                crate::math::f32::sin(core::f32::consts::TAU * 440.0 * t)
            })
            .collect();
        let spectrum = analyze(&samples, sample_rate).unwrap();
        let energy = spectrum.total_energy();
        assert!(energy > 0.0, "spectrum should have energy");
    }

    #[test]
    fn test_band_energy() {
        let sample_rate = 44100.0;
        let samples: Vec<f32> = (0..1024)
            .map(|i| {
                let t = i as f32 / sample_rate;
                crate::math::f32::sin(core::f32::consts::TAU * 440.0 * t)
            })
            .collect();
        let spectrum = analyze(&samples, sample_rate).unwrap();
        let near_440 = spectrum.band_energy(400.0, 500.0);
        let far_away = spectrum.band_energy(2000.0, 3000.0);
        assert!(
            near_440 > far_away,
            "energy near 440Hz ({near_440}) should exceed 2-3kHz ({far_away})"
        );
    }

    #[test]
    fn test_rms_level() {
        // Unit sine has RMS = 1/sqrt(2) ≈ 0.707
        let samples: Vec<f32> = (0..44100)
            .map(|i| {
                let t = i as f32 / 44100.0;
                crate::math::f32::sin(core::f32::consts::TAU * 440.0 * t)
            })
            .collect();
        let rms = rms_level(&samples);
        let expected = core::f32::consts::FRAC_1_SQRT_2;
        assert!(
            (rms - expected).abs() < 0.01,
            "sine RMS should be ~{expected}, got {rms}"
        );
    }

    #[test]
    fn test_rms_level_empty() {
        assert!((rms_level(&[]) - 0.0).abs() < f32::EPSILON);
    }
}
