# 0007 — Reject NaN formant parameters, diverging from the oracle

**Status**: Accepted
**Date**: 2026-08-26

## Context

`svara_formant_validate` was the last place in svara that did not follow
[ADR-0001](0001-signed-index-and-float-conversion-hazards.md)'s own rule —
*"reject NaN positively, never by negating a range test"* — and it was left that
way on purpose, because the oracle does the same thing.

`rust-old/src/formant.rs:479-486`:

```rust
for f in formants {
    if f.frequency <= 0.0 || f.frequency >= nyquist { return Err(..) }
    if f.bandwidth <= 0.0 { return Err(..) }
}
```

Both comparisons are **false** for NaN, so Rust accepts a NaN formant frequency.
It then reaches `biquad_coefficients`, where `cos(NaN)` and `sin(NaN)` are NaN,
every coefficient becomes NaN, and every sample the bank produces is NaN — with
`FormantFilter::new` having returned `Ok`.

The port reproduced that faithfully. Three releases of hardening work went past
it, each time recording it as a known parity divergence rather than fixing it,
because the standing rule is that diverging from `rust-old/` needs an ADR. This
is that ADR.

The question is sharpened by what the surrounding code now does. 3.1.3 fixed
exactly this shape in `svara_glottal_new` / `set_f0`: Rust's
`!(20.0..=2000.0).contains(&f0)` is TRUE for NaN, so Rust *rejects* it, and the
port's two one-sided rejections had inverted that. `svara_f0_in_range` tests the
range positively so NaN falls through to the reject. The formant validator is
the same test written the other way round — and here the oracle is the one that
gets it wrong.

## Decision

**Reject non-finite formant frequencies and bandwidths.** `svara_formant_validate`
tests positively:

```
var freq_ok = 0;
if ((f64_gt(freq, 0) == 1) & (f64_lt(freq, nyquist) == 1)) { freq_ok = 1; }
if (freq_ok == 0) { return SVARA_ERR_INVALID_FORMANT; }
var bw = SvFormant_bandwidth(f);
if (f64_gt(bw, 0) == 0) { return SVARA_ERR_INVALID_FORMANT; }
```

`f64_gt` and `f64_lt` are both false for NaN, so NaN — and `±inf` — fall through
to the reject. In-range values behave exactly as before; the boundary conditions
(0 Hz, at or above Nyquist, non-positive bandwidth) are unchanged.

This is svara's **third** deliberate divergence, all of the same kind and all
recorded: an overflowing sample count returns an error where Rust saturates and
aborts inside the allocator; `svara_tract_new` returns an error where Rust
panics; and now a NaN formant parameter is refused where Rust accepts it. Each
makes svara strictly stricter than the oracle, and each replaces a silent or
fatal outcome with a checkable error code.

## Consequences

- **Positive** — the failure mode this removes is the worst kind svara has: not
  a crash, but `FormantFilter::new` returning success and then producing an
  entire buffer of NaN. A caller has no reason to check for that. Now the error
  arrives where the mistake was made.

- **Positive** — svara's convention is uniform again. Every NaN rejection in the
  codebase is written positively, and ADR-0001's rule no longer has an exception
  that a reader has to know about.

- **Positive** — it closes the path through `svara_tract_set_formants_from_target`,
  so a NaN `VowelTarget` is refused rather than silently poisoning a tract. That
  is a public entry point.

- **Negative** — a real divergence from the oracle. A program that fed svara a
  NaN formant and consumed the NaN output would now get
  `SVARA_ERR_INVALID_FORMANT` instead. No such program is plausible, but the
  behaviour did change and this is where that is written down.

- **Neutral** — the parity suites are unaffected (all 909 assertions pass
  unchanged), because no test ever fed a NaN formant. `tests/hardening.tcyr`
  pins the new behaviour with six assertions, three of which are controls
  proving the ordinary range checks still work.

## Alternatives considered

- **Keep the divergence-free position and preserve the oracle's hole.** Rejected.
  It had already been deferred through 3.1.3, 3.2.0, 3.3.x and 3.5.x, appearing
  in each release's notes as a known residual. A parity bar is a means to
  correctness, not an end; preserving a defect the oracle has, in the one place
  the project's own hardening ADR forbids, inverts that.

- **Reject NaN but keep accepting `±inf`.** Rejected as arbitrary. `+inf >=
  nyquist` is already true so infinity was rejected before; a positive test
  covers both without a special case.

- **Clamp NaN to a default formant instead of erroring.** Rejected: it invents a
  value the caller did not ask for, and the codebase's convention is that
  invalid input produces an error code, not a substituted guess.

- **Fix it upstream in `rust-old/` too.** Not possible and not wanted —
  `rust-old/` is a frozen oracle and CLAUDE.md forbids modifying it. The
  divergence is recorded here instead, which is what an oracle retirement
  (roadmap 3.6.0) needs anyway.
