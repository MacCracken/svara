//! Error handling: demonstrates matching on SvaraError variants.
//!
//! Shows how consumers should handle errors from synthesis operations.

use svara::prelude::*;

fn main() {
    // Invalid f0
    match GlottalSource::new(5.0, 44100.0) {
        Ok(_) => println!("unexpected success"),
        Err(SvaraError::InvalidPitch(msg)) => {
            println!("Caught invalid pitch: {msg}");
        }
        Err(e) => println!("Unexpected error variant: {e}"),
    }

    // Invalid sample rate
    match GlottalSource::new(120.0, 0.0) {
        Ok(_) => println!("unexpected success"),
        Err(SvaraError::InvalidFormant(msg)) => {
            println!("Caught invalid sample rate: {msg}");
        }
        Err(e) => println!("Unexpected error variant: {e}"),
    }

    // Invalid duration
    let voice = VoiceProfile::new_male();
    match synthesize_phoneme(&Phoneme::VowelA, &voice, 44100.0, -1.0) {
        Ok(_) => println!("unexpected success"),
        Err(SvaraError::InvalidDuration(msg)) => {
            println!("Caught invalid duration: {msg}");
        }
        Err(e) => println!("Unexpected error variant: {e}"),
    }

    // Formant above Nyquist
    match FormantFilter::new(&[Formant::new(25000.0, 80.0, 1.0)], 44100.0) {
        Ok(_) => println!("unexpected success"),
        Err(SvaraError::InvalidFormant(msg)) => {
            println!("Caught Nyquist violation: {msg}");
        }
        Err(e) => println!("Unexpected error variant: {e}"),
    }

    // Successful synthesis for comparison
    match synthesize_phoneme(&Phoneme::VowelA, &voice, 44100.0, 0.1) {
        Ok(samples) => {
            println!(
                "Success: {} samples, rms={:.4}",
                samples.len(),
                rms_level(&samples)
            );
        }
        Err(e) => println!("Unexpected failure: {e}"),
    }
}
