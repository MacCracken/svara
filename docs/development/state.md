# svara — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures
> (durable); this file is **state** (volatile).

## Version

**3.2.0** (2026-08-26) — **container serde.** Serialized surface 8 → **13**
types: `SvSmoothedParam`, `SvRng`, `SvF0Point` (new — the raw 16-byte
`(time, value)` pair given a type), `SvProsodyContour` (`Vec<SvF0Point>`),
`SvPhonemeSequence` (`Vec<SvPhonemeEvent>`). Round-trips are asserted
**behaviourally**: a restored `Rng` reproduces 32 draws exactly, a restored
contour traces the same curve at 21 points, a restored sequence renders
**bit-identical audio**. Two premises of M2 turned out false and are corrected in
[ADR-0002](../adr/0002-serialization-boundary.md) — the Rust had **zero
`#[serde(skip)]`** to inherit, and the toolchain **cannot decode a nested
`#derive`-struct field** in either direction, which rules out 6 of the 9 types
M2 named. `SvProsodyContour`'s scratch moved to module level (a serialized struct
may not hold a raw pointer — no skip attribute exists). 745 assertions / 20
suites; every render path still bit-for-bit identical to 3.1.2.

**3.1.3** (2026-08-26) — **P1 audit sweep.** Five defects reachable through the
public API with no unsafe usage by the caller, all confirmed by running them
rather than reasoned about: a **SIGSEGV** (`alloc` returns 0 on failure and svara
wrote through it — `synthesize(vowel, 44100, 6100 s)` was the reproducer), three
**process aborts** (a negative spectrum bin reaching `vec_get`, from `f64_to`
emitting INT64_MIN where Rust's `as usize` saturates), a **silent NaN** (an
inverted range test accepted a NaN f0 that Rust rejects), a constructor that
returned an error code *as a pointer*, and an **infinite loop** in `next_pow2`.
Plus the per-sample allocation in `render_planned`: **1,121 → 33 bytes/sample**
untoned, **1,505 → 113** toned, into an arena that never frees. Output is
**bit-for-bit identical** across ten render paths. 634 → **713 assertions** / 20
suites. See [ADR-0001](../adr/0001-signed-index-and-float-conversion-hazards.md).

**3.1.2** (2026-08-26) — toolchain + dependency catch-up. Cyrius pin
**6.4.13 → 6.5.35** (109 releases), and every dependency to its current tag:
hisab **2.6.7 → 2.11.2**, naad **2.1.1 → 2.2.1**, goonj **2.0.0 → 2.0.4**,
sakshi **2.4.2 → 2.4.11** (transitive). One breaking upstream rename absorbed —
naad 2.2.0's `FILTER_* → NAAD_FILTER_*`, which `src/tract.cyr` uses in two
places (nasal antiformant, subglottal bandpass); values unchanged, so it is a
compile-time break only. 90 lines reindented by the fixed `cyrfmt` (6.5.28
taught it to track parentheses). 634 assertions / 18 suites green, benchmarks
within noise of 3.1.0, and **`cyrius audit` now exits 0** (see Quality gate).

**3.1.1** (2026-08-26) — pin-only: Cyrius `6.4.12 → 6.4.13`. No source change.
Tagged without a CHANGELOG section at the time; recorded retroactively in 3.1.2.

**3.1.0** (2026-07-06) — synthesis performance. `svara_ph_synth_diphthong`
re-solved the whole formant biquad bank on *every* sample of a glide; it now
recomputes at a control rate of 64 samples and holds the coefficients between
updates. Diphthong render **5.42 ms → 0.94 ms (~5.8×)**, now faster than the
Rust oracle (1.09 ms), which still re-solves per sample. Perceptually identical;
the tolerance suite passes unchanged. SIMD (`f64v4`) on the formant bank was
prototyped and reverted — the loop is memory-bound.

**3.0.1** (2026-07-05) — dependency maintenance: pinned naad **2.1.0 → 2.1.1**
(naad's abaco↔naad namespace de-collision — the two bare dB helpers gained the
`naad_` prefix). svara doesn't call them, so this was a pin-only bump.

**3.0.0** — Rust→Cyrius port complete (shipped 2026-07-03; started 2026-07-03
via `cyrius port`). **Full behavioral parity** with the Rust 2.0.0 surface:
all 19 modules ported, 634 test assertions / 18 suites (vs Rust 213 tests),
11 hot-path benchmarks. 8,785 lines of Rust preserved at `rust-old/` as the
parity oracle.

