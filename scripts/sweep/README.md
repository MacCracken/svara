# Sweep

Compares what svara does between a git ref and the working tree. Each tree is built
with its own toolchain pin, so it works across a cyrius bump.

```bash
scripts/sweep/compare.py 3.6.1
```

```bash
scripts/sweep/compare.py 3.6.1 --only values,json
```

It exits 1 if a difference is of a kind rounding cannot explain.

| Check | What it compares |
|---|---|
| `values` | exact f64 bit patterns: all 101 phonemes in three voices, the context and pool paths, glottal models, quality levels, sequences, spectra, prosody, trajectories, bridges, the RNG, crafted contours |
| `json` | bytes emitted by the 22 `#derive(Serialize)` types, and bytes emitted again after parsing |
| `hostile` | 11 hostile values through 58 entry-point and parameter combinations, one process per case, comparing exit code, length, error code and non-finite count |
| `counters` | retired instructions and cycles for fixed hot-path loops |

**Why it exists.** naad and goonj each found aborts at a toolchain bump while their
suites stayed green, and a sample sweep cannot see a change in serialized output.
Use it for every pin bump, and for any change that claims to move no result.

**Reading the counters.** Timing on a shared machine is often unusable. Retired
instructions do not depend on load. Equal instructions and a small cycle gap mean
code layout, and a change in instructions means the code changed.

**Adding to it.** A new `sec_*` in `sweep.cyr` needs a row in `SECTIONS` in
`compare.py`, and a new hostile entry needs `HOSTILE_ENTRIES` raised, plus a place in
`SUMMARISED` if it prints through `sum_vec` or `sum_buf`.
