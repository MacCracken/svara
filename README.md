# svara

**svara** (Sanskrit: स्वर — voice / tone / musical note) — Formant and vocal synthesis for Rust.

Complete formant-based vocal synthesis pipeline: dual glottal source models (Rosenberg B + LF), SOA-vectorized formant filtering, 48 phonemes, prosodic control, look-ahead coarticulation, and spectral analysis. Built on [hisab](https://crates.io/crates/hisab) for math and [naad](https://crates.io/crates/naad) for DSP primitives.

## Features

- **Dual glottal models**: Rosenberg B polynomial + Liljencrants-Fant (LF) with Rd voice quality parameter
- **SOA formant filter**: Structure-of-arrays biquad bank (MAX_FORMANTS=8) with compiler auto-vectorization — 2x faster than scalar
- **48 phonemes**: 15 vowels, 5 diphthongs, 6 plosives, 9 fricatives, 3 nasals, 4 approximants, 2 affricates, glottal stop, tap/flap, silence
- **Hillenbrand formant data**: Per-vowel frequencies and bandwidths from Hillenbrand et al. (1995)
- **Vocal tract**: Parallel formant bank + nasal coupling (place-dependent) + subglottal resonance + lip radiation + source-filter interaction + DC blocking + gain normalization
- **Prosody**: Monotone cubic f0 contours, 4 intonation patterns, stress, Catmull-Rom interpolation
- **Coarticulation**: Look-ahead onset, sigmoid crossfades, per-phoneme resistance coefficients (Recasens DAC), F2 locus equations
- **Voice profiles**: Male/female/child presets with f0-dependent bandwidth scaling, vibrato, builder pattern
- **Spectral analysis**: FFT-based spectrum, formant estimation, band energy, compensated RMS
- **Performance**: ~1,000x real-time, f64 biquad coefficients, `no_std` compatible, all types `Send + Sync`

## Quick Start

```rust
use svara::prelude::*;

let voice = VoiceProfile::new_male();
let samples = synthesize_phoneme(&Phoneme::VowelA, &voice, 44100.0, 0.5).unwrap();

let mut seq = PhonemeSequence::new();
seq.push(PhonemeEvent::new(Phoneme::VowelA, 0.15, Stress::Primary));
seq.push(PhonemeEvent::new(Phoneme::NasalN, 0.08, Stress::Unstressed));
seq.push(PhonemeEvent::new(Phoneme::VowelI, 0.15, Stress::Secondary));
let audio = seq.render(&voice, 44100.0).unwrap();
```

## Feature Flags

| Flag | Default | Description |
|------|---------|-------------|
| `std` | Yes | Standard library. Disable for `no_std` + `alloc` |
| `naad-backend` | Yes | Use naad for oscillators and filters |
| `logging` | No | Structured logging via tracing-subscriber |

## Architecture

```
GlottalSource (Rosenberg/LF) → VocalTract → Output
                                  │
                    ┌─────────────┼──────────────┐
                    │             │              │
              FormantFilter   Nasal         Lip Radiation
              (SOA biquad     Coupling       + Subglottal
               bank, 8-wide)  (place-dep)    + Interaction
```

## Consumers

- **dhvani** — AGNOS audio engine
- **vansh** — Voice AI shell (TTS/STT)
- **prani** — Creature vocal synthesis (depends on svara)

## License

GPL-3.0-only