## Toolchain

- **Cyrius pin**: `6.5.35` (in `cyrius.cyml [package].cyrius` — the single
  source of truth; CI reads it and never hardcodes a version)
- `lib/` holds 29 stdlib files vendored from `~/.cyrius/versions/6.5.35/lib`
  (verified byte-identical file by file) plus the 4 dependency bundles.
  `cyrius.lock`: 33 deps locked, 4 commit-pinned.
- **`cyrfmt` canonical continuation indent is 2 spaces per open-paren level**
  (4 also accepted). 6.5.28 fixed the formatter, which had never tracked
  parentheses; the pre-6.5.28 8-space convention is retired.
- **`lib/bench.cyr` measures the timer floor** and subtracts it from every
  sample (6.5.19). It prints the measured value and `bench_clock_overhead_ns()`
  returns it — never write a clock-overhead figure into a comment, it is host
  specific across a ~230× range.

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

713 `.tcyr` assertions across 20 suites: error / rng / smooth / lod / formant /
spectral / glottal / tract / voice / phoneme / prosody / trajectory / sequence /
pool / render / bridge / **serde** / **hardening** / **allocbudget** (+ the
`svara.tcyr` smoke) — all passing, all lint-clean, zero build warnings. Run one
suite: `cyrius test tests/<mod>.tcyr`; the whole tree recursively:
`cyrius tests tests`.

Two of them are not parity ports and exist for reasons worth knowing:

- **`hardening.tcyr` (45)** — adversarial inputs for the hazard class in
  [ADR-0001](../adr/0001-signed-index-and-float-conversion-hazards.md). Its
  discrimination is **measured, not asserted**: ten public-API probes re-run
  against a 3.1.2 checkout on the same toolchain give **9 failures** (4 wrong
  value, 3 process abort, 1 SIGSEGV, 1 hang) and 1 control that passes on both
  sides. Re-measure this whenever the group is extended — a group where every
  line flips is a group whose controls are not controls.
- **`allocbudget.tcyr` (34)** — each `_into` variant compared **bit for bit**
  against the allocating function it replaced (exact `assert_eq` on raw f64
  patterns, not tolerances: same arithmetic, same order, so it holds on every
  arch — unlike a stored golden digest, which transcendentals make
  arch-specific). Plus the marginal-bytes-per-sample budget, each bound paired
  with a control proving the measurement is not reading zero.

## Benchmarks

13 hot-path benches in `benches/hotpath.bcyr` (auto-discovered by `cyrius bench`),
results in [`../benchmarks.md`](../benchmarks.md), history in `benches/history.csv`
(via `./scripts/bench-history.sh`). Per-sample loops are batch-timed to remove
per-call clock overhead: glottal ~82 ns, formant ~179 ns, tract ~295 ns/sample —
a full chain ≈ 0.56 µs/sample (~40× real-time at 44.1 kHz). Renders: `/a/`
~874 µs, `/s/` ~461 µs, `/ai/` ~918 µs, 3-phoneme sequence ~3.37 ms,
`render_planned` ~18.7 ms (~21.4 ms toned) (x86_64, 2026-08-26, cycc 6.5.35).

⚠ **The allocation budget is deliberately NOT benchmarked.** A bump allocation is
a pointer add, so removing 1.1 KB of it per sample moved `render_planned` only
20.6 → 18.6 ms — the cost that mattered was *arena bytes*, which a timer cannot
see. It is pinned in `allocbudget.tcyr` instead. Anything that trades memory for
time in this codebase needs the same treatment.

## Quality gate

All green on the 6.5.35 pin:

| Gate | Command | Result |
|---|---|---|
| fmt | `cyrfmt --check <f>` | clean (40 files) |
| lint | `cyrlint <f>` | 0 warnings, 0 untracked deferrals |
| docs | `cyrdoc --check <f>` | 0 undocumented (252 public fns across 17 modules) |
| tests | `cyrius tests tests` / bare `cyrius test` | 20/20 suites, 745 assertions |
| fuzz | `cyrius fuzz` | 1/1 (`tests/svara.fcyr`) |
| deny | `cyrius deny src/main.cyr` | 21 deps, 0 violations |
| bench | `cyrius bench` | 2/2 bench files |
| **aggregate** | **`cyrius audit`** | **exits 0** |

