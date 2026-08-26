# svara — Roadmap

> **What is still to do**, in release order. State lives in
> [`state.md`](state.md); shipped detail lives in [`../../CHANGELOG.md`](../../CHANGELOG.md).
>
> Organised by **release**, not by milestone. ⛔ marks work blocked on something
> outside this repo, with the blocker named; everything else can be started
> today. Shipped lanes are compressed into the table below, with a few kept in
> full at the tail — those are the ones where the *plan* was wrong in a way worth
> remembering.
>
> **Two lanes are open and one is parked.** 3.6.0 (perf) has work available now.
> 3.7.0 (retire `rust-old/`) is **parked on dhvani**: everything svara can do is
> done, and the oracle stays until a downstream consumer builds against the
> bundle.

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
| **3.1.3** | 2026-08-26 | **P1 audit sweep.** Five defects reachable through the public API — a SIGSEGV, three process aborts, a silent NaN — plus a constructor returning an error code as a pointer and an infinite loop. Per-sample arena use in `render_planned` **1,121 → 33 B** (34×) untoned, **1,505 → 113 B** (13×) toned. Output bit-for-bit identical. 713 assertions / 20 suites. [ADR-0001](../adr/0001-signed-index-and-float-conversion-hazards.md). |
| **3.2.0** | 2026-08-26 | **Container serde.** Serialized surface 8 → 13 types: SmoothedParam, Rng, F0Point (new), ProsodyContour (`Vec<SvF0Point>`), PhonemeSequence (`Vec<SvPhonemeEvent>`). Round-trips asserted behaviourally — a restored sequence renders bit-identical audio. The boundary is decided in [ADR-0002](../adr/0002-serialization-boundary.md), because the Rust had no skip policy to inherit and the toolchain cannot decode a nested struct field. 745 assertions. |
| **3.3.0** | 2026-08-26 | **State companions.** The nine engine types ADR-0002 excluded from direct derive each got a flat companion + `save`/`restore`. Derived types 13 → 22, suite 745 → 826. Restore is measured behaviourally, and its two limits (naad's internal state; filter delay lines) are pinned by tests that assert the divergence exists. |
| **3.3.1** | 2026-08-26 | **Docs, CI, stale claims.** CI ran four steps and none of the gate it documents; it now runs `cyrius audit` + fuzz + deny, asserts the toolchain matches the pin, and checks `dist/` is current. Four Rust-era docs rewritten (incl. the threat model, which asserted mitigations 3.1.3 disproved); four pre-port ADRs re-homed as 0003–0006; `.gitignore` stopped hiding `benches/history.csv`. |
| **3.3.2** | 2026-08-26 | **Glottal restore is exact.** 3.3.0 documented a limitation that was not real — naad derives accessors on `NoiseGenerator` and `Lfo`, so their state was reachable all along. `SvGlottalState` now carries it; the test flipped from asserting divergence to asserting identity. Lesson recorded in ADR-0002: a foreign type lacks a **codec**, not accessors. |
| **3.4.0** | 2026-08-26 | **Structured logging (M-log).** sakshi tracing behind `-D LOGGING`, off by default and compiled out when off; four coarse entry points, never per-sample. Spans are token-based because every entry point has early returns. `scripts/check-logging.sh` builds and runs both ways — `cyrius test` does not forward `-D`, so `cyrius audit` only ever compiled the OFF half. |
| **3.5.0** | 2026-08-26 | **Redundant input delay line collapsed.** The bank's eight per-slot `x1`/`x2` buffers all held the same two values — verified by 28,672 cross-slot comparisons before changing anything. Now two scalars: −6.0% formant `process_sample`, −4.2% `process_block`, bit-identical. A `#inline` experiment was measured, rejected and recorded. |
| **3.5.1** | 2026-08-26 | **Five oracle test gaps closed** (`tests/oracle.tcyr`, 69 assertions, no defects found) — quality levels had **no** suite or bench at all. Also found that `cyrius audit` prints `scope: src` and never lints tests or benches; CI does now. |
| **3.5.2** | 2026-08-26 | **The oracle's preserve-first gate.** Five runnable `examples/` (there were none); the third v2.0.1 quirk annotated in code; the recovery incantation (`git show <tag>:rust-old/…`) recorded and verified; and the last NaN hole closed by [ADR-0007](../adr/0007-reject-nan-formant-parameters.md) — a deliberate divergence, since the oracle accepts a NaN formant and emits a buffer of NaN having returned Ok. |
| **3.5.3** | 2026-08-26 | **Documentation sweep.** The headline perf figure had been wrong since 3.0.0 — "0.56 µs/sample, ~40× real-time" summed glottal + formant + tract, but the tract bench **contains** the formant bank; measured end to end it is **0.35 µs, ~64×**. `SECURITY.md` still said "1.x: Yes" while 3.0.0–3.1.2 carry five reachable defects. Roadmap made future-facing, `rust-old/` parked on dhvani. |

**M-serde (value types) — done 2026-07-03.** Cyrius ships `#derive(Serialize)`
(emits `Type_to_json` / `_from_json_str` over bayan's typed JSON DOM); toolchain
6.3.40 added f64-field support. Eight pure-scalar public types derive it with
round-trip coverage (`tests/serde.tcyr`, 22 tests): `Formant`, `VowelTarget`,
`VoiceProfile`, `EffortParams`, `PhonemeEvent`, `Nasalization`,
`VoiceOnsetTime`, `RenderProgress`. The static data tables stayed as embedded
pure functions — a synth library shouldn't carry a runtime data file, and they
were never serde in the Rust either. Container types are [3.2.0](#320--container-serde).

---

## 3.6.0 — hot-path memory and SIMD

**M-perf, what is left.** Three rounds have shipped — 3.1.0's control-rate glides,
3.1.3's removal of the per-sample allocation, and 3.5.0's collapse of the
redundant input delay line. A same-machine head-to-head
([`../benchmarks-rust-v-cyrius.md`](../benchmarks-rust-v-cyrius.md)) put the
per-DSP-unit gap against the Rust at **10–38×**.

⚠ **Measure the right thing.** A bump allocation is a pointer add, so removing
1.1 KB of it per sample moved `render_planned` only 20.6 → 18.6 ms — the cost
that mattered was *arena bytes*, which no timer can see. And a single run on this
host is not trustworthy: one showed `glottal next_sample` moving 79 → 121 ns on
untouched code. Build both trees, interleave the runs, take the minimum. Anything
trading memory for time needs the `tests/allocbudget.tcyr` treatment, not a bench
row.

⚠ **Record negative results.** 3.5.0 measured `#inline` on the four per-sample
quality predicates at **+0.7%** and reverted it; `src/lod.cyr` says so, to stop
the next person re-trying it. A change with no measured effect does not earn a
place in the source.

### Ready to start

- [ ] **Pool the per-note render buffers.** `phoneme.cyr` allocates ~16× per note,
      and `pool.cyr` exists but is **not wired into the render path** — verified:
      `svara_pool_*` appears only in `src/main.cyr`'s smoke; `sequence.cyr` and
      `render.cyr` never reference it. 3.1.3 did this for the *per-sample* path;
      this is the remaining *per-note* one, same measurement method.
- [ ] **Tract per-sample chain** (~275 ns/sample: formant bank + nasal +
      subglottal + lip + feedback). The SIMD half is blocked below; what is
      available now is reducing memory traffic, which is what has actually paid
      every time.

### ⛔ Blocked on the Cyrius stdlib

- [ ] **`svara_ph_buf_to_vec` doubles its result vec from empty**, so every render
      leaves roughly one dead copy of its own output in the arena — ~16 of the 33
      remaining bytes/sample. Needs `vec_with_capacity` in `lib/vec.cyr`.
      **Do not hand-build a vec header in svara**: that couples the library to
      stdlib internals for a 2× win on an already-34×-improved number.

### ⛔ Blocked on hisab

- [ ] **`calc_monotone_cubic` allocates three internal arrays per call** (~80
      bytes) — the entire remaining cost of the toned render path. Needs an
      `_into` form upstream, or coefficients computed once per contour. Pinned at
      hisab 2.11.2; revisit on any hisab bump.

### ⛔ Blocked on Cyrius SIMD codegen — expected in a later 6.5.x

The toolchain is scheduled to improve SIMD in the 6.5.x line. **Revisit all three
of these when it lands**; until then they are not worth attempting.

- [ ] **SIMD the formant biquad bank.** Prototyped as a bit-identical `f64v4`
      (AVX2) 8-slot bank with a `simd_has_avx2()`-guarded fallback. It passed
      tolerance but bought only **~5%**: the loop is **memory-bound** — the SOA
      state shuffle and vector-op call overhead dominate, and the ptr/value
      `f64v4` API cannot keep lane state in registers across samples. Reverted.
      3.5.0 then got **−6%** from the same insight with no SIMD at all, which is
      the measure of how little the arithmetic was costing.
- [ ] **Register-resident block SIMD** — a `process_block` holding the 8-slot
      state in vector registers across the sample loop. This is the one that needs
      codegen not round-tripping each `f64v4` op through memory.
- [ ] **FMA + block audit** — `f64v4_fmadd` in every biquad/mix inner loop, and
      the glottal→tract→formant chain running in blocks.

**Gate:** tolerance-parity `.tcyr` stays green; any AVX2 path is
`simd_has_avx2()`-guarded with a scalar fallback byte-identical to today;
`bench-history.csv` records before/after per-op deltas; no regression on
non-AVX2 hosts.

---

## 3.7.0 — retire `rust-old/`

> **Everything svara can do here is done.** The remaining gate is not svara's:
> **dhvani has not been updated to the Cyrius svara yet.** Until a downstream
> consumer builds against `dist/svara.cyr`, the oracle stays in the tree. This
> lane is parked, not in progress.

**Why it is parked rather than done.** naad puts it plainly: *"Deleting it is the
last step of the port, not the first."* goonj records its own oracle as "cleared
for deletion (verified: build, all suites and the bundle check pass with it
removed)" and still holds, on the same gate. And naad 2.2.1 — after a complete,
green, apparently finished port — had to go **back** to the Rust to close 182
untested public functions, because transcribing expectations from its own output
would have frozen an existing bug in as correct. svara hit the smaller version of
that in 3.5.1.

**The port itself is not the obstacle.** A name-by-name sweep of every Rust `pub`
item found **zero missing counterparts**; the two apparent gaps are explained
renames (`DEFAULT_AMPLITUDES: [f32;5]` → `SVARA_AMP_0..4`; `context_mut` folded
into `svara_pool_context`, Cyrius having no `&`/`&mut` split). Enum parity is
exact on all ten, `Phoneme` at 101/101. No golden files, data fixtures or licence
text exist only there.

### ✅ Preserve-first — complete

Everything that needed the Rust *readable* was done in 3.2.0–3.5.2, so the
directory is no longer load-bearing for any scheduled work:

- The Rust **serde contract** transcribed (3.2.0) — 30 `Serialize` types, 3
  `#[serde(default)]`, **zero** `#[serde(skip)]`.
- The **five oracle test gaps** closed (3.5.1) — `tests/oracle.tcyr`, expectations
  re-derived from the Rust and cited by line.
- The **five examples** ported (3.5.2) — `examples/`, built and run by CI.
- The **third v2.0.1 quirk** annotated in code (3.5.2).
- The open **NaN-formant parity question** resolved (3.5.2,
  [ADR-0007](../adr/0007-reject-nan-formant-parameters.md)).
- The **recovery incantation** recorded and verified (3.5.2):
  `git show <tag>:rust-old/src/<module>.rs` works from every tag. **Cite a tag,
  not a bare path**, in anything written from here on.

### ⛔ The gate — not svara's to close

- [ ] **Consumer-green: dhvani (or vansh) builds and passes against
      `dist/svara.cyr`.** dhvani still needs updating to the Cyrius svara; that
      work lives in dhvani's repo, on dhvani's schedule. naad's own roadmap names
      svara as *its* consumer-green gate, so the same exercise unblocks both.
      Per naad, it is the exercise that finds surface gaps a library's own suite
      cannot.

### When the gate clears

- [ ] **Final parity sweep, closed out in the CHANGELOG** — goonj's 2.0.4 was
      "the last audit run while the Rust oracle was still in the working tree"
      and found seven behavioural divergences.
- [ ] **Verify green with it moved aside** (goonj's literal precondition):
      `cyrius audit` exits 0, every suite passes, `cyrius distlib` and
      `cyrius bench` run, `./scripts/check-logging.sh` passes.
- [ ] **Sweep the ~61 non-`dist` references** — rewrite `src/` and `tests/`
      headers to past tense, fix `docs/benchmarks-rust-v-cyrius.md`'s
      `cd rust-old && cargo bench`, fix `CONTRIBUTING.md`'s live instruction to
      cross-check phoneme work against `rust-old/src/phoneme.rs`, drop
      `.gitignore`'s `rust-old/target/`. Regenerate `dist/svara.cyr`.
- [ ] **Delete.** Recovers 223 MB of working tree (8,785 tracked lines; the bulk
      is untracked `target/`).

---

## v1.0 — hardening

- [ ] **Security audit** — `docs/audit/YYYY-MM-DD-audit.md`; the directory does
      not exist yet. Its prerequisite, the threat-model rewrite, landed in 3.3.1,
      and `SECURITY.md` was rewritten per-release in 3.5.3.
- [ ] **Decide the three preserved v2.0.1 quirks** — unapplied per-vowel spectral
      tilt; `set_speed_quotient` ignored by the Rosenberg pulse; `tone` honoured
      only by `render_planned`. All three are now annotated at their call sites
      (3.5.2), so the decision can be made without the oracle. Each needs its own
      test when fixed.
- [ ] **Consumer-green** — the same gate as [3.7.0](#370--retire-rust-old).

---

## Out of scope for v1.0

- **Bit-exact cross-arch transcendental output.** Tolerance parity instead —
  ganita's transcendentals are per-arch. See [`state.md`](state.md).
- **Non-American-English vowel targets.** The Hillenbrand data is US English;
  other languages and dialects need their own
  ([ADR-0005](../adr/0005-formant-data-source.md)).
- **Fixing the three v2.0.1 quirks** — deciding them is a v1.0 item; *fixing* one
  changes output and belongs in a minor of its own, with its own tests.

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

