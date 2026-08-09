use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawScalingFields {
    pub baseline_scaling_tasks: u64,
    pub final_fresh_scaling_tasks: u64,
    pub final_tasks_passed: u64,
    pub planning_difficulty_axes_measured: u64,
    pub work_decomposition_complete: bool,
    pub raw_plan_space_grows_faster_than_actual_work: bool,
    pub baseline_planning_work: u64,
    pub final_planning_work: u64,
    pub baseline_long_horizon_work: u64,
    pub final_long_horizon_work: u64,
    pub semantically_routed_work_below_raw_space: bool,
    pub adaptive_temporal_abstraction_observed: bool,
    pub distractor_world_scaling_pass: bool,
    pub relevant_entity_scaling_characterized: bool,
    pub branching_scaling_characterized: bool,
    pub horizon_scaling_characterized: bool,
    pub uncertainty_scaling_characterized: bool,
    pub constraint_scaling_characterized: bool,
    pub autonomous_efficiency_repairs_accepted: u64,
    pub frozen_baseline_all_scaling_gates_pass: bool,
    pub final_holdout_fresh: bool,
    pub development_final_instance_overlap: u64,
    pub novel_relation_topology_planning_pass: bool,
    pub entity_cardinality_planning_generalization_pass: bool,
    pub novel_goal_composition_pass: bool,
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
    pub reachability_efficiency_ablation_pass: bool,
    pub temporal_abstraction_ablation_pass: bool,
    pub hierarchical_planning_ablation_pass: bool,
    pub sparse_planning_scaling_ablation_pass: bool,
    pub goal_correctness_regressions: u64,
    pub reachability_regressions: u64,
    pub hierarchical_planning_regressions: u64,
    pub uncertainty_planning_regressions: u64,
    pub closed_loop_regressions: u64,
    pub structural_generalization_regressions: u64,
    pub planning_work_accounting_gaming_events: u64,
    pub uncounted_planning_side_work_events: u64,
    pub task_id_to_procedure_authority: bool,
    pub world_hash_to_procedure_authority: bool,
    pub goal_hash_to_procedure_authority: bool,
    pub whole_planning_architecture_transplants: u64,
    pub paper_name_is_promotion_authority: bool,
    pub sota_result_is_promotion_authority: bool,
    pub verifier_runner_transport_equivalence: bool,
    pub transport_semantic_roundtrip_diff: u64,
    pub transport_fail_open_events: u64,
    pub transport_field_drop_events: u64,
    pub raw_field_acceptance_authority: bool,
    pub acceptance_false_pass_events: u64,
    pub external_llm_calls: u64,
    pub local_teacher_calls: u64,
    pub network_reads: u64,
    pub network_writes: u64,
    pub remote_executions: u64,
    pub core_mandatory_vram: u64,
    pub core_depends_on_gpu_runtime: bool,
    pub human_planner_efficiency_repair_events: u64,
    pub human_temporal_scale_selection_events: u64,
    pub human_branch_pruning_rule_selection_events: u64,
    pub human_subgoal_policy_selection_events: u64,
    pub human_flat_hierarchical_mode_selection_events: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sem34Acceptance {
    pub sem34_pass: bool,
    pub levels: [bool; 8],
    pub violations: Vec<String>,
}

pub fn evaluate_raw(fields: &RawScalingFields) -> Sem34Acceptance {
    let level_a = fields.baseline_scaling_tasks >= 12
        && fields.final_fresh_scaling_tasks >= 10
        && fields.planning_difficulty_axes_measured >= 8
        && fields.work_decomposition_complete;
    let level_b = fields.raw_plan_space_grows_faster_than_actual_work
        && fields.semantically_routed_work_below_raw_space
        && fields.final_planning_work < fields.baseline_planning_work
        && fields.causal_prune_events > 0
        && fields.constraint_prune_events > 0
        && fields.reachability_prune_events > 0
        && fields.unsound_prune_events == 0
        && fields.full_action_tree_enumeration_events == 0;
    let level_c = fields.adaptive_temporal_abstraction_observed
        && fields.temporal_abstraction_ablation_pass
        && fields.hierarchical_planning_ablation_pass
        && fields.high_level_unrealizable_subgoal_accepts == 0;
    let level_d = fields.distractor_world_scaling_pass
        && fields.world_memory_full_scans == 0
        && fields.causal_mechanism_full_scans == 0
        && fields.sparse_planning_scaling_ablation_pass;
    let level_e = fields.autonomous_efficiency_repairs_accepted > 0
        || fields.frozen_baseline_all_scaling_gates_pass;
    let level_f = fields.final_holdout_fresh
        && fields.development_final_instance_overlap == 0
        && fields.final_tasks_passed == fields.final_fresh_scaling_tasks
        && fields.novel_relation_topology_planning_pass
        && fields.entity_cardinality_planning_generalization_pass
        && fields.novel_goal_composition_pass;
    let level_g = fields.goal_correctness_regressions == 0
        && fields.reachability_regressions == 0
        && fields.hierarchical_planning_regressions == 0
        && fields.uncertainty_planning_regressions == 0
        && fields.closed_loop_regressions == 0
        && fields.structural_generalization_regressions == 0
        && fields.constraint_violation_accepts == 0;
    let level_h = fields.reachability_efficiency_ablation_pass
        && fields.temporal_abstraction_ablation_pass
        && fields.hierarchical_planning_ablation_pass
        && fields.sparse_planning_scaling_ablation_pass;
    let invariants = fields.planning_work_accounting_gaming_events == 0
        && fields.uncounted_planning_side_work_events == 0
        && !fields.task_id_to_procedure_authority
        && !fields.world_hash_to_procedure_authority
        && !fields.goal_hash_to_procedure_authority
        && fields.whole_planning_architecture_transplants == 0
        && !fields.paper_name_is_promotion_authority
        && !fields.sota_result_is_promotion_authority
        && fields.verifier_runner_transport_equivalence
        && fields.transport_semantic_roundtrip_diff == 0
        && fields.transport_fail_open_events == 0
        && fields.transport_field_drop_events == 0
        && fields.raw_field_acceptance_authority
        && fields.acceptance_false_pass_events == 0
        && fields.external_llm_calls == 0
        && fields.local_teacher_calls == 0
        && fields.network_reads == 0
        && fields.network_writes == 0
        && fields.remote_executions == 0
        && fields.core_mandatory_vram == 0
        && !fields.core_depends_on_gpu_runtime
        && fields.human_planner_efficiency_repair_events == 0
        && fields.human_temporal_scale_selection_events == 0
        && fields.human_branch_pruning_rule_selection_events == 0
        && fields.human_subgoal_policy_selection_events == 0
        && fields.human_flat_hierarchical_mode_selection_events == 0;
    let levels = [
        level_a, level_b, level_c, level_d, level_e, level_f, level_g, level_h,
    ];
    let mut violations = Vec::new();
    for (index, passed) in levels.iter().enumerate() {
        if !passed {
            violations.push(format!(
                "SEM34_LEVEL_{}_FAILED",
                (b'A' + index as u8) as char
            ));
        }
    }
    if !invariants {
        violations.push("SEM34_INVARIANT_CONTRACT_FAILED".into());
    }
    Sem34Acceptance {
        sem34_pass: levels.into_iter().all(|passed| passed) && invariants,
        levels,
        violations,
    }
}

pub fn evaluate_raw_secondary(fields: &RawScalingFields) -> Sem34Acceptance {
    let checks = [
        fields.baseline_scaling_tasks >= 12
            && fields.final_fresh_scaling_tasks >= 10
            && fields.planning_difficulty_axes_measured >= 8
            && fields.work_decomposition_complete,
        fields.raw_plan_space_grows_faster_than_actual_work
            && fields.semantically_routed_work_below_raw_space
            && fields.final_planning_work < fields.baseline_planning_work
            && fields.causal_prune_events > 0
            && fields.constraint_prune_events > 0
            && fields.reachability_prune_events > 0
            && fields.unsound_prune_events == 0
            && fields.full_action_tree_enumeration_events == 0,
        fields.adaptive_temporal_abstraction_observed
            && fields.temporal_abstraction_ablation_pass
            && fields.hierarchical_planning_ablation_pass
            && fields.high_level_unrealizable_subgoal_accepts == 0,
        fields.distractor_world_scaling_pass
            && fields.world_memory_full_scans == 0
            && fields.causal_mechanism_full_scans == 0
            && fields.sparse_planning_scaling_ablation_pass,
        fields.autonomous_efficiency_repairs_accepted > 0
            || fields.frozen_baseline_all_scaling_gates_pass,
        fields.final_holdout_fresh
            && fields.development_final_instance_overlap == 0
            && fields.final_tasks_passed == fields.final_fresh_scaling_tasks
            && fields.novel_relation_topology_planning_pass
            && fields.entity_cardinality_planning_generalization_pass
            && fields.novel_goal_composition_pass,
        [
            fields.goal_correctness_regressions,
            fields.reachability_regressions,
            fields.hierarchical_planning_regressions,
            fields.uncertainty_planning_regressions,
            fields.closed_loop_regressions,
            fields.structural_generalization_regressions,
            fields.constraint_violation_accepts,
        ]
        .into_iter()
        .all(|count| count == 0),
        fields.reachability_efficiency_ablation_pass
            && fields.temporal_abstraction_ablation_pass
            && fields.hierarchical_planning_ablation_pass
            && fields.sparse_planning_scaling_ablation_pass,
    ];
    let zero_counts = [
        fields.planning_work_accounting_gaming_events,
        fields.uncounted_planning_side_work_events,
        fields.whole_planning_architecture_transplants,
        fields.transport_semantic_roundtrip_diff,
        fields.transport_fail_open_events,
        fields.transport_field_drop_events,
        fields.acceptance_false_pass_events,
        fields.external_llm_calls,
        fields.local_teacher_calls,
        fields.network_reads,
        fields.network_writes,
        fields.remote_executions,
        fields.core_mandatory_vram,
        fields.human_planner_efficiency_repair_events,
        fields.human_temporal_scale_selection_events,
        fields.human_branch_pruning_rule_selection_events,
        fields.human_subgoal_policy_selection_events,
        fields.human_flat_hierarchical_mode_selection_events,
    ];
    let false_flags = [
        fields.task_id_to_procedure_authority,
        fields.world_hash_to_procedure_authority,
        fields.goal_hash_to_procedure_authority,
        fields.paper_name_is_promotion_authority,
        fields.sota_result_is_promotion_authority,
        fields.core_depends_on_gpu_runtime,
    ];
    let invariants = zero_counts.into_iter().all(|count| count == 0)
        && false_flags.into_iter().all(|flag| !flag)
        && fields.verifier_runner_transport_equivalence
        && fields.raw_field_acceptance_authority;
    let mut violations = checks
        .iter()
        .enumerate()
        .filter(|(_, passed)| !**passed)
        .map(|(index, _)| format!("SEM34_LEVEL_{}_FAILED", (b'A' + index as u8) as char))
        .collect::<Vec<_>>();
    if !invariants {
        violations.push("SEM34_INVARIANT_CONTRACT_FAILED".into());
    }
    Sem34Acceptance {
        sem34_pass: checks.into_iter().all(|passed| passed) && invariants,
        levels: checks,
        violations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn passing_fields() -> RawScalingFields {
        RawScalingFields {
            baseline_scaling_tasks: 16,
            final_fresh_scaling_tasks: 13,
            final_tasks_passed: 13,
            planning_difficulty_axes_measured: 12,
            work_decomposition_complete: true,
            raw_plan_space_grows_faster_than_actual_work: true,
            baseline_planning_work: 1_000,
            final_planning_work: 400,
            baseline_long_horizon_work: 800,
            final_long_horizon_work: 250,
            semantically_routed_work_below_raw_space: true,
            adaptive_temporal_abstraction_observed: true,
            distractor_world_scaling_pass: true,
            relevant_entity_scaling_characterized: true,
            branching_scaling_characterized: true,
            horizon_scaling_characterized: true,
            uncertainty_scaling_characterized: true,
            constraint_scaling_characterized: true,
            autonomous_efficiency_repairs_accepted: 1,
            frozen_baseline_all_scaling_gates_pass: false,
            final_holdout_fresh: true,
            development_final_instance_overlap: 0,
            novel_relation_topology_planning_pass: true,
            entity_cardinality_planning_generalization_pass: true,
            novel_goal_composition_pass: true,
            causal_prune_events: 1,
            constraint_prune_events: 1,
            reachability_prune_events: 1,
            equivalence_prune_events: 0,
            dominance_prune_events: 1,
            unsound_prune_events: 0,
            high_level_unrealizable_subgoal_accepts: 0,
            constraint_violation_accepts: 0,
            full_action_tree_enumeration_events: 0,
            world_memory_full_scans: 0,
            causal_mechanism_full_scans: 0,
            reachability_efficiency_ablation_pass: true,
            temporal_abstraction_ablation_pass: true,
            hierarchical_planning_ablation_pass: true,
            sparse_planning_scaling_ablation_pass: true,
            goal_correctness_regressions: 0,
            reachability_regressions: 0,
            hierarchical_planning_regressions: 0,
            uncertainty_planning_regressions: 0,
            closed_loop_regressions: 0,
            structural_generalization_regressions: 0,
            planning_work_accounting_gaming_events: 0,
            uncounted_planning_side_work_events: 0,
            task_id_to_procedure_authority: false,
            world_hash_to_procedure_authority: false,
            goal_hash_to_procedure_authority: false,
            whole_planning_architecture_transplants: 0,
            paper_name_is_promotion_authority: false,
            sota_result_is_promotion_authority: false,
            verifier_runner_transport_equivalence: true,
            transport_semantic_roundtrip_diff: 0,
            transport_fail_open_events: 0,
            transport_field_drop_events: 0,
            raw_field_acceptance_authority: true,
            acceptance_false_pass_events: 0,
            external_llm_calls: 0,
            local_teacher_calls: 0,
            network_reads: 0,
            network_writes: 0,
            remote_executions: 0,
            core_mandatory_vram: 0,
            core_depends_on_gpu_runtime: false,
            human_planner_efficiency_repair_events: 0,
            human_temporal_scale_selection_events: 0,
            human_branch_pruning_rule_selection_events: 0,
            human_subgoal_policy_selection_events: 0,
            human_flat_hierarchical_mode_selection_events: 0,
        }
    }

    #[test]
    fn independent_acceptance_paths_agree() {
        let fields = passing_fields();
        assert_eq!(evaluate_raw(&fields), evaluate_raw_secondary(&fields));
        assert!(evaluate_raw(&fields).sem34_pass);
    }

    #[test]
    fn correctness_regression_fails_closed() {
        let mut fields = passing_fields();
        fields.goal_correctness_regressions = 1;
        assert!(!evaluate_raw(&fields).sem34_pass);
    }
}
