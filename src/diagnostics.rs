//! Search-wide statistics (AGENTS.md section 15). This is explainability
//! at the aggregate level: not just why one chord was chosen, but how
//! much of the search space was rejected and by what.

use crate::rules::RuleId;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Diagnostics {
    pub candidates_generated: u64,
    pub candidates_retained: u64,
    pub candidates_rejected: u64,
    /// `BTreeMap` (not `HashMap`) so iteration order — and therefore
    /// tie-broken display order — is deterministic.
    pub rejection_reasons: BTreeMap<RuleId, u64>,
}

impl Diagnostics {
    pub fn record_generated(&mut self) {
        self.candidates_generated += 1;
    }

    pub fn record_retained(&mut self) {
        self.candidates_retained += 1;
    }

    pub fn record_rejected(&mut self, rules: &[RuleId]) {
        self.candidates_rejected += 1;
        for rule in rules {
            *self.rejection_reasons.entry(*rule).or_insert(0) += 1;
        }
    }

    pub fn merge(&mut self, other: &Diagnostics) {
        self.candidates_generated += other.candidates_generated;
        self.candidates_retained += other.candidates_retained;
        self.candidates_rejected += other.candidates_rejected;
        for (rule, count) in &other.rejection_reasons {
            *self.rejection_reasons.entry(*rule).or_insert(0) += count;
        }
    }

    /// Rejection reasons ranked by count (descending), ties broken by
    /// `RuleId`'s declaration order for determinism.
    pub fn top_rejection_reasons(&self, n: usize) -> Vec<(RuleId, u64)> {
        let mut reasons: Vec<(RuleId, u64)> = self
            .rejection_reasons
            .iter()
            .map(|(r, c)| (*r, *c))
            .collect();
        reasons.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        reasons.truncate(n);
        reasons
    }
}

impl fmt::Display for Diagnostics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Candidates generated: {}", self.candidates_generated)?;
        writeln!(f, "Candidates retained:  {}", self.candidates_retained)?;
        writeln!(f, "Candidates rejected:  {}", self.candidates_rejected)?;
        writeln!(f)?;
        writeln!(f, "Top rejection reasons:")?;
        for (rule, count) in self.top_rejection_reasons(10) {
            writeln!(f, "{rule:<28} {count}")?;
        }
        Ok(())
    }
}
