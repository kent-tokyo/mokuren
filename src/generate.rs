//! Candidate generation and evaluation (AGENTS.md section 11).
//!
//! `CandidateGenerator::generate` produces the *full* evaluated
//! alternative set at one position — every diatonic harmonic candidate
//! whose chord contains the given soprano note, each paired with its
//! best-scoring voicing under the given previous-chord context. Search
//! (`search.rs`) filters this down to valid candidates and expands the
//! beam; explanation (`explain.rs`) replays this same call against the
//! winning path's actual context, so `why_not()` always answers from the
//! same evaluation the search itself used — no separate bookkeeping to
//! keep in sync.

use crate::chord::{Chord, ChordInversion, RomanNumeral};
use crate::diagnostics::Diagnostics;
use crate::key::{Key, Mode};
use crate::pitch::{Octave, Pitch, PitchClass};
use crate::rules::{Rule, RuleContext, RuleId, RuleStatus, Style};
use crate::score::{Penalty, Reason, ScoreBreakdown};
use crate::voice::{self, VoicePart, VoiceRange, Voicing};
use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq)]
pub enum CandidateStatus {
    Valid,
    Rejected(Vec<RuleId>),
}

/// One harmonic candidate, evaluated: its best voicing under context,
/// whether it's usable by search, and the structured reasons behind its
/// score.
#[derive(Debug, Clone, PartialEq)]
pub struct EvaluatedCandidate {
    pub roman_numeral: RomanNumeral,
    pub chord: Chord,
    pub voicing: Voicing,
    pub status: CandidateStatus,
    pub score: ScoreBreakdown,
    pub reasons: Vec<Reason>,
    /// Total melodic motion (semitones, summed over all 4 voices) from
    /// the previous voicing; 0 at the first position. Used only as a
    /// deterministic tie-break, not folded into `score`.
    pub voice_leading_cost: u32,
}

impl EvaluatedCandidate {
    pub fn is_valid(&self) -> bool {
        matches!(self.status, CandidateStatus::Valid)
    }
}

/// Deterministic tie-break chain (AGENTS.md section 16): valid before
/// rejected, then total score, then voice-leading cost, then canonical
/// Roman-numeral and voicing ordering. Never touches `f64::partial_cmp`.
pub fn compare_candidates(a: &EvaluatedCandidate, b: &EvaluatedCandidate) -> Ordering {
    status_rank(a)
        .cmp(&status_rank(b))
        .then_with(|| b.score.total().total_cmp(&a.score.total()))
        .then_with(|| a.voice_leading_cost.cmp(&b.voice_leading_cost))
        .then_with(|| canonical_rank(&a.roman_numeral).cmp(&canonical_rank(&b.roman_numeral)))
        .then_with(|| voicing_key(&a.voicing).cmp(&voicing_key(&b.voicing)))
}

fn status_rank(c: &EvaluatedCandidate) -> u8 {
    if c.is_valid() { 0 } else { 1 }
}

fn canonical_rank(rn: &RomanNumeral) -> (u8, u8, u8, u8) {
    // `applied_to` breaks ties among applied dominants, which all share
    // `degree == ScaleDegree::DOMINANT` (V/ii, V/iii, ... would otherwise
    // collide in root position, silently falling through to `voicing_key`
    // instead of the documented deterministic Roman-numeral ordering).
    (
        rn.degree.0,
        rn.quality as u8,
        rn.inversion as u8,
        rn.applied_to().map_or(0, |d| d.0),
    )
}

fn voicing_key(v: &Voicing) -> (i32, i32, i32, i32) {
    (
        v.soprano.midi(),
        v.alto.midi(),
        v.tenor.midi(),
        v.bass.midi(),
    )
}

