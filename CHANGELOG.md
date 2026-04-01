# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.0.0] - 2026-04-01

### Added

- **Whisper mode** (`GlottalModel::Whisper`): noise-only excitation with no periodic voicing and steep spectral tilt (~12 dB/octave), modeling turbulent airflow through a partially open glottis. `GlottalSource::set_whisper()` convenience method
- **Creaky voice / vocal fry** (`GlottalModel::Creaky`): LF pulse model with irregular period timing — ~40% doubled periods, ~10% tripled periods for subharmonic patterns. 3x shimmer amplification for characteristic amplitude irregularity. `GlottalSource::set_creaky(rd)` with Rd clamped to [0.3, 0.8] (pressed range)
- **Formant bandwidth widening for singing** (`VoiceProfile::bandwidth_widening`): configurable extra bandwidth scaling at high f0 (>300 Hz). Models increased source-tract coupling in singing. Formula: `bw_scale *= 1 + widening * 0.3 * ((f0 - 300) / 500)`. Builder: `with_bandwidth_widening(factor)`, range [0.0, 2.0]
- **Vocal effort continuum** (`VocalEffort` enum): coordinated voice quality control across 5 effort levels (Whisper, Soft, Normal, Loud, Shout). Each level maps to consistent GlottalModel, Rd, breathiness, spectral tilt, f0 range scaling, bandwidth scaling, and jitter/shimmer scaling. `VoiceProfile::with_effort()` builder, `create_glottal_source_with_effort()`, `apply_formant_scale_with_effort()`
- **Anticipatory nasalization** (`Nasalization` struct): vowels preceding nasals (/m/, /n/, /ŋ/) automatically receive gradual nasal coupling ramping up from 65% of the vowel segment. Anti-formant frequency tuned by place of articulation. `synthesize_phoneme_nasalized()` public API
- **Consonant cluster handling**: automatic detection of 2+ adjacent consonants in sequences with 30% duration compression per cluster member. Prevents unnaturally long consonant runs in /str/, /spl/, etc.
- **`SynthesisContext`**: reusable synthesis state (VocalTract + GlottalSource + buffer) for consumers who need to manage allocation. Supports all phoneme classes with nasalization
- **`SynthesisPool`** (`pool.rs`): pre-allocated object pool wrapping `SynthesisContext` with `render`/`render_nasalized`/`render_batch`, pre-warmed buffer via `with_capacity`, diagnostic counters (render_count, peak_samples)
- **`BatchRenderer`** (`render.rs`): non-real-time batch rendering API with `push`/`extend`/`render_all`/`render_with_progress` callback. Concatenates phoneme audio with stress modification and anticipatory nasalization
- **Non-pulmonic consonants**: 13 new phonemes — 5 clicks (ʘ ǀ ǃ ǂ ǁ), 5 ejectives (pʼ tʼ kʼ sʼ tʃʼ), 3 implosives (ɓ ɗ ɠ). New `PhonemeClass` variants: `Click`, `Ejective`, `Implosive`. Click synthesis uses sharp transient bursts shaped by place; ejectives use compressed burst with no aspiration; implosives use creaky-voiced LF pulse with reduced amplitude. Phoneme inventory: 48 → 61
- **Formant trajectory planning** (`trajectory.rs`): `TrajectoryPlanner` computes continuous formant trajectories across 3+ phoneme windows using Catmull-Rom spline interpolation weighted by coarticulation resistance. `PhonemeSequence::render_planned()` synthesizes continuously with per-sample formant updates instead of segment crossfading
- **IPA-complete phoneme inventory** (100 phonemes, up from 48): 7 additional vowels (y, ø, œ, ɯ, ɤ, ɨ, ʉ), 4 plosives (q, ɢ, ʈ, ɖ), 13 fricatives (ɸ, β, ç, ʝ, χ, ʁ, ħ, ʕ, ʂ, ʐ, ɬ, ɮ, ɦ), 3 nasals (ɳ, ɲ, ɴ), 3 trills (ʙ, r, ʀ), 3 approximants/laterals (ɻ, ʎ, ʟ), 2 flaps (ɽ, ɺ), 6 affricates (ts, dz, ʈʂ, ɖʐ, pf, tɬ). New `PhonemeClass::Trill` variant
- **Tone language support** (`Tone` enum): 9 lexical tone patterns — High, Rising, Dipping, Falling, Neutral (Mandarin 5 tones) plus Low, Mid, LowRising, HighFalling for Thai/Vietnamese/African languages. `Tone::to_contour()` produces `ProsodyContour` with f0/duration/amplitude scaling. `PhonemeEvent::with_tone()` constructor
- 2 new benchmarks: `glottal_whisper_1024`, `glottal_creaky_1024`
- 212 total tests (164 unit + 45 integration + 3 doc)

