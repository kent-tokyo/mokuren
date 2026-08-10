//! Pitch primitives: letter names, accidentals, octaves, and intervals.
//!
//! Pitch classes keep their spelling (letter + accidental) rather than
//! collapsing to a semitone number, so chord spelling and Roman-numeral
//! analysis stay meaningful instead of stringly-typed.

use crate::error::{MokurenError, Result};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NoteLetter {
    C,
    D,
    E,
    F,
    G,
    A,
    B,
}

impl NoteLetter {
    /// Semitones above C for the natural (unaltered) letter.
    const fn natural_semitone(self) -> i32 {
        match self {
            NoteLetter::C => 0,
            NoteLetter::D => 2,
            NoteLetter::E => 4,
            NoteLetter::F => 5,
            NoteLetter::G => 7,
            NoteLetter::A => 9,
            NoteLetter::B => 11,
        }
    }

    /// Index into the diatonic letter cycle (C=0..B=6), for counting
    /// generic interval numbers and scale degrees.
    const fn step_index(self) -> i32 {
        match self {
            NoteLetter::C => 0,
            NoteLetter::D => 1,
            NoteLetter::E => 2,
            NoteLetter::F => 3,
            NoteLetter::G => 4,
            NoteLetter::A => 5,
            NoteLetter::B => 6,
        }
    }

    const fn from_step_index(index: i32) -> Self {
        match index.rem_euclid(7) {
            0 => NoteLetter::C,
            1 => NoteLetter::D,
            2 => NoteLetter::E,
            3 => NoteLetter::F,
            4 => NoteLetter::G,
            5 => NoteLetter::A,
            _ => NoteLetter::B,
        }
    }
}

impl fmt::Display for NoteLetter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let c = match self {
            NoteLetter::C => 'C',
            NoteLetter::D => 'D',
            NoteLetter::E => 'E',
            NoteLetter::F => 'F',
            NoteLetter::G => 'G',
            NoteLetter::A => 'A',
            NoteLetter::B => 'B',
        };
        write!(f, "{c}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Accidental {
    DoubleFlat,
    Flat,
    Natural,
    Sharp,
    DoubleSharp,
}

impl Accidental {
    const fn semitone_offset(self) -> i32 {
        match self {
            Accidental::DoubleFlat => -2,
            Accidental::Flat => -1,
            Accidental::Natural => 0,
            Accidental::Sharp => 1,
            Accidental::DoubleSharp => 2,
        }
    }
}

impl fmt::Display for Accidental {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Accidental::DoubleFlat => "bb",
            Accidental::Flat => "b",
            Accidental::Natural => "",
            Accidental::Sharp => "#",
            Accidental::DoubleSharp => "##",
        };
        write!(f, "{s}")
    }
}

/// A pitch class: a letter name plus accidental (e.g. `Bb`, `F#`).
///
/// Spelling is kept rather than reduced to a semitone, since chord
/// spelling and doubling checks need to know *which* letter is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PitchClass {
    pub letter: NoteLetter,
    pub accidental: Accidental,
}

impl PitchClass {
    pub const fn new(letter: NoteLetter, accidental: Accidental) -> Self {
        PitchClass { letter, accidental }
    }

    pub const fn natural(letter: NoteLetter) -> Self {
        PitchClass::new(letter, Accidental::Natural)
    }

    pub const C: PitchClass = PitchClass::natural(NoteLetter::C);
    pub const D: PitchClass = PitchClass::natural(NoteLetter::D);
    pub const E: PitchClass = PitchClass::natural(NoteLetter::E);
    pub const F: PitchClass = PitchClass::natural(NoteLetter::F);
    pub const G: PitchClass = PitchClass::natural(NoteLetter::G);
    pub const A: PitchClass = PitchClass::natural(NoteLetter::A);
    pub const B: PitchClass = PitchClass::natural(NoteLetter::B);

    /// Semitone value in `0..12`, with C = 0.
    pub fn semitone(&self) -> u8 {
        (self.letter.natural_semitone() + self.accidental.semitone_offset()).rem_euclid(12) as u8
    }

