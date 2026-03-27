//! Phoneme definitions and synthesis.
//!
//! Provides an IPA-subset phoneme inventory covering English and major languages,
//! with formant targets, default durations, and synthesis functions for each
//! phoneme class (vowels, fricatives, plosives, nasals, approximants).

use serde::{Deserialize, Serialize};
use tracing::trace;

use crate::error::{Result, SvaraError};
use crate::formant::{Formant, FormantFilter, Vowel, VowelTarget};
use crate::tract::VocalTract;
use crate::voice::VoiceProfile;

/// A phoneme from the IPA subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Phoneme {
    // Vowels
    /// /a/ open front unrounded
    VowelA,
    /// /e/ close-mid front unrounded
    VowelE,
    /// /i/ close front unrounded
    VowelI,
    /// /o/ close-mid back rounded
    VowelO,
    /// /u/ close back rounded
    VowelU,
    /// /ə/ schwa (mid central)
    VowelSchwa,
    /// /ɔ/ open-mid back rounded
    VowelOpenO,
    /// /æ/ near-open front
    VowelAsh,
    /// /ɪ/ near-close near-front
    VowelNearI,
    /// /ʊ/ near-close near-back
    VowelNearU,
    /// /ɑ/ open back unrounded
    VowelOpenA,
    /// /ɛ/ open-mid front unrounded
    VowelOpenE,
    /// /ʌ/ open-mid back unrounded
    VowelCupV,
    /// /ɜ/ open-mid central unrounded
    VowelBird,
    /// /iː/ long close front
    VowelLongI,

    // Diphthongs
    /// /aɪ/ as in "my"
    DiphthongAI,
    /// /aʊ/ as in "now"
    DiphthongAU,
    /// /ɔɪ/ as in "boy"
    DiphthongOI,
    /// /eɪ/ as in "day"
    DiphthongEI,
    /// /oʊ/ as in "go"
    DiphthongOU,

    // Plosives
    /// /p/ voiceless bilabial
    PlosiveP,
    /// /b/ voiced bilabial
    PlosiveB,
    /// /t/ voiceless alveolar
    PlosiveT,
    /// /d/ voiced alveolar
    PlosiveD,
    /// /k/ voiceless velar
    PlosiveK,
    /// /ɡ/ voiced velar
    PlosiveG,

    // Fricatives
    /// /f/ voiceless labiodental
    FricativeF,
    /// /v/ voiced labiodental
    FricativeV,
    /// /s/ voiceless alveolar
    FricativeS,
    /// /z/ voiced alveolar
    FricativeZ,
    /// /ʃ/ voiceless postalveolar
    FricativeSh,
    /// /ʒ/ voiced postalveolar
    FricativeZh,
    /// /θ/ voiceless dental
    FricativeTh,
    /// /ð/ voiced dental
    FricativeDh,
    /// /h/ voiceless glottal
    FricativeH,

    // Nasals
    /// /m/ bilabial nasal
    NasalM,
    /// /n/ alveolar nasal
    NasalN,
    /// /ŋ/ velar nasal
    NasalNg,

    // Affricates
    /// /tʃ/ voiceless postalveolar affricate (as in "church")
    AffricateCh,
    /// /dʒ/ voiced postalveolar affricate (as in "judge")
    AffricateJ,

    // Glottal
    /// /ʔ/ glottal stop
    GlottalStop,

    // Tap/Flap
    /// /ɾ/ alveolar tap/flap (as in American English "butter")
    TapFlap,

    // Approximants
    /// /l/ alveolar lateral
    LateralL,
    /// /ɹ/ alveolar approximant
    ApproximantR,
    /// /w/ labial-velar approximant
    ApproximantW,
    /// /j/ palatal approximant
    ApproximantJ,

    // Silence
    /// Silent pause
    Silence,
}

/// Phoneme articulatory classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum PhonemeClass {
    /// Stop consonants (oral closure then burst release).
    Plosive,
    /// Turbulent airflow through constriction.
    Fricative,
    /// Airflow through nasal cavity with oral closure.
    Nasal,
    /// Vowel-like consonant with open vocal tract.
    Approximant,
    /// Stop + fricative combination (e.g., /tʃ/, /dʒ/).
    Affricate,
    /// Airflow around sides of tongue.
    Lateral,
    /// Pure vowel.
    Vowel,
    /// Gliding vowel sequence.
    Diphthong,
    /// No sound.
    Silence,
}

