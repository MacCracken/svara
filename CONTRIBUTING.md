# Contributing to svara

Thank you for your interest in contributing to svara.

svara is a **port**: `rust-old/` holds the original 8,785-line Rust library and is the parity
oracle. The correctness bar is "matches what the Rust did". Never modify `rust-old/`, and diverge
from it only with an ADR under [`docs/adr/`](docs/adr/).

## Getting Started

1. Fork the repository
2. Clone your fork
3. Create a feature branch: `git checkout -b feature/your-feature`
4. Make your changes following the guidelines below
5. Submit a pull request

## Development Requirements

The Cyrius toolchain, at the version pinned in `cyrius.cyml [package].cyrius`. **That pin is the
single source of truth** — never hardcode a version in CI YAML, scripts, or docs. Read it and hand
it to the canonical installer (this is exactly what `.github/workflows/ci.yml` does):

```bash
CYRIUS_VERSION="$(grep '^cyrius = ' cyrius.cyml | head -1 | sed 's/cyrius = "\(.*\)"/\1/')" && curl -sSf https://raw.githubusercontent.com/MacCracken/cyrius/main/scripts/install.sh | CYRIUS_VERSION="$CYRIUS_VERSION" sh
```

That lays out `$HOME/.cyrius/{bin,lib}` — add `$HOME/.cyrius/bin` to `PATH`. It also gives you the
CVE-21 checksum and CVE-13 signature verification; do not hand-roll `curl` + `tar` + `cp`.

Then resolve dependencies and build:

```bash
cyrius deps && cyrius build src/main.cyr build/svara
```

`cyrius deps` vendors the stdlib files and dependency bundles into `lib/`. **Never hand-edit
`lib/`** — it is generated, and a bump re-vendors it wholesale.

## Code Quality Requirements

Before submitting a PR, the aggregate gate must pass:

```bash
cyrius audit
```

That sweeps fmt · lint · docs · tests · bench and exits 0 when everything is green. It resolves
dependencies first, so it works standalone.

The finer-grained per-tool forms remain useful while iterating — `cyrfmt`, `cyrlint` and `cyrdoc`
live in `$HOME/.cyrius/bin`:

```bash
cyrfmt --check src/phoneme.cyr        # formatting (per file)
cyrlint src/phoneme.cyr               # 0 warnings, 0 untracked deferrals
cyrdoc --check src/phoneme.cyr        # 0 undocumented public functions
cyrius tests tests                    # every .tcyr suite, recursively
cyrius fuzz                           # tests/svara.fcyr
cyrius deny src/main.cyr              # dependency policy
cyrius bench                          # auto-discovers benches/*.bcyr
```

Two formatter notes:

- **The canonical continuation indent is 2 spaces per open-paren level** (4 is also accepted).
  Cyrius 6.5.28 taught `cyrfmt` to track parentheses; the older 8-space convention is retired.
- `cyrius fmt <file>` **rewrites the file in place** — it does not print to stdout.

## Code Standards

- **`svara_` / `SVARA_` / `SV_` / `Sv` prefix on every top-level symbol.** Cyrius has one flat
  namespace with no module system, so the svara distlib has to coexist with naad's, hisab's and
  goonj's bundles inside a single consumer. An unprefixed `lerp` or `ERR_NONE` is a collision
  waiting to land. Functions are `svara_*`, public constants `SVARA_*`, internal f64 literals and
  tolerances `SV_*`, struct types `Sv*`.
- **`#must_use` on pure functions** — 190 of them carry it today.
- **`#inline` on hot-path sample processing** — the per-sample formant biquad and DC blocker,
  `svara_tract_process_sample`, `svara_glottal_next_sample`, the smoother and the RNG.
- **`#derive(accessors)` for struct fields** — it emits the `SvType_field` / `SvType_set_field`
  pair; do not hand-write accessors.
- **`#derive(Serialize)` for serde** — it emits `Type_to_json` / `_from_json` / `_from_json_str`
  over bayan's typed JSON DOM. **No hand-written codecs**; that was explicitly rejected as
  throwaway work the derive supersedes. Any deriving entry file must also
  `include "lib/hashmap.cyr"` and `include "lib/bayan.cyr"` (both are opt-in, not `[deps].stdlib`
  entries) so the codecs are callable and warning-free. New types need a roundtrip `.tcyr`.
