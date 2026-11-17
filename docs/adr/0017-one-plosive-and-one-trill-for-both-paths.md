# 0017 — One plosive and one trill for both synthesis paths

**Status**: Accepted (3.14.0)
**Date**: 2026-11-16

## Context

The last 3.9.x item: plosives and trills depend on the render path. svara synthesizes a phoneme two
ways:
- **the free path**, `svara_ph_synth_*`, behind `svara_phoneme_synthesize` and
  `svara_sequence_render`;
- **the context path**, `svara_synthctx_*`, behind `svara_synthctx_synthesize`, the pool, the
  batch render and the stream.

3.8.7 shared every run the two write the same way. What it left apart is what the Rust itself does
differently, and both versions match the oracle. The plosives are `synthesize_plosive` and
`synthesize_plosive_ctx` (`git show 3.5.2:rust-old/src/phoneme.rs`); the trills are
`rust-old/src/phoneme.rs:1468` and `:1033` at tag `3.6.5`.

| | free path | context path |
|---|---|---|
| plosive timing | per plosive, from `svara_vot_for_plosive`: closure, burst and aspiration fractions, e.g. /t/ 0.30, 0.10 and 0.30, /b/ 0.30, 0.08 and none | fixed: closure n/3, burst n/6, no aspiration |
| /p t k/ aspiration | a decaying noise segment, about a quarter of the phoneme | none |
| voiced plosive tail | breathiness 0.30, voicing × 0.5 | breathiness 0.40, voicing × 0.5 |
| voiceless plosive tail | noise × 0.15 | noise × 0.30 |
| trill | the tract's output, amplitude-modulated at 25 Hz alveolar, 20 uvular and 15 bilabial | rendered as an approximant, with no modulation |

So a /t/ rendered through the context has no aspiration, and its burst lasts about two-thirds longer
than the free path's. A trill rendered through the context is a steady approximant. The context path
is the one hoosh renders with, and the one the stream, the pool and the batch render use.

## Decision

**Give both paths the free path's plosive and trill.** The maintainer chose option 1 below on
2026-11-16.

**As shipped in 3.14.0:**
- **The plosive.** `svara_ph_plosive_run` is the free path's plosive once its objects exist: the
  buffer zeroed, then the VOT closure, a burst of at least one sample, the aspiration and the tail.
  It reads the timing through `svara_vot_for_plosive_into`, on the stack, so the context still
  allocates nothing per phoneme.
- **The trill.** `svara_ph_trill_run` is the free path's amplitude-modulated loop, and
  `svara_ph_trill_rate` its 25, 20 and 15 Hz.
- **The free path** builds its plosive and trill over them, and its output is the same bits.
- **The context.** `svara_synthctx_plosive` builds over the plosive run, with the voiced tail's
  breathiness at 0.30, restored after. The new `svara_synthctx_trill` builds over the trill run, and
  `svara_synthctx_synthesize` sends trills to it, where it sent them to the approximant.
- **The tests.** `tests/synthpaths.tcyr` holds the context's plosive and trill to the free path's
  3.8.6 code, run on the context's objects, bit for bit, and checks the dispatch through
  `svara_synthctx_synthesize`.

The options:

1. **Give both paths the free path's plosive and trill (chosen).** The free path's versions are the
   phonetically detailed ones: per-plosive voice-onset timing with aspiration, and a trill that
   trills.
   - A shared `svara_ph_plosive_run` and `svara_ph_trill_run` replace the context's versions.
   - The context's dispatch sends trills to a trill method, not to the approximant.
   - The free path's output is unchanged.
   - The context path's plosives and trills change: in the stream, the pool, the batch render and
     hoosh.
   - It diverges from the oracle's context path: a minor (3.14.0).
2. **Give both paths the context's plosive and trill.** Fixed thirds and no aspiration, and a trill
   rendered as an approximant. The free path's plosives and trills change, and lose their aspiration
   and their modulation.
3. **Keep both, and document it.** The Rust's two paths stay two, and the item closes.

The proposal is option 1. A phoneme should not sound different because of which render produced it,
and of the two versions, the free path's is the one built from phonetic data.

## Consequences

- **Option 1** — /p t k/ gain their aspiration and VOT timing in every render, and trills trill in
  every render. The context path's plosives and trills move; the free path's do not. One plosive
  method and one trill method fewer to keep in step.
- **Option 2** — the free path's plosives and trills move, and lose detail.
- **Option 3** — nothing moves; the table above is the documentation.

## Alternatives considered

- **Per-plosive timing in the context path without aspiration.** It is half of option 1, and keeps
  two plosives to maintain.
