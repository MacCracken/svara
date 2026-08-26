# Testing Guide

> Rewritten 2026-08-26 for the Cyrius port. The pre-port version was Rust —
> `cargo test`, `make check`, criterion HTML reports, and a `tests/integration.rs`
> that lives only in `rust-old/` now.

## Running

```bash
cyrius audit                       # THE gate: fmt · lint · docs · tests · bench
cyrius tests tests                 # every suite, recursively
cyrius test tests/glottal.tcyr     # one suite
cyrius fuzz                        # tests/svara.fcyr
cyrius deny src/main.cyr           # dependency policy
cyrius bench                       # auto-discovers benches/*.bcyr
```

`cyrius audit` is what a PR must pass. It was unusable before toolchain 6.5.35 —
it skipped dependency resolution before compiling its test and bench legs, so it
failed with spurious "undefined variable F64_ONE" on any project with git-dep
bundles. That is fixed.

⚠ **Exiting 0 is necessary, not sufficient — two measured gaps.** It prints
`scope: src`, so its fmt/lint/docs legs **never touch `tests/` or `benches/`**;
a suite with five >120-char lines passed it clean. And `cyrius test` does not
forward `-D`, so its tests leg only ever compiles the no-defines half of an
`#ifdef`-gated feature. CI closes both — a tests/benches lint step and
`scripts/check-logging.sh`. Filed upstream as
`cyrius/docs/development/issues/2026-08-26-audit-scope-excludes-tests-and-defines.md`.

```bash
./scripts/check-logging.sh    # the -D LOGGING gate: builds + passes BOTH ways
```

Per-file tools, when you want one thing:

```bash
cyrfmt --check src/tract.cyr       # `cyrius fmt <file>` REWRITES IN PLACE
cyrlint src/tract.cyr
cyrdoc --check src/tract.cyr
```

⚠ **`cyrlint`'s "0 untracked deferrals" is not a completeness signal.** It scans
only `.cyr` sources — never Markdown — matches twelve literal terms (`TODO`,
`FIXME`, `deferred`, `for now`, …) and counts one "tracked" if the *same line*
also contains `CHANGELOG`, `roadmap`, `docs/`, `issue`, `See `, `v6.` or `v5.`.
That is a substring test, not a check that the roadmap says anything.

## The suites

**22 suites, 917 assertions.** One `.tcyr` per module, plus six that are not
module ports:

| Suite | What it is |
|---|---|
| `svara.tcyr` | smoke |
| `error` `rng` `smooth` `lod` `formant` `spectral` `glottal` `tract` `voice` `phoneme` `prosody` `trajectory` `sequence` `pool` `render` `bridge` | per-module ports of the Rust `#[test]` blocks |
| `serde.tcyr` | JSON round-trips — 137 assertions |
| `hardening.tcyr` | adversarial input for the [ADR-0001](../adr/0001-signed-index-and-float-conversion-hazards.md) hazard class |
| `allocbudget.tcyr` | `_into`-variant equivalence + the per-sample arena budget |
| `oracle.tcyr` | scenarios the Rust asserted and the port did not — expectations re-derived from `rust-old/`, cited by line |
| `logging.tcyr` | the `-D LOGGING` contract; run **both ways** by `scripts/check-logging.sh`, since `cyrius test` does not forward `-D` |

⚠ Counts drift. `cyrius audit` prints the live figure; treat the numbers here as
a shape, not a gate.

## How to write a test here

**Tolerance parity, not bit-exact.** Transcendentals come from `ganita` and are
not bit-identical across architectures, so compare within epsilon:

```
var TOL_1EM6 = 0x3EB0C6F7A0B5ED8D;   # 1e-6
#must_use
fn approx(a, b, eps) { return f64_lt(f64_abs(f64_sub(a, b)), eps); }
```

Bit-exact `assert_eq` is right in exactly two places: comparing two code paths
inside one binary (`allocbudget.tcyr` does this for the `_into` variants — same
arithmetic, same order, so it holds on every architecture), and integer state
like `SvRng`'s two words.

### Four rules, each learned from a real defect

**1. Follow a round-trip with a behavioural consequence.** Equal fields are
weaker evidence than equal behaviour. A restored `SvRng` must reproduce 32 draws;
a restored `SvPhonemeSequence` must render bit-identical audio. naad shipped a
round-trip assertion that held vacuously for three releases.

