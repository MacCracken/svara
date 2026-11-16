# svara — Benchmarks

Hot-path benchmarks for the Cyrius port, reproducing the intent of the Rust's 15
criterion benches (`git show 3.6.5:rust-old/benches/benchmarks.rs`). Source:
[`benches/hotpath.bcyr`](../benches/hotpath.bcyr).

## Running

```sh
cyrius bench                    # auto-discovers benches/hotpath.bcyr
./scripts/bench-history.sh      # runs + appends to benches/history.csv
```

## Measurement method

Per-sample DSP inner loops are **sub-microsecond**, so per-call `clock_gettime`
overhead would dominate a naive per-call timer. Those benches use
`bench_run_batch1/2`, which wraps **one** clock pair around a batch of 1000 calls
and averages over 200 batches (200,000 ops total) — the reported figure is honest
per-op cost. Block and full-render paths run long enough (>100 µs) that per-call
clock overhead is negligible, so they use `bench_run`.

⚠ **Do not write a clock-overhead figure down.** This document used to say
"~240 ns on this host". Since cyrius 6.5.19 `lib/bench.cyr` *measures* the floor
and subtracts it from every sample, and it prints what it measured:

```
[timer floor 1.341us per clock read, measured; subtracted from every sample]
```

**1.34 µs — 5.6× the number that had been written down**, and the spread across
supported targets is ~230× (a clock read is 15–32 ns on macOS arm64, ~3,550 ns on
aarch64 Linux). `bench_clock_overhead_ns()` returns the measured value; that is
the only correct place to read it from.

The batch-timed figures themselves did not move when the instrument changed,
because they never depended on the per-call floor — which is exactly why a stale
comment here was harmless in effect and still worth deleting.

## Results (x86_64, single core, 2026-11-16, cyrius 6.7.18, svara 3.8.2)

Each figure is the lowest of three `cyrius bench` runs, with load near 3.6 on 16 cores.
The 3.6.3 column is the previous table, measured the same way on 2026-11-07. Rows
marked ★ are the four Rust benches svara had no counterpart for until 3.8.2, on the
Rust's own inputs; their Rust medians are in
[`benchmarks-rust-v-cyrius.md`](benchmarks-rust-v-cyrius.md).

| Benchmark | 3.8.2 | 3.6.3 | Notes |
|---|---|---|---|
| glottal `next_sample` (Rosenberg) | ~48 ns | ~46 ns | periodic pulse + jitter/shimmer |
| glottal `next_sample` (whisper) | ~72 ns | ~70 ns | noise-only excitation |
| glottal `next_sample` (creaky) | ~64 ns | ~62 ns | LF + irregular period timing |
| formant filter `process_sample` | ~86 ns | ~130 ns | SOA bandpass bank, active slots only, + DC blocker |
| tract `process_sample` (Full) | ~135 ns | ~197 ns | **includes the formant bank**, + nasal + subglottal + lip + feedback |
| formant filter `process_block` 1024 | ~110 µs | ~152 µs | ~107 ns/sample amortized |
| tract `synthesize_into` 1024 | ~185 µs | ~254 µs | **the full chain** — glottal → tract, ~181 ns/sample |
| ★ tract `synthesize_into` 1024 (Reduced) | ~159 µs | — | no subglottal, no interaction (Rust: 13.27 µs) |
| ★ tract `synthesize_into` 1024 (Minimal) | ~155 µs | — | no nasal, subglottal, interaction or lip (Rust: 13.17 µs) |
| phoneme synth /a/ (vowel) | ~447 µs | ~591 µs | 0.05 s render (~2205 samples) + alloc |
| ★ phoneme synth /a/ female, 0.1 s | ~875 µs | — | 4410 samples (Rust: 84.68 µs) |
| phoneme synth /s/ (fricative) | ~168 µs | ~343 µs | noise excitation path |
| phoneme synth /ai/ (diphthong) | ~484 µs | ~628 µs | control-rate formant coefficients (3.1.0) |
| sequence render (3 phonemes) | ~1.88 ms | ~2.38 ms | per-phoneme synth + variable crossfade |
| ★ sequence render (10 phonemes) | ~5.05 ms | — | the Rust's ten events (Rust: 443.2 µs) |
| `render_planned` (3 phonemes) | ~10.6 ms | ~13.5 ms | per-sample formant interpolation + tract re-target |
| `render_planned` toned | ~13.2 ms | ~16.6 ms | as above, plus a per-sample prosody contour lookup |

**3.8.2 took the tract chain to 0.744x of 3.8.1's retired instructions and 0.709x
its cycles**, bit-identical. A component profile put 59% of `tract process_sample`
in the formant bank, which ran all eight slots for five formants. Four changes:

| Change | tract instructions | tract cycles |
|---|---:|---:|
| the bank skips slots past `count` while that is exact: no coefficients there, no state, finite input | -18.0% | -16.6% |
| the tract keeps its quality's four flags as bits, set with the quality, instead of four calls a sample | -3.3% | -8.6% |
| a converged, finite smoother returns at once | -3.7% | -3.5% |
| `svara_formant_filter_process_sample` written as one expression, so cyrius inlines it | -0.6% | -1.0% |

