# 0013 — Whether, where and how to apply the per-vowel spectral tilt

**Status**: Accepted (3.10.0)
**Date**: 2026-11-16

## Context

Each vowel and diphthong has a spectral tilt, `svara_phoneme_spectral_tilt`:
`clamp((F1 − 300) / 250, 0, 2)`, from the Rust's `phoneme_spectral_tilt`
(`rust-old/src/phoneme.rs:789` at tag `3.6.5`). The Rust documents it as dB/octave,
"0 dB/oct at F1=300, ~2 dB/oct at F1=800", after Stevens & Hanson (1995): an open vowel's
source falls off more steeply than a close one's.

The Rust never applies it. `synthesize_vowel` computes it into `_tilt` and drops it, because
its tilt filter "significantly reduces signal energy at even modest tilt values"
(`phoneme.rs:1548-1553`). [ADR-0012](0012-the-glottal-spectral-tilt-reads-its-units.md)
found why: the filter read dB as hertz, so any tilt muted the voice by about 33 dB. 3.9.0
fixed the filter: `svara_glottal_set_spectral_tilt` now reads Klatt's TL, dB down at 3 kHz.
The maintainer deferred this decision until the vowel tilt could be measured through the
fixed filter.

**What applying it does.** Measured in svara 3.9.0 on the free vowel path (male voice, 0.2 s,
44.1 kHz), with the vowel's tilt set on its glottal source:

| Vowel | tilt | as TL directly: level / below 1 kHz / above 2 kHz | converted ×3.32: level / below 1 kHz / above 2 kHz |
|---|---:|---|---|
| /a/ | 1.87 | −1.46 / −0.15 / −1.74 dB | −4.63 / −0.81 / −5.74 dB |
| open /a/ | 1.78 | −1.54 / −0.11 / −1.88 dB | −4.53 / −0.63 / −5.90 dB |
| /æ/ | 1.47 | −1.11 / −0.08 / −1.59 dB | −3.33 / −0.44 / −4.96 dB |
| /ɜ/ | 1.12 | −0.52 / −0.04 / −1.13 dB | −1.53 / −0.21 / −3.62 dB |
| /o/ | 0.78 | −0.37 / −0.07 / −0.91 dB | −1.08 / −0.31 / −2.85 dB |
| /i/ | 0.16 | −0.11 / 0.00 / −0.19 dB | −0.37 / 0.00 / −0.62 dB |
| diphthongs | 0.70–1.87 | −0.37 to −0.51 / −0.10 to +0.10 / −0.39 to −0.74 dB | −0.83 to −1.07 / −0.27 to +0.23 / −0.87 to −1.40 dB |

The diphthongs move less because their target glides away from the start vowel the tilt is
computed from.

- **"As TL directly"** reads the 0-to-2 value as dB down at 3 kHz, the unit ADR-0012 gave every
  tilt.
- **"Converted ×3.32"** reads it as the dB/octave the Rust documents, held from 300 Hz to 3 kHz:
  log2(3000 / 300) = 3.32 octaves. /a/'s 1.87 dB/octave becomes 6.2 dB at 3 kHz.

Directly, the largest effect is under 2 dB above 2 kHz, near the edge of what a listener would
notice. Converted, open vowels lose 5 to 6 dB above 2 kHz against close vowels' under 1, the
contrast the function's comment describes.

**What it costs.** A tilted voice runs the one-pole filter and its memo check every sample: about
121 instructions a sample, 4.7% more for /a/. Today every vowel skips the filter, since its tilt
is 0. The memo could be inlined later to win some of that back.

**Where it would apply.** Vowels are voiced by three paths, and each sets its glottal source up
differently:

| Path | Used by | What applying it takes |
|---|---|---|
| free (`svara_ph_synth_vowel`, `_diphthong`, `_vowel_nasalized`) | `svara_phoneme_synthesize`, `svara_sequence_render` | set the tilt on the fresh source |
| context (`svara_synthctx_voiced`) | `svara_synthctx_synthesize`, the pool, the batch render, `svara_sequence_render_into` | set it per phoneme, and reset it to 0 for every other class, since the source is reused |
| `svara_sequence_render_planned` | itself | set it per event on the one source the render shares, so it steps at each boundary |

## Decision

**Apply each vowel's and diphthong's tilt in every path, converted from dB/octave over the
3.32 octaves from 300 Hz to 3 kHz.** The maintainer chose this on 2026-11-16, from two
questions:
1. **Apply it?** In every path, in the per-phoneme paths only (free and context), or never.
2. **In what unit?** As TL directly, or converted.

The reasons:
- The Rust meant to apply it.
- Converted, it gives the open-against-close contrast the function describes. Read directly
  as TL, it would move nothing a listener hears and still cost 4.7%.
- Every path keeps a vowel's timbre the same in every render.

**As shipped in 3.10.0:**
- `svara_phoneme_tilt_tl(p)` is `svara_phoneme_spectral_tilt(p) × log2(10)`, and 0 for every
  class but vowels and diphthongs. `svara_phoneme_spectral_tilt` now reads F1 into a target on
  the stack, so neither allocates, which a warmed context's zero bytes per phoneme needs.
- **The free path.** `svara_ph_synth_vowel`, `_diphthong` and `_vowel_nasalized` set the tilt
  on the source each one builds.
- **The context.** `svara_synthctx_synthesize` sets it in its preamble for every phoneme, so a
  consonant clears the vowel before it.
- **`render_planned`.** It sets each event's tilt when the event begins.
- **The tests.** `tests/synthpaths.tcyr` holds the free path and `render_planned` to 3.9.0's
  code plus this one line, bit for bit, and checks the context's tilt after each phoneme.

Whatever the choice, the phonemes that are not vowels or diphthongs keep a tilt of 0, and
`svara_phoneme_spectral_tilt` stays public. A change of output gets a minor of its own (3.10.0).

## Consequences

- **Positive** (if applied) — open vowels lose high-frequency energy against close ones, as
  Stevens & Hanson describe. The Rust's intent is met. The function stops being dead weight.
- **Negative** (if applied) — every vowel and diphthong changes, in every path it is applied to,
  and the sweep will show it. A consumer that balanced levels against today's vowels hears them
  softer and darker, by up to 4.6 dB converted or 1.5 dB direct. Vowels cost about 4.7% more
  instructions. It is a divergence from the oracle.
- **Neutral** — "never" keeps everything as it is: the function stays query-only metadata, the
  item closes, and the comments say why.

## Alternatives considered

The options are the two questions above. Two more were not measured:
- **Interpolate the tilt per sample through a diphthong or across `render_planned`'s events.**
  It would re-solve the coefficient on most samples, which is an `exp`, a `cos` and a `sqrt`
  each.
- **A true dB/octave slope.** ADR-0012 already set that aside.
