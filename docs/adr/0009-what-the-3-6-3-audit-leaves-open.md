# 0009 — What the 3.6.3 audit leaves open

**Status**: Accepted
**Date**: 2026-11-07

## Context

The 3.6.3 sweep ran svara against the compiled Rust oracle and ran a fuzz target,
fault injection, an all-zero argument sweep and a line-by-line audit of every module.
The method and the counts are in
[`docs/audit/2026-11-07-audit.md`](../audit/2026-11-07-audit.md). 3.6.3 fixed what was
wrong on valid input and what was unsafe on hostile input.

Three kinds of finding remain, and each one needs a recorded reason:

- **Divergences.** svara gives a different answer from `rust-old/` on purpose, or as
  a side effect of an earlier decision. CLAUDE.md allows a divergence only with an
  ADR, and four of these shipped without one.
- **Gaps.** An input class that svara still does not defend, or a defence with no
  suite test.
- **Upstream defects.** A fault that is in a dependency, not in svara.

## Decision

**Keep each item below as it is, for the reason given.** Anything scheduled for later
says where it is scheduled.

### Divergences from `rust-old/` that stay

| ID | Input | svara | Rust | Since | Why it stays |
|---|---|---|---|---|---|
| SP-DIV | any contour through `svara_prosody_f0_at` | hisab's shape-preserving monotone cubic | hisab 1.4.0's cubic, which overshoots | 3.1.2 (hisab 2.8.3) | Rust's own comment says "no overshoot", so svara does what Rust meant. Up to 0.84% on the unstressed presets, and up to 10.3% (about 1.7 semitones) after `apply_stress`. |
| BD-4 | a sum with a `±inf` term (`svara_spectral_rms_level`, `svara_spectrum_total_energy`) | `+inf` | NaN | hisab 3.3.3 | `+inf` is the correct sum. Rust's NaN comes from its compensation term computing `inf - inf`. |
| BD-7 | a NaN or `±inf` base f0 in `svara_phoneme_synthesize` | `SVARA_ERR_INVALID_FORMANT` | `ArticulationFailed` | NaN 3.5.2 ([ADR-0007](0007-reject-nan-formant-parameters.md)), inf 3.6.3 | Both sides refuse. Only the error class differs, because the bad f0 reaches a bandwidth, and svara now validates bandwidths before Rust's later check fails. |
| PH-2 | `/p/` or `/q/` when `0.35 × n` is a whole number | the VOT boundary lands one sample early | one sample later | 3.0.0 | It comes from the f64 core, a consequence of [ADR-0008](0008-the-synthesis-core-stays-f64.md). It is within tolerance parity. |
| — | an infinite formant bandwidth | refused, `SVARA_ERR_INVALID_FORMANT` | accepted | 3.6.3 | ADR-0007 already says "reject non-finite formant frequencies and bandwidths". The code checked only the frequency, and 3.6.3 brings it into line with the ADR. |
| — | contour points with NaN times, more than 20 of them | stable insertion sort | `sort_by`, a merge sort above 20 elements | 3.6.3 | Without NaN the two sorts give the same order, because both are stable. With NaN the comparator is not a total order, and the orders can differ above 20 points. A contour with NaN times is refused by `f0_at` anyway. |

### A divergence with no decision yet

**RN-DIV: the diphthong glide is louder than Rust's.** Since 3.1.0,
`svara_ph_synth_diphthong` and the context path rebuild the formant bank every 64
samples. Rust rebuilds it on every sample through `FormantFilter::new`, which also
resets the filter state (`git show 3.5.2:rust-old/src/tract.rs`, `set_formants`), so
its resonance never builds up. The audit measured svara's free-path glide energy at
**7× to 69× Rust's** on all 60 configurations of phonemes 15–19.

3.1.0's CHANGELOG called the change "perceptually identical" without measuring the
oracle, and that claim was wrong. Rust's version is the odd one: a formant filter
reset on every sample is barely a filter. Matching it would make every diphthong
quieter for every consumer. 3.6.3's bar is that audio on valid input does not move,
so the decision goes to roadmap 3.9.x as the fifth item there, beside the four
preserved Rust quirks.

### Gaps that stay

