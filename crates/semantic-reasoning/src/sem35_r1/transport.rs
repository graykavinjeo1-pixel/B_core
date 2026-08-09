use serde::{Deserialize, Serialize};

use crate::sem35::engine::{
    TemporalArmMetrics, TemporalArmResult, TemporalProcessIr, TemporalProgram, TemporalSet,
    TemporalTaskClass, TemporalTaskEvidence, TemporalWork,
};

use super::numeric::{CanonicalFiniteF64, ExactRational};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalTaskEvidence {
    pub task_id: u64,
    pub class: TemporalTaskClass,
    pub primitive_action_horizon: u64,
    pub effective_temporal_decision_horizon: u64,
    pub temporal_horizon_compression_ratio: ExactRational,
    pub temporal_horizon_compression_display: CanonicalFiniteF64,
    pub subgoal_count: u64,
    pub temporal_process_count: u64,
    pub temporal_process_durations: Vec<u16>,
    pub temporal_boundaries: Vec<u16>,
    pub temporal_process_reuse: u64,
    pub temporal_process_compositions: u64,
    pub temporal_interruptions: u64,
    pub cross_scale_errors: u64,
    pub planning_work: TemporalWork,
    pub world_model_calls: u64,
    pub causal_mechanism_calls: u64,
    pub temporal_process_lookup_cost: u64,
    pub active_temporal_processes: u64,
    pub goal_success: bool,
    pub boundary_precision_milli: u16,
    pub boundary_recall_milli: u16,
    pub unrealizable_macro_accepts: u64,
    pub incompatible_sequence_accepts: u64,
    pub invalid_process_blind_completions: u64,
    pub duration_uncertainty_collapse_events: u64,
    pub overgeneralization_events: u64,
    pub reachability_false_accepts: u64,
    pub unsupported_confident_hallucinations: u64,
    pub unverified_observation_skip_events: u64,
    pub primitive_step_as_fake_subgoal_events: u64,
}

impl TryFrom<TemporalTaskEvidence> for CanonicalTaskEvidence {
    type Error = String;

    fn try_from(task: TemporalTaskEvidence) -> Result<Self, Self::Error> {
        let ratio = ExactRational::new(
            task.primitive_action_horizon,
            task.effective_temporal_decision_horizon.max(1),
        )?;
        Ok(Self {
            task_id: task.task_id,
            class: task.class,
            primitive_action_horizon: task.primitive_action_horizon,
            effective_temporal_decision_horizon: task.effective_temporal_decision_horizon,
            temporal_horizon_compression_ratio: ratio,
            temporal_horizon_compression_display: CanonicalFiniteF64::new(ratio.to_display_f64())?,
            subgoal_count: task.subgoal_count,
            temporal_process_count: task.temporal_process_count,
            temporal_process_durations: task.temporal_process_durations,
            temporal_boundaries: task.temporal_boundaries,
            temporal_process_reuse: task.temporal_process_reuse,
            temporal_process_compositions: task.temporal_process_compositions,
            temporal_interruptions: task.temporal_interruptions,
            cross_scale_errors: task.cross_scale_errors,
            planning_work: task.planning_work,
            world_model_calls: task.world_model_calls,
            causal_mechanism_calls: task.causal_mechanism_calls,
            temporal_process_lookup_cost: task.temporal_process_lookup_cost,
            active_temporal_processes: task.active_temporal_processes,
            goal_success: task.goal_success,
            boundary_precision_milli: task.boundary_precision_milli,
            boundary_recall_milli: task.boundary_recall_milli,
            unrealizable_macro_accepts: task.unrealizable_macro_accepts,
            incompatible_sequence_accepts: task.incompatible_sequence_accepts,
            invalid_process_blind_completions: task.invalid_process_blind_completions,
            duration_uncertainty_collapse_events: task.duration_uncertainty_collapse_events,
            overgeneralization_events: task.overgeneralization_events,
            reachability_false_accepts: task.reachability_false_accepts,
            unsupported_confident_hallucinations: task.unsupported_confident_hallucinations,
            unverified_observation_skip_events: task.unverified_observation_skip_events,
            primitive_step_as_fake_subgoal_events: task.primitive_step_as_fake_subgoal_events,
        })
    }
}

