# ADR-004: Scope Boundaries

## Status

Accepted (v1.0.0)

## Context

The AGNOS audio ecosystem has multiple crates handling different aspects of
sound. Clear boundaries prevent scope creep and duplication.

## Decision

### In svara

- Formant-based vocal synthesis (source-filter model)
- Glottal source models (Rosenberg B, LF)
- Vocal tract modeling (formant filter, nasal coupling, lip radiation)
- Phoneme inventory with articulatory synthesis per class
- Prosodic control (intonation, stress, f0 contours)
- Coarticulation and phoneme sequencing
- Voice profiles (speaker parameterization)
- Spectral analysis utilities (FFT, formant estimation)
- Bridge functions for upstream crate integration
- LOD quality control for multi-voice scenarios

### Not in svara (handled by sibling crates)

| Concern | Crate | Rationale |
|---------|-------|-----------|
| Text-to-phoneme conversion | **vansh** | NLP/linguistic processing, not DSP |
| Emotion/affect modeling | **bhava** | Psychological model, not synthesis |
| Creature vocalizations | **prani** | Non-human vocal production models |
| Mechanical sounds | **ghurni** | Different physics (gears, motors) |
| Environmental sounds | **garjan** | Nature/weather/impact synthesis |
| Spatial audio / 3D | **goonj** | Propagation, Doppler, HRTF |
| Mixing / effects / playback | **dhvani** | Audio engine concerns |
| Asset management | **dhvani/kiran** | Runtime concerns |

### Bridge pattern

svara provides dependency-free bridge functions in `bridge.rs` that convert
upstream crate outputs into svara parameters. This keeps svara decoupled:

- `bhava` arousal/valence -> Rd, breathiness, vibrato, jitter
- `vansh` speech rate/accent -> duration scale, stress
- `prani` body size/age -> formant scale, f0, jitter
- `goonj` distance/reverb -> gain, bandwidth, spectral tilt
- `badal` noise level/wind -> Lombard effort, f0 shift

## Consequences

- svara is focused and testable in isolation
- Consumers wire crates together through bridges, not direct dependencies
- Adding multi-language support is svara's concern (phoneme inventory)
- Adding emotion-to-voice mapping is a bridge concern, not a model concern