- **Decoding a document directly is outside the contract.** `restore` is the entry
  that validates ([ADR-0002](0002-serialization-boundary.md), decision 3), and every
  `*_restore` now refuses a null state, a missing vec field and an out-of-range
  enum, including each queued event's phoneme, stress and tone. A type decoded
  directly, such as `SvPhonemeSequence_from_json_str` or
  `SvProsodyContour_from_json_str`, is not validated. The generated decoder is
  lenient, so a missing key decodes to 0, and that faults at first use. Rust's serde
  would refuse the document. This is ADR-0002's "worth a guard if that ever becomes
  a supported input". A caller with untrusted JSON should check the text with
  `svara_validate_json`, then decode a state companion and `restore` it.
- **An explicit `null` reads as an empty list, even through `restore`.** The decoder
  turns a missing vec field into 0, which `restore` refuses. It turns `"events":null`
  into an empty vec, though, and drops a `null` element from `[null]`. After
  decoding, `restore` cannot tell either from `[]`, so a batch, plan or render output
  restored from such a document comes back empty instead of being refused. Rust's
  serde refuses both. The result is a valid empty object, not a fault. Refusing it
  would need a check on the text that knows which keys are vecs.
  `svara_validate_json` does not know that, and it has to take `null` anyway,
  because svara writes `null` for a non-finite number. Found by the independent
  review.
- **A refused allocation ends the process where an answer would look valid.** This
  matches naad's ADR-0006 and goonj's ADR-0004. Every allocation svara sizes from an
  argument goes through `svara_alloc_samples` and returns an error code.
  `svara_alloc_refused()` exits with status 1 in three places that used to return a
  plausible value: spectral scratch growth, contour scratch growth in `f0_at`, and
  hisab's `HSB_ERR_ALLOC` from the monotone cubic. Those functions return an f64
  with no error channel, in Rust too, so any sentinel would be a valid-looking
  value.
- **PH-3: unchecked fixed-size allocations fault at the null page.** These are small
  allocations of constant size: an amplitude table, VOT timings, a nasalization
  record. Under fault injection on the release tree (2,056 refused allocations),
  all 1,840 faults were at offset 0x10 or less, with no silent wrong answer and no
  hang. Under the
  all-zero argument sweep, the 184 faulting functions faulted at 0xb0 or less. A
  load or store at the null page is a fail-stop on Linux. Rust aborts here too.
  Checking all 52 raw `alloc` sites would add a branch to each for no change in
  outcome.
- **A huge finite bandwidth gives NaN, as in Rust.** Only non-finite bandwidths are
  refused, because any finite ceiling would be invented. Rust has none.
- **A glottal source given an infinite vibrato rate cannot round-trip.**
  `svara_glottal_set_vibrato` records the rate, as Rust does, and naad's LFO refuses
  it. `svara_glottal_restore` refuses a non-finite rate, because the LFO it rebuilds
  would run at a different rate from the source that was saved.
- **The nasal-place getter reports only a place naad accepted.** At a sample rate
  too low for the notch, `svara_tract_set_nasal_place` leaves both the filter and
  the recorded place unchanged. Before 3.6.3 it recorded a place the filter did not
  have, and a save and restore changed the notch.
- **There is no sample-rate ceiling.** `svara_validate_sample_rate` asks only for a
  positive, finite rate, as Rust's `validate_sample_rate` does. A sequence rendered
  at 3e9 Hz runs for more than 40 s. Rate is the same kind of risk as duration: the
  work is rate × duration, and `SV_MAX_SAMPLES` already caps that product. Any
  ceiling on the rate alone would be an invented number. SECURITY.md tells a caller
  with untrusted input to bound both.
- **Generated setters are public.** `#derive(accessors)` emits `Type_set_field` for
  every field, with no way to hide it, and these setters skip what the `svara_*`
  setter validates. The `svara_*` functions are the API. The raw accessors are the
  struct layout, and a state built through them still goes through `restore`, which
  validates. Revisit if `#derive` gains a visibility option.
- **RN-6 has no suite test.** Concatenating past `SV_MAX_SAMPLES` now returns
  `SVARA_ERR_COMPUTATION` where it used to abort in `vec_push`. A test would need
  about 2 GB of output, so it was measured with a probe instead: it returns -6 in
  2.6 s.

