# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [3.13.0] - 2026-11-16 — tone in every render

Minor release: toned output changes, by decision
([ADR-0016](docs/adr/0016-honour-tone-in-every-sequence-render.md)), the fifth 3.9.x quirk.

**The quirk.** An event's lexical tone (`svara_phoneme_event_with_tone`, nine tones, f0 contours
from 0.7 to 1.35×) was voiced only by `svara_sequence_render_planned`, as in the Rust. Three renders
gave bit-identical samples with or without each tone:
- `svara_sequence_render`;
- the streaming `svara_sequence_render_into`;
- the batch render.

For a tonal language they lost the word.

**The fix.** The maintainer chose to voice tone in every render. The glottal source reads its f0 only
at a period start, so `render_planned`'s per-sample `set_f0` takes effect there. A source can now
carry a tone and look its contour up at each period start, giving the same pitch track; the other
renders give each toned event's tone to the source that voices it.

**What moved.** Against 3.12.0, only toned input changes:
- 10,719 sweep values moved, all in the sweep's toned sequence through `sequence_render`;
- nothing else moved: every untoned render, the batch section, `render_planned`, the JSON and all
  638 hostile cases are bit-identical.

The glottal source retires 1.023× 3.12.0's instructions, for one check per sample, and the full
chain 1.006×. **1,441 assertions / 27 suites.**

### Changed

- **The tone moved to `src/glottal.cyr`**, ahead of `prosody.cyr`, because the source needs it:
  - the `SVARA_TONE_*` ids, including `SVARA_TONE_NONE`;
  - a non-allocating table of each tone's points, `svara_tone_points_into`;
  - the contour lookup, `svara_prosody_f0_at_filled` and `svara_prosody_eval`, unchanged.

  `svara_tone_to_contour` builds its contours from the table, and its values are the same bits.
- **`svara_glottal_set_tone(source, tone, base_f0, span)`** gives a source a tone over the next
  `span` samples. At each period start it sets f0 to `base_f0 × contour(sample / span)`, refusing an
  out-of-range f0 as `set_f0` does. Its points live in two buffers the source allocates on its first
  tone and keeps, so a warmed context still streams without allocating. `SVARA_TONE_NONE` clears it.
  An untoned source takes the old path after one check.
- **The free path.** The class synths, `svara_ph_synth_dispatch` and
  `svara_ph_synthesize_nasalized_raw` take a tone and give it to every source they build.
  `svara_phoneme_synthesize` keeps its signature.
- **The context.** `svara_synthctx_synthesize_toned` takes a tone, and the stream and the batch
  render call it. `svara_synthctx_synthesize` is the untoned case; its preamble clears the last
  phoneme's tone.
- **`sequence_render`** passes each event's tone.

One edge differs from `render_planned`: where base × contour leaves the f0 range, `render_planned`
keeps the last f0 accepted at any sample, and a source the last accepted at a period start.

### Added

- **ADR-0016**, accepted.
- **`tests/prosody.tcyr`** (2 groups):
  - **The table** is held to the Rust's `Tone::to_contour`, at tag `3.6.5`
    `rust-old/src/prosody.rs:231-285`, transcribed in the test. Since `svara_tone_to_contour` now
    builds from the table, comparing the two would prove nothing.
  - **The pitch track.** A source carrying each of the nine tones gives `render_planned`'s per-sample
    `set_f0` track, bit for bit. `SVARA_TONE_NONE` clears.
- **`tests/stream.tcyr`**: a toned sentence streams as its toned reference, bit for bit. The
  reference now passes each event's tone.
- **`tests/synthpaths.tcyr`**: `sequence_render` moves for the eight tones that move f0, and Mid,
  flat at 1.0, leaves it bit for bit.
- **`tests/render.tcyr`**: the batch render moves for falling and rising tones, Mid leaves it, and the
  length is the same.
- **`tests/allocbudget.tcyr`**: a warmed context streams a toned sequence keeping no bytes.
- **`docs/development/integration-guide.md`: "Tone in every render (3.13.0)".**
- **10 mutants, each failing an assertion:**
  - no tone at the period start;
  - a frozen sample count;
  - an off-by-one span;
  - no tone failing to clear;
  - a changed table point;
  - each of the five call sites dropping the tone: the context, the batch, the stream,
    `sequence_render` and the free path's helper.

## [3.12.0] - 2026-11-16 — formants that move keep ringing

Minor release: output changes, by decision
([ADR-0015](docs/adr/0015-retarget-the-formant-bank-without-resetting-it.md)), the fourth 3.9.x item:
the diphthong glide that ADR-0009 (RN-DIV) left open.

**The cause was larger than the item.** Every retarget of the formant bank also rebuilt it, and a
rebuild zeroes each resonator's state, as the Rust's `set_formants` does through
`FormantFilter::new`. A resonator with a 50 to 100 Hz bandwidth needs milliseconds to ring up, and
reset on every sample it never does. Through 3.11.0:
- **`svara_sequence_render_planned`** retargets every sample, for every phoneme, so it produced no
  formants at all. Its /a/, /i/ and /u/ peaked at the same harmonics of the voice, and the
  trajectory planner it exists for was inaudible.
- **The context's diphthong** retargets every sample, and had neither formants nor a glide.
- **The free diphthong** reset every 64 samples (3.1.0's control rate). Its output was mostly the
  transients of those resets: high-frequency energy 20 dB above /a/, and 3.11.0's Rosenberg gain
  scaled them by a further 17.9 dB.

The Rust does the same in all three. The maintainer chose to keep the resonators' state across
every retarget, in every path.

Measured (male voice, 0.4 s, the middle half, with peaks from `svara_spectrum_estimate_formants`):

| Render | 3.11.0 peaks | 3.11.0 level | 3.12.0 peaks | 3.12.0 level |
|---|---|---:|---|---:|
| `render_planned` /a/ | 126, 253, 379 | −63.0 dB | 629, 756, 2522 | −50.6 dB |
| `render_planned` /i/ | 126, 253, 379 | −64.1 dB | 253, 3028, 3657 | −58.2 dB |
| `render_planned` /u/ | 126, 253, 379 | −66.3 dB | 379, 1009 | −37.3 dB |
| free /aɪ/, start → end | 2756… → 2067… | −37.4 dB | 570, 678, 1496 → 344, 430, 2067 | −53.1 dB |
| context /aɪ/, start → end | 118, 226, 344 | −63.8 dB | as the free path | −53.1 dB |

**What moved.** 84,454 sweep values moved: phonemes 12,263 (the diphthongs), ctx 2,198,
pool 2,391, seq 57,302 (`render_planned` and the diphthongs in sequences) and spectrum 10,300. Batch,
glottal, tract, prosody, trajectory, bridge, misc and contours did not move, and the JSON is
byte-identical. In the hostile cases, 11 stream hashes moved, with no exit code, length or
non-finite count changed.

`render_planned` retires 0.937x 3.11.0's instructions, since it no longer zeroes and rebuilds the
bank every sample. **1,424 assertions / 27 suites.**

### Changed

- **`svara_tract_retarget(tract, target)`** moves a tract's formants and keeps every resonator's
  state, and the DC blocker's. It validates the target, then updates the five slots' coefficients in
  place. A tract whose bank holds some other number of formants gets the ordinary build.
- **The four loops that retarget as they go use it:**
  - `render_planned`, every sample;
  - the context's diphthong, every sample;
  - the free diphthong, every 64 samples;
  - the free nasalized diphthong, every sample.
- **`svara_tract_set_formants_from_target` keeps its reset**, for a one-off setting.

### Added

- **ADR-0015**, accepted. ADR-0009's RN-DIV now points to it as decided.
- **`tests/tract.tcyr`: the retarget** (11 assertions):
  - retargeting to the target a tract already holds changes nothing, bit for bit, where the
    resetting setter does;
  - a new target gives each slot fresh coefficients while slot 0 keeps its state;
  - a NaN target is refused, and the coefficients stay;
  - on a fresh tract a retarget builds, as the setter does.
- **`tests/synthpaths.tcyr`** (2 more assertions):
  - its references gain the retarget where 3.9.0 rebuilt;
  - a new reference, 3.9.0's `svara_synthctx_voiced`, holds the context's every vowel and
    diphthong, oral and nasalized, bit for bit;
  - the nasalized check now includes /aɪ/, which glides.
- **`docs/development/integration-guide.md`: "Moving formants (3.12.0)".**
- **7 mutants, each failing an assertion:**
  - a retarget that always resets;
  - one that skips validation;
  - one that skips slot 4;
  - each of the four call sites put back to the resetting setter.

### Not checked

- **Zipper noise.** Coefficients that move every sample over kept state can in principle add a
  slight zipper noise. The trajectory's targets move smoothly, and nothing in the measurements shows
  it, but no one has listened to these renders yet.

## [3.11.0] - 2026-11-16 — the Rosenberg pulse closes

Minor release: output changes, by decision
([ADR-0014](docs/adr/0014-the-rosenberg-pulse-and-its-speed-quotient.md)), the third 3.9.x quirk.

**The quirk.** `svara_glottal_set_speed_quotient` did nothing, because the Rosenberg pulse had no
closing phase for it to shape. Rosenberg's B pulse opens with `3x² − 2x³` and closes with
`1 − y²`, and the speed quotient is the ratio of the two phases. The Rust spreads the opening phase
over the whole open quotient and then drops from 1 to 0 at once. svara ported it line for line.

**The fix.** The maintainer chose to make the default pulse the full Rosenberg B: the model nearly
every voice uses. The full pulse is far less of a kick to the tract than an instant drop: a default
/a/ came out 17 dB quieter by RMS and 19 dB lower at its peak, which put it under the Rust
integration test's 0.001 audibility bar. Asked again with that number, the maintainer chose to keep
3.10.0's loudness. So the pulse carries a gain, `SV_G_ROSENBERG_GAIN` = 7.13033049099316
(+17.06 dB), solved so that a default male /a/ (0.5 s at 44.1 kHz) has 3.10.0's RMS to within
1e-6 dB.

**What it sounds like against 3.10.0.** Timbre is darker everywhere, and loudness by RMS (male,
0.5 s):

| /a/ | /e/ | /i/ | /o/ | /u/ | /ə/ | /aɪ/ | /m/ | /r/ | /z/ | /b/ |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0.00 | +0.38 | +7.51 | +2.09 | −0.87 | −1.31 | +17.94 | +5.01 | +5.17 | 0.00 | +0.65 dB |

/aɪ/'s +17.9 dB is nearly the gain itself. The free diphthong path rebuilds its formant bank every
64 samples, discarding its state, so its level is dominated by the transient each rebuild leaves,
and that transient scales with the pulse. That is the 3.9.x diphthong-glide item. Voices built at an
effort level use LF and do not change, and neither do whisper or creaky.

**What moved.** 318,085 sweep values moved, in every section that voices through Rosenberg:
phonemes, ctx, pool, glottal (2,448), tract (15,345), seq and batch. Prosody, trajectory, bridge,
misc and contours did not move. The spectrum section now prints two more lines than 3.10.0's, so it
was not compared value by value; that is most likely the formant estimator finding a different
number of peaks in the reshaped spectra. Eight JSON lines differ. In the hostile cases, values and
stream hashes moved, with no exit code, length or non-finite count changed.

The glottal source retires 1.073x 3.10.0's instructions, and the full chain 1.018x.
**1,411 assertions / 27 suites.**

### Changed

- **`svara_glottal_rosenberg_pulse` is Rosenberg's B**: `3x² − 2x³` over Tp = OQ·SQ/(1+SQ), then
  `1 − y²` over Tn = OQ − Tp, then 0, at a peak of `SV_G_ROSENBERG_GAIN`. SQ is Tp / Tn, the
  literature's definition, so the default 2 closes twice as fast as it opens. The Rust's doc comment
  describes it the other way round.
- **`svara_glottal_set_speed_quotient` refuses a NaN**, leaving the current value; ±inf clamp to
  [0.5, 5]. Through 3.10.0 a NaN was stored, harmless while SQ did nothing. `svara_glottal_restore`
  goes through the setter, so a restored NaN becomes the default 2.
- The per-sample path reads SQ at `SV_GS_SPEED_QUOTIENT`, pinned like the other offsets.

### Added

- **ADR-0014**, accepted. It records the measurements, the three options, the maintainer's choice
  and the second question about level.
- **`tests/glottal.tcyr`:**
  - **The golden first period**, re-derived in Python from the full pulse's definition, not from
    svara. It covers both phases and the peak at t = Tp.
  - **The speed quotient** (6 assertions): the peak sits at Tp for SQ 1 and SQ 4. A NaN leaves the
    value, ±inf clamp, and a restored NaN becomes 2.
- **`docs/development/integration-guide.md`: "The Rosenberg pulse (3.11.0)"**, with the per-phoneme
  level changes for consumers.
- **6 mutants, each failing an assertion:**
  - the closing phase removed;
  - Tp and Tn swapped;
  - the gain dropped, which also fails the Rust's audibility test and serde's not-silence
    controls;
  - a linear closing phase;
  - a NaN stored by the setter;
  - `restore` bypassing the setter.

## [3.10.0] - 2026-11-16 — every vowel carries its tilt

Minor release: output changes, by decision
([ADR-0013](docs/adr/0013-apply-the-per-vowel-spectral-tilt.md)), the second 3.9.x quirk. Each
vowel and diphthong has a spectral tilt, `svara_phoneme_spectral_tilt`: 0 to 2 dB/octave, from
F1, after Stevens & Hanson. The Rust computes it and drops it, because its tilt filter muted the
voice. 3.9.0 fixed that filter. The maintainer chose to apply the tilt in every render path,
converted from dB/octave to the filter's TL over the 3.32 octaves from 300 Hz to 3 kHz.

Measured on the free path (male, 0.2 s), against 3.9.0:

| Vowel | tilt | TL | level | above 2 kHz |
|---|---:|---:|---:|---:|
| /a/ | 1.87 | 6.2 dB | −4.6 dB | −5.7 dB |
| /æ/ | 1.47 | 4.9 dB | −3.3 dB | −5.0 dB |
| /o/ | 0.78 | 2.6 dB | −1.1 dB | −2.9 dB |
| /i/ | 0.16 | 0.5 dB | −0.4 dB | −0.6 dB |

Open vowels come out darker and softer, and close vowels barely move.

**What moved.** Against 3.9.0, 223,451 of the 558,721 sweep values moved, in every section that
renders a vowel:

| Section | values moved |
|---|---:|
| phonemes | 71,286 |
| ctx | 11,850 |
| pool | 14,335 |
| seq | 80,031 |
| spectrum | 41,102 |
| batch | 4,847 |

Nothing moved in glottal, tract, prosody, trajectory, bridge, misc or contours. Six JSON lines
differ: the spectrum magnitudes, and the saved context and pool states, which now carry a
vowel's tilt and its samples. In the hostile cases, 148 stream hashes and 17 raw values moved,
with no exit code, length or non-finite count changed.

A vowel now runs the tilt filter: phoneme /a/ retires 1.044x 3.9.0's instructions, and
`render_planned` 1.005x. **1,402 assertions / 27 suites.**

### Changed

- **`svara_phoneme_tilt_tl(p)`** is the tilt as the filter takes it:
  `svara_phoneme_spectral_tilt(p) × log2(10)`, and 0 for every class but vowels and diphthongs.
- **`svara_phoneme_spectral_tilt` allocates nothing.** It reads F1 into a target on the stack; it
  allocated one per call before. That keeps a warmed context at zero bytes per phoneme.
- **The free path**: `svara_ph_synth_vowel`, `_diphthong` and `_vowel_nasalized` set the tilt on
  the source each one builds. That covers `svara_phoneme_synthesize` and `svara_sequence_render`.
- **The context**: `svara_synthctx_synthesize` sets it in its preamble for every phoneme, so a
  consonant clears the vowel before it. That covers the pool, the batch render and
  `svara_sequence_render_into`.
- **`svara_sequence_render_planned`** sets each event's tilt when the event begins, on the one
  source the render shares.

### Added

- **ADR-0013**, accepted. It has the per-vowel measurements, both units, the cost, and where
  each path sets its source up.
- **`tests/synthpaths.tcyr`: every vowel path applies its tilt** (11 assertions):
  - the TL is the tilt times log2(10) for all 101 phonemes, 0 for consonants, and a hundred
    lookups allocate nothing;
  - the free path's every vowel and diphthong, at six lengths and two voices, and a nasalized /a/,
    match 3.9.0's synth plus the one tilt line, bit for bit. Control: /a/ no longer matches
    3.9.0's untilted samples;
  - the context's source holds /a/'s TL after /a/, 0 after /s/, and /ai/'s after /ai/;
  - `render_planned` matches 3.9.0's plus the one tilt line, bit for bit, on four sequences: a
    vowel, a consonant before two stressed vowels, a nasal before a diphthong, and a toned vowel.

  The reference computes the TL itself, so a wrong constant in `svara_phoneme_tilt_tl` is caught
  too.
- **`docs/development/integration-guide.md`** says how vowels changed, for consumers who balanced
  against 3.9.0.
- **7 mutants, each failing an assertion:**
  - the tilt dropped from the vowel synth, the diphthong synth, the nasalized vowel, the context
    preamble, and `render_planned`, one at a time;
  - the TL read directly instead of converted;
  - a context that sets the tilt but never clears it.

## [3.9.0] - 2026-11-16 — the glottal tilt reads its units

Minor release: output changes, by decision ([ADR-0012](docs/adr/0012-the-glottal-spectral-tilt-reads-its-units.md)),
the first of the 3.9.x quirks.

**The bug.** `svara_glottal_set_spectral_tilt` was documented as dB/octave, but the voiced
pulse's filter read the value as the corner of a one-pole low-pass in hertz:
`exp(-2π·tilt/sr)`, the Rust's own formula. So any tilt above 0 put a low-pass of a few hertz in
front of the voice and muted it by about 33 dB. That is why the Rust never applies the per-vowel
tilt.

The same filter takes the effort presets' tilts, so `svara_voice_create_glottal_source_with_effort`
gave an /a/ through the tract at these levels:

| Effort | tilt | 3.8.8 | 3.9.0 |
|---|---:|---:|---:|
| whisper | 12 | −101.3 dB | −101.3 dB |
| soft | 6 | −68.4 dB | −61.0 dB |
| **normal** | 2 | **−85.9 dB** | −60.4 dB |
| loud | 0 | −60.0 dB | −60.0 dB |
| shout | 0 | −63.5 dB | −63.5 dB |

"Normal" was 26 dB below "loud" and 17 dB below "soft". Now soft, normal and loud are in order.

**What moved.** Against 3.8.8, none of the 558,721 sweep values moved and the JSON is
byte-identical: no sweep case builds an effort voice, and none sets a tilt outside the hostile
cases. Of the 638 hostile cases, the exit codes are all unchanged. The 378 raw values that moved
are all entry 50, `svara_glottal_set_spectral_tilt` with a hostile value:
- **+Inf, MAX, 2^63, 1e19 and 3e9** read as near-infinite attenuation now, so they mute the voiced
  pulse to 0.0. They used to read as a corner above Nyquist and leave it unfiltered.
- **A subnormal tilt** used to give a coefficient of exactly 1, which froze the pulse at 0. It
  now gives exactly the tilt-0 output, as a TL that small should.

Retired instructions are unchanged on every probe, since none of them sets a tilt.
**1,391 assertions / 27 suites.**

### Changed

- **The voiced pulse's tilt is Klatt's TL**: the filter's attenuation at 3 kHz, in dB below its
  gain at 0 Hz.
  - `svara_glottal_tilt_alpha(tl, sample_rate)` solves the one-pole coefficient. The root is
    written so that a small TL does not cancel.
  - `svara_glottal_tilt_alpha_for` memoizes the coefficient on the exact bits of the tilt and the
    sample rate, so it is solved once per change instead of with an `exp` every sample. A tilt
    written through the derived accessor, or by `svara_glottal_restore`, is solved too.
  - The memo lives in three fields appended to `SvGlottalSource` (`SV_GS_TILT_ALPHA*`), so no
    existing offset moves.
- **The effort presets keep their numbers.** Read as TL, 2, 6 and 12 dB at 3 kHz are the mild to
  steep range Klatt's synthesizers use.
- **Whisper keeps the Rust's noise integrator**: a one-pole at the tilt in hertz, at least 12, with
  its 0.15 input scale tuned to it. Its comment now says so.
- **A tilt of 0 or below, or NaN, leaves the pulse unfiltered, as before.**
- Comments on `svara_glottal_set_spectral_tilt`, the effort constructor,
  `svara_bridge_spectral_tilt_from_distance` and the vowel synth say what the value means now.
  The vowel synth still leaves its own tilt unapplied: that is a separate decision on the 3.9.x
  list, and with the fixed filter /a/'s tilt would cost 1.5 dB, not 33.6.

### Added

- **ADR-0012**, accepted. It records the measurements, the maintainer's choice of TL over keeping
  the Rust's filter or designing a true dB/octave cascade, and the decision to leave the
  per-vowel tilt for later.
- **`tests/glottal.tcyr`: the tilt is TL** (10 assertions):
  - each coefficient's gain at 3 kHz, from the filter's transfer function, is TL dB down, for TL
    0.1 to 40 at 16, 44.1 and 96 kHz;
  - TL 0 gives exactly 0, and +inf exactly 1;
  - negative and NaN tilts give the unfiltered pulse, bit for bit, and a tilt of 0.5 filters it;
  - a tilt written through the accessor, and changed mid-stream, gives the setter's bits, and
    after the change the memo holds a fresh solve's coefficient.

  The offset pin now covers the three new fields.
- **`tests/glottal.tcyr`: whisper keeps the Rust's integrator** (2 assertions). The test rebuilds
  `glottal.rs:440-443` at tag 3.6.5 from a second noise generator on the source's seed. It holds
  whisper to that bit for bit at tilt 0, the 12 Hz floor, and at tilt 30.
- **`tests/voice.tcyr`: the effort order** (3 assertions). An /a/ at normal effort is within a dB
  of loud, below it, and above soft. Through 3.8.8 it was 26 dB below loud.
- **`docs/development/integration-guide.md`: "Spectral tilt (3.9.0)"**, for consumers who balanced
  levels against the old behaviour.
- **7 mutants, each failing an assertion:**
  - the Rust's formula put back;
  - the TL read at 2 kHz, or as an amplitude rather than a power;
  - normal effort's tilt set to 0;
  - a memo that ignores the tilt;
  - a filter engaged only above 1;
  - whisper switched to the TL coefficient.

  The last three passed the first version of these tests. The memo test then compared two
  sources running the same memo, so it could not see one that never re-solved; nothing rendered
  a tilt below 1; and nothing pinned whisper. The memo, small-tilt and whisper assertions above
  are what catch them.

## [3.8.8] - 2026-11-16 — the performance lane closed

The last 3.8.x item is decided, and the lane is measured end to end. `src/` differs from 3.8.7
only in one comment, so no output moves. **1,376 assertions / 27 suites.**

### Decided

- **`svara_seq_crossfade` stays public, in `src/`.** Since 3.8.3 no svara render calls it: both
  sequence renders crossfade through `svara_seq_xf_segment`, which `tests/stream.tcyr` holds to it
  bit for bit. The roadmap offered to move it into a test include. It stays because hoosh's suite
  calls it from `dist/svara.cyr`, as the reference for hoosh's own streamed crossfade (hoosh ADR
  016), so moving it would break hoosh at its next bump. Its comment now says so. Its fade-out
  loop, written twice as in the Rust, keeps the Rust's shape.

### Changed

- **`docs/benchmarks.md`'s table is measured across the lane.** 3.8.2's benches, built from its tag,
  ran in turn with 3.8.8's, three runs each, lowest of each. The 3.8.2 column reproduces 3.8.2's own
  published table to within a nanosecond or two a row.

  | Bench | 3.8.2 | 3.8.8 | ratio |
  |---|---:|---:|---:|
  | the full chain, `tract synthesize_into` 1024 | 184 µs | 154 µs | 0.84 |
  | tract `process_sample` | 134 ns | 114 ns | 0.85 |
  | formant filter `process_sample` | 85 ns | 73 ns | 0.86 |
  | phoneme /a/, 0.05 s | 445 µs | 368 µs | 0.83 |
  | sequence render, 10 phonemes | 5.04 ms | 3.81 ms | 0.76 |
  | `render_planned`, 3 phonemes | 10.6 ms | 9.75 ms | 0.92 |

  The chain is now ≈ 0.15 µs a sample, about 150x real time at 44.1 kHz on one core, up from
  125x. The README, `docs/architecture/overview.md`, ADR-0003 and `state.md` quote the new figures.
