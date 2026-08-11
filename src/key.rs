//! Key, mode, and scale degrees.
//!
//! `Mode::Minor` means *natural* minor — the scale that matches a minor
//! key's actual signature. Harmonic minor's raised leading tone isn't a
//! separate `Mode`: a piece doesn't change key signature to use it, it's
//! a situational chromatic alteration layered on top (see
//! `Key::raised_leading_tone` and `RomanNumeral::harmonic_minor_vocabulary`
//! in `chord.rs`) — the same "extra vocabulary, not a redesign" shape
//! secondary dominants used. Melodic minor (a different raised-6th
//! convention) isn't implemented at all — see README's current
//! limitations.

use crate::error::{MokurenError, Result};
use crate::pitch::{PitchClass, spell_above};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    Major,
    Minor,
}

/// A 1-indexed scale degree (1 = tonic .. 7 = leading tone).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ScaleDegree(pub u8);

impl ScaleDegree {
    pub const TONIC: ScaleDegree = ScaleDegree(1);
    pub const SUPERTONIC: ScaleDegree = ScaleDegree(2);
    pub const MEDIANT: ScaleDegree = ScaleDegree(3);
    pub const SUBDOMINANT: ScaleDegree = ScaleDegree(4);
    pub const DOMINANT: ScaleDegree = ScaleDegree(5);
    pub const SUBMEDIANT: ScaleDegree = ScaleDegree(6);
    pub const LEADING_TONE: ScaleDegree = ScaleDegree(7);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key {
    pub tonic: PitchClass,
    pub mode: Mode,
}

/// Major-scale letter offsets (0-indexed steps from the tonic letter)
/// and their accidental relative to the tonic's own accidental, expressed
/// as a semitone correction so any spelled tonic works out diatonically.
const MAJOR_STEP_SEMITONES: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];

/// Natural minor: same letter steps, semitones 3 (not 4) and 8/10 (not
/// 9/11) lower than major at the corresponding degree — no accidental
/// alteration is applied here; that's `raised_leading_tone`'s job.
const NATURAL_MINOR_STEP_SEMITONES: [i32; 7] = [0, 2, 3, 5, 7, 8, 10];

impl Key {
    /// Unvalidated: only for this module's own known-good consts below.
    /// A tonic with an ordinary accidental can still fail to produce a
    /// representable scale (e.g. a double-sharp tonic's own third can
    /// need a triple-sharp — this isn't as rare an edge as it sounds),
    /// so arbitrary construction goes through `new`, which checks that.
    const fn new_unchecked(tonic: PitchClass, mode: Mode) -> Self {
        Key { tonic, mode }
    }

    pub const C_MAJOR: Key = Key::new_unchecked(PitchClass::C, Mode::Major);
    pub const G_MAJOR: Key = Key::new_unchecked(PitchClass::G, Mode::Major);
    pub const F_MAJOR: Key = Key::new_unchecked(PitchClass::F, Mode::Major);
    pub const D_MAJOR: Key = Key::new_unchecked(PitchClass::D, Mode::Major);
    pub const A_MINOR: Key = Key::new_unchecked(PitchClass::A, Mode::Minor);

    /// Validated constructor: fails if any of the seven scale degrees —
    /// or, for a minor key, the harmonic-minor raised leading tone —
    /// would need an accidental beyond what `Accidental` can represent.
    /// This is the *only* public way to construct a `Key` with an
    /// arbitrary tonic, so once a `Key` value exists — from here or one
    /// of the consts above — every other method on it is infallible;
    /// an invalid `Key` simply can't exist (AGENTS.md section 4: make
    /// invalid states unrepresentable, rather than threading `Result`
    /// through `diatonic_pitch_class`, `scale`, `RomanNumeral::to_chord`,
    /// and everything downstream of them in the search hot path).
    pub fn new(tonic: PitchClass, mode: Mode) -> Result<Self> {
        let key = Key::new_unchecked(tonic, mode);
        for degree in 1..=7u8 {
            key.try_diatonic_pitch_class(ScaleDegree(degree))
                .ok_or_else(|| {
                    MokurenError::UnrepresentablePitch(format!(
                        "{tonic} {mode:?} has no representable spelling for scale degree {degree}"
                    ))
                })?;
        }
        if matches!(mode, Mode::Minor) {
            key.try_raised_leading_tone().ok_or_else(|| {
                MokurenError::UnrepresentablePitch(format!(
                    "{tonic} {mode:?} has no representable spelling for its raised (harmonic-minor) leading tone"
                ))
            })?;
        }
        Ok(key)
    }

    fn try_diatonic_pitch_class(&self, degree: ScaleDegree) -> Option<PitchClass> {
        let step = (degree.0 as i32 - 1).rem_euclid(7);
        let table = match self.mode {
            Mode::Major => MAJOR_STEP_SEMITONES,
            Mode::Minor => NATURAL_MINOR_STEP_SEMITONES,
        };
        spell_above(self.tonic, step, table[step as usize])
    }

    fn try_raised_leading_tone(&self) -> Option<PitchClass> {
        let natural_seventh = self.try_diatonic_pitch_class(ScaleDegree::LEADING_TONE)?;
        spell_above(natural_seventh, 0, 1)
    }

