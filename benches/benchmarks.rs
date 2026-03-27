//! Criterion benchmarks for svara synthesis pipeline.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use svara::prelude::*;

fn bench_glottal_source_1024(c: &mut Criterion) {
    c.bench_function("glottal_source_1024", |b| {
        let mut gs = GlottalSource::new(120.0, 44100.0).unwrap();
        b.iter(|| {
            for _ in 0..1024 {
                black_box(gs.next_sample());
            }
        });
    });
}

fn bench_formant_filter_1024(c: &mut Criterion) {
    c.bench_function("formant_filter_1024", |b| {
        let formants = VowelTarget::from_vowel(Vowel::A).to_formants();
        let mut filter = FormantFilter::new(&formants, 44100.0).unwrap();
        let mut gs = GlottalSource::new(120.0, 44100.0).unwrap();
        let input: Vec<f32> = (0..1024).map(|_| gs.next_sample()).collect();

        b.iter(|| {
            for &sample in &input {
                black_box(filter.process_sample(sample));
            }
        });
    });
}

fn bench_vocal_tract_1024(c: &mut Criterion) {
    c.bench_function("vocal_tract_1024", |b| {
        let mut tract = VocalTract::new(44100.0);
        tract.set_vowel(Vowel::A).unwrap();
        let mut gs = GlottalSource::new(120.0, 44100.0).unwrap();

        b.iter(|| {
            let output = tract.synthesize(&mut gs, 1024);
            black_box(output);
        });
    });
}

fn bench_vocal_tract_into_1024(c: &mut Criterion) {
    c.bench_function("vocal_tract_into_1024", |b| {
        let mut tract = VocalTract::new(44100.0);
        tract.set_vowel(Vowel::A).unwrap();
        let mut gs = GlottalSource::new(120.0, 44100.0).unwrap();
        let mut buf = vec![0.0f32; 1024];

        b.iter(|| {
            tract.synthesize_into(&mut gs, &mut buf);
            black_box(&buf);
        });
    });
}

fn bench_phoneme_render_vowel_a(c: &mut Criterion) {
    c.bench_function("phoneme_render_vowel_a", |b| {
        let voice = VoiceProfile::new_male();
        b.iter(|| {
            let samples = synthesize_phoneme(&Phoneme::VowelA, &voice, 44100.0, 0.1).unwrap();
            black_box(samples);
        });
    });
}

fn bench_phoneme_render_fricative_s(c: &mut Criterion) {
    c.bench_function("phoneme_render_fricative_s", |b| {
        let voice = VoiceProfile::new_male();
        b.iter(|| {
            let samples = synthesize_phoneme(&Phoneme::FricativeS, &voice, 44100.0, 0.1).unwrap();
            black_box(samples);
        });
    });
}

fn bench_phoneme_render_diphthong_ai(c: &mut Criterion) {
    c.bench_function("phoneme_render_diphthong_ai", |b| {
        let voice = VoiceProfile::new_male();
        b.iter(|| {
            let samples = synthesize_phoneme(&Phoneme::DiphthongAI, &voice, 44100.0, 0.15).unwrap();
            black_box(samples);
        });
    });
}

fn bench_phoneme_render_female_vowel_a(c: &mut Criterion) {
    c.bench_function("phoneme_render_female_vowel_a", |b| {
        let voice = VoiceProfile::new_female();
        b.iter(|| {
            let samples = synthesize_phoneme(&Phoneme::VowelA, &voice, 44100.0, 0.1).unwrap();
            black_box(samples);
        });
    });
}

fn bench_sequence_render_5_phonemes(c: &mut Criterion) {
    c.bench_function("sequence_render_5_phonemes", |b| {
        let voice = VoiceProfile::new_male();
        let mut seq = PhonemeSequence::new();
        seq.push(PhonemeEvent::new(Phoneme::VowelA, 0.08, Stress::Primary));
        seq.push(PhonemeEvent::new(Phoneme::NasalN, 0.05, Stress::Unstressed));
        seq.push(PhonemeEvent::new(Phoneme::VowelI, 0.08, Stress::Secondary));
        seq.push(PhonemeEvent::new(
            Phoneme::FricativeS,
            0.06,
            Stress::Unstressed,
        ));
        seq.push(PhonemeEvent::new(Phoneme::VowelE, 0.08, Stress::Primary));

        b.iter(|| {
            let samples = seq.render(&voice, 44100.0).unwrap();
            black_box(samples);
        });
    });
}

fn bench_sequence_render_10_phonemes(c: &mut Criterion) {
    c.bench_function("sequence_render_10_phonemes", |b| {
        let voice = VoiceProfile::new_male();
        let mut seq = PhonemeSequence::new();
        seq.push(PhonemeEvent::new(Phoneme::VowelA, 0.06, Stress::Primary));
        seq.push(PhonemeEvent::new(Phoneme::NasalN, 0.04, Stress::Unstressed));
        seq.push(PhonemeEvent::new(Phoneme::VowelI, 0.06, Stress::Secondary));
        seq.push(PhonemeEvent::new(
            Phoneme::FricativeS,
            0.04,
            Stress::Unstressed,
        ));
        seq.push(PhonemeEvent::new(Phoneme::VowelE, 0.06, Stress::Primary));
        seq.push(PhonemeEvent::new(
            Phoneme::PlosiveT,
            0.04,
            Stress::Unstressed,
        ));
        seq.push(PhonemeEvent::new(Phoneme::VowelO, 0.06, Stress::Secondary));
        seq.push(PhonemeEvent::new(Phoneme::NasalM, 0.04, Stress::Unstressed));
        seq.push(PhonemeEvent::new(Phoneme::VowelU, 0.06, Stress::Primary));
        seq.push(PhonemeEvent::new(
            Phoneme::LateralL,
            0.04,
            Stress::Unstressed,
        ));

        b.iter(|| {
            let samples = seq.render(&voice, 44100.0).unwrap();
            black_box(samples);
        });
    });
}

criterion_group!(
    benches,
    bench_glottal_source_1024,
    bench_formant_filter_1024,
    bench_vocal_tract_1024,
    bench_vocal_tract_into_1024,
    bench_phoneme_render_vowel_a,
    bench_phoneme_render_fricative_s,
    bench_phoneme_render_diphthong_ai,
    bench_phoneme_render_female_vowel_a,
    bench_sequence_render_5_phonemes,
    bench_sequence_render_10_phonemes,
);

criterion_main!(benches);