- **The roadmap's 3.8.x section is gone**, since nothing in it is left to do. Its method notes moved
  to `docs/benchmarks.md`'s new "Changing a hot path":
  - measure the right thing;
  - profile by component;
  - an inlined accessor is not free;
  - record negative results.

  3.9.x, the preserved Rust quirks, is next. Each of its items needs a decision.

## [3.8.7] - 2026-11-16 — the per-class synthesis, written once

Refactor release. Every consonant class was synthesized twice:
- by the free path's `svara_ph_synth_*`, which builds fresh objects per call;
- by the context's `svara_synthctx_*`, which reuses its own.

Where the Rust's two paths write the same samples, svara's two copies are now one. **No output
moves**: against 3.8.6, 558,721 values, the JSON and all 638 hostile cases are identical. The
nine counter probes are unchanged to four places. Consonant renders retire 0.995x their
instructions on the free path, which no longer zeroes a buffer it then overwrites, and 1.000x on
the context's. `src/phoneme.cyr` is 44 lines shorter. **1,376 assertions / 27 suites.**

### Changed

- **The runs both paths write the same way are shared** (`src/phoneme.cyr`, "the runs both
  synthesis paths share"):
  - `svara_ph_noise_run`, `svara_ph_voiced_run` and `svara_ph_zero_run`, the burst, voicing and
    silence spans every class is made of;
  - `svara_ph_fricative_run`, `_affricate_run`, `_click_run`, `_ejective_run` and
    `_implosive_run`, a whole class's samples once its objects exist;
  - `svara_ph_nasal_place`, `svara_ph_ejective_is_fricative`, `svara_ph_approx_breathiness` and
    `svara_ph_approx_gain`.

  Each path now only gathers its objects. The free path builds them, the context fetches its own,
  and each then calls the run. Every allocation is made in the same order as before, which
  `tests/allocrefused.tcyr` depends on.
- **What the Rust's two paths do differently stays apart**, for 3.9.x to decide:
  - **The plosive.** The free path uses the VOT fractions and an aspiration segment; the context
    uses fixed thirds. Both now call the shared bursts and tails.
  - **The trill.** The free path modulates its amplitude; the context renders it as an
    approximant.
  - **The vowel and the diphthong.**

  The roadmap's 3.9.x plosive item now names the trill too, citing both at tag `3.6.5`.

### Added

- **`tests/synthpaths.tcyr`, a 27th suite** (6 assertions). It rebuilds 3.8.6's sixteen class
  functions, verbatim but for their names, and holds both paths to them bit for bit:
  - every phoneme of the eight classes that share runs, at six lengths from 1 sample to 0.08 s,
    for the male and female voices;
  - the free path call by call, and the context path phoneme after phoneme on two fresh contexts,
    so state one phoneme leaves is compared too.

  Before it, no suite pinned a consonant's samples. Twelve mutants of the shared runs each passed
  all 26 suites: a burst or voicing gain ignored, a silence one sample short, a fricative's
  voicing dropped, an affricate's burst lengthened, a click's envelope not squared, an ejective
  released through the wrong filter, an implosive's closure shortened, a nasal's place swapped, an
  approximant's breathiness floor or gain changed, and /tʃ'/ treated as non-fricative.
  Each now fails this suite. Eleven fail on both paths. The silence one sample short fails on the
  context path only: the free path renders into fresh arena memory, which is already zero, while
  the context reuses a buffer that still holds the last phoneme.

## [3.8.6] - 2026-11-16 — two roadmap items closed by measurement, and one `approx`

No library code changes: `src/` is byte-identical to 3.8.5, and so is every output. The bundle
differs only in its version line. Two of the roadmap's 3.8.x items were measured and closed,
and the tests share one tolerance helper. **1,370 assertions / 26 suites.**

### Measured, and not taken

- **naad's block forms do not fit svara's tract.** The roadmap asked to call
  `filter_biquad_process_buffer` and `noise_fill_buffer` where the tract's feedback allows it,
  and it allows almost nowhere:
  - **The biquads run inside the feedback loop.** The tract's source-filter interaction feeds
    each output sample into the next excitation, and the subglottal resonator and the nasal
    notch sit inside that loop. The notch also advances only while its coupling is above zero.
  - **Noise can't be drawn ahead.** The glottal source draws one aspiration-noise sample per
    sample. Filling a block ahead would put the noise generator past the samples consumed,
    and the glottal save / restore reads that generator's state.

  The cost a block form would save is in naad's per-sample functions:

  | naad call | per-sample call | block form, per sample |
  |---|---:|---:|
  | `filter_biquad_process_sample` | 335 instructions | 195 |
  | `noise_next_sample`, white | 240 | 130 |

  The two were rebuilt in a scratch probe the way their block loops are written, nothing else
  changed. svara's output stayed byte-identical in all 14 sweep sections, and the instructions
  fell:
  - the full chain to 0.947x;
  - a phoneme to 0.951x;
  - the glottal source to 0.911x.

  So the item is now blocked upstream, on a naad issue.
- **The per-phoneme glottal source and tract stay.** `svara_sequence_render` builds both for
  every phoneme, as the Rust does. Together they cost 41,310 instructions and 2,272 bytes:
  - 0.43% of synthesizing a 0.08 s /a/;
  - under a byte a sample, against the render's 24.

  Reusing them would need a reset that gives a fresh object's bits, a second constructor to
  keep in step, for no measurable effect. `svara_sequence_render_into` already avoids them.

### Changed

- **The tests share one `approx`** (`tests/approx.cyr`). Thirteen suites each defined the same
  one-line tolerance test under eight names: `approx`, `br_`, `v_`, `pr_`, `ph_`, `se_`, `tr_`,
  `or_` and `rn_approx`. Each now includes the one file. The assertion count is unchanged.
  - Two mutants of the shared helper each fail serde's controls: one that always answers yes,
    and one that ignores `eps`. Those controls assert that two values are *not* within tolerance.

### Added

- **Upstream: naad
  `docs/development/issues/2026-11-16-per-sample-biquad-and-noise-cost-more-than-their-block-loops.md`**,
  with a repro that counts each function's instructions against its block form's. It asks that
  the per-sample paths be written like their block loops, or for a raw-pointer noise fill.
- **The cyrius accessor issue** (`2026-11-16-derived-accessor-costs-more-than-load-store.md`)
  gained svara 3.8.5's approach for a struct that keeps its accessors: offsets beside the
  struct, pinned by a test.

## [3.8.5] - 2026-11-16 — the per-sample path without derived accessors

Performance release. 3.8.4 measured that a `#derive(accessors)` accessor costs more than the
`load64` / `store64` it stands for: on cyrius 6.7.18 a getter retires 2 instructions more, a setter
6 more, and each keeps a jump. **The 84 accessor uses left on the per-sample path now read and
write at `const` offsets** — 66 getters and 18 setters, from the glottal source and its random
numbers through the tract, the smoothers, the formant bank's header and the DC blocker.
Against 3.8.4 (min of 9 interleaved runs):

| Probe | instructions | cycles |
|---|---:|---:|
| glottal source, 2M samples | 0.947x | 0.982x |
| formant filter, 1M | 0.960x | 0.959x |
| tract `process_sample`, 1M | 0.954x | 0.955x |
| tract, Reduced and Minimal | 0.948x, 0.952x | 0.952x, 0.963x |
| `synthesize_into`, 500 × 1024 | 0.952x | 0.924x |
| phoneme /a/ × 100 | 0.956x | 0.929x |
| `render_planned` and the toned render, × 20 | 0.992x, 0.993x | 0.990x, 0.992x |

The cycle geomean is 0.960. The instruction counts were the same in all three sweeps. The cycles
are from the one run made while the host was otherwise idle: two earlier runs, under a load of 12
to 15 from other work, swung the same binaries between 0.80x and 1.26x. **No output moves**: against 3.8.4, 558,721 values, the JSON and all 638 hostile cases are
identical. **1,370 assertions / 26 suites.**

### Changed

- **The per-sample functions use `const` offsets.** `svara_glottal_next_sample` and the pulse,
  whisper and new-period functions under it, `svara_rng_next_u32` / `_next_f32` /
  `_next_f32_unsigned`, `svara_tract_process_sample`, `svara_smooth_next`,
  `svara_formant_filter_process_sample`, `svara_formant_bank_process` and
  `svara_dc_blocker_process`. The rewrite is mechanical: each `T_f(p)` became
  `load64(p + SV_X_F)`, each `T_set_f(p, v)` became `store64(p + SV_X_F, v)`, and nothing else
  changed.
- **The structs keep their derived accessors.** Everything off the per-sample path still uses
  them, and they stay the API. The offsets are declared beside each struct, only for the fields
  the per-sample code touches: `SV_GS_*` (SvGlottalSource), `SV_VT_*` (SvVocalTract), `SV_SP_*`
  (SvSmoothedParam), `SV_RNG_*`, `SV_FB_*`, `SV_DC_*` and `SV_FF_*`.
- **Not taken: `render_planned`'s retargeting.** Its per-sample loop spends most of its
  instructions in the trajectory and the bank rebuild, which still read through accessors, and
  the path moved only to 0.992x. Not rewritten for under 1%.

### Added

- **Offset pins, one group per struct** (14 assertions, in `tests/glottal.tcyr`, `tract.tcyr`,
  `smooth.tcyr`, `rng.tcyr` and `formant.tcyr`). Every field gets a distinct value through its
  derived setter, and each offset must read its own field's value, so a reordered struct fails a
  test instead of reading a neighbour.

- **`tests/glottal.tcyr`: jitter moves only the period, shimmer only the amplitude** (4
  assertions). A source with shimmer 0 keeps its amplitude at exactly 1.0 through ten periods,
  and one with jitter 0 keeps its period at exactly `sr / f0`, as Rust's `new_period` does. The
  mutation run found the gap: `new_period` reading jitter where it reads shimmer passed the whole
  suite.

  8 mutants, each failing at least one assertion:
  - six offsets moved onto a neighbouring field: the tract's lip delay, the glottal phase, a
    smoother's target, the generator's increment, the DC blocker's output delay, and the bank's
    second input delay;
  - the tract's gain read from its nasal coupling;
  - `new_period`'s shimmer read from its jitter, caught only by the new jitter / shimmer group.

## [3.8.4] - 2026-11-16 — the formant bank, one record per slot

Performance release. The formant bank kept its eight slots as the Rust's structure of
arrays: seven parallel buffers for b0, b2, a1, a2, amp, y1 and y2. **Each slot is now one
56-byte record, read and written at `const` offsets**, and the bank and every path through it
retire fewer instructions in fewer cycles (min of 9 interleaved runs against 3.8.3):

| Probe | instructions | cycles |
|---|---:|---:|
| formant filter, 1M samples | 0.951x | 0.897x |
| tract `process_sample`, 1M | 0.970x | 0.895x |
| tract, Reduced and Minimal | 0.963x, 0.962x | 0.897x, 0.902x |
| `synthesize_into`, 500 × 1024 | 0.977x | 0.937x |
| phoneme /a/ × 100 | 0.979x | 0.943x |
| `render_planned` and the toned render, × 20 | 0.965x, 0.971x | 0.948x, 0.956x |

The cycle geomean over all nine probes is 0.934, with the glottal source's unchanged code at
1.042, host noise. **No output moves**: against 3.8.3, 558,721 values, the JSON and all 638
hostile cases are identical. **1,352 assertions / 26 suites.**

### Changed

- **The bank's slots are records** (`src/formant.cyr`). A slot is b0, b2, a1, a2, amp, y1 and y2,
  one f64 each, at `SV_FSLOT_B0` .. `SV_FSLOT_Y2`, and `svara_formant_bank_slot(bank, i)` gives
  slot i's record. The per-sample loop steps one pointer through the active records instead of
  indexing seven buffers.
- **A slot's coefficients are computed straight into its record.** b0 .. a2 sit in the order
  `svara_formant_biquad_from_trig` writes them, so the bank's 32-byte `coeff` scratch and its four
  copies per update are gone. A bank keeps 1,016 bytes instead of 1,104, and a formant filter 1,064
  instead of 1,152.
- **The record is `const` offsets, not a `#derive(accessors)` struct.** The layout alone, through
  derived accessors, retired 5.4% *more* instructions than 3.8.3. On cyrius 6.7.18 an accessor is
  inlined but keeps a jump: a getter retires 2 instructions more than a `load64`, a setter 6 more
  than a `store64`. Formant filter instructions per million samples:

  | The slot loop | M instructions |
  |---|---:|
  | 3.8.3: seven buffers, `load64` / `store64` | 1,272 |
  | records through derived getters and setters | 1,340 |
  | records through derived getters, `store64` | 1,280 |
  | records at `const` offsets | 1,210 |

  A `const` offset costs what a literal one does.
- **A rebuild zeroes the records past the new count whole**, delays included, where it zeroed five
  buffers' tails. The reset after it zeroes every slot's delays, as before.

### Added

- **`tests/formant.tcyr`: the bank against 3.8.3's, bit for bit** (10 assertions). The test
  rebuilds 3.8.3's bank of seven buffers, which runs all eight slots, and feeds both banks the
  same inputs:
  - an impulse, a step and an alternating input;
  - an update of an active slot, and of a slot past `count`;
  - +inf and the NaN it leaves, and a reset;
  - a rebuild to three formants, which must zero records 3 .. 7 whole and give a fresh
    three-formant bank's bits.
- **`tests/hardening.tcyr`**: slot 8 is refused and leaves the 56 bytes past the eighth record
  alone. Unchecked, it would land on slot 0's trig memo.

  9 mutants each fail at least one assertion:
  - the y2 offset aliasing y1, and records overlapping;
  - a reset that keeps y2;
  - a rebuild that skips slot n, or zeroes two records short;
  - the loop ignoring the unused-slot limit;
  - the weight stored over a2, and the coefficients written 8 bytes in;
  - `svara_formant_bank_update` without its slot check.

  Three also end a suite on a fault, after the failed assertions: the overlapping records, the
  rebuild that skips slot n, and the unchecked update.
- **Upstream: cyrius
  `docs/development/issues/2026-11-16-derived-accessor-costs-more-than-load-store.md`**, with a
  repro that counts retired instructions and branches. It asks that an inlined accessor cost what
  its `load64` / `store64` does.
- `svara_formant_bank_slot` and the `SV_FSLOT_*` offsets.

### Removed

- ⚠ **`SvFormantBank_b0`, `_b2`, `_a1`, `_a2`, `_amp`, `_y1`, `_y2` and `_coeff`**, with their
  setters. They were derived accessors on the bank's internal buffers, and only svara's tests read
  them. A slot's values are at `svara_formant_bank_slot(bank, i) + SV_FSLOT_*`.

## [3.8.3] - 2026-11-16 — each render keeps its samples about once

Performance release, in memory rather than time. A cyrius `vec` cannot be sized up front:
`vec_push` doubles from 16 slots and copies, and the bump allocator never frees. So every render
kept its output about four times over: the buffer it rendered into, then every buffer the vec
outgrew, then the vec. **Each render now keeps between 0.30x and 0.50x of 3.8.2's bytes**,
measured on the same inputs (male voice, 44.1 kHz):

| Render | samples | 3.8.2 | 3.8.3 |
|---|---:|---:|---:|
| `svara_phoneme_synthesize`, /a/ for 0.1 s | 4,410 | 169,160 B | 68,528 B |
| `svara_phoneme_synthesize`, /s/ for 0.08 s | 3,528 | 95,184 B | 34,376 B |
| `svara_sequence_render`, four phonemes | 7,513 | 610,496 B (81 a sample) | 184,656 B (24) |
| `svara_sequence_render_planned`, the same four | 13,589 | 375,744 B (27) | 136,144 B (10) |
| a warmed batch render, three events | 12,082 | 262,624 B (21) | 131,592 B (10) |
| a warmed pool batch, three pairs | 12,348 | 262,040 B (21) | 131,152 B (10) |

**No output moves**: against 3.8.2, 558,721 values, the JSON and all 638 hostile cases are
identical. Retired instructions are within 0.3% on every probe but phoneme /a/, at 0.984x. Cycles
moved only with host noise: the geomean was 1.001 and 1.005 in two sweeps. **1,342 assertions /
26 suites.**

### Changed

- **A sample vec keeps the buffer its samples were rendered into.** `svara_vec_adopt`
  (`src/error.cyr`) builds the vec through `vec_new_a` and `vec_push_a` with an Allocator of its
  own. That Allocator answers every data-sized request that fits with the rendered buffer, so the
  vec "grows" into it, its copy writes each word over itself, and it ends up holding the buffer.
  - **What it relies on.** `lib/alloc.cyr`'s Allocator contract, not `vec`'s layout, so the
    roadmap's "do not hand-build a vec header" still holds. A request under 64 bytes (the header)
    or larger than the buffer gets an ordinary allocation, so a short buffer is copied out of and
    never written past.
  - **What it costs.** The buffer must have the capacity the vec would grow to
    (`svara_alloc_vec_room`: 16 slots, doubled until they hold n), which is up to twice its
    samples. Each vec also carries a 40-byte Allocator and 16 bytes of state.
  - **What would remove that.** A `vec_with_capacity` in cyrius, now asked for upstream.
- **`svara_phoneme_synthesize` and `_nasalized` render into that room and adopt it.** The vowel,
  nasal and approximant synths write straight into it through `svara_tract_synthesize_into`, rather
  than through a vec of the tract's own. `svara_ph_buf_to_vec`, which pushed every sample onto a
  vec from empty, is gone.
- **`svara_sequence_render` keeps each phoneme's samples in a buffer of exactly their length**
  (`svara_ph_synthesize_nasalized_raw`), then crossfades them straight into one buffer sized for
  the output, which becomes the returned vec (`svara_seq_join`).
  - The crossfade step is `svara_seq_xf_segment`, the stream's step since 3.8.0.
    `tests/stream.tcyr` holds it to `svara_seq_crossfade` bit for bit.
  - One voice copy serves the whole render, instead of a clone per event.
  - `svara_seq_crossfade` stays as the reference that both are held to.
- **`svara_sequence_render_planned` adopts its output buffer** instead of copying it into a vec.
- **The batch render and the pool batch size their output once, from the plan**
  (`svara_synthctx_count_for`, `svara_samples_acc_*`). A batch render also copies its voice once
  instead of per event.
  - **A render that will fail sizes nothing.** If an event would be refused, they grow as they go
    instead, so the render keeps no room for the events after the refusal.
  - **One stress table left.** The batch render now takes its pitch and duration from
    `svara_seq_stress_f0` and `svara_seq_event_duration(e, 0)`. That removes the last copy of the
    stress table outside `src/sequence.cyr`, and the bits are the same.
- **A refused growth of a render's output returns `SVARA_ERR_COMPUTATION`.** It used to end the
  process inside `vec_push`. A refused vec header or Allocator still ends it, as `vec_push`'s did.
- **Not taken:**
  - **The batch render's progress object.** It is still built per event. The callback was never
    told the object is valid only during the call, and it is 24 bytes an event, not a sample.
  - **The glottal source and tract per phoneme** in `svara_sequence_render` stay too, about
    2.4 KB a phoneme. `svara_sequence_render_into` is the render that avoids them.

### Added

- **`tests/allocbudget.tcyr`: three groups for 3.8.3** (39 assertions):
  - **The vec keeps its buffer.** Adopting a vec's room keeps at most 128 bytes. A short buffer is
    copied out of and never grown into: a guard right after an 800-byte buffer survives pushes up
    to the vec's capacity. An adopted vec grows like any other. The accumulator holds its samples
    through two growths, an empty append and a refusal past `SV_MAX_SAMPLES`.
  - **What each render keeps.** Each one is bounded against its 3.8.2 figure, with controls. A
    pool batch and a batch render refused at their first event keep no room for the rest.
  - **The same samples.** Each render is compared bit for bit with the 3.8.2 code, rebuilt in the
    test from public pieces:
    - `svara_sequence_render` on a nine-event sequence through every crossfade branch, on one
      event, and with a second voice;
    - the batch render and the pool batch, over the same nine events.

  11 mutants each fail their own check:
  - the Allocator never adopting, and adopting past the buffer;
  - `render_planned` copying out;
  - the segments taking a vec's room;
  - the join ignoring the fades in its length;
  - both batch renders sizing past a refusal;
  - a growth dropping samples;
  - one voice copy with the stress compounding;
  - the batch render shortening clusters;
  - the pool batch dropping a sample.

  Two more were tried. Leaving the scratch voice uncopied ends the suite on a fault, and the other
  was equivalent and was dropped.
- **Upstream: svara's section in cyrius
  `docs/development/issues/2026-11-11-vec-keeps-outgrown-buffer-and-aborts.md`.** It asks for
  `vec_with_capacity` or a vec over an existing buffer, with a repro: 4,410 words pushed from empty
  keep 130,968 bytes, and adopted, 65,600.
- `svara_vec_adopt`, `svara_alloc_vec_room`, `svara_vec_room_bytes`, `svara_vec_cap_for`, the
  `svara_samples_acc_*` accumulator, `svara_ph_alloc`, `svara_ph_synthesize_nasalized_raw`,
  `svara_seq_join`, `svara_synthctx_count_for` and `svara_pool_batch_expect`.

### Removed

- `svara_ph_buf_to_vec`. Nothing calls it.

## [3.8.2] - 2026-11-16 — the tract chain in 0.74x the instructions

Performance release. **`tract process_sample` retires 0.744x of 3.8.1's instructions in 0.709x
the cycles**:
- the formant filter, 0.712x and 0.701x;
- `synthesize_into`, 0.794x and 0.716x;
- the Reduced and Minimal quality tiers, about 0.70x and 0.62–0.67x.

The full glottal → tract chain is now ≈ 0.18 µs a sample, about 125x real time at 44.1 kHz.
**No output moves**: against 3.8.1, 558,721 values, the JSON and all 638 hostile cases are
identical. **1,303 assertions / 26 suites.**

### How it was found

Counting retired instructions per component put 59% of `tract process_sample` in the formant
bank. The rest: the naad subglottal biquad 337 instructions a sample, two smoothers 136 each, the
DC blocker 133, and four calls to the quality predicates about 49 each. The bank ran all eight
slots for five formants, as Rust's does, because the unused three hold zero coefficients.

### Changed

Each change was measured on its own against 3.8.1, and each leaves every value the same bits:

- **The bank skips the slots past `count` while that is exact** (tract -18.0% instructions,
  -16.6% cycles):
  - **Why it is exact.** An unused slot adds exactly nothing while it has no coefficients and no
    state and the input is finite: the sum starts at +0.0 and can never become -0.0, so adding
    ±0 leaves its bits alone.
  - **When all eight still run.** A non-finite input (tested on the exponent bits) runs all eight,
    as Rust does, so NaN reaches the unused slots' delay lines and stays there exactly as before;
    the bank then keeps running all eight. So does a coefficient that `svara_formant_bank_update`
    sets past `count`.
  - **What clears that.** A reset or a rebuild returns the bank to the skip. Two new bank fields,
    `out_coef` and `out_dirty`, carry the state.
- **The tract keeps its quality's four flags as bits** (-3.3% instructions, -8.6% cycles).
  `svara_tract_store_quality` asks `lod.cyr`'s predicates once whenever the quality is set, from
  `new`, `set_quality` or `restore`. Four calls a sample cost more in cycles than in instructions.
  3.5.0 had timed `#inline` on the predicates and concluded call overhead did not matter there;
  `src/lod.cyr` now says otherwise.
- **A converged, finite smoother returns at once** (-3.7%, -3.5%). The full path would have stored
  the same bits. A non-finite value takes the full path, because a zero-time smoother at inf goes
  to NaN through it.
- **`svara_formant_filter_process_sample` is one expression**, so cyrius 6.7 inlines it (-0.6%,
  -1.0%). On cyrius 6.7.18, `#inline` takes a body under 32 tokens with no `var` or control flow
  and warns otherwise. The same marker on `svara_smooth_next` and `svara_dc_blocker_process` was
  ignored, and was removed.
- **Not taken:** laying the bank out one record per slot, instead of seven parallel arrays, might
  save about 4% more. Every bank user would change, so it is left on the roadmap. Moving the
  tract's field reads into locals would buy nothing: derived accessors have been inlined since
  cyrius 6.5.62.

### Added

- **Benches for the four Rust scenarios** that had none here, on the Rust's own inputs: the tract
  at Reduced and at Minimal quality, a female /a/ at 0.1 s, and the ten-event sequence.
  `docs/benchmarks.md`'s table is re-measured at 3.8.2, with the Rust medians beside them.
- `scripts/sweep/compare.py` counts the tract at Reduced and Minimal quality too.
- **`tests/allocbudget.tcyr`: 3.8.2's shortcuts against the long way** (6 assertions):
  - the bank skip against a bank made to run all eight slots, through finite input, +inf, NaN,
    -inf, resets, a coefficient past `count` and a rebuild;
  - the quality flags against the predicates, after `set_quality` and after `restore`;
  - the smoother against 3.8.1's code over 21,000 steps and a zero-time smoother at inf.

  Four mutants each fail their own check: a non-finite input not marking the unused slots,
  `bank_update` not marking a coefficient past `count`, `restore` setting the quality without its
  flags, and the smoother returning a non-finite value early.
- `svara_tract_store_quality`, the bank's `out_coef` and `out_dirty`, and the tract's `qflags`.

### Fixed

- **Stale throughput figures.** The README, `docs/architecture/overview.md` and ADR-0003 still
  said 0.35 µs a sample and 64x real time, which 3.6.3 had already improved on. All now give
  3.8.2's measurement.

### Verified

- `scripts/sweep/compare.py 3.8.1`: 558,721 values, the JSON and 638 hostile cases are identical.
  Retired instructions are equal on the glottal probe. Ratios on the rest:

  | Probe | Instructions |
  |---|---:|
  | formant | 0.7122 |
  | tract | 0.7440 |
  | tract Reduced | 0.7015 |
  | tract Minimal | 0.6946 |
  | `synthesize_into` | 0.7939 |
  | `phoneme /a/` | 0.8097 |
  | `render_planned` | 0.9722 |
  | toned render | 0.9766 |

  The cycle geomean is 0.777.
- `cyrius audit`, `cyrius fuzz`, `cyrius deny`, `scripts/check-logging.sh` both ways, the five
  examples, `cyrius bench`, `cyrius distlib` current, and `cyrius deps --verify` and `--locked`.

## [3.8.1] - 2026-11-16 — `render_planned` in 0.84x the instructions

Performance release, the first of 3.8.x's bit-identical items. **`render_planned` retires 0.845x
of 3.8.0's instructions and the toned render 0.840x, both in about 0.80x the cycles. No output
moves**: against 3.8.0, 558,721 values, the JSON and all 638 hostile cases are identical. No public
name changes. **1,297 assertions / 26 suites.**

### How it was found

A component profile of `render_planned`'s per-sample loop, counting retired instructions per
call, put 57% of it in `svara_tract_set_formants_from_target`. Nearly all of that was the bank
rebuild computing `cos`, `sin` and `sinh` for five slots on every sample: 1,519 of a coefficient
set's 1,533 instructions. `svara_trajectory_formants_at_into` took 21%, the synthesis itself 15%,
and the toned path's `f0_at` added 627 M. The table is in `docs/benchmarks.md`.

### Changed

Each change was measured on its own against 3.8.0, and each leaves every value the same bits:

- **The formant bank memoizes each slot's trig** (-6.9% of `render_planned`'s instructions).
  Bank slot memo:
  - `cos` and `sin` are kept keyed on the slot's frequency and the sample rate, and `sinh` on its
    bandwidth and the sample rate. The keys are compared as raw bits.
  - A rebuild still discards the delay lines every sample, as Rust's does. Only the transcendentals
    are reused.
  - On the probe sequence a slot's frequency repeats on 48% of samples and its bandwidth on 35%; on
    a longer sentence, 54% and 75%.
  - The coefficient arithmetic is now written once, in `svara_formant_biquad_from_trig`, so a
    memoized coefficient and a fresh one go through the same operations.
