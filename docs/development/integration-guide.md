# svara Integration Guide

> Rewritten 2026-08-26 for the Cyrius port. The pre-port version was Rust —
> `use svara::prelude::*`, `vec![0.0f32; 512]`, and a Cargo feature table for
> features this library does not have.

## Depending on svara

svara ships as a **distlib bundle**: `dist/svara.cyr` is Cyrius source that
compiles into *your* binary. Add it to your `cyrius.cyml`:

```toml
[deps.svara]
git = "https://github.com/MacCracken/svara.git"
tag = "3.13.0"
modules = ["dist/svara.cyr"]
```

⚠ **svara's bundle references hisab, naad and goonj, and those resolve from YOUR
manifest, not svara's.** Declare them at the tags svara pins, or you will get two
versions of the same symbol in one flat namespace:

```toml
[deps.hisab]
git = "https://github.com/MacCracken/hisab.git"
tag = "3.5.0"
modules = ["dist/hisab.cyr"]

[deps.naad]
git = "https://github.com/MacCracken/naad.git"
tag = "2.3.5"
modules = ["dist/naad.cyr"]

[deps.goonj]
git = "https://github.com/MacCracken/goonj.git"
tag = "2.3.2"
modules = ["dist/goonj.cyr"]
```

You need **cyrius 6.7.2 or later**, measured with a from-scratch consumer of the
bundle at each release.

**sakshi is a stdlib leaf**, not a package: put `"sakshi"` in `[deps].stdlib` if
you call it yourself or build with `-D LOGGING`. Do not declare a
`[deps.sakshi]` package. hisab dropped its own in 3.4.2, and a package of that
name now draws `warning: refusing to overwrite stdlib leaf 'sakshi'` on every
build.

⚠ **Move naad and svara together.** From naad 2.3.4 a refused allocation in
`noise_new` or `modulation_lfo_new` returns an error code instead of stopping
inside naad, and svara 3.6.3 kept that code as a handle. svara 3.6.4 reports it.

`dist/svara.deps` lists the stdlib leaves the bundle needs; `cyrius deps` reads
it. **Include order matters** — hisab → goonj → naad → svara.

**There are no feature flags**, but there is one compile-time switch.
Cyrius has no feature system, so the Rust's `std` / `naad-backend` features are
gone — svara carries only the naad-backend path. The Rust's `logging` feature
survives as `-D LOGGING`:

```sh
cyrius build -D LOGGING src/main.cyr build/myapp
```

Off by default and **compiled out entirely** when off — no log call, no runtime
level check, no call frame. On, svara routes to **sakshi**, so its spans nest
inside yours and correlate across the AGNOS stack. `SVARA_LOG` selects the level
(`off`/`fatal`/`error`/`warn`/`info`/`debug`/`trace`, default `info`); `off`
silences span events too, which sakshi alone would not do. Call
`svara_log_init()` once at startup, and `sakshi_output_*` first if you want a
sink other than stderr.

Only four coarse entry points are instrumented — sequence render, batch render,
synthesis-context render, pool render. **Never per-sample**: a span costs two
`clock_gettime` calls, which at 44.1 kHz would dominate the ~0.15 µs it takes to
synthesize a sample ([benchmarks](../benchmarks.md)). `scripts/check-logging.sh` asserts that structurally.

## Errors: codes, not exceptions

Cyrius has no `Result` idiom here. A function that can fail returns either a
valid value/pointer or a **negative** `SVARA_ERR_*`. Test with `svara_is_err`:

```
var samples = svara_phoneme_synthesize(SVARA_PH_VOWEL_A, voice, sr, dur);
if (svara_is_err(samples) == 1) {
    # svara_err_name(samples) gives static diagnostic text
    return samples;
}
```

A function returning a *pointer* signals failure with **0**, not a negative —
`svara_tract_synthesize` is the one to watch. Check both forms; writing through
a 0 writes to address 0.

## Quick start