/// The mode determines which base vocabulary applies. Major: the
/// diatonic set plus applied dominants for x in {ii, iii, IV, V, vi}.
/// Minor: the natural-diatonic set plus three chromatic layers
/// (`chord.rs`'s doc comments explain why these are layers rather than
/// a different `Mode`) — the harmonic-minor-altered V/V7/vii°, applied
/// dominants for x in {ii, IV, V, vi} (V/III excluded — real corpus
/// data found no minor chorale needing it), and the melodic-minor
/// raised-6th ii/IV.
fn harmonic_vocabulary(mode: Mode) -> Vec<RomanNumeral> {
    let mut out = Vec::new();
    let numerals: Vec<RomanNumeral> = match mode {
        Mode::Major => RomanNumeral::diatonic_vocabulary()
            .into_iter()
            .chain(RomanNumeral::applied_dominant_vocabulary())
            .collect(),
        Mode::Minor => RomanNumeral::natural_minor_vocabulary()
            .into_iter()
            .chain(RomanNumeral::harmonic_minor_vocabulary())
            .chain(RomanNumeral::minor_applied_dominant_vocabulary())
            .chain(RomanNumeral::melodic_minor_vocabulary())
            .collect(),
    };
    for rn in numerals {
        let inversions: &[ChordInversion] = if rn.quality.is_seventh() {
            &[
                ChordInversion::Root,
                ChordInversion::First,
                ChordInversion::Second,
                ChordInversion::Third,
            ]
        } else {
            &[
                ChordInversion::Root,
                ChordInversion::First,
                ChordInversion::Second,
            ]
        };
        out.extend(inversions.iter().map(|&inv| rn.with_inversion(inv)));
    }
    out
}

/// Every octave placement of `pc` that falls inside `range`.
fn pitches_in_range(pc: PitchClass, range: VoiceRange) -> Vec<Pitch> {
    (range.low.octave.0 - 1..=range.high.octave.0 + 1)
        .map(|oct| Pitch::new(pc, Octave(oct)))
        .filter(|p| range.contains(*p))
        .collect()
}

pub struct CandidateGenerator<'a> {
    pub key: &'a Key,
    pub style: &'a Style,
}

impl<'a> CandidateGenerator<'a> {
    pub fn new(key: &'a Key, style: &'a Style) -> Self {
        CandidateGenerator { key, style }
    }

