//! Evidence-bound intrinsic exploration and reward control.
//!
//! These values are operational control signals, not claims of subjective
//! experience. Curiosity ranks bounded executable hypotheses. Satisfaction
//! is granted only after independent behavioral verification proves a novel
//! frontier artifact, and can only bias later hypothesis ordering.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const INTRINSIC_DRIVE_SCHEMA: &str = "B_CORE_INTRINSIC_DRIVE_1";
const MAX_REWARDED_LESSON_FAMILIES: usize = 16;
const MAX_RECENT_INTRINSIC_OUTCOMES: usize = 64;

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
    pub frontier_advance_units: u64,
    pub novel_verified_artifacts: usize,
    pub reward: u16,
    pub verified_satisfaction: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntrinsicDriveMemory {
    pub schema: String,
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
    pub recent_outcomes: Vec<IntrinsicRewardOutcome>,
}

impl Default for IntrinsicDriveMemory {
    fn default() -> Self {
        Self {
            schema: INTRINSIC_DRIVE_SCHEMA.to_string(),
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
            recent_outcomes: Vec::new(),
        }
    }
}

impl IntrinsicDriveMemory {
    pub fn is_valid(&self) -> bool {
        self.schema == INTRINSIC_DRIVE_SCHEMA
            && self.current_curiosity <= 100
            && self.verified_satisfaction <= 100
            && self.last_reward <= 100
            && self.rewarded_lesson_weights.len() <= MAX_REWARDED_LESSON_FAMILIES
            && self
                .rewarded_lesson_weights
                .values()
                .all(|weight| *weight <= 100)
            && self.recent_outcomes.len() <= MAX_RECENT_INTRINSIC_OUTCOMES
            && self.hypotheses_attempted
                == self
                    .hypotheses_succeeded
                    .saturating_add(self.hypotheses_failed)
            && self.intrinsic_reward_events == self.hypotheses_succeeded
            && self
                .recent_outcomes
                .iter()
                .map(|outcome| outcome.hypothesis_id.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == self.recent_outcomes.len()
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

    pub fn record_outcome(
        &mut self,
        hypothesis: &IntrinsicCuriosityHypothesis,
        behaviorally_verified: bool,
        frontier_advance_units: u64,
        novel_verified_artifacts: usize,
    ) -> IntrinsicRewardOutcome {
        if let Some(existing) = self
            .recent_outcomes
            .iter()
            .find(|outcome| outcome.hypothesis_id == hypothesis.hypothesis_id)
        {
            return existing.clone();
        }
        self.hypotheses_attempted = self.hypotheses_attempted.saturating_add(1);
        self.last_hypothesis_id = Some(hypothesis.hypothesis_id.clone());
        for weight in self.rewarded_lesson_weights.values_mut() {
            *weight = weight.saturating_sub(1);
        }
        self.rewarded_lesson_weights.retain(|_, weight| *weight > 0);

        let verified_success =
            behaviorally_verified && frontier_advance_units > 0 && novel_verified_artifacts > 0;
        let reward = if verified_success {
            24_u64
                .saturating_add(frontier_advance_units.saturating_mul(12))
                .saturating_add(
                    (novel_verified_artifacts.min(u64::MAX as usize) as u64).saturating_mul(8),
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
            for lesson in &hypothesis.lesson_ids {
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
            hypothesis_id: hypothesis.hypothesis_id.clone(),
            behaviorally_verified: verified_success,
            frontier_advance_units,
            novel_verified_artifacts,
            reward,
            verified_satisfaction: self.verified_satisfaction,
        };
        self.recent_outcomes.push(outcome.clone());
        while self.recent_outcomes.len() > MAX_RECENT_INTRINSIC_OUTCOMES {
            self.recent_outcomes.remove(0);
        }
        outcome
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
        let result = memory.record_outcome(&hypothesis("h1"), false, 4, 4);
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
        let result = memory.record_outcome(&hypothesis("success"), true, 1, 1);
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
        let first = memory.record_outcome(&hypothesis("stable"), true, 1, 1);
        let replay = memory.record_outcome(&hypothesis("stable"), true, 1, 1);
        assert_eq!(first, replay);
        assert_eq!(memory.hypotheses_attempted, 1);
        assert_eq!(memory.intrinsic_reward_events, 1);
        assert_eq!(memory.intrinsic_reward_total, u64::from(first.reward));
        assert!(memory.is_valid());
    }
}