impl Phoneme {
    /// Returns the articulatory class of this phoneme.
    #[must_use]
    pub fn class(&self) -> PhonemeClass {
        match self {
            Self::VowelA
            | Self::VowelE
            | Self::VowelI
            | Self::VowelO
            | Self::VowelU
            | Self::VowelSchwa
            | Self::VowelOpenO
            | Self::VowelAsh
            | Self::VowelNearI
            | Self::VowelNearU
            | Self::VowelOpenA
            | Self::VowelOpenE
            | Self::VowelCupV
            | Self::VowelBird
            | Self::VowelLongI => PhonemeClass::Vowel,

            Self::DiphthongAI
            | Self::DiphthongAU
            | Self::DiphthongOI
            | Self::DiphthongEI
            | Self::DiphthongOU => PhonemeClass::Diphthong,

            Self::PlosiveP
            | Self::PlosiveB
            | Self::PlosiveT
            | Self::PlosiveD
            | Self::PlosiveK
            | Self::PlosiveG => PhonemeClass::Plosive,

            Self::FricativeF
            | Self::FricativeV
            | Self::FricativeS
            | Self::FricativeZ
            | Self::FricativeSh
            | Self::FricativeZh
            | Self::FricativeTh
            | Self::FricativeDh
            | Self::FricativeH => PhonemeClass::Fricative,

            Self::NasalM | Self::NasalN | Self::NasalNg => PhonemeClass::Nasal,

            Self::AffricateCh | Self::AffricateJ => PhonemeClass::Affricate,

            Self::GlottalStop => PhonemeClass::Plosive,

            Self::TapFlap => PhonemeClass::Plosive,

            Self::LateralL => PhonemeClass::Lateral,

            Self::ApproximantR | Self::ApproximantW | Self::ApproximantJ => {
                PhonemeClass::Approximant
            }

            Self::Silence => PhonemeClass::Silence,
        }
    }

    /// Returns whether this phoneme is voiced.
    #[must_use]
    pub fn is_voiced(&self) -> bool {
        match self {
            // Voiceless consonants
            Self::PlosiveP
            | Self::PlosiveT
            | Self::PlosiveK
            | Self::FricativeF
            | Self::FricativeS
            | Self::FricativeSh
            | Self::FricativeTh
            | Self::FricativeH
            | Self::AffricateCh
            | Self::GlottalStop
            | Self::Silence => false,
            // Everything else is voiced
            _ => true,
        }
    }

    /// Returns the coarticulation resistance (0.0-1.0) for this phoneme.
    ///
    /// Higher values mean the phoneme strongly maintains its articulatory target
    /// and resists influence from neighbors. Lower values mean it's more susceptible
    /// to coarticulatory blending (e.g., schwa adapts to context).
    ///
    /// Based on Recasens (1999) DAC (Degree of Articulatory Constraint) model.
    #[must_use]
    pub fn coarticulation_resistance(&self) -> f32 {
        match self {
            // High resistance: strong articulatory targets
            Self::VowelI | Self::VowelLongI | Self::VowelNearI => 0.9,
            Self::VowelU | Self::VowelNearU => 0.85,
            Self::FricativeS | Self::FricativeZ | Self::FricativeSh | Self::FricativeZh => 0.85,
            Self::AffricateCh | Self::AffricateJ => 0.85,

            // Medium-high: clear targets but some flexibility
            Self::VowelA | Self::VowelOpenA | Self::VowelAsh => 0.7,
            Self::VowelE | Self::VowelO | Self::VowelOpenO | Self::VowelOpenE => 0.7,
            Self::PlosiveT | Self::PlosiveD | Self::NasalN | Self::LateralL => 0.75,
            Self::PlosiveK | Self::PlosiveG | Self::NasalNg => 0.7,
            Self::PlosiveP | Self::PlosiveB | Self::NasalM => 0.65,

            // Medium: moderate resistance
            Self::FricativeF | Self::FricativeV => 0.6,
            Self::FricativeTh | Self::FricativeDh => 0.6,
            Self::VowelCupV | Self::VowelBird => 0.5,
            Self::ApproximantR | Self::ApproximantJ => 0.55,
            Self::ApproximantW => 0.5,
            Self::TapFlap => 0.4,

            // Low: highly susceptible to coarticulation
            Self::VowelSchwa => 0.2,
            Self::FricativeH => 0.15, // /h/ takes on color of adjacent vowel
            Self::GlottalStop => 0.1,
            Self::Silence => 0.0,

            // Diphthongs: medium (already have internal transitions)
            Self::DiphthongAI
            | Self::DiphthongAU
            | Self::DiphthongOI
            | Self::DiphthongEI
            | Self::DiphthongOU => 0.6,
        }
    }
}

