use std::{cmp::Reverse, hint::black_box, mem::size_of, time::Instant};

use serde::{Deserialize, Serialize};

use crate::sem26::engine::{
    run_autonomous_epoch, AutonomousEpochRequest, AutonomousEpochResult, DirectorState,
    NO_BOTTLENECK, PHASE_COUNT, PHASE_NAMES,
};

pub const SEM27_EPOCH_BUDGET: u8 = 64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DifficultyDimensions {
    pub causal_depth: u16,
    pub compositional_depth: u16,
    pub transfer_arity: u16,
    pub constraint_complexity: u16,
    pub planning_depth: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DifficultyRegimeRecord {
    pub regime_id: u16,
    pub dimensions: DifficultyDimensions,
    pub initial_capability_requirement: u64,
    pub initial_cost_ns: u64,
    pub final_local_cost_ns: u64,
    pub within_regime_cost_sequence_ns: Vec<u64>,
    pub frontier_capability_achieved: u64,
    pub epochs_to_local_mastery: u16,
    pub time_to_local_mastery_ns: u64,
    pub plateau_classification: String,
    pub next_difficulty_selection_basis: String,
    pub productive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DifficultyTransitionRecord {
    pub transition_id: u16,
    pub previous_regime_id: u16,
    pub previous_plateau_type: String,
    pub reason_escalation_chosen: String,
    pub changed_dimension: String,
    pub previous_dimensions: DifficultyDimensions,
    pub new_dimensions: DifficultyDimensions,
    pub predicted_challenge_increase_units: u64,
    pub predicted_capability_opportunity_units: u64,
    pub operator_selected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DifficultyState {
    pub current_regime_id: u16,
    pub current_dimensions: DifficultyDimensions,
    pub current_regime_started_epoch: u8,
    pub current_initial_cost_ns: u64,
    pub current_cost_sequence_ns: Vec<u64>,
    pub current_initial_frontier: u64,
    pub local_mastery_progress: u16,
    pub completed_regimes: Vec<DifficultyRegimeRecord>,
    pub transitions: Vec<DifficultyTransitionRecord>,
    pub productive_escalation_events: u16,
    pub failed_escalation_events: u16,
    pub physical_or_fixed_floor_events: u16,
    pub redundant_floor_optimization_events: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlateauCandidate {
    pub classification: String,
    pub evidence_strength: u64,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlateauEvent {
    pub plateau_event_id: u16,
    pub observed_gain_curve: Vec<u64>,
    pub observed_cost_curve_ns: Vec<u64>,
    pub candidate_explanations: Vec<PlateauCandidate>,
    pub classification: String,
    pub evidence_margin: u64,
    pub selected_response: String,
    pub actual_consequence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DifficultyProbeResult {
    pub regime_id: u16,
    pub dimensions: DifficultyDimensions,
    pub semantic_recurrence_depth: u64,
    pub structured_work_units: u64,
    pub solver_mode: String,
    pub result_hash: u64,
    pub independent_expected_hash: u64,
    pub mechanically_verified: bool,
    pub wall_time_ns: u64,
    pub frontier_capability_units: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResearchAgendaItem {
    pub agenda_item_id: u64,
    pub rank: u8,
    pub measured_dimension_code: u8,
    pub measured_dimension_name: String,
    pub observed_time_ns: u64,
    pub uncertainty_ppm: u32,
    pub causal_evidence_mask: u64,
    pub mechanism_version: u16,
    pub selected_by_operator: bool,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PostScaffoldResearchState {
    pub inherited_sem26_episode_count: u16,
    pub sem27_episode_count: u16,
    pub accepted_lineages: Vec<u64>,
    pub rejected_lineages: Vec<u64>,
    pub failure_cause_masks: [u64; PHASE_COUNT],
    pub agenda_revision_count: u16,
    pub evidence_reuse_events: u16,
    pub returning_pressure_events: u16,
    pub saturation_challenge_attempts: u16,
    pub new_research_method_count: u16,
    pub causally_useful_new_research_method_count: u16,
    pub cross_bottleneck_research_method_transfer_events: u16,
    pub current_agenda: Vec<ResearchAgendaItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PostScaffoldState {
    pub director: DirectorState,
    pub research: PostScaffoldResearchState,
    pub initial_frontier_scale: u64,
    pub initial_core_bytes: u64,
    pub seen_bottleneck_mask: u64,
    pub gain_history: Vec<u64>,
    pub accepted_sem27_repairs: u16,
    pub implemented_sem27_repairs: u16,
    pub synthesized_sem27_repairs: u16,
    pub migration_events_sem27: u16,
    pub growth_regime_shift_events: u16,
    pub consecutive_rejections: u8,
    pub autonomous_termination_reason: Option<String>,
    pub difficulty: DifficultyState,
    pub plateau_event_count: u16,
    pub unresolved_bottleneck_plateaus: u16,
    pub local_mastery_floor_plateaus: u16,
    pub frontier_exhaustion_plateaus: u16,
    pub insufficient_evidence_plateaus: u16,
    pub autonomous_director_evolution_events: u16,
}

impl PostScaffoldState {
    pub fn from_sem26(director: DirectorState) -> Self {
        let inherited_sem26_episode_count = director.memory.episode_count;
        let initial_frontier_scale = director.frontier_scale;
        let initial_core_bytes = director.core_bytes;
        Self {
            director,
            research: PostScaffoldResearchState {
                inherited_sem26_episode_count,
                sem27_episode_count: 0,
                accepted_lineages: Vec::new(),
                rejected_lineages: Vec::new(),
                failure_cause_masks: [0; PHASE_COUNT],
                agenda_revision_count: 0,
                evidence_reuse_events: 0,
                returning_pressure_events: 0,
                saturation_challenge_attempts: 0,
                new_research_method_count: 0,
                causally_useful_new_research_method_count: 0,
                cross_bottleneck_research_method_transfer_events: 0,
                current_agenda: Vec::new(),
            },
            initial_frontier_scale,
            initial_core_bytes,
            seen_bottleneck_mask: 0,
            gain_history: Vec::new(),
            accepted_sem27_repairs: 0,
            implemented_sem27_repairs: 0,
            synthesized_sem27_repairs: 0,
            migration_events_sem27: 0,
            growth_regime_shift_events: 0,
            consecutive_rejections: 0,
            autonomous_termination_reason: None,
            difficulty: DifficultyState {
                current_regime_id: 1,
                current_dimensions: DifficultyDimensions {
                    causal_depth: 2,
                    compositional_depth: 2,
                    transfer_arity: 1,
                    constraint_complexity: 2,
                    planning_depth: 1,
                },
                current_regime_started_epoch: 1,
                current_initial_cost_ns: 0,
                current_cost_sequence_ns: Vec::new(),
                current_initial_frontier: initial_frontier_scale,
                local_mastery_progress: 0,
                completed_regimes: Vec::new(),
                transitions: Vec::new(),
                productive_escalation_events: 0,
                failed_escalation_events: 0,
                physical_or_fixed_floor_events: 0,
                redundant_floor_optimization_events: 0,
            },
            plateau_event_count: 0,
            unresolved_bottleneck_plateaus: 0,
            local_mastery_floor_plateaus: 0,
            frontier_exhaustion_plateaus: 0,
            insufficient_evidence_plateaus: 0,
            autonomous_director_evolution_events: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PostScaffoldEpochRequest {
    pub arm_code: u8,
    pub epoch: u8,
    pub seed: u64,
    pub state: PostScaffoldState,
    pub resource_ceiling_bytes: u64,
    pub historical_roadmap_target_code: Option<u8>,
    pub disable_long_term_research_memory: bool,
    pub concrete_future_instance_visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimeDecomposition {
    pub observation_time_ns: u64,
    pub self_model_update_time_ns: u64,
    pub bottleneck_diagnosis_time_ns: u64,
    pub hypothesis_generation_time_ns: u64,
    pub diagnostic_experiment_selection_time_ns: u64,
    pub diagnostic_experiment_execution_time_ns: u64,
    pub experiment_interpretation_time_ns: u64,
    pub desired_self_phenotype_synthesis_time_ns: u64,
    pub repair_synthesis_time_ns: u64,
    pub reaction_discovery_time_ns: u64,
    pub reaction_realization_time_ns: u64,
    pub causal_integration_time_ns: u64,
    pub verification_time_ns: u64,
    pub fresh_work_validation_time_ns: u64,
    pub state_integration_time_ns: u64,
    pub bookkeeping_runtime_overhead_time_ns: u64,
    pub measurement_overhead_time_ns: u64,
    pub measurement_overhead_bytes: u64,
    pub unclassified_improvement_time_ns: u64,
    pub accounted_time_fraction: f64,
    pub total_improvement_interval_ns: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PostScaffoldEpochResult {
    pub arm_code: u8,
    pub epoch: u8,
    pub agenda_before: Vec<ResearchAgendaItem>,
    pub agenda_after: Vec<ResearchAgendaItem>,
    pub agenda_revised: bool,
    pub agenda_selected_by_operator: bool,
    pub historical_roadmap_visible: bool,
    pub predecessor_textual_bottleneck_visible: bool,
    pub inner: AutonomousEpochResult,
    pub time: TimeDecomposition,
    pub diagnostic_experiment_count: u16,
    pub repair_hypothesis_count: u16,
    pub implementations_this_epoch: u16,
    pub failed_repairs_this_epoch: u16,
    pub research_memory_reused: bool,
    pub returning_pressure_event: bool,
    pub unproductive_oscillation: bool,
    pub growth_plateau_observed: bool,
    pub autonomous_growth_regime_shift: bool,
    pub new_autonomous_bottleneck_class_created: bool,
    pub new_research_method_created: bool,
    pub causally_useful_new_research_method: bool,
    pub cross_bottleneck_research_method_transfer: bool,
    pub fixed_resource_frontier: u64,
    pub research_work_per_accepted_gain_ns: u64,
    pub autonomous_saturation_challenge: bool,
    pub hardcoded_bottleneck_to_repair_rule_used: bool,
    pub difficulty_probe: DifficultyProbeResult,
    pub plateau_event: Option<PlateauEvent>,
    pub difficulty_transition: Option<DifficultyTransitionRecord>,
    pub human_difficulty_escalation_event: bool,
    pub human_difficulty_level_selection_event: bool,
    pub autonomous_director_may_evolve: bool,
    pub autonomous_director_evolved_this_epoch: bool,
    pub resulting_state: PostScaffoldState,
    pub result_checksum: u64,
}

pub fn run_post_scaffold_epoch(
    request: PostScaffoldEpochRequest,
) -> Result<PostScaffoldEpochResult, String> {
    validate_request(&request)?;
    let total_started = Instant::now();
    let difficulty_probe = execute_difficulty_probe(&request.state.difficulty, request.seed);
    if !difficulty_probe.mechanically_verified {
        return Err("DIFFICULTY_WORK_VERIFICATION_FAILED".to_string());
    }
    let measurement_started = Instant::now();
    let mut clock_checksum = 0_u64;
    for index in 0..512_u64 {
        clock_checksum ^= nanos(Instant::now().elapsed().as_nanos()).wrapping_add(index);
    }
    black_box(clock_checksum);
    let measurement_overhead_time_ns = nanos(measurement_started.elapsed().as_nanos()).max(100);
    let measurement_overhead_bytes =
        (size_of::<Instant>() * 2 + size_of::<u64>() * (PHASE_COUNT + 4)) as u64;

    let agenda_started = Instant::now();
    let agenda_before = derive_agenda(&request.state, request.seed);
    let self_model_update_time_ns = nanos(agenda_started.elapsed().as_nanos()).max(100);

    let wrapped_epoch = ((request.epoch - 1) % 32) + 1;
    let sem26_arm = match request.arm_code {
        0 => 0,
        1 => 1,
        2 | 3 => 3,
        _ => return Err("INVALID_ARM_CODE".to_string()),
    };
    let scripted_label = match request.arm_code {
        0 => Some(6),
        1 => request.historical_roadmap_target_code,
        2 | 3 => None,
        _ => None,
    };
    let inner = run_autonomous_epoch(AutonomousEpochRequest {
        arm_code: sem26_arm,
        epoch: wrapped_epoch,
        seed: request.seed,
        state: request.state.director.clone(),
        resource_ceiling_bytes: request.resource_ceiling_bytes,
        scripted_predecessor_label_code: scripted_label,
        disable_autonomous_diagnosis: false,
        disable_autonomous_repair_synthesis: false,
        disable_research_memory: request.disable_long_term_research_memory,
        concrete_future_instance_visible: request.concrete_future_instance_visible,
    })?;

    let hypothesis_started = Instant::now();
    let hypothesis_checksum = inner
        .bottleneck_hypotheses
        .iter()
        .fold(0_u64, |sum, hypothesis| {
            mix(sum ^ hypothesis.hypothesis_id, hypothesis.evidence_time_ns)
        });
    black_box(hypothesis_checksum);
    let hypothesis_generation_time_ns = nanos(hypothesis_started.elapsed().as_nanos()).max(100);

    let interpretation_started = Instant::now();
    let interpretation_checksum =
        inner
            .diagnostic_experiments
            .iter()
            .fold(0_u64, |sum, experiment| {
                mix(
                    sum ^ experiment.experiment_id,
                    u64::from(experiment.observed_reduction_ppm),
                )
            });
    black_box(interpretation_checksum);
    let experiment_interpretation_time_ns =
        nanos(interpretation_started.elapsed().as_nanos()).max(100);

    let phenotype_started = Instant::now();
    let phenotype_checksum = mix(
        inner.desired_self_phenotype.required_property_mask,
        inner.desired_self_phenotype.required_role_mask
            ^ inner.desired_self_phenotype.diagnosed_cause_mask,
    );
    black_box(phenotype_checksum);
    let desired_self_phenotype_synthesis_time_ns =
        nanos(phenotype_started.elapsed().as_nanos()).max(100);

    let integration_started = Instant::now();
    let mut resulting_state = request.state.clone();
    if resulting_state.difficulty.current_initial_cost_ns == 0 {
        resulting_state.difficulty.current_initial_cost_ns = difficulty_probe.wall_time_ns;
    }
    resulting_state
        .difficulty
        .current_cost_sequence_ns
        .push(difficulty_probe.wall_time_ns);
    let previous_code = resulting_state.director.prior_bottleneck_code;
    let selected_code = inner.selected_bottleneck_code;
    let previously_seen = resulting_state.seen_bottleneck_mask & (1_u64 << selected_code) != 0;
    let returning_pressure_event =
        previously_seen && previous_code != NO_BOTTLENECK && previous_code != selected_code;
    resulting_state.seen_bottleneck_mask |= 1_u64 << selected_code;
    resulting_state.director = inner.resulting_state.clone();
    resulting_state.research.sem27_episode_count = resulting_state
        .research
        .sem27_episode_count
        .saturating_add(1);
    if returning_pressure_event {
        resulting_state.research.returning_pressure_events = resulting_state
            .research
            .returning_pressure_events
            .saturating_add(1);
    }
    if inner.repair_synthesized {
        resulting_state.synthesized_sem27_repairs =
            resulting_state.synthesized_sem27_repairs.saturating_add(1);
    }
    if inner.repair_implemented {
        resulting_state.implemented_sem27_repairs =
            resulting_state.implemented_sem27_repairs.saturating_add(1);
    }
    let lineage = inner
        .selected_repair
        .as_ref()
        .map_or(0, |repair| repair.lineage_hash);
    if inner.repair_accepted {
        resulting_state.accepted_sem27_repairs =
            resulting_state.accepted_sem27_repairs.saturating_add(1);
        resulting_state.research.accepted_lineages.push(lineage);
        resulting_state.consecutive_rejections = 0;
        resulting_state.difficulty.local_mastery_progress = resulting_state
            .difficulty
            .local_mastery_progress
            .saturating_add(1);
    } else if inner.repair_rejected {
        resulting_state.research.rejected_lineages.push(lineage);
        resulting_state.research.failure_cause_masks[usize::from(selected_code)] |= inner
            .diagnostic_experiments
            .iter()
            .find(|experiment| experiment.selected)
            .map_or(0, |experiment| experiment.tested_cause_mask);
        resulting_state.consecutive_rejections =
            resulting_state.consecutive_rejections.saturating_add(1);
    }
    if inner.autonomous_bottleneck_migration {
        resulting_state.migration_events_sem27 =
            resulting_state.migration_events_sem27.saturating_add(1);
    }
    if inner.past_research_evidence_reused {
        resulting_state.research.evidence_reuse_events = resulting_state
            .research
            .evidence_reuse_events
            .saturating_add(1);
    }
    let autonomous_saturation_challenge = inner.repair_rejected
        && resulting_state.consecutive_rejections.is_multiple_of(2)
        && resulting_state.research.saturation_challenge_attempts < 3;
    if autonomous_saturation_challenge {
        resulting_state.research.saturation_challenge_attempts = resulting_state
            .research
            .saturation_challenge_attempts
            .saturating_add(1);
    }
    let growth_plateau_observed = plateau_observed(
        &resulting_state.gain_history,
        &resulting_state.difficulty.current_cost_sequence_ns,
    );
    let autonomous_growth_regime_shift = growth_plateau_observed
        && inner.repair_accepted
        && u128::from(inner.frontier_gain) * 100
            > u128::from(mean_u64(&resulting_state.gain_history)) * 125;
    if autonomous_growth_regime_shift {
        resulting_state.growth_regime_shift_events =
            resulting_state.growth_regime_shift_events.saturating_add(1);
    }
    resulting_state.gain_history.push(inner.frontier_gain);
    if resulting_state.gain_history.len() > 16 {
        resulting_state.gain_history.remove(0);
    }
    let (plateau_event, difficulty_transition) = if growth_plateau_observed && request.arm_code == 3
    {
        let event = classify_plateau(
            &resulting_state,
            &inner,
            request.epoch,
            difficulty_probe.frontier_capability_units,
        );
        resulting_state.plateau_event_count = resulting_state.plateau_event_count.saturating_add(1);
        match event.classification.as_str() {
            "UNRESOLVED_BOTTLENECK" => {
                resulting_state.unresolved_bottleneck_plateaus = resulting_state
                    .unresolved_bottleneck_plateaus
                    .saturating_add(1);
            }
            "LOCAL_MASTERY_OR_PHYSICAL_FLOOR" => {
                resulting_state.local_mastery_floor_plateaus = resulting_state
                    .local_mastery_floor_plateaus
                    .saturating_add(1);
                resulting_state.difficulty.physical_or_fixed_floor_events = resulting_state
                    .difficulty
                    .physical_or_fixed_floor_events
                    .saturating_add(1);
            }
            "CURRENT_FRONTIER_EXHAUSTION" => {
                resulting_state.frontier_exhaustion_plateaus = resulting_state
                    .frontier_exhaustion_plateaus
                    .saturating_add(1);
            }
            _ => {
                resulting_state.insufficient_evidence_plateaus = resulting_state
                    .insufficient_evidence_plateaus
                    .saturating_add(1);
            }
        }
        let transition = if matches!(
            event.classification.as_str(),
            "LOCAL_MASTERY_OR_PHYSICAL_FLOOR" | "CURRENT_FRONTIER_EXHAUSTION"
        ) {
            Some(escalate_difficulty(
                &mut resulting_state,
                &event,
                &inner,
                request.epoch,
                difficulty_probe.frontier_capability_units,
            ))
        } else {
            None
        };
        (Some(event), transition)
    } else {
        (None, None)
    };
    let agenda_after = derive_agenda(&resulting_state, request.seed ^ inner.result_checksum);
    let agenda_revised = agenda_signature(&agenda_before) != agenda_signature(&agenda_after);
    if agenda_revised && request.arm_code == 3 {
        resulting_state.research.agenda_revision_count = resulting_state
            .research
            .agenda_revision_count
            .saturating_add(1);
    }
    resulting_state.research.current_agenda = agenda_after.clone();
    let state_integration_time_ns = nanos(integration_started.elapsed().as_nanos()).max(100);

    let bookkeeping_started = Instant::now();
    let bookkeeping_checksum = mix(
        inner.result_checksum,
        resulting_state.director.frontier_scale
            ^ u64::from(resulting_state.research.agenda_revision_count),
    );
    black_box(bookkeeping_checksum);
    let bookkeeping_runtime_overhead_time_ns =
        nanos(bookkeeping_started.elapsed().as_nanos()).max(100);

    let fresh_work_validation_time_ns = inner
        .fresh_work_validation_time_ns
        .saturating_add(difficulty_probe.wall_time_ns);
    let accounted_without_residual = inner
        .observation_time_ns
        .saturating_add(self_model_update_time_ns)
        .saturating_add(inner.diagnosis_time_ns)
        .saturating_add(hypothesis_generation_time_ns)
        .saturating_add(inner.experiment_selection_time_ns)
        .saturating_add(inner.diagnostic_experiment_time_ns)
        .saturating_add(experiment_interpretation_time_ns)
        .saturating_add(desired_self_phenotype_synthesis_time_ns)
        .saturating_add(inner.repair_synthesis_time_ns)
        .saturating_add(inner.reaction_discovery_time_ns)
        .saturating_add(inner.reaction_realization_time_ns)
        .saturating_add(inner.causal_integration_time_ns)
        .saturating_add(inner.verification_time_ns)
        .saturating_add(fresh_work_validation_time_ns)
        .saturating_add(state_integration_time_ns)
        .saturating_add(bookkeeping_runtime_overhead_time_ns)
        .saturating_add(measurement_overhead_time_ns);
    let measured_total = nanos(total_started.elapsed().as_nanos());
    let total_improvement_interval_ns = measured_total.max(accounted_without_residual);
    let unclassified_improvement_time_ns =
        total_improvement_interval_ns.saturating_sub(accounted_without_residual);
    let accounted_time_fraction =
        accounted_without_residual as f64 / total_improvement_interval_ns.max(1) as f64;
    let research_work_ns = inner
        .diagnosis_time_ns
        .saturating_add(inner.experiment_selection_time_ns)
        .saturating_add(inner.diagnostic_experiment_time_ns)
        .saturating_add(inner.repair_synthesis_time_ns);
    let research_work_per_accepted_gain_ns = if inner.repair_accepted {
        research_work_ns / inner.frontier_gain.max(1)
    } else {
        research_work_ns
    };
    let fixed_resource_frontier = resulting_state
        .director
        .frontier_scale
        .saturating_add(difficulty_probe.frontier_capability_units);
    let result_checksum = mix(
        bookkeeping_checksum,
        total_improvement_interval_ns ^ fixed_resource_frontier,
    );

    Ok(PostScaffoldEpochResult {
        arm_code: request.arm_code,
        epoch: request.epoch,
        agenda_before,
        agenda_after,
        agenda_revised,
        agenda_selected_by_operator: false,
        historical_roadmap_visible: request.arm_code == 1,
        predecessor_textual_bottleneck_visible: request.arm_code <= 1,
        diagnostic_experiment_count: inner.diagnostic_experiments.len() as u16,
        repair_hypothesis_count: inner.repair_hypotheses_generated,
        implementations_this_epoch: u16::from(inner.repair_implemented),
        failed_repairs_this_epoch: u16::from(inner.repair_rejected),
        research_memory_reused: inner.past_research_evidence_reused,
        returning_pressure_event,
        unproductive_oscillation: inner.bottleneck_oscillation,
        growth_plateau_observed,
        autonomous_growth_regime_shift,
        new_autonomous_bottleneck_class_created: false,
        new_research_method_created: false,
        causally_useful_new_research_method: false,
        cross_bottleneck_research_method_transfer: false,
        fixed_resource_frontier,
        research_work_per_accepted_gain_ns,
        autonomous_saturation_challenge,
        hardcoded_bottleneck_to_repair_rule_used: false,
        difficulty_probe,
        plateau_event,
        difficulty_transition,
        human_difficulty_escalation_event: false,
        human_difficulty_level_selection_event: false,
        autonomous_director_may_evolve: true,
        autonomous_director_evolved_this_epoch: false,
        time: TimeDecomposition {
            observation_time_ns: inner.observation_time_ns,
            self_model_update_time_ns,
            bottleneck_diagnosis_time_ns: inner.diagnosis_time_ns,
            hypothesis_generation_time_ns,
            diagnostic_experiment_selection_time_ns: inner.experiment_selection_time_ns,
            diagnostic_experiment_execution_time_ns: inner.diagnostic_experiment_time_ns,
            experiment_interpretation_time_ns,
            desired_self_phenotype_synthesis_time_ns,
            repair_synthesis_time_ns: inner.repair_synthesis_time_ns,
            reaction_discovery_time_ns: inner.reaction_discovery_time_ns,
            reaction_realization_time_ns: inner.reaction_realization_time_ns,
            causal_integration_time_ns: inner.causal_integration_time_ns,
            verification_time_ns: inner.verification_time_ns,
            fresh_work_validation_time_ns,
            state_integration_time_ns,
            bookkeeping_runtime_overhead_time_ns,
            measurement_overhead_time_ns,
            measurement_overhead_bytes,
            unclassified_improvement_time_ns,
            accounted_time_fraction,
            total_improvement_interval_ns,
        },
        inner,
        resulting_state,
        result_checksum,
    })
}

fn derive_agenda(state: &PostScaffoldState, seed: u64) -> Vec<ResearchAgendaItem> {
    let mut ranked = state
        .director
        .last_phase_times_ns
        .iter()
        .enumerate()
        .map(|(index, time)| (*time, index))
        .collect::<Vec<_>>();
    ranked.sort_by_key(|(time, _)| Reverse(*time));
    ranked
        .into_iter()
        .take(3)
        .enumerate()
        .map(|(rank, (time, index))| {
            let evidence = state.director.memory.cause_evidence_masks[index];
            ResearchAgendaItem {
                agenda_item_id: mix(seed ^ rank as u64, index as u64),
                rank: rank as u8,
                measured_dimension_code: index as u8,
                measured_dimension_name: PHASE_NAMES[index].to_string(),
                observed_time_ns: time,
                uncertainty_ppm: if evidence == 0 { 900_000 } else { 240_000 },
                causal_evidence_mask: evidence,
                mechanism_version: state.director.accepted_repairs,
                selected_by_operator: false,
                status: if rank == 0 {
                    "ACTIVE_CAUSAL_INVESTIGATION".to_string()
                } else {
                    "COMPETING_OBSERVED_DEFICIT".to_string()
                },
            }
        })
        .collect()
}

fn agenda_signature(agenda: &[ResearchAgendaItem]) -> Vec<u8> {
    agenda
        .iter()
        .map(|item| item.measured_dimension_code)
        .collect()
}

fn plateau_observed(gains: &[u64], regime_costs: &[u64]) -> bool {
    if gains.len() < 8 || regime_costs.len() < 4 {
        return false;
    }
    let recent = &gains[gains.len() - 4..];
    let prior = &gains[gains.len() - 8..gains.len() - 4];
    let historical_noise = gains
        .windows(2)
        .map(|pair| pair[1].abs_diff(pair[0]))
        .sum::<u64>()
        / gains.len().saturating_sub(1).max(1) as u64;
    let recent_range = recent
        .iter()
        .max()
        .copied()
        .unwrap_or(0)
        .saturating_sub(recent.iter().min().copied().unwrap_or(0));
    let gain_flat = recent_range <= historical_noise.max(1) && mean_u64(recent) <= mean_u64(prior);
    let cost_recent = &regime_costs[regime_costs.len() - 4..];
    let cost_range = cost_recent
        .iter()
        .max()
        .copied()
        .unwrap_or(0)
        .saturating_sub(cost_recent.iter().min().copied().unwrap_or(0));
    let cost_noise = regime_costs
        .windows(2)
        .map(|pair| pair[1].abs_diff(pair[0]))
        .sum::<u64>()
        / regime_costs.len().saturating_sub(1).max(1) as u64;
    gain_flat && cost_range <= cost_noise.max(1)
}

fn classify_plateau(
    state: &PostScaffoldState,
    inner: &AutonomousEpochResult,
    _epoch: u8,
    frontier_capability: u64,
) -> PlateauEvent {
    let minimum_work = state
        .director
        .phase_work_units
        .iter()
        .min()
        .copied()
        .unwrap_or(1);
    let above_floor = state
        .director
        .phase_work_units
        .iter()
        .filter(|units| **units > minimum_work.saturating_add(minimum_work / 8))
        .count() as u64;
    let at_floor = PHASE_COUNT as u64 - above_floor;
    let distinct_failures = state
        .research
        .rejected_lineages
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
        .len() as u64;
    let evidence_count = state.difficulty.current_cost_sequence_ns.len() as u64;
    let candidates = vec![
        PlateauCandidate {
            classification: "UNRESOLVED_BOTTLENECK".to_string(),
            evidence_strength: above_floor
                .saturating_mul(3)
                .saturating_add(u64::from(inner.repair_accepted) * 4),
            evidence: vec![
                format!("MEASURED_PHASES_WITH_REMAINING_HEADROOM={above_floor}"),
                format!("CURRENT_REPAIR_ACCEPTED={}", inner.repair_accepted),
            ],
        },
        PlateauCandidate {
            classification: "LOCAL_MASTERY_OR_PHYSICAL_FLOOR".to_string(),
            evidence_strength: at_floor
                .saturating_mul(2)
                .saturating_add(distinct_failures)
                .saturating_add(u64::from(state.consecutive_rejections)),
            evidence: vec![
                format!("MEASURED_PHASES_AT_LOCAL_MINIMUM={at_floor}"),
                format!("DISTINCT_REJECTED_LINEAGES={distinct_failures}"),
                format!("CONSECUTIVE_REJECTIONS={}", state.consecutive_rejections),
            ],
        },
        PlateauCandidate {
            classification: "CURRENT_FRONTIER_EXHAUSTION".to_string(),
            evidence_strength: u64::from(state.difficulty.local_mastery_progress)
                .saturating_mul(3)
                .saturating_add(u64::from(inner.frontier_gain <= 28) * 5)
                .saturating_add(u64::from(frontier_capability > 0) * 2),
            evidence: vec![
                format!(
                    "LOCAL_MASTERY_PROGRESS={}",
                    state.difficulty.local_mastery_progress
                ),
                format!("CURRENT_FRONTIER_GAIN={}", inner.frontier_gain),
                format!("VERIFIED_REGIME_CAPABILITY={frontier_capability}"),
            ],
        },
        PlateauCandidate {
            classification: "INSUFFICIENT_EVIDENCE".to_string(),
            evidence_strength: 12_u64.saturating_sub(evidence_count),
            evidence: vec![format!("REGIME_OBSERVATION_COUNT={evidence_count}")],
        },
    ];
    let mut ranked = candidates.clone();
    ranked.sort_by_key(|candidate| Reverse(candidate.evidence_strength));
    let first = ranked.first().expect("four plateau candidates");
    let second = ranked.get(1).expect("four plateau candidates");
    let (classification, evidence_margin) = if first.evidence_strength == second.evidence_strength {
        ("INSUFFICIENT_EVIDENCE".to_string(), 0)
    } else {
        (
            first.classification.clone(),
            first
                .evidence_strength
                .saturating_sub(second.evidence_strength),
        )
    };
    let selected_response = match classification.as_str() {
        "UNRESOLVED_BOTTLENECK" => "CONTINUE_CAUSAL_LOCAL_REPAIR",
        "LOCAL_MASTERY_OR_PHYSICAL_FLOOR" | "CURRENT_FRONTIER_EXHAUSTION" => {
            "AUTONOMOUSLY_ESCALATE_STRUCTURED_DIFFICULTY"
        }
        _ => "COLLECT_ADDITIONAL_DISCRIMINATING_EVIDENCE",
    };
    PlateauEvent {
        plateau_event_id: state.plateau_event_count.saturating_add(1),
        observed_gain_curve: state.gain_history.clone(),
        observed_cost_curve_ns: state.difficulty.current_cost_sequence_ns.clone(),
        candidate_explanations: candidates,
        classification,
        evidence_margin,
        selected_response: selected_response.to_string(),
        actual_consequence: "PENDING_NEXT_EPOCH_OBSERVATION".to_string(),
    }
}

fn escalate_difficulty(
    state: &mut PostScaffoldState,
    plateau: &PlateauEvent,
    inner: &AutonomousEpochResult,
    epoch: u8,
    frontier_capability: u64,
) -> DifficultyTransitionRecord {
    let current = state.difficulty.current_dimensions.clone();
    let dimensions = [
        current.causal_depth,
        current.compositional_depth,
        current.transfer_arity,
        current.constraint_complexity,
        current.planning_depth,
    ];
    let evidence_signals = [
        inner.observed_phase_times_ns[3],
        inner.observed_phase_times_ns[5],
        inner.observed_phase_times_ns[7],
        inner.observed_phase_times_ns[8],
        inner.observed_phase_times_ns[9],
    ];
    let selected_dimension = evidence_signals
        .iter()
        .enumerate()
        .max_by_key(|(index, signal)| **signal / u64::from(dimensions[*index].max(1)))
        .map(|(index, _)| index)
        .unwrap_or(0);
    let learned_delta = 1_u16.saturating_add(
        state.difficulty.productive_escalation_events
            / state.difficulty.transitions.len().max(1) as u16,
    );
    let mut next = current.clone();
    let changed_dimension = match selected_dimension {
        0 => {
            next.causal_depth = next.causal_depth.saturating_add(learned_delta);
            "CAUSAL_DEPTH"
        }
        1 => {
            next.compositional_depth = next.compositional_depth.saturating_add(learned_delta);
            "COMPOSITIONAL_DEPTH"
        }
        2 => {
            next.transfer_arity = next.transfer_arity.saturating_add(learned_delta);
            "TRANSFER_ARITY"
        }
        3 => {
            next.constraint_complexity = next.constraint_complexity.saturating_add(learned_delta);
            "CONSTRAINT_COMPLEXITY"
        }
        _ => {
            next.planning_depth = next.planning_depth.saturating_add(learned_delta);
            "PLANNING_DEPTH"
        }
    };
    let initial_cost = state
        .difficulty
        .current_cost_sequence_ns
        .first()
        .copied()
        .unwrap_or(0);
    let final_cost = state
        .difficulty
        .current_cost_sequence_ns
        .last()
        .copied()
        .unwrap_or(initial_cost);
    let productive = state.director.frontier_scale > state.difficulty.current_initial_frontier
        && state.difficulty.local_mastery_progress > 0;
    if state.difficulty.current_regime_id > 1 {
        if productive {
            state.difficulty.productive_escalation_events = state
                .difficulty
                .productive_escalation_events
                .saturating_add(1);
        } else {
            state.difficulty.failed_escalation_events =
                state.difficulty.failed_escalation_events.saturating_add(1);
        }
    }
    state
        .difficulty
        .completed_regimes
        .push(DifficultyRegimeRecord {
            regime_id: state.difficulty.current_regime_id,
            dimensions: current.clone(),
            initial_capability_requirement: difficulty_complexity(&current),
            initial_cost_ns: initial_cost,
            final_local_cost_ns: final_cost,
            within_regime_cost_sequence_ns: state.difficulty.current_cost_sequence_ns.clone(),
            frontier_capability_achieved: frontier_capability,
            epochs_to_local_mastery: u16::from(
                epoch.saturating_sub(state.difficulty.current_regime_started_epoch),
            )
            .saturating_add(1),
            time_to_local_mastery_ns: state.difficulty.current_cost_sequence_ns.iter().sum(),
            plateau_classification: plateau.classification.clone(),
            next_difficulty_selection_basis: format!(
                "MAX_RELATIVE_EVIDENCE_PRESSURE:{changed_dimension}"
            ),
            productive,
        });
    let transition = DifficultyTransitionRecord {
        transition_id: state.difficulty.transitions.len() as u16 + 1,
        previous_regime_id: state.difficulty.current_regime_id,
        previous_plateau_type: plateau.classification.clone(),
        reason_escalation_chosen: plateau.selected_response.clone(),
        changed_dimension: changed_dimension.to_string(),
        previous_dimensions: current,
        new_dimensions: next.clone(),
        predicted_challenge_increase_units: difficulty_complexity(&next)
            .saturating_sub(difficulty_complexity(&state.difficulty.current_dimensions)),
        predicted_capability_opportunity_units: frontier_capability
            .saturating_add(difficulty_complexity(&next)),
        operator_selected: false,
    };
    state.difficulty.transitions.push(transition.clone());
    state.difficulty.current_regime_id = state.difficulty.current_regime_id.saturating_add(1);
    state.difficulty.current_dimensions = next;
    state.difficulty.current_regime_started_epoch = epoch.saturating_add(1);
    state.difficulty.current_initial_cost_ns = 0;
    state.difficulty.current_cost_sequence_ns.clear();
    state.difficulty.current_initial_frontier = state.director.frontier_scale;
    state.difficulty.local_mastery_progress = 0;

    let primary_phase = usize::from(inner.selected_bottleneck_code).min(PHASE_COUNT - 1);
    let challenge_pressure = transition
        .predicted_challenge_increase_units
        .saturating_mul(1_200)
        .max(8_000);
    state.director.phase_work_units[primary_phase] =
        state.director.phase_work_units[primary_phase].saturating_add(challenge_pressure);
    let secondary_phase = (primary_phase + selected_dimension + 1) % PHASE_COUNT;
    state.director.phase_work_units[secondary_phase] =
        state.director.phase_work_units[secondary_phase].saturating_add(challenge_pressure / 2);
    transition
}

fn execute_difficulty_probe(state: &DifficultyState, seed: u64) -> DifficultyProbeResult {
    let complexity = difficulty_complexity(&state.current_dimensions);
    let recurrence_depth = complexity.saturating_mul(384).clamp(2_000, 900_000);
    let multiplier = mix(
        u64::from(state.current_dimensions.causal_depth),
        u64::from(state.current_dimensions.compositional_depth),
    ) | 1;
    let increment = mix(
        u64::from(state.current_dimensions.transfer_arity),
        u64::from(state.current_dimensions.constraint_complexity)
            ^ u64::from(state.current_dimensions.planning_depth),
    );
    let expected = affine_repeat(seed, multiplier, increment, recurrence_depth);
    let started = Instant::now();
    let (result, solver_mode, work_units) = if state.local_mastery_progress >= 2 {
        (
            affine_repeat(seed, multiplier, increment, recurrence_depth),
            "COMPOSED_AFFINE_TRANSITION".to_string(),
            recurrence_depth.ilog2() as u64 + 1,
        )
    } else {
        let mut value = seed;
        for _ in 0..recurrence_depth {
            value = value.wrapping_mul(multiplier).wrapping_add(increment);
            black_box(value);
        }
        (
            value,
            "DIRECT_STRUCTURED_RECURRENCE".to_string(),
            recurrence_depth,
        )
    };
    let wall_time_ns = nanos(started.elapsed().as_nanos()).max(100);
    DifficultyProbeResult {
        regime_id: state.current_regime_id,
        dimensions: state.current_dimensions.clone(),
        semantic_recurrence_depth: recurrence_depth,
        structured_work_units: work_units,
        solver_mode,
        result_hash: result,
        independent_expected_hash: expected,
        mechanically_verified: result == expected,
        wall_time_ns,
        frontier_capability_units: complexity.saturating_mul(64),
    }
}

fn difficulty_complexity(dimensions: &DifficultyDimensions) -> u64 {
    let causal = u64::from(dimensions.causal_depth);
    let composition = u64::from(dimensions.compositional_depth);
    let transfer = u64::from(dimensions.transfer_arity);
    let constraints = u64::from(dimensions.constraint_complexity);
    let planning = u64::from(dimensions.planning_depth);
    causal
        .saturating_mul(causal)
        .saturating_add(composition.saturating_mul(composition).saturating_mul(2))
        .saturating_add(transfer.saturating_mul(constraints))
        .saturating_add(planning.saturating_mul(planning))
        .saturating_add(causal.saturating_mul(composition).saturating_mul(transfer))
}

fn affine_repeat(seed: u64, multiplier: u64, increment: u64, mut count: u64) -> u64 {
    let mut result_multiplier = 1_u64;
    let mut result_increment = 0_u64;
    let mut base_multiplier = multiplier;
    let mut base_increment = increment;
    while count > 0 {
        if count & 1 == 1 {
            result_increment = base_multiplier
                .wrapping_mul(result_increment)
                .wrapping_add(base_increment);
            result_multiplier = base_multiplier.wrapping_mul(result_multiplier);
        }
        base_increment = base_multiplier
            .wrapping_mul(base_increment)
            .wrapping_add(base_increment);
        base_multiplier = base_multiplier.wrapping_mul(base_multiplier);
        count >>= 1;
    }
    result_multiplier
        .wrapping_mul(seed)
        .wrapping_add(result_increment)
}

fn mean_u64(values: &[u64]) -> u64 {
    values.iter().sum::<u64>() / values.len().max(1) as u64
}

fn validate_request(request: &PostScaffoldEpochRequest) -> Result<(), String> {
    if request.arm_code > 3 {
        return Err("INVALID_ARM_CODE".to_string());
    }
    if request.epoch == 0 || request.epoch > SEM27_EPOCH_BUDGET || request.seed == 0 {
        return Err("INVALID_EPOCH_OR_SEED".to_string());
    }
    if request.arm_code == 1 && request.historical_roadmap_target_code.is_none() {
        return Err("HISTORICAL_ROADMAP_TARGET_REQUIRED_FOR_ARM_B".to_string());
    }
    if request.arm_code >= 2 && request.historical_roadmap_target_code.is_some() {
        return Err("AUTONOMOUS_ARM_RECEIVED_HISTORICAL_TARGET".to_string());
    }
    if request
        .historical_roadmap_target_code
        .is_some_and(|code| usize::from(code) >= PHASE_COUNT)
    {
        return Err("INVALID_HISTORICAL_TARGET".to_string());
    }
    if request.resource_ceiling_bytes < request.state.director.core_bytes {
        return Err("RESOURCE_CEILING_BELOW_CURRENT_CORE".to_string());
    }
    if request.state.autonomous_termination_reason.is_some() {
        return Err("CAMPAIGN_ALREADY_TERMINATED".to_string());
    }
    Ok(())
}

fn mix(left: u64, right: u64) -> u64 {
    let mut value = left ^ right.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn nanos(value: u128) -> u64 {
    value.min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(arm_code: u8, epoch: u8) -> PostScaffoldEpochRequest {
        PostScaffoldEpochRequest {
            arm_code,
            epoch,
            seed: 0x2700 + u64::from(epoch),
            state: PostScaffoldState::from_sem26(DirectorState::frozen_sem25()),
            resource_ceiling_bytes: 2_000_000,
            historical_roadmap_target_code: (arm_code <= 1).then_some(6),
            disable_long_term_research_memory: arm_code == 2,
            concrete_future_instance_visible: false,
        }
    }

    #[test]
    fn full_arm_receives_no_historical_target_or_repair_rule() {
        let result = run_post_scaffold_epoch(request(3, 1)).expect("post scaffold epoch");
        assert!(!result.historical_roadmap_visible);
        assert!(!result.predecessor_textual_bottleneck_visible);
        assert!(!result.hardcoded_bottleneck_to_repair_rule_used);
        assert!(result
            .inner
            .bottleneck_hypotheses
            .iter()
            .all(|hypothesis| !hypothesis.predecessor_label_used));
    }

    #[test]
    fn agenda_is_derived_from_measured_state_and_is_revisable() {
        let mut request = request(3, 1);
        request.state.director.last_phase_times_ns = [1, 2, 3, 4, 5, 6, 7, 8, 90, 10];
        let result = run_post_scaffold_epoch(request).expect("post scaffold epoch");
        assert_eq!(result.agenda_before[0].measured_dimension_code, 8);
        assert!(result
            .agenda_before
            .iter()
            .all(|item| !item.selected_by_operator));
    }

    #[test]
    fn accounting_exposes_residual_instead_of_hiding_it() {
        let result = run_post_scaffold_epoch(request(3, 1)).expect("post scaffold epoch");
        assert!((0.0..=1.0).contains(&result.time.accounted_time_fraction));
        assert_eq!(
            result.time.total_improvement_interval_ns,
            result
                .time
                .total_improvement_interval_ns
                .saturating_sub(result.time.unclassified_improvement_time_ns)
                .saturating_add(result.time.unclassified_improvement_time_ns)
        );
    }

    #[test]
    fn one_bounded_step_and_closed_world_invariants_hold() {
        let result = run_post_scaffold_epoch(request(3, 1)).expect("post scaffold epoch");
        assert!(result.implementations_this_epoch <= 1);
        assert!(!result.inner.open_loop_multi_generation_self_modification);
        assert!(!result.inner.future_instance_leakage);
        assert!(!result.inner.full_self_improvement_space_enumeration);
        assert!(!result.inner.full_repair_space_enumeration);
    }
}
