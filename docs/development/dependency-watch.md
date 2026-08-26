# Dependency Watch

Track upstream dependencies for updates, breaking renames, and compatibility.

> Pins live in `cyrius.cyml`; resolved commits in `cyrius.lock`. This file is the
> *policy* and the *consumption map* — what svara actually calls, so a bump can be
> checked rather than hoped through. Current pins are mirrored in
> [`state.md`](state.md#dependencies).

## Direct dependencies

| Dep | Pin | What svara consumes |
|-----|-----|---------------------|
| [hisab](https://github.com/MacCracken/hisab) | 2.11.2 | `num_fft`, `num_neumaier_sum` (spectral.cyr) · `calc_monotone_cubic` (prosody.cyr) · `ease_in_out_smooth` (phoneme / trajectory / sequence) |
| [naad](https://github.com/MacCracken/naad) | 2.2.1 | `filter_biquad_new/process_sample/set_params/reset` + `NAAD_FILTER_NOTCH` / `NAAD_FILTER_BANDPASS` (tract.cyr) · `noise_new`/`noise_next_sample` + `NOISE_WHITE`, `modulation_lfo_new/next_value/set_frequency` + `MODULATION_LFO_SINE` + `Lfo_set_depth` (glottal.cyr) |
| [goonj](https://github.com/MacCracken/goonj) | 2.0.4 | **nothing directly** — the naad bundle references it, so it must resolve from svara's manifest |

## Transitive

| Dep | Pin | Arrives via |
|-----|-----|-------------|
| [sakshi](https://github.com/MacCracken/sakshi) | 2.4.11 | hisab (and goonj's logging backend) |

Pins are chosen to match what the bundles themselves pin — naad 2.2.1 pins hisab
2.11.2 and goonj 2.0.4; hisab 2.11.2 pins sakshi 2.4.11 — so a downstream consumer
(dhvani / vansh) that bundles svara **and** naad **and** hisab in one flat
namespace gets a coherent set rather than two versions of the same symbol.

## Toolchain

The Cyrius pin in `cyrius.cyml [package].cyrius` is the single source of truth.
CI reads it and hands it to the canonical installer — never hardcode a version in
YAML. Bumping it re-vendors all 29 stdlib files in `lib/` via `cyrius deps`.

## The failure mode this file exists to catch

svara's dependencies compile into **one flat namespace**. There is no module
system to disambiguate, so upstream de-collision work arrives as bare renames:

| Release | Rename | Reached svara? |
|---|---|---|
| naad 2.1.1 | `amplitude_to_db` / `db_to_amplitude` → `naad_*` | no |
| naad 2.1.3 | `ERR_*` → `NAAD_ERR_*` | no — svara's codes are `SVARA_ERR_*` |
| naad 2.2.0 | `FILTER_*` → `NAAD_FILTER_*` | **yes** — two call sites in `tract.cyr` |
| naad 2.2.0 | `VOICE_*` → `NAAD_VOICE_*` | no |
| naad 2.2.0 | `lerp` / `rms` / `peak` / `normalize` / `chromagram` / `crossfade_equal_power` → `naad_*` | no |
| naad 2.2.0 | `white_noise_sample`, `U32_MAXF` removed | no |

These break the **build**, which is the good case. The dangerous case is a rename
that silently *agrees* — naad renamed all eight `FILTER_*` constants precisely
because `nidhi` defines `FILTER_LOWPASS..NOTCH` at identical values, so a
flat-namespace collision between two enums that happen to match is invisible until
one side renumbers.

## Checking a bump

Before assuming a clean build, diff the used-symbol set against the new bundles:

```sh
cat lib/naad.cyr lib/hisab.cyr lib/goonj.cyr lib/sakshi.cyr \
  | grep -oE '^(fn|var|const|struct)[[:space:]]+[A-Za-z_][A-Za-z0-9_]*' | awk '{print $2}' | sort -u > /tmp/dep-symbols.txt
```

Intersect that with the identifiers in `src/`, `tests/` and `benches/`, subtract
svara's own definitions, and check every survivor against the *new* bundle. Filter
the result by hand — bare names like `peak` and `rms` show up as matches while
being local variables. Signatures matter too, not just presence.

Then run the gate: `cyrius audit` (green as of the 6.5.35 pin), or the individual
`cyrfmt --check` / `cyrlint` / `cyrdoc --check` / `cyrius tests tests` /
`cyrius fuzz` / `cyrius deny src/main.cyr` / `cyrius bench`.

## Security monitoring

- `cyrius deny <source>` validates the dependency set (currently 21 deps, 0 violations).
- `cyrius fuzz` drives `tests/svara.fcyr` against the public boundary.
- The toolchain installer verifies a published SHA256 (CVE-21) and, where present,
  an Ed25519 signature over `SHA256SUMS` (CVE-13). Both CI workflows install
  through it rather than hand-rolling `curl` + `tar`.

## Upgrade policy

- **Patch versions**: auto-merge if the gate passes.
- **Minor versions**: read the CHANGELOG's *BREAKING* / *Migration* sections
  before bumping, then re-run benchmarks (`./scripts/bench-history.sh`) — a minor
  can change floating-point rounding without changing behaviour (naad 2.2.0's
  `fit_polynomial` thin-QR port is the worked example).
- **Major versions**: full review, migration path, and an ADR if svara diverges
  from what `rust-old/` did.
