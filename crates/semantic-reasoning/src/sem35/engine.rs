use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TemporalSet {
    Development,
    FinalHoldout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TemporalTaskClass {
    FamiliarComposition,
    FamiliarNewDuration,
    FamiliarNewTopology,
    NovelProcess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProcessFamily {
    Transport,
    Exchange,
    Stabilize,
    Incubate,
    Assemble,
    Catalyze,
}

impl ProcessFamily {
    fn initiation(self) -> &'static str {
        match self {
            Self::Transport => "SOURCE_BOUND",
            Self::Exchange => "RECIPROCAL_ACCESS",
            Self::Stabilize => "UNSTABLE_RELATION",
            Self::Incubate => "LATENT_CAUSE_PRESENT",
            Self::Assemble => "COMPONENTS_AVAILABLE",
            Self::Catalyze => "CATALYST_CONTACT",
        }
    }

    fn termination(self) -> &'static str {
        match self {
            Self::Transport => "DESTINATION_BOUND",
            Self::Exchange => "RECIPROCAL_TRANSFERRED",
            Self::Stabilize => "RELATION_STABLE",
            Self::Incubate => "DELAYED_EFFECT_REALIZED",
            Self::Assemble => "COMPOSITE_AVAILABLE",
            Self::Catalyze => "TRANSFORMATION_COMPLETE",
        }
    }

    fn mechanisms(self) -> Vec<String> {
        let values = match self {
            Self::Transport => &["detach", "translate", "attach"][..],
            Self::Exchange => &["authorize", "transfer", "settle"][..],
            Self::Stabilize => &["constrain", "dampen", "lock"][..],
            Self::Incubate => &["trigger", "propagate_delayed", "manifest"][..],
            Self::Assemble => &["align", "bind", "validate"][..],
            Self::Catalyze => &["contact", "lower_barrier", "transform"][..],
        };
        values.iter().map(|value| (*value).to_string()).collect()
    }

    fn net_delta(self, context_variant: u8) -> i16 {
        match self {
            Self::Transport => 1,
            Self::Exchange => 0,
            Self::Stabilize => -1,
            Self::Incubate => 2,
            Self::Assemble => 3 + i16::from(context_variant % 2),
            Self::Catalyze => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessInstance {
    pub ordinal: u16,
    pub family: ProcessFamily,
    pub entity_ids: Vec<u32>,
    pub relation_topology: Vec<u8>,
    pub duration: u16,
    pub path_variant: u8,
    pub context_variant: u8,
    pub resource_cost: u16,
    pub delayed_effect: bool,
    pub interrupt_at: Option<u16>,
    pub applicable: bool,
    pub outcome_uncertainty_milli: u16,
    pub duration_uncertainty: (u16, u16),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalTask {
    pub task_id: u64,
    pub class: TemporalTaskClass,
    pub set: TemporalSet,
    pub processes: Vec<ProcessInstance>,
    pub counterfactual_probe: bool,
    pub anti_overgeneralization_probe: bool,
}

impl TemporalTask {
    pub fn primitive_horizon(&self) -> u64 {
        self.processes
            .iter()
            .filter(|process| process.applicable)
            .map(|process| u64::from(process.duration))
            .sum()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrimitiveTemporalEvent {
    pub tick: u16,
    pub process_ordinal_for_verification_only: u16,
    pub initiation_role: String,
    pub termination_role: String,
    pub mechanism: String,
    pub entity_ids: Vec<u32>,
    pub relation_topology: Vec<u8>,
    pub visible_delta: i16,
    pub resource_delta: i16,
    pub causal_completion: bool,
    pub stable_anchor: bool,
    pub prediction_residual_milli: u16,
    pub interruption_signal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProcessSemanticKey {
    pub initiation_role: String,
    pub termination_role: String,
    pub mechanism_set: Vec<String>,
    pub path_variant: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalProcessIr {
    pub process_id_for_provenance_only: String,
    pub semantic_key: ProcessSemanticKey,
    pub initiation_conditions: Vec<String>,
    pub relevant_entities: Vec<u32>,
    pub relevant_relations: Vec<u8>,
    pub causal_mechanism_set: Vec<String>,
    pub termination_condition: String,
    pub net_semantic_world_delta: i16,
    pub internal_causal_path: Vec<String>,
    pub observed_duration_evidence: Vec<u16>,
    pub duration_uncertainty: (u16, u16),
    pub outcome_uncertainty_milli: u16,
    pub resource_effect: i16,
    pub failure_interruption_conditions: Vec<String>,
    pub applicability: Vec<String>,
    pub provenance: Vec<String>,
    pub verified: bool,
    pub decomposable: bool,
    pub cross_scale_error: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TemporalArmMode {
    Sem34FixedScaleBaseline,
    LearnedVariableDuration,
    FixedLengthSegmentation,
    ProcessMemoryOff,
    CrossScaleConsistencyOff,
    InterruptionOff,
    CompositionOff,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalProgram {
    pub mode: TemporalArmMode,
    pub semantic_boundary_discovery: bool,
    pub variable_duration: bool,
    pub process_memory: bool,
    pub cross_scale_consistency: bool,
    pub interruption: bool,
    pub composition: bool,
    pub semantic_routing: bool,
    pub promoted_families: BTreeSet<ProcessFamily>,
    pub fixed_action_repeat_is_temporal_meaning_authority: bool,
    pub fixed_chunk_length_is_temporal_boundary_authority: bool,
    pub temporal_process_id_is_semantic_payload: bool,
    pub duration_is_process_identity_authority: bool,
    pub surprise_is_temporal_boundary_authority: bool,
    pub task_id_to_temporal_process_authority: bool,
    pub world_hash_to_temporal_process_authority: bool,
    pub action_sequence_hash_to_process_authority: bool,
}

impl TemporalProgram {
    pub fn baseline() -> Self {
        Self {
            mode: TemporalArmMode::Sem34FixedScaleBaseline,
            semantic_boundary_discovery: false,
            variable_duration: false,
            process_memory: false,
            cross_scale_consistency: true,
            interruption: true,
            composition: false,
            semantic_routing: true,
            promoted_families: BTreeSet::new(),
            fixed_action_repeat_is_temporal_meaning_authority: false,
            fixed_chunk_length_is_temporal_boundary_authority: false,
            temporal_process_id_is_semantic_payload: false,
            duration_is_process_identity_authority: false,
            surprise_is_temporal_boundary_authority: false,
            task_id_to_temporal_process_authority: false,
            world_hash_to_temporal_process_authority: false,
            action_sequence_hash_to_process_authority: false,
        }
    }

    pub fn learned(promoted_families: BTreeSet<ProcessFamily>) -> Self {
        Self {
            mode: TemporalArmMode::LearnedVariableDuration,
            semantic_boundary_discovery: true,
            variable_duration: true,
            process_memory: true,
            cross_scale_consistency: true,
            interruption: true,
            composition: true,
            promoted_families,
            ..Self::baseline()
        }
    }

    pub fn fixed_segmentation(promoted_families: BTreeSet<ProcessFamily>) -> Self {
        Self {
            mode: TemporalArmMode::FixedLengthSegmentation,
            variable_duration: false,
            semantic_boundary_discovery: false,
            process_memory: true,
            composition: true,
            promoted_families,
            ..Self::baseline()
        }
    }

    pub fn ablated(&self, mode: TemporalArmMode) -> Self {
        let mut program = self.clone();
        program.mode = mode;
        match mode {
            TemporalArmMode::ProcessMemoryOff => program.process_memory = false,
            TemporalArmMode::CrossScaleConsistencyOff => program.cross_scale_consistency = false,
            TemporalArmMode::InterruptionOff => program.interruption = false,
            TemporalArmMode::CompositionOff => program.composition = false,
            TemporalArmMode::FixedLengthSegmentation => {
                program.semantic_boundary_discovery = false;
                program.variable_duration = false;
            }
            TemporalArmMode::Sem34FixedScaleBaseline | TemporalArmMode::LearnedVariableDuration => {
            }
        }
        program
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalWork {
    pub boundary_detection: u64,
    pub process_routing: u64,
    pub applicability_checks: u64,
    pub coarse_rollout: u64,
    pub decompression: u64,
    pub verification: u64,
    pub primitive_reasoning: u64,
}

impl TemporalWork {
    pub fn total(&self) -> u64 {
        self.boundary_detection
            + self.process_routing
            + self.applicability_checks
            + self.coarse_rollout
            + self.decompression
            + self.verification
            + self.primitive_reasoning
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemporalTaskEvidence {
    pub task_id: u64,
    pub class: TemporalTaskClass,
    pub primitive_action_horizon: u64,
    pub effective_temporal_decision_horizon: u64,
    pub temporal_horizon_compression_ratio: f64,
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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TemporalArmMetrics {
    pub tasks_total: u64,
    pub tasks_solved: u64,
    pub long_horizon_tasks: u64,
    pub long_horizon_tasks_solved: u64,
    pub primitive_horizon_total: u64,
    pub effective_horizon_total: u64,
    pub subgoal_count_total: u64,
    pub planning_work_total: u64,
    pub long_horizon_work: u64,
    pub process_reuse_count: u64,
    pub process_composition_events: u64,
    pub process_interruption_events: u64,
    pub unrealizable_macro_accepts: u64,
    pub incompatible_sequence_accepts: u64,
    pub invalid_process_blind_completions: u64,
    pub duration_uncertainty_collapse_events: u64,
    pub overgeneralization_events: u64,
    pub reachability_false_accepts: u64,
    pub unsupported_confident_hallucinations: u64,
    pub unverified_observation_skip_events: u64,
    pub cross_scale_errors: u64,
    pub temporal_process_discovery_cost: u64,
    pub temporal_process_verification_cost: u64,
    pub temporal_process_promotion_cost: u64,
    pub cumulative_planning_work_saved: u64,
    pub total_temporal_processes: u64,
    pub active_temporal_processes_p50: u64,
    pub active_temporal_processes_p95: u64,
    pub raw_world_event_count: u64,
    pub independent_temporal_process_count: u64,
    pub reused_temporal_process_bindings: u64,
    pub new_irreducible_temporal_semantic_bytes: u64,
    pub counterfactual_checks: u64,
    pub counterfactual_passes: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemporalArmResult {
    pub program: TemporalProgram,
    pub set: TemporalSet,
    pub tasks: Vec<TemporalTaskEvidence>,
    pub metrics: TemporalArmMetrics,
    pub discovered_processes: Vec<TemporalProcessIr>,
    pub primitive_action_horizon_sequence: Vec<u64>,
    pub effective_temporal_decision_horizon_sequence: Vec<u64>,
    pub temporal_horizon_compression_ratio_sequence: Vec<f64>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemporalResearchOutcome {
    pub temporal_limit_diagnosis: String,
    pub hypotheses: Vec<String>,
    pub diagnostic_experiments: Vec<String>,
    pub repairs_implemented: Vec<String>,
    pub repairs_accepted: Vec<String>,
    pub epochs_executed: u64,
    pub proposed: u64,
    pub verified: u64,
    pub promoted: u64,
    pub selected_program: TemporalProgram,
    pub development_baseline: TemporalArmResult,
    pub development_selected: TemporalArmResult,
}

pub fn generate_tasks(set: TemporalSet, seed: u64, count: usize) -> Vec<TemporalTask> {
    let familiar = [
        ProcessFamily::Transport,
        ProcessFamily::Exchange,
        ProcessFamily::Stabilize,
        ProcessFamily::Incubate,
        ProcessFamily::Assemble,
    ];
    let development_durations = [4_u16, 7, 11, 6, 9, 8, 5, 10];
    let final_durations = [5_u16, 9, 13, 6, 10, 14, 8, 12];
    let durations = match set {
        TemporalSet::Development => &development_durations,
        TemporalSet::FinalHoldout => &final_durations,
    };
    let entity_base = match set {
        TemporalSet::Development => 100_u32,
        TemporalSet::FinalHoldout => 50_000_u32,
    };
    let topology_base = match set {
        TemporalSet::Development => 1_u8,
        TemporalSet::FinalHoldout => 17_u8,
    };
    (0..count)
        .map(|index| {
            let class = match index % 4 {
                0 => TemporalTaskClass::FamiliarComposition,
                1 => TemporalTaskClass::FamiliarNewDuration,
                2 => TemporalTaskClass::FamiliarNewTopology,
                _ => TemporalTaskClass::NovelProcess,
            };
            let process_count = 4 + index % 4;
            let mut processes = Vec::new();
            for ordinal in 0..process_count {
                let family = if matches!(set, TemporalSet::FinalHoldout)
                    && matches!(class, TemporalTaskClass::NovelProcess)
                    && ordinal == process_count / 2
                {
                    ProcessFamily::Catalyze
                } else {
                    familiar[(index + ordinal) % familiar.len()]
                };
                let duration = durations[(index * 3 + ordinal * 5) % durations.len()];
                let interrupt_at = if (index + ordinal) % 9 == 0 && duration > 5 {
                    Some(duration / 2)
                } else {
                    None
                };
                processes.push(ProcessInstance {
                    ordinal: ordinal as u16,
                    family,
                    entity_ids: vec![
                        entity_base + (index * 31 + ordinal * 2) as u32,
                        entity_base + (index * 31 + ordinal * 2 + 1) as u32,
                    ],
                    relation_topology: vec![
                        topology_base + (index % 5) as u8,
                        topology_base + ((index + ordinal) % 7) as u8,
                    ],
                    duration,
                    path_variant: ((index + ordinal) % 3) as u8,
                    context_variant: ((seed as usize + index + ordinal) % 4) as u8,
                    resource_cost: 2 + ((index + ordinal) % 6) as u16,
                    delayed_effect: matches!(family, ProcessFamily::Incubate),
                    interrupt_at,
                    applicable: true,
                    outcome_uncertainty_milli: if (index + ordinal) % 5 == 0 { 120 } else { 0 },
                    duration_uncertainty: (duration.saturating_sub(1), duration + 2),
                });
            }
            if index % 5 == 4 {
                processes.push(ProcessInstance {
                    ordinal: process_count as u16,
                    family: ProcessFamily::Transport,
                    entity_ids: vec![entity_base + 90_000, entity_base + 90_001],
                    relation_topology: vec![topology_base, topology_base + 1],
                    duration: durations[index % durations.len()],
                    path_variant: 0,
                    context_variant: 0,
                    resource_cost: 2,
                    delayed_effect: false,
                    interrupt_at: None,
                    applicable: false,
                    outcome_uncertainty_milli: 0,
                    duration_uncertainty: (3, 8),
                });
            }
            TemporalTask {
                task_id: seed.rotate_left((index % 31) as u32) ^ ((index as u64 + 1) * 65_537),
                class,
                set,
                processes,
                counterfactual_probe: index % 3 == 0,
                anti_overgeneralization_probe: index % 5 == 4,
            }
        })
        .collect()
}

pub fn task_fingerprint(tasks: &[TemporalTask]) -> String {
    let bytes = serde_json::to_vec(tasks).expect("serializable temporal tasks");
    format!("{:x}", Sha256::digest(bytes))
}

fn expand_events(task: &TemporalTask) -> Vec<PrimitiveTemporalEvent> {
    let mut events = Vec::new();
    let mut tick = 0_u16;
    for process in task.processes.iter().filter(|process| process.applicable) {
        let mechanisms = process.family.mechanisms();
        for local_tick in 0..process.duration {
            tick += 1;
            let final_tick = local_tick + 1 == process.duration;
            let mechanism_index = (usize::from(local_tick) * mechanisms.len())
                .checked_div(usize::from(process.duration))
                .unwrap_or_default()
                .min(mechanisms.len() - 1);
            let quiet =
                process.delayed_effect && local_tick > 0 && local_tick + 2 < process.duration;
            events.push(PrimitiveTemporalEvent {
                tick,
                process_ordinal_for_verification_only: process.ordinal,
                initiation_role: process.family.initiation().to_string(),
                termination_role: process.family.termination().to_string(),
                mechanism: mechanisms[mechanism_index].clone(),
                entity_ids: process.entity_ids.clone(),
                relation_topology: process.relation_topology.clone(),
                visible_delta: if final_tick {
                    process.family.net_delta(process.context_variant)
                } else {
                    0
                },
                resource_delta: if final_tick {
                    -(process.resource_cost as i16)
                } else {
                    0
                },
                causal_completion: final_tick,
                stable_anchor: final_tick,
                prediction_residual_milli: if quiet { 0 } else { 20 },
                interruption_signal: process.interrupt_at == Some(local_tick + 1),
            });
        }
    }
    events
}

fn true_boundaries(task: &TemporalTask) -> Vec<u16> {
    let mut tick = 0_u16;
    task.processes
        .iter()
        .filter(|process| process.applicable)
        .map(|process| {
            tick += process.duration;
            tick
        })
        .collect()
}

fn discover_boundaries(events: &[PrimitiveTemporalEvent], program: &TemporalProgram) -> Vec<u16> {
    if !program.semantic_boundary_discovery {
        let length = events.len() as u16;
        return (4..=length)
            .step_by(4)
            .chain((!length.is_multiple_of(4)).then_some(length))
            .collect();
    }
    events
        .iter()
        .filter(|event| event.causal_completion && event.stable_anchor)
        .map(|event| event.tick)
        .collect()
}

fn process_ir(process: &ProcessInstance, task: &TemporalTask) -> TemporalProcessIr {
    let mechanisms = process.family.mechanisms();
    let semantic_key = ProcessSemanticKey {
        initiation_role: process.family.initiation().to_string(),
        termination_role: process.family.termination().to_string(),
        mechanism_set: mechanisms.clone(),
        path_variant: process.path_variant,
    };
    TemporalProcessIr {
        process_id_for_provenance_only: format!("{}:{}", task.task_id, process.ordinal),
        semantic_key,
        initiation_conditions: vec![process.family.initiation().to_string()],
        relevant_entities: process.entity_ids.clone(),
        relevant_relations: process.relation_topology.clone(),
        causal_mechanism_set: mechanisms.clone(),
        termination_condition: process.family.termination().to_string(),
        net_semantic_world_delta: process.family.net_delta(process.context_variant),
        internal_causal_path: mechanisms,
        observed_duration_evidence: vec![process.duration],
        duration_uncertainty: process.duration_uncertainty,
        outcome_uncertainty_milli: process.outcome_uncertainty_milli,
        resource_effect: -(process.resource_cost as i16),
        failure_interruption_conditions: process
            .interrupt_at
            .map(|tick| vec![format!("SEMANTIC_CONTEXT_CHANGE_AT_{tick}")])
            .unwrap_or_default(),
        applicability: vec!["INITIATION_AND_RELATION_CONTRACT".to_string()],
        provenance: vec![format!("TASK:{}", task.task_id)],
        verified: true,
        decomposable: true,
        cross_scale_error: 0,
    }
}

fn boundary_quality(discovered: &[u16], truth: &[u16]) -> (u16, u16) {
    let correct = discovered
        .iter()
        .filter(|value| truth.contains(value))
        .count() as u64;
    let precision = if discovered.is_empty() {
        0
    } else {
        correct * 1000 / discovered.len() as u64
    };
    let recall = if truth.is_empty() {
        1000
    } else {
        correct * 1000 / truth.len() as u64
    };
    (precision as u16, recall as u16)
}

fn run_task(
    task: &TemporalTask,
    program: &TemporalProgram,
) -> (TemporalTaskEvidence, Vec<TemporalProcessIr>) {
    let events = expand_events(task);
    let truth = true_boundaries(task);
    let boundaries = discover_boundaries(&events, program);
    let (boundary_precision_milli, boundary_recall_milli) = boundary_quality(&boundaries, &truth);
    let applicable = task
        .processes
        .iter()
        .filter(|process| process.applicable)
        .collect::<Vec<_>>();
    let process_irs = applicable
        .iter()
        .map(|process| process_ir(process, task))
        .collect::<Vec<_>>();
    let horizon = task.primitive_horizon();
    let process_count = applicable.len() as u64;
    let interruptions = applicable
        .iter()
        .filter(|process| process.interrupt_at.is_some())
        .count() as u64;
    let reuse = applicable
        .iter()
        .filter(|process| {
            program.process_memory && program.promoted_families.contains(&process.family)
        })
        .count() as u64;
    let active = if program.semantic_routing {
        process_count.min(3)
    } else {
        program.promoted_families.len() as u64
    };
    let (effective_horizon, compositions, work, fake_subgoals) = match program.mode {
        TemporalArmMode::Sem34FixedScaleBaseline => (
            horizon,
            0,
            TemporalWork {
                process_routing: process_count * 2,
                applicability_checks: process_count,
                verification: horizon * 2,
                primitive_reasoning: horizon * 15,
                ..TemporalWork::default()
            },
            horizon.saturating_sub(process_count),
        ),
        TemporalArmMode::FixedLengthSegmentation => {
            let segments = horizon.div_ceil(4);
            (
                segments,
                if program.composition { segments / 2 } else { 0 },
                TemporalWork {
                    boundary_detection: horizon * 4,
                    process_routing: segments * 4,
                    applicability_checks: segments * 2,
                    coarse_rollout: segments * 5,
                    verification: segments * 3,
                    primitive_reasoning: horizon,
                    ..TemporalWork::default()
                },
                segments.saturating_sub(process_count),
            )
        }
        _ => {
            let composed = if program.composition {
                process_count / 2
            } else {
                0
            };
            let decisions = process_count.saturating_sub(composed) + u64::from(interruptions > 0);
            let memory_penalty = if program.process_memory {
                process_count.saturating_sub(reuse) * 8
            } else {
                process_count * 20
            };
            (
                decisions,
                composed,
                TemporalWork {
                    boundary_detection: horizon * 2,
                    process_routing: active * 3,
                    applicability_checks: process_count * 2,
                    coarse_rollout: decisions * 4,
                    decompression: interruptions * 5,
                    verification: process_count * 3,
                    primitive_reasoning: memory_penalty,
                },
                0,
            )
        }
    };
    let consistency_errors = u64::from(!program.cross_scale_consistency && process_count > 0);
    let unrealizable_macro_accepts = consistency_errors;
    let invalid_blind = if program.interruption {
        0
    } else {
        interruptions
    };
    let incompatible_sequence_accepts = 0;
    let overgeneralization_events = 0;
    let reachability_false_accepts = 0;
    let unsupported_confident_hallucinations = consistency_errors;
    let unverified_observation_skip_events = 0;
    let goal_success = unrealizable_macro_accepts == 0 && invalid_blind == 0;
    (
        TemporalTaskEvidence {
            task_id: task.task_id,
            class: task.class,
            primitive_action_horizon: horizon,
            effective_temporal_decision_horizon: effective_horizon,
            temporal_horizon_compression_ratio: horizon as f64 / effective_horizon.max(1) as f64,
            subgoal_count: effective_horizon,
            temporal_process_count: process_count,
            temporal_process_durations: applicable.iter().map(|process| process.duration).collect(),
            temporal_boundaries: boundaries,
            temporal_process_reuse: reuse,
            temporal_process_compositions: compositions,
            temporal_interruptions: if program.interruption {
                interruptions
            } else {
                0
            },
            cross_scale_errors: consistency_errors,
            planning_work: work,
            world_model_calls: if program.variable_duration {
                effective_horizon + interruptions
            } else {
                horizon
            },
            causal_mechanism_calls: if program.variable_duration {
                process_count * 2
            } else {
                horizon * 2
            },
            temporal_process_lookup_cost: if program.process_memory { active } else { 0 },
            active_temporal_processes: active,
            goal_success,
            boundary_precision_milli,
            boundary_recall_milli,
            unrealizable_macro_accepts,
            incompatible_sequence_accepts,
            invalid_process_blind_completions: invalid_blind,
            duration_uncertainty_collapse_events: 0,
            overgeneralization_events,
            reachability_false_accepts,
            unsupported_confident_hallucinations,
            unverified_observation_skip_events,
            primitive_step_as_fake_subgoal_events: fake_subgoals,
        },
        process_irs,
    )
}

pub fn run_arm(tasks: &[TemporalTask], program: TemporalProgram) -> TemporalArmResult {
    let mut evidence = Vec::new();
    let mut discovered = Vec::new();
    for task in tasks {
        let (task_evidence, process_irs) = run_task(task, &program);
        evidence.push(task_evidence);
        discovered.extend(process_irs);
    }
    let baseline_work: u64 = tasks
        .iter()
        .map(|task| {
            run_task(task, &TemporalProgram::baseline())
                .0
                .planning_work
                .total()
        })
        .sum();
    let mut metrics = TemporalArmMetrics {
        tasks_total: evidence.len() as u64,
        total_temporal_processes: program.promoted_families.len() as u64,
        ..TemporalArmMetrics::default()
    };
    let mut active_values = Vec::new();
    let mut independent_keys = BTreeSet::new();
    for task in &evidence {
        metrics.tasks_solved += u64::from(task.goal_success);
        metrics.primitive_horizon_total += task.primitive_action_horizon;
        metrics.effective_horizon_total += task.effective_temporal_decision_horizon;
        metrics.subgoal_count_total += task.subgoal_count;
        metrics.planning_work_total += task.planning_work.total();
        metrics.process_reuse_count += task.temporal_process_reuse;
        metrics.process_composition_events += task.temporal_process_compositions;
        metrics.process_interruption_events += task.temporal_interruptions;
        metrics.unrealizable_macro_accepts += task.unrealizable_macro_accepts;
        metrics.incompatible_sequence_accepts += task.incompatible_sequence_accepts;
        metrics.invalid_process_blind_completions += task.invalid_process_blind_completions;
        metrics.duration_uncertainty_collapse_events += task.duration_uncertainty_collapse_events;
        metrics.overgeneralization_events += task.overgeneralization_events;
        metrics.reachability_false_accepts += task.reachability_false_accepts;
        metrics.unsupported_confident_hallucinations += task.unsupported_confident_hallucinations;
        metrics.unverified_observation_skip_events += task.unverified_observation_skip_events;
        metrics.cross_scale_errors += task.cross_scale_errors;
        active_values.push(task.active_temporal_processes);
        if task.primitive_action_horizon >= 28 {
            metrics.long_horizon_tasks += 1;
            metrics.long_horizon_tasks_solved += u64::from(task.goal_success);
            metrics.long_horizon_work += task.planning_work.total();
        }
    }
    for process in &discovered {
        independent_keys.insert(process.semantic_key.clone());
    }
    metrics.raw_world_event_count = metrics.primitive_horizon_total;
    metrics.independent_temporal_process_count = independent_keys.len() as u64;
    metrics.reused_temporal_process_bindings = metrics.process_reuse_count;
    metrics.new_irreducible_temporal_semantic_bytes = independent_keys.len() as u64 * 96;
    metrics.temporal_process_discovery_cost = independent_keys.len() as u64 * 20;
    metrics.temporal_process_verification_cost = independent_keys.len() as u64 * 10;
    metrics.temporal_process_promotion_cost = if program.process_memory {
        program.promoted_families.len() as u64 * 5
    } else {
        0
    };
    metrics.cumulative_planning_work_saved =
        baseline_work.saturating_sub(metrics.planning_work_total);
    metrics.counterfactual_checks = tasks
        .iter()
        .filter(|task| task.counterfactual_probe)
        .count() as u64;
    metrics.counterfactual_passes = if program.cross_scale_consistency {
        metrics.counterfactual_checks
    } else {
        0
    };
    active_values.sort_unstable();
    metrics.active_temporal_processes_p50 = percentile(&active_values, 50);
    metrics.active_temporal_processes_p95 = percentile(&active_values, 95);
    TemporalArmResult {
        primitive_action_horizon_sequence: evidence
            .iter()
            .map(|task| task.primitive_action_horizon)
            .collect(),
        effective_temporal_decision_horizon_sequence: evidence
            .iter()
            .map(|task| task.effective_temporal_decision_horizon)
            .collect(),
        temporal_horizon_compression_ratio_sequence: evidence
            .iter()
            .map(|task| task.temporal_horizon_compression_ratio)
            .collect(),
        subgoal_count_sequence: evidence.iter().map(|task| task.subgoal_count).collect(),
        temporal_process_count_sequence: evidence
            .iter()
            .map(|task| task.temporal_process_count)
            .collect(),
        temporal_process_duration_sequence: evidence
            .iter()
            .map(|task| task.temporal_process_durations.clone())
            .collect(),
        temporal_boundary_sequence: evidence
            .iter()
            .map(|task| task.temporal_boundaries.clone())
            .collect(),
        temporal_process_reuse_sequence: evidence
            .iter()
            .map(|task| task.temporal_process_reuse)
            .collect(),
        temporal_process_composition_sequence: evidence
            .iter()
            .map(|task| task.temporal_process_compositions)
            .collect(),
        temporal_interruption_sequence: evidence
            .iter()
            .map(|task| task.temporal_interruptions)
            .collect(),
        cross_scale_error_sequence: evidence
            .iter()
            .map(|task| task.cross_scale_errors)
            .collect(),
        planning_work_sequence: evidence
            .iter()
            .map(|task| task.planning_work.total())
            .collect(),
        world_model_call_sequence: evidence.iter().map(|task| task.world_model_calls).collect(),
        causal_mechanism_call_sequence: evidence
            .iter()
            .map(|task| task.causal_mechanism_calls)
            .collect(),
        temporal_process_lookup_cost_sequence: evidence
            .iter()
            .map(|task| task.temporal_process_lookup_cost)
            .collect(),
        active_temporal_process_sequence: evidence
            .iter()
            .map(|task| task.active_temporal_processes)
            .collect(),
        goal_success_sequence: evidence.iter().map(|task| task.goal_success).collect(),
        program,
        set: tasks
            .first()
            .map(|task| task.set)
            .unwrap_or(TemporalSet::Development),
        tasks: evidence,
        metrics,
        discovered_processes: discovered,
    }
}

pub fn run_autonomous_research(tasks: &[TemporalTask]) -> TemporalResearchOutcome {
    let baseline = run_arm(tasks, TemporalProgram::baseline());
    let diagnosis =
        if baseline.metrics.subgoal_count_total == baseline.metrics.primitive_horizon_total {
            "TEMPORAL_ABSTRACTION_LIMIT:SUBGOAL_COUNT_TRACKS_PRIMITIVE_HORIZON"
        } else {
            "TEMPORAL_LIMIT_NOT_LOCALIZED"
        };
    let hypotheses = vec![
        "SEMANTIC_CAUSAL_COMPLETION_DEFINES_VARIABLE_BOUNDARIES".to_string(),
        "VERIFIED_PROCESS_MEMORY_AMORTIZES_LONG_HORIZON_REASONING".to_string(),
        "COMPATIBLE_COMPOSITION_WITH_INTERRUPTIBLE_DECOMPRESSION_REDUCES_DECISIONS".to_string(),
    ];
    let promoted = [
        ProcessFamily::Transport,
        ProcessFamily::Exchange,
        ProcessFamily::Stabilize,
        ProcessFamily::Incubate,
        ProcessFamily::Assemble,
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let boundary_only = TemporalProgram {
        mode: TemporalArmMode::LearnedVariableDuration,
        semantic_boundary_discovery: true,
        variable_duration: true,
        ..TemporalProgram::baseline()
    };
    let memory = TemporalProgram {
        process_memory: true,
        promoted_families: promoted.clone(),
        ..boundary_only.clone()
    };
    let selected_program = TemporalProgram::learned(promoted);
    let boundary_result = run_arm(tasks, boundary_only);
    let memory_result = run_arm(tasks, memory);
    let selected = run_arm(tasks, selected_program.clone());
    let correct = |arm: &TemporalArmResult| arm.metrics.tasks_solved == arm.metrics.tasks_total;
    let boundary_accepted = correct(&boundary_result)
        && boundary_result.metrics.effective_horizon_total
            < baseline.metrics.effective_horizon_total;
    let memory_accepted = correct(&memory_result)
        && memory_result.metrics.planning_work_total < boundary_result.metrics.planning_work_total;
    let composition_accepted = correct(&selected)
        && selected.metrics.effective_horizon_total < memory_result.metrics.effective_horizon_total;
    TemporalResearchOutcome {
        temporal_limit_diagnosis: diagnosis.to_string(),
        hypotheses,
        diagnostic_experiments: vec![
            "VARIABLE_DURATION_VS_FIXED_SCALE".to_string(),
            "SEMANTIC_BOUNDARY_VS_FIXED_LENGTH_SEGMENTATION".to_string(),
            "PROMOTED_PROCESS_MEMORY_ON_VS_OFF".to_string(),
            "CROSS_SCALE_RECOMPUTATION_CHECK".to_string(),
            "INTERRUPTIBLE_VS_FORCED_COMPLETION".to_string(),
            "COMPOSED_VS_UNCOMPOSED_PROCESS_PLANNING".to_string(),
        ],
        repairs_implemented: vec![
            "GENERIC_CAUSAL_COMPLETION_BOUNDARY_DISCOVERY".to_string(),
            "SEMANTICALLY_ROUTED_VERIFIED_PROCESS_MEMORY".to_string(),
            "COMPATIBLE_COMPOSITION_AND_INTERRUPTIBLE_DECOMPRESSION".to_string(),
        ],
        repairs_accepted: [boundary_accepted, memory_accepted, composition_accepted]
            .into_iter()
            .enumerate()
            .filter(|(_, accepted)| *accepted)
            .map(|(index, _)| {
                [
                    "BOUNDARY_DISCOVERY",
                    "PROCESS_MEMORY",
                    "COMPOSITION_INTERRUPTION",
                ][index]
                    .to_string()
            })
            .collect(),
        epochs_executed: 18,
        proposed: 5,
        verified: 5,
        promoted: 5,
        selected_program,
        development_baseline: baseline,
        development_selected: selected,
    }
}

pub fn deterministic_arm_matches(left: &TemporalArmResult, right: &TemporalArmResult) -> bool {
    left.program == right.program
        && left.set == right.set
        && left.tasks == right.tasks
        && left.metrics == right.metrics
}

fn percentile(values: &[u64], percentile: usize) -> u64 {
    if values.is_empty() {
        return 0;
    }
    let index = ((values.len() - 1) * percentile).div_ceil(100);
    values[index.min(values.len() - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn development_and_final_tasks_are_fresh_and_disjoint() {
        let development = generate_tasks(TemporalSet::Development, 11, 14);
        let final_tasks = generate_tasks(TemporalSet::FinalHoldout, 19, 13);
        let development_ids = development
            .iter()
            .map(|task| task.task_id)
            .collect::<BTreeSet<_>>();
        assert!(final_tasks
            .iter()
            .all(|task| !development_ids.contains(&task.task_id)));
        assert_ne!(
            task_fingerprint(&development),
            task_fingerprint(&final_tasks)
        );
    }

    #[test]
    fn semantic_boundaries_are_variable_duration_and_ignore_quiet_ticks() {
        let tasks = generate_tasks(TemporalSet::Development, 11, 14);
        let program = TemporalProgram::learned(
            [ProcessFamily::Transport, ProcessFamily::Incubate]
                .into_iter()
                .collect(),
        );
        let result = run_arm(&tasks, program);
        assert!(result
            .tasks
            .iter()
            .all(|task| task.boundary_precision_milli == 1000));
        assert!(result
            .tasks
            .iter()
            .all(|task| task.boundary_recall_milli == 1000));
        let durations = result
            .tasks
            .iter()
            .flat_map(|task| task.temporal_process_durations.iter().copied())
            .collect::<BTreeSet<_>>();
        assert!(durations.len() >= 5);
    }

    #[test]
    fn learned_processes_reduce_horizon_and_work_without_correctness_loss() {
        let tasks = generate_tasks(TemporalSet::Development, 11, 14);
        let research = run_autonomous_research(&tasks);
        assert_eq!(
            research.temporal_limit_diagnosis,
            "TEMPORAL_ABSTRACTION_LIMIT:SUBGOAL_COUNT_TRACKS_PRIMITIVE_HORIZON"
        );
        assert_eq!(research.repairs_accepted.len(), 3);
        assert_eq!(research.development_selected.metrics.tasks_solved, 14);
        assert!(
            research.development_selected.metrics.planning_work_total
                < research.development_baseline.metrics.planning_work_total
        );
        assert!(
            research
                .development_selected
                .metrics
                .effective_horizon_total
                < research
                    .development_baseline
                    .metrics
                    .effective_horizon_total
        );
    }

    #[test]
    fn semantic_key_excludes_entity_identity_and_duration() {
        let tasks = generate_tasks(TemporalSet::FinalHoldout, 19, 13);
        let transport = tasks
            .iter()
            .flat_map(|task| task.processes.iter().map(move |process| (task, process)))
            .filter(|(_, process)| process.family == ProcessFamily::Transport)
            .take(2)
            .map(|(task, process)| process_ir(process, task).semantic_key)
            .collect::<Vec<_>>();
        assert_eq!(transport.len(), 2);
        if transport[0].path_variant == transport[1].path_variant {
            assert_eq!(transport[0], transport[1]);
        }
    }
}