### Upstream

- **cyrius 6.7.18: the `#derive(Serialize)` decoder loops forever on a `Vec<struct>`
  array holding a stray `}`.** The generated element loop hands any byte other than
  `{` to `_cy_json_skipval`, and that returns without moving at a depth-0 `}`.
  Fuzz case 82 reached it: a contour with one `{` changed to `\x1a`. Every svara
  type with a vec of structs inherits it. That covers the state companions
  `SvVocalTractState`, `SvTrajectoryState` and `SvBatchRendererState`, the pool and
  synth-context states, and the directly decoded `SvPhonemeSequence` and
  `SvProsodyContour`. `SvVocalTractState_from_json_str` never returns on
  `{"formants":[}],"sample_rate":44100.0}`, so `svara_tract_restore` never sees the
  state. `restore` validates what a decode returns, and it cannot defend a decode
  that does not return. Filed in the local cyrius repo as
  `docs/development/issues/2026-11-07-derive-serialize-vec-struct-element-loop-hangs-on-stray-brace.md`,
  with a repro that exits 3 on all four toolchains tried, 6.6.0, 6.6.20, 6.7.9 and
  6.7.18. The same decoder also reads past the NUL on an unterminated key, which
  ghurni filed the same day.

  **svara guards both from 3.6.3 on.** `svara_validate_json` checks the text before
  any decode. It refuses every document svara's writer could not have produced:
  a string value, an escape, a bracket that closes the wrong kind, whitespace
  inside the object, nesting past 16 levels, a number longer than 32 bytes, or
  anything after the object. The decoder tracks strings without escapes and
  assumes a well-formed document, so a document that passes is one the decoder
  reads as JSON does, and neither defect can be reached. That is an argument
  about the decoder's code, so it was also measured. In a scratch probe, 70,000
  byte-damaged documents of the seven types gave 63,821 refusals, and all 6,179
  accepted documents decoded with no hang and no fault. Without the check, 303 of
  7,000 hung. The fuzz target now damages 8 documents per JSON case and decodes
  only what the check accepts. It fails when the check is removed. Keep the check
  after cyrius fixes the decoder: what it refuses is still text svara never
  writes.

### Corrections to earlier records

- ADR-0002's decision 2 lists `Spectrum` among the types that do not serialize. It
  derives `Serialize` since 3.3.0 and has a round-trip test. The type is flat, so
  that is within ADR-0002's own rules. Only the list was wrong.

## Consequences

- **Positive:** every place where svara and the oracle differ is now written down,
  with a measurement. 3.7.0 removes the oracle, so this is the last release where
  the record can be checked against it.
- **Negative:** two refusals change behaviour for hostile input, the infinite
  bandwidth and the error class for an infinite f0. Both are named in the CHANGELOG.
  A caller decoding JSON it did not get from svara must now call
  `svara_validate_json` first, and it refuses valid JSON that svara's writer would
  not produce, such as a pretty-printed document.
- **Neutral:** RN-DIV joins roadmap 3.9.x. Validated direct decoding remains
  possible future work, if a consumer ever needs it.

## Alternatives considered

- **Fix RN-DIV in 3.6.3.** Rejected. Either direction moves the audio for every
  diphthong, which breaks this release's bar, and choosing Rust's behaviour means
  choosing a filter reset on every sample.
- **Guard every directly decoded type.** Rejected for now. It would mean writing a
  validator for each derived type by hand, and the `restore` entry already gives a
  validated path. The decision can be revisited if a consumer decodes untrusted
  documents directly.
- **Return an error from the three fail-stop sites.** Rejected. They return an f64
  measurement, and any sentinel such as 0.0 or 1.0 is a value a caller would
  believe.
- **Accept any valid JSON in `svara_validate_json`.** Rejected. The decoder reads
  an escaped quote as the end of a key, and skips only spaces between a key and
  its value. Valid JSON with either one is misread, and an escape can line up the
  decoder's quotes so that a string's contents are read as structure. svara
  writes neither, so refusing them costs no document svara produced.
- **Time-limit the decode inside svara.** Rejected. svara takes scalars in and
  returns samples, with no signals and no processes
  ([ADR-0006](0006-scope-boundaries.md)). A check on the text needs neither.