    /// Enharmonic equality: same sounding pitch class regardless of spelling.
    pub fn is_enharmonic_to(&self, other: &PitchClass) -> bool {
        self.semitone() == other.semitone()
    }
}

impl fmt::Display for PitchClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.letter, self.accidental)
    }
}

impl FromStr for PitchClass {
    type Err = MokurenError;

    fn from_str(s: &str) -> Result<Self> {
        let mut chars = s.chars();
        let letter = match chars.next() {
            Some('C') => NoteLetter::C,
            Some('D') => NoteLetter::D,
            Some('E') => NoteLetter::E,
            Some('F') => NoteLetter::F,
            Some('G') => NoteLetter::G,
            Some('A') => NoteLetter::A,
            Some('B') => NoteLetter::B,
            _ => return Err(MokurenError::Parse(format!("invalid pitch class: {s:?}"))),
        };
        let rest: String = chars.collect();
        let accidental = match rest.as_str() {
            "" => Accidental::Natural,
            "b" => Accidental::Flat,
            "bb" => Accidental::DoubleFlat,
            "#" | "s" => Accidental::Sharp,
            "##" | "ss" => Accidental::DoubleSharp,
            other => {
                return Err(MokurenError::Parse(format!(
                    "invalid accidental {other:?} in {s:?}"
                )));
            }
        };
        Ok(PitchClass::new(letter, accidental))
    }
}

/// Scientific pitch octave (C4 = middle C).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Octave(pub i32);

/// A fully specified pitch: pitch class plus octave.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pitch {
    pub pitch_class: PitchClass,
    pub octave: Octave,
}

impl Pitch {
    pub const fn new(pitch_class: PitchClass, octave: Octave) -> Self {
        Pitch {
            pitch_class,
            octave,
        }
    }

    /// MIDI note number (C4 = 60), used for comparisons and distance.
    pub fn midi(&self) -> i32 {
        12 * (self.octave.0 + 1) + self.pitch_class.semitone() as i32
    }

    /// Diatonic letter-step position, used for generic interval counting.
    /// Not the same as `midi()`: it ignores accidentals.
    fn diatonic_step(&self) -> i32 {
        self.octave.0 * 7 + self.pitch_class.letter.step_index()
    }
}

impl fmt::Display for Pitch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.pitch_class, self.octave.0)
    }
}

impl FromStr for Pitch {
    type Err = MokurenError;

    fn from_str(s: &str) -> Result<Self> {
        let split_at = s
            .find(|c: char| c == '-' || c.is_ascii_digit())
            .ok_or_else(|| MokurenError::Parse(format!("missing octave in {s:?}")))?;
        let (pc_part, oct_part) = s.split_at(split_at);
        let pitch_class: PitchClass = pc_part.parse()?;
        let octave: i32 = oct_part
            .parse()
            .map_err(|_| MokurenError::Parse(format!("invalid octave in {s:?}")))?;
        if !(-1..=9).contains(&octave) {
            return Err(MokurenError::Parse(format!(
                "octave {octave} out of supported range -1..=9"
            )));
        }
        Ok(Pitch::new(pitch_class, Octave(octave)))
    }
}

/// Diatonic interval quality. Not every quality is meaningful for every
/// generic number (e.g. `Perfect` never applies to a second) — mokuren's
/// interval construction never produces an invalid pairing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntervalQuality {
    Diminished,
    Minor,
    Major,
    Perfect,
    Augmented,
}

impl fmt::Display for IntervalQuality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            IntervalQuality::Diminished => "diminished",
            IntervalQuality::Minor => "minor",
            IntervalQuality::Major => "major",
            IntervalQuality::Perfect => "perfect",
            IntervalQuality::Augmented => "augmented",
        };
        write!(f, "{s}")
    }
}

/// The interval between two pitches, keeping the generic (diatonic)
/// number separate from quality and semitone distance — required to
/// tell a perfect fifth from a diminished sixth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Interval {
    /// Generic diatonic number: 1 = unison, 2 = second, ... 8 = octave,
    /// continuing past 8 for compound intervals.
    pub number: i32,
    pub quality: IntervalQuality,
    /// Absolute semitone distance between the two pitches.
    pub semitones: i32,
}