- **`svara_tract_set_formants_from_target` writes the target straight into the tract's record**
  (-5.8%), roadmap 3.8.x's "copies into the tract's record on every sample". It validates the ten
  target values first with `svara_formant_validate_target`, which shares `svara_formant_check`
  with `svara_formant_validate`, then rebuilds without validating again. A rejected target still
  leaves the record alone.
- **`render_planned` computes per-event values once per event** (-2.0%): class, voicing, stressed
  f0, contour, nasal onset and segment spans.
- **`svara_tract_set_nasal_place` returns early for the place the tract already has** (-0.8%).
  `render_planned` asked for it on every sample of a nasalized vowel, at a `cos` and a `sin` each.
  The recorded place changes only when naad takes the notch's parameters, and a new tract's
  notch is exactly a retune to neutral, so the skip changes nothing.
- **The toned path fills each tone contour's buffers once per event** (-2.6% of the toned
  render's instructions). `svara_prosody_f0_at_filled` evaluates from filled buffers, and
  `svara_prosody_f0_at` now fills and then calls the same evaluator.
- **Left as it is:** `svara_trajectory_formants_at_into`. Its cost is ten-component Catmull-Rom
  arithmetic, with no exact shortcut. Caching its segment search would save about 1%.

### Added

- `svara_formant_biquad_from_trig`, `svara_formant_bank_coeffs`, `svara_formant_check`,
  `svara_formant_validate_target`, `svara_formant_filter_rebuild_valid`, `svara_prosody_fill`,
  `svara_prosody_f0_at_filled` and `svara_prosody_eval`.
- **`tests/allocbudget.tcyr`: the shortcuts against the long way** (11 assertions):
  - the memo's coefficients against fresh ones across eight key changes, a sample-rate change
    among them;
  - `validate_target` against `validate` on 93 hostile targets and rates;
  - a refused target leaving the record alone;
  - `f0_at_filled` against `f0_at` on all nine tones, one- and zero-point contours, t outside
    [0, 1] and NaN;
  - the notch keeping its coefficients on a retune to its own place.

  Three mutants each fail their own check: a memo ignoring the sample rate, a target validator
  skipping B5, and `f0_at_filled` mishandling one point.

### Verified

