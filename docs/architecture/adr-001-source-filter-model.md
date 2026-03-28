# ADR-001: Source-Filter Model Choice

## Status

Accepted (v1.0.0)

## Context

Vocal synthesis requires choosing how to model the human voice production system.
The two main approaches are:

1. **Source-filter model** (Fant 1960): Separate glottal excitation from vocal tract
   resonance. The source (glottal pulse) is independent of the filter (formant resonators).
2. **Waveform concatenation** (STRAIGHT, WORLD): Decompose recorded speech into
   spectral envelope + excitation, resynthesize by modifying parameters.
3. **Physical modeling** (digital waveguide, finite-element): Simulate the actual
   physics of airflow through the vocal tract.

## Decision

We use the **source-filter model** with two glottal pulse generators:

- **Rosenberg B** (`3t^2 - 2t^3`): Fast polynomial, adequate for most uses
- **LF (Liljencrants-Fant)**: Standard in speech science, parameterized by Rd
  for voice quality control from pressed to breathy

The vocal tract is modeled as a parallel formant filter bank (biquad resonators)
with nasal coupling, lip radiation, subglottal resonance, and source-filter
interaction feedback.

## Rationale

- **No sample dependency**: Pure mathematical synthesis, no audio assets needed.
  Matches AGNOS philosophy (garjan, ghurni, prani all synthesize from math).
- **Real-time controllable**: All parameters (f0, formants, breathiness, Rd) can
  be changed per-sample for smooth transitions.
- **Scientific foundation**: Formant frequencies are well-documented (Hillenbrand
  et al. 1995, Peterson & Barney 1952). Reproducible and verifiable.
- **Performance**: SOA biquad bank with fixed MAX_FORMANTS=8 enables SIMD
  auto-vectorization. ~1,000x real-time at 44.1kHz.
- **LF model**: Widely used in speech research, single Rd parameter captures the
  entire pressed-to-breathy voice quality dimension.

## Alternatives Rejected

- **WORLD/STRAIGHT**: Requires recorded speech database. Not suitable for
  procedural synthesis of arbitrary voices.
- **Physical modeling**: Too computationally expensive for real-time game use.
  Difficult to parameterize for arbitrary speaker characteristics.
- **Neural vocoders** (WaveNet, HiFi-GAN): Requires GPU, large models, training
  data. Opposite of svara's design goals.

## Consequences

- Quality ceiling is lower than neural approaches for naturalistic speech
- Excellent for stylized, procedural, and real-time voice generation
- All voice characteristics are explicit parameters (no black-box behavior)
- Easy to integrate with emotion/affect systems via bridge functions

## References

- Fant, G. (1960). *Acoustic Theory of Speech Production*
- Rosenberg, A.E. (1971). "Effect of Glottal Pulse Shape on the Quality of Natural Vowels." JASA 49(2B)
- Fant, G. et al. (1985). "A four-parameter model of glottal flow." STL-QPSR 4/1985
- Hillenbrand, J. et al. (1995). "Acoustic characteristics of American English vowels." JASA 97(5)
