use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{
    acceptance::{evaluate_raw, RawPlanningFields},
    engine::{
        BeliefStatus, DesiredWorldPhenotype, Fact, PlannerMode, PlannerProgram, PlannerRuntime,
        PlanningDecision, PublicPlanningTask, ReachabilityClass, SemanticAction,
    },
    transport::NestedCanary,
};

pub const CONTRACT_VERSION: &str = "SEM33_R1_BLIND_PLANNING_VERIFIER_1";
pub const HISTORICAL_SEM33_SEED: u64 = 6_787_946_092_034_902_772;
pub const HISTORICAL_SEM33_RULE_HASH: &str =
    "a83fa901e21ee2b6d8c21c5ea63d62b3a224cb18c41da87412a90245c08a1241";

#[derive(Debug, Clone, Serialize)]
struct HiddenPlanningTask {
    public: PublicPlanningTask,
    initial_truth: BTreeSet<Fact>,
    hidden_failure_once: Option<u64>,
    stochastic_failures: BTreeSet<u64>,
    expected_rejection: Option<ReachabilityClass>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlanningResult {
    pub task_id: u64,
    pub task_pass: bool,
    pub goal_satisfied: bool,
    pub constraints_preserved: bool,
    pub declared_reachability: ReachabilityClass,
    pub plan_length: u64,
    pub subgoal_count: u64,
    pub max_subgoal_depth: u64,
    pub causal_path_depth: u64,
    pub actions_executed: Vec<u64>,
    pub information_actions: u64,
    pub replan_events: u64,
    pub replans_caused_by_residual: u64,
    pub model_residuals: u64,
    pub stochastic_branch_events: u64,
    pub unsupported_confident_executions: u64,
    pub known_dead_end_entries: u64,
    pub unreachable_plan_accepts: u64,
    pub semantic_near_unreachable_shortcut_accepts: u64,
    pub resource_used: u64,
    pub time_used: u64,
    pub planning_cost_units: u64,
    pub causal_path_certificate_present: bool,
    pub decisions: Vec<PlanningDecision>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArmMetrics {
    pub goal_tasks_total: u64,
    pub goal_tasks_solved: u64,
    pub long_horizon_tasks: u64,
    pub long_horizon_tasks_solved: u64,
    pub reachability_queries: u64,
    pub unreachable_plan_cases: u64,
    pub unreachable_plan_accepts: u64,
    pub semantic_near_unreachable_shortcut_accepts: u64,
    pub autonomous_subgoals_created: u64,
    pub hierarchical_plan_events: u64,
    pub max_subgoal_depth: u64,
    pub information_gathering_actions: u64,
    pub unsupported_plan_confident_executions: u64,
    pub stochastic_plan_branch_events: u64,
    pub plan_execution_actions: u64,
    pub replan_events: u64,
    pub replan_caused_by_model_residual: u64,
    pub goals_satisfied_after_replan: u64,
    pub known_dead_end_entries: u64,
    pub planning_overgeneralization_events: u64,
    pub full_action_tree_enumeration_events: u64,
    pub world_memory_full_scans: u64,
    pub causal_mechanism_full_scans: u64,
    pub causal_path_certificates: u64,
    pub active_entities_p50: u64,
    pub active_entities_p95: u64,
    pub active_relations_p50: u64,
    pub active_relations_p95: u64,
    pub active_mechanisms_p50: u64,
    pub active_mechanisms_p95: u64,
    pub raw_action_branching_factor_sequence: Vec<u64>,
    pub semantically_routed_candidates_sequence: Vec<u64>,
    pub actually_rolled_out_candidates_sequence: Vec<u64>,
    pub plan_length_sequence: Vec<u64>,
    pub subgoal_count_sequence: Vec<u64>,
    pub subgoal_depth_sequence: Vec<u64>,
    pub causal_path_depth_sequence: Vec<u64>,
    pub active_entity_sequence: Vec<u64>,
    pub active_relation_sequence: Vec<u64>,
    pub active_mechanism_sequence: Vec<u64>,
    pub planning_cost_sequence: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArmEvidence {
    pub contract_version: String,
    pub challenge_hash: String,
    pub mode: PlannerMode,
    pub public_task_manifest: Vec<PublicPlanningTask>,
    pub task_results: Vec<TaskPlanningResult>,
    pub metrics: ArmMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignInstrumentation {
    pub requested_max_autonomous_research_epochs: u64,
    pub configured_max_autonomous_research_epochs: u64,
    pub autonomous_research_epochs_executed: u64,
    pub human_planner_architecture_selection_events: u64,
    pub human_subgoal_selection_events: u64,
    pub human_plan_selection_events: u64,
    pub human_planning_repair_events: u64,
    pub goal_specific_policy_training_events: u64,
    pub task_specific_planner_branches: u64,
    pub gold_action_reads: u64,
    pub gold_plan_reads: u64,
    pub expected_goal_state_lookups: u64,
    pub future_world_event_leakage_events: u64,
    pub whole_planner_architecture_transplants: u64,
    pub external_llm_calls: u64,
    pub local_teacher_calls: u64,
    pub canonical_network_reads: u64,
    pub canonical_network_writes: u64,
    pub remote_executions: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignBundle {
    pub baseline: ArmEvidence,
    pub full: ArmEvidence,
    pub flat: ArmEvidence,
    pub no_reachability: ArmEvidence,
    pub no_causal_model: ArmEvidence,
    pub no_uncertainty: ArmEvidence,
    pub open_loop: ArmEvidence,
    pub global_routing: ArmEvidence,
    pub instrumentation: CampaignInstrumentation,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VerifiedAblations {
    pub reachability_planning_ablation_pass: bool,
    pub hierarchical_planning_ablation_pass: bool,
    pub causal_model_planning_ablation_pass: bool,
    pub uncertainty_planning_ablation_pass: bool,
    pub closed_loop_replanning_ablation_pass: bool,
    pub sparse_planning_ablation_pass: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sem33VerificationResult {
    pub accepted: bool,
    pub violations: Vec<String>,
    pub raw_fields: RawPlanningFields,
    pub ablations: VerifiedAblations,
    pub fresh_topology_structurally_distinct: bool,
    pub novel_relation_topology_planning_pass: bool,
    pub entity_cardinality_planning_generalization_pass: bool,
    pub novel_goal_composition_pass: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoldoutManifest {
    pub contract_version: String,
    pub set_id: String,
    pub seed: u64,
    pub holdout_selection_rule_hash: String,
    pub task_generator_version: String,
    pub challenge_hash: String,
    pub hidden_instance_commitment_hash: String,
    pub instance_commitments: Vec<String>,
    pub task_count: u64,
    pub world_semantics_hash: String,
    pub action_semantics_hash: String,
    pub goal_semantics_hash: String,
    pub historical_holdout_instance_overlap: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "request_type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Sem33VerificationRequest {
    TransportProbe {
        contract_version: String,
        payload: NestedCanary,
    },
    FreezeManifest {
        contract_version: String,
        set_id: String,
        seed: u64,
        holdout_selection_rule_hash: String,
    },
    RunArm {
        contract_version: String,
        seed: u64,
        holdout_selection_rule_hash: String,
        program: PlannerProgram,
    },
    EvaluateBundle {
        contract_version: String,
        seed: u64,
        holdout_selection_rule_hash: String,
        bundle: Box<CampaignBundle>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "response_type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Sem33VerificationResponse {
    TransportProbed {
        payload: NestedCanary,
        semantic_hash: String,
    },
    ManifestFrozen {
        manifest: Box<HoldoutManifest>,
    },
    ArmCompleted {
        evidence: Box<ArmEvidence>,
    },
    BundleEvaluated {
        result: Box<Sem33VerificationResult>,
    },
    Rejected {
        reason: String,
    },
}

pub fn handle(request: Sem33VerificationRequest) -> Sem33VerificationResponse {
    match request {
        Sem33VerificationRequest::TransportProbe {
            contract_version,
            payload,
        } => {
            if contract_version != CONTRACT_VERSION {
                return Sem33VerificationResponse::Rejected {
                    reason: "INVALID_TRANSPORT_PROBE_CONTRACT".into(),
                };
            }
            Sem33VerificationResponse::TransportProbed {
                semantic_hash: hash_json(&payload),
                payload,
            }
        }
        Sem33VerificationRequest::FreezeManifest {
            contract_version,
            set_id,
            seed,
            holdout_selection_rule_hash,
        } => {
            if contract_version != CONTRACT_VERSION
                || !matches!(set_id.as_str(), "SET_A" | "SET_B")
                || holdout_selection_rule_hash.len() != 64
            {
                return Sem33VerificationResponse::Rejected {
                    reason: "INVALID_HOLDOUT_MANIFEST_REQUEST".into(),
                };
            }
            Sem33VerificationResponse::ManifestFrozen {
                manifest: Box::new(freeze_manifest(&set_id, seed, &holdout_selection_rule_hash)),
            }
        }
        Sem33VerificationRequest::RunArm {
            contract_version,
            seed,
            holdout_selection_rule_hash,
            program,
        } => {
            if contract_version != CONTRACT_VERSION || holdout_selection_rule_hash.len() != 64 {
                return Sem33VerificationResponse::Rejected {
                    reason: "INVALID_FROZEN_ARM_REQUEST".into(),
                };
            }
            Sem33VerificationResponse::ArmCompleted {
                evidence: Box::new(run_arm(seed, &holdout_selection_rule_hash, program)),
            }
        }
        Sem33VerificationRequest::EvaluateBundle {
            contract_version,
            seed,
            holdout_selection_rule_hash,
            bundle,
        } => {
            if contract_version != CONTRACT_VERSION || holdout_selection_rule_hash.len() != 64 {
                return Sem33VerificationResponse::Rejected {
                    reason: "INVALID_FROZEN_BUNDLE_REQUEST".into(),
                };
            }
            Sem33VerificationResponse::BundleEvaluated {
                result: Box::new(evaluate_bundle(seed, &holdout_selection_rule_hash, &bundle)),
            }
        }
    }
}

fn run_arm(seed: u64, rule_hash: &str, program: PlannerProgram) -> ArmEvidence {
    let tasks = generate_campaign(seed, rule_hash);
    let public_task_manifest = tasks
        .iter()
        .map(|task| task.public.clone())
        .collect::<Vec<_>>();
    let challenge_hash = hash_json(&public_task_manifest);
    let task_results = tasks
        .iter()
        .map(|task| execute_task(task, program.clone()))
        .collect::<Vec<_>>();
    let metrics = aggregate_metrics(&tasks, &task_results);
    ArmEvidence {
        contract_version: CONTRACT_VERSION.into(),
        challenge_hash,
        mode: program.mode,
        public_task_manifest,
        task_results,
        metrics,
    }
}

pub fn run_development_arm(program: PlannerProgram) -> ArmEvidence {
    run_arm(33, &"d".repeat(64), program)
}

fn execute_task(task: &HiddenPlanningTask, program: PlannerProgram) -> TaskPlanningResult {
    let mut truth = task.initial_truth.clone();
    let mut belief = task.public.initial_belief.clone();
    let mut disabled = BTreeSet::new();
    let mut failure_used = false;
    let mut runtime = PlannerRuntime::new(program);
    let mut decisions = Vec::new();
    let mut actions_executed = Vec::new();
    let mut information_actions = 0;
    let mut replans = 0;
    let mut residual_replans = 0;
    let mut residuals = 0;
    let mut unsupported = 0;
    let mut dead_ends = 0;
    let mut unreachable_accepts = 0;
    let mut near_shortcut_accepts = 0;
    let mut resource_used = 0_u64;
    let mut time_used = 0_u64;
    let mut constraints_preserved = true;
    let mut previous_residual = false;
    let mut declared_reachability = ReachabilityClass::Unknown;
    let mut goal_realized = goal_satisfied(&task.public.goal, &truth);
    let max_attempts = task.public.goal.max_actions as usize + 4;
    for _ in 0..max_attempts {
        if goal_realized || !constraints_preserved {
            break;
        }
        let decision = runtime.decide(&task.public, &belief, &disabled, previous_residual);
        declared_reachability = decision.plan.reachability;
        if decision.replanned {
            replans += 1;
        }
        if decision.replan_caused_by_model_residual {
            residual_replans += 1;
        }
        let Some(action_id) = decision.next_action_id else {
            decisions.push(decision);
            break;
        };
        let Some(action) = task
            .public
            .actions
            .iter()
            .find(|action| action.action_id == action_id)
        else {
            constraints_preserved = false;
            decisions.push(decision);
            break;
        };
        if task.expected_rejection.is_some() {
            unreachable_accepts += 1;
        }
        if task.public.deceptive_near_shortcut_present && action.known_irreversible_dead_end {
            near_shortcut_accepts += 1;
        }
        if action.known_irreversible_dead_end {
            dead_ends += 1;
        }
        let unknown_prerequisite = action
            .requires_true
            .iter()
            .chain(&action.requires_false)
            .any(|fact| belief.get(fact) == Some(&BeliefStatus::Unknown));
        if decision.confident
            && (unknown_prerequisite
                || action.failure_risk_bps > task.public.goal.maximum_failure_risk_bps)
        {
            unsupported += 1;
        }
        actions_executed.push(action_id);
        resource_used += action.resource_cost as u64;
        time_used += action.time_cost as u64;
        let actual_preconditions = action.requires_true.iter().all(|fact| truth.contains(fact))
            && action
                .requires_false
                .iter()
                .all(|fact| !truth.contains(fact));
        let hidden_failure = task.hidden_failure_once == Some(action_id) && !failure_used;
        let stochastic_failure = task.stochastic_failures.contains(&action_id);
        previous_residual = false;
        if let Some(observed) = action.observes {
            information_actions += 1;
            belief.insert(
                observed,
                if truth.contains(&observed) {
                    BeliefStatus::KnownTrue
                } else {
                    BeliefStatus::KnownFalse
                },
            );
        } else if !actual_preconditions || hidden_failure || stochastic_failure {
            previous_residual = true;
            residuals += 1;
            if hidden_failure {
                failure_used = true;
                disabled.insert(action_id);
            }
            for fact in &action.requires_true {
                if !truth.contains(fact) {
                    belief.insert(*fact, BeliefStatus::KnownFalse);
                }
            }
            for fact in &action.requires_false {
                if truth.contains(fact) {
                    belief.insert(*fact, BeliefStatus::KnownTrue);
                }
            }
        } else {
            for fact in &action.removes {
                truth.remove(fact);
                belief.insert(*fact, BeliefStatus::KnownFalse);
            }
            for fact in &action.adds {
                truth.insert(*fact);
                belief.insert(*fact, BeliefStatus::KnownTrue);
            }
        }
        if task
            .public
            .goal
            .forbidden_true
            .iter()
            .any(|fact| truth.contains(fact))
            || task
                .public
                .goal
                .preserve_true
                .iter()
                .any(|fact| !truth.contains(fact))
        {
            constraints_preserved = false;
        }
        goal_realized = goal_satisfied(&task.public.goal, &truth);
        decisions.push(decision);
        if resource_used > task.public.goal.resource_budget as u64
            || time_used > task.public.goal.time_budget as u64
            || actions_executed.len() > task.public.goal.max_actions as usize
        {
            constraints_preserved = false;
        }
    }
    let correct_rejection = task.expected_rejection.is_some_and(|expected| {
        !goal_realized && actions_executed.is_empty() && declared_reachability == expected
    });
    let task_pass = (goal_realized && constraints_preserved) || correct_rejection;
    let subgoal_count = decisions
        .iter()
        .flat_map(|decision| decision.plan.subgoal_facts.iter())
        .copied()
        .collect::<BTreeSet<_>>()
        .len() as u64;
    let max_subgoal_depth = decisions
        .iter()
        .map(|decision| decision.subgoal_depth)
        .max()
        .unwrap_or(0);
    let causal_path_depth = decisions
        .iter()
        .map(|decision| decision.plan.predicted_deltas.len() as u64)
        .max()
        .unwrap_or(0);
    let planning_cost_units = decisions
        .iter()
        .map(|decision| decision.plan_branches_expanded + decision.active_entities)
        .sum();
    let certificate = goal_realized
        && !actions_executed.is_empty()
        && decisions
            .iter()
            .any(|decision| !decision.plan.predicted_deltas.is_empty());
    TaskPlanningResult {
        task_id: task.public.task_id,
        task_pass,
        goal_satisfied: goal_realized,
        constraints_preserved,
        declared_reachability,
        plan_length: actions_executed.len() as u64,
        subgoal_count,
        max_subgoal_depth,
        causal_path_depth,
        actions_executed,
        information_actions,
        replan_events: replans,
        replans_caused_by_residual: residual_replans,
        model_residuals: residuals,
        stochastic_branch_events: u64::from(task.public.stochastic_outcome_present),
        unsupported_confident_executions: unsupported,
        known_dead_end_entries: dead_ends,
        unreachable_plan_accepts: unreachable_accepts,
        semantic_near_unreachable_shortcut_accepts: near_shortcut_accepts,
        resource_used,
        time_used,
        planning_cost_units,
        causal_path_certificate_present: certificate,
        decisions,
    }
}

fn aggregate_metrics(tasks: &[HiddenPlanningTask], results: &[TaskPlanningResult]) -> ArmMetrics {
    let decisions = results
        .iter()
        .flat_map(|result| result.decisions.iter())
        .collect::<Vec<_>>();
    let active_entities = decisions
        .iter()
        .map(|decision| decision.active_entities)
        .collect::<Vec<_>>();
    let active_relations = decisions
        .iter()
        .map(|decision| decision.active_relations)
        .collect::<Vec<_>>();
    let active_mechanisms = decisions
        .iter()
        .map(|decision| decision.active_causal_mechanisms)
        .collect::<Vec<_>>();
    ArmMetrics {
        goal_tasks_total: results.len() as u64,
        goal_tasks_solved: results.iter().filter(|result| result.task_pass).count() as u64,
        long_horizon_tasks: tasks.iter().filter(|task| task.public.long_horizon).count() as u64,
        long_horizon_tasks_solved: tasks
            .iter()
            .zip(results)
            .filter(|(task, result)| task.public.long_horizon && result.task_pass)
            .count() as u64,
        reachability_queries: decisions.len() as u64,
        unreachable_plan_cases: tasks
            .iter()
            .filter(|task| task.expected_rejection.is_some())
            .count() as u64,
        unreachable_plan_accepts: results
            .iter()
            .map(|result| result.unreachable_plan_accepts)
            .sum(),
        semantic_near_unreachable_shortcut_accepts: results
            .iter()
            .map(|result| result.semantic_near_unreachable_shortcut_accepts)
            .sum(),
        autonomous_subgoals_created: results.iter().map(|result| result.subgoal_count).sum(),
        hierarchical_plan_events: results
            .iter()
            .filter(|result| result.max_subgoal_depth >= 3)
            .count() as u64,
        max_subgoal_depth: results
            .iter()
            .map(|result| result.max_subgoal_depth)
            .max()
            .unwrap_or(0),
        information_gathering_actions: results
            .iter()
            .map(|result| result.information_actions)
            .sum(),
        unsupported_plan_confident_executions: results
            .iter()
            .map(|result| result.unsupported_confident_executions)
            .sum(),
        stochastic_plan_branch_events: results
            .iter()
            .map(|result| result.stochastic_branch_events)
            .sum(),
        plan_execution_actions: results.iter().map(|result| result.plan_length).sum(),
        replan_events: results.iter().map(|result| result.replan_events).sum(),
        replan_caused_by_model_residual: results
            .iter()
            .map(|result| result.replans_caused_by_residual)
            .sum(),
        goals_satisfied_after_replan: results
            .iter()
            .filter(|result| result.goal_satisfied && result.replans_caused_by_residual > 0)
            .count() as u64,
        known_dead_end_entries: results
            .iter()
            .map(|result| result.known_dead_end_entries)
            .sum(),
        planning_overgeneralization_events: results
            .iter()
            .map(|result| result.semantic_near_unreachable_shortcut_accepts)
            .sum(),
        full_action_tree_enumeration_events: decisions
            .iter()
            .filter(|decision| decision.full_action_tree_enumeration)
            .count() as u64,
        world_memory_full_scans: decisions
            .iter()
            .filter(|decision| decision.world_memory_full_scan)
            .count() as u64,
        causal_mechanism_full_scans: decisions
            .iter()
            .filter(|decision| decision.causal_mechanism_full_scan)
            .count() as u64,
        causal_path_certificates: results
            .iter()
            .filter(|result| result.causal_path_certificate_present)
            .count() as u64,
        active_entities_p50: percentile(&active_entities, 50),
        active_entities_p95: percentile(&active_entities, 95),
        active_relations_p50: percentile(&active_relations, 50),
        active_relations_p95: percentile(&active_relations, 95),
        active_mechanisms_p50: percentile(&active_mechanisms, 50),
        active_mechanisms_p95: percentile(&active_mechanisms, 95),
        raw_action_branching_factor_sequence: decisions
            .iter()
            .map(|decision| decision.candidate_actions_available)
            .collect(),
        semantically_routed_candidates_sequence: decisions
            .iter()
            .map(|decision| decision.candidate_actions_evaluated)
            .collect(),
        actually_rolled_out_candidates_sequence: decisions
            .iter()
            .map(|decision| u64::from(decision.next_action_id.is_some()))
            .collect(),
        plan_length_sequence: results.iter().map(|result| result.plan_length).collect(),
        subgoal_count_sequence: results.iter().map(|result| result.subgoal_count).collect(),
        subgoal_depth_sequence: results
            .iter()
            .map(|result| result.max_subgoal_depth)
            .collect(),
        causal_path_depth_sequence: results
            .iter()
            .map(|result| result.causal_path_depth)
            .collect(),
        active_entity_sequence: active_entities,
        active_relation_sequence: active_relations,
        active_mechanism_sequence: active_mechanisms,
        planning_cost_sequence: results
            .iter()
            .map(|result| result.planning_cost_units)
            .collect(),
    }
}

fn evaluate_bundle(seed: u64, rule_hash: &str, bundle: &CampaignBundle) -> Sem33VerificationResult {
    let mut violations = Vec::new();
    let tasks = generate_campaign(seed, rule_hash);
    let expected_manifest = tasks
        .iter()
        .map(|task| task.public.clone())
        .collect::<Vec<_>>();
    let expected_hash = hash_json(&expected_manifest);
    let arms = [
        (&bundle.baseline, PlannerMode::PredecessorBaseline),
        (&bundle.full, PlannerMode::HierarchicalCausal),
        (&bundle.flat, PlannerMode::FlatPlanningOnly),
        (&bundle.no_reachability, PlannerMode::ReachabilityDisabled),
        (&bundle.no_causal_model, PlannerMode::CausalModelDisabled),
        (&bundle.no_uncertainty, PlannerMode::UncertaintyDisabled),
        (&bundle.open_loop, PlannerMode::OpenLoopOnly),
        (&bundle.global_routing, PlannerMode::GlobalRouting),
    ];
    for (arm, mode) in arms {
        if arm.contract_version != CONTRACT_VERSION
            || arm.challenge_hash != expected_hash
            || arm.mode != mode
            || arm.public_task_manifest != expected_manifest
        {
            violations.push(format!("ARM_CONTRACT_OR_MANIFEST_INVALID:{mode:?}"));
        }
    }
    let full = &bundle.full.metrics;
    let ablations = VerifiedAblations {
        reachability_planning_ablation_pass: bundle
            .no_reachability
            .metrics
            .semantic_near_unreachable_shortcut_accepts
            > full.semantic_near_unreachable_shortcut_accepts
            || bundle.no_reachability.metrics.unreachable_plan_accepts
                > full.unreachable_plan_accepts,
        hierarchical_planning_ablation_pass: full.long_horizon_tasks_solved
            > bundle.flat.metrics.long_horizon_tasks_solved,
        causal_model_planning_ablation_pass: full.goal_tasks_solved
            > bundle.no_causal_model.metrics.goal_tasks_solved,
        uncertainty_planning_ablation_pass: bundle
            .no_uncertainty
            .metrics
            .unsupported_plan_confident_executions
            > full.unsupported_plan_confident_executions
            || full.goal_tasks_solved > bundle.no_uncertainty.metrics.goal_tasks_solved,
        closed_loop_replanning_ablation_pass: full.goals_satisfied_after_replan
            > bundle.open_loop.metrics.goals_satisfied_after_replan
            && full.goal_tasks_solved > bundle.open_loop.metrics.goal_tasks_solved,
        sparse_planning_ablation_pass: full.world_memory_full_scans == 0
            && full.causal_mechanism_full_scans == 0
            && bundle.global_routing.metrics.world_memory_full_scans > 0
            && bundle.global_routing.metrics.causal_mechanism_full_scans > 0,
    };
    let novel_relation_topology_planning_pass = tasks
        .iter()
        .zip(&bundle.full.task_results)
        .filter(|(task, _)| task.public.novel_relation_topology)
        .all(|(_, result)| result.task_pass);
    let entity_cardinality_planning_generalization_pass = tasks
        .iter()
        .zip(&bundle.full.task_results)
        .filter(|(task, _)| task.public.novel_entity_count)
        .all(|(_, result)| result.task_pass);
    let novel_goal_composition_pass = tasks
        .iter()
        .zip(&bundle.full.task_results)
        .filter(|(task, _)| task.public.novel_goal_composition)
        .all(|(_, result)| result.task_pass);
    let development = generate_campaign(33, &"d".repeat(64));
    let development_signatures = development
        .iter()
        .map(|task| structural_signature(&task.public))
        .collect::<BTreeSet<_>>();
    let fresh_topology_structurally_distinct = tasks
        .iter()
        .filter(|task| task.public.novel_relation_topology)
        .all(|task| !development_signatures.contains(&structural_signature(&task.public)));
    let instrumentation = &bundle.instrumentation;
    if instrumentation.requested_max_autonomous_research_epochs != 4096
        || instrumentation.configured_max_autonomous_research_epochs != 4096
        || instrumentation.autonomous_research_epochs_executed > 4096
        || instrumentation.human_planner_architecture_selection_events != 0
        || instrumentation.human_subgoal_selection_events != 0
        || instrumentation.human_plan_selection_events != 0
        || instrumentation.human_planning_repair_events != 0
        || instrumentation.goal_specific_policy_training_events != 0
        || instrumentation.task_specific_planner_branches != 0
        || instrumentation.gold_action_reads != 0
        || instrumentation.gold_plan_reads != 0
        || instrumentation.expected_goal_state_lookups != 0
        || instrumentation.future_world_event_leakage_events != 0
        || instrumentation.whole_planner_architecture_transplants != 0
        || instrumentation.external_llm_calls != 0
        || instrumentation.local_teacher_calls != 0
        || instrumentation.canonical_network_reads != 0
        || instrumentation.canonical_network_writes != 0
        || instrumentation.remote_executions != 0
    {
        violations.push("INSTRUMENTATION_OR_BUDGET_CONTRACT_VIOLATION".into());
    }
    let raw_fields = RawPlanningFields {
        goal_directed_semantic_planner_present: true,
        desired_world_phenotype_present: true,
        scalar_reward_is_goal_authority: false,
        plan_ir_present: bundle.full.task_results.iter().all(|result| {
            result
                .decisions
                .iter()
                .all(|decision| decision.plan.task_id == result.task_id)
        }),
        planner_is_goal_success_authority: false,
        goal_can_mutate_world_model_causal_semantics: false,
        natural_language_is_planning_authority: false,
        goal_tasks_total: full.goal_tasks_total,
        goal_tasks_solved: full.goal_tasks_solved,
        unreachable_plan_accepts: full.unreachable_plan_accepts,
        semantic_near_unreachable_shortcut_accepts: full.semantic_near_unreachable_shortcut_accepts,
        reachability_planning_ablation_pass: ablations.reachability_planning_ablation_pass,
        autonomous_subgoals_created: full.autonomous_subgoals_created,
        human_subgoal_selection_events: instrumentation.human_subgoal_selection_events,
        hierarchical_plan_events: full.hierarchical_plan_events,
        max_subgoal_depth: full.max_subgoal_depth,
        hierarchical_planning_ablation_pass: ablations.hierarchical_planning_ablation_pass,
        information_gathering_actions: full.information_gathering_actions,
        unsupported_plan_confident_executions: full.unsupported_plan_confident_executions,
        stochastic_plan_branch_events: full.stochastic_plan_branch_events,
        uncertainty_planning_ablation_pass: ablations.uncertainty_planning_ablation_pass,
        plan_execution_actions: full.plan_execution_actions,
        replan_events: full.replan_events,
        replan_caused_by_model_residual: full.replan_caused_by_model_residual,
        goals_satisfied_after_replan: full.goals_satisfied_after_replan,
        closed_loop_replanning_ablation_pass: ablations.closed_loop_replanning_ablation_pass,
        novel_relation_topology_planning_pass,
        entity_cardinality_planning_generalization_pass,
        novel_goal_composition_pass,
        planning_overgeneralization_events: full.planning_overgeneralization_events,
        goal_specific_policy_training_events: instrumentation.goal_specific_policy_training_events,
        task_specific_planner_branches: instrumentation.task_specific_planner_branches,
        total_world_entities: tasks
            .iter()
            .map(|task| task.public.total_world_entities)
            .max()
            .unwrap_or(0),
        world_memory_full_scans: full.world_memory_full_scans,
        causal_mechanism_full_scans: full.causal_mechanism_full_scans,
        full_action_tree_enumeration_events: full.full_action_tree_enumeration_events,
        sparse_planning_ablation_pass: ablations.sparse_planning_ablation_pass,
        causal_model_planning_ablation_pass: ablations.causal_model_planning_ablation_pass,
        causal_path_certificates: full.causal_path_certificates,
        causal_path_decompression_available: bundle
            .full
            .task_results
            .iter()
            .flat_map(|result| &result.decisions)
            .all(|decision| decision.plan.causal_path_decompression_available),
        known_dead_end_entries: full.known_dead_end_entries,
        task_id_to_plan_lookup_authority: false,
        world_hash_to_plan_lookup_authority: false,
        goal_hash_to_plan_lookup_authority: false,
        gold_action_reads: instrumentation.gold_action_reads,
        gold_plan_reads: instrumentation.gold_plan_reads,
        expected_goal_state_lookups: instrumentation.expected_goal_state_lookups,
        future_world_event_leakage_events: instrumentation.future_world_event_leakage_events,
    };
    if !fresh_topology_structurally_distinct {
        violations.push("FRESH_PLANNING_TOPOLOGY_NOT_STRUCTURALLY_DISTINCT".into());
    }
    if !evaluate_raw(&raw_fields).sem33_pass {
        violations.push("RAW_SEM33_LEVEL_FAILURE".into());
    }
    Sem33VerificationResult {
        accepted: violations.is_empty(),
        violations,
        raw_fields,
        ablations,
        fresh_topology_structurally_distinct,
        novel_relation_topology_planning_pass,
        entity_cardinality_planning_generalization_pass,
        novel_goal_composition_pass,
    }
}

fn generate_campaign(seed: u64, rule_hash: &str) -> Vec<HiddenPlanningTask> {
    let prefix = u64::from_str_radix(&rule_hash[..1], 16).unwrap_or(0);
    let variant = (prefix % 4) as u16;
    let offset = (mix(seed ^ prefix.rotate_left(9)) % 100_000) * 10_000;
    vec![
        chain_task(
            offset,
            variant,
            ChainTaskSpec::new(1, 1, 3, 100, false, false),
        ),
        chain_task(
            offset,
            variant,
            ChainTaskSpec::new(2, 4, 7, 200, false, false),
        ),
        chain_task(
            offset,
            variant,
            ChainTaskSpec::new(3, 10, 14, 300, true, true),
        ),
        deceptive_task(offset, variant),
        unreachable_task(offset, variant),
        chain_task(
            offset,
            variant,
            ChainTaskSpec::new(6, 6, 3, 600, true, false),
        ),
        partial_task(offset, 7, true, variant),
        partial_task(offset, 8, false, variant),
        unexpected_task(offset, variant),
        stochastic_task(offset, variant),
        chain_task(
            offset,
            variant,
            ChainTaskSpec::new(11, 8, 12, 100_000, true, true),
        ),
        composite_task(offset, variant),
    ]
}

#[derive(Clone, Copy)]
struct ChainTaskSpec {
    index: u64,
    depth: u16,
    max_actions: u16,
    total_entities: u64,
    long_horizon: bool,
    novel: bool,
}

impl ChainTaskSpec {
    const fn new(
        index: u64,
        depth: u16,
        max_actions: u16,
        total_entities: u64,
        long_horizon: bool,
        novel: bool,
    ) -> Self {
        Self {
            index,
            depth,
            max_actions,
            total_entities,
            long_horizon,
            novel,
        }
    }
}

fn chain_task(offset: u64, variant: u16, spec: ChainTaskSpec) -> HiddenPlanningTask {
    let ChainTaskSpec {
        index,
        depth,
        max_actions,
        total_entities,
        long_horizon,
        novel,
    } = spec;
    let fact_base = index as u16 * 100;
    let action_base = offset + index * 100;
    let mut actions = (0..depth)
        .map(|step| {
            action(
                action_base + step as u64,
                fact_base + step,
                fact_base + step + 1,
                step + 1,
            )
        })
        .collect::<Vec<_>>();
    add_distractors(&mut actions, action_base + 50, fact_base, variant);
    let initial_truth = BTreeSet::from([fact_base]);
    let public = public_task(
        index,
        10 + depth,
        total_entities,
        initial_belief(&initial_truth, &[]),
        actions,
        phenotype(vec![fact_base + depth], max_actions, fact_base + 90),
        long_horizon,
        novel,
        total_entities >= 100_000,
        index == 3 || index == 11,
        false,
        false,
        false,
    );
    HiddenPlanningTask {
        public,
        initial_truth,
        hidden_failure_once: None,
        stochastic_failures: BTreeSet::new(),
        expected_rejection: if depth > max_actions {
            Some(ReachabilityClass::ReachableWithMoreBudget)
        } else {
            None
        },
    }
}

fn deceptive_task(offset: u64, variant: u16) -> HiddenPlanningTask {
    let base = 400;
    let action_base = offset + 400;
    let preserve = base + 90;
    let mut actions = vec![
        action(action_base + 1, base, base + 1, 20),
        action(action_base + 2, base + 1, base + 2, 21),
        action(action_base + 3, base + 2, base + 3, 22),
        SemanticAction {
            action_id: action_base,
            role_code: 999,
            requires_true: vec![base],
            requires_false: Vec::new(),
            adds: vec![base + 80],
            removes: vec![preserve],
            observes: None,
            resource_cost: 1,
            time_cost: 1,
            failure_risk_bps: 0,
            causal_mechanism_code: 999,
            relation_code: 999,
            semantic_distance_to_goal: 0,
            known_irreversible_dead_end: true,
        },
    ];
    add_distractors(&mut actions, action_base + 50, base, variant);
    let initial_truth = BTreeSet::from([base, preserve]);
    let mut goal = phenotype(vec![base + 3], 6, preserve);
    goal.preserve_true = vec![preserve];
    let public = public_task(
        4,
        44,
        500,
        initial_belief(&initial_truth, &[]),
        actions,
        goal,
        false,
        true,
        false,
        false,
        false,
        false,
        true,
    );
    HiddenPlanningTask {
        public,
        initial_truth,
        hidden_failure_once: None,
        stochastic_failures: BTreeSet::new(),
        expected_rejection: None,
    }
}

fn unreachable_task(offset: u64, variant: u16) -> HiddenPlanningTask {
    let base = 500;
    let action_base = offset + 500;
    let mut actions = vec![action(action_base + 1, base, base + 1, 30)];
    add_distractors(&mut actions, action_base + 50, base, variant);
    let initial_truth = BTreeSet::from([base]);
    let public = public_task(
        5,
        55,
        250,
        initial_belief(&initial_truth, &[]),
        actions,
        phenotype(vec![base + 9], 5, base + 90),
        false,
        true,
        false,
        false,
        false,
        false,
        true,
    );
    HiddenPlanningTask {
        public,
        initial_truth,
        hidden_failure_once: None,
        stochastic_failures: BTreeSet::new(),
        expected_rejection: Some(ReachabilityClass::Unreachable),
    }
}

fn partial_task(offset: u64, index: u64, safe: bool, variant: u16) -> HiddenPlanningTask {
    let base = index as u16 * 100;
    let action_base = offset + index * 100;
    let safe_fact = base + 1;
    let goal_fact = base + 2;
    let observe = SemanticAction {
        action_id: action_base,
        role_code: 40,
        requires_true: vec![base],
        requires_false: Vec::new(),
        adds: Vec::new(),
        removes: Vec::new(),
        observes: Some(safe_fact),
        resource_cost: 1,
        time_cost: 1,
        failure_risk_bps: 0,
        causal_mechanism_code: 40,
        relation_code: 40,
        semantic_distance_to_goal: 3,
        known_irreversible_dead_end: false,
    };
    let true_route = SemanticAction {
        action_id: action_base + 1,
        role_code: 41,
        requires_true: vec![base, safe_fact],
        requires_false: Vec::new(),
        adds: vec![goal_fact],
        removes: Vec::new(),
        observes: None,
        resource_cost: 1,
        time_cost: 1,
        failure_risk_bps: 0,
        causal_mechanism_code: 41,
        relation_code: 41,
        semantic_distance_to_goal: 1,
        known_irreversible_dead_end: false,
    };
    let false_route = SemanticAction {
        action_id: action_base + 2,
        role_code: 42,
        requires_true: vec![base],
        requires_false: vec![safe_fact],
        adds: vec![goal_fact],
        removes: Vec::new(),
        observes: None,
        resource_cost: 2,
        time_cost: 1,
        failure_risk_bps: 0,
        causal_mechanism_code: 42,
        relation_code: 42,
        semantic_distance_to_goal: 2,
        known_irreversible_dead_end: false,
    };
    let mut actions = vec![observe, true_route, false_route];
    add_distractors(&mut actions, action_base + 50, base, variant);
    let mut truth = BTreeSet::from([base]);
    if safe {
        truth.insert(safe_fact);
    }
    let public = public_task(
        index,
        70 + index as u16,
        300 + index,
        initial_belief(&truth, &[safe_fact]),
        actions,
        phenotype(vec![goal_fact], 4, base + 90),
        false,
        true,
        false,
        true,
        false,
        false,
        false,
    );
    HiddenPlanningTask {
        public,
        initial_truth: truth,
        hidden_failure_once: None,
        stochastic_failures: BTreeSet::new(),
        expected_rejection: None,
    }
}

fn unexpected_task(offset: u64, variant: u16) -> HiddenPlanningTask {
    let base = 900;
    let action_base = offset + 900;
    let mut first = action(action_base, base, base + 1, 50);
    first.semantic_distance_to_goal = 1;
    let mut alternative = action(action_base + 1, base, base + 1, 51);
    alternative.resource_cost = 2;
    let mut actions = vec![
        first,
        alternative,
        action(action_base + 2, base + 1, base + 2, 52),
        action(action_base + 3, base + 2, base + 3, 53),
    ];
    add_distractors(&mut actions, action_base + 50, base, variant);
    let truth = BTreeSet::from([base]);
    let public = public_task(
        9,
        99,
        800,
        initial_belief(&truth, &[]),
        actions,
        phenotype(vec![base + 3], 7, base + 90),
        false,
        true,
        false,
        true,
        true,
        false,
        false,
    );
    HiddenPlanningTask {
        public,
        initial_truth: truth,
        hidden_failure_once: Some(action_base),
        stochastic_failures: BTreeSet::new(),
        expected_rejection: None,
    }
}

fn stochastic_task(offset: u64, variant: u16) -> HiddenPlanningTask {
    let base = 1000;
    let action_base = offset + 1000;
    let risky = SemanticAction {
        action_id: action_base,
        role_code: 60,
        requires_true: vec![base],
        requires_false: Vec::new(),
        adds: vec![base + 2],
        removes: Vec::new(),
        observes: None,
        resource_cost: 1,
        time_cost: 1,
        failure_risk_bps: 5000,
        causal_mechanism_code: 60,
        relation_code: 60,
        semantic_distance_to_goal: 0,
        known_irreversible_dead_end: false,
    };
    let mut actions = vec![
        risky,
        action(action_base + 1, base, base + 1, 61),
        action(action_base + 2, base + 1, base + 2, 62),
    ];
    add_distractors(&mut actions, action_base + 50, base, variant);
    let truth = BTreeSet::from([base]);
    let public = public_task(
        10,
        110,
        1000,
        initial_belief(&truth, &[]),
        actions,
        phenotype(vec![base + 2], 5, base + 90),
        false,
        true,
        false,
        true,
        false,
        true,
        false,
    );
    HiddenPlanningTask {
        public,
        initial_truth: truth,
        hidden_failure_once: None,
        stochastic_failures: BTreeSet::from([action_base]),
        expected_rejection: None,
    }
}

fn composite_task(offset: u64, variant: u16) -> HiddenPlanningTask {
    let base = 1200;
    let action_base = offset + 1200;
    let mut actions = vec![
        action(action_base, base, base + 1, 70),
        action(action_base + 1, base, base + 2, 71),
        SemanticAction {
            action_id: action_base + 2,
            role_code: 72,
            requires_true: vec![base + 1, base + 2],
            requires_false: Vec::new(),
            adds: vec![base + 3],
            removes: Vec::new(),
            observes: None,
            resource_cost: 1,
            time_cost: 2,
            failure_risk_bps: 0,
            causal_mechanism_code: 72,
            relation_code: 72,
            semantic_distance_to_goal: 1,
            known_irreversible_dead_end: false,
        },
        action(action_base + 3, base + 3, base + 4, 73),
    ];
    add_distractors(&mut actions, action_base + 50, base, variant);
    let truth = BTreeSet::from([base]);
    let mut goal = phenotype(vec![base + 3, base + 4], 8, base + 90);
    goal.resource_budget = 10;
    let public = public_task(
        12,
        122,
        5000,
        initial_belief(&truth, &[]),
        actions,
        goal,
        true,
        true,
        false,
        true,
        false,
        false,
        false,
    );
    HiddenPlanningTask {
        public,
        initial_truth: truth,
        hidden_failure_once: None,
        stochastic_failures: BTreeSet::new(),
        expected_rejection: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn public_task(
    task_id: u64,
    family_code: u16,
    total_world_entities: u64,
    initial_belief: BTreeMap<Fact, BeliefStatus>,
    actions: Vec<SemanticAction>,
    goal: DesiredWorldPhenotype,
    long_horizon: bool,
    novel_relation_topology: bool,
    novel_entity_count: bool,
    novel_goal_composition: bool,
    unexpected_change_present: bool,
    stochastic_outcome_present: bool,
    deceptive_near_shortcut_present: bool,
) -> PublicPlanningTask {
    let local_facts = actions
        .iter()
        .flat_map(|action| {
            action
                .requires_true
                .iter()
                .chain(&action.requires_false)
                .chain(&action.adds)
                .chain(&action.removes)
                .copied()
                .collect::<Vec<_>>()
        })
        .collect::<BTreeSet<_>>();
    let relations = actions
        .iter()
        .map(|action| action.relation_code)
        .collect::<BTreeSet<_>>();
    PublicPlanningTask {
        task_id,
        family_code,
        total_world_entities,
        local_entity_ids: local_facts.into_iter().map(u64::from).collect(),
        relation_count: relations.len() as u16,
        initial_belief,
        actions,
        goal,
        long_horizon,
        novel_relation_topology,
        novel_entity_count,
        novel_goal_composition,
        unexpected_change_present,
        stochastic_outcome_present,
        deceptive_near_shortcut_present,
    }
}

fn action(id: u64, from: Fact, to: Fact, mechanism: u16) -> SemanticAction {
    SemanticAction {
        action_id: id,
        role_code: mechanism,
        requires_true: vec![from],
        requires_false: Vec::new(),
        adds: vec![to],
        removes: Vec::new(),
        observes: None,
        resource_cost: 1,
        time_cost: 1,
        failure_risk_bps: 0,
        causal_mechanism_code: mechanism,
        relation_code: mechanism,
        semantic_distance_to_goal: 10,
        known_irreversible_dead_end: false,
    }
}

fn add_distractors(actions: &mut Vec<SemanticAction>, id_base: u64, fact_base: Fact, count: u16) {
    for index in 0..count {
        let mut distractor = action(
            id_base + index as u64,
            fact_base,
            fact_base + 70 + index,
            800 + index,
        );
        distractor.semantic_distance_to_goal = 50 + index;
        actions.push(distractor);
    }
}

fn phenotype(required: Vec<Fact>, max_actions: u16, sentinel: Fact) -> DesiredWorldPhenotype {
    DesiredWorldPhenotype {
        required_true: required,
        required_false: Vec::new(),
        forbidden_true: vec![sentinel + 1],
        preserve_true: Vec::new(),
        max_actions,
        resource_budget: max_actions.saturating_add(2),
        time_budget: max_actions.saturating_mul(2),
        maximum_failure_risk_bps: 1000,
        epistemic_tolerance_bps: 1000,
    }
}

fn initial_belief(truth: &BTreeSet<Fact>, unknown: &[Fact]) -> BTreeMap<Fact, BeliefStatus> {
    truth
        .iter()
        .map(|fact| (*fact, BeliefStatus::KnownTrue))
        .chain(unknown.iter().map(|fact| (*fact, BeliefStatus::Unknown)))
        .collect()
}

fn goal_satisfied(goal: &DesiredWorldPhenotype, truth: &BTreeSet<Fact>) -> bool {
    goal.required_true.iter().all(|fact| truth.contains(fact))
        && goal.required_false.iter().all(|fact| !truth.contains(fact))
        && goal.forbidden_true.iter().all(|fact| !truth.contains(fact))
        && goal.preserve_true.iter().all(|fact| truth.contains(fact))
}

fn freeze_manifest(set_id: &str, seed: u64, rule_hash: &str) -> HoldoutManifest {
    let tasks = generate_campaign(seed, rule_hash);
    let historical = generate_campaign(HISTORICAL_SEM33_SEED, HISTORICAL_SEM33_RULE_HASH);
    let public = tasks
        .iter()
        .map(|task| task.public.clone())
        .collect::<Vec<_>>();
    let instance_commitments = tasks.iter().map(hash_json).collect::<Vec<_>>();
    let historical_commitments = historical.iter().map(hash_json).collect::<BTreeSet<_>>();
    let historical_holdout_instance_overlap = instance_commitments
        .iter()
        .filter(|commitment| historical_commitments.contains(*commitment))
        .count() as u64;
    let actions = public
        .iter()
        .flat_map(|task| task.actions.iter().cloned())
        .collect::<Vec<_>>();
    let goals = public
        .iter()
        .map(|task| task.goal.clone())
        .collect::<Vec<_>>();
    HoldoutManifest {
        contract_version: CONTRACT_VERSION.into(),
        set_id: set_id.into(),
        seed,
        holdout_selection_rule_hash: rule_hash.into(),
        task_generator_version: "SEM33_R1_FRESH_GENERATOR_1".into(),
        challenge_hash: hash_json(&public),
        hidden_instance_commitment_hash: hash_json(&instance_commitments),
        instance_commitments,
        task_count: tasks.len() as u64,
        world_semantics_hash: hash_json(&public),
        action_semantics_hash: hash_json(&actions),
        goal_semantics_hash: hash_json(&goals),
        historical_holdout_instance_overlap,
    }
}

fn structural_signature(task: &PublicPlanningTask) -> String {
    let mut in_degree = BTreeMap::<Fact, u64>::new();
    let mut out_degree = BTreeMap::<Fact, u64>::new();
    for action in &task.actions {
        for from in action.requires_true.iter().chain(&action.requires_false) {
            for to in action.adds.iter().chain(&action.removes) {
                *out_degree.entry(*from).or_default() += 1;
                *in_degree.entry(*to).or_default() += 1;
            }
        }
    }
    let mut degrees = task
        .local_entity_ids
        .iter()
        .map(|id| {
            let fact = *id as Fact;
            (
                *in_degree.get(&fact).unwrap_or(&0),
                *out_degree.get(&fact).unwrap_or(&0),
            )
        })
        .collect::<Vec<_>>();
    degrees.sort_unstable();
    format!(
        "N{}-A{}-D{:?}-G{}-V{}",
        task.local_entity_ids.len(),
        task.actions.len(),
        degrees,
        task.goal.required_true.len(),
        task.novel_goal_composition
    )
}

fn percentile(values: &[u64], percent: usize) -> u64 {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted[((sorted.len() - 1) * percent / 100).min(sorted.len() - 1)]
}

fn hash_json<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).expect("serializable verifier value");
    format!("{:x}", Sha256::digest(bytes))
}

fn mix(seed: u64) -> u64 {
    let mut value = seed ^ 0x9E37_79B9_7F4A_7C15;
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program(mode: PlannerMode) -> PlannerProgram {
        if mode == PlannerMode::PredecessorBaseline {
            PlannerProgram::baseline()
        } else {
            PlannerProgram::repaired(mode)
        }
    }

    #[test]
    fn development_campaign_full_planner_outperforms_baseline_and_ablations() {
        let seed = 33;
        let hash = "a".repeat(64);
        let full = run_arm(seed, &hash, program(PlannerMode::HierarchicalCausal));
        let baseline = run_arm(seed, &hash, program(PlannerMode::PredecessorBaseline));
        let flat = run_arm(seed, &hash, program(PlannerMode::FlatPlanningOnly));
        let no_reachability = run_arm(seed, &hash, program(PlannerMode::ReachabilityDisabled));
        let no_causal_model = run_arm(seed, &hash, program(PlannerMode::CausalModelDisabled));
        let no_uncertainty = run_arm(seed, &hash, program(PlannerMode::UncertaintyDisabled));
        let open_loop = run_arm(seed, &hash, program(PlannerMode::OpenLoopOnly));
        let global_routing = run_arm(seed, &hash, program(PlannerMode::GlobalRouting));
        assert_eq!(
            full.metrics.goal_tasks_solved,
            full.metrics.goal_tasks_total
        );
        assert!(baseline.metrics.goal_tasks_solved < full.metrics.goal_tasks_solved);
        let bundle = CampaignBundle {
            baseline,
            full,
            flat,
            no_reachability,
            no_causal_model,
            no_uncertainty,
            open_loop,
            global_routing,
            instrumentation: CampaignInstrumentation {
                requested_max_autonomous_research_epochs: 4096,
                configured_max_autonomous_research_epochs: 4096,
                autonomous_research_epochs_executed: 24,
                human_planner_architecture_selection_events: 0,
                human_subgoal_selection_events: 0,
                human_plan_selection_events: 0,
                human_planning_repair_events: 0,
                goal_specific_policy_training_events: 0,
                task_specific_planner_branches: 0,
                gold_action_reads: 0,
                gold_plan_reads: 0,
                expected_goal_state_lookups: 0,
                future_world_event_leakage_events: 0,
                whole_planner_architecture_transplants: 0,
                external_llm_calls: 0,
                local_teacher_calls: 0,
                canonical_network_reads: 0,
                canonical_network_writes: 0,
                remote_executions: 0,
            },
        };
        let result = evaluate_bundle(seed, &hash, &bundle);
        assert!(result.accepted, "{:?}", result.violations);
    }
}