**`cyrius audit` works again as of this pin.** Through 3.1.1 it skipped
dependency resolution before compiling its test and bench legs, so both failed
with spurious *"undefined variable F64_ONE / SYS_WRITE"* — reproducible
identically in the sibling `naad` 2.1.0, hence a toolchain bug rather than a
svara defect, and the workaround was to gate on the individual tools. On 6.5.35
it resolves deps first (`4 deps resolved`) and runs fmt · lint · docs · tests ·
bench green. The per-tool commands still work and remain the finer-grained form.

⚠ **CI runs almost none of this.** `.github/workflows/ci.yml` has four steps —
install, `cyrius deps`, `cyrius build`, `cyrius test` — so fmt, lint, docs, fuzz,
deny, bench and the aggregate `cyrius audit` run only on a developer's machine,
and `release.yml` inherits the same thin gate. Tracked for 3.1.4 in
[`roadmap.md`](roadmap.md).

`README.md` and `CONTRIBUTING.md` were rewritten for Cyrius on 2026-08-26. Four
other docs were not — `docs/architecture/overview.md`,
`docs/development/integration-guide.md`, `docs/guides/testing.md` and
`docs/development/threat-model.md` are still pre-port Rust. Also tracked for
3.1.4.

⚠ The `cyrius fmt` / `cyrius doc` wrappers used to mangle their arguments; on
this pin `cyrius fmt <file>` **rewrites in place** (it does not print to stdout).
`cyrfmt` / `cyrlint` / `cyrdoc` from `~/.cyrius/bin` remain the direct forms.

## Dependencies

Pinned tags as of **3.1.2**; see [`dependency-watch.md`](dependency-watch.md) for
the upgrade policy and the symbols svara actually consumes.

| Dep | Pin | Declared | What svara uses |
|---|---|---|---|
| **hisab** | `2.11.2` | `[deps.hisab]` | `num_fft` + `num_neumaier_sum` (spectral.cyr), `calc_monotone_cubic` (prosody.cyr), `ease_in_out_smooth` (phoneme / trajectory / sequence) |
| **naad** | `2.2.1` | `[deps.naad]` | `filter_biquad_*` + `NAAD_FILTER_NOTCH`/`_BANDPASS` (tract.cyr), `noise_*` + `modulation_lfo_*` + `Lfo_set_depth` (glottal.cyr) |
| **goonj** | `2.0.4` | `[deps.goonj]` | nothing directly — the naad bundle references it, so it must resolve from this manifest |
| **sakshi** | `2.4.11` | transitive (via hisab) | nothing directly — goonj's logging backend |

**stdlib** (`[deps].stdlib`) — syscalls, string, alloc, str, fmt, vec, io, args,
assert, math, ganita, tagged, fnptr, bench. `bayan` + `hashmap` are **opt-in via
explicit `include`**, not stdlib entries (auto-vendor fails for them); every
entry file that includes a `#derive(Serialize)` module also includes those two so
the codecs are callable and warning-free.

> Decision (2026-07-03): glottal's noise + vibrato use the **full naad bundle**
> (per user choice), pulling goonj + sakshi transitively — maximally faithful to
> shipped 2.0.0-default behavior.