/// Returns the F2 locus frequency for stop consonants by place of articulation.
///
/// Locus equations: the F2 at consonant release is approximately
/// `F2_locus + slope * (F2_vowel - F2_locus)`. The locus and slope vary
/// by place of articulation.
///
/// Returns `(locus_hz, slope)` or `None` if the phoneme is not a stop/nasal.
///
/// Based on Sussman et al. (1991) locus equation data.
#[must_use]
pub fn f2_locus_equation(phoneme: &Phoneme) -> Option<(f32, f32)> {
    match phoneme {
        // Bilabial: F2 locus ~800-1000 Hz, slope ~0.85 (highly variable)
        Phoneme::PlosiveP | Phoneme::PlosiveB | Phoneme::NasalM => Some((900.0, 0.85)),
        // Alveolar: F2 locus ~1700-1800 Hz, slope ~0.55
        Phoneme::PlosiveT | Phoneme::PlosiveD | Phoneme::NasalN | Phoneme::TapFlap => {
            Some((1750.0, 0.55))
        }
        // Velar: F2 locus ~1800-2300 Hz, slope ~0.70 ("velar pinch")
        Phoneme::PlosiveK | Phoneme::PlosiveG | Phoneme::NasalNg => Some((2000.0, 0.70)),
        _ => None,
    }
}

