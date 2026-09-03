//! Evidence-bounded evaluation of conditional antecedents.
//!
//! A supported guard only makes its consequent available for deliberation. It
//! never establishes world truth, authorizes execution, or permits affirming
//! the antecedent from observation of the consequent.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::attribution::AttributedPropositionPolarityIR;
use crate::epistemic::{
    proposition_signature, BeliefRecordIR, BeliefRecordStatusIR, EpistemicLedgerIR,
    PropositionSignatureIR, SemanticStateValueIR, TemporalAnchorIR,
};
use crate::language_knowledge::LanguageCodeIR;
use crate::modality::{ConditionalKindIR, ConditionalRelationIR, ModalWorldIR};

pub const CONDITIONAL_GUARD_STORE_SCHEMA: &str = "B_CORE_CONDITIONAL_GUARD_STORE_IR_1";
pub const CONDITIONAL_GUARD_EVALUATION_SCHEMA: &str = "B_CORE_CONDITIONAL_GUARD_EVALUATION_IR_1";

const MAX_GUARDS: usize = 32;
const MAX_GUARD_EVIDENCE: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GuardStatusIR {
    Unresolved,
    SupportedByDialogueEvidence,
    ContradictedByDialogueEvidence,
    Contested,
    IneligibleCounterfactual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GuardEvidencePolarityIR {
    Supports,
    Contradicts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardEvidenceIR {
    pub belief_id: String,
    pub proposition_surface: String,
    pub source_actor: String,
    pub polarity: GuardEvidencePolarityIR,
    pub introduced_turn: u64,
    pub modal_world: ModalWorldIR,
    pub dialogue_truth_established: bool,
    pub external_execution_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionalGuardIR {
    pub guard_id: String,
    pub kind: ConditionalKindIR,
    pub antecedent_surface: String,
    pub consequent_surface: String,
    pub antecedent_negated: bool,
    pub expected_signature: PropositionSignatureIR,
    pub consequent_is_directive: bool,
    pub status: GuardStatusIR,
    pub evidence: Vec<GuardEvidenceIR>,
    pub introduced_turn: u64,
    pub last_evaluated_turn: u64,
    pub deliberation_eligible: bool,
    pub dialogue_truth_established: bool,
    pub reverse_inference_authorized: bool,
    pub external_execution_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionalGuardEvaluationIR {
    pub schema: String,
    pub guard_id: String,
    pub status: GuardStatusIR,
    pub antecedent_surface: String,
    pub consequent_surface: String,
    pub evidence: Vec<GuardEvidenceIR>,
    pub evaluation_turn: u64,
    pub deliberation_eligible: bool,
    pub status_changed: bool,
    pub realized_text: String,
    pub unsupported_claims: usize,
    pub dialogue_truth_established: bool,
    pub reverse_inference_authorized: bool,
    pub external_execution_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionalGuardStoreIR {
    pub schema: String,
    pub guards: Vec<ConditionalGuardIR>,
}

impl Default for ConditionalGuardStoreIR {
    fn default() -> Self {
        Self {
            schema: CONDITIONAL_GUARD_STORE_SCHEMA.to_string(),
            guards: Vec::new(),
        }
    }
}

impl ConditionalGuardStoreIR {
    pub fn apply_turn(
        &mut self,
        turn_index: u64,
        conditionals: &[ConditionalRelationIR],
        ledger: &EpistemicLedgerIR,
        language: LanguageCodeIR,
    ) -> Vec<ConditionalGuardEvaluationIR> {
        let mut new_guard_ids = BTreeSet::new();
        for (index, conditional) in conditionals.iter().take(8).enumerate() {
            if self
                .guards
                .iter()
                .any(|guard| same_rule(guard, conditional))
            {
                continue;
            }
            let guard_id = format!("GUARD-{turn_index:06}-{:02}", index + 1);
            let mut expected_signature = proposition_signature(
                &conditional.antecedent,
                AttributedPropositionPolarityIR::Positive,
            );
            if conditional.antecedent_negated {
                expected_signature.state_value = expected_signature.state_value.map(invert_state);
            }
            self.guards.push(ConditionalGuardIR {
                guard_id: guard_id.clone(),
                kind: conditional.kind,
                antecedent_surface: conditional.antecedent.clone(),
                consequent_surface: conditional.consequent.clone(),
                antecedent_negated: conditional.antecedent_negated,
                expected_signature,
                consequent_is_directive: conditional.consequent_is_directive,
                status: GuardStatusIR::Unresolved,
                evidence: Vec::new(),
                introduced_turn: turn_index,
                last_evaluated_turn: turn_index,
                deliberation_eligible: false,
                dialogue_truth_established: false,
                reverse_inference_authorized: false,
                external_execution_authorized: false,
            });
            new_guard_ids.insert(guard_id);
        }

        let mut evaluations = Vec::new();
        for guard in &mut self.guards {
            let old_status = guard.status;
            let old_evidence_ids = guard
                .evidence
                .iter()
                .map(|evidence| evidence.belief_id.clone())
                .collect::<Vec<_>>();
            evaluate_guard(guard, ledger, turn_index);
            let new_evidence_ids = guard
                .evidence
                .iter()
                .map(|evidence| evidence.belief_id.clone())
                .collect::<Vec<_>>();
            let changed = new_guard_ids.contains(&guard.guard_id)
                || old_status != guard.status
                || old_evidence_ids != new_evidence_ids;
            if changed {
                evaluations.push(evaluation_from_guard(
                    guard,
                    language,
                    old_status != guard.status,
                ));
            }
        }
        self.prune();
        debug_assert!(self.validate(turn_index, ledger));
        evaluations
    }

    pub fn validate(&self, completed_turns: u64, ledger: &EpistemicLedgerIR) -> bool {
        if self.schema != CONDITIONAL_GUARD_STORE_SCHEMA || self.guards.len() > MAX_GUARDS {
            return false;
        }
        let guard_ids = self
            .guards
            .iter()
            .map(|guard| guard.guard_id.as_str())
            .collect::<BTreeSet<_>>();
        if guard_ids.len() != self.guards.len() {
            return false;
        }
        self.guards.iter().all(|guard| {
            let evidence_ids = guard
                .evidence
                .iter()
                .map(|evidence| evidence.belief_id.as_str())
                .collect::<BTreeSet<_>>();
            !guard.guard_id.trim().is_empty()
                && !guard.antecedent_surface.trim().is_empty()
                && !guard.consequent_surface.trim().is_empty()
                && !guard.expected_signature.subject_key.trim().is_empty()
                && guard.evidence.len() <= MAX_GUARD_EVIDENCE
                && evidence_ids.len() == guard.evidence.len()
                && guard.introduced_turn > 0
                && guard.introduced_turn <= guard.last_evaluated_turn
                && guard.last_evaluated_turn <= completed_turns
                && !guard.dialogue_truth_established
                && !guard.reverse_inference_authorized
                && !guard.external_execution_authorized
                && guard.deliberation_eligible
                    == (guard.status == GuardStatusIR::SupportedByDialogueEvidence)
                && status_matches_evidence(guard, ledger)
                && guard.evidence.iter().all(|evidence| {
                    !evidence.dialogue_truth_established
                        && !evidence.external_execution_authorized
                        && evidence.modal_world == ModalWorldIR::Actual
                        && ledger.record(&evidence.belief_id).is_some_and(|record| {
                            record.status.is_reference_active()
                                && record.proposition_surface == evidence.proposition_surface
                                && record.source_actor == evidence.source_actor
                                && record.introduced_turn == evidence.introduced_turn
                                && !record.dialogue_truth_established
                                && !record.external_execution_authorized
                        })
                })
        })
    }

    fn prune(&mut self) {
        if self.guards.len() > MAX_GUARDS {
            let remove = self.guards.len() - MAX_GUARDS;
            self.guards.drain(..remove);
        }
    }
}

impl ConditionalGuardEvaluationIR {
    pub fn validate(&self, store: &ConditionalGuardStoreIR, ledger: &EpistemicLedgerIR) -> bool {
        if self.schema != CONDITIONAL_GUARD_EVALUATION_SCHEMA
            || self.guard_id.trim().is_empty()
            || self.antecedent_surface.trim().is_empty()
            || self.consequent_surface.trim().is_empty()
            || self.realized_text.trim().is_empty()
            || self.unsupported_claims != 0
            || self.dialogue_truth_established
            || self.reverse_inference_authorized
            || self.external_execution_authorized
            || self.deliberation_eligible
                != (self.status == GuardStatusIR::SupportedByDialogueEvidence)
        {
            return false;
        }
        store
            .guards
            .iter()
            .find(|guard| guard.guard_id == self.guard_id)
            .is_some_and(|guard| {
                guard.status == self.status
                    && guard.antecedent_surface == self.antecedent_surface
                    && guard.consequent_surface == self.consequent_surface
                    && guard.evidence == self.evidence
                    && guard.last_evaluated_turn == self.evaluation_turn
                    && guard.deliberation_eligible == self.deliberation_eligible
                    && self.evidence.iter().all(|evidence| {
                        ledger.record(&evidence.belief_id).is_some_and(|record| {
                            record.status.is_reference_active()
                                && record.signature.modal_world == ModalWorldIR::Actual
                        })
                    })
            })
    }
}

fn same_rule(guard: &ConditionalGuardIR, conditional: &ConditionalRelationIR) -> bool {
    guard.kind == conditional.kind
        && normalize_rule_arm(&guard.antecedent_surface)
            == normalize_rule_arm(&conditional.antecedent)
        && normalize_rule_arm(&guard.consequent_surface)
            == normalize_rule_arm(&conditional.consequent)
        && guard.antecedent_negated == conditional.antecedent_negated
}

fn normalize_rule_arm(text: &str) -> String {
    text.to_lowercase()
        .split(|character: char| !character.is_alphanumeric() && character != '_')
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn invert_state(value: SemanticStateValueIR) -> SemanticStateValueIR {
    match value {
        SemanticStateValueIR::Positive => SemanticStateValueIR::Negative,
        SemanticStateValueIR::Negative => SemanticStateValueIR::Positive,
    }
}

fn evaluate_guard(guard: &mut ConditionalGuardIR, ledger: &EpistemicLedgerIR, turn_index: u64) {
    guard.last_evaluated_turn = turn_index;
    if guard.kind == ConditionalKindIR::Counterfactual {
        guard.status = GuardStatusIR::IneligibleCounterfactual;
        guard.evidence.clear();
        guard.deliberation_eligible = false;
        return;
    }
    let mut evidence = ledger
        .records
        .iter()
        .filter(|record| {
            record.status.is_reference_active()
                && record.signature.modal_world == ModalWorldIR::Actual
        })
        .filter_map(|record| evidence_for_record(&guard.expected_signature, record))
        .collect::<Vec<_>>();
    evidence.sort_by(|left, right| {
        left.introduced_turn
            .cmp(&right.introduced_turn)
            .then_with(|| left.belief_id.cmp(&right.belief_id))
    });
    if evidence.len() > MAX_GUARD_EVIDENCE {
        let remove = evidence.len() - MAX_GUARD_EVIDENCE;
        evidence.drain(..remove);
    }
    let supports = evidence
        .iter()
        .any(|item| item.polarity == GuardEvidencePolarityIR::Supports);
    let contradicts = evidence
        .iter()
        .any(|item| item.polarity == GuardEvidencePolarityIR::Contradicts);
    let contested_record = evidence.iter().any(|item| {
        ledger
            .record(&item.belief_id)
            .is_some_and(|record| record.status == BeliefRecordStatusIR::Contested)
    });
    guard.status = match (supports, contradicts, contested_record) {
        (_, _, true) | (true, true, false) => GuardStatusIR::Contested,
        (true, false, false) => GuardStatusIR::SupportedByDialogueEvidence,
        (false, true, false) => GuardStatusIR::ContradictedByDialogueEvidence,
        (false, false, false) => GuardStatusIR::Unresolved,
    };
    guard.deliberation_eligible = guard.status == GuardStatusIR::SupportedByDialogueEvidence;
    guard.evidence = evidence;
}

fn evidence_for_record(
    expected: &PropositionSignatureIR,
    record: &BeliefRecordIR,
) -> Option<GuardEvidenceIR> {
    let observed = &record.signature;
    if !temporal_anchors_compatible(expected.temporal_anchor, observed.temporal_anchor)
        || expected.subject_key != observed.subject_key
    {
        return None;
    }
    let polarity = match (
        expected.state_axis.as_deref(),
        expected.state_value,
        observed.state_axis.as_deref(),
        observed.state_value,
    ) {
        (Some(expected_axis), Some(expected_value), Some(observed_axis), Some(observed_value))
            if expected_axis == observed_axis =>
        {
            if expected_value == observed_value {
                GuardEvidencePolarityIR::Supports
            } else {
                GuardEvidencePolarityIR::Contradicts
            }
        }
        _ if expected.normalized_fingerprint == observed.normalized_fingerprint => {
            GuardEvidencePolarityIR::Supports
        }
        _ => return None,
    };
    Some(GuardEvidenceIR {
        belief_id: record.belief_id.clone(),
        proposition_surface: record.proposition_surface.clone(),
        source_actor: record.source_actor.clone(),
        polarity,
        introduced_turn: record.introduced_turn,
        modal_world: record.signature.modal_world,
        dialogue_truth_established: false,
        external_execution_authorized: false,
    })
}

fn temporal_anchors_compatible(left: TemporalAnchorIR, right: TemporalAnchorIR) -> bool {
    left == right || left == TemporalAnchorIR::Unspecified || right == TemporalAnchorIR::Unspecified
}

fn status_matches_evidence(guard: &ConditionalGuardIR, ledger: &EpistemicLedgerIR) -> bool {
    if guard.kind == ConditionalKindIR::Counterfactual {
        return guard.status == GuardStatusIR::IneligibleCounterfactual
            && guard.evidence.is_empty();
    }
    let supports = guard
        .evidence
        .iter()
        .any(|item| item.polarity == GuardEvidencePolarityIR::Supports);
    let contradicts = guard
        .evidence
        .iter()
        .any(|item| item.polarity == GuardEvidencePolarityIR::Contradicts);
    let contested_record = guard.evidence.iter().any(|item| {
        ledger
            .record(&item.belief_id)
            .is_some_and(|record| record.status == BeliefRecordStatusIR::Contested)
    });
    match guard.status {
        GuardStatusIR::Unresolved => !supports && !contradicts,
        GuardStatusIR::SupportedByDialogueEvidence => supports && !contradicts && !contested_record,
        GuardStatusIR::ContradictedByDialogueEvidence => {
            !supports && contradicts && !contested_record
        }
        GuardStatusIR::Contested => contested_record || (supports && contradicts),
        GuardStatusIR::IneligibleCounterfactual => false,
    }
}

fn evaluation_from_guard(
    guard: &ConditionalGuardIR,
    language: LanguageCodeIR,
    status_changed: bool,
) -> ConditionalGuardEvaluationIR {
    ConditionalGuardEvaluationIR {
        schema: CONDITIONAL_GUARD_EVALUATION_SCHEMA.to_string(),
        guard_id: guard.guard_id.clone(),
        status: guard.status,
        antecedent_surface: guard.antecedent_surface.clone(),
        consequent_surface: guard.consequent_surface.clone(),
        evidence: guard.evidence.clone(),
        evaluation_turn: guard.last_evaluated_turn,
        deliberation_eligible: guard.deliberation_eligible,
        status_changed,
        realized_text: realize_guard(guard, language),
        unsupported_claims: 0,
        dialogue_truth_established: false,
        reverse_inference_authorized: false,
        external_execution_authorized: false,
    }
}

fn realize_guard(guard: &ConditionalGuardIR, language: LanguageCodeIR) -> String {
    let korean = matches!(language, LanguageCodeIR::Korean | LanguageCodeIR::Mixed);
    match (korean, guard.status) {
        (true, GuardStatusIR::Unresolved) => format!(
            "조건 ‘{}’은 아직 대화 근거로 확인되지 않았어. 후건 ‘{}’은 활성화하지 않아.",
            guard.antecedent_surface, guard.consequent_surface
        ),
        (true, GuardStatusIR::SupportedByDialogueEvidence) => format!(
            "대화 기록은 조건 ‘{}’을 지지해. 후건 ‘{}’은 검토할 수 있지만 자동 실행 권한은 없어.",
            guard.antecedent_surface, guard.consequent_surface
        ),
        (true, GuardStatusIR::ContradictedByDialogueEvidence) => format!(
            "대화 기록은 조건 ‘{}’과 반대야. 후건 ‘{}’은 활성화하지 않아.",
            guard.antecedent_surface, guard.consequent_surface
        ),
        (true, GuardStatusIR::Contested) => format!(
            "조건 ‘{}’에 관한 대화 근거가 충돌해. 후건 ‘{}’은 활성화하지 않아.",
            guard.antecedent_surface, guard.consequent_surface
        ),
        (true, GuardStatusIR::IneligibleCounterfactual) => format!(
            "조건 ‘{}’은 반사실 가정이므로 현재 후건 ‘{}’을 활성화하지 않아.",
            guard.antecedent_surface, guard.consequent_surface
        ),
        (false, GuardStatusIR::Unresolved) => format!(
            "The condition ‘{}’ is not established by the dialogue evidence, so the consequent ‘{}’ remains inactive.",
            guard.antecedent_surface, guard.consequent_surface
        ),
        (false, GuardStatusIR::SupportedByDialogueEvidence) => format!(
            "The dialogue record supports the condition ‘{}’. The consequent ‘{}’ is available for deliberation, but it has no automatic execution authority.",
            guard.antecedent_surface, guard.consequent_surface
        ),
        (false, GuardStatusIR::ContradictedByDialogueEvidence) => format!(
            "The dialogue record contradicts the condition ‘{}’, so the consequent ‘{}’ remains inactive.",
            guard.antecedent_surface, guard.consequent_surface
        ),
        (false, GuardStatusIR::Contested) => format!(
            "The dialogue evidence for condition ‘{}’ conflicts, so the consequent ‘{}’ remains inactive.",
            guard.antecedent_surface, guard.consequent_surface
        ),
        (false, GuardStatusIR::IneligibleCounterfactual) => format!(
            "The condition ‘{}’ is counterfactual and cannot activate the current consequent ‘{}’.",
            guard.antecedent_surface, guard.consequent_surface
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attribution::{AttributionAttitudeIR, EpistemicStatusIR};
    use crate::epistemic::EpistemicObservationIR;
    use crate::modality::ModalSemanticAnalyzer;

    fn record_turn(ledger: &mut EpistemicLedgerIR, turn: u64, surface: &str, world: ModalWorldIR) {
        ledger.apply_turn(
            turn,
            surface,
            &[],
            &[EpistemicObservationIR {
                origin_referent_id: format!("REF-{turn}"),
                source_actor: "DIALOGUE_USER".to_string(),
                proposition_surface: surface.to_string(),
                proposition_polarity: AttributedPropositionPolarityIR::Positive,
                modal_world: world,
                attribution_attitude: AttributionAttitudeIR::Say,
                epistemic_status: EpistemicStatusIR::Reported,
            }],
        );
    }

    #[test]
    fn actual_antecedent_supports_deliberation_without_execution_authority() {
        let relation = ModalSemanticAnalyzer
            .analyze("If the tests pass, deploy the service.")
            .conditionals
            .remove(0);
        let mut ledger = EpistemicLedgerIR::default();
        let mut store = ConditionalGuardStoreIR::default();
        store.apply_turn(1, &[relation], &ledger, LanguageCodeIR::English);
        record_turn(&mut ledger, 2, "The tests passed.", ModalWorldIR::Actual);
        let evaluations = store.apply_turn(2, &[], &ledger, LanguageCodeIR::English);
        assert_eq!(
            evaluations[0].status,
            GuardStatusIR::SupportedByDialogueEvidence
        );
        assert!(evaluations[0].deliberation_eligible);
        assert!(!evaluations[0].external_execution_authorized);
        assert!(!evaluations[0].reverse_inference_authorized);
        assert!(evaluations[0].validate(&store, &ledger));
    }

    #[test]
    fn possible_antecedent_does_not_satisfy_actual_guard() {
        let relation = ModalSemanticAnalyzer
            .analyze("If the tests pass, deploy the service.")
            .conditionals
            .remove(0);
        let mut ledger = EpistemicLedgerIR::default();
        record_turn(
            &mut ledger,
            1,
            "The tests might pass.",
            ModalWorldIR::EpistemicPossible,
        );
        let mut store = ConditionalGuardStoreIR::default();
        let evaluations = store.apply_turn(2, &[relation], &ledger, LanguageCodeIR::English);
        assert_eq!(evaluations[0].status, GuardStatusIR::Unresolved);
        assert!(!evaluations[0].deliberation_eligible);
    }

    #[test]
    fn observing_consequent_does_not_reverse_infer_antecedent() {
        let relation = ModalSemanticAnalyzer
            .analyze("If the tests pass, deploy the service.")
            .conditionals
            .remove(0);
        let mut ledger = EpistemicLedgerIR::default();
        record_turn(
            &mut ledger,
            1,
            "The service deployed.",
            ModalWorldIR::Actual,
        );
        let mut store = ConditionalGuardStoreIR::default();
        let evaluations = store.apply_turn(2, &[relation], &ledger, LanguageCodeIR::English);
        assert_eq!(evaluations[0].status, GuardStatusIR::Unresolved);
        assert!(!evaluations[0].reverse_inference_authorized);
    }

    #[test]
    fn unless_inverts_expected_state() {
        let relation = ModalSemanticAnalyzer
            .analyze("Unless the tests pass, stop the deployment.")
            .conditionals
            .remove(0);
        let mut ledger = EpistemicLedgerIR::default();
        record_turn(&mut ledger, 1, "The tests failed.", ModalWorldIR::Actual);
        let mut store = ConditionalGuardStoreIR::default();
        let evaluations = store.apply_turn(2, &[relation], &ledger, LanguageCodeIR::English);
        assert_eq!(
            evaluations[0].status,
            GuardStatusIR::SupportedByDialogueEvidence
        );
    }

    #[test]
    fn counterfactual_never_activates_current_consequent() {
        let relation = ModalSemanticAnalyzer
            .analyze("If the tests had passed, the deploy would have succeeded.")
            .conditionals
            .remove(0);
        let mut store = ConditionalGuardStoreIR::default();
        let ledger = EpistemicLedgerIR::default();
        let evaluations = store.apply_turn(1, &[relation], &ledger, LanguageCodeIR::English);
        assert_eq!(
            evaluations[0].status,
            GuardStatusIR::IneligibleCounterfactual
        );
        assert!(!evaluations[0].deliberation_eligible);
    }
}
