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
tag = "3.4.0"
modules = ["dist/svara.cyr"]
```

⚠ **svara's bundle references hisab, naad, goonj and sakshi, and those resolve
from YOUR manifest, not svara's.** Declare them at the tags svara pins, or you
will get two versions of the same symbol in one flat namespace:

```toml
[deps.hisab]
git = "https://github.com/MacCracken/hisab.git"
tag = "2.11.2"
modules = ["dist/hisab.cyr"]

[deps.naad]
git = "https://github.com/MacCracken/naad.git"
tag = "2.2.1"
modules = ["dist/naad.cyr"]

[deps.goonj]
git = "https://github.com/MacCracken/goonj.git"
tag = "2.0.4"
modules = ["dist/goonj.cyr"]

# 3.4.0: svara's bundle carries src/logging.cyr, so `dist/svara.deps` now names
# sakshi. It is NOT a new dependency -- sakshi already arrived transitively via
# hisab, which svara's bundle requires -- but it is now explicit, and declaring
# it keeps resolution independent of hisab's own pin.
[deps.sakshi]
git = "https://github.com/MacCracken/sakshi.git"
tag = "2.4.11"
modules = ["dist/sakshi.cyr"]
```

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
`clock_gettime` calls, which at 44.1 kHz would dominate the ~0.56 µs it takes to
synthesize a sample. `scripts/check-logging.sh` asserts that structurally.

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
# ... SvGlottalState_to_json / _from_json_str ...
var g2 = svara_glottal_restore(st);
if (svara_is_err(g2) == 1) { return g2; }
```

Three things to know before relying on it:

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
