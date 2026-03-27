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

fn bench_phoneme_render_vowel_a(c: &mut Criterion) {
    c.bench_function("phoneme_render_vowel_a", |b| {
        let voice = VoiceProfile::new_male();
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

criterion_group!(
    benches,
    bench_glottal_source_1024,
    bench_formant_filter_1024,
    bench_vocal_tract_1024,
    bench_phoneme_render_vowel_a,
    bench_sequence_render_5_phonemes,
);

criterion_main!(benches);
