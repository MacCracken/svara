# 0006 — Scope boundaries

**Status**: Accepted
**Date**: 2026-03-27 (Rust v1.0.0); carried into the Cyrius port unchanged

> Re-homed 2026-08-26 from `docs/architecture/` to `docs/adr/` as 0006 (was ADR-004, `docs/architecture/`).
> These are DECISIONS, so `docs/adr/README.md`'s own scheme puts them here.
> The decision is a domain one and survives the port; only Rust-specific
> wording below was corrected.

## Context

The AGNOS audio ecosystem has multiple components handling different aspects of
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
- Bridge functions for upstream component integration
- LOD quality control for multi-voice scenarios

### Not in svara (handled by sibling components)

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

svara provides dependency-free bridge functions in `src/bridge.cyr` (19 of them) that convert
upstream component outputs into svara parameters. This keeps svara decoupled:

- `bhava` arousal/valence -> Rd, breathiness, vibrato, jitter
- `vansh` speech rate/accent -> duration scale, stress
- `prani` body size/age -> formant scale, f0, jitter
- `goonj` distance/reverb -> gain, bandwidth, spectral tilt
- `badal` noise level/wind -> Lombard effort, f0 shift

## Consequences

- svara is focused and testable in isolation
- Consumers wire components together through bridges, not direct dependencies
- Adding multi-language support is svara's concern (phoneme inventory)
- Adding emotion-to-voice mapping is a bridge concern, not a model concern
