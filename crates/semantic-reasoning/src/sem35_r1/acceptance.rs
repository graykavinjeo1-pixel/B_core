use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::sem35::engine::{
    ProcessSemanticKey, TemporalArmMode, TemporalArmResult, TemporalProcessIr,
    TemporalResearchOutcome,
};

type ContextEvidence = (BTreeSet<u16>, BTreeSet<Vec<u32>>, BTreeSet<Vec<u8>>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecondaryAcceptance {
    pub sem35_r1_status: String,
    pub level_a_pass: bool,
    pub level_b_pass: bool,
    pub level_c_pass: bool,
    pub level_d_pass: bool,
    pub level_e_pass: bool,
    pub level_f_pass: bool,
    pub level_g_pass: bool,
    pub level_h_pass: bool,
    pub invariants_pass: bool,
    pub violations: Vec<String>,
}

fn required_arm(
    arms: &[TemporalArmResult],
    mode: TemporalArmMode,
) -> Result<&TemporalArmResult, String> {
    arms.iter()
        .find(|arm| arm.program.mode == mode)
        .ok_or_else(|| format!("SEM35_R1_SECONDARY_ARM_MISSING:{mode:?}"))
}

fn collect_context_evidence(
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

/// Independent raw-field implementation of Levels A-H. This deliberately does
/// not call the primary evaluator or consume any of its derived booleans.
pub fn evaluate_secondary_independent(
    research: &TemporalResearchOutcome,
    arms: &[TemporalArmResult],
) -> Result<SecondaryAcceptance, String> {
    let baseline = required_arm(arms, TemporalArmMode::Sem34FixedScaleBaseline)?;
    let full = required_arm(arms, TemporalArmMode::LearnedVariableDuration)?;
    let fixed = required_arm(arms, TemporalArmMode::FixedLengthSegmentation)?;
    let no_memory = required_arm(arms, TemporalArmMode::ProcessMemoryOff)?;
    let no_consistency = required_arm(arms, TemporalArmMode::CrossScaleConsistencyOff)?;
    let no_interruption = required_arm(arms, TemporalArmMode::InterruptionOff)?;
    let no_composition = required_arm(arms, TemporalArmMode::CompositionOff)?;

    let contexts = collect_context_evidence(&full.discovered_processes);
    let cross_duration = contexts
        .values()
        .any(|(durations, _, _)| durations.len() >= 2);
    let entity_transfer = contexts
        .values()
        .any(|(_, entities, _)| entities.len() >= 2);
    let topology_transfer = contexts
        .values()
        .any(|(_, _, topologies)| topologies.len() >= 2);
    let boundaries_correct = full
        .tasks
        .iter()
        .all(|task| task.boundary_precision_milli == 1000 && task.boundary_recall_milli == 1000);
    let fixed_boundaries_worse = fixed
        .tasks
        .iter()
        .any(|task| task.boundary_precision_milli < 1000 || task.boundary_recall_milli < 1000);
    let full_correct = full.metrics.tasks_total > 0
        && full.metrics.tasks_solved == full.metrics.tasks_total
        && full.metrics.long_horizon_tasks_solved == full.metrics.long_horizon_tasks;

    let level_a = research
        .temporal_limit_diagnosis
        .starts_with("TEMPORAL_ABSTRACTION_LIMIT:")
        && research.development_baseline.metrics.subgoal_count_total
            == research
                .development_baseline
                .metrics
                .primitive_horizon_total;
    let level_b = boundaries_correct && fixed_boundaries_worse;
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
        && fixed_boundaries_worse
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
    let levels = [
        level_a, level_b, level_c, level_d, level_e, level_f, level_g, level_h,
    ];
    let mut violations = Vec::new();
    for (index, passed) in levels.iter().enumerate() {
        if !passed {
            violations.push(format!(
                "SEM35_R1_SECONDARY_LEVEL_{}_FAILED",
                (b'A' + index as u8) as char
            ));
        }
    }
    if !invariants {
        violations.push("SEM35_R1_SECONDARY_INVARIANTS_FAILED".to_string());
    }
    Ok(SecondaryAcceptance {
        sem35_r1_status: if violations.is_empty() {
            "PASS"
        } else {
            "FAIL"
        }
        .to_string(),
        level_a_pass: level_a,
        level_b_pass: level_b,
        level_c_pass: level_c,
        level_d_pass: level_d,
        level_e_pass: level_e,
        level_f_pass: level_f,
        level_g_pass: level_g,
        level_h_pass: level_h,
        invariants_pass: invariants,
        violations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem35::engine::{
        generate_tasks, run_arm, run_autonomous_research, TemporalProgram, TemporalSet,
    };

    #[test]
    fn independent_secondary_accepts_complete_fixture() {
        let development = generate_tasks(TemporalSet::Development, 11, 14);
        let research = run_autonomous_research(&development);
        let tasks = generate_tasks(TemporalSet::FinalHoldout, 19, 13);
        let full = research.selected_program.clone();
        let arms = vec![
            run_arm(&tasks, TemporalProgram::baseline()),
            run_arm(&tasks, full.clone()),
            run_arm(
                &tasks,
                TemporalProgram::fixed_segmentation(full.promoted_families.clone()),
            ),
            run_arm(&tasks, full.ablated(TemporalArmMode::ProcessMemoryOff)),
            run_arm(
                &tasks,
                full.ablated(TemporalArmMode::CrossScaleConsistencyOff),
            ),
            run_arm(&tasks, full.ablated(TemporalArmMode::InterruptionOff)),
            run_arm(&tasks, full.ablated(TemporalArmMode::CompositionOff)),
        ];
        let result = evaluate_secondary_independent(&research, &arms).unwrap();
        assert_eq!(result.sem35_r1_status, "PASS");
    }
}
