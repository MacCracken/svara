# 0015 — Retarget the formant bank without resetting it

**Status**: Accepted (3.12.0)
**Date**: 2026-11-16

## Context

The roadmap's 3.9.x diphthong item, from [ADR-0009](0009-what-the-3-6-3-audit-leaves-open.md)
(RN-DIV), says the free path's diphthong glide is 7× to 69× louder than the Rust's. Since 3.1.0,
`svara_ph_synth_diphthong` rebuilds the formant bank every 64 samples; the Rust rebuilds it every
sample.

**The cause is the rebuild itself.** `svara_tract_set_formants_from_target` rebuilds the bank, and
a rebuild zeroes every resonator's state, because the Rust's `set_formants` calls
`FormantFilter::new` (`git show 3.5.2:rust-old/src/tract.rs`). A path that retargets the tract
as it goes therefore resets its resonators each time:

| Path | Retargets | So its resonators |
|---|---|---|
| free diphthong (`svara_ph_synth_diphthong`) | every 64 samples, since 3.1.0 | restart every 1.45 ms |
| context diphthong (`svara_synthctx_voiced`) | every sample, as the Rust does | never ring |
| `svara_sequence_render_planned` | every sample, for every phoneme, as the Rust does | never ring |

A formant resonator with a 50 to 100 Hz bandwidth takes several milliseconds to ring up. Reset
every sample, it is "barely a filter", in ADR-0009's words: its output is little more than its
input times b0, which depends on the bandwidth and not on the frequency.

**Measured in 3.11.0** (male voice, 0.4 s, the middle half; peaks from
`svara_spectrum_estimate_formants`). "Kept" is a prototype, in a scratch copy, that updates the
five formants' coefficients in place and keeps each resonator's state:

| Render | today: peaks | today: level | kept: peaks | kept: level |
|---|---|---:|---|---:|
| `render_planned` /a/ (F1 730, F2 1090) | 126, 253, 379 | −63.0 dB | 629, 756, 2522 | −50.6 dB |
| `render_planned` /i/ (F1 270, F2 2290) | 126, 253, 379 | −64.1 dB | 253, 3028, 3657 | −58.2 dB |
| `render_planned` /u/ (F1 300, F2 870) | 126, 253, 379 | −66.3 dB | 379, 1009 | −37.3 dB |
| free /aɪ/, start / end | 2756, 3445, 3563 / 2067, 2756, 3563 | −37.4 dB | 570, 678, 1496 / 344, 430, 2067 | −53.1 dB |
| context /aɪ/, start / end | 118, 226, 344 / same | −63.8 dB | 570, 678, 1496 / 344, 430, 2067 | −53.1 dB |
| free /a/, for scale (no retarget) | 802, 1375, 2522 | −57.8 dB | unchanged | |

Today:
- **`render_planned` has no formants.** Its /a/, /i/ and /u/ peak at the same harmonics of the
  voice's 126 Hz. The trajectory planner it is built around is inaudible.
- **The context's diphthong is the same.** It has no formants and no glide.
- **The free diphthong is mostly rebuild transients.** It is high-frequency energy 20 dB above
  /a/. 3.11.0's Rosenberg gain made it 17.9 dB louder still, since the transients scale with the
  pulse.

The Rust does the same in `render_planned` and in both diphthong paths. Only svara's 3.1.0 control
rate changed the free diphthong, and in a way that did not help.

Kept, every path has its formants: `render_planned`'s vowels peak near their targets, and both
diphthongs glide from an /a/ region towards an /i/ region.

## Decision

**Keep each resonator's state across every retarget, in every path.** The maintainer chose
option 1 below on 2026-11-16.

**As shipped in 3.12.0:**
- `svara_tract_retarget(tract, target)` validates the target, then updates the five slots'
  coefficients in place with `svara_formant_bank_update`. Each resonator's state and the DC
  blocker's are kept. A tract whose bank holds some other number of formants gets
  `svara_tract_set_formants_from_target`.
- It replaces the rebuild in the four loops that retarget as they go:
  - `svara_sequence_render_planned`, every sample;
  - the context's diphthong (`svara_synthctx_voiced`), every sample;
  - the free diphthong (`svara_ph_synth_diphthong`), every 64 samples;
  - the free nasalized diphthong (`svara_ph_synth_vowel_nasalized`), every sample.
- `svara_tract_set_formants_from_target` keeps its reset, for one-off settings.
- `render_planned` retires 0.937x 3.11.0's instructions, because it no longer zeroes and rebuilds
  the bank every sample.

The options:

1. **Keep state on every retarget, in every path (proposed).** Add a tract retarget that updates the
   bank's coefficients in place and keeps each resonator's state. Use it wherever a path retargets
   as it goes: `render_planned` every sample, the context's diphthong every sample, the free diphthong
   every 64 samples. `svara_tract_set_formants_from_target` keeps its reset, for a one-off setting.
   - `render_planned`'s vowels get their formants, and diphthongs glide in both paths.
   - Every `render_planned` output and every diphthong changes.
   - It diverges from the oracle: a minor (3.12.0).
2. **Keep state in the diphthong paths only.** The two diphthong paths glide with their formants;
   `render_planned` keeps resetting every sample and stays formantless.
3. **Match the Rust.** Reset every sample in the free diphthong path too. Every diphthong loses its
   formants, as the context's already has; the free /aɪ/ drops about 26 dB.
4. **Keep it as it is, and document it.**

The proposal is option 1. `render_planned` and the context's diphthong exist to move formants
through time, and today they produce none. A resonator that keeps its state as its coefficients
move is how a time-varying formant synthesizer works; Klatt's updates its coefficients every 5 ms
without a reset.

## Consequences

- **Option 1** — `render_planned` changes completely: vowels gain their formants, and levels move by
  +6 to +29 dB against today, varying by vowel, since a ringing resonator's gain depends on where it
  sits. Diphthongs in both paths glide, at about −53 dB. Every consumer of `render_planned` hears a
  different voice. hoosh renders each phoneme through `svara_synthctx_synthesize` and does not call
  `render_planned`, so for hoosh only diphthongs change, under option 1 or 2.
  - Coefficients that move every sample over state that is kept can, in principle, add a slight
    zipper noise. The trajectory's targets move smoothly, so none was heard in the prototype, but
    the sweep and a listening check are the gate.
- **Option 2** — diphthongs as in option 1. `render_planned` keeps its formantless output, now
  documented.
- **Option 3** — diphthongs quieter and formantless everywhere: the Rust's behaviour, made
  consistent.
- **Option 4** — nothing moves, and the table above is the documentation.

## Alternatives considered

- **Reset at a slower control rate everywhere.** Any reset rate leaves transients at that rate, as
  the free diphthong shows; it changes how bad the artifact is, not whether it is there.
- **Crossfade two banks across each retarget.** It avoids transients but doubles the bank's cost
  per sample, for no gain over updating coefficients in place.