/// Returns the formant targets for a given phoneme.
///
/// For consonants, these represent the formant transitions near the consonant
/// (locus frequencies). For vowels, these are the steady-state targets.
#[must_use]
pub fn phoneme_formants(phoneme: &Phoneme) -> VowelTarget {
    match phoneme {
        // Vowels map directly
        Phoneme::VowelA => VowelTarget::from_vowel(Vowel::A),
        // /ɑ/ — open back unrounded, lower F2 than /a/
        Phoneme::VowelOpenA => VowelTarget::with_bandwidths(
            [745.0, 1100.0, 2440.0, 3300.0, 3750.0],
            [85.0, 90.0, 110.0, 130.0, 150.0],
        ),
        Phoneme::VowelE => VowelTarget::from_vowel(Vowel::E),
        Phoneme::VowelI | Phoneme::VowelLongI => VowelTarget::from_vowel(Vowel::I),
        Phoneme::VowelO => VowelTarget::from_vowel(Vowel::O),
        Phoneme::VowelU => VowelTarget::from_vowel(Vowel::U),
        Phoneme::VowelSchwa => VowelTarget::from_vowel(Vowel::Schwa),
        // /ɜ/ — open-mid central, higher F1 and more centralized F2 than schwa
        Phoneme::VowelBird => VowelTarget::with_bandwidths(
            [580.0, 1400.0, 2500.0, 3300.0, 3750.0],
            [70.0, 80.0, 100.0, 120.0, 140.0],
        ),
        Phoneme::VowelOpenO => VowelTarget::from_vowel(Vowel::OpenO),
        Phoneme::VowelAsh => VowelTarget::from_vowel(Vowel::Ash),
        Phoneme::VowelNearI => VowelTarget::from_vowel(Vowel::NearI),
        Phoneme::VowelNearU => VowelTarget::from_vowel(Vowel::NearU),
        Phoneme::VowelOpenE | Phoneme::VowelCupV => {
            VowelTarget::new(600.0, 1770.0, 2500.0, 3300.0, 3750.0)
        }

        // Diphthongs: start target (transitions are handled in synthesis)
        Phoneme::DiphthongAI | Phoneme::DiphthongAU => VowelTarget::from_vowel(Vowel::A),
        Phoneme::DiphthongOI => VowelTarget::from_vowel(Vowel::OpenO),
        Phoneme::DiphthongEI => VowelTarget::from_vowel(Vowel::E),
        Phoneme::DiphthongOU => VowelTarget::from_vowel(Vowel::O),

        // Consonant locus frequencies (F2 locus determines place of articulation)
        // Bilabial locus: F2 ≈ 800-1000 Hz
        Phoneme::PlosiveP | Phoneme::PlosiveB | Phoneme::NasalM => {
            VowelTarget::new(350.0, 900.0, 2400.0, 3300.0, 3750.0)
        }
        // Alveolar locus: F2 ≈ 1700-1800 Hz
        Phoneme::PlosiveT | Phoneme::PlosiveD | Phoneme::NasalN | Phoneme::LateralL => {
            VowelTarget::new(400.0, 1750.0, 2600.0, 3300.0, 3750.0)
        }
        // Velar locus: F2 ≈ 1300-2300 Hz (variable, "velar pinch")
        Phoneme::PlosiveK | Phoneme::PlosiveG | Phoneme::NasalNg => {
            VowelTarget::new(350.0, 1800.0, 2500.0, 3300.0, 3750.0)
        }
        // Labiodental
        Phoneme::FricativeF | Phoneme::FricativeV => {
            VowelTarget::new(350.0, 1050.0, 2400.0, 3300.0, 3750.0)
        }
        // Alveolar fricatives
        Phoneme::FricativeS | Phoneme::FricativeZ => {
            VowelTarget::new(400.0, 1750.0, 2600.0, 3300.0, 3750.0)
        }
        // Postalveolar fricatives
        Phoneme::FricativeSh | Phoneme::FricativeZh => {
            VowelTarget::new(350.0, 1600.0, 2500.0, 3300.0, 3750.0)
        }
        // Dental fricatives
        Phoneme::FricativeTh | Phoneme::FricativeDh => {
            VowelTarget::new(350.0, 1400.0, 2500.0, 3300.0, 3750.0)
        }
        // Glottal fricative
        Phoneme::FricativeH => VowelTarget::from_vowel(Vowel::Schwa),

        // Affricates (postalveolar locus)
        Phoneme::AffricateCh | Phoneme::AffricateJ => {
            VowelTarget::new(350.0, 1600.0, 2500.0, 3300.0, 3750.0)
        }

        // Glottal stop: neutral tract position
        Phoneme::GlottalStop => VowelTarget::from_vowel(Vowel::Schwa),

        // Tap/flap: alveolar locus (similar to /d/ but very brief)
        Phoneme::TapFlap => VowelTarget::new(400.0, 1750.0, 2600.0, 3300.0, 3750.0),

        // Approximants
        Phoneme::ApproximantR => VowelTarget::new(350.0, 1300.0, 1600.0, 3300.0, 3750.0),
        Phoneme::ApproximantW => VowelTarget::new(300.0, 700.0, 2200.0, 3300.0, 3750.0),
        Phoneme::ApproximantJ => VowelTarget::new(280.0, 2200.0, 2900.0, 3300.0, 3750.0),

        // Silence
        Phoneme::Silence => VowelTarget::new(500.0, 1500.0, 2500.0, 3300.0, 3750.0),
    }
}

/// Returns the default duration in seconds for a phoneme.
#[must_use]
pub fn phoneme_duration(phoneme: &Phoneme) -> f32 {
    match phoneme.class() {
        PhonemeClass::Vowel => 0.12,
        PhonemeClass::Diphthong => 0.18,
        PhonemeClass::Plosive => 0.08,
        PhonemeClass::Fricative => 0.10,
        PhonemeClass::Nasal => 0.08,
        PhonemeClass::Affricate => 0.12,
        PhonemeClass::Approximant | PhonemeClass::Lateral => 0.07,
        PhonemeClass::Silence => 0.05,
    }
}

/// Simple PRNG for synthesis noise (consistent with glottal module's approach).
struct NoiseGen {
    state: u64,
}

