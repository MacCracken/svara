# 0002 — Where serialization stops

**Status**: Accepted
**Date**: 2026-08-26

## Context

M2 ("container serde") was written as: *add `#derive(Serialize)` plus a
round-trip `.tcyr` to each container type, honouring Rust serde parity — match
`rust-old/`'s `#[derive(Serialize, Deserialize)]` / `#[serde(skip)]` per type.*

Two things turned out to be false, and both had to be measured rather than
assumed.

### The Rust contract does not say what the roadmap thought it said

Transcribed from `rust-old/src/*.rs` — this is the whole of it:

| | Count |
|---|---|
| Types deriving `Serialize, Deserialize` | **30** (17 structs, 13 enums) |
| `#[serde(skip)]` attributes | **0** |
| `#[serde(default)]` attributes | **3** |

The three defaults: `rust-old/src/sequence.rs:37` (`PhonemeEvent.tone`),
`rust-old/src/sequence.rs:87` (`PhonemeSequence.speaking_rate`, via
`default_speaking_rate`), `rust-old/src/voice.rs:150` (`VoiceProfile`).

The 30 types: `SvaraError`, `Vowel`, `Formant`, `FormantFilter`, `VowelTarget`,
`GlottalModel`, `GlottalSource`, `Quality`, `Phoneme`, `PhonemeClass`,
`Nasalization`, `SynthesisContext`, `VoiceOnsetTime`, `SynthesisPool`,
`IntonationPattern`, `Stress`, `Tone`, `ProsodyContour`, `BatchRenderer`,
`RenderOutput`, `RenderProgress`, `PhonemeEvent`, `PhonemeSequence`,
`NasalPlace`, `VocalTract`, `FormantKeypoint`, `TrajectoryPlanner`,
`VocalEffort`, `EffortParams`, `VoiceProfile`.

**There is no skip policy to inherit.** Rust serialized *everything*, including
live filter state, because it could: `VocalTract` holds a
`naad::filter::BiquadFilter` (`rust-old/src/tract.rs:72`, `:94`) and that type is
itself `Serialize` in the Rust naad. Cyrius naad exposes those filters as opaque
handles with no JSON codec, so "serialize everything" is not portable. The
policy has to be **decided here**, not transcribed.

### The toolchain reaches less than the changelog implies

Verified on cyrius 6.5.35 with a standalone probe, not inferred:

| Field shape | Encode | Decode | Usable |
|---|---|---|---|
| scalar (`f64`, `i64`) | ✅ | ✅ | yes |
| `Vec<f64>` / `Vec<iNN>` | ✅ | ✅ | yes |
| `Vec<T>` where T is a **flat** `#derive` struct | ✅ | ✅ | yes, via `_from_json_str` only |
| **nested `#derive`-struct field** | ⚠ | ❌ | **no** |
| `Vec<Str>` | — | — | no (rejected by the compiler) |

The nested-struct row is worse than "unsupported", and this is the finding that
shaped the decision:

- A nested struct field is laid out **inline**. `sizeof(ND{tag: i64; inner: NA})`
  is 24 for a 16-byte `NA` — one slot for `tag`, two for `inner` in place.
- `#derive(accessors)` gives that same field **pointer** semantics:
  `ND_set_inner(d, p)` writes a pointer into the first inline word.
- So the two derives disagree about one field, and the encoder emits the pointer
  reinterpreted as the first member: `{"inner":{"x":6.9e-310,"n":0}}` where the
  value was `{x: 99, n: 3}`. Written inline instead, the encoder is correct —
  `{"inner":{"x":99.0,"n":3}}`.
- **Either way `_from_json_str` returns zeros for the nested fields.** A decoded
  object whose nested field is a pointer slot therefore holds a **null pointer**,
  and the next dereference is a segfault — the same shape of defect ADR-0001 was
  written about.

svara is not currently exposed to the corruption, because every struct-valued
field in `src/` is declared **untyped** (`filter;`, `progress;`, `target;`) and so
is a plain i64 slot with correct pointer semantics. Adding a type annotation to
make `Serialize` see such a field would change its layout and break the
accessors. That is a trap worth naming.

## Decision

**Serialization stops at the boundary between configuration and engine.**

1. **Configuration and data serialize.** Anything a consumer would save, send or
   author by hand: voice profiles, effort parameters, phoneme events and
   sequences, prosody contours and their points, formants and vowel targets,
   nasalization, voice-onset timings, render progress, RNG state. These derive
   `#derive(Serialize)` and carry a round-trip test.

2. **Engine objects do not serialize, and no codec pretends otherwise.**
   `VocalTract`, `GlottalSource`, `FormantFilter` / the SOA biquad bank,
   `SynthesisContext`, `SynthesisPool`, `BatchRenderer`, `TrajectoryPlanner`,
   `RenderOutput`, `Spectrum`. Each holds at least one of: a nested struct
   pointer the toolchain cannot decode, a raw sample buffer, or a foreign naad
   handle with no JSON form at all. A consumer rebuilds them from the
   configuration in (1), which is what Rust's "rebuild on load" convention means
   in practice even though Rust never had to write it down.

