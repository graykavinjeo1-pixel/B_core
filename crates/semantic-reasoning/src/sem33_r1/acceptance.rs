use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawPlanningFields {
    pub goal_directed_semantic_planner_present: bool,
    pub desired_world_phenotype_present: bool,
    pub scalar_reward_is_goal_authority: bool,
    pub plan_ir_present: bool,
    pub planner_is_goal_success_authority: bool,
    pub goal_can_mutate_world_model_causal_semantics: bool,
    pub natural_language_is_planning_authority: bool,
    pub goal_tasks_total: u64,
    pub goal_tasks_solved: u64,
    pub unreachable_plan_accepts: u64,
    pub semantic_near_unreachable_shortcut_accepts: u64,
    pub reachability_planning_ablation_pass: bool,
    pub autonomous_subgoals_created: u64,
    pub human_subgoal_selection_events: u64,
    pub hierarchical_plan_events: u64,
    pub max_subgoal_depth: u64,
    pub hierarchical_planning_ablation_pass: bool,
    pub information_gathering_actions: u64,
    pub unsupported_plan_confident_executions: u64,
    pub stochastic_plan_branch_events: u64,
    pub uncertainty_planning_ablation_pass: bool,
    pub plan_execution_actions: u64,
    pub replan_events: u64,
    pub replan_caused_by_model_residual: u64,
    pub goals_satisfied_after_replan: u64,
    pub closed_loop_replanning_ablation_pass: bool,
    pub novel_relation_topology_planning_pass: bool,
    pub entity_cardinality_planning_generalization_pass: bool,
    pub novel_goal_composition_pass: bool,
    pub planning_overgeneralization_events: u64,
    pub goal_specific_policy_training_events: u64,
    pub task_specific_planner_branches: u64,
    pub total_world_entities: u64,
    pub world_memory_full_scans: u64,
    pub causal_mechanism_full_scans: u64,
    pub full_action_tree_enumeration_events: u64,
    pub sparse_planning_ablation_pass: bool,
    pub causal_model_planning_ablation_pass: bool,
    pub causal_path_certificates: u64,
    pub causal_path_decompression_available: bool,
    pub known_dead_end_entries: u64,
    pub task_id_to_plan_lookup_authority: bool,
    pub world_hash_to_plan_lookup_authority: bool,
    pub goal_hash_to_plan_lookup_authority: bool,
    pub gold_action_reads: u64,
    pub gold_plan_reads: u64,
    pub expected_goal_state_lookups: u64,
    pub future_world_event_leakage_events: u64,
}

