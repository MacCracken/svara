# A server's render keeps what each phoneme allocates, and there is no streaming sequence render — PARTLY RESOLVED

**Status:** 🟡 **PARTLY RESOLVED in 3.6.5**: fix 1 shipped, fixes 2 and 3 are open (see *Resolution so far* at the end)
**Placement:** fix 1 in 3.6.5. Fixes 2 and 3 add public API, so they wait for a minor: roadmap 3.8.x, "A streaming sequence render".
**Re-verified:** against 3.6.4 on 2026-11-16: every figure above is unchanged (696,280 bytes, 127 per sample; the per-phoneme table; 1,880 for `svara_synthctx_new`).
**Where the fix would live:**
- the per-class context renders: `svara_synthctx_fricative` (`src/phoneme.cyr:1382`), `svara_synthctx_plosive`
  (`:1417`), `svara_synthctx_affricate` (`:1463`) and the `svara_vowel_target_new` in `svara_synthctx_voiced` (`:1344`), each of
  which builds a formant filter or a target per call;
- `svara_sequence_render` (`src/sequence.cyr:320`) and `svara_seq_crossfade` (`:243`), for the streaming form below.

**Discovered:** 2026-11-11, building hoosh 2.12.2's `POST /v1/audio/speech` (hoosh's ADR 016).
**Reporter:** hoosh 2.12.2
**Severity:** Medium.
- **Why it matters.** Cyrius's default allocator never frees. A server renders on every request, so whatever a
  render allocates stays for the life of the process.
- **Who can reach it.** Any long-running consumer. The integration guide's *Real-time streaming* section says the
  per-sample path allocates nothing (pinned by `tests/allocbudget.tcyr`), and it does not. Per phoneme, it does.

**Affects:** svara 3.6.3 (tag `3.6.3`, `7783d18`), built with cyrius 6.7.18.

## Summary

[`repros/2026-11-11-a-server-render-keeps-what-each-phoneme-allocates.cyr`](repros/2026-11-11-a-server-render-keeps-what-each-phoneme-allocates.cyr)
measures `alloc_used()` around each render (male voice, 24 kHz):

1. **`svara_sequence_render` keeps 127 bytes per output sample:** 696,280 bytes for a 12-phoneme, 5,460-sample
   sequence. Every phoneme gets a new glottal source, tract and sample vector, and the crossfade pushes into a new
   vector. hoosh measured about 88 bytes per sample on longer sentences: 4.5 MB for two seconds of speech.
2. **A warmed synthesis context still keeps something on most phonemes**, rendered for 0.1 s each:

   | phoneme | bytes kept |
   |---|---:|
   | vowels /a/, /i/, /o/; nasal /m/; lateral /l/ | 160 each (a `VowelTarget`) |
   | diphthong /ai/ | 400 |
   | fricatives /s/, /z/ | 816, 976 |
   | affricate /tʃ/ | 976 |
   | plosives /p/, /d/ | 1,048 each |
   | silence | 0 |

   And `svara_synthctx_new` is 1,880 bytes.

hoosh renders through the context path, one fresh context per request over a sample buffer the worker thread keeps,
and it crossfades and writes the reply as it goes. Its samples never reach the heap. What a request still keeps from
svara is the table above: 133,736 bytes for a 343-phoneme input, a third of the whole.

## What would fix it

1. **A warmed context allocates nothing per phoneme.** Keep the fricative and plosive formant filters, and the
   voiced path's target, in the context, and re-initialize them in place (a `svara_formant_filter_reset_into`,
   alongside the existing `_into` forms) instead of building new ones. Then `allocbudget.tcyr` could pin the
   context path too: after one render per class, zero bytes per render.
2. **A streaming sequence render**, so a consumer does not have to copy the crossfade. For example:
   `svara_sequence_render_into(seq, ctx, sample_rate, sink, user)`, which renders each phoneme on the caller's
   context and calls `sink(user, samples, n)` with output that no later fade-in can still change. A fade-in reaches
   back at most `min(cf_prev, len)` samples, so everything older than the longest one is final. hoosh carries that
   copy today (`src/lib/svara_local.cyr`: the plan and `spk_xf_*`), checked against `svara_seq_crossfade` bit for bit,
   and would drop it.
3. **Optional:** `_a` forms of `svara_seq_detect_clusters`, `svara_seq_durations` and `svara_detect_nasalization`,
   or the plan in (2) computed into caller storage. Each returns a fresh vector, and `svara_detect_nasalization`
   builds a `Nasalization` per nasalized vowel, when there are only three places. hoosh builds that table once
   (`svara_nasalization_for_nasal` for every phoneme) and lays out the plan in its own arena.

## References

- `docs/development/integration-guide.md`, *Real-time streaming*.
- hoosh `docs/decisions/016-in-process-speech-through-svara.md` and `src/lib/svara_local.cyr`.
- shabda `docs/development/issues/2026-11-11-conversion-allocates-on-the-default-heap.md` (the other half of what a
  speech request keeps).
- [`archived/2026-11-11-module-scratch-makes-concurrent-renders-unsafe.md`](archived/2026-11-11-module-scratch-makes-concurrent-renders-unsafe.md) (filed with this; resolved in 3.6.5).

## Resolution so far

**Fix 1 shipped in 3.6.5.** A warmed context allocates nothing per phoneme. The
context keeps its scaled target, the diphthong's end and interpolated targets, a
burst filter with its five formants, and a frication filter with its formants.
The two filters are re-initialized in place with `svara_formant_filter_rebuild`,
which gives the same filter a new one would. The table, voice scaling, diphthong
end target and fricative formants each gained an `_into` form that shares one
implementation with the allocating one, so the values are the same bits.

Re-run against 3.6.5, this repro prints `bytes kept: 0` for every phoneme in its
table, where 3.6.4 kept 160 to 1,048. `svara_synthctx_new` grows from 1,880 to 2,280
bytes for the four kept targets. `tests/allocbudget.tcyr` now pins it: after one
pass over every class (nasalized vowel and diphthong, both ejective releases and
silence included), the second and third passes keep 0 bytes. Against 3.6.4's
source they keep 12,616 each.

**Fixes 2 and 3 are open.** `svara_sequence_render` still keeps about 127 bytes per
output sample: it builds a source and tract per phoneme, and the crossfade copies.
`svara_sequence_render_into` and the `_a` forms add public API, which this
project releases in a minor, so they are on the roadmap under 3.8.x.