impl CanonicalTaskEvidence {
    fn into_temporal(self) -> Result<TemporalTaskEvidence, String> {
        let expected = ExactRational::new(
            self.primitive_action_horizon,
            self.effective_temporal_decision_horizon.max(1),
        )?;
        if self.temporal_horizon_compression_ratio != expected
            || self.temporal_horizon_compression_display.bits()
                != CanonicalFiniteF64::new(expected.to_display_f64())?.bits()
        {
            return Err("INCONSISTENT_EXACT_DERIVED_TEMPORAL_RATIO".to_string());
        }
        Ok(TemporalTaskEvidence {
            task_id: self.task_id,
            class: self.class,
            primitive_action_horizon: self.primitive_action_horizon,
            effective_temporal_decision_horizon: self.effective_temporal_decision_horizon,
            temporal_horizon_compression_ratio: expected.to_display_f64(),
            subgoal_count: self.subgoal_count,
            temporal_process_count: self.temporal_process_count,
            temporal_process_durations: self.temporal_process_durations,
            temporal_boundaries: self.temporal_boundaries,
            temporal_process_reuse: self.temporal_process_reuse,
            temporal_process_compositions: self.temporal_process_compositions,
            temporal_interruptions: self.temporal_interruptions,
            cross_scale_errors: self.cross_scale_errors,
            planning_work: self.planning_work,
            world_model_calls: self.world_model_calls,
            causal_mechanism_calls: self.causal_mechanism_calls,
            temporal_process_lookup_cost: self.temporal_process_lookup_cost,
            active_temporal_processes: self.active_temporal_processes,
            goal_success: self.goal_success,
            boundary_precision_milli: self.boundary_precision_milli,
            boundary_recall_milli: self.boundary_recall_milli,
            unrealizable_macro_accepts: self.unrealizable_macro_accepts,
            incompatible_sequence_accepts: self.incompatible_sequence_accepts,
            invalid_process_blind_completions: self.invalid_process_blind_completions,
            duration_uncertainty_collapse_events: self.duration_uncertainty_collapse_events,
            overgeneralization_events: self.overgeneralization_events,
            reachability_false_accepts: self.reachability_false_accepts,
            unsupported_confident_hallucinations: self.unsupported_confident_hallucinations,
            unverified_observation_skip_events: self.unverified_observation_skip_events,
            primitive_step_as_fake_subgoal_events: self.primitive_step_as_fake_subgoal_events,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalTemporalArm {
    pub program: TemporalProgram,
    pub set: TemporalSet,
    pub tasks: Vec<CanonicalTaskEvidence>,
    pub metrics: TemporalArmMetrics,
    pub discovered_processes: Vec<TemporalProcessIr>,
    pub primitive_action_horizon_sequence: Vec<u64>,
    pub effective_temporal_decision_horizon_sequence: Vec<u64>,
    pub temporal_horizon_compression_ratio_sequence: Vec<ExactRational>,
    pub temporal_horizon_compression_display_sequence: Vec<CanonicalFiniteF64>,
    pub subgoal_count_sequence: Vec<u64>,
    pub temporal_process_count_sequence: Vec<u64>,
    pub temporal_process_duration_sequence: Vec<Vec<u16>>,
    pub temporal_boundary_sequence: Vec<Vec<u16>>,
    pub temporal_process_reuse_sequence: Vec<u64>,
    pub temporal_process_composition_sequence: Vec<u64>,
    pub temporal_interruption_sequence: Vec<u64>,
    pub cross_scale_error_sequence: Vec<u64>,
    pub planning_work_sequence: Vec<u64>,
    pub world_model_call_sequence: Vec<u64>,
    pub causal_mechanism_call_sequence: Vec<u64>,
    pub temporal_process_lookup_cost_sequence: Vec<u64>,
    pub active_temporal_process_sequence: Vec<u64>,
    pub goal_success_sequence: Vec<bool>,
}

impl TryFrom<TemporalArmResult> for CanonicalTemporalArm {
    type Error = String;

    fn try_from(arm: TemporalArmResult) -> Result<Self, Self::Error> {
        let tasks = arm
            .tasks
            .into_iter()
            .map(CanonicalTaskEvidence::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        let ratios = tasks
            .iter()
            .map(|task| task.temporal_horizon_compression_ratio)
            .collect::<Vec<_>>();
        let displays = tasks
            .iter()
            .map(|task| task.temporal_horizon_compression_display)
            .collect::<Vec<_>>();
        Ok(Self {
            program: arm.program,
            set: arm.set,
            tasks,
            metrics: arm.metrics,
            discovered_processes: arm.discovered_processes,
            primitive_action_horizon_sequence: arm.primitive_action_horizon_sequence,
            effective_temporal_decision_horizon_sequence: arm
                .effective_temporal_decision_horizon_sequence,
            temporal_horizon_compression_ratio_sequence: ratios,
            temporal_horizon_compression_display_sequence: displays,
            subgoal_count_sequence: arm.subgoal_count_sequence,
            temporal_process_count_sequence: arm.temporal_process_count_sequence,
            temporal_process_duration_sequence: arm.temporal_process_duration_sequence,
            temporal_boundary_sequence: arm.temporal_boundary_sequence,
            temporal_process_reuse_sequence: arm.temporal_process_reuse_sequence,
            temporal_process_composition_sequence: arm.temporal_process_composition_sequence,
            temporal_interruption_sequence: arm.temporal_interruption_sequence,
            cross_scale_error_sequence: arm.cross_scale_error_sequence,
            planning_work_sequence: arm.planning_work_sequence,
            world_model_call_sequence: arm.world_model_call_sequence,
            causal_mechanism_call_sequence: arm.causal_mechanism_call_sequence,
            temporal_process_lookup_cost_sequence: arm.temporal_process_lookup_cost_sequence,
            active_temporal_process_sequence: arm.active_temporal_process_sequence,
            goal_success_sequence: arm.goal_success_sequence,
        })
    }
}

impl CanonicalTemporalArm {
    pub fn into_temporal(self) -> Result<TemporalArmResult, String> {
        let tasks = self
            .tasks
            .into_iter()
            .map(CanonicalTaskEvidence::into_temporal)
            .collect::<Result<Vec<_>, _>>()?;
        let ratios = tasks
            .iter()
            .map(|task| task.temporal_horizon_compression_ratio)
            .collect::<Vec<_>>();
        if self.temporal_horizon_compression_ratio_sequence.len() != ratios.len()
            || self.temporal_horizon_compression_display_sequence.len() != ratios.len()
        {
            return Err("TEMPORAL_RATIO_SEQUENCE_LENGTH_MISMATCH".to_string());
        }
        for (index, ratio) in self
            .temporal_horizon_compression_ratio_sequence
            .iter()
            .enumerate()
        {
            let expected = ExactRational::new(
                tasks[index].primitive_action_horizon,
                tasks[index].effective_temporal_decision_horizon.max(1),
            )?;
            if *ratio != expected
                || self.temporal_horizon_compression_display_sequence[index].bits()
                    != CanonicalFiniteF64::new(expected.to_display_f64())?.bits()
            {
                return Err("TEMPORAL_RATIO_SEQUENCE_AUTHORITY_MISMATCH".to_string());
            }
        }
        Ok(TemporalArmResult {
            program: self.program,
            set: self.set,
            tasks,
            metrics: self.metrics,
            discovered_processes: self.discovered_processes,
            primitive_action_horizon_sequence: self.primitive_action_horizon_sequence,
            effective_temporal_decision_horizon_sequence: self
                .effective_temporal_decision_horizon_sequence,
            temporal_horizon_compression_ratio_sequence: ratios,
            subgoal_count_sequence: self.subgoal_count_sequence,
            temporal_process_count_sequence: self.temporal_process_count_sequence,
            temporal_process_duration_sequence: self.temporal_process_duration_sequence,
            temporal_boundary_sequence: self.temporal_boundary_sequence,
            temporal_process_reuse_sequence: self.temporal_process_reuse_sequence,
            temporal_process_composition_sequence: self.temporal_process_composition_sequence,
            temporal_interruption_sequence: self.temporal_interruption_sequence,
            cross_scale_error_sequence: self.cross_scale_error_sequence,
            planning_work_sequence: self.planning_work_sequence,
            world_model_call_sequence: self.world_model_call_sequence,
            causal_mechanism_call_sequence: self.causal_mechanism_call_sequence,
            temporal_process_lookup_cost_sequence: self.temporal_process_lookup_cost_sequence,
            active_temporal_process_sequence: self.active_temporal_process_sequence,
            goal_success_sequence: self.goal_success_sequence,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use crate::sem35::engine::{
        generate_tasks, run_arm, ProcessFamily, TemporalProgram, TemporalSet,
    };

    use super::*;

    #[test]
    fn temporal_arm_json_roundtrip_uses_exact_ratio_authority() {
        let tasks = generate_tasks(TemporalSet::FinalHoldout, 19, 13);
        let program = TemporalProgram::learned(
            [ProcessFamily::Transport, ProcessFamily::Incubate]
                .into_iter()
                .collect::<BTreeSet<_>>(),
        );
        let canonical = CanonicalTemporalArm::try_from(run_arm(&tasks, program)).unwrap();
        let bytes = serde_json::to_vec(&canonical).unwrap();
        let decoded: CanonicalTemporalArm = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded, canonical);
        assert_eq!(decoded.clone().into_temporal().unwrap().tasks.len(), 13);
        assert!(decoded
            .temporal_horizon_compression_ratio_sequence
            .iter()
            .all(|ratio| ratio.denominator() != 0));
    }
}
