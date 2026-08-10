//! Melodic input and the time/position types used to address it.

use crate::error::{MokurenError, Result};
use crate::pitch::Pitch;
use crate::voice::VoicePart;
use std::fmt;

/// Note duration relative to a quarter note. Extended (2026-08-10) with
/// dotted variants after the chorale benchmark's real rhythm data — not
/// the synthetic equal-duration spine melody — showed dotted quarters
/// and dotted halves are common (chorale phrase-ending fermatas in
/// particular). Still a closed enum, not a general rational duration:
/// nothing in this crate needs tuplets, and Common Practice chorale
/// writing doesn't produce them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Duration {
    Whole,
    DottedHalf,
    Half,
    DottedQuarter,
    #[default]
    Quarter,
    DottedEighth,
    Eighth,
    Sixteenth,
}

impl Duration {
    /// Length in quarter-note beats.
    pub fn beats(&self) -> f64 {
        match self {
            Duration::Whole => 4.0,
            Duration::DottedHalf => 3.0,
            Duration::Half => 2.0,
            Duration::DottedQuarter => 1.5,
            Duration::Quarter => 1.0,
            Duration::DottedEighth => 0.75,
            Duration::Eighth => 0.5,
            Duration::Sixteenth => 0.25,
        }
    }

    /// The `Duration` whose `beats()` exactly matches, if any — used to
    /// parse an external duration (e.g. a chorale benchmark fixture's
    /// note-value fraction) back into this closed vocabulary.
    pub fn from_beats(beats: f64) -> Option<Duration> {
        const EPSILON: f64 = 1e-9;
        [
            Duration::Whole,
            Duration::DottedHalf,
            Duration::Half,
            Duration::DottedQuarter,
            Duration::Quarter,
            Duration::DottedEighth,
            Duration::Eighth,
            Duration::Sixteenth,
        ]
        .into_iter()
        .find(|d| (d.beats() - beats).abs() < EPSILON)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Note {
    pub pitch: Pitch,
    pub duration: Duration,
}

impl Note {
    pub const fn new(pitch: Pitch, duration: Duration) -> Self {
        Note { pitch, duration }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rest {
    pub duration: Duration,
}

/// An index into a melody / harmonization sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Position(pub usize);

impl Position {
    pub const fn new(index: usize) -> Self {
        Position(index)
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Meter {
    pub beats_per_measure: u8,
    pub beat_unit: Duration,
}

impl Meter {
    pub const FOUR_FOUR: Meter = Meter {
        beats_per_measure: 4,
        beat_unit: Duration::Quarter,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Measure(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Beat(pub u32);

/// A monophonic sequence of notes — the fixed soprano line mokuren
/// harmonizes in the v0.1 spine.
#[derive(Debug, Clone, PartialEq)]
pub struct Melody {
    pub notes: Vec<Note>,
}

impl Melody {
    pub fn new(notes: Vec<Note>) -> Self {
        Melody { notes }
    }

    pub fn len(&self) -> usize {
        self.notes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }

    pub fn pitch_at(&self, position: Position) -> Option<Pitch> {
        self.notes.get(position.0).map(|n| n.pitch)
    }

    /// Parses a whitespace-separated list of pitches (e.g. `"C4 C4 G4"`),
    /// each becoming a quarter note. A boundary parser: never panics,
    /// always returns `Result`.
    pub fn parse(s: &str) -> Result<Melody> {
        let notes = s
            .split_whitespace()
            .map(|tok| Ok(Note::new(tok.parse()?, Duration::Quarter)))
            .collect::<Result<Vec<Note>>>()?;
        if notes.is_empty() {
            return Err(MokurenError::Parse("melody has no notes".to_string()));
        }
        Ok(Melody::new(notes))
    }

    /// The (measure, beat) location of a position under a given meter.
    /// Beat is truncated to a whole beat — adequate while all v0.1 input
    /// melodies are equal-duration.
    pub fn measure_and_beat(&self, position: Position, meter: &Meter) -> (Measure, Beat) {
        let end = position.0.min(self.notes.len());
        let cumulative: f64 = self.notes[..end].iter().map(|n| n.duration.beats()).sum();
        let beats_per_measure = meter.beats_per_measure as f64;
        let measure = (cumulative / beats_per_measure).floor() as u32;
        let beat = cumulative.rem_euclid(beats_per_measure);
        (Measure(measure), Beat(beat as u32))
    }
}

/// One event in a raw input line: either a sounding note or a rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MelodyEvent {
    Note(Note),
    Rest(Rest),
}

/// A monophonic line that may contain rests — the raw shape of an input
/// melody before harmonization. `Melody` itself (and everything
/// downstream: `Composer::harmonize`, search, explain) stays a plain
/// `Vec<Note>` with no rest variant, so a rest is resolved *before*
/// reaching that API rather than threaded through it: `phrases()` splits
/// a `MelodyLine` at each rest into contiguous runs of notes, matching
/// how a breath rest in a chorale actually functions — a phrase boundary,
/// not a gap inside one continuous harmonic idea.
#[derive(Debug, Clone, PartialEq)]
pub struct MelodyLine {
    pub events: Vec<MelodyEvent>,
}

impl MelodyLine {
    pub fn new(events: Vec<MelodyEvent>) -> Self {
        MelodyLine { events }
    }

    /// Splits into contiguous note runs at each rest. Leading, trailing,
    /// and consecutive rests never produce an empty phrase. A rest-free
    /// line always yields exactly one phrase equal to its own notes.
    pub fn phrases(&self) -> Vec<Melody> {
        let mut phrases = Vec::new();
        let mut current = Vec::new();
        for event in &self.events {
            match event {
                MelodyEvent::Note(note) => current.push(*note),
                MelodyEvent::Rest(_) => {
                    if !current.is_empty() {
                        phrases.push(Melody::new(std::mem::take(&mut current)));
                    }
                }
            }
        }
        if !current.is_empty() {
            phrases.push(Melody::new(current));
        }
        phrases
    }
}

/// One voice's line within a `Passage` — e.g. the alto line of a
/// harmonization result.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub voice: VoicePart,
    pub notes: Vec<Note>,
}

/// A set of parts sounding together, such as an SATB realization.
#[derive(Debug, Clone, PartialEq)]
pub struct Passage {
    pub parts: Vec<Part>,
}

/// A complete piece: key, meter, and the passage that realizes it.
#[derive(Debug, Clone, PartialEq)]
pub struct Score {
    pub key: crate::key::Key,
    pub meter: Meter,
    pub passage: Passage,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_spine_melody() {
        let m = Melody::parse("C4 C4 G4 G4 A4 A4 G4").unwrap();
        assert_eq!(m.len(), 7);
        assert_eq!(m.notes[0].duration, Duration::Quarter);
    }

    #[test]
    fn rejects_empty_and_garbage_input() {
        assert!(Melody::parse("").is_err());
        assert!(Melody::parse("   ").is_err());
        assert!(Melody::parse("C4 H9").is_err());
    }

    #[test]
    fn measure_and_beat_for_four_four_quarter_notes() {
        let m = Melody::parse("C4 C4 G4 G4 A4 A4 G4").unwrap();
        assert_eq!(
            m.measure_and_beat(Position(0), &Meter::FOUR_FOUR),
            (Measure(0), Beat(0))
        );
        assert_eq!(
            m.measure_and_beat(Position(4), &Meter::FOUR_FOUR),
            (Measure(1), Beat(0))
        );
        assert_eq!(
            m.measure_and_beat(Position(6), &Meter::FOUR_FOUR),
            (Measure(1), Beat(2))
        );
    }

    #[test]
    fn duration_from_beats_round_trips_and_rejects_unrepresentable_values() {
        for d in [
            Duration::Whole,
            Duration::DottedHalf,
            Duration::Half,
            Duration::DottedQuarter,
            Duration::Quarter,
            Duration::DottedEighth,
            Duration::Eighth,
            Duration::Sixteenth,
        ] {
            assert_eq!(Duration::from_beats(d.beats()), Some(d));
        }
        assert_eq!(Duration::from_beats(1.0 / 3.0), None); // a triplet: not representable
    }

    #[test]
    fn rest_free_line_yields_exactly_one_phrase_equal_to_its_notes() {
        let notes = Melody::parse("C4 C4 G4 G4 A4 A4 G4").unwrap().notes;
        let line = MelodyLine::new(notes.iter().copied().map(MelodyEvent::Note).collect());
        let phrases = line.phrases();
        assert_eq!(phrases, vec![Melody::new(notes)]);
    }

    #[test]
    fn a_rest_splits_the_line_into_two_phrases() {
        let n = |p: &str| Note::new(p.parse().unwrap(), Duration::Quarter);
        let r = MelodyEvent::Rest(Rest {
            duration: Duration::Quarter,
        });
        let line = MelodyLine::new(vec![
            MelodyEvent::Note(n("C4")),
            MelodyEvent::Note(n("D4")),
            r,
            MelodyEvent::Note(n("E4")),
        ]);
        assert_eq!(
            line.phrases(),
            vec![
                Melody::new(vec![n("C4"), n("D4")]),
                Melody::new(vec![n("E4")]),
            ]
        );
    }

    #[test]
    fn leading_trailing_and_consecutive_rests_produce_no_empty_phrase() {
        let n = |p: &str| Note::new(p.parse().unwrap(), Duration::Quarter);
        let r = MelodyEvent::Rest(Rest {
            duration: Duration::Quarter,
        });
        let line = MelodyLine::new(vec![
            r,
            r,
            MelodyEvent::Note(n("C4")),
            r,
            r,
            MelodyEvent::Note(n("D4")),
            r,
        ]);
        assert_eq!(
            line.phrases(),
            vec![Melody::new(vec![n("C4")]), Melody::new(vec![n("D4")])]
        );
    }
}
