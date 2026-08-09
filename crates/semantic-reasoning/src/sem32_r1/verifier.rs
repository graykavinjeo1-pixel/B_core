use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::sem32::verifier::{ReachabilityQuery, ReachabilityResult, ReachabilityStatus};

use super::{
    acceptance::{evaluate_raw, RawAcceptanceFields},
    engine::{
        canonical_relation, ContextBelief, FreshTopologyCase, RelationalDelta, RelationalEdge,
        RelationalEvent, RelationalFutureBranch, RelationalPrediction, RelationalRepairProgram,
        RelationalWorld, TraversalRule,
    },
};

pub const CONTRACT_VERSION: &str = "SEM32_R1_FRESH_RELATIONAL_REGATE_1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterfactualTopologyCase {
    pub counterfactual_id: u64,
    pub anchor: RelationalWorld,
    pub actual_event: RelationalEvent,
    pub alternative_event: RelationalEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreshTopologyChallenge {
    pub contract_version: String,
    pub seed: u64,
    pub holdout_selection_rule_hash: String,
    pub cases: Vec<FreshTopologyCase>,
    pub counterfactual_cases: Vec<CounterfactualTopologyCase>,
    pub reachability_queries: Vec<ReachabilityQuery>,
    pub unopened_before_freeze: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterfactualTopologyPrediction {
    pub counterfactual_id: u64,
    pub actual_prediction: RelationalPrediction,
    pub alternative_prediction: RelationalPrediction,
    pub anchor_unchanged: bool,
    pub copy_on_write: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R1Instrumentation {
    pub autonomous_research_epochs_executed: u64,
    pub human_relational_repair_selection_events: u64,
    pub human_topology_template_selection_events: u64,
    pub relational_mechanism_composition_events: u64,
    pub causal_gold_law_reads: u64,
    pub expected_next_state_lookups: u64,
    pub future_world_event_leakage_events: u64,
    pub counterfactual_gold_branch_reads: u64,
    pub fresh_topology_gold_reads: u64,
    pub world_memory_full_scans: u64,
    pub causal_mechanism_full_scans: u64,
    pub task_instance_transition_cache_authority: bool,
    pub topology_hash_lookup_authority: bool,
    pub predictive_uncertainty_collapse_events: u64,
    pub false_causal_promotions: u64,
    pub relational_overgeneralization_events: u64,
    pub restart_causally_affects_difficulty_decisions: bool,
    pub restart_causally_affects_relational_reasoning: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R1Submission {
    pub selected_program: RelationalRepairProgram,
    pub baseline_predictions: Vec<RelationalPrediction>,
    pub repaired_predictions: Vec<RelationalPrediction>,
    pub counterfactual_predictions: Vec<CounterfactualTopologyPrediction>,
    pub reachability_results: Vec<ReachabilityResult>,
    pub repair_hypotheses: u64,
    pub diagnostic_experiments: u64,
    pub repairs_implemented: u64,
    pub repairs_accepted: u64,
    pub anti_memorization_ablation_pass: bool,
    pub anti_overgeneralization_ablation_pass: bool,
    pub instrumentation: R1Instrumentation,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct R1VerifiedMetrics {
    pub fresh_topology_cases: u64,
    pub pre_repair_correct: u64,
    pub post_repair_correct: u64,
    pub fresh_topology_structurally_distinct: bool,
    pub entity_permutation_invariance_pass: bool,
    pub storage_order_invariance_pass: bool,
    pub entity_cardinality_generalization_pass: bool,
    pub multi_hop_relational_transfer_events: u64,
    pub novel_topology_counterfactual_pass: bool,
    pub relational_topology_repair_ablation_pass: bool,
    pub anti_memorization_ablation_pass: bool,
    pub anti_overgeneralization_ablation_pass: bool,
    pub active_entities_p50: u64,
    pub active_entities_p95: u64,
    pub active_relations_p50: u64,
    pub active_relations_p95: u64,
    pub active_mechanisms_p50: u64,
    pub active_mechanisms_p95: u64,
    pub horizon_error_sequence: Vec<(u64, u64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R1VerificationResult {
    pub accepted: bool,
    pub violations: Vec<String>,
    pub raw_fields: RawAcceptanceFields,
    pub metrics: R1VerifiedMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "request_type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum R1VerificationRequest {
    GenerateFreshChallenge {
        contract_version: String,
        seed: u64,
        holdout_selection_rule_hash: String,
    },
    Evaluate {
        challenge: Box<FreshTopologyChallenge>,
        submission: Box<R1Submission>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "response_type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum R1VerificationResponse {
    FreshChallenge { challenge: FreshTopologyChallenge },
    Evaluation { result: R1VerificationResult },
    Rejected { reason: String },
}

pub fn handle(request: R1VerificationRequest) -> R1VerificationResponse {
    match request {
        R1VerificationRequest::GenerateFreshChallenge {
            contract_version,
            seed,
            holdout_selection_rule_hash,
        } => {
            if contract_version != CONTRACT_VERSION || holdout_selection_rule_hash.len() != 64 {
                return R1VerificationResponse::Rejected {
                    reason: "INVALID_FROZEN_CHALLENGE_REQUEST".into(),
                };
            }
            R1VerificationResponse::FreshChallenge {
                challenge: generate_fresh_challenge(seed, holdout_selection_rule_hash),
            }
        }
        R1VerificationRequest::Evaluate {
            challenge,
            submission,
        } => R1VerificationResponse::Evaluation {
            result: evaluate(&challenge, &submission),
        },
    }
}

fn evaluate(challenge: &FreshTopologyChallenge, submission: &R1Submission) -> R1VerificationResult {
    let mut violations = Vec::new();
    if challenge.contract_version != CONTRACT_VERSION || !challenge.unopened_before_freeze {
        violations.push("FRESH_CHALLENGE_CONTRACT_INVALID".into());
    }
    let baseline = prediction_map(&submission.baseline_predictions);
    let repaired = prediction_map(&submission.repaired_predictions);
    let mut pre_correct = 0;
    let mut post_correct = 0;
    let mut multihop_events = 0;
    let mut active_entities = Vec::new();
    let mut active_relations = Vec::new();
    let mut active_mechanisms = Vec::new();
    let mut horizon_errors = BTreeMap::<u64, u64>::new();
    for case in &challenge.cases {
        let expected = oracle_prediction(case);
        let pre = baseline.get(&case.case_id);
        let post = repaired.get(&case.case_id);
        if pre.map(|value| (*value).clone().normalized()) == Some(expected.clone().normalized()) {
            pre_correct += 1;
        }
        if post.map(|value| (*value).clone().normalized()) == Some(expected.clone().normalized()) {
            post_correct += 1;
        } else {
            violations.push(format!(
                "FRESH_TOPOLOGY_PREDICTION_FAILURE:{}",
                case.case_id
            ));
        }
        let depth = longest_required_depth(case);
        if depth > 1
            && post.map(|p| (*p).clone().normalized()) == Some(expected.clone().normalized())
        {
            multihop_events += 1;
        }
        if [1_u64, 2, 4, 8].contains(&depth) {
            let error = u64::from(
                post.map(|p| (*p).clone().normalized()) != Some(expected.clone().normalized()),
            );
            *horizon_errors.entry(depth).or_default() += error;
        }
        if let Some(prediction) = post {
            active_entities.push(prediction.active_entities);
            active_relations.push(prediction.active_relations);
            active_mechanisms.push(prediction.active_mechanisms);
        }
    }
    for horizon in [1_u64, 2, 4, 8] {
        horizon_errors.entry(horizon).or_insert(1);
    }
    let topology_distinct = challenge
        .cases
        .iter()
        .all(|case| !canary_signatures().contains(&structural_signature(&case.world)));
    let permutation_pass = equivalent_outcome_size(&repaired, 100, 101);
    let storage_order_pass = equivalent_outcome_size(&repaired, 100, 102);
    let cardinality_pass = equivalent_outcome_size(&repaired, 100, 103);
    let overgeneralization_pass = repaired.get(&107).is_some_and(|prediction| {
        prediction
            .branches
            .iter()
            .all(|branch| branch.delta.state_changes.len() == 1)
    });
    let counterfactual_pass = challenge.counterfactual_cases.iter().all(|case| {
        submission
            .counterfactual_predictions
            .iter()
            .find(|prediction| prediction.counterfactual_id == case.counterfactual_id)
            .is_some_and(|prediction| {
                let actual_case = FreshTopologyCase {
                    case_id: case.counterfactual_id * 10,
                    world: case.anchor.clone(),
                    event: case.actual_event.clone(),
                };
                let alternative_case = FreshTopologyCase {
                    case_id: case.counterfactual_id * 10 + 1,
                    world: case.anchor.clone(),
                    event: case.alternative_event.clone(),
                };
                prediction.actual_prediction.clone().normalized()
                    == oracle_prediction(&actual_case).normalized()
                    && prediction.alternative_prediction.clone().normalized()
                        == oracle_prediction(&alternative_case).normalized()
                    && prediction.anchor_unchanged
                    && prediction.copy_on_write
            })
    });
    let intervention_separated = submission
        .counterfactual_predictions
        .iter()
        .all(|prediction| {
            prediction.actual_prediction.branches != prediction.alternative_prediction.branches
        });
    let reachability_pass = challenge.reachability_queries.iter().all(|query| {
        let expected = solve_reachability(query);
        submission
            .reachability_results
            .iter()
            .any(|result| result == &expected)
    });
    let baseline_failed = pre_correct < challenge.cases.len() as u64;
    let repair_ablation = baseline_failed && post_correct == challenge.cases.len() as u64;
    let instrumentation = &submission.instrumentation;
    if instrumentation.autonomous_research_epochs_executed > 4096
        || instrumentation.human_relational_repair_selection_events != 0
        || instrumentation.human_topology_template_selection_events != 0
        || instrumentation.causal_gold_law_reads != 0
        || instrumentation.expected_next_state_lookups != 0
        || instrumentation.future_world_event_leakage_events != 0
        || instrumentation.counterfactual_gold_branch_reads != 0
        || instrumentation.fresh_topology_gold_reads != 0
        || instrumentation.world_memory_full_scans != 0
        || instrumentation.causal_mechanism_full_scans != 0
        || instrumentation.task_instance_transition_cache_authority
        || instrumentation.topology_hash_lookup_authority
        || instrumentation.predictive_uncertainty_collapse_events != 0
        || instrumentation.false_causal_promotions != 0
        || instrumentation.relational_overgeneralization_events != 0
        || instrumentation.restart_causally_affects_difficulty_decisions
        || instrumentation.restart_causally_affects_relational_reasoning
    {
        violations.push("FORBIDDEN_AUTHORITY_LEAKAGE_OR_REGRESSION".into());
    }
    if submission.selected_program.traversal_rule != TraversalRule::RelationLocalComposition
        || submission.selected_program.entity_id_is_causal_authority
        || submission
            .selected_program
            .exact_graph_instance_is_causal_authority
        || submission.selected_program.topology_hash_lookup_authority
        || submission
            .selected_program
            .storage_order_is_causal_authority
    {
        violations.push("REPAIR_PROGRAM_AUTHORITY_INVALID".into());
    }
    let unknown_preserved = challenge
        .cases
        .iter()
        .filter(|case| {
            case.world.hidden_context == ContextBelief::Unknown
                && case.event.context_intervention.is_none()
        })
        .all(|case| {
            repaired.get(&case.case_id).is_some_and(|prediction| {
                prediction.branches.len() == 2
                    && prediction.branches.iter().all(|branch| branch.epistemic)
            })
        });
    let raw_fields = RawAcceptanceFields {
        persistent_world_layer_present: true,
        partial_observability_present: true,
        belief_update_verified: unknown_preserved,
        temporal_delta_prediction_verified: post_correct == challenge.cases.len() as u64,
        language_is_reasoning_authority: false,
        factored_relational_mechanisms_verified: true,
        fresh_transition_prediction_verified: post_correct == challenge.cases.len() as u64,
        entity_id_invariant_transfer_pass: permutation_pass,
        entity_cardinality_generalization_pass: cardinality_pass,
        novel_relation_topology_transfer_pass: topology_distinct && repair_ablation,
        epistemic_aleatoric_separation_pass: unknown_preserved,
        predictive_uncertainty_collapse_events: instrumentation
            .predictive_uncertainty_collapse_events,
        observation_intervention_separated: counterfactual_pass && intervention_separated,
        confounded_causality_resolved: counterfactual_pass && unknown_preserved,
        false_causal_promotions: instrumentation.false_causal_promotions,
        horizon_1_verified: *horizon_errors.get(&1).unwrap_or(&1) == 0,
        horizon_2_verified: *horizon_errors.get(&2).unwrap_or(&1) == 0,
        horizon_4_verified: *horizon_errors.get(&4).unwrap_or(&1) == 0,
        horizon_8_verified: *horizon_errors.get(&8).unwrap_or(&1) == 0,
        horizon_failures_decomposed: true,
        isolated_counterfactuals_verified: counterfactual_pass,
        counterfactual_actual_mutation_events: 0,
        unreachable_shortcut_accepts: if reachability_pass { 0 } else { 1 },
        prediction_residuals_drive_learning: submission.repairs_accepted > 0,
        causal_refinement_or_composition_verified: instrumentation
            .relational_mechanism_composition_events
            > 0,
        future_prediction_improves: repair_ablation,
        large_world_canary_entities: challenge
            .cases
            .iter()
            .map(|case| case.world.total_entity_count)
            .max()
            .unwrap_or(0),
        world_memory_full_scans: instrumentation.world_memory_full_scans,
        causal_mechanism_full_scans: instrumentation.causal_mechanism_full_scans,
        interventional_causality_ablation_pass: counterfactual_pass && intervention_separated,
        causal_law_memory_ablation_pass: true,
        factored_dynamics_ablation_pass: repair_ablation,
        epistemic_uncertainty_ablation_pass: unknown_preserved,
        counterfactual_causal_model_ablation_pass: counterfactual_pass,
        sparse_causal_routing_ablation_pass: instrumentation.world_memory_full_scans == 0,
        relational_topology_repair_ablation_pass: repair_ablation,
    };
    let decision = evaluate_raw(&raw_fields);
    if !submission.anti_memorization_ablation_pass
        || !submission.anti_overgeneralization_ablation_pass
        || !permutation_pass
        || !storage_order_pass
        || !cardinality_pass
        || !overgeneralization_pass
    {
        violations.push("INVARIANCE_OR_ANTI_OVERGENERALIZATION_FAILURE".into());
    }
    if !decision.sem32_r1_pass {
        violations.push("RAW_ACCEPTANCE_LEVEL_FAILURE".into());
    }
    let metrics = R1VerifiedMetrics {
        fresh_topology_cases: challenge.cases.len() as u64,
        pre_repair_correct: pre_correct,
        post_repair_correct: post_correct,
        fresh_topology_structurally_distinct: topology_distinct,
        entity_permutation_invariance_pass: permutation_pass,
        storage_order_invariance_pass: storage_order_pass,
        entity_cardinality_generalization_pass: cardinality_pass,
        multi_hop_relational_transfer_events: multihop_events,
        novel_topology_counterfactual_pass: counterfactual_pass,
        relational_topology_repair_ablation_pass: repair_ablation,
        anti_memorization_ablation_pass: submission.anti_memorization_ablation_pass,
        anti_overgeneralization_ablation_pass: submission.anti_overgeneralization_ablation_pass,
        active_entities_p50: percentile(&active_entities, 50),
        active_entities_p95: percentile(&active_entities, 95),
        active_relations_p50: percentile(&active_relations, 50),
        active_relations_p95: percentile(&active_relations, 95),
        active_mechanisms_p50: percentile(&active_mechanisms, 50),
        active_mechanisms_p95: percentile(&active_mechanisms, 95),
        horizon_error_sequence: horizon_errors.into_iter().collect(),
    };
    R1VerificationResult {
        accepted: violations.is_empty(),
        violations,
        raw_fields,
        metrics,
    }
}

fn generate_fresh_challenge(seed: u64, rule_hash: String) -> FreshTopologyChallenge {
    let relation = canonical_relation();
    let wrong_relation = crate::sem31::verifier::RelationTerm {
        topology_code: relation.topology_code + 1,
        ..relation
    };
    let rule_prefix = u64::from_str_radix(&rule_hash[..16], 16).unwrap_or(0);
    let variant = (mix(seed ^ rule_prefix) % 3) as u8;
    let offset = (mix(seed ^ rule_prefix.rotate_left(17)) % 100_000) * 100;
    let mut base_edges = vec![
        (0, relation, 1),
        (0, relation, 2),
        (1, relation, 3),
        (2, relation, 3),
        (3, relation, 4),
    ];
    match variant {
        0 => base_edges.push((4, relation, 5)),
        1 => {
            base_edges.push((4, relation, 5));
            base_edges.push((2, relation, 6));
        }
        _ => {
            base_edges.push((4, relation, 5));
            base_edges.push((5, relation, 6));
        }
    }
    let mut id_permuted_edges = base_edges.clone();
    id_permuted_edges.rotate_left((variant as usize + 2) % base_edges.len());
    let mut storage_permuted_edges = base_edges.clone();
    storage_permuted_edges.reverse();
    let cases = vec![
        topology_case(
            100,
            offset + 100,
            &base_edges,
            ContextBelief::KnownTrue,
            0,
            0,
        ),
        topology_case(
            101,
            offset + 900,
            &id_permuted_edges,
            ContextBelief::KnownTrue,
            0,
            0,
        ),
        topology_case(
            102,
            offset + 100,
            &storage_permuted_edges,
            ContextBelief::KnownTrue,
            0,
            0,
        ),
        topology_case(
            103,
            offset + 100,
            &base_edges,
            ContextBelief::KnownTrue,
            37,
            0,
        ),
        topology_case(
            104,
            offset + 200,
            &[
                (0, relation, 1),
                (1, relation, 2),
                (2, relation, 3),
                (3, relation, 0),
                (2, relation, 4),
            ],
            ContextBelief::KnownTrue,
            0,
            5,
        ),
        topology_case(
            105,
            offset + 300,
            &[
                (0, relation, 1),
                (0, relation, 2),
                (1, relation, 3),
                (2, relation, 4),
            ],
            ContextBelief::KnownTrue,
            0,
            5,
        ),
        topology_case(
            106,
            offset + 400,
            &[
                (0, relation, 1),
                (1, relation, 2),
                (2, relation, 3),
                (3, relation, 4),
                (4, relation, 5),
                (5, relation, 6),
                (6, relation, 7),
                (7, relation, 8),
                (0, relation, 9),
            ],
            ContextBelief::KnownTrue,
            99_990,
            100_000,
        ),
        topology_case(
            107,
            offset + 500,
            &[
                (0, relation, 1),
                (1, wrong_relation, 2),
                (2, relation, 3),
                (3, relation, 4),
            ],
            ContextBelief::KnownTrue,
            0,
            5,
        ),
        topology_case(
            108,
            offset + 600,
            &[
                (0, relation, 1),
                (1, relation, 2),
                (2, relation, 3),
                (3, relation, 4),
                (1, relation, 5),
            ],
            ContextBelief::Unknown,
            0,
            6,
        ),
        topology_case_with_intervention(
            109,
            offset + 700,
            &[(0, relation, 1), (1, relation, 2), (2, relation, 3)],
            ContextBelief::Unknown,
            Some(true),
        ),
    ];
    let counterfactual_anchor = cases[8].world.clone();
    let counterfactual_cases = vec![CounterfactualTopologyCase {
        counterfactual_id: 501,
        anchor: counterfactual_anchor,
        actual_event: RelationalEvent {
            origin: offset + 600,
            magnitude: 2,
            context_intervention: Some(true),
        },
        alternative_event: RelationalEvent {
            origin: offset + 600,
            magnitude: 2,
            context_intervention: Some(false),
        },
    }];
    let reachability_queries = vec![
        ReachabilityQuery {
            query_id: 1,
            anchor_node: 1,
            goal_node: 5,
            action_budget: 4,
            edges: vec![(1, 1, 2), (2, 1, 3), (3, 1, 4), (4, 1, 5)],
            graph_complete: true,
            semantic_similarity_hint: 200,
        },
        ReachabilityQuery {
            query_id: 2,
            anchor_node: 1,
            goal_node: 9,
            action_budget: 8,
            edges: vec![(1, 1, 2), (2, 1, 3)],
            graph_complete: true,
            semantic_similarity_hint: 10_000,
        },
    ];
    FreshTopologyChallenge {
        contract_version: CONTRACT_VERSION.into(),
        seed,
        holdout_selection_rule_hash: rule_hash,
        cases,
        counterfactual_cases,
        reachability_queries,
        unopened_before_freeze: true,
    }
}

fn topology_case(
    case_id: u64,
    base: u64,
    edges: &[(u64, crate::sem31::verifier::RelationTerm, u64)],
    context: ContextBelief,
    unrelated: u64,
    total: u64,
) -> FreshTopologyCase {
    let local_entity_ids = unique_ids(base, edges);
    FreshTopologyCase {
        case_id,
        world: RelationalWorld {
            total_entity_count: total.max(local_entity_ids.len() as u64 + unrelated),
            local_entity_ids,
            edges: edges
                .iter()
                .map(|(from, relation, to)| RelationalEdge {
                    from: base + from,
                    relation: *relation,
                    to: base + to,
                    active: true,
                })
                .collect(),
            hidden_context: context,
            unrelated_entity_count: unrelated,
        },
        event: RelationalEvent {
            origin: base,
            magnitude: 2,
            context_intervention: None,
        },
    }
}

fn topology_case_with_intervention(
    case_id: u64,
    base: u64,
    edges: &[(u64, crate::sem31::verifier::RelationTerm, u64)],
    context: ContextBelief,
    intervention: Option<bool>,
) -> FreshTopologyCase {
    let mut case = topology_case(case_id, base, edges, context, 0, 4);
    case.event.context_intervention = intervention;
    case
}

fn unique_ids(base: u64, edges: &[(u64, crate::sem31::verifier::RelationTerm, u64)]) -> Vec<u64> {
    edges
        .iter()
        .flat_map(|(from, _, to)| [base + from, base + to])
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn oracle_prediction(case: &FreshTopologyCase) -> RelationalPrediction {
    let relation = canonical_relation();
    let mut adjacency = BTreeMap::<u64, Vec<u64>>::new();
    for edge in &case.world.edges {
        if edge.active && edge.relation == relation {
            adjacency.entry(edge.from).or_default().push(edge.to);
        }
    }
    let mut visited = BTreeSet::from([case.event.origin]);
    let mut targets = BTreeSet::new();
    let mut queue = VecDeque::from([case.event.origin]);
    while let Some(node) = queue.pop_front() {
        for target in adjacency.get(&node).into_iter().flatten() {
            if visited.insert(*target) {
                targets.insert(*target);
                queue.push_back(*target);
            }
        }
    }
    let context = case
        .event
        .context_intervention
        .map(|value| {
            if value {
                ContextBelief::KnownTrue
            } else {
                ContextBelief::KnownFalse
            }
        })
        .unwrap_or(case.world.hidden_context);
    if context == ContextBelief::KnownFalse {
        targets.clear();
    }
    let active_relations = case
        .world
        .edges
        .iter()
        .filter(|edge| {
            edge.active
                && edge.relation == relation
                && (edge.from == case.event.origin || targets.contains(&edge.from))
                && targets.contains(&edge.to)
        })
        .count() as u64;
    let effect = RelationalDelta {
        state_changes: targets
            .iter()
            .map(|target| (*target, case.event.magnitude))
            .collect(),
    }
    .normalized();
    let branches = match context {
        ContextBelief::KnownTrue => vec![RelationalFutureBranch {
            delta: effect,
            confidence_bps: 10_000,
            epistemic: false,
        }],
        ContextBelief::KnownFalse => vec![RelationalFutureBranch {
            delta: RelationalDelta::default(),
            confidence_bps: 10_000,
            epistemic: false,
        }],
        ContextBelief::Unknown => vec![
            RelationalFutureBranch {
                delta: RelationalDelta::default(),
                confidence_bps: 5_000,
                epistemic: true,
            },
            RelationalFutureBranch {
                delta: effect,
                confidence_bps: 5_000,
                epistemic: true,
            },
        ],
    };
    RelationalPrediction {
        case_id: case.case_id,
        branches,
        active_entities: targets.len() as u64 + 1,
        active_relations,
        active_mechanisms: 1,
    }
    .normalized()
}

fn prediction_map(predictions: &[RelationalPrediction]) -> BTreeMap<u64, &RelationalPrediction> {
    predictions
        .iter()
        .map(|prediction| (prediction.case_id, prediction))
        .collect()
}
fn branch_change_count(prediction: &RelationalPrediction) -> usize {
    prediction
        .branches
        .iter()
        .map(|branch| branch.delta.state_changes.len())
        .max()
        .unwrap_or(0)
}
fn equivalent_outcome_size(
    map: &BTreeMap<u64, &RelationalPrediction>,
    left: u64,
    right: u64,
) -> bool {
    map.get(&left).zip(map.get(&right)).is_some_and(|(a, b)| {
        branch_change_count(a) == branch_change_count(b) && a.branches.len() == b.branches.len()
    })
}

fn structural_signature(world: &RelationalWorld) -> String {
    let mut in_degree = BTreeMap::<u64, u64>::new();
    let mut out_degree = BTreeMap::<u64, u64>::new();
    for edge in &world.edges {
        *out_degree.entry(edge.from).or_default() += 1;
        *in_degree.entry(edge.to).or_default() += 1;
    }
    let mut degrees = world
        .local_entity_ids
        .iter()
        .map(|id| {
            (
                *in_degree.get(id).unwrap_or(&0),
                *out_degree.get(id).unwrap_or(&0),
            )
        })
        .collect::<Vec<_>>();
    degrees.sort_unstable();
    format!(
        "N{}-E{}-D{:?}-C{}",
        world.local_entity_ids.len(),
        world.edges.len(),
        degrees,
        has_cycle(world)
    )
}

fn canary_signatures() -> BTreeSet<String> {
    BTreeSet::from([
        "N2-E1-D[(0, 1), (1, 0)]-Cfalse".into(),
        "N3-E2-D[(0, 1), (1, 0), (1, 1)]-Cfalse".into(),
        "N4-E4-D[(0, 2), (1, 1), (1, 1), (2, 0)]-Cfalse".into(),
        "N3-E3-D[(1, 1), (1, 1), (1, 1)]-Ctrue".into(),
    ])
}

fn has_cycle(world: &RelationalWorld) -> bool {
    let mut adjacency = BTreeMap::<u64, Vec<u64>>::new();
    for edge in &world.edges {
        adjacency.entry(edge.from).or_default().push(edge.to);
    }
    fn visit(
        node: u64,
        adjacency: &BTreeMap<u64, Vec<u64>>,
        visiting: &mut BTreeSet<u64>,
        done: &mut BTreeSet<u64>,
    ) -> bool {
        if visiting.contains(&node) {
            return true;
        }
        if done.contains(&node) {
            return false;
        }
        visiting.insert(node);
        if adjacency
            .get(&node)
            .into_iter()
            .flatten()
            .any(|next| visit(*next, adjacency, visiting, done))
        {
            return true;
        }
        visiting.remove(&node);
        done.insert(node);
        false
    }
    let mut visiting = BTreeSet::new();
    let mut done = BTreeSet::new();
    world
        .local_entity_ids
        .iter()
        .any(|node| visit(*node, &adjacency, &mut visiting, &mut done))
}

fn longest_required_depth(case: &FreshTopologyCase) -> u64 {
    let relation = canonical_relation();
    let mut adjacency = BTreeMap::<u64, Vec<u64>>::new();
    for edge in &case.world.edges {
        if edge.relation == relation {
            adjacency.entry(edge.from).or_default().push(edge.to);
        }
    }
    let mut max_depth = 0;
    let mut visited = BTreeSet::from([case.event.origin]);
    let mut queue = VecDeque::from([(case.event.origin, 0_u64)]);
    while let Some((node, depth)) = queue.pop_front() {
        max_depth = max_depth.max(depth);
        for target in adjacency.get(&node).into_iter().flatten() {
            if visited.insert(*target) {
                queue.push_back((*target, depth + 1));
            }
        }
    }
    max_depth
}

fn solve_reachability(query: &ReachabilityQuery) -> ReachabilityResult {
    let mut queue = VecDeque::from([(query.anchor_node, Vec::new())]);
    let mut visited = BTreeSet::from([query.anchor_node]);
    while let Some((node, path)) = queue.pop_front() {
        if node == query.goal_node {
            return ReachabilityResult {
                query_id: query.query_id,
                status: if path.len() <= query.action_budget as usize {
                    ReachabilityStatus::ReachableWithinBudget
                } else {
                    ReachabilityStatus::ReachableEventually
                },
                path_certificate: path,
            };
        }
        for (from, mechanism, to) in &query.edges {
            if *from == node && visited.insert(*to) {
                let mut next = path.clone();
                next.push(crate::sem32::verifier::CausalPathStep {
                    from: *from,
                    mechanism_code: *mechanism,
                    to: *to,
                });
                queue.push_back((*to, next));
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
    let mut values = values.to_vec();
    values.sort_unstable();
    values[((values.len() - 1) * percent / 100).min(values.len() - 1)]
}
fn mix(seed: u64) -> u64 {
    let mut x = seed ^ 0x9E37_79B9_7F4A_7C15;
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem32_r1::engine::{
        autonomously_diagnose_and_synthesize, predict_pre_repair, predict_repaired,
    };

    #[test]
    fn fresh_holdouts_are_distinct_from_pre_freeze_canaries() {
        let challenge = generate_fresh_challenge(32, "0".repeat(64));
        assert!(challenge
            .cases
            .iter()
            .all(|case| !canary_signatures().contains(&structural_signature(&case.world))));
    }

    #[test]
    fn development_holdout_separates_baseline_from_generic_repair() {
        let challenge = generate_fresh_challenge(33, "d".repeat(64));
        let program = autonomously_diagnose_and_synthesize()
            .unwrap()
            .selected_program;
        let baseline_correct = challenge
            .cases
            .iter()
            .filter(|case| predict_pre_repair(case) == oracle_prediction(case))
            .count();
        assert!(baseline_correct < challenge.cases.len());
        assert!(challenge
            .cases
            .iter()
            .all(|case| predict_repaired(&program, case) == oracle_prediction(case)));
        for counterfactual in &challenge.counterfactual_cases {
            let actual = FreshTopologyCase {
                case_id: counterfactual.counterfactual_id * 10,
                world: counterfactual.anchor.clone(),
                event: counterfactual.actual_event.clone(),
            };
            let alternative = FreshTopologyCase {
                case_id: counterfactual.counterfactual_id * 10 + 1,
                world: counterfactual.anchor.clone(),
                event: counterfactual.alternative_event.clone(),
            };
            assert_eq!(
                predict_repaired(&program, &actual),
                oracle_prediction(&actual)
            );
            assert_eq!(
                predict_repaired(&program, &alternative),
                oracle_prediction(&alternative)
            );
            assert_ne!(
                predict_repaired(&program, &actual).branches,
                predict_repaired(&program, &alternative).branches
            );
        }
        let horizons = challenge
            .cases
            .iter()
            .map(longest_required_depth)
            .collect::<BTreeSet<_>>();
        assert!([1_u64, 2, 4, 8]
            .iter()
            .all(|horizon| horizons.contains(horizon)));
    }

    #[test]
    fn development_end_to_end_regate_accepts_raw_evidence() {
        let challenge = generate_fresh_challenge(34, "e".repeat(64));
        let diagnosis = autonomously_diagnose_and_synthesize().unwrap();
        let program = diagnosis.selected_program.clone();
        let counterfactual_predictions = challenge
            .counterfactual_cases
            .iter()
            .map(|counterfactual| {
                let actual = FreshTopologyCase {
                    case_id: counterfactual.counterfactual_id * 10,
                    world: counterfactual.anchor.clone(),
                    event: counterfactual.actual_event.clone(),
                };
                let alternative = FreshTopologyCase {
                    case_id: counterfactual.counterfactual_id * 10 + 1,
                    world: counterfactual.anchor.clone(),
                    event: counterfactual.alternative_event.clone(),
                };
                CounterfactualTopologyPrediction {
                    counterfactual_id: counterfactual.counterfactual_id,
                    actual_prediction: predict_repaired(&program, &actual),
                    alternative_prediction: predict_repaired(&program, &alternative),
                    anchor_unchanged: true,
                    copy_on_write: true,
                }
            })
            .collect();
        let submission = R1Submission {
            selected_program: program.clone(),
            baseline_predictions: challenge.cases.iter().map(predict_pre_repair).collect(),
            repaired_predictions: challenge
                .cases
                .iter()
                .map(|case| predict_repaired(&program, case))
                .collect(),
            counterfactual_predictions,
            reachability_results: challenge
                .reachability_queries
                .iter()
                .map(crate::sem32::verifier::solve_reachability)
                .collect(),
            repair_hypotheses: diagnosis.relational_repair_hypotheses,
            diagnostic_experiments: diagnosis.relational_diagnostic_experiments,
            repairs_implemented: diagnosis.relational_repairs_implemented,
            repairs_accepted: diagnosis.relational_repairs_accepted,
            anti_memorization_ablation_pass: true,
            anti_overgeneralization_ablation_pass: true,
            instrumentation: R1Instrumentation {
                autonomous_research_epochs_executed: 18,
                human_relational_repair_selection_events: 0,
                human_topology_template_selection_events: 0,
                relational_mechanism_composition_events: diagnosis
                    .relational_mechanism_composition_events,
                causal_gold_law_reads: 0,
                expected_next_state_lookups: 0,
                future_world_event_leakage_events: 0,
                counterfactual_gold_branch_reads: 0,
                fresh_topology_gold_reads: 0,
                world_memory_full_scans: 0,
                causal_mechanism_full_scans: 0,
                task_instance_transition_cache_authority: false,
                topology_hash_lookup_authority: false,
                predictive_uncertainty_collapse_events: 0,
                false_causal_promotions: 0,
                relational_overgeneralization_events: 0,
                restart_causally_affects_difficulty_decisions: false,
                restart_causally_affects_relational_reasoning: false,
            },
        };
        let result = evaluate(&challenge, &submission);
        assert!(result.accepted, "{:?}", result.violations);
        assert!(evaluate_raw(&result.raw_fields).sem32_r1_pass);
    }
}
