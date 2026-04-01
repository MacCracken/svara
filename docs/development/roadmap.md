# svara Roadmap

## Completed

### v0.1.0 (2026-03-26)

Initial scaffold — core pipeline, Rosenberg B, 44 phonemes, prosody, voice profiles, coarticulation, CI/CD.

### v1.0.0 (2026-03-27)

LF glottal model, Hillenbrand formant data, SOA formant bank (2x), look-ahead coarticulation, 48 phonemes, `no_std`, architecture docs.

### v1.1.0 (2026-03-28)

Bridge module, LOD/Quality system (-34%/-38%), naad-backend wiring, parameter smoothing, validation hardening, ADRs, examples, docs.

### v1.2.0 → v2.0.0 (2026-04-01)

- Whisper mode, creaky voice / vocal fry, vocal effort continuum (5 levels)
- Formant bandwidth widening for singing
- Anticipatory nasalization (vowels before nasals)
- Consonant cluster handling with duration compression
- SynthesisContext (reusable state for block synthesis)
- SIMD: auto-vectorization already optimal (SSE2 4.8µs, AVX2 3.8µs with `-C target-cpu=native`); manual intrinsics slower due to `#[target_feature]` call boundary
- 162 tests, 15 benchmarks

## v2.0.0 — Remaining

### Coarticulation

- [x] Formant trajectory planning across 3+ phoneme windows — done 2026-04-01

### Multi-Language

- [ ] IPA-complete phoneme inventory (>100 phonemes)
- [ ] Tone language support (Mandarin tones as prosody patterns)
- [x] Click consonants (ʘ ǀ ǃ ǂ ǁ), ejectives (pʼ tʼ kʼ sʼ tʃʼ), implosives (ɓ ɗ ɠ) — done 2026-04-01

### Infrastructure

- [x] Object pooling (`SynthesisPool`) — done 2026-04-01
- [x] Batch rendering API (`BatchRenderer`) with progress callbacks — done 2026-04-01

## v3.0.0 — Future

### Infrastructure

- [ ] Voice pool for multi-speaker management

### Singing Voice

- [ ] Vibrato with jitter/shimmer modulation
- [ ] Register transitions (chest → head voice)
- [ ] Formant tuning for singing (singer's formant ~3kHz boost)

### Advanced Modeling

- [ ] KLGLOTT88 glottal model
- [ ] 2D vocal tract area function (replace parallel biquads with waveguide)
- [ ] Subglottal system with tracheal resonances
- [ ] Turbulence noise model at constrictions
