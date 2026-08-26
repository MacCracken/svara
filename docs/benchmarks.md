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

## Results (x86_64, single core, 2026-08-26, cycc 6.5.35)

| Benchmark | Avg | vs 3.1.0 | Notes |
|---|---|---|---|
| glottal `next_sample` (Rosenberg) | ~82 ns | = | periodic pulse + jitter/shimmer |
| glottal `next_sample` (whisper) | ~86 ns | +4% | noise-only excitation |
| glottal `next_sample` (creaky) | ~102 ns | = | LF + irregular period timing |
| formant filter `process_sample` | ~179 ns | +1% | 8× SOA bandpass bank + DC blocker |
| tract `process_sample` (Full) | ~295 ns | = | filter + nasal + subglottal + lip + feedback |
| formant filter `process_block` 1024 | ~194 µs | −3% | ~190 ns/sample amortized |
| tract `synthesize_into` 1024 | ~377 µs | +2% | glottal → tract, ~368 ns/sample |
| phoneme synth /a/ (vowel) | ~857 µs | −1% | 0.05 s render (~2205 samples) + alloc |
| phoneme synth /s/ (fricative) | ~458 µs | −3% | noise excitation path |
| phoneme synth /ai/ (diphthong) | ~921 µs | −2% | control-rate formant coeffs (3.1.0) |
| sequence render (3 phonemes) | ~3.36 ms | −1% | per-phoneme synth + variable crossfade |

A full glottal → formant → tract per-sample chain is ≈ 0.56 µs/sample, i.e.
roughly **40× real-time** at 44.1 kHz on one core.

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

## History

`benches/history.csv` accumulates one row per benchmark per run
(`timestamp,git_rev,benchmark,avg_us`) so regressions are visible across commits.