impl RawPlanningFields {
    pub fn all_pass() -> Self {
        Self {
            goal_directed_semantic_planner_present: true,
            desired_world_phenotype_present: true,
            scalar_reward_is_goal_authority: false,
            plan_ir_present: true,
            planner_is_goal_success_authority: false,
            goal_can_mutate_world_model_causal_semantics: false,
            natural_language_is_planning_authority: false,
            goal_tasks_total: 12,
            goal_tasks_solved: 12,
            unreachable_plan_accepts: 0,
            semantic_near_unreachable_shortcut_accepts: 0,
            reachability_planning_ablation_pass: true,
            autonomous_subgoals_created: 8,
            human_subgoal_selection_events: 0,
            hierarchical_plan_events: 4,
            max_subgoal_depth: 8,
            hierarchical_planning_ablation_pass: true,
            information_gathering_actions: 2,
            unsupported_plan_confident_executions: 0,
            stochastic_plan_branch_events: 1,
            uncertainty_planning_ablation_pass: true,
            plan_execution_actions: 32,
            replan_events: 20,
            replan_caused_by_model_residual: 1,
            goals_satisfied_after_replan: 1,
            closed_loop_replanning_ablation_pass: true,
            novel_relation_topology_planning_pass: true,
            entity_cardinality_planning_generalization_pass: true,
            novel_goal_composition_pass: true,
            planning_overgeneralization_events: 0,
            goal_specific_policy_training_events: 0,
            task_specific_planner_branches: 0,
            total_world_entities: 100_000,
            world_memory_full_scans: 0,
            causal_mechanism_full_scans: 0,
            full_action_tree_enumeration_events: 0,
            sparse_planning_ablation_pass: true,
            causal_model_planning_ablation_pass: true,
            causal_path_certificates: 10,
            causal_path_decompression_available: true,
            known_dead_end_entries: 0,
            task_id_to_plan_lookup_authority: false,
            world_hash_to_plan_lookup_authority: false,
            goal_hash_to_plan_lookup_authority: false,
            gold_action_reads: 0,
            gold_plan_reads: 0,
            expected_goal_state_lookups: 0,
            future_world_event_leakage_events: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanningAcceptanceDecision {
    pub levels: [bool; 8],
    pub sem33_pass: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NegativeAcceptanceCanary {
    pub field: String,
    pub overall_pass: bool,
    pub primary_secondary_equal: bool,
}

pub fn mandatory_negative_canaries() -> Vec<NegativeAcceptanceCanary> {
    let baseline = serde_json::to_value(RawPlanningFields::all_pass())
        .expect("raw planning fields are serializable");
    baseline
        .as_object()
        .expect("raw planning fields serialize as an object")
        .keys()
        .map(|field| {
            let mut candidate = baseline.clone();
            let value = &mut candidate[field];
            if let Some(boolean) = value.as_bool() {
                *value = serde_json::Value::Bool(!boolean);
            } else if field == "goal_tasks_total" {
                *value = serde_json::Value::from(13_u64);
            } else if field == "total_world_entities" {
                *value = serde_json::Value::from(99_999_u64);
            } else if [
                "max_subgoal_depth",
                "unreachable_plan_accepts",
                "semantic_near_unreachable_shortcut_accepts",
                "human_subgoal_selection_events",
                "unsupported_plan_confident_executions",
                "planning_overgeneralization_events",
                "goal_specific_policy_training_events",
                "task_specific_planner_branches",
                "world_memory_full_scans",
                "causal_mechanism_full_scans",
                "full_action_tree_enumeration_events",
                "known_dead_end_entries",
                "gold_action_reads",
                "gold_plan_reads",
                "expected_goal_state_lookups",
                "future_world_event_leakage_events",
            ]
            .contains(&field.as_str())
            {
                *value = serde_json::Value::from(1_u64);
            } else {
                *value = serde_json::Value::from(0_u64);
            }
            let raw: RawPlanningFields =
                serde_json::from_value(candidate).expect("valid negative canary");
            let primary = evaluate_raw(&raw);
            let secondary = evaluate_raw_secondary(&raw);
            NegativeAcceptanceCanary {
                field: field.clone(),
                overall_pass: primary.sem33_pass,
                primary_secondary_equal: primary == secondary,
            }
        })
        .collect()
}

pub fn evaluate_raw(fields: &RawPlanningFields) -> PlanningAcceptanceDecision {
    let levels = [
        fields.goal_directed_semantic_planner_present
            && fields.desired_world_phenotype_present
            && !fields.scalar_reward_is_goal_authority
            && fields.plan_ir_present
            && !fields.planner_is_goal_success_authority
            && !fields.goal_can_mutate_world_model_causal_semantics
            && !fields.natural_language_is_planning_authority,
        fields.goal_tasks_total > 0
            && fields.goal_tasks_solved == fields.goal_tasks_total
            && fields.unreachable_plan_accepts == 0
            && fields.semantic_near_unreachable_shortcut_accepts == 0
            && fields.reachability_planning_ablation_pass,
        fields.autonomous_subgoals_created > 0
            && fields.human_subgoal_selection_events == 0
            && fields.hierarchical_plan_events > 0
            && fields.max_subgoal_depth >= 2
            && fields.hierarchical_planning_ablation_pass,
        fields.information_gathering_actions > 0
            && fields.unsupported_plan_confident_executions == 0
            && fields.stochastic_plan_branch_events > 0
            && fields.uncertainty_planning_ablation_pass,
        fields.plan_execution_actions > 0
            && fields.replan_events > 0
            && fields.replan_caused_by_model_residual > 0
            && fields.goals_satisfied_after_replan > 0
            && fields.closed_loop_replanning_ablation_pass,
        fields.novel_relation_topology_planning_pass
            && fields.entity_cardinality_planning_generalization_pass
            && fields.novel_goal_composition_pass
            && fields.planning_overgeneralization_events == 0
            && fields.goal_specific_policy_training_events == 0
            && fields.task_specific_planner_branches == 0,
        fields.total_world_entities >= 100_000
            && fields.world_memory_full_scans == 0
            && fields.causal_mechanism_full_scans == 0
            && fields.full_action_tree_enumeration_events == 0
            && fields.sparse_planning_ablation_pass,
        fields.causal_model_planning_ablation_pass
            && fields.causal_path_certificates > 0
            && fields.causal_path_decompression_available
            && fields.known_dead_end_entries == 0
            && !fields.task_id_to_plan_lookup_authority
            && !fields.world_hash_to_plan_lookup_authority
            && !fields.goal_hash_to_plan_lookup_authority
            && fields.gold_action_reads == 0
            && fields.gold_plan_reads == 0
            && fields.expected_goal_state_lookups == 0
            && fields.future_world_event_leakage_events == 0,
    ];
    PlanningAcceptanceDecision {
        sem33_pass: levels.iter().copied().all(|pass| pass),
        levels,
    }
}

pub fn evaluate_raw_secondary(fields: &RawPlanningFields) -> PlanningAcceptanceDecision {
    let a = [
        fields.goal_directed_semantic_planner_present,
        fields.desired_world_phenotype_present,
        !fields.scalar_reward_is_goal_authority,
        fields.plan_ir_present,
        !fields.planner_is_goal_success_authority,
        !fields.goal_can_mutate_world_model_causal_semantics,
        !fields.natural_language_is_planning_authority,
    ]
    .into_iter()
    .all(|value| value);
    let b = fields.goal_tasks_total != 0
        && fields.goal_tasks_total == fields.goal_tasks_solved
        && fields.unreachable_plan_accepts == 0
        && fields.semantic_near_unreachable_shortcut_accepts == 0
        && fields.reachability_planning_ablation_pass;
    let c = fields.autonomous_subgoals_created != 0
        && fields.human_subgoal_selection_events == 0
        && fields.hierarchical_plan_events != 0
        && fields.max_subgoal_depth >= 2
        && fields.hierarchical_planning_ablation_pass;
    let d = fields.information_gathering_actions != 0
        && fields.unsupported_plan_confident_executions == 0
        && fields.stochastic_plan_branch_events != 0
        && fields.uncertainty_planning_ablation_pass;
    let e = fields.plan_execution_actions != 0
        && fields.replan_events != 0
        && fields.replan_caused_by_model_residual != 0
        && fields.goals_satisfied_after_replan != 0
        && fields.closed_loop_replanning_ablation_pass;
    let f = [
        fields.novel_relation_topology_planning_pass,
        fields.entity_cardinality_planning_generalization_pass,
        fields.novel_goal_composition_pass,
        fields.planning_overgeneralization_events == 0,
        fields.goal_specific_policy_training_events == 0,
        fields.task_specific_planner_branches == 0,
    ]
    .into_iter()
    .all(|value| value);
    let g = fields.total_world_entities >= 100_000
        && fields.world_memory_full_scans == 0
        && fields.causal_mechanism_full_scans == 0
        && fields.full_action_tree_enumeration_events == 0
        && fields.sparse_planning_ablation_pass;
    let h = fields.causal_model_planning_ablation_pass
        && fields.causal_path_certificates != 0
        && fields.causal_path_decompression_available
        && fields.known_dead_end_entries == 0
        && !fields.task_id_to_plan_lookup_authority
        && !fields.world_hash_to_plan_lookup_authority
        && !fields.goal_hash_to_plan_lookup_authority
        && fields.gold_action_reads == 0
        && fields.gold_plan_reads == 0
        && fields.expected_goal_state_lookups == 0
        && fields.future_world_event_leakage_events == 0;
    let levels = [a, b, c, d, e, f, g, h];
    PlanningAcceptanceDecision {
        sem33_pass: levels.iter().copied().all(|pass| pass),
        levels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mandatory_level_canaries_fail_closed() {
        for level in 0..8 {
            let mut raw = RawPlanningFields::all_pass();
            match level {
                0 => raw.plan_ir_present = false,
                1 => raw.unreachable_plan_accepts = 1,
                2 => raw.human_subgoal_selection_events = 1,
                3 => raw.unsupported_plan_confident_executions = 1,
                4 => raw.closed_loop_replanning_ablation_pass = false,
                5 => raw.novel_goal_composition_pass = false,
                6 => raw.world_memory_full_scans = 1,
                7 => raw.gold_plan_reads = 1,
                _ => unreachable!(),
            }
            let primary = evaluate_raw(&raw);
            assert!(!primary.levels[level]);
            assert!(!primary.sem33_pass);
            assert_eq!(primary, evaluate_raw_secondary(&raw));
        }
    }

    #[test]
    fn every_mandatory_raw_field_fails_closed_individually() {
        let canaries = mandatory_negative_canaries();
        assert!(!canaries.is_empty());
        assert!(canaries
            .iter()
            .all(|canary| !canary.overall_pass && canary.primary_secondary_equal));
    }
}