### Performance

- Glottal whisper (1024 samples): 5.0µs (-9% vs Rosenberg, no pulse computation)
- Glottal creaky (1024 samples): 6.9µs (+25% vs Rosenberg, LF pulse + period doubling logic)
- SIMD investigation: manual AVX2+FMA intrinsics benchmarked but `#[target_feature]` call boundary prevents inlining — auto-vectorized loop is faster for runtime-detected paths. Build with `RUSTFLAGS="-C target-cpu=native"` for AVX2 auto-vec: formant filter 4.8µs → 3.8µs (-21%)

## [1.1.1] - 2026-04-01

### Changed

- Removed 7 unused license allowances from `deny.toml` (kept MIT, Apache-2.0, GPL-3.0-only, Unicode-3.0)

### Infrastructure

- Initialized `cargo vet` supply-chain auditing (83 crates exempted)
- Dependency updates: hisab 1.2→1.4, zerocopy 0.8.47→0.8.48, wasm-bindgen 0.2.114→0.2.117, web-sys 0.3.91→0.3.94, js-sys 0.3.91→0.3.94

## [1.1.0] - 2026-03-28

### Added

- **Bridge module** (`bridge.rs`): 18 dependency-free conversion functions for ecosystem integration
  - bhava (emotion/affect): `rd_from_arousal`, `breathiness_from_arousal`, `jitter_from_arousal`, `vibrato_depth_from_valence`, `f0_range_scale_from_arousal`, `intonation_from_emotion`
  - vansh (TTS): `duration_scale_from_speech_rate`, `stress_from_tobi_accent`, `f0_peak_from_prominence`
  - prani (creature): `formant_scale_from_body_size`, `f0_from_body_size`, `jitter_from_age`, `glottal_model_from_effort`
  - goonj (acoustics): `gain_from_distance`, `bandwidth_scale_from_reverb`, `spectral_tilt_from_distance`
  - badal (weather): `lombard_effort_from_noise`, `lombard_f0_shift`, `breathiness_reduction_from_wind`
- **LOD/Quality system** (`lod.rs`): `Quality` enum (Full, Reduced, Minimal) integrated into `VocalTract` for multi-voice CPU scaling
- **Shared RNG module** (`rng.rs`): Deduplicated PCG32 from glottal and phoneme modules
- **Shared DSP utilities** (`dsp.rs`): `validate_sample_rate`, `validate_duration`, `map_naad_error`
- 18 new tests: validation edge cases (NaN, Inf, negative, zero), deterministic replay (single + sequence), streaming API, LOD quality levels — total 45 integration tests (up from 27)
- 13 bridge function tests with range and edge case coverage
- LOD unit tests with serde roundtrips
- no_std testing in Makefile and CI matrix (previously only `--all-features`)
- 4 new examples: `voice_comparison`, `prosody_patterns`, `error_handling`, `streaming`
- 4 Architecture Decision Records: source-filter model, coarticulation model, formant data source, scope boundaries
- Documentation: integration guide, testing guide, dependency watch, threat model
- Send+Sync assertion for `Quality` type

### Changed

