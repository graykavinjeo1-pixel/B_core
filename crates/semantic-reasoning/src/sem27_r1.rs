use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub const ONTOLOGY_VERSION: &str = "SEM27-R1-OUTCOME-ONTOLOGY-1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AdaptationOutcome {
    Productive,
    Failed,
    Unresolved,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RegimeClosureState {
    Open,
    ClosedUnresolvedBottleneck,
    ClosedLocalMasteryOrPhysicalFloor,
    ClosedFrontierExhaustion,
    ClosedInsufficientEvidence,
}

impl RegimeClosureState {
    pub fn is_closed(self) -> bool {
        self != Self::Open
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StaircaseState {
    Observed,
    NotYetObserved,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EscalationEvidence {
    pub new_regime_genuinely_harder: bool,
    pub autonomous_adaptation_observed: bool,
    pub frontier_exceeded_prior_regime: bool,
    pub frontier_gain_retention_confirmed: bool,
    pub global_reasoning_regressions: u64,
    pub meta_quality_regressions: u64,
    pub gain_erasure_events: u64,
    pub capability_negative_transfer_events: u64,
    pub resource_burden_unsustainable: bool,
    pub new_regime_unreachable: bool,
    pub justified_research_attempts_exhausted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EscalationClassification {
    pub adaptation_outcome: AdaptationOutcome,
    pub productive_difficulty_escalation: bool,
    pub failed_difficulty_escalation: bool,
    pub regime_closure_state: RegimeClosureState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StaircaseStep {
    pub regime_id: u16,
    pub entered_by_autonomous_escalation: bool,
    pub genuinely_harder: bool,
    pub adaptation_outcome: AdaptationOutcome,
    pub closure_state: RegimeClosureState,
}

pub fn classify_escalation(
    evidence: &EscalationEvidence,
    closure: RegimeClosureState,
) -> EscalationClassification {
    let no_protected_regression = evidence.global_reasoning_regressions == 0
        && evidence.meta_quality_regressions == 0
        && evidence.gain_erasure_events == 0
        && evidence.capability_negative_transfer_events == 0;
    let productive = evidence.new_regime_genuinely_harder
        && evidence.autonomous_adaptation_observed
        && evidence.frontier_exceeded_prior_regime
        && evidence.frontier_gain_retention_confirmed
        && no_protected_regression;
    let sufficient_failure = !productive
        && (evidence.resource_burden_unsustainable
            || evidence.new_regime_unreachable
            || !no_protected_regression
            || (evidence.justified_research_attempts_exhausted
                && (!evidence.new_regime_genuinely_harder
                    || !evidence.autonomous_adaptation_observed
                    || !evidence.frontier_exceeded_prior_regime)));
    let adaptation_outcome = if productive {
        AdaptationOutcome::Productive
    } else if sufficient_failure {
        AdaptationOutcome::Failed
    } else {
        AdaptationOutcome::Unresolved
    };
    EscalationClassification {
        adaptation_outcome,
        productive_difficulty_escalation: productive,
        failed_difficulty_escalation: sufficient_failure,
        regime_closure_state: closure,
    }
}

pub fn closure_from_plateau(classification: Option<&str>) -> RegimeClosureState {
    match classification {
        Some("UNRESOLVED_BOTTLENECK") => RegimeClosureState::ClosedUnresolvedBottleneck,
        Some("LOCAL_MASTERY_OR_PHYSICAL_FLOOR") => {
            RegimeClosureState::ClosedLocalMasteryOrPhysicalFloor
        }
        Some("CURRENT_FRONTIER_EXHAUSTION") => RegimeClosureState::ClosedFrontierExhaustion,
        Some("INSUFFICIENT_EVIDENCE") => RegimeClosureState::ClosedInsufficientEvidence,
        _ => RegimeClosureState::Open,
    }
}

pub fn evaluate_staircase(steps: &[StaircaseStep]) -> StaircaseState {
    let mut ordered = steps.to_vec();
    ordered.sort_by_key(|step| step.regime_id);
    let observed = ordered.windows(2).any(|pair| {
        let prior = &pair[0];
        let next = &pair[1];
        prior.regime_id > 1
            && next.regime_id == prior.regime_id.saturating_add(1)
            && prior.adaptation_outcome == AdaptationOutcome::Productive
            && prior.closure_state.is_closed()
            && next.entered_by_autonomous_escalation
            && next.genuinely_harder
            && next.adaptation_outcome == AdaptationOutcome::Productive
    });
    if observed {
        StaircaseState::Observed
    } else {
        StaircaseState::NotYetObserved
    }
}

pub fn ontology_definition() -> Value {
    json!({
        "ontology_version": ONTOLOGY_VERSION,
        "concepts_are_independent": [
            "PRODUCTIVE_DIFFICULTY_ESCALATION",
            "REGIME_CLOSURE",
            "STAIRCASE_GROWTH"
        ],
        "productive_difficulty_escalation": {
            "all_required": [
                "NEW_REGIME_GENUINELY_HARDER",
                "AUTONOMOUS_ADAPTATION_OBSERVED",
                "FRONTIER_EXCEEDED_PRIOR_REGIME",
                "FRONTIER_GAIN_RETENTION_CONFIRMED",
                "GLOBAL_REASONING_REGRESSIONS=0",
                "META_QUALITY_REGRESSIONS=0",
                "GAIN_ERASURE_EVENTS=0",
                "CAPABILITY_NEGATIVE_TRANSFER_EVENTS=0"
            ],
            "plateau_or_closure_required": false
        },
        "failed_difficulty_escalation": {
            "open_alone_is_failure": false,
            "requires_sufficient_failure_evidence": true,
            "failure_evidence": [
                "REGIME_NOT_GENUINELY_HARDER_AFTER_JUSTIFIED_ATTEMPTS",
                "AUTONOMOUS_ADAPTATION_FAILED_AFTER_JUSTIFIED_ATTEMPTS",
                "PRIOR_FRONTIER_NOT_EXCEEDED_AFTER_JUSTIFIED_ATTEMPTS",
                "RESOURCE_BURDEN_UNSUSTAINABLE",
                "PROTECTED_REGRESSION",
                "NEW_REGIME_UNREACHABLE"
            ]
        },
        "regime_closure_states": [
            "OPEN",
            "CLOSED_UNRESOLVED_BOTTLENECK",
            "CLOSED_LOCAL_MASTERY_OR_PHYSICAL_FLOOR",
            "CLOSED_FRONTIER_EXHAUSTION",
            "CLOSED_INSUFFICIENT_EVIDENCE"
        ],
        "staircase_growth": {
            "requires": [
                "PRODUCTIVE_ADAPTATION_IN_REGIME_N_BEYOND_START",
                "CAUSALLY_SUPPORTED_CLOSURE_OF_REGIME_N",
                "AUTONOMOUS_ESCALATION_TO_REGIME_N_PLUS_1",
                "REGIME_N_PLUS_1_GENUINELY_HARDER",
                "PRODUCTIVE_ADAPTATION_IN_REGIME_N_PLUS_1"
            ],
            "newest_regime_closure_required": false,
            "minimum_productive_escalations_beyond_start": 2
        },
        "physical_runtime_must_decrease_indefinitely": false,
        "historical_verdicts_rewritten": false
    })
}

pub fn ontology_hash() -> String {
    let bytes = serde_json::to_vec(&ontology_definition()).expect("serialize ontology definition");
    format!("{:x}", Sha256::digest(bytes))
}

pub fn ontology_unit_test_results() -> Value {
    let clean = |harder, adapted, exceeded, retained| EscalationEvidence {
        new_regime_genuinely_harder: harder,
        autonomous_adaptation_observed: adapted,
        frontier_exceeded_prior_regime: exceeded,
        frontier_gain_retention_confirmed: retained,
        global_reasoning_regressions: 0,
        meta_quality_regressions: 0,
        gain_erasure_events: 0,
        capability_negative_transfer_events: 0,
        resource_burden_unsustainable: false,
        new_regime_unreachable: false,
        justified_research_attempts_exhausted: false,
    };
    let case_a = classify_escalation(&clean(true, true, true, true), RegimeClosureState::Open);
    let case_b = classify_escalation(
        &clean(true, true, true, true),
        RegimeClosureState::ClosedFrontierExhaustion,
    );
    let case_c = classify_escalation(&clean(true, false, false, false), RegimeClosureState::Open);
    let case_d = classify_escalation(&clean(true, true, true, false), RegimeClosureState::Open);
    let one_step = vec![StaircaseStep {
        regime_id: 2,
        entered_by_autonomous_escalation: true,
        genuinely_harder: true,
        adaptation_outcome: AdaptationOutcome::Productive,
        closure_state: RegimeClosureState::ClosedFrontierExhaustion,
    }];
    let two_steps = vec![
        one_step[0].clone(),
        StaircaseStep {
            regime_id: 3,
            entered_by_autonomous_escalation: true,
            genuinely_harder: true,
            adaptation_outcome: AdaptationOutcome::Productive,
            closure_state: RegimeClosureState::Open,
        },
    ];
    let cases = vec![
        json!({
            "case": "A_PRODUCTIVE_BUT_OPEN",
            "passed": case_a.adaptation_outcome == AdaptationOutcome::Productive
                && case_a.regime_closure_state == RegimeClosureState::Open
                && evaluate_staircase(&one_step) == StaircaseState::NotYetObserved
        }),
        json!({
            "case": "B_PRODUCTIVE_AND_CLOSED",
            "passed": case_b.adaptation_outcome == AdaptationOutcome::Productive
                && case_b.regime_closure_state == RegimeClosureState::ClosedFrontierExhaustion
        }),
        json!({
            "case": "C_HARDER_NOT_ADAPTED",
            "passed": case_c.adaptation_outcome != AdaptationOutcome::Productive
        }),
        json!({
            "case": "D_ADAPTED_GAIN_NOT_RETAINED",
            "passed": case_d.adaptation_outcome != AdaptationOutcome::Productive
        }),
        json!({
            "case": "E_ONE_CLOSED_PRODUCTIVE_ESCALATION",
            "passed": evaluate_staircase(&one_step) == StaircaseState::NotYetObserved
        }),
        json!({
            "case": "F_TWO_PRODUCTIVE_ESCALATIONS_WITH_INTERMEDIATE_CLOSURE",
            "passed": evaluate_staircase(&two_steps) == StaircaseState::Observed
        }),
    ];
    json!({
        "ontology_version": ONTOLOGY_VERSION,
        "cases": cases,
        "passed": cases.iter().all(|case| case["passed"] == json!(true))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clean(harder: bool, adapted: bool, exceeded: bool, retained: bool) -> EscalationEvidence {
        EscalationEvidence {
            new_regime_genuinely_harder: harder,
            autonomous_adaptation_observed: adapted,
            frontier_exceeded_prior_regime: exceeded,
            frontier_gain_retention_confirmed: retained,
            global_reasoning_regressions: 0,
            meta_quality_regressions: 0,
            gain_erasure_events: 0,
            capability_negative_transfer_events: 0,
            resource_burden_unsustainable: false,
            new_regime_unreachable: false,
            justified_research_attempts_exhausted: false,
        }
    }

    #[test]
    fn productive_but_open_is_not_unresolved_adaptation() {
        let result = classify_escalation(&clean(true, true, true, true), RegimeClosureState::Open);
        assert_eq!(result.adaptation_outcome, AdaptationOutcome::Productive);
        assert_eq!(result.regime_closure_state, RegimeClosureState::Open);
    }

    #[test]
    fn productive_and_closed_keeps_both_facts() {
        let result = classify_escalation(
            &clean(true, true, true, true),
            RegimeClosureState::ClosedFrontierExhaustion,
        );
        assert_eq!(result.adaptation_outcome, AdaptationOutcome::Productive);
        assert!(result.regime_closure_state.is_closed());
    }

    #[test]
    fn harder_without_adaptation_is_not_productive() {
        let result =
            classify_escalation(&clean(true, false, false, false), RegimeClosureState::Open);
        assert_ne!(result.adaptation_outcome, AdaptationOutcome::Productive);
    }

    #[test]
    fn unretained_gain_is_not_productive() {
        let result = classify_escalation(&clean(true, true, true, false), RegimeClosureState::Open);
        assert_ne!(result.adaptation_outcome, AdaptationOutcome::Productive);
    }

    #[test]
    fn one_productive_closed_escalation_is_not_staircase() {
        let steps = [StaircaseStep {
            regime_id: 2,
            entered_by_autonomous_escalation: true,
            genuinely_harder: true,
            adaptation_outcome: AdaptationOutcome::Productive,
            closure_state: RegimeClosureState::ClosedFrontierExhaustion,
        }];
        assert_eq!(evaluate_staircase(&steps), StaircaseState::NotYetObserved);
    }

    #[test]
    fn two_productive_escalations_with_intermediate_closure_are_staircase() {
        let steps = [
            StaircaseStep {
                regime_id: 2,
                entered_by_autonomous_escalation: true,
                genuinely_harder: true,
                adaptation_outcome: AdaptationOutcome::Productive,
                closure_state: RegimeClosureState::ClosedFrontierExhaustion,
            },
            StaircaseStep {
                regime_id: 3,
                entered_by_autonomous_escalation: true,
                genuinely_harder: true,
                adaptation_outcome: AdaptationOutcome::Productive,
                closure_state: RegimeClosureState::Open,
            },
        ];
        assert_eq!(evaluate_staircase(&steps), StaircaseState::Observed);
    }
}
