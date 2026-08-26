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

## 3.1.4 — documentation, CI, and stale-claim sweep

**No source change; patch release.** Everything here is unblocked and cheap. The
common thread is that `cyrlint`'s "0 untracked deferrals" is accurate and nearly
meaningless as a completeness signal: it scans **only `.cyr` sources**, matches
twelve literal terms (`NOT_IMPLEMENTED`, `SCAFFOLD`, `TODO`, `FIXME`, `XXX`,
`deferred`, `follow-up`, `for now`, `not yet`, `later bite`, `future bite`,
`out of scope`), and counts one "tracked" if the *same line* also contains
`CHANGELOG`, `roadmap`, `docs/`, `issue`, `See `, `v6.` or `v5.` — a substring
test, not a check that the roadmap says anything. Markdown is never linted.

### CI actually runs the gate

- [ ] **`.github/workflows/ci.yml` has four steps** — install, `cyrius deps`,
      `cyrius build`, `cyrius test`. No fmt, lint, docs, fuzz, deny, bench or
      audit runs on any push or PR, while `CONTRIBUTING.md` names `cyrius audit`
      as the pre-PR gate and [`state.md`](state.md#quality-gate) documents eight
      gates. `release.yml` inherits the same thin gate. `cyrius audit` exits 0 on
      the 6.5.35 pin, so this is one step.
- [ ] **Assert the toolchain matches the pin** before `cyrius deps` fails two
      steps later with a path error, as hisab 2.11.2 does: `cyrius version` must
      equal `cyrius.cyml [package].cyrius`, and
      `~/.cyrius/versions/<pin>/lib` must exist.

### Four docs that never left Rust

`README.md` and `CONTRIBUTING.md` were swept for Cyrius in 2026-08-26; these four
were not, and still document a language this project does not use.

- [ ] **`docs/architecture/overview.md`** — a `math` module that doesn't exist,
      "stored as f32 for processing" (the port is f64 throughout), "~5,000×
      real-time" (measured ~40×), a `## no_std Support` section with
      `#![cfg_attr]` and `default-features = false`.
- [ ] **`docs/development/integration-guide.md`** — `use svara::prelude::*;`,
      `vec![0.0f32; 512]`, a Cargo feature table (`std` / `naad-backend` /
      `logging`) that `README.md` explicitly says does not exist.
- [ ] **`docs/guides/testing.md`** — `cargo test --all-features`, `make check`,
      criterion, `target/criterion/report/index.html`.
- [ ] **`docs/development/threat-model.md`** — `cargo deny` / `cargo audit` /
      "`deny.toml` restricts to crates.io only (no git dependencies)", for a
      project whose dependencies are now *entirely* git. Its §1 parameter-injection
      mitigations were disproved by 3.1.3 and it does not reference ADR-0001.
      **Prerequisite for the v1.0 security audit.**

### ADR housekeeping

- [ ] **Re-home the four pre-port ADRs.** `docs/architecture/adr-001-source-filter-model.md`
      … `adr-004-scope-boundaries.md` sit outside the `docs/adr/` scheme that
      `docs/adr/README.md` states, are absent from its index, and still reference
      `bridge.rs` and "~1,000× real-time". Meanwhile `docs/architecture/README.md`
      declares itself "_Empty._" while sitting beside them.
- [ ] **ADR-0001 says three raw `f64_to` sites remain; there are four** —
      `src/phoneme.cyr:714,715,716` and `src/sequence.cyr:296`. All four are
      correctly annotated in place; only the count is wrong.

### Stale claims to retire

- [ ] **`state.md` warns that README and CONTRIBUTING "are still the pre-port
      Rust files"** — they were rewritten on 2026-08-26. A reader trusting the
      state authority would redo finished work.
- [ ] **`tests/serde.tcyr:7-9`** says "Cyrius has no array-typed struct fields
      yet (deferred to the compiler's v6.4.x)". It landed in 6.4.11–6.4.13 and
      the pin carries it.
- [ ] **`docs/benchmarks-rust-v-cyrius.md`** is headed "svara v3.0.1 / cycc
      6.4.11", still calls SIMD **P0** where this file marks it deferred, and its
      table predates 3.1.3 entirely.
- [ ] **`docs/benchmarks.md` lists 11 benchmark rows**; `benches/hotpath.bcyr`
      defines 13 — the two `render_planned` benches from 3.1.3 are missing.
- [ ] **`CLAUDE.md`'s mission statement is still the scaffold placeholder**
      (`_TODO: one-or-two-sentence mission statement…_`) at 3.1.3. `cyrlint`
      never sees it because it does not lint Markdown.
- [ ] **README says the bridge has 19 scalar maps**; `state.md` and the CHANGELOG
      say 18. Count once, fix the other.
- [ ] **README's consumer example pins `tag = "3.1.2"`** against a 3.1.3 `VERSION`.
- [ ] **`docs/guides/getting-started.md`** sends new tests to `tests/svara.tcyr`
      (the smoke suite) rather than the 20 per-module suites.
- [ ] **`.github/workflows/ci.yml`** still explains a v5.6.28 `cyrius test`
      workaround under a 6.5.35 pin.
- [ ] **Log the README/CONTRIBUTING sweep in the CHANGELOG** — it shipped
      unrecorded; `## [Unreleased]` still says "Nothing yet."

### `.gitignore` decision

- [ ] **`*.csv` shadows `benches/history.csv`** (verified: `git check-ignore -v`
      → `.gitignore:7`), so the benchmark history is untracked and cannot serve
      the cross-commit regression purpose `docs/benchmarks.md` claims for it.
      3.1.3 deliberately left this as "a call about committing a data file"; the
      call still needs making. Same sweep retires `/target`, `Cargo.lock`,
      `**/*.rs.bk`, `/coverage/`.

---

## 3.2.0 — container serde

**M2. Unblocked** — array-typed struct fields landed across Cyrius **6.4.11**
(`Vec<T>` handle fields), **6.4.12** (`#derive` Ser/De for `Vec<primitive>`) and
**6.4.13** (`Vec<#derive-struct>`); the 6.5.35 pin carries all three. The standing
rule holds: add the derive plus a round-trip `.tcyr` per type. **No hand-written
codecs** — that was explicitly rejected as throwaway work the derive supersedes.

### Transcribe the Rust serde contract first

- [ ] **Record the oracle's serde surface in-repo, as a table**, before anything
      else in this release. This is also the largest single thing that
      [3.5.0](#350--retire-rust-old) needs `rust-old/` for, so doing it here
      converts a removal blocker into a finished artifact.

      ⚠ **The contract is smaller than this file used to claim.** The Rust has
      **30 `Serialize`-deriving types**, **3 `#[serde(default)]` attributes**
      (`rust-old/src/sequence.rs:37`, `:87`, `rust-old/src/voice.rs:150`) and
      **zero `#[serde(skip)]`**. The previous wording told an implementer to
      "match `rust-old/`'s `#[serde(skip)]` per type" — there is nothing to
      match. Decide svara's own skip policy explicitly instead of inheriting one
      that does not exist.

### Derive the containers

- [ ] `ProsodyContour` (f0-point vec) · `PhonemeSequence` (event vec) ·
      `TrajectoryPlanner` + `FormantKeypoint` · `RenderOutput` / `BatchRenderer` ·
      `SynthesisContext` · `SynthesisPool` · `FormantFilter` + the SOA biquad
      bank · `GlottalSource` · `VocalTract`.
- [ ] **Three pure-scalar types that M-serde missed** — each defers a codec in a
      source comment pointing at this file, and none was ever listed here:
      `Quality` (`src/lod.cyr:3-4`), `Rng` (`src/rng.cyr:17-18`, specifically for
      state persistence), `SmoothedParam` (`src/smooth.cyr:9-10`). All three could
      derive today.

### Skip the scratch fields

- [ ] **Seven fields added in 3.1.3 are transient and must not serialize** —
      `SvFormantBank.coeff`, `SvVocalTract.scratch_formants`,
      `SvTrajectoryPlanner.scratch_a` / `.scratch_b`, `SvProsodyContour.xs` /
      `.ys` / `.buf_cap`. Every one is overwritten before it is read, so all seven
      are skip-equivalent and must be rebuilt on load. A correct implementation
      cannot be written from the pre-3.1.3 type list alone.

**Gate:** every annotated type round-trips within the same f64 tolerance the
value-type serde uses (~1e-3, 6-decimal text); the scratch fields are absent from
the emitted JSON and reconstructed on load.

---

## 3.3.0 — structured logging

**M-log. Unblocked** — `lib/sakshi.cyr` is already vendored (transitively via
hisab), so this is wiring, not a new dependency. svara currently emits no logs;
diagnostics are error codes only (`error.cyr` + `svara_err_name`). It also
completes the sakshi routing that varna's `logging.cyr` explicitly deferred.

Plan (mirrors varna's `src/logging.cyr`):

- [ ] `src/logging.cyr` gated behind `-D LOGGING`; **zero cost when off** (no log
      calls compiled in). Off by default so the synthesis hot path stays
      allocation-free.
- [ ] Level-gated `svara_log_*` wrappers over sakshi's trace API; init reads a
      `SVARA_LOG` level (default `info`).
- [ ] sakshi spans on the coarse entry points for call-chain correlation —
      `svara_sequence_render`, `svara_batch_render_all` / `_with_progress`,
      `svara_synthctx_synthesize`, `svara_pool_render`. **Never per-sample.**
- [ ] Tie errors to the active span via `sakshi_err_at_span(code, category)` so a
      `SVARA_ERR_*` carries span context for dhvani / vansh.

**Gate:** builds warning-free with **and** without `-D LOGGING`; no allocation or
log call on the per-sample path; output correlates across the AGNOS stack (same
substrate as varna and shabdakosh).

---

## 3.4.0 — hot-path memory and SIMD

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

- [ ] **Collapse the redundant input delay line.** `x1`/`x2` are per-slot buffers
      but the input is shared across all 8 formant biquads, so every slot holds
      identical values — replace two 8-wide buffers with two scalars, dropping
      ~16 memory ops/sample. Bit-identical, no toolchain dependency. This is the
      lever that attacks the measured bottleneck.
- [ ] **Pool the per-note render buffers.** `phoneme.cyr` allocates ~16× per note
      and `pool.cyr` exists but is **not wired into the render path** (verified:
      `svara_pool_*` appears only in `src/main.cyr`'s smoke; `sequence.cyr` and
      `render.cyr` never reference it). 3.1.3 did this for the *per-sample* path;
      this is the remaining *per-note* one, same measurement method.
- [ ] **Tract per-sample chain** (~366 ns/sample: filter + nasal + subglottal +
      lip + feedback) — hoist coefficient recompute out of the per-sample loop.
      The SIMD half of this item is blocked below.

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

## 3.5.0 — retire `rust-old/`

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
      annotation work in [3.5.0](#350--retire-rust-old) to be done first.
- [ ] **Consumer-green** — shared gate with 3.5.0.

---

## Out of scope for v1.0

- **Bit-exact cross-arch transcendental output.** Tolerance parity instead —
  ganita's transcendentals are per-arch. See [`state.md`](state.md).
- **Non-American-English vowel targets.** The Hillenbrand data is US English;
  other languages and dialects need their own targets
  (`docs/architecture/adr-003-formant-data.md`).
- **Populating `docs/examples/`** as a separate concern — the runnable examples
  land in `examples/` in [3.5.0](#350--retire-rust-old), ported from the Rust.
  Either point `CLAUDE.md` at `examples/` or drop the `docs/examples/` reference.