**2. Never compare through `f64_to`.** It truncates, so an assertion comparing
converted values holds for everything in (−1.0, 1.0) and asserts nothing.

**3. Pair every claim with a control that proves the measurement is real.** If
you assert "these two are identical", also assert something that would make them
differ. If you assert "this is rejected", also assert a well-formed input is
accepted.

> This is not ceremony. In 3.3.0 the assertion `"control: a well-formed state
> restores"` — written only to prove the *rejection* tests were not vacuous —
> failed, and caught a real bug: a fresh `VocalTract` misreported its own
> formants.

**4. Guard loops that iterate a vec.** `while (i < vec_len(v))` passes vacuously
on an empty vec. Assert `vec_len(v) > 0` first, or count the iterations and
assert the count.

### When the test is for a defect

Measure discrimination — do not assume it. Materialise the previous release and
run the same probe against both trees:

```bash
git archive HEAD | tar -x -C /tmp/prev && cp -a lib cyrius.lock /tmp/prev/
# build one small binary per probe in each tree, then read the EXIT STATUS:
#   139 = SIGSEGV   124 = hang (timeout)   1 = abort or wrong value   0 = correct
```

`hardening.tcyr`'s header records the result of doing this: 9 of 10 probes fail
against 3.1.2, 1 control passes on both sides. A group where *everything* flips
is a group whose controls are not controls.

## Benchmarks

```bash
cyrius bench                  # 13 benches in benches/hotpath.bcyr
./scripts/bench-history.sh    # runs + appends to benches/history.csv
```

⚠ **A bump allocation is a pointer add, so a timer cannot see the cost that
matters most here.** Removing 1,121 bytes of arena per sample from
`render_planned` moved wall-clock only 20.6 → 18.6 ms. Anything that trades
memory for time needs the `allocbudget.tcyr` treatment — marginal arena bytes per
sample, measured by doubling the duration so fixed setup cancels — not just a
bench row.

⚠ **Do not write a clock-overhead figure into a comment.** `lib/bench.cyr`
measures the timer floor and subtracts it; a clock read spans ~15 ns to ~3,550 ns
across supported targets. Call `bench_clock_overhead_ns()`.

Sub-microsecond operations must use `bench_run_batch1/2`, which wraps one clock
pair around a batch. Benches live in `benches/` — `cyrius bench` does **not**
discover `tests/*.bcyr`.

## Adding coverage

- **New phoneme** — extend the all-classes sweep in `tests/phoneme.tcyr`.
- **New public function** — `cyrdoc --check` requires a doc comment; the gate is
  0 undocumented.
- **New serializable type** — round-trip in `tests/serde.tcyr`, plus a
  behavioural consequence and a control (rules 1 and 3).
- **New validation** — a rejection case *and* an acceptance control, in
  `tests/hardening.tcyr` if it belongs to the ADR-0001 class.
- **Performance change** — `./scripts/bench-history.sh` before and after, and
  state whether the metric is time or arena bytes.

## Coverage gaps — closed in 3.5.1

Five scenarios the Rust asserted and the port asserted nowhere were found by
auditing `rust-old/`'s 213 tests against the Cyrius suites. They are closed in
`tests/oracle.tcyr` (69 assertions, no defects found), with every expected value
**re-derived from the Rust and cited by line** rather than taken from running
svara — because after `rust-old/` is retired the only remaining source would be
svara's own output, which would freeze any existing bug in as "correct".

What they were, kept as the record of how a gap of this kind hides:

- **`svara_tract_set_quality` appeared in no suite or bench at all**, so every
  quality-conditional branch ran Full-only under test — an entire feature with no
  evidence behind it.
- Non-zero jitter/shimmer was never asserted to perturb (the glottal suite sets
  both to 0 to keep its goldens deterministic).
- `svara_glottal_set_breathiness` was referenced in no suite.
- Nothing fed *synthesized speech* to the analyzer; `spectral.tcyr` used
  synthetic sines only, so the one cross-module acoustic check was missing.
- Seven `bridge` maps were tested on neither side of the port.

The lesson generalises: **a suite that ports another suite inherits its blind
spots.** The Rust's tests were the source for svara's, so anything the Rust
under-tested, svara under-tested identically — and the module suites all looked
healthy. Auditing coverage against the oracle, rather than counting assertions,
is what surfaced it.
