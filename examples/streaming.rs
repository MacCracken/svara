//! Streaming synthesis: real-time process_sample and synthesize_into usage.
//!
//! Demonstrates the zero-allocation hot path for real-time audio rendering.

use svara::prelude::*;

fn main() {
    let sample_rate = 44100.0;
    let block_size = 512;

    // Set up voice pipeline
    let voice = VoiceProfile::new_male();
    let mut glottal = voice
        .create_glottal_source(sample_rate)
        .expect("glottal source creation should succeed");
    let mut tract = VocalTract::new(sample_rate);
    tract
        .set_vowel(Vowel::A)
        .expect("vowel setup should succeed");

    // Pre-allocate output buffer (zero allocations in the loop)
    let mut buffer = vec![0.0f32; block_size];

    // Simulate 10 blocks of real-time rendering
    println!("Streaming {block_size}-sample blocks at {sample_rate}Hz");
    for block in 0..10 {
        // Zero-allocation synthesis into pre-allocated buffer
        tract.synthesize_into(&mut glottal, &mut buffer);

        let rms: f32 = (buffer.iter().map(|s| s * s).sum::<f32>() / block_size as f32).sqrt();
        let peak: f32 = buffer.iter().map(|s| s.abs()).fold(0.0, f32::max);
        println!("  Block {block:>2}: rms={rms:.4}  peak={peak:.4}");

        // Demonstrate real-time parameter change mid-stream
        if block == 4 {
            println!("  --- Switching to vowel /i/ ---");
            tract
                .set_vowel(Vowel::I)
                .expect("vowel switch should succeed");
        }
    }

    // Quality scaling for background voices
    println!("\n--- Quality levels ---");
    for quality in [Quality::Full, Quality::Reduced, Quality::Minimal] {
        tract.reset();
        tract.set_quality(quality);
        tract
            .set_vowel(Vowel::A)
            .expect("vowel setup should succeed");
        let mut gl = voice
            .create_glottal_source(sample_rate)
            .expect("glottal source should succeed");
        tract.synthesize_into(&mut gl, &mut buffer);
        let rms: f32 = (buffer.iter().map(|s| s * s).sum::<f32>() / block_size as f32).sqrt();
        println!("  {quality:?}: rms={rms:.4}");
    }
}