3. **No hand-written codecs.** Standing since M-serde and unchanged: hand-writing
   what a derive should emit is throwaway work. Where a derive cannot reach a
   type, the answer is a **flat state companion** that can be derived plus a
   `restore`, never a bespoke `to_json`.

4. **A serialized struct may not hold a scratch buffer.** `#derive(Serialize)`
   has no skip attribute, so *every* field is emitted. A scratch pointer would
   leak a heap address into the output — an ASLR disclosure if that JSON ever
   crosses a trust boundary — and would come back as a dangling pointer. When a
   type needs both serialization and scratch, the scratch moves out of the
   struct. `ProsodyContour` is the worked example: its interpolation pair moved
   to module level in 3.2.0, which is also less memory than the per-contour pair
   3.1.3 introduced.

5. **Enums serialize as their integer, with no wrapper struct.** Rust's 13
   `Serialize` enums (`Quality`, `Phoneme`, `Tone`, `Stress`, `NasalPlace`,
   `VocalEffort`, …) are `var SVARA_*` integer constants here. They already
   round-trip wherever they appear as a field — `SvNasalization.place`,
   `SvPhonemeEvent.stress`/`.tone`. Deriving a one-field wrapper for each would
   add API surface to serialize an integer.

6. **Decode through `_from_json_str`, not the pairs form.** The pairs-form
   `_from_json` returns an **empty vec** for an array-of-objects field, because
   bayan truncates that value at the first inner comma. Every `Vec<struct>` field
   in svara must be decoded with `_from_json_str`.

### What 3.2.0 ships under this decision

| Type | How | Note |
|---|---|---|
| `SvSmoothedParam` | derived | was untyped; three `f64` annotations |
| `SvRng` | derived | two `i64`; restoring resumes the exact draw sequence |
| `SvF0Point` | derived, **new** | the raw 16-byte `(time, value)` pair given a type; same layout |
| `SvProsodyContour` | derived | `Vec<SvF0Point>`; scratch hoisted per (4) |
| `SvPhonemeSequence` | derived | `Vec<SvPhonemeEvent>`; the element type is flat |

Round-trips are asserted **behaviourally**, not just field-by-field: a restored
`Rng` must reproduce 32 draws exactly, a restored contour must trace the same
curve at 21 points, and a restored `PhonemeSequence` must render **bit-identical
audio**. Equal fields are weaker evidence than equal behaviour, and the vacuous
round-trip is a known failure mode in this ecosystem (naad shipped one for three
releases).

## Consequences

- **Positive** — the serialized surface is now the surface a consumer actually
  persists, and every member of it is proven by behaviour. The un-derivable set
  is un-derivable for a stated, measured reason rather than by omission, and
  `tests/serde.tcyr`'s last group pins the two facts (`FormantKeypoint` holds a
  target pointer; `GlottalSource` holds three dependency handles) so a toolchain
  release that fixes nested-field decode surfaces here as a failing test rather
  than going unnoticed.

- **Negative** — svara is *less* serializable than the Rust was. A caller who
  relied on round-tripping a whole `VocalTract` cannot; they must save the
  configuration and rebuild. This is a real divergence from the oracle, taken
  deliberately, because the alternative is a codec that silently returns an
  object with a null pointer in it.

- **Negative** — decode is `_from_json_str`-only for any type with a
  `Vec<struct>` field. The pairs form compiles and returns an empty vec, so
  misuse is silent. Stated in each affected module header.

- **Neutral** — the three `#[serde(default)]` behaviours are not reproduced;
  Cyrius's derive has no default mechanism, and a missing field decodes to zero.
  For `PhonemeEvent.tone` that is wrong in a specific way (`0` is
  `SVARA_TONE_HIGH`, while the Rust default was "no tone", which svara spells
  `SVARA_TONE_NONE = -1`). svara always emits the field, so this only bites on
  externally-authored JSON. Worth a guard if that ever becomes a supported input.

- **Neutral** — the un-derivable engine types still want a flat state companion
  plus `restore` (decision 3). That work is scoped in the roadmap rather than
  done here; the boundary in this ADR is what makes it a mechanical exercise.

## Alternatives considered

- **Follow the Rust literally and derive on all 30 types.** Rejected on the
  measurement: 6 of the 9 named container types hold a nested struct pointer, and
  the derive would emit garbage for it and decode a null back. That is a worse
  outcome than no codec.

- **Annotate the nested fields with their struct type so `Serialize` sees them.**
  Rejected: it changes the field from a pointer slot to an inline region while
  `#derive(accessors)` keeps writing a pointer into it — silent corruption of
  live objects, to gain an encoder whose decoder still returns zeros.

- **Wrap the derived decoder to zero the scratch fields afterwards.** Rejected
  for `ProsodyContour`: it fixes the dangling pointer but not the address leak,
  since the encoder still emits `"xs": <heap address>`. Moving the scratch out
  fixes both, and made the code smaller.

- **Hand-write codecs for the engine types.** Rejected, standing since M-serde.
  A hand-written codec for `VocalTract` would still have to invent a
  representation for two naad biquads; that representation is the state
  companion, so write that instead and let the derive do the encoding.
