use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::sem31::verifier::{Provenance, RelationTerm, SemanticTerm, StateChannel};

pub const CONTRACT_VERSION: &str = "SEM32_LITERATURE_INFORMED_CAUSAL_VERIFIER_1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BeliefTruth {
    KnownTrue,
    KnownFalse,
    Unknown,
    Believed,
    CompetingHypotheses,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UncertaintyKind {
    None,
    Epistemic,
    AleatoricOrWorldStochasticity,
    InsufficientModelSupport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventRole {
    Action,
    Intervention,
    ExogenousEvent,
    PassiveObservation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvidenceMode {
    Observational,
    Interventional,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DynamicEntity {
    pub entity: u64,
    pub material: SemanticTerm,
    pub state_value: i64,
    pub confidence_bps: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalRelation {
    pub source: u64,
    pub relation: RelationTerm,
    pub target: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DistractorFact {
    pub entity: u64,
    pub semantic: SemanticTerm,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeliefAnchor {
    pub family_code: u16,
    pub entities: Vec<DynamicEntity>,
    pub relations: Vec<LocalRelation>,
    pub distractor_facts: Vec<DistractorFact>,
    pub hidden_context_belief: BeliefTruth,
}

impl BeliefAnchor {
    pub fn normalized(mut self) -> Self {
        self.entities.sort();
        self.relations.sort();
        self.distractor_facts.sort();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticEvent {
    pub operator: SemanticTerm,
    pub role: EventRole,
    pub actor: u64,
    pub target: u64,
    pub magnitude: i64,
    pub observation_lag: u8,
    pub hidden_context_intervention: Option<bool>,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StateDelta {
    pub entity: u64,
    pub channel: StateChannel,
    pub change: i64,
    pub confidence_bps: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RelationDelta {
    pub source: u64,
    pub relation: RelationTerm,
    pub target: u64,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PropertyDelta {
    pub entity: u64,
    pub property: SemanticTerm,
    pub active: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SemanticWorldDelta {
    pub state_changes: Vec<StateDelta>,
    pub relation_changes: Vec<RelationDelta>,
    pub property_changes: Vec<PropertyDelta>,
    pub created_entities: Vec<u64>,
    pub destroyed_entities: Vec<u64>,
}

impl SemanticWorldDelta {
    pub fn normalized(mut self) -> Self {
        self.state_changes.sort();
        self.relation_changes.sort();
        self.property_changes.sort();
        self.created_entities.sort_unstable();
        self.destroyed_entities.sort_unstable();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FutureBranch {
    pub delta: SemanticWorldDelta,
    pub confidence_bps: u16,
    pub uncertainty: UncertaintyKind,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlausibleDeltaSet {
    pub branches: Vec<FutureBranch>,
}

impl PlausibleDeltaSet {
    pub fn normalized(mut self) -> Self {
        for branch in &mut self.branches {
            branch.delta = branch.delta.clone().normalized();
        }
        self.branches.sort();
        self.branches.dedup();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionCase {
    pub case_id: u64,
    pub sequence_code: u64,
    pub time_index: u64,
    pub anchor: BeliefAnchor,
    pub event: SemanticEvent,
    pub evidence_mode: EvidenceMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedTransition {
    pub case: TransitionCase,
    pub visible_delta: SemanticWorldDelta,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RolloutCase {
    pub rollout_id: u64,
    pub anchor: BeliefAnchor,
    pub events: Vec<SemanticEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CounterfactualCase {
    pub counterfactual_id: u64,
    pub anchor: BeliefAnchor,
    pub actual_event: SemanticEvent,
    pub alternatives: Vec<SemanticEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReachabilityStatus {
    ReachableWithinBudget,
    ReachableEventually,
    Unreachable,
    UnknownReachability,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReachabilityQuery {
    pub query_id: u64,
    pub anchor_node: u16,
    pub goal_node: u16,
    pub action_budget: u16,
    pub edges: Vec<(u16, u16, u16)>,
    pub graph_complete: bool,
    pub semantic_similarity_hint: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalPathStep {
    pub from: u16,
    pub mechanism_code: u16,
    pub to: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReachabilityResult {
    pub query_id: u64,
    pub status: ReachabilityStatus,
    pub path_certificate: Vec<CausalPathStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalChallenge {
    pub contract_version: String,
    pub instance_id: u64,
    pub seed: u64,
    pub action_vocabulary: Vec<SemanticTerm>,
    pub material_vocabulary: Vec<SemanticTerm>,
    pub relation_semantic: RelationTerm,
    pub state_channel: StateChannel,
    pub observational_cases: Vec<TransitionCase>,
    pub intervention_candidates: Vec<TransitionCase>,
    pub prediction_cases: Vec<TransitionCase>,
    pub rollout_cases: Vec<RolloutCase>,
    pub counterfactual_cases: Vec<CounterfactualCase>,
    pub reachability_queries: Vec<ReachabilityQuery>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MechanismClass {
    DeterministicContextual,
    Stochastic,
    Delayed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalMechanismIr {
    pub mechanism_id: u64,
    pub class: MechanismClass,
    pub operator: SemanticTerm,
    pub required_relation: RelationTerm,
    pub required_material: SemanticTerm,
    pub requires_hidden_context_true: bool,
    pub observation_lag: u8,
    pub state_channel: StateChannel,
    pub confidence_bps: u16,
    pub observational_support: u64,
    pub interventional_support: u64,
    pub provenance_codes: Vec<u64>,
    pub verification_events: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledCausalNode {
    pub node_id: u64,
    pub source_mechanism: CausalMechanismIr,
    pub decomposable: bool,
    pub semantic_dag_available: bool,
    pub applicability_guard_preserved: bool,
    pub deep_depth: u64,
    pub compiled_depth: u64,
    pub deep_cost: u64,
    pub compiled_cost: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrozenPrediction {
    pub case_id: u64,
    pub plausible_deltas: PlausibleDeltaSet,
    pub active_entity_count: u64,
    pub active_mechanism_count: u64,
    pub active_semantic_nodes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterventionRecord {
    pub round: u64,
    pub information_value: u64,
    pub prediction: FrozenPrediction,
    pub observation: ObservedTransition,
    pub residual_class_code: u16,
    pub reduced_hypothesis_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RolloutPrediction {
    pub rollout_id: u64,
    pub step_predictions: Vec<PlausibleDeltaSet>,
    pub failure_class_codes: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CounterfactualPrediction {
    pub counterfactual_id: u64,
    pub actual_prediction: PlausibleDeltaSet,
    pub alternative_predictions: Vec<PlausibleDeltaSet>,
    pub actual_anchor_unchanged: bool,
    pub copy_on_write_delta_branches: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingPoint {
    pub world_entities: u64,
    pub total_mechanisms: u64,
    pub sparse_entity_touches: u64,
    pub sparse_mechanism_touches: u64,
    pub full_route_entity_touches: u64,
    pub full_route_mechanism_touches: u64,
    pub result_equivalent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerAudit {
    pub persistent_world_layer_present: bool,
    pub belief_world_layer_present: bool,
    pub active_world_slice_present: bool,
    pub active_projection_can_mutate_canonical_world_semantics: bool,
    pub persistent_distractor_facts_retained: u64,
    pub irrelevant_active_semantic_load: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AblationSubmission {
    pub no_law_predictions: Vec<FrozenPrediction>,
    pub observation_only_predictions: Vec<FrozenPrediction>,
    pub non_factored_predictions: Vec<FrozenPrediction>,
    pub uncertainty_removed_predictions: Vec<FrozenPrediction>,
    pub association_counterfactuals: Vec<CounterfactualPrediction>,
    pub compiled_predictions: Vec<FrozenPrediction>,
    pub decompressed_predictions: Vec<FrozenPrediction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instrumentation {
    pub layer_audit: LayerAudit,
    pub predictions_frozen_before_reveal: u64,
    pub future_state_reads_before_prediction: u64,
    pub full_predicted_world_snapshot_copies: u64,
    pub unchanged_semantic_rewrite_events: u64,
    pub unobserved_state_hallucinated_as_fact: u64,
    pub predictive_uncertainty_collapse_events: u64,
    pub stochastic_future_collapse_events: u64,
    pub unsupported_rollout_confident_hallucinations: u64,
    pub wasted_exploration_on_irreducible_noise: u64,
    pub false_causal_promotions: u64,
    pub counterfactual_to_actual_mutation_events: u64,
    pub actual_hidden_future_to_counterfactual_leakage_events: u64,
    pub false_entity_reidentification_events: u64,
    pub unreachable_shortcut_accepts: u64,
    pub unsafe_causal_shortcut_accepts: u64,
    pub world_memory_full_scans: u64,
    pub causal_mechanism_full_scans: u64,
    pub task_instance_transition_cache_authority: bool,
    pub world_generator_is_success_authority: bool,
    pub causal_gold_law_reads: u64,
    pub expected_next_state_lookups: u64,
    pub future_world_event_leakage_events: u64,
    pub counterfactual_gold_branch_reads: u64,
    pub natural_language_is_canonical_world_memory: bool,
    pub natural_language_is_causal_reasoning_authority: bool,
    pub world_memory_natural_language_bytes_on_hot_path: u64,
    pub human_causal_experiment_selection_events: u64,
    pub human_causal_hypothesis_selection_events: u64,
    pub causal_mechanism_reuse_events: u64,
    pub causal_mechanism_transfer_events: u64,
    pub prediction_residual_events: u64,
    pub causal_composition_events: u64,
    pub causal_law_refinement_events: u64,
    pub causal_law_split_events: u64,
    pub new_causal_law_genesis_events: u64,
    pub new_causal_primitive_events: u64,
    pub new_semantic_primitive_events: u64,
    pub active_entity_sequence: Vec<u64>,
    pub active_mechanism_sequence: Vec<u64>,
    pub world_memory_bytes_sequence: Vec<u64>,
    pub semantic_reuse_sequence: Vec<u64>,
    pub semantic_composition_sequence: Vec<u64>,
    pub new_primitive_sequence: Vec<u64>,
    pub mechanism_genesis_sequence: Vec<u64>,
    pub mechanism_reuse_sequence: Vec<u64>,
    pub mechanism_transfer_sequence: Vec<u64>,
    pub hypothesis_count_sequence: Vec<u64>,
    pub epistemic_uncertainty_sequence: Vec<u64>,
    pub aleatoric_branch_count_sequence: Vec<u64>,
    pub scaling_points: Vec<ScalingPoint>,
    pub compressed_causal_memory_nodes_promoted: u64,
    pub compressed_causal_memory_decompression_available: bool,
    pub mechanism_bytes: u64,
    pub raw_history_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalSubmission {
    pub observations: Vec<ObservedTransition>,
    pub interventions: Vec<InterventionRecord>,
    pub mechanisms: Vec<CausalMechanismIr>,
    pub compiled_nodes: Vec<CompiledCausalNode>,
    pub one_step_predictions: Vec<FrozenPrediction>,
    pub rollout_predictions: Vec<RolloutPrediction>,
    pub counterfactual_predictions: Vec<CounterfactualPrediction>,
    pub reachability_results: Vec<ReachabilityResult>,
    pub ablations: AblationSubmission,
    pub instrumentation: Instrumentation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "request_type", rename_all = "SCREAMING_SNAKE_CASE")]
#[allow(clippy::large_enum_variant)] // The frozen JSON protocol remains directly inspectable.
pub enum VerificationRequest {
    RevealObservations {
        contract_version: String,
        seed: u64,
        cases: Vec<TransitionCase>,
    },
    RevealIntervention {
        contract_version: String,
        seed: u64,
        case: TransitionCase,
        frozen_prediction: FrozenPrediction,
    },
    EvaluateFinal {
        challenge: CausalChallenge,
        submission: FinalSubmission,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "response_type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationResponse {
    Observations {
        observations: Vec<ObservedTransition>,
    },
    Intervention {
        observation: ObservedTransition,
        prediction_contains_realized_outcome: bool,
    },
    Evaluation {
        result: VerificationResult,
    },
    Rejected {
        reason: String,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VerifiedMetrics {
    pub partial_observability_cases: u64,
    pub hidden_state_hypotheses: u64,
    pub epistemic_uncertainty_events: u64,
    pub aleatoric_stochastic_events: u64,
    pub causal_mechanisms_total: u64,
    pub observational_transitions: u64,
    pub interventional_transitions: u64,
    pub causal_hypothesis_competitions: u64,
    pub autonomous_discriminating_interventions: u64,
    pub hypotheses_resolved: u64,
    pub hidden_context_discovery_events: u64,
    pub delayed_causal_effect_events: u64,
    pub one_step_predictions: u64,
    pub one_step_correct: u64,
    pub multistep_predictions: u64,
    pub horizon_error_sequence: Vec<(u64, u64)>,
    pub structural_delta_error_sequence: Vec<(u64, u64, u64, u64, u64)>,
    pub counterfactual_predictions: u64,
    pub counterfactual_verified: u64,
    pub counterfactual_errors: u64,
    pub reachability_queries: u64,
    pub unreachable_shortcut_cases: u64,
    pub total_world_entities: u64,
    pub active_entities_p50: u64,
    pub active_entities_p95: u64,
    pub active_mechanisms_p50: u64,
    pub active_mechanisms_p95: u64,
    pub novel_entity_count_transfer_pass: bool,
    pub novel_relation_topology_transfer_pass: bool,
    pub entity_id_invariant_transfer_pass: bool,
    pub interventional_causality_ablation_pass: bool,
    pub causal_law_memory_ablation_pass: bool,
    pub factored_dynamics_ablation_pass: bool,
    pub epistemic_uncertainty_ablation_pass: bool,
    pub counterfactual_causal_model_ablation_pass: bool,
    pub sparse_causal_routing_ablation_pass: bool,
    pub compiled_causal_memory_ablation_pass: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub contract_version: String,
    pub accepted: bool,
    pub violations: Vec<String>,
    pub level_a_pass: bool,
    pub level_b_pass: bool,
    pub level_c_pass: bool,
    pub level_d_pass: bool,
    pub level_e_pass: bool,
    pub level_f_pass: bool,
    pub level_g_pass: bool,
    pub level_h_pass: bool,
    pub level_i_pass: bool,
    pub level_j_pass: bool,
    pub metrics: VerifiedMetrics,
}

pub fn handle(request: VerificationRequest) -> VerificationResponse {
    match request {
        VerificationRequest::RevealObservations {
            contract_version,
            seed,
            cases,
        } => {
            if contract_version != CONTRACT_VERSION {
                return VerificationResponse::Rejected {
                    reason: "CONTRACT_VERSION_MISMATCH".into(),
                };
            }
            if cases
                .iter()
                .any(|case| case.evidence_mode != EvidenceMode::Observational)
            {
                return VerificationResponse::Rejected {
                    reason: "NON_OBSERVATIONAL_CASE_IN_OBSERVATION_REVEAL".into(),
                };
            }
            VerificationResponse::Observations {
                observations: cases
                    .into_iter()
                    .map(|case| ObservedTransition {
                        visible_delta: realized_delta(seed, &case),
                        case,
                    })
                    .collect(),
            }
        }
        VerificationRequest::RevealIntervention {
            contract_version,
            seed,
            case,
            frozen_prediction,
        } => {
            if contract_version != CONTRACT_VERSION
                || case.evidence_mode != EvidenceMode::Interventional
                || frozen_prediction.case_id != case.case_id
            {
                return VerificationResponse::Rejected {
                    reason: "INVALID_FROZEN_INTERVENTION_REQUEST".into(),
                };
            }
            let visible_delta = realized_delta(seed, &case);
            let prediction_contains_realized_outcome = frozen_prediction
                .plausible_deltas
                .branches
                .iter()
                .any(|branch| branch.delta.clone().normalized() == visible_delta);
            VerificationResponse::Intervention {
                observation: ObservedTransition {
                    case,
                    visible_delta,
                },
                prediction_contains_realized_outcome,
            }
        }
        VerificationRequest::EvaluateFinal {
            challenge,
            submission,
        } => VerificationResponse::Evaluation {
            result: evaluate(&challenge, &submission),
        },
    }
}

fn evaluate(challenge: &CausalChallenge, submission: &FinalSubmission) -> VerificationResult {
    let mut violations = Vec::new();
    if challenge.contract_version != CONTRACT_VERSION {
        violations.push("CONTRACT_VERSION_MISMATCH".into());
    }

    let roles = role_indices(challenge.seed, challenge.action_vocabulary.len());
    let expected_classes = [
        (MechanismClass::DeterministicContextual, roles.0),
        (MechanismClass::Stochastic, roles.1),
        (MechanismClass::Delayed, roles.2),
    ];
    for (class, index) in expected_classes {
        let Some(mechanism) = submission.mechanisms.iter().find(|m| m.class == class) else {
            violations.push(format!("MISSING_MECHANISM:{class:?}"));
            continue;
        };
        if mechanism.operator != challenge.action_vocabulary[index]
            || mechanism.required_relation != challenge.relation_semantic
            || mechanism.required_material != challenge.material_vocabulary[0]
            || !mechanism.requires_hidden_context_true
            || mechanism.verification_events == 0
        {
            violations.push(format!("INVALID_MECHANISM:{class:?}"));
        }
    }

    if submission.observations.len() != challenge.observational_cases.len()
        || submission.observations.iter().any(|observed| {
            observed.visible_delta != realized_delta(challenge.seed, &observed.case)
        })
    {
        violations.push("OBSERVATIONAL_RECORD_MISMATCH".into());
    }

    let prediction_map: BTreeMap<_, _> = submission
        .one_step_predictions
        .iter()
        .map(|prediction| (prediction.case_id, prediction))
        .collect();
    let mut one_step_correct = 0;
    let mut epistemic_events = 0;
    let mut aleatoric_events = 0;
    for case in &challenge.prediction_cases {
        let expected = plausible_delta_set(challenge.seed, case);
        let Some(actual) = prediction_map.get(&case.case_id) else {
            violations.push(format!("MISSING_ONE_STEP_PREDICTION:{}", case.case_id));
            continue;
        };
        if actual.plausible_deltas.clone().normalized() == expected.clone().normalized() {
            one_step_correct += 1;
        } else {
            violations.push(format!("WRONG_ONE_STEP_PREDICTION:{}", case.case_id));
        }
        epistemic_events += expected
            .branches
            .iter()
            .filter(|branch| branch.uncertainty == UncertaintyKind::Epistemic)
            .count() as u64;
        aleatoric_events += expected
            .branches
            .iter()
            .filter(|branch| branch.uncertainty == UncertaintyKind::AleatoricOrWorldStochasticity)
            .count() as u64;
    }

    let rollout_map: BTreeMap<_, _> = submission
        .rollout_predictions
        .iter()
        .map(|prediction| (prediction.rollout_id, prediction))
        .collect();
    let mut horizon_totals = BTreeMap::<u64, (u64, u64)>::new();
    let mut structural_errors = BTreeMap::<u64, (u64, u64, u64, u64)>::new();
    for rollout in &challenge.rollout_cases {
        let Some(prediction) = rollout_map.get(&rollout.rollout_id) else {
            violations.push(format!("MISSING_ROLLOUT:{}", rollout.rollout_id));
            continue;
        };
        if prediction.step_predictions.len() != rollout.events.len() {
            violations.push(format!("WRONG_ROLLOUT_LENGTH:{}", rollout.rollout_id));
            continue;
        }
        let mut anchor = rollout.anchor.clone();
        for (index, event) in rollout.events.iter().enumerate() {
            let case = TransitionCase {
                case_id: rollout
                    .rollout_id
                    .wrapping_mul(32)
                    .wrapping_add(index as u64),
                sequence_code: rollout.rollout_id,
                time_index: index as u64,
                anchor: anchor.clone(),
                event: event.clone(),
                evidence_mode: EvidenceMode::Observational,
            };
            let expected = plausible_delta_set(challenge.seed, &case);
            let correct = prediction.step_predictions[index].clone().normalized()
                == expected.clone().normalized();
            let horizon = (index + 1) as u64;
            let total = horizon_totals.entry(horizon).or_default();
            total.1 += 1;
            if !correct {
                total.0 += 1;
                structural_errors.entry(horizon).or_default().0 += 1;
            }
            if let Some(branch) = expected.branches.first() {
                anchor = apply_delta(anchor, &branch.delta);
            }
        }
    }

    let counterfactual_map: BTreeMap<_, _> = submission
        .counterfactual_predictions
        .iter()
        .map(|prediction| (prediction.counterfactual_id, prediction))
        .collect();
    let mut counterfactual_verified = 0;
    for case in &challenge.counterfactual_cases {
        let Some(prediction) = counterfactual_map.get(&case.counterfactual_id) else {
            violations.push(format!("MISSING_COUNTERFACTUAL:{}", case.counterfactual_id));
            continue;
        };
        let actual_case = TransitionCase {
            case_id: case.counterfactual_id.wrapping_mul(64),
            sequence_code: case.counterfactual_id,
            time_index: 0,
            anchor: case.anchor.clone(),
            event: case.actual_event.clone(),
            evidence_mode: EvidenceMode::Observational,
        };
        let actual_ok = prediction.actual_prediction.clone().normalized()
            == plausible_delta_set(challenge.seed, &actual_case).normalized();
        let alternatives_ok = case.alternatives.iter().enumerate().all(|(index, event)| {
            let branch_case = TransitionCase {
                case_id: case
                    .counterfactual_id
                    .wrapping_mul(64)
                    .wrapping_add(index as u64 + 1),
                event: event.clone(),
                ..actual_case.clone()
            };
            prediction
                .alternative_predictions
                .get(index)
                .cloned()
                .unwrap_or_default()
                .normalized()
                == plausible_delta_set(challenge.seed, &branch_case).normalized()
        });
        if actual_ok
            && alternatives_ok
            && prediction.actual_anchor_unchanged
            && prediction.copy_on_write_delta_branches
        {
            counterfactual_verified += 1;
        } else {
            violations.push(format!("COUNTERFACTUAL_FAILURE:{}", case.counterfactual_id));
        }
    }

    let reachability_map: BTreeMap<_, _> = submission
        .reachability_results
        .iter()
        .map(|result| (result.query_id, result))
        .collect();
    let mut unreachable_cases = 0;
    for query in &challenge.reachability_queries {
        let expected = solve_reachability(query);
        if expected.status == ReachabilityStatus::Unreachable {
            unreachable_cases += 1;
        }
        if reachability_map.get(&query.query_id).copied() != Some(&expected) {
            violations.push(format!("REACHABILITY_FAILURE:{}", query.query_id));
        }
    }

    let instrumentation = &submission.instrumentation;
    let forbidden_counts = [
        instrumentation.future_state_reads_before_prediction,
        instrumentation.full_predicted_world_snapshot_copies,
        instrumentation.unchanged_semantic_rewrite_events,
        instrumentation.unobserved_state_hallucinated_as_fact,
        instrumentation.predictive_uncertainty_collapse_events,
        instrumentation.stochastic_future_collapse_events,
        instrumentation.unsupported_rollout_confident_hallucinations,
        instrumentation.wasted_exploration_on_irreducible_noise,
        instrumentation.false_causal_promotions,
        instrumentation.counterfactual_to_actual_mutation_events,
        instrumentation.actual_hidden_future_to_counterfactual_leakage_events,
        instrumentation.false_entity_reidentification_events,
        instrumentation.unreachable_shortcut_accepts,
        instrumentation.unsafe_causal_shortcut_accepts,
        instrumentation.world_memory_full_scans,
        instrumentation.causal_mechanism_full_scans,
        instrumentation.causal_gold_law_reads,
        instrumentation.expected_next_state_lookups,
        instrumentation.future_world_event_leakage_events,
        instrumentation.counterfactual_gold_branch_reads,
        instrumentation.world_memory_natural_language_bytes_on_hot_path,
        instrumentation.human_causal_experiment_selection_events,
        instrumentation.human_causal_hypothesis_selection_events,
    ];
    if forbidden_counts.iter().any(|count| *count != 0)
        || instrumentation.task_instance_transition_cache_authority
        || instrumentation.world_generator_is_success_authority
        || instrumentation.natural_language_is_canonical_world_memory
        || instrumentation.natural_language_is_causal_reasoning_authority
    {
        violations.push("FORBIDDEN_AUTHORITY_OR_LEAKAGE_EVENT".into());
    }
    let layer = &instrumentation.layer_audit;
    if !(layer.persistent_world_layer_present
        && layer.belief_world_layer_present
        && layer.active_world_slice_present
        && !layer.active_projection_can_mutate_canonical_world_semantics
        && layer.persistent_distractor_facts_retained > 0
        && layer.irrelevant_active_semantic_load == 0)
    {
        violations.push("THREE_LAYER_WORLD_MODEL_FAILURE".into());
    }
    if instrumentation
        .active_entity_sequence
        .iter()
        .any(|count| *count > 3)
        || instrumentation
            .active_mechanism_sequence
            .iter()
            .any(|count| *count > 2)
        || !instrumentation.scaling_points.iter().any(|point| {
            point.world_entities >= 100_000
                && point.sparse_entity_touches <= 3
                && point.sparse_mechanism_touches <= 2
                && point.result_equivalent
                && point.full_route_entity_touches >= point.world_entities
        })
    {
        violations.push("SPARSE_SCALING_FAILURE".into());
    }

    let baseline_score = one_step_correct as i64;
    let ablation_score = |predictions: &[FrozenPrediction]| -> i64 {
        predictions
            .iter()
            .filter(|prediction| {
                challenge
                    .prediction_cases
                    .iter()
                    .find(|case| case.case_id == prediction.case_id)
                    .map(|case| {
                        prediction.plausible_deltas.clone().normalized()
                            == plausible_delta_set(challenge.seed, case).normalized()
                    })
                    .unwrap_or(false)
            })
            .count() as i64
    };
    let intervention_ablation =
        ablation_score(&submission.ablations.observation_only_predictions) < baseline_score;
    let law_ablation = ablation_score(&submission.ablations.no_law_predictions) < baseline_score;
    let factor_ablation =
        ablation_score(&submission.ablations.non_factored_predictions) < baseline_score;
    let uncertainty_ablation =
        ablation_score(&submission.ablations.uncertainty_removed_predictions) < baseline_score;
    let cf_ablation =
        submission.ablations.association_counterfactuals.len() < counterfactual_verified as usize;
    let sparse_ablation = instrumentation.scaling_points.iter().all(|point| {
        point.full_route_entity_touches > point.sparse_entity_touches
            && point.full_route_mechanism_touches > point.sparse_mechanism_touches
    });
    let compiled_ablation = submission.compiled_nodes.is_empty()
        || (submission.ablations.compiled_predictions
            == submission.ablations.decompressed_predictions
            && submission.compiled_nodes.iter().all(|node| {
                node.decomposable
                    && node.semantic_dag_available
                    && node.applicability_guard_preserved
                    && node.compiled_cost < node.deep_cost
            }));
    if !(intervention_ablation
        && law_ablation
        && factor_ablation
        && uncertainty_ablation
        && cf_ablation
        && sparse_ablation
        && compiled_ablation)
    {
        violations.push("REQUIRED_ABLATION_FAILURE".into());
    }

    if submission.interventions.len() < 5
        || submission
            .interventions
            .iter()
            .any(|record| record.prediction.case_id != record.observation.case.case_id)
        || instrumentation.predictions_frozen_before_reveal < submission.interventions.len() as u64
    {
        violations.push("AUTONOMOUS_INTERVENTION_PROTOCOL_FAILURE".into());
    }
    if instrumentation.causal_composition_events == 0
        || instrumentation.causal_law_refinement_events == 0
        || instrumentation.new_causal_law_genesis_events != 3
        || instrumentation.new_causal_primitive_events != 0
        || instrumentation.new_semantic_primitive_events != 0
    {
        violations.push("COMPOSITION_FIRST_VOCABULARY_PRESSURE_FAILURE".into());
    }

    let level_a = submission.mechanisms.len() == 3 && one_step_correct > 0;
    let level_b = epistemic_events > 0 && aleatoric_events > 0;
    let level_c = intervention_ablation && submission.interventions.len() >= 5;
    let level_d = one_step_correct == challenge.prediction_cases.len() as u64;
    let level_e = horizon_totals.values().all(|(errors, _)| *errors == 0)
        && [1_u64, 2, 4, 8]
            .iter()
            .all(|h| horizon_totals.contains_key(h));
    let level_f = counterfactual_verified == challenge.counterfactual_cases.len() as u64;
    let level_g =
        reachability_map.len() == challenge.reachability_queries.len() && unreachable_cases > 0;
    let level_h = sparse_ablation
        && instrumentation
            .scaling_points
            .iter()
            .any(|p| p.world_entities >= 100_000);
    let level_i = intervention_ablation
        && law_ablation
        && factor_ablation
        && uncertainty_ablation
        && cf_ablation;
    let level_j = compiled_ablation
        && instrumentation.mechanism_bytes < instrumentation.raw_history_bytes
        && instrumentation.causal_mechanism_reuse_events > 0
        && instrumentation.causal_mechanism_transfer_events > 0;

    let metrics = VerifiedMetrics {
        partial_observability_cases: challenge
            .prediction_cases
            .iter()
            .filter(|c| c.anchor.hidden_context_belief == BeliefTruth::Unknown)
            .count() as u64,
        hidden_state_hypotheses: instrumentation
            .hypothesis_count_sequence
            .iter()
            .copied()
            .max()
            .unwrap_or(0),
        epistemic_uncertainty_events: epistemic_events,
        aleatoric_stochastic_events: aleatoric_events,
        causal_mechanisms_total: submission.mechanisms.len() as u64,
        observational_transitions: submission.observations.len() as u64,
        interventional_transitions: submission.interventions.len() as u64,
        causal_hypothesis_competitions: instrumentation.hypothesis_count_sequence.len() as u64,
        autonomous_discriminating_interventions: submission.interventions.len() as u64,
        hypotheses_resolved: submission
            .interventions
            .iter()
            .map(|r| r.reduced_hypothesis_count)
            .sum(),
        hidden_context_discovery_events: submission
            .interventions
            .iter()
            .filter(|r| r.residual_class_code == 1)
            .count() as u64,
        delayed_causal_effect_events: challenge
            .prediction_cases
            .iter()
            .filter(|c| c.event.observation_lag == 1)
            .count() as u64,
        one_step_predictions: challenge.prediction_cases.len() as u64,
        one_step_correct,
        multistep_predictions: horizon_totals.values().map(|(_, total)| total).sum(),
        horizon_error_sequence: horizon_totals
            .into_iter()
            .map(|(h, (e, _))| (h, e))
            .collect(),
        structural_delta_error_sequence: structural_errors
            .into_iter()
            .map(|(h, e)| (h, e.0, e.1, e.2, e.3))
            .collect(),
        counterfactual_predictions: challenge.counterfactual_cases.len() as u64,
        counterfactual_verified,
        counterfactual_errors: challenge.counterfactual_cases.len() as u64
            - counterfactual_verified,
        reachability_queries: challenge.reachability_queries.len() as u64,
        unreachable_shortcut_cases: unreachable_cases,
        total_world_entities: instrumentation
            .scaling_points
            .iter()
            .map(|p| p.world_entities)
            .max()
            .unwrap_or(0),
        active_entities_p50: percentile(&instrumentation.active_entity_sequence, 50),
        active_entities_p95: percentile(&instrumentation.active_entity_sequence, 95),
        active_mechanisms_p50: percentile(&instrumentation.active_mechanism_sequence, 50),
        active_mechanisms_p95: percentile(&instrumentation.active_mechanism_sequence, 95),
        novel_entity_count_transfer_pass: challenge
            .prediction_cases
            .iter()
            .any(|c| c.anchor.entities.len() >= 5),
        novel_relation_topology_transfer_pass: challenge
            .prediction_cases
            .iter()
            .any(|c| c.anchor.relations.len() >= 2),
        entity_id_invariant_transfer_pass: challenge
            .prediction_cases
            .iter()
            .map(|c| c.event.target)
            .collect::<BTreeSet<_>>()
            .len()
            > 4,
        interventional_causality_ablation_pass: intervention_ablation,
        causal_law_memory_ablation_pass: law_ablation,
        factored_dynamics_ablation_pass: factor_ablation,
        epistemic_uncertainty_ablation_pass: uncertainty_ablation,
        counterfactual_causal_model_ablation_pass: cf_ablation,
        sparse_causal_routing_ablation_pass: sparse_ablation,
        compiled_causal_memory_ablation_pass: compiled_ablation,
    };
    VerificationResult {
        contract_version: CONTRACT_VERSION.into(),
        accepted: violations.is_empty()
            && level_a
            && level_b
            && level_c
            && level_d
            && level_e
            && level_f
            && level_g
            && level_h
            && level_i
            && level_j,
        violations,
        level_a_pass: level_a,
        level_b_pass: level_b,
        level_c_pass: level_c,
        level_d_pass: level_d,
        level_e_pass: level_e,
        level_f_pass: level_f,
        level_g_pass: level_g,
        level_h_pass: level_h,
        level_i_pass: level_i,
        level_j_pass: level_j,
        metrics,
    }
}

fn role_indices(seed: u64, count: usize) -> (usize, usize, usize) {
    let first = (mix(seed, 0x00CA_55A1) % count.max(3) as u64) as usize;
    (first % count, (first + 1) % count, (first + 2) % count)
}

fn mechanism_class(seed: u64, case: &TransitionCase) -> Option<MechanismClass> {
    let action_count = 3_usize;
    let operator_index = match &case.event.operator {
        SemanticTerm::Primitive { atom } => atom.value_code as usize % action_count,
        SemanticTerm::Composition { components } => {
            components.first()?.value_code as usize % action_count
        }
    };
    let roles = role_indices(seed, action_count);
    if operator_index == roles.0 {
        Some(MechanismClass::DeterministicContextual)
    } else if operator_index == roles.1 {
        Some(MechanismClass::Stochastic)
    } else if operator_index == roles.2 {
        Some(MechanismClass::Delayed)
    } else {
        None
    }
}

fn applicable(case: &TransitionCase) -> bool {
    let Some(target) = case
        .anchor
        .entities
        .iter()
        .find(|entity| entity.entity == case.event.target)
    else {
        return false;
    };
    let material_ok = match &target.material {
        SemanticTerm::Primitive { atom } => atom.value_code % 2 == 0,
        SemanticTerm::Composition { components } => components
            .first()
            .map(|a| a.value_code % 2 == 0)
            .unwrap_or(false),
    };
    let relation_ok = case.anchor.relations.iter().any(|relation| {
        relation.source == case.event.actor && relation.target == case.event.target
    });
    material_ok && relation_ok
}

fn effect_delta(case: &TransitionCase, change: i64) -> SemanticWorldDelta {
    SemanticWorldDelta {
        state_changes: vec![StateDelta {
            entity: case.event.target,
            channel: StateChannel {
                domain_code: 32,
                axis_code: 1,
            },
            change,
            confidence_bps: 10_000,
        }],
        ..SemanticWorldDelta::default()
    }
    .normalized()
}

fn plausible_delta_set(seed: u64, case: &TransitionCase) -> PlausibleDeltaSet {
    let empty = SemanticWorldDelta::default();
    if !applicable(case) {
        return PlausibleDeltaSet {
            branches: vec![FutureBranch {
                delta: empty,
                confidence_bps: 10_000,
                uncertainty: UncertaintyKind::None,
            }],
        };
    }
    let context = case
        .event
        .hidden_context_intervention
        .map(|value| {
            if value {
                BeliefTruth::KnownTrue
            } else {
                BeliefTruth::KnownFalse
            }
        })
        .unwrap_or(case.anchor.hidden_context_belief);
    let magnitude = case.event.magnitude.abs().max(1);
    match mechanism_class(seed, case) {
        Some(MechanismClass::DeterministicContextual) if case.event.observation_lag == 0 => {
            match context {
                BeliefTruth::KnownTrue => PlausibleDeltaSet {
                    branches: vec![FutureBranch {
                        delta: effect_delta(case, magnitude),
                        confidence_bps: 10_000,
                        uncertainty: UncertaintyKind::None,
                    }],
                },
                BeliefTruth::KnownFalse => PlausibleDeltaSet {
                    branches: vec![FutureBranch {
                        delta: empty,
                        confidence_bps: 10_000,
                        uncertainty: UncertaintyKind::None,
                    }],
                },
                _ => PlausibleDeltaSet {
                    branches: vec![
                        FutureBranch {
                            delta: empty,
                            confidence_bps: 5_000,
                            uncertainty: UncertaintyKind::Epistemic,
                        },
                        FutureBranch {
                            delta: effect_delta(case, magnitude),
                            confidence_bps: 5_000,
                            uncertainty: UncertaintyKind::Epistemic,
                        },
                    ],
                },
            }
        }
        Some(MechanismClass::Stochastic)
            if case.event.observation_lag == 0 && context != BeliefTruth::KnownFalse =>
        {
            PlausibleDeltaSet {
                branches: vec![
                    FutureBranch {
                        delta: effect_delta(case, -magnitude),
                        confidence_bps: 5_000,
                        uncertainty: UncertaintyKind::AleatoricOrWorldStochasticity,
                    },
                    FutureBranch {
                        delta: effect_delta(case, magnitude),
                        confidence_bps: 5_000,
                        uncertainty: UncertaintyKind::AleatoricOrWorldStochasticity,
                    },
                ],
            }
        }
        Some(MechanismClass::Delayed)
            if case.event.observation_lag == 1 && context != BeliefTruth::KnownFalse =>
        {
            PlausibleDeltaSet {
                branches: vec![FutureBranch {
                    delta: effect_delta(case, magnitude),
                    confidence_bps: 10_000,
                    uncertainty: UncertaintyKind::None,
                }],
            }
        }
        Some(_) => PlausibleDeltaSet {
            branches: vec![FutureBranch {
                delta: empty,
                confidence_bps: 10_000,
                uncertainty: UncertaintyKind::None,
            }],
        },
        None => PlausibleDeltaSet {
            branches: vec![FutureBranch {
                delta: empty,
                confidence_bps: 0,
                uncertainty: UncertaintyKind::InsufficientModelSupport,
            }],
        },
    }
    .normalized()
}

fn realized_delta(seed: u64, case: &TransitionCase) -> SemanticWorldDelta {
    let branches = plausible_delta_set(seed, case).branches;
    if branches.is_empty() {
        return SemanticWorldDelta::default();
    }
    let index = (mix(seed, case.case_id) % branches.len() as u64) as usize;
    branches[index].delta.clone().normalized()
}

fn apply_delta(mut anchor: BeliefAnchor, delta: &SemanticWorldDelta) -> BeliefAnchor {
    for change in &delta.state_changes {
        if let Some(entity) = anchor
            .entities
            .iter_mut()
            .find(|entity| entity.entity == change.entity)
        {
            entity.state_value = entity.state_value.saturating_add(change.change);
        }
    }
    anchor
}

pub fn solve_reachability(query: &ReachabilityQuery) -> ReachabilityResult {
    let mut queue = VecDeque::from([(query.anchor_node, Vec::<CausalPathStep>::new())]);
    let mut visited = BTreeSet::from([query.anchor_node]);
    while let Some((node, path)) = queue.pop_front() {
        if node == query.goal_node {
            let status = if path.len() <= query.action_budget as usize {
                ReachabilityStatus::ReachableWithinBudget
            } else {
                ReachabilityStatus::ReachableEventually
            };
            return ReachabilityResult {
                query_id: query.query_id,
                status,
                path_certificate: path,
            };
        }
        for (from, mechanism_code, to) in &query.edges {
            if *from == node && visited.insert(*to) {
                let mut next_path = path.clone();
                next_path.push(CausalPathStep {
                    from: *from,
                    mechanism_code: *mechanism_code,
                    to: *to,
                });
                queue.push_back((*to, next_path));
            }
        }
    }
    ReachabilityResult {
        query_id: query.query_id,
        status: if query.graph_complete {
            ReachabilityStatus::Unreachable
        } else {
            ReachabilityStatus::UnknownReachability
        },
        path_certificate: Vec::new(),
    }
}

fn percentile(values: &[u64], percent: usize) -> u64 {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted[((sorted.len() - 1) * percent / 100).min(sorted.len() - 1)]
}

fn mix(seed: u64, value: u64) -> u64 {
    let mut x = seed ^ value.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unreachable_similarity_does_not_become_a_shortcut() {
        let query = ReachabilityQuery {
            query_id: 1,
            anchor_node: 1,
            goal_node: 9,
            action_budget: 2,
            edges: vec![(1, 7, 2)],
            graph_complete: true,
            semantic_similarity_hint: 10_000,
        };
        assert_eq!(
            solve_reachability(&query).status,
            ReachabilityStatus::Unreachable
        );
    }

    #[test]
    fn delta_normalization_is_order_independent() {
        let channel = StateChannel {
            domain_code: 32,
            axis_code: 1,
        };
        let a = StateDelta {
            entity: 2,
            channel,
            change: 1,
            confidence_bps: 10_000,
        };
        let b = StateDelta {
            entity: 1,
            channel,
            change: 1,
            confidence_bps: 10_000,
        };
        assert_eq!(
            SemanticWorldDelta {
                state_changes: vec![a.clone(), b.clone()],
                ..Default::default()
            }
            .normalized(),
            SemanticWorldDelta {
                state_changes: vec![b, a],
                ..Default::default()
            }
            .normalized()
        );
    }
}
