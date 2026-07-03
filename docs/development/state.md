# svara — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures
> (durable); this file is **state** (volatile).

## Version

**3.0.0** — Rust→Cyrius port complete (shipped 2026-07-03; started 2026-07-03
via `cyrius port`). **Full behavioral parity** with the Rust 2.0.0 surface:
all 19 modules ported, 634 test assertions / 18 suites (vs Rust 213 tests),
11 hot-path benchmarks. 8,785 lines of Rust preserved at `rust-old/` as the
parity oracle.

## Toolchain

- **Cyrius pin**: `6.3.40` (in `cyrius.cyml [package].cyrius`)

## Port decisions (locked 2026-07-03)

- **Backend**: carry only the `naad-backend` path (reuse Cyrius naad's biquad /
  noise / LFO); drop svara's internal fallback; collapse the tract/glottal CFG split.
- **Static data tables** (phoneme / vowel / formant / duration / tilt): the Rust
  tables are `match`-based *pure functions*, not serde — so they port as **embedded
  Cyrius code** (if-chains, like formant's `from_vowel`), NOT externalized data
  files. (Refined 2026-07-03 after reading phoneme.rs — a synth library shouldn't
  carry a runtime data-file dependency; JSON typed-DOM belongs to the actual serde
  surface, M-serde.) A `#derive(json)` proposal to erase the serde-codec tedium was
  filed in the cyrius repo: `docs/development/proposals/2026-07-03-derive-json-codec.md`.
- **Parity bar**: tolerance parity — port the 213 Rust tests as `.tcyr` with f64
  within-epsilon assertions + benchmark parity. Transcendentals aren't bit-identical across arches.
- **v2.0.1 quirks**: preserve all three for parity (unapplied per-vowel spectral
  tilt; `set_speed_quotient` ignored by Rosenberg pulse; `Tone` only in `render_planned`).
- **Naming**: every svara symbol is `svara_`/`SVARA_`/`SV_`/`Sv`-prefixed so the
  svara distlib coexists with naad's bundle in one flat namespace.

## Port progress (module by module)

Order: foundation → DSP primitives → excitation/tract → speech-science → orchestration.

| # | Rust module | Cyrius | Status | Tests | Notes |
|---|-------------|--------|--------|-------|-------|
| L0 | error.rs + dsp.rs | src/error.cyr | ✅ ported | 25 | enum→codes; validators + tolerances folded in |
| L0 | rng.rs | src/rng.cyr | ✅ ported | 17 | PCG32, golden-value verified |
| L0 | smooth.rs | src/smooth.cyr | ✅ ported | 6 | one-pole; exp via ganita builtin |
| L0 | math.rs | — | ✅ n/a | — | libm/std shim → maps to ganita/f64 builtins; no module needed |
| L1 | formant.rs | src/formant.cyr | ✅ ported | 25 | svara's own SOA 8× bandpass bank (NOT naad); golden coeffs+output verified |
| L1 | spectral.rs | src/spectral.cyr | ✅ ported | 14 | hisab num_fft interleaved-buffer idiom + Neumaier energy |
| L2 | glottal.rs | src/glottal.cyr | ✅ ported | 35 | naad-backend (NoiseGenerator + Lfo); golden Rosenberg + from_rd verified |
| L2 | tract.rs | src/tract.cyr | ✅ ported | 14 | naad Notch + BandPass biquads; source-filter feedback; CFG collapsed to naad-backend |
| L4 | lod.rs | src/lod.cyr | ✅ ported | 15 | Quality predicates (pulled early — tract needs it) |
| L3 | phoneme.rs | src/phoneme.cyr | ✅ ported | 299 | FULLY ported: inventory + classification + data tables + free synthesis + SynthesisContext. The crate's largest module (2,636 LOC) |
| L3 | voice.rs | src/voice.cyr | ✅ ported | 33 | VoiceProfile / VocalEffort / EffortParams; presets, builders, formant scaling, effort→glottal |
| L3 | prosody.rs | src/prosody.cyr | ✅ ported | 17 | ProsodyContour (f0 points + monotone-cubic), 4 intonation patterns, 9 tones, stress |
| L3 | trajectory.rs | src/trajectory.cyr | ✅ ported | 12 | TrajectoryPlanner + FormantKeypoint; Catmull-Rom + resistance blend; speaking-rate |
| L3 | sequence.rs | src/sequence.cyr | ✅ ported | 29 | PhonemeEvent/Sequence; coarticulation crossfade render + trajectory-planned render_planned; cluster compression |
| L4 | pool.rs | src/pool.cyr | ✅ ported | 16 | SynthesisPool (pooled SynthesisContext + render_batch + counters) |
| L4 | render.rs | src/render.cyr | ✅ ported | 16 | BatchRenderer / RenderOutput / RenderProgress; progress via callptr |
| L4 | bridge.rs | src/bridge.cyr | ✅ ported | 37 | 18 scalar emotion/TTS/creature/acoustics/weather → synth-param maps |

**ALL 19 Rust modules ported (16 `.cyr` modules; dsp folded into error, math → ganita).
634 assertions passing, all lint-clean.** The full library builds + links; the smoke binary
exercises the entire pipeline (synthesize phoneme, SynthesisContext, sequence render,
pool, batch renderer, bridge maps). The module port is complete.

## Tests

634 `.tcyr` assertions across 18 suites: error / rng / smooth / lod / formant /
spectral / glottal / tract / voice / phoneme / prosody / trajectory / sequence /
pool / render / bridge / **serde** (+ the `svara.tcyr` smoke) — all passing,
all lint-clean, zero build warnings. Run one suite: `cyrius test tests/<mod>.tcyr`;
the whole tree recursively: `cyrius tests tests`.

## Benchmarks

11 hot-path benches in `benches/hotpath.bcyr` (auto-discovered by `cyrius bench`),
results in [`../benchmarks.md`](../benchmarks.md), history in `benches/history.csv`
(via `./scripts/bench-history.sh`). Per-sample loops are batch-timed to remove
per-call clock overhead: glottal ~82 ns, formant ~175 ns, tract ~294 ns/sample —
a full chain ≈ 0.55 µs/sample (~40× real-time at 44.1 kHz).

## Quality gate

fmt (`cyrfmt`), lint (`cyrlint`, 0 warnings), docs (`cyrdoc`, **0 undocumented**),
tests (`cyrius tests tests`, 18/18), and `cyrius bench` are each green. The
aggregate `cyrius audit` command's test/bench legs report compile errors because
that command skips dependency resolution before compiling (stdlib prelude +
hisab/naad/goonj bundles absent) — reproducible identically in the sibling
`naad` 2.1.0 release, so a toolchain limitation, not a svara defect. Gate on the
individual commands (all green) rather than the aggregate.

## Dependencies

Direct (declared in `cyrius.cyml`):

- **stdlib** — syscalls, string, alloc, str, fmt, vec, io, args, assert, math, ganita, tagged, fnptr, bench
- **hisab** (git, pinned `2.6.7`) — FFT / HComplex / compensated sum. **Added** (spectral.cyr).
- **naad** (git, pinned `2.1.0`) — NoiseGenerator / Lfo / BiquadFilter backends. **Added** (glottal.cyr).
- **goonj** (git, pinned `2.0.0`) — transitive: the naad bundle references it. sakshi resolves via hisab.

> Decision (2026-07-03): glottal's noise + vibrato use the **full naad bundle**
> (per user choice), pulling goonj + sakshi transitively — maximally faithful to
> shipped 2.0.0-default behavior.

## Deferred work (tracked)

- **serde surface**: ~40 public types + private state structs (Rng, SmoothedParam, …)
  need JSON codecs or `.cyml` representation. See [`roadmap.md`](roadmap.md) M-serde.

## Consumers

dhvani (voice AI shell), vansh (voice shell TTS/STT) — will pull `dist/svara.cyr`.

## Serde (M-serde) — done via `#derive(Serialize)`

Cyrius already ships `#derive(Serialize)` (emits `Type_to_json`/`_from_json`/
`_from_json_str` over bayan's typed DOM); **6.3.40 fixed f64-field support** (the
"repair"). 8 pure-scalar public types derive it + round-trip (tests/serde.tcyr, 22
tests): Formant, VowelTarget, VoiceProfile, EffortParams, PhonemeEvent, Nasalization,
VoiceOnsetTime, RenderProgress. `bayan` is opt-in (`include "lib/bayan.cyr"`, not a
`[deps] stdlib` module) — tests that include a deriving module also include
`lib/hashmap.cyr` + `lib/bayan.cyr` so codecs are callable + warning-free.
**Deferred:** container types (vec/buffer fields — ProsodyContour/PhonemeSequence/
TrajectoryPlanner/RenderOutput/SynthesisContext/pool/BiquadBankSoa/FormantFilter/
GlottalSource) can't derive until Cyrius gains array-typed struct fields (compiler v6.4.x).

## Next — post-3.0.0

3.0.0 is shipped. All release items done: ✅ distlib (`dist/svara.cyr`, 4530 lines
at v3.0.0), ✅ M-serde value types, ✅ benchmarks + `docs/benchmarks.md`,
✅ quality gate (fmt/lint/docs/tests/bench), ✅ version bump + CHANGELOG/roadmap.

Follow-ups (not 3.0.0 blockers):

1. **3.1.0 — container serde** (⏳ blocked on Cyrius **6.4.x**). Do NOT start until
   the toolchain lands array-typed struct fields; then just add `#derive(Serialize)`
   + a roundtrip `.tcyr` to each container type (no hand-written codecs — that was
   explicitly rejected as throwaway). See [`roadmap.md`](roadmap.md) M2.
2. **Consumer smoke** — build dhvani / vansh against `dist/svara.cyr` end-to-end.
3. **Security audit** — `docs/audit/YYYY-MM-DD-audit.md` for a v1.0 hardening pass.
