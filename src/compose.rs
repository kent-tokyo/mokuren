//! The public entry point (AGENTS.md section 17): build a `Composer`,
//! call `.harmonize(melody)`, get back a `HarmonizationResult`.

use crate::diagnostics::Diagnostics;
use crate::error::{MokurenError, Result};
use crate::explain::{Decision, HarmonizationResult};
use crate::generate::CandidateGenerator;
use crate::key::Key;
use crate::melody::{Melody, Position};
use crate::rules::Style;
use crate::search::{BeamSearch, CompositionProblem, SearchStrategy};

/// Voice arrangement. v0.1 only harmonizes SATB (AGENTS.md section 7);
/// the enum exists so a future arrangement doesn't need an API break.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Voices {
    SATB,
}

pub struct Composer {
    key: Key,
    style: Style,
    voices: Voices,
    search: Box<dyn SearchStrategy>,
}

impl Composer {
    pub fn new() -> Self {
        Composer {
            key: Key::C_MAJOR,
            style: Style::CommonPractice,
            voices: Voices::SATB,
            search: Box::new(BeamSearch::new()),
        }
    }

    pub fn key(mut self, key: Key) -> Self {
        self.key = key;
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn voices(mut self, voices: Voices) -> Self {
        self.voices = voices;
        self
    }

    pub fn search(mut self, search: impl SearchStrategy + 'static) -> Self {
        self.search = Box::new(search);
        self
    }

    pub fn harmonize(&self, melody: Melody) -> Result<HarmonizationResult> {
        let Voices::SATB = self.voices;
        if melody.is_empty() {
            return Err(MokurenError::Parse("melody has no notes".to_string()));
        }

        let problem = CompositionProblem {
            melody: &melody,
            key: &self.key,
            style: &self.style,
        };
        let outcome = self.search.search(&problem)?;

        // Rebuild the full evaluated-alternative set at each position from
        // the winning path's actual context. This reuses the exact same
        // `CandidateGenerator::generate` call the search itself used, so
        // `why_not()` never drifts from what the search actually saw; see
        // PLAN.md for why this replay is preferred over threading the
        // full alternative set through the beam search hot loop.
        let generator = CandidateGenerator::new(&self.key, &self.style);
        let mut explain_diagnostics = Diagnostics::default();
        let decisions = outcome
            .path
            .iter()
            .enumerate()
            .map(|(index, (selected, _))| {
                let soprano = melody
                    .pitch_at(Position(index))
                    .expect("index is within melody bounds");
                let is_final = index == melody.len() - 1;
                let previous = if index == 0 {
                    None
                } else {
                    Some(&outcome.path[index - 1].1)
                };
                let previous_rn = if index == 0 {
                    None
                } else {
                    Some(&outcome.path[index - 1].0)
                };
                let evaluated = generator.generate(
                    soprano,
                    previous,
                    previous_rn,
                    is_final,
                    &mut explain_diagnostics,
                );
                Decision::new(Position(index), *selected, evaluated)
            })
            .collect();

        Ok(HarmonizationResult {
            melody,
            key: self.key,
            decisions,
            diagnostics: outcome.diagnostics,
        })
    }
}

impl Default for Composer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::melody::Melody;

    #[test]
    fn harmonizes_the_spine_melody_end_to_end() {
        let melody = Melody::parse("C4 C4 G4 G4 A4 A4 G4").unwrap();
        let result = Composer::new()
            .key(Key::C_MAJOR)
            .style(Style::CommonPractice)
            .harmonize(melody)
            .unwrap();
        assert_eq!(result.decisions.len(), 7);
        assert!(
            result
                .decisions
                .iter()
                .all(|d| d.selected_candidate().is_valid())
        );
    }

    #[test]
    fn rejects_empty_melody() {
        let melody = Melody::new(Vec::new());
        assert!(Composer::new().harmonize(melody).is_err());
    }

    #[test]
    fn soprano_touching_a5_still_harmonizes() {
        // A5 is above the pre-widening soprano ceiling (G5, see
        // src/voice.rs) — 5 real Bach chorales in the v0.1.0 baseline
        // failed outright because a soprano note this high made every
        // candidate fail VoiceRangeRule at that position.
        let melody = Melody::parse("C5 D5 A5 G5").unwrap();
        let result = Composer::new()
            .key(Key::C_MAJOR)
            .style(Style::CommonPractice)
            .harmonize(melody)
            .unwrap();
        assert!(
            result
                .decisions
                .iter()
                .all(|d| d.selected_candidate().is_valid())
        );
    }

    #[test]
    fn chromatic_soprano_only_harmonizable_by_an_applied_dominant() {
        // F#4 isn't in C major's diatonic scale at all — no diatonic
        // chord contains it, so before applied dominants this melody
        // would fail with `NoValidHarmonization`. It's the third of V/V
        // (D-F#-A), the applied dominant of the dominant, which is the
        // only vocabulary that can harmonize this note.
        let melody = Melody::parse("C4 D4 F#4 G4").unwrap();
        let result = Composer::new()
            .key(Key::C_MAJOR)
            .style(Style::CommonPractice)
            .harmonize(melody)
            .unwrap();
        assert_eq!(result.decisions.len(), 4);
        let chromatic_position = result.decisions[2].selected();
        assert!(
            chromatic_position.applied_to.is_some(),
            "expected an applied dominant at the F#4 position, got {chromatic_position}"
        );
    }

    #[test]
    fn held_applied_dominant_tone_resolves_after_prolongation() {
        // G#4 is the leading tone of A (vi in C major), reachable only
        // via V/vi or V7/vi. Held across two consecutive notes before
        // resolving up to A4 — this exact pattern (a chromatic tone
        // tied/repeated before resolving) made a real Bach chorale
        // (Riemenschneider 102, D#5 held across two notes in G major)
        // structurally unharmonizable before
        // SecondaryDominantResolutionRule was taught to allow
        // prolonging the same applied dominant across a repeat instead
        // of demanding resolution at every single position.
        let melody = Melody::parse("E4 G#4 G#4 A4").unwrap();
        let result = Composer::new()
            .key(Key::C_MAJOR)
            .style(Style::CommonPractice)
            .harmonize(melody)
            .unwrap();
        assert!(
            result
                .decisions
                .iter()
                .all(|d| d.selected_candidate().is_valid())
        );
        assert!(result.decisions[1].selected().applied_to.is_some());
        assert!(result.decisions[2].selected().applied_to.is_some());
    }
}