impl Interval {
    /// The interval spanned by two pitches, order-independent.
    pub fn between(a: Pitch, b: Pitch) -> Interval {
        let (low, high) = if a.diatonic_step() <= b.diatonic_step() {
            (a, b)
        } else {
            (b, a)
        };
        let number = high.diatonic_step() - low.diatonic_step() + 1;
        let semitones = (high.midi() - low.midi()).abs();
        let quality = Self::quality_for(number, semitones);
        Interval {
            number,
            quality,
            semitones,
        }
    }

    fn quality_for(number: i32, semitones: i32) -> IntervalQuality {
        let octaves = (number - 1) / 7;
        let degree_in_octave = (number - 1) % 7;
        let base = [0, 2, 4, 5, 7, 9, 11][degree_in_octave as usize];
        let expected_major = base + 12 * octaves;
        let diff = semitones - expected_major;
        let is_perfect_family = matches!(degree_in_octave, 0 | 3 | 4);
        if is_perfect_family {
            match diff {
                0 => IntervalQuality::Perfect,
                d if d > 0 => IntervalQuality::Augmented,
                _ => IntervalQuality::Diminished,
            }
        } else {
            match diff {
                0 => IntervalQuality::Major,
                -1 => IntervalQuality::Minor,
                d if d > 0 => IntervalQuality::Augmented,
                _ => IntervalQuality::Diminished,
            }
        }
    }

    /// Generic number reduced to `1..=8` (unison..octave), collapsing
    /// compound intervals to their simple form.
    pub fn simple_number(&self) -> i32 {
        let octaves = (self.number - 1) / 7;
        (self.number - 1) - octaves * 7 + 1
    }

    /// Whether this interval belongs to the perfect-fifth/octave/unison
    /// class checked by parallel-motion rules.
    pub fn is_perfect_fifth_class(&self) -> bool {
        self.simple_number() == 5 && self.quality == IntervalQuality::Perfect
    }

    /// True unison: the same pitch, not just the same pitch class an
    /// octave apart.
    pub fn is_unison(&self) -> bool {
        self.number == 1
    }

    /// A perfect octave or any compound multiple of one (15th, ...),
    /// but not a plain unison.
    pub fn is_octave_class(&self) -> bool {
        self.number > 1 && self.simple_number() == 1 && self.quality == IntervalQuality::Perfect
    }

    /// Simplified consonance classification (v0.1: no contextual
    /// treatment of the fourth or species-counterpoint nuance).
    pub fn is_consonant(&self) -> bool {
        matches!(
            (self.simple_number(), self.quality),
            (1, IntervalQuality::Perfect)
                | (3, IntervalQuality::Major | IntervalQuality::Minor)
                | (5, IntervalQuality::Perfect)
                | (6, IntervalQuality::Major | IntervalQuality::Minor)
                | (8, IntervalQuality::Perfect)
        )
    }
}

/// Spells a pitch class a given number of letter-steps and semitones
/// above another, choosing whatever accidental makes the semitone math
/// work out. Shared by `Key` (scale spelling) and `Chord` (tertian
/// stacking) so both stay diatonically correct without a circular
/// dependency between them.
///
/// Returns `None` when the required accidental exceeds what
/// `Accidental` can represent — see `accidental_for_offset`. Every
/// caller in this crate only ever spells from diatonic, single-accidental
/// roots (a practical key's tonic, or a chord root drawn from one),
/// which never hits that limit — `Key::diatonic_pitch_class` proves this
/// for scale construction specifically (`tests/properties.rs`) and
/// unwraps accordingly. A `root` with an unusual accidental of its own
/// (as `Chord::new` — a public constructor — technically permits) can
/// exceed it; `Chord::pitch_classes` surfaces that as `Err` rather than
/// a wrong pitch.
pub(crate) fn spell_above(
    root: PitchClass,
    letter_steps: i32,
    semitones: i32,
) -> Option<PitchClass> {
    let letter_index = (root.letter.step_index() + letter_steps).rem_euclid(7);
    let letter = NoteLetter::from_step_index(letter_index);
    let target_semitone = (root.semitone() as i32 + semitones).rem_euclid(12);
    let natural = PitchClass::natural(letter).semitone() as i32;
    let accidental = accidental_for_offset((target_semitone - natural).rem_euclid(12))?;
    Some(PitchClass::new(letter, accidental))
}

