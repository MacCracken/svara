# svara

**svara** (Sanskrit: स्वर — voice / tone / musical note) — Formant and vocal synthesis for Cyrius.

Complete formant-based vocal synthesis pipeline: dual glottal source models (Rosenberg B + LF),
an SOA formant filter bank, a 101-phoneme inventory, prosodic control, look-ahead coarticulation,
and spectral analysis. Built on [hisab](https://github.com/MacCracken/hisab) for math and
[naad](https://github.com/MacCracken/naad) for DSP primitives.

Cyrius port of an 8,785-line Rust library; the original is preserved at `rust-old/` as the parity
oracle. See [`docs/development/state.md`](docs/development/state.md) for current state.

## Features

- **Dual glottal models**: Rosenberg B polynomial + Liljencrants-Fant (LF) with Rd voice quality
  parameter, plus whisper (noise-only) and creaky (irregular LF) variants
- **SOA formant filter**: structure-of-arrays biquad bank, fixed `SVARA_MAX_FORMANTS = 8` slots,
  svara's own topology (not naad's `BiquadFilter`)
- **101 phonemes**: 22 vowels, 5 diphthongs, 10 plosives, 22 fricatives, 8 affricates, 6 nasals,
  7 approximants/laterals, 3 trills, 3 taps/flaps, 5 clicks, 5 ejectives, 3 implosives, glottal
  stop, silence — `SVARA_PH_*` constants `0..100`, in Rust variant order
- **Hillenbrand formant data**: per-vowel frequencies and bandwidths from Hillenbrand et al. (1995)
- **Vocal tract**: parallel formant bank + nasal coupling (place-dependent) + subglottal resonance
  + lip radiation + source-filter interaction + DC blocking + gain normalization
- **Prosody**: monotone cubic f0 contours, 4 intonation patterns, 9 tones, stress, Catmull-Rom
  trajectory interpolation
- **Coarticulation**: look-ahead onset, sigmoid crossfades, per-phoneme resistance coefficients
  (Recasens DAC), F2 locus equations
- **Voice profiles**: male/female/child presets with f0-dependent bandwidth scaling, vibrato, and
  builders; a whisper..shout vocal-effort continuum
- **Spectral analysis**: FFT-based spectrum, formant estimation, band energy, compensated RMS
- **LOD**: Full / Reduced / Minimal quality levels that skip pipeline stages for background voices
- **Performance**: ~40× real-time at 44.1 kHz on one core, f64 end-to-end — see
  [`docs/benchmarks.md`](docs/benchmarks.md)

## Quick Start

The Cyrius surface is prefixed free functions — `svara_*` functions, `SVARA_*` constants,
`SV_*` internal literals, `Sv*` struct types — so the svara bundle coexists with naad's in
Cyrius's flat namespace. Handles are opaque pointers passed as the first argument; struct fields
are reached through `#derive(accessors)` pairs (`SvVoiceProfile_base_f0` /
`SvVoiceProfile_set_base_f0`). Fallible calls return a negative `SVARA_ERR_*` code rather than a
`Result` — test with `svara_is_err`, name with `svara_err_name`.

```cyrius
include "lib/hashmap.cyr"
include "lib/bayan.cyr"
include "dist/svara.cyr"

fn main() {
    alloc_init();

    # A male voice profile; builders mutate and return the profile.
    var voice = svara_voice_with_breathiness(svara_voice_new_male(), 0.03);
    var sr = f64_from(44100);

    # One phoneme -> a vec<f64> of samples, or a negative SVARA_ERR_*.
    var pcm = svara_phoneme_synthesize(SVARA_PH_VOWEL_A, voice, sr, 0.5);
    if (svara_is_err(pcm) == 1) {
        println(svara_err_name(pcm));
        return 1;
    }
    println_int(vec_len(pcm));   # 22050

    # A sequence -> coarticulated speech.
    var seq = svara_sequence_new();
    svara_sequence_push(seq, svara_phoneme_event_new(SVARA_PH_VOWEL_A, 0.15, SVARA_STRESS_PRIMARY));
    svara_sequence_push(seq, svara_phoneme_event_new(SVARA_PH_NASAL_N, 0.08, SVARA_STRESS_UNSTRESSED));
    svara_sequence_push(seq, svara_phoneme_event_new(SVARA_PH_VOWEL_I, 0.15, SVARA_STRESS_SECONDARY));
    var audio = svara_sequence_render(seq, voice, sr);
    if (svara_is_err(audio) == 1) {
        println(svara_err_name(audio));
        return 1;
    }
    println_int(vec_len(audio));
    return 0;
}

var r = main();
syscall(SYS_EXIT, r);
```

For real-time use, `svara_synthctx_*` (a reusable tract + glottal + scratch buffer) and
`svara_pool_*` avoid the per-note allocation the free `svara_phoneme_synthesize` path pays;
`svara_tract_synthesize_into` renders into a caller-owned buffer.

## Building

```sh
cyrius deps                              # resolve dependencies into lib/
cyrius build src/main.cyr build/svara    # compile the smoke entry
cyrius tests tests                       # run the .tcyr suites
```

`src/main.cyr` is a smoke entry that links every module and exercises the pipeline end to end —
it is not the consumption path. Consumers pull the distlib bundle instead.

## Consuming svara

svara ships as a **distlib bundle**: `cyrius distlib` concatenates the `[lib] modules` listed in
`cyrius.cyml` — in dependency order, each self-contained, no `include` lines — into
[`dist/svara.cyr`](dist/svara.cyr), with [`dist/svara.deps`](dist/svara.deps) as a sidecar naming
the stdlib leaves the fold requires. This is how dhvani and vansh consume svara.

A consumer declares svara alongside the deps the bundle resolves against — the bundle does not
carry them:

```toml
[deps.svara]
git = "https://github.com/MacCracken/svara.git"
tag = "3.1.2"
modules = ["dist/svara.cyr"]

# svara's bundle references these; they resolve from the CONSUMER's manifest.
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
```

Pin exactly these tags: naad 2.2.1 itself pins hisab 2.11.2 and goonj 2.0.4, so a consumer that
bundles svara *and* naad *and* hisab in one flat namespace gets a coherent symbol set rather than
two versions of the same name.

`bayan` and `hashmap` are **opt-in via explicit `include`**, not `[deps].stdlib` entries. Include
both before the svara bundle if you use the `#derive(Serialize)` JSON codecs (or simply to build
warning-free — the derived `Type_to_json` / `_from_json` bodies reference bayan whether or not you
call them).

## Dependencies

svara has no crates.io dependencies — everything is a Cyrius git dependency pinned by tag in
`cyrius.cyml`. See [`docs/development/dependency-watch.md`](docs/development/dependency-watch.md)
for the full consumption map and the flat-namespace rename hazard.

| Dep | Pin | What svara uses |
|---|---|---|
| [hisab](https://github.com/MacCracken/hisab) | `2.11.2` | `num_fft` + `num_neumaier_sum` (spectral), `calc_monotone_cubic` (prosody), `ease_in_out_smooth` (phoneme / trajectory / sequence) |
| [naad](https://github.com/MacCracken/naad) | `2.2.1` | biquads + `NAAD_FILTER_NOTCH`/`_BANDPASS` (tract), noise + LFO (glottal) |
| [goonj](https://github.com/MacCracken/goonj) | `2.0.4` | nothing directly — the naad bundle references it, so it must resolve |
| [sakshi](https://github.com/MacCracken/sakshi) | `2.4.11` | transitive via hisab; goonj's logging backend |

**No feature flags.** Cyrius has none, and the port collapsed the Rust CFG split: svara carries
only the naad-backend path (naad's biquad / noise / LFO), with no internal fallback. See
[`docs/development/state.md`](docs/development/state.md) "Port decisions".

## Architecture

```
GlottalSource (Rosenberg/LF) → VocalTract → Output
   svara_glottal_*                │  svara_tract_*
                    ┌─────────────┼──────────────┐
                    │             │              │
              FormantFilter   Nasal         Lip Radiation
              (SOA biquad     Coupling       + Subglottal
               bank, 8-wide)  (place-dep)    + Interaction
              svara_formant_*
```

More in [`docs/architecture/`](docs/architecture/) and the ADRs under [`docs/adr/`](docs/adr/).

## Performance

A full glottal → formant → tract per-sample chain runs ≈ 0.56 µs/sample on x86_64 — roughly
**40× real-time at 44.1 kHz on one core**. Per-unit: glottal ~82 ns, formant bank ~179 ns, tract
~295 ns per sample.

Cyrius emits **scalar** code; there is no auto-vectorization. A bit-identical `f64v4` (AVX2)
version of the formant bank was prototyped in 3.1.0 and **reverted** — it passed tolerance but
bought only ~5%, because the loop is memory-bound (the SOA state shuffle), not compute-bound.
A same-machine head-to-head against the Rust oracle puts the per-DSP-unit gap at 10–38×; the
diphthong render is the exception, where 3.1.0's control-rate coefficient updates (5.42 ms →
0.94 ms) made Cyrius *faster* than Rust, which still re-solves per sample.

Numbers and method: [`docs/benchmarks.md`](docs/benchmarks.md); head-to-head:
[`docs/benchmarks-rust-v-cyrius.md`](docs/benchmarks-rust-v-cyrius.md); remaining work: **M-perf**
in [`docs/development/roadmap.md`](docs/development/roadmap.md).

## Consumers

- **dhvani** — AGNOS audio engine (mixing / effects / playback); pulls `dist/svara.cyr`
- **vansh** — voice shell (text → phoneme sequences → `svara_sequence_render`); pulls
  `dist/svara.cyr`

An end-to-end consumer build against the bundle is still an open item (see
[`docs/development/state.md`](docs/development/state.md) "Next").

Sibling projects that svara does *not* depend on are served through the dependency-free bridge
functions in `src/bridge.cyr` — 19 scalar maps turning **bhava** affect, **vansh** speech rate,
**prani** creature body size/age, **goonj** acoustics and **badal** weather into svara synthesis
parameters. See [ADR-004](docs/architecture/adr-004-scope-boundaries.md) for the boundaries.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

GPL-3.0-only
