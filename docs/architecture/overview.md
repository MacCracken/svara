# svara Architecture

> Rewritten 2026-08-26 for the Cyrius port. The pre-port version described a Rust
> crate: a `math` module that does not exist, "coefficients stored as f32 for
> processing" (the port is f64 throughout), compiler auto-vectorization, a
> `no_std` section, and "~5,000× real-time" against a measured ~64×.

## Synthesis pipeline

```
                     SvVoiceProfile
                          |
                     +-----------+
                     |  Phoneme  |   svara_phoneme_formants()
                     | inventory |   svara_phoneme_duration()
                     |   (101)   |   svara_phoneme_coarticulation_resistance()
                     +-----------+   svara_ph_f2_locus_equation()
                          |
                          v
+-----------------+  +--------------+  +------------------+
| SvGlottalSource |->| SvVocalTract |->|  samples (vec f64)|
| (Rosenberg / LF)|  |              |  +------------------+
+-----------------+  +--------------+
     |                     |
     | next_sample()       | process_sample()
     v                     v
  Pulse model         SvFormantFilter (bank, 8 slot records)
  + spectral tilt     + nasal antiformant   [naad Notch]
  + jitter / shimmer  + subglottal ~600 Hz  [naad BandPass]
  + vibrato   [naad Lfo]                    + lip radiation (1st-order HPF)
  + aspiration noise  [naad NoiseGenerator] + source-filter interaction
    (glottal-gated)                         + DC blocker
                                            + gain normalization
```

## Module map

17 library modules plus `main.cyr`, the smoke entry. `dsp.rs` folded into
`error.cyr`; `math.rs` maps to the `ganita` stdlib and has no module; `logging`
has no Rust counterpart beyond the `logging` Cargo feature.

| Module | Purpose | Key types |
|---|---|---|
| `error` | Error codes, tolerances, the conversion/allocation guards | `SVARA_ERR_*`, `svara_count_from_f64`, `svara_alloc_samples` |
| `rng` | Seedable PCG32 | `SvRng` |
| `smooth` | One-pole parameter smoothing | `SvSmoothedParam` |
| `lod` | Level-of-detail predicates | `SVARA_QUALITY_*` |
| `formant` | Parallel formant bank | `SvFormantFilter`, `SvFormantBank`, `SvVowelTarget`, `SvFormant`, `SvDcBlocker` |
| `spectral` | FFT analysis (hisab `num_fft`) | `SvSpectrum` |
| `glottal` | Excitation source | `SvGlottalSource`, `SvGlottalState` |
| `tract` | Vocal tract model | `SvVocalTract`, `SvVocalTractState`, `SVARA_NASAL_*` |
| `voice` | Speaker characteristics | `SvVoiceProfile`, `SvEffortParams` |
| `phoneme` | 101-phoneme inventory + synthesis + context | `SVARA_PH_*`, `SvSynthCtx`, `SvNasalization`, `SvVOT` |
| `prosody` | Pitch, timing, stress | `SvProsodyContour`, `SvF0Point`, `SVARA_TONE_*` |
| `trajectory` | Formant trajectory planning | `SvTrajectoryPlanner`, `SvFormantKeypoint` |
| `sequence` | Phoneme sequencing + coarticulation | `SvPhonemeSequence`, `SvPhonemeEvent` |
| `pool` | Pooled synthesis context | `SvSynthesisPool` |
| `render` | Batch rendering | `SvBatchRenderer`, `SvRenderOutput`, `SvRenderProgress` |
| `bridge` | 19 scalar maps from sibling components | (free functions) |
| `logging` | sakshi tracing, `-D LOGGING` only | `svara_log_*`, `svara_span_*` |

## Naming

Every top-level symbol is prefixed — `svara_` for functions, `SVARA_` / `SV_` for
constants, `Sv` for struct types. **This is load-bearing, not style.** Cyrius has
one flat namespace, and `dist/svara.cyr` is compiled alongside naad's and hisab's
bundles in the consumer's build. naad already exports `validate_sample_rate`,
`flush_denormal`, `formant_*` and `voice_*`; an unprefixed svara symbol would
collide silently.

## Data flow

1. **`SvVoiceProfile`** defines the speaker — base f0, formant scale, breathiness,
   vibrato, jitter, shimmer, bandwidth widening.
2. **`SvPhonemeSequence`** holds timed events with stress and optional tone.
3. **`svara_sequence_render`** — per-phoneme synthesis, then a variable-length
   sigmoid crossfade at each boundary whose length comes from the two phonemes'
   coarticulation resistance.
4. **`svara_sequence_render_planned`** — the continuous path. A
   `SvTrajectoryPlanner` lays down formant keypoints, and every output sample
   interpolates a target (Catmull-Rom between interior keypoints, resistance-
   weighted against linear; eased linear at the edges) and re-targets the tract.

