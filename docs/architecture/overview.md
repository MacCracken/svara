# svara Architecture

## Synthesis Pipeline

```
                     VoiceProfile
                         |
                    +-----------+
                    | Phoneme   |   phoneme_formants()
                    | Inventory |   phoneme_duration()
                    +-----------+   coarticulation_resistance()
                         |          f2_locus_equation()
                         v
+---------------+   +----------+   +------------------+
| GlottalSource |-->| VocalTract|-->| Output (samples) |
| (Rosenberg/LF)|   |          |   +------------------+
+---------------+   +----------+
    |                    |
    | next_sample()      | process_sample()
    |                    |
    v                    v
 Pulse Model         FormantFilter (SOA BiquadBankSoa)
 + Spectral Tilt     + NasalAntiformant (place-dependent)
 + Jitter/Shimmer    + Subglottal Resonance (~600Hz)
 + Vibrato           + Lip Radiation (1st-order HPF)
 + Aspiration Noise  + Source-Filter Interaction
   (glottal-gated)   + DC Blocker
                     + Gain Normalization
```

## Module Map

| Module | Purpose | Key Types |
|--------|---------|-----------|
| `glottal` | Excitation source | `GlottalSource`, `GlottalModel` |
| `formant` | Parallel formant filter bank | `FormantFilter`, `BiquadBankSoa`, `VowelTarget`, `Vowel`, `Formant` |
| `tract` | Vocal tract model | `VocalTract`, `NasalPlace` |
| `phoneme` | Phoneme inventory + synthesis | `Phoneme`, `PhonemeClass`, `synthesize_phoneme()` |
| `prosody` | Pitch/timing/stress | `ProsodyContour`, `IntonationPattern`, `Stress` |
| `sequence` | Phoneme sequencing | `PhonemeSequence`, `PhonemeEvent` |
| `voice` | Speaker characteristics | `VoiceProfile` |
| `error` | Error types | `SvaraError` |
| `math` | `no_std` math compat | (internal) |

## Data Flow

1. **VoiceProfile** defines speaker characteristics (f0, formant scale, breathiness, vibrato)
2. **PhonemeSequence** holds timed phoneme events with stress markers
3. **render()** iterates events:
   - Creates `GlottalSource` from voice profile (Rosenberg or LF model)
   - Creates `VocalTract` with formant targets from `phoneme_formants()`
   - Per-sample: `GlottalSource::next_sample()` -> `VocalTract::process_sample()` -> output
4. **Coarticulation** at boundaries:
   - Variable crossfade length based on `coarticulation_resistance()`
   - Sigmoid interpolation curves (Hermite smoothstep)
   - Look-ahead onset at 60% of each segment

## Performance Architecture

- **SOA Layout**: `BiquadBankSoa` uses structure-of-arrays with fixed `MAX_FORMANTS=8` slots
- **Auto-vectorization**: Fixed loop bounds enable compiler SIMD (SSE2 default, AVX2 with `-C target-cpu=native`)
- **Zero allocation**: Hot path uses fixed-size arrays, `synthesize_into()` for pre-allocated output
- **f64 coefficients**: Biquad coefficients computed in f64, stored as f32 for processing
- **~5,000x real-time**: Formant filter processes 1024 samples in ~5us (44.1kHz)

## Consumers

- **dhvani**: Voice AI shell (orchestration, I/O, voice management)
- **vansh**: Voice shell TTS/STT
- **bhava**: Emotion -> prosody parameter mapping
- Any AGNOS component needing speech synthesis

## no_std Support

Core DSP works without `std` via `#![cfg_attr(not(feature = "std"), no_std)]`:
- Math functions via `libm` (sin, cos, exp, sqrt, sinh)
- Collections via `alloc` (Vec, String, format!)
- Enable with `default-features = false`