- `scripts/sweep/compare.py 3.8.0`: 558,721 values, the JSON and 638 hostile cases are identical.
  Retired instructions are equal on the four per-sample probes, 1.0004 on `phoneme /a/` (the
  memo's bookkeeping in a bank built once), and 0.8448 and 0.8401 on the two `render_planned`
  probes.
- `cyrius audit`, `cyrius fuzz`, `cyrius deny`, `scripts/check-logging.sh` both ways, the five
  examples, `cyrius distlib` current, and `cyrius deps --verify` and `--locked`.

## [3.8.0] - 2026-11-16 — a streaming sequence render

Feature release, and the first of the 3.8.x lane. It adds `svara_sequence_render_into`, the
streaming render hoosh asked for, which makes it a minor. **No existing output moves**: against
3.7.0, 558,721 values, the JSON, all 638 hostile cases and the retired instructions are identical.
**1,286 assertions / 26 suites.** The consumer floor stays cyrius 6.7.2.

### Added

- **`svara_sequence_render_into(seq, voice, ctx, sink, user)`**
  ([issue](docs/development/issues/archived/2026-11-11-a-server-render-keeps-what-each-phoneme-allocates.md),
  hoosh, fixes 2 and 3). It renders a sequence on a context the caller keeps and passes each
  sample to `sink(user, samples, count)` as soon as no later crossfade can change it. It returns
  the number of samples passed on.
  - **The plan is `svara_sequence_render`'s**: the same stress pitch, durations, anticipatory
    nasalization, crossfade lengths and sigmoid crossfade, and the same length. The samples are
    `svara_seq_crossfade` over the segments the caller's context renders, which carries glottal
    and noise state from phoneme to phoneme, so they are not `svara_sequence_render`'s.
  - **It allocates nothing** on a context that has already streamed a sequence as long. The plan
    is computed per event, never into a vec. `svara_sequence_render` keeps 63 bytes a sample on
    the same sequence, and 127 on hoosh's twelve phonemes.
  - **It refuses before the first sample** anything a later event would refuse: an invalid
    phoneme, a bad duration, a stressed f0 outside 20 to 2,000 Hz, or a phoneme longer than
    `SV_MAX_SAMPLES / 2`. Only a failure that rendering alone can find, such as a refused
    allocation, can come mid-stream.
  - **It holds back the longest fade-in in the plan**, not just the next crossfade. The issue's
    bound, "a fade-in reaches back at most `min(cf_prev, len)` samples", is true of each
    fade-in. But one can reach back past the previous phoneme into the one before, whenever the
    0.05 s transition window (2,205 samples at 44.1 kHz) is longer than a short consonant.
    Holding back only the next crossfade changed 2,383 samples of the test sentence after the
    sink had them, measured with that variant in a scratch copy.
- **Plan helpers**, each now shared with `svara_sequence_render` so the two cannot drift:
  - `svara_seq_stress_f0` and `svara_seq_event_duration`;
  - `svara_seq_is_consonant` and `svara_seq_in_cluster`, the cluster flag for one event;
  - `svara_seq_min_crossfade` and `svara_seq_crossfade_len`;
  - `svara_voice_copy_into`, `svara_nasalization_for_nasal_into` and `svara_nasal_place_for`.
- The stream's own steps, `svara_seq_stream`, `svara_seq_xf_room` and `svara_seq_xf_segment`, and
  four context fields: its voice, nasalization and pending output.
- **`tests/stream.tcyr` (35)**:
  - the per-event plan against the vec forms;
  - the stream bit for bit against `svara_seq_crossfade` over a second context's segments: a
    sentence with stress, a three-consonant cluster, nasals, a diphthong, an affricate and a
    pause, at two voices and rates; a 40-event run; a single event; an event too short for one
    sample; and the reach-back case;
  - the length against `svara_sequence_render`;
  - every up-front refusal arriving before any sample.

  With the naive hold-back, 5 of its bit-for-bit checks fail.
- `tests/allocbudget.tcyr` pins the zero bytes, `tests/logging.tcyr` makes the stream its fifth
  instrumented entry point with balanced spans on success and error, and `tests/concurrency.tcyr`
  streams from four threads at once (400 streams, all matching one streamed alone).

### Changed

- `svara_sequence_render`, `svara_seq_durations`, `svara_seq_detect_clusters`,
  `svara_seq_voice_clone` and `svara_nasalization_for_nasal` now call the shared helpers above.
  Their output is unchanged.
- hoosh's issue is resolved and archived. The integration guide's *Real-time streaming* section
  shows the stream.

### Verified

- `scripts/sweep/compare.py 3.7.0`: nothing moved. 558,721 values, the JSON and 638 hostile cases
  are identical, and retired instructions are equal on every probe.
- `cyrius audit`, `cyrius fuzz`, `cyrius deny`, `scripts/check-logging.sh` both ways, the five
  examples, `cyrius distlib` current, and `cyrius deps --verify` and `--locked`.

## [3.7.0] - 2026-11-16 — `rust-old/` is retired, and its bar kept

The Rust oracle leaves the tree.
[ADR-0011](docs/adr/0011-retire-rust-old-keep-its-bar.md) records why it was safe and how
to reach the Rust now. **No `src/` change**: `build/svara` is byte-identical to 3.6.5, and
`dist/svara.cyr` differs only in its version line. **1,239 assertions / 25 suites.** A
consumer needs to change nothing, and the floor stays cyrius 6.7.2.

### Removed

- **`rust-old/`**: 36 tracked files, 8,785 lines of Rust, and about 445 MB of ignored
  build output. The Rust stays readable at tag **`3.6.5`**, the last that carries it:
  `git show 3.6.5:rust-old/src/<module>.rs`. ADR-0011 gives the recipe to rebuild and run
  it.
- `.gitignore`'s `rust-old/target/` rule.

### Changed

- **The live instructions point at the tag.** CLAUDE.md's rules and principles,
  CONTRIBUTING's *Reading the Rust oracle*, the README, the getting-started and testing
  guides, `examples/README.md`, the two benchmark documents (whose *Reproduce* step now
  extracts the tag) and a CI comment. The bar is unchanged: match what the Rust did,
  diverge only with an ADR, and take an expectation from the oracle, never from svara's
  own output.
- **The `rust-old/src/x.rs:NN` citations in `src/`, `tests/` and the ADRs stay** as the
  audit trail. All 74 resolve at `3.6.5`.
- **ADR-0001's tag claim is corrected.** It said every tag from `1.0.0` carries
  `rust-old/`. Only `3.0.0` to `3.6.5` do. The `1.x` and `2.0.0` tags are the Rust
  releases themselves, with the source at the root, and `2.0.0:src/` is byte-identical to
  `3.6.5:rust-old/src/`.
- The roadmap drops the 3.7.0 lane. 3.8.x gains an item to bench the four Rust scenarios
  with no Cyrius bench: the tract at reduced and at minimal quality, the female /a/, and
  the 10-phoneme sequence.

### Verified

What ADR-0011's table rests on, measured on this tree with the directory gone unless
noted:

- **The gate.** Five consumers build against svara 3.6.3 at their own pins on cyrius
  6.7.18: dhvani, prani, hoosh, shabda and shabdakosh.
- **The oracle rebuilds from the tag.** `git archive 3.6.5 rust-old`, then
  `cargo test --all-features --offline --locked`: 165 unit, 45 integration and 3 doc
  tests pass.
- **Nothing lives only there.** 3.6.5 mapped every Rust `#[test]` onto the suites and
  ported the 25 that had no Cyrius assertion or a weaker one. The files found only there
  are the Rust build, its benches and its tool configs, and the tag keeps all of them.
- **Every gate passes without it:** `cyrius audit`, `cyrius fuzz`, `cyrius deny`,
  `scripts/check-logging.sh`, the five examples, `cyrius bench`, `cyrius distlib` (current
  and deterministic), and `cyrius deps --verify` and `--locked`.
- **Nothing moved.** `scripts/sweep/compare.py 3.6.5`: 558,721 values, the JSON and all
  638 hostile cases are identical, and retired instructions are equal on every probe.

## [3.6.5] - 2026-11-16 — the Rust tests left unported, and the three filed issues

Repair release. It closes the 25 Rust tests that had no Cyrius counterpart or only a weaker one,
the last check before `rust-old/` is retired (roadmap 3.7.0). It also resolves two of the three
issues consumers filed against 3.6.3 and the first fix of the third:
- concurrent renders could come back wrong;
- a finite bandwidth past `sinh`'s overflow rendered NaN;
- a warmed context allocated on every phoneme.

**Audio on valid input and the JSON are identical to 3.6.4** across 558,721 values. Seven hostile
cases change, all refusals (below). **1,239 assertions / 25 suites.** No function is removed or
changes signature, 20 are added (below), and the consumer floor stays cyrius 6.7.2.

### Fixed

- **Two threads rendering at once, each on its own context, could get each other's samples**
  ([issue](docs/development/issues/archived/2026-11-11-module-scratch-makes-concurrent-renders-unsafe.md),
  hoosh). Module-level scratch was shared between objects, so one render read formants another
  had written, and the wrong render returned success: 2 to 7 of 4,000 in the issue's repro.
  - The tract's target scratch is now each tract's own, and the glottal LF triple a stack buffer.
  - A prosody contour serializes, so it cannot hold a pointer (ADR-0002). Up to 64 points its
    interpolation buffers are on the stack. Past that, and for the three spectral buffers, the
    module-level scratch stays, under a lock per buffer.
  - The locks follow the stdlib allocator's pattern: until a thread exists they cost a load and a
    branch.
  - **One object shared by two threads at once is still the caller's to avoid.** So is
    `-D LOGGING` with concurrent renders: sakshi 2.5.10 keeps one unlocked span stack.
- **A finite bandwidth past `asinh(f64::MAX) × sr / π` passed validation and rendered NaN**
  ([issue](docs/development/issues/archived/2026-11-07-finite-bandwidth-past-sinh-overflow-renders-nan.md),
  prani). At 44.1 kHz that is every bandwidth above 9,973,280 Hz, where `sinh` in the
  coefficients overflows and every coefficient is NaN.
  [ADR-0010](docs/adr/0010-refuse-a-bandwidth-whose-coefficients-overflow.md) supersedes the
  ADR-0009 item that kept it, and `svara_formant_validate` and `svara_formant_filter_update` now
  refuse such a bandwidth.
  - The bound is the coefficient code's own: `svara_formant_bw_overflows` forms the argument
    the same way and asks the same `f64_sinh`.
  - Below `222 × sample_rate` it costs one comparison per formant, which replaced the finiteness
    call `validate` made before, so `render_planned` retires 0.17% *fewer* instructions.
- **A warmed synthesis context allocated on every phoneme**
  ([issue](docs/development/issues/archived/2026-11-11-a-server-render-keeps-what-each-phoneme-allocates.md),
  hoosh, fix 1 of 3). It kept 160 bytes per vowel, nasal or lateral, 400 per diphthong, and 816
  to 1,048 per fricative, affricate or plosive. The bump allocator never frees, so a server kept
  all of it.
  - The context now keeps its targets and two filters, re-initialized in place with
    `svara_formant_filter_rebuild`, which gives the same filter a new one would. It allocates
    **nothing** per phoneme after the first of each class.
  - `svara_synthctx_new` grows from 1,880 to 2,280 bytes.
  - The streaming sequence render the same issue asks for adds public API, so it waits for a
    minor (roadmap 3.8.x).

### Added

- **The 25 Rust test gaps, closed from the Rust.** Every `#[test]` in `rust-old/` (210, plus 3
  doctests) was mapped onto the Cyrius suites. 4 had no assertion anywhere:
  - cluster compression at render level;
  - `bandwidth_widening`'s default;
  - widening growing with f0;
  - a nasalized diphthong.

  21 had a weaker one: one click, ejective or implosive of each set where Rust renders all of
  them; 4 of 34 IPA consonants; no nasal through `render_planned`; a context that never rendered
  four of its classes; looser thresholds. Each is now in `tests/oracle.tcyr` (69 -> 120),
  `tests/render.tcyr`, `tests/spectral.tcyr` or `tests/serde.tcyr`, citing
  `git show 3.6.4:rust-old/...`.
  - Where a control goes beyond Rust's assertion, its value was read from the compiled oracle,
    not from svara. That caught one wrong expectation before it was written: Rust's cluster
    sequence is **shorter** than the one without a cluster (5,765 vs 6,701 samples), not longer.
  - **No defect was found.** One undocumented divergence was: an unstressed 0.1 s vowel at
    44.1 kHz is 3,969 samples here and 3,968 in Rust. f32 computes `0.1 × 0.9 × 44100` as
    3968.9998 and truncates down. It is recorded as ADR-0009 SQ-LEN, from the f64 core like PH-2.
- **`tests/concurrency.tcyr` (12).** The issue's repro, then each former scratch hammered from
  two or four threads and compared bit for bit with a result computed alone. Against 3.6.4's
  source all five checks fail, every time: up to 310,349 mismatches. The suite needs the stdlib
  `thread` module, now in `[deps].stdlib`; nothing in `src/` or the bundle creates a thread.
- `tests/hardening.tcyr`: the bandwidth issue's rows, and a 2,001-step sweep across the bound that
  `validate` must accept exactly where the coefficients are finite. Against 3.6.4 it gives 672
  mismatches.
- `tests/allocbudget.tcyr`: after one pass over every class through a context, the second and
  third keep 0 bytes. 3.6.4 keeps 12,616.
- **20 functions**, none removed:
  - for callers who keep their own targets: `svara_vowel_target_set`,
    `svara_vowel_target_from_vowel_into`, `svara_voice_apply_formant_scale_into`,
    `svara_phoneme_formants_into`, `svara_ph_diphthong_end_target_into` and
    `svara_ph_fricative_formants_into`. Each shares one implementation with its allocating
    form, so the two give the same bits;
  - `svara_formant_bw_overflows`, the ADR-0010 test;
  - `svara_lock_acquire` and `svara_lock_release`;
  - helpers the fixes need: `svara_ph_vt_into`, `svara_ph_vtb_into`, `svara_ph_fricative_params`,
    `svara_ph_fric_put`, `svara_prosody_interp`, the three spectral `_held` bodies, and
    `svara_synthctx_target`, `_burst_filter` and `_fric_filter`.

### Changed

- **Seven hostile cases now refuse** in `scripts/sweep/compare.py 3.6.4`:
  - four huge bandwidths given to a formant filter, which rendered NaN before;
  - three huge base f0 values. Their scaled bandwidths now trip ADR-0010 before the glottal
    source rejects the pitch, so the code is `SVARA_ERR_INVALID_FORMANT` where it was
    `SVARA_ERR_ARTICULATION`. Both refuse, as ADR-0009's BD-7 row already records for NaN and
    ±inf f0.
- Two issues move to `docs/development/issues/archived/` with their repros. The memory issue
  stays open, marked partly resolved.
- The integration guide's *Threads* section says what is safe now and what is not, and its
  *Real-time streaming* section gives the 3.6.5 figures.

### Verified

- **Old tree against new** (`scripts/sweep/compare.py 3.6.4`): 558,721 values, none moved; JSON
  byte-identical; 638 hostile cases, the seven above changed and none other (two cases at a
  3e9 Hz sample rate time out on both trees). Retired instructions are equal on five probes, and
  0.9983 and 0.9996 on the two `render_planned` probes. Run twice, with the same result.
- **Consumer floor**, re-measured with the from-scratch consumer: 6.7.2 to 6.7.18 build with no
  warning and print the same output as 3.6.4's bundle, and 6.7.1 fails at compile time.
- **Each fix's test fails without it**, run against 3.6.4's source in a scratch copy.
- **Gates:** `cyrius audit`, `cyrius fuzz`, `cyrius deny`, `scripts/check-logging.sh`, the five
  examples, `cyrius distlib` current, and `cyrius deps --verify`.

## [3.6.4] - 2026-11-16 — each dep's newest tag, and a refused naad allocation is reported

Dependency release. `hisab` 3.4.1 -> **3.5.0**, `naad` 2.3.3 -> **2.3.5**, `goonj` 2.3.1 ->
**2.3.2**. sakshi stays **2.5.10**, now a stdlib leaf rather than a package. cyrius stays
**6.7.18**, still its newest tag, with ganita 1.2.15. Each dep is at its newest tag, and they agree:
naad 2.3.5 and goonj 2.3.2 both pin hisab 3.5.0 and cyrius 6.7.18.

No public name or signature changes. **Audio on valid input, the JSON and every hostile-input
result are identical to 3.6.3.** One behaviour changes under a refused allocation, below.
**1,150 assertions / 24 suites.** The consumer floor stays cyrius 6.7.2.

### Fixed

- **A refused allocation inside naad built a glottal source that reported success.** From naad
  2.3.4, `noise_new` and `modulation_lfo_new` return `NAAD_ERR_ALLOCATION` (-6) when an allocation
  is refused, where they used to write through 0 and stop inside naad (naad's ADR-0010).
  `svara_glottal_new` stored both results unchecked, so on the new naad a refusal there returned a
  source holding -6 as a handle. Measured on that tree with gdb:
  - a refused noise allocation faulted on the first sample at `0xfffffffffffffffa`. That is away
    from the null page, where [ADR-0009](docs/adr/0009-what-the-3-6-3-audit-leaves-open.md) says
    every other refusal in svara stops;
  - a refused LFO allocation faulted at `0x2`, in `set_vibrato`;
  - `svara_glottal_restore` faulted at `0x2`.

  `svara_glottal_new` now returns `SVARA_ERR_COMPUTATION`, as `svara_tract_new` already did for its
  two biquads. Both voice wrappers and `restore` pass the code on. This never shipped: 3.6.3 pins
  naad 2.3.3, where the same refusal stops inside naad. ⚠ A consumer that moves its own naad pin
  to 2.3.4 or later while keeping svara 3.6.3 gets the fault. No consumer has that pairing on
  2026-11-16.

### Added

- **`tests/allocrefused.tcyr` (23 assertions).** It replaces the stdlib `alloc`, as naad's
  `tests/alloc_refused.tcyr` does, and refuses each of naad's six allocations in
  `svara_glottal_new`, once and from then on. It also refuses them through the two voice wrappers
  and `restore`, and refuses each biquad allocation in `svara_tract_new`. Counts are measured, not
  written down. Each loop has a floor, and a control refuses one past the last request.
  - **Without the fix**, the glottal groups report 0 of 6 refusals, and `restore` ends the run
    with SIGSEGV.
  - ⚠ It is the one test build that warns, `duplicate fn 'alloc'`, because the override is how it
    works. cyrius cannot mark the override as intended, and naad's suite has the same warning.

### Changed

- **`sakshi` is declared in `[deps].stdlib`.** hisab 3.4.2 dropped its `[deps.sakshi]` package, so
  sakshi is no longer a commit-pinned dep. It is the stdlib leaf, and `lib/sakshi.cyr` has the same
  bytes as before. svara calls it from `src/logging.cyr`, so svara declares it itself instead of
  relying on goonj's sidecar. `cyrius.lock` has 3 commit rows, down from 4.
- **The integration guide is current again.**
  - Its dependency stanza named svara 3.5.2 and hisab 2.11.2. It also told consumers to declare
    `[deps.sakshi]`, which now draws `warning: refusing to overwrite stdlib leaf 'sakshi'` on
    every build.
  - It said a sample takes 0.56 µs, which 3.5.3 corrected everywhere else.
  - It said svara allocates nothing per sample, which is true, but did not say that each phoneme
    allocates.
  - It said nothing about threads. Concurrent renders can give wrong samples, because module-level
    scratch is shared. Both of these were filed by hoosh; see the roadmap.

### Dependencies

- **hisab 3.4.1 -> 3.5.0**, through 3.4.2. 3.5.0 retires 16 names and turns 23 constants into enum
  members, and svara uses none of them. svara calls five hisab functions:
  - `num_fft`, `num_neumaier_sum` and `ease_in_out_smooth` are unchanged through their call trees;
  - `calc_monotone_cubic_checked`, and `cx_i` in a test, now allocate through `hisab_alloc`. Its
    pointer defaults to `alloc`, so nothing moves unless a caller sets hisab's allocation scope.
- **naad 2.3.3 -> 2.3.5.** 2.3.4 renames `power_spectrum` to `naad_power_spectrum`, which svara does
  not call. It also makes the three constructors svara uses return `NAAD_ERR_ALLOCATION` (see
  Fixed). Nothing svara reaches changes in 2.3.5. The `NoiseGenerator` and `Lfo` layouts, which
  svara reads through accessors, are unchanged.
- **goonj 2.3.1 -> 2.3.2.** svara calls nothing in goonj; naad's bundle needs it.
- **Size:** `build/svara` grows from 4,067,744 to 4,392,112 bytes, mostly from hisab's larger
  bundle. `dist/svara.cyr` changes only in its version line and the glottal fix.

### Verified

- **Old tree against new** (`scripts/sweep/compare.py 3.6.3`):
  - **values:** 558,721, none moved;
  - **JSON:** byte-identical, 40,729 bytes;
  - **hostile inputs:** 638 cases, no change in exit code or output. `render_planned` at a 3e9 Hz
    sample rate times out on both trees, as it did before;
  - **retired instructions:** equal on six of seven probes, and 0.9994 on the toned render.
- **The used-symbol diff.** svara references 46 dep functions and constants by name, plus the
  `NoiseGenerator` and `Lfo` accessors. Only `noise_new` and `modulation_lfo_new` changed body,
  and `filter_biquad_new` changed in its call tree. All three changes are the allocation result
  above.
- **Consumer floor**, re-measured with a from-scratch consumer: `examples/basic.cyr` built against
  `dist/svara.cyr` and the new bundles.
  - 6.7.2, 6.7.5, 6.7.9, 6.7.12, 6.7.17 and 6.7.18 build with no warning and print identical
    output;
  - 6.6.20, 6.7.0 and 6.7.1 fail at compile time, now in naad's bundle (`array size identifier
    must be an enum constant`).
- **Gates:** `cyrius audit` exit 0, `cyrius fuzz` 2,400 cases, `cyrius deny` 23 deps with 0
  violations, `scripts/check-logging.sh` both ways, examples 5/5, `cyrius distlib` current and
  deterministic, `cyrius deps --verify` and `--locked`.
- **The three filed issues** in `docs/development/issues/` were re-run against this tree, and all
  three still reproduce. They are on the roadmap.

### Upgrading

- **Move the pins together:** hisab 3.5.0, naad 2.3.5, goonj 2.3.2. Then remove any
  `[deps.sakshi]` package, and put `"sakshi"` in `[deps].stdlib` if you call it yourself.
- **If you move naad to 2.3.4 or later, move svara to 3.6.4 with it.** See Fixed.

## [3.6.3] - 2026-11-07 — the P(-1) sweep

Audit and repair release. It is the first audit since 3.1.3 and the first ever run against the
compiled Rust oracle. It found **39 defects and divergences**. **30 are repaired**: 1 P0, 4 P1, 13 P2
and 12 P3. Each repair has a test that fails without it. **8 are recorded** in
[ADR-0009](docs/adr/0009-what-the-3-6-3-audit-leaves-open.md), and **1 is upstream**, in the cyrius
compiler's `#derive(Serialize)` decoder. The
full report is [`docs/audit/2026-11-07-audit.md`](docs/audit/2026-11-07-audit.md).

**Audio on valid input does not move.** `scripts/sweep/compare.py 3.6.2` compares 558,721 values
and none moved. The JSON and hostile-input changes are named under Changed. **1,127 assertions / 23
suites.** The toolchain stays cyrius 6.7.18 with the same deps, and the consumer floor stays 6.7.2.

### How it was audited

- **Six cluster audits against the oracle.** Each reader took a cluster of modules, read it line by
  line against `git show 3.5.2:rust-old/src/…`, and reproduced every finding with a probe on both
  sides, using the oracle built in a scratch copy. A finding without a reproduction was dropped.
- **A real fuzz target.** `tests/svara.fcyr` replaces the scaffold stub with 15 groups over every
  public entry, forked batches, and a replay mode. It started at 175 failures in 2,400 cases and
  ends at 0. It passes `cyrius fuzz`, `--poison` and `--bounds`, and 24,000 cases run in 11 s.
  Each JSON case also damages 8 documents byte by byte and decodes only what
  `svara_validate_json` accepts. With the check removed, the run fails.
- **Fault injection.** A scratch build refused the Nth allocation, once or from then on, across 14
  pipelines, each run hashed against a clean one. Before the fixes, 8 of 3,050 cases gave a silent
  wrong answer. On the release tree: 2,056 cases, **0 silent, 0 hangs**, and every one of the 1,840
  faults is at the null page (`si_addr` ≤ 0x10). There are fewer cases because the optimizations
  removed per-sample allocations, which left fewer allocations to refuse.
- **An all-zero sweep.** All 290 public functions outside logging were called with every argument
  0, which is what a lenient decoder returns for a missing field: 105 return, 184 fault at the null
  page (≤ 0xb0), 1 exits by design, and 0 hang.
- **Execution coverage** with `cyrius coverage --exec`, and **mutation proofs**: every fix's test
  was run against the reverted code and had to fail.
- **An independent review** of the merged diff, by a reader who saw none of the work. It found
  two more defects in 3.6.2 and one gap, and it caught a P0 in this release's own work before it
  shipped. A new peak scratch in `estimate_formants` was capped by bin count, so a valid
  2²⁶+1-bin spectrum from `analyze` ended the process. It is now sized by the most peaks a
  spectrum can have.

### Fixed

- **P0. A batch renderer extended from its own saved state never finished.** `svara_batch_save`
  stored the live queue, so `svara_batch_extend(r, svara_batch_save(r))` walked a vec it was
  appending to, until the capacity overflowed past 2 GiB.
- **P1. A restored context, pool or batch renderer did not resume.** The states dropped the glottal
  and tract state, while the header said they held nothing between calls. Every voiced phoneme
  after a restore differed from the original. They now resume bit-exactly.
- **P1. Retargeting a tract left stale upper formants.** After a six-to-eight-formant
  `svara_tract_set_formants`, `set_vowel` and `set_formants_from_target` replaced only the first
  five, and the bank kept ringing at the rest.
- **P1. `svara_spectrum_estimate_formants` picked a different formant set on magnitude ties.** Its
  selection sort was unstable and Rust's is stable. It is now a stable merge sort.
- **P1. Objects kept pointers the caller still owned.** These were sequence push, batch extend,
  save and restore, and render output save and restore. Two restores from one state shared a queue.
  Each now copies at the boundary, as Rust does by moving or cloning.
- **P2. `svara_formant_bank_update` wrote through an unbounded slot index.** It now returns
  `SVARA_ERR_INVALID_FORMANT` outside `[0, 8)`.
- **P2. Parallel vecs of different lengths aborted.** `svara_trajectory_plan` and
  `svara_pool_render_batch` now return `SVARA_ERR_INVALID_DURATION`. Before, the plan ignored extra
  durations when it did not abort.
- **P2. Every `*_restore` faulted on a null state**, which is what a decoder returns for a malformed
  document. Each now refuses a null state, a missing vec field and an out-of-range enum.
- **P2. A rejected target was written into the tract's record**, so save captured formants the
  filter did not have, and restore then refused the saved state.
- **P2. `estimate_formants` was O(n²).** A 2²⁰-sample frame took 23,424 ms where Rust takes 6 ms.
  It now takes 45 ms and returns the same formants.
- **P2. Spectral queries leaked scratch on every call**, 8 KB per `svara_spectral_rms_level`, and
  returned 0.0 once the arena ran out. The scratch is now grow-only.
- **P2. A refused allocation gave a plausible answer** in 8 fault-injection cases: the spectral
  queries returned 0.0 or an empty result, and `svara_prosody_f0_at` returned 1.0. Each now ends the
  process.
- **P2. The progress callback's return value was treated as an error code.** In Rust the callback
  returns `()`, so a callback that happened to return a negative value stopped the render.
- **P2. Concatenated output past `SV_MAX_SAMPLES` aborted in `vec_push`.** It now returns
  `SVARA_ERR_COMPUTATION`.
- **P2. An invalid phoneme id rendered as something.** Id -1 rendered as a vowel, and id 101 counted
  as voiced.
- **P2. `svara_formant_filter_update` took an infinite bandwidth or a NaN frequency**, and every
  sample after it was NaN. ADR-0007's refusal now covers it too.
- **P2. `svara_batch_restore` took out-of-range event enums.** Stress 7 restored and rendered as
  unstressed. An event's phoneme, stress and tone are now checked.
- **P3.** The fixes in this group are:
  - The nasal place is recorded only once naad accepts it.
  - An infinite bandwidth is refused, as ADR-0007 already said.
  - Out-of-range enums are refused by restore and ignored by the setters. Before, glottal model 7
    acted as LF and diplophonia phase 2 disabled it.
  - `svara_prosody_make` copies its points.
  - A one-keypoint `formants_at` no longer returns the plan's own target.
  - Contour points sort stably.
  - `SVARA_LOG=OFF` no longer turns logging on.
  - `svara_ph_apply_envelope` guards a negative length.
  - The spectral header and ADR-0002 no longer say `Spectrum` does not serialize.
  - A restored render output, and a saved batch, no longer say in their comments that they share
    the caller's vec.
  - `tests/bridge.tcyr`'s range check passed a NaN, so its range assertions could not see one. It
    now fails on NaN, and the suite asserts that it does.
  - One envelope assertion could not fail. It looked at the wrong slot, and now checks all ten.

### Changed

- **New failure returns.** All of these used to crash, hang or give a wrong answer.
  - `svara_formant_bank_update` returns an error code, and `svara_formant_filter_update` refuses a
    non-finite formant.
  - Two entries refuse mismatched parallel vecs.
  - Restores refuse more, including a queued event's out-of-range phoneme, stress or tone.
  - Synthesis refuses an invalid phoneme id or an infinite bandwidth.
  - Concatenation past the sample cap returns an error.
- **The context, pool and batch states gain `glottal` and `tract` fields**, each a vec of one state.
  These are the 6 JSON lines that differ from 3.6.2. A 3.6.2 document has neither field and still
  restores, starting the voice fresh as before. A vec longer than one is refused.
- **Two hostile results move.** An infinite base f0 now returns `SVARA_ERR_INVALID_FORMANT` (-1)
  where 3.6.2 returned `SVARA_ERR_ARTICULATION` (-5), and an infinite bandwidth is now refused.
- **A refused allocation in a spectral query or `f0_at` ends the process.** It writes
  `svara: allocation refused` to stderr and exits with status 1, the fail-stop that naad and goonj
  already use.
- **The progress callback's return value is ignored.**
- **`SVARA_LOG` level names are case-insensitive.**

### Refactored and optimized

Everything here is bit-identical: `compare.py --old-tree` was run against a snapshot taken before
the work began.

- **Diphthongs stop allocating per sample.** A context diphthong goes from 3,881,120 to 353,200
  bytes per second, and a nasalized one from 4,931,424 to 1,403,480.
- **`svara_formant_filter_rebuild` zeroes only the unused slots.** `render_planned` retires 2.8%
  fewer instructions and a toned render 2.35% fewer, with cycles flat.
- **Duplicated bodies are gone.** `interpolate`, `to_formants`, `formants_at` and
  `catmull_rom_vowel` now call their `_into` forms, `svara_smooth_new` calls `svara_smooth_with_time`,
  and `svara_glottal_apply_rd` uses a module scratch.
- **`tests/svara.bcyr`, the scaffold benchmark stub, is deleted.** `benches/` holds the real
  benchmarks.
- ⚠ **Net, `render_planned` is slower than 3.6.2**, at 3.1% more instructions, and a toned render
  2.6% more, with cycles about the same. The stale-formant fix copies each sample's target into
  the tract's record after validating it, which costs 4.3 points. That was measured against a
  variant with the old write-in-place path. Every other probe is within 0.01%. Validating before
  writing in place would recover it, and it is in roadmap 3.8.x.

### Added

- **[ADR-0009](docs/adr/0009-what-the-3-6-3-audit-leaves-open.md)** records what stays open:
  - four divergences that shipped without an ADR, including hisab's shape-preserving cubic, up to
    10.3% from Rust after stress;
  - the diphthong glide, whose energy has been 7× to 69× Rust's since 3.1.0, now in roadmap 3.9.x;
  - the policy for refused allocations;
  - why decoding a document directly stays outside the contract.
- **`svara_validate_json(doc)`** returns `SVARA_ERR_NONE` for a document svara's writer could have
  produced, and `SVARA_ERR_COMPUTATION` for anything else. It refuses a string value, an escape,
  a bracket that closes the wrong kind, whitespace inside the object, nesting past 16 levels, a
  number longer than 32 bytes, and trailing content. Call it before decoding text svara did not
  write (see Found upstream). It allocates nothing.
- **[`docs/audit/2026-11-07-audit.md`](docs/audit/2026-11-07-audit.md)**, the audit report.
- **`tests/integration.tcyr`** ports all 16 of the Rust integration tests, 10 of which were
  missing, with 26 assertions.
- **`tests/hardening.tcyr`** and **`tests/serde.tcyr`** pin every repair. The serde suite checks
  mid-utterance resume bit-exactly and checks that a literal 3.6.2 document still restores.
- **`scripts/sweep/compare.py --old-tree DIR`** compares against a directory instead of a git ref.

### Found upstream

- **The cyrius 6.7.18 derive decoder loops forever on a vec of structs holding a stray `}`.**
  `<Name>_from_json_str` never returns on `[}]` in a `Vec<struct>` field. Five svara state companions have such a field,
  and so do `SvPhonemeSequence` and `SvProsodyContour`. The hang comes before `restore` can
  validate. The same decoder reads past the end of an unterminated key, which ghurni filed the same
  day. Filed in the local cyrius repo as
  `docs/development/issues/2026-11-07-derive-serialize-vec-struct-element-loop-hangs-on-stray-brace.md`.
  The repro exits 3 on all four toolchains tried, from 6.6.0 to 6.7.18. bayan is not involved.
  **svara guards both with `svara_validate_json`.** In a scratch probe, 70,000 byte-damaged
  documents of the seven affected types gave 63,821 refusals, and the 6,179 accepted ones all
  decoded with no hang or fault. Without the check, 303 of 7,000 hung.

### Upgrading

- **Check the return of `svara_formant_bank_update` and `svara_formant_filter_update`** if you call
  them directly. The second now refuses a non-finite frequency or bandwidth.
- **A saved batch whose events hold an out-of-range phoneme, stress or tone no longer restores.**
- **A refused allocation in a spectral query or `f0_at` exits the process with status 1.** Before,
  it returned a value that looked valid.
- **Expect `SVARA_ERR_INVALID_DURATION`** from `svara_trajectory_plan` and
  `svara_pool_render_batch` when the parallel vecs differ in length, and
  **`SVARA_ERR_INVALID_PHONEME`** for a phoneme id outside the inventory.
- **A pushed or extended event is a copy.** Changing it after the push no longer changes the
  sequence or the batch. The same holds for prosody points, saved states and render outputs.
- **Check untrusted JSON with `svara_validate_json` before decoding it,** then go through a state
  companion and `restore`. Without the check, a damaged document can hang the decoder. The check
  refuses valid JSON svara does not write, such as a pretty-printed document. Decoding a sequence
  or contour directly is still not validated (ADR-0009).

### Verified

- `cyrius test tests`: 1,127 assertions in 23 suites. `cyrius audit` exits 0. `cyrius fuzz`,
  `--poison` and `--bounds`: 2,400 cases each, 0 failures.
- `cyrius coverage --exec`: 273 of 307 functions executed, up from 255 of 290. `cyrius deny`,
  `distlib --check`, `deps --verify` and `--locked`, the logging gate and all 5 examples are clean.
- `compare.py 3.6.2`: 558,721 values, 0 moved. 6 JSON lines differ and 2 hostile results changed,
  all named above.
- Fault injection on the release tree: 2,056 cases, 0 silent wrong answers, 0 hangs, every fault at
  the null page.

## [3.6.2] - 2026-11-07 — the roadmap holds only what is left

Cleanup release. **No code change:** every `src/` edit is a comment, and `build/svara` is
byte-identical to 3.6.1. **941 assertions / 22 suites**; `cyrius audit`, `cyrius deny`, the logging
gate and `cyrius distlib --check` are all clean.

### Changed

- **The roadmap holds only what is left**, 464 lines down to 223. The shipped table and lane
  write-ups are gone, because the CHANGELOG has every release and their lessons live in `state.md`,
  the ADRs and `docs/guides/testing.md`. Remaining work is now 3.6.3 (the P(-1) sweep), 3.7.0
  (retire `rust-old/`), 3.8.x (performance), 3.9.x (the preserved Rust quirks) and 3.x.x (SIMD and
  upstream blockers). The v1.0 lane dissolves into those: its security audit is a 3.6.3 deliverable.
- **The f32 P0 is closed** by [ADR-0008](docs/adr/0008-the-synthesis-core-stays-f64.md). It was blocked
  on naad, which measured 0% on `filter_biquad_process_sample` and closed its own P0. svara could
  not gain alone, because its filters and noise stay f64 in naad.
- **The `rust-old/` gate is measured, not assumed.** It read "dhvani has not been updated to the
  Cyrius svara", which stopped being true. In a scratch copy on cyrius 6.7.18, dhvani builds against
  svara 3.6.1 and passes 65 of 65 suites. Whether that clears the gate is left as a decision.
- **A fourth preserved quirk is recorded.** Plosive timing depends on the render path:
  `svara_ph_synth_plosive` uses the VOT fractions and `svara_synthctx_plosive` a fixed n/3 and n/6.
  Both match the oracle.
- **SIMD stops waiting on "a later 6.5.x".** naad measured 3.3× from `f64v256_*` on cyrius 6.7.18, so
  the lane now says to measure again.

### Added

- **`scripts/sweep/compare.py`** compares a git ref with the working tree, each built on its own
  toolchain pin: exact values, JSON bytes, 638 hostile cases, and retired instructions. It is the
  harness behind the last three releases, kept for 3.6.3 and every later bump.

### Fixed

- **Test builds are warning-free.** `state.md` claimed it and 18 warnings said otherwise: six
  suites compiled a `#derive(Serialize)` module without bayan, and `tests/pool.tcyr` discarded two
  `#must_use` results, which it now asserts on.
- **Comments and docs that had stopped being true.** "serde dropped" in ten modules, when 22 types
  derive it since 3.3.0. Pointers to the `M-serde`, `M-perf` and `M-log` milestones, and to the lane
  that used to be numbered 3.6.0. `state.md`'s consumers, deferred work and serde count.

### Found, not fixed

- ⚠ **`tests/svara.fcyr` is the `cyrius port` scaffold stub.** It calls no svara function, so
  `cyrius fuzz` has passed vacuously in CI since 3.3.1, and `dependency-watch.md` claimed otherwise.
  The claim is corrected here. A real target is the first audit item in 3.6.3, because it may find
  defects, and this release changes no code.

### Verified

- `build/svara` is byte-identical to 3.6.1.
- `scripts/sweep/compare.py 3.6.1` moves no value, no JSON byte and no hostile outcome. The same tool
  against 3.5.6 reports exactly the 165 contour values 3.6.0 changed, so it can see a change.
- Every release the old table listed has a CHANGELOG entry, and each lesson in the removed write-ups
  was found elsewhere first. The two that were not, on static tables and `Quality`, moved to `state.md`.

## [3.6.1] - 2026-11-07 — cyrius 6.7.18, the newest deps, and a toned render stops allocating

Toolchain and dependency release. `cyrius` 6.7.9 -> **6.7.18**, `hisab` 3.4.0 -> **3.4.1**,
`naad` 2.3.0 -> **2.3.3**, `goonj` 2.1.0 -> **2.3.1**, `sakshi` 2.5.8 -> **2.5.10** (transitive);
ganita stays 1.2.15. Each is its project's newest tag, and they agree: hisab, goonj and naad each
pin exactly 6.7.18, and sakshi 2.5.10 is what hisab declares and the toolchain bundles.

No API change and no source change in `src/`. Audio, JSON and every hostile-input result are
identical to 3.6.0, and `dist/svara.cyr` differs only in its version line. The consumer floor stays
cyrius 6.7.2. **939 assertions / 22 suites**; `cyrius audit`, `cyrius fuzz`, `cyrius deny`, the
logging gate and `cyrius distlib --check` are all clean.

### Changed

- **A toned render stops allocating per sample.** hisab 3.4.1 keeps
  `calc_monotone_cubic_checked`'s scratch on its stack up to 16 points, and every tone contour has
  at most 5. A toned `render_planned` falls from 115 to 35 bytes per sample, the same as an untoned
  one, and `svara_prosody_f0_at` from 80 bytes a call to 0. A contour past 16 points still
  allocates, at 464 bytes a call for 20. This closes the roadmap's "blocked on hisab" item.
- **`tests/allocbudget.tcyr` pins that exactly.** Its control "the tone path costs more than the
  untoned one" became false, so it is replaced: a toned render must equal an untoned one, 16 points
  must allocate nothing, and 17 must allocate, which proves the measure can see a cost. On 3.6.0
  the first two fail, at 115 against 35 and at 36,800 bytes against 0.
- **`cyrius tests` is gone, and the docs now say `cyrius test tests`.** cyrius removed the verb at
  6.7.12. CI never called it: `cyrius audit` runs the suites itself.
- **`lib/` and `cyrius.lock` are regenerated from clean.** `lib/simd.cyr` joins `lib/`, pulled in by
  goonj. 33 of the 36 files are byte-identical to the 6.7.18 snapshot and the other three to their
  tag's `dist/`, and `cyrius deps --locked` confirms it.
- **Build size.** hisab's bundle grows from 2.64 MB to 3.66 MB, and `build/svara` from 2.94 MB to
  4.05 MB.

### Verified

- **Value sweep, 3.6.0 on 6.7.9 against this tree on 6.7.18.** All 558,721 values are bit-identical,
  including the 2,625 crafted contours 3.6.0 changed.
- **JSON, new at this release.** Every earlier sweep compared samples and scalars and never the
  serialized form, and bayan moves from 1.5.13 to 1.11.1 across these toolchains. All 22
  `#derive(Serialize)` types were dumped, parsed and dumped again, with NaN, +/-Inf, -0, denormals,
  1/3 and f64::MAX in the fields. The output is byte-identical.
- **Hostile inputs, one process per case.** 638 cases give no change in exit code or output. Neither
  tree aborts, and `render_planned` at a 3e9 Hz sample rate times out on both.
- **What reaches svara.** Of the 18 dep symbols svara uses, three changed in body:
  `calc_monotone_cubic_checked` above, `num_fft` (a guard for a negative length) and naad's
  `noise_new` (a refactor). The `NoiseGenerator` and `Lfo` layouts svara reaches into are unchanged.
- **Instruction counts are the same.** A first timing pass showed `tract synthesize_into` and
  `phoneme /a/` about 4.5% slower with disjoint ranges, on a busy machine whose load later reached
  22 on 16 cores, so each path was re-measured with hardware counters. Retired instructions are
  0.9985 to 1.0000 of 3.6.0, and 0.980 on the toned path.
- **Cycles, 15 interleaved rounds on one core:** geomean 0.996. `tract process_sample` is +2.8% and
  `synthesize_into` +1.9%, the rest flat or lower, and `render_planned` -3.0%. Instructions are
  equal and the old source on the new compiler shows no such gap (0.994), so this reads as code
  layout from the larger hisab bundle, not codegen. It was not isolated further.
- **Consumer floor re-measured** on eight toolchains: 6.7.2 through 6.7.18 build and run, and 6.7.1
  and older fail at compile time.
- These checks ran on x86_64 only, which is also all CI covers.

## [3.6.0] - 2026-11-07 — cyrius 6.7.9, and a contour at 0.0 stops reading as 1.0

Toolchain and dependency release. `cyrius` 6.6.20 -> **6.7.9**, `hisab` 3.3.7 -> **3.4.0**,
`naad` 2.2.5 -> **2.3.0**, `goonj` 2.0.6 -> **2.1.0**, `sakshi` 2.5.7 -> **2.5.8** (transitive),
with ganita 1.2.15 from the stdlib snapshot. hisab 3.4.0, goonj 2.1.0 and naad 2.3.0 each pin
6.7.9 exactly, and the next tag of each needs 6.7.18. It is a minor because the floor for
consumers moves: **they now need cyrius 6.7.2 or later.**

No API change. Synthesis output is bit-identical to 3.5.6, and one contour result changes,
below. **937 assertions / 22 suites**; `cyrius audit`, `cyrius fuzz`, `cyrius deny`, the logging
gate and `cyrius distlib --check` are all clean.

### Fixed

- **`svara_prosody_f0_at` read a contour at exactly 0.0 as 1.0.** `calc_monotone_cubic` returned
  0 for a refusal, and 0 is also a legal answer, so svara took every 0 for a refusal. Rust's
  `.unwrap_or(1.0)` keeps `Some(0.0)`, so this was a parity bug carried since the port. It is
  reachable through caller-built contours and through `svara_prosody_insert_f0_boost` with a
  boost of 0. Renders are not affected: their contours come from `svara_tone_to_contour` and
  stay above 0. A zero f0 is refused by `svara_f0_in_range` and ignored, as in Rust, so it
  cannot reach a division.

### Changed

- **`calc_monotone_cubic` is gone from hisab, and svara moved to
  `calc_monotone_cubic_checked`.** It is the one of hisab 3.4.0's 27 retired names that svara
  calls, and the retired name fails the build loudly. The new form separates a refusal from an
  answer, which is what fixes the bug above. `num_fft`, `num_neumaier_sum` and
  `ease_in_out_smooth` are byte-identical between hisab 3.3.7 and 3.4.0.
- **`lib/` and `cyrius.lock` are regenerated from clean.** 32 of the 35 files are byte-identical
  to the 6.7.9 snapshot and the other three to their tag's `dist/`, and the new
  `cyrius deps --locked` confirms it. `naad` and `goonj` move only their version lines.
- **sakshi is 2.5.8, not the newest tag that pins 6.7.9.** 2.5.9 and 2.5.10 pin it too, but 2.5.8
  is what hisab 3.4.0 declares and what the toolchain bundles.
- **Build size.** hisab's bundle grows from 2.10 MB to 2.64 MB, and `build/svara` from 2.60 MB to
  2.94 MB. `dist/svara.cyr` does not carry hisab, so it barely moves.

### Upgrading

- ⚠ **Consumers need cyrius 6.7.2 or later.** sakshi 2.5.8 needs `const`, and goonj pulls sakshi
  in. Measured with a from-scratch consumer of this bundle: 6.7.2, 6.7.5, 6.7.8 and 6.7.9 build
  and run; 6.6.20, 6.7.0 and 6.7.1 fail at compile time (`undefined variable 'SK_OUT_STDERR'`).
  The failure is loud, not silent. Every repo pinning svara today is below the floor.
- **Move the pins together:** cyrius, hisab 3.4.0, naad 2.3.0 and goonj 2.1.0. A consumer that
  calls one of hisab's 27 retired names itself gets a build error.
- A contour at 0.0 now reads as 0.0. A caller that relied on it reading 1.0 will see the change.

### Verified

- **Value sweep, 3.5.6 on 6.6.20 against this tree on 6.7.9.** The same 556,096 values across all
  101 phonemes, the context and pool paths, glottal models, quality levels, sequences, spectra,
  prosody, trajectories, bridges and the RNG are **bit-identical**. That sweep moved 451,192
  values last release, so it can see a change.
- **Contours, where the code changed.** Of 2,625 crafted values (0, 0.5, 1, 2 and -1 at three
  points), 165 differ, every one 1.0 -> 0.0 as predicted, and none in any other way.
- **Hostile inputs, one process per case.** 11 values through 58 entry-point and parameter
  combinations, 638 cases, including contours and boosts. No exit code, length or error code
  changed, and the only differences are the two zero cases above. Neither tree aborted. One case
  times out on both: `render_planned` at a 3e9 Hz sample rate.
- **The new tests can fail.** Restoring the zero check fails 2 assertions, and dropping the error
  check fails 3.
- **The language changes do not reach svara's source:** no lone `!` (a no-op until 6.7.3), no `~`,
  no unary `-` on a constant, and no reserved word as a name.
- **Benchmarks are flat**, three interleaved runs on the same machine, minimum of each: geomean
  0.996, with `render_planned toned` 15,466 -> 15,460 us and every range overlapping.
- These checks ran on x86_64 only, which is also all CI covers.

## [3.5.6] - 2026-11-07 — cyrius 6.6.20, and the matched dep set

Toolchain and dependency release. `cyrius` 6.6.3 -> **6.6.20**, `hisab` 2.22.1 ->
**3.3.7**, `naad` 2.2.4 -> **2.2.5**, `goonj` 2.0.4 -> **2.0.6**, `sakshi` 2.5.1 ->
**2.5.7** (transitive), with ganita 1.2.14 from the stdlib snapshot. This is the matched
set for 6.6.20: hisab 3.3.7, goonj 2.0.6 and naad 2.2.5 each pin it exactly, sakshi 2.5.7
is the copy the toolchain bundles, and the next tag of hisab, goonj and naad needs cyrius
6.7.x.

No API change. **927 assertions / 22 suites**; `cyrius audit`, `cyrius fuzz`, `cyrius deny`,
the logging gate and `cyrius distlib --check` are all clean.

### Changed

- **`num_fft` moved to `Result`, the one hisab call that needed it.** hisab 3.0.0 made it
  return a `Result`, and `if (num_fft(data, fft_size) != 0)` still compiled, because a
  `Result` in comparison position degrades to its tag. It kept working only because `Ok`'s
  tag is 0. It is now `var t, e = num_fft(...)` plus `is_err_result(t)`, and `result` joins
  `[deps] stdlib`. The other three hisab fns svara calls return plain values and are
  `public` under hisab 3.3's `private` bundle, so nothing else moved.
- **91 fns gained a doc comment, 69 in `src/` and 22 in `tests/`.** cyrdoc used to count
  `#must_use` as documentation (`src/voice.cyr`: 0 undocumented on 6.6.3, 11 on 6.6.20), and
  the audit's docs leg now walks `tests/` and `benches/` too. `cyrius audit` failed on this,
  and CI runs it.
- **`cyrius.lock` is regenerated from clean.** `cyrius deps` keeps old commit rows beside the
  new ones, which is how the lock came to say naad 2.2.2 while `lib/` held 2.2.4. It now has
  exactly four commit rows and `cyrius deps --verify` passes all 35 files.
- **`lib/` equals a clean resolve at the pin.** `callback.cyr` goes; `alloc_cx.cyr` and
  `poison.cyr` arrive. 32 of the 35 files are byte-identical to the 6.6.20 snapshot, and the
  other three are byte-identical to their tag's `dist/`.
- **`dist/svara.deps` shrinks from 17 leaves to 1 (`bayan`)**, and `dist/svara.cyr` gains a
  `# Requires` block. This is `cyrius distlib` >= 6.6.18 behaviour. A consumer no longer
  includes `bayan` or `hashmap` by hand to get the JSON codecs.
- **`CYRIUS_NO_WARN_SHADOW_LIB` is gone from `scripts/check-logging.sh`.** It covered a
  transitive sakshi that lagged the toolchain's. Both are 2.5.7 now, byte-identical, and the
  advisory no longer fires.
- **Samples move by up to 4.3e-14 of peak.** Most sample values differ in their last bits.
  The likely source is the toolchain's transcendentals: cyrius 6.6.9 took x86 sin and cos off
  x87 `fsin` / `fcos` for fdlibm ports. The cause was not isolated further.
- **Comments and ADR-0001 stop describing the old `f64_to`.** Since cyrius 6.6.8 it saturates
  to the signed range. ADR-0001 gains an amendment: its rule stands, because a negative stays
  negative and `INT64_MAX` walks past a floor-only guard.

### Upgrading

- **Move the pins together:** cyrius 6.6.20, hisab 3.3.7, naad 2.2.5 and goonj 2.0.6. naad
  2.2.5 needs hisab 3.x, so svara's bundle cannot sit beside the older set.
- Golden values taken from an earlier svara may need a tolerance rather than bit equality.

### Verified

- **Value sweep, old tree on 6.6.3 against new tree on 6.6.20.** 556,096 values across all 101
  phonemes in three voices, the context and pool paths, the four glottal models, the three
  quality levels, sequences, spectra, prosody, trajectories, all 19 bridges and the RNG. 451,192
  differ, none by more than 4.3e-14 of peak, with no sign, NaN, Inf or length change. Prosody,
  trajectory, bridge and RNG output is bit-identical.
- **Hostile inputs, one process per case.** naad and goonj each found aborts here that their
  suites stayed green over, so svara was checked directly: 11 values (NaN, +/-Inf, f64::MAX,
  2^63, 1e19, 3e9, the smallest denormal and more) through 56 entry-point and parameter
  combinations, 616 cases. No exit code, length, error code or non-finite count changed, and raw
  floats moved by at most 1.5e-14 relative.
  Neither tree aborted. One case times out on both: `render_planned` at a 3e9 Hz sample rate.
- **Benchmarks are flat**, same machine: `tract synthesize_into 1024` 259.0 -> 260.6 us,
  `/a/` 598.5 -> 592.9 us, `render_planned` 14.0 -> 13.3 ms.
- **A from-scratch consumer** declaring only hisab, goonj, naad and a minimal stdlib builds and
  runs `dist/svara.cyr`, including a JSON codec.
- 6.6.6's struct copy rules miss svara, as the roadmap predicted. Benchmarks and sweeps ran on
  x86_64 only, which is also all CI covers.

## [3.5.5] - Unblocked by cyrius 6.6.3

### Also — the `#inline` markers that never inlined

The M-log gate (`scripts/check-logging.sh`) requires svara to build with **no** `warning:`
at all. cyrius 6.5.63 made `#inline` a real directive *and* gave it a diagnostic for the
cases it cannot honour, so the first warning-free build on 6.6.3 surfaced 516 warning lines:
**12 from svara's own `src/`** (rng, smooth, formant, glottal, tract) and **464 inherited
from the vendored `lib/naad.cyr` bundle**. Every one of them said the same thing — the fn's
body exceeds 32 tokens, or takes more than 2 parameters, or contains control flow, so the
marker did nothing.

svara's 12 are removed here. The 464 are fixed upstream in **naad 2.2.4**, which dropped 117
dead markers of its own; the `[deps.naad]` pin moves 2.2.2 → **2.2.4** to pick that up.

⭐ v6.5.63 recorded this exact trap against this exact repo: svara carried four `#inline`
markers and `src/lod.cyr` recorded measuring one at "+0.7% -- noise" — a measurement of an
optimisation that was not there. The compiler can finally say so.

`CYRIUS_NO_WARN_SHADOW_LIB=1` is exported by the gate for ONE advisory it cannot act on:
`lib/sakshi.cyr` arrives transitively through goonj, whose lock pins sakshi 2.5.1 while the
6.6.3 toolchain bundles 2.5.2 — and the two files are byte-identical apart from the version
stamp on line 2. `cyrius deps` reverts any local edit because the resolver owns that file,
so the only real fix is a goonj release. The gate still fails on every compiler warning.

Toolchain-pin release. `cyrius` 6.6.2 -> **6.6.3**; no source change.

6.6.2 could not compile this project. `#inline` became a real directive at cyrius
v6.5.63, which taught the compiler's PASS 2 to arm the directive — but PASS 1, the
declaration-collection scan, was never taught to CONSUME the token. An unconsumed
directive there falls through to the catchall and **terminates the scan**, so every
declaration after the first `#inline` went unregistered and the next `#derive`d
struct reached the parser as an unknown top-level token. The error named the struct,
which was innocent, hundreds of lines from the real trigger.

cyrius 6.6.3 closes both halves in all seven per-target compiler forks. Re-vendored
and rebuilt against it; tests green.

## [Unreleased]

Nothing yet.

## [3.5.4] - 2026-08-31 — every sample rate below 7500 Hz killed the process

Patch: **two memory-safety repairs**, same family — a dependency's return value
believed rather than checked. No API change, and no behaviour change at any
sample rate that already worked. **927 assertions / 22 suites**, `cyrius audit`
exit 0.

Between them these two defects meant **every sample rate in (1000, 7500] aborted
the process**, through `svara_tract_new` reporting success and handing back a
tract that could not survive its own next call. Both are now checkable failures,
and the band above 1200 Hz — which never worked — now renders.

### Fixed — CRITICAL: `svara_tract_set_formants_from_target` ran off the end of a fallback bank

`svara_vowel_target_to_formants_into` writes **five** formants unconditionally and
documents that precondition — *"`out` must already hold >= 5 SvFormant pointers"*
(`src/formant.cyr`). `svara_tract_set_formants_from_target` violated it from
inside svara:

Every vowel target carries **F5 = 3750 Hz**, so at or below **7500 Hz** that
formant sits at or above nyquist and `svara_formant_filter_new` refuses the
five-formant schwa. `svara_tract_new` then takes its documented fallback —
*"absolute fallback: single 500 Hz formant"* — and stores that **one-element**
vec as the tract's formant record, **returning the tract as valid**. The next
`set_formants_from_target` wrote five formants into one slot:

```
svara_tract_new(4000.0)                      -> ok, svara_is_err == 0
svara_tract_set_formants_from_target(t, tgt) -> "vec: index out of bounds", process aborts
```

Not a code any caller could check — the constructor had already reported success,
so there was nothing to test. Every sample rate in **(1000, 7500]** was affected,
which is the whole low-rate band including 2000, 4000 and 6000.

The fix builds a fresh five-formant vec when the tract's own record is short, and
lets `svara_tract_set_formants` → `svara_formant_filter_rebuild` validate it
against the sample rate. `rebuild` validates **before** it mutates, so the call
now returns `SVARA_ERR_INVALID_FORMANT` and leaves the fallback bank intact and
usable — a checkable failure instead of a crash.

`SVARA_VOWEL_FORMANT_COUNT` names the invariant that was violated, and
`svara_formant_scratch_vec` now builds against it instead of a bare `5`.

**Found by prani**, whose `crvoice_vocalize` aborted for every sample rate in
that band — reached through `crtract_new`, which correctly checks
`svara_tract_new`'s return and had no way to see this coming.

### Note — vendored `lib/` re-synced

`lib/` was stale against this project's own `cyrius = "6.5.36"` pin, so the
toolchain refreshed six files (`io.cyr` and the four `syscalls_*`) on the first
build of this release. Not part of the repair and not hand-edited — any build
would have picked it up. The largest item is a stdlib `getenv` fix: the environ
block was read into a fixed 8 KB buffer with a single read, so any variable past
byte 8192 was invisible and indistinguishable from unset.

`tests/hardening.tcyr` grows a group that **aborts the process on 3.5.3** rather
than failing, in that suite's established style, with the 7500/7501 boundary
pinned and controls at 7501 and 44100 so a narrowing or widening of svara's own
range is caught.

### Fixed — CRITICAL: `svara_tract_new` stored two unchecked naad error codes as filter pointers

Found while verifying the repair above: below 7500 Hz the abort simply moved.

`svara_tract_new` builds two **fixed-frequency** biquads — a 250 Hz nasal notch
and a 600 Hz subglottal bandpass — and stored both `filter_biquad_new` returns
without checking them. naad rejects a centre frequency at or above nyquist
(`validate_frequency`), so each carries an implied minimum rate: **> 500 Hz** for
the notch and **> 1200 Hz** for the bandpass. Below those, a negative naad error
code was written straight in as the filter pointer, and the first render
dereferenced it.

```
svara_tract_new(1001.0)   -> ok, svara_is_err == 0   (subglottal filter is -2)
  ... first process_sample -> dereferences -2, process dies
```

Both returns are now checked and mapped through `svara_map_naad_error`, so a rate
that cannot carry svara's own fixed filters is refused up front rather than
producing a tract that is a husk. A tract that cannot build its own filters is
not a usable tract.

`tests/hardening.tcyr` pins the 1200/1201 boundary and asserts the 1201 Hz tract
actually renders a finite sample, so "refused" cannot quietly become "refuses
everything".

## [3.5.3] - 2026-08-26 — documentation sweep, and a performance figure that was wrong by 1.6×

**Docs only; no source change.** Every document swept against the tree rather
than against the last document that said it. The roadmap is now what is *still to
do*, with the `rust-old/` lane explicitly **parked on dhvani**.

### Fixed — svara is ~64× real-time, not ~40×

⭐ **The headline performance figure has been wrong since 3.0.0, and it
understated svara.** Every doc said "a full glottal → formant → tract chain ≈
0.56 µs/sample, roughly 40× real-time", computed by adding three benchmark rows:
glottal + formant + tract.

**`tract process_sample` already contains the formant bank** — the tract calls
`svara_formant_filter_process_sample` (`src/tract.cyr`). Summing them
double-counts it.

The chain is measured *directly* by `tract synthesize_into 1024`, which is
glottal → tract per sample. The arithmetic settles it:

| | ns/sample |
|---|---|
| glottal `next_sample` | 77 |
| tract `process_sample` (contains the formant bank) | 275 |
| **sum, no double count** | **352** |
| **`tract synthesize_into`, measured end to end** | **351.5** |
| the old summed figure (glottal + formant + tract) | 508 |

Agreement to 0.5 ns. The correct figures are **≈ 0.35 µs/sample** and **≈ 64×
real-time at 44.1 kHz on one core**. Corrected in `README.md`,
`docs/benchmarks.md`, `docs/architecture/overview.md`, `docs/development/state.md`
and ADR-0003's parenthetical, each now carrying the warning that the per-unit
rows must not be added.

### Changed — `SECURITY.md` was materially misleading

It listed **"1.x: Yes"** and nothing else — inherited from the Rust crate, and
still saying so three releases after the Cyrius port shipped, while **3.0.0–3.1.2
carry five defects reachable through the public API**.

Rewritten per-release, with those five named and given reproducers (SIGSEGV,
three process aborts, a silent NaN), the later hardening in 3.2.0 / 3.3.0 / 3.5.2,
what svara explicitly does *not* defend against (unbounded `duration`; `SvRng` is
not cryptographic), and the supply-chain position. This is goonj's precedent: a
supported-versions table that hides known-vulnerable releases is worse than none.

### Changed — the roadmap is future-facing, and `rust-old/` is parked

The open lanes had accumulated ✅ items until they read as history. They now
contain only what is left:

- **3.6.0 — hot-path memory and SIMD.** Work available now (pool the per-note
  buffers). SIMD items wait on Cyrius codegen that does not round-trip each
  `f64v4` op through memory — **expected in a later 6.5.x**, so they are marked
  to revisit rather than attempt.
- **3.7.0 — retire `rust-old/`** — ⛔ **parked on dhvani.** Everything svara can
  do is done: the serde contract transcribed, the five oracle test gaps closed,
  the five examples ported, the third quirk annotated, the NaN parity question
  resolved, the recovery incantation verified. **The oracle stays until a
  downstream consumer builds against `dist/svara.cyr`, and dhvani has not been
  updated to the Cyrius svara yet.** That work is dhvani's, on dhvani's schedule;
  naad's roadmap names svara as *its* consumer-green gate, so the same exercise
  unblocks both.
- **v1.0 — hardening.** All three items' prerequisites are now met.

The shipped table is sorted ascending again (it had drifted as rows were
prepended) and the header says plainly which lanes are open, which is parked, and
why.

### Changed — the rest of the sweep

- **`docs/guides/testing.md`** listed five coverage gaps that 3.5.1 closed. They
  are now recorded as *closed*, with the generalisable lesson kept: **a suite that
  ports another suite inherits its blind spots** — the Rust's tests were the
  source for svara's, so anything the Rust under-tested, svara under-tested
  identically, and every module suite still looked healthy. It also now carries
  the two `cyrius audit` coverage gaps.
- **`docs/architecture/overview.md`** said "16 `.cyr` modules" and omitted
  `logging` entirely (added 3.4.0). Now 17 + the smoke entry, with the row.
- **`docs/guides/getting-started.md`** told a newcomer to "edit `src/main.cyr`",
  which is the smoke entry, not where features go. Rewritten with the real
  layout, the `[lib] modules` step, the distlib regeneration step, and the two
  things that actually bite (the never-freeing allocator; error codes rather than
  `Result`, with two different failure shapes).
- **`sakshi` promoted from transitive to direct** in `dependency-watch.md` and
  `README.md` — since 3.4.0 `dist/svara.deps` names it, because the bundle
  carries `src/logging.cyr`.
- **Consumer tag refreshed** to 3.5.2 in `README.md` and the integration guide;
  benchmark table refreshed to current numbers with per-release deltas dropped
  (`benches/history.csv` is the honest place for those); `CLAUDE.md`'s oracle
  principle now says to cite a **tag**, not a bare path.

### Notes

- **Every relative link in every Markdown file was checked** and resolves.
- Assertion, module, suite and bench counts re-derived from the tree rather than
  copied forward. `docs/guides/testing.md` now says outright that counts drift
  and `cyrius audit` prints the live figure.
- `dist/svara.cyr` regenerated at v3.5.3 (no source change; the version stamp).

## [3.5.2] - 2026-08-26 — the oracle's preserve-first gate

Everything roadmap 3.6.0 requires to exist **before** `rust-old/` can be deleted,
except consumer-green — which is not svara's to close. The theme is that each
item is a thing the Rust still holds that nothing in the Cyrius tree does.

Suite **909 → 917 assertions**; five runnable examples where there were none.

### Added — `examples/`, five programs, ported from `rust-old/examples/`

svara had **no examples directory at all**; `docs/examples/` held a `.gitkeep`
while `CLAUDE.md` advertised "runnable examples". goonj set the same gate before
its own oracle retirement.

| Example | Shows |
|---|---|
| `basic.cyr` | a male /a/, a three-phoneme sequence, sample statistics |
| `voice_comparison.cyr` | male / female / child presets, plus a breathy voice via the chained builders |
| `error_handling.cyr` | every failure shape — negative `SVARA_ERR_*`, and the 0-pointer form |
| `prosody_patterns.cyr` | four intonation patterns and nine lexical tones as f0 contours |
| `streaming.cyr` | **the zero-allocation real-time path** |

`streaming.cyr` is the one that carries a warning rather than a demo: Cyrius's
bump allocator never frees, so an allocation inside an audio callback is not a
leak that grows slowly — it is one that ends the process. It allocates its block
once, uses the `_into` forms, and checks `svara_alloc_samples` for 0.

**CI builds and runs all five on every push**, so they cannot rot against the
API — which matters more than usual, since they have to keep working after the
oracle goes.

### Changed — NaN formant parameters are now rejected ([ADR-0007](docs/adr/0007-reject-nan-formant-parameters.md))

⭐ **A deliberate divergence from the oracle, and the last exception to
ADR-0001's own rule.** `rust-old/src/formant.rs:479-486` is
`f.frequency <= 0.0 || f.frequency >= nyquist` — both comparisons are **false**
for NaN, so Rust accepts a NaN formant frequency, `cos(NaN)` makes every
coefficient NaN, and the filter emits an entire buffer of NaN having returned
`Ok`. The port reproduced that faithfully and three releases of hardening walked
past it, each recording it as a known residual.

`svara_formant_validate` now tests positively, so NaN and `±inf` fall through to
the reject — the same shape as `svara_f0_in_range`. It also closes the path
through `svara_tract_set_formants_from_target`, so a NaN `VowelTarget` is
refused rather than silently poisoning a tract.

This is svara's **third** documented divergence, all the same kind: an
overflowing sample count errors where Rust saturates and aborts;
`svara_tract_new` errors where Rust panics; a NaN formant is refused where Rust
accepts it. Each replaces a silent or fatal outcome with a checkable code.

Six new assertions in `tests/hardening.tcyr`, three of them controls proving the
ordinary range checks (0 Hz, at/above Nyquist, a normal formant) are unchanged.
All ten render-path digests are still identical to 3.1.2 — no valid input moved.

### Added — the third v2.0.1 quirk is annotated where it lives

Two of the three preserved quirks had call-site annotations
(`set_speed_quotient` ignored by the Rosenberg pulse; per-vowel spectral tilt
computed but not applied). The third — **`tone` is honoured only by
`svara_sequence_render_planned`**, so setting a tone and calling
`svara_sequence_render` produces the same audio as setting none — was described
once in `state.md` and annotated nowhere in code. It is now at
`SVARA_TONE_NONE`'s definition, where someone reading the field will find it.

### Added — how to read the oracle after it is gone

ADR-0001 and `CONTRIBUTING.md` now carry the recovery incantation, following
goonj's ADR-0002:

```sh
git show 3.5.1:rust-old/src/formant.rs
```

**Verified**, not asserted: every tag from `1.0.0` to `3.5.1` carries the full
tree, and that command reproduces lines 479-486 exactly as ADR-0001 and
`src/formant.cyr` cite them. The standing instruction from here on is to **cite a
tag, not a bare path** — a bare `rust-old/src/x.rs:NN` in a comment becomes
unresolvable the day the directory goes; `git show <tag>:…` does not.

### Notes

- **ADR numbering collision caught before it shipped.** The new ADR was drafted
  as 0003, which 3.3.1 had already used for the re-homed source-filter ADR.
  Renumbered to 0007; `docs/adr/README.md` never renumbers, per its own rule.
- What remains before `rust-old/` can go: **consumer-green** (dhvani or vansh
  building against `dist/svara.cyr` — both siblings hold on the same gate), a
  final parity sweep, verifying the tree is green with the directory moved aside,
  and the reference sweep.
- `dist/svara.cyr` regenerated at v3.5.2.

## [3.5.1] - 2026-08-26 — the five scenarios the Rust asserted and the port did not

**Test-only; no source change.** An audit of `rust-old/`'s 213 tests against the
port's suites found five behaviours the Rust asserted and Cyrius asserted
nowhere. They are closed now, **while the oracle is still readable** — after
`rust-old/` goes (roadmap 3.6.0) the expectations could only be transcribed from
svara's own output, which would freeze any existing bug in as "correct". That is
naad 2.2.1's lesson, learned there while closing 182 untested public functions.

**No defects found.** 69 new assertions, all green on the first run. That is the
useful result, not a disappointing one: code that had never been exercised now
has evidence behind it.

Suite **840 → 909 assertions** across 22 suites.

### Added — `tests/oracle.tcyr`

Every expected value is **re-derived from the Rust source and cited by line**,
never taken from running svara.

| Gap | Why it mattered | Oracle |
|---|---|---|
| **Quality levels** | ⭐ `svara_tract_set_quality` appeared in **no suite or bench**. Every quality-conditional branch in `svara_tract_process_sample` ran Full-only under test — an entire feature with no evidence behind it. | `integration.rs:593-645` |
| **Jitter / shimmer** | The glottal suite sets both to **0** to keep its goldens deterministic, so nothing asserted that non-zero perturbation perturbs. | `integration.rs:111-138` |
| **Breathiness** | `svara_glottal_set_breathiness` was referenced in no suite at all. | `glottal.rs:609-615` |
| **Spectral energy at F1** | `tests/spectral.tcyr` analyses synthetic sines only. Nothing fed **synthesized speech** to the analyzer, so the one cross-module acoustic check was absent. | `integration.rs:40-63` |
| **Seven bridge maps** | `vibrato_depth_from_valence`, `intonation_from_emotion`, `f0_range_scale_from_arousal`, `f0_peak_from_prominence`, `jitter_from_age`, `spectral_tilt_from_distance`, `lombard_f0_shift` — tested on **neither** side of the port. | `bridge.rs`, per-function |

The Goertzel estimator is ported from the Rust test helper
(`integration.rs:655-677`) rather than reimplemented, so the acoustic check uses
the oracle's own estimator instead of a different one that might disagree.

Each group carries a control that would fail if the assertion were vacuous: two
**Full** tracts must agree *exactly* (or "Full ≠ Minimal" would pass for any two
independently-driven tracts); with jitter and shimmer **off** the same periods
must differ far less; the Goertzel estimator must read **0** on silence; and
`jitter_from_age` must be U-shaped in both directions around its minimum.

### Fixed — `cyrius audit` does not lint tests or benches

⭐ **`cyrius audit` prints `scope: src`.** Its fmt, lint and docs legs cover
`src/` only — `tests/*.tcyr` and `benches/*.bcyr` are *compiled* by the tests leg
but never linted. So an over-long line or an untracked deferral in a test file
passed the entire gate.

Found the honest way: the new suite had **five lines over 120 characters** and
`cyrius audit` still reported *lint clean*. CI now lints `tests/` and `benches/`
explicitly — `cyrfmt --check`, zero warnings, and zero untracked deferrals.

This is the second gap of its kind in three releases: 3.4.0 found that
`cyrius test` does not forward `-D`, so audit only ever compiled the logging-OFF
half. **`cyrius audit` exiting 0 is necessary, not sufficient**; what it covers
is worth re-checking rather than assumed.

- `dist/svara.cyr` regenerated at v3.5.1.

## [3.5.0] - 2026-08-26 — the redundant delay line, and a roadmap premise that was wrong

**M-perf, the bit-identical half.** The formant bank kept its input delay in
eight per-slot buffers when one scalar pair would do. Removing the redundancy is
worth **−6.0%** on the per-sample formant path and **−4.2%** on the block path,
with **bit-identical output** and no new dependency.

For scale: that is about what the AVX2 prototype bought before it was reverted in
3.1.0 — from the lever the roadmap predicted would work, because it attacks
memory traffic rather than arithmetic.

### Changed — the input delay line is two scalars, not sixteen slots

All eight biquads in the bank are driven by the **same** input sample, so
`x1[i] = input` and `x2[i] = previous input` held identical values in every slot.
Sixteen bytes of state doing the work of two, and 32 memory operations per sample
doing the work of three.

⭐ **Measured before changing anything**, not inferred from reading: over 4,096
samples of real signal, **28,672 cross-slot comparisons found zero divergence**
in `x1` or `x2` — while `y1`, which is genuinely per-slot, differed as expected.
That control is what makes the zero meaningful rather than a check on a buffer of
zeros.

`svara_formant_bank_process` now reads the shared delay once before the loop and
advances it once after. Same arithmetic, same order — the ten render-path digests
are still identical to 3.1.2, and all 840 assertions pass unchanged.

| benchmark | before | after | |
|---|---|---|---|
| formant filter `process_sample` | 168 ns | **158 ns** | **−6.0%** |
| formant filter `process_block` 1024 | 180.4 µs | **172.7 µs** | **−4.2%** |
| tract `process_sample` (Full) | 286 ns | **279 ns** | **−2.4%** |

Minimum of three interleaved runs of each binary, so machine drift hits both
sides equally. A single non-interleaved run had `glottal next_sample` moving
79 → 121 ns on code this release does not touch, which is why the numbers above
are not from one.

Also drops **128 bytes of state per bank** (two 8-slot f64 buffers).

⚠ `SvFormantBank_x1` / `_x2` are gone, replaced by `_xin1` / `_xin2` scalars.
They were derived accessors on an internal delay line; nothing outside
`formant.cyr` read them.

### Fixed — the roadmap asked for a hoist that had nothing to hoist

M-perf listed *"tract per-sample chain — hoist coefficient recompute out of the
per-sample loop"*. There is no coefficient recompute in
`svara_tract_process_sample`: the naad biquads are configured when formants are
set, and the only per-sample state work is the two `SmoothedParam` advances,
which must happen every sample by definition. The item is corrected in the
roadmap rather than left to send someone looking.

### Notes — one change measured, rejected, and recorded

The four quality predicates the tract calls once per sample (`use_interaction`,
`use_nasal_coupling`, `use_subglottal`, `use_lip_radiation`) are one or two
integer comparisons each, so marking them `#inline` looked free. It moved
`tract process_sample` by **+0.7%** — noise, in the wrong direction — over the
same interleaved three-run protocol.

Reverted. The source carries no pragma that does nothing, and `src/lod.cyr` now
records the negative result so nobody re-tries it without measuring. The call
overhead is not what that loop spends its time on.

### Still open in M-perf

Unchanged and now the whole of the remaining lane: pooling the per-note render
buffers (unblocked), and three items blocked outside this repo — a stdlib
`vec_with_capacity`, a non-allocating hisab `calc_monotone_cubic`, and the SIMD
work waiting on Cyrius codegen that does not round-trip every `f64v4` op through
memory.

- `dist/svara.cyr` regenerated at v3.5.0.

## [3.4.0] - 2026-08-26 — structured logging through sakshi

**M-log.** svara emitted no diagnostics at all — errors were integer codes and
nothing else. It now routes tracing through **sakshi**, the AGNOS structured
logging substrate, so its spans nest inside a consumer's and correlate across
the stack. This also completes the sakshi routing varna's own `logging.cyr`
explicitly deferred, approximating it with a level-gated stderr writer.

**Off by default, and compiled out entirely when off** — no log call, no runtime
level check, no call frame. Suite **828 → 840 assertions** (21 suites), plus
**58** more that only exist in the logging build.

```sh
cyrius build -D LOGGING src/main.cyr build/svara
SVARA_LOG=debug ./build/svara
```

### Added — `src/logging.cyr`

`svara_log_init()` reads `SVARA_LOG` (`off`/`fatal`/`error`/`warn`/`info`/
`debug`/`trace`, default `info`); an unrecognised value falls back to `info`
rather than silencing a program that asked for logs. Level-gated
`svara_log_error`/`_warn`/`_info`/`_debug`/`_trace`, structured `svara_log_kv`,
and `svara_log_err`, which binds a `SVARA_ERR_*` to the active span via
`sakshi_err_at_span` — the span depth becomes the error's context, so a consumer
can tell *which nesting level* failed.

Instrumented: **four coarse entry points** — `svara_sequence_render`,
`svara_batch_render_all`, `svara_synthctx_synthesize`, `svara_pool_render`.
**Never per-sample**: a span costs two `clock_gettime` calls, which at 44.1 kHz
would dominate the ~0.56 µs it takes to synthesize a sample.

### Added — token-based spans, because every entry point has early returns

⭐ **A plain `span_enter` / `span_exit` pair would leak a span on every error
return**, and sakshi's stack is 16 deep — so a leak per failed render would
slowly poison the log rather than fail loudly. `svara_synthctx_synthesize` alone
has seven returns.

`svara_span_enter` returns the depth to unwind *to*, and `svara_span_leave(token)`
unwinds to it — correct even if an inner span leaked, and harmless called twice.

Asserted, not assumed: **40 consecutive failing renders leave the span stack at
depth 0**, which is well past the 16 that would overflow it. Each error exit is
covered separately, including the pre-span argument rejections that correctly
open no span at all.

### Fixed — `SVARA_LOG=off` now means off, spans included

⚠ **sakshi does not level-gate span events.** `_sk_emit_span` writes regardless
of `sakshi_set_level`, because a span is structural rather than a log line. That
is defensible in sakshi and wrong here: setting `off` still streamed every
ENTER/EXIT pair. svara gates spans itself.

Asserted with the ring buffer: a whole `sequence_render` at level `off` emits
**zero** events, and the *same* render at `debug` emits some — so the silence is
the level, not a broken sink.

### Added — `scripts/check-logging.sh`, and CI runs it

⚠ **`cyrius test` does not forward `-D`.** So `cyrius audit` only ever compiles
the logging-OFF half, and without this script the entire `#ifdef LOGGING` half of
the codebase — the module and the guarded blocks through four entry points —
**would be built by nobody**.

The script builds and runs both ways, requires both builds to emit no `warning:`
at all, and asserts structurally that **no log or span call appears on the
per-sample path** (`svara_glottal_next_sample`, `svara_tract_process_sample`,
`svara_formant_bank_process`, …). That last check was verified to discriminate by
injecting a `svara_log_trace` into `svara_formant_bank_process` and confirming it
fails.

### Notes

- **`tests/logging.tcyr` is useful in both modes.** With logging off it asserts
  the four instrumented entry points still behave exactly as before — which is
  what proves the `#ifdef` blocks threaded through their return paths changed
  nothing. Benchmarks agree: every row is within noise of 3.3.2, and the ten
  render-path digests are still identical to 3.1.2.
- ⚠ **`dist/svara.deps` now names sakshi**, because the bundle carries
  `src/logging.cyr`. This is **not a new dependency** — sakshi already arrived
  transitively through hisab, which svara's bundle requires — but it is now
  explicit. `docs/development/integration-guide.md` shows the stanza.
- **The lint rule I documented in 3.3.1 caught this release's own module**: the
  header used the word "deferred" without a tracking pointer on the same line,
  and `cyrlint` flagged it. Fixed by citing the roadmap where the word appears.
- `dist/svara.cyr` regenerated at v3.4.0 (5,728 lines).

## [3.3.2] - 2026-08-26 — glottal restore is exact; a 3.3.0 limitation was self-inflicted

⭐ **3.3.0 shipped a documented limitation that was not real.** `SvGlottalState`
said naad's `NoiseGenerator` cell and `Lfo` phase "are internal to naad and
cannot be read out", so a restored source restarted its aspiration and vibrato
streams — and `tests/serde.tcyr` *asserted that divergence*, which made a wrong
belief look like a measured fact.

naad puts `#derive(accessors)` on both structs, and svara links against naad's
bundle in **one flat namespace**. The state was reachable the whole time:
`NoiseGenerator_rng` and `Lfo_rng` return pointers to 8-byte PRNG cells, and
`Lfo_phase` / `Lfo_sh_value` are ordinary fields. Verified by probe before
changing anything — two generators given the same cell produce 64 identical
samples.

**What a foreign type lacks is a codec, not accessors.** That is the
generalisable lesson, and it is now recorded in
[ADR-0002](docs/adr/0002-serialization-boundary.md).

### Fixed — `svara_glottal_restore` is now exact in every configuration svara builds

`SvGlottalState` gains four fields — `noise_cell`, `lfo_phase`, `lfo_sh_value`,
`lfo_rng_cell` — captured in `save` and written back in `restore` *after*
`svara_glottal_set_vibrato`, which would otherwise leave the LFO at its
construction phase.

The test group flipped from asserting divergence to asserting identity: a
restored source now matches **exactly over 512 samples with breathiness at 0.5
and vibrato at 5 Hz**, where 3.3.0 asserted it must differ. A control confirms a
*fresh* source does not match, so the capture is doing the work rather than the
comparison being vacuous.

⚠ **Complete for the generators svara constructs** — a WHITE `NoiseGenerator`,
whose entire state is that one cell, and a SINE `Lfo`. Pink noise keeps an octave
**buffer** (`NoiseGenerator_pink_octaves`, a pointer) that this does not carry.
If svara ever selects a non-white noise type, this must grow. State held in a
buffer rather than a scalar is the boundary that genuinely remains.

### Changed — corrections to what 3.3.0 and its docs claimed

The 3.3.0 CHANGELOG entry is corrected **in place** rather than quietly
superseded, because it stated the limitation as a measured finding. `ADR-0002`,
`state.md` and `tests/serde.tcyr`'s header are amended the same way.

**No naad change was needed.** The accessors already existed; nothing upstream
had to move.

### Notes

- The one restore limit that *is* real remains: **filter delay lines are not
  state**, and `tests/serde.tcyr` still asserts a warm tract and its restore
  differ. That matches the oracle — Rust's `set_formants` is
  `self.filter = FormantFilter::new(..)?`, which discards them on every call.
- Suite 826 → **828 assertions**, 20 suites.
- `dist/svara.cyr` regenerated at v3.3.2.

## [3.3.1] - 2026-08-26 — documentation, CI, and the claims that had gone false

**No source change.** A sweep of everything the repo asserts about itself. The
common thread: `cyrlint` reports *0 untracked deferrals* and always has, which is
accurate and nearly meaningless — it scans only `.cyr` sources, never Markdown,
matches twelve literal terms, and counts one "tracked" if the **same line**
contains `CHANGELOG`, `roadmap`, `docs/`, `issue`, `See `, `v6.` or `v5.`. That
is a substring test, not a check that the roadmap says anything. Everything below
was invisible to it.

### Fixed — CI ran almost none of the gate it documents

⭐ **`.github/workflows/ci.yml` had four steps: install, `cyrius deps`,
`cyrius build`, `cyrius test`.** No fmt, lint, docs, fuzz, deny, bench or
`cyrius audit` ran on any push or PR, while `CONTRIBUTING.md` names `cyrius audit`
as what a PR must pass and `state.md` documents eight gates. `release.yml`
inherited the same thin gate, so a release could ship having run only the tests.

That was defensible while `cyrius audit` was broken — it skipped dependency
resolution before compiling its test and bench legs and failed with spurious
*"undefined variable F64_ONE"* on any project with git-dep bundles. It was fixed
in 6.5.35 and svara has been on that pin since 3.1.2.

CI now runs `cyrius audit`, `cyrius fuzz` and `cyrius deny`, plus two new checks:

- **Toolchain matches the pin**, asserted *before* `cyrius deps` — the original
  failure mode surfaced two steps later as an opaque path error. `cyrius version`
  must equal the manifest pin and `~/.cyrius/versions/<pin>/lib` must exist.
  (hisab 2.11.2 added the same step after hitting exactly this.)
- **`dist/` is regenerable and current.** The bundle is what consumers actually
  compile and it is checked in, so it can drift from `src/` silently. CI
  regenerates and fails if the tree is dirty.

Both new steps were verified locally before being written, including that
`cyrius distlib` output is deterministic.

### Fixed — four documents that never left Rust

`README.md` and `CONTRIBUTING.md` were swept for Cyrius earlier; these four were
not, and still described a language this project does not use.

- **`docs/development/threat-model.md`** — the important one, and a prerequisite
  for the v1.0 security audit. It claimed `cargo deny` / `cargo audit` /
  "`deny.toml` restricts to crates.io only (no git dependencies)" for a project
  whose dependencies are now **entirely** git, and it asserted parameter
  validation mitigations that **3.1.3 disproved by finding five defects reachable
  through the public API**. Rewritten around what actually changed the model:
  Cyrius has no bounds checking on raw memory, `alloc` returns 0 rather than
  aborting, `f64_to` does not saturate, there are no unsigned types, and there is
  no panic. Resource exhaustion is now the sharpest section, because the bump
  allocator never frees.
- **`docs/architecture/overview.md`** — a `math` module that does not exist,
  "coefficients stored as f32 for processing" (the port is f64 throughout),
  compiler auto-vectorization (Cyrius emits scalar code), a `no_std` section, and
  "~5,000× real-time" against a measured **~40×**.
- **`docs/development/integration-guide.md`** — `use svara::prelude::*`,
  `vec![0.0f32; 512]`, and a Cargo feature table for features this library does
  not have. Rewritten around what a consumer actually needs: the manifest stanza,
  the fact that **svara's transitive dependencies resolve from the CONSUMER's
  manifest**, the error-code convention, and the zero-allocation streaming path.
- **`docs/guides/testing.md`** — `cargo test --all-features`, `make check`,
  criterion HTML reports. Rewritten around the real commands, plus the four rules
  a new assertion here has to follow, each traced to a defect that motivated it.

### Changed — the four pre-port ADRs are ADRs again

`docs/architecture/adr-001-source-filter-model.md` … `adr-004-scope-boundaries.md`
sat outside the scheme `docs/adr/README.md` states, were absent from its index,
and still referenced `bridge.rs`. They are **decisions**, so they moved to
`docs/adr/` as **0003–0006** and are indexed. Their substance is domain-level and
survives the port untouched; only Rust-specific wording was corrected.

`docs/architecture/README.md` had meanwhile declared itself *"Empty"* while
sitting beside all four.

### Changed — `.gitignore` no longer hides the benchmark history

⭐ **`*.csv` shadowed `benches/history.csv`**, so the file `docs/benchmarks.md`
describes as existing "so regressions are visible across commits" could never be
committed. 3.1.3 deliberately left this as "a call about committing a data file";
the call is made — the history is tracked.

Three more Rust-era rules retired with it, each with the reason recorded in the
file: `Cargo.lock` (`rust-old/Cargo.lock` is part of the frozen oracle and **is**
tracked, so the rule only created confusion), `**/*.rs.bk` (no rustfmt here) and
`/coverage/` (no coverage tooling here).

### Fixed — claims that had gone false

- **`CLAUDE.md`'s mission statement was still the scaffold placeholder** —
  `_TODO: one-or-two-sentence mission statement…_` — 3 releases and five months
  after the port. Written.
- **The bridge map count disagreed with itself**: README said 19, `state.md` and
  the CHANGELOG said 18. Counted — `src/bridge.cyr` defines **19** public
  `svara_bridge_*` functions. README was right; `state.md` is corrected.
- **`docs/benchmarks.md` listed 11 benchmark rows** where `benches/hotpath.bcyr`
  defines **13** — the two `render_planned` benches from 3.1.3 were missing. Added,
  with a note that they are the two rows a timer measures badly.
- **`docs/benchmarks-rust-v-cyrius.md`** was headed "v3.0.1 / cycc 6.4.11" and
  still called SIMD **P0** where the roadmap marks it deferred. It is now framed
  as the dated head-to-head it is — the figures stand as taken — with the
  priorities corrected: control-rate glides shipped, per-sample allocation
  (never on the list) mattered more than SIMD, and SIMD bought ~5% and was
  reverted.
- **README's consumer example pinned `tag = "3.1.2"`**, and its ADR link pointed
  at a file that just moved.
- **`docs/guides/getting-started.md`** sent new tests to `tests/svara.tcyr`, the
  smoke suite, rather than the per-module suite for whatever was touched.
- **The README / CONTRIBUTING Cyrius sweep shipped unrecorded** — it landed in a
  commit with no CHANGELOG entry, and `state.md` went on warning that both files
  "are still the pre-port Rust files" until 3.1.3. Recorded here.

### Notes

- `docs/guides/testing.md` now records **five coverage gaps** rather than leaving
  them implicit — `svara_tract_set_quality` appears in no suite or bench at all,
  so every quality-conditional branch runs Full-only under test. All five are
  scoped in the roadmap to be closed while `rust-old/` is still readable.
- The roadmap lane for this work was numbered 3.1.4 and was overtaken by 3.2.0 and
  3.3.0 shipping first; renumbered.
## [3.3.0] - 2026-08-26 — state companions: everything svara owns now round-trips

The follow-on ADR-0002 scoped. Nine engine types could not be derived — each
holds a nested struct pointer, a raw buffer, or a foreign naad handle — so each
now has a **flat state companion** the derive *can* reach, plus `save` / `restore`.
No hand-written codecs.

Derived types **13 → 22**; serde suite **54 → 135 assertions**; whole suite
**745 → 826** across 20 suites. `cyrius audit` exits 0, the 3.1.3 allocation
budget is intact, and every render path is still bit-for-bit identical to 3.1.2.

### Added — eight `save` / `restore` pairs and one direct derive

| Engine type | Companion | What restore rebuilds |
|---|---|---|
| `SvSpectrum` | *none needed* | already flat — `Vec<f64>` plus two scalars; it only wanted the annotations |
| `SvRenderOutput` | `SvRenderOutputState` | the nested `progress` pointer, inlined as three counters |
| `SvGlottalSource` | `SvGlottalState` | the naad noise generator + LFO; the Rng resumes exactly |
| `SvVocalTract` | `SvVocalTractState` | the formant bank and both naad biquads, from the recorded formants |
| `SvFormantKeypoint` | `SvKeypointState` | the nested `SvVowelTarget`, inlined as ten components |
| `SvTrajectoryPlanner` | `SvTrajectoryState` | the whole keypoint list, as `Vec<SvKeypointState>` |
| `SvSynthCtx` | `SvSynthCtxState` | the context, from the voice + the noise PRNG |
| `SvSynthesisPool` | `SvSynthesisPoolState` | as above, plus the two diagnostic counters |
| `SvBatchRenderer` | `SvBatchRendererState` | the event queue and the context |

⭐ **The context carries far less state than its field list suggests, and finding
that out is what made three of these small.** `svara_synthctx_synthesize` calls
`svara_tract_reset` and re-applies *every* glottal parameter from the supplied
voice at the top of each call — so the tract and glottal source hold nothing
across calls, and the buffer is scratch. What actually persists is the noise
PRNG and the sample rate. `SvSynthCtxState` is three fields.

**Restore takes the voice as an argument** for the context, pool and renderer. A
context is derived *from* a voice, and `SvVoiceProfile` already serializes on its
own; inlining its nine fields into three more structs would duplicate a schema
that then has to be kept in step.

### Changed — the tract now records its own formants and nasal place

Two pieces of information the tract used to throw away, both needed by restore
and both worth having anyway:

- **`formants`** — a built biquad bank holds coefficients, from which frequency
  and bandwidth cannot be recovered. The 3.1.3 `scratch_formants` vec is now that
  record as well as the scratch: `set_formants` syncs it after a successful
  rebuild. The sync copies **values**, is a no-op self-copy on the
  `set_formants_from_target` path, and allocates only when the formant *count*
  changes — so the per-sample allocation budget is unchanged, and
  `tests/allocbudget.tcyr` still passes.
- **`nasal_place`** — `set_nasal_place` used to retune the naad notch and forget
  which place it was. A tract could not report its own configuration. It is now
  stored and readable via `svara_tract_nasal_place`.

⚠ `SvVocalTract_scratch_formants` is now `SvVocalTract_formants`. Like 3.2.0's
`SvProsodyContour_xs`, it was `#derive(accessors)` output for internal scratch,
introduced two releases ago, never documented API.

### Fixed — a fresh tract misreported its own formants

Found by a control assertion, not by the feature test it belonged to. The first
version of the formant record left `svara_tract_new` setting it to a fresh zeroed
scratch vec while the filter was built from the schwa targets — so a
brand-new tract claimed **five 0 Hz formants**, `svara_tract_save` captured that,
and `svara_tract_restore` then correctly refused the state as invalid. The
constructor now keeps the vec it actually built the filter from.

The assertion that caught it was `"control: a well-formed state restores"` —
there to prove the *rejection* tests were not passing vacuously. It failed
instead, which is the control doing its job.

### Notes — what restore does NOT reproduce, measured rather than caveated

Two limits are real, and each is pinned by a test that asserts the divergence
*exists* — so they stay measured facts rather than warnings nobody checked.

- **naad's internal state cannot be read out.** A restored `GlottalSource`
  reproduces the deterministic pulse train **bit-for-bit over 1024 samples** when
  breathiness and vibrato are 0 — that is the whole output in that
  configuration. With breathiness > 0 the aspiration stream restarts, because
  naad's `NoiseGenerator` cell and `Lfo` phase are internal to naad. Asserted
  both ways: identical in the first case, *different* in the second.

  > ⚠ **Corrected in 3.3.2: this was wrong.** naad derives accessors on both
  > structs, so the state was reachable all along and the restore is now exact in
  > every configuration svara builds. The test that asserted the divergence now
  > asserts identity.
- **Filter delay lines are not state.** A restored `VocalTract` matches the
  original **over 512 samples from a clean filter state**, and a *warm* tract and
  its restore deliberately differ. That matches the oracle: Rust's `set_formants`
  is `self.filter = FormantFilter::new(..)?`, which discards the delay lines on
  every call, so there is nothing there a caller could have relied on.
  `lip_prev` and `interaction_feedback` — one-sample memories svara owns — *are*
  carried.

Behavioural equivalence is the standard throughout, as in 3.2.0: a restored
`SynthesisContext` renders the same noise-driven fricative (with a fresh context
asserted *not* to match, proving the stream really was resumed), a restored
`BatchRenderer` renders bit-identical audio, and a restored `TrajectoryPlanner`
interpolates identically across its span.

### Notes — restore validates, it does not trust

A state that has been through JSON is externally-authored data. Every `restore`
runs its inputs through the ordinary constructors: `svara_glottal_restore`
rejects an out-of-range or NaN `f0` and a non-positive `sample_rate`;
`svara_tract_restore` rejects a bad sample rate and an empty formant list. All
pinned, each alongside a control proving a well-formed state still restores.

- `dist/svara.cyr` regenerated at v3.3.0 (5,470 lines).
- **Why minor, not patch:** ten new public types and sixteen new public functions,
  plus the one accessor rename above.
## [3.2.0] - 2026-08-26 — container serde, and where serialization stops

**M2.** The serialized surface grows from 8 types to **13**, and — more usefully —
the boundary is now decided and written down instead of inherited. Suite
**713 → 745 assertions** across 20 suites. `cyrius audit` exits 0; every render
path stays bit-for-bit identical to 3.1.2.

### Added — five types now serialize

| Type | How |
|---|---|
| `SvSmoothedParam` | derived — it was already pure scalar and only needed three `f64` annotations |
| `SvRng` | derived — two `i64`; restoring a saved state **resumes the exact draw sequence** |
| `SvF0Point` | **new** — the raw 16-byte `(time, value)` pair given a type. Identical layout; `svara_pt_new`/`_time`/`_value` are unchanged in behaviour |
| `SvProsodyContour` | derived — `Vec<SvF0Point>` |
| `SvPhonemeSequence` | derived — `Vec<SvPhonemeEvent>`, whose element type is flat |

⭐ **Round-trips are asserted behaviourally, not field-by-field.** A restored
`Rng` must reproduce 32 draws exactly; a restored contour must trace the same
curve at 21 points; a restored `PhonemeSequence` must render **bit-identical
audio**. Equal fields are weaker evidence than equal behaviour, and the vacuous
round-trip is a live failure mode in this ecosystem — naad shipped one that held
for three releases because it compared through a truncating conversion.

### Changed — `ProsodyContour`'s interpolation scratch moved to module level

3.1.3 gave each contour its own `xs`/`ys`/`buf_cap` to get the allocation out of
the render loop. Those fields cannot coexist with serialization: **`#derive(Serialize)`
has no skip attribute**, so every field is emitted — a scratch pointer would leak
a heap address into the JSON (an ASLR disclosure if it crosses a trust boundary)
and come back dangling.

The buffers hold nothing between calls — they are refilled on every one — so a
single module-level pair serves every contour. That is less memory than the
per-contour pair, keeps the raw pointers out of the serialized surface, and the
allocation budget is unchanged.

⚠ **`SvProsodyContour_xs` / `_ys` / `_buf_cap` are gone.** They were
`#derive(accessors)` output for internal scratch, existed only in 3.1.3, and were
never documented API.

### Fixed — the roadmap told M2 to match something that does not exist

M2 instructed an implementer to *"match `rust-old/`'s `#[derive(Serialize,
Deserialize)]` / `#[serde(skip)]` per type"*. Transcribed from the oracle, the
contract is **30 `Serialize`-deriving types, 3 `#[serde(default)]`, and zero
`#[serde(skip)]`**. There was no skip policy to inherit.