```
var sr = f64_from(44100);
var voice = svara_voice_new_male();

# one phoneme -> vec<f64>
var samples = svara_phoneme_synthesize(SVARA_PH_VOWEL_A, voice, sr, 0.5);
if (svara_is_err(samples) == 1) { return samples; }

# or a sequence, with coarticulation crossfades at the boundaries
var seq = svara_sequence_new();
svara_sequence_push(seq, svara_phoneme_event_new(SVARA_PH_VOWEL_A, 0.15, SVARA_STRESS_PRIMARY));
svara_sequence_push(seq, svara_phoneme_event_new(SVARA_PH_NASAL_N, 0.08, SVARA_STRESS_UNSTRESSED));
svara_sequence_push(seq, svara_phoneme_event_new(SVARA_PH_VOWEL_I, 0.15, SVARA_STRESS_SECONDARY));
var audio = svara_sequence_render(seq, voice, sr);
```

`svara_sequence_render_planned` is the alternative: continuous formant
trajectories interpolated per sample instead of per-phoneme renders crossfaded
together. It is the path that honours per-event `tone`.

## Real-time streaming

⚠ **This is the section that matters most in Cyrius, because the bump allocator
never frees.** Memory returns to the OS at process exit and nowhere else, so an
allocation in an audio callback is not a leak that grows slowly — it is a leak
that ends the process.

Use the `_into` forms, which write into a buffer you own:

```
var glottal = svara_voice_create_glottal_source(voice, sr);
if (svara_is_err(glottal) == 1) { return glottal; }
var tract = svara_tract_new(sr);
if (svara_is_err(tract) == 1) { return tract; }
var e = svara_tract_set_vowel(tract, SVARA_VOWEL_A);
if (svara_is_err(e) == 1) { return e; }

# allocate ONCE, outside the callback
var frames = 512;
var buffer = svara_alloc_samples(frames);
if (buffer == 0) { return SVARA_ERR_COMPUTATION; }

# each callback: zero allocation
svara_tract_synthesize_into(tract, glottal, buffer, frames);
```

`svara_synthctx_*` and `svara_pool_*` are the same idea one level up — a reusable
synthesis context whose sample buffer grows but is never re-allocated per call.

The per-sample synthesis path inside svara allocates nothing; that is pinned by
`tests/allocbudget.tcyr`. Keep your side of the boundary the same way.

**Per phoneme**, measured at 3.6.5, male voice, 24 kHz:

- A warmed `svara_synthctx_*` context allocates **nothing** per phoneme, for every
  class (`tests/allocbudget.tcyr` pins it). The first phoneme of a class builds
  what the context keeps for it, such as a frication filter. `svara_synthctx_new`
  is 2,280 bytes. Through 3.6.4 a warmed context kept 160 to 1,048 bytes a phoneme.
- ⚠ `svara_sequence_render` keeps every sample about twice, and the default
  allocator never frees. Each phoneme gets a new glottal source, tract and sample
  buffer, and the crossfade writes the output into one more. 3.8.3 measured 24 bytes
  per output sample on four phonemes at 44.1 kHz. Before 3.8.3 the vecs doubled as
  they grew: 81 bytes on the same four, and hoosh measured 127 on twelve.

**A server should stream a sequence** with `svara_sequence_render_into` (3.8.0).
It renders on a context you keep and hands the samples to your sink as soon as no
later crossfade can change them. On a context that has already streamed a
sequence as long, it allocates nothing (`tests/allocbudget.tcyr`):

```
# your sink: called with `count` final samples, valid only during the call
fn my_sink(user, samples, count) {
    write_pcm(user, samples, count);
    return 0;                       # the return value is ignored
}

var ctx = svara_synthctx_new(voice, sr);   # once per worker, or per request
var n = svara_sequence_render_into(seq, voice, ctx, &my_sink, conn);
if (svara_is_err(n) == 1) { return n; }    # n: samples passed to the sink
```

- **The plan is `svara_sequence_render`'s**: the same stress pitch, durations,
  nasalization, crossfade lengths and sigmoid crossfade. The length is the same
  too. The samples differ, because each phoneme renders on your context, which
  carries glottal and noise state from one phoneme to the next, rather than on a
  fresh source and tract. `tests/stream.tcyr` pins the stream bit for bit against
  `svara_seq_crossfade` over the segments the same context renders.
- **It holds back at most one crossfade.** A fade-in can reach back past the
  previous phoneme into the one before, whenever the transition window is longer
  than a short consonant, so the stream holds back the longest fade-in in the
  plan, not just the next one.
