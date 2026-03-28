# ADR-003: Formant Data Source

## Status

Accepted (v1.0.0)

## Context

Formant synthesis requires accurate frequency and bandwidth targets for each
vowel. The choice of reference data affects the perceived naturalness of
synthesized speech.

Major datasets:
1. **Peterson & Barney (1952)**: Classic, 10 vowels, male/female averages.
   Limited bandwidth data.
2. **Hillenbrand et al. (1995)**: Updated Peterson & Barney with modern
   recording equipment. 12 vowels, individual F1-F4 + B1-B3 measurements.
   300+ speakers across gender and age groups.
3. **Hawkins & Midgley (2005)**: British English formants
4. **Adank et al. (2004)**: Dutch English formants

## Decision

We use **Hillenbrand et al. (1995)** male averages as the reference, with
per-vowel bandwidths B1-B5.

The scaffold (v0.1.0) shipped with Peterson & Barney (1952) data. During P(-1)
hardening, we upgraded to Hillenbrand data which provides:
- More vowel distinctions (OpenA vs A, Bird vs Schwa)
- Per-vowel bandwidth measurements (not available in Peterson & Barney)
- Modern recording conditions (less room coloration)

## Rationale

- **Per-vowel bandwidths are critical**: Without them, all vowels use the same
  bandwidths, causing unnatural uniformity. Hillenbrand provides measured B1-B3.
- **Male as reference**: Female and child voices are derived by scaling formant
  frequencies by `formant_scale` (1.17 for female, 1.3 for child) and bandwidths
  by `sqrt(f0/120)`. This is more flexible than storing separate tables.
- **Modern data**: Hillenbrand's 1995 measurements better reflect contemporary
  American English pronunciation.

## Consequences

- Vowel targets are specific to American English
- Other languages/dialects will need additional vowel targets (future work)
- The `VowelTarget` struct supports arbitrary formant values for custom voices
- Bandwidth scaling by f0 produces reasonable female/child voices without
  separate measured data

## References

- Peterson, G.E. & Barney, H.L. (1952). "Control methods used in a study of the vowels." JASA 24(2)
- Hillenbrand, J. et al. (1995). "Acoustic characteristics of American English vowels." JASA 97(5)