Rust could serialize everything because `VocalTract` holds a
`naad::filter::BiquadFilter` (`rust-old/src/tract.rs:72`, `:94`) and that type is
`Serialize` in the Rust naad. Cyrius naad exposes those filters as opaque handles
with no JSON codec, so the policy had to be **decided**, not transcribed. It now
is — [ADR-0002](docs/adr/0002-serialization-boundary.md).

### Notes — three toolchain facts, measured

Verified with a standalone probe on cyrius 6.5.35, not read off a changelog:

- `Vec<f64>` / `Vec<iNN>` fields round-trip. ✅
- `Vec<T>` where T is a **flat** `#derive` struct round-trips — via
  `_from_json_str` **only**. The pairs-form `_from_json` returns an *empty vec*
  for an array-of-objects field, because bayan truncates that value at the first
  inner comma. Silent, so it is stated in each affected module header.
- ⚠ **A nested `#derive`-struct field is broken in both directions, and worse
  than "unsupported".** It is laid out **inline** — `sizeof(ND{tag: i64; inner: NA})`
  is 24 for a 16-byte `NA` — while `#derive(accessors)` gives the same field
  **pointer** semantics. The two derives disagree, so the encoder emits the
  pointer reinterpreted as the first member: `{"inner":{"x":6.9e-310,"n":0}}`
  where the value was `{x: 99, n: 3}`. Written inline instead, the encoder is
  correct. **Either way the decoder returns zeros**, so a decoded object holds a
  null pointer where a struct should be — the next dereference is a segfault, the
  same shape as ADR-0001's findings.

