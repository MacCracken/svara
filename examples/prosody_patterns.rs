//! Prosody patterns: declarative, interrogative, exclamatory, continuation.
//!
//! Demonstrates how intonation patterns affect f0 contours across a
//! phoneme sequence.

use svara::prelude::*;

fn main() {
    let voice = VoiceProfile::new_male();
    let sample_rate = 44100.0;

    let patterns = [
        ("Declarative", IntonationPattern::Declarative),
        ("Interrogative", IntonationPattern::Interrogative),
        ("Exclamatory", IntonationPattern::Exclamatory),
        ("Continuation", IntonationPattern::Continuation),
    ];

    for (name, pattern) in &patterns {
        let contour = ProsodyContour::from_pattern(*pattern, voice.base_f0);

        // Show f0 at key points
        let f0_start = contour.f0_at(0.0) * voice.base_f0;
        let f0_mid = contour.f0_at(0.5) * voice.base_f0;
        let f0_end = contour.f0_at(1.0) * voice.base_f0;

        println!(
            "{name:>14}: f0 start={:.0}Hz  mid={:.0}Hz  end={:.0}Hz  dur_scale={:.2}",
            f0_start, f0_mid, f0_end, contour.duration_scale
        );
    }

    // Sequence with stress
    println!("\n--- Stressed sequence ---");
    let mut seq = PhonemeSequence::new();
    seq.push(PhonemeEvent::new(Phoneme::VowelA, 0.15, Stress::Primary));
    seq.push(PhonemeEvent::new(Phoneme::NasalN, 0.06, Stress::Unstressed));
    seq.push(PhonemeEvent::new(Phoneme::VowelI, 0.12, Stress::Secondary));

    let samples = seq
        .render(&voice, sample_rate)
        .expect("render should succeed");
    let rms = rms_level(&samples);
    println!(
        "Sequence: {} samples, {:.1}ms, rms={:.4}",
        samples.len(),
        samples.len() as f32 / sample_rate * 1000.0,
        rms
    );
}