impl NoiseGen {
    fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_add(1),
        }
    }

    #[inline]
    fn next_f32(&mut self) -> f32 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let bits = (self.state >> 33) as i32;
        bits as f32 / (i32::MAX as f32)
    }
}

/// Diphthong end targets.
fn diphthong_end_target(phoneme: &Phoneme) -> VowelTarget {
    match phoneme {
        Phoneme::DiphthongAI => VowelTarget::from_vowel(Vowel::I),
        Phoneme::DiphthongAU => VowelTarget::from_vowel(Vowel::U),
        Phoneme::DiphthongOI => VowelTarget::from_vowel(Vowel::I),
        Phoneme::DiphthongEI => VowelTarget::from_vowel(Vowel::I),
        Phoneme::DiphthongOU => VowelTarget::from_vowel(Vowel::U),
        _ => phoneme_formants(phoneme),
    }
}

/// Synthesizes a single phoneme with the given voice profile.
///
/// # Errors
///
/// Returns `SvaraError::ArticulationFailed` if synthesis parameters are invalid.
pub fn synthesize_phoneme(
    phoneme: &Phoneme,
    voice: &VoiceProfile,
    sample_rate: f32,
    duration: f32,
) -> Result<Vec<f32>> {
    if duration <= 0.0 {
        return Err(SvaraError::InvalidDuration(format!(
            "duration must be positive, got {duration}"
        )));
    }
    if sample_rate <= 0.0 {
        return Err(SvaraError::ArticulationFailed(
            "sample_rate must be positive".to_string(),
        ));
    }

    let num_samples = (duration * sample_rate) as usize;
    if num_samples == 0 {
        return Ok(Vec::new());
    }

    trace!(?phoneme, duration, num_samples, "synthesizing phoneme");

    match phoneme {
        Phoneme::Silence => Ok(vec![0.0; num_samples]),
        _ => match phoneme.class() {
            PhonemeClass::Vowel => synthesize_vowel(phoneme, voice, sample_rate, num_samples),
            PhonemeClass::Diphthong => {
                synthesize_diphthong(phoneme, voice, sample_rate, num_samples)
            }
            PhonemeClass::Plosive => synthesize_plosive(phoneme, voice, sample_rate, num_samples),
            PhonemeClass::Fricative => {
                synthesize_fricative(phoneme, voice, sample_rate, num_samples)
            }
            PhonemeClass::Nasal => synthesize_nasal(phoneme, voice, sample_rate, num_samples),
            PhonemeClass::Affricate => {
                synthesize_affricate(phoneme, voice, sample_rate, num_samples)
            }
            PhonemeClass::Approximant | PhonemeClass::Lateral => {
                synthesize_approximant(phoneme, voice, sample_rate, num_samples)
            }
            PhonemeClass::Silence => Ok(vec![0.0; num_samples]),
        },
    }
}

fn synthesize_vowel(
    phoneme: &Phoneme,
    voice: &VoiceProfile,
    sample_rate: f32,
    num_samples: usize,
) -> Result<Vec<f32>> {
    let target = voice.apply_formant_scale(&phoneme_formants(phoneme));
    let mut tract = VocalTract::new(sample_rate);
    tract.set_formants_from_target(&target)?;

    let mut glottal = voice
        .create_glottal_source(sample_rate)
        .map_err(|e| SvaraError::ArticulationFailed(e.to_string()))?;

    let mut output = tract.synthesize(&mut glottal, num_samples);
    apply_amplitude_envelope(&mut output, num_samples);
    Ok(output)
}

fn synthesize_diphthong(
    phoneme: &Phoneme,
    voice: &VoiceProfile,
    sample_rate: f32,
    num_samples: usize,
) -> Result<Vec<f32>> {
    let start_target = voice.apply_formant_scale(&phoneme_formants(phoneme));
    let end_target = voice.apply_formant_scale(&diphthong_end_target(phoneme));

    let mut glottal = voice
        .create_glottal_source(sample_rate)
        .map_err(|e| SvaraError::ArticulationFailed(e.to_string()))?;

    let mut tract = VocalTract::new(sample_rate);
    let mut output = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        let t = i as f32 / num_samples as f32;
        let current = VowelTarget::interpolate(&start_target, &end_target, t);
        tract.set_formants_from_target(&current)?;
        let excitation = glottal.next_sample();
        output.push(tract.process_sample(excitation));
    }

    apply_amplitude_envelope(&mut output, num_samples);
    Ok(output)
}