svara is not exposed to that corruption: every struct-valued field in `src/` is
declared untyped (`filter;`, `progress;`, `target;`) and so is a plain slot with
correct pointer semantics. **Adding a type annotation to make `Serialize` see
such a field would change its layout and break the accessors** — a trap now named
in ADR-0002.

### Not shipped, and why

`VocalTract`, `GlottalSource`, `FormantFilter` / the SOA bank,
`TrajectoryPlanner`, `SynthesisContext`, `SynthesisPool`, `BatchRenderer`,
`RenderOutput` and `Spectrum` are **not** derived. Each holds a nested struct
pointer, a raw sample buffer, or a foreign naad handle with no JSON form at all.
Under ADR-0002 these are engine state, not configuration: a consumer rebuilds
them from the configuration that does serialize.

`tests/serde.tcyr`'s final group pins the two facts that make them un-derivable —
`FormantKeypoint` holds a target pointer, `GlottalSource` holds three dependency
handles — so a toolchain release that fixes nested-field decode shows up here as
a **failing test** rather than going unnoticed.

Giving each of them a flat state companion plus a `restore` is the mechanical
follow-on, scoped in the roadmap.

### Notes

- **The three `#[serde(default)]` behaviours are not reproduced** — Cyrius's
  derive has no default mechanism and a missing field decodes to zero. For
  `PhonemeEvent.tone` that is wrong in a specific way: `0` is `SVARA_TONE_HIGH`,
  while the Rust default meant "no tone", which svara spells
  `SVARA_TONE_NONE = -1`. svara always emits the field, so this only bites on
  externally-authored JSON — worth a guard if that becomes a supported input.
