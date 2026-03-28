# svara Integration Guide

## Quick Start

```rust
use svara::prelude::*;

// 1. Create a voice profile
let voice = VoiceProfile::new_male();

// 2. Synthesize a single phoneme
let samples = synthesize_phoneme(&Phoneme::VowelA, &voice, 44100.0, 0.5)?;

// 3. Or build a phoneme sequence with coarticulation
let mut seq = PhonemeSequence::new();
seq.push(PhonemeEvent::new(Phoneme::VowelA, 0.15, Stress::Primary));
seq.push(PhonemeEvent::new(Phoneme::NasalN, 0.08, Stress::Unstressed));
seq.push(PhonemeEvent::new(Phoneme::VowelI, 0.15, Stress::Secondary));
let audio = seq.render(&voice, 44100.0)?;
```

## Real-Time Streaming

For real-time use, avoid per-frame allocations:

```rust
let mut glottal = voice.create_glottal_source(44100.0)?;
let mut tract = VocalTract::new(44100.0);
tract.set_vowel(Vowel::A)?;

// Pre-allocate once
let mut buffer = vec![0.0f32; 512];

// Each audio callback: zero allocations
loop {
    tract.synthesize_into(&mut glottal, &mut buffer);
    // Send buffer to audio output...
}
```

## Using Bridge Functions

Bridge functions convert upstream crate outputs into svara parameters without
requiring direct dependencies between crates.

```rust
use svara::bridge;

// From bhava emotion system
let rd = bridge::rd_from_arousal(0.8); // Excited → pressed voice
let breathiness = bridge::breathiness_from_arousal(0.8);

// From goonj acoustics
let gain = bridge::gain_from_distance(1.0, 5.0); // 5m away

// From badal weather
let effort = bridge::lombard_effort_from_noise(70.0); // 70 dB SPL ambient
```

## Quality Scaling (LOD)

For multi-voice scenarios (crowd, chorus), reduce quality for background voices:

```rust
use svara::lod::Quality;

tract.set_quality(Quality::Full);     // Foreground: all pipeline stages
tract.set_quality(Quality::Reduced);  // Mid-distance: 3 formants, no subglottal
tract.set_quality(Quality::Minimal);  // Background: 2 formants, no extras
```

## Feature Flags

| Feature | Default | Effect |
|---------|---------|--------|
| `std` | Yes | Standard library (disable for embedded/WASM) |
| `naad-backend` | Yes | High-quality DSP from naad crate |
| `logging` | No | Structured tracing output |

```toml
# Minimal (no_std)
svara = { version = "1.1", default-features = false }

# Full
svara = { version = "1.1", features = ["full"] }
```

## Consumers

| Crate | How it uses svara |
|-------|------------------|
| **dhvani** | Receives audio samples, mixes/processes/plays |
| **vansh** | Converts text to phoneme sequences, calls render() |
| **prani** | Creates creature voice profiles via bridge functions |
