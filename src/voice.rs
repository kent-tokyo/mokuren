//! SATB voices, ranges, and the pure voice-leading detectors that the
//! rule engine (`rules.rs`) wraps as `Rule` implementations.
//!
//! Keeping detection here as plain functions on `Voicing` makes them
//! directly unit-testable against hand-built chords, independent of
//! candidate generation or search.

use crate::pitch::{Interval, Octave, Pitch, PitchClass};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VoicePart {
    Soprano,
    Alto,
    Tenor,
    Bass,
}

impl VoicePart {
    pub const fn all() -> [VoicePart; 4] {
        [
            VoicePart::Soprano,
            VoicePart::Alto,
            VoicePart::Tenor,
            VoicePart::Bass,
        ]
    }

    /// All 6 unordered pairs among the four voices, for parallel-motion
    /// checks that must consider every pair, not just adjacent ones.
    pub fn all_pairs() -> [(VoicePart, VoicePart); 6] {
        [
            (VoicePart::Soprano, VoicePart::Alto),
            (VoicePart::Soprano, VoicePart::Tenor),
            (VoicePart::Soprano, VoicePart::Bass),
            (VoicePart::Alto, VoicePart::Tenor),
            (VoicePart::Alto, VoicePart::Bass),
            (VoicePart::Tenor, VoicePart::Bass),
        ]
    }

    /// Adjacent pairs (top to bottom), for spacing and overlap checks.
    pub fn adjacent_pairs() -> [(VoicePart, VoicePart); 3] {
        [
            (VoicePart::Soprano, VoicePart::Alto),
            (VoicePart::Alto, VoicePart::Tenor),
            (VoicePart::Tenor, VoicePart::Bass),
        ]
    }

