//! Evidence-bound intrinsic exploration and reward control.
//!
//! These values are operational control signals, not claims of subjective
//! experience. Curiosity ranks bounded executable hypotheses. Satisfaction
//! is granted only after independent behavioral verification proves a novel
//! frontier artifact, and can only bias later hypothesis ordering.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const INTRINSIC_DRIVE_SCHEMA: &str = "B_CORE_INTRINSIC_DRIVE_1";
pub const INTRINSIC_REWARD_CONTRACT_REVISION: u64 = 2;
const MAX_REWARDED_LESSON_FAMILIES: usize = 16;
const MAX_RECENT_INTRINSIC_OUTCOMES: usize = 64;
const MAX_PENDING_INTRINSIC_ATTEMPTS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntrinsicCuriosityHypothesis {
    pub hypothesis_id: String,
    pub lesson_ids: Vec<String>,
    pub signal_diversity: u16,
    pub executable_goal_count: u16,
    pub structural_novelty: u16,
    pub prediction_uncertainty: u16,
    pub expected_information_gain: u16,
    pub predicted_cost_units: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntrinsicRewardOutcome {
    pub hypothesis_id: String,
    pub behaviorally_verified: bool,
    #[serde(default)]
    pub campaign_accepted: bool,
    pub frontier_advance_units: u64,
    pub novel_verified_artifacts: usize,
    pub reward: u16,
    pub verified_satisfaction: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntrinsicCuriosityAttempt {
    pub hypothesis: IntrinsicCuriosityHypothesis,
    pub behaviorally_verified: bool,
    pub frontier_advance_units: u64,
    pub novel_verified_artifacts: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntrinsicDriveMemory {
    pub schema: String,
    #[serde(default)]
    pub reward_contract_revision: u64,
    #[serde(default)]
    pub legacy_precommit_hypotheses_attempted: u64,
    #[serde(default)]
    pub legacy_precommit_hypotheses_succeeded: u64,
    #[serde(default)]
    pub legacy_precommit_reward_events: u64,
    #[serde(default)]
    pub legacy_precommit_reward_total: u64,
    pub hypotheses_attempted: u64,
    pub hypotheses_succeeded: u64,
    pub hypotheses_failed: u64,
    pub intrinsic_reward_events: u64,
    pub intrinsic_reward_total: u64,
    pub current_curiosity: u16,
    pub verified_satisfaction: u16,
    pub last_hypothesis_id: Option<String>,
    pub last_reward: u16,
    pub rewarded_lesson_weights: BTreeMap<String, u16>,
    #[serde(default)]
    pub pending_attempts: Vec<IntrinsicCuriosityAttempt>,
    pub recent_outcomes: Vec<IntrinsicRewardOutcome>,
}

impl Default for IntrinsicDriveMemory {
    fn default() -> Self {
        Self {
            schema: INTRINSIC_DRIVE_SCHEMA.to_string(),
            reward_contract_revision: INTRINSIC_REWARD_CONTRACT_REVISION,
            legacy_precommit_hypotheses_attempted: 0,
            legacy_precommit_hypotheses_succeeded: 0,
            legacy_precommit_reward_events: 0,
            legacy_precommit_reward_total: 0,
            hypotheses_attempted: 0,
            hypotheses_succeeded: 0,
            hypotheses_failed: 0,
            intrinsic_reward_events: 0,
            intrinsic_reward_total: 0,
            current_curiosity: 50,
            verified_satisfaction: 0,
            last_hypothesis_id: None,
            last_reward: 0,
            rewarded_lesson_weights: BTreeMap::new(),
            pending_attempts: Vec::new(),
            recent_outcomes: Vec::new(),
        }
    }
}

impl IntrinsicDriveMemory {
    pub fn ensure_post_promotion_reward_contract(&mut self) {
        if self.reward_contract_revision >= INTRINSIC_REWARD_CONTRACT_REVISION {
            return;
        }
        self.legacy_precommit_hypotheses_attempted = self
            .legacy_precommit_hypotheses_attempted
            .saturating_add(self.hypotheses_attempted);
        self.legacy_precommit_hypotheses_succeeded = self
            .legacy_precommit_hypotheses_succeeded
            .saturating_add(self.hypotheses_succeeded);
        self.legacy_precommit_reward_events = self
            .legacy_precommit_reward_events
            .saturating_add(self.intrinsic_reward_events);
        self.legacy_precommit_reward_total = self
            .legacy_precommit_reward_total
            .saturating_add(self.intrinsic_reward_total);
        self.hypotheses_attempted = 0;
        self.hypotheses_succeeded = 0;
        self.hypotheses_failed = 0;
        self.intrinsic_reward_events = 0;
        self.intrinsic_reward_total = 0;
        self.current_curiosity = 50;
        self.verified_satisfaction = 0;
        self.last_hypothesis_id = None;
        self.last_reward = 0;
        self.rewarded_lesson_weights.clear();
        self.pending_attempts.clear();
        self.recent_outcomes.clear();
        self.reward_contract_revision = INTRINSIC_REWARD_CONTRACT_REVISION;
    }

    pub fn is_valid(&self) -> bool {
        self.schema == INTRINSIC_DRIVE_SCHEMA
            && self.reward_contract_revision == INTRINSIC_REWARD_CONTRACT_REVISION
            && self.current_curiosity <= 100
            && self.verified_satisfaction <= 100
            && self.last_reward <= 100
            && self.rewarded_lesson_weights.len() <= MAX_REWARDED_LESSON_FAMILIES
            && self
                .rewarded_lesson_weights
                .values()
                .all(|weight| *weight <= 100)
            && self.recent_outcomes.len() <= MAX_RECENT_INTRINSIC_OUTCOMES
            && self.pending_attempts.len() <= MAX_PENDING_INTRINSIC_ATTEMPTS
            && self.hypotheses_attempted
                == self
                    .hypotheses_succeeded
                    .saturating_add(self.hypotheses_failed)
                    .saturating_add(self.pending_attempts.len().min(u64::MAX as usize) as u64)
            && self.intrinsic_reward_events == self.hypotheses_succeeded
            && self
                .recent_outcomes
                .iter()
                .map(|outcome| outcome.hypothesis_id.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == self.recent_outcomes.len()
            && self
                .pending_attempts
                .iter()
                .map(|attempt| attempt.hypothesis.hypothesis_id.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == self.pending_attempts.len()
            && self.pending_attempts.iter().all(|attempt| {
                !self
                    .recent_outcomes
                    .iter()
                    .any(|outcome| outcome.hypothesis_id == attempt.hypothesis.hypothesis_id)
            })
    }

    pub fn score(&self, hypothesis: &IntrinsicCuriosityHypothesis) -> i32 {
        let learned_reward = hypothesis
            .lesson_ids
            .iter()
            .filter_map(|lesson| self.rewarded_lesson_weights.get(lesson))
            .copied()
            .map(i32::from)
            .sum::<i32>()
            .checked_div(hypothesis.lesson_ids.len().max(1) as i32)
            .unwrap_or(0)
            .min(20);
        i32::from(hypothesis.expected_information_gain) * 2
            + i32::from(hypothesis.structural_novelty)
            + i32::from(hypothesis.prediction_uncertainty)
            + i32::from(hypothesis.signal_diversity)
            + i32::from(self.current_curiosity / 5)
            + learned_reward
            - i32::from(hypothesis.predicted_cost_units)
    }

    pub fn begin_attempt(
        &mut self,
        hypothesis: &IntrinsicCuriosityHypothesis,
        behaviorally_verified: bool,
        frontier_advance_units: u64,
        novel_verified_artifacts: usize,
    ) -> bool {
        if self
            .recent_outcomes
            .iter()
            .any(|outcome| outcome.hypothesis_id == hypothesis.hypothesis_id)
        {
            return false;
        }
        if self
            .pending_attempts
            .iter()
            .any(|attempt| attempt.hypothesis.hypothesis_id == hypothesis.hypothesis_id)
        {
            return true;
        }
        self.hypotheses_attempted = self.hypotheses_attempted.saturating_add(1);
        self.last_hypothesis_id = Some(hypothesis.hypothesis_id.clone());
        self.pending_attempts.push(IntrinsicCuriosityAttempt {
            hypothesis: hypothesis.clone(),
            behaviorally_verified,
            frontier_advance_units,
            novel_verified_artifacts,
        });
        let pending =
            behaviorally_verified && frontier_advance_units > 0 && novel_verified_artifacts > 0;
        if !pending {
            let _ = self.resolve_attempt(&hypothesis.hypothesis_id, false);
        }
        pending
    }

    pub fn resolve_attempt(
        &mut self,
        hypothesis_id: &str,
        campaign_accepted: bool,
    ) -> Option<IntrinsicRewardOutcome> {
        if let Some(existing) = self
            .recent_outcomes
            .iter()
            .find(|outcome| outcome.hypothesis_id == hypothesis_id)
        {
            return Some(existing.clone());
        }
        let attempt_index = self
            .pending_attempts
            .iter()
            .position(|attempt| attempt.hypothesis.hypothesis_id == hypothesis_id)?;
        let attempt = self.pending_attempts.remove(attempt_index);
        for weight in self.rewarded_lesson_weights.values_mut() {
            *weight = weight.saturating_sub(1);
        }
        self.rewarded_lesson_weights.retain(|_, weight| *weight > 0);

        let verified_success = campaign_accepted
            && attempt.behaviorally_verified
            && attempt.frontier_advance_units > 0
            && attempt.novel_verified_artifacts > 0;
        let reward = if verified_success {
            24_u64
                .saturating_add(attempt.frontier_advance_units.saturating_mul(12))
                .saturating_add(
                    (attempt.novel_verified_artifacts.min(u64::MAX as usize) as u64)
                        .saturating_mul(8),
                )
                .min(100) as u16
        } else {
            0
        };
        if verified_success {
            self.hypotheses_succeeded = self.hypotheses_succeeded.saturating_add(1);
            self.intrinsic_reward_events = self.intrinsic_reward_events.saturating_add(1);
            self.intrinsic_reward_total = self
                .intrinsic_reward_total
                .saturating_add(u64::from(reward));
            self.current_curiosity = self.current_curiosity.saturating_sub(reward / 3).max(12);
            self.verified_satisfaction = reward;
            let lesson_bonus = (reward / 5).max(1);
            for lesson in &attempt.hypothesis.lesson_ids {
                let weight = self
                    .rewarded_lesson_weights
                    .entry(lesson.clone())
                    .or_default();
                *weight = weight.saturating_add(lesson_bonus).min(100);
            }
            while self.rewarded_lesson_weights.len() > MAX_REWARDED_LESSON_FAMILIES {
                let Some(evict) = self
                    .rewarded_lesson_weights
                    .iter()
                    .min_by(|left, right| left.1.cmp(right.1).then_with(|| left.0.cmp(right.0)))
                    .map(|(lesson, _)| lesson.clone())
                else {
                    break;
                };
                self.rewarded_lesson_weights.remove(&evict);
            }
        } else {
            self.hypotheses_failed = self.hypotheses_failed.saturating_add(1);
            self.current_curiosity = self.current_curiosity.saturating_add(5).min(100);
            self.verified_satisfaction = 0;
        }
        self.last_reward = reward;
        let outcome = IntrinsicRewardOutcome {
            hypothesis_id: attempt.hypothesis.hypothesis_id,
            behaviorally_verified: attempt.behaviorally_verified,
            campaign_accepted,
            frontier_advance_units: attempt.frontier_advance_units,
            novel_verified_artifacts: attempt.novel_verified_artifacts,
            reward,
            verified_satisfaction: self.verified_satisfaction,
        };
        self.recent_outcomes.push(outcome.clone());
        while self.recent_outcomes.len() > MAX_RECENT_INTRINSIC_OUTCOMES {
            self.recent_outcomes.remove(0);
        }
        Some(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hypothesis(id: &str) -> IntrinsicCuriosityHypothesis {
        IntrinsicCuriosityHypothesis {
            hypothesis_id: id.to_string(),
            lesson_ids: vec!["lesson-a".to_string(), "lesson-b".to_string()],
            signal_diversity: 4,
            executable_goal_count: 2,
            structural_novelty: 6,
            prediction_uncertainty: 5,
            expected_information_gain: 40,
            predicted_cost_units: 12,
        }
    }

    #[test]
    fn unverified_novelty_never_receives_reward_or_satisfaction() {
        let mut memory = IntrinsicDriveMemory::default();
        assert!(!memory.begin_attempt(&hypothesis("h1"), false, 4, 4));
        let result = memory.recent_outcomes.last().unwrap();
        assert_eq!(result.reward, 0);
        assert_eq!(result.verified_satisfaction, 0);
        assert_eq!(memory.intrinsic_reward_events, 0);
        assert_eq!(memory.hypotheses_failed, 1);
    }

    #[test]
    fn verified_frontier_rewards_and_biases_related_later_hypotheses() {
        let mut memory = IntrinsicDriveMemory::default();
        let unrelated = IntrinsicCuriosityHypothesis {
            lesson_ids: vec!["lesson-x".to_string(), "lesson-y".to_string()],
            ..hypothesis("unrelated")
        };
        let before = memory.score(&hypothesis("related"));
        let unrelated_before = memory.score(&unrelated);
        assert!(memory.begin_attempt(&hypothesis("success"), true, 1, 1));
        assert_eq!(memory.intrinsic_reward_events, 0);
        assert_eq!(memory.verified_satisfaction, 0);
        let result = memory.resolve_attempt("success", true).unwrap();
        assert!(result.reward > 0);
        assert_eq!(result.verified_satisfaction, result.reward);
        assert!(
            memory.score(&hypothesis("related")) - before
                > memory.score(&unrelated) - unrelated_before
        );
        assert!(memory.is_valid());
    }

    #[test]
    fn replayed_receipt_does_not_double_reward() {
        let mut memory = IntrinsicDriveMemory::default();
        assert!(memory.begin_attempt(&hypothesis("stable"), true, 1, 1));
        assert!(memory.begin_attempt(&hypothesis("stable"), true, 1, 1));
        let first = memory.resolve_attempt("stable", true).unwrap();
        let replay = memory.resolve_attempt("stable", true).unwrap();
        assert_eq!(first, replay);
        assert_eq!(memory.hypotheses_attempted, 1);
        assert_eq!(memory.intrinsic_reward_events, 1);
        assert_eq!(memory.intrinsic_reward_total, u64::from(first.reward));
        assert!(memory.is_valid());
    }

    #[test]
    fn semantic_duplicate_campaign_resolves_as_failure_without_reward() {
        let mut memory = IntrinsicDriveMemory::default();
        assert!(memory.begin_attempt(&hypothesis("duplicate"), true, 1, 1));
        let result = memory.resolve_attempt("duplicate", false).unwrap();
        assert!(result.behaviorally_verified);
        assert!(!result.campaign_accepted);
        assert_eq!(result.reward, 0);
        assert_eq!(memory.hypotheses_failed, 1);
        assert_eq!(memory.intrinsic_reward_events, 0);
        assert!(memory.is_valid());
    }

    #[test]
    fn precommit_reward_state_is_quarantined_on_contract_upgrade() {
        let mut memory = IntrinsicDriveMemory {
            reward_contract_revision: 1,
            hypotheses_attempted: 4,
            hypotheses_succeeded: 4,
            intrinsic_reward_events: 4,
            intrinsic_reward_total: 176,
            ..IntrinsicDriveMemory::default()
        };
        memory.ensure_post_promotion_reward_contract();
        assert_eq!(memory.hypotheses_attempted, 0);
        assert_eq!(memory.intrinsic_reward_events, 0);
        assert_eq!(memory.legacy_precommit_hypotheses_attempted, 4);
        assert_eq!(memory.legacy_precommit_reward_events, 4);
        assert_eq!(memory.legacy_precommit_reward_total, 176);
        assert!(memory.is_valid());
    }

    #[test]
    fn deployed_revision_one_state_deserializes_before_quarantine() {
        let mut original = IntrinsicDriveMemory::default();
        assert!(original.begin_attempt(&hypothesis("legacy"), true, 1, 1));
        original.resolve_attempt("legacy", true).unwrap();
        let mut old = serde_json::to_value(original).unwrap();
        let object = old.as_object_mut().unwrap();
        object.remove("reward_contract_revision");
        object.remove("legacy_precommit_hypotheses_attempted");
        object.remove("legacy_precommit_hypotheses_succeeded");
        object.remove("legacy_precommit_reward_events");
        object.remove("legacy_precommit_reward_total");
        object.remove("pending_attempts");
        object["recent_outcomes"].as_array_mut().unwrap()[0]
            .as_object_mut()
            .unwrap()
            .remove("campaign_accepted");
        let mut loaded: IntrinsicDriveMemory = serde_json::from_value(old).unwrap();
        assert_eq!(loaded.reward_contract_revision, 0);
        loaded.ensure_post_promotion_reward_contract();
        assert!(loaded.is_valid());
    }
}
