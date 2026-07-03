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
overhead (~240 ns on this host) would dominate a naive per-call timer. Those
benches use `bench_run_batch1/2`, which wraps **one** clock pair around a batch of
1000 calls and averages over 200 batches (200,000 ops total) — the reported
figure is honest per-op cost. Block and full-render paths run long enough
(>100 µs) that per-call clock overhead is negligible, so they use `bench_run`.

## Results (x86_64, single core, 2026-07-03)

| Benchmark | Avg | Notes |
|---|---|---|
| glottal `next_sample` (Rosenberg) | ~82 ns | periodic pulse + jitter/shimmer |
| glottal `next_sample` (whisper) | ~82 ns | noise-only excitation |
| glottal `next_sample` (creaky) | ~100 ns | LF + irregular period timing |
| formant filter `process_sample` | ~175 ns | 8× SOA bandpass bank + DC blocker |
| tract `process_sample` (Full) | ~294 ns | filter + nasal + subglottal + lip + feedback |
| formant filter `process_block` 1024 | ~186 µs | ~182 ns/sample amortized |
| tract `synthesize_into` 1024 | ~375 µs | glottal → tract, ~366 ns/sample |
| phoneme synth /a/ (vowel) | ~880 µs | 0.05 s render (~2205 samples) + alloc |
| phoneme synth /s/ (fricative) | ~467 µs | noise excitation path |
| phoneme synth /ai/ (diphthong) | ~5.4 ms | re-solves formant targets **per sample** |
| sequence render (3 phonemes) | ~3.4 ms | per-phoneme synth + variable crossfade |

A full glottal → formant → tract per-sample chain is ≈ 0.55 µs/sample, i.e.
roughly **40× real-time** at 44.1 kHz on one core.

### Reading the numbers

- The diphthong render is the outlier (~5.4 ms) because it re-derives formant
  targets on every sample as the vowel glides — faithful to the Rust behavior,
  not a regression.
- These are ~3–5× the Rust/LLVM figures per sample: Cyrius emits scalar code
  (no auto-vectorization), and the SOA bank loop that LLVM vectorized runs as
  scalar `load64`/`store64` + accessor calls here. Still comfortably real-time.

## History

`benches/history.csv` accumulates one row per benchmark per run
(`timestamp,git_rev,benchmark,avg_us`) so regressions are visible across commits.
