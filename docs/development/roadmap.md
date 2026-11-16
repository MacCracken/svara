# svara — Roadmap

> **What is still to do**, in release order. State lives in [`state.md`](state.md),
> and everything shipped lives in [`../../CHANGELOG.md`](../../CHANGELOG.md).
>
> ⛔ marks work blocked outside this repo, with the blocker named. Everything else
> can start today.

| Release | Lane | Status |
|---|---|---|
| **3.8.x** | [Performance](#38x--performance): loop shape, block forms, per-note memory, the refactors 3.6.3 left | next; 3.8.0 shipped the streaming render, 3.8.1 `render_planned`, 3.8.2 the tract chain |
| **3.9.x** | [The preserved Rust quirks](#39x--the-preserved-rust-quirks), and the diphthong glide | needs a decision per item |
| **3.x.x** | [SIMD, and work blocked upstream](#3xx--simd-and-work-blocked-upstream) | unscheduled |

---

## 3.8.x — performance

prani measured its port 15.7× slower than its Rust oracle, and float width is not the
cause ([ADR-0008](../adr/0008-the-synthesis-core-stays-f64.md)). naad found the same
arithmetic costs 22.7 ns through accessors and one call per sample, and 9.5 ns in one
loop in locals. Shape is the lever svara still has.

⚠ **Measure the right thing.** A bump allocation is a pointer add, so removing 1.1 KB
of it per sample moved `render_planned` only 20.6 → 18.6 ms. The cost that mattered
was arena bytes, which no timer can see. Anything trading memory for time needs the
`tests/allocbudget.tcyr` treatment, and a timing gap needs checking against retired
instructions.

**Profile by component first.** 3.8.1 counted retired instructions per call for
each piece of `render_planned`'s loop and found 57% in the bank rebuild's trig, where
the roadmap had pointed at a 4.3% copy. `docs/benchmarks.md` has the table, and the
method is a scratch probe that replays each component the number of times a render
does, timed with `scripts/sweep/compare.py`'s `counters`.
`svara_trajectory_formants_at_into` is the largest piece left there, at about a
quarter of the loop now, and has no exact shortcut.

**Call overhead counts in cycles.** 3.8.2 replaced four predicate calls a sample with
bits read once: 3.3% of the tract's instructions but 8.6% of its cycles. On cyrius
6.7.18 `#inline` takes a body under 32 tokens with no `var` or control flow, and
derived accessors are already inlined, so reading fields into locals buys nothing.

⚠ **Record negative results.** 3.5.0 measured `#inline` on the four per-sample quality
predicates at +0.7% and reverted it, and `src/lod.cyr` says so. A change with no
measured effect does not earn a place in the source.

- [ ] **Lay the formant bank out one record per slot** instead of seven parallel
      arrays, so a slot's loads share one base pointer. 3.8.2 estimated about 4% of
      the bank, which is now about half of `tract process_sample`. Every bank user
      changes, `allocbudget.tcyr`'s slot checks with them.
- [ ] **Call naad's block forms** where the tract's feedback allows it.
      `filter_biquad_process_buffer` and `noise_fill_buffer` are real loops since naad
      2.3.3, and a per-sample caller gets none of that. naad's ADR-0009 says what a
      form owes the path it stands for.
- [ ] **Pool the per-note render buffers.** `phoneme.cyr` allocates about 16× per note,
      and `pool.cyr` is not wired into the render path: `sequence.cyr` and `render.cyr`
      never reference it.
- [ ] **A voice is cloned, and a progress object built, per event.**

### Refactors the 3.6.3 sweep left

Each changes the shape of a hot path, so each needs the measurement above.

- [ ] **Two copies of per-class synthesis.** `svara_ph_synth_*` and `svara_synthctx_*`
      repeat each other. Share what is the same, and leave what differs to 3.9.x.
- [ ] **13 test files define their own `approx`.** Move it to one include.
- [ ] **The crossfade is written twice over.** `svara_seq_crossfade` has its fade-out
      loop twice, and `svara_seq_xf_segment` (3.8.0) repeats all of it for the stream.
      `tests/stream.tcyr` pins the two to the same bits, so a shared step is safe to
      try. The stress table is down to `svara_seq_stress_f0` and
      `svara_seq_event_duration` (3.8.0 and 3.8.1), with one copy left in
      `svara_batch_render_with_progress`.

**Gate:** `scripts/sweep/compare.py` shows no moved value, and `tests/allocbudget.tcyr`
holds.

---

## 3.9.x — the preserved Rust quirks

Four behaviours svara keeps because the oracle has them, and one where svara left the
oracle without a decision. Each needs a decision, recorded in an ADR. A fix changes
output, so each one gets a minor of its own and its own test.

The oracle left the tree in 3.7.0. Read it at tag `3.6.5`, and rebuild and run it with
[ADR-0011](../adr/0011-retire-rust-old-keep-its-bar.md)'s recipe when a decision needs
the Rust's output rather than its source.

- [ ] **Unapplied per-vowel spectral tilt.**
- [ ] **`set_speed_quotient` is ignored by the Rosenberg pulse.**
- [ ] **`tone` is honoured only by `render_planned`.**
- [ ] **Plosive timing depends on the render path.** `svara_ph_synth_plosive` uses the
      VOT fractions, and `svara_synthctx_plosive` uses a fixed n/3 and n/6. Both match
      the oracle: `git show 3.5.2:rust-old/src/phoneme.rs`, `synthesize_plosive` and
      `synthesize_plosive_ctx`.
- [ ] **The diphthong glide is 7× to 69× louder than Rust's.** Since 3.1.0 svara
      rebuilds the formant bank every 64 samples. Rust rebuilds it, and resets its
      state, on every sample, so its resonance never builds. 3.1.0 called the change
      perceptually identical without measuring the oracle. Matching Rust means a filter
      reset per sample. See
      [ADR-0009](../adr/0009-what-the-3-6-3-audit-leaves-open.md), RN-DIV.

---

## 3.x.x — SIMD, and work blocked upstream

### SIMD, to be measured again

This used to wait on codegen, and that claim is out of date. naad measured a 64-filter
bank going from 10.5 to 3.2 ns per filter with `f64v256_*` in structure-of-arrays form,
3.3×, on cyrius 6.7.18. svara's own prototype bought about 5% through the ptr/value
`f64v4` API, and 3.5.0 then got 6% with no SIMD at all. Measure before planning.

- [ ] **SIMD the formant bank**, structure-of-arrays over `f64v256_*`. naad leaves SIMD
      dispatch to dhvani, so agree that first.
- [ ] **Register-resident block SIMD**, holding the 8-slot state in vector registers
      across the sample loop.
- [ ] **FMA and block audit**: `f64v4_fmadd` in the biquad and mix loops, and the
      glottal → tract → formant chain running in blocks.

**Gate:** tolerance-parity `.tcyr` stays green, any AVX2 path is
`simd_has_avx2()`-guarded with a scalar fallback byte-identical to today,
`benches/history.csv` records before and after, and non-AVX2 hosts do not regress.

### ⛔ Blocked upstream

- [ ] ⛔ **`svara_ph_buf_to_vec` doubles its result vec from empty**, so every render
      leaves about one dead copy of its output in the arena. **Blocked on
      `vec_with_capacity` in `lib/vec.cyr`**, still absent at cyrius 6.7.18. Do not
      hand-build a vec header in svara.
- ⛔ **Codegen.** naad measured 9.5 ns against Rust's 2.8 on the same loop. It is the
  largest single factor left, and it belongs to cyrius.
- [ ] ⛔ **The generated JSON decoder hangs on a stray `}` in a vec of structs.**
      `svara_validate_json` guards it since 3.6.3, and callers must call it.
      Blocked on cyrius `docs/development/issues/2026-11-07-derive-serialize-vec-struct-element-loop-hangs-on-stray-brace.md`.
      When the fix ships, run the issue's repro against that release, and run the
      fuzz target once with the check removed to confirm the decoder itself holds.
      Keep the check either way: it refuses only text svara never writes.

---

## Out of scope

- **Moving the core to f32.** Closed by
  [ADR-0008](../adr/0008-the-synthesis-core-stays-f64.md), which says what would reopen
  it.
- **Bit-exact cross-arch transcendental output.** Tolerance parity instead: ganita's
  transcendentals are per-arch. See [`state.md`](state.md).
- **Non-American-English vowel targets.** The Hillenbrand data is US English, and other
  languages and dialects need their own
  ([ADR-0005](../adr/0005-formant-data-source.md)).