## Constraints that shape the code

**The bump allocator never frees.** Memory returns to the OS at process exit,
nowhere else. An allocation in a per-sample loop is not a performance nuisance —
it is unbounded growth. The per-sample path therefore allocates nothing, which
took deliberate work: `svara_tract_set_formants` rebuilds the filter *in place*,
`svara_trajectory_formants_at_into` and `svara_vowel_target_to_formants_into`
write into caller-owned storage, and the tone contour is built once per event
rather than once per sample. A growing vec is the same hazard one level up:
`vec_push` doubles and copies, and the arena keeps every buffer it outgrows. So since
3.8.3 a render never pushes its samples onto a vec. It renders into a buffer with a
vec's capacity, and `svara_vec_adopt` makes that buffer the vec's own.
`tests/allocbudget.tcyr` pins the result as marginal arena bytes per sample. See
[ADR-0001](../adr/0001-signed-index-and-float-conversion-hazards.md) for the
related allocation-failure discipline.

**Cyrius emits scalar code.** There is no auto-vectorization. A bit-identical
AVX2 formant bank was prototyped and reverted — it bought ~5%, because the loop
is memory-bound (the SOA state shuffle dominates), not compute-bound. 3.5.0 then
got **−6%** from the same insight without any SIMD, by collapsing the redundant
per-slot input delay line: all eight biquads share one input, so `x1`/`x2` held
identical values in every slot. naad has since measured `f64v256_*` over a
structure-of-arrays bank at 3.3× on cyrius 6.7.18, so this is worth measuring again.
See [`../development/roadmap.md`](../development/roadmap.md).

Nor is an accessor free. A `#derive(accessors)` getter retires 2 instructions more than
the `load64` it stands for, and a setter 6 more than its `store64`, so the per-sample
path reads and writes fields at `const` offsets declared beside each struct (`SV_GS_*`,
`SV_VT_*`, `SV_FSLOT_*` …): the bank since 3.8.4, the rest of the chain since 3.8.5.
Everything else keeps the accessors, and a test pins each offset to its field.

## Precision and performance

- **f64 throughout.** The Rust was f32 with f64 biquad coefficients; the port is
  f64 everywhere, because hisab and naad are f64-only. `SV_EPSILON` preserves the
  f32 tolerance the Rust tests used.
- **A bank of slot records**, `SVARA_MAX_FORMANTS = 8` fixed slots; unused slots hold
  zeroed coefficients and contribute nothing, so since 3.8.2 the bank skips them while
  that is provably exact (no coefficients there, and finite input). Through 3.8.3 the
  slots were the Rust's structure of arrays, seven parallel buffers. Since 3.8.4 each
  slot is one 56-byte record read at `const` offsets (`SV_FSLOT_*`), not through
  derived accessors, which cost a jump and 2 to 6 instructions each on cyrius 6.7.18.
- **Measured**: glottal ~48 ns/sample, formant ~86 ns, tract ~135 ns; the full
  glottal → tract chain **≈ 0.18 µs/sample**, about **125× real-time** at 44.1 kHz
  on one core (x86_64, cyrius 6.7.18, svara 3.8.2).

  ⚠ **The chain is measured end to end, not summed.** `tract process_sample`
  already contains the formant bank, so adding the three per-unit rows
  double-counts it — which is how this figure was computed until 3.5.3, and it
  understated svara by ~1.6×. A same-machine head-to-head puts the per-DSP-unit
  gap against the Rust at 10–38×; see [`../benchmarks.md`](../benchmarks.md) and
  [`../benchmarks-rust-v-cyrius.md`](../benchmarks-rust-v-cyrius.md).
- **Tolerance parity, not bit-exact.** Transcendentals come from `ganita` and are
  not bit-identical across architectures, so the `.tcyr` suites assert within
  epsilon rather than on exact bits.

## Serialization

Configuration and data serialize; engine objects do not, and reach JSON through
flat state companions instead. The boundary and its reasons are
[ADR-0002](../adr/0002-serialization-boundary.md).

## Backend

svara carries **only** the naad-backend path — the Rust's `naad-backend` feature
was made unconditional and the CFG split collapsed. There are **no feature
flags**; Cyrius has no equivalent, and the fallback biquad implementations were
dropped rather than maintained unbuilt.

## Consumers

- **dhvani** — voice AI shell (orchestration, I/O, voice management)
- **vansh** — voice shell TTS/STT
- **bhava** — emotion → prosody parameter mapping, via `bridge.cyr`
- any AGNOS component needing speech synthesis

They consume `dist/svara.cyr` plus its `.deps` sidecar, resolved from their own
manifest.
