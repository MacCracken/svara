# svara — Roadmap

> Milestone plan through v1.0. State lives in [`state.md`](state.md);
> this file is the sequencing — what ships, in what order, against
> what dependency gates.

## v1.0 criteria

_Define before tagging v0.1.0:_

- [ ] Rust → Cyrius surface parity verified (function-level diff against `rust-old/`)
- [ ] Test coverage adequate for the surface area
- [ ] Benchmarks captured in `docs/benchmarks.md`
- [ ] At least one downstream consumer green
- [ ] CHANGELOG complete from v0.1.0 onward
- [ ] Security audit pass (`docs/audit/YYYY-MM-DD-audit.md`)

## Milestones

### M0 — Port scaffold (v0.1.0) — ✅ shipped 2026-07-03

- `cyrius port` scaffold landed
- Rust source moved to `rust-old/`
- Doc-tree per [first-party-documentation.md](https://github.com/MacCracken/agnosticos/blob/main/docs/development/applications/first-party-documentation.md)

### M1 — Surface parity → 3.0.0

Port all 19 Rust modules function-for-function against `rust-old/`, bottom-up,
each gated by `cyrius audit` + `.tcyr` tolerance tests. **3.0.0 = full parity**
with Rust 2.0.0 (213 tests / 15 benches reproduced). Acceptance per the locked
decisions in [`state.md`](state.md) (naad-backend only, `.cyml` data tables,
tolerance parity, preserve the three v2.0.1 quirks).

Port order (dependency layers):

1. **Foundation (L0)** — ✅ error, rng, smooth (48 tests). math.rs → ganita (no module).
2. **DSP primitives (L1)** — formant (SOA biquad, reuse naad `BiquadFilter`), spectral
   (hisab `num_fft`). Adds hisab + naad git deps.
3. **Excitation / tract (L2)** — glottal (pulse models + PCG noise), tract (biquad
   chain + source-filter feedback; collapse the naad-backend CFG split).
4. **Speech-science (L3)** — phoneme (2,636 LOC, `.cyml` tables), prosody, voice,
   sequence, trajectory.
5. **Orchestration / glue (L4)** — lod, pool, render, bridge.

### M-serde — serialization surface

Rust invariant "every public type is Serialize + Deserialize + roundtrip test" has
no Cyrius equivalent (no serde/derive). Plan:

- **Static data tables** (phoneme / vowel / formant / duration / tilt) → `.cyml`
  files parsed once at init (vidya pattern), not per-type codecs.
- **Stateful/config types** (~40: Rng, SmoothedParam, VoiceProfile, ProsodyContour,
  formant/tract state, …) → hand-written `svara_<type>_to_json` / `_from_json`
  built on `bayan` + `str_builder` + `f64_parse`/`fmt_float_buf`, each with a
  roundtrip `.tcyr` (tolerance-compare f64, exact-compare integer/sentinel fields).

Sequenced after the numeric core is green so the types are stable first.

## Out of scope (for v1.0)

- Bit-exact cross-arch transcendental output (tolerance parity instead — see state.md).
- Fixing the three preserved v2.0.1 quirks (a later minor, with their own tests).
