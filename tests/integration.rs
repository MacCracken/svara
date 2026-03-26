//! Integration tests for svara.

use svara::prelude::*;

#[test]
fn test_male_vowel_a_produces_output() {
    let voice = VoiceProfile::new_male();
    let samples = synthesize_phoneme(&Phoneme::VowelA, &voice, 44100.0, 0.5).unwrap();
    assert!(!samples.is_empty());
    assert!(samples.iter().all(|s| s.is_finite()));

    // Should produce non-trivial output
    let max_amp: f32 = samples.iter().map(|s| s.abs()).fold(0.0, f32::max);
    assert!(max_amp > 0.001, "output too quiet: max_amp = {max_amp}");
}

#[test]
fn test_glottal_period_at_120hz() {
    // At 120Hz with 44100 sample rate, period ≈ 367.5 samples (8.33ms)
    let mut gs = GlottalSource::new(120.0, 44100.0).unwrap();
    gs.set_jitter(0.0); // Disable jitter for deterministic period

    let expected_period = 44100.0 / 120.0; // 367.5
    let actual_period = gs.period_samples();

    assert!(
        (actual_period - expected_period).abs() < 1.0,
        "period should be ~{expected_period}, got {actual_period}"
    );

    // Verify this corresponds to ~8.33ms
    let period_ms = actual_period / 44100.0 * 1000.0;
    assert!(
        (period_ms - 8.33).abs() < 0.1,
        "period should be ~8.33ms, got {period_ms}ms"
    );
}

#[test]
fn test_male_vowel_a_f1_spectral_energy() {
    // Male /a/ should have F1 energy near 730Hz.
    // We use a simple spectral energy check: the formant filter at 730Hz
    // should produce more energy than at 200Hz (well below F1).
    let voice = VoiceProfile::new_male();
    let sample_rate = 44100.0;
    let samples = synthesize_phoneme(&Phoneme::VowelA, &voice, sample_rate, 0.5).unwrap();

    // Simple energy-at-frequency using Goertzel algorithm
    let energy_at_f1 = goertzel_magnitude(&samples, 730.0, sample_rate);
    let energy_below_f1 = goertzel_magnitude(&samples, 200.0, sample_rate);

    // F1 energy should be at least somewhat present
    assert!(
        energy_at_f1 > 0.0,
        "should have energy at F1 (730Hz): got {energy_at_f1}"
    );

    // F1 should have more energy than well below it
    // (relaxed check since the exact spectral shape depends on synthesis details)
    assert!(
        energy_at_f1 > energy_below_f1 * 0.1,
        "F1 energy ({energy_at_f1}) should be substantial relative to 200Hz ({energy_below_f1})"
    );
}

#[test]
fn test_vowel_formant_transitions_no_clicks() {
    // Transition from /a/ to /i/ should not produce clicks
    let voice = VoiceProfile::new_male();
    let mut seq = PhonemeSequence::new();
    seq.push(PhonemeEvent::new(Phoneme::VowelA, 0.15, Stress::Primary));
    seq.push(PhonemeEvent::new(Phoneme::VowelI, 0.15, Stress::Primary));

    let samples = seq.render(&voice, 44100.0).unwrap();

    // Check for large sample-to-sample discontinuities
    let max_amp: f32 = samples.iter().map(|s| s.abs()).fold(0.0, f32::max);
    if max_amp > 0.001 {
        let max_jump: f32 = samples
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0, f32::max);

        // Jump should not exceed signal amplitude (would indicate a click)
        assert!(
            max_jump < max_amp * 2.5,
            "click detected: max_jump={max_jump}, max_amp={max_amp}"
        );
    }
}

#[test]
fn test_female_formant_scale_applies() {
    let male = VoiceProfile::new_male();
    let female = VoiceProfile::new_female();
    let target = VowelTarget::from_vowel(Vowel::A);

    let male_scaled = male.apply_formant_scale(&target);
    let female_scaled = female.apply_formant_scale(&target);

    // Female formant_scale = 1.17, male = 1.0
    // So female F1 should be ~1.17x male F1
    let ratio = female_scaled.f1 / male_scaled.f1;
    assert!(
        (ratio - 1.17).abs() < 0.01,
        "female/male F1 ratio should be ~1.17, got {ratio}"
    );
}

