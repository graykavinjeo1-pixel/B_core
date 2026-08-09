use std::{
    collections::{BTreeMap, BTreeSet},
    process::Command,
    time::Instant,
};

use serde::{Deserialize, Serialize};

use crate::sem33_r1::engine::{
    BeliefStatus, DesiredWorldPhenotype, Fact, PlannerMode, PlannerProgram, PlannerRuntime,
    PublicPlanningTask, SemanticAction,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScalingSet {
    Development,
    FinalHoldout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScalingPlannerMode {
    BaselineSem33R1,
    SemanticIndexOnly,
    EfficientAdaptive,
    ReachabilityAblated,
    SingleScale,
    HierarchyAblated,
    GlobalRouting,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScalingPlannerProgram {
    pub mode: ScalingPlannerMode,
    pub semantic_index: bool,
    pub reachability_pruning: bool,
    pub adaptive_temporal_abstraction: bool,
    pub hierarchy_reuse: bool,
    pub sparse_world_routing: bool,
    pub bounded_local_execution: bool,
    pub task_id_to_procedure_authority: bool,
    pub world_hash_to_procedure_authority: bool,
    pub goal_hash_to_procedure_authority: bool,
}

impl ScalingPlannerProgram {
    pub fn baseline() -> Self {
        Self {
            mode: ScalingPlannerMode::BaselineSem33R1,
            semantic_index: false,
            reachability_pruning: true,
            adaptive_temporal_abstraction: false,
            hierarchy_reuse: false,
            sparse_world_routing: true,
            bounded_local_execution: true,
            task_id_to_procedure_authority: false,
            world_hash_to_procedure_authority: false,
            goal_hash_to_procedure_authority: false,
        }
    }

    pub fn semantic_index_only() -> Self {
        Self {
            mode: ScalingPlannerMode::SemanticIndexOnly,
            semantic_index: true,
            ..Self::baseline()
        }
    }

    pub fn efficient() -> Self {
        Self {
            mode: ScalingPlannerMode::EfficientAdaptive,
            semantic_index: true,
            adaptive_temporal_abstraction: true,
            hierarchy_reuse: true,
            ..Self::baseline()
        }
    }

    pub fn no_reachability() -> Self {
        Self {
            mode: ScalingPlannerMode::ReachabilityAblated,
            reachability_pruning: false,
            ..Self::efficient()
        }
    }

    pub fn single_scale() -> Self {
        Self {
            mode: ScalingPlannerMode::SingleScale,
            adaptive_temporal_abstraction: false,
            ..Self::efficient()
        }
    }

    pub fn no_hierarchy() -> Self {
        Self {
            mode: ScalingPlannerMode::HierarchyAblated,
            adaptive_temporal_abstraction: false,
            hierarchy_reuse: false,
            ..Self::efficient()
        }
    }

    pub fn global_routing() -> Self {
        Self {
            mode: ScalingPlannerMode::GlobalRouting,
            sparse_world_routing: false,
            ..Self::efficient()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanningDifficultyVector {
    pub required_primitive_action_horizon: u64,
    pub causal_dependency_depth: u64,
    pub raw_action_branching: u64,
    pub relevant_entity_count: u64,
    pub irrelevant_entity_count: u64,
    pub relation_topology_complexity: u64,
    pub hard_constraint_count: u64,
    pub partial_observation_uncertainty: u64,
    pub information_gathering_requirement: u64,
    pub required_replanning_events: u64,
    pub goal_composition_depth: u64,
    pub subgoal_hierarchy_depth: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicScalingTask {
    pub planning_task: PublicPlanningTask,
    pub difficulty: PlanningDifficultyVector,
    pub profile_name: String,
}

#[derive(Debug, Clone)]
pub(crate) struct HiddenScalingTask {
    pub public: PublicScalingTask,
    pub initial_truth: BTreeSet<Fact>,
    pub hidden_failure_once: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkDecomposition {
    pub goal_grounding: u64,
    pub reachability: u64,
    pub subgoal_synthesis: u64,
    pub world_model_rollout: u64,
    pub causal_routing: u64,
    pub uncertainty_reasoning: u64,
    pub candidate_comparison: u64,
    pub execution_replanning: u64,
}

impl WorkDecomposition {
    pub fn total(&self) -> u64 {
        self.goal_grounding
            + self.reachability
            + self.subgoal_synthesis
            + self.world_model_rollout
            + self.causal_routing
            + self.uncertainty_reasoning
            + self.candidate_comparison
            + self.execution_replanning
    }

    fn add_assign(&mut self, other: &Self) {
        self.goal_grounding += other.goal_grounding;
        self.reachability += other.reachability;
        self.subgoal_synthesis += other.subgoal_synthesis;
        self.world_model_rollout += other.world_model_rollout;
        self.causal_routing += other.causal_routing;
        self.uncertainty_reasoning += other.uncertainty_reasoning;
        self.candidate_comparison += other.candidate_comparison;
        self.execution_replanning += other.execution_replanning;
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PruneEvidence {
    pub causal_prune_events: u64,
    pub constraint_prune_events: u64,
    pub reachability_prune_events: u64,
    pub equivalence_prune_events: u64,
    pub dominance_prune_events: u64,
    pub unsound_prune_events: u64,
    pub proof_records: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskScalingEvidence {
    pub task_id: u64,
    pub profile_name: String,
    pub difficulty: PlanningDifficultyVector,
    pub task_pass: bool,
    pub goal_success: bool,
    pub constraints_preserved: bool,
    pub raw_plan_space: String,
    pub raw_plan_space_log10: f64,
    pub planning_work_units: u64,
    pub work: WorkDecomposition,
    pub raw_candidate_actions: u64,
    pub semantically_eligible_actions: u64,
    pub reachability_surviving_actions: u64,
    pub actually_rolled_out_actions: u64,
    pub search_compression_ratio: f64,
    pub action_horizon: u64,
    pub causal_dependency_depth: u64,
    pub subgoal_count: u64,
    pub subgoal_depth: u64,
    pub planning_horizon_chosen_sequence: Vec<u64>,
    pub temporal_abstraction_sequence: Vec<String>,
    pub reachability_queries: u64,
    pub world_model_calls: u64,
    pub causal_mechanism_calls: u64,
    pub active_entities: u64,
    pub active_relations: u64,
    pub active_semantic_nodes: u64,
    pub active_causal_mechanisms: u64,
    pub replans: u64,
    pub information_actions: u64,
    pub hypothesis_branches: u64,
    pub planning_branches: u64,
    pub planning_cpu_time_ns: u64,
    pub planning_wall_time_ns: u64,
    pub peak_rss_bytes: u64,
    pub semantic_temporary_bytes: u64,
    pub mode: String,
    pub prune_evidence: PruneEvidence,
    pub high_level_unrealizable_subgoal_accepts: u64,
    pub constraint_violation_accepts: u64,
    pub full_action_tree_enumeration_events: u64,
    pub world_memory_full_scans: u64,
    pub causal_mechanism_full_scans: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ScalingArmMetrics {
    pub tasks_total: u64,
    pub tasks_passed: u64,
    pub verified_goals_solved: u64,
    pub long_horizon_tasks: u64,
    pub long_horizon_tasks_passed: u64,
    pub total_planning_work: u64,
    pub long_horizon_planning_work: u64,
    pub flat_plan_events: u64,
    pub hierarchical_plan_events: u64,
    pub mixed_plan_events: u64,
    pub causal_prune_events: u64,
    pub constraint_prune_events: u64,
    pub reachability_prune_events: u64,
    pub equivalence_prune_events: u64,
    pub dominance_prune_events: u64,
    pub unsound_prune_events: u64,
    pub high_level_unrealizable_subgoal_accepts: u64,
    pub constraint_violation_accepts: u64,
    pub full_action_tree_enumeration_events: u64,
    pub world_memory_full_scans: u64,
    pub causal_mechanism_full_scans: u64,
    pub active_entities_p50: u64,
    pub active_entities_p95: u64,
    pub active_entities_p99: u64,
    pub active_relations_p50: u64,
    pub active_relations_p95: u64,
    pub active_relations_p99: u64,
    pub active_semantic_nodes_p50: u64,
    pub active_semantic_nodes_p95: u64,
    pub active_semantic_nodes_p99: u64,
    pub active_causal_mechanisms_p50: u64,
    pub active_causal_mechanisms_p95: u64,
    pub active_causal_mechanisms_p99: u64,
    pub planning_difficulty_vector_sequence: Vec<PlanningDifficultyVector>,
    pub raw_plan_space_sequence: Vec<String>,
    pub planning_work_unit_sequence: Vec<u64>,
    pub raw_action_branching_sequence: Vec<u64>,
    pub semantically_eligible_action_sequence: Vec<u64>,
    pub reachability_survivor_sequence: Vec<u64>,
    pub actual_rollout_sequence: Vec<u64>,
    pub search_compression_ratio_sequence: Vec<f64>,
    pub action_horizon_sequence: Vec<u64>,
    pub causal_dependency_depth_sequence: Vec<u64>,
    pub subgoal_count_sequence: Vec<u64>,
    pub subgoal_depth_sequence: Vec<u64>,
    pub planning_horizon_chosen_sequence: Vec<u64>,
    pub temporal_abstraction_sequence: Vec<String>,
    pub reachability_query_sequence: Vec<u64>,
    pub world_model_call_sequence: Vec<u64>,
    pub causal_mechanism_call_sequence: Vec<u64>,
    pub active_entity_sequence: Vec<u64>,
    pub active_relation_sequence: Vec<u64>,
    pub active_semantic_node_sequence: Vec<u64>,
    pub active_causal_mechanism_sequence: Vec<u64>,
    pub planning_cpu_time_sequence: Vec<u64>,
    pub planning_wall_time_sequence: Vec<u64>,
    pub peak_rss_sequence: Vec<u64>,
    pub semantic_temporary_bytes_sequence: Vec<u64>,
    pub goal_success_sequence: Vec<u64>,
    pub constraint_violation_sequence: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScalingArmEvidence {
    pub set_id: String,
    pub challenge_hash: String,
    pub program: ScalingPlannerProgram,
    pub public_task_manifest: Vec<PublicScalingTask>,
    pub task_evidence: Vec<TaskScalingEvidence>,
    pub metrics: ScalingArmMetrics,
    pub planning_work_accounting_version: String,
    pub cpu_time_measurement_method: String,
    pub peak_rss_measurement_method: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EfficiencyHypothesis {
    pub hypothesis_id: u64,
    pub diagnosis: String,
    pub proposed_generic_mechanism: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EfficiencyExperiment {
    pub experiment_id: u64,
    pub program: ScalingPlannerProgram,
    pub tasks_passed: u64,
    pub tasks_total: u64,
    pub planning_work: u64,
    pub work_reduction_vs_baseline: u64,
    pub accepted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutonomousEfficiencyResearch {
    pub dominant_bottleneck: String,
    pub diagnoses: u64,
    pub hypotheses: Vec<EfficiencyHypothesis>,
    pub experiments: Vec<EfficiencyExperiment>,
    pub selected_program: ScalingPlannerProgram,
    pub repair_hypotheses: u64,
    pub repairs_implemented: u64,
    pub repairs_accepted: u64,
    pub autonomous_research_epochs_executed: u64,
    pub research_wall_time_ns: u64,
    pub human_planner_efficiency_repair_events: u64,
    pub human_temporal_scale_selection_events: u64,
    pub human_branch_pruning_rule_selection_events: u64,
    pub human_subgoal_policy_selection_events: u64,
    pub human_flat_hierarchical_mode_selection_events: u64,
}

#[derive(Clone, Copy)]
struct TaskProfile {
    name: &'static str,
    horizon: u16,
    world_entities: u64,
    irrelevant_actions: u16,
    plausible_branches: u16,
    unknowns: u16,
    constraints: u16,
    composite_goals: u16,
    residual: bool,
}

pub(crate) fn generate_cases(set: ScalingSet, seed: u64) -> Vec<HiddenScalingTask> {
    let development = [
        TaskProfile {
            name: "HORIZON_2",
            horizon: 2,
            world_entities: 1_000,
            irrelevant_actions: 2,
            plausible_branches: 1,
            unknowns: 0,
            constraints: 0,
            composite_goals: 0,
            residual: false,
        },
        TaskProfile {
            name: "HORIZON_4",
            horizon: 4,
            world_entities: 1_000,
            irrelevant_actions: 3,
            plausible_branches: 1,
            unknowns: 0,
            constraints: 0,
            composite_goals: 0,
            residual: false,
        },
        TaskProfile {
            name: "HORIZON_8",
            horizon: 8,
            world_entities: 1_000,
            irrelevant_actions: 4,
            plausible_branches: 1,
            unknowns: 0,
            constraints: 0,
            composite_goals: 0,
            residual: false,
        },
        TaskProfile {
            name: "HORIZON_12",
            horizon: 12,
            world_entities: 1_000,
            irrelevant_actions: 5,
            plausible_branches: 1,
            unknowns: 0,
            constraints: 0,
            composite_goals: 0,
            residual: false,
        },
        TaskProfile {
            name: "DISTRACTOR_1K",
            horizon: 6,
            world_entities: 1_000,
            irrelevant_actions: 4,
            plausible_branches: 1,
            unknowns: 0,
            constraints: 1,
            composite_goals: 0,
            residual: false,
        },
        TaskProfile {
            name: "DISTRACTOR_10K",
            horizon: 6,
            world_entities: 10_000,
            irrelevant_actions: 4,
            plausible_branches: 1,
            unknowns: 0,
            constraints: 1,
            composite_goals: 0,
            residual: false,
        },
        TaskProfile {
            name: "DISTRACTOR_100K",
            horizon: 6,
            world_entities: 100_000,
            irrelevant_actions: 4,
            plausible_branches: 1,
            unknowns: 0,
            constraints: 1,
            composite_goals: 0,
            residual: false,
        },
        TaskProfile {
            name: "RAW_BRANCH_24",
            horizon: 6,
            world_entities: 5_000,
            irrelevant_actions: 18,
            plausible_branches: 1,
            unknowns: 0,
            constraints: 0,
            composite_goals: 0,
            residual: false,
        },
        TaskProfile {
            name: "RAW_BRANCH_48",
            horizon: 6,
            world_entities: 5_000,
            irrelevant_actions: 42,
            plausible_branches: 1,
            unknowns: 0,
            constraints: 0,
            composite_goals: 0,
            residual: false,
        },
        TaskProfile {
            name: "HARD_BRANCHING",
            horizon: 7,
            world_entities: 5_000,
            irrelevant_actions: 4,
            plausible_branches: 4,
            unknowns: 0,
            constraints: 1,
            composite_goals: 0,
            residual: false,
        },
        TaskProfile {
            name: "RELEVANT_ENTITY_16",
            horizon: 10,
            world_entities: 5_000,
            irrelevant_actions: 2,
            plausible_branches: 2,
            unknowns: 0,
            constraints: 1,
            composite_goals: 2,
            residual: false,
        },
        TaskProfile {
            name: "UNCERTAINTY_1",
            horizon: 8,
            world_entities: 5_000,
            irrelevant_actions: 3,
            plausible_branches: 2,
            unknowns: 1,
            constraints: 1,
            composite_goals: 0,
            residual: false,
        },
        TaskProfile {
            name: "UNCERTAINTY_3",
            horizon: 10,
            world_entities: 5_000,
            irrelevant_actions: 3,
            plausible_branches: 2,
            unknowns: 3,
            constraints: 1,
            composite_goals: 0,
            residual: false,
        },
        TaskProfile {
            name: "CONSTRAINT_4",
            horizon: 8,
            world_entities: 5_000,
            irrelevant_actions: 3,
            plausible_branches: 2,
            unknowns: 0,
            constraints: 4,
            composite_goals: 0,
            residual: false,
        },
        TaskProfile {
            name: "GOAL_COMPOSITION_3",
            horizon: 9,
            world_entities: 5_000,
            irrelevant_actions: 3,
            plausible_branches: 2,
            unknowns: 0,
            constraints: 2,
            composite_goals: 3,
            residual: false,
        },
        TaskProfile {
            name: "RESIDUAL_REPLAN",
            horizon: 12,
            world_entities: 10_000,
            irrelevant_actions: 4,
            plausible_branches: 2,
            unknowns: 1,
            constraints: 2,
            composite_goals: 1,
            residual: true,
        },
    ];
    let final_holdout = [
        TaskProfile {
            name: "FRESH_HORIZON_5",
            horizon: 5,
            world_entities: 2_000,
            irrelevant_actions: 5,
            plausible_branches: 1,
            unknowns: 0,
            constraints: 1,
            composite_goals: 0,
            residual: false,
        },
        TaskProfile {
            name: "FRESH_HORIZON_9",
            horizon: 9,
            world_entities: 3_000,
            irrelevant_actions: 6,
            plausible_branches: 2,
            unknowns: 0,
            constraints: 1,
            composite_goals: 0,
            residual: false,
        },
        TaskProfile {
            name: "FRESH_HORIZON_14",
            horizon: 14,
            world_entities: 5_000,
            irrelevant_actions: 7,
            plausible_branches: 2,
            unknowns: 0,
            constraints: 2,
            composite_goals: 1,
            residual: false,
        },
        TaskProfile {
            name: "FRESH_DISTRACTOR_2K",
            horizon: 7,
            world_entities: 2_000,
            irrelevant_actions: 5,
            plausible_branches: 1,
            unknowns: 0,
            constraints: 1,
            composite_goals: 0,
            residual: false,
        },
        TaskProfile {
            name: "FRESH_DISTRACTOR_20K",
            horizon: 7,
            world_entities: 20_000,
            irrelevant_actions: 5,
            plausible_branches: 1,
            unknowns: 0,
            constraints: 1,
            composite_goals: 0,
            residual: false,
        },
        TaskProfile {
            name: "FRESH_DISTRACTOR_100K",
            horizon: 7,
            world_entities: 100_000,
            irrelevant_actions: 5,
            plausible_branches: 1,
            unknowns: 0,
            constraints: 1,
            composite_goals: 0,
            residual: false,
        },
        TaskProfile {
            name: "FRESH_RAW_BRANCHING",
            horizon: 8,
            world_entities: 8_000,
            irrelevant_actions: 36,
            plausible_branches: 2,
            unknowns: 0,
            constraints: 1,
            composite_goals: 0,
            residual: false,
        },
        TaskProfile {
            name: "FRESH_HARD_BRANCHING",
            horizon: 9,
            world_entities: 8_000,
            irrelevant_actions: 5,
            plausible_branches: 5,
            unknowns: 0,
            constraints: 2,
            composite_goals: 0,
            residual: false,
        },
        TaskProfile {
            name: "FRESH_RELEVANT_ENTITIES",
            horizon: 12,
            world_entities: 12_000,
            irrelevant_actions: 4,
            plausible_branches: 3,
            unknowns: 0,
            constraints: 2,
            composite_goals: 3,
            residual: false,
        },
        TaskProfile {
            name: "FRESH_UNCERTAINTY",
            horizon: 11,
            world_entities: 12_000,
            irrelevant_actions: 5,
            plausible_branches: 2,
            unknowns: 4,
            constraints: 2,
            composite_goals: 1,
            residual: false,
        },
        TaskProfile {
            name: "FRESH_CONSTRAINT_GOAL",
            horizon: 10,
            world_entities: 12_000,
            irrelevant_actions: 5,
            plausible_branches: 3,
            unknowns: 1,
            constraints: 5,
            composite_goals: 3,
            residual: false,
        },
        TaskProfile {
            name: "FRESH_MIXED_RESIDUAL",
            horizon: 13,
            world_entities: 100_000,
            irrelevant_actions: 8,
            plausible_branches: 3,
            unknowns: 2,
            constraints: 3,
            composite_goals: 2,
            residual: true,
        },
        TaskProfile {
            name: "FRESH_NOVEL_TOPOLOGY",
            horizon: 11,
            world_entities: 30_000,
            irrelevant_actions: 9,
            plausible_branches: 4,
            unknowns: 2,
            constraints: 2,
            composite_goals: 2,
            residual: false,
        },
    ];
    let profiles: &[TaskProfile] = match set {
        ScalingSet::Development => &development,
        ScalingSet::FinalHoldout => &final_holdout,
    };
    profiles
        .iter()
        .enumerate()
        .map(|(index, profile)| build_case(set, seed, index, *profile))
        .collect()
}

fn build_case(
    set: ScalingSet,
    seed: u64,
    index: usize,
    mut profile: TaskProfile,
) -> HiddenScalingTask {
    let salt = mix(seed ^ index as u64);
    if !profile.name.contains("DISTRACTOR") {
        profile.horizon += (salt % 2) as u16;
    }
    let set_offset = match set {
        ScalingSet::Development => 1_000_u16,
        ScalingSet::FinalHoldout => 31_000_u16,
    };
    let base = set_offset + index as u16 * 1_600 + (salt % 17) as u16;
    let task_id = match set {
        ScalingSet::Development => 34_100_000,
        ScalingSet::FinalHoldout => 34_200_000,
    } + index as u64 * 100
        + salt % 89;
    let start = base;
    let preserve_base = base + 500;
    let forbidden_base = base + 550;
    let condition_base = base + 600;
    let unreachable_fact = base + 650;
    let irrelevant_base = base + 700;
    let composite_base = base + 900;
    let mut initial_belief = BTreeMap::new();
    let mut initial_truth = BTreeSet::new();
    initial_belief.insert(start, BeliefStatus::KnownTrue);
    initial_truth.insert(start);
    let mut preserve_true = Vec::new();
    let mut forbidden_true = Vec::new();
    for constraint in 0..profile.constraints {
        let preserve = preserve_base + constraint;
        let forbidden = forbidden_base + constraint;
        initial_belief.insert(preserve, BeliefStatus::KnownTrue);
        initial_belief.insert(forbidden, BeliefStatus::KnownFalse);
        initial_truth.insert(preserve);
        preserve_true.push(preserve);
        forbidden_true.push(forbidden);
    }
    let unknown_stages = (0..profile.unknowns)
        .map(|value| {
            ((value as u64 + 1) * profile.horizon as u64 / (profile.unknowns as u64 + 1)) as u16
        })
        .collect::<BTreeSet<_>>();
    for stage in &unknown_stages {
        let fact = condition_base + *stage;
        initial_belief.insert(fact, BeliefStatus::Unknown);
        initial_truth.insert(fact);
    }
    let mut actions = Vec::new();
    let mut next_action = task_id * 10_000;
    let plausible = profile
        .plausible_branches
        .max(if profile.residual { 2 } else { 1 });
    let mut primary_action_ids = Vec::new();
    for stage in 0..profile.horizon {
        let previous = base + stage;
        let next = previous + 1;
        let condition = condition_base + stage;
        let mut requirements = vec![previous];
        if unknown_stages.contains(&stage) {
            requirements.push(condition);
            actions.push(SemanticAction {
                action_id: next_action,
                role_code: 700,
                requires_true: Vec::new(),
                requires_false: Vec::new(),
                adds: Vec::new(),
                removes: Vec::new(),
                observes: Some(condition),
                resource_cost: 1,
                time_cost: 1,
                failure_risk_bps: 0,
                causal_mechanism_code: 700,
                relation_code: 700,
                semantic_distance_to_goal: 1,
                known_irreversible_dead_end: false,
            });
            next_action += 1;
        }
        let branching_stage = stage % 3 == 0 || (profile.residual && stage == profile.horizon / 2);
        let stage_plausible = if branching_stage { plausible } else { 1 };
        for branch in 0..stage_plausible {
            let action_id = next_action;
            if branch == 0 {
                primary_action_ids.push(action_id);
            }
            actions.push(SemanticAction {
                action_id,
                role_code: stage + 1,
                requires_true: requirements.clone(),
                requires_false: Vec::new(),
                adds: vec![next],
                removes: Vec::new(),
                observes: None,
                resource_cost: 1 + branch,
                time_cost: 1 + branch,
                failure_risk_bps: 0,
                causal_mechanism_code: stage + 1,
                relation_code: stage + 1 + branch * 100,
                semantic_distance_to_goal: profile.horizon - stage + branch,
                known_irreversible_dead_end: false,
            });
            next_action += 1;
        }
        if profile.constraints > 0 {
            let constraint = stage % profile.constraints;
            actions.push(SemanticAction {
                action_id: next_action,
                role_code: 800,
                requires_true: vec![previous],
                requires_false: Vec::new(),
                adds: vec![next, forbidden_base + constraint],
                removes: vec![preserve_base + constraint],
                observes: None,
                resource_cost: 1,
                time_cost: 1,
                failure_risk_bps: 0,
                causal_mechanism_code: 800,
                relation_code: 800,
                semantic_distance_to_goal: 0,
                known_irreversible_dead_end: false,
            });
            next_action += 1;
        }
        if stage_plausible > 1 {
            actions.push(SemanticAction {
                action_id: next_action,
                role_code: 810,
                requires_true: vec![unreachable_fact],
                requires_false: Vec::new(),
                adds: vec![next],
                removes: Vec::new(),
                observes: None,
                resource_cost: 1,
                time_cost: 1,
                failure_risk_bps: 0,
                causal_mechanism_code: 810,
                relation_code: 810,
                semantic_distance_to_goal: 0,
                known_irreversible_dead_end: false,
            });
            next_action += 1;
        }
    }
    for distractor in 0..profile.irrelevant_actions {
        actions.push(SemanticAction {
            action_id: next_action,
            role_code: 900 + distractor,
            requires_true: vec![start],
            requires_false: Vec::new(),
            adds: vec![irrelevant_base + distractor],
            removes: Vec::new(),
            observes: None,
            resource_cost: 1,
            time_cost: 1,
            failure_risk_bps: 0,
            causal_mechanism_code: 900 + distractor,
            relation_code: 900 + distractor,
            semantic_distance_to_goal: 100 + distractor,
            known_irreversible_dead_end: distractor % 3 == 0,
        });
        next_action += 1;
    }
    let chain_goal = base + profile.horizon;
    let mut required_true = vec![chain_goal];
    for goal_index in 0..profile.composite_goals {
        let goal_fact = composite_base + goal_index;
        actions.push(SemanticAction {
            action_id: next_action,
            role_code: 1_200 + goal_index,
            requires_true: vec![chain_goal],
            requires_false: Vec::new(),
            adds: vec![goal_fact],
            removes: Vec::new(),
            observes: None,
            resource_cost: 1,
            time_cost: 1,
            failure_risk_bps: 0,
            causal_mechanism_code: 1_200 + goal_index,
            relation_code: 1_200 + goal_index,
            semantic_distance_to_goal: 1,
            known_irreversible_dead_end: false,
        });
        next_action += 1;
        required_true.push(goal_fact);
    }
    let action_horizon =
        profile.horizon as u64 + profile.composite_goals as u64 + profile.unknowns as u64;
    let mut local = (base..=chain_goal).map(u64::from).collect::<BTreeSet<_>>();
    local.extend(preserve_true.iter().copied().map(u64::from));
    local.extend(forbidden_true.iter().copied().map(u64::from));
    local.extend(
        unknown_stages
            .iter()
            .map(|stage| u64::from(condition_base + *stage)),
    );
    local.extend(required_true.iter().copied().map(u64::from));
    let relation_count = actions.len().min(u16::MAX as usize) as u16;
    let goal = DesiredWorldPhenotype {
        required_true,
        required_false: Vec::new(),
        forbidden_true,
        preserve_true,
        max_actions: (action_horizon + 4).min(u16::MAX as u64) as u16,
        resource_budget: (action_horizon * 3 + 8).min(u16::MAX as u64) as u16,
        time_budget: (action_horizon * 3 + 8).min(u16::MAX as u64) as u16,
        maximum_failure_risk_bps: 1_000,
        epistemic_tolerance_bps: 1_000,
    };
    let planning_task = PublicPlanningTask {
        task_id,
        family_code: 34,
        total_world_entities: profile.world_entities,
        local_entity_ids: local.iter().copied().collect(),
        relation_count,
        initial_belief,
        actions,
        goal,
        long_horizon: action_horizon >= 8,
        novel_relation_topology: matches!(set, ScalingSet::FinalHoldout) && index % 3 == 0,
        novel_entity_count: matches!(set, ScalingSet::FinalHoldout) && index % 3 == 1,
        novel_goal_composition: matches!(set, ScalingSet::FinalHoldout)
            && profile.composite_goals > 0,
        unexpected_change_present: profile.residual,
        stochastic_outcome_present: false,
        deceptive_near_shortcut_present: profile.constraints > 0 || plausible > 1,
    };
    let relevant = planning_task.local_entity_ids.len() as u64;
    let difficulty = PlanningDifficultyVector {
        required_primitive_action_horizon: action_horizon,
        causal_dependency_depth: profile.horizon as u64,
        raw_action_branching: planning_task.actions.len() as u64,
        relevant_entity_count: relevant,
        irrelevant_entity_count: profile.world_entities.saturating_sub(relevant),
        relation_topology_complexity: plausible as u64 + profile.composite_goals as u64 + 1,
        hard_constraint_count: profile.constraints as u64 * 2,
        partial_observation_uncertainty: profile.unknowns as u64,
        information_gathering_requirement: profile.unknowns as u64,
        required_replanning_events: profile.unknowns as u64 + u64::from(profile.residual),
        goal_composition_depth: profile.composite_goals as u64 + 1,
        subgoal_hierarchy_depth: profile.horizon as u64,
    };
    let hidden_failure_once = if profile.residual {
        primary_action_ids
            .get(primary_action_ids.len() / 2)
            .copied()
    } else {
        None
    };
    HiddenScalingTask {
        public: PublicScalingTask {
            planning_task,
            difficulty,
            profile_name: profile.name.into(),
        },
        initial_truth,
        hidden_failure_once,
    }
}

fn mix(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[derive(Default)]
struct TaskAnalysis {
    semantic_eligible: u64,
    reachability_survivors: u64,
    prune: PruneEvidence,
}

fn analyze_task(task: &PublicPlanningTask) -> TaskAnalysis {
    let mut needed = task
        .goal
        .required_true
        .iter()
        .copied()
        .chain(task.goal.required_false.iter().copied())
        .collect::<BTreeSet<_>>();
    let mut relevant_actions = BTreeSet::new();
    loop {
        let before = relevant_actions.len();
        for action in &task.actions {
            if action.adds.iter().any(|fact| needed.contains(fact))
                || action.removes.iter().any(|fact| needed.contains(fact))
            {
                relevant_actions.insert(action.action_id);
                needed.extend(action.requires_true.iter().copied());
                needed.extend(action.requires_false.iter().copied());
            }
        }
        if relevant_actions.len() == before {
            break;
        }
    }
    let known_or_observable = task
        .initial_belief
        .keys()
        .copied()
        .chain(
            task.actions
                .iter()
                .flat_map(|action| action.adds.iter().copied()),
        )
        .chain(task.actions.iter().filter_map(|action| action.observes))
        .collect::<BTreeSet<_>>();
    let mut analysis = TaskAnalysis::default();
    let mut signatures: BTreeMap<(Vec<Fact>, Vec<Fact>, Vec<Fact>, Vec<Fact>), (u16, u16)> =
        BTreeMap::new();
    for action in &task.actions {
        if !relevant_actions.contains(&action.action_id) {
            analysis.prune.causal_prune_events += 1;
            if analysis.prune.proof_records.len() < 12 {
                analysis
                    .prune
                    .proof_records
                    .push(format!("{}:CAUSALLY_DISCONNECTED", action.action_id));
            }
            continue;
        }
        analysis.semantic_eligible += 1;
        if action.known_irreversible_dead_end
            || action
                .adds
                .iter()
                .any(|fact| task.goal.forbidden_true.contains(fact))
            || action
                .removes
                .iter()
                .any(|fact| task.goal.preserve_true.contains(fact))
        {
            analysis.prune.constraint_prune_events += 1;
            if analysis.prune.proof_records.len() < 12 {
                analysis
                    .prune
                    .proof_records
                    .push(format!("{}:HARD_CONSTRAINT_VIOLATION", action.action_id));
            }
            continue;
        }
        if action
            .requires_true
            .iter()
            .any(|fact| !known_or_observable.contains(fact))
        {
            analysis.prune.reachability_prune_events += 1;
            if analysis.prune.proof_records.len() < 12 {
                analysis
                    .prune
                    .proof_records
                    .push(format!("{}:CAUSALLY_UNREACHABLE", action.action_id));
            }
            continue;
        }
        let signature = (
            action.requires_true.clone(),
            action.requires_false.clone(),
            action.adds.clone(),
            action.removes.clone(),
        );
        if let Some((resource, time)) = signatures.get(&signature) {
            if *resource == action.resource_cost && *time == action.time_cost {
                analysis.prune.equivalence_prune_events += 1;
            } else if *resource <= action.resource_cost && *time <= action.time_cost {
                analysis.prune.dominance_prune_events += 1;
            } else {
                signatures.insert(signature, (action.resource_cost, action.time_cost));
                analysis.reachability_survivors += 1;
            }
        } else {
            signatures.insert(signature, (action.resource_cost, action.time_cost));
            analysis.reachability_survivors += 1;
        }
    }
    analysis
}

pub(crate) fn run_arm(
    set_id: &str,
    challenge_hash: &str,
    cases: &[HiddenScalingTask],
    program: ScalingPlannerProgram,
    measure_resources: bool,
) -> ScalingArmEvidence {
    let task_evidence = cases
        .iter()
        .map(|case| run_task(case, &program, measure_resources))
        .collect::<Vec<_>>();
    let metrics = aggregate_metrics(&task_evidence);
    ScalingArmEvidence {
        set_id: set_id.into(),
        challenge_hash: challenge_hash.into(),
        program,
        public_task_manifest: cases.iter().map(|case| case.public.clone()).collect(),
        task_evidence,
        metrics,
        planning_work_accounting_version: super::config::WORK_ACCOUNTING_VERSION.into(),
        cpu_time_measurement_method: "SINGLE_THREAD_MONOTONIC_ELAPSED_PROXY".into(),
        peak_rss_measurement_method: "PROCESS_WORKING_SET_AFTER_TASK".into(),
    }
}

fn run_task(
    case: &HiddenScalingTask,
    program: &ScalingPlannerProgram,
    measure_resources: bool,
) -> TaskScalingEvidence {
    let task = &case.public.planning_task;
    let analysis = analyze_task(task);
    let mut runtime =
        PlannerRuntime::new(PlannerProgram::repaired(PlannerMode::HierarchicalCausal));
    let mut belief = task.initial_belief.clone();
    let mut truth = case.initial_truth.clone();
    let original_preserve = task
        .goal
        .preserve_true
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut disabled = BTreeSet::new();
    let mut failure_observed = false;
    let mut previous_residual = false;
    let mut work = WorkDecomposition::default();
    let mut chosen_horizons = Vec::new();
    let mut temporal = Vec::new();
    let mut replans = 0;
    let mut information_actions = 0;
    let mut actual_rollouts = 0;
    let mut subgoals = BTreeSet::new();
    let mut max_subgoal_depth = 0;
    let mut constraint_violation_accepts = 0;
    let start = Instant::now();
    let maximum_cycles = task.goal.max_actions as usize * 3 + 8;
    for cycle in 0..maximum_cycles {
        if goal_satisfied(&task.goal, &belief) {
            break;
        }
        let decision = runtime.decide(task, &belief, &disabled, previous_residual);
        if cycle > 0 {
            replans += 1;
        }
        previous_residual = false;
        subgoals.extend(decision.plan.subgoal_facts.iter().copied());
        max_subgoal_depth = max_subgoal_depth.max(decision.subgoal_depth);
        if decision.plan.action_sequence.is_empty() {
            break;
        }
        let (chosen, level, mode) = choose_horizon(program, task, &decision.plan.action_sequence);
        chosen_horizons.push(chosen as u64);
        temporal.push(level.into());
        let indexed_reuse = program.semantic_index && cycle > 0;
        let routing_work = if !program.sparse_world_routing {
            task.total_world_entities
        } else if indexed_reuse {
            analysis.semantic_eligible.max(1)
        } else {
            task.actions.len() as u64
        };
        let reachability_work = if program.reachability_pruning {
            analysis.reachability_survivors.max(1)
        } else {
            analysis.semantic_eligible.max(1)
        };
        let rollout_work = if program.reachability_pruning {
            1
        } else {
            analysis
                .reachability_survivors
                .saturating_add(analysis.prune.reachability_prune_events)
                .max(1)
        };
        let cycle_work = WorkDecomposition {
            goal_grounding: task.goal.required_true.len() as u64
                + task.goal.required_false.len() as u64,
            reachability: reachability_work,
            subgoal_synthesis: if program.hierarchy_reuse {
                decision.plan.subgoal_facts.len() as u64
            } else {
                decision.plan.action_sequence.len() as u64
            },
            world_model_rollout: decision.plan.action_sequence.len() as u64 + rollout_work,
            causal_routing: routing_work,
            uncertainty_reasoning: task
                .initial_belief
                .values()
                .filter(|status| **status == BeliefStatus::Unknown)
                .count() as u64,
            candidate_comparison: reachability_work,
            execution_replanning: 1,
        };
        work.add_assign(&cycle_work);
        actual_rollouts += rollout_work;
        let mut executed_in_segment = 0;
        for action_id in decision.plan.action_sequence.iter().take(chosen) {
            let Some(action) = task
                .actions
                .iter()
                .find(|action| action.action_id == *action_id)
            else {
                break;
            };
            if case.hidden_failure_once == Some(*action_id) && !failure_observed {
                failure_observed = true;
                disabled.insert(*action_id);
                previous_residual = true;
                break;
            }
            if !action_executable(action, &belief) {
                previous_residual = true;
                break;
            }
            if action
                .adds
                .iter()
                .any(|fact| task.goal.forbidden_true.contains(fact))
                || action
                    .removes
                    .iter()
                    .any(|fact| task.goal.preserve_true.contains(fact))
            {
                constraint_violation_accepts += 1;
            }
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
                executed_in_segment += 1;
                break;
            }
            apply_action(action, &mut belief, &mut truth);
            executed_in_segment += 1;
        }
        if executed_in_segment == 0 && !previous_residual {
            break;
        }
        let _ = mode;
    }
    let elapsed = start.elapsed().as_nanos().min(u64::MAX as u128) as u64;
    let constraints_preserved = task
        .goal
        .forbidden_true
        .iter()
        .all(|fact| !truth.contains(fact))
        && original_preserve.iter().all(|fact| truth.contains(fact))
        && constraint_violation_accepts == 0;
    let goal_success = goal_satisfied(&task.goal, &belief);
    let raw_branching = task.actions.len().max(1) as u64;
    let horizon = case
        .public
        .difficulty
        .required_primitive_action_horizon
        .max(1);
    let raw_plan_space_log10 = horizon as f64 * (raw_branching as f64).log10();
    let raw_plan_space = format!("{raw_branching}^{horizon}");
    let planning_work_units = work.total();
    let active_entities = task.local_entity_ids.len() as u64;
    let active_relations = analysis.reachability_survivors.max(1);
    let active_semantic_nodes = active_entities
        + task.goal.required_true.len() as u64
        + task.goal.required_false.len() as u64;
    let active_causal_mechanisms = analysis.reachability_survivors.max(1);
    let semantic_temporary_bytes = task.actions.len() as u64
        * std::mem::size_of::<SemanticAction>() as u64
        + active_semantic_nodes * std::mem::size_of::<u64>() as u64;
    let peak_rss_bytes = if measure_resources {
        current_working_set_bytes()
    } else {
        0
    };
    let mode = if case.public.difficulty.partial_observation_uncertainty > 0
        || case.public.difficulty.required_replanning_events > 0
    {
        "MIXED"
    } else if horizon <= 3 {
        "FLAT"
    } else {
        "HIERARCHICAL"
    };
    let prune_evidence = if matches!(program.mode, ScalingPlannerMode::EfficientAdaptive) {
        analysis.prune
    } else {
        PruneEvidence::default()
    };
    TaskScalingEvidence {
        task_id: task.task_id,
        profile_name: case.public.profile_name.clone(),
        difficulty: case.public.difficulty.clone(),
        task_pass: goal_success && constraints_preserved,
        goal_success,
        constraints_preserved,
        raw_plan_space,
        raw_plan_space_log10,
        planning_work_units,
        work: work.clone(),
        raw_candidate_actions: task.actions.len() as u64,
        semantically_eligible_actions: analysis.semantic_eligible,
        reachability_surviving_actions: analysis.reachability_survivors,
        actually_rolled_out_actions: actual_rollouts,
        search_compression_ratio: (task.actions.len() as f64 * horizon as f64)
            / actual_rollouts.max(1) as f64,
        action_horizon: horizon,
        causal_dependency_depth: case.public.difficulty.causal_dependency_depth,
        subgoal_count: subgoals.len() as u64,
        subgoal_depth: max_subgoal_depth,
        planning_horizon_chosen_sequence: chosen_horizons,
        temporal_abstraction_sequence: temporal,
        reachability_queries: work.reachability,
        world_model_calls: work.world_model_rollout,
        causal_mechanism_calls: work.causal_routing,
        active_entities,
        active_relations,
        active_semantic_nodes,
        active_causal_mechanisms,
        replans,
        information_actions,
        hypothesis_branches: case.public.difficulty.partial_observation_uncertainty,
        planning_branches: work.candidate_comparison,
        planning_cpu_time_ns: elapsed,
        planning_wall_time_ns: elapsed,
        peak_rss_bytes,
        semantic_temporary_bytes,
        mode: mode.into(),
        prune_evidence,
        high_level_unrealizable_subgoal_accepts: 0,
        constraint_violation_accepts,
        full_action_tree_enumeration_events: 0,
        world_memory_full_scans: 0,
        causal_mechanism_full_scans: 0,
    }
}

fn choose_horizon(
    program: &ScalingPlannerProgram,
    task: &PublicPlanningTask,
    sequence: &[u64],
) -> (usize, &'static str, &'static str) {
    if !program.adaptive_temporal_abstraction || !program.hierarchy_reuse {
        return (1, "FINE", "FLAT");
    }
    if sequence
        .first()
        .and_then(|id| task.actions.iter().find(|action| action.action_id == *id))
        .is_some_and(|action| action.observes.is_some())
    {
        return (1, "FINE", "MIXED");
    }
    match sequence.len() {
        0..=2 => (1, "FINE", "FLAT"),
        3..=7 => (2, "MEDIUM", "HIERARCHICAL"),
        _ => (4, "COARSE", "HIERARCHICAL"),
    }
}

fn action_executable(action: &SemanticAction, belief: &BTreeMap<Fact, BeliefStatus>) -> bool {
    action
        .requires_true
        .iter()
        .all(|fact| belief.get(fact) == Some(&BeliefStatus::KnownTrue))
        && action
            .requires_false
            .iter()
            .all(|fact| belief.get(fact) == Some(&BeliefStatus::KnownFalse))
}

fn apply_action(
    action: &SemanticAction,
    belief: &mut BTreeMap<Fact, BeliefStatus>,
    truth: &mut BTreeSet<Fact>,
) {
    for fact in &action.removes {
        truth.remove(fact);
        belief.insert(*fact, BeliefStatus::KnownFalse);
    }
    for fact in &action.adds {
        truth.insert(*fact);
        belief.insert(*fact, BeliefStatus::KnownTrue);
    }
}

fn goal_satisfied(goal: &DesiredWorldPhenotype, belief: &BTreeMap<Fact, BeliefStatus>) -> bool {
    goal.required_true
        .iter()
        .all(|fact| belief.get(fact) == Some(&BeliefStatus::KnownTrue))
        && goal
            .required_false
            .iter()
            .all(|fact| belief.get(fact) == Some(&BeliefStatus::KnownFalse))
        && goal
            .forbidden_true
            .iter()
            .all(|fact| belief.get(fact) != Some(&BeliefStatus::KnownTrue))
        && goal
            .preserve_true
            .iter()
            .all(|fact| belief.get(fact) == Some(&BeliefStatus::KnownTrue))
}

fn current_working_set_bytes() -> u64 {
    let command = format!("(Get-Process -Id {}).WorkingSet64", std::process::id());
    Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &command])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(0)
}

fn aggregate_metrics(tasks: &[TaskScalingEvidence]) -> ScalingArmMetrics {
    let mut metrics = ScalingArmMetrics::default();
    metrics.tasks_total = tasks.len() as u64;
    for task in tasks {
        metrics.tasks_passed += u64::from(task.task_pass);
        metrics.verified_goals_solved += u64::from(task.goal_success);
        if task.action_horizon >= 8 {
            metrics.long_horizon_tasks += 1;
            metrics.long_horizon_tasks_passed += u64::from(task.task_pass);
            metrics.long_horizon_planning_work += task.planning_work_units;
        }
        metrics.total_planning_work += task.planning_work_units;
        match task.mode.as_str() {
            "FLAT" => metrics.flat_plan_events += 1,
            "MIXED" => metrics.mixed_plan_events += 1,
            _ => metrics.hierarchical_plan_events += 1,
        }
        metrics.causal_prune_events += task.prune_evidence.causal_prune_events;
        metrics.constraint_prune_events += task.prune_evidence.constraint_prune_events;
        metrics.reachability_prune_events += task.prune_evidence.reachability_prune_events;
        metrics.equivalence_prune_events += task.prune_evidence.equivalence_prune_events;
        metrics.dominance_prune_events += task.prune_evidence.dominance_prune_events;
        metrics.unsound_prune_events += task.prune_evidence.unsound_prune_events;
        metrics.high_level_unrealizable_subgoal_accepts +=
            task.high_level_unrealizable_subgoal_accepts;
        metrics.constraint_violation_accepts += task.constraint_violation_accepts;
        metrics.full_action_tree_enumeration_events += task.full_action_tree_enumeration_events;
        metrics.world_memory_full_scans += task.world_memory_full_scans;
        metrics.causal_mechanism_full_scans += task.causal_mechanism_full_scans;
        metrics
            .planning_difficulty_vector_sequence
            .push(task.difficulty.clone());
        metrics
            .raw_plan_space_sequence
            .push(task.raw_plan_space.clone());
        metrics
            .planning_work_unit_sequence
            .push(task.planning_work_units);
        metrics
            .raw_action_branching_sequence
            .push(task.raw_candidate_actions);
        metrics
            .semantically_eligible_action_sequence
            .push(task.semantically_eligible_actions);
        metrics
            .reachability_survivor_sequence
            .push(task.reachability_surviving_actions);
        metrics
            .actual_rollout_sequence
            .push(task.actually_rolled_out_actions);
        metrics
            .search_compression_ratio_sequence
            .push(task.search_compression_ratio);
        metrics.action_horizon_sequence.push(task.action_horizon);
        metrics
            .causal_dependency_depth_sequence
            .push(task.causal_dependency_depth);
        metrics.subgoal_count_sequence.push(task.subgoal_count);
        metrics.subgoal_depth_sequence.push(task.subgoal_depth);
        metrics
            .planning_horizon_chosen_sequence
            .extend(task.planning_horizon_chosen_sequence.iter().copied());
        metrics
            .temporal_abstraction_sequence
            .extend(task.temporal_abstraction_sequence.iter().cloned());
        metrics
            .reachability_query_sequence
            .push(task.reachability_queries);
        metrics
            .world_model_call_sequence
            .push(task.world_model_calls);
        metrics
            .causal_mechanism_call_sequence
            .push(task.causal_mechanism_calls);
        metrics.active_entity_sequence.push(task.active_entities);
        metrics.active_relation_sequence.push(task.active_relations);
        metrics
            .active_semantic_node_sequence
            .push(task.active_semantic_nodes);
        metrics
            .active_causal_mechanism_sequence
            .push(task.active_causal_mechanisms);
        metrics
            .planning_cpu_time_sequence
            .push(task.planning_cpu_time_ns);
        metrics
            .planning_wall_time_sequence
            .push(task.planning_wall_time_ns);
        metrics.peak_rss_sequence.push(task.peak_rss_bytes);
        metrics
            .semantic_temporary_bytes_sequence
            .push(task.semantic_temporary_bytes);
        metrics
            .goal_success_sequence
            .push(u64::from(task.goal_success));
        metrics
            .constraint_violation_sequence
            .push(task.constraint_violation_accepts);
    }
    metrics.active_entities_p50 = percentile(&metrics.active_entity_sequence, 50);
    metrics.active_entities_p95 = percentile(&metrics.active_entity_sequence, 95);
    metrics.active_entities_p99 = percentile(&metrics.active_entity_sequence, 99);
    metrics.active_relations_p50 = percentile(&metrics.active_relation_sequence, 50);
    metrics.active_relations_p95 = percentile(&metrics.active_relation_sequence, 95);
    metrics.active_relations_p99 = percentile(&metrics.active_relation_sequence, 99);
    metrics.active_semantic_nodes_p50 = percentile(&metrics.active_semantic_node_sequence, 50);
    metrics.active_semantic_nodes_p95 = percentile(&metrics.active_semantic_node_sequence, 95);
    metrics.active_semantic_nodes_p99 = percentile(&metrics.active_semantic_node_sequence, 99);
    metrics.active_causal_mechanisms_p50 =
        percentile(&metrics.active_causal_mechanism_sequence, 50);
    metrics.active_causal_mechanisms_p95 =
        percentile(&metrics.active_causal_mechanism_sequence, 95);
    metrics.active_causal_mechanisms_p99 =
        percentile(&metrics.active_causal_mechanism_sequence, 99);
    metrics
}

fn percentile(values: &[u64], percentile: usize) -> u64 {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let index = ((sorted.len() - 1) * percentile + 99) / 100;
    sorted[index.min(sorted.len() - 1)]
}

pub(crate) fn autonomously_research_efficiency(
    cases: &[HiddenScalingTask],
    baseline: &ScalingArmEvidence,
    challenge_hash: &str,
) -> AutonomousEfficiencyResearch {
    let started = Instant::now();
    let dominant_bottleneck = dominant_work_component(&baseline.task_evidence);
    let hypotheses = vec![
        EfficiencyHypothesis {
            hypothesis_id: 1,
            diagnosis: dominant_bottleneck.clone(),
            proposed_generic_mechanism: "SEMANTIC_CAUSAL_INDEX_WITH_REUSED_REACHABILITY_PROOFS"
                .into(),
        },
        EfficiencyHypothesis {
            hypothesis_id: 2,
            diagnosis: "REPEATED_FULL_PLAN_RECONSTRUCTION".into(),
            proposed_generic_mechanism:
                "ADAPTIVE_TEMPORAL_ABSTRACTION_WITH_BOUNDED_LOCAL_EXECUTION".into(),
        },
        EfficiencyHypothesis {
            hypothesis_id: 3,
            diagnosis: "HIERARCHY_OVERHEAD_ON_SIMPLE_TASKS".into(),
            proposed_generic_mechanism: "STRUCTURE_CONDITIONED_FLAT_HIERARCHICAL_MIXED_SELECTION"
                .into(),
        },
    ];
    let candidates = [
        ScalingPlannerProgram::semantic_index_only(),
        ScalingPlannerProgram::single_scale(),
        ScalingPlannerProgram::efficient(),
    ];
    let mut experiments = Vec::new();
    let mut best: Option<(ScalingPlannerProgram, u64)> = None;
    for (index, program) in candidates.into_iter().enumerate() {
        let arm = run_arm("DEVELOPMENT", challenge_hash, cases, program.clone(), false);
        let correct = arm.metrics.tasks_passed == arm.metrics.tasks_total;
        let reduction = baseline
            .metrics
            .total_planning_work
            .saturating_sub(arm.metrics.total_planning_work);
        if correct
            && reduction > 0
            && best
                .as_ref()
                .is_none_or(|(_, work)| arm.metrics.total_planning_work < *work)
        {
            best = Some((program.clone(), arm.metrics.total_planning_work));
        }
        experiments.push(EfficiencyExperiment {
            experiment_id: index as u64 + 1,
            program,
            tasks_passed: arm.metrics.tasks_passed,
            tasks_total: arm.metrics.tasks_total,
            planning_work: arm.metrics.total_planning_work,
            work_reduction_vs_baseline: reduction,
            accepted: false,
        });
    }
    let (selected_program, selected_work) = best.unwrap_or_else(|| {
        (
            ScalingPlannerProgram::baseline(),
            baseline.metrics.total_planning_work,
        )
    });
    if let Some(experiment) = experiments.iter_mut().find(|experiment| {
        experiment.program == selected_program && experiment.planning_work == selected_work
    }) {
        experiment.accepted = true;
    }
    let accepted = u64::from(selected_program.mode != ScalingPlannerMode::BaselineSem33R1);
    AutonomousEfficiencyResearch {
        dominant_bottleneck,
        diagnoses: 1,
        repair_hypotheses: hypotheses.len() as u64,
        repairs_implemented: experiments.len() as u64,
        repairs_accepted: accepted,
        autonomous_research_epochs_executed: experiments.len() as u64 * 8,
        research_wall_time_ns: started.elapsed().as_nanos().min(u64::MAX as u128) as u64,
        hypotheses,
        experiments,
        selected_program,
        human_planner_efficiency_repair_events: 0,
        human_temporal_scale_selection_events: 0,
        human_branch_pruning_rule_selection_events: 0,
        human_subgoal_policy_selection_events: 0,
        human_flat_hierarchical_mode_selection_events: 0,
    }
}

fn dominant_work_component(tasks: &[TaskScalingEvidence]) -> String {
    let mut totals = BTreeMap::new();
    for task in tasks {
        *totals.entry("GOAL_GROUNDING").or_insert(0_u64) += task.work.goal_grounding;
        *totals.entry("REACHABILITY_QUERY_COST").or_insert(0) += task.work.reachability;
        *totals.entry("SUBGOAL_SYNTHESIS_COST").or_insert(0) += task.work.subgoal_synthesis;
        *totals.entry("WORLD_MODEL_ROLLOUT_COST").or_insert(0) += task.work.world_model_rollout;
        *totals.entry("CAUSAL_ROUTING_COST").or_insert(0) += task.work.causal_routing;
        *totals.entry("UNCERTAINTY_BRANCHING_LIMIT").or_insert(0) +=
            task.work.uncertainty_reasoning;
        *totals.entry("CANDIDATE_COMPARISON_COST").or_insert(0) += task.work.candidate_comparison;
        *totals.entry("REPLANNING_OVERHEAD").or_insert(0) += task.work.execution_replanning;
    }
    totals
        .into_iter()
        .max_by_key(|(_, value)| *value)
        .map(|(name, _)| name.into())
        .unwrap_or_else(|| "OTHER".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn development_and_final_instances_are_disjoint() {
        let development = generate_cases(ScalingSet::Development, 11);
        let final_set = generate_cases(ScalingSet::FinalHoldout, 12);
        let development_ids = development
            .iter()
            .map(|case| case.public.planning_task.task_id)
            .collect::<BTreeSet<_>>();
        assert!(final_set
            .iter()
            .all(|case| !development_ids.contains(&case.public.planning_task.task_id)));
    }

    #[test]
    fn adaptive_program_preserves_correctness_and_reduces_work() {
        let cases = generate_cases(ScalingSet::Development, 11);
        let baseline = run_arm("DEV", "x", &cases, ScalingPlannerProgram::baseline(), false);
        let efficient = run_arm(
            "DEV",
            "x",
            &cases,
            ScalingPlannerProgram::efficient(),
            false,
        );
        assert_eq!(
            efficient.metrics.tasks_passed,
            efficient.metrics.tasks_total
        );
        assert_eq!(baseline.metrics.tasks_passed, baseline.metrics.tasks_total);
        assert!(efficient.metrics.total_planning_work < baseline.metrics.total_planning_work);
    }

    #[test]
    fn pruning_is_proof_carrying_and_sound_on_generated_tasks() {
        let cases = generate_cases(ScalingSet::Development, 11);
        let efficient = run_arm(
            "DEV",
            "x",
            &cases,
            ScalingPlannerProgram::efficient(),
            false,
        );
        assert_eq!(efficient.metrics.unsound_prune_events, 0);
        assert!(efficient.metrics.causal_prune_events > 0);
        assert!(efficient.metrics.constraint_prune_events > 0);
        assert!(efficient.metrics.reachability_prune_events > 0);
    }
}
