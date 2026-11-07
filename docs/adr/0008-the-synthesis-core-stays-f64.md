# 0008 — The synthesis core stays f64

**Status**: Accepted
**Date**: 2026-11-07

## Context

The Rust oracle computes in f32 and the port widened to f64. On 2026-08-31 prani
filed a P0 here: its Cyrius port ran 15.7× slower than its f32 Rust oracle, and
nearly all of that time is svara's per-sample path. The filing gave three reasons
to convert, SIMD width, memory traffic and parity, and set a rule: measure one hot
path first, because the honest prior was that codegen dominates, not float width.

The work was sequenced behind naad, because svara's per-sample calls go through
naad's filters and noise. naad ran the experiment on `filter_biquad_process_sample`,
in f64 and f32, in Cyrius and Rust, and closed its own P0 in naad's
[ADR-0008](https://github.com/MacCracken/naad/blob/main/docs/adr/0008-the-dsp-core-stays-f64.md):

| Claim | Measured by naad |
|---|---|
| f32 is faster per sample | **No.** 18.3 ns against 18.3 ns. In Rust, f32 is 0.5% to 3.7% slower |
| Eight lanes beat four | 18% all-in. SIMD itself gives 3.3× in f64 |
| Half the memory traffic | Half the bytes, no speed |
| f32 parity with the oracle | Yes, bit-exact |

It also found that ganita's f32 tier routes through f64, so `sin` is 15% slower and
`exp` 93% slower, and that `f32` cannot be a fn return type at cyrius 6.7.18, so an
f32 would cross svara's whole per-sample surface as raw bits. naad's ADR says svara
and prani should not convert on naad's account.

## Decision

**The synthesis core stays f64, and the P0 is closed.** svara cannot gain from f32
alone: its filters and noise stay f64 in naad, so it would widen at every per-sample
call, which is more work than doing nothing.

## Consequences

- **Positive** — no rewrite of every float in the engine and in prani for no measured
  speed, and prani's roadmap is no longer blocked on this.
- **Positive** — the effort goes where the measurement points. naad found the same
  arithmetic costs 22.7 ns through accessors and one call per sample, and 9.5 ns in
  one loop in locals. That is the roadmap's performance lane.
- **Negative** — ported tests keep tolerances where the oracle is f32. Bit-exact
  parity with the oracle is real and is given up. This is the tolerance parity the
  port already chose.
- **Neutral** — reopen this if `f32` becomes a return type, ganita gets native f32
  math, and a measured workload shows it matters.

## Alternatives considered

- **Repeat the experiment on svara's own hot path before closing.** Not done. The
  biquad is the arithmetic svara's formant bank runs, and the conclusion would still
  need naad to convert. A svara-only result could not change the decision.
- **Convert svara alone.** Rejected above: every call into naad would widen and
  narrow, per sample.
