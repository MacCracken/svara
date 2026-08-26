# Benchmarks: Rust vs Cyrius

> svara **v3.0.1** synthesis parity benchmark — the criterion benches in
> `rust-old/benches/benchmarks.rs` run against the CYRIUS port's hot-path benches
> (`benches/hotpath.bcyr`), on the same machine, 2026-07-06.
>
> ⚠ **This is a dated head-to-head, not a live scoreboard.** The measurement is
> preserved as taken; the Cyrius side has moved since. 3.1.0 cut the diphthong
> 5.42 → 0.94 ms (below the Rust oracle's 1.09 ms) and 3.1.3 removed the
> per-sample allocation. For current Cyrius figures see
> [`benchmarks.md`](benchmarks.md) and `benches/history.csv`; re-running the Rust
> side needs `rust-old/`, which is scheduled for retirement (roadmap 3.6.0), so
> this table is likely to be the last of its kind.
>
> - **Rust**: criterion 0.5, `--release`. Deps from crates.io: hisab 1.2 (`num`,
>   `calc`), naad 1. f32/f64 mixed, LLVM auto-vectorized.
> - **Cyrius**: cycc **6.4.11**, `bench.cyr`. f64 throughout, scalar codegen (no
>   SIMD used yet).
> - **Platform**: x86_64 Linux. Comparisons are per identical DSP unit; the
>   Cyrius side is a fresh run (this supersedes the "~3–5×" estimate in
>   `benchmarks.md`, which understated the real gap by roughly an order of
>   magnitude).

## Head-to-Head (matched DSP units)

| Operation | Rust | Cyrius | Ratio | Notes |
|-----------|------|--------|-------|-------|
| **formant filter** — 1024 samples | 4.83 µs | 186 µs | **38×** | the 8-wide SOA biquad bank; LLVM vectorized, Cyrius scalar + f64-op overhead |
| **vocal tract** — 1024 (Full) | 19.3 µs | 376 µs | **19×** | formant + nasal + subglottal + lip + feedback per sample |
| **glottal source** — 1024 | 5.50 µs | ~85 µs¹ | ~15× | Rosenberg/LF excitation |
| **phoneme /s/** (fricative) | 28.6 µs | 467 µs | 16× | noise-excitation path |
| **phoneme /a/** (vowel) | 85.1 µs | 880 µs | 10× | 0.05 s render (~2205 samples) + per-note alloc |
| **phoneme /ai/** (diphthong) | 1.094 ms | **0.94 ms** | **0.86× ✓** | 3.1.0 control-rate coeffs — Cyrius now FASTER (Rust still re-solves per sample) |

¹ Cyrius glottal isn't block-benched; 83 ns/sample × 1024 ≈ 85 µs.

The formant bank (**38×**) is both the largest per-op gap and the biggest share
of every voiced sample — it dominates `vocal_tract` and thus `phoneme` and
`sequence` render. The diphthong ratio is the *smallest* (5×) precisely because
Rust pays the same per-sample coefficient re-solve (1.094 ms vs an 85 µs vowel —
13× more expensive *in Rust too*).

## Full Rust set (criterion, release — median)

| Benchmark | median |
|-----------|--------|
| glottal_source_1024 | 5.505 µs |
| glottal_whisper_1024 | 5.027 µs |
| glottal_creaky_1024 | 6.913 µs |
| formant_filter_1024 | 4.840 µs |
| formant_filter_block_1024 | 4.832 µs |
| vocal_tract_1024 | 19.31 µs |
| vocal_tract_into_1024 | 19.28 µs |
| vocal_tract_reduced_1024 | 13.27 µs |
| vocal_tract_minimal_1024 | 13.17 µs |
| phoneme_render_vowel_a | 85.11 µs |
| phoneme_render_female_vowel_a | 84.68 µs |
| phoneme_render_fricative_s | 28.62 µs |
| phoneme_render_diphthong_ai | 1.094 ms |
| sequence_render_5_phonemes | 308.7 µs |
| sequence_render_10_phonemes | 443.2 µs |

## Full Cyrius set (`cyrius bench benches/hotpath.bcyr`, cycc 6.4.11)

| Benchmark | avg |
|-----------|-----|
| glottal next_sample (Rosenberg) | 83 ns |
| glottal next_sample (whisper) | 79 ns |
| glottal next_sample (creaky) | 96 ns |
| formant filter process_sample | 176 ns |
| tract process_sample (Full) | 298 ns |
| formant filter process_block 1024 | 186.1 µs |
| tract synthesize_into 1024 | 376.2 µs |
| phoneme synth /a/ (vowel) | 880.6 µs |
| phoneme synth /s/ (fricative) | 467.3 µs |
| phoneme synth /ai/ (diphthong) | 0.94 ms (was 5.42 ms — 3.1.0 control-rate) |
| sequence render 3 phonemes | 3.463 ms |

## Analysis

### Why the formant bank is 38× (not the ~4× SIMD alone implies)

`svara_formant_bank_process` (`formant.cyr:257`) runs a fixed 8-slot SOA biquad
bank scalar: per sample, 8 × (`y = b0·in + b2·x2 − a1·y1 − a2·y2`) plus the
state shuffle and an amp-weighted sum — ~48 f64 ops + ~32 `load64`/`store64`.
Rust does the same 48 ops in **4.7 ns/sample**; Cyrius takes **182 ns/sample**.
That 38× decomposes into roughly:

| Factor | Est. share |
|--------|-----------|
| No SIMD (8 scalar lanes vs 4-wide AVX2) | ~4× |
| f64-op overhead — scalar path not fully collapsing `f64_mul`/`f64_add` to pipelined `mulsd`/`addsd` + no FMA | ~5–8× |
| `load64`/`store64` SOA traffic + no register residency across the loop | ~1.5–2× |

**SIMD was tried and it doesn't move this loop (measured, 3.1.0).** A bit-identical
`f64v4` version of the bank (two 4-lane groups, `simd_has_avx2()`-guarded) passed
tolerance but bought only **~5%** — because the loop is **memory-bound, not
compute-bound**. The arithmetic (row 1) was never the bottleneck; the state shuffle
(row 3, ~28 `load64`/`store64` per group) and the vector-op call overhead are, and
the ptr/value `f64v4` API round-trips each op through memory rather than keeping
lane state in registers. It was reverted. The real bit-identical levers here attack
the *memory traffic*: (a) collapse the redundant per-slot input delay line — `x1`/
`x2` are identical across all 8 slots since the input is shared, so two scalars
replace two 8-wide buffers (~16 fewer mem ops/sample); (b) register-resident block
SIMD, once the codegen keeps `f64v4` state in registers across the sample loop.
Tracked in roadmap **M-perf**.

### The diphthong: a shared algorithmic cost — fixed in 3.1.0

`svara_ph_synth_diphthong` re-solved formant filter coefficients
(`svara_tract_set_formants_from_target`) on **every** sample of the glide — and so
does the Rust oracle (hence Rust's own 1.094 ms vs an 85 µs vowel). **3.1.0** moves
Cyrius to **control-rate** coefficient updates (every 64 samples, held between): the
diphthong dropped **5.42 ms → 0.94 ms (5.8×)**, back in line with a steady vowel and
now **faster than the still-per-sample Rust path** (1.09 ms). Tolerance suite
unchanged. This is an algorithmic win the language-agnostic Rust path hasn't taken.

### Where Cyrius already wins

| Metric | Rust | Cyrius |
|--------|------|--------|
| Dependencies | hisab + naad + glam + criterion (crates.io) | hisab/naad/goonj (path/git), sovereign |
| Binary | dynamic + libc | single static `.cyr` bundle |
| Precision | f32/f64 mixed | f64 throughout |
| Build | cargo resolve + compile | `cyrius build`, near-instant |

### Roadmap

Tracked in [`development/roadmap.md`](development/roadmap.md) 3.5.0. **The
priorities in the original version of this section are out of date and inverted:**

- ✅ **Control-rate glide coefficients — shipped 3.1.0.** The diphthong outlier is
  gone (5.42 → 0.94 ms), now faster than the oracle.
- ✅ **Per-sample allocation — removed 3.1.3.** Not a listed item at the time; it
  turned out to matter more than SIMD, at 1,121 → 33 bytes of never-freed arena
  per output sample.
- ⚠ **SIMD the formant bank — DEFERRED, not P0.** A bit-identical `f64v4` AVX2
  bank was prototyped and reverted: it bought **~5%**, because the loop is
  memory-bound (the SOA state shuffle dominates), not compute-bound. The
  bit-identical lever that attacks the real bottleneck is collapsing the
  redundant per-slot input delay line — `x1`/`x2` are identical across all eight
  slots because the input is shared.
- ⏳ **Pool the per-note buffers** — still open.

Gate: tolerance `.tcyr` tests stay green; any AVX2 path is `simd_has_avx2()`
-guarded with a byte-identical scalar fallback.

### Reproduce

```sh
cd rust-old && cargo bench          # Rust baseline (crates.io hisab/naad)
cyrius bench benches/hotpath.bcyr   # Cyrius
```

⚠ The Rust half of that requires `rust-old/` and a cargo toolchain. When the
oracle is retired (roadmap 3.6.0) it becomes
`git show 3.3.0:rust-old/benches/benchmarks.rs`, and this table is the archive.