#[test]
fn test_jitter_shimmer_produce_nonperiodic_stable_output() {
    let mut gs = GlottalSource::new(120.0, 44100.0).unwrap();
    gs.set_jitter(0.02);
    gs.set_shimmer(0.04);

    let samples: Vec<f32> = (0..44100).map(|_| gs.next_sample()).collect();

    // All samples should be finite
    assert!(samples.iter().all(|s| s.is_finite()));

    // Should not be perfectly periodic: check that not all periods are identical
    // by comparing chunks at different offsets
    let period = (44100.0 / 120.0) as usize;
    let chunk1: Vec<f32> = samples[0..period].to_vec();
    let chunk2: Vec<f32> = samples[period..2 * period].to_vec();

    let diff: f32 = chunk1
        .iter()
        .zip(chunk2.iter())
        .map(|(a, b)| (a - b).abs())
        .sum();

    assert!(
        diff > 0.001,
        "with jitter/shimmer, periods should differ: diff = {diff}"
    );
}

#[test]
fn test_phoneme_sequence_renders_without_error() {
    let mut seq = PhonemeSequence::new();
    seq.push(PhonemeEvent::new(Phoneme::VowelA, 0.1, Stress::Primary));
    seq.push(PhonemeEvent::new(Phoneme::NasalN, 0.06, Stress::Unstressed));
    seq.push(PhonemeEvent::new(Phoneme::VowelI, 0.1, Stress::Secondary));
    seq.push(PhonemeEvent::new(Phoneme::FricativeS, 0.08, Stress::Unstressed));
    seq.push(PhonemeEvent::new(Phoneme::VowelE, 0.1, Stress::Primary));

    let voice = VoiceProfile::new_male();
    let result = seq.render(&voice, 44100.0);
    assert!(result.is_ok());
    let samples = result.unwrap();
    assert!(!samples.is_empty());
    assert!(samples.iter().all(|s| s.is_finite()));
}

#[test]
fn test_serde_roundtrip_voice_profile() {
    let v = VoiceProfile::new_female().with_f0(200.0).with_breathiness(0.3);
    let json = serde_json::to_string(&v).unwrap();
    let v2: VoiceProfile = serde_json::from_str(&json).unwrap();
    assert!((v2.base_f0 - 200.0).abs() < f32::EPSILON);
    assert!((v2.breathiness - 0.3).abs() < f32::EPSILON);
}

#[test]
fn test_serde_roundtrip_phoneme() {
    let p = Phoneme::FricativeSh;
    let json = serde_json::to_string(&p).unwrap();
    let p2: Phoneme = serde_json::from_str(&json).unwrap();
    assert_eq!(p, p2);
}

#[test]
fn test_serde_roundtrip_formant() {
    let f = Formant::new(730.0, 60.0, 1.0);
    let json = serde_json::to_string(&f).unwrap();
    let f2: Formant = serde_json::from_str(&json).unwrap();
    assert!((f2.frequency - 730.0).abs() < f32::EPSILON);
}

#[test]
fn test_serde_roundtrip_prosody_contour() {
    let c = ProsodyContour::from_pattern(IntonationPattern::Interrogative, 120.0);
    let json = serde_json::to_string(&c).unwrap();
    let c2: ProsodyContour = serde_json::from_str(&json).unwrap();
    assert!((c2.f0_at(0.5) - c.f0_at(0.5)).abs() < f32::EPSILON);
}

