//! Structured scoring: every score is a breakdown with named reasons,
//! never a bare `f64` (AGENTS.md section 13). This is what `why()` and
//! `why_not()` read from directly instead of re-deriving explanations
//! after the fact (section 2.1).

use crate::chord::HarmonicFunction;
use crate::rules::RuleId;
use std::fmt;

/// A named, structured justification for part of a score. Natural-language
/// explanations are generated from these, not the other way around.
#[derive(Debug, Clone, PartialEq)]
pub enum Reason {
    HarmonicFunction {
        from: HarmonicFunction,
        to: HarmonicFunction,
        score_delta: f64,
    },
    VoiceLeading {
        total_motion: u32,
        common_tones: u8,
        contrary_motion: bool,
        score_delta: f64,
    },
    CadenceSupport {
        cadence: Cadence,
        score_delta: f64,
    },
    RuleViolation {
        rule: RuleId,
        severity: Severity,
    },
}

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = |d: f64| if d >= 0.0 { '+' } else { '-' };
        match self {
            Reason::HarmonicFunction {
                from,
                to,
                score_delta,
            } => write!(
                f,
                "{} harmonic function: {} -> {}: {}{:.2}",
                sign(*score_delta),
                from,
                to,
                sign(*score_delta),
                score_delta.abs()
            ),
            Reason::VoiceLeading {
                total_motion,
                common_tones,
                contrary_motion,
                score_delta,
            } => write!(
                f,
                "{} voice leading: {total_motion} semitones of motion, {common_tones} common tone(s), contrary motion: {contrary_motion}: {}{:.2}",
                sign(*score_delta),
                sign(*score_delta),
                score_delta.abs()
            ),
            Reason::CadenceSupport {
                cadence,
                score_delta,
            } => write!(
                f,
                "{} cadence support ({cadence}): {}{:.2}",
                sign(*score_delta),
                sign(*score_delta),
                score_delta.abs()
            ),
            Reason::RuleViolation { rule, severity } => {
                write!(f, "- {severity} rule violated: {rule}")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Hard,
    Soft,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            if *self == Severity::Hard {
                "hard"
            } else {
                "soft"
            }
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cadence {
    Authentic,
    Plagal,
    Half,
    Deceptive,
    None,
}

impl fmt::Display for Cadence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Cadence::Authentic => "authentic",
            Cadence::Plagal => "plagal",
            Cadence::Half => "half",
            Cadence::Deceptive => "deceptive",
            Cadence::None => "none",
        };
        write!(f, "{s}")
    }
}

/// A single named contribution that dragged a score down, kept alongside
/// the score-breakdown fields for a human-readable "what hurt this
/// candidate" list (not summed separately — see `ScoreBreakdown::total`).
#[derive(Debug, Clone, PartialEq)]
pub struct Penalty {
    pub rule: RuleId,
    pub amount: f64,
}

/// Every score mokuren produces is broken down by contributing factor.
/// `total()` sums exactly these six fields, in this declared order, so
/// results are reproducible regardless of how rules were evaluated
/// (AGENTS.md section 16).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ScoreBreakdown {
    pub harmonic_function: f64,
    pub voice_leading: f64,
    pub cadence: f64,
    pub melodic_motion: f64,
    pub doubling: f64,
    pub style: f64,
    pub penalties: Vec<Penalty>,
}

impl ScoreBreakdown {
    pub fn total(&self) -> f64 {
        self.harmonic_function
            + self.voice_leading
            + self.cadence
            + self.melodic_motion
            + self.doubling
            + self.style
    }
}

impl fmt::Display for ScoreBreakdown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "harmonic function      {:+.2}", self.harmonic_function)?;
        writeln!(f, "voice leading           {:+.2}", self.voice_leading)?;
        writeln!(f, "cadence                 {:+.2}", self.cadence)?;
        writeln!(f, "melodic motion          {:+.2}", self.melodic_motion)?;
        writeln!(f, "doubling                {:+.2}", self.doubling)?;
        writeln!(f, "style                   {:+.2}", self.style)?;
        write!(f, "total                   {:+.2}", self.total())
    }
}
