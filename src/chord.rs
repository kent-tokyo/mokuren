//! Chords, Roman numerals, and harmonic function.
//!
//! v0.1 keeps Roman numeral and harmonic function as distinct concepts
//! (AGENTS.md section 6): `RomanNumeral` names *which* diatonic chord,
//! `HarmonicFunction` names its role (tonic/predominant/dominant) so
//! progression evaluation can reason at the function level.

use crate::error::{MokurenError, Result};
use crate::key::{Key, ScaleDegree};
use crate::pitch::{PitchClass, spell_above};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChordQuality {
    MajorTriad,
    MinorTriad,
    DiminishedTriad,
    AugmentedTriad,
    MajorSeventh,
    MinorSeventh,
    DominantSeventh,
    HalfDiminishedSeventh,
    DiminishedSeventh,
}

impl ChordQuality {
    pub fn is_seventh(&self) -> bool {
        matches!(
            self,
            ChordQuality::MajorSeventh
                | ChordQuality::MinorSeventh
                | ChordQuality::DominantSeventh
                | ChordQuality::HalfDiminishedSeventh
                | ChordQuality::DiminishedSeventh
        )
    }

    /// Semitones above the root for each chord tone, root first.
    pub fn interval_semitones(&self) -> &'static [i32] {
        match self {
            ChordQuality::MajorTriad => &[0, 4, 7],
            ChordQuality::MinorTriad => &[0, 3, 7],
            ChordQuality::DiminishedTriad => &[0, 3, 6],
            ChordQuality::AugmentedTriad => &[0, 4, 8],
            ChordQuality::MajorSeventh => &[0, 4, 7, 11],
            ChordQuality::MinorSeventh => &[0, 3, 7, 10],
            ChordQuality::DominantSeventh => &[0, 4, 7, 10],
            ChordQuality::HalfDiminishedSeventh => &[0, 3, 6, 10],
            ChordQuality::DiminishedSeventh => &[0, 3, 6, 9],
        }
    }

    /// Whether this quality is spelled with a lowercase Roman numeral.
    fn is_minor_case(&self) -> bool {
        matches!(
            self,
            ChordQuality::MinorTriad
                | ChordQuality::DiminishedTriad
                | ChordQuality::MinorSeventh
                | ChordQuality::HalfDiminishedSeventh
                | ChordQuality::DiminishedSeventh
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChordInversion {
    Root,
    First,
    Second,
    /// Only meaningful for seventh chords.
    Third,
}

impl ChordInversion {
    /// Index into `Chord::pitch_classes()` that must be in the bass.
    pub fn bass_chord_tone_index(&self) -> usize {
        match self {
            ChordInversion::Root => 0,
            ChordInversion::First => 1,
            ChordInversion::Second => 2,
            ChordInversion::Third => 3,
        }
    }

    fn figured_bass(&self, is_seventh: bool) -> &'static str {
        match (self, is_seventh) {
            (ChordInversion::Root, false) => "",
            (ChordInversion::First, false) => "6",
            (ChordInversion::Second, false) => "64",
            (ChordInversion::Root, true) => "7",
            (ChordInversion::First, true) => "65",
            (ChordInversion::Second, true) => "43",
            (ChordInversion::Third, true) => "42",
            (ChordInversion::Third, false) => "", // not meaningful; treated as root
        }
    }
}

/// Tonic / predominant / dominant. Kept separate from `RomanNumeral` so
/// progression evaluation can reason about function-level motion
/// (T -> PD -> D -> T) without switching on scale degree everywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HarmonicFunction {
    Tonic,
    Predominant,
    Dominant,
}

impl fmt::Display for HarmonicFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            HarmonicFunction::Tonic => "tonic",
            HarmonicFunction::Predominant => "predominant",
            HarmonicFunction::Dominant => "dominant",
        };
        write!(f, "{s}")
    }
}

/// Standard tonic/predominant/dominant grouping for a diatonic major-key
/// scale degree (I, iii, vi = tonic; ii, IV = predominant; V, vii° = dominant).
fn harmonic_function_of_degree(degree: ScaleDegree) -> HarmonicFunction {
    match degree.0 {
        1 | 3 | 6 => HarmonicFunction::Tonic,
        2 | 4 => HarmonicFunction::Predominant,
        5 | 7 => HarmonicFunction::Dominant,
        _ => HarmonicFunction::Tonic,
    }
}

/// A diatonic Roman numeral: scale degree, chord quality, and inversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RomanNumeral {
    pub degree: ScaleDegree,
    pub quality: ChordQuality,
    pub inversion: ChordInversion,
}

