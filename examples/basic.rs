//! Basic svara usage: synthesize a male /a/ vowel for 0.5 seconds.

use svara::prelude::*;

fn main() {
    // Create a male voice profile
    let voice = VoiceProfile::new_male();

    // Synthesize /a/ vowel for 0.5 seconds at 44100 Hz
    let sample_rate = 44100.0;
    let duration = 0.5;

    let samples = synthesize_phoneme(&Phoneme::VowelA, &voice, sample_rate, duration)
        .expect("synthesis should succeed");

    // Print first 10 samples
    println!("=== svara: Male /a/ vowel synthesis ===");
    println!("Sample rate: {sample_rate} Hz");
    println!("Duration: {duration} s");
    println!("Total samples: {}", samples.len());
    println!();

    println!("First 10 samples:");
    for (i, sample) in samples.iter().take(10).enumerate() {
        println!("  [{i:4}] {sample:+.6}");
    }
    println!();

    // Basic statistics
    let min = samples.iter().copied().fold(f32::INFINITY, f32::min);
    let max = samples.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let mean = samples.iter().sum::<f32>() / samples.len() as f32;
    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();

    println!("Statistics:");
    println!("  Min: {min:+.6}");
    println!("  Max: {max:+.6}");
    println!("  Mean: {mean:+.6}");
    println!("  RMS: {rms:.6}");
    println!();

    // Also render a short phoneme sequence
    let mut seq = PhonemeSequence::new();
    seq.push(PhonemeEvent::new(Phoneme::VowelA, 0.15, Stress::Primary));
    seq.push(PhonemeEvent::new(Phoneme::NasalN, 0.08, Stress::Unstressed));
    seq.push(PhonemeEvent::new(Phoneme::VowelI, 0.15, Stress::Secondary));

    let seq_samples = seq
        .render(&voice, sample_rate)
        .expect("sequence render should succeed");

    println!("Sequence 'a-n-i':");
    println!("  Total samples: {}", seq_samples.len());
    let seq_rms =
        (seq_samples.iter().map(|s| s * s).sum::<f32>() / seq_samples.len() as f32).sqrt();
    println!("  RMS: {seq_rms:.6}");
}
