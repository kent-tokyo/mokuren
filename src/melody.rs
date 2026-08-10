//! Melodic input and the time/position types used to address it.

use crate::error::{MokurenError, Result};
use crate::pitch::Pitch;
use crate::voice::VoicePart;
use std::fmt;

/// Note duration relative to a quarter note. v0.1 only needs enough to
/// support the equal-duration melodies in the spine; a full rational
/// duration type can replace this if irregular rhythms are needed later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Duration {
    Whole,
    Half,
    #[default]
    Quarter,
    Eighth,
    Sixteenth,
}

impl Duration {
    /// Length in quarter-note beats.
    pub fn beats(&self) -> f64 {
        match self {
            Duration::Whole => 4.0,
            Duration::Half => 2.0,
            Duration::Quarter => 1.0,
            Duration::Eighth => 0.5,
            Duration::Sixteenth => 0.25,
        }
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
}
