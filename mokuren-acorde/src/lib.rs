//! Converts mokuren's `HarmonizationResult` into an `acorde_core::Score`
//! — an SATB choir layout (2 staves: treble = soprano+alto, bass =
//! tenor+bass), split into measures under the given meter.
//!
//! mokuren's own `Score`/`Part`/`Note` types (`mokuren::melody`, built by
//! `HarmonizationResult::to_score()`) never depend on acorde. This crate
//! is the only place a mokuren type and an acorde type are named
//! together, so mokuren core stays free of any notation-library
//! dependency, optional or otherwise.

use acorde_core::{
    Clef, KeySignature, Measure, Note as AcordeNote, Part as AcordePart, Pitch as AcordePitch,
    Score as AcordeScore, Staff, Step, TimeSignature,
};
use mokuren::explain::HarmonizationResult;
use mokuren::key::{Key, Mode};
use mokuren::melody::{
    Duration as MokurenDuration, Meter, Note as MokurenNote, Part as MokurenPart,
};
use mokuren::pitch::{Accidental, NoteLetter, Pitch as MokurenPitch};
use mokuren::voice::VoicePart;

pub trait ToAcordeScore {
    /// SATB choir layout: one part, two staves (soprano+alto on treble,
    /// tenor+bass on bass), split into measures under `meter`. A note
    /// that would straddle a measure boundary is pushed whole to the
    /// start of the next measure rather than tie-split — every melody
    /// mokuren currently produces is equal-duration, so this never
    /// triggers today; revisit if that changes.
    fn to_acorde_score(&self, meter: Meter) -> AcordeScore;
}

impl ToAcordeScore for HarmonizationResult {
    fn to_acorde_score(&self, meter: Meter) -> AcordeScore {
        let score = self.to_score(meter);
        let find = |voice| {
            score
                .passage
                .parts
                .iter()
                .find(|p| p.voice == voice)
                .expect("to_score() always produces all four SATB parts")
        };
        let soprano = find(VoicePart::Soprano);
        let alto = find(VoicePart::Alto);
        let tenor = find(VoicePart::Tenor);
        let bass = find(VoicePart::Bass);

        let boundaries = measure_boundaries(&soprano.notes, meter.beats_per_measure as f64);
        let time_sig = TimeSignature {
            numerator: meter.beats_per_measure,
            denominator: time_sig_denominator(meter.beat_unit),
        };
        let key_sig = acorde_key_signature(&score.key);

        let mut part = AcordePart::new("Choir", "Ch.");
        part.staves.push(staff(
            Clef::Treble,
            &boundaries,
            soprano,
            alto,
            &time_sig,
            &key_sig,
        ));
        part.staves.push(staff(
            Clef::Bass,
            &boundaries,
            tenor,
            bass,
            &time_sig,
            &key_sig,
        ));

        let mut acorde_score = AcordeScore::default();
        acorde_score.metadata.title = format!("mokuren: {}", score.key);
        acorde_score.parts = vec![part];
        acorde_score
    }
}

/// Splits a flat note sequence into per-measure `[start, end)` index
/// ranges using each note's own duration. Soprano/alto/tenor/bass always
/// share the same rhythm in `to_score()`'s output, so splitting once
/// (against soprano) and reusing the ranges for all four parts keeps
/// them aligned by construction rather than by assumption.
fn measure_boundaries(notes: &[MokurenNote], beats_per_measure: f64) -> Vec<(usize, usize)> {
    const EPSILON: f64 = 1e-9;
    let mut boundaries = Vec::new();
    let mut start = 0;
    let mut beats_so_far = 0.0;
    for (i, note) in notes.iter().enumerate() {
        let note_beats = note.duration.beats();
        if beats_so_far > EPSILON && beats_so_far + note_beats > beats_per_measure + EPSILON {
            boundaries.push((start, i));
            start = i;
            beats_so_far = 0.0;
        }
        beats_so_far += note_beats;
    }
    if start < notes.len() {
        boundaries.push((start, notes.len()));
    }
    boundaries
}

