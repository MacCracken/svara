# svara — Roadmap

> Release sequencing through v1.0. State lives in [`state.md`](state.md); this
> file is the order of work — what ships in which release, against what gate.
>
> Organised by **release**, not by milestone. A milestone that spans releases
> says so. Anything marked ⛔ is blocked on something outside this repo and the
> blocker is named; anything else can be started today.

---

## Shipped

| Release | Date | What |
|---|---|---|
| **0.1.0** | 2026-07-03 | Port scaffold. `cyrius port` ran, Rust moved to `rust-old/`, doc tree laid out. |
| **3.0.0** | 2026-07-03 | **Full surface parity.** All 19 Rust modules ported bottom-up (16 `.cyr`; `dsp.rs` folded into `error.cyr`, `math.rs` → the `ganita` stdlib), each gated by `.tcyr` tolerance tests. 634 assertions / 18 suites vs Rust's 213 tests, 11 hot-path benches, fmt/lint/docs clean. Locked decisions all honoured: naad-backend only, embedded data tables, tolerance parity, the three v2.0.1 quirks preserved. |
| **3.0.1** | 2026-07-05 | naad 2.1.0 → 2.1.1 (namespace de-collision). Pin-only. |
| **3.1.0** | 2026-07-06 | **Control-rate glide coefficients.** `svara_ph_synth_diphthong` re-solved the whole biquad bank every sample; now every 64. `/ai/` **5.42 ms → 0.94 ms (5.8×)**, faster than the Rust oracle. |
| **3.1.1** | 2026-08-26 | Toolchain pin 6.4.12 → 6.4.13. |
| **3.1.2** | 2026-08-26 | Toolchain 6.4.13 → **6.5.35**; hisab → 2.11.2, naad → 2.2.1, goonj → 2.0.4, sakshi → 2.4.11. Absorbed naad's `FILTER_* → NAAD_FILTER_*` break. `cyrius audit` works again. |
| **3.2.0** | 2026-08-26 | **Container serde.** Serialized surface 8 → 13 types: SmoothedParam, Rng, F0Point (new), ProsodyContour (`Vec<SvF0Point>`), PhonemeSequence (`Vec<SvPhonemeEvent>`). Round-trips asserted behaviourally — a restored sequence renders bit-identical audio. The boundary is decided in [ADR-0002](../adr/0002-serialization-boundary.md), because the Rust had no skip policy to inherit and the toolchain cannot decode a nested struct field. 745 assertions. |
| **3.5.0** | 2026-08-26 | **Redundant input delay line collapsed.** The bank's eight per-slot `x1`/`x2` buffers all held the same two values — verified by 28,672 cross-slot comparisons before changing anything. Now two scalars: −6.0% formant `process_sample`, −4.2% `process_block`, bit-identical. A `#inline` experiment was measured, rejected and recorded. |
| **3.4.0** | 2026-08-26 | **Structured logging (M-log).** sakshi tracing behind `-D LOGGING`, off by default and compiled out when off; four coarse entry points, never per-sample. Spans are token-based because every entry point has early returns. `scripts/check-logging.sh` builds and runs both ways — `cyrius test` does not forward `-D`, so `cyrius audit` only ever compiled the OFF half. |
| **3.3.2** | 2026-08-26 | **Glottal restore is exact.** 3.3.0 documented a limitation that was not real — naad derives accessors on `NoiseGenerator` and `Lfo`, so their state was reachable all along. `SvGlottalState` now carries it; the test flipped from asserting divergence to asserting identity. Lesson recorded in ADR-0002: a foreign type lacks a **codec**, not accessors. |
| **3.3.1** | 2026-08-26 | **Docs, CI, stale claims.** CI ran four steps and none of the gate it documents; it now runs `cyrius audit` + fuzz + deny, asserts the toolchain matches the pin, and checks `dist/` is current. Four Rust-era docs rewritten (incl. the threat model, which asserted mitigations 3.1.3 disproved); four pre-port ADRs re-homed as 0003–0006; `.gitignore` stopped hiding `benches/history.csv`. |
| **3.3.0** | 2026-08-26 | **State companions.** The nine engine types ADR-0002 excluded from direct derive each got a flat companion + `save`/`restore`. Derived types 13 → 22, suite 745 → 826. Restore is measured behaviourally, and its two limits (naad's internal state; filter delay lines) are pinned by tests that assert the divergence exists. |
| **3.1.3** | 2026-08-26 | **P1 audit sweep.** Five defects reachable through the public API — a SIGSEGV, three process aborts, a silent NaN — plus a constructor returning an error code as a pointer and an infinite loop. Per-sample arena use in `render_planned` **1,121 → 33 B** (34×) untoned, **1,505 → 113 B** (13×) toned. Output bit-for-bit identical. 713 assertions / 20 suites. [ADR-0001](../adr/0001-signed-index-and-float-conversion-hazards.md). |

**M-serde (value types) — done 2026-07-03.** Cyrius ships `#derive(Serialize)`
(emits `Type_to_json` / `_from_json_str` over bayan's typed JSON DOM); toolchain
6.3.40 added f64-field support. Eight pure-scalar public types derive it with
round-trip coverage (`tests/serde.tcyr`, 22 tests): `Formant`, `VowelTarget`,
`VoiceProfile`, `EffortParams`, `PhonemeEvent`, `Nasalization`,
`VoiceOnsetTime`, `RenderProgress`. The static data tables stayed as embedded
pure functions — a synth library shouldn't carry a runtime data file, and they
were never serde in the Rust either. Container types are [3.2.0](#320--container-serde).

---

## 3.5.0 — hot-path memory and SIMD

**M-perf, remainder.** A same-machine head-to-head
([`../benchmarks-rust-v-cyrius.md`](../benchmarks-rust-v-cyrius.md)) put the
per-DSP-unit gap at **10–38×**: formant bank 38×, tract 19×, glottal 15×, vowel
10×. Two rounds have shipped (3.1.0 control-rate glides, 3.1.3 the per-sample
allocation); what remains is memory traffic and vectorisation.

⚠ **Measure the right thing.** A bump allocation is a pointer add, so removing
1.1 KB of it per sample moved `render_planned` only 20.6 → 18.6 ms. The cost that
mattered was *arena bytes*, which no timer can see. Anything in this release that
trades memory for time needs the `tests/allocbudget.tcyr` treatment, not just a
bench row.

### Unblocked

- [x] **Collapse the redundant input delay line.** ✅ **Shipped 3.5.0.**
      Measured first: 28,672 cross-slot comparisons over 4,096 samples found
      **zero** divergence in `x1`/`x2`, with `y1` as the control proving the
      check discriminates. Now two scalars. **−6.0%** on formant
      `process_sample`, **−4.2%** on `process_block`, **−2.4%** on tract
      `process_sample`, bit-identical output, 128 fewer bytes of state per bank
      — about what the reverted AVX2 prototype bought, from the lever this file
      predicted would work.
- [ ] **Pool the per-note render buffers.** `phoneme.cyr` allocates ~16× per note
      and `pool.cyr` exists but is **not wired into the render path** (verified:
      `svara_pool_*` appears only in `src/main.cyr`'s smoke; `sequence.cyr` and
      `render.cyr` never reference it). 3.1.3 did this for the *per-sample* path;
      this is the remaining *per-note* one, same measurement method.
- [ ] **Tract per-sample chain** (~366 ns/sample: filter + nasal + subglottal +
      lip + feedback). The SIMD half of this item is blocked below.

      ⚠ **This item used to say "hoist coefficient recompute out of the
      per-sample loop". There is no coefficient recompute there** — the naad
      biquads are configured when formants are set, and the only per-sample state
      work is the two `SmoothedParam` advances, which must happen every sample by
      definition. Corrected 3.5.0 rather than left to send someone looking.

      Also measured and rejected in 3.5.0: marking the four per-sample quality
      predicates `#inline` moved the benchmark **+0.7%** — noise, wrong direction.
      `src/lod.cyr` records that; do not re-try without a measurement.

### ⛔ Blocked on the Cyrius stdlib

- [ ] **`svara_ph_buf_to_vec` doubles its result vec from empty**, so every render
      leaves roughly one dead copy of its own output in the arena — ~16 of the 33
      remaining bytes/sample. Needs a `vec_with_capacity` in `lib/vec.cyr`.
      **Do not hand-build a vec header in svara**: that couples the library to
      stdlib internals for a 2× win on an already-34×-improved number.

### ⛔ Blocked on hisab

- [ ] **`calc_monotone_cubic` allocates three internal arrays per call** (~80
      bytes) — the entire remaining cost of the toned render path. Needs an
      `_into` form upstream, or coefficients computed once per contour. Pinned at
      hisab 2.11.2; revisit on any hisab bump.

### ⛔ Blocked on Cyrius codegen

- [ ] **SIMD the formant biquad bank** — prototyped as a bit-identical `f64v4`
      (AVX2) 8-slot bank with a `simd_has_avx2()`-guarded fallback. It passed
      tolerance but bought only **~5%**: the loop is **memory-bound**, not
      compute-bound — the SOA state shuffle (~28 `load64`/`store64` per group) and
      vector-op call overhead dominate, and the ptr/value `f64v4` API cannot keep
      lane state in registers across samples. Reverted (also avoids a `simd` dep
      and a distlib-sidecar gap: `simd_has_avx2` isn't auto-folded).
- [ ] **Register-resident block SIMD** — a `process_block` holding the 8-slot
      state in vector registers across the sample loop. Needs codegen that doesn't
      round-trip each `f64v4` op through memory.
- [ ] **FMA + block audit** — fused multiply-add (`f64v4_fmadd`) in every
      biquad/mix inner loop, and the glottal→tract→formant chain running in
      blocks. `process_block` already does this for the bank; extend it through
      the tract. Same `simd` dependency question as above.

**Gate:** tolerance-parity `.tcyr` stays green; any AVX2 path is `simd_has_avx2()`
-guarded with a scalar fallback byte-identical to today; `bench-history.csv`
records before/after per-op deltas; no regression on non-AVX2 hosts.

---

## 3.6.0 — retire `rust-old/`

**The port is complete; the debt is the work that still reads the Rust.** A
name-by-name sweep of every Rust `pub` item found **zero missing counterparts** —
the two apparent gaps are explained renames (`DEFAULT_AMPLITUDES: [f32;5]` split
into `SVARA_AMP_0..4`; `context_mut` folded into `svara_pool_context`, since
Cyrius has no `&`/`&mut` split). Enum parity is exact on all ten enums, including
`Phoneme` at 101/101. No golden files, data fixtures or licence text exist only
there. Reference debt is trivial: **one** line-number-precise citation in svara's
own source (`src/formant.cyr:409` → `rust-old/src/formant.rs:479-486`); everything
else is module-level provenance that reads as history after deletion.

**Precedent: neither sibling has deleted.** goonj records its oracle as "cleared
for deletion (verified: build, all suites and the bundle check pass with it
removed)" yet holds on consumer-green. naad is blunter: *"Deleting it is the last
step of the port, not the first."* And naad 2.2.1 — after a complete, green,
apparently finished port — had to go **back** to the Rust to close 182 untested
public functions, because transcribing expectations from its own output would
have frozen an existing bug in as correct.

### Preserve first — each of these removes a reason to keep the directory

- [ ] **The serde contract** — done in [3.2.0](#320--container-serde). This is
      the largest one.
- [ ] **Port the five Rust examples** to `examples/*.cyr` — `basic`,
      `error_handling`, `prosody_patterns`, `streaming`, `voice_comparison` (269
      lines). svara has no `examples/` directory at all; `docs/examples/` holds
      only a `.gitkeep` while `CLAUDE.md` advertises "Runnable examples". This is
      goonj's stated precondition, verbatim.
- [ ] **Annotate the third v2.0.1 quirk at its call site.** Two are annotated
      (`src/glottal.cyr:141`, `src/phoneme.cyr:659`); the third — `Tone` honoured
      only in `render_planned` — is described once in `state.md` and annotated
      nowhere in code. Its Rust baseline is in `rust-old/src/sequence.rs`.
- [ ] **Resolve or freeze the open parity question at `src/formant.cyr:409`** —
      `svara_formant_validate` accepts a NaN formant frequency because Rust's
      `f.frequency <= 0.0 || f.frequency >= nyquist` accepts it too. Either accept
      it permanently in ADR-0001 with the Rust quoted inline, or tighten it under
      ADR-0002. Deciding needs the oracle.
- [ ] **Record the recovery incantation** in ADR-0001 and `CONTRIBUTING.md`,
      following goonj ADR-0002: after deletion the Rust is reachable as
      `git show 3.1.3:rust-old/src/<module>.rs`. Verified working — all ten tags
      carry the full tree.

### Close the test gaps while the oracle is readable

Five Rust-asserted scenarios are asserted nowhere in Cyrius. Re-derive
expectations **from `rust-old/`**, not from svara's own output — that is naad
2.2.1's method, and the reason for it.

- [ ] **Quality levels are never exercised.** `svara_tract_set_quality` appears in
      no suite and no bench; `tests/lod.tcyr` covers only the predicates and
      `tests/tract.tcyr` only asserts the default is Full. Every quality-conditional
      branch in `svara_tract_process_sample` runs Full-only under test. Rust
      asserted finite non-silent output at Full/Reduced/Minimal and Minimal ≠ Full.
- [ ] **Non-zero jitter/shimmer is never asserted to perturb.** The glottal suite
      sets both to 0 to keep goldens deterministic.
- [ ] **`svara_glottal_set_breathiness` is referenced in no suite.**
- [ ] **No test feeds synthesized speech into the analyzer.** `tests/spectral.tcyr`
      analyses synthetic sines only; Rust checked that a male /a/ has more energy
      at F1 (768 Hz) than at 200 Hz — the one cross-module acoustic-correctness
      check.
- [ ] **Seven `bridge` maps are tested on neither side** —
      `vibrato_depth_from_valence`, `intonation_from_emotion`,
      `f0_range_scale_from_arousal`, `f0_peak_from_prominence`, `jitter_from_age`,
      `spectral_tilt_from_distance`, `lombard_f0_shift`. Not a port regression;
      naad closed exactly this class in 2.2.1.

### ⛔ The hard gate

- [ ] **Consumer-green** — dhvani or vansh builds and passes against
      `dist/svara.cyr`. Not svara's to close alone; it is the gate both siblings
      are holding on, and per naad it is the exercise that finds surface gaps the
      suite cannot. naad's own roadmap names svara as *its* consumer-green gate.

### Then remove

- [ ] **Final parity sweep, closed out in the CHANGELOG** — goonj's 2.0.4 was
      "the last audit run while the Rust oracle was still in the working tree" and
      found seven behavioural divergences.
- [ ] **Verify green with it moved aside** (goonj's literal precondition):
      `cyrius audit` exits 0, 20 suites pass, `cyrius distlib` and `cyrius bench`
      run.
- [ ] **Sweep the 61 non-`dist` references** — rewrite `src/` and `tests/` headers
      to past tense, fix `docs/benchmarks-rust-v-cyrius.md`'s
      `cd rust-old && cargo bench`, fix `CONTRIBUTING.md`'s two live instructions
      to cross-check phoneme work against `rust-old/src/phoneme.rs`, drop
      `.gitignore`'s `rust-old/target/`. Regenerate `dist/svara.cyr`.
- [ ] **Delete.** Recovers 223 MB of working tree (8,785 tracked lines; the bulk
      is untracked `target/`).

---

## v1.0 — hardening

- [ ] **Security audit** — `docs/audit/YYYY-MM-DD-audit.md`. The directory does
      not exist. Depends on the threat-model refresh in
      [3.1.4](#314--documentation-ci-and-stale-claim-sweep): the current model
      claims parameter-validation mitigations that 3.1.3 disproved and
      supply-chain controls (`cargo deny`, crates.io-only) that do not apply.
- [ ] **Decide the three preserved v2.0.1 quirks.** Unapplied per-vowel spectral
      tilt; `set_speed_quotient` ignored by the Rosenberg pulse; `Tone` honoured
      only in `render_planned`. Each needs its own test when fixed. Requires the
      annotation work in [3.6.0](#360--retire-rust-old) to be done first.
- [ ] **Consumer-green** — shared gate with 3.5.0.

---

## Out of scope for v1.0

- **Bit-exact cross-arch transcendental output.** Tolerance parity instead —
  ganita's transcendentals are per-arch. See [`state.md`](state.md).
- **Non-American-English vowel targets.** The Hillenbrand data is US English;
  other languages and dialects need their own targets
  ([ADR-0005](../adr/0005-formant-data-source.md)).
- **Populating `docs/examples/`** as a separate concern — the runnable examples
  land in `examples/` in [3.6.0](#360--retire-rust-old), ported from the Rust.
  Either point `CLAUDE.md` at `examples/` or drop the `docs/examples/` reference.

---

## Shipped lanes — what actually happened

Kept because in all three the *plan* was wrong in a way worth remembering, not
just because the work is done.

## 3.4.0 — structured logging

✅ **Shipped 2026-08-26.** M-log. svara emitted no diagnostics at all; it now
routes tracing through sakshi, off by default behind `-D LOGGING` and compiled
out entirely when off. Four coarse entry points instrumented — sequence render,
batch render, synthesis-context render, pool render — **never per-sample**.

Three things worth carrying forward:

- **Spans are token-based, not paired.** Every instrumented entry point has early
  returns (`svara_synthctx_synthesize` has seven), and a plain enter/exit pair
  leaks a span on each of them. sakshi's stack is 16 deep, so leaks would poison
  the log slowly rather than fail loudly. `svara_span_leave(token)` unwinds to a
  recorded depth. Asserted with 40 consecutive failing renders.
- ⚠ **sakshi does not level-gate span events** — `_sk_emit_span` writes
  regardless of `sakshi_set_level`, because a span is structural rather than a
  log line. svara gates them itself, or `SVARA_LOG=off` would still stream every
  ENTER/EXIT pair.
- ⚠ **`cyrius test` does not forward `-D`.** `cyrius audit` therefore only ever
  compiles the logging-OFF half. `scripts/check-logging.sh` builds and runs both
  ways and is wired into CI; without it the whole `#ifdef LOGGING` half of the
  codebase would be built by nobody. It also asserts structurally that no log or
  span call sits on the per-sample path — verified to discriminate by injecting
  one and watching it fail.

---

## 3.2.0 — container serde

✅ **Shipped 2026-08-26. M2, delivered — but not as this file described it.** Two of the milestone's
premises were false and had to be measured:

1. **There was no Rust skip policy to inherit.** The oracle has **30
   `Serialize`-deriving types, 3 `#[serde(default)]`, and zero `#[serde(skip)]`**.
   Rust serialized everything because `VocalTract` holds a
   `naad::filter::BiquadFilter` that is itself `Serialize` in the Rust naad;
   Cyrius naad exposes opaque handles with no codec, so the policy had to be
   decided. It is, in [ADR-0002](../adr/0002-serialization-boundary.md).
2. **The toolchain reaches less than 6.4.13's changelog implies.** `Vec<f64>` and
   `Vec<flat-derive-struct>` round-trip; a **nested `#derive`-struct field does
   not, in either direction** — it is laid out inline while
   `#derive(accessors)` gives it pointer semantics, so the encoder emits the
   pointer reinterpreted as the first member, and the decoder returns zeros
   regardless. That rules out 6 of the 9 container types this file named.

**Shipped:** `SvSmoothedParam`, `SvRng`, `SvF0Point` (new — the raw 16-byte
`(time, value)` pair given a type), `SvProsodyContour` (`Vec<SvF0Point>`),
`SvPhonemeSequence` (`Vec<SvPhonemeEvent>`). Serialized surface 8 → 13 types;
suite 713 → 745; every render path still bit-for-bit identical to 3.1.2.

Round-trips are asserted **behaviourally** — a restored `Rng` reproduces 32 draws
exactly, a restored contour traces the same curve at 21 points, a restored
`PhonemeSequence` renders bit-identical audio.

`SvProsodyContour`'s interpolation scratch moved to module level: `#derive(Serialize)`
has no skip attribute, so a scratch pointer would have leaked a heap address into
the JSON and come back dangling. That also answered the "skip the seven 3.1.3
scratch fields" item — the other six live on types that are not serialized.

**`Quality` needed no work.** It is a `var SVARA_*` integer, not a struct, and
already round-trips wherever it appears as a field. Same for the other 12 Rust
`Serialize` enums.

---

## 3.3.0 — state companions for the engine types

✅ **Shipped 2026-08-26.** The nine types ADR-0002 excluded from direct derive
each got a flat state companion plus `save` / `restore`. Derived types 13 → 22;
serde suite 54 → 135 assertions; whole suite 745 → 826.

`SvSpectrum` needed no companion — it was already flat and only wanted the
annotations. `SvFormantFilter` needed none either: its state is
(formants, sample_rate), and the tract now records its formants, so
`svara_tract_restore` rebuilds the filter without a separate type.

**Released as a minor, not the planned patch** — ten new public types and sixteen
new public functions.

⚠ **3.3.0 also shipped a limitation that was not real** — see 3.3.2. It claimed
naad's `NoiseGenerator` cell and `Lfo` phase could not be read out, and the test
suite *asserted that divergence*, which made a wrong belief look measured. naad
derives accessors on both structs. Before concluding a foreign handle is opaque,
check for accessors: what it lacks is a codec.

Two findings worth carrying forward:

- **The SynthesisContext holds almost nothing across calls.**
  `svara_synthctx_synthesize` resets the tract and re-applies every glottal
  parameter from the voice at the top of each call, so its whole persistent state
  is the noise PRNG and the sample rate. `SvSynthCtxState` is three fields. The
  same is true of the pool and the batch renderer, which is why all three take
  the voice as a `restore` argument rather than inlining `SvVoiceProfile`'s nine
  fields into three more schemas.
- **A fresh tract used to misreport its own formants** — the constructor built
  the filter from the schwa targets while setting the record to a zeroed scratch
  vec, so a new tract claimed five 0 Hz formants. Caught by a control assertion
  written to prove the *rejection* tests were not vacuous; it failed instead.

---

## 3.3.1 — documentation, CI, and stale-claim sweep

✅ **Shipped 2026-08-26.** No source change. Lane was numbered 3.1.4 and was
overtaken by 3.2.0 and 3.3.0 shipping first.

**The finding with real consequences: CI ran almost none of the gate.**
`ci.yml` had four steps — install, deps, build, `cyrius test` — while
`CONTRIBUTING.md` names `cyrius audit` as the pre-PR gate and `state.md`
documents eight. `release.yml` inherited it, so a release could ship having run
only the tests. CI now runs `cyrius audit`, `cyrius fuzz` and `cyrius deny`, plus
a toolchain-matches-the-pin assertion *before* `cyrius deps` (whose failure used
to surface two steps later as an opaque path error) and a check that `dist/` is
regenerable and current.

Also done: the four Rust-era docs rewritten (`threat-model.md` — the v1.0 audit
prerequisite, and the one asserting mitigations 3.1.3 disproved;
`architecture/overview.md`; `integration-guide.md`; `guides/testing.md`); the
four pre-port ADRs re-homed to `docs/adr/` as **0003–0006** and indexed;
`.gitignore`'s Rust-era `*.csv` retired so `benches/history.csv` can finally be
committed; `CLAUDE.md`'s mission statement written; and the stale claims retired
(bridge count — README's **19** was right and `state.md` wrong; `benchmarks.md`
11 → 13 rows; the head-to-head doc dated and its priorities corrected; README's
consumer tag; `getting-started.md`'s test target).

⚠ **Carried forward: `cyrlint`'s "0 untracked deferrals" is not a completeness
signal.** It scans only `.cyr` sources, never Markdown, matches twelve literal
terms, and counts one "tracked" if the *same line* also contains `CHANGELOG`,
`roadmap`, `docs/`, `issue`, `See `, `v6.` or `v5.` — a substring test, not a
check that the roadmap says anything. Every item in this lane was invisible to
it. Do not cite it as evidence that there is no deferred work.

---