The quality tiers gain most: Reduced 0.702x instructions and 0.619x cycles, Minimal
0.695x and 0.672x.

⚠ **Most of the gain since 3.5.3 is the toolchain, not svara.** The pin went from
cycc 6.5.35 to cyrius 6.7.18 in that time. Against 3.6.2 on the same toolchain, 3.6.3
retires 3.1% *more* instructions in `render_planned` and 2.6% more in the toned
render, because a correctness fix copies each sample's formant target. Every other
probe is within 0.01%. `scripts/sweep/compare.py` measures that with retired-
instruction counters, which this host's timers cannot resolve. See the 3.6.3
CHANGELOG entry and roadmap 3.8.x.

**3.8.1 cut `render_planned` to 0.845x of 3.8.0's retired instructions, and the toned
render to 0.840x; both run in about 0.80x the cycles.** The output is bit-identical.
The rows in the table above predate it. A component profile said where the
instructions went, per 20 renders of the probe's three phonemes (3,196 M in all):

| Component | M instructions | Share |
|---|---:|---:|
| `svara_tract_set_formants_from_target`, nearly all of it the bank rebuild's `cos`, `sin` and `sinh` | 1,832 | 57% |
| `svara_trajectory_formants_at_into` | 664 | 21% |
| the glottal source and tract per sample | 468 | 15% |
| the toned path's `svara_prosody_f0_at` | +627 on the toned render | |

Four changes took it down:

| Change | `render_planned` instructions |
|---|---:|
| memoize each bank slot's trig on its exact frequency, bandwidth and sample rate | -6.9% |
| validate the target, then write it straight into the tract's record | -5.8% |
| compute per-event values once per event, not per sample | -2.0% |
| skip retuning the nasal notch to the place it already has | -0.8% |

and on the toned render, filling the tone contour's buffers once per event takes
2.6% more. `svara_trajectory_formants_at_into` was left as it is: its cost is the
ten-component Catmull-Rom arithmetic, with no exact shortcut.

Per-release deltas are not tracked in this table — `benches/history.csv` has one
row per benchmark per run, which is the honest place for them. A single run on
this host is not trustworthy on its own: an unrelated run showed
`glottal next_sample` moving 79 → 121 ns on code that had not changed. **Compare
by building both trees and interleaving the runs**, taking the minimum of each;
that is how 3.5.0's −6% was established.

A full glottal → tract per-sample chain is **≈ 0.18 µs/sample**, i.e. roughly
**125× real-time** at 44.1 kHz on one core — measured end to end by
`tract synthesize_into 1024`, not derived (the table's ~181 ns/sample at 3.8.2;
3.6.3 measured 0.25 µs and 3.5.3 0.35 µs).

⚠ **Do not add the per-unit rows together to get the chain cost.** `tract
process_sample` **already contains** `formant filter process_sample` — the tract
calls the formant filter (`src/tract.cyr`). Summing glottal + formant + tract
double-counts the bank and was how this figure was computed until 3.5.3, which
understated svara by ~1.6×. The chain is measured directly by
`tract synthesize_into 1024` (glottal → tract, per sample); the sum
glottal + tract agrees with it to within 1 ns.

Every row is within a few percent of the 3.1.0 run — host noise, not a toolchain
effect. The 6.4.13 → 6.5.35 bump changed the *instrument* (measured timer floor,
self-sizing `bench_run` batches) without moving svara's numbers, because the
per-sample benches already batch-timed.

### Reading the numbers

- The diphthong render used to be the outlier (~5.4 ms) because it re-derived
  formant targets on every sample as the vowel glides. **3.1.0 moved that to a
  control rate of 64 samples** — it now sits at ~0.92 ms, in line with a steady
  vowel and *faster* than the Rust oracle, which still re-solves per sample.
- A same-machine head-to-head ([`benchmarks-rust-v-cyrius.md`](benchmarks-rust-v-cyrius.md))
  puts the real per-DSP-unit gap at **10–38×** (formant bank 38×, tract 19×,
  glottal 15×, vowel 10×) — NOT the "~3–5×" earlier estimated here. Cyrius emits
  scalar code (no auto-vectorization) and the SOA bank loop LLVM vectorized runs
  scalar `load64`/`store64` + f64-op overhead here. Addressable via SIMD + FMA —
  see the roadmap's performance lane. Still comfortably real-time in absolute terms.

⚠ **The two `render_planned` rows are the ones a timer measures badly.** 3.1.3
took that path from 1,121 to 33 bytes of never-freed arena per output sample
(34x), and wall clock moved only 20.6 → 18.6 ms — a bump allocation is a pointer
add. The metric that mattered is pinned in `tests/allocbudget.tcyr` as marginal
arena bytes per sample, not here. Anything that trades memory for time needs the
same treatment.

## History

`benches/history.csv` accumulates one row per benchmark per run
(`timestamp,git_rev,benchmark,avg_us`) so regressions are visible across commits.