fn staff(
    clef: Clef,
    boundaries: &[(usize, usize)],
    top: &MokurenPart,
    bottom: &MokurenPart,
    time_sig: &TimeSignature,
    key_sig: &KeySignature,
) -> Staff {
    let mut acorde_staff = Staff::new(clef);
    for (i, &(start, end)) in boundaries.iter().enumerate() {
        let mut measure = Measure::empty(time_sig.numerator, time_sig.denominator);
        measure.number = i as u32 + 1;
        if i == 0 {
            measure.time_sig = Some(time_sig.clone());
            measure.key_sig = Some(key_sig.clone());
        }
        measure.voices[0] = top.notes[start..end].iter().map(acorde_note).collect();
        measure.voices[1] = bottom.notes[start..end].iter().map(acorde_note).collect();
        acorde_staff.measures.push(measure);
    }
    acorde_staff
}

fn acorde_note(note: &MokurenNote) -> AcordeNote {
    let (duration, dot_count) = acorde_duration(note.duration);
    let mut acorde_note = AcordeNote::new(acorde_pitch(note.pitch), duration);
    acorde_note.dot_count = dot_count;
    acorde_note
}

fn acorde_pitch(pitch: MokurenPitch) -> AcordePitch {
    AcordePitch::with_alter(
        acorde_step(pitch.pitch_class.letter),
        pitch.octave.0 as i8,
        accidental_alter(pitch.pitch_class.accidental),
    )
}

fn acorde_step(letter: NoteLetter) -> Step {
    match letter {
        NoteLetter::C => Step::C,
        NoteLetter::D => Step::D,
        NoteLetter::E => Step::E,
        NoteLetter::F => Step::F,
        NoteLetter::G => Step::G,
        NoteLetter::A => Step::A,
        NoteLetter::B => Step::B,
    }
}

fn accidental_alter(accidental: Accidental) -> i8 {
    match accidental {
        Accidental::DoubleFlat => -2,
        Accidental::Flat => -1,
        Accidental::Natural => 0,
        Accidental::Sharp => 1,
        Accidental::DoubleSharp => 2,
    }
}

fn acorde_duration(duration: MokurenDuration) -> (acorde_core::Duration, u8) {
    use acorde_core::Duration as A;
    match duration {
        MokurenDuration::Whole => (A::Whole, 0),
        MokurenDuration::DottedHalf => (A::Half, 1),
        MokurenDuration::Half => (A::Half, 0),
        MokurenDuration::DottedQuarter => (A::Quarter, 1),
        MokurenDuration::Quarter => (A::Quarter, 0),
        MokurenDuration::DottedEighth => (A::Eighth, 1),
        MokurenDuration::Eighth => (A::Eighth, 0),
        MokurenDuration::Sixteenth => (A::Sixteenth, 0),
    }
}

fn time_sig_denominator(beat_unit: MokurenDuration) -> u8 {
    match beat_unit {
        MokurenDuration::Whole => 1,
        MokurenDuration::Half | MokurenDuration::DottedHalf => 2,
        MokurenDuration::Quarter | MokurenDuration::DottedQuarter => 4,
        MokurenDuration::Eighth | MokurenDuration::DottedEighth => 8,
        MokurenDuration::Sixteenth => 16,
    }
}

/// Circle-of-fifths position of each natural letter, C major = 0.
const NATURAL_FIFTHS: [i8; 7] = [0, 2, 4, -1, 1, 3, 5]; // C D E F G A B

fn letter_index(letter: NoteLetter) -> usize {
    match letter {
        NoteLetter::C => 0,
        NoteLetter::D => 1,
        NoteLetter::E => 2,
        NoteLetter::F => 3,
        NoteLetter::G => 4,
        NoteLetter::A => 5,
        NoteLetter::B => 6,
    }
}