- **The 13 Rust `Serialize` enums need no counterpart.** They are `var SVARA_*`
  integers here and already round-trip wherever they appear as a field
  (`SvNasalization.place`, `SvPhonemeEvent.stress`/`.tone`). Deriving a one-field
  wrapper per enum would add API surface to serialize an integer.
- `dist/svara.cyr` regenerated at v3.2.0 (4,974 lines).
## [3.1.3] - 2026-08-26 — P1 audit: five reachable defects, and the leak in the render loop

An adversarial-input and allocation sweep of the whole public surface. **Five
defects reachable through the public API with no unsafe usage by the caller** —
one SIGSEGV, three process aborts, one silent NaN — plus the per-sample
allocation that made the trajectory-planned render path scale in gigabytes.

Suite **634 → 713 assertions** across 20 suites. `cyrius audit` exits 0; fuzz and
deny clean. **Every render path is bit-for-bit identical to 3.1.2** — ten
representative renders, digest-compared across the two trees.

### Fixed — SIGSEGV: `alloc` failure was never checked

`lib/alloc.cyr`'s `alloc` **returns 0** on failure (`size <= 0`, `size >
ALLOC_MAX` = 2 GiB, or mmap refusal). It does not abort, the way Rust's
allocator does — so the Rust code had nothing to check and the port carried no
check over. Every size-driven allocation in svara wrote through the returned
pointer on the very next line.

⭐ **`svara_phoneme_synthesize(VowelA, voice, 44100.0, 6100.0)` segfaulted.**
6100 seconds is 269,010,000 samples → 2,152,080,000 bytes, just past `ALLOC_MAX`;
`alloc` returned 0 and `svara_formant_zero_buf` then stored to address 0 and
upward. Confirmed as signal 11, not inferred. It now returns
`SVARA_ERR_COMPUTATION`.

- New `svara_alloc_samples(n)` in `src/error.cyr` — bounds `n` against
  `SV_MAX_SAMPLES` **before** the `* 8`, so the product cannot wrap past its own
  cap, and returns 0 for anything unusable. **19 allocation sites** — 12 in
  `phoneme.cyr`, 3 in `spectral.cyr`, 2 in `prosody.cyr`, 1 each in
  `sequence.cyr` and `tract.cyr` — now route through it and check the result.
- `svara_tract_synthesize` returns 0 rather than a buffer it could not allocate;
  its three callers check.
- `svara_synthctx_ensure_buffer` no longer stores a null over a working buffer on
  a failed grow — that would have poisoned every later call on the context.

### Fixed — `f64_to` does not saturate, and the sentinel reached sample counts

Rust's float → int `as` casts saturate (NaN → 0, negative → 0, overflow → MAX).
Cyrius `f64_to` returns **`INT64_MIN`** for NaN, `±inf` and anything outside
`i64` — measured on this toolchain, not assumed. So the ported line
`let n = (duration * sample_rate) as usize;` yielded `INT64_MIN` wherever Rust
yielded 0 or a cap.

- New `svara_count_from_f64(x)` reproduces Rust's saturating `as usize`.
  **Eleven cast sites** now use it (sample counts, sample offsets, nasalization
  onsets, spectrum bin indices, the crossfade floor, the pool pre-warm).
- ⭐ **A NaN sample rate now returns an empty vec, as Rust does.** Rust checks
  only `sample_rate <= 0.0` there, which is false for NaN, so the cast saturates
  to 0 and `synthesize_phoneme` returns `Ok(Vec::new())`. svara returned
  `INT64_MIN` and carried on. Parity restored, and pinned.
- Three raw `f64_to` calls remain, all provably bounded (the VOT fractions are
  in-tree constants, the crossfade fraction is built from in-tree coarticulation
  constants). Each now says so at the call site.

### Fixed — process abort: a negative spectrum bin reached `vec_get`

Rust's `Spectrum::frequency_bin` returns `usize`, so a negative or NaN frequency
— or a zero `freq_resolution`, which makes the quotient `±inf` — saturates to 0.
The three callers can then test only the *upper* bound, and Rust's do. Ported to
signed `i64` that is half a bound, and `f64_to`'s `INT64_MIN` went straight
through it into `vec_get`, which aborts.

⭐ `svara_spectrum_magnitude_at`, `svara_spectrum_band_energy` and
`svara_spectrum_peak_in_range` **killed the process** on any negative or NaN
frequency argument. Fixed at the source: `svara_spectrum_frequency_bin` now
saturates, so all three callers' single bound is a complete bound again.

### Fixed — silent NaN: an inverted range test accepted a NaN f0

Rust writes `if !(20.0..=2000.0).contains(&f0) { reject }`. `contains` is
`20.0 <= f0 && f0 <= 2000.0`, **false for NaN** — so Rust *rejects* a NaN f0.
The port split it into two one-sided rejections (`f0 < 20`, `f0 > 2000`), and
`f64_lt`/`f64_gt` are both false for NaN, which inverts the meaning.

⭐ `svara_glottal_new` and `svara_glottal_set_f0` **accepted NaN**, stored it,
returned `SVARA_ERR_NONE`, and every sample derived from it came out NaN. Both
now go through `svara_f0_in_range`, which tests the range positively so NaN falls
through to the reject.

### Fixed — a constructor that reported success by handing back an error code

`svara_tract_new` builds a formant filter, retries with a single-500 Hz fallback,
and — before this release — stored whatever came back in the `filter` field. On a
non-positive or non-finite sample rate **both attempts fail**, so it stored the
negative error code as a pointer and returned a tract that looked valid; the
first `process_sample` dereferenced a small negative address.

Rust's `VocalTract::new` returns `Self` and ends that fallback with
`.expect("fallback formant filter must succeed")` — it panics. The port had
neither the panic nor a check. `svara_tract_new` now returns the error, and all
**13 call sites** check it.

### Fixed — infinite loop in `svara_next_pow2`

`p = p * 2` wraps negative past 2^62 and `p < n` then stays true forever, so an
input above that hung the process. It now reports 0 and `svara_spectral_analyze`
rejects it.

### Changed — the render loop stopped allocating per sample

Cyrius's bump allocator **never frees**, so an allocation inside a per-sample
loop is not a performance nuisance, it is an unbounded leak.

⭐ **`svara_sequence_render_planned` spent 1,121 bytes of arena per output sample
untoned and 1,505 toned** — measured as the marginal cost of doubling the
duration, so the fixed setup cancels. A minute of speech at 44.1 kHz therefore
needed ~3 GB (untoned) or ~4 GB (toned) that was never released.

| | 3.1.2 | 3.1.3 | |
|---|---|---|---|
| untoned | 1,121 B/sample | **33 B/sample** | 34× |
| toned | 1,505 B/sample | **113 B/sample** | 13× |

What remains is the output buffer plus the vec it is copied into (8 bytes each
per sample); the per-sample synthesis work itself now allocates nothing. The
toned path additionally pays ~80 B/sample to hisab's `calc_monotone_cubic`,
which allocates three internal arrays per call — upstream, not svara's.

Where it went, per sample, measured call by call:

- **`svara_tract_set_formants`: 736 B → 0.** It called `svara_formant_filter_new`
  and discarded the previous filter. New `svara_formant_filter_rebuild` puts the
  filter into *exactly* the state `new` would have produced — every slot zeroed,
  delay lines cleared, DC blocker reset — while reusing the storage. That
  discard-the-state behaviour is Rust's (`self.filter = FormantFilter::new(..)?`)
  and is preserved deliberately, not "fixed"; only the allocation is saved.
  Validation runs before any mutation, so a rejected call leaves the filter
  untouched, as Rust's `?` does.
- **`svara_vowel_target_to_formants`: 272 B → 0.** New
  `svara_vowel_target_to_formants_into` overwrites five reusable `SvFormant`
  structs owned by the tract.
- **`svara_trajectory_formants_at`: 80–240 B → 0.** New
  `svara_trajectory_formants_at_into`, with the two Catmull-Rom intermediates in
  planner-owned scratch targets.
- **`svara_tone_to_contour`: 240 B → 0.** It was rebuilt every sample from a
  value — the event's tone — that only changes at a phoneme boundary. Now built
  once per event, before the loop.
- **`svara_prosody_f0_at`: 144 B → 80 B.** The two f64 buffers hisab needs are
  now contour-owned. They are **refilled on every call, never cached**, so a
  caller mutating a point through `f0_points` cannot read a stale value; only the
  allocation is reused. The residual 80 B is inside `calc_monotone_cubic`.
- **`svara_formant_bank_update`: 32 B → 0.** The coefficient scratch is
  bank-owned.

CPU time improved too, though that was not the point: **render_planned
20.638 → 18.598 ms** (−10%) and **toned 25.271 → 21.520 ms** (−15%) for the same
three-phoneme sequence.

### Added — two suites, and the ADR that names the class

- **`tests/hardening.tcyr` (45 assertions)** — every defect above, pinned.
  ⚠ **Discrimination measured, not asserted.** Ten of its probes were re-run
  against a 3.1.2 checkout on the same toolchain: **9 of 10 fail there** — four
  by wrong value, three by process abort (`vec: index < 0`), one by SIGSEGV, one
  by hang — and the tenth is the control, which passes on both sides. A group
  where every line flipped would mean the controls were not controls.
- **`tests/allocbudget.tcyr` (34 assertions)** — pins the two properties the
  allocation work rests on. Each `_into` variant is compared **bit for bit**
  against the allocating function it replaced (exact `assert_eq` on raw f64
  patterns, not tolerances — the two paths run the same arithmetic in the same
  order, so this holds on every architecture, unlike a stored golden digest).
  The budget itself is asserted as marginal bytes per sample, each bound paired
  with a control proving the measurement is not reading zero.
- **[ADR-0001](docs/adr/0001-signed-index-and-float-conversion-hazards.md)** —
  the hazard class, its four mechanisms, and the five standing conventions.
  Written because goonj recorded the same two root causes independently, and
  because its 2.0.2 release swept two modules for exactly this and missed a
  third: an unnamed class does not get swept completely.

### Notes

- **Bit-for-bit parity verified directly.** Ten renders — vowel, diphthong,
  fricative, plosive, nasal, crossfade sequence, planned sequence untoned and
  toned, nasalized vowel, female-voice diphthong — were FNV-1a digested over
  their raw f64 sample patterns and compared between a 3.1.2 tree and this one.
  All ten digests match, and the lengths match.
- **Two `benchmarks` added** for `render_planned` (untoned and toned), a major
  public path that had no coverage. The allocation budget is *not* benchmarked —
  a bump allocation is nearly free in CPU time and would not show up — it is
  pinned as a test instead.
- **Preserved deliberately, not overlooked**: `svara_formant_validate` accepts a
  NaN formant frequency, because Rust's
  `f.frequency <= 0.0 || f.frequency >= nyquist` accepts it too. Annotated in
  `src/formant.cyr` rather than quietly tightened — the parity bar is "matches
  what Rust did", and diverging needs an ADR.
- **Known residual**: `svara_ph_buf_to_vec` builds its result with `vec_push`
  from empty, so the vec's doubling leaves roughly one dead copy of the output in
  the arena. Pre-sizing it would mean reaching into `lib/vec.cyr`'s header
  layout; the right fix is a `vec_with_capacity` upstream.
## [3.1.2] - 2026-08-26 — toolchain + dependency catch-up

Maintenance release. Bumps the Cyrius pin **6.4.13 → 6.5.35** (109 toolchain
releases) and every pinned dependency to its current tag. One **breaking upstream
rename** had to be absorbed; behaviour is otherwise unchanged. Suite **634
assertions / 18 suites**, all green; fmt · lint · docs · fuzz · deny clean, and
`cyrius audit` now exits 0 (see below).

### Changed — toolchain

- **`[package].cyrius` `6.4.13` → `6.5.35`.**
- **`lib/` re-vendored from the 6.5.35 snapshot** — all 29 stdlib files verified
  byte-identical to `~/.cyrius/versions/6.5.35/lib`, file by file, rather than
  assumed. `lib/callback.cyr` is newly vendored (a stdlib leaf the refreshed
  dependency bundles now fold in).
- **90 lines reindented across 12 files.** cyrius **6.5.28** fixed `cyrfmt`,
  which had never tracked parentheses: continuation lines inside an unclosed `(`
  were indented at `brace_depth * 4` regardless of nesting. Canonical is now
  2 spaces per open-paren level. **Whitespace only** — `git diff -w` over
  `src/`, `tests/` and `benches/` is empty apart from the `NAAD_FILTER_*` rename
  below.

### Changed — dependencies

| Dep | Was | Now | |
|---|---|---|---|
| hisab | 2.6.7 | **2.11.2** | FFT / compensated sum / monotone-cubic / easing |
| naad | 2.1.1 | **2.2.1** | biquad · noise · LFO backends |
| goonj | 2.0.0 | **2.0.4** | acoustics, referenced by the naad bundle |
| sakshi | 2.4.2 | **2.4.11** | transitive via hisab |

All four match what the bundles themselves pin — naad 2.2.1 pins hisab 2.11.2
and goonj 2.0.4; hisab 2.11.2 pins sakshi 2.4.11 — so a downstream consumer
(dhvani / vansh) bundling svara + naad + hisab in one flat namespace gets a
coherent set.

### Fixed — BREAKING upstream rename absorbed (`src/tract.cyr`)

naad **2.2.0** renamed all eight `FILTER_*` constants to `NAAD_FILTER_*` to clear
a flat-namespace collision with `nidhi`, which defines `FILTER_LOWPASS..NOTCH` at
**identical values**. svara calls exactly two of them:

- `FILTER_NOTCH` → `NAAD_FILTER_NOTCH` (nasal antiformant)
- `FILTER_BANDPASS` → `NAAD_FILTER_BANDPASS` (subglottal bandpass)

Values are unchanged (3 and 2), so this is a compile-time break, not a behaviour
change — the tract suite's 14 assertions pass unchanged.

⚠ **The other renames in that wave were checked and none reach svara.** The full
set of dependency symbols svara actually references was extracted — 24 candidate
identifiers, 17 real top-level references plus the derive-generated
`Lfo_set_depth` accessor — and diffed against the new bundles: `ERR_* → NAAD_ERR_*`
(2.1.3) is inert here because svara's codes are `SVARA_ERR_*`-prefixed and its one
bare `ERR_NONE` mention is a comment; `VOICE_*`, `lerp`, `rms`, `peak`,
`crossfade_equal_power` and the removed `white_noise_sample` / `U32_MAXF` have no
call site. Every remaining signature — `num_fft`, `num_neumaier_sum`,
`calc_monotone_cubic`, `ease_in_out_smooth`, the four `filter_biquad_*`, the two
`noise_*`, the three `modulation_lfo_*` — is byte-identical across the bump.

### Fixed — a stale constant in the benchmark harness

`benches/hotpath.bcyr` and `docs/benchmarks.md` both stated per-call clock
overhead as **"~240 ns on this host"**. cyrius 6.5.19 taught `lib/bench.cyr` to
**measure** the timer floor and subtract it from every sample, and upstream
retired the figure outright: a clock read spans ~15 ns (macOS arm64) to ~3,550 ns
(aarch64 Linux), a 230× range that no single number in a comment can carry. The
harness now prints its own measured floor — **1.34 µs on this host, 5.6× the
number that was written down** — and `bench_clock_overhead_ns()` returns it. Both
mentions now point at the function instead of quoting a value.

The batch-timed figures themselves did not move (`bench_run_batch1/2` already
wrapped one clock pair around 1000 calls), which is what makes the stale comment
harmless in effect and worth fixing anyway.

### Changed — `cyrius audit` is now the gate again

3.0.0 recorded that `cyrius audit` **skipped dependency resolution** before
compiling its test and bench legs, so both failed with spurious *"undefined
variable F64_ONE / SYS_WRITE"* on any project whose tests need the stdlib prelude
plus git-dep bundles — reproducible identically in naad 2.1.0, hence a toolchain
bug rather than a svara defect. The workaround was to gate on the individual
tools.

On 6.5.35 `cyrius audit` resolves deps first (`4 deps resolved`) and **exits 0**:
fmt clean · lint clean · docs complete · 18 suites / 634 assertions · 2 bench
files. The per-tool commands still work as the finer-grained form; the aggregate
is simply no longer broken.

### Notes

- **Benchmarks re-run, no regression.** Per-sample: glottal 82 ns, formant
  179 ns, tract 295 ns. Renders: `/a/` 857 µs, `/s/` 458 µs, `/ai/` 921 µs,
  3-phoneme sequence 3.36 ms. Every row is within a few percent of the 3.1.0 run
  — the spread is host noise, not the toolchain. Recorded in
  `benches/history.csv`; `docs/benchmarks.md` refreshed (its results table still
  carried the pre-3.1.0 `/ai/` figure of ~5.4 ms).
- **`dist/svara.cyr` regenerated at v3.1.2** (4,543 lines). The `.deps` sidecar
  grew from 2 entries to 16 — `cyrius distlib` now emits the full stdlib leaf set
  the bundle folds in, not just the opt-in `hashmap` / `bayan` pair.
- **The 3.1.0 entry's assertion count was wrong and is corrected in place.** It
  said 652; `src/` and `tests/` are byte-identical between the 3.1.0 tag and 3.1.1
  and the suite measures **634** on both, so 652 was never right. `state.md`,
  which said 634, was the one telling the truth.
- **`docs/development/dependency-watch.md` rewritten.** It was still the pre-port
  Rust table — hisab 1.2, naad 1.0, libm, serde, thiserror, tracing, criterion,
  `cargo audit` / `cargo deny`, and a `codecov.yml` coverage threshold for a file
  that does not exist in this repo. It now carries the real pins, the symbols
  svara consumes from each, the flat-namespace rename history, and the procedure
  for checking the next bump.
- **M2 (container serde) is no longer blocked.** It was gated on array-typed
  struct fields, which landed across cyrius **6.4.11–6.4.13** (`Vec<T>` handle
  fields → `#derive` for `Vec<primitive>` → `Vec<#derive-struct>`). Deliberately
  **not** started here — this release is a pin bump, and M2 is its own milestone.

### Known — not addressed here

- **`README.md` and `CONTRIBUTING.md` are still the pre-port Rust files.** They
  survived `cyrius port` untouched: "Formant and vocal synthesis for **Rust**",
  crates.io links for hisab / naad, a `use svara::prelude::*` example, a Cargo
  feature-flag table, "48 phonemes" where the port ships 101, "~1,000× real-time"
  where the measured figure is ~40×, and a quality-gate section made entirely of
  `cargo` commands. Out of scope for a pin bump; they need their own doc sweep.
- **`benches/history.csv` cannot be committed.** `.gitignore` carries a Rust-era
  `*.csv` rule (almost certainly meant for coverage output), so the benchmark
  history is untracked — which defeats the "so regressions are visible across
  commits" purpose `docs/benchmarks.md` states for it. Left alone because
  un-ignoring it is a call about committing a data file, not a maintenance fix.

## [3.1.1] - 2026-08-26 — toolchain pin (retroactive entry)

Pin-only release, tagged without a changelog section at the time; recorded here so
the release workflow's changelog extraction has something to publish.

### Changed

- **`[package].cyrius` `6.4.12` → `6.4.13`.** No source, dependency or behaviour
  change — `VERSION` and the manifest pin are the entire diff.

## [3.1.0] - 2026-07-06 — synthesis performance (control-rate glides)

Performance minor. The faithful v3.0.x port re-derived formant filter coefficients
on every sample of a vowel glide; this release moves that to control rate.

### Changed
- **Diphthong synthesis ~5.8× faster** (`phoneme synth /ai/`: 5.42 ms → 0.94 ms).
  `svara_ph_synth_diphthong` re-solved the entire formant biquad bank from the
  interpolated target on *every* sample; it now recomputes at a control rate of 64
  samples (~1.45 ms at 44.1 kHz — standard for formant synthesizers) and holds the
  coefficients between updates. The per-sample tract filtering is unchanged.
  Perceptually identical (the glide target moves smoothly); the 634-assertion
  tolerance suite passes unchanged. [*Corrected 3.1.2: this entry originally said
  652. `src/` and `tests/` are byte-identical between the 3.1.0 tag and 3.1.1, and
  the suite measures 634 on both, so 652 was never right.*] The diphthong path now matches a steady vowel
  (~0.86 ms) and is faster than the Rust oracle (1.09 ms), which still re-solves
  per sample.
- **Toolchain pin 6.3.40 → 6.4.12** (current release; removes drift, aligns with
  downstream consumers).

### Notes
- SIMD vectorization of the formant biquad bank (`f64v4`) was prototyped and
  reverted: the per-sample loop is **memory-bound** (the SOA state shuffle +
  builtin-call overhead dominate), so packed arithmetic bought only ~5% while the
  ptr/value vector-op API can't keep lane state in registers across samples. The
  real bit-identical lever there is eliminating the redundant per-slot input delay
  line (x1/x2 are identical across all 8 formant slots — the input is shared),
  tracked as future M-perf work alongside register-resident SIMD.

## [3.0.1] - 2026-07-05 — naad 2.1.1 (namespace de-collision)

Dependency maintenance. Bumps the pinned naad dependency **2.1.0 → 2.1.1**, in
which naad renames its two bare dB helpers (`amplitude_to_db` / `db_to_amplitude`
→ `naad_amplitude_to_db` / `naad_db_to_amplitude`) to clear a flat-distlib symbol
clash with abaco. svara does **not** call those helpers — it consumes naad's
biquad / noise (`NoiseGenerator`) / LFO (modulation) backends — so this is a
pin-only bump with **no svara source or behavior change**. It keeps svara
coherent with naad 2.1.1 for downstream consumers (dhvani / vansh) that bundle
svara + naad (+ abaco) in one namespace.

### Changed

- `[deps.naad]` tag `2.1.0` → `2.1.1`. Re-vendor `lib/naad.cyr` with `cyrius deps`
  once the naad 2.1.1 tag is published.

## [3.0.0] - 2026-07-03 — Cyrius port (full parity)

Complete Rust → Cyrius port: **full behavioral parity** with the Rust 2.0.0
surface. All 19 Rust modules ported (16 `.cyr` modules), **634 test assertions
across 18 suites** (vs the Rust 213 tests), **11 hot-path benchmarks** (covering
the intent of the 15 Rust criterion benches), fmt/lint/docs all clean. The Rust
source is preserved at `rust-old/` as the parity oracle. Port plan and locked
decisions: [`docs/development/state.md`](docs/development/state.md),
sequencing: [`docs/development/roadmap.md`](docs/development/roadmap.md).

### Added (port scaffold + foundation layer)

- **Scaffold** via `cyrius port`: `cyrius.cyml` manifest (stdlib incl. `math`,
  `ganita`, `tagged`, `bench`; `[lib]` bundle list), `src/main.cyr` smoke entry,
  `.tcyr`/`.bcyr`/`.fcyr` harnesses. Toolchain pinned to `6.3.38`.
- **`src/error.cyr`** (L0) — ports `error.rs` (`SvaraError` → integer `SVARA_ERR_*`
  codes) + `dsp.rs` validators (`svara_validate_sample_rate`/`_duration`) + shared
  f64 tolerances (`SV_EPSILON`, `SV_POS_INF`) and `svara_is_finite`. **25 tests.**
- **`src/rng.cyr`** (L0) — ports `rng.rs` PCG32 (`SvRng`, seedable). u64/u32 math
  hand-emulated on i64 (`& 0xFFFFFFFF` masking, logical shifts). Verified against
  golden values from a faithful algorithm replica. **17 tests.**
- **`src/smooth.cyr`** (L0) — ports `smooth.rs` one-pole `SvSmoothedParam`
  (`exp(-1/(τ·sr))` via ganita builtin). **6 tests.**
- **`src/formant.cyr`** (L1) — ports `formant.rs`: `Formant`, `Vowel`,
  `VowelTarget` (Hillenbrand 1995 table, interpolation) + the structure-of-arrays
  parallel bandpass biquad bank (`MAX_FORMANTS=8`), `DcBlocker`, and
  `FormantFilter`. svara's own SOA topology (not naad's `BiquadFilter`); nine
  `[f32;8]` arrays become raw f64 buffers held as struct fields. Golden biquad
  coefficients + full impulse-response output verified against an f64 oracle.
  **25 tests.** No external dep.
