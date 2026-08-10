//! Golden tests (AGENTS.md section 22): small, fixed SATB passages with
//! a hand-verified expected valid/invalid outcome and expected major
//! violation — checked against the rule engine directly (not against
//! the search's own output), so these pin music-theoretic correctness
//! independent of scoring or beam-search behavior.
//!
//! Every voicing below was worked out by hand against real voice-leading
//! rules before being encoded here; one early draft of the valid
//! progression's final chord turned out to contain a genuine parallel
//! fifth (tenor/bass, V7 -> I) and had to be re-voiced — a reminder of
//! why this category of test earns its keep even in a small engine.

use mokuren::chord::{Chord, ChordInversion, RomanNumeral};
use mokuren::key::Key;
use mokuren::pitch::{Octave, Pitch, PitchClass};
use mokuren::rules::{RuleContext, RuleId, RuleStatus, Style};
use mokuren::voice::Voicing;

fn p(pc: PitchClass, octave: i32) -> Pitch {
    Pitch::new(pc, Octave(octave))
}

fn voicing(
    s: (PitchClass, i32),
    a: (PitchClass, i32),
    t: (PitchClass, i32),
    b: (PitchClass, i32),
) -> Voicing {
    Voicing::new(p(s.0, s.1), p(a.0, a.1), p(t.0, t.1), p(b.0, b.1))
}

/// Every hard-rule verdict for one chord transition, in the engine's
/// own rule order — the same evaluation `CandidateGenerator` runs
/// internally, exposed here for direct assertions.
fn hard_violations(
    key: &Key,
    previous: Option<&Voicing>,
    previous_roman_numeral: Option<&RomanNumeral>,
    current: &Voicing,
    roman_numeral: &RomanNumeral,
    is_final_position: bool,
) -> Vec<RuleId> {
    let chord = roman_numeral.to_chord(key);
    let previous_chord: Option<Chord> = previous_roman_numeral.map(|rn| rn.to_chord(key));
    let ctx = RuleContext {
        key,
        previous,
        previous_chord: previous_chord.as_ref(),
        previous_roman_numeral,
        current,
        chord: &chord,
        roman_numeral,
        is_final_position,
    };
    Style::CommonPractice
        .rules()
        .into_iter()
        .filter(|rule| rule.evaluate(&ctx).status == RuleStatus::Violation)
        .map(|rule| rule.id())
        .collect()
}

/// I - IV - I6 - V7 - I in C major (AGENTS.md section 1's own example
/// progression, spelled out as a hand-checked SATB realization):
/// stepwise/common-tone voice leading throughout, correct leading-tone
/// and chordal-seventh resolution into the final cadence, no parallel
/// 5ths/8ves, no crossing, no overlap, no range violations.
#[test]
fn textbook_progression_is_fully_valid_at_every_transition() {
    use PitchClass as Pc;
    let key = Key::C_MAJOR;

    let positions: [(RomanNumeral, Voicing); 5] = [
        (
            RomanNumeral::I,
            voicing((Pc::C, 5), (Pc::E, 4), (Pc::G, 3), (Pc::C, 3)),
        ),
        (
            RomanNumeral::IV,
            voicing((Pc::C, 5), (Pc::F, 4), (Pc::A, 3), (Pc::F, 3)),
        ),
        (
            RomanNumeral::I.with_inversion(ChordInversion::First),
            voicing((Pc::C, 5), (Pc::G, 4), (Pc::E, 4), (Pc::E, 3)),
        ),
        (
            RomanNumeral::V7,
            voicing((Pc::D, 5), (Pc::B, 4), (Pc::F, 4), (Pc::G, 3)),
        ),
        (
            RomanNumeral::I,
            voicing((Pc::G, 5), (Pc::C, 5), (Pc::E, 4), (Pc::C, 3)),
        ),
    ];

    let mut previous: Option<(&RomanNumeral, &Voicing)> = None;
    for (index, (rn, v)) in positions.iter().enumerate() {
        let is_final = index == positions.len() - 1;
        let violations = hard_violations(
            &key,
            previous.map(|(_, v)| v),
            previous.map(|(rn, _)| rn),
            v,
            rn,
            is_final,
        );
        assert!(
            violations.is_empty(),
            "position {index} ({rn}) unexpectedly violated: {violations:?}"
        );
        previous = Some((rn, v));
    }
}

/// The classic textbook counter-example: I -> ii in root position with
/// every voice moving up by step in similar ("parallel") motion. This
/// is the single most-cited forbidden progression in Common Practice
/// pedagogy — often shown, as here, moving the whole triad in lockstep
/// specifically because it demonstrates more than one forbidden
/// parallel at once: soprano/alto keep a perfect fifth (G-C -> A-D) and
/// alto/bass keep a perfect octave (C-C -> D-D), both by similar
/// motion.
#[test]
fn parallel_motion_triad_shift_is_rejected_for_both_fifths_and_octaves() {
    use PitchClass as Pc;
    let key = Key::C_MAJOR;

    let previous = voicing((Pc::G, 4), (Pc::C, 4), (Pc::E, 3), (Pc::C, 3));
    let current = voicing((Pc::A, 4), (Pc::D, 4), (Pc::F, 3), (Pc::D, 3));

    let violations = hard_violations(
        &key,
        Some(&previous),
        Some(&RomanNumeral::I),
        &current,
        &RomanNumeral::II,
        false,
    );

    assert!(
        violations.contains(&RuleId::ParallelFifths),
        "expected a parallel-fifths violation, got {violations:?}"
    );
    assert!(
        violations.contains(&RuleId::ParallelOctaves),
        "expected a parallel-octaves violation too (alto/bass), got {violations:?}"
    );
}

/// A root-position tonic voiced with the melody note in an octave the
/// soprano can't reach — a hard range violation, independent of any
/// voice-leading context (no previous chord needed to detect it).
#[test]
fn out_of_range_soprano_is_rejected() {
    use PitchClass as Pc;
    let key = Key::C_MAJOR;

    // C6 is above the default soprano ceiling (G5).
    let current = voicing((Pc::C, 6), (Pc::E, 4), (Pc::G, 3), (Pc::C, 3));

    let violations = hard_violations(&key, None, None, &current, &RomanNumeral::I, false);

    assert!(
        violations.contains(&RuleId::VoiceRange),
        "expected a voice-range violation, got {violations:?}"
    );
}
