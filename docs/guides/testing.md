# Testing Guide

## Running Tests

```bash
# All features (default)
cargo test --all-features

# no_std mode
cargo test --no-default-features

# Both (what CI runs)
make test

# Specific test
cargo test --all-features test_deterministic_replay
```

## Test Categories

### Unit Tests (in src/)

Each module has inline `#[cfg(test)]` tests for internal logic:
- `glottal.rs`: Source creation, period accuracy, model switching, serde
- `formant.rs`: Coefficient computation, filter processing
- `tract.rs`: Vocal tract synthesis, nasal coupling, reset behavior
- `bridge.rs`: All bridge function ranges and edge cases
- `lod.rs`: Quality feature flags, formant counts, serde

### Integration Tests (tests/integration.rs)

45+ tests covering:

| Category | Count | What's tested |
|----------|-------|---------------|
| Synthesis correctness | 8 | All vowels, consonants, diphthongs, child voice |
| Spectral quality | 2 | F1 energy, transition smoothness |
| Glottal period/jitter | 2 | Period accuracy, non-periodicity |
| Serde roundtrips | 11 | Every public type serializes/deserializes |
| Interpolation | 2 | VowelTarget endpoints, midpoint |
| Sequence rendering | 1 | Multi-phoneme sequence |
| Validation edge cases | 7 | NaN, Inf, negative, zero inputs |
| Streaming API | 2 | process_block, empty buffer |
| Deterministic replay | 2 | Single phoneme, sequence |
| LOD/Quality | 3 | All levels produce output, differ, serde |

### Adding New Tests

1. **New phoneme**: Add to `test_all_consonant_classes_synthesize` or create specific test
2. **New public type**: Add serde roundtrip test + Send/Sync assertion in lib.rs
3. **New bridge function**: Add range test + edge case test in bridge.rs
4. **Performance change**: Run `make bench` before and after

## Benchmarks

```bash
# Run with history tracking
make bench

# Run specific benchmark
cargo bench -- glottal_source

# View HTML report
open target/criterion/report/index.html
```

### Current Benchmarks (11)

| Benchmark | Measures |
|-----------|----------|
| `glottal_source_1024` | Glottal pulse generation |
| `formant_filter_1024` | Per-sample formant filtering |
| `formant_filter_block_1024` | Block-based formant filtering |
| `vocal_tract_1024` | Full tract pipeline |
| `vocal_tract_into_1024` | Pre-allocated tract pipeline |
| `phoneme_render_vowel_a` | Single vowel synthesis |
| `phoneme_render_fricative_s` | Fricative noise synthesis |
| `phoneme_render_diphthong_ai` | Diphthong interpolation |
| `phoneme_render_female_vowel_a` | Formant-scaled synthesis |
| `sequence_render_5_phonemes` | Short sequence with coarticulation |
| `sequence_render_10_phonemes` | Longer sequence |

## Full CI Check (Local)

```bash
make check  # fmt + clippy + test + audit
make deny   # Supply chain
make doc    # Rustdoc with -D warnings
make bench  # Benchmarks with history
```
