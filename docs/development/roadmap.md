# svara Roadmap

## Completed

### v0.1.0 (2026-03-26)

Initial scaffold — core pipeline, Rosenberg B, 44 phonemes, prosody, voice profiles, coarticulation, CI/CD.

### v1.0.0 (2026-03-27)

LF glottal model, Hillenbrand formant data, SOA formant bank (2x), look-ahead coarticulation, 48 phonemes, `no_std`, architecture docs.

### v1.1.0 (2026-03-28)

Bridge module, LOD/Quality system (-34%/-38%), naad-backend wiring, parameter smoothing, validation hardening, ADRs, examples, docs.

### v2.0.0 (2026-04-01)

- Whisper mode, creaky voice, vocal effort continuum (5 levels)
- Formant bandwidth widening for singing
- Anticipatory nasalization, consonant cluster handling, formant trajectory planning (Catmull-Rom 3+ windows)
- SynthesisContext, SynthesisPool, BatchRenderer with progress callbacks
- IPA-complete inventory (100 phonemes), tone language support (9 tones)
- Non-pulmonic consonants (clicks, ejectives, implosives)
- SIMD: auto-vectorization optimal; manual intrinsics slower due to call boundary
- 213 tests, 15 benchmarks

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

## v2.0.1 — Post-Release Cleanup

Known limitations from v2.0.0 audit flagged for immediate follow-up:

- [ ] **Per-vowel spectral tilt not applied in synthesis**: `phoneme_spectral_tilt()` returns correct values but is metadata-only — the one-pole tilt filter over-attenuates signal energy. Replace with a proper shelf filter that preserves overall energy
- [ ] **Speed quotient stored but unused in Rosenberg pulse**: `set_speed_quotient()` sets the field but the pulse shape is unchanged — asymmetric polynomials altered the pulse energy characteristics through the bandpass formant bank. Needs a proper asymmetric glottal model (e.g., modified Rosenberg C or Fant asymmetric pulse)
- [ ] **Tone integration only in `render_planned()`**: `PhonemeEvent.tone` is read by `render_planned()` but ignored by the legacy `render()` path and `BatchRenderer`. Wire tone contours into all render paths

## Post-3.0 — Research Findings

### Synthesis Quality (P(-1) domain research, 2026-04-01)

- [x] **Aspiration VOT modeling** (`VoiceOnsetTime`): per-phoneme voice onset time with place-dependent closure/burst/aspiration fractions (Lisker & Abramson 1964) — done 2026-04-01
- [x] **Speaking rate formant transitions** (`PhonemeSequence::set_speaking_rate`): Lindblom undershoot via `TrajectoryPlanner::apply_speaking_rate` — faster speech reduces coarticulation resistance — done 2026-04-01
- [x] **Per-vowel spectral tilt** (`phoneme_spectral_tilt`): returns height-dependent tilt (0-2 dB/oct based on F1), available as metadata for post-processing — done 2026-04-01
- [x] **Formant amplitude coupling** (`height_adjusted_amplitudes`): F3-F5 attenuate with vowel openness (high F1), modeling source-tract coupling (Fant 1960) — done 2026-04-01
- [x] **Glottal pulse asymmetry** (`GlottalSource::set_speed_quotient`): speed quotient field (0.5-5.0) for future asymmetric pulse models — done 2026-04-01
- [x] **Diplophonia** (`GlottalSource::set_diplophonia`): alternating strong/weak pulse amplitude (0.0-1.0), distinct from creaky voice period doubling — done 2026-04-01

### Coarticulation & Prosody

- [ ] **Prosodic phrase boundaries**: pitch reset, pre-boundary lengthening, and final lowering at phrase/utterance boundaries. Current sequences are flat (no phrase structure)
- [ ] **Durational model (Klatt 1979)**: segment durations depend on position in word/phrase, phonological context, speaking rate. Current model uses fixed class-based defaults
- [ ] **Tone sandhi**: Mandarin tone 3 + tone 3 → tone 2 + tone 3. Current tones don't interact with neighbors
- [ ] **Stress-to-vowel reduction**: unstressed vowels should centralize toward schwa, not just shorten. Current stress only scales f0/duration/amplitude
- [ ] **Phonological processes**: assimilation (voicing, place), elision, lenition. Currently phonemes are synthesized as specified with no phonological rules

### Multi-Language & Phonetics

- [ ] **Vowel nasalization as phonemic contrast**: French, Portuguese have phonemically nasal vowels (not just anticipatory). Need distinct nasal vowel targets, not just coupling ramp
- [ ] **Geminate consonants**: Italian, Japanese, Arabic have phonemic length contrast for consonants. Current duration model has no gemination support
- [ ] **Prenasalized stops**: common in Bantu languages (/mb/, /nd/, /ŋɡ/). Hybrid nasal+plosive synthesis
- [ ] **Labialization, palatalization, pharyngealization**: secondary articulations that modify formants. Common in Caucasian, Semitic, Slavic languages
- [ ] **Breathy/murmured vowels**: Hindi, Gujarati have breathy vowels as phonemic contrast. Need per-phoneme breathiness, not just per-voice

### Performance & Architecture

- [ ] **Ring buffer for real-time streaming**: current `Vec`-based synthesis has latency from allocation. A pre-allocated ring buffer would enable true real-time output
- [ ] **WASM target**: verify no_std + alloc works for WebAssembly. May need wasm-bindgen bindings for browser-based TTS
- [ ] **Parallel multi-voice rendering**: current `Send + Sync` assertions enable this, but no API exists for rendering multiple voices concurrently with shared output mixing
- [ ] **Incremental sequence rendering**: add phonemes to a running sequence and get audio output incrementally, without re-rendering the entire sequence
