# svara — Roadmap

> **What is still to do**, in release order. State lives in [`state.md`](state.md),
> and everything shipped lives in [`../../CHANGELOG.md`](../../CHANGELOG.md).
>
> ⛔ marks work blocked outside this repo, with the blocker named. Everything else
> can start today.

| Release | Lane | Status |
|---|---|---|
| **3.8.x** | [Performance](#38x--performance): loop shape, block forms, per-note memory, the refactors 3.6.3 left | next; 3.8.0 shipped the streaming render, 3.8.1 `render_planned`, 3.8.2 the tract chain, 3.8.3 the sample vecs, 3.8.4 the formant bank's records, 3.8.5 the per-sample accessors, 3.8.6 closed two items by measurement |
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
6.7.18 `#inline` takes a body under 32 tokens with no `var` or control flow.

⚠ **An inlined accessor is not free.** A derived getter retires 2 instructions more
than a `load64`, a setter 6 more than a `store64`, and each keeps a jump. 3.8.4 wrote
the bank's slot record as `const` offsets and the bank went to 0.951x its instructions
and 0.897x its cycles. 3.8.5 did the same for the 84 accessor uses left on the per-sample
path, glottal source to DC blocker, and the chain went to 0.952x its instructions. `render_planned` moved
only to 0.992x: its loop's cost is the retargeting, which still reads through accessors
and is not worth rewriting for under 1% unless a profile says otherwise. The cyrius issue
is `docs/development/issues/2026-11-16-derived-accessor-costs-more-than-load-store.md`
in the cyrius repo; its fix would make the offsets unnecessary, not wrong.

⚠ **Record negative results.** 3.5.0 measured `#inline` on the four per-sample quality
predicates at +0.7% and reverted it, and `src/lod.cyr` says so. A change with no
measured effect does not earn a place in the source. 3.8.6 closed two items this way:
- **naad's block forms do not fit the tract.** Its interaction feedback ties every
  stage to the last output, so the subglottal resonator and the nasal notch run inside
  the loop. Noise filled ahead would put the generator past the samples consumed, which
  the glottal save / restore reads. What a block form would save is in naad's
  per-sample functions, so the item is blocked upstream, below.
- **The glottal source and tract `svara_sequence_render` builds per phoneme** cost
  41,310 instructions and 2,272 bytes: 0.43% of a 0.08 s /a/, and under a byte a sample
  against the render's 24. A reset that gives a fresh object's bits would be a second
  constructor to keep in step for that. Not taken; `svara_sequence_render_into` already
  avoids them.

### Refactors the 3.6.3 sweep left

Each changes the shape of a hot path, so each needs the measurement above.

- [ ] **Two copies of per-class synthesis.** `svara_ph_synth_*` and `svara_synthctx_*`
      repeat each other. Share what is the same, and leave what differs to 3.9.x.
- [ ] **The crossfade's reference.** Since 3.8.3 both sequence renders crossfade through
      `svara_seq_xf_segment`. `svara_seq_crossfade` is the Rust's shape, with its
      fade-out loop written twice, and only the tests call it: `tests/stream.tcyr` and
      `tests/allocbudget.tcyr` hold the step to it bit for bit. Either move it into a
      test include, or keep it in `src/` as the port's record of the Rust. The stress
      table is down to `svara_seq_stress_f0` and `svara_seq_event_duration`, which the
      batch render uses too since 3.8.3.

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
