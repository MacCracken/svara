# svara Roadmap

## Completed

### v0.1.0 — Initial Scaffold (2026-03-26)

- Core synthesis pipeline: GlottalSource -> VocalTract -> FormantFilter
- Rosenberg B glottal pulse model with jitter, shimmer, breathiness
- Parallel biquad formant filter bank with 5 formants (F1-F5)
- VocalTract with nasal coupling and lip radiation
- 44 phonemes: 15 vowels, 5 diphthongs, 6 plosives, 9 fricatives, 3 nasals, 4 approximants, silence
- Peterson & Barney (1952) vowel formant targets
- Prosody: intonation patterns, stress, f0 contours
- Voice profiles: male/female/child presets with builder pattern
- Coarticulation: crossfade-based phoneme boundary blending
- Criterion benchmarks, integration tests, CI/CD

### P(-1) Scaffold Hardening (2026-03-27)

- Fixed spectral tilt: replaced constant multiply with one-pole low-pass filter
- Wired up vibrato: sinusoidal f0 modulation from VoiceProfile params
- Fixed Rosenberg pulse: removed non-standard sinusoidal shaping, pure 3t^2-2t^3
- Fixed formant topology naming: parallel bank, not cascade
- Added VoiceProfile::create_glottal_source() helper
- Added 10 missing serde roundtrip tests (Vowel, FormantFilter, VocalTract, PhonemeEvent, IntonationPattern, Stress, PhonemeClass, SvaraError, PhonemeSequence deep, VowelTarget)
- Fixed cargo-deny GPL-3.0-only license, doc link escaping, bench-history script
- Glottal source 36% faster (removed sqrt(abs(sin)) computation)

## Backlog — High Priority

### Correctness & Quality

- [ ] Per-vowel bandwidths from Hillenbrand et al. (1995) instead of fixed 60/80/100/120/140
- [ ] Fix vowel aliasing: VowelOpenA != VowelA, VowelBird != Schwa (distinct formant targets)
- [ ] DC-blocking filter at formant bank output (single-pole HPF ~20Hz)
- [ ] Aspiration noise gated by glottal cycle phase, not constant mix
- [ ] Bandwidth scaling by f0 for female/child voices
- [ ] Proper error propagation in tract.rs set_formants() (currently silent fallback)

### Phoneme Inventory

- [ ] Affricates: /tʃ/ (church), /dʒ/ (judge)
- [ ] Glottal stop /ʔ/
- [ ] Tap/flap /ɾ/

### API Quality

- [ ] `#[must_use]` on all pure functions (process_sample, process, etc.)
- [ ] PartialEq/Eq on Formant, VowelTarget
- [ ] `to_formants()` returns `[Formant; 5]` instead of `Vec<Formant>` (avoids allocation)
- [ ] Named constants for magic numbers (PRNG seed, nasal antiformant freq, lip radiation coeff)

### Coarticulation

- [ ] Look-ahead coarticulation: start formant transition in last 30-40% of current segment
- [ ] Asymmetric formant trajectory interpolation (sigmoid/exponential, not linear)
- [ ] Per-phoneme coarticulation resistance coefficients
- [ ] Locus equations for stop consonant F2 transitions

## Backlog — Medium Priority

### Performance

- [ ] Block-based processing (64-512 samples) instead of per-sample
- [ ] SOA layout for parallel formant SIMD (f32x4/f32x8)
- [ ] Pre-allocated output buffers in VocalTract::synthesize()
- [ ] Additional benchmarks: diphthongs, consonant classes, high f0, large sequences, low sample rates

### Glottal Modeling

- [ ] LF (Liljencrants-Fant) glottal model as alternative to Rosenberg
- [ ] Rd parameterization for voice quality (breathy/modal/pressed as single knob)
- [ ] Source-filter interaction coupling term

### Vocal Tract

- [ ] Dynamic nasal resonances (vary anti-formant by place of articulation)
- [ ] Subglottal resonance coupling (~600Hz)
- [ ] f64 filter coefficients for narrow bandwidths / high sample rates

## Future — v1.0 Criteria

- [ ] LF glottal model available
- [ ] Hillenbrand formant data with per-vowel bandwidths
- [ ] Look-ahead coarticulation
- [ ] Block-based real-time capable processing
- [ ] DC-blocking and gain normalization
- [ ] Complete English phoneme inventory (including affricates, glottal stop)
- [ ] `no_std` core DSP compatibility
- [ ] All public types: Serialize + Deserialize + roundtrip tested
- [ ] All benchmarks baselined with history tracking
- [ ] Comprehensive documentation with pipeline diagrams