⚠ **naad renames land as compile-time breaks, not silent ones.** 2.1.3 moved
`ERR_* → NAAD_ERR_*` (inert here: svara's codes are `SVARA_ERR_*`) and 2.2.0
moved `FILTER_* → NAAD_FILTER_*`, `VOICE_* → NAAD_VOICE_*` and six bare helpers
(`lerp`, `rms`, `peak`, `normalize`, `chromagram`, `crossfade_equal_power`) under
`naad_`. Only the two `FILTER_*` constants reached svara. On any future bump,
diff the used-symbol set against the new bundle before assuming a clean build.

## Deferred work (tracked)

- **serde surface**: ~40 public types + private state structs (Rng, SmoothedParam, …)
  need JSON codecs or `.cyml` representation. See [`roadmap.md`](roadmap.md) M-serde.

## Consumers

dhvani (voice AI shell), vansh (voice shell TTS/STT) — will pull `dist/svara.cyr`.

## Serde — 13 derived types, and a decided boundary

Cyrius ships `#derive(Serialize)` (emits `Type_to_json` / `_from_json` /
`_from_json_str` over bayan's typed DOM); 6.3.40 added f64-field support.
**`bayan` is opt-in** — `include "lib/bayan.cyr"` plus `lib/hashmap.cyr`, not a
`[deps] stdlib` module — so any entry file that includes a deriving module needs
both, or the codecs are undefined at link.

**Derived (13):** `SvFormant`, `SvVowelTarget`, `SvVoiceProfile`,
`SvEffortParams`, `SvPhonemeEvent`, `SvNasalization`, `SvVOT`,
`SvRenderProgress` (M-serde, 3.0.0) · `SvSmoothedParam`, `SvRng`, `SvF0Point`,
`SvProsodyContour`, `SvPhonemeSequence` (3.2.0). Round-trip coverage in
`tests/serde.tcyr` (54 assertions).

**Not derived, by decision:** `SvVocalTract`, `SvGlottalSource`,
`SvFormantFilter` / `SvFormantBank`, `SvTrajectoryPlanner` / `SvFormantKeypoint`,
`SvSynthCtx`, `SvSynthesisPool`, `SvBatchRenderer`, `SvRenderOutput`,
`SvSpectrum`. Each holds a nested struct pointer, a raw sample buffer, or a
foreign naad handle with no JSON form. These are engine state, not configuration
— see [ADR-0002](../adr/0002-serialization-boundary.md), and `roadmap.md` 3.2.1
for the state-companion follow-on.

Three toolchain facts, measured on 6.5.35 rather than read off a changelog:

| Field shape | Encode | Decode |
|---|---|---|
| scalar, `Vec<f64>`, `Vec<iNN>` | ✅ | ✅ |
| `Vec<T>`, T a **flat** derive struct | ✅ | ✅ — `_from_json_str` **only** |
| **nested derive-struct field** | ⚠ garbage | ❌ zeros |

⚠ **Two traps worth knowing before touching a `#derive(Serialize)` struct:**

1. **A nested struct field is laid out INLINE** (`sizeof(ND{tag: i64; inner: NA})`
   is 24 for a 16-byte `NA`) while `#derive(accessors)` gives the same field
   **pointer** semantics. The two derives disagree, so annotating one of svara's
   currently-untyped struct fields (`filter;`, `progress;`, `target;`) to make
   `Serialize` see it would change its layout and silently corrupt live objects.
   Leave them untyped.
2. **There is no skip attribute.** Every field is emitted, so a serialized struct
   may not hold a scratch buffer — the pointer leaks a heap address into the JSON
   and returns dangling. `SvProsodyContour`'s interpolation pair moved to module
   level in 3.2.0 for exactly this reason; the other 3.1.3 scratch fields
   (`SvFormantBank.coeff`, `SvVocalTract.scratch_formants`,
   `SvTrajectoryPlanner.scratch_a`/`_b`) live on types that are not serialized.

**The pairs-form `_from_json` is not usable** for any type with a `Vec<struct>`
field: bayan truncates an array-of-objects value at the first inner comma, so it
returns an empty vec. Silently. Use `_from_json_str`.

## Next

3.1.3 is release-ready: ✅ five reachable defects fixed and pinned, ✅ per-sample
allocation removed from the render loop (34× / 13×), ✅ bit-for-bit parity
verified against 3.1.2 across ten render paths, ✅ two new suites + ADR-0001,
✅ distlib regenerated (`dist/svara.cyr`, 4,950 lines at v3.1.3), ✅ full gate
incl. `cyrius audit` exiting 0, ✅ benchmarks re-run + recorded, ✅ CHANGELOG +
this file.

**Sequencing lives in [`roadmap.md`](roadmap.md)**, which is organised by release
rather than by milestone: 3.1.4 (docs, CI, stale claims) → 3.2.0 (container
serde) → 3.3.0 (structured logging) → 3.4.0 (hot-path memory and SIMD) → 3.5.0
(retire `rust-old/`) → v1.0 (hardening). Do not duplicate that list here.

Two things about the current state that the roadmap depends on and that are
easy to get wrong:

- **`cyrlint`'s "0 untracked deferrals" is accurate and nearly meaningless as a
  completeness signal.** It scans only `.cyr` sources — never Markdown — matches
  twelve literal terms, and counts one "tracked" if the *same line* also contains
  `CHANGELOG`, `roadmap`, `docs/`, `issue`, `See `, `v6.` or `v5.`. That is a
  substring test, not a check that the roadmap says anything. Most of svara's
  real deferral language (`not started`, `not wired`, `blocked on`, `upstream`,
  `known limitation`, `P1`/`P2`) is invisible to it.
- **The `rust-old/` port is complete** — a name-by-name sweep of every Rust `pub`
  item found zero missing counterparts, and enum parity is exact on all ten enums
  including `Phoneme` at 101/101. What holds the directory in the tree is the
  work that still *reads* it, not a gap in the port. See
  [`roadmap.md`](roadmap.md) 3.5.0.
