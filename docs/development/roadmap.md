# svara — Roadmap

> Milestone plan through v1.0. State lives in [`state.md`](state.md);
> this file is the sequencing — what ships, in what order, against
> what dependency gates.

## 3.0.0 release criteria — ✅ met 2026-07-03

- [x] Rust → Cyrius surface parity: all 19 modules ported, verified against `rust-old/`
- [x] Test coverage: 634 assertions / 18 suites (vs Rust 213 tests), all passing
- [x] Benchmarks captured in [`docs/benchmarks.md`](../benchmarks.md) (11 hot-path benches)
- [x] CHANGELOG complete from the port scaffold onward
- [x] Quality gate green: fmt, lint (0 warnings), docs (0 undocumented)

Remaining for a later v1.0 hardening pass (not 3.0.0 blockers):

- [ ] **P1 — structured logging through sakshi** (see [M-log](#m-log--structured-logging-via-sakshi--p1))
- [ ] At least one downstream consumer (dhvani / vansh) green against `dist/svara.cyr`
- [ ] Security audit pass (`docs/audit/YYYY-MM-DD-audit.md`)
- [ ] Container-type serde once Cyrius gains array-typed struct fields (v6.4.x)

## Milestones

### M0 — Port scaffold (v0.1.0) — ✅ shipped 2026-07-03

- `cyrius port` scaffold landed
- Rust source moved to `rust-old/`
- Doc-tree per [first-party-documentation.md](https://github.com/MacCracken/agnosticos/blob/main/docs/development/applications/first-party-documentation.md)

### M1 — Surface parity → 3.0.0 — ✅ shipped 2026-07-03

Ported all 19 Rust modules function-for-function against `rust-old/`, bottom-up,
each gated by `.tcyr` tolerance tests. **3.0.0 = full parity** with Rust 2.0.0
(634 assertions / 11 hot-path benches). Acceptance per the locked decisions in
[`state.md`](state.md) (naad-backend only, embedded data tables, tolerance parity,
preserve the three v2.0.1 quirks — all honored).

Port order (dependency layers) — all ✅:

1. **Foundation (L0)** — ✅ error, rng, smooth (48 tests). math.rs → ganita (no module).
2. **DSP primitives (L1)** — formant (SOA biquad, reuse naad `BiquadFilter`), spectral
   (hisab `num_fft`). Adds hisab + naad git deps.
3. **Excitation / tract (L2)** — glottal (pulse models + PCG noise), tract (biquad
   chain + source-filter feedback; collapse the naad-backend CFG split).
4. **Speech-science (L3)** — phoneme (2,636 LOC, `.cyml` tables), prosody, voice,
   sequence, trajectory.
5. **Orchestration / glue (L4)** — lod, pool, render, bridge.

### M-serde — serialization surface — ✅ done (value types) 2026-07-03

Resolved via Cyrius's built-in `#derive(Serialize)` (no hand-written codecs
needed): the derive emits `Type_to_json` / `_from_json_str` over bayan's typed
JSON DOM, and toolchain **6.3.40** added f64-field support. The 8 pure-scalar
public value types derive it with round-trip `.tcyr` coverage (22 tests):
`Formant`, `VowelTarget`, `VoiceProfile`, `EffortParams`, `PhonemeEvent`,
`Nasalization`, `VoiceOnsetTime`, `RenderProgress`.

- The static data tables (phoneme / vowel / formant / duration / tilt) stayed as
  embedded pure functions (if-chains) — a synth library shouldn't carry a runtime
  data-file dependency, and they were never serde in the Rust either.
- **Deferred to [M2](#m2--container-serde--320--unblocked-as-of-6413)**:
  container-bearing types couldn't derive until Cyrius gained array-typed struct
  fields. That landed in 6.4.11–6.4.13 — M2 is now unblocked.

### M2 — container serde → 3.2.0 — unblocked as of 6.4.13

**Not started; no longer blocked.** The remaining public types carry `vec`/raw-buffer
fields, which `#derive(Serialize)` could not introspect. The plan was deliberately
**do nothing until Cyrius landed array-typed struct fields** — hand-writing codecs
was explicitly rejected as throwaway work the derive would supersede.

✅ **The toolchain shipped it**, in three rounds:

| Release | What landed |
|---|---|
| 6.4.11 | array-typed struct fields R1 — `Vec<T>` handle fields (parse + layout + access) |
| 6.4.12 | R2 — `#derive` Serialize/Deserialize for `Vec<primitive>` |
| 6.4.13 | R3 — `#derive` for `Vec<#derive-struct>` (the arc closes) |

The 3.1.2 pin (**6.5.35**) carries all three. Retargeted to **3.2.0** — 3.1.0 went
to control-rate glides and 3.1.1/3.1.2 to toolchain + dependency maintenance.

M2 = add the derive + a roundtrip `.tcyr` to each container type, honoring Rust
serde parity (match `rust-old/`'s `#[derive(Serialize, Deserialize)]` /
`#[serde(skip)]` per type — skip transient runtime state and dep handles, rebuild
them on load, exactly as the Rust did):

- `ProsodyContour` (f0-point vec), `PhonemeSequence` (event vec),
  `TrajectoryPlanner` + `FormantKeypoint`, `RenderOutput` / `BatchRenderer`,
  `SynthesisContext`, `SynthesisPool`, `FormantFilter` + the SOA biquad bank,
  `GlottalSource`, `VocalTract`.

Gate: the annotated types must round-trip within the same f64 tolerance the
value-type serde uses (~1e-3, 6-decimal text).

### M-log — structured logging via sakshi — P1

**Not started.** svara currently emits no logs — diagnostics are error codes only
(`error.cyr` + `svara_err_name`). Route svara's tracing/diagnostics through
**sakshi** (`lib/sakshi.cyr` — साक्षी, the AGNOS structured-logging/tracing
substrate: levels error/warn/info/debug/trace, nestable timed spans, selectable
output targets). sakshi is already in svara's transitive dep set (via hisab), so
this is wiring, not a new dependency — and it completes the sakshi routing that
varna's `logging.cyr` explicitly deferred.

Plan (mirrors varna's `src/logging.cyr`):

- Add `src/logging.cyr` gated behind `-D LOGGING`; **zero cost when off** (no log
  calls compiled in). Off by default so the synthesis hot path stays
  allocation-free.
- Level-gated `svara_log_*` wrappers over sakshi's trace API; init reads a
  `SVARA_LOG` level (default `info`).
- Wrap the coarse entry points with sakshi spans for call-chain correlation —
  `svara_sequence_render`, `svara_batch_render_all` / `_with_progress`,
  `svara_synthctx_synthesize`, `svara_pool_render`. **Never per-sample** (hot path).
- Tie errors to the active span via `sakshi_err_at_span(code, category)` so a
  `SVARA_ERR_*` carries span context for downstream consumers (dhvani / vansh).

Gate: builds warning-free with **and** without `-D LOGGING`; no allocation or log
calls on the per-sample synthesis path; span/log output correlates across the
AGNOS stack (same sakshi substrate as varna and shabdakosh).

### M-perf — synthesis SIMD + hot-path optimization — P1 (in progress)

**Control-rate glides shipped in 3.1.0; the render loop stopped allocating in
3.1.3; SIMD investigated + deferred; memory-traffic work remains.** A same-machine Rust-vs-Cyrius run
([`../benchmarks-rust-v-cyrius.md`](../benchmarks-rust-v-cyrius.md)) put the real
per-DSP-unit gap at **10–38×**: formant bank 38×, vocal tract 19×, glottal 15×,
vowel 10×. Optimize the synthesis hot path WITHOUT breaking tolerance parity
(outputs stay within the existing `.tcyr` tolerance tests).

- **✅ DONE (3.1.0) — control-rate formant coefficients in glides**
  (`svara_ph_synth_diphthong`, `phoneme.cyr`). Was re-solving the whole biquad bank
  from the interpolated target on *every* sample (the ~5.4 ms `/ai/` outlier); now
  recomputes at a control rate of 64 samples (~1.45 ms) and holds between. Result:
  **5.42 ms → 0.94 ms (5.8×)**, tolerance suite unchanged (634/0). The diphthong
  now matches a steady vowel and beats the Rust oracle (1.09 ms, still per-sample).

- **✅ DONE (3.1.3) — no allocation on the per-sample render path.**
  `svara_sequence_render_planned` spent **1,121 bytes of arena per output sample**
  untoned and **1,505 toned**, into a bump allocator that never frees — a minute
  of speech needed ~3–4 GB it could not release. Now **33 / 113 B per sample**
  (34× / 13×), of which 16 is the output itself. The five sources, each measured:
  `svara_tract_set_formants` 736 → 0 (rebuild in place, same discard-the-state
  semantics as Rust's `self.filter = FormantFilter::new(..)?`),
  `svara_vowel_target_to_formants` 272 → 0, `svara_trajectory_formants_at`
  80–240 → 0, `svara_tone_to_contour` 240 → 0 (hoisted out of the loop — it was
  rebuilt every sample from the event's tone, which only changes at a boundary),
  `svara_prosody_f0_at` 144 → 80, `svara_formant_bank_update` 32 → 0. Output is
  bit-for-bit identical across ten render paths; the budget is pinned in
  `tests/allocbudget.tcyr`. CPU fell 10–15% as a side effect, which is *not* the
  metric — see the warning in [`state.md`](state.md#benchmarks).

  Residue, both upstream: `svara_ph_buf_to_vec` doubles its result vec from empty
  (~16 B/sample of dead copies; needs a stdlib `vec_with_capacity`), and hisab's
  `calc_monotone_cubic` allocates three internal arrays per call (the whole
  remaining 80 B of the toned path).

- **⚠ DEFERRED — SIMD the formant biquad bank** (`svara_formant_bank_process`).
  Prototyped a bit-identical `f64v4` (AVX2) version of the 8-slot SOA bank (two
  4-lane groups, scalar op order, `simd_has_avx2()`-guarded fallback). It passed
  tolerance (634/0) but bought only **~5%**: the per-sample loop is **memory-bound**,
  not compute-bound — the SOA *state shuffle* (~28 `load64`/`store64` per group) and
  the vector-op call overhead dominate, and the ptr/value `f64v4` API can't keep
  lane state in registers across samples. Reverted (also avoids a `simd` dep + a
  distlib-sidecar gap: `simd_has_avx2` isn't auto-folded). Two better bit-identical
  levers that attack the *actual* bottleneck (memory traffic):
    - **Collapse the redundant input delay line.** `x1`/`x2` are per-slot buffers
      but the input is shared across all 8 formant biquads, so every slot holds the
      same `x1`/`x2` — replace the two 8-wide buffers with two scalars and drop
      ~16 memory ops/sample. Bit-identical, no toolchain dep.
    - **Register-resident block SIMD.** A `process_block` that keeps the 8-slot
      state in vector registers across the sample loop (needs codegen that doesn't
      round-trip each `f64v4` op through memory) — revisit when the SIMD API/codegen
      supports it.

- **P1 — pool the per-note render buffers.** `phoneme.cyr` allocates ~16× per note and
  `pool.cyr` exists but is **not wired into the render path**. Reuse a per-render
  scratch arena for the transient sample / `Formant` vecs instead of allocating per
  phoneme — removes the "+alloc" overhead that shows in every `synthesize_phoneme` and
  compounds across `svara_sequence_render`. 3.1.3 did this for the *per-sample*
  path; this item is the remaining *per-note* one, and the same measurement
  method applies (marginal arena bytes, not wall clock).

- **P1 — FMA + block audit.** Ensure every biquad/mix inner loop uses fused
  multiply-add (`f64v4_fmadd`) and that the glottal→tract→formant chain runs in blocks
  (amortize per-call overhead, expose the vectorizable spans). `process_block` already
  does this for the bank; extend it through the tract chain.

- **P2 — tract per-sample chain** (`tract.cyr`, ~366 ns/sample: filter + nasal +
  subglottal + lip + feedback). Hoist coefficient recompute out of the per-sample loop
  and SIMD the independent parallel sub-filters.

- **Track it:** add `docs/benchmarks-rust-v-cyrius.md` (the hisab/shabda format) so the
  per-op gap and each optimization's win stay visible across commits, alongside
  `bench-history.csv`.

Gate: all tolerance-parity `.tcyr` tests stay green (SIMD / control-rate paths match the
scalar output within tolerance); the AVX2 path is guarded by `simd_has_avx2()` with a
scalar fallback byte-identical to today; `bench-history.csv` records before/after per-op
deltas; no regression on non-AVX2 hosts.

## Out of scope (for v1.0)

- Bit-exact cross-arch transcendental output (tolerance parity instead — see state.md).
- Fixing the three preserved v2.0.1 quirks (a later minor, with their own tests).