    /// A conventional default choral range for this voice.
    pub fn default_range(&self) -> VoiceRange {
        match self {
            // Ceiling widened from G5 (see PLAN.md/tasks/lessons.md): the
            // v0.1.0 chorale baseline found 5 real Bach soprano lines
            // that briefly reach A5, one step above G5 — every candidate
            // at that position was rejected (soprano is taken directly
            // from the input melody, never range-filtered like the
            // generated inner voices), killing the whole harmonization.
            VoicePart::Soprano => VoiceRange::new(
                Pitch::new(PitchClass::C, Octave(4)),
                Pitch::new(PitchClass::A, Octave(5)),
            ),
            VoicePart::Alto => VoiceRange::new(
                Pitch::new(PitchClass::G, Octave(3)),
                Pitch::new(PitchClass::C, Octave(5)),
            ),
            VoicePart::Tenor => VoiceRange::new(
                Pitch::new(PitchClass::C, Octave(3)),
                Pitch::new(PitchClass::G, Octave(4)),
            ),
            VoicePart::Bass => VoiceRange::new(
                Pitch::new(PitchClass::E, Octave(2)),
                Pitch::new(PitchClass::C, Octave(4)),
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VoiceRange {
    pub low: Pitch,
    pub high: Pitch,
}

impl VoiceRange {
    pub const fn new(low: Pitch, high: Pitch) -> Self {
        VoiceRange { low, high }
    }

    pub fn contains(&self, pitch: Pitch) -> bool {
        (self.low.midi()..=self.high.midi()).contains(&pitch.midi())
    }
}

/// The four simultaneous pitches of one SATB chord realization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Voicing {
    pub soprano: Pitch,
    pub alto: Pitch,
    pub tenor: Pitch,
    pub bass: Pitch,
}

impl Voicing {
    pub const fn new(soprano: Pitch, alto: Pitch, tenor: Pitch, bass: Pitch) -> Self {
        Voicing {
            soprano,
            alto,
            tenor,
            bass,
        }
    }

    pub fn pitch(&self, voice: VoicePart) -> Pitch {
        match voice {
            VoicePart::Soprano => self.soprano,
            VoicePart::Alto => self.alto,
            VoicePart::Tenor => self.tenor,
            VoicePart::Bass => self.bass,
        }
    }
}

/// How two voices move relative to each other between two chords.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionType {
    Contrary,
    Oblique,
    Similar,
}

pub fn classify_motion(prev_a: Pitch, curr_a: Pitch, prev_b: Pitch, curr_b: Pitch) -> MotionType {
    let da = curr_a.midi() - prev_a.midi();
    let db = curr_b.midi() - prev_b.midi();
    if da == 0 || db == 0 {
        MotionType::Oblique
    } else if (da > 0) == (db > 0) {
        MotionType::Similar
    } else {
        MotionType::Contrary
    }
}

/// Voices out of order (a lower voice pitched above the voice meant to
/// sit above it).
pub fn voice_crossings(v: &Voicing) -> Vec<(VoicePart, VoicePart)> {
    VoicePart::adjacent_pairs()
        .into_iter()
        .filter(|&(upper, lower)| v.pitch(upper).midi() < v.pitch(lower).midi())
        .collect()
}

/// A voice moves beyond the pitch an adjacent voice held (or now holds),
/// swapping their relative order across the two chords even without an
/// outright crossing at either instant.
pub fn voice_overlaps(prev: &Voicing, curr: &Voicing) -> Vec<(VoicePart, VoicePart)> {
    VoicePart::adjacent_pairs()
        .into_iter()
        .filter(|&(upper, lower)| {
            curr.pitch(lower).midi() > prev.pitch(upper).midi()
                || curr.pitch(upper).midi() < prev.pitch(lower).midi()
        })
        .collect()
}

pub fn range_violations(v: &Voicing) -> Vec<VoicePart> {
    VoicePart::all()
        .into_iter()
        .filter(|&voice| !voice.default_range().contains(v.pitch(voice)))
        .collect()
}

/// Adjacent upper-voice pairs spread wider than an octave. Tenor-bass
/// spacing is conventionally left unrestricted.
pub fn spacing_violations(v: &Voicing) -> Vec<(VoicePart, VoicePart)> {
    [
        (VoicePart::Soprano, VoicePart::Alto),
        (VoicePart::Alto, VoicePart::Tenor),
    ]
    .into_iter()
    .filter(|&(upper, lower)| v.pitch(upper).midi() - v.pitch(lower).midi() > 12)
    .collect()
}

/// Voice pairs that move into forbidden parallel motion: same voices
/// sound the same perfect-interval class at both chords, both voices
/// actually move, and they move in the same direction.
fn parallel_pairs(
    prev: &Voicing,
    curr: &Voicing,
    is_forbidden_class: impl Fn(&Interval) -> bool,
) -> Vec<(VoicePart, VoicePart)> {
    VoicePart::all_pairs()
        .into_iter()
        .filter(|&(a, b)| {
            let prev_iv = Interval::between(prev.pitch(a), prev.pitch(b));
            let curr_iv = Interval::between(curr.pitch(a), curr.pitch(b));
            if !is_forbidden_class(&prev_iv) || !is_forbidden_class(&curr_iv) {
                return false;
            }
            let moved_a = prev.pitch(a) != curr.pitch(a);
            let moved_b = prev.pitch(b) != curr.pitch(b);
            moved_a
                && moved_b
                && classify_motion(prev.pitch(a), curr.pitch(a), prev.pitch(b), curr.pitch(b))
                    == MotionType::Similar
        })
        .collect()
}

pub fn parallel_fifths(prev: &Voicing, curr: &Voicing) -> Vec<(VoicePart, VoicePart)> {
    parallel_pairs(prev, curr, Interval::is_perfect_fifth_class)
}

pub fn parallel_octaves(prev: &Voicing, curr: &Voicing) -> Vec<(VoicePart, VoicePart)> {
    parallel_pairs(prev, curr, Interval::is_octave_class)
}

pub fn parallel_unisons(prev: &Voicing, curr: &Voicing) -> Vec<(VoicePart, VoicePart)> {
    parallel_pairs(prev, curr, Interval::is_unison)
}

/// Voices that hold the same pitch across both chords.
pub fn common_tone_count(prev: &Voicing, curr: &Voicing) -> u8 {
    VoicePart::all()
        .into_iter()
        .filter(|&v| prev.pitch(v) == curr.pitch(v))
        .count() as u8
}

/// Sum of absolute melodic motion, in semitones, across all four voices.
pub fn total_motion(prev: &Voicing, curr: &Voicing) -> u32 {
    VoicePart::all()
        .into_iter()
        .map(|v| (curr.pitch(v).midi() - prev.pitch(v).midi()).unsigned_abs())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::PitchClass;

    fn p(pc: PitchClass, oct: i32) -> Pitch {
        Pitch::new(pc, Octave(oct))
    }

    #[test]
    fn detects_voice_crossing() {
        // Alto below tenor: crossed.
        let v = Voicing::new(
            p(PitchClass::G, 4),
            p(PitchClass::C, 4),
            p(PitchClass::E, 4),
            p(PitchClass::C, 3),
        );
        assert_eq!(
            voice_crossings(&v),
            vec![(VoicePart::Alto, VoicePart::Tenor)]
        );
    }

    #[test]
    fn detects_parallel_fifths() {
        // Soprano/alto a P5 apart (G4-C4), moving to another P5 (A4-D4),
        // both voices up a step in the same direction.
        let prev = Voicing::new(
            p(PitchClass::G, 4),
            p(PitchClass::C, 4),
            p(PitchClass::E, 3),
            p(PitchClass::C, 3),
        );
        let curr = Voicing::new(
            p(PitchClass::A, 4),
            p(PitchClass::D, 4),
            p(PitchClass::F, 3),
            p(PitchClass::D, 3),
        );
        let fifths = parallel_fifths(&prev, &curr);
        assert!(fifths.contains(&(VoicePart::Soprano, VoicePart::Alto)));
    }

    #[test]
    fn static_fifth_is_not_a_parallel_violation() {
        // Soprano/alto hold the same P5 (no motion) while tenor/bass move;
        // a static interval is not a "parallel" one.
        let prev = Voicing::new(
            p(PitchClass::G, 4),
            p(PitchClass::C, 4),
            p(PitchClass::E, 3),
            p(PitchClass::C, 3),
        );
        let curr = Voicing::new(
            p(PitchClass::G, 4),
            p(PitchClass::C, 4),
            p(PitchClass::F, 3),
            p(PitchClass::D, 3),
        );
        assert!(!parallel_fifths(&prev, &curr).contains(&(VoicePart::Soprano, VoicePart::Alto)));
    }

    #[test]
    fn range_violation_detects_out_of_range_bass() {
        let v = Voicing::new(
            p(PitchClass::C, 5),
            p(PitchClass::G, 4),
            p(PitchClass::E, 3),
            p(PitchClass::C, 6), // absurdly high for bass
        );
        assert_eq!(range_violations(&v), vec![VoicePart::Bass]);
    }

    #[test]
    fn soprano_range_reaches_a5_but_not_b5() {
        // A5 (see PLAN.md): real Bach soprano lines reach it; the
        // default range was widened from G5 to cover it.
        let a5 = Voicing::new(
            p(PitchClass::A, 5),
            p(PitchClass::E, 4),
            p(PitchClass::C, 4),
            p(PitchClass::A, 2),
        );
        assert!(range_violations(&a5).is_empty());

        let b5 = Voicing::new(
            p(PitchClass::B, 5),
            p(PitchClass::E, 4),
            p(PitchClass::C, 4),
            p(PitchClass::A, 2),
        );
        assert_eq!(range_violations(&b5), vec![VoicePart::Soprano]);
    }

    #[test]
    fn common_tone_count_counts_static_voices() {
        let prev = Voicing::new(
            p(PitchClass::C, 5),
            p(PitchClass::G, 4),
            p(PitchClass::E, 3),
            p(PitchClass::C, 3),
        );
        let curr = Voicing::new(
            p(PitchClass::C, 5), // held
            p(PitchClass::A, 4),
            p(PitchClass::F, 3),
            p(PitchClass::C, 3), // held
        );
        assert_eq!(common_tone_count(&prev, &curr), 2);
    }
}
