//! Explainability (AGENTS.md sections 2-3): `why()` and `why_not()` are
//! first-class, reading directly from the same `EvaluatedCandidate` data
//! search already produced — never re-derived after the fact.

use crate::chord::RomanNumeral;
use crate::diagnostics::Diagnostics;
use crate::error::{MokurenError, Result};
use crate::generate::{CandidateStatus, EvaluatedCandidate, compare_candidates};
use crate::key::Key;
use crate::melody::{Melody, Meter, Note, Part, Passage, Position, Score};
use crate::voice::VoicePart;
use std::fmt::Write as _;

/// Everything evaluated at one position, plus which candidate was chosen.
/// Mirrors AGENTS.md section 2.1's `Decision { selected, alternatives,
/// reasons }`: `alternatives()` is every other evaluated candidate, and
/// `reasons` live on the selected candidate itself.
///
/// Fields are private and only constructible from within the crate
/// (via `Composer::harmonize`), so `selected` is always guaranteed to
/// be present in `evaluated` — `selected_candidate()` can't be made to
/// panic by handing it a hand-built, inconsistent `Decision`.
#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    position: Position,
    selected: RomanNumeral,
    evaluated: Vec<EvaluatedCandidate>,
}

impl Decision {
    pub(crate) fn new(
        position: Position,
        selected: RomanNumeral,
        evaluated: Vec<EvaluatedCandidate>,
    ) -> Self {
        Decision {
            position,
            selected,
            evaluated,
        }
    }

    pub fn position(&self) -> Position {
        self.position
    }

    pub fn selected(&self) -> RomanNumeral {
        self.selected
    }

    /// The full evaluated candidate set at this position, selected
    /// candidate included — not just what survived the beam.
    pub fn evaluated(&self) -> &[EvaluatedCandidate] {
        &self.evaluated
    }

    pub fn selected_candidate(&self) -> &EvaluatedCandidate {
        self.evaluated
            .iter()
            .find(|c| c.roman_numeral == self.selected)
            .expect("the selected candidate is always part of its own evaluated set")
    }

    pub fn alternatives(&self) -> impl Iterator<Item = &EvaluatedCandidate> {
        let selected = self.selected;
        self.evaluated
            .iter()
            .filter(move |c| c.roman_numeral != selected)
    }
}

/// The outcome of harmonizing a melody: the chosen progression, every
/// alternative considered at each step, and search-wide diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct HarmonizationResult {
    pub melody: Melody,
    pub key: Key,
    pub decisions: Vec<Decision>,
    pub diagnostics: Diagnostics,
}

impl HarmonizationResult {
    pub fn progression(&self) -> Vec<RomanNumeral> {
        self.decisions.iter().map(|d| d.selected).collect()
    }

    pub fn decision_at(&self, position: Position) -> Result<&Decision> {
        self.decisions
            .get(position.0)
            .ok_or(MokurenError::UnknownPosition(position.0))
    }

    pub fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }

    /// A one-voicing-per-position `Score`, reusing the input melody's
    /// rhythm for every voice.
    pub fn to_score(&self, meter: Meter) -> Score {
        let parts = VoicePart::all()
            .into_iter()
            .map(|voice| {
                let notes = self
                    .decisions
                    .iter()
                    .zip(&self.melody.notes)
                    .map(|(d, melody_note)| {
                        Note::new(
                            d.selected_candidate().voicing.pitch(voice),
                            melody_note.duration,
                        )
                    })
                    .collect();
                Part { voice, notes }
            })
            .collect();
        Score {
            key: self.key,
            meter,
            passage: Passage { parts },
        }
    }

    /// A full overview: the chosen progression and every position's local
    /// score total.
    pub fn explain(&self) -> String {
        let mut out = format!("Harmonization in {}\n\n", self.key);
        for decision in &self.decisions {
            let candidate = decision.selected_candidate();
            let _ = writeln!(
                out,
                "Position {}: {} (score {:+.2})",
                decision.position,
                decision.selected,
                candidate.score.total()
            );
        }
        let progression = self
            .progression()
            .iter()
            .map(RomanNumeral::to_string)
            .collect::<Vec<_>>()
            .join(" - ");
        let _ = write!(out, "\nProgression: {progression}");
        out
    }

    /// Why the selected chord was chosen at `position`.
    pub fn why(&self, position: Position) -> Result<String> {
        let decision = self.decision_at(position)?;
        let candidate = decision.selected_candidate();
        let mut out = format!("Why {}?\n\n", decision.selected);
        for reason in &candidate.reasons {
            let _ = writeln!(out, "{reason}");
        }
        let _ = write!(out, "\nFinal local score: {:.2}", candidate.score.total());
        Ok(out)
    }

    /// Why `alternative` was *not* chosen at `position`, even though it
    /// may have been a legal candidate.
    pub fn why_not(&self, position: Position, alternative: RomanNumeral) -> Result<String> {
        let decision = self.decision_at(position)?;
        let candidate = decision
            .evaluated
            .iter()
            .find(|c| c.roman_numeral == alternative)
            .ok_or_else(|| MokurenError::UnknownAlternative(alternative.to_string()))?;
        let selected = decision.selected_candidate();

        let mut out = format!("Why not {alternative}?\n\n");
        match &candidate.status {
            CandidateStatus::Valid => {
                let mut valid: Vec<&EvaluatedCandidate> =
                    decision.evaluated.iter().filter(|c| c.is_valid()).collect();
                valid.sort_by(|a, b| compare_candidates(a, b));
                let rank = valid
                    .iter()
                    .position(|c| c.roman_numeral == alternative)
                    .map(|i| i + 1);
                if let Some(rank) = rank {
                    let _ = writeln!(out, "{alternative} was valid and ranked #{rank}.\n");
                }
            }
            CandidateStatus::Rejected(rules) => {
                let names: Vec<String> =
                    rules.iter().map(crate::rules::RuleId::to_string).collect();
                let _ = writeln!(out, "{alternative} was rejected: {}.\n", names.join(", "));
            }
        }
        for reason in &candidate.reasons {
            let _ = writeln!(out, "{reason}");
        }
        let _ = writeln!(out, "\nFinal local score: {:.2}", candidate.score.total());
        let _ = write!(
            out,
            "Difference from selected {}: {:+.2}",
            selected.roman_numeral,
            candidate.score.total() - selected.score.total()
        );
        Ok(out)
    }
}
