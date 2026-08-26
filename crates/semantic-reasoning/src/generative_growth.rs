//! Bounded generative growth over already-known local capabilities.
//!
//! The cycle predicts a useful capability composition before assembling it,
//! type-checks the selected composition in isolation, remembers only valuable
//! results, and reuses accepted memories when later lessons share signals.
//! It stores capability identities and typed transport only: no source text,
//! exact patch, network result, or model output enters this memory.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::autonomous_source_mutation::{
    execute_improvement_operator_behavioral_canary,
    execute_improvement_operator_graph_family_behavioral_canary,
    execute_improvement_operator_on_source, improvement_operator_graph_id_for_nodes,
    ImprovementOperatorIR, MAX_IMPROVEMENT_OPERATOR_GRAPH_NODES,
};
use crate::fullstack_ops_knowledge::{execute_fullstack_recipe_behavioral_canary, promoted_bundle};
use crate::integrated_development::{
    execute_behavioral_composition_canary,
    execute_typed_behavior_goal_canary_with_operator_proposals,
};
use crate::self_healing_pipeline::{
    execute_self_healing_behavioral_canary, validate_composition_lesson, CompositionEdgeIR,
    RepairCompositionLessonIR, RepairPrimitiveIR,
};
use crate::self_repair_contract::sha256;
use crate::sem23_engine::{
    predict_base_properties, GenerativeRequest, PROPERTY_FAMILY_TRANSFER, PROPERTY_REACTION_LAW,
    PROPERTY_RECURSIVE_CLOSURE, PROPERTY_STRUCTURED_EMERGENCE,
};
use crate::sem25_engine::{run_growth_probe, GrowthProbeRequest};
use crate::sem5::typed_mechanism::{
    validate_typed_mechanism_improvement_operator, validate_typed_mechanism_synthesis_receipt,
    TypedMechanismImprovementOperatorIR, TypedMechanismSynthesisGoalIR,
    TypedMechanismSynthesisReceiptIR,
};

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
const MAX_IMPROVEMENT_OPERATOR_GRAPH_VERIFIED_ARTIFACTS: u64 = 4_096;
const MAX_FULLSTACK_VERIFIED_ARTIFACTS: u64 = 3;
const MAX_SELF_HEALING_VERIFIED_ARTIFACTS: u64 = 1;
const MAX_ARTIFACT_CONTEXT_ATTEMPTS: usize = MAX_VERIFIED_ARTIFACTS_PER_CYCLE * 4;
const FRONTIER_EVIDENCE_CONTRACT_REVISION: u64 = 2;
const BEHAVIORAL_HEURISTIC_EXCLUSION_CONTRACT_REVISION: u64 = 4;
const WRAPPER_CAPABILITY_CONTRACT_REVISION: u64 = 5;
const BEHAVIORAL_VALUE_CONTRACT_REVISION: u64 = 6;
const IMPROVEMENT_OPERATOR_GRAPH_CONTRACT_REVISION: u64 = 2;
const BEHAVIORAL_EXECUTION_SCHEMA_LEGACY: &str = "B_CORE_BEHAVIORAL_COMPOSITION_EXECUTION_3";
const BEHAVIORAL_EXECUTION_SCHEMA: &str = "B_CORE_BEHAVIORAL_COMPOSITION_EXECUTION_4";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GenerativePredictorIR {
    Sem23ReactionOutcome,
    Sem25MultiHorizonRouter,
}

