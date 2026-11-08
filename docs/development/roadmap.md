# svara — Roadmap

> **What is still to do**, in release order. State lives in [`state.md`](state.md),
> and everything shipped lives in [`../../CHANGELOG.md`](../../CHANGELOG.md).
>
> ⛔ marks work blocked outside this repo, with the blocker named. Everything else
> can start today.

| Release | Lane | Status |
|---|---|---|
| **3.7.0** | [Retire `rust-old/`](#370--retire-rust-old) | next; the gate needs a decision |
| **3.8.x** | [Performance](#38x--performance): loop shape, block forms, per-note memory, the refactors 3.6.3 left | ready |
| **3.9.x** | [The preserved Rust quirks](#39x--the-preserved-rust-quirks), and the diphthong glide | needs a decision per item |
| **3.x.x** | [SIMD, and work blocked upstream](#3xx--simd-and-work-blocked-upstream) | unscheduled |

---

## 3.7.0 — retire `rust-old/`

3.6.3 ran the differential, the oracle's last planned use, which is the order goonj
took: sweep at 2.1.3, delete at 2.2.0. Every divergence it found is recorded in
[ADR-0009](../adr/0009-what-the-3-6-3-audit-leaves-open.md). The port itself is not
the obstacle. A name-by-name sweep of every Rust `pub` item found zero missing
counterparts.

### The gate, measured 2026-11-07

This used to read "dhvani has not been updated to the Cyrius svara", which stopped
being true. dhvani links `dist/svara.cyr` through its `voice_synth` wrapper, and
prani, shabda and shabdakosh link it too.

| Consumer | Pins today | Against svara 3.6.1, cyrius 6.7.18, hisab 3.4.1, goonj 2.3.1, naad 2.3.3 |
|---|---|---|
| dhvani 2.2.6 | svara 3.5.5, cyrius 6.6.10 | **Builds, and 65 of 65 suites pass**, in a scratch copy. `tests/voice_synth.tcyr` calls svara directly, 61 assertions |

naad measured the same stack and found no failure that traces to it. What is left for
dhvani is moving its pins, which is dhvani's to do. They are below the 6.7.2 floor.

- [ ] **Decide whether a measurement in a scratch copy clears the gate**, or whether
      a consumer has to move its pins first. naad holds on the second reading, and
      goonj deleted on the first.

### Steps

- [ ] **An ADR on the model of goonj's ADR-0006**: delete the tree and keep its bar.
      It names the last tag that has the Rust, keeps the `rust-old/src/x.rs:NN`
      citations as an audit trail, and gives the recipe to rebuild the oracle.
- [ ] **Confirm nothing lives only there.** Rust tests never ported, the `Cargo.lock`
      versions, and files with no counterpart.
- [ ] **Verify green with it moved aside:** `cyrius audit`, every suite,
      `cyrius distlib`, `cyrius bench` and `./scripts/check-logging.sh`.
- [ ] **Rewrite the live instructions**: `CONTRIBUTING.md`'s cross-check,
      `CLAUDE.md`'s rules, `docs/benchmarks-rust-v-cyrius.md`'s
      `cd rust-old && cargo bench`, and `.gitignore`'s `rust-old/target/`.
- [ ] **Delete**, and regenerate `dist/svara.cyr`.

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

⚠ **Record negative results.** 3.5.0 measured `#inline` on the four per-sample quality
predicates at +0.7% and reverted it, and `src/lod.cyr` says so. A change with no
measured effect does not earn a place in the source.

- [ ] **Tract per-sample chain** (about 200 ns/sample: formant bank, nasal, subglottal,
      lip, feedback). Move the accessors into locals around one loop. Reducing memory
      traffic is what has paid every time.
- [ ] **Call naad's block forms** where the tract's feedback allows it.
      `filter_biquad_process_buffer` and `noise_fill_buffer` are real loops since naad
      2.3.3, and a per-sample caller gets none of that. naad's ADR-0009 says what a
      form owes the path it stands for.
- [ ] **Pool the per-note render buffers.** `phoneme.cyr` allocates about 16× per note,
      and `pool.cyr` is not wired into the render path: `sequence.cyr` and `render.cyr`
      never reference it.
- [ ] **`render_planned` recomputes per-event values on every sample.** Hoist what is
      fixed for the event out of the sample loop.
- [ ] **`f0_at` refills both contour buffers on every call**, and the toned path calls
      it per sample.
- [ ] **A voice is cloned, and a progress object built, per event.**
- [ ] **`set_formants_from_target` copies into the tract's record on every sample.**
      3.6.3 builds the target in scratch so a rejected one cannot reach the record,
      then copies it over. That costs `render_planned` 4.3% of its instructions,
      measured against the old write-in-place path. Validate the five target values
      first, then write in place.

### Refactors the 3.6.3 sweep left

Each changes the shape of a hot path, so each needs the measurement above.

- [ ] **Two copies of per-class synthesis.** `svara_ph_synth_*` and `svara_synthctx_*`
      repeat each other. Share what is the same, and leave what differs to 3.9.x.
- [ ] **13 test files define their own `approx`.** Move it to one include.
- [ ] **The crossfade's fade-out loop is written twice**, and the stress table four
      times.

**Gate:** `scripts/sweep/compare.py` shows no moved value, and `tests/allocbudget.tcyr`
holds.

---

## 3.9.x — the preserved Rust quirks

Four behaviours svara keeps because the oracle has them, and one where svara left the
oracle without a decision. Each needs a decision, recorded in an ADR. A fix changes
output, so each one gets a minor of its own and its own test.

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
