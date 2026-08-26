# 0001 — Signed-index and float-conversion hazards inherited from the Rust port

**Status**: Accepted
**Date**: 2026-08-26

## Context

The 3.0.0 port reproduced `rust-old/` faithfully, module by module, and the
634-assertion tolerance suite passed. What the suite could not see is that Rust
encodes several guarantees in its *type system* that Cyrius has no equivalent
for. Porting the code faithfully therefore dropped them silently, because the
ported line looks correct.

An adversarial-input sweep in 3.1.3 found **five instances across four modules**.
None was caught by the existing suite; all five were reachable through the public
API with no unsafe usage by the caller. Two were memory-unsafe, two aborted the
process, one silently poisoned the output.

The four mechanisms, each verified on this toolchain rather than assumed:

1. **`f64_to` does not saturate.** Rust's float → int `as` casts saturate (since
   1.45): `NaN → 0`, negative → `0`, overflow → `MAX`. Cyrius `f64_to` returns
   **`INT64_MIN`** for NaN, `±inf`, and anything outside `i64`, and otherwise
   truncates toward zero. So the Rust idiom
   `let n = (duration * sample_rate) as usize;` ports to a line that yields
   `INT64_MIN` for a NaN sample rate. Measured: `f64_to` returns
   `-9223372036854775808` for all four of NaN, `+inf`, `-inf` and `1e27`.

2. **`usize` cannot be negative.** Rust therefore writes a bounds check as a
   *single* upper-bound test — `if bin < self.magnitudes.len()`. That is a
   complete check in Rust. In signed `i64` it keeps only half the bound, and the
   index then reaches `vec_get` (which aborts) or raw `load64`/`store64` (which
   does not).

3. **`alloc` returns 0; it does not abort.** `lib/alloc.cyr` returns `0` for
   `size <= 0`, for `size > ALLOC_MAX` (2 GiB), and on mmap refusal. Rust's
   allocator aborts the process instead, so the Rust code has nothing to check
   and the port had no check to carry over. Every one of svara's size-driven
   allocations wrote through the returned pointer on the next line.

4. **A negated range test inverts on NaN.** Rust's
   `if !(20.0..=2000.0).contains(&f0) { reject }` is TRUE for NaN — Rust
   *rejects* it. Rewriting that as two one-sided rejections (`f0 < 20`,
   `f0 > 2000`) inverts the meaning, because `f64_lt` and `f64_gt` are both
   false for NaN.

A fifth, narrower member: **an unbounded doubling loop**. `svara_next_pow2`'s
`p = p * 2` wraps negative past 2^62 and `p < n` then stays true forever, so an
input past that hung the process rather than erroring.

This class is not svara's alone — goonj recorded the same two root causes in its
own ADR-0002 after 13 of 18 defects in two releases traced to them. That the
same shapes appeared here, independently, is the argument for naming it rather
than fixing the five and moving on.

## Decision

Treat these as standing conventions, checked in review and in new code.

1. **Never call `f64_to` on a value that has not already been bounded in f64.**
   Use `svara_count_from_f64` (`src/error.cyr`), which reproduces Rust's
   saturating `as usize`: NaN, negatives and zero give 0; anything at or past
   `SV_MAX_SAMPLES` gives `SV_MAX_SAMPLES + 1`, which no in-range test accepts
   and which `svara_alloc_samples` rejects. It applies to every f64 → count or
   index cast: sample counts, sample offsets, and spectrum bin indices alike.

   A raw `f64_to` is permitted **only** where both operands are in-tree
   constants or already-bounded counts, and the call site must say so in a
   comment. Four such calls remain — the three VOT fractions at
   `src/phoneme.cyr:714-716` and the crossfade fraction at
   `src/sequence.cyr:296` — each covered by that note.

2. **Saturate at the source, not at each use.** `svara_spectrum_frequency_bin`
   returns a non-negative bin, exactly as Rust's `-> usize` does. That makes the
   three callers' single upper-bound tests complete bounds again, rather than
   requiring a lower-bound test to be added — and remembered — at each one.

3. **Every size-driven allocation goes through `svara_alloc_samples`, and every
   caller checks the result for 0.** The helper bounds `n` against
   `SV_MAX_SAMPLES` *before* multiplying by 8, so the product cannot wrap past
   its own cap. A pointer-returning function propagates 0; a code-returning
   function returns `SVARA_ERR_COMPUTATION`.

4. **Reject NaN positively, never by negating a range test.** Write
   `if (in_range(x) == 0) { reject }` where `in_range` uses the NaN-correct
   `f64_ge`/`f64_le` pair, so NaN falls through to the reject
   (`svara_f0_in_range` is the worked example).

5. **A constructor that cannot build its object returns a negative error code**,
   even where the Rust returned `Self` and panicked. `svara_tract_new` used to
   store the error code *in the filter field* and hand back a tract that looked
   valid; the first `process_sample` then dereferenced a small negative address.
   Reporting the error is the code/sentinel equivalent of Rust's
   `.expect("fallback formant filter must succeed")`.

Scope: this ADR governs conversions, bounds and allocation failure. It does not
license tightening validation that the oracle also lacks — see below.

## Consequences

- **Positive** — the class is named, so it is greppable (`f64_to(`, `alloc(`)
  and reviewable rather than rediscovered per module. `tests/hardening.tcyr`
  pins every instance found so far; **9 of its 10 public-API probes fail against
  3.1.2** (four by wrong value, three by process abort, one by SIGSEGV, one by
  hang) while the control passes on both sides, so the suite is measuring
  something real rather than decorating a fix.

- **Negative** — the guards are wordier than the Rust they replace, and the
  duplication is unavoidable without an unsigned type or a saturating conversion
  in the language. Two guards are deliberately *stricter* than the oracle:
  an overflowing sample count returns an error where Rust saturates to
  `usize::MAX` and aborts inside the allocator, and `svara_tract_new` returns an
  error where Rust panics. Both are documented at the call site.

- **Neutral** — some Rust weaknesses are carried forward on purpose, because the
  parity bar is "matches what Rust did" and diverging needs its own ADR. The
  known one: `svara_formant_validate` accepts a NaN formant frequency, because
  Rust's `f.frequency <= 0.0 || f.frequency >= nyquist` accepts it too and
  produces NaN coefficients. It is annotated in `src/formant.cyr` rather than
  quietly tightened.

## Alternatives considered

- **Fix the five sites and move on, without an ADR.** Rejected on goonj's
  evidence: its 2.0.2 release swept `fdtd` and `dwm` for exactly this class and
  missed `gfpe`, which 2.0.4 then had to fix. An unnamed class does not get
  swept completely.

- **A saturating `f64_to_sat` wrapper with no cap, leaving the ceiling to each
  call site.** Rejected: the whole failure mode is a cast whose result nobody
  bounds, and a helper that returns an unbounded number just moves the omission
  one line down. The cap is what makes `svara_alloc_samples` a complete check.

- **Bound only at the public entry points, leaving internal allocations
  unchecked.** Rejected as defence-in-depth: entry bounding is what restores
  *parity*, but `alloc` can still return 0 on mmap refusal, and the internal
  sites are where the null would be written through.

- **Per-call-site lower-bound tests in `spectral.cyr` instead of saturating
  `frequency_bin`.** Rejected: three call sites today, and the next one added
  would have to remember. Saturating the producer makes the consumers correct by
  construction and matches the Rust signature.
