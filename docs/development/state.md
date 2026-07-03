# svara — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures
> (durable); this file is **state** (volatile).

## Version

**0.1.0** — in-progress Rust→Cyrius port (started 2026-07-03 via `cyrius port`).
8,785 lines of Rust preserved at `rust-old/` for parity reference. Target:
**3.0.0 = full parity** with the Rust 2.0.0 surface (213 tests / 15 benches).

## Toolchain

- **Cyrius pin**: `6.3.38` (in `cyrius.cyml [package].cyrius`)

## Port decisions (locked 2026-07-03)

- **Backend**: carry only the `naad-backend` path (reuse Cyrius naad's biquad /
  noise / LFO); drop svara's internal fallback; collapse the tract/glottal CFG split.
- **Static data tables** (phoneme / vowel / formant / duration / tilt): ship as
  `.cyml` files parsed once at init (vidya pattern), not per-type serde codecs.
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
| L2 | glottal.rs | src/glottal.cyr | ⏳ next | — | hottest path; pulse models + PCG noise; needs naad noise/LFO |
| L2 | tract.rs | src/tract.cyr | ⏳ | — | biquad chain + feedback; collapse CFG; needs naad biquad |
| L3 | phoneme.rs | src/phoneme.cyr | ⏳ | — | 2,636 LOC; `.cyml` data tables |
| L3 | prosody/voice/sequence/trajectory | … | ⏳ | — | speech-science layer |
| L4 | lod/pool/render/bridge | … | ⏳ | — | orchestration + glue |

**Total ported: 5 modules, 87 tests passing.** Smoke binary (`build/svara`) green.

## Tests

87 `.tcyr` assertions across error / rng / smooth / formant / spectral — all passing.
Run one suite: `cyrius test tests/<mod>.tcyr` (no auto-discovery).

## Dependencies

Direct (declared in `cyrius.cyml`):

- **stdlib** — syscalls, string, alloc, str, fmt, vec, io, args, assert, math, ganita, tagged, bench
- **hisab** (git, pinned `2.6.7`) — FFT / HComplex / compensated sum. **Added** (spectral.cyr).
- **naad** (git, pinned) — biquad / noise / LFO backends. Added when tract/glottal land.

## Deferred work (tracked)

- **serde surface**: ~40 public types + private state structs (Rng, SmoothedParam, …)
  need JSON codecs or `.cyml` representation. See [`roadmap.md`](roadmap.md) M-serde.

## Consumers

dhvani (voice AI shell), vansh (voice shell TTS/STT) — will pull `dist/svara.cyr`.

## Next

Excitation/tract layer (L2, highest float risk): `glottal.cyr` (pulse models +
PCG-driven aspiration noise; the hottest path) then `tract.cyr` (biquad chain +
source-filter feedback). Both add the **naad** git dep (noise/LFO for glottal;
biquad for tract's nasal notch + subglottal) and require collapsing the
naad-backend-vs-fallback CFG split to the naad-backend path. See [`roadmap.md`](roadmap.md).
