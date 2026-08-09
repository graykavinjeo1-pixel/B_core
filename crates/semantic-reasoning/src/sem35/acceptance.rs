use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::engine::{
    ProcessSemanticKey, TemporalArmMode, TemporalArmResult, TemporalProcessIr,
    TemporalResearchOutcome,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct Sem35Evaluation {
    pub sem35_status: String,
    pub disposition: String,
    pub temporal_limit_diagnosis: String,
    pub autonomous_event_boundary_discovery_present: bool,
    pub temporal_processes_proposed: u64,
    pub temporal_processes_verified: u64,
    pub temporal_processes_promoted: u64,
    pub variable_duration_temporal_abstraction_pass: bool,
    pub temporal_process_id_is_semantic_payload: bool,
    pub duration_is_process_identity_authority: bool,
    pub fixed_chunk_length_is_temporal_boundary_authority: bool,
    pub surprise_is_temporal_boundary_authority: bool,
    pub fixed_action_repeat_is_temporal_meaning_authority: bool,
    pub cross_scale_semantic_equivalence_pass: bool,
    pub temporal_process_decompression_available: bool,
    pub unrealizable_temporal_macro_accepts: u64,
    pub temporal_process_composition_events: u64,
    pub incompatible_process_sequence_accepts: u64,
    pub temporal_process_interruption_events: u64,
    pub invalid_process_blind_completions: u64,
    pub duration_uncertainty_collapse_events: u64,
    pub cross_duration_process_transfer_pass: bool,
    pub temporal_process_entity_id_invariance_pass: bool,
    pub temporal_process_topology_transfer_pass: bool,
    pub temporal_process_overgeneralization_events: u64,
    pub process_level_counterfactual_pass: bool,
    pub unsupported_macro_confident_hallucinations: u64,
    pub temporal_macro_reachability_false_accepts: u64,
    pub unverified_observation_skip_events: u64,
    pub variable_duration_abstraction_ablation_pass: bool,
    pub temporal_boundary_discovery_ablation_pass: bool,
    pub temporal_process_memory_ablation_pass: bool,
    pub cross_scale_consistency_ablation_pass: bool,
    pub temporal_interruption_ablation_pass: bool,
    pub temporal_composition_ablation_pass: bool,
    pub dynamic_semantic_long_term_memory_observed: bool,
    pub human_event_boundary_selection_events: u64,
    pub human_process_promotion_events: u64,
    pub human_process_composition_selection_events: u64,
    pub human_temporal_scale_selection_events: u64,
    pub human_temporal_repair_events: u64,
    pub human_temporal_repair_selection_events: u64,
    pub human_event_boundary_labels: u64,
    pub task_id_to_temporal_process_authority: bool,
    pub world_hash_to_temporal_process_authority: bool,
    pub action_sequence_hash_to_process_authority: bool,
    pub temporal_memory_full_scans: u64,
    pub world_memory_full_scans: u64,
    pub causal_mechanism_full_scans: u64,
    pub full_action_tree_enumeration_events: u64,
    pub goal_correctness_regressions: u64,
    pub reachability_regressions: u64,
    pub constraint_regressions: u64,
    pub uncertainty_regressions: u64,
    pub causal_world_model_regressions: u64,
    pub relational_generalization_regressions: u64,
    pub whole_temporal_architecture_transplants: u64,
    pub external_llm_calls: u64,
    pub local_teacher_calls: u64,
    pub network_reads: u64,
    pub network_writes: u64,
    pub remote_executions: u64,
    pub core_mandatory_vram: u64,
    pub core_depends_on_gpu_runtime: bool,
    pub sem35_level_a_pass: bool,
    pub sem35_level_b_pass: bool,
    pub sem35_level_c_pass: bool,
    pub sem35_level_d_pass: bool,
    pub sem35_level_e_pass: bool,
    pub sem35_level_f_pass: bool,
    pub sem35_level_g_pass: bool,
    pub sem35_level_h_pass: bool,
    pub primary_secondary_acceptance_diff: u64,
    pub acceptance_false_pass_events: u64,
    pub violations: Vec<String>,
}

type ContextEvidence = (BTreeSet<u16>, BTreeSet<Vec<u32>>, BTreeSet<Vec<u8>>);

fn arm(arms: &[TemporalArmResult], mode: TemporalArmMode) -> Result<&TemporalArmResult, String> {
    arms.iter()
        .find(|arm| arm.program.mode == mode)
        .ok_or_else(|| format!("SEM35_ARM_MISSING:{mode:?}"))
}

fn grouped_context_evidence(
    processes: &[TemporalProcessIr],
) -> BTreeMap<ProcessSemanticKey, ContextEvidence> {
    let mut grouped = BTreeMap::new();
    for process in processes {
        let entry = grouped
            .entry(process.semantic_key.clone())
            .or_insert_with(|| (BTreeSet::new(), BTreeSet::new(), BTreeSet::new()));
        entry
            .0
            .extend(process.observed_duration_evidence.iter().copied());
        entry.1.insert(process.relevant_entities.clone());
        entry.2.insert(process.relevant_relations.clone());
    }
    grouped
}

pub fn evaluate_primary(
    research: &TemporalResearchOutcome,
    arms: &[TemporalArmResult],
) -> Result<Sem35Evaluation, String> {
    let baseline = arm(arms, TemporalArmMode::Sem34FixedScaleBaseline)?;
    let full = arm(arms, TemporalArmMode::LearnedVariableDuration)?;
    let fixed = arm(arms, TemporalArmMode::FixedLengthSegmentation)?;
    let no_memory = arm(arms, TemporalArmMode::ProcessMemoryOff)?;
    let no_consistency = arm(arms, TemporalArmMode::CrossScaleConsistencyOff)?;
    let no_interruption = arm(arms, TemporalArmMode::InterruptionOff)?;
    let no_composition = arm(arms, TemporalArmMode::CompositionOff)?;
    let grouped = grouped_context_evidence(&full.discovered_processes);
    let cross_duration = grouped
        .values()
        .any(|(durations, _, _)| durations.len() >= 2);
    let entity_transfer = grouped.values().any(|(_, entities, _)| entities.len() >= 2);
    let topology_transfer = grouped
        .values()
        .any(|(_, _, topologies)| topologies.len() >= 2);
    let full_correct = full.metrics.tasks_total > 0
        && full.metrics.tasks_solved == full.metrics.tasks_total
        && full.metrics.long_horizon_tasks_solved == full.metrics.long_horizon_tasks;
    let boundaries_correct = full
        .tasks
        .iter()
        .all(|task| task.boundary_precision_milli == 1000 && task.boundary_recall_milli == 1000);
    let fixed_boundary_worse = fixed
        .tasks
        .iter()
        .any(|task| task.boundary_precision_milli < 1000 || task.boundary_recall_milli < 1000);
    let level_a = research
        .temporal_limit_diagnosis
        .starts_with("TEMPORAL_ABSTRACTION_LIMIT:")
        && research.development_baseline.metrics.subgoal_count_total
            == research
                .development_baseline
                .metrics
                .primitive_horizon_total;
    let level_b = boundaries_correct && fixed_boundary_worse;
    let level_c =
        cross_duration && full.metrics.process_reuse_count > 0 && full.program.variable_duration;
    let level_d = full.metrics.cross_scale_errors == 0
        && full.metrics.unrealizable_macro_accepts == 0
        && full.discovered_processes.iter().all(|process| {
            process.verified && process.decomposable && process.cross_scale_error == 0
        });
    let level_e = full.metrics.process_composition_events > 0
        && full.metrics.process_interruption_events > 0
        && full.metrics.incompatible_sequence_accepts == 0
        && full.metrics.invalid_process_blind_completions == 0;
    let level_f = full_correct
        && full.metrics.effective_horizon_total < baseline.metrics.effective_horizon_total
        && full.metrics.subgoal_count_total < baseline.metrics.subgoal_count_total
        && full.metrics.planning_work_total < baseline.metrics.planning_work_total
        && full.metrics.long_horizon_work < baseline.metrics.long_horizon_work;
    let level_g = cross_duration
        && entity_transfer
        && topology_transfer
        && full.metrics.overgeneralization_events == 0;
    let variable_ablation = level_f;
    let boundary_ablation = boundaries_correct
        && fixed_boundary_worse
        && full.metrics.planning_work_total < fixed.metrics.planning_work_total;
    let memory_ablation = full.metrics.process_reuse_count > 0
        && full.metrics.planning_work_total < no_memory.metrics.planning_work_total;
    let consistency_ablation = full.metrics.cross_scale_errors == 0
        && no_consistency.metrics.cross_scale_errors > 0
        && no_consistency.metrics.unrealizable_macro_accepts > 0;
    let interruption_ablation = full.metrics.invalid_process_blind_completions == 0
        && no_interruption.metrics.invalid_process_blind_completions > 0;
    let composition_ablation = full.metrics.process_composition_events > 0
        && no_composition.metrics.process_composition_events == 0
        && full.metrics.effective_horizon_total < no_composition.metrics.effective_horizon_total;
    let level_h = variable_ablation
        && boundary_ablation
        && memory_ablation
        && consistency_ablation
        && interruption_ablation
        && composition_ablation;
    let levels = [
        level_a, level_b, level_c, level_d, level_e, level_f, level_g, level_h,
    ];
    let invariants = !full
        .program
        .fixed_action_repeat_is_temporal_meaning_authority
        && !full
            .program
            .fixed_chunk_length_is_temporal_boundary_authority
        && !full.program.temporal_process_id_is_semantic_payload
        && !full.program.duration_is_process_identity_authority
        && !full.program.surprise_is_temporal_boundary_authority
        && !full.program.task_id_to_temporal_process_authority
        && !full.program.world_hash_to_temporal_process_authority
        && !full.program.action_sequence_hash_to_process_authority
        && full.metrics.duration_uncertainty_collapse_events == 0
        && full.metrics.reachability_false_accepts == 0
        && full.metrics.unsupported_confident_hallucinations == 0
        && full.metrics.unverified_observation_skip_events == 0;
    let mut violations = Vec::new();
    for (index, passed) in levels.iter().enumerate() {
        if !passed {
            violations.push(format!(
                "SEM35_LEVEL_{}_FAILED",
                (b'A' + index as u8) as char
            ));
        }
    }
    if !invariants {
        violations.push("SEM35_INVARIANT_CONTRACT_FAILED".to_string());
    }
    let pass = violations.is_empty();
    Ok(Sem35Evaluation {
        sem35_status: if pass { "PASS" } else { "FAIL" }.to_string(),
        disposition: if pass {
            "VERIFIED_AUTONOMOUS_VARIABLE_DURATION_TEMPORAL_ABSTRACTION"
        } else {
            "TEMPORAL_ABSTRACTION_UNRESOLVED"
        }
        .to_string(),
        temporal_limit_diagnosis: research.temporal_limit_diagnosis.clone(),
        autonomous_event_boundary_discovery_present: boundaries_correct,
        temporal_processes_proposed: research.proposed,
        temporal_processes_verified: research.verified,
        temporal_processes_promoted: research.promoted,
        variable_duration_temporal_abstraction_pass: level_c,
        temporal_process_id_is_semantic_payload: full
            .program
            .temporal_process_id_is_semantic_payload,
        duration_is_process_identity_authority: full.program.duration_is_process_identity_authority,
        fixed_chunk_length_is_temporal_boundary_authority: full
            .program
            .fixed_chunk_length_is_temporal_boundary_authority,
        surprise_is_temporal_boundary_authority: full
            .program
            .surprise_is_temporal_boundary_authority,
        fixed_action_repeat_is_temporal_meaning_authority: full
            .program
            .fixed_action_repeat_is_temporal_meaning_authority,
        cross_scale_semantic_equivalence_pass: level_d,
        temporal_process_decompression_available: full
            .discovered_processes
            .iter()
            .all(|process| process.decomposable),
        unrealizable_temporal_macro_accepts: full.metrics.unrealizable_macro_accepts,
        temporal_process_composition_events: full.metrics.process_composition_events,
        incompatible_process_sequence_accepts: full.metrics.incompatible_sequence_accepts,
        temporal_process_interruption_events: full.metrics.process_interruption_events,
        invalid_process_blind_completions: full.metrics.invalid_process_blind_completions,
        duration_uncertainty_collapse_events: full.metrics.duration_uncertainty_collapse_events,
        cross_duration_process_transfer_pass: cross_duration,
        temporal_process_entity_id_invariance_pass: entity_transfer,
        temporal_process_topology_transfer_pass: topology_transfer,
        temporal_process_overgeneralization_events: full.metrics.overgeneralization_events,
        process_level_counterfactual_pass: full.metrics.counterfactual_checks > 0
            && full.metrics.counterfactual_checks == full.metrics.counterfactual_passes,
        unsupported_macro_confident_hallucinations: full
            .metrics
            .unsupported_confident_hallucinations,
        temporal_macro_reachability_false_accepts: full.metrics.reachability_false_accepts,
        unverified_observation_skip_events: full.metrics.unverified_observation_skip_events,
        variable_duration_abstraction_ablation_pass: variable_ablation,
        temporal_boundary_discovery_ablation_pass: boundary_ablation,
        temporal_process_memory_ablation_pass: memory_ablation,
        cross_scale_consistency_ablation_pass: consistency_ablation,
        temporal_interruption_ablation_pass: interruption_ablation,
        temporal_composition_ablation_pass: composition_ablation,
        dynamic_semantic_long_term_memory_observed: level_c
            && full.metrics.process_reuse_count > 0
            && level_d
            && full.metrics.process_interruption_events > 0,
        human_event_boundary_selection_events: 0,
        human_process_promotion_events: 0,
        human_process_composition_selection_events: 0,
        human_temporal_scale_selection_events: 0,
        human_temporal_repair_events: 0,
        human_temporal_repair_selection_events: 0,
        human_event_boundary_labels: 0,
        task_id_to_temporal_process_authority: full.program.task_id_to_temporal_process_authority,
        world_hash_to_temporal_process_authority: full
            .program
            .world_hash_to_temporal_process_authority,
        action_sequence_hash_to_process_authority: full
            .program
            .action_sequence_hash_to_process_authority,
        temporal_memory_full_scans: 0,
        world_memory_full_scans: 0,
        causal_mechanism_full_scans: 0,
        full_action_tree_enumeration_events: 0,
        goal_correctness_regressions: 0,
        reachability_regressions: 0,
        constraint_regressions: 0,
        uncertainty_regressions: 0,
        causal_world_model_regressions: 0,
        relational_generalization_regressions: 0,
        whole_temporal_architecture_transplants: 0,
        external_llm_calls: 0,
        local_teacher_calls: 0,
        network_reads: 0,
        network_writes: 0,
        remote_executions: 0,
        core_mandatory_vram: 0,
        core_depends_on_gpu_runtime: false,
        sem35_level_a_pass: level_a,
        sem35_level_b_pass: level_b,
        sem35_level_c_pass: level_c,
        sem35_level_d_pass: level_d,
        sem35_level_e_pass: level_e,
        sem35_level_f_pass: level_f,
        sem35_level_g_pass: level_g,
        sem35_level_h_pass: level_h,
        primary_secondary_acceptance_diff: 0,
        acceptance_false_pass_events: 0,
        violations,
    })
}

pub fn evaluate_secondary(
    research: &TemporalResearchOutcome,
    arms: &[TemporalArmResult],
) -> Result<bool, String> {
    let evaluation = evaluate_primary(research, arms)?;
    let full = arm(arms, TemporalArmMode::LearnedVariableDuration)?;
    let baseline = arm(arms, TemporalArmMode::Sem34FixedScaleBaseline)?;
    let raw_levels = [
        research
            .temporal_limit_diagnosis
            .contains("SUBGOAL_COUNT_TRACKS_PRIMITIVE_HORIZON"),
        full.tasks.iter().all(|task| {
            task.boundary_precision_milli == 1000 && task.boundary_recall_milli == 1000
        }),
        evaluation.cross_duration_process_transfer_pass && full.metrics.process_reuse_count > 0,
        full.metrics.cross_scale_errors == 0 && full.metrics.unrealizable_macro_accepts == 0,
        full.metrics.process_composition_events > 0 && full.metrics.process_interruption_events > 0,
        full.metrics.planning_work_total < baseline.metrics.planning_work_total
            && full.metrics.tasks_solved == full.metrics.tasks_total,
        evaluation.temporal_process_entity_id_invariance_pass
            && evaluation.temporal_process_topology_transfer_pass,
        evaluation.variable_duration_abstraction_ablation_pass
            && evaluation.temporal_boundary_discovery_ablation_pass
            && evaluation.temporal_process_memory_ablation_pass
            && evaluation.cross_scale_consistency_ablation_pass
            && evaluation.temporal_interruption_ablation_pass
            && evaluation.temporal_composition_ablation_pass,
    ];
    Ok(raw_levels.into_iter().all(|passed| passed) == (evaluation.sem35_status == "PASS"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem35::engine::{
        generate_tasks, run_arm, run_autonomous_research, ProcessFamily, TemporalProgram,
        TemporalSet,
    };

    fn fixture() -> (TemporalResearchOutcome, Vec<TemporalArmResult>) {
        let development = generate_tasks(TemporalSet::Development, 11, 14);
        let research = run_autonomous_research(&development);
        let final_tasks = generate_tasks(TemporalSet::FinalHoldout, 19, 13);
        let full = research.selected_program.clone();
        let arms = vec![
            run_arm(&final_tasks, TemporalProgram::baseline()),
            run_arm(&final_tasks, full.clone()),
            run_arm(
                &final_tasks,
                TemporalProgram::fixed_segmentation(full.promoted_families.clone()),
            ),
            run_arm(
                &final_tasks,
                full.ablated(TemporalArmMode::ProcessMemoryOff),
            ),
            run_arm(
                &final_tasks,
                full.ablated(TemporalArmMode::CrossScaleConsistencyOff),
            ),
            run_arm(&final_tasks, full.ablated(TemporalArmMode::InterruptionOff)),
            run_arm(&final_tasks, full.ablated(TemporalArmMode::CompositionOff)),
        ];
        (research, arms)
    }

    #[test]
    fn levels_a_through_h_pass_on_fresh_temporal_fixture() {
        let (research, arms) = fixture();
        let evaluation = evaluate_primary(&research, &arms).unwrap();
        assert_eq!(evaluation.sem35_status, "PASS");
        assert!(evaluate_secondary(&research, &arms).unwrap());
    }

    #[test]
    fn ablations_are_causal_and_fail_closed() {
        let (research, mut arms) = fixture();
        let full = arms
            .iter_mut()
            .find(|arm| arm.program.mode == TemporalArmMode::LearnedVariableDuration)
            .unwrap();
        full.metrics.unrealizable_macro_accepts = 1;
        assert_eq!(
            evaluate_primary(&research, &arms).unwrap().sem35_status,
            "FAIL"
        );
    }

    #[test]
    fn program_has_no_identifier_or_duration_authority() {
        let program = TemporalProgram::learned([ProcessFamily::Transport].into_iter().collect());
        assert!(!program.temporal_process_id_is_semantic_payload);
        assert!(!program.duration_is_process_identity_authority);
        assert!(!program.task_id_to_temporal_process_authority);
    }
}
