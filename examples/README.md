# svara examples

Runnable programs, ported from `rust-old/examples/` (roadmap 3.6.0's
preserve-first gate: they had to exist in Cyrius before the Rust oracle could be
deleted). Each is a standalone entry point — build it like any other:

```sh
cyrius build examples/basic.cyr build/example-basic && ./build/example-basic
```

| Example | Shows |
|---|---|
| [`basic.cyr`](basic.cyr) | synthesize a male /a/, then a three-phoneme sequence; sample statistics |
| [`voice_comparison.cyr`](voice_comparison.cyr) | the male / female / child presets on the same vowel, plus a custom breathy voice built with the chained builders |
| [`error_handling.cyr`](error_handling.cyr) | every way svara reports failure — negative `SVARA_ERR_*` codes, and the two shapes (`svara_is_err` vs a 0 pointer) |
| [`prosody_patterns.cyr`](prosody_patterns.cyr) | the four intonation patterns and nine lexical tones as f0 contours |
| [`streaming.cyr`](streaming.cyr) | **the zero-allocation real-time path** — pre-allocated blocks, `_into` forms, mid-stream parameter changes, quality scaling |

⚠ **`streaming.cyr` is the one to read first if you are writing an audio
callback.** Cyrius's bump allocator never frees, so an allocation inside a
callback is not a leak that grows slowly — it is one that ends the process.

CI builds and runs all five on every push, so they cannot rot against the API.
