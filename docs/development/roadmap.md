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

- [x] Per-vowel bandwidths from Hillenbrand et al. (1995) — done 2026-03-27
- [x] Fix vowel aliasing: VowelOpenA != VowelA, VowelBird != Schwa — done 2026-03-27
- [x] DC-blocking filter at formant bank output — done 2026-03-27
- [x] Proper error propagation in tract.rs set_formants() — done 2026-03-27
- [x] Aspiration noise gated by glottal cycle phase — done 2026-03-27
- [x] Bandwidth scaling by f0 for female/child voices — done 2026-03-27

### Phoneme Inventory

- [x] Affricates: /tʃ/ (church), /dʒ/ (judge) — done 2026-03-27
- [x] Glottal stop /ʔ/ — done 2026-03-27
- [x] Tap/flap /ɾ/ — done 2026-03-27

### API Quality

- [x] `#[must_use]` on all pure functions — already covered in scaffold
- [x] PartialEq on Formant, VowelTarget — done 2026-03-27
- [x] `to_formants()` returns `[Formant; 5]` — done 2026-03-27
- [x] Named constants for magic numbers — done 2026-03-27

### Coarticulation

- [x] Look-ahead coarticulation with configurable onset — done 2026-03-27
- [x] Sigmoid formant trajectory interpolation — done 2026-03-27
- [x] Per-phoneme coarticulation resistance coefficients (Recasens DAC) — done 2026-03-27
- [x] F2 locus equations for stop consonants (Sussman et al.) — done 2026-03-27

## Backlog — Medium Priority

### Performance

- [x] Block-based processing: `FormantFilter::process_block()` — done 2026-03-27
- [x] SOA layout for formant SIMD: `BiquadBankSoa` with fixed MAX_FORMANTS=8, 2x speedup — done 2026-03-27
- [x] Pre-allocated output buffers: `VocalTract::synthesize_into()` — done 2026-03-27
- [x] Additional benchmarks: fricative, diphthong, female, 10-phoneme, pre-alloc — done 2026-03-27

### Glottal Modeling

- [x] LF (Liljencrants-Fant) glottal model with Rd parameterization — done 2026-03-27
- [x] Source-filter interaction coupling term — done 2026-03-27

### Vocal Tract

- [x] Dynamic nasal resonances by place of articulation — done 2026-03-27
- [x] Subglottal resonance coupling (~600Hz) — done 2026-03-27
- [x] f64 filter coefficients for narrow bandwidths — done 2026-03-27
- [x] Gain normalization — done 2026-03-27

### Infrastructure

- [x] `no_std` core DSP compatibility with `libm` — done 2026-03-27
- [x] Architecture documentation with pipeline diagrams — done 2026-03-27

## v1.0 Criteria — All Met

- [x] LF glottal model available
- [x] Hillenbrand formant data with per-vowel bandwidths
- [x] Look-ahead coarticulation
- [x] Block-based real-time capable processing
- [x] DC-blocking and gain normalization
- [x] Complete English phoneme inventory (48 phonemes)
- [x] `no_std` core DSP compatibility
- [x] All public types: Serialize + Deserialize + roundtrip tested
- [x] All benchmarks baselined with history tracking (11 benchmarks)
- [x] Comprehensive documentation with pipeline diagrams
