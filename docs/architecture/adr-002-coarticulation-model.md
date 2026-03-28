# ADR-002: Coarticulation Model

## Status

Accepted (v1.0.0)

## Context

When phonemes are sequenced, their boundaries must be handled to avoid clicks
and to model the natural blending of articulatory gestures (coarticulation).

Options considered:

1. **Simple crossfade**: Fixed-length overlap with linear or sigmoid fade
2. **Formant interpolation**: Smoothly morph formant frequencies between targets
3. **DAC resistance model** (Recasens 1999): Per-phoneme coarticulation resistance
   determines how much a phoneme is influenced by its neighbors
4. **Articulatory planning** (gestural scores): Full articulatory model with
   overlapping gestures

## Decision

We use a **variable-length sigmoid crossfade** with per-phoneme coarticulation
resistance based on the **DAC (Degree of Articulatory Constraint)** model
from Recasens (1999).

Key parameters:
- **Coarticulation resistance** (0.0-0.9): High = phoneme resists blending
- **Crossfade fraction** (0.15-0.45): Derived from average resistance of adjacent pair
- **Look-ahead onset**: Transition begins at 60% of the preceding segment
- **Fade curve**: Smootherstep (Ken Perlin) via `hisab::calc::ease_in_out_smooth()`

## Rationale

- **Phoneme-specific blending**: /i/ and /u/ resist coarticulation strongly (tongue
  position is constrained), while schwa blends freely. DAC captures this.
- **No articulatory model needed**: The resistance coefficients encode articulatory
  behavior without simulating the articulators themselves.
- **Smooth transitions**: Smootherstep has zero 1st and 2nd derivatives at endpoints,
  preventing clicks and audible discontinuities.
- **Computationally cheap**: Just a crossfade with variable length, no per-sample
  formant interpolation during transitions.

## Consequences

- Transitions sound natural for most phoneme pairs
- Not physically accurate for complex clusters (e.g., /str/)
- Diphthongs use a separate per-sample formant interpolation path
- F2 locus equations (Sussman et al. 1991) augment stop-vowel transitions

## References

- Recasens, D. (1999). "Lingual coarticulation." In *Coarticulation*, Cambridge UP
- Sussman, H.M. et al. (1991). "An investigation of locus equations." JASA 90(3)
