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
- **Deferred**: container-bearing types (vec/buffer fields — ProsodyContour,
  PhonemeSequence, TrajectoryPlanner, RenderOutput, SynthesisContext, pool,
  BiquadBankSoa, FormantFilter, GlottalSource) can't derive until Cyrius gains
  array-typed struct fields (compiler v6.4.x). Tracked as a post-3.0.0 item.

## Out of scope (for v1.0)

- Bit-exact cross-arch transcendental output (tolerance parity instead — see state.md).
- Fixing the three preserved v2.0.1 quirks (a later minor, with their own tests).