impl RomanNumeral {
    pub const fn new(
        degree: ScaleDegree,
        quality: ChordQuality,
        inversion: ChordInversion,
    ) -> Self {
        RomanNumeral {
            degree,
            quality,
            inversion,
        }
    }

    pub const fn root_position(degree: ScaleDegree, quality: ChordQuality) -> Self {
        RomanNumeral::new(degree, quality, ChordInversion::Root)
    }

    pub const fn with_inversion(self, inversion: ChordInversion) -> Self {
        RomanNumeral::new(self.degree, self.quality, inversion)
    }

    // Diatonic triads in a major key.
    pub const I: RomanNumeral =
        RomanNumeral::root_position(ScaleDegree::TONIC, ChordQuality::MajorTriad);
    pub const II: RomanNumeral =
        RomanNumeral::root_position(ScaleDegree::SUPERTONIC, ChordQuality::MinorTriad);
    pub const III: RomanNumeral =
        RomanNumeral::root_position(ScaleDegree::MEDIANT, ChordQuality::MinorTriad);
    pub const IV: RomanNumeral =
        RomanNumeral::root_position(ScaleDegree::SUBDOMINANT, ChordQuality::MajorTriad);
    pub const V: RomanNumeral =
        RomanNumeral::root_position(ScaleDegree::DOMINANT, ChordQuality::MajorTriad);
    pub const VI: RomanNumeral =
        RomanNumeral::root_position(ScaleDegree::SUBMEDIANT, ChordQuality::MinorTriad);
    pub const VII_DIM: RomanNumeral =
        RomanNumeral::root_position(ScaleDegree::LEADING_TONE, ChordQuality::DiminishedTriad);
    pub const V7: RomanNumeral =
        RomanNumeral::root_position(ScaleDegree::DOMINANT, ChordQuality::DominantSeventh);

    /// All seven diatonic triads plus V7, root position — the harmonic
    /// vocabulary of the v0.1 spine (AGENTS.md section 5).
    pub fn diatonic_vocabulary() -> [RomanNumeral; 8] {
        [
            RomanNumeral::I,
            RomanNumeral::II,
            RomanNumeral::III,
            RomanNumeral::IV,
            RomanNumeral::V,
            RomanNumeral::VI,
            RomanNumeral::VII_DIM,
            RomanNumeral::V7,
        ]
    }

    pub fn harmonic_function(&self) -> HarmonicFunction {
        harmonic_function_of_degree(self.degree)
    }

    pub fn to_chord(&self, key: &Key) -> Chord {
        Chord {
            root: key.diatonic_pitch_class(self.degree),
            quality: self.quality,
        }
    }
}

impl fmt::Display for RomanNumeral {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const NUMERALS: [&str; 7] = ["I", "II", "III", "IV", "V", "VI", "VII"];
        let base = NUMERALS[(self.degree.0 as usize - 1) % 7];
        let mut text = if self.quality.is_minor_case() {
            base.to_lowercase()
        } else {
            base.to_string()
        };
        if matches!(
            self.quality,
            ChordQuality::DiminishedTriad | ChordQuality::DiminishedSeventh
        ) {
            text.push('°');
        } else if self.quality == ChordQuality::HalfDiminishedSeventh {
            text.push('ø');
        }
        write!(
            f,
            "{text}{}",
            self.inversion.figured_bass(self.quality.is_seventh())
        )
    }
}

/// An absolute chord: root pitch class plus quality. Unlike
/// `RomanNumeral`, a `Chord` doesn't need a `Key` to spell its own tones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Chord {
    pub root: PitchClass,
    pub quality: ChordQuality,
}

impl Chord {
    pub const fn new(root: PitchClass, quality: ChordQuality) -> Self {
        Chord { root, quality }
    }

    /// Chord tones stacked in thirds above the root, correctly spelled.
    ///
    /// Fails closed: if a tone needs an accidental beyond what
    /// `Accidental` can represent (double-flat..double-sharp), this
    /// returns `Err` rather than a silently wrong pitch. Every chord
    /// `RomanNumeral::to_chord` builds from a practical key is safe —
    /// this is only reachable by constructing a `Chord` directly with an
    /// unusual `root` (see `tests/properties.rs`).
    pub fn pitch_classes(&self) -> Result<Vec<PitchClass>> {
        self.quality
            .interval_semitones()
            .iter()
            .enumerate()
            .map(|(i, &semitones)| {
                spell_above(self.root, 2 * i as i32, semitones).ok_or_else(|| {
                    MokurenError::UnrepresentablePitch(format!(
                        "{:?} chord on {} has no representable spelling for tone {i}",
                        self.quality, self.root
                    ))
                })
            })
            .collect()
    }

