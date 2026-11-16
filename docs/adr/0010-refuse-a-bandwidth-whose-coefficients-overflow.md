# 0010 — Refuse a bandwidth whose coefficients overflow

**Status**: Accepted
**Date**: 2026-11-16
**Supersedes**: the ADR-0009 item "A huge finite bandwidth gives NaN, as in Rust"

## Context

[ADR-0009](0009-what-the-3-6-3-audit-leaves-open.md) kept one silent wrong answer on
purpose: a finite formant bandwidth large enough passed validation, and every sample
after it was NaN. Only non-finite bandwidths were refused, "because any finite ceiling
would be invented. Rust has none."

prani filed it during its 3.6.3 bump
([issue](../development/issues/archived/2026-11-07-finite-bandwidth-past-sinh-overflow-renders-nan.md))
and disputed the reason, not the facts: the ceiling does not have to be invented,
because the coefficient formula defines it. `svara_formant_biquad_coeffs` forms

```
alpha = sin(omega) * sinh(pi * bw / sr)
a0    = 1 + alpha
```

`sinh` overflows f64 once its argument passes `asinh(f64::MAX)`, about 710.476. From
there `alpha` and `a0` are `+inf`, three of the four coefficients are `inf / inf`, and
the filter can only produce NaN, for any formant frequency and any input. At 44.1 kHz
that is every bandwidth above 9,973,280 Hz. Below it every coefficient is finite.

[ADR-0007](0007-reject-nan-formant-parameters.md) exists to refuse exactly this
shape: a call that returns success and then renders NaN.

## Decision

**Refuse a bandwidth whose coefficients overflow, in `svara_formant_validate` and in
`svara_formant_filter_update`, with `SVARA_ERR_INVALID_FORMANT`.** It is a
divergence from Rust, which accepts the bandwidth and renders NaN.

- **The bound is the coefficient code's own.** `svara_formant_bw_overflows` forms the
  `sinh` argument with the same operations as `svara_formant_biquad_coeffs` and asks
  the same `f64_sinh`. A constant derived beside it could disagree with ganita's
  `sinh` by a rounding step at the edge, and a toolchain could move it.
- **It costs one comparison per formant.** Below `222 × sample_rate` the argument is
  under 697, where `sinh` cannot overflow, so `validate` compares against that and
  only calls `sinh` above it. The ADR-0007 finiteness test moved inside the same
  branch, since `+inf` exceeds the threshold too, which is capped at `f64::MAX` for
  that reason. `validate` runs once per sample on the `render_planned` path, which
  retires 0.17% fewer instructions than 3.6.4 as a result.
- `update_formants` refuses non-finite parameters since ADR-0007 but none of the
  range checks `new` makes, as Rust's checks nothing. It gets this check because
  an overflowing bandwidth can only produce NaN there too, and `update` keeps the
  filter's state, so a NaN there would last.

## Consequences

- **Positive** — No finite bandwidth renders NaN from a call that returned success.
  `tests/hardening.tcyr` sweeps 2,001 bandwidths across the edge and finds `validate`
  accepting exactly those whose coefficients are finite. Before the fix, 672 of
  them mismatched, and all four of the issue's rows past the bound rendered NaN.
- **Negative** — One more divergence from the oracle, recorded here and in ADR-0009's
  table. Rust renders NaN at these bandwidths, and svara refuses them.
- **Neutral** — Nothing a caller could use changes. Every refused bandwidth gave NaN
  coefficients, and every accepted bandwidth gives the same output as before:
  `scripts/sweep/compare.py 3.6.4` moves none of 558,721 values. Of its 638 hostile
  cases, 7 change, all refusals: four huge bandwidths given to a formant filter
  (NaN before), and three huge base f0 values whose scaled bandwidths now trip this
  check before the glottal source rejects the pitch, so the code changes from
  `SVARA_ERR_ARTICULATION` to `SVARA_ERR_INVALID_FORMANT`. Those three were refused
  before too; ADR-0009's BD-7 row records that class.

## Alternatives considered

- **Keep ADR-0009's position** — rejected. "Any finite ceiling would be invented" was
  the reason, and the ceiling turned out to be the formula's.
- **A constant bound, `asinh(f64::MAX) × sr / π`** — rejected. It is the right value,
  but computed apart from the coefficient code it can disagree with the `sinh` that
  code calls by a rounding step at the edge.
- **Check the computed coefficients for finiteness** — rejected. `validate` runs per
  sample on the trajectory path, and four coefficients and a `sinh` per formant per
  sample would cost far more than the prefilter.
