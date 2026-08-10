//! Key, mode, and scale degrees.
//!
//! v0.1 only implements the major mode (AGENTS.md section 5 leaves minor
//! as "necessity to be evaluated" — no melody in the v0.1 spine needs it).
//! `Mode` is still an enum, not a bool, so minor variants can be added
//! without an API break.

use crate::error::{MokurenError, Result};
use crate::pitch::{PitchClass, spell_above};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    Major,
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

    /// Validated constructor: fails if any of the seven scale degrees
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
        Ok(key)
    }

    fn try_diatonic_pitch_class(&self, degree: ScaleDegree) -> Option<PitchClass> {
        let Mode::Major = self.mode;
        let step = (degree.0 as i32 - 1).rem_euclid(7);
        spell_above(self.tonic, step, MAJOR_STEP_SEMITONES[step as usize])
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
        let Mode::Major = self.mode;
        write!(f, "{} major", self.tonic)
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
    fn degree_of_round_trips_with_diatonic_pitch_class() {
        for degree in 1..=7u8 {
            let pc = Key::C_MAJOR.diatonic_pitch_class(ScaleDegree(degree));
            assert_eq!(Key::C_MAJOR.degree_of(pc), Some(ScaleDegree(degree)));
        }
    }
}