    /// `None` both when this isn't a seventh chord and when the seventh
    /// can't be spelled — a rule that can't determine the seventh can't
    /// enforce its resolution either way, so both cases mean "nothing to
    /// check here," never a fabricated pitch.
    pub fn chordal_seventh(&self) -> Option<PitchClass> {
        if !self.quality.is_seventh() {
            return None;
        }
        self.pitch_classes().ok().map(|tones| tones[3])
    }

    /// `false` both when `pc` genuinely isn't a chord tone and when the
    /// chord can't be spelled at all — used as a candidate-generation
    /// filter, where "can't confirm this chord tone" and "exclude it"
    /// are the same safe outcome.
    pub fn contains_pitch_class(&self, pc: PitchClass) -> bool {
        self.pitch_classes()
            .is_ok_and(|tones| tones.iter().any(|t| t.is_enharmonic_to(&pc)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::{Accidental, NoteLetter};

    #[test]
    fn c_major_triad_spelling() {
        let chord = RomanNumeral::I.to_chord(&Key::C_MAJOR);
        let tones = chord.pitch_classes().unwrap();
        assert_eq!(tones, vec![PitchClass::C, PitchClass::E, PitchClass::G]);
    }

    #[test]
    fn v7_in_c_major_is_g_b_d_f() {
        let chord = RomanNumeral::V7.to_chord(&Key::C_MAJOR);
        assert_eq!(
            chord.pitch_classes().unwrap(),
            vec![PitchClass::G, PitchClass::B, PitchClass::D, PitchClass::F]
        );
    }

    #[test]
    fn unspellable_chord_fails_closed_instead_of_returning_a_wrong_pitch() {
        // Same case pinned in src/pitch.rs: a double-flat root combined
        // with a quality whose third needs a triple-flat to spell
        // correctly. Reachable only via `Chord::new` directly — no
        // `RomanNumeral::to_chord` ever produces this root/quality pair.
        let cbb = PitchClass::new(NoteLetter::C, Accidental::DoubleFlat);
        let chord = Chord::new(cbb, ChordQuality::MinorSeventh);
        assert!(matches!(
            chord.pitch_classes(),
            Err(MokurenError::UnrepresentablePitch(_))
        ));
        assert!(!chord.contains_pitch_class(PitchClass::E));
        assert_eq!(chord.chordal_seventh(), None);
    }

    #[test]
    fn vii_dim_in_c_major_is_b_d_f() {
        let chord = RomanNumeral::VII_DIM.to_chord(&Key::C_MAJOR);
        assert_eq!(
            chord.pitch_classes().unwrap(),
            vec![PitchClass::B, PitchClass::D, PitchClass::F]
        );
    }

    #[test]
    fn display_matches_textbook_notation() {
        assert_eq!(RomanNumeral::I.to_string(), "I");
        assert_eq!(RomanNumeral::VI.to_string(), "vi");
        assert_eq!(RomanNumeral::VII_DIM.to_string(), "vii°");
        assert_eq!(RomanNumeral::V7.to_string(), "V7");
        assert_eq!(
            RomanNumeral::I
                .with_inversion(ChordInversion::First)
                .to_string(),
            "I6"
        );
        assert_eq!(
            RomanNumeral::V7
                .with_inversion(ChordInversion::First)
                .to_string(),
            "V65"
        );
    }

    #[test]
    fn harmonic_functions_match_textbook_grouping() {
        assert_eq!(RomanNumeral::I.harmonic_function(), HarmonicFunction::Tonic);
        assert_eq!(
            RomanNumeral::II.harmonic_function(),
            HarmonicFunction::Predominant
        );
        assert_eq!(
            RomanNumeral::IV.harmonic_function(),
            HarmonicFunction::Predominant
        );
        assert_eq!(
            RomanNumeral::V.harmonic_function(),
            HarmonicFunction::Dominant
        );
        assert_eq!(
            RomanNumeral::VI.harmonic_function(),
            HarmonicFunction::Tonic
        );
        assert_eq!(
            RomanNumeral::VII_DIM.harmonic_function(),
            HarmonicFunction::Dominant
        );
    }

    #[test]
    fn f_major_iv_chord_uses_bb_not_a_sharp() {
        let chord = RomanNumeral::IV.to_chord(&Key::F_MAJOR);
        assert_eq!(chord.root, PitchClass::new(NoteLetter::B, Accidental::Flat));
    }
}