fn synthesize_plosive(
    phoneme: &Phoneme,
    voice: &VoiceProfile,
    sample_rate: f32,
    num_samples: usize,
) -> Result<Vec<f32>> {
    let mut output = vec![0.0; num_samples];
    let mut noise = NoiseGen::new(17);

    // Plosive: silence (closure) + burst + aspiration
    let closure_end = num_samples / 3;
    let burst_end = closure_end + (num_samples / 6).max(1);

    // Burst: short noise burst
    let target = voice.apply_formant_scale(&phoneme_formants(phoneme));
    let formants = target.to_formants();
    let mut filter = FormantFilter::new(&formants, sample_rate)
        .map_err(|e| SvaraError::ArticulationFailed(e.to_string()))?;

    for sample in output.iter_mut().take(burst_end).skip(closure_end) {
        let n = noise.next_f32() * 0.8;
        *sample = filter.process_sample(n);
    }

    // Aspiration/voicing transition
    if phoneme.is_voiced() {
        let mut glottal = voice
            .create_glottal_source(sample_rate)
            .map_err(|e| SvaraError::ArticulationFailed(e.to_string()))?;
        glottal.set_breathiness(0.4); // Override: plosive voicing onset is breathier
        let mut tract = VocalTract::new(sample_rate);
        tract.set_formants_from_target(&target)?;

        for sample in output.iter_mut().skip(burst_end) {
            let excitation = glottal.next_sample();
            *sample = tract.process_sample(excitation) * 0.5;
        }
    } else {
        // Voiceless aspiration
        for sample in output.iter_mut().skip(burst_end) {
            let n = noise.next_f32() * 0.3;
            *sample = filter.process_sample(n);
        }
    }

    apply_amplitude_envelope(&mut output, num_samples);
    Ok(output)
}

fn synthesize_fricative(
    phoneme: &Phoneme,
    voice: &VoiceProfile,
    sample_rate: f32,
    num_samples: usize,
) -> Result<Vec<f32>> {
    let target = voice.apply_formant_scale(&phoneme_formants(phoneme));
    let mut noise = NoiseGen::new(31);

    // Fricatives: filtered noise, optionally mixed with voicing
    let formants = fricative_formants(phoneme, sample_rate);
    let mut filter = FormantFilter::new(&formants, sample_rate)
        .map_err(|e| SvaraError::ArticulationFailed(e.to_string()))?;

    let mut output = Vec::with_capacity(num_samples);

    if phoneme.is_voiced() {
        let mut glottal = voice
            .create_glottal_source(sample_rate)
            .map_err(|e| SvaraError::ArticulationFailed(e.to_string()))?;
        let mut tract = VocalTract::new(sample_rate);
        tract.set_formants_from_target(&target)?;

        for _ in 0..num_samples {
            let n = noise.next_f32() * 0.5;
            let friction = filter.process_sample(n);
            let voicing = tract.process_sample(glottal.next_sample()) * 0.4;
            output.push(friction + voicing);
        }
    } else {
        for _ in 0..num_samples {
            let n = noise.next_f32() * 0.6;
            output.push(filter.process_sample(n));
        }
    }

    apply_amplitude_envelope(&mut output, num_samples);
    Ok(output)
}