- Input validation strengthened: NaN and Infinity now rejected on all public constructors (`GlottalSource::new`, `FormantFilter::new`, `synthesize_phoneme`)
- `VocalTract::process_sample` respects `Quality` setting — skips subglottal, interaction, lip radiation, nasal coupling at lower quality levels
- `GlottalSource`: aspiration noise now uses `naad::noise::NoiseGenerator` (White) when naad-backend enabled; PCG32 fallback otherwise
- `GlottalSource`: vibrato now uses `naad::modulation::Lfo` (Sine) when naad-backend enabled; manual sine fallback otherwise
- `VocalTract`: nasal antiformant now uses `naad::filter::BiquadFilter` (Notch) when naad-backend enabled
- `VocalTract`: subglottal resonance now uses `naad::filter::BiquadFilter` (BandPass) when naad-backend enabled
- `VocalTract`: nasal coupling and gain use `SmoothedParam` one-pole smoother to prevent clicks on real-time parameter changes (5ms time constant)
- `NasalAntiformant` manual implementation gated to `#[cfg(not(feature = "naad-backend"))]`
- CI test matrix now includes `--no-default-features` (ubuntu) alongside `--all-features` (ubuntu + macos)
- SECURITY.md: supported version updated from 0.1.x to 1.x

### Performance

- Vocal tract (Full): 27µs → 23µs (-15%, naad filters more efficient than manual)
- LOD Reduced quality: 15µs (-34% vs Full) — skips subglottal, interaction
- LOD Minimal quality: 14µs (-38% vs Full) — skips nasal coupling, lip radiation too
- 2 new LOD benchmarks added to suite (13 total)

### Infrastructure

- Roadmap updated with v1.1, v1.2, v2.0+ plans
- Forward-looking backlog: whisper/creaky voice, singing, multi-language, SIMD intrinsics

## [1.0.0] - 2026-03-27

### Fixed

- **Spectral tilt**: Replaced constant multiply (no frequency dependence) with a proper one-pole low-pass filter (`y[n] = (1-α)*x[n] + α*y[n-1]`), giving correct frequency-dependent tilt
- **Rosenberg pulse**: Removed non-standard `sqrt(abs(sin))` shaping that deviated from the Rosenberg B model; now pure `3t²-2t³` polynomial. Glottal source benchmark **36% faster** (6.34µs → 4.07µs / 1024 samples)
- **Formant topology naming**: Corrected docs/comments from "cascade" to "parallel bank" (the actual topology: input goes to all filters, outputs are summed)
- **Vowel aliasing**: `VowelOpenA` (/ɑ/) now has distinct formants from `VowelA` (/a/); `VowelBird` (/ɜ/) now has distinct formants from `Schwa` (/ə/)
- **`set_formants()` error propagation**: `VocalTract::set_formants()` and related methods now return `Result` instead of silently ignoring errors
- **License identifier**: `GPL-3.0` → `GPL-3.0-only` in Cargo.toml and deny.toml
- **Doc link**: Escaped `[0,1]` in prosody doc comment to prevent broken intra-doc link

### Added

