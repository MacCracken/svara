# 0012 — What the glottal spectral tilt's value means

**Status**: Accepted (3.9.0)
**Date**: 2026-11-16

## Context

The roadmap's first 3.9.x item is "unapplied per-vowel spectral tilt". Each vowel has a tilt
(`svara_phoneme_spectral_tilt`: 0 to 2 dB/octave, from F1), and nothing applies it.

The Rust leaves it unapplied on purpose. At tag `3.6.5`, `rust-old/src/phoneme.rs:1548-1553`
computes the tilt into `_tilt` and drops it, "since the one-pole tilt filter significantly
reduces signal energy at even modest tilt values". svara ports that and says so in
`svara_ph_synth_vowel`.

The reason is in the filter. `GlottalSource::set_spectral_tilt` documents its value as dB/octave
(`rust-old/src/glottal.rs:255`), but the voiced pulse's one-pole low-pass computes:

```
alpha = exp(-2π · tilt / sample_rate)          # glottal.rs:380-389
```

That is the coefficient of a one-pole low-pass whose cutoff is `tilt` **hertz**. A tilt of 2
becomes a 2 Hz low-pass in front of a 120 Hz voice. svara's `svara_glottal_next_sample` is the
same formula, line for line.

**It reaches every voice built with an effort level**, not only the per-vowel tilt.
`svara_voice_create_glottal_source_with_effort` sets the preset's tilt, at the Rust's values
(`rust-old/src/voice.rs:70-113`): whisper 12, soft 6, normal 2, loud 0, shout 0. Measured in
svara 3.8.8, an /a/ for 1 s at 44.1 kHz through the tract, skipping the first 0.1 s:

| Effort | tilt | output level | with the prototype below |
|---|---:|---:|---:|
| whisper | 12 | −101.3 dB | −101.3 dB (its own filter, unchanged) |
| soft | 6 | −68.4 dB | −61.0 dB |
| **normal** | 2 | **−85.9 dB** | −60.4 dB |
| loud | 0 | −60.0 dB | −60.0 dB |
| shout | 0 | −63.5 dB | −63.5 dB |

Today "normal" effort comes out 26 dB below "loud" and 17 dB below "soft". It sits below soft
only because soft's breathiness noise is added after the filter.

Applying /a/'s own tilt, 1.87, through today's filter drops the output 33.6 dB. That is the
energy loss the Rust's comment describes. Any tilt from 1 to 12 costs about 33 dB, since every
one of them is a low-pass far below the voice.

**The whisper path is different, on purpose.** It filters its noise with `tilt.max(12)`, a 12 Hz
one-pole (`glottal.rs:436-443`). Its comment says the filter integrates the noise, and it scales
the input by 0.15 to suit. Its level is tuned to that integrator.

## Decision

**Read the voiced pulse's tilt as Klatt's TL: the filter's attenuation at 3 kHz, in dB below its
gain at 0 Hz.** The maintainer chose this over keeping the Rust's filter on 2026-11-16, and chose
to decide the per-vowel tilt separately, later. The one-pole coefficient is solved from the tilt
once per change, not per sample:

```
g     = 10^(−TL / 10)                  # the power gain wanted at 3 kHz
c     = cos(2π · 3000 / sample_rate)
alpha = ((1 − g·c) − sqrt((1 − g·c)² − (1 − g)²)) / (1 − g)      # 0 when TL is 0
```

- The presets keep their numbers. 2, 6 and 12 dB at 3 kHz are the mild-to-steep range Klatt's
  synthesizers use.
- The whisper path keeps its 12 Hz integrator. Its comment and docs say what it is, not dB/octave.
- `svara_glottal_set_spectral_tilt` and the effort docs say "dB of attenuation at 3 kHz".
- The per-vowel tilt stays unapplied. That is a separate decision, made after this one by
  measuring what it does to each vowel. With the fixed filter, /a/'s 1.87 costs 1.5 dB.
- A tilt of 0 or below, or NaN, leaves the pulse unfiltered, as before. A tilt of +inf, or one
  large enough that 10^(−TL/10) rounds to 0, mutes the voiced pulse: alpha is 1. Through 3.8.8
  such a value meant a corner above Nyquist and left the pulse unfiltered.

**As shipped in 3.9.0:**
- `svara_glottal_tilt_alpha(tl, sample_rate)` solves the coefficient in the form above, which
  does not cancel at a small TL.
- `svara_glottal_tilt_alpha_for` memoizes it on the exact bits of the tilt and the sample rate,
  in three fields appended to `SvGlottalSource`. A tilt written through the derived accessor, or
  by `svara_glottal_restore`, is therefore solved too.
- `tests/glottal.tcyr` checks the coefficient's gain at 3 kHz against TL, for TL 0.1 to 40 at 16,
  44.1 and 96 kHz.
- `tests/voice.tcyr` checks the effort order: normal within a dB of loud, above soft.

The right-hand column above is from a prototype of this, in a scratch copy; 3.9.0 gives the same.

## Consequences

- **Positive** — the effort scale is in order: soft −61.0, normal −60.4, loud −60.0 dB. A tilt
  now shapes the spectrum instead of muting the voice. The per-vowel tilt becomes a real option.
- **Negative** — output changes for every voice built at soft or normal effort (whisper, loud and
  shout do not change), and for any caller of `svara_glottal_set_spectral_tilt` with a value
  above 0. This is a divergence from the oracle, and needs a minor (3.9.0).
  - The sweep moves the effort-built voices; every other value stays the same bits.
  - Callers who tuned around the old behaviour hear a louder, brighter voice.
- **Neutral** — the computation moves from per sample to per tilt change. Today the per-sample
  `exp` is the cost; with the change it runs once.

## Alternatives considered

- **Keep the Rust's filter.** No output moves, and the quirk is documented, not fixed. Rejected in
  this proposal: "normal" effort 26 dB below "loud" is not a behaviour a consumer can want, and the
  Rust's own comment treats the energy loss as a defect it worked around.
- **A true dB/octave tilt.** A one-pole filter cannot hold a constant slope; it falls 6 dB/octave
  above its corner. A tilt in dB/octave needs a cascade fitted over the speech band: more design,
  more cost per sample, and no convention to match.
- **Fix whisper too.** Its level is tuned to the integrator, through the 0.15 scale. Changing it
  would need a new whisper level, and whisper sounds as intended today.
- **Apply the per-vowel tilt in the same release.** That changes every vowel in every render path,
  which is a bigger change than the effort fix. It deserves its own measurement and release.
