//! Bounded generative growth over already-known local capabilities.
//!
//! The cycle predicts a useful capability composition before assembling it,
//! type-checks the selected composition in isolation, remembers only valuable
//! results, and reuses accepted memories when later lessons share signals.
//! It stores capability identities and typed transport only: no source text,
//! exact patch, network result, or model output enters this memory.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::autonomous_source_mutation::execute_improvement_operator_behavioral_canary;
use crate::fullstack_ops_knowledge::{execute_fullstack_recipe_behavioral_canary, promoted_bundle};
use crate::integrated_development::execute_behavioral_composition_canary;
use crate::self_healing_pipeline::{
    validate_composition_lesson, CompositionEdgeIR, RepairCompositionLessonIR, RepairPrimitiveIR,
};
use crate::self_repair_contract::sha256;
use crate::sem23_engine::{
    predict_base_properties, GenerativeRequest, PROPERTY_FAMILY_TRANSFER, PROPERTY_REACTION_LAW,
    PROPERTY_RECURSIVE_CLOSURE, PROPERTY_STRUCTURED_EMERGENCE,
};
use crate::sem25_engine::{run_growth_probe, GrowthProbeRequest};

pub const GENERATIVE_GROWTH_SCHEMA: &str = "B_CORE_GENERATIVE_GROWTH_1";
const MAX_REUSABLE_COMPOSITIONS: usize = 64;
const MAX_COMPOSITION_TRIALS: usize = 256;
const MAX_VERIFIED_ARTIFACTS_PER_CYCLE: usize = 32;
const MAX_SEM5_VERIFIED_ARTIFACTS: u64 = 64;
const MAX_IMPROVEMENT_OPERATOR_SELECTORS: usize = 25;
// The 5 x 5 canary product lowers to 20 distinct generalized operator
// identities because normalized edit/postcondition contracts intentionally
// merge five scenario aliases.
const MAX_IMPROVEMENT_OPERATOR_VERIFIED_ARTIFACTS: u64 = 20;
const MAX_FULLSTACK_VERIFIED_ARTIFACTS: u64 = 3;
const MAX_ARTIFACT_CONTEXT_ATTEMPTS: usize = MAX_VERIFIED_ARTIFACTS_PER_CYCLE * 4;
const FRONTIER_EVIDENCE_CONTRACT_REVISION: u64 = 2;
const BEHAVIORAL_HEURISTIC_EXCLUSION_CONTRACT_REVISION: u64 = 4;
const BEHAVIORAL_VALUE_CONTRACT_REVISION: u64 = 5;
const GENERATIVE_PREDICTORS: [(&str, &str); 2] = [
    (
        "SEM23_REACTION_OUTCOME_PREDICTOR",
        "sem23::engine::predict_base_properties",
    ),
    (
        "SEM25_MULTI_HORIZON_ROUTER",
        "sem25::engine::run_growth_probe",
    ),
];
const GENERATIVE_COMPOSERS: [(&str, &str); 4] = [
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
    (
        "IMPROVEMENT_OPERATOR_PROGRAM_COMPOSER",
        "autonomous_source_mutation::execute_improvement_operator_behavioral_canary",
    ),
];
const GENERATIVE_VERIFIERS: [(&str, &str); 1] = [(
    "INDEPENDENT_GROWTH_VERIFIER",
    "growth_supervisor::run_verifier_request",
)];
const STATIC_GENERATIVE_CANDIDATE_COUNT: usize =
    GENERATIVE_PREDICTORS.len() * GENERATIVE_COMPOSERS.len() * GENERATIVE_VERIFIERS.len();

