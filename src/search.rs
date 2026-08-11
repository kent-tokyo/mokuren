//! Search over harmonization candidates (AGENTS.md section 12).
//!
//! `SearchStrategy` is the swappable interface; `BeamSearch` is v0.1's
//! only implementation. The beam only ever carries *valid* candidates —
//! rejected ones are still visible in `Diagnostics` and, later, in
//! `explain.rs`'s replay of `CandidateGenerator::generate`, but they
//! never survive into a path.

use crate::chord::RomanNumeral;
use crate::diagnostics::Diagnostics;
use crate::error::{MokurenError, Result};
use crate::generate::CandidateGenerator;
use crate::key::Key;
use crate::melody::{Melody, Position};
use crate::rules::Style;
use crate::voice::Voicing;
use std::cmp::Ordering;

pub struct CompositionProblem<'a> {
    pub melody: &'a Melody,
    pub key: &'a Key,
    pub style: &'a Style,
}

pub struct SearchOutcome {
    pub path: Vec<(RomanNumeral, Voicing)>,
    pub diagnostics: Diagnostics,
}

pub trait SearchStrategy {
    fn search(&self, problem: &CompositionProblem) -> Result<SearchOutcome>;
}

struct BeamEntry {
    path: Vec<(RomanNumeral, Voicing)>,
    cumulative_score: f64,
    cumulative_voice_leading_cost: u32,
}

type RomanNumeralRank = (u8, u8, u8, u8);
type VoicingKey = (i32, i32, i32, i32);

fn path_key(path: &[(RomanNumeral, Voicing)]) -> Vec<(RomanNumeralRank, VoicingKey)> {
    path.iter()
        .map(|(rn, v)| {
            (
                (
                    rn.degree.0,
                    rn.quality as u8,
                    rn.inversion as u8,
                    rn.applied_to().map_or(0, |d| d.0),
                ),
                (
                    v.soprano.midi(),
                    v.alto.midi(),
                    v.tenor.midi(),
                    v.bass.midi(),
                ),
            )
        })
        .collect()
}

/// Deterministic ordering mirroring `generate::compare_candidates`, but
/// over whole paths: cumulative score, then cumulative voice-leading
/// cost, then canonical path ordering.
fn compare_beam_entries(a: &BeamEntry, b: &BeamEntry) -> Ordering {
    b.cumulative_score
        .total_cmp(&a.cumulative_score)
        .then_with(|| {
            a.cumulative_voice_leading_cost
                .cmp(&b.cumulative_voice_leading_cost)
        })
        .then_with(|| path_key(&a.path).cmp(&path_key(&b.path)))
}

/// Beam search with a fixed width, keeping only valid candidates at each
/// step (AGENTS.md section 8: hard constraints exclude from search).
pub struct BeamSearch {
    width: usize,
}

impl BeamSearch {
    /// Beam search only rewards a strong cadence at the very last
    /// position, so a narrow beam can prune away the path that would
    /// have resolved best before it ever reaches that reward (a known
    /// horizon effect of greedy beam search — see PLAN.md). 32 is wide
    /// enough to avoid that for melodies of the length v0.1 targets;
    /// widen it (`.width(n)`) for longer melodies or a richer harmonic
    /// vocabulary.
    pub fn new() -> Self {
        BeamSearch { width: 32 }
    }

    pub fn width(mut self, width: usize) -> Self {
        self.width = width.max(1);
        self
    }
}

impl Default for BeamSearch {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchStrategy for BeamSearch {
    fn search(&self, problem: &CompositionProblem) -> Result<SearchOutcome> {
        let generator = CandidateGenerator::new(problem.key, problem.style);
        let mut diagnostics = Diagnostics::default();
        let mut beam = vec![BeamEntry {
            path: Vec::new(),
            cumulative_score: 0.0,
            cumulative_voice_leading_cost: 0,
        }];

        for index in 0..problem.melody.len() {
            let soprano = problem
                .melody
                .pitch_at(Position(index))
                .expect("index is within melody bounds by construction of the loop range");
            let is_final = index == problem.melody.len() - 1;

            let mut next_beam = Vec::new();
            for entry in &beam {
                let previous = entry.path.last().map(|(_, v)| v);
                let previous_rn = entry.path.last().map(|(rn, _)| rn);
                let candidates =
                    generator.generate(soprano, previous, previous_rn, is_final, &mut diagnostics);
                for candidate in candidates.into_iter().filter(|c| c.is_valid()) {
                    let mut path = entry.path.clone();
                    path.push((candidate.roman_numeral, candidate.voicing));
                    next_beam.push(BeamEntry {
                        path,
                        cumulative_score: entry.cumulative_score + candidate.score.total(),
                        cumulative_voice_leading_cost: entry.cumulative_voice_leading_cost
                            + candidate.voice_leading_cost,
                    });
                }
            }

            if next_beam.is_empty() {
                return Err(MokurenError::NoValidHarmonization);
            }
            next_beam.sort_by(compare_beam_entries);
            next_beam.truncate(self.width);
            beam = next_beam;
        }

        let best = beam
            .into_iter()
            .next()
            .ok_or(MokurenError::NoValidHarmonization)?;
        Ok(SearchOutcome {
            path: best.path,
            diagnostics,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::melody::Melody;

    #[test]
    fn finds_a_valid_full_path_for_the_spine_melody() {
        let melody = Melody::parse("C4 C4 G4 G4 A4 A4 G4").unwrap();
        let key = Key::C_MAJOR;
        let style = Style::CommonPractice;
        let problem = CompositionProblem {
            melody: &melody,
            key: &key,
            style: &style,
        };
        let outcome = BeamSearch::new().width(8).search(&problem).unwrap();
        assert_eq!(outcome.path.len(), melody.len());
        assert!(outcome.diagnostics.candidates_generated > 0);
    }

    #[test]
    fn search_is_deterministic_across_runs() {
        let melody = Melody::parse("C4 C4 G4 G4 A4 A4 G4").unwrap();
        let key = Key::C_MAJOR;
        let style = Style::CommonPractice;
        let problem = CompositionProblem {
            melody: &melody,
            key: &key,
            style: &style,
        };
        let a = BeamSearch::new().width(8).search(&problem).unwrap();
        let b = BeamSearch::new().width(8).search(&problem).unwrap();
        let numerals_a: Vec<_> = a.path.iter().map(|(rn, _)| rn.to_string()).collect();
        let numerals_b: Vec<_> = b.path.iter().map(|(rn, _)| rn.to_string()).collect();
        assert_eq!(numerals_a, numerals_b);
    }
}
