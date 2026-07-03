# svara — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures
> (durable); this file is **state** (volatile).

## Version

**0.1.0** — in-progress Rust→Cyrius port (started 2026-07-03 via `cyrius port`).
8,785 lines of Rust preserved at `rust-old/` for parity reference. Target:
**3.0.0 = full parity** with the Rust 2.0.0 surface (213 tests / 15 benches).

## Toolchain

- **Cyrius pin**: `6.3.39` (in `cyrius.cyml [package].cyrius`)

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
| L3 | sequence.rs | src/sequence.cyr | ⏳ next | — | PhonemeEvent/Sequence coarticulation; render + render_planned (uses trajectory) |
| L4 | pool/render/bridge | … | ⏳ | — | SynthesisPool / BatchRenderer / scalar glue |

**Total ported: 12 modules, 512 tests passing.** Smoke binary (`build/svara`) green
— synthesizes a full phoneme end-to-end (voice → glottal → tract → PCM), via both
the free API and SynthesisContext, plus prosody contours + trajectory planning.

## Tests

512 `.tcyr` assertions across error / rng / smooth / lod / formant / spectral /
glottal / tract / voice / phoneme / prosody / trajectory — all passing (all 12 test
files lint-clean). Run one suite: `cyrius test tests/<mod>.tcyr` (no auto-discovery).

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

## Next

**`sequence.cyr`** (PhonemeEvent / PhonemeSequence — coarticulation crossfade
`render` + continuous-trajectory `render_planned`; consonant-cluster compression;
uses phoneme + voice + SynthesisContext + prosody + trajectory + hisab easing).
Then **pool / render / bridge** (orchestration + glue). Then M-serde codecs +
parity/bench gates + `dist/svara.cyr`. See [`roadmap.md`](roadmap.md).
