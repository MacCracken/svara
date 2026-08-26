# svara — Benchmarks

Hot-path benchmarks for the Cyrius port, reproducing the intent of the 15
criterion benches in `rust-old/benches/benchmarks.rs`. Source:
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

## Results (x86_64, single core, 2026-08-26, cycc 6.5.35, svara 3.5.3)

| Benchmark | Avg | Notes |
|---|---|---|
| glottal `next_sample` (Rosenberg) | ~85 ns | periodic pulse + jitter/shimmer |
| glottal `next_sample` (whisper) | ~81 ns | noise-only excitation |
| glottal `next_sample` (creaky) | ~103 ns | LF + irregular period timing |
| formant filter `process_sample` | ~170 ns | 8× SOA bandpass bank + DC blocker |
| tract `process_sample` (Full) | ~285 ns | **includes the formant bank**, + nasal + subglottal + lip + feedback |
| formant filter `process_block` 1024 | ~202 µs | ~197 ns/sample amortized |
| tract `synthesize_into` 1024 | ~373 µs | **the full chain** — glottal → tract, ~364 ns/sample |
| phoneme synth /a/ (vowel) | ~849 µs | 0.05 s render (~2205 samples) + alloc |
| phoneme synth /s/ (fricative) | ~439 µs | noise excitation path |
| phoneme synth /ai/ (diphthong) | ~897 µs | control-rate formant coefficients (3.1.0) |
| sequence render (3 phonemes) | ~3.33 ms | per-phoneme synth + variable crossfade |
| `render_planned` (3 phonemes) | ~18.4 ms | per-sample formant interpolation + tract re-target |
| `render_planned` toned | ~21.5 ms | as above, plus a per-sample prosody contour lookup |

Per-release deltas are not tracked in this table — `benches/history.csv` has one
row per benchmark per run, which is the honest place for them. A single run on
this host is not trustworthy on its own: an unrelated run showed
`glottal next_sample` moving 79 → 121 ns on code that had not changed. **Compare
by building both trees and interleaving the runs**, taking the minimum of each;
that is how 3.5.0's −6% was established.

A full glottal → tract per-sample chain is **≈ 0.35 µs/sample**, i.e. roughly
**64× real-time** at 44.1 kHz on one core — measured end to end by
`tract synthesize_into 1024`, not derived.

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
  see roadmap **M-perf**. Still comfortably real-time in absolute terms.

⚠ **The two `render_planned` rows are the ones a timer measures badly.** 3.1.3
took that path from 1,121 to 33 bytes of never-freed arena per output sample
(34x), and wall clock moved only 20.6 → 18.6 ms — a bump allocation is a pointer
add. The metric that mattered is pinned in `tests/allocbudget.tcyr` as marginal
arena bytes per sample, not here. Anything that trades memory for time needs the
same treatment.

## History

`benches/history.csv` accumulates one row per benchmark per run
(`timestamp,git_rev,benchmark,avg_us`) so regressions are visible across commits.