    /// All diatonic harmonic candidates whose chord contains `soprano`'s
    /// pitch class, each evaluated (and its raw voicings counted) against
    /// `previous`/`previous_roman_numeral` as context.
    #[allow(clippy::too_many_arguments)]
    pub fn generate(
        &self,
        soprano: Pitch,
        previous: Option<&Voicing>,
        previous_roman_numeral: Option<&RomanNumeral>,
        is_final_position: bool,
        diagnostics: &mut Diagnostics,
    ) -> Vec<EvaluatedCandidate> {
        let rules = self.style.rules();
        let previous_chord = previous_roman_numeral.and_then(|rn| rn.to_chord(self.key));
        harmonic_vocabulary(self.key.mode)
            .into_iter()
            .filter_map(|rn| {
                // An applied dominant whose root can't be spelled simply
                // isn't offered — same "exclude, don't fabricate" outcome
                // as an unspellable chord's `pitch_classes()` failing.
                let chord = rn.to_chord(self.key)?;
                if !chord.contains_pitch_class(soprano.pitch_class) {
                    return None;
                }
                self.best_voicing_for(
                    &rn,
                    &chord,
                    soprano,
                    previous,
                    previous_chord.as_ref(),
                    previous_roman_numeral,
                    is_final_position,
                    &rules,
                    diagnostics,
                )
            })
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    fn best_voicing_for(
        &self,
        rn: &RomanNumeral,
        chord: &Chord,
        soprano: Pitch,
        previous: Option<&Voicing>,
        previous_chord: Option<&Chord>,
        previous_roman_numeral: Option<&RomanNumeral>,
        is_final_position: bool,
        rules: &[Box<dyn Rule>],
        diagnostics: &mut Diagnostics,
    ) -> Option<EvaluatedCandidate> {
        // An unspellable chord simply isn't offered as a candidate — the
        // same "exclude, don't fabricate" outcome as a hard-rule
        // rejection, just decided one step earlier.
        let tones = chord.pitch_classes().ok()?;
        let bass_pc = tones[rn.inversion.bass_chord_tone_index()];
        let mut best: Option<EvaluatedCandidate> = None;

        for &alto_pc in &tones {
            for &tenor_pc in &tones {
                for bass in pitches_in_range(bass_pc, VoicePart::Bass.default_range()) {
                    for alto in pitches_in_range(alto_pc, VoicePart::Alto.default_range()) {
                        for tenor in pitches_in_range(tenor_pc, VoicePart::Tenor.default_range()) {
                            let voicing = Voicing::new(soprano, alto, tenor, bass);
                            diagnostics.record_generated();

                            let (status, score, reasons) = evaluate(
                                self.key,
                                previous,
                                previous_chord,
                                previous_roman_numeral,
                                &voicing,
                                chord,
                                rn,
                                is_final_position,
                                rules,
                            );
                            match &status {
                                CandidateStatus::Valid => diagnostics.record_retained(),
                                CandidateStatus::Rejected(ids) => diagnostics.record_rejected(ids),
                            }
                            let voice_leading_cost = previous
                                .map(|p| voice::total_motion(p, &voicing))
                                .unwrap_or(0);
                            let candidate = EvaluatedCandidate {
                                roman_numeral: *rn,
                                chord: *chord,
                                voicing,
                                status,
                                score,
                                reasons,
                                voice_leading_cost,
                            };
                            if best
                                .as_ref()
                                .is_none_or(|b| compare_candidates(&candidate, b) == Ordering::Less)
                            {
                                best = Some(candidate);
                            }
                        }
                    }
                }
            }
        }
        best
    }
}

#[allow(clippy::too_many_arguments)]
fn evaluate(
    key: &Key,
    previous: Option<&Voicing>,
    previous_chord: Option<&Chord>,
    previous_roman_numeral: Option<&RomanNumeral>,
    current: &Voicing,
    chord: &Chord,
    roman_numeral: &RomanNumeral,
    is_final_position: bool,
    rules: &[Box<dyn Rule>],
) -> (CandidateStatus, ScoreBreakdown, Vec<Reason>) {
    let ctx = RuleContext {
        key,
        previous,
        previous_chord,
        previous_roman_numeral,
        current,
        chord,
        roman_numeral,
        is_final_position,
    };
    let mut breakdown = ScoreBreakdown::default();
    let mut reasons = Vec::new();
    let mut violated = Vec::new();

    for rule in rules {
        let result = rule.evaluate(&ctx);
        reasons.extend(result.reasons.iter().cloned());
        if result.status == RuleStatus::Violation {
            violated.push(rule.id());
            continue;
        }
        let delta = result.penalty;
        match rule.id() {
            RuleId::HarmonicFunctionProgression => breakdown.harmonic_function += delta,
            RuleId::VoiceLeadingQuality => breakdown.voice_leading += delta,
            RuleId::CadenceSupport => breakdown.cadence += delta,
            RuleId::MelodicMotion => breakdown.melodic_motion += delta,
            RuleId::DoublingPreference => breakdown.doubling += delta,
            RuleId::RepeatedChord => breakdown.style += delta,
            _ => {}
        }
        if delta < 0.0 {
            breakdown.penalties.push(Penalty {
                rule: rule.id(),
                amount: delta,
            });
        }
    }

    let status = if violated.is_empty() {
        CandidateStatus::Valid
    } else {
        CandidateStatus::Rejected(violated)
    };
    (status, breakdown, reasons)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::Key;
    use crate::pitch::{Accidental, NoteLetter};

    #[test]
    fn c4_soprano_in_c_major_offers_every_diatonic_chord_containing_c() {
        let key = Key::C_MAJOR;
        let style = Style::CommonPractice;
        let generator = CandidateGenerator::new(&key, &style);
        let mut diag = Diagnostics::default();
        let soprano = Pitch::new(PitchClass::C, Octave(4));
        let candidates = generator.generate(soprano, None, None, false, &mut diag);

        // C appears in I (C,E,G), IV (F,A,C), and vi (A,C,E) — not in
        // diatonic ii, iii, V, V7, or vii°. Applied dominants share the
        // diatonic V's `degree` field (5) regardless of target, so this
        // checks the diatonic vocabulary only (`applied_to.is_none()`).
        let diatonic_numerals: std::collections::HashSet<_> = candidates
            .iter()
            .filter(|c| c.roman_numeral.applied_to().is_none())
            .map(|c| c.roman_numeral.degree.0)
            .collect();
        assert!(diatonic_numerals.contains(&1)); // I
        assert!(diatonic_numerals.contains(&4)); // IV
        assert!(diatonic_numerals.contains(&6)); // vi
        assert!(!diatonic_numerals.contains(&2)); // ii has no C
        assert!(!diatonic_numerals.contains(&5)); // V/V7 has no C

        // V/IV (the applied dominant of IV) is C-E-G — enharmonically
        // identical to I, since IV's own dominant is the tonic itself —
        // so it's expected to be offered here too.
        assert!(
            candidates
                .iter()
                .any(|c| c.roman_numeral.applied_to() == Some(crate::key::ScaleDegree::SUBDOMINANT))
        );
        assert!(diag.candidates_generated > 0);
    }

    #[test]
    fn harmonic_minor_vii_dim_has_at_least_one_valid_voicing_despite_no_root_doubling() {
        // vii° root position forces the leading tone into *both* soprano
        // (fixed here) and bass (the inversion's designated chord tone),
        // which `LeadingToneDoublingRule` (hard) always rejects — vii°
        // is only reachable at all via an inversion whose bass isn't the
        // root (vii°6 in real practice). Confirms the chord isn't
        // vocabulary-only dead weight that never survives to a valid
        // candidate.
        let key = Key::A_MINOR;
        let style = Style::CommonPractice;
        let generator = CandidateGenerator::new(&key, &style);
        let mut diag = Diagnostics::default();
        let g_sharp = Pitch::new(PitchClass::new(NoteLetter::G, Accidental::Sharp), Octave(4));
        let candidates = generator.generate(g_sharp, None, None, false, &mut diag);
        let vii_candidates: Vec<_> = candidates
            .iter()
            .filter(|c| {
                c.roman_numeral.source == crate::chord::NumeralSource::HarmonicMinorRaisedSeventh
                    && c.roman_numeral.degree == crate::key::ScaleDegree::LEADING_TONE
            })
            .collect();
        assert!(
            !vii_candidates.is_empty(),
            "vii° should be offered for a G# soprano in A minor"
        );
        assert!(
            vii_candidates.iter().any(|c| c.is_valid()),
            "expected at least one valid vii° voicing (via an inversion that doesn't double the leading tone), got {:?}",
            vii_candidates.iter().map(|c| &c.status).collect::<Vec<_>>()
        );
        // Root position specifically should never be the valid one.
        let root_position = vii_candidates
            .iter()
            .find(|c| c.roman_numeral.inversion == ChordInversion::Root)
            .expect("root position vii° should still be offered (just invalid)");
        assert!(!root_position.is_valid());
    }

    #[test]
    fn at_least_one_root_position_tonic_voicing_is_valid() {
        let key = Key::C_MAJOR;
        let style = Style::CommonPractice;
        let generator = CandidateGenerator::new(&key, &style);
        let mut diag = Diagnostics::default();
        let soprano = Pitch::new(PitchClass::C, Octave(5));
        let candidates = generator.generate(soprano, None, None, false, &mut diag);
        let tonic_root = candidates
            .iter()
            .find(|c| c.roman_numeral == RomanNumeral::I)
            .expect("I should be offered for a C soprano");
        assert!(
            tonic_root.is_valid(),
            "expected a valid voicing, got {:?}",
            tonic_root.status
        );
    }
}