/// Returns appropriate noise-shaping formants for fricative consonants.
fn fricative_formants(phoneme: &Phoneme, _sample_rate: f32) -> Vec<Formant> {
    match phoneme {
        // /s/ /z/: high-frequency energy around 4-8 kHz
        Phoneme::FricativeS | Phoneme::FricativeZ => vec![
            Formant::new(4500.0, 500.0, 1.0),
            Formant::new(7000.0, 800.0, 0.7),
        ],
        // /ʃ/ /ʒ/: energy around 2.5-6 kHz
        Phoneme::FricativeSh | Phoneme::FricativeZh => vec![
            Formant::new(2800.0, 600.0, 1.0),
            Formant::new(5000.0, 800.0, 0.6),
        ],
        // /f/ /v/: weak, flat spectrum
        Phoneme::FricativeF | Phoneme::FricativeV => vec![
            Formant::new(3000.0, 2000.0, 0.5),
            Formant::new(8000.0, 2000.0, 0.3),
        ],
        // /θ/ /ð/: weak, mid-high frequency
        Phoneme::FricativeTh | Phoneme::FricativeDh => vec![
            Formant::new(4000.0, 1500.0, 0.4),
            Formant::new(7500.0, 1500.0, 0.3),
        ],
        // /h/: shaped by following vowel
        Phoneme::FricativeH => vec![
            Formant::new(1500.0, 800.0, 0.4),
            Formant::new(2500.0, 800.0, 0.3),
        ],
        // /tʃ/ /dʒ/: postalveolar frication, similar to /ʃ/ /ʒ/
        Phoneme::AffricateCh | Phoneme::AffricateJ => vec![
            Formant::new(2800.0, 600.0, 1.0),
            Formant::new(5000.0, 800.0, 0.6),
        ],
        _ => vec![Formant::new(3000.0, 1000.0, 0.5)],
    }
}

fn synthesize_nasal(
    phoneme: &Phoneme,
    voice: &VoiceProfile,
    sample_rate: f32,
    num_samples: usize,
) -> Result<Vec<f32>> {
    let target = voice.apply_formant_scale(&phoneme_formants(phoneme));
    let mut glottal = voice
        .create_glottal_source(sample_rate)
        .map_err(|e| SvaraError::ArticulationFailed(e.to_string()))?;

    let mut tract = VocalTract::new(sample_rate);
    tract.set_formants_from_target(&target)?;
    tract.set_nasal_coupling(0.8);

    let mut output = tract.synthesize(&mut glottal, num_samples);
    apply_amplitude_envelope(&mut output, num_samples);
    Ok(output)
}

fn synthesize_approximant(
    phoneme: &Phoneme,
    voice: &VoiceProfile,
    sample_rate: f32,
    num_samples: usize,
) -> Result<Vec<f32>> {
    let target = voice.apply_formant_scale(&phoneme_formants(phoneme));
    let mut glottal = voice
        .create_glottal_source(sample_rate)
        .map_err(|e| SvaraError::ArticulationFailed(e.to_string()))?;
    glottal.set_breathiness(voice.breathiness.max(0.1)); // Approximants need slight breathiness

    let mut tract = VocalTract::new(sample_rate);
    tract.set_formants_from_target(&target)?;

    let mut output = tract.synthesize(&mut glottal, num_samples);

    // Approximants have a slight amplitude reduction
    for sample in &mut output {
        *sample *= 0.7;
    }
    apply_amplitude_envelope(&mut output, num_samples);
    Ok(output)
}

fn synthesize_affricate(
    phoneme: &Phoneme,
    voice: &VoiceProfile,
    sample_rate: f32,
    num_samples: usize,
) -> Result<Vec<f32>> {
    // Affricate = plosive closure/burst (first third) + fricative release (remaining)
    let mut output = vec![0.0; num_samples];
    let mut noise = NoiseGen::new(23);

    let closure_end = num_samples / 4;
    let burst_end = closure_end + (num_samples / 8).max(1);

    // Burst: short noise burst at postalveolar locus
    let target = voice.apply_formant_scale(&phoneme_formants(phoneme));
    let fric_formants = fricative_formants(phoneme, sample_rate);
    let mut filter = FormantFilter::new(&fric_formants, sample_rate)
        .map_err(|e| SvaraError::ArticulationFailed(e.to_string()))?;

    for sample in output.iter_mut().take(burst_end).skip(closure_end) {
        let n = noise.next_f32() * 0.8;
        *sample = filter.process_sample(n);
    }

    // Fricative release phase
    if phoneme.is_voiced() {
        let mut glottal = voice
            .create_glottal_source(sample_rate)
            .map_err(|e| SvaraError::ArticulationFailed(e.to_string()))?;
        let mut tract = VocalTract::new(sample_rate);
        tract.set_formants_from_target(&target)?;

        for sample in output.iter_mut().skip(burst_end) {
            let exc = glottal.next_sample();
            let voiced = tract.process_sample(exc);
            let fric = filter.process_sample(noise.next_f32() * 0.5);
            *sample = voiced * 0.5 + fric * 0.5;
        }
    } else {
        for sample in output.iter_mut().skip(burst_end) {
            let n = noise.next_f32() * 0.6;
            *sample = filter.process_sample(n);
        }
    }

    apply_amplitude_envelope(&mut output, num_samples);
    Ok(output)
}