/// A minor key's fifths count is always its major-on-the-same-letter
/// count minus 3 (e.g. A major = 3 sharps, A minor = 0; D major = 2
/// sharps, D minor = -1 = one flat) — the standard relative-major
/// relationship expressed directly in circle-of-fifths terms, without
/// needing to spell out the relative major's own tonic.
fn acorde_key_signature(key: &Key) -> KeySignature {
    let major_fifths =
        NATURAL_FIFTHS[letter_index(key.tonic.letter)] + 7 * accidental_alter(key.tonic.accidental);
    let (fifths, mode) = match key.mode {
        Mode::Major => (major_fifths, "major"),
        Mode::Minor => (major_fifths - 3, "minor"),
    };
    KeySignature {
        fifths,
        mode: mode.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mokuren::prelude::*;

    fn harmonize(melody: &str, key: Key) -> HarmonizationResult {
        Composer::new()
            .key(key)
            .style(Style::CommonPractice)
            .search(BeamSearch::new().width(16))
            .harmonize(Melody::parse(melody).unwrap())
            .unwrap()
    }

    #[test]
    fn round_trips_the_default_melody_into_a_valid_acorde_score() {
        let result = harmonize("C4 C4 G4 G4 A4 A4 G4", Key::C_MAJOR);
        let score = result.to_acorde_score(Meter::FOUR_FOUR);

        assert_eq!(score.parts.len(), 1);
        assert_eq!(score.parts[0].staves.len(), 2);
        assert_eq!(score.parts[0].staves[0].clef, Clef::Treble);
        assert_eq!(score.parts[0].staves[1].clef, Clef::Bass);
        // 7 quarter notes in 4/4: measure 1 full (4), measure 2 partial (3).
        assert_eq!(score.parts[0].staves[0].measures.len(), 2);
        assert_eq!(score.parts[0].staves[0].measures[0].voices[0].len(), 4);
        assert_eq!(score.parts[0].staves[0].measures[1].voices[0].len(), 3);

        let report = acorde_core::validate(&score);
        assert!(
            report.errors.is_empty(),
            "expected no structural errors, got {:?}",
            report.errors
        );
    }

    #[test]
    fn key_signature_matches_known_fifths() {
        let major = harmonize("C4 C4 G4 G4 A4 A4 G4", Key::C_MAJOR);
        assert_eq!(
            major.to_acorde_score(Meter::FOUR_FOUR).parts[0].staves[0].measures[0]
                .key_sig
                .as_ref()
                .unwrap()
                .fifths,
            0
        );

        let a_minor = harmonize(
            "A4 A4 E4 E4 F4 F4 E4",
            Key::new("A".parse().unwrap(), Mode::Minor).unwrap(),
        );
        assert_eq!(
            a_minor.to_acorde_score(Meter::FOUR_FOUR).parts[0].staves[0].measures[0]
                .key_sig
                .as_ref()
                .unwrap()
                .fifths,
            0
        );

        // G major: 1 sharp. D minor: 1 flat (relative major F, which is
        // also 1 flat) — both non-zero, unlike the two cases above.
        let g_major = harmonize(
            "D4 D4 A4 A4 B4 B4 A4",
            Key::new("G".parse().unwrap(), Mode::Major).unwrap(),
        );
        assert_eq!(
            g_major.to_acorde_score(Meter::FOUR_FOUR).parts[0].staves[0].measures[0]
                .key_sig
                .as_ref()
                .unwrap()
                .fifths,
            1
        );

        let d_minor = harmonize(
            "D4 D4 A4 A4 Bb4 Bb4 A4",
            Key::new("D".parse().unwrap(), Mode::Minor).unwrap(),
        );
        assert_eq!(
            d_minor.to_acorde_score(Meter::FOUR_FOUR).parts[0].staves[0].measures[0]
                .key_sig
                .as_ref()
                .unwrap()
                .fifths,
            -1
        );
    }
}
