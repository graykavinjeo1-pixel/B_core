//! Bounded counterexample-guided revision inside one Supervisor turn.
//!
//! The tracker owns no synthesis or installation authority. It only prevents
//! duplicate candidates, requires a successor to consume every fresh
//! counterexample, and stops revision when exact rollback is unavailable.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

pub const MAX_SAME_ATTEMPT_EXECUTIONS: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SameAttemptCounterexample {
    pub counterexample_id: String,
    pub required_changes: BTreeSet<String>,
}

impl SameAttemptCounterexample {
    pub fn new(
        counterexample_id: impl Into<String>,
        required_changes: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            counterexample_id: counterexample_id.into(),
            required_changes: required_changes.into_iter().collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CandidateAdmission {
    Execute,
    Duplicate,
    MissingCounterexampleConsumption,
    BudgetExhausted,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SameAttemptRevisionMetrics {
    pub candidates_admitted: usize,
    pub duplicate_candidates_rejected: usize,
    pub candidates_pruned_by_counterexample: usize,
    pub counterexamples_consumed: usize,
    pub plan_revisions: usize,
    pub exact_rollback_failures: usize,
}

#[derive(Debug, Clone)]
pub struct SameAttemptRevisionTracker {
    maximum_executions: usize,
    seen_fingerprints: BTreeSet<String>,
    required_changes: BTreeSet<String>,
    metrics: SameAttemptRevisionMetrics,
}

impl SameAttemptRevisionTracker {
    pub fn new(maximum_executions: usize) -> Self {
        Self {
            maximum_executions: maximum_executions.clamp(1, MAX_SAME_ATTEMPT_EXECUTIONS),
            seen_fingerprints: BTreeSet::new(),
            required_changes: BTreeSet::new(),
            metrics: SameAttemptRevisionMetrics::default(),
        }
    }

    pub fn admit_candidate(
        &mut self,
        fingerprint: &str,
        changed_dimensions: &BTreeSet<String>,
    ) -> CandidateAdmission {
        if self.metrics.candidates_admitted >= self.maximum_executions {
            return CandidateAdmission::BudgetExhausted;
        }
        if self.seen_fingerprints.contains(fingerprint) {
            self.metrics.duplicate_candidates_rejected =
                self.metrics.duplicate_candidates_rejected.saturating_add(1);
            return CandidateAdmission::Duplicate;
        }
        if !self.required_changes.is_subset(changed_dimensions) {
            self.metrics.candidates_pruned_by_counterexample = self
                .metrics
                .candidates_pruned_by_counterexample
                .saturating_add(1);
            return CandidateAdmission::MissingCounterexampleConsumption;
        }
        self.seen_fingerprints.insert(fingerprint.to_string());
        self.metrics.candidates_admitted = self.metrics.candidates_admitted.saturating_add(1);
        CandidateAdmission::Execute
    }

    /// Returns true only when another candidate may be synthesized from a
    /// fresh counterexample after exact restoration of the predecessor.
    pub fn observe_failure(
        &mut self,
        counterexample: SameAttemptCounterexample,
        rollback_exact: bool,
    ) -> bool {
        if !rollback_exact {
            self.metrics.exact_rollback_failures =
                self.metrics.exact_rollback_failures.saturating_add(1);
            return false;
        }
        let fresh = counterexample
            .required_changes
            .difference(&self.required_changes)
            .cloned()
            .collect::<BTreeSet<_>>();
        if fresh.is_empty() {
            return false;
        }
        self.required_changes.extend(fresh);
        self.metrics.counterexamples_consumed =
            self.metrics.counterexamples_consumed.saturating_add(1);
        self.metrics.plan_revisions = self.metrics.plan_revisions.saturating_add(1);
        self.metrics.candidates_admitted < self.maximum_executions
    }

    pub fn metrics(&self) -> &SameAttemptRevisionMetrics {
        &self.metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successor_must_consume_fresh_counterexample_and_cannot_repeat() {
        let mut tracker = SameAttemptRevisionTracker::new(3);
        assert_eq!(
            tracker.admit_candidate("candidate-a", &BTreeSet::from(["replace".to_string()])),
            CandidateAdmission::Execute
        );
        let requirement = "counterexample:ce-1".to_string();
        assert!(tracker.observe_failure(
            SameAttemptCounterexample::new("ce-1", [requirement.clone()]),
            true,
        ));
        assert_eq!(
            tracker.admit_candidate("candidate-a", &BTreeSet::from([requirement.clone()])),
            CandidateAdmission::Duplicate
        );
        assert_eq!(
            tracker.admit_candidate("candidate-b", &BTreeSet::from(["replace".to_string()])),
            CandidateAdmission::MissingCounterexampleConsumption
        );
        assert_eq!(
            tracker.admit_candidate("candidate-c", &BTreeSet::from([requirement])),
            CandidateAdmission::Execute
        );
    }

    #[test]
    fn missing_exact_rollback_terminates_revision() {
        let mut tracker = SameAttemptRevisionTracker::new(3);
        assert_eq!(
            tracker.admit_candidate("candidate-a", &BTreeSet::new()),
            CandidateAdmission::Execute
        );
        assert!(!tracker.observe_failure(
            SameAttemptCounterexample::new("ce-1", ["counterexample:ce-1".to_string()],),
            false,
        ));
        assert_eq!(tracker.metrics().exact_rollback_failures, 1);
    }

    #[test]
    fn execution_budget_is_hard_bounded_to_three() {
        let mut tracker = SameAttemptRevisionTracker::new(99);
        for index in 0..3 {
            assert_eq!(
                tracker.admit_candidate(&format!("candidate-{index}"), &BTreeSet::new()),
                CandidateAdmission::Execute
            );
        }
        assert_eq!(
            tracker.admit_candidate("candidate-3", &BTreeSet::new()),
            CandidateAdmission::BudgetExhausted
        );
    }
}
