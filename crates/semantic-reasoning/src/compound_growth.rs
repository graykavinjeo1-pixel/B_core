//! Closed compound-improvement substrate.
//!
//! This module joins seven previously separated contracts: causal credit,
//! mechanism abstraction, callable source operators, information-gain
//! experiments, semantic counterexample revision, sparse operator lifecycle,
//! and independent promotion.  Nothing in this module installs a patch or
//! grants itself approval. It emits and executes predecessor-bound operator
//! programs; compile/public validation and the existing atomic installer
//! remain the final source-mutation authorities.

use std::collections::{BTreeMap, BTreeSet};

use dockable_semantic_core::deliberation::validate_causal_mechanism;
use dockable_semantic_core::{
    ActionAuthorityIR, LiteralIR, MechanismKindIR, MechanismKnowledgeIR, MECHANISM_KNOWLEDGE_SCHEMA,
};
use serde::{Deserialize, Serialize};

use crate::autonomous_source_mutation::{
    compose_improvement_operator_graph, execute_improvement_operator_graph_on_sources,
    execute_improvement_operator_program_on_source, improvement_operator_ir_for_program,
    ImprovementOperatorExecution, ImprovementOperatorGraphIR, ImprovementOperatorGraphNodeProgram,
    ImprovementOperatorIR,
};
use crate::generalized_self_application::WeaknessEvidenceKind;
use crate::self_repair_contract::sha256;
use crate::structural_source_repair::StructuralRepairProgram;

