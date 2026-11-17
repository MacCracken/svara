# svara — Roadmap

> **What is still to do**, in release order. State lives in [`state.md`](state.md),
> and everything shipped lives in [`../../CHANGELOG.md`](../../CHANGELOG.md).
>
> ⛔ marks work blocked outside this repo, with the blocker named. Everything else
> can start today.

| Release | Lane | Status |
|---|---|---|
| **3.9.x** | [The preserved Rust quirks](#39x--the-preserved-rust-quirks), and the diphthong glide | next; needs a decision per item. The 3.8.x performance lane closed at 3.8.8, and its method moved to [`benchmarks.md`](../benchmarks.md#changing-a-hot-path) |
| **3.x.x** | [SIMD, and work blocked upstream](#3xx--simd-and-work-blocked-upstream) | unscheduled |

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
- [ ] **Plosives and trills depend on the render path.** Since 3.8.7 the two paths share
      every run they write the same way (`svara_ph_*_run`), so what is left apart is
      what the Rust itself does differently:
      - `svara_ph_synth_plosive` uses the VOT fractions, an aspiration segment, a 0.30
        breathiness and a 0.15 voiceless tail, while `svara_synthctx_plosive` uses a
        fixed n/3 and n/6, no aspiration, 0.40 and 0.30. Both match the oracle:
        `git show 3.5.2:rust-old/src/phoneme.rs`, `synthesize_plosive` and
        `synthesize_plosive_ctx`.
      - `svara_ph_synth_trill` modulates its amplitude at 15 to 25 Hz, while the
        context renders a trill as an approximant. Both match the oracle too: at tag
        `3.6.5`, `rust-old/src/phoneme.rs:1033` and `:1468`.
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
      dispatch to dhvani, so agree that first. Since 3.8.4 the scalar bank keeps one
      record per slot, which suits a scalar loop. A SIMD bank needs four slots'
      coefficients side by side, so it brings its own layout, such as records of four
      slots, and is measured against this one.
- [ ] **Register-resident block SIMD**, holding the 8-slot state in vector registers
      across the sample loop.
- [ ] **FMA and block audit**: `f64v4_fmadd` in the biquad and mix loops, and the
      glottal → tract → formant chain running in blocks.

**Gate:** tolerance-parity `.tcyr` stays green, any AVX2 path is
`simd_has_avx2()`-guarded with a scalar fallback byte-identical to today,
`benches/history.csv` records before and after, and non-AVX2 hosts do not regress.

### ⛔ Blocked upstream

- [ ] ⛔ **naad's per-sample biquad and noise.** `filter_biquad_process_sample` retires
      about 335 instructions a sample and its block form's loop 195; white
      `noise_next_sample` 240 against 130. svara calls both per sample, inside the
      tract's feedback loop. Rebuilt in a scratch probe the way their block loops are
      written, they left svara's output byte-identical and took the chain to 0.947x its
      instructions, the glottal source to 0.911x. **Blocked on naad
      `docs/development/issues/2026-11-16-per-sample-biquad-and-noise-cost-more-than-their-block-loops.md`.**
      When it ships, bump naad and sweep. Do not carry svara's own copies of naad's
      functions.

- [ ] ⛔ **A vec sized up front.** Since 3.8.3 a sample vec adopts the buffer it was
      rendered into, through an Allocator of svara's own (`svara_vec_adopt`). Two costs
      remain:
      - the buffer is rounded up to a vec's power-of-two capacity, up to twice its
        samples;
      - each vec carries 56 bytes of Allocator and state.

      **Blocked on `vec_with_capacity`, or a vec over an existing buffer, in
      `lib/vec.cyr`**, still absent at cyrius 6.7.18. It is asked for in cyrius
      `docs/development/issues/2026-11-11-vec-keeps-outgrown-buffer-and-aborts.md`, in
      svara's 2026-11-16 section. When it ships, drop the Allocator and size each
      buffer to its samples. Do not hand-build a vec header in svara.
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
