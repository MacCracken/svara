# svara

**svara** (Sanskrit: voice/tone/musical note) — Formant and vocal synthesis crate for the AGNOS project.

Provides a complete formant-based vocal synthesis pipeline: glottal source generation, vocal tract modeling, phoneme-level synthesis, prosodic control, and sequenced speech rendering.

## Features

- **Glottal source**: Rosenberg pulse model with jitter, shimmer, breathiness, spectral tilt
- **Formant filtering**: Cascade biquad resonators with Peterson & Barney (1952) vowel targets
- **Vocal tract**: Formant filters + nasal coupling (anti-formant) + lip radiation
- **Phoneme inventory**: ~44 phonemes covering English and major languages (vowels, plosives, fricatives, nasals, approximants, diphthongs)
- **Prosody**: F0 contours, intonation patterns (declarative, interrogative, continuation, exclamatory), stress
- **Voice profiles**: Male/female/child presets with formant scaling, vibrato, builder pattern
- **Phoneme sequences**: Coarticulated rendering with crossfade transitions
- **Serde support**: All types serialize/deserialize

## Usage

```rust
use svara::prelude::*;

// Synthesize a single vowel
let voice = VoiceProfile::new_male();
let samples = synthesize_phoneme(
    &Phoneme::VowelA,
    &voice,
    44100.0,
    0.5,
).expect("synthesis should succeed");

// Build a phoneme sequence
let mut seq = PhonemeSequence::new();
seq.push(PhonemeEvent::new(Phoneme::VowelA, 0.15, Stress::Primary));
seq.push(PhonemeEvent::new(Phoneme::NasalN, 0.08, Stress::Unstressed));
seq.push(PhonemeEvent::new(Phoneme::VowelI, 0.15, Stress::Secondary));
let audio = seq.render(&voice, 44100.0).expect("render should succeed");
```

## Feature Flags

| Flag | Default | Description |
|------|---------|-------------|
| `naad-backend` | Yes | Use naad crate for oscillators, filters, and noise generation |
| `logging` | No | Enable tracing-subscriber for structured log output |

Without `naad-backend`, svara uses internal minimal implementations (PRNG noise, biquad filters) and compiles standalone.

## Architecture

```
GlottalSource ──> VocalTract ──> Output
                    │
          ┌─────────┼─────────┐
          │         │         │
    FormantFilter  Nasal   Lip Radiation
    (5 biquads)   Coupling  (HPF)
```

Higher-level constructs:

- **Phoneme**: Articulatory targets + synthesis strategy per class
- **Prosody**: F0 contour + stress + intonation
- **VoiceProfile**: Speaker parameterization
- **PhonemeSequence**: Coarticulated multi-phoneme rendering

## Consumers

- **dhvani** — AGNOS voice/sound subsystem
- **vansh** — AGNOS voice AI shell (TTS/STT)

## License

GPL-3.0
