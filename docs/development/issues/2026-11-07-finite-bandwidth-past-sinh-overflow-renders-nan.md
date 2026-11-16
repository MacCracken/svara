# A finite bandwidth past `sinh`'s overflow passes validation and renders NaN — OPEN

**Status:** 🟡 **OPEN**. This asks to revisit one item that
[ADR-0009](../../adr/0009-what-the-3-6-3-audit-leaves-open.md) leaves open. Nothing in svara is changed.
**Placement:** roadmap 3.7.x, "A finite bandwidth above about 226 × the sample rate renders NaN". It needs an ADR that supersedes the ADR-0009 item.
**Re-verified:** against 3.6.4 on 2026-11-16: the same nine rows, `wrong rows: 4`, exit 4.
**Where the fix would live:** `svara_formant_validate`, `src/formant.cyr:441-445` at 3.6.3.
**Discovered:** 2026-11-07, during prani 2.1.1's bump to svara 3.6.3. prani was pinning svara's new
infinite-bandwidth refusal through `crtract_set_formant_blend` when it found this.
**Reporter:** prani 2.1.1
**Severity:** Medium.
- **Why it matters.** The call returns `SVARA_ERR_NONE`, and every sample after it is NaN. That is a silent wrong
  answer, which is the class of defect [ADR-0007](../../adr/0007-reject-nan-formant-parameters.md) exists to
  refuse.
- **Who can reach it.** Only a caller passing its own bandwidth. svara's vowel tables never get near the bound.
  Neither do prani's species parameters, which are capped at 20 kHz.

**Affects:** svara 3.6.3 (tag `3.6.3`, `7783d18`), built with cyrius 6.7.18. ADR-0009 records the same behaviour
for Rust.

## Summary

ADR-0009 leaves this open on purpose:

> **A huge finite bandwidth gives NaN, as in Rust.** Only non-finite bandwidths are refused, because any finite
> ceiling would be invented. Rust has none.

This report does not dispute the facts there. It disputes the reason: **the ceiling does not have to be invented,
because the coefficient formula defines it.** `svara_formant_biquad_coeffs` (`src/formant.cyr:237`) computes

```
bw_omega = TAU * bw / sr
alpha    = sin(omega) * sinh(bw_omega / 2)
a0       = 1 + alpha
```

`sinh` overflows f64 once its argument passes `asinh(f64::MAX)`, which is about 710.476. Past that point:
- `alpha` and `a0` are both `+inf`.
- `b0 = alpha / a0`, `b2 = -alpha / a0` and `a2 = (1 - alpha) / a0` are all `inf / inf`, which is NaN.

So every bandwidth above

```
bw_max(sr) = asinh(f64::MAX) * sr / pi  ~=  226.1515 * sr
```

gives NaN coefficients, at any formant frequency and for any input. At 44.1 kHz the bound is 9,973,280.7 Hz.

Below the bound every coefficient is finite, and so is the output: see the repro's 1e6 and 9.97e6 rows. Above the
bound the filter can only produce NaN, so refusing there takes away nothing a caller could have used.

## Reproduction

[`repros/2026-11-07-finite-bandwidth-past-sinh-overflow-renders-nan.cyr`](repros/2026-11-07-finite-bandwidth-past-sinh-overflow-renders-nan.cyr)
works through the tract API:
1. It builds a fresh 44.1 kHz tract.
2. It retargets the tract with `svara_tract_set_formants_from_target`, setting F2's bandwidth to the row's value.
3. It runs 2,205 samples of a constant 0.5.

A row is WRONG when the retarget returns `SVARA_ERR_NONE` and any sample is non-finite. The exit code is the number
of wrong rows.

```
cyrius build docs/development/issues/repros/2026-11-07-finite-bandwidth-past-sinh-overflow-renders-nan.cyr /tmp/bw-nan
/tmp/bw-nan; echo "exit=$?"
```

Actual output on 3.6.3 with cyrius 6.7.18, built in a copy of the tag's tree:

```
  ok    bw 100 Hz (control)           rc=0 nonfinite=0
  ok    bw 22050 Hz (nyquist)         rc=0 nonfinite=0
  ok    bw 1e6 Hz                     rc=0 nonfinite=0
  ok    bw 9.97e6 Hz (just below)     rc=0 nonfinite=0
  WRONG bw 9.98e6 Hz (just above)     rc=0 nonfinite=2205
  WRONG bw 1e8 Hz                     rc=0 nonfinite=2205
  WRONG bw f32::MAX                   rc=0 nonfinite=2205
  WRONG bw f64::MAX                   rc=0 nonfinite=2205
  ok    bw +inf (refused since 3.6.3) rc=-1 nonfinite=0
wrong rows: 4
exit=4
```

The 9.97e6 and 9.98e6 rows bracket the computed bound of 9,973,280.7 Hz, and the output flips between them.

Expected: 9.98e6 Hz and above return `SVARA_ERR_INVALID_FORMANT`, as `+inf` now does, and the repro exits 0. The
control rows pass either way.

## Where

- **`svara_formant_validate`** (`src/formant.cyr:441-445`) asks only that `bw > 0` and that it is finite. It guards
  `svara_formant_filter_new` (`:454`) and `svara_formant_filter_rebuild` (`:477`). Through those it also guards
  `svara_tract_new`, `svara_tract_set_formants` and `svara_tract_set_formants_from_target`. The repro takes this
  path.
- **`svara_formant_filter_update`** (`:518`) checks only that the bandwidth is finite.
- **`svara_formant_bank_update`** (`:320`) checks no bandwidth at all.

The last two pass whatever they are given to `svara_formant_biquad_coeffs`. That is from reading the source; neither
path was run.

## Proposed fix (not tested)

Refuse at the point where the formula's own `sinh` overflows, not at a chosen number. Compute the argument with the
same expression the coefficient function uses, so the validator and the filter agree bit for bit at the boundary:

```
var half_bw_omega = f64_mul(f64_div(f64_mul(F64_TAU, bw), sample_rate), F64_HALF);
if (svara_is_finite(f64_sinh(half_bw_omega)) == 0) { return SVARA_ERR_INVALID_FORMANT; }
```

This also refuses `+inf`, since `sinh(inf)` is `inf`, but the explicit finiteness check can stay. The same test
belongs in `svara_formant_filter_update`, beside the finiteness check it already has.
`svara_formant_bank_update` can return the slot-index error it already has.

This would be another divergence of the kind ADR-0007 lists: stricter than Rust, and replacing a silent wrong answer
with an error code. So ADR-0009's item would need to be revisited, not just the code changed.

## Impact on prani

- **What prani pins.** prani 2.1.1 pins svara's infinite-bandwidth refusal in `tests/tract.tcyr`, and lists this
  case on its roadmap's watch list.
- **prani's own paths are safe.** Its species bandwidths are capped at 20 kHz (`PR_SP_BW_MAX`). That cap reaches
  `bw_max` only below an 88.4 Hz sample rate, which svara refuses.
- **What remains reachable.** A caller of `crtract_set_formant_blend` that passes its own bandwidths can still get
  here.

If svara takes the fix, prani adds the 9.98e6 case to that test group, and nothing in prani's source changes.