fn is_zero(value: &u64) -> bool {
    *value == 0
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerativeInput {
    pub source_lesson_id: String,
    pub diagnostic_signals: Vec<String>,
    pub observed_composition_roles: Vec<String>,
    pub learning_score: u16,
    pub verification_evidence_count: usize,
    pub measured_performance_gain: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReusableCompositionMemory {
    pub composition: RepairCompositionLessonIR,
    pub trigger_signals: Vec<String>,
    pub source_lesson_ids: Vec<String>,
    pub predicted_value: u16,
    pub observed_value: u16,
    pub reuse_count: u64,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub context_use_counts: BTreeMap<String, u64>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub successful_uses: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub observed_value_total: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub verified_artifact_sha256s: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub verified_artifact_contexts: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerativeCompositionTrial {
    pub composition_id: String,
    pub context_sha256: String,
    pub predicted_value: u16,
    pub observed_value: u16,
    pub valuable: bool,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub composition_trials: Vec<GenerativeCompositionTrial>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub exploration_events: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub productive_reuse_events: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub frontier_advance_events: u64,
    /// Number of distinct behaviorally verified capability artifacts, kept
    /// separate from composition-selection events. One campaign can now
    /// validate a bounded family instead of being hard-capped at +1 unit.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub frontier_capability_units: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub unverified_frontier_candidate_events: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub legacy_unverified_frontier_advance_events: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub legacy_wrapper_frontier_advance_events: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub behavioral_verification_events: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub redundant_selection_events: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub prediction_absolute_error_total: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub calibrated_prediction_records: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub legacy_uncalibrated_prediction_error_total: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub behavioral_value_contract_revision: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub legacy_heuristic_composition_trials: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub legacy_heuristic_accepted_compositions: u64,
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
            composition_trials: Vec::new(),
            exploration_events: 0,
            productive_reuse_events: 0,
            frontier_advance_events: 0,
            frontier_capability_units: 0,
            unverified_frontier_candidate_events: 0,
            legacy_unverified_frontier_advance_events: 0,
            legacy_wrapper_frontier_advance_events: 0,
            behavioral_verification_events: 0,
            redundant_selection_events: 0,
            prediction_absolute_error_total: 0,
            calibrated_prediction_records: 0,
            legacy_uncalibrated_prediction_error_total: 0,
            behavioral_value_contract_revision: BEHAVIORAL_VALUE_CONTRACT_REVISION,
            legacy_heuristic_composition_trials: 0,
            legacy_heuristic_accepted_compositions: 0,
        }
    }
}

impl GenerativeGrowthMemory {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }

    pub fn distinct_verified_artifact_count(&self) -> u64 {
        self.accepted_compositions
            .iter()
            .flat_map(|composition| composition.verified_artifact_sha256s.iter())
            .collect::<BTreeSet<_>>()
            .len()
            .min(u64::MAX as usize) as u64
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerativeCycleResult {
    pub schema: String,
    pub source_lesson_id: String,
    pub candidates_considered: usize,
    /// Static catalog entries rejected before prediction because their
    /// composer cannot produce a behaviorally testable artifact for this
    /// input. These entries are not exploration arms and spend no campaign
    /// execution budget.
    #[serde(default)]
    pub behaviorally_inapplicable_candidates_screened: usize,
    pub selected_composition: RepairCompositionLessonIR,
    pub selected_from_precomposition_prediction: bool,
    pub prediction_recorded_before_composition: bool,
    pub predicted_value: u16,
    #[serde(default)]
    pub selection_score: u16,
    pub predicted_resource_units: u16,
    pub isolated_composition_executed: bool,
    pub composition_typecheck_pass: bool,
    #[serde(default)]
    pub behavioral_composition_executed: bool,
    #[serde(default)]
    pub behavioral_verification_sha256: Option<String>,
    #[serde(default)]
    pub behavioral_execution_receipt: Option<BehavioralCompositionExecutionReceipt>,
    #[serde(default)]
    pub observed_value_is_heuristic_proxy: bool,
    pub observed_value: u16,
    pub prediction_error: u16,
    pub valuable: bool,
    pub accepted_for_memory: bool,
    pub reused_memory_composition_id: Option<String>,
    pub applied_to_self_improvement: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub applied_policy_signals: Vec<String>,
    #[serde(default)]
    pub context_sha256: String,
    #[serde(default)]
    pub exploration_selected: bool,
    #[serde(default)]
    pub prior_composition_trials: u64,
    #[serde(default)]
    pub prior_context_trials: u64,
    #[serde(default)]
    pub productive_reuse: bool,
    #[serde(default)]
    pub novel_context_transfer_candidate: bool,
    #[serde(default)]
    pub novel_verified_artifact: bool,
    #[serde(default)]
    pub verified_artifact_count: usize,
    #[serde(default)]
    pub novel_verified_artifact_count: usize,
    #[serde(default)]
    pub unverified_frontier_candidate: bool,
    #[serde(default)]
    pub frontier_advance: bool,
    #[serde(default)]
    pub frontier_advance_units: u64,
    pub exact_source_fragments: usize,
    pub codex_calls: usize,
    pub external_llm_calls: usize,
    pub network_reads: usize,
    pub network_writes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BehavioralCompositionExecutionReceipt {
    pub schema: String,
    pub context_sha256: String,
    pub predictor_id: String,
    pub predictor_output_sha256: String,
    pub composer_id: String,
    pub composite_artifact_sha256: Option<String>,
    pub verifier_id: String,
    pub verifier_output_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub verified_artifacts: Vec<VerifiedBehavioralArtifact>,
    #[serde(default)]
    pub cases_executed: usize,
    #[serde(default)]
    pub cases_passed: usize,
    pub executed: bool,
    pub abstention_reason: Option<String>,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifiedBehavioralArtifact {
    pub artifact_context_sha256: String,
    pub artifact_sha256: String,
    pub cases_executed: usize,
    pub cases_passed: usize,
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

fn context_sha256(input: &GenerativeInput) -> String {
    let signals = input
        .diagnostic_signals
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(":");
    let roles = input
        .observed_composition_roles
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(":");
    sha256(
        format!(
            "signals={signals}|roles={roles}|measured_gain={}",
            input.measured_performance_gain
        )
        .as_bytes(),
    )
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
    if ids.contains("IMPROVEMENT_OPERATOR_PROGRAM_COMPOSER")
        && (signals.contains("CAPABILITY_SURFACE_ADDED")
            || signals.contains("DEFECT_REPAIR")
            || roles.contains("IMPLEMENTATION"))
    {
        bonus += 10;
    }
    bonus
}

fn applicable_policy_signals(
    composition: &RepairCompositionLessonIR,
    input: &GenerativeInput,
) -> Vec<String> {
    let ids = composition
        .primitives
        .iter()
        .map(|value| value.primitive_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut supported = BTreeSet::new();
    if ids.contains("SEM25_MULTI_HORIZON_ROUTER") {
        supported.extend(["MUTUAL_REVALIDATION_GAP", "CAPABILITY_SURFACE_ADDED"]);
    }
    if ids.contains("SELF_HEALING_CONTRACT_COMPOSER") {
        supported.extend(["DEFECT_REPAIR", "ERROR_HANDLING_ADDED", "VALIDATION_ADDED"]);
    }
    if ids.contains("FULLSTACK_TYPED_RECIPE_COMPOSER") {
        supported.extend(["FRONTEND_CONTRACT", "BACKEND_CONTRACT", "OPERATIONS_CHANGE"]);
    }
    if ids.contains("SEM5_PROGRAM_IR_COMPOSER") {
        supported.extend(["CODE_CHANGE", "REFACTOR", "CAPABILITY_SURFACE_ADDED"]);
    }
    if ids.contains("IMPROVEMENT_OPERATOR_PROGRAM_COMPOSER") {
        supported.extend([
            "CODE_CHANGE",
            "REFACTOR",
            "CAPABILITY_SURFACE_ADDED",
            "DEFECT_REPAIR",
            "VALIDATION_ADDED",
        ]);
    }
    input
        .diagnostic_signals
        .iter()
        .filter(|signal| supported.contains(signal.as_str()))
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn selected_stage<'a>(
    composition: &'a RepairCompositionLessonIR,
    role: &str,
) -> Result<&'a str, String> {
    composition
        .primitives
        .iter()
        .find(|primitive| primitive.semantic_role == role)
        .map(|primitive| primitive.primitive_id.as_str())
        .ok_or_else(|| format!("GENERATION_STAGE_MISSING:{role}"))
}

fn execute_predictor(
    predictor_id: &str,
    input: &GenerativeInput,
    seed: u64,
) -> Result<String, String> {
    match predictor_id {
        "SEM23_REACTION_OUTCOME_PREDICTOR" => {
            let request = GenerativeRequest {
                representation_mode: 0,
                mechanism_mask: 0b0_1111,
                reactant_property_mask: 0b1_1111,
                reactant_count: 2 + input.observed_composition_roles.len().min(4) as u8,
                composite_reactant_count: 1,
                topology_code: 3,
                stoichiometry_code: 1,
                desired_property_mask: PROPERTY_STRUCTURED_EMERGENCE | PROPERTY_RECURSIVE_CLOSURE,
                predicted_property_mask: 0,
                family_prior_mask: PROPERTY_FAMILY_TRANSFER,
                reaction_law_mask: PROPERTY_REACTION_LAW,
                new_element_property_mask: 0,
                recursive_depth: 2 + input.observed_composition_roles.len().min(4) as u8,
                scale: 32,
                seed: seed.max(1),
                required_assumptions: 0,
                local_codebook: true,
            };
            let prediction = predict_base_properties(&request);
            Ok(sha256(
                format!("{}:{}:{prediction}", context_sha256(input), seed.max(1)).as_bytes(),
            ))
        }
        "SEM25_MULTI_HORIZON_ROUTER" => {
            let epoch = (seed % 24 + 1) as u8;
            let result = run_growth_probe(GrowthProbeRequest {
                arm_code: 3,
                epoch,
                seed: seed.max(1),
                gap_code: 1 + (input.diagnostic_signals.len() % 5) as u8,
                required_properties_mask: 1_u64 << (input.diagnostic_signals.len() % 32),
                required_roles_mask: 1_u64 << (input.observed_composition_roles.len() % 32),
                resource_ceiling: 24,
                total_reaction_objects: 64,
                theoretical_reaction_space: 10_000,
                growth_routing_laws: 2,
                growth_routing_schemas: 2,
                disable_growth_opportunity_index: false,
                disable_multi_horizon: false,
                disable_routing_laws: false,
                disable_future_affordances: false,
                disable_frontier_portfolio: false,
                disable_dead_end_knowledge: false,
            })?;
            if result.future_instance_leakage
                || result.open_loop_multi_step_self_modification
                || result.full_growth_opportunity_scan
                || result.full_reaction_space_enumeration
            {
                return Err("SEM25_BEHAVIORAL_PREDICTOR_BOUNDARY_FAILURE".to_string());
            }
            Ok(sha256(
                format!(
                    "{}:{}:{}:{}:{}",
                    context_sha256(input),
                    result.selected_opportunity.opportunity_id,
                    result.selected_prediction_horizon,
                    result.predicted_future_affordances,
                    result.observed_future_affordances
                )
                .as_bytes(),
            ))
        }
        _ => Err(format!("UNKNOWN_GENERATIVE_PREDICTOR:{predictor_id}")),
    }
}

fn execute_composer(
    composer_id: &str,
    selected: &RepairCompositionLessonIR,
    input: &GenerativeInput,
    context: &str,
    artifact_family_width: usize,
    previously_verified: &BTreeSet<String>,
) -> Result<(Vec<VerifiedBehavioralArtifact>, Option<String>), String> {
    match composer_id {
        "SEM5_PROGRAM_IR_COMPOSER" => {
            if input.observed_composition_roles.len() < 2 {
                return Ok((
                    Vec::new(),
                    Some("SEM5_REQUIRES_AT_LEAST_TWO_OBSERVED_ROLES".to_string()),
                ));
            }
            if artifact_family_width == 0 {
                return Ok((
                    Vec::new(),
                    Some("VERIFIED_ARTIFACT_CAPACITY_REACHED".to_string()),
                ));
            }
            let target_width = artifact_family_width.clamp(1, MAX_VERIFIED_ARTIFACTS_PER_CYCLE);
            let mut artifacts = Vec::new();
            let mut artifact_hashes = previously_verified.clone();
            for ordinal in 0..MAX_ARTIFACT_CONTEXT_ATTEMPTS {
                if artifacts.len() >= target_width {
                    break;
                }
                let artifact_context = if ordinal == 0 {
                    context.to_string()
                } else {
                    sha256(
                        format!(
                            "{context}:{}:{ordinal}:BOUND_ARTIFACT_FAMILY",
                            selected.composition_id
                        )
                        .as_bytes(),
                    )
                };
                let receipt = execute_behavioral_composition_canary(&artifact_context)?;
                if receipt.cases_executed == 0 || receipt.cases_passed != receipt.cases_executed {
                    return Err("SEM5_BEHAVIORAL_CANARY_INCOMPLETE".to_string());
                }
                if artifact_hashes.insert(receipt.program_ir_sha256.clone()) {
                    artifacts.push(VerifiedBehavioralArtifact {
                        artifact_context_sha256: artifact_context,
                        artifact_sha256: receipt.program_ir_sha256,
                        cases_executed: receipt.cases_executed,
                        cases_passed: receipt.cases_passed,
                    });
                }
            }
            if artifacts.is_empty() {
                return Err("SEM5_BEHAVIORAL_ARTIFACT_FAMILY_EMPTY".to_string());
            }
            Ok((artifacts, None))
        }
        "SELF_HEALING_CONTRACT_COMPOSER" => {
            // The present self-healing API needs a frozen defect contract and
            // a promoted repair lesson.  A generic growth input has neither,
            // so treating graph validation as execution would manufacture a
            // capability result.  Keep the candidate but explicitly abstain.
            Ok((
                Vec::new(),
                Some("SELF_HEALING_REQUIRES_FROZEN_REPAIR_SCENARIO".to_string()),
            ))
        }
        "FULLSTACK_TYPED_RECIPE_COMPOSER" => {
            if artifact_family_width == 0 {
                return Ok((
                    Vec::new(),
                    Some("FULLSTACK_RECIPE_ARTIFACT_CAPACITY_REACHED".to_string()),
                ));
            }
            let bundle = promoted_bundle();
            let target_width = artifact_family_width.clamp(1, MAX_VERIFIED_ARTIFACTS_PER_CYCLE);
            let mut artifacts = Vec::new();
            let mut artifact_hashes = previously_verified.clone();
            let mut recipe_ids = bundle
                .recipes
                .iter()
                .map(|recipe| recipe.recipe_id.as_str())
                .collect::<Vec<_>>();
            recipe_ids.sort_unstable();
            for recipe_id in recipe_ids {
                if artifacts.len() >= target_width {
                    break;
                }
                let receipt = execute_fullstack_recipe_behavioral_canary(&bundle, recipe_id)?;
                if receipt.cases_executed == 0
                    || receipt.cases_passed != receipt.cases_executed
                    || !receipt.exact_pipeline_observed
                    || !receipt.wrong_input_contract_rejected
                    || !receipt.reordered_pipeline_rejected
                {
                    return Err("FULLSTACK_BEHAVIORAL_CANARY_INCOMPLETE".to_string());
                }
                if artifact_hashes.insert(receipt.behavioral_artifact_sha256.clone()) {
                    artifacts.push(VerifiedBehavioralArtifact {
                        artifact_context_sha256: sha256(
                            format!("{context}:{recipe_id}:FULLSTACK_TYPED_RECIPE").as_bytes(),
                        ),
                        artifact_sha256: receipt.behavioral_artifact_sha256,
                        cases_executed: receipt.cases_executed,
                        cases_passed: receipt.cases_passed,
                    });
                }
            }
            if artifacts.is_empty() {
                return Ok((
                    Vec::new(),
                    Some("FULLSTACK_EXECUTABLE_RECIPE_UNIVERSE_SATURATED".to_string()),
                ));
            }
            Ok((artifacts, None))
        }
        "IMPROVEMENT_OPERATOR_PROGRAM_COMPOSER" => {
            if artifact_family_width == 0 {
                return Ok((
                    Vec::new(),
                    Some("IMPROVEMENT_OPERATOR_ARTIFACT_CAPACITY_REACHED".to_string()),
                ));
            }
            let target_width = artifact_family_width.clamp(1, MAX_VERIFIED_ARTIFACTS_PER_CYCLE);
            let mut artifacts = Vec::new();
            let mut artifact_hashes = previously_verified.clone();
            for ordinal in 0..MAX_ARTIFACT_CONTEXT_ATTEMPTS {
                if artifacts.len() >= target_width {
                    break;
                }
                // The canary maps the first eight context nibbles onto the
                // finite 5 x 5 operator algebra. Enumerate that selector
                // directly so a bounded family pass is guaranteed to visit
                // every operator instead of depending on hash luck. Keep the
                // remainder content-derived so receipts still bind the exact
                // frontier context and composition.
                let context_tail = sha256(
                    format!(
                        "{context}:{}:{ordinal}:IMPROVEMENT_OPERATOR_FAMILY",
                        selected.composition_id
                    )
                    .as_bytes(),
                );
                let operator_selector = ordinal % MAX_IMPROVEMENT_OPERATOR_SELECTORS;
                let artifact_context = format!("{operator_selector:08x}{}", &context_tail[8..]);
                let receipt = execute_improvement_operator_behavioral_canary(&artifact_context)?;
                if receipt.cases_executed == 0
                    || receipt.cases_passed != receipt.cases_executed
                    || !receipt.exact_candidate_observed
                    || !receipt.wrong_predecessor_rejected
                    || !receipt.tampered_target_rejected
                {
                    return Err("IMPROVEMENT_OPERATOR_BEHAVIORAL_CANARY_INCOMPLETE".to_string());
                }
                if artifact_hashes.insert(receipt.operator.operator_id.clone()) {
                    artifacts.push(VerifiedBehavioralArtifact {
                        artifact_context_sha256: artifact_context,
                        artifact_sha256: receipt.operator.operator_id,
                        cases_executed: receipt.cases_executed,
                        cases_passed: receipt.cases_passed,
                    });
                }
            }
            if artifacts.is_empty() {
                // Every bounded selector executed and verified, but all of
                // their generalized operator identities were already in
                // memory. The nominal selector-product upper bound may
                // contain identity aliases after normalization; that is safe
                // finite-substrate exhaustion, not a campaign failure.
                return Ok((
                    Vec::new(),
                    Some("IMPROVEMENT_OPERATOR_EXECUTABLE_UNIVERSE_SATURATED".to_string()),
                ));
            }
            Ok((artifacts, None))
        }
        _ => Err(format!("UNKNOWN_GENERATIVE_COMPOSER:{composer_id}")),
    }
}

/// The catalog also contains knowledge-only composers used by direct repair
/// workflows. They remain callable there, but must not compete for generative
/// frontier budget until this cycle can execute and independently observe the
/// artifact they produce. Graph validation or a typed recipe alone is not a
/// capability outcome.
fn composition_uses_composer(composition: &ReusableCompositionMemory, composer_id: &str) -> bool {
    composition.composition.primitives.iter().any(|primitive| {
        primitive.semantic_role == "COMPOSE" && primitive.primitive_id == composer_id
    })
}

fn distinct_verified_artifact_count_for_composer(
    memory: &GenerativeGrowthMemory,
    composer_id: &str,
) -> u64 {
    memory
        .accepted_compositions
        .iter()
        .filter(|composition| composition_uses_composer(composition, composer_id))
        .flat_map(|composition| composition.verified_artifact_sha256s.iter())
        .collect::<BTreeSet<_>>()
        .len()
        .min(u64::MAX as usize) as u64
}

fn verified_artifacts_for_composer(
    memory: &GenerativeGrowthMemory,
    composer_id: &str,
) -> BTreeSet<String> {
    memory
        .accepted_compositions
        .iter()
        .filter(|composition| composition_uses_composer(composition, composer_id))
        .flat_map(|composition| composition.verified_artifact_sha256s.iter().cloned())
        .collect()
}

fn verified_artifact_capacity(composer_id: &str) -> u64 {
    match composer_id {
        "SEM5_PROGRAM_IR_COMPOSER" => MAX_SEM5_VERIFIED_ARTIFACTS,
        "IMPROVEMENT_OPERATOR_PROGRAM_COMPOSER" => MAX_IMPROVEMENT_OPERATOR_VERIFIED_ARTIFACTS,
        "FULLSTACK_TYPED_RECIPE_COMPOSER" => MAX_FULLSTACK_VERIFIED_ARTIFACTS,
        _ => 0,
    }
}

fn verified_artifact_family_width(memory: &GenerativeGrowthMemory, composer_id: &str) -> usize {
    let verified = distinct_verified_artifact_count_for_composer(memory, composer_id);
    let remaining = verified_artifact_capacity(composer_id).saturating_sub(verified);
    usize::try_from(
        verified
            .max(1)
            .min(remaining)
            .min(MAX_VERIFIED_ARTIFACTS_PER_CYCLE as u64),
    )
    .unwrap_or(MAX_VERIFIED_ARTIFACTS_PER_CYCLE)
}

fn composer_is_behaviorally_executable(composer_id: &str, memory: &GenerativeGrowthMemory) -> bool {
    match composer_id {
        "SEM5_PROGRAM_IR_COMPOSER" => verified_artifact_family_width(memory, composer_id) > 0,
        "IMPROVEMENT_OPERATOR_PROGRAM_COMPOSER" => {
            verified_artifact_family_width(memory, "SEM5_PROGRAM_IR_COMPOSER") == 0
                && verified_artifact_family_width(memory, composer_id) > 0
        }
        "FULLSTACK_TYPED_RECIPE_COMPOSER" => {
            verified_artifact_family_width(memory, "SEM5_PROGRAM_IR_COMPOSER") == 0
                && verified_artifact_family_width(memory, "IMPROVEMENT_OPERATOR_PROGRAM_COMPOSER")
                    == 0
                && verified_artifact_family_width(memory, composer_id) > 0
        }
        _ => false,
    }
}

pub fn executable_generative_substrate_available(memory: &GenerativeGrowthMemory) -> bool {
    GENERATIVE_COMPOSERS
        .iter()
        .any(|composer| composer_is_behaviorally_executable(composer.0, memory))
}

fn behavioral_execution_receipt_sha256(
    receipt: &BehavioralCompositionExecutionReceipt,
) -> Result<String, String> {
    let mut identity = receipt.clone();
    identity.receipt_sha256.clear();
    let bytes = serde_json::to_vec(&identity)
        .map_err(|error| format!("BEHAVIORAL_EXECUTION_RECEIPT_SERIALIZE:{error}"))?;
    Ok(sha256(&bytes))
}

fn execute_behavioral_composition(
    selected: &RepairCompositionLessonIR,
    input: &GenerativeInput,
    seed: u64,
    artifact_family_width: usize,
    previously_verified: &BTreeSet<String>,
) -> Result<BehavioralCompositionExecutionReceipt, String> {
    let context = context_sha256(input);
    let predictor_id = selected_stage(selected, "PREDICT")?.to_string();
    let composer_id = selected_stage(selected, "COMPOSE")?.to_string();
    let verifier_id = selected_stage(selected, "VERIFY")?.to_string();
    let predictor_output_sha256 = execute_predictor(&predictor_id, input, seed)?;
    let (verified_artifacts, abstention_reason) = execute_composer(
        &composer_id,
        selected,
        input,
        &context,
        artifact_family_width,
        previously_verified,
    )?;
    let cases_executed = verified_artifacts
        .iter()
        .map(|artifact| artifact.cases_executed)
        .sum::<usize>();
    let cases_passed = verified_artifacts
        .iter()
        .map(|artifact| artifact.cases_passed)
        .sum::<usize>();
    let composite_artifact_sha256 = verified_artifacts
        .first()
        .map(|artifact| artifact.artifact_sha256.clone());
    let executed = !verified_artifacts.is_empty()
        && cases_executed > 0
        && cases_passed == cases_executed
        && abstention_reason.is_none();
    let verifier_output_sha256 = executed.then(|| {
        let family = verified_artifacts
            .iter()
            .map(|artifact| {
                format!(
                    "{}:{}:{}:{}",
                    artifact.artifact_context_sha256,
                    artifact.artifact_sha256,
                    artifact.cases_executed,
                    artifact.cases_passed
                )
            })
            .collect::<Vec<_>>()
            .join(":");
        sha256(format!("{context}:{predictor_output_sha256}:{family}:{verifier_id}").as_bytes())
    });
    let mut receipt = BehavioralCompositionExecutionReceipt {
        schema: "B_CORE_BEHAVIORAL_COMPOSITION_EXECUTION_2".to_string(),
        context_sha256: context,
        predictor_id,
        predictor_output_sha256,
        composer_id,
        composite_artifact_sha256,
        verifier_id,
        verifier_output_sha256,
        verified_artifacts,
        cases_executed,
        cases_passed,
        executed,
        abstention_reason,
        receipt_sha256: String::new(),
    };
    receipt.receipt_sha256 = behavioral_execution_receipt_sha256(&receipt)?;
    Ok(receipt)
}

#[derive(Debug, Clone)]
struct CandidatePrediction {
    predicted_value: u16,
    selection_score: u16,
    reused_memory_composition_id: Option<String>,
    prior_composition_trials: u64,
    prior_context_trials: u64,
    exploration: bool,
}

fn prediction_score(
    composition: &RepairCompositionLessonIR,
    input: &GenerativeInput,
    memory: &GenerativeGrowthMemory,
) -> CandidatePrediction {
    let context = context_sha256(input);
    let compatible_behavioral_values = memory.behavioral_value_contract_revision
        >= BEHAVIORAL_HEURISTIC_EXCLUSION_CONTRACT_REVISION;
    let reusable = compatible_behavioral_values.then(|| {
        memory
            .accepted_compositions
            .iter()
            .find(|candidate| candidate.composition.composition_id == composition.composition_id)
    });
    let reusable = reusable.flatten();
    let trials = if compatible_behavioral_values {
        memory
            .composition_trials
            .iter()
            .filter(|trial| trial.composition_id == composition.composition_id)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let failed_trials = trials
        .iter()
        .filter(|trial| !trial.valuable)
        .collect::<Vec<_>>();
    let (prior_composition_trials, prior_context_trials, successful_trials, observed_total) =
        if let Some(candidate) = reusable {
            let successful = candidate
                .successful_uses
                .max(candidate.reuse_count.saturating_add(1));
            let failed = failed_trials.len() as u64;
            let observed = candidate
                .observed_value_total
                .max(u64::from(candidate.observed_value).saturating_mul(successful))
                .saturating_add(
                    failed_trials
                        .iter()
                        .map(|trial| u64::from(trial.observed_value))
                        .sum::<u64>(),
                );
            let same_context_successes = candidate
                .context_use_counts
                .get(&context)
                .copied()
                .unwrap_or(0);
            let same_context_failures = failed_trials
                .iter()
                .filter(|trial| trial.context_sha256 == context)
                .count() as u64;
            (
                successful.saturating_add(failed),
                same_context_successes.saturating_add(same_context_failures),
                successful,
                observed,
            )
        } else {
            (
                trials.len() as u64,
                trials
                    .iter()
                    .filter(|trial| trial.context_sha256 == context)
                    .count() as u64,
                trials.iter().filter(|trial| trial.valuable).count() as u64,
                trials
                    .iter()
                    .map(|trial| u64::from(trial.observed_value))
                    .sum::<u64>(),
            )
        };
    let observed_average = if prior_composition_trials == 0 {
        0
    } else {
        observed_total
            .checked_div(prior_composition_trials)
            .unwrap_or(0)
            .min(100) as u16
    };
    let overlap = reusable
        .map(|candidate| overlap_count(&input.diagnostic_signals, &candidate.trigger_signals))
        .unwrap_or(0);
    let failed_trials = prior_composition_trials.saturating_sub(successful_trials);
    let exploration = prior_composition_trials == 0;
    // Keep the expected outcome independent from the exploration incentive.
    // Otherwise a never-tried composition looks artificially certain and can
    // be rejected only because its UCB-style exploration bonus inflated the
    // recorded prediction.
    let evidence_value = input.verification_evidence_count.min(4) as u16 * 2;
    let role_value = input.observed_composition_roles.len().min(4) as u16;
    let prior_free_prediction = 45_u16
        .saturating_add(input.learning_score.min(100) / 4)
        .saturating_add(domain_bonus(composition, input))
        .saturating_add(evidence_value)
        .saturating_add(role_value)
        .saturating_add(if input.measured_performance_gain {
            6
        } else {
            0
        })
        .min(100);
    let predicted_value = if prior_composition_trials == 0 {
        prior_free_prediction
    } else {
        u32::from(prior_free_prediction)
            .saturating_add(u32::from(observed_average).saturating_mul(2))
            .checked_div(3)
            .unwrap_or(0)
            .min(100) as u16
    };
    let mut selection_score = i32::from(predicted_value);
    if exploration {
        // Exhaust the bounded typed search surface before a successful early
        // choice can monopolize all later campaigns.
        selection_score += 40;
    } else {
        selection_score += i32::from(observed_average.saturating_sub(60).min(30) / 3);
        selection_score += i32::try_from(overlap.min(4)).unwrap_or(4);
        if reusable.is_some() && prior_context_trials == 0 {
            selection_score += 4;
        }
        selection_score -= i32::try_from(prior_composition_trials.min(12) * 2).unwrap_or(24);
        selection_score -= i32::try_from(failed_trials.min(4) * 6).unwrap_or(24);
        selection_score -= i32::try_from(prior_context_trials.min(3) * 8).unwrap_or(24);
    }
    CandidatePrediction {
        predicted_value,
        selection_score: selection_score.clamp(0, 160) as u16,
        reused_memory_composition_id: reusable
            .filter(|_| overlap > 0)
            .map(|candidate| candidate.composition.composition_id.clone()),
        prior_composition_trials,
        prior_context_trials,
        exploration,
    }
}

pub fn run_generative_cycle(
    memory: &GenerativeGrowthMemory,
    input: &GenerativeInput,
    seed: u64,
) -> Result<GenerativeCycleResult, String> {
    if memory.schema != GENERATIVE_GROWTH_SCHEMA || input.source_lesson_id.is_empty() {
        return Err("GENERATIVE_INPUT_OR_MEMORY_INVALID".to_string());
    }
    // Verification is not an optimization arm. Every candidate must pass the
    // same independent verifier and the evaluator mutation audit is already a
    // mandatory check inside that boundary. Treating both as selectable
    // alternatives doubled the search space without changing behavior.
    let mut candidates = Vec::new();
    let mut behaviorally_inapplicable_candidates_screened = 0_usize;
    for predictor in GENERATIVE_PREDICTORS {
        for composer in GENERATIVE_COMPOSERS {
            for verifier in GENERATIVE_VERIFIERS {
                if !composer_is_behaviorally_executable(composer.0, memory) {
                    behaviorally_inapplicable_candidates_screened =
                        behaviorally_inapplicable_candidates_screened.saturating_add(1);
                    continue;
                }
                let composition = candidate_composition(predictor, composer, verifier);
                let prediction = prediction_score(&composition, input, memory);
                let tie = sha256(format!("{}:{}", seed, composition.composition_id).as_bytes());
                candidates.push((prediction.selection_score, tie, prediction, composition));
            }
        }
    }
    candidates.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    let (_, _, prediction, selected) = candidates
        .first()
        .cloned()
        .ok_or_else(|| "NO_BEHAVIORALLY_EXECUTABLE_GENERATIVE_COMPOSITION_CANDIDATE".to_string())?;
    let predicted_value = prediction.predicted_value;
    let typecheck_pass = validate_composition_lesson(&selected).is_ok();
    let selected_composer = selected_stage(&selected, "COMPOSE")?;
    let available_family_width = verified_artifact_family_width(memory, selected_composer);
    // Replaying an already-seen composition context revalidates its canonical
    // first artifact. Only a genuinely new frontier context may spend the
    // geometric family budget searching past previously verified artifacts.
    let artifact_family_width = if prediction.prior_context_trials > 0 && available_family_width > 0
    {
        1
    } else {
        available_family_width
    };
    let previously_verified = if prediction.prior_context_trials > 0 {
        BTreeSet::new()
    } else {
        verified_artifacts_for_composer(memory, selected_composer)
    };
    let behavioral_execution_receipt = execute_behavioral_composition(
        &selected,
        input,
        seed,
        artifact_family_width,
        &previously_verified,
    )?;
    let behavioral_composition_executed = behavioral_execution_receipt.executed;
    let behavioral_verification_sha256 = behavioral_composition_executed
        .then(|| behavioral_execution_receipt.receipt_sha256.clone());
    // Predictions may rank candidates, but observed value is the public
    // behavioral pass rate. Type-check-only artifacts are explicit
    // abstentions and cannot become capability or frontier evidence.
    let observed_value =
        if behavioral_composition_executed && behavioral_execution_receipt.cases_executed > 0 {
            behavioral_execution_receipt
                .cases_passed
                .saturating_mul(100)
                .checked_div(behavioral_execution_receipt.cases_executed)
                .unwrap_or(0)
                .min(100) as u16
        } else {
            0
        };
    let prediction_error = predicted_value.abs_diff(observed_value);
    let structural_candidate =
        typecheck_pass && input.verification_evidence_count > 0 && predicted_value >= 72;
    let valuable = structural_candidate
        && behavioral_composition_executed
        && observed_value >= 72
        && prediction_error <= 30;
    let accepted_for_memory = valuable && prediction.exploration;
    let previously_verified = memory
        .accepted_compositions
        .iter()
        .flat_map(|candidate| candidate.verified_artifact_sha256s.iter())
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let verified_artifact_count = behavioral_execution_receipt.verified_artifacts.len();
    let novel_verified_artifact_count = if valuable {
        behavioral_execution_receipt
            .verified_artifacts
            .iter()
            .filter(|artifact| !previously_verified.contains(artifact.artifact_sha256.as_str()))
            .count()
    } else {
        0
    };
    let novel_verified_artifact = novel_verified_artifact_count > 0;
    let novel_context_transfer_candidate = valuable
        && prediction.reused_memory_composition_id.is_some()
        && prediction.prior_context_trials == 0;
    let unverified_frontier_candidate = structural_candidate
        && !behavioral_composition_executed
        && (prediction.exploration
            || (prediction.reused_memory_composition_id.is_some()
                && prediction.prior_context_trials == 0));
    let productive_reuse =
        prediction.reused_memory_composition_id.is_some() && novel_verified_artifact;
    // A newly selected wrapper over an already-known artifact is not a
    // capability advance. Frontier units are the distinct verified artifacts.
    let frontier_advance = novel_verified_artifact && (accepted_for_memory || productive_reuse);
    let frontier_advance_units = if frontier_advance {
        novel_verified_artifact_count.min(u64::MAX as usize) as u64
    } else {
        0
    };
    let applied_policy_signals = if frontier_advance {
        applicable_policy_signals(&selected, input)
    } else {
        Vec::new()
    };
    Ok(GenerativeCycleResult {
        schema: GENERATIVE_GROWTH_SCHEMA.to_string(),
        source_lesson_id: input.source_lesson_id.clone(),
        candidates_considered: candidates.len(),
        behaviorally_inapplicable_candidates_screened,
        selected_composition: selected,
        selected_from_precomposition_prediction: true,
        prediction_recorded_before_composition: true,
        predicted_value,
        selection_score: prediction.selection_score,
        predicted_resource_units: u16::try_from(12_usize.saturating_mul(artifact_family_width))
            .unwrap_or(u16::MAX),
        isolated_composition_executed: true,
        composition_typecheck_pass: typecheck_pass,
        behavioral_composition_executed,
        behavioral_verification_sha256,
        behavioral_execution_receipt: Some(behavioral_execution_receipt),
        observed_value_is_heuristic_proxy: false,
        observed_value,
        prediction_error,
        valuable,
        accepted_for_memory,
        reused_memory_composition_id: prediction.reused_memory_composition_id,
        applied_to_self_improvement: !applied_policy_signals.is_empty(),
        applied_policy_signals,
        context_sha256: context_sha256(input),
        exploration_selected: prediction.exploration,
        prior_composition_trials: prediction.prior_composition_trials,
        prior_context_trials: prediction.prior_context_trials,
        productive_reuse,
        novel_context_transfer_candidate,
        novel_verified_artifact,
        verified_artifact_count,
        novel_verified_artifact_count,
        unverified_frontier_candidate,
        frontier_advance,
        frontier_advance_units,
        exact_source_fragments: 0,
        codex_calls: 0,
        external_llm_calls: 0,
        network_reads: 0,
        network_writes: 0,
    })
}

pub fn validate_behavioral_execution_receipt(result: &GenerativeCycleResult) -> bool {
    result
        .behavioral_execution_receipt
        .as_ref()
        .is_some_and(|receipt| {
            let expected_receipt_sha256 = behavioral_execution_receipt_sha256(receipt).ok();
            let artifact_hashes = receipt
                .verified_artifacts
                .iter()
                .map(|artifact| artifact.artifact_sha256.as_str())
                .collect::<BTreeSet<_>>();
            let artifact_contexts = receipt
                .verified_artifacts
                .iter()
                .map(|artifact| artifact.artifact_context_sha256.as_str())
                .collect::<BTreeSet<_>>();
            let aggregate_cases = receipt.verified_artifacts.iter().fold(
                (0_usize, 0_usize),
                |(executed, passed), artifact| {
                    (
                        executed.saturating_add(artifact.cases_executed),
                        passed.saturating_add(artifact.cases_passed),
                    )
                },
            );
            receipt.schema == "B_CORE_BEHAVIORAL_COMPOSITION_EXECUTION_2"
                && receipt.context_sha256 == result.context_sha256
                && expected_receipt_sha256.as_deref() == Some(receipt.receipt_sha256.as_str())
                && receipt.predictor_id
                    == selected_stage(&result.selected_composition, "PREDICT").unwrap_or("")
                && receipt.composer_id
                    == selected_stage(&result.selected_composition, "COMPOSE").unwrap_or("")
                && receipt.verifier_id
                    == selected_stage(&result.selected_composition, "VERIFY").unwrap_or("")
                && receipt.executed == result.behavioral_composition_executed
                && result.verified_artifact_count == receipt.verified_artifacts.len()
                && if receipt.executed {
                    !receipt.verified_artifacts.is_empty()
                        && receipt.verified_artifacts.len() <= MAX_VERIFIED_ARTIFACTS_PER_CYCLE
                        && artifact_hashes.len() == receipt.verified_artifacts.len()
                        && artifact_contexts.len() == receipt.verified_artifacts.len()
                        && receipt.verified_artifacts.iter().all(|artifact| {
                            artifact.artifact_sha256.len() == 64
                                && artifact
                                    .artifact_sha256
                                    .bytes()
                                    .all(|byte| byte.is_ascii_hexdigit())
                                && artifact.artifact_context_sha256.len() == 64
                                && artifact
                                    .artifact_context_sha256
                                    .bytes()
                                    .all(|byte| byte.is_ascii_hexdigit())
                                && artifact.cases_executed > 0
                                && artifact.cases_passed == artifact.cases_executed
                        })
                        && receipt.composite_artifact_sha256.as_deref()
                            == receipt
                                .verified_artifacts
                                .first()
                                .map(|artifact| artifact.artifact_sha256.as_str())
                        && receipt.verifier_output_sha256.is_some()
                        && receipt.cases_executed == aggregate_cases.0
                        && receipt.cases_passed == aggregate_cases.1
                        && receipt.cases_passed == receipt.cases_executed
                        && result.observed_value
                            == receipt
                                .cases_passed
                                .saturating_mul(100)
                                .checked_div(receipt.cases_executed)
                                .unwrap_or(0)
                                .min(100) as u16
                        && receipt.abstention_reason.is_none()
                        && result.behavioral_verification_sha256.as_deref()
                            == Some(receipt.receipt_sha256.as_str())
                } else {
                    receipt.composite_artifact_sha256.is_none()
                        && receipt.verifier_output_sha256.is_none()
                        && receipt.verified_artifacts.is_empty()
                        && receipt.cases_executed == 0
                        && receipt.cases_passed == 0
                        && result.observed_value == 0
                        && receipt.abstention_reason.is_some()
                        && result.behavioral_verification_sha256.is_none()
                }
        })
}

pub fn promote_generative_cycle(
    current: &GenerativeGrowthMemory,
    input: &GenerativeInput,
    result: &GenerativeCycleResult,
) -> Result<GenerativeGrowthMemory, String> {
    let result_artifacts = result
        .behavioral_execution_receipt
        .as_ref()
        .map(|receipt| receipt.verified_artifacts.as_slice())
        .unwrap_or_default();
    let previously_verified = current
        .accepted_compositions
        .iter()
        .flat_map(|candidate| candidate.verified_artifact_sha256s.iter())
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let expected_novel_verified_artifact_count = if result.valuable {
        result_artifacts
            .iter()
            .filter(|artifact| !previously_verified.contains(artifact.artifact_sha256.as_str()))
            .count()
    } else {
        0
    };
    let expected_novel_verified_artifact = expected_novel_verified_artifact_count > 0;
    let expected_accepted_for_memory = result.valuable && result.exploration_selected;
    let expected_novel_context_transfer = result.valuable
        && result.reused_memory_composition_id.is_some()
        && result.prior_context_trials == 0;
    let expected_productive_reuse =
        result.reused_memory_composition_id.is_some() && expected_novel_verified_artifact;
    let expected_frontier_advance = expected_novel_verified_artifact
        && (expected_accepted_for_memory || expected_productive_reuse);
    let expected_frontier_advance_units = if expected_frontier_advance {
        expected_novel_verified_artifact_count.min(u64::MAX as usize) as u64
    } else {
        0
    };
    if current.schema != GENERATIVE_GROWTH_SCHEMA
        || result.schema != GENERATIVE_GROWTH_SCHEMA
        || result.source_lesson_id != input.source_lesson_id
        || result.candidates_considered == 0
        || result
            .candidates_considered
            .saturating_add(result.behaviorally_inapplicable_candidates_screened)
            != STATIC_GENERATIVE_CANDIDATE_COUNT
        || !composer_is_behaviorally_executable(
            selected_stage(&result.selected_composition, "COMPOSE").unwrap_or(""),
            current,
        )
        || !result.prediction_recorded_before_composition
        || !result.selected_from_precomposition_prediction
        || result.observed_value_is_heuristic_proxy
        || !validate_behavioral_execution_receipt(result)
        || (!result.behavioral_composition_executed
            && (result.behavioral_verification_sha256.is_some()
                || result.frontier_advance
                || result.productive_reuse
                || result.applied_to_self_improvement
                || !result.applied_policy_signals.is_empty()))
        || (result.behavioral_composition_executed
            && result.behavioral_verification_sha256.is_none())
        || result.exact_source_fragments != 0
        || result.codex_calls != 0
        || result.external_llm_calls != 0
        || result.network_reads != 0
        || result.network_writes != 0
        || result.verified_artifact_count != result_artifacts.len()
        || result.novel_verified_artifact_count != expected_novel_verified_artifact_count
        || result.novel_verified_artifact != expected_novel_verified_artifact
        || result.accepted_for_memory != expected_accepted_for_memory
        || result.novel_context_transfer_candidate != expected_novel_context_transfer
        || result.productive_reuse != expected_productive_reuse
        || result.frontier_advance != expected_frontier_advance
        || result.frontier_advance_units != expected_frontier_advance_units
    {
        return Err("GENERATIVE_PROMOTION_BOUNDARY_FAILURE".to_string());
    }
    let mut next = current.clone();
    if next.behavioral_value_contract_revision < FRONTIER_EVIDENCE_CONTRACT_REVISION {
        next.legacy_unverified_frontier_advance_events = next
            .legacy_unverified_frontier_advance_events
            .saturating_add(next.frontier_advance_events);
        next.frontier_advance_events = 0;
    }
    if next.behavioral_value_contract_revision < BEHAVIORAL_HEURISTIC_EXCLUSION_CONTRACT_REVISION {
        next.legacy_heuristic_composition_trials = next
            .legacy_heuristic_composition_trials
            .saturating_add(next.composition_trials.len().min(u64::MAX as usize) as u64);
        next.legacy_heuristic_accepted_compositions = next
            .legacy_heuristic_accepted_compositions
            .saturating_add(next.accepted_compositions.len().min(u64::MAX as usize) as u64);
        next.composition_trials.clear();
        next.accepted_compositions.clear();
        next.legacy_uncalibrated_prediction_error_total = next
            .legacy_uncalibrated_prediction_error_total
            .saturating_add(next.prediction_absolute_error_total);
        next.prediction_absolute_error_total = 0;
        next.calibrated_prediction_records = 0;
        next.behavioral_value_contract_revision = BEHAVIORAL_HEURISTIC_EXCLUSION_CONTRACT_REVISION;
    }
    if next.behavioral_value_contract_revision < BEHAVIORAL_VALUE_CONTRACT_REVISION {
        next.legacy_wrapper_frontier_advance_events = next
            .legacy_wrapper_frontier_advance_events
            .saturating_add(next.frontier_advance_events);
        next.frontier_advance_events = 0;
        next.frontier_capability_units = next.distinct_verified_artifact_count();
        next.behavioral_value_contract_revision = BEHAVIORAL_VALUE_CONTRACT_REVISION;
    }
    next.generation = next.generation.saturating_add(1);
    next.prediction_records = next.prediction_records.saturating_add(1);
    if next.calibrated_prediction_records == 0 && next.prediction_absolute_error_total > 0 {
        next.legacy_uncalibrated_prediction_error_total = next
            .legacy_uncalibrated_prediction_error_total
            .saturating_add(next.prediction_absolute_error_total);
        next.prediction_absolute_error_total = 0;
    }
    next.prediction_absolute_error_total = next
        .prediction_absolute_error_total
        .saturating_add(u64::from(result.prediction_error));
    next.calibrated_prediction_records = next.calibrated_prediction_records.saturating_add(1);
    next.composition_trials.push(GenerativeCompositionTrial {
        composition_id: result.selected_composition.composition_id.clone(),
        context_sha256: result.context_sha256.clone(),
        predicted_value: result.predicted_value,
        observed_value: result.observed_value,
        valuable: result.valuable,
    });
    if result.exploration_selected {
        next.exploration_events = next.exploration_events.saturating_add(1);
    }
    if result.productive_reuse {
        next.productive_reuse_events = next.productive_reuse_events.saturating_add(1);
    }
    if result.frontier_advance {
        next.frontier_advance_events = next.frontier_advance_events.saturating_add(1);
        next.frontier_capability_units = next
            .frontier_capability_units
            .saturating_add(result.frontier_advance_units);
    }
    if result.unverified_frontier_candidate {
        next.unverified_frontier_candidate_events =
            next.unverified_frontier_candidate_events.saturating_add(1);
    }
    if result.behavioral_composition_executed && result.behavioral_verification_sha256.is_some() {
        next.behavioral_verification_events = next.behavioral_verification_events.saturating_add(1);
    }
    if result.valuable && !result.frontier_advance && !result.unverified_frontier_candidate {
        next.redundant_selection_events = next.redundant_selection_events.saturating_add(1);
    }
    if result.valuable {
        if result_artifacts.is_empty() {
            return Err("VALUABLE_COMPOSITION_ARTIFACT_MISSING".to_string());
        }
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
            *existing
                .context_use_counts
                .entry(result.context_sha256.clone())
                .or_insert(0) += 1;
            if existing.successful_uses == 0 {
                existing.successful_uses = existing.reuse_count;
                existing.observed_value_total =
                    u64::from(existing.observed_value).saturating_mul(existing.successful_uses);
            }
            existing.successful_uses = existing.successful_uses.saturating_add(1);
            existing.observed_value_total = existing
                .observed_value_total
                .saturating_add(u64::from(result.observed_value));
            existing.observed_value = existing
                .observed_value_total
                .checked_div(existing.successful_uses)
                .unwrap_or(0)
                .min(100) as u16;
            for artifact in result_artifacts {
                if !existing
                    .verified_artifact_sha256s
                    .contains(&artifact.artifact_sha256)
                {
                    existing
                        .verified_artifact_sha256s
                        .push(artifact.artifact_sha256.clone());
                }
                existing.verified_artifact_contexts.insert(
                    artifact.artifact_sha256.clone(),
                    artifact.artifact_context_sha256.clone(),
                );
            }
            existing.verified_artifact_sha256s.sort();
        } else if result.accepted_for_memory {
            let mut context_use_counts = BTreeMap::new();
            context_use_counts.insert(result.context_sha256.clone(), 1);
            let verified_artifact_sha256s = result_artifacts
                .iter()
                .map(|artifact| artifact.artifact_sha256.clone())
                .collect::<Vec<_>>();
            let verified_artifact_contexts = result_artifacts
                .iter()
                .map(|artifact| {
                    (
                        artifact.artifact_sha256.clone(),
                        artifact.artifact_context_sha256.clone(),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            next.accepted_compositions.push(ReusableCompositionMemory {
                composition: result.selected_composition.clone(),
                trigger_signals: input.diagnostic_signals.clone(),
                source_lesson_ids: vec![input.source_lesson_id.clone()],
                predicted_value: result.predicted_value,
                observed_value: result.observed_value,
                reuse_count: 0,
                context_use_counts,
                successful_uses: 1,
                observed_value_total: u64::from(result.observed_value),
                verified_artifact_sha256s,
                verified_artifact_contexts,
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
    while next.composition_trials.len() > MAX_COMPOSITION_TRIALS {
        next.composition_trials.remove(0);
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
            learning_score: 80,
            verification_evidence_count: 1,
            measured_performance_gain: false,
        }
    }

    #[test]
    fn prediction_precedes_isolated_typed_composition() {
        let result = run_generative_cycle(&GenerativeGrowthMemory::default(), &input(), 7).unwrap();
        assert_eq!(result.candidates_considered, 2);
        assert_eq!(result.behaviorally_inapplicable_candidates_screened, 6);
        assert!(result.prediction_recorded_before_composition);
        assert!(result.selected_from_precomposition_prediction);
        assert!(result.isolated_composition_executed);
        assert!(result.composition_typecheck_pass);
        assert!(result.behavioral_composition_executed);
        assert!(result.behavioral_verification_sha256.is_some());
        assert!(result
            .behavioral_execution_receipt
            .as_ref()
            .is_some_and(|receipt| {
                receipt.executed
                    && receipt.cases_executed == 3
                    && receipt.cases_passed == receipt.cases_executed
            }));
        assert!(!result.observed_value_is_heuristic_proxy);
        assert_eq!(result.observed_value, 100);
        assert!(result.selection_score > result.predicted_value);
        assert!(result.prediction_error <= 30);
        assert!(result.valuable);
        assert!(result.accepted_for_memory);
        assert_eq!(result.verified_artifact_count, 1);
        assert_eq!(result.novel_verified_artifact_count, 1);
        assert_eq!(result.frontier_advance_units, 1);
        assert!(!result.unverified_frontier_candidate);
        assert!(result.frontier_advance);
        assert!(result.applied_to_self_improvement);
        assert!(!result.applied_policy_signals.is_empty());
        assert_eq!(result.external_llm_calls, 0);
    }

    #[test]
    fn bounded_exploration_spends_budget_only_on_behaviorally_executable_candidates() {
        let mut memory = GenerativeGrowthMemory::default();
        let mut selected = BTreeSet::new();
        for ordinal in 0..2 {
            let mut current = input();
            current.source_lesson_id = format!("lesson-{ordinal}");
            let result = run_generative_cycle(&memory, &current, ordinal).unwrap();
            assert!(result.exploration_selected);
            assert_eq!(result.prior_composition_trials, 0);
            selected.insert(result.selected_composition.composition_id.clone());
            memory = promote_generative_cycle(&memory, &current, &result).unwrap();
        }
        assert_eq!(selected.len(), 2);
        assert_eq!(memory.composition_trials.len(), 2);
        assert_eq!(memory.exploration_events, 2);
        assert_eq!(memory.accepted_compositions.len(), 2);
        assert_eq!(memory.distinct_verified_artifact_count(), 2);
        // The second predictor is allowed to search past the already verified
        // artifact because this is its first bounded composition context.
        assert_eq!(memory.frontier_advance_events, 2);
        assert_eq!(memory.frontier_capability_units, 2);
        assert_eq!(memory.unverified_frontier_candidate_events, 0);
        assert_eq!(memory.behavioral_verification_events, 2);
        assert!(memory.prediction_absolute_error_total < 2 * 100);
        assert_eq!(memory.calibrated_prediction_records, 2);

        let mut repeated_context = input();
        repeated_context.source_lesson_id = "lesson-repeated-context".to_string();
        let repeated = run_generative_cycle(&memory, &repeated_context, 99).unwrap();
        assert!(!repeated.exploration_selected);
        assert!(repeated.reused_memory_composition_id.is_some());
        assert!(repeated.prior_context_trials > 0);
        assert!(!repeated.productive_reuse);
        assert!(!repeated.novel_verified_artifact);
        assert!(!repeated.frontier_advance);
        assert!(!repeated.applied_to_self_improvement);
        memory = promote_generative_cycle(&memory, &repeated_context, &repeated).unwrap();
        assert_eq!(memory.reuse_events, 1);
        assert_eq!(memory.redundant_selection_events, 1);

        let mut new_context = input();
        new_context.source_lesson_id = "lesson-new-context".to_string();
        new_context
            .diagnostic_signals
            .push("DEFECT_REPAIR".to_string());
        let transferred = run_generative_cycle(&memory, &new_context, 101).unwrap();
        assert!(transferred.reused_memory_composition_id.is_some());
        assert_eq!(transferred.prior_context_trials, 0);
        assert!(transferred.novel_context_transfer_candidate);
        assert!(!transferred.unverified_frontier_candidate);
        assert!(transferred.productive_reuse);
        assert!(transferred.novel_verified_artifact);
        assert!(transferred.frontier_advance);
        assert!(!transferred.applied_to_self_improvement);
        assert!(transferred.applied_policy_signals.is_empty());
        memory = promote_generative_cycle(&memory, &new_context, &transferred).unwrap();
        assert_eq!(memory.unverified_frontier_candidate_events, 0);
        assert_eq!(memory.distinct_verified_artifact_count(), 4);
        assert_eq!(memory.frontier_capability_units, 4);
    }

    #[test]
    fn verified_success_expands_the_next_bounded_artifact_family() {
        let first_input = input();
        let first_result =
            run_generative_cycle(&GenerativeGrowthMemory::default(), &first_input, 7).unwrap();
        let mut memory = promote_generative_cycle(
            &GenerativeGrowthMemory::default(),
            &first_input,
            &first_result,
        )
        .unwrap();
        let accepted = memory.accepted_compositions.first_mut().unwrap();
        accepted.verified_artifact_sha256s = (0..5)
            .map(|ordinal| sha256(format!("verified-{ordinal}").as_bytes()))
            .collect();
        memory.frontier_capability_units = 5;

        let mut next_input = input();
        next_input.source_lesson_id = "bounded-family-after-success".to_string();
        next_input
            .diagnostic_signals
            .push("CAPABILITY_SURFACE_ADDED".to_string());
        let result = run_generative_cycle(&memory, &next_input, 19).unwrap();

        assert_eq!(
            verified_artifact_family_width(&memory, "SEM5_PROGRAM_IR_COMPOSER"),
            5
        );
        assert_eq!(result.verified_artifact_count, 5);
        assert_eq!(result.novel_verified_artifact_count, 5);
        assert_eq!(result.frontier_advance_units, 5);
        let next = promote_generative_cycle(&memory, &next_input, &result).unwrap();
        assert_eq!(next.frontier_capability_units, 10);
        assert_eq!(next.distinct_verified_artifact_count(), 10);
    }

    #[test]
    fn verified_family_growth_routes_across_executable_substrates() {
        let memory_with = |count: u64| {
            let mut memory = GenerativeGrowthMemory::default();
            let mut result = run_generative_cycle(&memory, &input(), 7).unwrap();
            result
                .behavioral_execution_receipt
                .as_mut()
                .unwrap()
                .verified_artifacts
                .truncate(1);
            memory = promote_generative_cycle(&memory, &input(), &result).unwrap();
            let accepted = memory.accepted_compositions.first_mut().unwrap();
            accepted.verified_artifact_sha256s = (0..count)
                .map(|ordinal| sha256(format!("bounded-{ordinal}").as_bytes()))
                .collect();
            memory
        };

        assert_eq!(
            verified_artifact_family_width(&memory_with(0), "SEM5_PROGRAM_IR_COMPOSER"),
            1
        );
        assert_eq!(
            verified_artifact_family_width(&memory_with(1), "SEM5_PROGRAM_IR_COMPOSER"),
            1
        );
        assert_eq!(
            verified_artifact_family_width(&memory_with(8), "SEM5_PROGRAM_IR_COMPOSER"),
            8
        );
        assert_eq!(
            verified_artifact_family_width(&memory_with(33), "SEM5_PROGRAM_IR_COMPOSER"),
            31
        );
        let saturated = memory_with(64);
        assert_eq!(
            verified_artifact_family_width(&saturated, "SEM5_PROGRAM_IR_COMPOSER"),
            0
        );
        let saturated_result = run_generative_cycle(&saturated, &input(), 11).unwrap();
        assert_eq!(
            selected_stage(&saturated_result.selected_composition, "COMPOSE").unwrap(),
            "IMPROVEMENT_OPERATOR_PROGRAM_COMPOSER"
        );
        assert_eq!(saturated_result.verified_artifact_count, 1);
        assert!(saturated_result.behavioral_composition_executed);
        assert!(saturated_result.novel_verified_artifact);
        assert!(saturated_result.frontier_advance);
        let mut expanded =
            promote_generative_cycle(&saturated, &input(), &saturated_result).unwrap();
        assert_eq!(
            distinct_verified_artifact_count_for_composer(
                &expanded,
                "IMPROVEMENT_OPERATOR_PROGRAM_COMPOSER"
            ),
            1
        );
        assert_eq!(expanded.distinct_verified_artifact_count(), 65);

        let operator_memory = expanded
            .accepted_compositions
            .iter_mut()
            .find(|composition| {
                composition_uses_composer(composition, "IMPROVEMENT_OPERATOR_PROGRAM_COMPOSER")
            })
            .unwrap();
        operator_memory.verified_artifact_sha256s = (0
            ..MAX_IMPROVEMENT_OPERATOR_VERIFIED_ARTIFACTS)
            .map(|ordinal| sha256(format!("operator-{ordinal}").as_bytes()))
            .collect();
        let fullstack = run_generative_cycle(&expanded, &input(), 13).unwrap();
        assert_eq!(
            selected_stage(&fullstack.selected_composition, "COMPOSE").unwrap(),
            "FULLSTACK_TYPED_RECIPE_COMPOSER"
        );
        assert_eq!(fullstack.verified_artifact_count, 1);
        assert!(fullstack.behavioral_composition_executed);
        assert!(fullstack.novel_verified_artifact);
        assert!(fullstack.frontier_advance);
        let routed = promote_generative_cycle(&expanded, &input(), &fullstack).unwrap();
        assert_eq!(
            distinct_verified_artifact_count_for_composer(
                &routed,
                "FULLSTACK_TYPED_RECIPE_COMPOSER"
            ),
            1
        );
    }

    #[test]
    fn promotion_rejects_a_structural_only_composer_disguised_as_growth() {
        let memory = GenerativeGrowthMemory::default();
        let current = input();
        let mut result = run_generative_cycle(&memory, &current, 7).unwrap();
        let composer = result
            .selected_composition
            .primitives
            .iter_mut()
            .find(|primitive| primitive.semantic_role == "COMPOSE")
            .expect("composer primitive");
        composer.primitive_id = "FULLSTACK_TYPED_RECIPE_COMPOSER".to_string();
        assert_eq!(
            promote_generative_cycle(&memory, &current, &result),
            Err("GENERATIVE_PROMOTION_BOUNDARY_FAILURE".to_string())
        );
    }

    #[test]
    fn legacy_structural_frontier_claims_are_quarantined_before_new_promotion() {
        let mut memory = GenerativeGrowthMemory {
            frontier_advance_events: 6,
            behavioral_verification_events: 1,
            behavioral_value_contract_revision: 0,
            ..GenerativeGrowthMemory::default()
        };
        let current = input();
        let result = run_generative_cycle(&memory, &current, 19).unwrap();

        memory = promote_generative_cycle(&memory, &current, &result).unwrap();

        assert_eq!(memory.frontier_advance_events, 1);
        assert_eq!(memory.legacy_unverified_frontier_advance_events, 6);
        assert_eq!(memory.behavioral_verification_events, 2);
        assert_eq!(
            memory.behavioral_value_contract_revision,
            BEHAVIORAL_VALUE_CONTRACT_REVISION
        );
    }

    #[test]
    fn legacy_wrapper_events_are_separated_from_capability_units() {
        let first_input = input();
        let first_result =
            run_generative_cycle(&GenerativeGrowthMemory::default(), &first_input, 7).unwrap();
        let mut memory = promote_generative_cycle(
            &GenerativeGrowthMemory::default(),
            &first_input,
            &first_result,
        )
        .unwrap();
        memory.behavioral_value_contract_revision =
            BEHAVIORAL_HEURISTIC_EXCLUSION_CONTRACT_REVISION;
        memory.frontier_advance_events = 7;
        memory.frontier_capability_units = 0;

        // The planner may route the same input through a not-yet-tried typed
        // composition. That is a genuine capability event and must remain
        // separate from the seven legacy wrapper counters being migrated.
        let repeated = first_input.clone();
        let result = run_generative_cycle(&memory, &repeated, 9).unwrap();
        assert!(result.frontier_advance);
        let next = promote_generative_cycle(&memory, &repeated, &result).unwrap();

        assert_eq!(next.legacy_wrapper_frontier_advance_events, 7);
        assert_eq!(next.frontier_advance_events, 1);
        assert_eq!(next.frontier_capability_units, 2);
    }

    #[test]
    fn legacy_heuristic_value_memory_is_not_reused_as_behavioral_evidence() {
        let first_input = input();
        let first_result =
            run_generative_cycle(&GenerativeGrowthMemory::default(), &first_input, 7).unwrap();
        let mut legacy = promote_generative_cycle(
            &GenerativeGrowthMemory::default(),
            &first_input,
            &first_result,
        )
        .unwrap();
        assert_eq!(legacy.accepted_compositions.len(), 1);
        assert_eq!(legacy.composition_trials.len(), 1);
        legacy.behavioral_value_contract_revision = FRONTIER_EVIDENCE_CONTRACT_REVISION;
        legacy.prediction_absolute_error_total = 55;
        legacy.calibrated_prediction_records = 2;

        let mut next_input = input();
        next_input.source_lesson_id = "lesson-after-value-contract-migration".to_string();
        let result = run_generative_cycle(&legacy, &next_input, 7).unwrap();
        assert!(result.exploration_selected);
        assert_eq!(result.prior_composition_trials, 0);
        let next = promote_generative_cycle(&legacy, &next_input, &result).unwrap();

        assert_eq!(
            next.behavioral_value_contract_revision,
            BEHAVIORAL_VALUE_CONTRACT_REVISION
        );
        assert_eq!(next.legacy_heuristic_composition_trials, 1);
        assert_eq!(next.legacy_heuristic_accepted_compositions, 1);
        assert_eq!(next.composition_trials.len(), 1);
        assert_eq!(next.accepted_compositions.len(), 1);
        assert_eq!(next.legacy_uncalibrated_prediction_error_total, 55);
        assert_eq!(
            next.prediction_absolute_error_total,
            u64::from(result.prediction_error)
        );
        assert_eq!(next.calibrated_prediction_records, 1);
    }

    #[test]
    fn tampered_behavioral_receipt_cannot_cross_promotion_boundary() {
        let memory = GenerativeGrowthMemory::default();
        let current = input();
        let mut result = run_generative_cycle(&memory, &current, 7).unwrap();
        result
            .behavioral_execution_receipt
            .as_mut()
            .expect("behavioral receipt")
            .context_sha256 = "0".repeat(64);
        assert_eq!(
            promote_generative_cycle(&memory, &current, &result),
            Err("GENERATIVE_PROMOTION_BOUNDARY_FAILURE".to_string())
        );
    }

    #[test]
    fn unknown_legacy_error_sample_count_is_quarantined_not_guessed() {
        let memory = GenerativeGrowthMemory {
            prediction_records: 18,
            prediction_absolute_error_total: 14,
            ..GenerativeGrowthMemory::default()
        };
        let current = input();
        let result = run_generative_cycle(&memory, &current, 77).unwrap();
        let next = promote_generative_cycle(&memory, &current, &result).unwrap();
        assert_eq!(next.legacy_uncalibrated_prediction_error_total, 14);
        assert_eq!(next.calibrated_prediction_records, 1);
        assert_eq!(
            next.prediction_absolute_error_total,
            u64::from(result.prediction_error)
        );
    }
}