- **What a later event would refuse is refused before the first sample**: an
  invalid phoneme, a bad duration, or a stressed f0 outside 20 to 2,000 Hz. A
  failure only rendering can find, such as a refused allocation, can still come
  mid-stream, and the samples already passed on stand.
- Give each thread its own context. A voice may be shared.

## Threads

**Since 3.6.5, threads can render at once if they share no object.** Give each
thread its own context, tract, glottal source, contour and buffers. A voice
profile is only read, so threads may share one, as the suite's do. `tests/concurrency.tcyr`
checks this: four threads render 4,000 vowels, and two or four threads each hammer
the tract, glottal, prosody and spectral paths. Every result is compared bit for
bit with one computed alone.

- ⚠ **One object used by two threads at once is not safe.** Contexts, tracts,
  sources, contours and filters take no lock of their own.
- The spectral functions, and prosody contours of more than 64 points, use
  module-level buffers under a lock, so threads in them take turns there. Until
  a thread has been created, the lock costs a load and a branch.
- ⚠ **With `-D LOGGING`, keep one render at a time.** The four instrumented entry
  points open spans on sakshi's span stack, and sakshi 2.5.10 keeps one unlocked
  stack per process, so concurrent renders would interleave their spans. The
  concurrency suite runs with logging off.

**Through 3.6.4 this was not safe.** Module-level scratch was shared between
objects, and 2 to 7 of 4,000 concurrent renders came back wrong while reporting
success. On 3.6.4 or earlier, hold one lock around every call into svara.

## Bridge functions

19 scalar maps that turn a sibling component's output into a svara parameter,
with no dependency between the two.

```
# from bhava (affect)
var rd = svara_bridge_rd_from_arousal(0.8);            # excited -> pressed voice
var br = svara_bridge_breathiness_from_arousal(0.8);

# from goonj (acoustics)
var gain = svara_bridge_gain_from_distance(F64_ONE, f64_from(5));   # 5 m away

# from badal (weather)
var effort = svara_bridge_lombard_effort_from_noise(f64_from(70));  # 70 dB SPL
```

## Spectral tilt (3.9.0)

`svara_glottal_set_spectral_tilt` reads its value as Klatt's TL: the voiced pulse's
attenuation at 3 kHz, in dB below its gain at 0 Hz. Through 3.8.8 it read the value as
the corner of a low-pass in hertz, as the Rust does, so any tilt above 0 muted the
voice by about 33 dB. A voice built at soft or normal effort
(`svara_voice_create_glottal_source_with_effort`) is therefore louder and brighter
since 3.9.0: normal now sits within a dB of loud instead of 26 dB below it. Whisper is
unchanged. Retune anything balanced against the old levels. The reasons are in
[ADR-0012](../adr/0012-the-glottal-spectral-tilt-reads-its-units.md).

**Since 3.10.0 every vowel and diphthong carries its own tilt**, in every render path
([ADR-0013](../adr/0013-apply-the-per-vowel-spectral-tilt.md)). The value is
`svara_phoneme_tilt_tl(p)`, which is `svara_phoneme_spectral_tilt(p)`'s dB/octave held from
300 Hz to 3 kHz. Open vowels come out darker and softer than in 3.9.0: /a/ loses 4.6 dB
overall and 5.7 dB above 2 kHz. Close vowels barely move: /i/ loses 0.4 dB.

## The Rosenberg pulse (3.11.0)

The default glottal model's pulse is Rosenberg's B in full since 3.11.0: an opening phase, then a
closing phase, with `svara_glottal_set_speed_quotient` setting their ratio. Through 3.10.0 it was
the Rust's opening phase alone, closed at once, and the speed quotient did nothing. Every voice on
the default model sounds darker.

A gain keeps a default male /a/ at 3.10.0's loudness, but other phonemes move against 3.10.0:
- /i/ gains 7.5 dB, /m/ and /r/ about 5, and /o/ 2;
- /u/ and /ə/ lose about 1;
- the diphthong /aɪ/ gains 17.9 dB.

Voices built at an effort level use the LF model and do not change. The details are in
[ADR-0014](../adr/0014-the-rosenberg-pulse-and-its-speed-quotient.md).

## Moving formants (3.12.0)

Through 3.11.0, as in the Rust, every retarget of the formant bank also reset it:
- `svara_sequence_render_planned`, which retargets every sample, produced no formants at all. Its
  /a/, /i/ and /u/ had the same spectrum.
