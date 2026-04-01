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

## v1.0.0 Released — 2026-03-27

All criteria met. Published to crates.io.

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

## v1.1.0 — In Progress

### Ecosystem Alignment (garjan reference parity)

- [x] Bridge module: dependency-free conversion functions (bhava, vansh, prani, goonj, badal)
- [x] Shared RNG module (deduplicated PCG32 from glottal + phoneme)
- [x] Shared DSP utilities module (validation, naad error mapping)
- [x] LOD/Quality system (Full, Reduced, Minimal) integrated into VocalTract
- [x] no_std CI testing (Makefile + CI matrix)
- [x] Validation edge case tests (NaN, Inf, negative, zero inputs)
- [x] Deterministic replay tests (single phoneme + sequence)
- [x] Streaming API tests (process_block, empty buffer)
- [x] ADRs: source-filter model, coarticulation model, formant data, scope boundaries
- [x] 4 new examples (voice_comparison, prosody_patterns, error_handling, streaming)
- [x] Missing docs (integration guide, testing guide, dependency watch, threat model)
- [x] SECURITY.md updated to v1.x
- [x] Strengthened input validation (NaN/Inf rejection on all public constructors)

### naad-backend Wiring

- [x] Wire naad-backend into glottal.rs (NoiseGenerator for aspiration, Lfo for vibrato) — done 2026-03-28
- [x] Wire naad-backend into tract.rs (BiquadFilter for nasal antiformant + subglottal resonance) — done 2026-03-28
- [x] Parameter smoothing (one-pole SmoothedParam on nasal coupling, gain) — done 2026-03-28
- [x] LOD benchmarks: Full=23µs, Reduced=15µs (-34%), Minimal=14µs (-38%) — done 2026-03-28

## Backlog — v1.2.0 (High Priority)

### Voice Quality

- [x] Whisper mode (noise excitation only, no voicing) — done 2026-04-01
- [x] Creaky voice / vocal fry (irregular glottal pulses, very low Rd) — done 2026-04-01
- [ ] Vocal effort continuum (whisper → normal → shout)
- [x] Formant bandwidth widening under high f0 (singing) — done 2026-04-01

### Coarticulation

- [ ] Anticipatory nasalization (vowels before nasals)
- [ ] Formant trajectory planning across 3+ phoneme windows
- [ ] Consonant cluster handling (/str/, /spl/)

### Performance

- [ ] SIMD intrinsics for BiquadBankSoa (currently auto-vectorized)
- [ ] Block-based phoneme synthesis (avoid per-phoneme allocation)

## Backlog — v2.0+ (Future)

### Multi-Language

- [ ] IPA-complete phoneme inventory (>100 phonemes)
- [ ] Tone language support (Mandarin tones as prosody patterns)
- [ ] Click consonants, ejectives, implosives

### Singing Voice

- [ ] Vibrato with jitter/shimmer modulation
- [ ] Register transitions (chest → head voice)
- [ ] Formant tuning for singing (singer's formant ~3kHz boost)

### Advanced Modeling

- [ ] KLGLOTT88 glottal model
- [ ] 2D vocal tract area function (replace parallel biquads with waveguide)
- [ ] Subglottal system with tracheal resonances
- [ ] Turbulence noise model at constrictions

### Infrastructure

- [ ] Voice pool for multi-speaker management
- [ ] Object pooling for transient phoneme synthesis
- [ ] Async rendering API for non-real-time batch synthesis
