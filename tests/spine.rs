//! End-to-end test of the v0.1 vertical slice (AGENTS.md section 1):
//! the exact melody/key/meter from the spec, harmonized and explained
//! through the public API only.

use mokuren::prelude::*;

fn harmonize_spine() -> HarmonizationResult {
    let melody = Melody::parse("C4 C4 G4 G4 A4 A4 G4").unwrap();
    Composer::new()
        .key(Key::C_MAJOR)
        .style(Style::CommonPractice)
        .voices(Voices::SATB)
        .search(BeamSearch::new().width(32))
        .harmonize(melody)
        .unwrap()
}

#[test]
fn harmonizes_every_position_with_a_valid_candidate() {
    let result = harmonize_spine();
    assert_eq!(result.decisions.len(), 7);
    for decision in &result.decisions {
        assert!(decision.selected_candidate().is_valid());
    }
}

#[test]
fn ends_on_tonic_with_a_recognized_cadence() {
    use mokuren::score::{Cadence, Reason};

    let result = harmonize_spine();
    let last = result.decisions.last().unwrap();
    // The melody's last two soprano notes are A4 then G4; no diatonic
    // dominant-function chord in C major contains A, so a V/V7 -> I
    // authentic cadence is not reachable for *this* melody (only a
    // predominant -> I plagal close is). Assert what v0.1 actually
    // promises: the phrase closes on tonic, and CadenceSupportRule
    // recognized the close as a cadence, not that it's specifically
    // authentic (AGENTS.md section 24: don't treat one expected
    // progression as ground truth).
    assert_eq!(last.selected().degree, mokuren::key::ScaleDegree::TONIC);
    let cadence = last
        .selected_candidate()
        .reasons
        .iter()
        .find_map(|r| match r {
            Reason::CadenceSupport { cadence, .. } => Some(*cadence),
            _ => None,
        });
    assert!(
        matches!(cadence, Some(Cadence::Authentic) | Some(Cadence::Plagal)),
        "expected an authentic or plagal cadence at the final position, got {cadence:?}"
    );
}

#[test]
fn explain_mentions_every_position_and_the_final_progression() {
    let result = harmonize_spine();
    let text = result.explain();
    assert!(text.contains("Position 0:"));
    assert!(text.contains("Position 6:"));
    assert!(text.contains("Progression:"));
}

#[test]
fn why_reports_the_selected_numeral_and_a_final_score() {
    let result = harmonize_spine();
    let decision = &result.decisions[2];
    let text = result.why(Position::new(2)).unwrap();
    assert!(text.starts_with(&format!("Why {}?", decision.selected())));
    assert!(text.contains("Final local score:"));
}

#[test]
fn why_not_distinguishes_a_valid_alternative_from_the_selected_candidate() {
    let result = harmonize_spine();
    let decision = &result.decisions[2];
    let alternative = decision
        .alternatives()
        .find(|c| c.is_valid())
        .expect("at least one valid alternative")
        .roman_numeral;

    let text = result.why_not(Position::new(2), alternative).unwrap();
    assert!(text.contains("was valid and ranked #"));
    assert!(text.contains("Difference from selected"));
}

#[test]
fn why_not_reports_an_unevaluated_alternative_as_an_error() {
    let result = harmonize_spine();
    // Third inversion of a plain triad is never part of the generated
    // vocabulary (only seventh chords get a third inversion), so this is
    // guaranteed to be absent from every position's evaluated set.
    let outside_vocabulary = RomanNumeral::V.with_inversion(ChordInversion::Third);
    assert!(
        result
            .why_not(Position::new(0), outside_vocabulary)
            .is_err()
    );
}

#[test]
fn diagnostics_account_for_every_generated_candidate() {
    let result = harmonize_spine();
    let diagnostics = result.diagnostics();
    assert_eq!(
        diagnostics.candidates_generated,
        diagnostics.candidates_retained + diagnostics.candidates_rejected
    );
    assert!(diagnostics.candidates_generated > 0);
    assert!(!diagnostics.top_rejection_reasons(5).is_empty());
}

#[test]
fn search_is_deterministic_across_repeated_runs() {
    let a = harmonize_spine();
    let b = harmonize_spine();
    assert_eq!(a.progression(), b.progression());
}