/// `Accidental` only spans double-flat..double-sharp (-2..=2 semitones
/// from natural). An offset outside that range has no representable
/// accidental: `None`, not a `Natural` fallback that would silently
/// hand back a wrong pitch. Widening `Accidental` to cover it would be
/// speculative: nothing in this crate ever constructs a root that
/// triggers it (AGENTS.md section 5 keeps v0.1 diatonic-only) — see
/// `spell_above`'s callers for how they fail closed instead.
pub(crate) fn accidental_for_offset(offset: i32) -> Option<Accidental> {
    match offset.rem_euclid(12) {
        0 => Some(Accidental::Natural),
        1 => Some(Accidental::Sharp),
        2 => Some(Accidental::DoubleSharp),
        11 => Some(Accidental::Flat),
        10 => Some(Accidental::DoubleFlat),
        _ => None,
    }
}

impl fmt::Display for Interval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.quality, self.number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spell_above_fails_closed_beyond_the_representable_accidental_range() {
        // Cbb (a root Chord::new() permits but the engine itself never
        // constructs) + a minor seventh's third (3 semitones) needs a
        // triple-flat E to spell correctly. `Accidental` can't represent
        // that, so this pins `None` rather than a wrong pitch — see
        // `spell_above`'s doc comment.
        let cbb = PitchClass::new(NoteLetter::C, Accidental::DoubleFlat);
        assert_eq!(spell_above(cbb, 2, 3), None);
    }

    #[test]
    fn middle_c_is_midi_60() {
        let c4 = Pitch::new(PitchClass::C, Octave(4));
        assert_eq!(c4.midi(), 60);
    }

    #[test]
    fn parses_pitch_with_accidental_and_negative_octave() {
        let p: Pitch = "F#3".parse().unwrap();
        assert_eq!(
            p.pitch_class,
            PitchClass::new(NoteLetter::F, Accidental::Sharp)
        );
        assert_eq!(p.octave, Octave(3));

        let p: Pitch = "Bb-1".parse().unwrap();
        assert_eq!(p.octave, Octave(-1));
    }

    #[test]
    fn rejects_garbage_pitch() {
        assert!("H4".parse::<Pitch>().is_err());
        assert!("C".parse::<Pitch>().is_err());
    }

    #[test]
    fn c_to_g_is_perfect_fifth() {
        let c4 = Pitch::new(PitchClass::C, Octave(4));
        let g4 = Pitch::new(PitchClass::G, Octave(4));
        let iv = Interval::between(c4, g4);
        assert_eq!(iv.number, 5);
        assert_eq!(iv.quality, IntervalQuality::Perfect);
        assert_eq!(iv.semitones, 7);
        assert!(iv.is_perfect_fifth_class());
    }

    #[test]
    fn b_to_f_is_diminished_fifth_not_augmented_fourth() {
        let b3 = Pitch::new(PitchClass::B, Octave(3));
        let f4 = Pitch::new(PitchClass::F, Octave(4));
        let iv = Interval::between(b3, f4);
        assert_eq!(iv.number, 5);
        assert_eq!(iv.quality, IntervalQuality::Diminished);
    }

    #[test]
    fn octave_is_perfect_octave_class() {
        let c4 = Pitch::new(PitchClass::C, Octave(4));
        let c5 = Pitch::new(PitchClass::C, Octave(5));
        let iv = Interval::between(c4, c5);
        assert_eq!(iv.number, 8);
        assert!(iv.is_octave_class());
        assert!(!iv.is_unison());
    }

    #[test]
    fn interval_between_is_order_independent() {
        let c4 = Pitch::new(PitchClass::C, Octave(4));
        let g4 = Pitch::new(PitchClass::G, Octave(4));
        assert_eq!(Interval::between(c4, g4), Interval::between(g4, c4));
    }
}