- The context's diphthongs had no formants either.
- The free path's diphthongs were mostly the transients of a reset every 64 samples.

Since 3.12.0 those paths retarget with `svara_tract_retarget`, which keeps the resonators' state.
`render_planned`'s vowels carry their formants, and diphthongs glide in both paths, at about
−53 dB for /aɪ/. `render_planned`'s levels move by vowel: /a/ +12 dB, /i/ +6 dB and /u/ +29 dB
against 3.11.0. `svara_tract_set_formants_from_target` still resets, for a one-off setting. See
[ADR-0015](../adr/0015-retarget-the-formant-bank-without-resetting-it.md).

## Tone in every render (3.13.0)

An event's lexical tone (`svara_phoneme_event_with_tone`) is voiced by every sequence render since
3.13.0:
- `svara_sequence_render`;
- the stream, `svara_sequence_render_into`;
- the batch render;
- `svara_sequence_render_planned`, as before.

Through 3.12.0, as in the Rust, only `render_planned` voiced it, and the others gave the same samples
with or without a tone. Untoned input is unchanged, bit for bit.

To voice a tone on a single phoneme through a context of your own, call
`svara_synthctx_synthesize_toned`. A glottal source of your own takes one through
`svara_glottal_set_tone`. See [ADR-0016](../adr/0016-honour-tone-in-every-sequence-render.md).

## Quality scaling

For multi-voice scenes, drop pipeline stages on background voices:

```
svara_tract_set_quality(tract, SVARA_QUALITY_FULL);     # all stages
svara_tract_set_quality(tract, SVARA_QUALITY_REDUCED);  # 3 formants, no subglottal
svara_tract_set_quality(tract, SVARA_QUALITY_MINIMAL);  # 2 formants, no extras
```

## Saving and restoring

Configuration serializes directly; engine objects go through a flat **state
companion** plus `save` / `restore`. `bayan` and `hashmap` are opt-in, so an
entry file that calls a codec must include them:

```
include "lib/hashmap.cyr"
include "lib/bayan.cyr"
```

```
# configuration: derived codecs
var sb = str_builder_new();
SvVoiceProfile_to_json(voice, sb);
var json = str_data(str_builder_build(sb));
var restored = SvVoiceProfile_from_json_str(json);

# engine object: save -> JSON -> restore
var st = svara_glottal_save(glottal);
# ... SvGlottalState_to_json ...

# text from outside svara: check it before decoding
if (svara_validate_json(text) != SVARA_ERR_NONE) { return SVARA_ERR_COMPUTATION; }
var g2 = svara_glottal_restore(SvGlottalState_from_json_str(text));
if (svara_is_err(g2) == 1) { return g2; }
```

Four things to know before relying on it:

- **Check text svara did not write with `svara_validate_json` first.** The decoders
  are generated by the cyrius compiler, and at 6.7.18 a damaged document can hang
  one or make it read past the end of the string, before `restore` runs
  ([ADR-0009](../adr/0009-what-the-3-6-3-audit-leaves-open.md)). The check accepts
  everything svara writes, and refuses valid JSON that svara does not write, such as
  a pretty-printed document.

- **Decode with `_from_json_str`, never the pairs-form `_from_json`.** For any
  type with a `Vec<struct>` field the pairs form returns an **empty vec**,
  silently.
- **`restore` validates.** A state that has been through JSON is
  externally-authored data, so an out-of-range `f0` or a bad sample rate comes
  back as an error, not a broken object.
- **Restore does not reproduce filter delay lines**, deliberately: Rust's
  `set_formants` discards them on every call too, so there is nothing there to
  preserve. Everything else — including naad's noise and LFO state, as of 3.3.2 —
  is carried, so a restored glottal source is bit-exact. Both limits are
  measured, not assumed — see
  [ADR-0002](../adr/0002-serialization-boundary.md).

## Consumers

| Component | How it uses svara |
|---|---|
| **dhvani** | receives sample buffers; mixes, processes, plays |
| **vansh** | converts text to phoneme sequences, calls render |
| **prani** | creature voice profiles via the bridge functions |
| **bhava** | affect → prosody parameters, through `bridge.cyr` |