- **`src/spectral.cyr`** (L1) — ports `spectral.rs`: `Spectrum` + FFT `analyze`
  (Hann window, hisab `num_fft` over an interleaved HComplex buffer), `rms_level`,
  `total_energy`/`band_energy` (hisab `num_neumaier_sum`), peak picking and
  `estimate_formants`. **14 tests.** Adds the **hisab** git dep (pinned `2.6.7`).
- **`src/glottal.cyr`** (L2) — ports `glottal.rs`: `GlottalSource` with Rosenberg B,
  LF (Rd-parameterized via `LfParams::from_rd`), Whisper, and Creaky models, plus
  jitter/shimmer/breathiness/diplophonia/vibrato. Carries the **naad-backend path**
  (aspiration noise = naad `NoiseGenerator::White`, vibrato = naad `Lfo::Sine`;
  svara's PCG32 still drives jitter/shimmer/period). Golden-verified LF params and
  first-period Rosenberg output. **35 tests.** Adds the **naad** (`2.1.0`) + **goonj**
  (`2.0.0`) git deps (full naad bundle).
- **`src/lod.cyr`** (L4, pulled early) — ports `lod.rs` `Quality` (Full/Reduced/Minimal)
  + pipeline-stage predicates (`max_formants`, `use_nasal_coupling`, `use_subglottal`,
  `use_interaction`, `use_lip_radiation`). **15 tests.**
- **`src/tract.cyr`** (L2) — ports `tract.rs`: `VocalTract` + `NasalPlace`. Pipeline =
  source-filter interaction feedback → `FormantFilter` → nasal antiformant (naad `Notch`
  biquad, place-dependent) → subglottal resonance (naad `BandPass` ~600 Hz) → lip-radiation
  HPF → gain, all LOD-gated. Carries the naad-backend path (CFG split collapsed).
  Determinism verified. **14 tests.**
- **`src/phoneme.cyr` (part 1/3)** (L3) — ports `phoneme.rs` inventory +
  classification: the 101-phoneme IPA inventory (`Phoneme` → integer constants in
  exact Rust order), `PhonemeClass`, and the pure lookups `class` / `is_voiced` /
  `coarticulation_resistance` (Recasens DAC). All 101 phonemes golden-verified
  against an independent transcription of the Rust match logic. **229 tests.**
  Data tables (part 2) + synthesis (part 3, after `voice`) follow. The phoneme
  tables are `match`-based pure functions → ported as embedded Cyrius (if-chains),
  not externalized data files.
- **`src/phoneme.cyr` (part 2/3)** (L3) — the data tables: `phoneme_formants`
  (full 101-arm locus/steady-state table), `phoneme_duration`, `phoneme_spectral_tilt`,
  `height_adjusted_amplitudes`, `f2_locus_equation`, `VoiceOnsetTime` (+`for_plosive`),
  `Nasalization` (+`for_nasal`), `detect_nasalization`. Integer targets exact;
  computed values (tilt, height amps) golden-verified. **+44 tests (273 total).**
- **`src/voice.cyr`** (L3) — ports `voice.rs`: `VoiceProfile` (presets
  male/female/child, builder methods, `apply_formant_scale` with f0/singing
  bandwidth scaling), `VocalEffort` + `EffortParams` (whisper..shout continuum →
  coordinated glottal params), and `create_glottal_source[_with_effort]`.
  Formant-scaling goldens + effort energy ordering verified. **33 tests.**
- Cleanup: all 10 `.tcyr` suites brought under the 120-col lint (test files are
  audit-gated too).
- **`src/phoneme.cyr` (part 3a/3)** (L3) — the free synthesis path:
  `synthesize_phoneme` + `synthesize_phoneme_nasalized` dispatching to per-class
  synthesizers (vowel, diphthong, plosive, fricative, nasal, approximant,
  affricate, trill, click, ejective, implosive), plus `fricative_formants`,
  `diphthong_end_target`, and the attack/release envelope (hisab
  `ease_in_out_smooth`). Ties voice + glottal + tract + `FormantFilter` into
  audio (returns a vec of f64 samples). **+17 tests (290 total).** svara can now
  synthesize a phoneme end-to-end. `SynthesisContext` (part 3b) follows.
- **`src/phoneme.cyr` (part 3b/3)** (L3) — `SynthesisContext`: the allocation-reuse
  wrapper owning a `VocalTract` + `GlottalSource` + shared PRNG + growable scratch
  buffer, resetting state between phonemes (the path multi-phoneme rendering drives).
  Ports `synthesize` + the per-class `synthesize_*_ctx` methods (which use fixed
  n/3,n/6,n/4,n/8 timing, not VOT, and route Trill through the approximant path,
  faithful to the Rust). **+9 tests (299 total).** **phoneme is now fully ported**
  (the crate's largest module, 2,636 LOC).
- **`src/prosody.cyr`** (L3) — ports `prosody.rs`: `ProsodyContour` (f0 point list
  interpolated with hisab monotone-cubic; `apply_stress` with dynamic f0-boost
  insertion + sort), the 4 `IntonationPattern`s, 9 lexical `Tone`s (`to_contour`),
  and `Stress`. Contour constants use Cyrius decimal float literals. **17 tests.**
- Toolchain pin → `6.3.39`.
- **`src/trajectory.cyr`** (L3) — ports `trajectory.rs`: `TrajectoryPlanner` +
  `FormantKeypoint`. `plan` builds lead/midpoint/trailing keypoints from phoneme
  events; `formants_at` interpolates with 4-point Catmull-Rom (resistance-weighted
  blend with linear) or sigmoid fallback; `apply_speaking_rate` (Lindblom
  undershoot). Endpoint/boundary/finite behavior verified. **12 tests.**
- **`src/sequence.cyr`** (L3) — ports `sequence.rs`: `PhonemeEvent` / `PhonemeSequence`.
  `render` (per-phoneme synthesis with stress/cluster duration scaling + anticipatory
  nasalization, then resistance-weighted variable crossfade), `render_planned`
  (trajectory-driven continuous synthesis with per-sample formant updates + tone f0),
  `detect_consonant_clusters`, and the variable crossfade blender. **29 tests.**
  Completes the L3 speech-science layer — svara renders whole utterances.
- **`src/pool.cyr`** (L4) — ports `pool.rs`: `SynthesisPool` (pooled `SynthesisContext`
  with `render`/`render_nasalized`/`render_batch`, pre-warmed `with_capacity`, and
  render-count/peak-samples diagnostics). **16 tests.**
- **`src/render.cyr`** (L4) — ports `render.rs`: `BatchRenderer` / `RenderOutput` /
  `RenderProgress` (queue phonemes, render concatenated, stress-scaled + nasalized).
  The Rust FnMut progress callback becomes a `callptr`-based fn-pointer + user-data
  cell. **16 tests.**
- **`src/bridge.cyr`** (L4) — ports `bridge.rs`: 18 dependency-free scalar bridges
  mapping upstream AGNOS crate outputs (bhava emotion, vansh TTS, prani creature,
  goonj acoustics, badal weather) to synth parameters. **37 tests.**
- **All 19 Rust modules are now ported** (16 `.cyr` modules; `dsp` folded into
  `error`, `math` maps to `ganita`). **610 tests total, all lint-clean.**
- **Serialization** — 8 pure-scalar public value types use Cyrius `#derive(Serialize)`
  (`Type_to_json`/`_from_json_str`, f64 fields via toolchain v6.3.40): `Formant`,
  `VowelTarget`, `VoiceProfile`, `EffortParams`, `PhonemeEvent`, `Nasalization`,
  `VoiceOnsetTime`, `RenderProgress`. JSON round-trip tests in `tests/serde.tcyr`
  (**22 tests → 632 total**). Container types (vec/buffer-bearing) await Cyrius
  array-typed struct fields (v6.4.x). Toolchain pin → `6.3.40`.
- **distlib** — `cyrius distlib` bundles all modules into `dist/svara.cyr` (+ `.deps`)
  for consumers (dhvani/vansh), which supply hisab/naad/goonj + stdlib.
- **Benchmarks** — `benches/hotpath.bcyr` (11 benches, auto-discovered by
  `cyrius bench`) reproduces the intent of `rust-old/benches/benchmarks.rs`. The
  per-sample inner loops are batch-timed (one clock pair per 1000 calls) to remove
  the ~240 ns per-call clock overhead. On x86_64 (single core): glottal
  `next_sample` ≈ **82 ns**, formant filter `process_sample` ≈ **175 ns**, tract
  `process_sample` (Full) ≈ **294 ns** — a full glottal→formant→tract chain ≈
  0.55 µs/sample (~40× real-time at 44.1 kHz). Block/render paths: formant
  `process_block` 1024 ≈ **186 µs**, tract `synthesize_into` 1024 ≈ **375 µs**,
  phoneme synth /a/ ≈ **880 µs**, /s/ ≈ **467 µs**, /ai/ (per-sample formant
  re-solve) ≈ **5.4 ms**, 3-phoneme sequence render ≈ **3.4 ms**.
- **API docs** — doc comments added to all remaining public setters/mutators
  (glottal ×8, tract ×6, sequence ×4, batch renderer ×3, DC blocker, pool reset):
  `cyrdoc --check` now reports **0 undocumented public fns** (was 24).
- **Tooling** — `scripts/bench-history.sh` and `scripts/version-bump.sh` rewritten
  for the Cyrius toolchain (parse `cyrius bench` output → `benches/history.csv`;
  version sourced from `VERSION` via `${file:VERSION}`, no `Cargo.toml`).

### Changed

- **`VERSION` → `3.0.0`** — full-parity release. `math.rs` needs no Cyrius module
  (its `libm`/std shims map directly to `ganita` / f64 builtins).
- Symbol convention: all svara symbols `svara_`/`SVARA_`/`SV_`/`Sv`-prefixed to
  coexist with naad's distlib bundle in one flat namespace.

### Notes (parity)

- f32 → f64 throughout (hisab/naad are f64-only); tolerance parity, not bit-exact.
- `next_f32` ports the Rust **code** ([0.0, 1.0), not the doc's [-1.0, 1.0]).
- svara does not flush denormals (its Rust never did) — none added.
- **Quality gate**: fmt (`cyrfmt`), lint (`cyrlint`, 0 warnings), docs (`cyrdoc`,
  0 undocumented), tests (`cyrius tests tests` → 18/18 suites, 634 assertions), and
  benchmarks (`cyrius bench`) are each green. The aggregate `cyrius audit` command's
  test/bench legs currently report compile errors because that command skips
  dependency resolution before compiling (so the stdlib prelude + hisab/naad/goonj
  bundles are absent) — a toolchain issue reproducible identically in the sibling
  `naad` 2.1.0 release, not a svara defect. Run the individual gate commands instead.

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
- **Synthesis quality improvements**:
  - `VoiceOnsetTime`: per-phoneme VOT with place-dependent closure/burst/aspiration fractions (Lisker & Abramson 1964). Voiceless stops get long-lag aspiration, voiced stops get short-lag
  - `phoneme_spectral_tilt()`: per-vowel spectral tilt (0-2 dB/oct based on F1 height), available as metadata
  - `height_adjusted_amplitudes()`: F3-F5 attenuate with vowel openness, modeling source-tract coupling (Fant 1960)
  - `GlottalSource::set_speed_quotient()`: speed quotient (0.5-5.0) for asymmetric pulse models
  - `GlottalSource::set_diplophonia()`: alternating strong/weak pulse amplitude for pathological/expressive voice
  - `PhonemeSequence::set_speaking_rate()`: Lindblom undershoot — faster speech reduces coarticulation resistance in trajectory planning
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