pub const COMPOUND_GROWTH_SCHEMA: &str = "B_CORE_COMPOUND_GROWTH_IR_1";
pub const COMPOUND_OPERATOR_REPOSITORY_SCHEMA: &str = "B_CORE_COMPOUND_OPERATOR_REPOSITORY_IR_1";
const MAX_MECHANISMS: usize = 256;
const MAX_TRACES: usize = 512;
const MAX_EXPERIMENTS: usize = 128;
const MAX_HYPOTHESES: usize = 64;
const MAX_COUNTEREXAMPLES: usize = 128;
const MAX_CANDIDATES: usize = 128;
const MAX_OPERATOR_REPOSITORY: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AttestationRoleIR {
    CoreExecution,
    IndependentEvaluator,
    PublicObservation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndependentAttestationIR {
    pub schema: String,
    pub role: AttestationRoleIR,
    pub authority_id: String,
    pub candidate_sha256: String,
    pub observation_sha256: String,
    pub passed: bool,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriangularPromotionEvidenceIR {
    pub schema: String,
    pub promotion_id: String,
    pub proposer_authority_id: String,
    pub candidate_sha256: String,
    pub attestations: Vec<IndependentAttestationIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriangularPromotionReceiptIR {
    pub schema: String,
    pub promotion_id: String,
    pub candidate_sha256: String,
    pub accepted: bool,
    pub rejection_reasons: Vec<String>,
    pub independent_authorities: usize,
    pub proposer_self_approval_events: usize,
    pub receipt_sha256: String,
}

pub fn seal_attestation(
    role: AttestationRoleIR,
    authority_id: &str,
    candidate_sha256: &str,
    observation_sha256: &str,
    passed: bool,
) -> Result<IndependentAttestationIR, String> {
    if !valid_identity(authority_id)
        || !valid_hash(candidate_sha256)
        || !valid_hash(observation_sha256)
    {
        return Err("COMPOUND_ATTESTATION_INPUT_INVALID".to_string());
    }
    let mut attestation = IndependentAttestationIR {
        schema: COMPOUND_GROWTH_SCHEMA.to_string(),
        role,
        authority_id: authority_id.to_string(),
        candidate_sha256: candidate_sha256.to_string(),
        observation_sha256: observation_sha256.to_string(),
        passed,
        receipt_sha256: String::new(),
    };
    attestation.receipt_sha256 = content_hash(&attestation)?;
    Ok(attestation)
}

fn validate_attestation(attestation: &IndependentAttestationIR) -> bool {
    if attestation.schema != COMPOUND_GROWTH_SCHEMA
        || !valid_identity(&attestation.authority_id)
        || !valid_hash(&attestation.candidate_sha256)
        || !valid_hash(&attestation.observation_sha256)
    {
        return false;
    }
    let mut unsigned = attestation.clone();
    unsigned.receipt_sha256.clear();
    content_hash(&unsigned).is_ok_and(|hash| hash == attestation.receipt_sha256)
}

fn validate_promotion_receipt(receipt: &TriangularPromotionReceiptIR) -> bool {
    if receipt.schema != COMPOUND_GROWTH_SCHEMA
        || !valid_identity(&receipt.promotion_id)
        || !valid_hash(&receipt.candidate_sha256)
        || !valid_hash(&receipt.receipt_sha256)
    {
        return false;
    }
    let mut unsigned = receipt.clone();
    unsigned.receipt_sha256.clear();
    content_hash(&unsigned).is_ok_and(|hash| hash == receipt.receipt_sha256)
}

pub fn evaluate_triangular_promotion(
    evidence: &TriangularPromotionEvidenceIR,
) -> TriangularPromotionReceiptIR {
    let mut reasons = Vec::new();
    if evidence.schema != COMPOUND_GROWTH_SCHEMA
        || !valid_identity(&evidence.promotion_id)
        || !valid_identity(&evidence.proposer_authority_id)
        || !valid_hash(&evidence.candidate_sha256)
    {
        reasons.push("PROMOTION_IDENTITY_INVALID".to_string());
    }
    let roles = evidence
        .attestations
        .iter()
        .map(|attestation| attestation.role)
        .collect::<BTreeSet<_>>();
    let authorities = evidence
        .attestations
        .iter()
        .map(|attestation| attestation.authority_id.clone())
        .collect::<BTreeSet<_>>();
    if evidence.attestations.len() != 3
        || roles
            != [
                AttestationRoleIR::CoreExecution,
                AttestationRoleIR::IndependentEvaluator,
                AttestationRoleIR::PublicObservation,
            ]
            .into_iter()
            .collect()
    {
        reasons.push("PROMOTION_ATTESTATION_ROLES_INCOMPLETE".to_string());
    }
    if authorities.len() != evidence.attestations.len() {
        reasons.push("PROMOTION_AUTHORITIES_NOT_INDEPENDENT".to_string());
    }
    let proposer_self_approval_events = evidence
        .attestations
        .iter()
        .filter(|attestation| {
            attestation.authority_id == evidence.proposer_authority_id
                && attestation.role != AttestationRoleIR::CoreExecution
        })
        .count();
    if proposer_self_approval_events > 0 {
        reasons.push("PROMOTION_PROPOSER_SELF_APPROVAL".to_string());
    }
    if evidence.attestations.iter().any(|attestation| {
        !validate_attestation(attestation)
            || attestation.candidate_sha256 != evidence.candidate_sha256
            || !attestation.passed
    }) {
        reasons.push("PROMOTION_ATTESTATION_FAILED_OR_UNBOUND".to_string());
    }
    reasons.sort();
    reasons.dedup();
    let mut receipt = TriangularPromotionReceiptIR {
        schema: COMPOUND_GROWTH_SCHEMA.to_string(),
        promotion_id: evidence.promotion_id.clone(),
        candidate_sha256: evidence.candidate_sha256.clone(),
        accepted: reasons.is_empty(),
        rejection_reasons: reasons,
        independent_authorities: authorities.len(),
        proposer_self_approval_events,
        receipt_sha256: String::new(),
    };
    receipt.receipt_sha256 = content_hash(&receipt).unwrap_or_default();
    receipt
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MechanismExecutionTraceIR {
    pub trace_id: String,
    pub mechanism_knowledge_id: String,
    pub mechanism_id: String,
    pub candidate_sha256: String,
    pub action_executed: bool,
    pub action_output_consumed: bool,
    pub observed_effects: Vec<LiteralIR>,
    pub capability_units_before: u64,
    pub capability_units_after: u64,
    pub no_action_control_gain: u64,
    pub validation_cost_millis: u64,
    pub context_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CreditDispositionIR {
    Credited,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalCreditReceiptIR {
    pub schema: String,
    pub trace_id: String,
    pub mechanism_knowledge_id: String,
    pub candidate_sha256: String,
    pub disposition: CreditDispositionIR,
    pub rejection_reasons: Vec<String>,
    pub attributable_capability_gain: u64,
    pub cost_normalized_credit_micros: u64,
    pub promotion_receipt_sha256: String,
    pub receipt_sha256: String,
}

pub fn assign_causal_credit(
    knowledge: &MechanismKnowledgeIR,
    trace: &MechanismExecutionTraceIR,
    promotion: &TriangularPromotionReceiptIR,
) -> CausalCreditReceiptIR {
    let mut reasons = Vec::new();
    if trace.mechanism_knowledge_id != knowledge.knowledge_id
        || trace.mechanism_id != knowledge.mechanism.mechanism_id
    {
        reasons.push("CREDIT_MECHANISM_IDENTITY_MISMATCH".to_string());
    }
    if trace.candidate_sha256 != promotion.candidate_sha256
        || !promotion.accepted
        || !validate_promotion_receipt(promotion)
    {
        reasons.push("CREDIT_PROMOTION_NOT_ACCEPTED_OR_UNBOUND".to_string());
    }
    if !trace.action_executed || !trace.action_output_consumed {
        reasons.push("CREDIT_ACTION_NOT_EXECUTED_AND_CONSUMED".to_string());
    }
    let expected_effects = knowledge
        .mechanism
        .effects
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let observed_effects = trace
        .observed_effects
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if expected_effects.is_empty() || !expected_effects.is_subset(&observed_effects) {
        reasons.push("CREDIT_EXPECTED_EFFECT_NOT_OBSERVED".to_string());
    }
    let raw_gain = trace
        .capability_units_after
        .saturating_sub(trace.capability_units_before);
    let attributable_gain = raw_gain.saturating_sub(trace.no_action_control_gain);
    if attributable_gain == 0 {
        reasons.push("CREDIT_NO_GAIN_OVER_CONTROL".to_string());
    }
    reasons.sort();
    reasons.dedup();
    let credited = reasons.is_empty();
    let cost_normalized_credit_micros = if credited {
        attributable_gain
            .saturating_mul(1_000_000)
            .checked_div(trace.validation_cost_millis.max(1))
            .unwrap_or(u64::MAX)
    } else {
        0
    };
    let mut receipt = CausalCreditReceiptIR {
        schema: COMPOUND_GROWTH_SCHEMA.to_string(),
        trace_id: trace.trace_id.clone(),
        mechanism_knowledge_id: knowledge.knowledge_id.clone(),
        candidate_sha256: trace.candidate_sha256.clone(),
        disposition: if credited {
            CreditDispositionIR::Credited
        } else {
            CreditDispositionIR::Rejected
        },
        rejection_reasons: reasons,
        attributable_capability_gain: if credited { attributable_gain } else { 0 },
        cost_normalized_credit_micros,
        promotion_receipt_sha256: promotion.receipt_sha256.clone(),
        receipt_sha256: String::new(),
    };
    receipt.receipt_sha256 = content_hash(&receipt).unwrap_or_default();
    receipt
}

fn validate_causal_credit_receipt(receipt: &CausalCreditReceiptIR) -> bool {
    if receipt.schema != COMPOUND_GROWTH_SCHEMA
        || !valid_identity(&receipt.trace_id)
        || !valid_identity(&receipt.mechanism_knowledge_id)
        || !valid_hash(&receipt.candidate_sha256)
        || !valid_hash(&receipt.promotion_receipt_sha256)
        || !valid_hash(&receipt.receipt_sha256)
    {
        return false;
    }
    let mut unsigned = receipt.clone();
    unsigned.receipt_sha256.clear();
    content_hash(&unsigned).is_ok_and(|hash| hash == receipt.receipt_sha256)
}

fn validate_mechanism_knowledge(knowledge: &MechanismKnowledgeIR) -> bool {
    knowledge.schema == MECHANISM_KNOWLEDGE_SCHEMA
        && valid_identity(&knowledge.knowledge_id)
        && !knowledge.semantic_tags.is_empty()
        && knowledge.semantic_tags.len() <= 64
        && knowledge
            .semantic_tags
            .iter()
            .all(|tag| valid_identity(tag))
        && !knowledge.validation_evidence_refs.is_empty()
        && knowledge.confidence_millis > 0
        && knowledge.confidence_millis <= 1_000
        && validate_causal_mechanism(&knowledge.mechanism).is_ok()
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MechanismLiteralRoleIR {
    pub role_index: usize,
    pub value: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneralizedMechanismSchemaIR {
    pub schema: String,
    pub schema_id: String,
    pub kind: MechanismKindIR,
    pub authority: ActionAuthorityIR,
    pub prerequisite_roles: Vec<MechanismLiteralRoleIR>,
    pub effect_roles: Vec<MechanismLiteralRoleIR>,
    pub observation_role_count: usize,
    pub source_knowledge_ids: Vec<String>,
    pub shared_semantic_tags: Vec<String>,
    pub minimum_confidence_millis: u16,
    pub fresh_identity_transfer_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct MechanismShape {
    kind: MechanismKindIR,
    authority: ActionAuthorityIR,
    prerequisites: Vec<bool>,
    effects: Vec<bool>,
    observes: usize,
}

fn mechanism_shape(knowledge: &MechanismKnowledgeIR) -> MechanismShape {
    let mut prerequisites = knowledge
        .mechanism
        .prerequisites
        .iter()
        .map(|literal| literal.value)
        .collect::<Vec<_>>();
    let mut effects = knowledge
        .mechanism
        .effects
        .iter()
        .map(|literal| literal.value)
        .collect::<Vec<_>>();
    prerequisites.sort();
    effects.sort();
    MechanismShape {
        kind: knowledge.mechanism.kind,
        authority: knowledge.mechanism.authority,
        prerequisites,
        effects,
        observes: knowledge.mechanism.observes.len(),
    }
}

pub fn generalize_credited_mechanisms(
    knowledge: &[MechanismKnowledgeIR],
    credits: &[CausalCreditReceiptIR],
) -> Vec<GeneralizedMechanismSchemaIR> {
    let credited = credits
        .iter()
        .filter(|credit| {
            credit.disposition == CreditDispositionIR::Credited
                && validate_causal_credit_receipt(credit)
        })
        .map(|credit| credit.mechanism_knowledge_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut groups = BTreeMap::<MechanismShape, Vec<&MechanismKnowledgeIR>>::new();
    for item in knowledge.iter().filter(|item| {
        credited.contains(item.knowledge_id.as_str()) && validate_mechanism_knowledge(item)
    }) {
        groups.entry(mechanism_shape(item)).or_default().push(item);
    }
    groups
        .into_iter()
        .filter(|(_, items)| {
            items
                .iter()
                .map(|item| item.mechanism.mechanism_id.as_str())
                .collect::<BTreeSet<_>>()
                .len()
                >= 2
        })
        .map(|(shape, items)| {
            let mut source_knowledge_ids = items
                .iter()
                .map(|item| item.knowledge_id.clone())
                .collect::<Vec<_>>();
            source_knowledge_ids.sort();
            let shared_semantic_tags = items
                .iter()
                .map(|item| item.semantic_tags.iter().cloned().collect::<BTreeSet<_>>())
                .reduce(|left, right| left.intersection(&right).cloned().collect())
                .unwrap_or_default()
                .into_iter()
                .collect::<Vec<_>>();
            let unsigned = (
                shape.clone(),
                source_knowledge_ids.clone(),
                shared_semantic_tags.clone(),
            );
            let schema_id = content_hash(&unsigned).unwrap_or_default();
            GeneralizedMechanismSchemaIR {
                schema: COMPOUND_GROWTH_SCHEMA.to_string(),
                schema_id,
                kind: shape.kind,
                authority: shape.authority,
                prerequisite_roles: shape
                    .prerequisites
                    .into_iter()
                    .enumerate()
                    .map(|(role_index, value)| MechanismLiteralRoleIR { role_index, value })
                    .collect(),
                effect_roles: shape
                    .effects
                    .into_iter()
                    .enumerate()
                    .map(|(role_index, value)| MechanismLiteralRoleIR { role_index, value })
                    .collect(),
                observation_role_count: shape.observes,
                source_knowledge_ids,
                shared_semantic_tags,
                minimum_confidence_millis: items
                    .iter()
                    .map(|item| item.confidence_millis)
                    .min()
                    .unwrap_or(0),
                fresh_identity_transfer_required: true,
            }
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneralizationTransferReceiptIR {
    pub schema: String,
    pub generalized_schema_id: String,
    pub fresh_knowledge_id: String,
    pub accepted: bool,
    pub rejection_reasons: Vec<String>,
    pub causal_credit_receipt_sha256: String,
    pub receipt_sha256: String,
}

pub fn evaluate_generalization_transfer(
    schema: &GeneralizedMechanismSchemaIR,
    fresh_knowledge: &MechanismKnowledgeIR,
    credit: &CausalCreditReceiptIR,
) -> GeneralizationTransferReceiptIR {
    let mut reasons = Vec::new();
    if !validate_generalized_schema(schema) {
        reasons.push("GENERALIZATION_SCHEMA_INVALID".to_string());
    }
    if !validate_mechanism_knowledge(fresh_knowledge)
        || schema
            .source_knowledge_ids
            .iter()
            .any(|id| id == &fresh_knowledge.knowledge_id)
    {
        reasons.push("GENERALIZATION_IDENTITY_NOT_FRESH".to_string());
    }
    let expected_shape = MechanismShape {
        kind: schema.kind,
        authority: schema.authority,
        prerequisites: schema
            .prerequisite_roles
            .iter()
            .map(|role| role.value)
            .collect(),
        effects: schema.effect_roles.iter().map(|role| role.value).collect(),
        observes: schema.observation_role_count,
    };
    if mechanism_shape(fresh_knowledge) != expected_shape {
        reasons.push("GENERALIZATION_STRUCTURE_NOT_TRANSFERRED".to_string());
    }
    if !validate_causal_credit_receipt(credit)
        || credit.disposition != CreditDispositionIR::Credited
        || credit.mechanism_knowledge_id != fresh_knowledge.knowledge_id
    {
        reasons.push("GENERALIZATION_TRANSFER_NOT_CAUSALLY_CREDITED".to_string());
    }
    reasons.sort();
    reasons.dedup();
    let mut receipt = GeneralizationTransferReceiptIR {
        schema: COMPOUND_GROWTH_SCHEMA.to_string(),
        generalized_schema_id: schema.schema_id.clone(),
        fresh_knowledge_id: fresh_knowledge.knowledge_id.clone(),
        accepted: reasons.is_empty(),
        rejection_reasons: reasons,
        causal_credit_receipt_sha256: credit.receipt_sha256.clone(),
        receipt_sha256: String::new(),
    };
    receipt.receipt_sha256 = content_hash(&receipt).unwrap_or_default();
    receipt
}

fn validate_generalization_transfer_receipt(receipt: &GeneralizationTransferReceiptIR) -> bool {
    if receipt.schema != COMPOUND_GROWTH_SCHEMA
        || !valid_hash(&receipt.generalized_schema_id)
        || !valid_identity(&receipt.fresh_knowledge_id)
        || !valid_hash(&receipt.causal_credit_receipt_sha256)
        || !valid_hash(&receipt.receipt_sha256)
    {
        return false;
    }
    let mut unsigned = receipt.clone();
    unsigned.receipt_sha256.clear();
    content_hash(&unsigned).is_ok_and(|hash| hash == receipt.receipt_sha256)
}

fn validate_generalized_schema(schema: &GeneralizedMechanismSchemaIR) -> bool {
    if schema.schema != COMPOUND_GROWTH_SCHEMA
        || !valid_hash(&schema.schema_id)
        || schema.source_knowledge_ids.len() < 2
        || !all_unique(schema.source_knowledge_ids.iter().map(String::as_str))
        || !schema.fresh_identity_transfer_required
    {
        return false;
    }
    let shape = MechanismShape {
        kind: schema.kind,
        authority: schema.authority,
        prerequisites: schema
            .prerequisite_roles
            .iter()
            .map(|role| role.value)
            .collect(),
        effects: schema.effect_roles.iter().map(|role| role.value).collect(),
        observes: schema.observation_role_count,
    };
    content_hash(&(
        shape,
        schema.source_knowledge_ids.clone(),
        schema.shared_semantic_tags.clone(),
    ))
    .is_ok_and(|hash| hash == schema.schema_id)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HypothesisIR {
    pub hypothesis_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperimentPredictionIR {
    pub hypothesis_id: String,
    pub observation_signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveExperimentCandidateIR {
    pub experiment_id: String,
    pub predictions: Vec<ExperimentPredictionIR>,
    pub reliability_millis: u16,
    pub cost_millis: u64,
    pub risk_millis: u64,
    pub read_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveExperimentSelectionIR {
    pub schema: String,
    pub experiment_id: String,
    pub discriminated_hypothesis_pairs: usize,
    pub information_value_micros: u64,
    pub read_only: bool,
}

pub fn select_information_gain_experiment(
    hypotheses: &[HypothesisIR],
    candidates: &[ActiveExperimentCandidateIR],
) -> Option<ActiveExperimentSelectionIR> {
    if hypotheses.is_empty()
        || hypotheses
            .iter()
            .any(|hypothesis| !valid_identity(&hypothesis.hypothesis_id))
    {
        return None;
    }
    let hypothesis_ids = hypotheses
        .iter()
        .map(|hypothesis| hypothesis.hypothesis_id.as_str())
        .collect::<BTreeSet<_>>();
    candidates
        .iter()
        .filter(|candidate| {
            valid_identity(&candidate.experiment_id)
                && candidate.read_only
                && candidate.reliability_millis > 0
                && candidate.reliability_millis <= 1_000
                && candidate.predictions.len() == hypothesis_ids.len()
                && candidate.predictions.iter().all(|prediction| {
                    valid_identity(&prediction.hypothesis_id)
                        && valid_identity(&prediction.observation_signature)
                })
                && candidate
                    .predictions
                    .iter()
                    .map(|prediction| prediction.hypothesis_id.as_str())
                    .collect::<BTreeSet<_>>()
                    == hypothesis_ids
        })
        .filter_map(|candidate| {
            let predictions = candidate
                .predictions
                .iter()
                .map(|prediction| {
                    (
                        prediction.hypothesis_id.as_str(),
                        prediction.observation_signature.as_str(),
                    )
                })
                .collect::<Vec<_>>();
            let mut discriminated = 0_usize;
            for left in 0..predictions.len() {
                for right in left + 1..predictions.len() {
                    if predictions[left].1 != predictions[right].1 {
                        discriminated = discriminated.saturating_add(1);
                    }
                }
            }
            if discriminated == 0 {
                return None;
            }
            let value = u64::try_from(discriminated)
                .unwrap_or(u64::MAX)
                .saturating_mul(u64::from(candidate.reliability_millis))
                .saturating_mul(1_000_000)
                .checked_div(
                    candidate
                        .cost_millis
                        .saturating_add(candidate.risk_millis)
                        .max(1),
                )
                .unwrap_or(u64::MAX);
            Some(ActiveExperimentSelectionIR {
                schema: COMPOUND_GROWTH_SCHEMA.to_string(),
                experiment_id: candidate.experiment_id.clone(),
                discriminated_hypothesis_pairs: discriminated,
                information_value_micros: value,
                read_only: true,
            })
        })
        .max_by(|left, right| {
            left.information_value_micros
                .cmp(&right.information_value_micros)
                .then_with(|| right.experiment_id.cmp(&left.experiment_id))
        })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticValidationCounterexampleIR {
    pub counterexample_id: String,
    pub failed_candidate_sha256: String,
    pub expected_effects: Vec<LiteralIR>,
    pub observed_effects: Vec<LiteralIR>,
    pub missing_expected_effects: Vec<LiteralIR>,
    pub unexpected_effects: Vec<LiteralIR>,
    pub failed_invariant_classes: Vec<String>,
    pub compiler_error_codes: Vec<String>,
}

pub fn semantic_counterexample(
    failed_candidate_sha256: &str,
    expected_effects: Vec<LiteralIR>,
    observed_effects: Vec<LiteralIR>,
    mut failed_invariant_classes: Vec<String>,
    mut compiler_error_codes: Vec<String>,
) -> Result<SemanticValidationCounterexampleIR, String> {
    if !valid_hash(failed_candidate_sha256)
        || expected_effects.is_empty()
        || expected_effects
            .iter()
            .chain(&observed_effects)
            .any(|literal| !valid_identity(&literal.proposition_id))
        || failed_invariant_classes
            .iter()
            .chain(&compiler_error_codes)
            .any(|value| !valid_identity(value))
    {
        return Err("COMPOUND_COUNTEREXAMPLE_INVALID".to_string());
    }
    let expected = expected_effects.iter().cloned().collect::<BTreeSet<_>>();
    let observed = observed_effects.iter().cloned().collect::<BTreeSet<_>>();
    let missing_expected_effects = expected.difference(&observed).cloned().collect::<Vec<_>>();
    let unexpected_effects = observed.difference(&expected).cloned().collect::<Vec<_>>();
    failed_invariant_classes.sort();
    failed_invariant_classes.dedup();
    compiler_error_codes.sort();
    compiler_error_codes.dedup();
    let identity = (
        failed_candidate_sha256,
        &expected_effects,
        &observed_effects,
        &failed_invariant_classes,
        &compiler_error_codes,
    );
    Ok(SemanticValidationCounterexampleIR {
        counterexample_id: content_hash(&identity).unwrap_or_default(),
        failed_candidate_sha256: failed_candidate_sha256.to_string(),
        expected_effects,
        observed_effects,
        missing_expected_effects,
        unexpected_effects,
        failed_invariant_classes,
        compiler_error_codes,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevisionCandidateIR {
    pub candidate_id: String,
    pub candidate_sha256: String,
    pub predicted_effects: Vec<LiteralIR>,
    pub repaired_invariant_classes: Vec<String>,
    pub addressed_compiler_error_codes: Vec<String>,
    pub estimated_validation_cost_millis: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RankedRevisionCandidateIR {
    pub candidate: RevisionCandidateIR,
    pub semantic_feedback_score: i64,
    pub derived_from_counterexample_ids: Vec<String>,
}

pub fn revise_candidates_from_counterexamples(
    candidates: &[RevisionCandidateIR],
    counterexamples: &[SemanticValidationCounterexampleIR],
) -> Vec<RankedRevisionCandidateIR> {
    let mut ranked = candidates
        .iter()
        .filter(|candidate| {
            valid_identity(&candidate.candidate_id)
                && valid_hash(&candidate.candidate_sha256)
                && !counterexamples.iter().any(|counterexample| {
                    counterexample.failed_candidate_sha256 == candidate.candidate_sha256
                })
        })
        .filter_map(|candidate| {
            let predicted = candidate
                .predicted_effects
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>();
            let repaired = candidate
                .repaired_invariant_classes
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            let addressed_codes = candidate
                .addressed_compiler_error_codes
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            let mut score = 0_i64;
            let mut derived_from = Vec::new();
            for counterexample in counterexamples {
                let missing = counterexample
                    .missing_expected_effects
                    .iter()
                    .filter(|effect| predicted.contains(*effect))
                    .count();
                let invariants = counterexample
                    .failed_invariant_classes
                    .iter()
                    .filter(|class| repaired.contains(class.as_str()))
                    .count();
                let codes = counterexample
                    .compiler_error_codes
                    .iter()
                    .filter(|code| addressed_codes.contains(code.as_str()))
                    .count();
                if missing > 0 || invariants > 0 || codes > 0 {
                    derived_from.push(counterexample.counterexample_id.clone());
                }
                score = score
                    .saturating_add(
                        i64::try_from(missing)
                            .unwrap_or(i64::MAX)
                            .saturating_mul(40),
                    )
                    .saturating_add(
                        i64::try_from(invariants)
                            .unwrap_or(i64::MAX)
                            .saturating_mul(30),
                    )
                    .saturating_add(i64::try_from(codes).unwrap_or(i64::MAX).saturating_mul(20));
            }
            if !counterexamples.is_empty() && derived_from.is_empty() {
                return None;
            }
            score = score.saturating_sub(
                i64::try_from(candidate.estimated_validation_cost_millis / 100).unwrap_or(i64::MAX),
            );
            derived_from.sort();
            Some(RankedRevisionCandidateIR {
                candidate: candidate.clone(),
                semantic_feedback_score: score,
                derived_from_counterexample_ids: derived_from,
            })
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .semantic_feedback_score
            .cmp(&left.semantic_feedback_score)
            .then_with(|| {
                left.candidate
                    .candidate_id
                    .cmp(&right.candidate.candidate_id)
            })
    });
    ranked.truncate(3);
    ranked
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceBoundMechanismProgramIR {
    pub mechanism_knowledge_id: String,
    pub transformation: String,
    pub solution_strategy: String,
    pub structural_program: StructuralRepairProgram,
    pub semantic_tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutableMechanismOperatorBundleIR {
    pub schema: String,
    pub bundle_id: String,
    pub mechanism_knowledge_id: String,
    pub generalized_schema_ids: Vec<String>,
    pub operator: ImprovementOperatorIR,
    pub structural_program: StructuralRepairProgram,
    pub causal_credit_receipt_sha256: String,
    pub promotion_receipt_sha256: String,
    pub semantic_tags: Vec<String>,
}

pub fn compile_mechanism_to_improvement_operator(
    knowledge: &MechanismKnowledgeIR,
    binding: &SourceBoundMechanismProgramIR,
    credit: &CausalCreditReceiptIR,
    promotion: &TriangularPromotionReceiptIR,
    generalized_transfers: Vec<GeneralizationTransferReceiptIR>,
) -> Result<ExecutableMechanismOperatorBundleIR, String> {
    if !validate_mechanism_knowledge(knowledge)
        || !validate_causal_credit_receipt(credit)
        || !validate_promotion_receipt(promotion)
        || binding.mechanism_knowledge_id != knowledge.knowledge_id
        || credit.mechanism_knowledge_id != knowledge.knowledge_id
        || credit.disposition != CreditDispositionIR::Credited
        || !promotion.accepted
        || credit.promotion_receipt_sha256 != promotion.receipt_sha256
        || credit.candidate_sha256 != binding.structural_program.target_source_sha256
        || promotion.candidate_sha256 != binding.structural_program.target_source_sha256
        || binding.semantic_tags.is_empty()
        || generalized_transfers
            .iter()
            .any(|receipt| !receipt.accepted || !validate_generalization_transfer_receipt(receipt))
    {
        return Err("COMPOUND_OPERATOR_CAUSAL_BINDING_INVALID".to_string());
    }
    let operator = improvement_operator_ir_for_program(
        WeaknessEvidenceKind::ValidationCounterexample,
        &format!("PROGRAM_IR:MECHANISM_KNOWLEDGE: {}", binding.transformation),
        &binding.solution_strategy,
        &binding.structural_program,
    )?;
    let mut bundle = ExecutableMechanismOperatorBundleIR {
        schema: COMPOUND_GROWTH_SCHEMA.to_string(),
        bundle_id: String::new(),
        mechanism_knowledge_id: knowledge.knowledge_id.clone(),
        generalized_schema_ids: generalized_transfers
            .into_iter()
            .map(|receipt| receipt.generalized_schema_id)
            .collect(),
        operator,
        structural_program: binding.structural_program.clone(),
        causal_credit_receipt_sha256: credit.receipt_sha256.clone(),
        promotion_receipt_sha256: promotion.receipt_sha256.clone(),
        semantic_tags: binding.semantic_tags.clone(),
    };
    bundle.generalized_schema_ids.sort();
    bundle.generalized_schema_ids.dedup();
    bundle.semantic_tags.sort();
    bundle.semantic_tags.dedup();
    bundle.bundle_id = content_hash(&bundle)?;
    Ok(bundle)
}

fn validate_bundle(bundle: &ExecutableMechanismOperatorBundleIR) -> Result<(), String> {
    if bundle.schema != COMPOUND_GROWTH_SCHEMA || !valid_hash(&bundle.bundle_id) {
        return Err("COMPOUND_OPERATOR_BUNDLE_INVALID".to_string());
    }
    let mut unsigned = bundle.clone();
    unsigned.bundle_id.clear();
    if content_hash(&unsigned)? != bundle.bundle_id {
        return Err("COMPOUND_OPERATOR_BUNDLE_HASH_MISMATCH".to_string());
    }
    Ok(())
}

pub fn execute_mechanism_improvement_operator(
    bundle: &ExecutableMechanismOperatorBundleIR,
    predecessor_source: &str,
) -> Result<ImprovementOperatorExecution, String> {
    validate_bundle(bundle)?;
    execute_improvement_operator_program_on_source(
        &bundle.operator,
        predecessor_source,
        &bundle.structural_program,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OperatorLifecycleIR {
    Hot,
    Warm,
    Cold,
    Quarantined,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompoundOperatorProfileIR {
    pub bundle: ExecutableMechanismOperatorBundleIR,
    pub lifecycle: OperatorLifecycleIR,
    pub attempts: u64,
    pub successes: u64,
    pub rollbacks: u64,
    pub cumulative_validation_cost_millis: u64,
    pub last_used_generation: u64,
    pub successful_context_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorCoUseEdgeIR {
    pub left_operator_id: String,
    pub right_operator_id: String,
    pub successful_context_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompoundOperatorRepositoryIR {
    pub schema: String,
    pub profiles: Vec<CompoundOperatorProfileIR>,
    pub co_use_edges: Vec<OperatorCoUseEdgeIR>,
    pub max_profiles: usize,
    pub repository_sha256: String,
}

impl CompoundOperatorRepositoryIR {
    pub fn bounded(max_profiles: usize) -> Self {
        let mut repository = Self {
            schema: COMPOUND_OPERATOR_REPOSITORY_SCHEMA.to_string(),
            profiles: Vec::new(),
            co_use_edges: Vec::new(),
            max_profiles: max_profiles.clamp(1, MAX_OPERATOR_REPOSITORY),
            repository_sha256: String::new(),
        };
        repository.reseal();
        repository
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != COMPOUND_OPERATOR_REPOSITORY_SCHEMA
            || self.max_profiles == 0
            || self.max_profiles > MAX_OPERATOR_REPOSITORY
            || self.profiles.len() > self.max_profiles
            || !valid_hash(&self.repository_sha256)
            || self.profiles.iter().any(|profile| {
                validate_bundle(&profile.bundle).is_err()
                    || profile.attempts != profile.successes.saturating_add(profile.rollbacks)
                    || profile.successful_context_ids.len()
                        > usize::try_from(profile.successes).unwrap_or(usize::MAX)
                    || !all_unique(profile.successful_context_ids.iter().map(String::as_str))
            })
        {
            return Err("COMPOUND_OPERATOR_REPOSITORY_INVALID".to_string());
        }
        let operator_ids = self
            .profiles
            .iter()
            .map(|profile| profile.bundle.operator.operator_id.as_str())
            .collect::<BTreeSet<_>>();
        let edge_ids = self
            .co_use_edges
            .iter()
            .map(|edge| (&edge.left_operator_id, &edge.right_operator_id))
            .collect::<BTreeSet<_>>();
        if operator_ids.len() != self.profiles.len()
            || edge_ids.len() != self.co_use_edges.len()
            || self.co_use_edges.iter().any(|edge| {
                edge.left_operator_id >= edge.right_operator_id
                    || !operator_ids.contains(edge.left_operator_id.as_str())
                    || !operator_ids.contains(edge.right_operator_id.as_str())
                    || edge.successful_context_ids.is_empty()
                    || !all_unique(edge.successful_context_ids.iter().map(String::as_str))
            })
        {
            return Err("COMPOUND_OPERATOR_REPOSITORY_TOPOLOGY_INVALID".to_string());
        }
        let mut unsigned = self.clone();
        unsigned.repository_sha256.clear();
        if content_hash(&unsigned)? != self.repository_sha256 {
            return Err("COMPOUND_OPERATOR_REPOSITORY_HASH_MISMATCH".to_string());
        }
        Ok(())
    }

    pub fn advance_generation(&mut self, generation: u64) -> Result<(), String> {
        self.validate()?;
        self.maintain(generation);
        Ok(())
    }

    pub fn insert_promoted(
        &mut self,
        bundle: ExecutableMechanismOperatorBundleIR,
        generation: u64,
    ) -> Result<bool, String> {
        self.validate()?;
        validate_bundle(&bundle)?;
        if let Some(profile) = self
            .profiles
            .iter()
            .find(|profile| profile.bundle.operator.operator_id == bundle.operator.operator_id)
        {
            if profile.bundle != bundle {
                return Err("COMPOUND_OPERATOR_REPOSITORY_COLLISION".to_string());
            }
            return Ok(false);
        }
        self.profiles.push(CompoundOperatorProfileIR {
            bundle,
            lifecycle: OperatorLifecycleIR::Warm,
            attempts: 0,
            successes: 0,
            rollbacks: 0,
            cumulative_validation_cost_millis: 0,
            last_used_generation: generation,
            successful_context_ids: Vec::new(),
        });
        self.maintain(generation);
        Ok(true)
    }

    pub fn record_outcome(
        &mut self,
        operator_ids: &[String],
        context_id: &str,
        generation: u64,
        succeeded: bool,
        validation_cost_millis: u64,
    ) -> Result<(), String> {
        self.validate()?;
        let unique = operator_ids.iter().cloned().collect::<BTreeSet<_>>();
        if unique.is_empty() || unique.len() != operator_ids.len() || !valid_identity(context_id) {
            return Err("COMPOUND_OPERATOR_OUTCOME_INVALID".to_string());
        }
        for operator_id in &unique {
            let profile = self
                .profiles
                .iter_mut()
                .find(|profile| profile.bundle.operator.operator_id == *operator_id)
                .ok_or_else(|| "COMPOUND_OPERATOR_OUTCOME_UNKNOWN_OPERATOR".to_string())?;
            profile.attempts = profile.attempts.saturating_add(1);
            profile.last_used_generation = generation;
            profile.cumulative_validation_cost_millis = profile
                .cumulative_validation_cost_millis
                .saturating_add(validation_cost_millis);
            if succeeded {
                profile.successes = profile.successes.saturating_add(1);
                if !profile
                    .successful_context_ids
                    .iter()
                    .any(|context| context == context_id)
                {
                    profile.successful_context_ids.push(context_id.to_string());
                    profile.successful_context_ids.sort();
                }
            } else {
                profile.rollbacks = profile.rollbacks.saturating_add(1);
            }
        }
        if succeeded && unique.len() >= 2 {
            let ids = unique.into_iter().collect::<Vec<_>>();
            for left in 0..ids.len() {
                for right in left + 1..ids.len() {
                    let edge = self.co_use_edges.iter_mut().find(|edge| {
                        edge.left_operator_id == ids[left] && edge.right_operator_id == ids[right]
                    });
                    if let Some(edge) = edge {
                        if !edge
                            .successful_context_ids
                            .iter()
                            .any(|context| context == context_id)
                        {
                            edge.successful_context_ids.push(context_id.to_string());
                            edge.successful_context_ids.sort();
                        }
                    } else {
                        self.co_use_edges.push(OperatorCoUseEdgeIR {
                            left_operator_id: ids[left].clone(),
                            right_operator_id: ids[right].clone(),
                            successful_context_ids: vec![context_id.to_string()],
                        });
                    }
                }
            }
        }
        self.maintain(generation);
        Ok(())
    }

    pub fn recall(
        &self,
        semantic_tags: &[String],
        generalized_schema_ids: &[String],
        max_results: usize,
    ) -> Vec<ExecutableMechanismOperatorBundleIR> {
        if self.validate().is_err() {
            return Vec::new();
        }
        let tags = semantic_tags
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let schemas = generalized_schema_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let mut matches = self
            .profiles
            .iter()
            .filter(|profile| profile.lifecycle != OperatorLifecycleIR::Quarantined)
            .filter_map(|profile| {
                let tag_hits = profile
                    .bundle
                    .semantic_tags
                    .iter()
                    .filter(|tag| tags.contains(tag.as_str()))
                    .count();
                let schema_hits = profile
                    .bundle
                    .generalized_schema_ids
                    .iter()
                    .filter(|schema| schemas.contains(schema.as_str()))
                    .count();
                if tag_hits == 0 && schema_hits == 0 {
                    return None;
                }
                let lifecycle = match profile.lifecycle {
                    OperatorLifecycleIR::Hot => 300_i64,
                    OperatorLifecycleIR::Warm => 150,
                    OperatorLifecycleIR::Cold => 0,
                    OperatorLifecycleIR::Quarantined => -1_000,
                };
                let score = lifecycle
                    .saturating_add(
                        i64::try_from(tag_hits)
                            .unwrap_or(i64::MAX)
                            .saturating_mul(20),
                    )
                    .saturating_add(
                        i64::try_from(schema_hits)
                            .unwrap_or(i64::MAX)
                            .saturating_mul(80),
                    )
                    .saturating_add(
                        i64::try_from(profile.successes)
                            .unwrap_or(i64::MAX)
                            .saturating_mul(10),
                    );
                Some((score, &profile.bundle))
            })
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| {
            right
                .0
                .cmp(&left.0)
                .then_with(|| left.1.bundle_id.cmp(&right.1.bundle_id))
        });
        matches
            .into_iter()
            .take(max_results.min(32))
            .map(|(_, bundle)| bundle.clone())
            .collect()
    }

    pub fn compile_productive_composites(&self) -> Vec<ImprovementOperatorGraphIR> {
        if self.validate().is_err() {
            return Vec::new();
        }
        self.co_use_edges
            .iter()
            .filter(|edge| edge.successful_context_ids.len() >= 2)
            .filter_map(|edge| {
                let left = self.profiles.iter().find(|profile| {
                    profile.bundle.operator.operator_id == edge.left_operator_id
                        && profile.lifecycle != OperatorLifecycleIR::Quarantined
                })?;
                let right = self.profiles.iter().find(|profile| {
                    profile.bundle.operator.operator_id == edge.right_operator_id
                        && profile.lifecycle != OperatorLifecycleIR::Quarantined
                })?;
                (left.bundle.structural_program.file_id != right.bundle.structural_program.file_id)
                    .then(|| {
                        compose_improvement_operator_graph(&[
                            left.bundle.operator.clone(),
                            right.bundle.operator.clone(),
                        ])
                        .ok()
                    })
                    .flatten()
            })
            .collect()
    }

    fn maintain(&mut self, generation: u64) {
        for profile in &mut self.profiles {
            profile.lifecycle = if profile.rollbacks >= 3 && profile.successes == 0 {
                OperatorLifecycleIR::Quarantined
            } else if profile.successes >= 2
                && profile.successful_context_ids.len() >= 2
                && profile.successes > profile.rollbacks
            {
                OperatorLifecycleIR::Hot
            } else if generation.saturating_sub(profile.last_used_generation) >= 3 {
                OperatorLifecycleIR::Cold
            } else {
                OperatorLifecycleIR::Warm
            };
        }
        self.profiles.sort_by(|left, right| {
            lifecycle_rank(right.lifecycle)
                .cmp(&lifecycle_rank(left.lifecycle))
                .then_with(|| right.successes.cmp(&left.successes))
                .then_with(|| left.rollbacks.cmp(&right.rollbacks))
                .then_with(|| right.last_used_generation.cmp(&left.last_used_generation))
                .then_with(|| left.bundle.bundle_id.cmp(&right.bundle.bundle_id))
        });
        self.profiles.truncate(self.max_profiles);
        let active = self
            .profiles
            .iter()
            .map(|profile| profile.bundle.operator.operator_id.as_str())
            .collect::<BTreeSet<_>>();
        self.co_use_edges.retain(|edge| {
            active.contains(edge.left_operator_id.as_str())
                && active.contains(edge.right_operator_id.as_str())
        });
        self.co_use_edges.sort_by(|left, right| {
            left.left_operator_id
                .cmp(&right.left_operator_id)
                .then_with(|| left.right_operator_id.cmp(&right.right_operator_id))
        });
        self.reseal();
    }

    fn reseal(&mut self) {
        self.repository_sha256.clear();
        self.repository_sha256 = content_hash(self).unwrap_or_default();
    }
}

pub fn execute_productive_composite(
    repository: &CompoundOperatorRepositoryIR,
    graph: &ImprovementOperatorGraphIR,
    predecessor_sources: &BTreeMap<String, String>,
) -> Result<Vec<ImprovementOperatorExecution>, String> {
    repository.validate()?;
    let nodes = graph
        .operator_ids
        .iter()
        .map(|operator_id| {
            let profile = repository
                .profiles
                .iter()
                .find(|profile| profile.bundle.operator.operator_id == *operator_id)
                .ok_or_else(|| "COMPOUND_GRAPH_OPERATOR_MISSING".to_string())?;
            let source = predecessor_sources
                .get(&profile.bundle.structural_program.file_id)
                .ok_or_else(|| "COMPOUND_GRAPH_PREDECESSOR_MISSING".to_string())?;
            Ok(ImprovementOperatorGraphNodeProgram {
                operator: profile.bundle.operator.clone(),
                predecessor_source: source.clone(),
                structural_repair_program: profile.bundle.structural_program.clone(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    execute_improvement_operator_graph_on_sources(graph, &nodes)
}

fn lifecycle_rank(lifecycle: OperatorLifecycleIR) -> u8 {
    match lifecycle {
        OperatorLifecycleIR::Hot => 3,
        OperatorLifecycleIR::Warm => 2,
        OperatorLifecycleIR::Cold => 1,
        OperatorLifecycleIR::Quarantined => 0,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorOutcomeIR {
    pub operator_ids: Vec<String>,
    pub context_id: String,
    pub succeeded: bool,
    pub validation_cost_millis: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompoundGrowthCycleRequestIR {
    pub schema: String,
    pub generation: u64,
    pub mechanisms: Vec<MechanismKnowledgeIR>,
    pub execution_traces: Vec<MechanismExecutionTraceIR>,
    pub promotion_evidence: Vec<TriangularPromotionEvidenceIR>,
    pub source_bindings: Vec<SourceBoundMechanismProgramIR>,
    pub hypotheses: Vec<HypothesisIR>,
    pub experiment_candidates: Vec<ActiveExperimentCandidateIR>,
    pub counterexamples: Vec<SemanticValidationCounterexampleIR>,
    pub revision_candidates: Vec<RevisionCandidateIR>,
    pub repository: CompoundOperatorRepositoryIR,
    pub operator_outcomes: Vec<OperatorOutcomeIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompoundGrowthCycleIR {
    pub schema: String,
    pub generation: u64,
    pub promotion_receipts: Vec<TriangularPromotionReceiptIR>,
    pub causal_credit_receipts: Vec<CausalCreditReceiptIR>,
    pub generalized_schemas: Vec<GeneralizedMechanismSchemaIR>,
    pub compiled_operator_bundle_ids: Vec<String>,
    pub selected_experiment: Option<ActiveExperimentSelectionIR>,
    pub revised_candidates: Vec<RankedRevisionCandidateIR>,
    pub productive_composite_graphs: Vec<ImprovementOperatorGraphIR>,
    pub repository: CompoundOperatorRepositoryIR,
    pub text_only_growth_events: usize,
    pub external_model_calls: usize,
    pub cycle_sha256: String,
}

pub fn run_compound_growth_cycle(
    request: &CompoundGrowthCycleRequestIR,
) -> Result<CompoundGrowthCycleIR, String> {
    validate_cycle_bounds(request)?;
    let promotion_receipts = request
        .promotion_evidence
        .iter()
        .map(evaluate_triangular_promotion)
        .collect::<Vec<_>>();
    let knowledge_by_id = request
        .mechanisms
        .iter()
        .map(|knowledge| (knowledge.knowledge_id.as_str(), knowledge))
        .collect::<BTreeMap<_, _>>();
    let promotions_by_candidate = promotion_receipts
        .iter()
        .map(|receipt| (receipt.candidate_sha256.as_str(), receipt))
        .collect::<BTreeMap<_, _>>();
    let causal_credit_receipts = request
        .execution_traces
        .iter()
        .map(|trace| {
            let knowledge = knowledge_by_id
                .get(trace.mechanism_knowledge_id.as_str())
                .ok_or_else(|| "COMPOUND_CREDIT_KNOWLEDGE_MISSING".to_string())?;
            let promotion = promotions_by_candidate
                .get(trace.candidate_sha256.as_str())
                .ok_or_else(|| "COMPOUND_CREDIT_PROMOTION_MISSING".to_string())?;
            Ok(assign_causal_credit(knowledge, trace, promotion))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let generalized_schemas =
        generalize_credited_mechanisms(&request.mechanisms, &causal_credit_receipts);
    let credit_by_knowledge = causal_credit_receipts
        .iter()
        .filter(|credit| credit.disposition == CreditDispositionIR::Credited)
        .map(|credit| (credit.mechanism_knowledge_id.as_str(), credit))
        .collect::<BTreeMap<_, _>>();
    let mut repository = request.repository.clone();
    let mut compiled_operator_bundle_ids = Vec::new();
    for binding in &request.source_bindings {
        let Some(knowledge) = knowledge_by_id.get(binding.mechanism_knowledge_id.as_str()) else {
            continue;
        };
        let Some(credit) = credit_by_knowledge.get(binding.mechanism_knowledge_id.as_str()) else {
            continue;
        };
        let Some(promotion) = promotions_by_candidate.get(credit.candidate_sha256.as_str()) else {
            continue;
        };
        let bundle = compile_mechanism_to_improvement_operator(
            knowledge,
            binding,
            credit,
            promotion,
            Vec::new(),
        )?;
        compiled_operator_bundle_ids.push(bundle.bundle_id.clone());
        repository.insert_promoted(bundle, request.generation)?;
    }
    for outcome in &request.operator_outcomes {
        repository.record_outcome(
            &outcome.operator_ids,
            &outcome.context_id,
            request.generation,
            outcome.succeeded,
            outcome.validation_cost_millis,
        )?;
    }
    compiled_operator_bundle_ids.sort();
    let selected_experiment =
        select_information_gain_experiment(&request.hypotheses, &request.experiment_candidates);
    let revised_candidates = revise_candidates_from_counterexamples(
        &request.revision_candidates,
        &request.counterexamples,
    );
    let productive_composite_graphs = repository.compile_productive_composites();
    let mut result = CompoundGrowthCycleIR {
        schema: COMPOUND_GROWTH_SCHEMA.to_string(),
        generation: request.generation,
        promotion_receipts,
        causal_credit_receipts,
        generalized_schemas,
        compiled_operator_bundle_ids,
        selected_experiment,
        revised_candidates,
        productive_composite_graphs,
        repository,
        text_only_growth_events: 0,
        external_model_calls: 0,
        cycle_sha256: String::new(),
    };
    result.cycle_sha256 = content_hash(&result)?;
    Ok(result)
}

fn validate_cycle_bounds(request: &CompoundGrowthCycleRequestIR) -> Result<(), String> {
    if request.schema != COMPOUND_GROWTH_SCHEMA
        || request.mechanisms.len() > MAX_MECHANISMS
        || request.execution_traces.len() > MAX_TRACES
        || request.promotion_evidence.len() > MAX_TRACES
        || request.source_bindings.len() > MAX_TRACES
        || request.hypotheses.len() > MAX_HYPOTHESES
        || request.experiment_candidates.len() > MAX_EXPERIMENTS
        || request.counterexamples.len() > MAX_COUNTEREXAMPLES
        || request.revision_candidates.len() > MAX_CANDIDATES
        || request.repository.max_profiles > MAX_OPERATOR_REPOSITORY
    {
        return Err("COMPOUND_GROWTH_BOUNDS_INVALID".to_string());
    }
    request.repository.validate()?;
    if request
        .mechanisms
        .iter()
        .any(|knowledge| !validate_mechanism_knowledge(knowledge))
        || !all_unique(
            request
                .mechanisms
                .iter()
                .map(|knowledge| knowledge.knowledge_id.as_str()),
        )
        || !all_unique(
            request
                .execution_traces
                .iter()
                .map(|trace| trace.trace_id.as_str()),
        )
        || !all_unique(
            request
                .promotion_evidence
                .iter()
                .map(|evidence| evidence.promotion_id.as_str()),
        )
        || request.execution_traces.iter().any(|trace| {
            !valid_identity(&trace.trace_id)
                || !valid_identity(&trace.mechanism_knowledge_id)
                || !valid_identity(&trace.mechanism_id)
                || !valid_identity(&trace.context_id)
                || !valid_hash(&trace.candidate_sha256)
        })
    {
        return Err("COMPOUND_GROWTH_IDENTITY_INVALID".to_string());
    }
    Ok(())
}

fn all_unique<'a>(mut values: impl Iterator<Item = &'a str>) -> bool {
    let mut seen = BTreeSet::new();
    values.all(|value| valid_identity(value) && seen.insert(value))
}

fn valid_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_identity(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 256
}

fn content_hash<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|error| format!("COMPOUND_GROWTH_JSON:{error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dockable_semantic_core::CausalMechanismIR;

    use crate::structural_source_repair::synthesize_structural_repair;

    fn literal(id: &str) -> LiteralIR {
        LiteralIR {
            proposition_id: id.to_string(),
            value: true,
        }
    }

    fn knowledge(
        knowledge_id: &str,
        mechanism_id: &str,
        prerequisite: &str,
        effect: &str,
    ) -> MechanismKnowledgeIR {
        MechanismKnowledgeIR {
            schema: MECHANISM_KNOWLEDGE_SCHEMA.to_string(),
            knowledge_id: knowledge_id.to_string(),
            mechanism: CausalMechanismIR {
                mechanism_id: mechanism_id.to_string(),
                kind: MechanismKindIR::Intervention,
                prerequisites: vec![literal(prerequisite)],
                effects: vec![literal(effect)],
                observes: vec![format!("OBS-{effect}")],
                authority: ActionAuthorityIR::ReversibleMutation,
                authorized: true,
                reversible: true,
                recovery_reference: Some(format!("sealed:{knowledge_id}")),
                cost_millis: 10,
                risk_millis: 1,
                provenance_refs: vec![format!("test:{knowledge_id}")],
            },
            semantic_tags: vec!["source-repair".to_string(), "typed-edit".to_string()],
            validation_evidence_refs: vec![format!("test:{knowledge_id}:pass")],
            confidence_millis: 950,
        }
    }

    fn promotion(candidate_sha256: &str, suffix: &str) -> TriangularPromotionReceiptIR {
        let evidence = TriangularPromotionEvidenceIR {
            schema: COMPOUND_GROWTH_SCHEMA.to_string(),
            promotion_id: format!("PROMOTION-{suffix}"),
            proposer_authority_id: format!("CORE-{suffix}"),
            candidate_sha256: candidate_sha256.to_string(),
            attestations: vec![
                seal_attestation(
                    AttestationRoleIR::CoreExecution,
                    &format!("CORE-{suffix}"),
                    candidate_sha256,
                    &sha256(format!("core:{suffix}").as_bytes()),
                    true,
                )
                .unwrap(),
                seal_attestation(
                    AttestationRoleIR::IndependentEvaluator,
                    &format!("EVALUATOR-{suffix}"),
                    candidate_sha256,
                    &sha256(format!("evaluator:{suffix}").as_bytes()),
                    true,
                )
                .unwrap(),
                seal_attestation(
                    AttestationRoleIR::PublicObservation,
                    &format!("PUBLIC-{suffix}"),
                    candidate_sha256,
                    &sha256(format!("public:{suffix}").as_bytes()),
                    true,
                )
                .unwrap(),
            ],
        };
        evaluate_triangular_promotion(&evidence)
    }

    fn trace(
        knowledge: &MechanismKnowledgeIR,
        candidate_sha256: &str,
        suffix: &str,
    ) -> MechanismExecutionTraceIR {
        MechanismExecutionTraceIR {
            trace_id: format!("TRACE-{suffix}"),
            mechanism_knowledge_id: knowledge.knowledge_id.clone(),
            mechanism_id: knowledge.mechanism.mechanism_id.clone(),
            candidate_sha256: candidate_sha256.to_string(),
            action_executed: true,
            action_output_consumed: true,
            observed_effects: knowledge.mechanism.effects.clone(),
            capability_units_before: 10,
            capability_units_after: 25,
            no_action_control_gain: 3,
            validation_cost_millis: 20,
            context_id: format!("CONTEXT-{suffix}"),
        }
    }

    fn credited(
        knowledge: &MechanismKnowledgeIR,
        candidate_sha256: &str,
        suffix: &str,
    ) -> (TriangularPromotionReceiptIR, CausalCreditReceiptIR) {
        let promotion = promotion(candidate_sha256, suffix);
        let credit = assign_causal_credit(
            knowledge,
            &trace(knowledge, candidate_sha256, suffix),
            &promotion,
        );
        assert!(promotion.accepted);
        assert_eq!(credit.disposition, CreditDispositionIR::Credited);
        (promotion, credit)
    }

    fn bundle(
        knowledge: &MechanismKnowledgeIR,
        file_id: &str,
        before: &str,
        after: &str,
        strategy: &str,
        suffix: &str,
    ) -> (
        SourceBoundMechanismProgramIR,
        TriangularPromotionReceiptIR,
        CausalCreditReceiptIR,
        ExecutableMechanismOperatorBundleIR,
    ) {
        let structural_program = synthesize_structural_repair(file_id, before, after).unwrap();
        let (promotion, credit) =
            credited(knowledge, &structural_program.target_source_sha256, suffix);
        let binding = SourceBoundMechanismProgramIR {
            mechanism_knowledge_id: knowledge.knowledge_id.clone(),
            transformation: format!("{strategy}_TRANSFORMATION"),
            solution_strategy: strategy.to_string(),
            structural_program,
            semantic_tags: knowledge.semantic_tags.clone(),
        };
        let compiled = compile_mechanism_to_improvement_operator(
            knowledge,
            &binding,
            &credit,
            &promotion,
            Vec::new(),
        )
        .unwrap();
        (binding, promotion, credit, compiled)
    }

    #[test]
    fn triangular_promotion_and_credit_are_causally_bound() {
        let knowledge = knowledge("K-CREDIT", "M-CREDIT", "BROKEN", "REPAIRED");
        let candidate = sha256(b"candidate");
        let promotion = promotion(&candidate, "CREDIT");
        let credit = assign_causal_credit(
            &knowledge,
            &trace(&knowledge, &candidate, "CREDIT"),
            &promotion,
        );
        assert!(promotion.accepted);
        assert_eq!(promotion.independent_authorities, 3);
        assert_eq!(promotion.proposer_self_approval_events, 0);
        assert_eq!(credit.disposition, CreditDispositionIR::Credited);
        assert_eq!(credit.attributable_capability_gain, 12);

        let mut forged = promotion.clone();
        forged.candidate_sha256 = sha256(b"forged");
        let rejected = assign_causal_credit(
            &knowledge,
            &trace(&knowledge, &forged.candidate_sha256, "FORGED"),
            &forged,
        );
        assert_eq!(rejected.disposition, CreditDispositionIR::Rejected);
        assert!(rejected
            .rejection_reasons
            .iter()
            .any(|reason| reason == "CREDIT_PROMOTION_NOT_ACCEPTED_OR_UNBOUND"));
    }

    #[test]
    fn mechanism_generalization_requires_a_credited_fresh_identity() {
        let first = knowledge("K-FIRST", "M-FIRST", "P-FIRST", "E-FIRST");
        let second = knowledge("K-SECOND", "M-SECOND", "P-SECOND", "E-SECOND");
        let fresh = knowledge("K-FRESH", "M-FRESH", "P-FRESH", "E-FRESH");
        let (_, first_credit) = credited(&first, &sha256(b"first"), "FIRST");
        let (_, second_credit) = credited(&second, &sha256(b"second"), "SECOND");
        let schemas = generalize_credited_mechanisms(
            &[first.clone(), second.clone()],
            &[first_credit, second_credit],
        );
        assert_eq!(schemas.len(), 1);
        assert!(schemas[0].fresh_identity_transfer_required);

        let (_, fresh_credit) = credited(&fresh, &sha256(b"fresh"), "FRESH");
        let receipt = evaluate_generalization_transfer(&schemas[0], &fresh, &fresh_credit);
        assert!(receipt.accepted, "{:?}", receipt.rejection_reasons);

        let rejected = evaluate_generalization_transfer(&schemas[0], &first, &fresh_credit);
        assert!(!rejected.accepted);
        assert!(rejected
            .rejection_reasons
            .iter()
            .any(|reason| reason == "GENERALIZATION_IDENTITY_NOT_FRESH"));
    }

    #[test]
    fn active_experiments_and_counterexamples_change_the_next_candidate() {
        let hypotheses = vec![
            HypothesisIR {
                hypothesis_id: "H-A".to_string(),
            },
            HypothesisIR {
                hypothesis_id: "H-B".to_string(),
            },
            HypothesisIR {
                hypothesis_id: "H-C".to_string(),
            },
        ];
        let candidates = vec![
            ActiveExperimentCandidateIR {
                experiment_id: "LOW-INFORMATION".to_string(),
                predictions: hypotheses
                    .iter()
                    .map(|hypothesis| ExperimentPredictionIR {
                        hypothesis_id: hypothesis.hypothesis_id.clone(),
                        observation_signature: "SAME".to_string(),
                    })
                    .collect(),
                reliability_millis: 1_000,
                cost_millis: 1,
                risk_millis: 0,
                read_only: true,
            },
            ActiveExperimentCandidateIR {
                experiment_id: "HIGH-INFORMATION".to_string(),
                predictions: vec![
                    ExperimentPredictionIR {
                        hypothesis_id: "H-A".to_string(),
                        observation_signature: "A".to_string(),
                    },
                    ExperimentPredictionIR {
                        hypothesis_id: "H-B".to_string(),
                        observation_signature: "B".to_string(),
                    },
                    ExperimentPredictionIR {
                        hypothesis_id: "H-C".to_string(),
                        observation_signature: "C".to_string(),
                    },
                ],
                reliability_millis: 900,
                cost_millis: 10,
                risk_millis: 0,
                read_only: true,
            },
        ];
        assert_eq!(
            select_information_gain_experiment(&hypotheses, &candidates)
                .unwrap()
                .experiment_id,
            "HIGH-INFORMATION"
        );

        let failed_hash = sha256(b"failed");
        let counterexample = semantic_counterexample(
            &failed_hash,
            vec![literal("EXPECTED")],
            vec![literal("WRONG")],
            vec!["OWNER_RECEIVER_IDENTITY".to_string()],
            vec!["E0308".to_string()],
        )
        .unwrap();
        let ranked = revise_candidates_from_counterexamples(
            &[
                RevisionCandidateIR {
                    candidate_id: "REPEAT".to_string(),
                    candidate_sha256: failed_hash,
                    predicted_effects: vec![literal("EXPECTED")],
                    repaired_invariant_classes: Vec::new(),
                    addressed_compiler_error_codes: Vec::new(),
                    estimated_validation_cost_millis: 1,
                },
                RevisionCandidateIR {
                    candidate_id: "SEMANTIC-REVISION".to_string(),
                    candidate_sha256: sha256(b"revised"),
                    predicted_effects: vec![literal("EXPECTED")],
                    repaired_invariant_classes: vec!["OWNER_RECEIVER_IDENTITY".to_string()],
                    addressed_compiler_error_codes: vec!["E0308".to_string()],
                    estimated_validation_cost_millis: 10,
                },
            ],
            &[counterexample],
        );
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].candidate.candidate_id, "SEMANTIC-REVISION");
        assert!(ranked[0].semantic_feedback_score > 0);
    }

    #[test]
    fn operator_repository_executes_single_and_productive_composite_programs() {
        let first = knowledge("K-REPLACE", "M-REPLACE", "P-REPLACE", "E-REPLACE");
        let second = knowledge("K-INSERT", "M-INSERT", "P-INSERT", "E-INSERT");
        let before_first = "pub fn adjust(value: i64) -> i64 { value + 1 }\n";
        let after_first = "pub fn adjust(value: i64) -> i64 { value - 1 }\n";
        let before_second = "pub fn identity(value: i64) -> i64 { value }\n";
        let after_second = concat!(
            "pub fn helper(value: i64) -> i64 { value * 2 }\n",
            "pub fn identity(value: i64) -> i64 { value }\n"
        );
        let (_, _, _, first_bundle) = bundle(
            &first,
            "src/first.rs",
            before_first,
            after_first,
            "REPLACE_EXPRESSION",
            "REPLACE",
        );
        let (_, _, _, second_bundle) = bundle(
            &second,
            "src/second.rs",
            before_second,
            after_second,
            "INSERT_CALLABLE",
            "INSERT",
        );
        assert_ne!(
            first_bundle.operator.operator_id,
            second_bundle.operator.operator_id
        );
        let execution =
            execute_mechanism_improvement_operator(&first_bundle, before_first).unwrap();
        assert!(execution.applicable);
        assert_eq!(execution.candidate_source.as_deref(), Some(after_first));

        let mut repository = CompoundOperatorRepositoryIR::bounded(8);
        repository.insert_promoted(first_bundle.clone(), 1).unwrap();
        repository
            .insert_promoted(second_bundle.clone(), 1)
            .unwrap();
        let ids = vec![
            first_bundle.operator.operator_id.clone(),
            second_bundle.operator.operator_id.clone(),
        ];
        repository
            .record_outcome(&ids, "FAMILY-A", 1, true, 20)
            .unwrap();
        repository
            .record_outcome(&ids, "FAMILY-B", 2, true, 18)
            .unwrap();
        let graphs = repository.compile_productive_composites();
        assert_eq!(graphs.len(), 1);
        assert!(repository
            .profiles
            .iter()
            .all(|profile| profile.lifecycle == OperatorLifecycleIR::Hot));
        let sources = BTreeMap::from([
            ("src/first.rs".to_string(), before_first.to_string()),
            ("src/second.rs".to_string(), before_second.to_string()),
        ]);
        let executions = execute_productive_composite(&repository, &graphs[0], &sources).unwrap();
        assert_eq!(executions.len(), 2);
        assert!(executions.iter().all(|execution| execution.applicable));
    }

    #[test]
    fn operator_lifecycle_ages_and_quarantines_repeated_failures() {
        let item = knowledge("K-LIFECYCLE", "M-LIFECYCLE", "P-LIFE", "E-LIFE");
        let before = "pub fn lifecycle(value: i64) -> i64 { value + 1 }\n";
        let after = "pub fn lifecycle(value: i64) -> i64 { value - 1 }\n";
        let (_, _, _, bundle) = bundle(
            &item,
            "src/lifecycle.rs",
            before,
            after,
            "LIFECYCLE_REPLACE",
            "LIFECYCLE",
        );
        let operator_id = bundle.operator.operator_id.clone();
        let mut repository = CompoundOperatorRepositoryIR::bounded(1);
        repository.insert_promoted(bundle, 1).unwrap();
        repository.advance_generation(4).unwrap();
        assert_eq!(repository.profiles[0].lifecycle, OperatorLifecycleIR::Cold);
        for generation in 4..=6 {
            repository
                .record_outcome(
                    std::slice::from_ref(&operator_id),
                    &format!("FAILED-{generation}"),
                    generation,
                    false,
                    10,
                )
                .unwrap();
        }
        assert_eq!(
            repository.profiles[0].lifecycle,
            OperatorLifecycleIR::Quarantined
        );
        assert!(repository
            .recall(&["source-repair".to_string()], &[], 8)
            .is_empty());
    }

    #[test]
    fn closed_cycle_exposes_all_growth_products_without_external_calls() {
        let first = knowledge("K-CYCLE-A", "M-CYCLE-A", "P-A", "E-A");
        let second = knowledge("K-CYCLE-B", "M-CYCLE-B", "P-B", "E-B");
        let before_first = "pub fn left(value: i64) -> i64 { value + 1 }\n";
        let after_first = "pub fn left(value: i64) -> i64 { value - 1 }\n";
        let before_second = "pub fn right(value: i64) -> i64 { value }\n";
        let after_second = concat!(
            "pub fn extra(value: i64) -> i64 { value * 3 }\n",
            "pub fn right(value: i64) -> i64 { value }\n"
        );
        let (binding_a, promotion_a, credit_a, bundle_a) = bundle(
            &first,
            "src/left.rs",
            before_first,
            after_first,
            "CYCLE_REPLACE",
            "CYCLE-A",
        );
        let (binding_b, promotion_b, credit_b, bundle_b) = bundle(
            &second,
            "src/right.rs",
            before_second,
            after_second,
            "CYCLE_INSERT",
            "CYCLE-B",
        );
        let mut repository = CompoundOperatorRepositoryIR::bounded(16);
        repository.insert_promoted(bundle_a.clone(), 0).unwrap();
        repository.insert_promoted(bundle_b.clone(), 0).unwrap();
        let operator_ids = vec![
            bundle_a.operator.operator_id.clone(),
            bundle_b.operator.operator_id.clone(),
        ];
        let failed_hash = sha256(b"cycle-failed");
        let counterexample = semantic_counterexample(
            &failed_hash,
            vec![literal("E-A")],
            Vec::new(),
            vec!["TYPE_EFFECT".to_string()],
            vec!["E0308".to_string()],
        )
        .unwrap();
        let request = CompoundGrowthCycleRequestIR {
            schema: COMPOUND_GROWTH_SCHEMA.to_string(),
            generation: 3,
            mechanisms: vec![first.clone(), second.clone()],
            execution_traces: vec![
                trace(
                    &first,
                    &binding_a.structural_program.target_source_sha256,
                    "CYCLE-A",
                ),
                trace(
                    &second,
                    &binding_b.structural_program.target_source_sha256,
                    "CYCLE-B",
                ),
            ],
            promotion_evidence: vec![
                TriangularPromotionEvidenceIR {
                    schema: COMPOUND_GROWTH_SCHEMA.to_string(),
                    promotion_id: promotion_a.promotion_id.clone(),
                    proposer_authority_id: "CORE-CYCLE-A".to_string(),
                    candidate_sha256: promotion_a.candidate_sha256.clone(),
                    attestations: [
                        (AttestationRoleIR::CoreExecution, "CORE-CYCLE-A", "core"),
                        (
                            AttestationRoleIR::IndependentEvaluator,
                            "EVALUATOR-CYCLE-A",
                            "evaluator",
                        ),
                        (
                            AttestationRoleIR::PublicObservation,
                            "PUBLIC-CYCLE-A",
                            "public",
                        ),
                    ]
                    .into_iter()
                    .map(|(role, authority, observation)| {
                        seal_attestation(
                            role,
                            authority,
                            &promotion_a.candidate_sha256,
                            &sha256(format!("{observation}:CYCLE-A").as_bytes()),
                            true,
                        )
                        .unwrap()
                    })
                    .collect(),
                },
                TriangularPromotionEvidenceIR {
                    schema: COMPOUND_GROWTH_SCHEMA.to_string(),
                    promotion_id: promotion_b.promotion_id.clone(),
                    proposer_authority_id: "CORE-CYCLE-B".to_string(),
                    candidate_sha256: promotion_b.candidate_sha256.clone(),
                    attestations: [
                        (AttestationRoleIR::CoreExecution, "CORE-CYCLE-B", "core"),
                        (
                            AttestationRoleIR::IndependentEvaluator,
                            "EVALUATOR-CYCLE-B",
                            "evaluator",
                        ),
                        (
                            AttestationRoleIR::PublicObservation,
                            "PUBLIC-CYCLE-B",
                            "public",
                        ),
                    ]
                    .into_iter()
                    .map(|(role, authority, observation)| {
                        seal_attestation(
                            role,
                            authority,
                            &promotion_b.candidate_sha256,
                            &sha256(format!("{observation}:CYCLE-B").as_bytes()),
                            true,
                        )
                        .unwrap()
                    })
                    .collect(),
                },
            ],
            source_bindings: vec![binding_a, binding_b],
            hypotheses: vec![
                HypothesisIR {
                    hypothesis_id: "H-A".to_string(),
                },
                HypothesisIR {
                    hypothesis_id: "H-B".to_string(),
                },
            ],
            experiment_candidates: vec![ActiveExperimentCandidateIR {
                experiment_id: "READ-ONLY-DIAGNOSTIC".to_string(),
                predictions: vec![
                    ExperimentPredictionIR {
                        hypothesis_id: "H-A".to_string(),
                        observation_signature: "A".to_string(),
                    },
                    ExperimentPredictionIR {
                        hypothesis_id: "H-B".to_string(),
                        observation_signature: "B".to_string(),
                    },
                ],
                reliability_millis: 900,
                cost_millis: 10,
                risk_millis: 0,
                read_only: true,
            }],
            counterexamples: vec![counterexample],
            revision_candidates: vec![RevisionCandidateIR {
                candidate_id: "CYCLE-REVISION".to_string(),
                candidate_sha256: sha256(b"cycle-revision"),
                predicted_effects: vec![literal("E-A")],
                repaired_invariant_classes: vec!["TYPE_EFFECT".to_string()],
                addressed_compiler_error_codes: vec!["E0308".to_string()],
                estimated_validation_cost_millis: 10,
            }],
            repository,
            operator_outcomes: vec![
                OperatorOutcomeIR {
                    operator_ids: operator_ids.clone(),
                    context_id: "CYCLE-FAMILY-A".to_string(),
                    succeeded: true,
                    validation_cost_millis: 20,
                },
                OperatorOutcomeIR {
                    operator_ids,
                    context_id: "CYCLE-FAMILY-B".to_string(),
                    succeeded: true,
                    validation_cost_millis: 18,
                },
            ],
        };
        let cycle = run_compound_growth_cycle(&request).unwrap();
        assert_eq!(cycle.promotion_receipts.len(), 2);
        assert!(cycle
            .promotion_receipts
            .iter()
            .all(|receipt| receipt.accepted));
        assert_eq!(cycle.causal_credit_receipts.len(), 2);
        assert!(cycle
            .causal_credit_receipts
            .iter()
            .all(|receipt| { receipt.disposition == CreditDispositionIR::Credited }));
        assert_eq!(cycle.generalized_schemas.len(), 1);
        assert_eq!(cycle.compiled_operator_bundle_ids.len(), 2);
        assert!(cycle.selected_experiment.is_some());
        assert_eq!(cycle.revised_candidates.len(), 1);
        assert_eq!(cycle.productive_composite_graphs.len(), 1);
        assert_eq!(cycle.text_only_growth_events, 0);
        assert_eq!(cycle.external_model_calls, 0);
        assert!(valid_hash(&cycle.cycle_sha256));
        cycle.repository.validate().unwrap();

        assert_eq!(credit_a.disposition, CreditDispositionIR::Credited);
        assert_eq!(credit_b.disposition, CreditDispositionIR::Credited);
    }
}
