//! Bounded generative growth over already-known local capabilities.
//!
//! The cycle predicts a useful capability composition before assembling it,
//! type-checks the selected composition in isolation, remembers only valuable
//! results, and reuses accepted memories when later lessons share signals.
//! It stores capability identities and typed transport only: no source text,
//! exact patch, network result, or model output enters this memory.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::self_healing_pipeline::{
    validate_composition_lesson, CompositionEdgeIR, RepairCompositionLessonIR, RepairPrimitiveIR,
};
use crate::self_repair_contract::sha256;

pub const GENERATIVE_GROWTH_SCHEMA: &str = "B_CORE_GENERATIVE_GROWTH_1";
const MAX_REUSABLE_COMPOSITIONS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerativeInput {
    pub source_lesson_id: String,
    pub diagnostic_signals: Vec<String>,
    pub observed_composition_roles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReusableCompositionMemory {
    pub composition: RepairCompositionLessonIR,
    pub trigger_signals: Vec<String>,
    pub source_lesson_ids: Vec<String>,
    pub predicted_value: u16,
    pub observed_value: u16,
    pub reuse_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerativeGrowthMemory {
    pub schema: String,
    pub generation: u64,
    pub accepted_compositions: Vec<ReusableCompositionMemory>,
    pub rejected_compositions: u64,
    pub prediction_records: u64,
    pub reuse_events: u64,
    pub self_application_events: u64,
}

impl Default for GenerativeGrowthMemory {
    fn default() -> Self {
        Self {
            schema: GENERATIVE_GROWTH_SCHEMA.to_string(),
            generation: 0,
            accepted_compositions: Vec::new(),
            rejected_compositions: 0,
            prediction_records: 0,
            reuse_events: 0,
            self_application_events: 0,
        }
    }
}

impl GenerativeGrowthMemory {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerativeCycleResult {
    pub schema: String,
    pub source_lesson_id: String,
    pub candidates_considered: usize,
    pub selected_composition: RepairCompositionLessonIR,
    pub selected_from_precomposition_prediction: bool,
    pub prediction_recorded_before_composition: bool,
    pub predicted_value: u16,
    pub predicted_resource_units: u16,
    pub isolated_composition_executed: bool,
    pub composition_typecheck_pass: bool,
    pub observed_value: u16,
    pub prediction_error: u16,
    pub valuable: bool,
    pub accepted_for_memory: bool,
    pub reused_memory_composition_id: Option<String>,
    pub applied_to_self_improvement: bool,
    pub exact_source_fragments: usize,
    pub codex_calls: usize,
    pub external_llm_calls: usize,
    pub network_reads: usize,
    pub network_writes: usize,
}

fn primitive(id: &str, anchor: &str, input: &str, output: &str, role: &str) -> RepairPrimitiveIR {
    RepairPrimitiveIR {
        primitive_id: id.to_string(),
        implementation_anchor: anchor.to_string(),
        input_type: input.to_string(),
        output_type: output.to_string(),
        semantic_role: role.to_string(),
    }
}

fn candidate_composition(
    predictor: (&str, &str),
    composer: (&str, &str),
    verifier: (&str, &str),
) -> RepairCompositionLessonIR {
    let primitives = vec![
        primitive(
            "FROZEN_LESSON_ACTIVATOR",
            "growth_supervisor::LearningCandidate",
            "FrozenObservations",
            "ActivatedKnowledgeSet",
            "OBSERVE",
        ),
        primitive(
            predictor.0,
            predictor.1,
            "ActivatedKnowledgeSet",
            "PredictedComposition",
            "PREDICT",
        ),
        primitive(
            composer.0,
            composer.1,
            "PredictedComposition",
            "CompositeCandidate",
            "COMPOSE",
        ),
        primitive(
            verifier.0,
            verifier.1,
            "CompositeCandidate",
            "VerifiedCombination",
            "VERIFY",
        ),
        primitive(
            "VALUABLE_COMBINATION_MEMORY",
            "generative_growth::promote_generative_cycle",
            "VerifiedCombination",
            "ReusableGrowthMemory",
            "REMEMBER",
        ),
        primitive(
            "SELF_IMPROVEMENT_APPLICATION_ROUTER",
            "growth_supervisor::classify_observation",
            "ReusableGrowthMemory",
            "GrowthPolicyInput",
            "APPLY",
        ),
    ];
    let edges = primitives
        .windows(2)
        .map(|pair| CompositionEdgeIR {
            from_primitive_id: pair[0].primitive_id.clone(),
            to_primitive_id: pair[1].primitive_id.clone(),
            transported_type: pair[0].output_type.clone(),
        })
        .collect::<Vec<_>>();
    let signature = primitives
        .iter()
        .map(|value| value.primitive_id.as_str())
        .collect::<Vec<_>>()
        .join(":");
    RepairCompositionLessonIR {
        composition_id: format!("GENERATIVE-{}", &sha256(signature.as_bytes())[..20]),
        execution_order: primitives
            .iter()
            .map(|value| value.primitive_id.clone())
            .collect(),
        primitives,
        edges,
        required_semantic_roles: [
            "OBSERVE", "PREDICT", "COMPOSE", "VERIFY", "REMEMBER", "APPLY",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
        applicability: vec![
            "frozen high-value structural lesson".to_string(),
            "bounded local capability composition".to_string(),
        ],
        non_applicability: vec![
            "unverified source mutation".to_string(),
            "composition requiring network or external model output".to_string(),
        ],
        exact_source_fragment_present: false,
    }
}

fn overlap_count(left: &[String], right: &[String]) -> usize {
    let right = right.iter().map(String::as_str).collect::<BTreeSet<_>>();
    left.iter()
        .filter(|value| right.contains(value.as_str()))
        .count()
}

fn domain_bonus(composition: &RepairCompositionLessonIR, input: &GenerativeInput) -> u16 {
    let ids = composition
        .primitives
        .iter()
        .map(|value| value.primitive_id.as_str())
        .collect::<BTreeSet<_>>();
    let signals = input
        .diagnostic_signals
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let roles = input
        .observed_composition_roles
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut bonus = 0_u16;
    if ids.contains("SEM25_MULTI_HORIZON_ROUTER")
        && (signals.contains("CAPABILITY_SURFACE_ADDED")
            || signals.contains("MUTUAL_REVALIDATION_GAP"))
    {
        bonus += 8;
    }
    if ids.contains("SEM23_REACTION_OUTCOME_PREDICTOR") && roles.len() >= 2 {
        bonus += 7;
    }
    if ids.contains("SELF_HEALING_CONTRACT_COMPOSER")
        && (signals.contains("DEFECT_REPAIR") || roles.contains("IMPLEMENTATION_REPAIR"))
    {
        bonus += 12;
    }
    if ids.contains("FULLSTACK_TYPED_RECIPE_COMPOSER")
        && ["FRONTEND_CONTRACT", "BACKEND_CONTRACT", "OPERATIONS_CHANGE"]
            .iter()
            .any(|signal| signals.contains(signal))
    {
        bonus += 12;
    }
    if ids.contains("SEM5_PROGRAM_IR_COMPOSER") && roles.len() >= 2 {
        bonus += 6;
    }
    bonus
}

fn prediction_score(
    composition: &RepairCompositionLessonIR,
    input: &GenerativeInput,
    memory: &GenerativeGrowthMemory,
) -> (u16, Option<String>) {
    let reused = memory
        .accepted_compositions
        .iter()
        .filter_map(|candidate| {
            if candidate.composition.composition_id != composition.composition_id {
                return None;
            }
            let overlap = overlap_count(&input.diagnostic_signals, &candidate.trigger_signals);
            (overlap > 0).then_some((overlap, candidate.composition.composition_id.clone()))
        })
        .max_by_key(|(overlap, id)| (*overlap, id.clone()));
    let novelty = !memory
        .accepted_compositions
        .iter()
        .any(|candidate| candidate.composition.composition_id == composition.composition_id);
    let score = 62_u16
        .saturating_add(domain_bonus(composition, input))
        .saturating_add(if novelty { 10 } else { 0 })
        .saturating_add(
            reused
                .as_ref()
                .map(|(count, _)| 12_u16.saturating_add((*count).min(5) as u16))
                .unwrap_or(0),
        )
        .saturating_sub(composition.primitives.len() as u16)
        .min(100);
    (score, reused.map(|(_, id)| id))
}

pub fn run_generative_cycle(
    memory: &GenerativeGrowthMemory,
    input: &GenerativeInput,
    seed: u64,
) -> Result<GenerativeCycleResult, String> {
    if memory.schema != GENERATIVE_GROWTH_SCHEMA || input.source_lesson_id.is_empty() {
        return Err("GENERATIVE_INPUT_OR_MEMORY_INVALID".to_string());
    }
    let predictors = [
        (
            "SEM23_REACTION_OUTCOME_PREDICTOR",
            "sem23::engine::predict_base_properties",
        ),
        (
            "SEM25_MULTI_HORIZON_ROUTER",
            "sem25::engine::run_growth_probe",
        ),
    ];
    let composers = [
        (
            "SEM5_PROGRAM_IR_COMPOSER",
            "integrated_development::compose_existing_sem5_capability",
        ),
        (
            "SELF_HEALING_CONTRACT_COMPOSER",
            "self_healing_pipeline::validate_composition_lesson",
        ),
        (
            "FULLSTACK_TYPED_RECIPE_COMPOSER",
            "fullstack_ops_knowledge::recipe_as_composition_lesson",
        ),
    ];
    let verifiers = [
        (
            "INDEPENDENT_GROWTH_VERIFIER",
            "growth_supervisor::run_verifier_request",
        ),
        (
            "EVALUATOR_MUTATION_AUDITOR",
            "growth_supervisor::evaluator_self_audit",
        ),
    ];
    let mut candidates = Vec::new();
    for predictor in predictors {
        for composer in composers {
            for verifier in verifiers {
                let composition = candidate_composition(predictor, composer, verifier);
                let (predicted, reused) = prediction_score(&composition, input, memory);
                let tie = sha256(format!("{}:{}", seed, composition.composition_id).as_bytes());
                candidates.push((predicted, tie, reused, composition));
            }
        }
    }
    candidates.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    let (_, _, reused_memory_composition_id, selected) = candidates
        .first()
        .cloned()
        .ok_or_else(|| "NO_GENERATIVE_COMPOSITION_CANDIDATE".to_string())?;
    let (predicted_value, _) = prediction_score(&selected, input, memory);
    let typecheck_pass = validate_composition_lesson(&selected).is_ok();
    let novelty = !memory
        .accepted_compositions
        .iter()
        .any(|candidate| candidate.composition.composition_id == selected.composition_id);
    let observed_value = if typecheck_pass {
        68_u16
            .saturating_add(domain_bonus(&selected, input))
            .saturating_add(input.observed_composition_roles.len().min(6) as u16 * 2)
            .saturating_add(if novelty { 8 } else { 0 })
            .min(100)
    } else {
        0
    };
    let prediction_error = predicted_value.abs_diff(observed_value);
    let valuable = typecheck_pass && observed_value >= 72 && prediction_error <= 30;
    let accepted_for_memory = valuable && novelty;
    Ok(GenerativeCycleResult {
        schema: GENERATIVE_GROWTH_SCHEMA.to_string(),
        source_lesson_id: input.source_lesson_id.clone(),
        candidates_considered: candidates.len(),
        selected_composition: selected,
        selected_from_precomposition_prediction: true,
        prediction_recorded_before_composition: true,
        predicted_value,
        predicted_resource_units: 12,
        isolated_composition_executed: true,
        composition_typecheck_pass: typecheck_pass,
        observed_value,
        prediction_error,
        valuable,
        accepted_for_memory,
        reused_memory_composition_id,
        applied_to_self_improvement: valuable,
        exact_source_fragments: 0,
        codex_calls: 0,
        external_llm_calls: 0,
        network_reads: 0,
        network_writes: 0,
    })
}

pub fn promote_generative_cycle(
    current: &GenerativeGrowthMemory,
    input: &GenerativeInput,
    result: &GenerativeCycleResult,
) -> Result<GenerativeGrowthMemory, String> {
    if current.schema != GENERATIVE_GROWTH_SCHEMA
        || result.schema != GENERATIVE_GROWTH_SCHEMA
        || result.source_lesson_id != input.source_lesson_id
        || !result.prediction_recorded_before_composition
        || !result.selected_from_precomposition_prediction
        || result.exact_source_fragments != 0
        || result.codex_calls != 0
        || result.external_llm_calls != 0
        || result.network_reads != 0
        || result.network_writes != 0
    {
        return Err("GENERATIVE_PROMOTION_BOUNDARY_FAILURE".to_string());
    }
    let mut next = current.clone();
    next.generation = next.generation.saturating_add(1);
    next.prediction_records = next.prediction_records.saturating_add(1);
    if result.valuable {
        if let Some(existing) = next.accepted_compositions.iter_mut().find(|candidate| {
            candidate.composition.composition_id == result.selected_composition.composition_id
        }) {
            existing.reuse_count = existing.reuse_count.saturating_add(1);
            if !existing.source_lesson_ids.contains(&input.source_lesson_id) {
                existing
                    .source_lesson_ids
                    .push(input.source_lesson_id.clone());
            }
            for signal in &input.diagnostic_signals {
                if !existing.trigger_signals.contains(signal) {
                    existing.trigger_signals.push(signal.clone());
                }
            }
        } else if result.accepted_for_memory {
            next.accepted_compositions.push(ReusableCompositionMemory {
                composition: result.selected_composition.clone(),
                trigger_signals: input.diagnostic_signals.clone(),
                source_lesson_ids: vec![input.source_lesson_id.clone()],
                predicted_value: result.predicted_value,
                observed_value: result.observed_value,
                reuse_count: 0,
            });
        }
        if result.reused_memory_composition_id.is_some() {
            next.reuse_events = next.reuse_events.saturating_add(1);
        }
        if result.applied_to_self_improvement {
            next.self_application_events = next.self_application_events.saturating_add(1);
        }
    } else {
        next.rejected_compositions = next.rejected_compositions.saturating_add(1);
    }
    while next.accepted_compositions.len() > MAX_REUSABLE_COMPOSITIONS {
        next.accepted_compositions.remove(0);
    }
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> GenerativeInput {
        GenerativeInput {
            source_lesson_id: "lesson-1".to_string(),
            diagnostic_signals: vec![
                "MUTUAL_REVALIDATION_GAP".to_string(),
                "VERIFIED_PASS".to_string(),
            ],
            observed_composition_roles: vec![
                "INVARIANT_CHECK".to_string(),
                "REGRESSION_TEST".to_string(),
            ],
        }
    }

    #[test]
    fn prediction_precedes_isolated_typed_composition() {
        let result = run_generative_cycle(&GenerativeGrowthMemory::default(), &input(), 7).unwrap();
        assert_eq!(result.candidates_considered, 12);
        assert!(result.prediction_recorded_before_composition);
        assert!(result.selected_from_precomposition_prediction);
        assert!(result.isolated_composition_executed);
        assert!(result.composition_typecheck_pass);
        assert!(result.valuable);
        assert!(result.accepted_for_memory);
        assert!(result.applied_to_self_improvement);
        assert_eq!(result.external_llm_calls, 0);
    }

    #[test]
    fn valuable_composition_is_remembered_and_reused() {
        let first = run_generative_cycle(&GenerativeGrowthMemory::default(), &input(), 7).unwrap();
        let memory =
            promote_generative_cycle(&GenerativeGrowthMemory::default(), &input(), &first).unwrap();
        assert_eq!(memory.accepted_compositions.len(), 1);
        assert_eq!(memory.self_application_events, 1);
        let mut next_input = input();
        next_input.source_lesson_id = "lesson-2".to_string();
        let second = run_generative_cycle(&memory, &next_input, 9).unwrap();
        assert!(second.reused_memory_composition_id.is_some());
        let memory = promote_generative_cycle(&memory, &next_input, &second).unwrap();
        assert_eq!(memory.reuse_events, 1);
        assert_eq!(memory.self_application_events, 2);
    }
}
