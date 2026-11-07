# svara — Roadmap

> **What is still to do**, in release order. State lives in [`state.md`](state.md),
> and everything shipped lives in [`../../CHANGELOG.md`](../../CHANGELOG.md).
>
> ⛔ marks work blocked outside this repo, with the blocker named. Everything else
> can start today.

| Release | Lane | Status |
|---|---|---|
| **3.6.3** | [P(-1) sweep](#363--p-1-sweep): audit, hardening, refactor, optimization, repair | next |
| **3.7.0** | [Retire `rust-old/`](#370--retire-rust-old) | after 3.6.3 |
| **3.8.x** | [Performance](#38x--performance): loop shape, block forms, per-note memory | ready |
| **3.9.x** | [The preserved Rust quirks](#39x--the-preserved-rust-quirks) | needs a decision per quirk |
| **3.x.x** | [SIMD, and work blocked upstream](#3xx--simd-and-work-blocked-upstream) | unscheduled |

---

## 3.6.3 — P(-1) sweep

A full pass over the code: audit it, harden it, refactor it, optimize it, and repair
what turns up. It runs before 3.7.0 because the differential needs the oracle.

**Why it earns a release.** naad's sweep fixed 45 defects that its 2,418 assertions
caught none of, and goonj's found 21 public accessors reading or writing out of
bounds. svara's last audit was 3.1.3. It has never run a differential against the
compiled oracle or any fault injection, and its fuzz target is still the scaffold
stub.

**The bar.** Audio and JSON on valid input do not move, checked with
`scripts/sweep/compare.py 3.6.2`, and any exception is named in the CHANGELOG. Every
fix has a test shown to fail without it.

### Audit

- [ ] **Differential against `rust-old/`.** Build the oracle in a scratch copy and
      drive both sides over every public entry point, comparing outcome class and
      value: hostile values for each numeric parameter, plus realistic input. goonj's
      hostile differential barely reached its valid paths, so it had to add realistic
      scenes before it could trust the result.
- [ ] **Line by line against `rust-old/`**, one pass per module cluster, each finding
      reproduced before it is listed: source (`glottal`, `rng`, `smooth`), filter
      (`formant`, `tract`, `lod`), speech (`phoneme`, `prosody`, `trajectory`,
      `voice`), render (`sequence`, `pool`, `render`), boundary (`error`, `logging`,
      `spectral`, `bridge`).
- [ ] **A real fuzz target.** `tests/svara.fcyr` calls no svara function, so
      `cyrius fuzz` has passed vacuously in CI since 3.3.1. Drive the public API, and
      run it under `--poison` and `--bounds`.
- [ ] **Fault injection.** Refuse the Nth allocation across the render pipelines,
      singly and for good.
- [ ] **All-zero sweep.** Every public fn with 0 for every argument. Each returns or
      faults at the null page, with no hang.
- [ ] **Execution coverage.** `cyrius coverage --exec` names what no suite runs.
- [ ] **Independent review of the merged diff**, by a reader who saw none of the work.

### Leads already in hand

- **Ownership.** Rust moves or copies what it keeps. `svara_prosody_make` keeps the
  caller's vec and `svara_sequence_push` keeps the caller's event, so a later write
  by the caller reaches them. naad found this class across its port and fixed it
  under its ADR-0005.
- **No sample-rate ceiling.** `svara_validate_sample_rate` asks only for positive and
  finite, and a sequence render at 3e9 Hz runs past 40 s. Decide whether to bound it.
- **`br_in_range` passes NaN**, so `tests/bridge.tcyr`'s range assertions cannot see one.
- **`#derive` setters are public** and bypass what a real setter validates.

### Refactor

- [ ] **Two copies of per-class synthesis.** `svara_ph_synth_*` and `svara_synthctx_*`
      repeat each other. Share what is the same, and leave what differs to 3.9.x.
- [ ] **13 test files define their own `approx`.** Move it to one include.
- [ ] **Scaffold leftovers.** `tests/svara.bcyr` benches a no-op.

### Optimization

Only what leaves output bit-identical. A change that reshapes a loop belongs to 3.8.x.

- [ ] **`f0_at` refills both contour buffers on every call**, and the toned path calls
      it per sample.
- [ ] Whatever the audit turns up. Measure with the counters in
      `scripts/sweep/compare.py`, because a timer on this host is often unusable.
- [ ] **Refresh `docs/benchmarks.md` and `benches/history.csv`** on a quiet machine.
      Both stop at 3.5.3.

### Deliverables

- [ ] **`docs/audit/YYYY-MM-DD-audit.md`**, the security audit, on the threat model
      rewritten in 3.3.1.
- [ ] **An ADR for what stays open**: accepted hostile-input gaps and the policy on a
      refused allocation, as naad's ADR-0006 and ADR-0007 do.
- [ ] **A "How it was audited" section in the CHANGELOG**, with method and counts.

---

## 3.7.0 — retire `rust-old/`

The 3.6.3 differential is the oracle's last planned use, which is the order goonj
took: sweep at 2.1.3, delete at 2.2.0. The port itself is not the obstacle. A
name-by-name sweep of every Rust `pub` item found zero missing counterparts.

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

**Gate:** `scripts/sweep/compare.py` shows no moved value, and `tests/allocbudget.tcyr`
holds.

---

## 3.9.x — the preserved Rust quirks

Four behaviours svara keeps because the oracle has them. Each needs a decision,
recorded in an ADR. A fix changes output, so each one gets a minor of its own and its
own test.

- [ ] **Unapplied per-vowel spectral tilt.**
- [ ] **`set_speed_quotient` is ignored by the Rosenberg pulse.**
- [ ] **`tone` is honoured only by `render_planned`.**
- [ ] **Plosive timing depends on the render path.** `svara_ph_synth_plosive` uses the
      VOT fractions, and `svara_synthctx_plosive` uses a fixed n/3 and n/6. Both match
      the oracle: `git show 3.5.2:rust-old/src/phoneme.rs`, `synthesize_plosive` and
      `synthesize_plosive_ctx`.

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