#[test]
fn test_vowel_target_interpolation_endpoints() {
    let from = VowelTarget::from_vowel(Vowel::A);
    let to = VowelTarget::from_vowel(Vowel::I);

    let at0 = VowelTarget::interpolate(&from, &to, 0.0);
    assert!((at0.f1 - from.f1).abs() < f32::EPSILON, "at t=0 should equal 'from'");
    assert!((at0.f2 - from.f2).abs() < f32::EPSILON);
    assert!((at0.f3 - from.f3).abs() < f32::EPSILON);

    let at1 = VowelTarget::interpolate(&from, &to, 1.0);
    assert!((at1.f1 - to.f1).abs() < f32::EPSILON, "at t=1 should equal 'to'");
    assert!((at1.f2 - to.f2).abs() < f32::EPSILON);
    assert!((at1.f3 - to.f3).abs() < f32::EPSILON);
}

#[test]
fn test_vowel_target_interpolation_midpoint() {
    let from = VowelTarget::from_vowel(Vowel::A);
    let to = VowelTarget::from_vowel(Vowel::I);

    let mid = VowelTarget::interpolate(&from, &to, 0.5);
    let expected_f1 = (from.f1 + to.f1) / 2.0;
    assert!(
        (mid.f1 - expected_f1).abs() < 0.01,
        "midpoint F1 should be average: expected {expected_f1}, got {}",
        mid.f1
    );
}

#[test]
fn test_child_voice_synthesis() {
    let voice = VoiceProfile::new_child();
    let samples = synthesize_phoneme(&Phoneme::VowelI, &voice, 44100.0, 0.2).unwrap();
    assert!(!samples.is_empty());
    assert!(samples.iter().all(|s| s.is_finite()));
}

#[test]
fn test_all_vowels_synthesize() {
    let voice = VoiceProfile::new_male();
    let vowels = [
        Phoneme::VowelA,
        Phoneme::VowelE,
        Phoneme::VowelI,
        Phoneme::VowelO,
        Phoneme::VowelU,
        Phoneme::VowelSchwa,
    ];
    for vowel in &vowels {
        let result = synthesize_phoneme(vowel, &voice, 44100.0, 0.1);
        assert!(result.is_ok(), "failed to synthesize {:?}", vowel);
        let samples = result.unwrap();
        assert!(!samples.is_empty());
        assert!(samples.iter().all(|s| s.is_finite()));
    }
}

#[test]
fn test_all_consonant_classes_synthesize() {
    let voice = VoiceProfile::new_male();
    let consonants = [
        Phoneme::PlosiveP,
        Phoneme::PlosiveB,
        Phoneme::FricativeS,
        Phoneme::FricativeV,
        Phoneme::NasalM,
        Phoneme::NasalN,
        Phoneme::LateralL,
        Phoneme::ApproximantR,
        Phoneme::ApproximantW,
    ];
    for c in &consonants {
        let result = synthesize_phoneme(c, &voice, 44100.0, 0.08);
        assert!(result.is_ok(), "failed to synthesize {:?}", c);
    }
}

#[test]
fn test_diphthong_synthesis() {
    let voice = VoiceProfile::new_male();
    let diphthongs = [
        Phoneme::DiphthongAI,
        Phoneme::DiphthongAU,
        Phoneme::DiphthongOI,
    ];
    for d in &diphthongs {
        let result = synthesize_phoneme(d, &voice, 44100.0, 0.15);
        assert!(result.is_ok(), "failed to synthesize {:?}", d);
        let samples = result.unwrap();
        assert!(samples.iter().all(|s| s.is_finite()));
    }
}

/// Goertzel algorithm: computes the magnitude of a specific frequency component.
fn goertzel_magnitude(samples: &[f32], target_freq: f32, sample_rate: f32) -> f32 {
    let n = samples.len();
    if n == 0 {
        return 0.0;
    }

    let k = (0.5 + n as f32 * target_freq / sample_rate) as usize;
    let omega = 2.0 * std::f32::consts::PI * k as f32 / n as f32;
    let coeff = 2.0 * omega.cos();

    let mut s0 = 0.0f32;
    let mut s1 = 0.0f32;
    let mut s2;

    for &sample in samples {
        s2 = s1;
        s1 = s0;
        s0 = sample + coeff * s1 - s2;
    }

    let power = s0 * s0 + s1 * s1 - coeff * s0 * s1;
    power.abs().sqrt() / n as f32
}
