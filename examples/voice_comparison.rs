//! Voice comparison: male, female, and child synthesizing the same vowel.
//!
//! Demonstrates VoiceProfile presets, formant scaling, and how speaker
//! characteristics affect synthesis output.

use svara::prelude::*;

fn main() {
    let profiles = [
        ("Male", VoiceProfile::new_male()),
        ("Female", VoiceProfile::new_female()),
        ("Child", VoiceProfile::new_child()),
    ];

    let sample_rate = 44100.0;
    let duration = 0.3;

    for (name, voice) in &profiles {
        let samples = synthesize_phoneme(&Phoneme::VowelA, voice, sample_rate, duration)
            .expect("synthesis should succeed");

        let rms = rms_level(&samples);
        let max_amp: f32 = samples.iter().map(|s| s.abs()).fold(0.0, f32::max);

        println!(
            "{name:>6}: f0={:>5.1}Hz  formant_scale={:.2}  samples={}  rms={:.4}  peak={:.4}",
            voice.base_f0,
            voice.formant_scale,
            samples.len(),
            rms,
            max_amp,
        );
    }

    // Custom breathy voice
    let breathy = VoiceProfile::new_male()
        .with_breathiness(0.4)
        .with_vibrato_rate(5.5)
        .with_vibrato_depth(0.04);

    let samples = synthesize_phoneme(&Phoneme::VowelA, &breathy, sample_rate, duration)
        .expect("synthesis should succeed");
    let rms = rms_level(&samples);
    println!(
        "Breathy: f0={:>5.1}Hz  breathiness={:.1}  rms={:.4}",
        breathy.base_f0, breathy.breathiness, rms
    );
}