    /// The pitch class that actually functions as a leading tone in this
    /// key — the plain diatonic 7th degree in major (already a semitone
    /// below the tonic), or the harmonic-minor-*raised* 7th in minor
    /// (natural minor's own unraised 7th, a whole step below the tonic,
    /// doesn't pull upward the same way and isn't treated as one — see
    /// `RomanNumeral::harmonic_minor_vocabulary`). Used by
    /// `LeadingToneDoublingRule`/`LeadingToneResolutionRule` so they
    /// check the pitch that's actually chromatically active, not always
    /// the plain diatonic 7th.
    pub fn functional_leading_tone(&self) -> PitchClass {
        match self.mode {
            Mode::Major => self.diatonic_pitch_class(ScaleDegree::LEADING_TONE),
            Mode::Minor => self.try_raised_leading_tone().expect(
                "a minor Key can only be constructed (via `new`) when its raised leading tone is representable",
            ),
        }
    }

    /// The pitch class at a given scale degree, spelled diatonically
    /// (each of the 7 letters used exactly once per octave).
    pub fn diatonic_pitch_class(&self, degree: ScaleDegree) -> PitchClass {
        self.try_diatonic_pitch_class(degree).expect(
            "a Key can only be constructed (via `new`) when every scale degree is representable",
        )
    }

    /// The seven diatonic pitch classes, degree 1 through 7.
    pub fn scale(&self) -> [PitchClass; 7] {
        std::array::from_fn(|i| self.diatonic_pitch_class(ScaleDegree((i + 1) as u8)))
    }

    /// The scale degree of a pitch class, if it belongs to this key's
    /// diatonic scale.
    pub fn degree_of(&self, pitch_class: PitchClass) -> Option<ScaleDegree> {
        self.scale()
            .iter()
            .position(|pc| pc.is_enharmonic_to(&pitch_class))
            .map(|i| ScaleDegree((i + 1) as u8))
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mode = match self.mode {
            Mode::Major => "major",
            Mode::Minor => "minor",
        };
        write!(f, "{} {mode}", self.tonic)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::{Accidental, NoteLetter};

    #[test]
    fn c_major_scale_is_all_naturals() {
        let scale = Key::C_MAJOR.scale();
        let expected = [
            NoteLetter::C,
            NoteLetter::D,
            NoteLetter::E,
            NoteLetter::F,
            NoteLetter::G,
            NoteLetter::A,
            NoteLetter::B,
        ];
        for (pc, letter) in scale.iter().zip(expected) {
            assert_eq!(pc.letter, letter);
            assert_eq!(pc.accidental, Accidental::Natural);
        }
    }

    #[test]
    fn g_major_has_f_sharp() {
        let scale = Key::G_MAJOR.scale();
        assert_eq!(scale[6], PitchClass::new(NoteLetter::F, Accidental::Sharp));
    }

    #[test]
    fn f_major_has_b_flat() {
        let scale = Key::F_MAJOR.scale();
        assert_eq!(scale[3], PitchClass::new(NoteLetter::B, Accidental::Flat));
    }

    #[test]
    fn a_minor_scale_is_all_naturals() {
        let scale = Key::A_MINOR.scale();
        let expected = [
            NoteLetter::A,
            NoteLetter::B,
            NoteLetter::C,
            NoteLetter::D,
            NoteLetter::E,
            NoteLetter::F,
            NoteLetter::G,
        ];
        for (pc, letter) in scale.iter().zip(expected) {
            assert_eq!(pc.letter, letter);
            assert_eq!(pc.accidental, Accidental::Natural);
        }
    }

    #[test]
    fn natural_minor_seventh_degree_is_a_whole_step_below_tonic() {
        // A natural minor: G natural, not G# — the *un*raised subtonic.
        let scale = Key::A_MINOR.scale();
        assert_eq!(scale[6], PitchClass::G);
    }

    #[test]
    fn raised_leading_tone_across_several_minor_tonics() {
        // Hand-verified against real key signatures — checked directly
        // rather than trusting spell_above(_, 0, 1) by inspection alone
        // (it's never exercised with 0 letter-steps anywhere else in
        // this crate).
        let cases = [
            (
                PitchClass::A,
                PitchClass::new(NoteLetter::G, Accidental::Sharp),
            ), // A minor: G#
            (PitchClass::C, PitchClass::B), // C minor: B natural
            (PitchClass::F, PitchClass::E), // F minor: E natural
            (
                PitchClass::new(NoteLetter::B, Accidental::Flat),
                PitchClass::A,
            ), // Bb minor: A natural
            (
                PitchClass::new(NoteLetter::E, Accidental::Flat),
                PitchClass::D,
            ), // Eb minor: D natural
            (
                PitchClass::new(NoteLetter::C, Accidental::Sharp),
                PitchClass::new(NoteLetter::B, Accidental::Sharp),
            ), // C# minor: B#
        ];
        for (tonic, expected_leading_tone) in cases {
            let key = Key::new(tonic, Mode::Minor).unwrap();
            assert_eq!(
                key.functional_leading_tone(),
                expected_leading_tone,
                "raised leading tone of {tonic} minor"
            );
        }
    }

    #[test]
    fn major_functional_leading_tone_is_the_plain_diatonic_seventh() {
        assert_eq!(
            Key::C_MAJOR.functional_leading_tone(),
            Key::C_MAJOR.diatonic_pitch_class(ScaleDegree::LEADING_TONE)
        );
    }

    #[test]
    fn degree_of_round_trips_with_diatonic_pitch_class() {
        for degree in 1..=7u8 {
            let pc = Key::C_MAJOR.diatonic_pitch_class(ScaleDegree(degree));
            assert_eq!(Key::C_MAJOR.degree_of(pc), Some(ScaleDegree(degree)));
        }
    }
}
