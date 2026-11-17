# 0016 — Honour tone in every sequence render

**Status**: Accepted (3.13.0)
**Date**: 2026-11-16

## Context

A 3.9.x quirk, preserved from the Rust: an event's `tone` is honoured only by
`svara_sequence_render_planned`. `svara_phoneme_event_with_tone` takes one of nine lexical tones,
and `svara_tone_to_contour` turns each into an f0 multiplier over the event:

| Tone | f0 over the event |
|---|---|
| High | 1.2 throughout |
| Rising | 0.85 → 1.25 |
| Dipping | 1.0 → 0.75 → 1.05 |
| Falling | 1.3 → 0.75 |
| Neutral | 1.0 → 0.95 |
| Low | 0.75 throughout |
| Mid | 1.0 throughout |
| Low rising | 0.7 → 1.1 |
| High falling | 1.35 → 0.7 |

The range runs from 0.7 to 1.35: about six semitones down to five up.

`render_planned` multiplies the event's stressed f0 by the contour at every sample
(`src/sequence.cyr`, at the per-sample `svara_glottal_set_f0`). Three other renders take the same
events and ignore the tone: `svara_sequence_render`, the streaming `svara_sequence_render_into`, and
the batch render. Measured in 3.12.0, a primary-stressed /a/ of 0.3 s:
- **`sequence_render`** gives bit-identical samples with and without each of the nine tones;
- **`render_planned`** differs for eight of them, all but Mid, whose contour is flat at 1.0.

The Rust is the same: `Tone` is wired into `render_planned` only (v2.0.1). For a tonal language, the
crossfade render, the stream and the batch render lose a word's meaning.

**How it could be honoured elsewhere.** The glottal source reads its f0 only when a period begins
(`svara_glottal_new_period`); `svara_glottal_set_f0` just stores it. So `render_planned`'s
per-sample `set_f0` takes effect at each period start, with the contour's value at the sample that
starts the period. A source that carries the event's contour and looks it up at each period start
gives the same pitch track without anyone setting f0 per sample.

The other renders build or reuse a source per phoneme:
- the free path inside each class synth;
- the context in `svara_synthctx_synthesize`.

The tone would reach the source through the free path's dispatch and a context entry that takes a
tone.

## Decision

**Honour tone in every sequence render.** The maintainer chose option 1 below on 2026-11-16.

**As shipped in 3.13.0:**
- **The tone moved to the source.** The tone ids, a non-allocating table of each tone's points
  (`svara_tone_points_into`) and the contour lookup (`svara_prosody_f0_at_filled`, unchanged) moved
  to `src/glottal.cyr`. `svara_tone_to_contour` builds its contours from that table.
- **The source carries a tone.** `svara_glottal_set_tone(source, tone, base_f0, span)` gives it a
  tone, at most 5 points in two buffers it allocates on its first tone and keeps. At each period
  start it sets f0 to base × contour(sample / span), refusing an out-of-range f0 as `set_f0` does.
  `SVARA_TONE_NONE` clears it. An untoned source pays one check per sample.
- **The free path.** Its class synths, its dispatch and `svara_ph_synthesize_nasalized_raw` take a
  tone, and give it to every source they build. `svara_phoneme_synthesize` keeps its signature.
- **The context.** `svara_synthctx_synthesize_toned` takes a tone, and the stream and the batch
  render call it with each event's. `svara_synthctx_synthesize` is the untoned case.
- **`sequence_render`** passes each event's tone. `render_planned` is unchanged.
- **The tests.** `tests/prosody.tcyr` holds a toned source to `render_planned`'s per-sample `set_f0`,
  bit for bit, for all nine tones. The table is held to the Rust's points, at tag `3.6.5`
  `rust-old/src/prosody.rs:231-285`. A warmed context streams a toned sequence keeping no bytes.

One edge differs from `render_planned`: where base × contour leaves the f0 range, `render_planned`
keeps the last f0 accepted at any sample, and a source the last accepted at a period start.

The options:

1. **Honour tone in every sequence render (proposed).**
   - The glottal source gains an optional pitch contour: an event's tone, its base f0 and its length
     in samples. At each period start it sets f0 to base × contour(t), and refuses an out-of-range
     f0 as `render_planned` does.
   - `sequence_render`, the stream and the batch render give each toned event's contour to the source
     that voices it. `render_planned` is unchanged.
   - **Untoned input is unchanged, bit for bit**, so only callers who set a tone hear a difference.
   - The cost is one branch per glottal sample when no contour is set. A minor (3.13.0), since toned
     output changes.
2. **Honour it in `sequence_render` and the stream only.** As option 1, without the batch render,
   whose events are phoneme lists more often than words.
3. **Keep it as it is, and document it.** Tone stays `render_planned`'s only, as in the Rust, and the
   other renders' docs say they ignore it.

The proposal is option 1. A tone that changes the meaning of a word should not depend on which render
a caller picked, and untoned callers see nothing change.

## Consequences

- **Option 1** — the three renders voice tones as `render_planned` does. `svara_ph_synth_dispatch`
  and the class synths gain a tone argument, and the context gains an entry that takes one; the
  public `svara_phoneme_synthesize` keeps its signature. The glottal source carries three more
  fields. Any sweep case with a toned event moves in those renders.
- **Option 2** — as option 1, with the batch render left as it is and documented.
- **Option 3** — nothing moves, and tone stays a render_planned-only feature.

## Alternatives considered

- **Set f0 per sample in each class synth.** It is the same pitch track, but it means a tone loop in
  every class synth, where one lookup at each period start in the source does it once.
- **Carry the tone in `SvVoiceProfile`.** The voice is serialized config; a tone is a property of an
  event, not of a speaker.
