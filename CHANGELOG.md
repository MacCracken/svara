# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- **Spectral tilt**: Replaced constant multiply (no frequency dependence) with a proper one-pole low-pass filter (`y[n] = (1-α)*x[n] + α*y[n-1]`), giving correct frequency-dependent tilt
- **Rosenberg pulse**: Removed non-standard `sqrt(abs(sin))` shaping that deviated from the Rosenberg B model; now pure `3t²-2t³` polynomial. Glottal source benchmark **36% faster** (6.34µs → 4.07µs / 1024 samples)
- **Formant topology naming**: Corrected docs/comments from "cascade" to "parallel bank" (the actual topology: input goes to all filters, outputs are summed)
- **License identifier**: `GPL-3.0` → `GPL-3.0-only` in Cargo.toml and deny.toml
- **Doc link**: Escaped `[0,1]` in prosody doc comment to prevent broken intra-doc link

### Added

- **Vibrato**: `GlottalSource` now applies sinusoidal f0 modulation using `vibrato_rate` and `vibrato_depth` from `VoiceProfile` (previously defined but never wired up)
- **`VoiceProfile::create_glottal_source()`**: Helper that configures a `GlottalSource` with all voice profile parameters (f0, breathiness, jitter, shimmer, vibrato)
- 10 serde roundtrip tests: `Vowel`, `FormantFilter`, `VocalTract`, `PhonemeEvent`, `IntonationPattern`, `Stress`, `PhonemeClass`, `SvaraError`, `PhonemeSequence` (deep verify), `VowelTarget`
- `scripts/bench-history.sh` for tracking benchmark results over time
- `docs/development/roadmap.md` with backlog and v1.0 criteria

### Performance

- Glottal source: **6.34µs → 4.07µs** (-36%) per 1024 samples (sinusoidal shaping removed)
- Phoneme render (vowel /a/): **82.71µs → 73.69µs** (-11%)

## [0.1.0] - 2026-03-26

### Added

- Initial scaffold of the svara crate
- `GlottalSource`: Rosenberg glottal pulse model with f0, open quotient, spectral tilt, jitter, shimmer, breathiness
- `FormantFilter`: Cascade biquad resonator bank with parallel summing
- `VowelTarget`: Peterson & Barney (1952) formant frequencies for 10 vowel categories with F1-F5
- `VowelTarget::interpolate`: Linear interpolation between vowel targets for smooth transitions
- `VocalTract`: Formant filtering + nasal coupling (anti-formant at 250Hz) + lip radiation (first-order HPF)
- `Phoneme` enum: 44 phonemes (15 vowels, 5 diphthongs, 6 plosives, 9 fricatives, 3 nasals, 4 approximants/laterals, silence)
- `PhonemeClass` enum: Plosive, Fricative, Nasal, Approximant, Lateral, Vowel, Diphthong, Silence
- `synthesize_phoneme`: Class-specific synthesis (vowels via glottal+tract, fricatives via filtered noise, plosives via burst+aspiration, nasals via nasal coupling, diphthongs via formant interpolation)
- `ProsodyContour`: Time-value f0 contour with linear interpolation
- `IntonationPattern`: Declarative (falling), Interrogative (rising), Continuation (rise-fall), Exclamatory (high-fall)
- `Stress` enum with f0/duration/amplitude modifications
- `VoiceProfile`: Male (120Hz), female (220Hz, 1.17x formant scale), child (300Hz, 1.3x) presets with builder pattern
- `PhonemeSequence`: Ordered phoneme events with coarticulation crossfading at boundaries (configurable 50ms window)
- `SvaraError`: InvalidFormant, InvalidPhoneme, InvalidPitch, InvalidDuration, ArticulationFailed, ComputationError
- Integration tests: spectral energy, glottal period, click detection, serde roundtrips, interpolation endpoints
- Criterion benchmarks: glottal, formant filter, vocal tract, phoneme render, sequence render
- Feature flags: `naad-backend` (default), `logging`
- CI/CD: GitHub Actions workflows for test, lint, coverage, release
