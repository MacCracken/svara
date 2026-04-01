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

### v1.0.0 (2026-03-27)

- P(-1) scaffold hardening: spectral tilt, vibrato, Rosenberg pulse fixes
- Hillenbrand et al. (1995) formant data with per-vowel bandwidths
- LF (Liljencrants-Fant) glottal model with Rd parameterization
- SOA formant filter bank (2x speedup), block processing, pre-allocated buffers
- Look-ahead coarticulation with sigmoid interpolation and Recasens DAC resistance
- F2 locus equations, DC blocking, gain normalization, source-filter interaction
- Dynamic nasal resonances, subglottal coupling, f64 biquad coefficients
- 48 phonemes (affricates, glottal stop, tap/flap)
- `no_std` core DSP compatibility
- Architecture documentation with pipeline diagrams

### v1.1.0 (2026-03-28)

- Bridge module (18 conversion functions for bhava, vansh, prani, goonj, badal)
- LOD/Quality system (Full, Reduced -34%, Minimal -38%)
- naad-backend wiring (NoiseGenerator, Lfo, BiquadFilter)
- Parameter smoothing (SmoothedParam on nasal coupling, gain)
- Shared RNG, DSP utilities, 4 ADRs, 4 examples, integration/testing/security docs
- Strengthened input validation (NaN/Inf rejection)

### v1.2.0 (2026-04-01)

- Whisper mode (GlottalModel::Whisper): noise-only excitation, steep spectral tilt
- Creaky voice / vocal fry (GlottalModel::Creaky): irregular period doubling/tripling
- Vocal effort continuum (VocalEffort enum): 5 effort levels with coordinated parameter mapping
- Formant bandwidth widening for singing (VoiceProfile::bandwidth_widening)
- Whisper 5.0µs (-9% vs Rosenberg), Creaky 7.1µs (+29%)
- 15 new benchmarks total, 144 tests

## Backlog — v2.0.0

### Coarticulation

- [ ] Anticipatory nasalization (vowels before nasals)
- [ ] Formant trajectory planning across 3+ phoneme windows
- [ ] Consonant cluster handling (/str/, /spl/)

### Performance

- [ ] SIMD intrinsics for BiquadBankSoa (currently auto-vectorized)
- [ ] Block-based phoneme synthesis (avoid per-phoneme allocation)

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