- **Hillenbrand formant data**: Replaced Peterson & Barney (1952) with Hillenbrand et al. (1995) male averages for all 10 vowels, including per-vowel bandwidths (B1-B5)
- **Per-vowel bandwidths**: `VowelTarget` now stores B1-B5 alongside F1-F5; `with_bandwidths()` constructor; bandwidths interpolated during transitions
- **DC-blocking filter**: `FormantFilter` now includes a one-pole DC blocker (~20 Hz cutoff) to prevent numerical drift
- **Affricates**: `AffricateCh` (/tʃ/) and `AffricateJ` (/dʒ/) with `Affricate` phoneme class and plosive-burst + fricative-release synthesis
- **Glottal stop**: `GlottalStop` (/ʔ/) phoneme
- **Vibrato**: `GlottalSource` now applies sinusoidal f0 modulation using `vibrato_rate` and `vibrato_depth` from `VoiceProfile` (previously defined but never wired up)
- **`VoiceProfile::create_glottal_source()`**: Helper that configures a `GlottalSource` with all voice profile parameters (f0, breathiness, jitter, shimmer, vibrato)
- **`PartialEq`** on `Formant` and `VowelTarget`
- Named constants for magic numbers: `DEFAULT_RNG_SEED`, `DEFAULT_OPEN_QUOTIENT`, `DEFAULT_JITTER`, `DEFAULT_SHIMMER`, `NASAL_ANTIFORMANT_FREQ`, `NASAL_ANTIFORMANT_BW`, `DEFAULT_LIP_RADIATION`, `DEFAULT_BANDWIDTHS`, `DEFAULT_AMPLITUDES`
- 10 serde roundtrip tests: `Vowel`, `FormantFilter`, `VocalTract`, `PhonemeEvent`, `IntonationPattern`, `Stress`, `PhonemeClass`, `SvaraError`, `PhonemeSequence` (deep verify), `VowelTarget`
- **Aspiration noise gating**: Breathiness noise now temporally gated by glottal open phase — full noise during open quotient, reduced during closure
- **Bandwidth scaling by f0**: `apply_formant_scale()` now widens bandwidths proportionally to `sqrt(f0/120)` for female/child voices
- **Tap/flap** `/ɾ/` phoneme (`TapFlap`) — short voiced alveolar contact
- **Look-ahead coarticulation**: Transition to next phoneme begins at configurable onset (default 60% of segment), with sigmoid interpolation curves
- **Coarticulation resistance**: Per-phoneme resistance coefficients (0.0-1.0) based on Recasens DAC model — controls crossfade length at each boundary
- **F2 locus equations**: `f2_locus_equation()` returns (locus, slope) by place of articulation (bilabial, alveolar, velar) per Sussman et al. (1991)
- **Variable crossfade**: Per-boundary crossfade lengths modulated by adjacent phoneme resistance (low-resistance phonemes get longer blending regions)
- **`VocalTract::synthesize_into()`**: Zero-allocation synthesis into pre-allocated buffer
- **SOA formant filter**: Structure-of-arrays `BiquadBankSoa` with fixed `MAX_FORMANTS=8` loop bound enabling compiler auto-vectorization
- **`FormantFilter::process_block()`**: Block-based formant processing for batched audio
- 6 new benchmarks: fricative, diphthong, female vowel, 10-phoneme sequence, pre-allocated tract, block formant filter
- **LF glottal model**: Liljencrants-Fant model with Rd voice quality parameter (0.3=pressed, 1.0=modal, 2.7=breathy). `set_rd()` auto-switches to LF model
- **Source-filter interaction**: Vocal tract impedance feedback modifies excitation signal (configurable 0.0-0.3 strength)
- **Dynamic nasal resonances**: `NasalPlace` enum varies anti-formant by place of articulation (bilabial 750Hz, alveolar 1450Hz, velar 3000Hz)
- **Subglottal resonance**: Tracheal coupling at ~600Hz that interacts with F1 (configurable 0.0-0.2)
- **Gain normalization**: `VocalTract::set_gain()` for output level consistency
- **`no_std` support**: Core DSP works without `std` via `libm` + `alloc`. Enable with `default-features = false`
- **f64 biquad coefficients**: Coefficient computation in f64 prevents quantization errors with narrow bandwidths at high sample rates
- `docs/architecture/overview.md` — module map, data flow, pipeline diagram
- `scripts/bench-history.sh` for tracking benchmark results over time
- `docs/development/roadmap.md` — all v1.0 criteria met

### Changed

- **`FormantFilter` internals**: Refactored from `Vec<BiquadResonator>` (AOS) to `BiquadBankSoa` (SOA) with fixed-size arrays. Formant filter **2x faster** from auto-vectorization of the fixed-bound inner loop
- **`VowelTarget::to_formants()`** now returns `[Formant; 5]` instead of `Vec<Formant>` (zero allocation)
- **`VocalTract::set_formants()`**, `set_formants_from_target()`, `set_vowel()` now return `Result<()>` instead of silently swallowing errors
- **`PhonemeSequence`** now uses variable-length sigmoid crossfades modulated by coarticulation resistance (was fixed-length cosine)
- Phoneme inventory: 48 phonemes (was 44) — added affricates, glottal stop, tap/flap

### Performance

All benchmarks measured at default SSE2. Building with `RUSTFLAGS="-C target-cpu=native"` enables AVX2 for an additional ~20% on formant processing.

- Formant filter (1024 samples): **11.0µs → 5.4µs** (-51%, SOA auto-vectorization)
- Glottal source (1024 samples): **6.34µs → 4.15µs** (-35%)
- Vocal tract (1024 samples): **18.7µs → 12.4µs** (-34%)
- Phoneme render (vowel /a/): **82.7µs → 56.5µs** (-32%)
- Sequence render (5 phonemes): **350µs → 252µs** (-28%)
- Sequence render (10 phonemes): **430µs → 357µs** (-17%)

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
