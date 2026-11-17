# 0014 — The Rosenberg pulse and its speed quotient

**Status**: Accepted (3.11.0)
**Date**: 2026-11-16

## Context

A 3.9.x quirk: `svara_glottal_set_speed_quotient` is ignored by the Rosenberg pulse.

**Why it is ignored.** Rosenberg's B pulse (Rosenberg 1971, JASA 49(2B):583–590) has two phases
within the open quotient:
- an opening phase, `3x² − 2x³` over Tp, rising from 0 to 1;
- a closing phase, `1 − y²` over Tn, falling back to 0.

The speed quotient is SQ = Tp / Tn, the asymmetry between them.

The Rust implements only the opening phase. At tag `3.6.5`, `rust-old/src/glottal.rs:465-473`
spreads `3t² − 2t³` over the whole open quotient, then drops from 1 to 0 at once. With no closing
phase, SQ has nothing to act on, and the doc comment says it is "available for future asymmetric
models" (`glottal.rs:461-463`). The same comment says the pulse "smoothly rises from 0 to 1 and back
to 0", which it does not. svara's `svara_glottal_rosenberg_pulse` is a line-for-line port.

**This is the pulse almost every voice uses.** Rosenberg is the default model:
- `svara_glottal_new` sets it, so every voice from `svara_voice_create_glottal_source` has it;
- every phoneme synth uses one of those sources;
- the context resets its source to Rosenberg before each phoneme.

The effort presets and `set_rd` switch to LF, and whisper and creaky are their own models. The
instant closure is the strongest excitation the tract gets: after lip radiation it is close to an
impulse every period.

**What the full pulse would do.** A prototype in a scratch copy, not in `src/`, implemented both
phases, with Tp = OQ·SQ/(1+SQ) and Tn = OQ/(1+SQ). It was measured on an /a/ and an /i/ through
the tract (male voice, 0.3 s, the first 0.1 s skipped), against today's pulse:

| Pulse | /a/ level | /a/ < 1 kHz | /a/ > 2 kHz | /i/ level | /i/ < 1 kHz | /i/ > 2 kHz |
|---|---:|---:|---:|---:|---:|---:|
| full Rosenberg B, SQ 1 | −22.7 | −13.4 | −26.5 | −15.3 | −12.5 | −26.8 dB |
| full Rosenberg B, SQ 2 (the default) | −20.1 | −11.2 | −23.4 | −9.0 | −6.1 | −23.8 dB |
| full Rosenberg B, SQ 4 | −15.5 | −6.3 | −18.9 | −4.7 | −1.8 | −19.8 dB |
| today's pulse at SQ 4 | 0 | 0 | 0 | 0 | 0 | 0 dB |

The last row confirms that SQ does nothing today. A correct Rosenberg B at the default SQ would make
every default voice 9 to 20 dB quieter and about 23 dB darker above 2 kHz. Every level and balance a
consumer has tuned against would move: prani, hoosh and anything built on `svara_phoneme_synthesize`.

## Decision

**Make the default pulse the full Rosenberg B, honouring the speed quotient, at a gain that keeps
3.10.0's loudness on /a/.** The maintainer chose option 2 below on 2026-11-16, over the opt-in model
this record first proposed.

Measuring that choice turned up something the first table did not show. The default /a/'s peak would
fall from 0.00478 to 0.00053, under the 0.001 the Rust's own integration test requires
(`rust-old/tests/integration.rs:6-15`): about −66 dBFS. Asked again with that number, the maintainer
chose to normalize the level to 3.10.0's.

**As shipped in 3.11.0:**
- **The pulse.** `svara_glottal_rosenberg_pulse` is the opening phase `3x² − 2x³` over Tp = OQ·SQ/(1+SQ),
  then the closing phase `1 − y²` over Tn = OQ − Tp, then 0. Both phases are scaled by
  `SV_G_ROSENBERG_GAIN`.
- **The gain** is 7.13033049099316, +17.06 dB. It is solved by iteration so that a default male /a/
  from `svara_phoneme_synthesize` (0.5 s at 44.1 kHz) has 3.10.0's RMS to within 1e-6 dB. One pass is
  not enough, because the breathiness noise, which the gain does not scale, is part of that RMS.
- **SQ = Tp / Tn**, the literature's definition, so 2, the default, closes twice as fast as it opens.
  The Rust's doc comment describes the default the other way round ("opening ~2x faster than
  closing"); its SQ never acted, so nothing depended on that.
- **A NaN speed quotient is refused.** It leaves the current value; ±inf clamp to the bounds
  [0.5, 5]. Through 3.10.0 a NaN was stored, which did no harm while SQ did nothing; now it would make
  the voice NaN. `svara_glottal_restore` goes through the same setter, so a restored NaN becomes the
  default 2.

The options as they were put:

1. **Add the full Rosenberg B as an opt-in model (first proposed).** A new `SVARA_GLOTTAL_ROSENBERG_B`
   that honours SQ, with today's pulse kept as the default. No existing output changes.
2. **Make the default pulse the full Rosenberg B (chosen).** Every default voice changes.
3. **Keep it inert, and say so.** No change; document that SQ does nothing.

## Consequences

- **Positive** — the Rosenberg pulse is Rosenberg's, with an opening and a closing phase, and the
  speed quotient shapes it. The Rust's comment ("smoothly rises from 0 to 1 and back to 0") is now
  true. The default /a/ keeps its loudness and passes the Rust's audibility test.
- **Negative** — every Rosenberg voice changes timbre: the closing phase replaces an instant drop, so
  there is far less high-frequency excitation. Normalized on /a/, other phonemes move by different
  amounts, RMS against 3.10.0 (male, 0.5 s):

  | /a/ | /e/ | /i/ | /o/ | /u/ | /ə/ | /aɪ/ | /m/ | /r/ | /z/ | /b/ |
  |---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
  | 0.00 | +0.38 | +7.51 | +2.09 | −0.87 | −1.31 | +17.94 | +5.01 | +5.17 | 0.00 | +0.65 dB |

  /aɪ/'s +17.9 dB is nearly the gain itself. The free diphthong path rebuilds its formant bank every
  64 samples, discarding its state, and its level is dominated by the transient each rebuild leaves,
  which scales with the pulse's amplitude. That is the 3.9.x diphthong-glide item, and this makes it
  more pressing.
- **Cost** — the glottal source retires 1.073x 3.10.0's instructions, and the full chain 1.018x.
- **Neutral** — a magic gain lives in `src/glottal.cyr`, documented with the rule that set it. If a
  later release changes the /a/ path, the gain does not move by itself; it was solved once, against
  3.10.0.

## Alternatives considered

- **Leave the full pulse unnormalized.** This record first argued for it, since the gain is one no
  paper specifies. It was set aside once the default /a/ turned out to fall below the Rust's
  audibility bar. Normalizing keeps loudness but not timbre: the high band still falls about 12 to
  18 dB further than the low band against 3.10.0.
- **The opt-in model, and keeping SQ inert** (options 1 and 3), as above.
- **Remove `set_speed_quotient`.** It is public API and part of the serialized state. Removing it
  breaks callers and stored states, which is a major version for no gain over option 3.