impl GenerativePredictorIR {
    const fn metadata(self) -> (&'static str, &'static str) {
        match self {
            Self::Sem23ReactionOutcome => (
                "SEM23_REACTION_OUTCOME_PREDICTOR",
                "sem23::engine::predict_base_properties",
            ),
            Self::Sem25MultiHorizonRouter => (
                "SEM25_MULTI_HORIZON_ROUTER",
                "sem25::engine::run_growth_probe",
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GenerativeComposerIR {
    Sem5Program,
    SelfHealingContract,
    FullstackTypedRecipe,
    ImprovementOperatorProgram,
    ImprovementOperatorGraph,
}

impl GenerativeComposerIR {
    const fn metadata(self) -> (&'static str, &'static str) {
        match self {
            Self::Sem5Program => (
                "SEM5_PROGRAM_IR_COMPOSER",
                "integrated_development::compose_existing_sem5_capability",
            ),
            Self::SelfHealingContract => (
                "SELF_HEALING_CONTRACT_COMPOSER",
                "self_healing_pipeline::validate_composition_lesson",
            ),
            Self::FullstackTypedRecipe => (
                "FULLSTACK_TYPED_RECIPE_COMPOSER",
                "fullstack_ops_knowledge::recipe_as_composition_lesson",
            ),
            Self::ImprovementOperatorProgram => (
                "IMPROVEMENT_OPERATOR_PROGRAM_COMPOSER",
                "autonomous_source_mutation::execute_improvement_operator_behavioral_canary",
            ),
            Self::ImprovementOperatorGraph => (
                "IMPROVEMENT_OPERATOR_GRAPH_COMPOSER",
                "autonomous_source_mutation::execute_improvement_operator_graph_behavioral_canary",
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GenerativeVerifierIR {
    IndependentGrowthVerifier,
}

impl GenerativeVerifierIR {
    const fn metadata(self) -> (&'static str, &'static str) {
        match self {
            Self::IndependentGrowthVerifier => (
                "INDEPENDENT_GROWTH_VERIFIER",
                "growth_supervisor::run_verifier_request",
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerativeExecutionPlanIR {
    pub predictor: GenerativePredictorIR,
    pub composer: GenerativeComposerIR,
    pub verifier: GenerativeVerifierIR,
}
const GENERATIVE_PREDICTORS: [GenerativePredictorIR; 2] = [
    GenerativePredictorIR::Sem23ReactionOutcome,
    GenerativePredictorIR::Sem25MultiHorizonRouter,
];
const GENERATIVE_COMPOSERS: [GenerativeComposerIR; 5] = [
    GenerativeComposerIR::Sem5Program,
    GenerativeComposerIR::SelfHealingContract,
    GenerativeComposerIR::FullstackTypedRecipe,
    GenerativeComposerIR::ImprovementOperatorProgram,
    GenerativeComposerIR::ImprovementOperatorGraph,
];
const GENERATIVE_VERIFIERS: [GenerativeVerifierIR; 1] =
    [GenerativeVerifierIR::IndependentGrowthVerifier];
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub typed_behavior_goals: Vec<TypedMechanismSynthesisGoalIR>,
    /// Executable proposals derived from already-authorized operators. The
    /// public goals and independent verifier remain the acceptance authority.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub typed_behavior_operator_proposals: Vec<TypedMechanismImprovementOperatorIR>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub executable_performance_operators: Vec<ImprovementOperatorIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReusableCompositionMemory {
    pub composition: RepairCompositionLessonIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_plan: Option<GenerativeExecutionPlanIR>,
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
    /// Exact machine-consumable payload used to create each typed artifact.
    /// Hash/context pairs without this payload are legacy canary evidence and
    /// cannot reconstruct or install a learned program.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub verified_typed_behavior_goals: BTreeMap<String, TypedMechanismSynthesisGoalIR>,
    /// Exact synthesis receipts retained at behavioral verification time.
    /// Goals explain the desired behavior; these receipts preserve the
    /// executable recipe that can be promoted after source installation.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub verified_typed_mechanism_receipts: BTreeMap<String, TypedMechanismSynthesisReceiptIR>,
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
            .flat_map(|composition| {
                composition
                    .verified_artifact_sha256s
                    .iter()
                    .filter(|artifact| composition.has_executable_artifact(artifact))
            })
            .collect::<BTreeSet<_>>()
            .len()
            .min(u64::MAX as usize) as u64
    }
}

impl ReusableCompositionMemory {
    /// Returns true only when the serialized explanatory composition is an
    /// exact projection of the typed executable plan. Consumers outside this
    /// module must never rediscover execution authority from primitive names.
    pub fn has_executable_composer(&self, composer: GenerativeComposerIR) -> bool {
        self.execution_plan.is_some_and(|plan| {
            plan.composer == composer && execution_plan_matches_metadata(&self.composition, plan)
        })
    }

    pub fn has_executable_artifact(&self, artifact_sha256: &str) -> bool {
        let Some(plan) = self.execution_plan else {
            return false;
        };
        if !execution_plan_matches_metadata(&self.composition, plan) {
            return false;
        }
        if !self
            .verified_artifact_contexts
            .contains_key(artifact_sha256)
        {
            return false;
        }
        match plan.composer {
            GenerativeComposerIR::Sem5Program => {
                let Some(goal) = self.verified_typed_behavior_goals.get(artifact_sha256) else {
                    return false;
                };
                if !validate_typed_behavior_goal_for_memory(goal) {
                    return false;
                }
                match self.verified_typed_mechanism_receipts.get(artifact_sha256) {
                    Some(receipt) => {
                        validate_typed_mechanism_synthesis_receipt(receipt).is_ok()
                            && receipt.synthesis_request.as_ref() == Some(goal)
                    }
                    // Revision-6 sealed memories predate exact receipt
                    // retention. They remain reconstructable from their
                    // context-bound typed goal; all new promotions carry the
                    // stronger exact receipt.
                    None => true,
                }
            }
            // These catalogs remain direct workflow canaries. Until their
            // exact executable payload is retained in this memory schema they
            // cannot contribute generative capability units.
            GenerativeComposerIR::SelfHealingContract
            | GenerativeComposerIR::FullstackTypedRecipe
            | GenerativeComposerIR::ImprovementOperatorProgram
            | GenerativeComposerIR::ImprovementOperatorGraph => false,
        }
    }
}

fn validate_typed_behavior_goal_for_memory(goal: &TypedMechanismSynthesisGoalIR) -> bool {
    crate::sem5::typed_mechanism::validate_typed_mechanism_synthesis_goal(goal).is_ok()
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_execution_plan: Option<GenerativeExecutionPlanIR>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_plan: Option<GenerativeExecutionPlanIR>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typed_behavior_goal: Option<TypedMechanismSynthesisGoalIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typed_mechanism_synthesis_receipt: Option<TypedMechanismSynthesisReceiptIR>,
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

fn candidate_composition(plan: GenerativeExecutionPlanIR) -> RepairCompositionLessonIR {
    let predictor = plan.predictor.metadata();
    let composer = plan.composer.metadata();
    let verifier = plan.verifier.metadata();
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
    let mut signature = primitives
        .iter()
        .map(|value| value.primitive_id.as_str())
        .collect::<Vec<_>>()
        .join(":");
    if plan.composer == GenerativeComposerIR::ImprovementOperatorGraph {
        signature.push_str(&format!(
            ":CONTRACT_REVISION_{IMPROVEMENT_OPERATOR_GRAPH_CONTRACT_REVISION}"
        ));
    }
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

fn stage_metadata_matches(
    composition: &RepairCompositionLessonIR,
    role: &str,
    expected: (&str, &str),
) -> bool {
    composition.primitives.iter().any(|primitive| {
        primitive.semantic_role == role
            && primitive.primitive_id == expected.0
            && primitive.implementation_anchor == expected.1
    })
}

fn execution_plan_matches_metadata(
    composition: &RepairCompositionLessonIR,
    plan: GenerativeExecutionPlanIR,
) -> bool {
    stage_metadata_matches(composition, "PREDICT", plan.predictor.metadata())
        && stage_metadata_matches(composition, "COMPOSE", plan.composer.metadata())
        && stage_metadata_matches(composition, "VERIFY", plan.verifier.metadata())
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
    let goal_hashes = input
        .typed_behavior_goals
        .iter()
        .filter_map(|goal| serde_json::to_vec(goal).ok())
        .map(|bytes| sha256(&bytes))
        .collect::<Vec<_>>()
        .join(":");
    let performance_operator_ids = input
        .executable_performance_operators
        .iter()
        .map(|operator| operator.operator_id.as_str())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(":");
    let typed_operator_proposal_ids = input
        .typed_behavior_operator_proposals
        .iter()
        .map(|operator| operator.operator_id.as_str())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(":");
    sha256(
        format!(
            "signals={signals}|roles={roles}|goals={goal_hashes}|typed_operator_proposals={typed_operator_proposal_ids}|performance_operators={performance_operator_ids}|measured_gain={}",
            input.measured_performance_gain,
        )
        .as_bytes(),
    )
}

fn domain_bonus(plan: GenerativeExecutionPlanIR, input: &GenerativeInput) -> u16 {
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
    if plan.predictor == GenerativePredictorIR::Sem25MultiHorizonRouter
        && (signals.contains("CAPABILITY_SURFACE_ADDED")
            || signals.contains("MUTUAL_REVALIDATION_GAP"))
    {
        bonus += 8;
    }
    if plan.predictor == GenerativePredictorIR::Sem23ReactionOutcome && roles.len() >= 2 {
        bonus += 7;
    }
    if plan.composer == GenerativeComposerIR::SelfHealingContract
        && (signals.contains("DEFECT_REPAIR") || roles.contains("IMPLEMENTATION_REPAIR"))
    {
        bonus += 12;
    }
    if plan.composer == GenerativeComposerIR::FullstackTypedRecipe
        && ["FRONTEND_CONTRACT", "BACKEND_CONTRACT", "OPERATIONS_CHANGE"]
            .iter()
            .any(|signal| signals.contains(signal))
    {
        bonus += 12;
    }
    if plan.composer == GenerativeComposerIR::Sem5Program
        && (roles.len() >= 2 || !input.typed_behavior_goals.is_empty())
    {
        bonus += 6;
    }
    if plan.composer == GenerativeComposerIR::ImprovementOperatorProgram
        && (signals.contains("CAPABILITY_SURFACE_ADDED")
            || signals.contains("DEFECT_REPAIR")
            || roles.contains("IMPLEMENTATION")
            || !input.executable_performance_operators.is_empty())
    {
        bonus += 10;
    }
    if plan.composer == GenerativeComposerIR::ImprovementOperatorGraph
        && (signals.contains("CAPABILITY_SURFACE_ADDED")
            || signals.contains("DEFECT_REPAIR")
            || roles.len() >= 2)
    {
        bonus += 14;
    }
    bonus
}

fn applicable_policy_signals(
    plan: GenerativeExecutionPlanIR,
    input: &GenerativeInput,
) -> Vec<String> {
    let mut supported = BTreeSet::new();
    if plan.predictor == GenerativePredictorIR::Sem25MultiHorizonRouter {
        supported.extend(["MUTUAL_REVALIDATION_GAP", "CAPABILITY_SURFACE_ADDED"]);
    }
    if plan.composer == GenerativeComposerIR::SelfHealingContract {
        supported.extend(["DEFECT_REPAIR", "ERROR_HANDLING_ADDED", "VALIDATION_ADDED"]);
    }
    if plan.composer == GenerativeComposerIR::FullstackTypedRecipe {
        supported.extend(["FRONTEND_CONTRACT", "BACKEND_CONTRACT", "OPERATIONS_CHANGE"]);
    }
    if plan.composer == GenerativeComposerIR::Sem5Program {
        supported.extend(["CODE_CHANGE", "REFACTOR", "CAPABILITY_SURFACE_ADDED"]);
    }
    if plan.composer == GenerativeComposerIR::ImprovementOperatorProgram {
        supported.extend([
            "CODE_CHANGE",
            "REFACTOR",
            "CAPABILITY_SURFACE_ADDED",
            "DEFECT_REPAIR",
            "VALIDATION_ADDED",
        ]);
    }
    if plan.composer == GenerativeComposerIR::ImprovementOperatorGraph {
        supported.extend([
            "CODE_CHANGE",
            "REFACTOR",
            "CAPABILITY_SURFACE_ADDED",
            "DEFECT_REPAIR",
            "VALIDATION_ADDED",
            "COMPOSITIONAL_REPAIR",
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

/// Executes independent, pure behavioral probes as a bounded worker graph and
/// restores deterministic input order at the join. This keeps verification
/// reproducible while avoiding an accidental serial critical path.
fn parallel_execute_ordered<T, R, F>(
    items: Vec<T>,
    parallel_cost_per_item: usize,
    worker: F,
) -> Result<Vec<R>, String>
where
    T: Sync,
    R: Send,
    F: Fn(&T) -> Result<R, String> + Sync,
{
    if items.is_empty() {
        return Ok(Vec::new());
    }
    let available_workers = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1);
    let worker_count = available_workers
        .checked_div(parallel_cost_per_item.max(1))
        .unwrap_or(1)
        .max(1)
        .min(items.len())
        .max(1);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results = std::sync::Mutex::new(Vec::<(usize, Result<R, String>)>::with_capacity(
        items.len(),
    ));
    std::thread::scope(|scope| -> Result<(), String> {
        let mut handles = Vec::with_capacity(worker_count);
        for _ in 0..worker_count {
            handles.push(scope.spawn(|| -> Result<(), String> {
                loop {
                    let index = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(item) = items.get(index) else {
                        break;
                    };
                    let result = worker(item);
                    results
                        .lock()
                        .map_err(|_| "GENERATIVE_PARALLEL_RESULT_LOCK_POISONED".to_string())?
                        .push((index, result));
                }
                Ok(())
            }));
        }
        for handle in handles {
            handle
                .join()
                .map_err(|_| "GENERATIVE_PARALLEL_WORKER_PANICKED".to_string())??;
        }
        Ok(())
    })?;
    let mut ordered = results
        .into_inner()
        .map_err(|_| "GENERATIVE_PARALLEL_RESULT_LOCK_POISONED".to_string())?;
    ordered.sort_by_key(|(index, _)| *index);
    ordered.into_iter().map(|(_, result)| result).collect()
}

fn execute_predictor(
    predictor: GenerativePredictorIR,
    input: &GenerativeInput,
    seed: u64,
) -> Result<String, String> {
    match predictor {
        GenerativePredictorIR::Sem23ReactionOutcome => {
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
        GenerativePredictorIR::Sem25MultiHorizonRouter => {
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
    }
}

fn bounded_binomial(n: usize, k: usize) -> u64 {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    let mut value = 1_u64;
    for step in 0..k {
        value = value
            .saturating_mul((n - step).min(u64::MAX as usize) as u64)
            .checked_div((step + 1).min(u64::MAX as usize) as u64)
            .unwrap_or(u64::MAX);
    }
    value
}

fn improvement_operator_graph_capacity(operator_count: usize) -> u64 {
    (2..=MAX_IMPROVEMENT_OPERATOR_GRAPH_NODES.min(operator_count))
        .map(|arity| bounded_binomial(operator_count, arity))
        .fold(0_u64, u64::saturating_add)
        .min(MAX_IMPROVEMENT_OPERATOR_GRAPH_VERIFIED_ARTIFACTS)
}

fn operator_combination_for_ordinal(
    operator_ids: &[String],
    arity: usize,
    mut ordinal: u64,
) -> Option<Vec<String>> {
    if !(2..=operator_ids.len()).contains(&arity)
        || ordinal >= bounded_binomial(operator_ids.len(), arity)
    {
        return None;
    }
    let mut combination = Vec::with_capacity(arity);
    let mut start = 0_usize;
    for position in 0..arity {
        let remaining = arity - position - 1;
        let last_candidate = operator_ids.len().checked_sub(remaining + 1)?;
        let mut selected = None;
        for candidate in start..=last_candidate {
            let suffix_count = bounded_binomial(operator_ids.len() - candidate - 1, remaining);
            if ordinal < suffix_count {
                selected = Some(candidate);
                break;
            }
            ordinal = ordinal.saturating_sub(suffix_count);
        }
        let selected = selected?;
        combination.push(operator_ids[selected].clone());
        start = selected + 1;
    }
    Some(combination)
}

fn improvement_operator_graph_for_global_ordinal(
    operator_ids: &[String],
    mut ordinal: u64,
) -> Option<Vec<String>> {
    for arity in 2..=MAX_IMPROVEMENT_OPERATOR_GRAPH_NODES.min(operator_ids.len()) {
        let width = bounded_binomial(operator_ids.len(), arity);
        if ordinal < width {
            return operator_combination_for_ordinal(operator_ids, arity, ordinal);
        }
        ordinal = ordinal.saturating_sub(width);
    }
    None
}

fn execute_composer(
    composer: GenerativeComposerIR,
    selected: &RepairCompositionLessonIR,
    input: &GenerativeInput,
    context: &str,
    artifact_family_width: usize,
    previously_verified: &BTreeSet<String>,
    verified_operator_ids: &BTreeSet<String>,
) -> Result<(Vec<VerifiedBehavioralArtifact>, Option<String>), String> {
    match composer {
        GenerativeComposerIR::Sem5Program => {
            if input.observed_composition_roles.len() < 2 && input.typed_behavior_goals.is_empty() {
                return Ok((
                    Vec::new(),
                    Some("SEM5_REQUIRES_OBSERVED_ROLES_OR_A_TYPED_BEHAVIOR_GOAL".to_string()),
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
            if !input.typed_behavior_goals.is_empty() {
                for (ordinal, goal) in input
                    .typed_behavior_goals
                    .iter()
                    .take(target_width)
                    .enumerate()
                {
                    let artifact_context = sha256(
                        format!(
                            "{context}:{}:{ordinal}:PUBLIC_TYPED_BEHAVIOR_GOAL",
                            selected.composition_id
                        )
                        .as_bytes(),
                    );
                    let receipt = execute_typed_behavior_goal_canary_with_operator_proposals(
                        &artifact_context,
                        goal,
                        &input.typed_behavior_operator_proposals,
                    )?;
                    if receipt.cases_executed == 0 || receipt.cases_passed != receipt.cases_executed
                    {
                        return Err("TYPED_BEHAVIOR_GOAL_CANARY_INCOMPLETE".to_string());
                    }
                    if artifact_hashes.insert(receipt.program_ir_sha256.clone()) {
                        artifacts.push(VerifiedBehavioralArtifact {
                            artifact_context_sha256: artifact_context,
                            artifact_sha256: receipt.program_ir_sha256,
                            cases_executed: receipt.cases_executed,
                            cases_passed: receipt.cases_passed,
                            typed_behavior_goal: Some(goal.clone()),
                            typed_mechanism_synthesis_receipt: receipt
                                .typed_mechanism_synthesis_receipt,
                        });
                    }
                }
                if artifacts.is_empty() {
                    return Ok((
                        Vec::new(),
                        Some("LESSON_BOUND_EXECUTABLE_KNOWLEDGE_ALREADY_VERIFIED".to_string()),
                    ));
                }
                return Ok((artifacts, None));
            }
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
                        typed_behavior_goal: None,
                        typed_mechanism_synthesis_receipt: None,
                    });
                }
            }
            if artifacts.is_empty() {
                return Err("SEM5_BEHAVIORAL_ARTIFACT_FAMILY_EMPTY".to_string());
            }
            Ok((artifacts, None))
        }
        GenerativeComposerIR::SelfHealingContract => {
            if artifact_family_width == 0 {
                return Ok((
                    Vec::new(),
                    Some("SELF_HEALING_ARTIFACT_CAPACITY_REACHED".to_string()),
                ));
            }
            let receipt = execute_self_healing_behavioral_canary()?;
            if receipt.cases_executed == 0
                || receipt.cases_passed != receipt.cases_executed
                || receipt.fresh_candidate_sha256s.len() < 3
                || !receipt.negative_non_applicability_observed
                || !receipt.defect_class_mismatch_rejected
                || receipt.exact_patch_lookup_events != 0
                || receipt.codex_calls != 0
                || receipt.external_llm_calls != 0
                || receipt.network_reads != 0
                || receipt.network_writes != 0
            {
                return Err("SELF_HEALING_BEHAVIORAL_CANARY_INCOMPLETE".to_string());
            }
            if previously_verified.contains(&receipt.behavioral_artifact_sha256) {
                return Ok((
                    Vec::new(),
                    Some("SELF_HEALING_EXECUTABLE_UNIVERSE_SATURATED".to_string()),
                ));
            }
            Ok((
                vec![VerifiedBehavioralArtifact {
                    artifact_context_sha256: sha256(
                        format!("{context}:GENERALIZED_SELF_HEALING_REPAIR").as_bytes(),
                    ),
                    artifact_sha256: receipt.behavioral_artifact_sha256,
                    cases_executed: receipt.cases_executed,
                    cases_passed: receipt.cases_passed,
                    typed_behavior_goal: None,
                    typed_mechanism_synthesis_receipt: None,
                }],
                None,
            ))
        }
        GenerativeComposerIR::FullstackTypedRecipe => {
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
                        typed_behavior_goal: None,
                        typed_mechanism_synthesis_receipt: None,
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
        GenerativeComposerIR::ImprovementOperatorProgram => {
            if artifact_family_width == 0 {
                return Ok((
                    Vec::new(),
                    Some("IMPROVEMENT_OPERATOR_ARTIFACT_CAPACITY_REACHED".to_string()),
                ));
            }
            let target_width = artifact_family_width.clamp(1, MAX_VERIFIED_ARTIFACTS_PER_CYCLE);
            let mut artifacts = Vec::new();
            let mut artifact_hashes = previously_verified.clone();
            if !input.executable_performance_operators.is_empty() {
                let applicable_source = "pub fn measured(value: u32) -> bool { value % 2 == 0 }\n";
                let negative_source = "pub fn measured(value: u32) -> bool { value > 0 }\n";
                for operator in input
                    .executable_performance_operators
                    .iter()
                    .take(target_width)
                {
                    let positive =
                        execute_improvement_operator_on_source(operator, applicable_source)?;
                    let negative =
                        execute_improvement_operator_on_source(operator, negative_source)?;
                    if !positive.applicable
                        || positive.candidate_source.is_none()
                        || negative.applicable
                        || negative.candidate_source.is_some()
                    {
                        return Err("PERFORMANCE_OPERATOR_BEHAVIORAL_CANARY_INCOMPLETE".to_string());
                    }
                    if artifact_hashes.insert(operator.operator_id.clone()) {
                        artifacts.push(VerifiedBehavioralArtifact {
                            artifact_context_sha256: sha256(
                                format!(
                                    "{context}:{}:EXECUTABLE_PERFORMANCE_OPERATOR",
                                    operator.operator_id
                                )
                                .as_bytes(),
                            ),
                            artifact_sha256: operator.operator_id.clone(),
                            cases_executed: 2,
                            cases_passed: 2,
                            typed_behavior_goal: None,
                            typed_mechanism_synthesis_receipt: None,
                        });
                    }
                }
                if artifacts.is_empty() {
                    return Ok((
                        Vec::new(),
                        Some("PERFORMANCE_OPERATOR_ALREADY_VERIFIED".to_string()),
                    ));
                }
                return Ok((artifacts, None));
            }
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
                        typed_behavior_goal: None,
                        typed_mechanism_synthesis_receipt: None,
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
        GenerativeComposerIR::ImprovementOperatorGraph => {
            if artifact_family_width == 0 {
                return Ok((
                    Vec::new(),
                    Some("IMPROVEMENT_OPERATOR_GRAPH_CAPACITY_REACHED".to_string()),
                ));
            }
            let target_width = artifact_family_width.clamp(1, MAX_VERIFIED_ARTIFACTS_PER_CYCLE);
            let operator_ids = verified_operator_ids.iter().cloned().collect::<Vec<_>>();
            let graph_capacity = improvement_operator_graph_capacity(operator_ids.len());
            let mut ordinal = if previously_verified.is_empty() {
                0
            } else {
                previously_verified.len().min(u64::MAX as usize) as u64
            };
            let mut tasks = Vec::new();
            while ordinal < graph_capacity && tasks.len() < target_width {
                let Some(graph_operator_ids) =
                    improvement_operator_graph_for_global_ordinal(&operator_ids, ordinal)
                else {
                    break;
                };
                ordinal = ordinal.saturating_add(1);
                let graph_id = improvement_operator_graph_id_for_nodes(&graph_operator_ids)?;
                if previously_verified.contains(&graph_id) {
                    continue;
                }
                let artifact_context = sha256(
                    format!(
                        "{context}:{}:{graph_id}:IMPROVEMENT_OPERATOR_PARALLEL_GRAPH",
                        selected.composition_id
                    )
                    .as_bytes(),
                );
                tasks.push((graph_operator_ids, artifact_context));
            }
            if tasks.is_empty() {
                return Ok((
                    Vec::new(),
                    Some("IMPROVEMENT_OPERATOR_GRAPH_UNIVERSE_SATURATED".to_string()),
                ));
            }
            let parallel_cost = tasks.iter().map(|task| task.0.len()).max().unwrap_or(1);
            let receipts = parallel_execute_ordered(tasks, parallel_cost, |task| {
                execute_improvement_operator_graph_family_behavioral_canary(&task.0, &task.1)
            })?;
            let mut artifacts = Vec::with_capacity(receipts.len());
            for receipt in receipts {
                if receipt.cases_executed == 0
                    || receipt.cases_passed != receipt.cases_executed
                    || !receipt.parallel_nodes_executed
                    || !receipt.exact_postimages_observed
                    || !receipt.negative_controls_rejected
                    || !receipt.canonical_join_observed
                {
                    return Err(
                        "IMPROVEMENT_OPERATOR_GRAPH_BEHAVIORAL_CANARY_INCOMPLETE".to_string()
                    );
                }
                artifacts.push(VerifiedBehavioralArtifact {
                    artifact_context_sha256: receipt.context_sha256,
                    artifact_sha256: receipt.graph.graph_id,
                    cases_executed: receipt.cases_executed,
                    cases_passed: receipt.cases_passed,
                    typed_behavior_goal: None,
                    typed_mechanism_synthesis_receipt: None,
                });
            }
            Ok((artifacts, None))
        }
    }
}

/// The catalog also contains knowledge-only composers used by direct repair
/// workflows. They remain callable there, but must not compete for generative
/// frontier budget until this cycle can execute and independently observe the
/// artifact they produce. Graph validation or a typed recipe alone is not a
/// capability outcome.
fn composition_uses_composer(
    composition: &ReusableCompositionMemory,
    composer: GenerativeComposerIR,
) -> bool {
    composition.execution_plan.is_some_and(|plan| {
        plan.composer == composer && execution_plan_matches_metadata(&composition.composition, plan)
    })
}

fn distinct_verified_artifact_count_for_composer(
    memory: &GenerativeGrowthMemory,
    composer: GenerativeComposerIR,
) -> u64 {
    memory
        .accepted_compositions
        .iter()
        .filter(|composition| composition_uses_composer(composition, composer))
        .flat_map(|composition| {
            composition
                .verified_artifact_sha256s
                .iter()
                .filter(|artifact| composition.has_executable_artifact(artifact))
        })
        .collect::<BTreeSet<_>>()
        .len()
        .min(u64::MAX as usize) as u64
}

fn verified_artifacts_for_composer(
    memory: &GenerativeGrowthMemory,
    composer: GenerativeComposerIR,
) -> BTreeSet<String> {
    memory
        .accepted_compositions
        .iter()
        .filter(|composition| composition_uses_composer(composition, composer))
        .flat_map(|composition| {
            composition
                .verified_artifact_sha256s
                .iter()
                .filter(|artifact| composition.has_executable_artifact(artifact))
                .cloned()
        })
        .collect()
}

fn verified_artifact_capacity(
    memory: &GenerativeGrowthMemory,
    composer: GenerativeComposerIR,
) -> u64 {
    match composer {
        GenerativeComposerIR::Sem5Program => MAX_SEM5_VERIFIED_ARTIFACTS,
        GenerativeComposerIR::ImprovementOperatorProgram => {
            MAX_IMPROVEMENT_OPERATOR_VERIFIED_ARTIFACTS
        }
        GenerativeComposerIR::ImprovementOperatorGraph => {
            let operators = distinct_verified_artifact_count_for_composer(
                memory,
                GenerativeComposerIR::ImprovementOperatorProgram,
            );
            improvement_operator_graph_capacity(operators.min(usize::MAX as u64) as usize)
        }
        GenerativeComposerIR::FullstackTypedRecipe => MAX_FULLSTACK_VERIFIED_ARTIFACTS,
        GenerativeComposerIR::SelfHealingContract => MAX_SELF_HEALING_VERIFIED_ARTIFACTS,
    }
}

fn verified_artifact_family_width(
    memory: &GenerativeGrowthMemory,
    composer: GenerativeComposerIR,
) -> usize {
    let verified = distinct_verified_artifact_count_for_composer(memory, composer);
    let remaining = verified_artifact_capacity(memory, composer).saturating_sub(verified);
    usize::try_from(
        verified
            .max(1)
            .min(remaining)
            .min(MAX_VERIFIED_ARTIFACTS_PER_CYCLE as u64),
    )
    .unwrap_or(MAX_VERIFIED_ARTIFACTS_PER_CYCLE)
}

fn composer_is_behaviorally_executable(
    composer: GenerativeComposerIR,
    memory: &GenerativeGrowthMemory,
) -> bool {
    match composer {
        GenerativeComposerIR::Sem5Program => verified_artifact_family_width(memory, composer) > 0,
        GenerativeComposerIR::ImprovementOperatorProgram => {
            verified_artifact_family_width(memory, GenerativeComposerIR::Sem5Program) == 0
                && verified_artifact_family_width(memory, composer) > 0
        }
        GenerativeComposerIR::ImprovementOperatorGraph => {
            verified_artifact_family_width(memory, GenerativeComposerIR::Sem5Program) == 0
                && verified_artifact_family_width(
                    memory,
                    GenerativeComposerIR::ImprovementOperatorProgram,
                ) == 0
                && verified_artifact_family_width(memory, composer) > 0
        }
        GenerativeComposerIR::FullstackTypedRecipe => {
            verified_artifact_family_width(memory, GenerativeComposerIR::Sem5Program) == 0
                && verified_artifact_family_width(
                    memory,
                    GenerativeComposerIR::ImprovementOperatorProgram,
                ) == 0
                && verified_artifact_family_width(
                    memory,
                    GenerativeComposerIR::ImprovementOperatorGraph,
                ) == 0
                && verified_artifact_family_width(memory, composer) > 0
        }
        GenerativeComposerIR::SelfHealingContract => {
            verified_artifact_family_width(memory, GenerativeComposerIR::Sem5Program) == 0
                && verified_artifact_family_width(
                    memory,
                    GenerativeComposerIR::ImprovementOperatorProgram,
                ) == 0
                && verified_artifact_family_width(
                    memory,
                    GenerativeComposerIR::ImprovementOperatorGraph,
                ) == 0
                && verified_artifact_family_width(
                    memory,
                    GenerativeComposerIR::FullstackTypedRecipe,
                ) == 0
                && verified_artifact_family_width(memory, composer) > 0
        }
    }
}

/// A canary proves that a fixed implementation still works; it does not prove
/// that knowledge from the current lesson was recognized or used. Only the
/// typed SEM-5 path currently consumes a lesson-bound executable payload.
/// The other composers remain callable by their direct workflows, but cannot
/// turn labels, prose or a static fixture replay into generative frontier.
fn composer_consumes_executable_knowledge(
    composer: GenerativeComposerIR,
    input: &GenerativeInput,
) -> bool {
    composer == GenerativeComposerIR::Sem5Program && !input.typed_behavior_goals.is_empty()
}

fn composer_is_executable_for_input(
    composer: GenerativeComposerIR,
    memory: &GenerativeGrowthMemory,
    input: &GenerativeInput,
) -> bool {
    composer_is_behaviorally_executable(composer, memory)
        && composer_consumes_executable_knowledge(composer, input)
}

pub fn executable_generative_substrate_available(memory: &GenerativeGrowthMemory) -> bool {
    // Availability here means that at least one lesson-bound compiler still
    // has capacity. Input-specific eligibility is checked before execution.
    composer_is_behaviorally_executable(GenerativeComposerIR::Sem5Program, memory)
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
    execution_plan: GenerativeExecutionPlanIR,
    input: &GenerativeInput,
    seed: u64,
    artifact_family_width: usize,
    previously_verified: &BTreeSet<String>,
    verified_operator_ids: &BTreeSet<String>,
) -> Result<BehavioralCompositionExecutionReceipt, String> {
    if !execution_plan_matches_metadata(selected, execution_plan) {
        return Err("GENERATIVE_EXECUTION_PLAN_METADATA_MISMATCH".to_string());
    }
    let context = context_sha256(input);
    let predictor_id = execution_plan.predictor.metadata().0.to_string();
    let composer_id = execution_plan.composer.metadata().0.to_string();
    let verifier_id = execution_plan.verifier.metadata().0.to_string();
    let predictor_output_sha256 = execute_predictor(execution_plan.predictor, input, seed)?;
    let (verified_artifacts, abstention_reason) = execute_composer(
        execution_plan.composer,
        selected,
        input,
        &context,
        artifact_family_width,
        previously_verified,
        verified_operator_ids,
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
        schema: BEHAVIORAL_EXECUTION_SCHEMA.to_string(),
        context_sha256: context,
        execution_plan: Some(execution_plan),
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
    execution_plan: GenerativeExecutionPlanIR,
    input: &GenerativeInput,
    memory: &GenerativeGrowthMemory,
) -> CandidatePrediction {
    let context = context_sha256(input);
    let compatible_behavioral_values = memory.behavioral_value_contract_revision
        >= BEHAVIORAL_HEURISTIC_EXCLUSION_CONTRACT_REVISION;
    let reusable = compatible_behavioral_values.then(|| {
        memory.accepted_compositions.iter().find(|candidate| {
            candidate.composition.composition_id == composition.composition_id
                && candidate.execution_plan == Some(execution_plan)
                && execution_plan_matches_metadata(&candidate.composition, execution_plan)
        })
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
        .saturating_add(domain_bonus(execution_plan, input))
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
    if input.typed_behavior_operator_proposals.len() > MAX_VERIFIED_ARTIFACTS_PER_CYCLE {
        return Err("GENERATIVE_TYPED_OPERATOR_PROPOSAL_BOUND".to_string());
    }
    let mut proposal_ids = BTreeSet::new();
    for proposal in &input.typed_behavior_operator_proposals {
        validate_typed_mechanism_improvement_operator(proposal)?;
        let compatible_goal = input.typed_behavior_goals.iter().any(|goal| {
            goal.output_type == proposal.output_type
                && goal
                    .operands
                    .iter()
                    .map(|operand| &operand.value_type)
                    .eq(proposal.operand_types.iter())
        });
        if !proposal_ids.insert(proposal.operator_id.as_str()) || !compatible_goal {
            return Err("GENERATIVE_TYPED_OPERATOR_PROPOSAL_UNBOUND".to_string());
        }
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
                if !composer_is_executable_for_input(composer, memory, input) {
                    behaviorally_inapplicable_candidates_screened =
                        behaviorally_inapplicable_candidates_screened.saturating_add(1);
                    continue;
                }
                let execution_plan = GenerativeExecutionPlanIR {
                    predictor,
                    composer,
                    verifier,
                };
                let composition = candidate_composition(execution_plan);
                let prediction = prediction_score(&composition, execution_plan, input, memory);
                let tie = sha256(format!("{}:{}", seed, composition.composition_id).as_bytes());
                candidates.push((
                    prediction.selection_score,
                    tie,
                    prediction,
                    execution_plan,
                    composition,
                ));
            }
        }
    }
    candidates.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    let (_, _, prediction, selected_execution_plan, selected) = candidates
        .first()
        .cloned()
        .ok_or_else(|| "NO_BEHAVIORALLY_EXECUTABLE_GENERATIVE_COMPOSITION_CANDIDATE".to_string())?;
    let predicted_value = prediction.predicted_value;
    let typecheck_pass = validate_composition_lesson(&selected).is_ok();
    let selected_composer = selected_execution_plan.composer;
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
    let verified_operator_ids =
        verified_artifacts_for_composer(memory, GenerativeComposerIR::ImprovementOperatorProgram);
    let behavioral_execution_receipt = execute_behavioral_composition(
        &selected,
        selected_execution_plan,
        input,
        seed,
        artifact_family_width,
        &previously_verified,
        &verified_operator_ids,
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
    // Prediction ranks which bounded experiment to run. Once the selected
    // program has passed the typed public contract, a stale predictor must be
    // calibrated from that result rather than vetoing the verified behavior.
    // Otherwise every underprediction becomes a permanent capability ceiling.
    let structural_candidate = typecheck_pass && input.verification_evidence_count > 0;
    let valuable = structural_candidate && behavioral_composition_executed && observed_value >= 72;
    let previously_verified = memory
        .accepted_compositions
        .iter()
        .flat_map(|candidate| {
            candidate
                .verified_artifact_sha256s
                .iter()
                .filter(|artifact| candidate.has_executable_artifact(artifact))
        })
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
    // Exploration controls which candidate is tried; it must not control
    // whether independently verified new behavior is retained. A candidate
    // selected from prior trials can still materialize a distinct artifact,
    // and dropping it here traps the supervisor on a productive plateau.
    let accepted_for_memory = valuable && novel_verified_artifact;
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
        applicable_policy_signals(selected_execution_plan, input)
    } else {
        Vec::new()
    };
    Ok(GenerativeCycleResult {
        schema: GENERATIVE_GROWTH_SCHEMA.to_string(),
        source_lesson_id: input.source_lesson_id.clone(),
        candidates_considered: candidates.len(),
        behaviorally_inapplicable_candidates_screened,
        selected_composition: selected,
        selected_execution_plan: Some(selected_execution_plan),
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
            let Some(execution_plan) = result.selected_execution_plan else {
                return false;
            };
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
            matches!(
                receipt.schema.as_str(),
                BEHAVIORAL_EXECUTION_SCHEMA | BEHAVIORAL_EXECUTION_SCHEMA_LEGACY
            ) && receipt.context_sha256 == result.context_sha256
                && receipt.execution_plan == Some(execution_plan)
                && execution_plan_matches_metadata(&result.selected_composition, execution_plan)
                && expected_receipt_sha256.as_deref() == Some(receipt.receipt_sha256.as_str())
                && receipt.predictor_id == execution_plan.predictor.metadata().0
                && receipt.composer_id == execution_plan.composer.metadata().0
                && receipt.verifier_id == execution_plan.verifier.metadata().0
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
                                && match execution_plan.composer {
                                    GenerativeComposerIR::Sem5Program => {
                                        match (
                                            artifact.typed_behavior_goal.as_ref(),
                                            artifact.typed_mechanism_synthesis_receipt.as_ref(),
                                        ) {
                                            (Some(goal), Some(synthesis)) => {
                                                validate_typed_behavior_goal_for_memory(goal)
                                                    && validate_typed_mechanism_synthesis_receipt(
                                                        synthesis,
                                                    )
                                                    .is_ok()
                                                    && synthesis.synthesis_request.as_ref()
                                                        == Some(goal)
                                            }
                                            (Some(goal), None) => {
                                                receipt.schema == BEHAVIORAL_EXECUTION_SCHEMA_LEGACY
                                                    && validate_typed_behavior_goal_for_memory(goal)
                                            }
                                            _ => false,
                                        }
                                    }
                                    _ => {
                                        artifact.typed_behavior_goal.is_none()
                                            && artifact.typed_mechanism_synthesis_receipt.is_none()
                                    }
                                }
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
    let Some(execution_plan) = result.selected_execution_plan else {
        return Err("GENERATIVE_PROMOTION_BOUNDARY_FAILURE".to_string());
    };
    let result_artifacts = result
        .behavioral_execution_receipt
        .as_ref()
        .map(|receipt| receipt.verified_artifacts.as_slice())
        .unwrap_or_default();
    let previously_verified = current
        .accepted_compositions
        .iter()
        .flat_map(|candidate| {
            candidate
                .verified_artifact_sha256s
                .iter()
                .filter(|artifact| candidate.has_executable_artifact(artifact))
        })
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let expected_valuable = result.composition_typecheck_pass
        && input.verification_evidence_count > 0
        && result.behavioral_composition_executed
        && result.observed_value >= 72;
    let expected_novel_verified_artifact_count = if expected_valuable {
        result_artifacts
            .iter()
            .filter(|artifact| !previously_verified.contains(artifact.artifact_sha256.as_str()))
            .count()
    } else {
        0
    };
    let expected_novel_verified_artifact = expected_novel_verified_artifact_count > 0;
    let expected_accepted_for_memory = expected_valuable && expected_novel_verified_artifact;
    let expected_novel_context_transfer = expected_valuable
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
        || !execution_plan_matches_metadata(&result.selected_composition, execution_plan)
        || !composer_is_executable_for_input(execution_plan.composer, current, input)
        || !result.prediction_recorded_before_composition
        || !result.selected_from_precomposition_prediction
        || result.observed_value_is_heuristic_proxy
        || result.valuable != expected_valuable
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
    if next.behavioral_value_contract_revision < WRAPPER_CAPABILITY_CONTRACT_REVISION {
        next.legacy_wrapper_frontier_advance_events = next
            .legacy_wrapper_frontier_advance_events
            .saturating_add(next.frontier_advance_events);
        next.frontier_advance_events = 0;
        next.frontier_capability_units = next.distinct_verified_artifact_count();
        next.behavioral_value_contract_revision = WRAPPER_CAPABILITY_CONTRACT_REVISION;
    }
    if next.behavioral_value_contract_revision < BEHAVIORAL_VALUE_CONTRACT_REVISION {
        // Revision 6 changes only the authority boundary: a prediction ranks
        // experiments, while executed public-contract evidence decides value.
        // Existing typed executable memories and frontier units remain valid.
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
                && candidate.execution_plan == Some(execution_plan)
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
                if let Some(goal) = &artifact.typed_behavior_goal {
                    existing
                        .verified_typed_behavior_goals
                        .insert(artifact.artifact_sha256.clone(), goal.clone());
                }
                if let Some(receipt) = &artifact.typed_mechanism_synthesis_receipt {
                    existing
                        .verified_typed_mechanism_receipts
                        .insert(artifact.artifact_sha256.clone(), receipt.clone());
                }
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
            let verified_typed_behavior_goals = result_artifacts
                .iter()
                .filter_map(|artifact| {
                    artifact
                        .typed_behavior_goal
                        .as_ref()
                        .map(|goal| (artifact.artifact_sha256.clone(), goal.clone()))
                })
                .collect::<BTreeMap<_, _>>();
            let verified_typed_mechanism_receipts = result_artifacts
                .iter()
                .filter_map(|artifact| {
                    artifact
                        .typed_mechanism_synthesis_receipt
                        .as_ref()
                        .map(|receipt| (artifact.artifact_sha256.clone(), receipt.clone()))
                })
                .collect::<BTreeMap<_, _>>();
            next.accepted_compositions.push(ReusableCompositionMemory {
                composition: result.selected_composition.clone(),
                execution_plan: Some(execution_plan),
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
                verified_typed_behavior_goals,
                verified_typed_mechanism_receipts,
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

    fn executable_goal() -> TypedMechanismSynthesisGoalIR {
        use crate::sem5::model::Value;
        use crate::sem5::model::{DataSplit, Effect, ProgramType};
        use crate::sem5::typed_mechanism::{
            SourceOperandIR, TypedMechanismObservationIR, TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA,
        };

        TypedMechanismSynthesisGoalIR {
            schema: TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA.to_string(),
            goal_id: "generative-test-goal".to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![SourceOperandIR {
                role: "ARG_0".to_string(),
                source: "value".to_string(),
                value_type: ProgramType::Int,
            }],
            output_type: ProgramType::Int,
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: Vec::new(),
            postconditions: vec!["result preserves the public integer observation".to_string()],
            invariants: Vec::new(),
            public_observations: vec![TypedMechanismObservationIR {
                operands: [("ARG_0".to_string(), Value::Int(7))].into_iter().collect(),
                expected_postimage: Value::Int(7),
            }],
            require_conditional: false,
            max_expression_depth: 2,
            max_candidates: 32,
            provenance: vec!["EXECUTABLE_TEST_FIXTURE".to_string()],
        }
    }

    fn verified_operator_canary_ids() -> Vec<String> {
        (0..MAX_IMPROVEMENT_OPERATOR_SELECTORS)
            .map(|selector| {
                let context = format!("{selector:08x}{}", "0".repeat(56));
                execute_improvement_operator_behavioral_canary(&context)
                    .unwrap()
                    .operator
                    .operator_id
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

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
            typed_behavior_goals: vec![executable_goal()],
            typed_behavior_operator_proposals: Vec::new(),
            executable_performance_operators: Vec::new(),
        }
    }

    #[test]
    fn derived_operator_proposal_executes_and_promotes_through_the_normal_frontier() {
        use crate::sem5::model::{DataSplit, Effect, ProgramType, Value};
        use crate::sem5::typed_mechanism::{
            compose_authorized_typed_operator_programs, synthesize_typed_mechanism_goal,
            typed_mechanism_improvement_operator_from_receipt, SourceOperandIR,
            TypedMechanismObservationIR, TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA,
        };

        let operator = |goal_id: &str, samples: &[(i64, i64, i64)], evidence: char| {
            let request = TypedMechanismSynthesisGoalIR {
                schema: TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA.to_string(),
                goal_id: goal_id.to_string(),
                split: DataSplit::FreshBlind,
                operands: vec![
                    SourceOperandIR {
                        role: "left".to_string(),
                        source: "input.left".to_string(),
                        value_type: ProgramType::Int,
                    },
                    SourceOperandIR {
                        role: "right".to_string(),
                        source: "input.right".to_string(),
                        value_type: ProgramType::Int,
                    },
                ],
                output_type: ProgramType::Int,
                definitions: Vec::new(),
                allowed_effects: vec![Effect::Pure],
                preconditions: Vec::new(),
                postconditions: Vec::new(),
                invariants: Vec::new(),
                public_observations: samples
                    .iter()
                    .map(|(left, right, expected)| TypedMechanismObservationIR {
                        operands: BTreeMap::from([
                            ("left".to_string(), Value::Int(*left)),
                            ("right".to_string(), Value::Int(*right)),
                        ]),
                        expected_postimage: Value::Int(*expected),
                    })
                    .collect(),
                require_conditional: false,
                max_expression_depth: 2,
                max_candidates: 1_024,
                provenance: vec!["AUTHORIZED_COMPONENT_TEST".to_string()],
            };
            let receipt = synthesize_typed_mechanism_goal(&request).unwrap();
            typed_mechanism_improvement_operator_from_receipt(
                &receipt,
                evidence.to_string().repeat(64),
            )
            .unwrap()
        };
        let addition = operator("frontier-add", &[(2, 3, 5), (-4, 9, 5)], 'a');
        let multiplication = operator("frontier-mul", &[(2, 3, 6), (-4, 2, -8)], 'b');
        let program = compose_authorized_typed_operator_programs(
            &[addition.clone(), multiplication.clone()],
            8,
        )
        .unwrap()
        .into_iter()
        .find(|program| {
            program.producer_operator_id == addition.operator_id
                && program.consumer_operator_id == multiplication.operator_id
        })
        .unwrap();
        let input = GenerativeInput {
            source_lesson_id: "recursive-operator-frontier".to_string(),
            diagnostic_signals: vec!["AUTONOMOUS_OPERATOR_RECURSIVE_COMPOSITION".to_string()],
            observed_composition_roles: vec!["PREDICT".to_string(), "COMPOSE".to_string()],
            learning_score: 100,
            verification_evidence_count: 2,
            measured_performance_gain: false,
            typed_behavior_goals: vec![program.goal.clone()],
            typed_behavior_operator_proposals: vec![program.operator_proposal.clone()],
            executable_performance_operators: Vec::new(),
        };
        let cycle = run_generative_cycle(&GenerativeGrowthMemory::default(), &input, 27).unwrap();
        assert!(cycle.frontier_advance);
        let synthesis = cycle
            .behavioral_execution_receipt
            .as_ref()
            .unwrap()
            .verified_artifacts[0]
            .typed_mechanism_synthesis_receipt
            .as_ref()
            .unwrap();
        assert_eq!(
            synthesis.selected_operator_id.as_deref(),
            Some(program.operator_proposal.operator_id.as_str())
        );
        let mut legacy_goal_only = cycle.clone();
        {
            let execution = legacy_goal_only
                .behavioral_execution_receipt
                .as_mut()
                .unwrap();
            execution.schema = BEHAVIORAL_EXECUTION_SCHEMA_LEGACY.to_string();
            for artifact in &mut execution.verified_artifacts {
                artifact.typed_mechanism_synthesis_receipt = None;
            }
            execution.receipt_sha256 = behavioral_execution_receipt_sha256(execution).unwrap();
            legacy_goal_only.behavioral_verification_sha256 =
                Some(execution.receipt_sha256.clone());
        }
        assert!(validate_behavioral_execution_receipt(&legacy_goal_only));
        {
            let execution = legacy_goal_only
                .behavioral_execution_receipt
                .as_mut()
                .unwrap();
            execution.schema = BEHAVIORAL_EXECUTION_SCHEMA.to_string();
            execution.receipt_sha256 = behavioral_execution_receipt_sha256(execution).unwrap();
            legacy_goal_only.behavioral_verification_sha256 =
                Some(execution.receipt_sha256.clone());
        }
        assert!(!validate_behavioral_execution_receipt(&legacy_goal_only));
        let promoted =
            promote_generative_cycle(&GenerativeGrowthMemory::default(), &input, &cycle).unwrap();
        assert_eq!(promoted.frontier_advance_events, 1);

        let mut unbound = input;
        unbound.typed_behavior_goals[0].output_type = ProgramType::Bool;
        assert_eq!(
            run_generative_cycle(&GenerativeGrowthMemory::default(), &unbound, 28),
            Err("GENERATIVE_TYPED_OPERATOR_PROPOSAL_UNBOUND".to_string())
        );
    }

    #[test]
    fn operator_graph_ordinal_expands_arity_without_new_stage_code() {
        let operator_ids = verified_operator_canary_ids();
        let pair_count = bounded_binomial(operator_ids.len(), 2);
        let last_pair = improvement_operator_graph_for_global_ordinal(
            &operator_ids,
            pair_count.saturating_sub(1),
        )
        .unwrap();
        let first_triple =
            improvement_operator_graph_for_global_ordinal(&operator_ids, pair_count).unwrap();
        assert_eq!(last_pair.len(), 2);
        assert_eq!(first_triple.len(), 3);
        assert_eq!(
            improvement_operator_graph_capacity(operator_ids.len()),
            MAX_IMPROVEMENT_OPERATOR_GRAPH_VERIFIED_ARTIFACTS
        );
    }

    #[test]
    fn prediction_precedes_isolated_typed_composition() {
        let result = run_generative_cycle(&GenerativeGrowthMemory::default(), &input(), 7).unwrap();
        assert_eq!(result.candidates_considered, 2);
        assert_eq!(result.behaviorally_inapplicable_candidates_screened, 8);
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
                    && receipt.cases_executed > 0
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
        let promoted =
            promote_generative_cycle(&GenerativeGrowthMemory::default(), &input(), &result)
                .unwrap();
        let accepted = promoted.accepted_compositions.first().unwrap();
        assert_eq!(accepted.verified_typed_behavior_goals.len(), 1);
        assert_eq!(accepted.verified_typed_mechanism_receipts.len(), 1);
        let artifact = accepted.verified_artifact_sha256s.first().unwrap();
        assert!(accepted.has_executable_artifact(artifact));
    }

    #[test]
    fn stale_underprediction_cannot_veto_an_executed_public_contract() {
        let mut low_confidence_input = input();
        low_confidence_input.learning_score = 0;
        low_confidence_input.diagnostic_signals.clear();
        low_confidence_input.observed_composition_roles.clear();
        let memory = GenerativeGrowthMemory::default();

        let result = run_generative_cycle(&memory, &low_confidence_input, 7).unwrap();

        assert!(result.prediction_error > 30);
        assert_eq!(result.observed_value, 100);
        assert!(result.behavioral_composition_executed);
        assert!(result.valuable);
        assert!(result.accepted_for_memory);
        assert!(result.frontier_advance);
        let promoted = promote_generative_cycle(&memory, &low_confidence_input, &result).unwrap();
        assert_eq!(promoted.frontier_capability_units, 1);
        assert_eq!(
            promoted.behavioral_value_contract_revision,
            BEHAVIORAL_VALUE_CONTRACT_REVISION
        );
    }

    #[test]
    fn verified_novel_artifact_is_retained_after_exploration_is_exhausted() {
        let current_input = input();
        let mut memory = GenerativeGrowthMemory::default();

        // Record one verified trial for each bounded candidate without adding
        // reusable memory. The next selection is therefore exploitation, but
        // its independently verified artifact is still new to memory.
        for seed in [7, 19] {
            let result = run_generative_cycle(&memory, &current_input, seed).unwrap();
            assert!(result.exploration_selected);
            memory.composition_trials.push(GenerativeCompositionTrial {
                composition_id: result.selected_composition.composition_id,
                context_sha256: result.context_sha256,
                predicted_value: result.predicted_value,
                observed_value: result.observed_value,
                valuable: result.valuable,
            });
        }

        let result = run_generative_cycle(&memory, &current_input, 31).unwrap();
        assert!(!result.exploration_selected);
        assert!(result.reused_memory_composition_id.is_none());
        assert!(result.valuable);
        assert!(result.novel_verified_artifact);
        assert!(result.accepted_for_memory);
        assert!(result.frontier_advance);

        let promoted = promote_generative_cycle(&memory, &current_input, &result).unwrap();
        assert_eq!(promoted.accepted_compositions.len(), 1);
        assert_eq!(promoted.frontier_advance_events, 1);
    }

    #[test]
    fn repeated_text_context_cannot_mint_another_executable_artifact() {
        let mut memory = GenerativeGrowthMemory::default();
        let first = input();
        let result = run_generative_cycle(&memory, &first, 7).unwrap();
        memory = promote_generative_cycle(&memory, &first, &result).unwrap();
        let mut repeated = input();
        repeated.source_lesson_id = "renamed-text-lesson".to_string();
        repeated
            .diagnostic_signals
            .push("DEFECT_REPAIR".to_string());
        let repeated = run_generative_cycle(&memory, &repeated, 99).unwrap();
        assert!(!repeated.behavioral_composition_executed);
        assert!(!repeated.novel_verified_artifact);
        assert!(!repeated.frontier_advance);
        assert!(!repeated.applied_to_self_improvement);
        assert_eq!(repeated.verified_artifact_count, 0);
        assert_eq!(memory.distinct_verified_artifact_count(), 1);
    }

    #[test]
    fn text_only_input_has_no_generative_compiler_candidate() {
        let mut text_only = input();
        text_only.typed_behavior_goals.clear();
        assert_eq!(
            run_generative_cycle(&GenerativeGrowthMemory::default(), &text_only, 19),
            Err("NO_BEHAVIORALLY_EXECUTABLE_GENERATIVE_COMPOSITION_CANDIDATE".to_string())
        );
    }

    #[test]
    fn artifact_hashes_without_executable_payload_cannot_saturate_growth() {
        let mut hash_only = GenerativeGrowthMemory::default();
        let execution_plan = GenerativeExecutionPlanIR {
            predictor: GENERATIVE_PREDICTORS[0],
            composer: GENERATIVE_COMPOSERS[0],
            verifier: GENERATIVE_VERIFIERS[0],
        };
        hash_only
            .accepted_compositions
            .push(ReusableCompositionMemory {
                composition: candidate_composition(execution_plan),
                execution_plan: Some(execution_plan),
                trigger_signals: Vec::new(),
                source_lesson_ids: Vec::new(),
                predicted_value: 100,
                observed_value: 100,
                reuse_count: 0,
                context_use_counts: BTreeMap::new(),
                successful_uses: 1,
                observed_value_total: 100,
                verified_artifact_sha256s: (0..MAX_SEM5_VERIFIED_ARTIFACTS)
                    .map(|ordinal| sha256(format!("sem5-{ordinal}").as_bytes()))
                    .collect(),
                verified_artifact_contexts: (0..MAX_SEM5_VERIFIED_ARTIFACTS)
                    .map(|ordinal| {
                        (
                            sha256(format!("sem5-{ordinal}").as_bytes()),
                            sha256(format!("context-{ordinal}").as_bytes()),
                        )
                    })
                    .collect(),
                verified_typed_behavior_goals: BTreeMap::new(),
                verified_typed_mechanism_receipts: BTreeMap::new(),
            });
        assert_eq!(hash_only.distinct_verified_artifact_count(), 0);
        assert!(executable_generative_substrate_available(&hash_only));
        assert!(run_generative_cycle(&hash_only, &input(), 11).is_ok());

        let mut legacy_text_only = hash_only;
        legacy_text_only.accepted_compositions[0].execution_plan = None;
        assert_eq!(legacy_text_only.distinct_verified_artifact_count(), 0);
        assert!(executable_generative_substrate_available(&legacy_text_only));
        assert!(run_generative_cycle(&legacy_text_only, &input(), 11).is_ok());
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

        let repeated = first_input.clone();
        let result = run_generative_cycle(&memory, &repeated, 9).unwrap();
        assert!(!result.frontier_advance);
        let next = promote_generative_cycle(&memory, &repeated, &result).unwrap();

        assert_eq!(next.legacy_wrapper_frontier_advance_events, 7);
        assert_eq!(next.frontier_advance_events, 0);
        assert_eq!(next.frontier_capability_units, 1);
    }

    #[test]
    fn revision_five_executable_memory_migrates_without_frontier_loss() {
        let first_input = input();
        let first_result =
            run_generative_cycle(&GenerativeGrowthMemory::default(), &first_input, 7).unwrap();
        let mut revision_five = promote_generative_cycle(
            &GenerativeGrowthMemory::default(),
            &first_input,
            &first_result,
        )
        .unwrap();
        revision_five.behavioral_value_contract_revision = WRAPPER_CAPABILITY_CONTRACT_REVISION;
        assert_eq!(revision_five.frontier_capability_units, 1);

        let mut next_input = first_input;
        next_input.source_lesson_id = "revision-six-authority-migration".to_string();
        let result = run_generative_cycle(&revision_five, &next_input, 17).unwrap();
        let migrated = promote_generative_cycle(&revision_five, &next_input, &result).unwrap();

        assert_eq!(
            migrated.behavioral_value_contract_revision,
            BEHAVIORAL_VALUE_CONTRACT_REVISION
        );
        assert_eq!(migrated.frontier_capability_units, 1);
        assert_eq!(migrated.frontier_advance_events, 1);
        assert_eq!(migrated.legacy_wrapper_frontier_advance_events, 0);
        assert_eq!(migrated.accepted_compositions.len(), 1);
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
        assert!(!result.behavioral_composition_executed);
        assert!(!result.frontier_advance);
        let next = promote_generative_cycle(&legacy, &next_input, &result).unwrap();

        assert_eq!(
            next.behavioral_value_contract_revision,
            BEHAVIORAL_VALUE_CONTRACT_REVISION
        );
        assert_eq!(next.legacy_heuristic_composition_trials, 1);
        assert_eq!(next.legacy_heuristic_accepted_compositions, 1);
        assert_eq!(next.composition_trials.len(), 1);
        assert_eq!(next.accepted_compositions.len(), 0);
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
    fn receipt_without_exact_executable_goal_cannot_become_growth() {
        let memory = GenerativeGrowthMemory::default();
        let current = input();
        let mut result = run_generative_cycle(&memory, &current, 7).unwrap();
        let receipt = result
            .behavioral_execution_receipt
            .as_mut()
            .expect("behavioral receipt");
        receipt.verified_artifacts[0].typed_behavior_goal = None;
        receipt.receipt_sha256 = behavioral_execution_receipt_sha256(receipt).unwrap();
        result.behavioral_verification_sha256 = Some(receipt.receipt_sha256.clone());
        assert!(!validate_behavioral_execution_receipt(&result));
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
