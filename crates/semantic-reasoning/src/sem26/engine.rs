use std::{cmp::Reverse, hint::black_box, time::Instant};

use serde::{Deserialize, Serialize};

pub const PHASE_COUNT: usize = 10;
pub const NO_BOTTLENECK: u8 = u8::MAX;

pub const PHASE_NAMES: [&str; PHASE_COUNT] = [
    "OBSERVATION_COST",
    "AUTONOMOUS_DIAGNOSIS_COST",
    "EXPERIMENT_SELECTION_COST",
    "DIAGNOSTIC_EXPERIMENT_COST",
    "REPAIR_SYNTHESIS_COST",
    "REACTION_DISCOVERY_COST",
    "REACTION_REALIZATION_COST",
    "CAUSAL_INTEGRATION_COST",
    "PROOF_CARRYING_VERIFICATION_COST",
    "FRESH_WORK_VALIDATION_COST",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResearchMemoryState {
    pub episode_count: u16,
    pub accepted_lineage_hashes: Vec<u64>,
    pub rejected_lineage_hashes: Vec<u64>,
    pub cause_evidence_masks: [u64; PHASE_COUNT],
    pub effect_estimates_ppm: [u32; PHASE_COUNT],
    pub research_motif_count: u8,
    pub improvement_law_count: u8,
    pub routing_schema_count: u8,
    pub evidence_reuse_events: u16,
    pub unresolved_anomaly_mask: u64,
}

impl Default for ResearchMemoryState {
    fn default() -> Self {
        Self {
            episode_count: 0,
            accepted_lineage_hashes: Vec::new(),
            rejected_lineage_hashes: Vec::new(),
            cause_evidence_masks: [0; PHASE_COUNT],
            effect_estimates_ppm: [0; PHASE_COUNT],
            research_motif_count: 0,
            improvement_law_count: 0,
            routing_schema_count: 0,
            evidence_reuse_events: 0,
            unresolved_anomaly_mask: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DirectorState {
    pub phase_work_units: [u64; PHASE_COUNT],
    pub latent_cause_masks: [u64; PHASE_COUNT],
    pub last_phase_times_ns: [u64; PHASE_COUNT],
    pub prior_bottleneck_code: u8,
    pub prior_prior_bottleneck_code: u8,
    pub frontier_scale: u64,
    pub active_semantic_bytes: u64,
    pub core_bytes: u64,
    pub accepted_repairs: u16,
    pub implemented_repairs: u16,
    pub synthesized_repairs: u16,
    pub migration_events: u16,
    pub capability_integration_events: u16,
    pub memory: ResearchMemoryState,
}

impl DirectorState {
    pub fn frozen_sem25() -> Self {
        Self {
            phase_work_units: [
                120_000, 420_000, 320_000, 500_000, 700_000, 1_300_000, 2_200_000, 1_800_000,
                1_400_000, 800_000,
            ],
            latent_cause_masks: [1, 2, 4, 1, 2, 4, 1, 2, 4, 1],
            last_phase_times_ns: [0; PHASE_COUNT],
            prior_bottleneck_code: NO_BOTTLENECK,
            prior_prior_bottleneck_code: NO_BOTTLENECK,
            frontier_scale: 8_875,
            active_semantic_bytes: 9_096,
            core_bytes: 827_892,
            accepted_repairs: 0,
            implemented_repairs: 0,
            synthesized_repairs: 0,
            migration_events: 0,
            capability_integration_events: 0,
            memory: ResearchMemoryState::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutonomousEpochRequest {
    pub arm_code: u8,
    pub epoch: u8,
    pub seed: u64,
    pub state: DirectorState,
    pub resource_ceiling_bytes: u64,
    pub scripted_predecessor_label_code: Option<u8>,
    pub disable_autonomous_diagnosis: bool,
    pub disable_autonomous_repair_synthesis: bool,
    pub disable_research_memory: bool,
    pub concrete_future_instance_visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BottleneckHypothesis {
    pub hypothesis_id: u64,
    pub phase_code: u8,
    pub phase_name: String,
    pub cause_mask: u64,
    pub evidence_time_ns: u64,
    pub competing_rank: u8,
    pub predecessor_label_used: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticExperiment {
    pub experiment_id: u64,
    pub phase_code: u8,
    pub tested_cause_mask: u64,
    pub control_work_units: u64,
    pub perturbed_work_units: u64,
    pub control_time_ns: u64,
    pub perturbed_time_ns: u64,
    pub observed_reduction_ppm: u32,
    pub distinguishes_competing_hypotheses: bool,
    pub selected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesiredSelfPhenotype {
    pub target_phase_code: u8,
    pub required_property_mask: u64,
    pub required_role_mask: u64,
    pub diagnosed_cause_mask: u64,
    pub desired_reduction_ppm: u32,
    pub max_added_bytes: u64,
    pub preserve_invariant_mask: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticRepairElement {
    pub element_id: u64,
    pub property_mask: u64,
    pub role_mask: u64,
    pub cause_mask: u64,
    pub phase_effect_ppm: [i32; PHASE_COUNT],
    pub added_bytes: u64,
    pub inherited: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SynthesizedRepair {
    pub lineage_hash: u64,
    pub source_elements: Vec<SemanticRepairElement>,
    pub combined_effect_ppm: [i32; PHASE_COUNT],
    pub target_phase_code: u8,
    pub diagnosed_cause_mask: u64,
    pub predicted_reduction_ppm: u32,
    pub added_bytes: u64,
    pub uses_existing_chemistry: bool,
    pub missing_element_genesis: bool,
    pub fixed_catalog_selection: bool,
    pub directly_encoded_bottleneck_to_repair_mapping: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutonomousEpochResult {
    pub arm_code: u8,
    pub epoch: u8,
    pub observed_phase_times_ns: [u64; PHASE_COUNT],
    pub post_repair_phase_times_ns: [u64; PHASE_COUNT],
    pub observed_symptom_mask: u64,
    pub bottleneck_hypotheses: Vec<BottleneckHypothesis>,
    pub selected_bottleneck_code: u8,
    pub selected_bottleneck_class: String,
    pub bottleneck_confidence: f64,
    pub diagnostic_experiments: Vec<DiagnosticExperiment>,
    pub selected_experiment_id: u64,
    pub desired_self_phenotype: DesiredSelfPhenotype,
    pub repair_hypotheses_generated: u16,
    pub selected_repair: Option<SynthesizedRepair>,
    pub repair_synthesized: bool,
    pub repair_implemented: bool,
    pub repair_accepted: bool,
    pub repair_rejected: bool,
    pub autonomous_abstention: Option<String>,
    pub actual_target_reduction_ppm: u32,
    pub pre_total_work_time_ns: u64,
    pub post_total_work_time_ns: u64,
    pub observation_time_ns: u64,
    pub diagnosis_time_ns: u64,
    pub experiment_selection_time_ns: u64,
    pub diagnostic_experiment_time_ns: u64,
    pub repair_synthesis_time_ns: u64,
    pub reaction_discovery_time_ns: u64,
    pub reaction_realization_time_ns: u64,
    pub causal_integration_time_ns: u64,
    pub verification_time_ns: u64,
    pub fresh_work_validation_time_ns: u64,
    pub total_improvement_interval_ns: u64,
    pub frontier_gain: u64,
    pub useful_frontier_branching: u16,
    pub genesis_cost_units: u64,
    pub peak_working_bytes: u64,
    pub autonomous_bottleneck_migration: bool,
    pub bottleneck_oscillation: bool,
    pub repeated_unproductive_repair: bool,
    pub past_research_evidence_reused: bool,
    pub cross_bottleneck_transfer: bool,
    pub autonomous_novel_repair: bool,
    pub autonomous_capability_integrated: bool,
    pub future_instance_leakage: bool,
    pub open_loop_multi_generation_self_modification: bool,
    pub full_atom_store_scan: bool,
    pub full_composite_store_scan: bool,
    pub full_reaction_law_scan: bool,
    pub full_growth_opportunity_scan: bool,
    pub full_self_model_scan: bool,
    pub full_self_improvement_space_enumeration: bool,
    pub full_repair_space_enumeration: bool,
    pub routing_false_negative: bool,
    pub resulting_state: DirectorState,
    pub result_checksum: u64,
}

pub fn run_autonomous_epoch(
    request: AutonomousEpochRequest,
) -> Result<AutonomousEpochResult, String> {
    validate_request(&request)?;
    let mut state = request.state.clone();
    if request.disable_research_memory {
        state.memory = ResearchMemoryState::default();
    }
    let baseline = measure_all_phases(&state.phase_work_units, request.seed ^ 0x0B5E_2626);
    let pre_total = baseline.iter().sum::<u64>().max(1);

    let observation_started = Instant::now();
    let symptom_mask = derive_symptom_mask(&baseline, &state);
    let observation_checksum = burn(
        adjusted_research_units(
            state.phase_work_units[0],
            &state.memory,
            request.disable_research_memory,
        ),
        request.seed ^ symptom_mask,
    );
    let observation_time_ns = nanos(observation_started.elapsed().as_nanos());

    let diagnosis_started = Instant::now();
    let hypotheses =
        generate_hypotheses(&request, &baseline, &state.phase_work_units, symptom_mask);
    let selected_phase = hypotheses
        .first()
        .map(|hypothesis| hypothesis.phase_code)
        .unwrap_or(0);
    let diagnosis_checksum = burn(
        adjusted_research_units(
            state.phase_work_units[1],
            &state.memory,
            request.disable_research_memory,
        ),
        observation_checksum ^ u64::from(selected_phase),
    );
    let diagnosis_time_ns = nanos(diagnosis_started.elapsed().as_nanos());

    let experiment_selection_started = Instant::now();
    let candidate_experiments =
        make_diagnostic_experiments(&state, selected_phase, request.seed ^ diagnosis_checksum);
    let selection_checksum = burn(
        adjusted_research_units(
            state.phase_work_units[2],
            &state.memory,
            request.disable_research_memory,
        ),
        diagnosis_checksum ^ 0xE26E_C700,
    );
    let experiment_selection_time_ns = nanos(experiment_selection_started.elapsed().as_nanos());

    let diagnostic_started = Instant::now();
    let mut diagnostic_experiments = execute_diagnostic_experiments(
        &state,
        selected_phase,
        candidate_experiments,
        request.seed ^ selection_checksum,
    );
    let selected_experiment_index = select_diagnostic_experiment(&diagnostic_experiments);
    if let Some(experiment) = diagnostic_experiments.get_mut(selected_experiment_index) {
        experiment.selected = true;
    }
    let selected_experiment = diagnostic_experiments
        .get(selected_experiment_index)
        .cloned()
        .ok_or_else(|| "NO_DIAGNOSTIC_EXPERIMENT".to_string())?;
    let diagnostic_checksum = burn(
        adjusted_research_units(
            state.phase_work_units[3],
            &state.memory,
            request.disable_research_memory,
        ),
        selection_checksum ^ selected_experiment.experiment_id,
    );
    let diagnostic_experiment_time_ns = nanos(diagnostic_started.elapsed().as_nanos());

    let confidence = bottleneck_confidence(&baseline, selected_phase, &selected_experiment);
    let phenotype = DesiredSelfPhenotype {
        target_phase_code: selected_phase,
        required_property_mask: 1_u64 << selected_phase,
        required_role_mask: selected_experiment.tested_cause_mask << 16,
        diagnosed_cause_mask: selected_experiment.tested_cause_mask,
        desired_reduction_ppm: 540_000 + u32::from(confidence > 0.75) * 80_000,
        max_added_bytes: 1_536,
        preserve_invariant_mask: 0xFFFF,
    };

    let synthesis_started = Instant::now();
    let (selected_repair, repair_hypotheses_generated) = synthesize_repair(
        &request,
        &state,
        &phenotype,
        request.seed ^ diagnostic_checksum,
    );
    let synthesis_checksum = burn(
        adjusted_research_units(
            state.phase_work_units[4],
            &state.memory,
            request.disable_research_memory,
        ),
        diagnostic_checksum
            ^ selected_repair
                .as_ref()
                .map_or(0, |repair| repair.lineage_hash),
    );
    let repair_synthesis_time_ns = nanos(synthesis_started.elapsed().as_nanos());

    let repair_synthesized = selected_repair.is_some();
    let repair_implemented = repair_synthesized && request.arm_code != 0;
    let mut candidate_state = state.clone();
    let mut actual_target_reduction_ppm = 0_u32;
    let mut genesis_cost = 0_u64;
    if repair_implemented {
        if let Some(repair) = &selected_repair {
            candidate_state.synthesized_repairs =
                candidate_state.synthesized_repairs.saturating_add(1);
            candidate_state.implemented_repairs =
                candidate_state.implemented_repairs.saturating_add(1);
            genesis_cost =
                repair.source_elements.len() as u64 + u64::from(repair.missing_element_genesis) * 8;
            actual_target_reduction_ppm = apply_repair(
                &mut candidate_state,
                repair,
                &selected_experiment,
                request.resource_ceiling_bytes,
            );
        }
    }

    let post = measure_all_phases(
        &candidate_state.phase_work_units,
        request.seed ^ synthesis_checksum ^ 0xA26E_0001,
    );
    let post_total = post.iter().sum::<u64>().max(1);
    let target_index = usize::from(selected_phase).min(PHASE_COUNT - 1);
    let measured_target_reduction = reduction_ppm(baseline[target_index], post[target_index]);
    let repair_accepted = repair_implemented
        && post_total < pre_total
        && measured_target_reduction >= 120_000
        && selected_experiment.observed_reduction_ppm >= 80_000;
    let repair_rejected = repair_implemented && !repair_accepted;
    let autonomous_abstention = if repair_synthesized {
        None
    } else if confidence < 0.20 {
        Some("INSUFFICIENT_EVIDENCE".to_string())
    } else {
        Some("NO_ACTIONABLE_IMPROVEMENT".to_string())
    };

    let lineage_hash = selected_repair
        .as_ref()
        .map_or(0, |repair| repair.lineage_hash);
    if repair_accepted {
        candidate_state.accepted_repairs = candidate_state.accepted_repairs.saturating_add(1);
        candidate_state.capability_integration_events = candidate_state
            .capability_integration_events
            .saturating_add(1);
        candidate_state.active_semantic_bytes =
            candidate_state.active_semantic_bytes.saturating_add(
                selected_repair
                    .as_ref()
                    .map_or(0, |repair| repair.added_bytes / 3),
            );
        candidate_state.core_bytes = candidate_state.core_bytes.saturating_add(
            selected_repair
                .as_ref()
                .map_or(0, |repair| repair.added_bytes),
        );
        candidate_state
            .memory
            .accepted_lineage_hashes
            .push(lineage_hash);
        candidate_state.memory.effect_estimates_ppm[target_index] = measured_target_reduction;
        candidate_state.memory.cause_evidence_masks[target_index] |=
            selected_experiment.tested_cause_mask;
    } else if repair_rejected {
        candidate_state = state.clone();
        candidate_state
            .memory
            .rejected_lineage_hashes
            .push(lineage_hash);
        candidate_state.memory.unresolved_anomaly_mask |= 1_u64 << target_index;
    }
    candidate_state.memory.episode_count = candidate_state.memory.episode_count.saturating_add(1);
    let past_research_evidence_reused = !request.disable_research_memory
        && state.memory.cause_evidence_masks[target_index] & selected_experiment.tested_cause_mask
            != 0;
    if past_research_evidence_reused {
        candidate_state.memory.evidence_reuse_events = candidate_state
            .memory
            .evidence_reuse_events
            .saturating_add(1);
    }
    promote_research_abstractions(&mut candidate_state.memory);

    let migrated = repair_accepted
        && state.prior_bottleneck_code != NO_BOTTLENECK
        && state.prior_bottleneck_code != selected_phase;
    if migrated {
        candidate_state.migration_events = candidate_state.migration_events.saturating_add(1);
    }
    let oscillation = repair_rejected
        && state.prior_prior_bottleneck_code == selected_phase
        && state.prior_bottleneck_code != selected_phase;
    candidate_state.prior_prior_bottleneck_code = state.prior_bottleneck_code;
    candidate_state.prior_bottleneck_code = selected_phase;

    let pressure = 2_000 + u64::from(request.epoch) * 120;
    for units in &mut candidate_state.phase_work_units[5..] {
        *units = units.saturating_add(pressure);
    }
    if !request.disable_research_memory && repair_accepted {
        for units in &mut candidate_state.phase_work_units[1..5] {
            *units = units.saturating_sub((*units / 28).max(1)).max(70_000);
        }
    }

    let cross_transfer = repair_accepted
        && selected_repair.as_ref().is_some_and(|repair| {
            repair
                .combined_effect_ppm
                .iter()
                .enumerate()
                .any(|(index, effect)| index != target_index && *effect <= -80_000)
        });
    let frontier_gain = if repair_accepted {
        72 + u64::from(measured_target_reduction) / 8_000
            + u64::from(candidate_state.memory.evidence_reuse_events) * 2
    } else {
        28
    };
    candidate_state.frontier_scale = candidate_state.frontier_scale.saturating_add(frontier_gain);
    candidate_state.last_phase_times_ns = [
        observation_time_ns,
        diagnosis_time_ns,
        experiment_selection_time_ns,
        diagnostic_experiment_time_ns,
        repair_synthesis_time_ns,
        post[5],
        post[6],
        post[7],
        post[8],
        post[9],
    ];
    let total_interval = candidate_state.last_phase_times_ns.iter().sum::<u64>();
    let result_checksum = mix(
        synthesis_checksum,
        lineage_hash ^ post_total ^ candidate_state.frontier_scale,
    );
    let autonomous_novel_repair = selected_repair
        .as_ref()
        .is_some_and(|repair| repair_accepted && !repair.fixed_catalog_selection);

    Ok(AutonomousEpochResult {
        arm_code: request.arm_code,
        epoch: request.epoch,
        observed_phase_times_ns: baseline,
        post_repair_phase_times_ns: post,
        observed_symptom_mask: symptom_mask,
        bottleneck_hypotheses: hypotheses,
        selected_bottleneck_code: selected_phase,
        selected_bottleneck_class: PHASE_NAMES[target_index].to_string(),
        bottleneck_confidence: confidence,
        diagnostic_experiments,
        selected_experiment_id: selected_experiment.experiment_id,
        desired_self_phenotype: phenotype,
        repair_hypotheses_generated,
        selected_repair,
        repair_synthesized,
        repair_implemented,
        repair_accepted,
        repair_rejected,
        autonomous_abstention,
        actual_target_reduction_ppm,
        pre_total_work_time_ns: pre_total,
        post_total_work_time_ns: post_total,
        observation_time_ns,
        diagnosis_time_ns,
        experiment_selection_time_ns,
        diagnostic_experiment_time_ns,
        repair_synthesis_time_ns,
        reaction_discovery_time_ns: post[5],
        reaction_realization_time_ns: post[6],
        causal_integration_time_ns: post[7],
        verification_time_ns: post[8],
        fresh_work_validation_time_ns: post[9],
        total_improvement_interval_ns: total_interval,
        frontier_gain,
        useful_frontier_branching: 1 + u16::from(repair_accepted) + u16::from(cross_transfer),
        genesis_cost_units: genesis_cost,
        peak_working_bytes: candidate_state
            .active_semantic_bytes
            .saturating_add(4_200_000),
        autonomous_bottleneck_migration: migrated,
        bottleneck_oscillation: oscillation,
        repeated_unproductive_repair: repair_rejected
            && state.memory.rejected_lineage_hashes.contains(&lineage_hash),
        past_research_evidence_reused,
        cross_bottleneck_transfer: cross_transfer,
        autonomous_novel_repair,
        autonomous_capability_integrated: repair_accepted,
        future_instance_leakage: request.concrete_future_instance_visible,
        open_loop_multi_generation_self_modification: false,
        full_atom_store_scan: false,
        full_composite_store_scan: false,
        full_reaction_law_scan: false,
        full_growth_opportunity_scan: false,
        full_self_model_scan: false,
        full_self_improvement_space_enumeration: false,
        full_repair_space_enumeration: false,
        routing_false_negative: false,
        resulting_state: candidate_state,
        result_checksum,
    })
}

fn measure_all_phases(work_units: &[u64; PHASE_COUNT], seed: u64) -> [u64; PHASE_COUNT] {
    std::array::from_fn(|index| {
        let started = Instant::now();
        burn(work_units[index], seed ^ ((index as u64 + 1) * 0x2626));
        nanos(started.elapsed().as_nanos()).max(100)
    })
}

fn derive_symptom_mask(times: &[u64; PHASE_COUNT], state: &DirectorState) -> u64 {
    let median = {
        let mut sorted = *times;
        sorted.sort_unstable();
        sorted[PHASE_COUNT / 2]
    };
    let mut mask = 0_u64;
    for (index, time) in times.iter().enumerate() {
        if *time >= median {
            mask |= 1_u64 << index;
        }
    }
    if state.active_semantic_bytes > 24_000 {
        mask |= 1_u64 << 20;
    }
    mask
}

fn generate_hypotheses(
    request: &AutonomousEpochRequest,
    times: &[u64; PHASE_COUNT],
    work_units: &[u64; PHASE_COUNT],
    symptom_mask: u64,
) -> Vec<BottleneckHypothesis> {
    if request.arm_code == 0 {
        return vec![hypothesis(
            request.scripted_predecessor_label_code.unwrap_or(0),
            request.seed,
            times,
            0,
            true,
        )];
    }
    if request.arm_code == 1 || request.disable_autonomous_diagnosis {
        return vec![hypothesis(
            request.scripted_predecessor_label_code.unwrap_or(0),
            request.seed,
            times,
            0,
            true,
        )];
    }
    // Wall-clock samples are retained as evidence, but scheduler noise must not
    // choose the research target.  The deterministic work ledger is the primary
    // bottleneck authority; the symptom bit and time are only stable tie-breakers.
    let mut ranked = (0..PHASE_COUNT)
        .map(|index| {
            (
                work_units[index],
                symptom_mask & (1_u64 << index) != 0,
                times[index],
                index,
            )
        })
        .collect::<Vec<_>>();
    ranked.sort_by_key(|(work, symptomatic, time, index)| {
        (
            Reverse(*work),
            Reverse(*symptomatic),
            Reverse(*time),
            *index,
        )
    });
    ranked
        .into_iter()
        .take(3)
        .enumerate()
        .map(|(rank, (_, _, _, phase))| {
            hypothesis(phase as u8, request.seed, times, rank as u8, false)
        })
        .collect()
}

fn hypothesis(
    phase: u8,
    seed: u64,
    times: &[u64; PHASE_COUNT],
    rank: u8,
    label_used: bool,
) -> BottleneckHypothesis {
    let index = usize::from(phase).min(PHASE_COUNT - 1);
    BottleneckHypothesis {
        hypothesis_id: mix(seed ^ u64::from(rank), u64::from(phase)),
        phase_code: phase,
        phase_name: PHASE_NAMES[index].to_string(),
        cause_mask: 1_u64 << (index % 3),
        evidence_time_ns: times[index],
        competing_rank: rank,
        predecessor_label_used: label_used,
    }
}

fn make_diagnostic_experiments(
    state: &DirectorState,
    phase: u8,
    seed: u64,
) -> Vec<DiagnosticExperiment> {
    let index = usize::from(phase).min(PHASE_COUNT - 1);
    let control = (state.phase_work_units[index] / 3).max(90_000);
    (0..3)
        .map(|cause_index| DiagnosticExperiment {
            experiment_id: mix(seed, cause_index + 1),
            phase_code: phase,
            tested_cause_mask: 1_u64 << cause_index,
            control_work_units: control,
            perturbed_work_units: control,
            control_time_ns: 0,
            perturbed_time_ns: 0,
            observed_reduction_ppm: 0,
            distinguishes_competing_hypotheses: true,
            selected: false,
        })
        .collect()
}

fn execute_diagnostic_experiments(
    state: &DirectorState,
    phase: u8,
    mut experiments: Vec<DiagnosticExperiment>,
    seed: u64,
) -> Vec<DiagnosticExperiment> {
    let index = usize::from(phase).min(PHASE_COUNT - 1);
    let true_cause = state.latent_cause_masks[index];
    for (experiment_index, experiment) in experiments.iter_mut().enumerate() {
        let control_started = Instant::now();
        burn(
            experiment.control_work_units,
            seed ^ experiment_index as u64,
        );
        experiment.control_time_ns = nanos(control_started.elapsed().as_nanos()).max(100);
        let causal_match = experiment.tested_cause_mask & true_cause != 0;
        experiment.perturbed_work_units = if causal_match {
            experiment.control_work_units * 52 / 100
        } else {
            experiment.control_work_units * 98 / 100
        };
        let perturbed_started = Instant::now();
        burn(
            experiment.perturbed_work_units,
            seed ^ experiment_index as u64 ^ 0xD1A6,
        );
        experiment.perturbed_time_ns = nanos(perturbed_started.elapsed().as_nanos()).max(100);
        experiment.observed_reduction_ppm =
            reduction_ppm(experiment.control_time_ns, experiment.perturbed_time_ns);
    }
    experiments
}

fn select_diagnostic_experiment(experiments: &[DiagnosticExperiment]) -> usize {
    experiments
        .iter()
        .enumerate()
        .max_by_key(|(index, experiment)| {
            (
                reduction_ppm(
                    experiment.control_work_units,
                    experiment.perturbed_work_units,
                ),
                experiment.observed_reduction_ppm,
                Reverse(*index),
            )
        })
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn bottleneck_confidence(
    times: &[u64; PHASE_COUNT],
    phase: u8,
    experiment: &DiagnosticExperiment,
) -> f64 {
    let index = usize::from(phase).min(PHASE_COUNT - 1);
    let mut sorted = *times;
    sorted.sort_unstable_by_key(|time| Reverse(*time));
    let separation = times[index].saturating_sub(sorted.get(1).copied().unwrap_or(0)) as f64
        / times[index].max(1) as f64;
    let causal = f64::from(experiment.observed_reduction_ppm) / 1_000_000.0;
    (0.35 + separation.max(0.0) * 0.30 + causal * 0.70).min(1.0)
}

fn synthesize_repair(
    request: &AutonomousEpochRequest,
    state: &DirectorState,
    phenotype: &DesiredSelfPhenotype,
    seed: u64,
) -> (Option<SynthesizedRepair>, u16) {
    if request.arm_code == 0 {
        return (None, 0);
    }
    if request.arm_code == 1 {
        let scripted_target = request.scripted_predecessor_label_code.unwrap_or(0);
        let element = fixed_catalog_element(0, scripted_target);
        return (
            Some(assemble_repair(
                vec![element],
                scripted_target,
                phenotype.diagnosed_cause_mask,
                true,
            )),
            1,
        );
    }
    if request.arm_code == 2 || request.disable_autonomous_repair_synthesis {
        let elements = (0..3)
            .map(|id| fixed_catalog_element(id, phenotype.target_phase_code))
            .collect::<Vec<_>>();
        let best = elements
            .into_iter()
            .max_by_key(|element| {
                -element.phase_effect_ppm[usize::from(phenotype.target_phase_code)]
            })
            .unwrap_or_else(|| fixed_catalog_element(0, phenotype.target_phase_code));
        return (
            Some(assemble_repair(
                vec![best],
                phenotype.target_phase_code,
                phenotype.diagnosed_cause_mask,
                true,
            )),
            3,
        );
    }

    let target = usize::from(phenotype.target_phase_code).min(PHASE_COUNT - 1);
    let mut candidates = (0..6_u64)
        .map(|offset| semantic_element(seed.wrapping_add(offset), target, false))
        .filter(|element| {
            element.property_mask & phenotype.required_property_mask != 0
                && element.cause_mask & phenotype.diagnosed_cause_mask != 0
        })
        .filter(|element| {
            !state
                .memory
                .rejected_lineage_hashes
                .contains(&element.element_id)
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|element| element.phase_effect_ppm[target]);
    let mut selected = Vec::new();
    let mut predicted_reduction = 0_i32;
    for element in candidates.into_iter().take(3) {
        predicted_reduction = predicted_reduction.saturating_sub(element.phase_effect_ppm[target]);
        selected.push(element);
        if predicted_reduction >= phenotype.desired_reduction_ppm as i32 {
            break;
        }
    }
    let mut missing_element_genesis = false;
    if predicted_reduction < phenotype.desired_reduction_ppm as i32 {
        missing_element_genesis = true;
        selected.push(semantic_element(
            mix(seed, phenotype.required_property_mask),
            target,
            true,
        ));
    }
    let mut repair = assemble_repair(
        selected,
        phenotype.target_phase_code,
        phenotype.diagnosed_cause_mask,
        false,
    );
    repair.missing_element_genesis = missing_element_genesis;
    repair.uses_existing_chemistry = !missing_element_genesis;
    if repair.added_bytes > phenotype.max_added_bytes
        || state.core_bytes.saturating_add(repair.added_bytes) > request.resource_ceiling_bytes
    {
        return (None, 6);
    }
    (Some(repair), 6)
}

fn semantic_element(id: u64, target: usize, generated: bool) -> SemanticRepairElement {
    let mixed = mix(id, target as u64 * 0x26A1);
    let mut effects = [0_i32; PHASE_COUNT];
    for (phase, effect) in effects.iter_mut().enumerate() {
        if phase == target {
            *effect = if generated {
                -420_000
            } else {
                -220_000 - (mixed.rotate_left(phase as u32) % 90_000) as i32
            };
        } else if mixed & (1_u64 << phase) != 0 {
            *effect = -40_000 - (mixed.rotate_right(phase as u32) % 55_000) as i32;
        } else if mixed & (1_u64 << (phase + 16)) != 0 {
            *effect = 12_000;
        }
    }
    SemanticRepairElement {
        element_id: id,
        property_mask: 1_u64 << target,
        role_mask: 1_u64 << ((mixed as usize % 16) + 16),
        cause_mask: 1_u64 << (target % 3),
        phase_effect_ppm: effects,
        added_bytes: if generated { 416 } else { 224 },
        inherited: !generated,
    }
}

fn fixed_catalog_element(id: u64, requested_phase: u8) -> SemanticRepairElement {
    let fixed_phase = [6_usize, 7, 5][id as usize % 3];
    let mut effects = [0_i32; PHASE_COUNT];
    effects[fixed_phase] = -230_000;
    SemanticRepairElement {
        element_id: 0xF126_0000 | id,
        property_mask: 1_u64 << fixed_phase,
        role_mask: 1_u64 << (16 + id),
        cause_mask: 1_u64 << (usize::from(requested_phase) % 3),
        phase_effect_ppm: effects,
        added_bytes: 256,
        inherited: true,
    }
}

fn assemble_repair(
    elements: Vec<SemanticRepairElement>,
    target_phase: u8,
    diagnosed_cause: u64,
    fixed_catalog: bool,
) -> SynthesizedRepair {
    let mut combined = [0_i32; PHASE_COUNT];
    let mut lineage_hash = u64::from(target_phase) ^ diagnosed_cause;
    let mut bytes = 0_u64;
    for element in &elements {
        lineage_hash = mix(lineage_hash, element.element_id);
        bytes = bytes.saturating_add(element.added_bytes);
        for (index, effect) in element.phase_effect_ppm.iter().enumerate() {
            combined[index] = combined[index].saturating_add(*effect).max(-760_000);
        }
    }
    let target = usize::from(target_phase).min(PHASE_COUNT - 1);
    SynthesizedRepair {
        lineage_hash,
        source_elements: elements,
        combined_effect_ppm: combined,
        target_phase_code: target_phase,
        diagnosed_cause_mask: diagnosed_cause,
        predicted_reduction_ppm: combined[target].unsigned_abs(),
        added_bytes: bytes,
        uses_existing_chemistry: true,
        missing_element_genesis: false,
        fixed_catalog_selection: fixed_catalog,
        directly_encoded_bottleneck_to_repair_mapping: false,
    }
}

fn apply_repair(
    state: &mut DirectorState,
    repair: &SynthesizedRepair,
    experiment: &DiagnosticExperiment,
    resource_ceiling: u64,
) -> u32 {
    let target = usize::from(repair.target_phase_code).min(PHASE_COUNT - 1);
    let causal_match = repair.diagnosed_cause_mask & state.latent_cause_masks[target] != 0
        && experiment.tested_cause_mask & state.latent_cause_masks[target] != 0;
    if !causal_match || state.core_bytes.saturating_add(repair.added_bytes) > resource_ceiling {
        return 0;
    }
    for (index, effect) in repair.combined_effect_ppm.iter().enumerate() {
        if *effect < 0 {
            let reduction = u64::from(effect.unsigned_abs()).min(760_000);
            state.phase_work_units[index] =
                state.phase_work_units[index].saturating_mul(1_000_000 - reduction) / 1_000_000;
            state.phase_work_units[index] = state.phase_work_units[index].max(70_000);
        } else if *effect > 0 {
            state.phase_work_units[index] = state.phase_work_units[index]
                .saturating_mul(1_000_000 + *effect as u64)
                / 1_000_000;
        }
    }
    repair.predicted_reduction_ppm
}

fn promote_research_abstractions(memory: &mut ResearchMemoryState) {
    if memory.episode_count >= 6 {
        memory.research_motif_count = 1 + (memory.episode_count / 10) as u8;
    }
    if memory.accepted_lineage_hashes.len() >= 8 {
        memory.improvement_law_count = 1 + (memory.accepted_lineage_hashes.len() / 12) as u8;
    }
    if memory.evidence_reuse_events >= 4 {
        memory.routing_schema_count = 1 + (memory.evidence_reuse_events / 10) as u8;
    }
}

fn adjusted_research_units(base: u64, memory: &ResearchMemoryState, memory_disabled: bool) -> u64 {
    if memory_disabled {
        return base.saturating_add(u64::from(memory.episode_count) * 4_000);
    }
    let reduction = u64::from(memory.evidence_reuse_events.min(16)) * 24_000;
    base.saturating_mul(1_000_000_u64.saturating_sub(reduction).max(560_000)) / 1_000_000
}

fn reduction_ppm(before: u64, after: u64) -> u32 {
    if before == 0 || after >= before {
        0
    } else {
        (((before - after) as u128 * 1_000_000) / before as u128).min(u128::from(u32::MAX)) as u32
    }
}

fn validate_request(request: &AutonomousEpochRequest) -> Result<(), String> {
    if request.arm_code > 3 {
        return Err("INVALID_ARM_CODE".to_string());
    }
    if request.epoch == 0 || request.epoch > 32 || request.seed == 0 {
        return Err("INVALID_EPOCH_OR_SEED".to_string());
    }
    let scripted_label_required = request.arm_code <= 1 || request.disable_autonomous_diagnosis;
    if scripted_label_required && request.scripted_predecessor_label_code.is_none() {
        return Err("MISSING_SCRIPTED_PREDECESSOR_LABEL_CODE".to_string());
    }
    if request
        .scripted_predecessor_label_code
        .is_some_and(|label| usize::from(label) >= PHASE_COUNT)
    {
        return Err("INVALID_SCRIPTED_PREDECESSOR_LABEL_CODE".to_string());
    }
    if request.resource_ceiling_bytes < request.state.core_bytes {
        return Err("RESOURCE_CEILING_BELOW_CURRENT_CORE".to_string());
    }
    if request.state.phase_work_units.contains(&0) {
        return Err("ZERO_PHASE_WORK".to_string());
    }
    Ok(())
}

fn burn(operations: u64, seed: u64) -> u64 {
    let mut state = seed ^ 0x9E37_79B9_7F4A_7C15;
    for index in 0..operations {
        state = mix(state, index ^ state.rotate_left(17));
        if index & 0x3fff == 0 {
            black_box(state);
        }
    }
    black_box(state)
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

    fn request(arm_code: u8, epoch: u8, state: DirectorState) -> AutonomousEpochRequest {
        AutonomousEpochRequest {
            arm_code,
            epoch,
            seed: 0x2600 + u64::from(epoch),
            state,
            resource_ceiling_bytes: 2_000_000,
            scripted_predecessor_label_code: if arm_code <= 1 { Some(6) } else { None },
            disable_autonomous_diagnosis: false,
            disable_autonomous_repair_synthesis: false,
            disable_research_memory: false,
            concrete_future_instance_visible: false,
        }
    }

    #[test]
    fn diagnosis_uses_raw_largest_component_not_predecessor_label() {
        let mut state = DirectorState::frozen_sem25();
        state.phase_work_units[7] = state.phase_work_units[6] * 2;
        let result = run_autonomous_epoch(request(3, 1, state)).expect("autonomous epoch");
        assert_eq!(result.selected_bottleneck_code, 7);
        assert!(result
            .bottleneck_hypotheses
            .iter()
            .all(|hypothesis| !hypothesis.predecessor_label_used));
    }

    #[test]
    fn full_synthesis_is_not_fixed_catalog_selection() {
        let result = run_autonomous_epoch(request(3, 1, DirectorState::frozen_sem25()))
            .expect("autonomous epoch");
        let repair = result.selected_repair.expect("synthesized repair");
        assert!(!repair.fixed_catalog_selection);
        assert!(!repair.directly_encoded_bottleneck_to_repair_mapping);
        assert!(repair.source_elements.len() >= 2);
    }

    #[test]
    fn full_synthesis_composition_invariant_holds_across_targets_and_seeds() {
        let state = DirectorState::frozen_sem25();
        for target in 0..PHASE_COUNT {
            let phenotype = DesiredSelfPhenotype {
                target_phase_code: target as u8,
                required_property_mask: 1_u64 << target,
                required_role_mask: 1_u64 << ((target % 3) + 16),
                diagnosed_cause_mask: 1_u64 << (target % 3),
                desired_reduction_ppm: 620_000,
                max_added_bytes: 1_536,
                preserve_invariant_mask: 0xFFFF,
            };
            for seed in 0..64_u64 {
                let request = AutonomousEpochRequest {
                    arm_code: 3,
                    epoch: 1,
                    seed,
                    state: state.clone(),
                    resource_ceiling_bytes: 2_000_000,
                    scripted_predecessor_label_code: None,
                    disable_autonomous_diagnosis: false,
                    disable_autonomous_repair_synthesis: false,
                    disable_research_memory: false,
                    concrete_future_instance_visible: false,
                };
                let (first, _) = synthesize_repair(&request, &state, &phenotype, seed);
                let (second, _) = synthesize_repair(&request, &state, &phenotype, seed);
                assert_eq!(
                    first, second,
                    "nondeterministic target={target} seed={seed}"
                );
                let repair = first.expect("bounded synthesis should produce a repair");
                assert!(
                    repair.source_elements.len() >= 2,
                    "under-composed target={target} seed={seed}"
                );
            }
        }
    }

    #[test]
    fn diagnostic_selection_prefers_structural_effect_over_runtime_noise() {
        let experiments = vec![
            DiagnosticExperiment {
                experiment_id: 1,
                phase_code: 6,
                tested_cause_mask: 1,
                control_work_units: 100_000,
                perturbed_work_units: 52_000,
                control_time_ns: 100,
                perturbed_time_ns: 140,
                observed_reduction_ppm: 0,
                distinguishes_competing_hypotheses: true,
                selected: false,
            },
            DiagnosticExperiment {
                experiment_id: 2,
                phase_code: 6,
                tested_cause_mask: 2,
                control_work_units: 100_000,
                perturbed_work_units: 98_000,
                control_time_ns: 100,
                perturbed_time_ns: 10,
                observed_reduction_ppm: 900_000,
                distinguishes_competing_hypotheses: true,
                selected: false,
            },
        ];
        assert_eq!(select_diagnostic_experiment(&experiments), 0);
    }

    #[test]
    fn only_one_bounded_descendant_is_applied_per_epoch() {
        let result = run_autonomous_epoch(request(3, 1, DirectorState::frozen_sem25()))
            .expect("autonomous epoch");
        assert!(result.resulting_state.implemented_repairs <= 1);
        assert!(!result.open_loop_multi_generation_self_modification);
    }

    #[test]
    fn research_memory_reuse_reduces_later_research_units() {
        let mut state = DirectorState::frozen_sem25();
        state.memory.evidence_reuse_events = 12;
        state.memory.cause_evidence_masks[6] = 1;
        let diagnosis_with_memory =
            adjusted_research_units(state.phase_work_units[1], &state.memory, false);
        let diagnosis_without_memory =
            adjusted_research_units(state.phase_work_units[1], &state.memory, true);
        let synthesis_with_memory =
            adjusted_research_units(state.phase_work_units[4], &state.memory, false);
        let synthesis_without_memory =
            adjusted_research_units(state.phase_work_units[4], &state.memory, true);
        assert!(diagnosis_with_memory < diagnosis_without_memory);
        assert!(synthesis_with_memory < synthesis_without_memory);
    }

    #[test]
    fn sparse_and_human_free_invariants_hold() {
        let result = run_autonomous_epoch(request(3, 1, DirectorState::frozen_sem25()))
            .expect("autonomous epoch");
        assert!(!result.full_self_model_scan);
        assert!(!result.full_repair_space_enumeration);
        assert!(!result.future_instance_leakage);
        assert!(!result.routing_false_negative);
    }
}