/// Applies a gentle attack/release envelope to avoid clicks.
fn apply_amplitude_envelope(samples: &mut [f32], _total: usize) {
    let len = samples.len();
    if len == 0 {
        return;
    }
    // 5ms ramp at 44100 Hz ≈ 220 samples, but scale proportionally
    let ramp_len = (len / 10).clamp(1, 256);

    for (i, sample) in samples.iter_mut().enumerate().take(ramp_len) {
        let t = i as f32 / ramp_len as f32;
        // Raised cosine ramp
        let gain = 0.5 * (1.0 - (std::f32::consts::PI * t).cos());
        *sample *= gain;
    }

    for i in 0..ramp_len {
        let idx = len - 1 - i;
        let t = i as f32 / ramp_len as f32;
        let gain = 0.5 * (1.0 - (std::f32::consts::PI * t).cos());
        samples[idx] *= gain;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phoneme_class() {
        assert_eq!(Phoneme::VowelA.class(), PhonemeClass::Vowel);
        assert_eq!(Phoneme::PlosiveP.class(), PhonemeClass::Plosive);
        assert_eq!(Phoneme::FricativeS.class(), PhonemeClass::Fricative);
        assert_eq!(Phoneme::NasalM.class(), PhonemeClass::Nasal);
        assert_eq!(Phoneme::DiphthongAI.class(), PhonemeClass::Diphthong);
        assert_eq!(Phoneme::ApproximantR.class(), PhonemeClass::Approximant);
        assert_eq!(Phoneme::LateralL.class(), PhonemeClass::Lateral);
    }

    #[test]
    fn test_voicing() {
        assert!(!Phoneme::PlosiveP.is_voiced());
        assert!(Phoneme::PlosiveB.is_voiced());
        assert!(Phoneme::VowelA.is_voiced());
        assert!(!Phoneme::FricativeS.is_voiced());
        assert!(Phoneme::FricativeZ.is_voiced());
    }

    #[test]
    fn test_synthesize_vowel() {
        let voice = VoiceProfile::new_male();
        let result = synthesize_phoneme(&Phoneme::VowelA, &voice, 44100.0, 0.1);
        assert!(result.is_ok());
        let samples = result.unwrap();
        assert!(!samples.is_empty());
        assert!(samples.iter().all(|s| s.is_finite()));
        assert!(samples.iter().any(|&s| s.abs() > 1e-6));
    }

    #[test]
    fn test_synthesize_fricative() {
        let voice = VoiceProfile::new_male();
        let result = synthesize_phoneme(&Phoneme::FricativeS, &voice, 44100.0, 0.08);
        assert!(result.is_ok());
    }

    #[test]
    fn test_synthesize_plosive() {
        let voice = VoiceProfile::new_male();
        let result = synthesize_phoneme(&Phoneme::PlosiveP, &voice, 44100.0, 0.08);
        assert!(result.is_ok());
    }

    #[test]
    fn test_synthesize_silence() {
        let voice = VoiceProfile::new_male();
        let result = synthesize_phoneme(&Phoneme::Silence, &voice, 44100.0, 0.05);
        assert!(result.is_ok());
        let samples = result.unwrap();
        assert!(samples.iter().all(|&s| s.abs() < f32::EPSILON));
    }

    #[test]
    fn test_invalid_duration() {
        let voice = VoiceProfile::new_male();
        assert!(synthesize_phoneme(&Phoneme::VowelA, &voice, 44100.0, -1.0).is_err());
    }

    #[test]
    fn test_serde_roundtrip() {
        let p = Phoneme::VowelA;
        let json = serde_json::to_string(&p).unwrap();
        let p2: Phoneme = serde_json::from_str(&json).unwrap();
        assert_eq!(p, p2);
    }
}