- **Negative error codes, not `Result`.** Fallible functions return a negative `SVARA_ERR_*` from
  `src/error.cyr` (or a valid pointer / count on success); callers test with `svara_is_err` and
  name with `svara_err_name`. Never panic or abort in library code.
- **No `println!`-style output from library code.** svara emits no logs today — diagnostics are
  error codes only. Structured logging through **sakshi** is unimplemented and tracked as
  **M-log** in [`docs/development/roadmap.md`](docs/development/roadmap.md); do not wire ad-hoc
  printing in ahead of it.
- **f64 end-to-end.** The Rust was f32/f64 mixed; the port is f64 throughout (hisab and naad are
  f64-only). `SV_EPSILON` preserves the f32 tolerance the Rust tests used, promoted to f64.
- Test after every change, not after the feature is "done". ONE change at a time — never bundle
  unrelated changes.

## Adding New Phonemes

The inventory is integer constants in `src/phoneme.cyr`, **in exact Rust variant order** so index
and serialization mappings match the oracle. Adding a phoneme past 100 therefore diverges from
`rust-old/` and needs an ADR first.

1. Add the `SVARA_PH_*` constant to the inventory block in
   [`src/phoneme.cyr`](src/phoneme.cyr) and bump `SVARA_PHONEME_COUNT`.
2. Give it an arm in `svara_phoneme_class()`. The chain is range-based over contiguous class runs
   with scattered specials explicit, and it **falls through to `SVARA_CLASS_SILENCE`** — a new
   value needs its arm *before* that fallthrough or it silently synthesizes as silence.
3. Add it to `svara_phoneme_is_voiced()` if voiceless — the voiceless set is explicit and
   everything else is voiced by default.
4. Add formant targets in `svara_phoneme_formants()` (vowels get steady-state targets, consonants
   get locus frequencies; use the `svara_ph_vt` / `svara_ph_vtb` helpers).
5. Add a `svara_phoneme_coarticulation_resistance()` arm, or confirm the class-based default is
   right for it.
6. `svara_phoneme_duration()` and `svara_phoneme_spectral_tilt()` are class-driven — they need no
   change unless the phoneme wants a non-class-default. Plosives also need
   `svara_vot_for_plosive()`, nasals `svara_nasalization_for_nasal()`, consonants with a locus
   `svara_f2_locus_equation()`.
7. Make sure both synthesis paths reach it: the free `svara_ph_synth_dispatch()` and the pooled
   `svara_synthctx_synthesize()` both route by class, so a new class (not just a new phoneme) needs
   an arm in each plus a `svara_ph_synth_*` / `svara_synthctx_*` pair.
8. Add tests to [`tests/phoneme.tcyr`](tests/phoneme.tcyr) and cross-check the output against
   `rust-old/src/phoneme.rs`.

## Benchmarks

All performance-related changes must include benchmark results. Benches live in
`benches/*.bcyr` and are auto-discovered:

```bash
cyrius bench
```

To record a run into `benches/history.csv` (one row per benchmark per run, so regressions stay
visible across commits):

```bash
./scripts/bench-history.sh
```

Update [`docs/benchmarks.md`](docs/benchmarks.md) when the numbers move, and note the host and
toolchain version with them. **Never write a clock-overhead figure into a comment or a doc** —
`lib/bench.cyr` measures the timer floor and subtracts it from every sample, and the value spans
a ~230× range across supported targets. Read it from `bench_clock_overhead_ns()`.

Per-sample DSP loops must use `bench_run_batch1/2` (one clock pair around a batch), not `bench_run`
— they are sub-microsecond, so per-call clock overhead would otherwise dominate.

## Releasing

`VERSION` at the repo root is the single source of truth; `cyrius.cyml` reads it via
`${file:VERSION}`. Use `./scripts/version-bump.sh <version>`, then update `CHANGELOG.md`,
[`docs/development/state.md`](docs/development/state.md) and
[`docs/development/roadmap.md`](docs/development/roadmap.md), and regenerate the distlib with
`cyrius distlib` so `dist/svara.cyr` and `dist/svara.deps` match the tag. CI verifies that
`VERSION`, the manifest and the pushed tag all agree.

## License

By contributing, you agree that your contributions will be licensed under GPL-3.0.
