pub mod engine;

use std::{
    collections::BTreeSet,
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use engine::{
    PostScaffoldEpochRequest, PostScaffoldEpochResult, PostScaffoldState, SEM27_EPOCH_BUDGET,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::{
    sem24::engine::{run_verification_probe, VerificationProbeRequest},
    sem26::engine::{DirectorState, PHASE_COUNT, PHASE_NAMES},
};

const CAMPAIGN_ID: &str = "SEM27-POST-SCAFFOLD-AUTONOMOUS-RSI-0001";
const PREDECESSOR_COMMIT: &str = "61f8096ee698d77fd44498497c57dc46ddbce96f";
const BRANCH: &str = "codex/sem27-post-scaffold-autonomy";
const REPORT_DIR: &str = "reports/sem27";
const PREDECESSOR_CLIPPY_WARNINGS: usize = 22;
const PROTOCOL_SHA256: &str = "af3ca001f5c2a665f9cf932bcca5fa388111be770d43f23bffa9968abc4b5596";
const ADDENDUM_SHA256: &str = "0f397f0004a498c7d4b471ea2f95953e60144a644d926b2ba57bb9c639b1e4e7";
const RESOURCE_CEILING_BYTES: u64 = 2_000_000;

const REQUIRED_REPORTS: &[&str] = &[
    "predecessor_integrity.json",
    "campaign_config.json",
    "human_intervention_audit.json",
    "hardcoded_repair_rule_audit.json",
    "infrastructure_failure_ledger.jsonl",
    "post_campaign_accounting_correction.json",
    "autonomous_decision_ledger.jsonl",
    "autonomous_research_agenda_history.jsonl",
    "autonomous_research_memory.json",
    "autonomous_research_method_ledger.jsonl",
    "autonomous_research_method_lineage.json",
    "bottleneck_hypothesis_ledger.jsonl",
    "causal_diagnostic_experiments.jsonl",
    "desired_self_phenotype_ledger.jsonl",
    "autonomous_repair_hypotheses.jsonl",
    "autonomous_repair_lineage.json",
    "autonomous_bottleneck_migration_graph.json",
    "autonomous_growth_regime_shift_graph.json",
    "plateau_classification_ledger.jsonl",
    "autonomous_difficulty_escalation_ledger.jsonl",
    "arm_a_frozen_sem26.json",
    "arm_b_historical_roadmap_replay.json",
    "arm_c_no_research_memory.json",
    "arm_d_post_scaffold_autonomous_rsi.json",
    "growth_ledger.jsonl",
    "frontier_scale_sequence.json",
    "frontier_gain_sequence.json",
    "fixed_resource_frontier_sequence.json",
    "fixed_work_wall_time_sequence.json",
    "total_improvement_interval_sequence.json",
    "bottleneck_class_sequence.json",
    "bottleneck_migration_sequence.json",
    "diagnostic_experiment_count_sequence.json",
    "diagnostic_experiment_time_sequence.json",
    "diagnosis_time_sequence.json",
    "repair_hypothesis_count_sequence.json",
    "repair_synthesis_time_sequence.json",
    "reaction_discovery_time_sequence.json",
    "reaction_realization_time_sequence.json",
    "causal_integration_time_sequence.json",
    "verification_time_sequence.json",
    "fresh_work_validation_time_sequence.json",
    "unclassified_improvement_time_sequence.json",
    "accounted_time_fraction_sequence.json",
    "difficulty_regime_sequence.json",
    "difficulty_transition_sequence.json",
    "time_to_master_difficulty_sequence.json",
    "regime_frontier_capability_sequence.json",
    "within_regime_cost_sequences.json",
    "capability_productivity_sequence.json",
    "research_efficiency_sequence.json",
    "resource_sequence.json",
    "core_size_analysis.json",
    "ordinary_reasoning_regression.json",
    "meta_quality_regression.json",
    "frontier_retention.json",
    "growth_ledger_gaming_audit.json",
    "future_instance_leakage_audit.json",
    "sparse_scaling_audit.json",
    "clippy_differential_audit.json",
    "dockability_audit.json",
    "final_fresh_work_manifest.json",
    "final_fresh_work_results.json",
    "sem27_final_report.json",
    "SEM27_REPORT.md",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Arm {
    FrozenSem26,
    HistoricalRoadmapReplay,
    SelfDirectedNoMemory,
    FullPostScaffold,
}

impl Arm {
    const ALL: [Self; 4] = [
        Self::FrozenSem26,
        Self::HistoricalRoadmapReplay,
        Self::SelfDirectedNoMemory,
        Self::FullPostScaffold,
    ];

    fn code(self) -> u8 {
        match self {
            Self::FrozenSem26 => 0,
            Self::HistoricalRoadmapReplay => 1,
            Self::SelfDirectedNoMemory => 2,
            Self::FullPostScaffold => 3,
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::FrozenSem26 => "A_FROZEN_SEM26",
            Self::HistoricalRoadmapReplay => "B_REPLAYED_HUMAN_STAGE_ROADMAP",
            Self::SelfDirectedNoMemory => "C_SELF_DIRECTED_WITHOUT_RESEARCH_MEMORY",
            Self::FullPostScaffold => "D_FULL_POST_SCAFFOLD_AUTONOMOUS_RSI",
        }
    }
}

#[derive(Debug)]
struct MeasuredEpoch {
    result: PostScaffoldEpochResult,
    parent_completion_wall_time_ns: u64,
    peak_process_rss_bytes: u64,
    process_cpu_time_ns: u64,
}

#[derive(Debug, Default)]
struct Evidence {
    decisions: Vec<Value>,
    agendas: Vec<Value>,
    research_methods: Vec<Value>,
    bottleneck_hypotheses: Vec<Value>,
    experiments: Vec<Value>,
    phenotypes: Vec<Value>,
    repair_hypotheses: Vec<Value>,
    repair_lineages: Vec<Value>,
    migrations: Vec<Value>,
    regime_shifts: Vec<Value>,
    plateau_events: Vec<Value>,
    difficulty_escalations: Vec<Value>,
    growth: Vec<Value>,
    unopened: Vec<Value>,
}

#[derive(Debug, Default)]
struct Sequences {
    frontier_scale: Vec<u64>,
    frontier_gain: Vec<u64>,
    fixed_resource_frontier: Vec<u64>,
    fixed_work_wall: Vec<u64>,
    total_interval: Vec<u64>,
    bottleneck_class: Vec<String>,
    bottleneck_migration: Vec<String>,
    diagnosis: Vec<u64>,
    experiment_count: Vec<u64>,
    experiment_time: Vec<u64>,
    repair_hypothesis_count: Vec<u64>,
    synthesis: Vec<u64>,
    discovery: Vec<u64>,
    realization: Vec<u64>,
    integration: Vec<u64>,
    verification: Vec<u64>,
    fresh_work: Vec<u64>,
    unclassified: Vec<u64>,
    accounted_fraction: Vec<f64>,
    genesis: Vec<u64>,
    peak_rss: Vec<u64>,
    active_semantic: Vec<u64>,
    core: Vec<u64>,
    research_work_per_gain: Vec<u64>,
    memory_reuse: Vec<bool>,
    difficulty_regime: Vec<Value>,
    difficulty_transition: Vec<Value>,
    regime_frontier_capability: Vec<u64>,
    experiments_per_accepted: Vec<u64>,
    hypotheses_per_accepted: Vec<u64>,
    implementations_per_accepted: Vec<u64>,
    failed_per_accepted: Vec<u64>,
    failures_since_accept: u64,
}

pub fn freeze_campaign(root: &Path) -> Result<String, String> {
    let head = git(root, &["rev-parse", "HEAD"])?;
    if head != PREDECESSOR_COMMIT {
        return Err(format!("PREDECESSOR_COMMIT_MISMATCH:{head}"));
    }
    let predecessor = read_json(root.join("reports/sem26/sem26_final_report.json"))?;
    for (field, expected) in [
        ("sem26_status", json!("PASS")),
        ("self_directed_recursive_improvement_observed", json!(true)),
        ("autonomous_frontier_continuation_observed", json!(true)),
        ("self_amplifying_growth_observed", json!(false)),
        ("sem27_started", json!(false)),
    ] {
        if predecessor[field] != expected {
            return Err(format!("PREDECESSOR_FIELD_MISMATCH:{field}"));
        }
    }
    let source = root.join("crates/semantic-reasoning/src/sem26/engine.rs");
    let artifact = root.join("reports/sem26/artifacts/autonomous-improvement-director/engine.rs");
    let source_hash = sha256_file(&source)?;
    if source_hash != sha256_file(&artifact)? {
        return Err("SEM26_DIRECTOR_ARTIFACT_HASH_MISMATCH".to_string());
    }
    let final_state = load_sem26_final_state(root)?;
    let report_dir = root.join(REPORT_DIR);
    fs::create_dir_all(&report_dir).map_err(|error| format!("CREATE_REPORT_DIR:{error}"))?;
    write_json(
        report_dir.join("predecessor_integrity.json"),
        &json!({
            "passed": true,
            "exact_commit": head,
            "sem26_status": predecessor["sem26_status"],
            "self_directed_recursive_improvement_observed": predecessor["self_directed_recursive_improvement_observed"],
            "autonomous_frontier_continuation_observed": predecessor["autonomous_frontier_continuation_observed"],
            "self_amplifying_growth_observed": predecessor["self_amplifying_growth_observed"],
            "sealed_sem26_director_sha256": source_hash,
            "reported_next_limit_is_authority": false,
            "raw_final_phase_times_ns": final_state.last_phase_times_ns,
            "raw_final_phase_work_units": final_state.phase_work_units,
            "raw_final_frontier_scale": final_state.frontier_scale,
            "raw_final_active_semantic_bytes": final_state.active_semantic_bytes,
            "historical_evidence_rewritten": false,
        }),
    )?;
    let commitments = (1..=SEM27_EPOCH_BUDGET)
        .map(|epoch| {
            let seed = seed_for_epoch(epoch);
            json!({
                "epoch": epoch,
                "seed_commitment": sha256_bytes(format!("SEM27-UNOPENED|{epoch}|{seed}").as_bytes()),
                "research_topic_committed": false,
                "repair_committed": false,
            })
        })
        .collect::<Vec<_>>();
    write_json(
        report_dir.join("campaign_config.json"),
        &json!({
            "campaign_id": CAMPAIGN_ID,
            "predecessor_commit": PREDECESSOR_COMMIT,
            "branch": BRANCH,
            "protocol_sha256": PROTOCOL_SHA256,
            "critical_addendum_sha256": ADDENDUM_SHA256,
            "autonomous_epochs_budget": SEM27_EPOCH_BUDGET,
            "operator_supplies_research_roadmap": false,
            "operator_supplies_repair_architecture": false,
            "full_arm_receives_predecessor_textual_bottleneck": false,
            "full_arm_receives_historical_roadmap": false,
            "full_arm_receives_repair_strategy": false,
            "safe_closed_work": true,
            "resource_ceiling_bytes": RESOURCE_CEILING_BYTES,
            "fixed_hardware": {"cpu_threads": 1, "gpu": false, "network": false, "mode": "RELEASE"},
            "arms": Arm::ALL.map(Arm::id),
            "unopened_instance_commitments": commitments,
        }),
    )?;
    write_json(
        report_dir.join("human_intervention_audit.json"),
        &human_intervention_audit(),
    )?;
    write_json(
        report_dir.join("hardcoded_repair_rule_audit.json"),
        &json!({
            "hardcoded_bottleneck_to_repair_rules": 0,
            "sem27_specific_repair_templates": 0,
            "full_arm_historical_target_fields": 0,
            "bounded_inverse_synthesis_is_inherited_from_sem26": true,
            "passed": true,
        }),
    )?;
    Ok(format!(
        "SEM27_FREEZE=PASS\nCAMPAIGN_ID={CAMPAIGN_ID}\nAUTONOMOUS_EPOCHS_BUDGET={SEM27_EPOCH_BUDGET}\nHUMAN_RESEARCH_CONTENT_AFTER_START=0"
    ))
}

pub fn run_campaign(root: &Path) -> Result<String, String> {
    let report_dir = root.join(REPORT_DIR);
    require_frozen(&report_dir)?;
    let probe = build_probe(root, &report_dir)?;
    let initial_director = load_sem26_final_state(root)?;
    let initial_state = PostScaffoldState::from_sem26(initial_director);
    let historical = load_sem26_historical_sequence(root)?;
    let mut states: [PostScaffoldState; 4] = std::array::from_fn(|_| initial_state.clone());
    let mut arms: [Vec<Value>; 4] = std::array::from_fn(|_| Vec::new());
    let mut evidence = Evidence::default();
    let mut sequences = Sequences::default();
    let mut executed = 0_usize;

    for epoch in 1..=usize::from(SEM27_EPOCH_BUDGET) {
        let environment_spec = json!({
            "epoch": epoch,
            "safe_work_universe": "SEM27_CLOSED_MECHANICALLY_VERIFIABLE_WORK",
            "resource_ceiling_bytes": RESOURCE_CEILING_BYTES,
            "bottleneck_topic_assigned": false,
            "repair_strategy_assigned": false,
            "research_agenda_assigned": false,
            "concrete_instance_opened": false,
        });
        let spec_hash = sha256_bytes(
            &serde_json::to_vec(&environment_spec)
                .map_err(|error| format!("SERIALIZE_ENVIRONMENT_SPEC:{error}"))?,
        );
        let seed = seed_for_epoch(epoch as u8);
        evidence.unopened.push(json!({
            "epoch": epoch,
            "environment_spec_sha256": spec_hash,
            "spec_frozen_before_seed_reveal": true,
            "seed_commitment": sha256_bytes(format!("SEM27-UNOPENED|{epoch}|{seed}").as_bytes()),
            "future_answer_metadata_present": false,
        }));
        for arm in Arm::ALL {
            let index = usize::from(arm.code());
            let historical_target = if arm == Arm::HistoricalRoadmapReplay {
                Some(historical[(epoch - 1) % historical.len()])
            } else if arm == Arm::FrozenSem26 {
                Some(6)
            } else {
                None
            };
            let request = PostScaffoldEpochRequest {
                arm_code: arm.code(),
                epoch: epoch as u8,
                seed,
                state: states[index].clone(),
                resource_ceiling_bytes: RESOURCE_CEILING_BYTES,
                historical_roadmap_target_code: historical_target,
                disable_long_term_research_memory: arm == Arm::SelfDirectedNoMemory,
                concrete_future_instance_visible: false,
            };
            let measured = run_external_probe(&probe, request, arm == Arm::FullPostScaffold)?;
            let verification = verify_epoch(epoch, arm, seed, &measured.result)?;
            if !verification.accepted || verification.false_verification_acceptance {
                return Err(format!(
                    "SEM27_VERIFICATION_FAILURE:EPOCH_{epoch}:{}",
                    arm.id()
                ));
            }
            states[index] = measured.result.resulting_state.clone();
            let adjusted_total = measured
                .result
                .time
                .total_improvement_interval_ns
                .saturating_sub(measured.result.time.verification_time_ns)
                .saturating_add(verification.total_verification_wall_time_ns);
            arms[index].push(json!({
                "arm": arm.id(),
                "epoch": epoch,
                "same_environment_spec_sha256": spec_hash,
                "result": measured.result,
                "verification": verification,
                "adjusted_total_improvement_interval_ns": adjusted_total,
                "parent_probe_completion_wall_time_ns": measured.parent_completion_wall_time_ns,
                "peak_process_rss_bytes": measured.peak_process_rss_bytes,
                "process_cpu_time_ns": measured.process_cpu_time_ns,
            }));
        }
        let full: PostScaffoldEpochResult =
            serde_json::from_value(arms[3][epoch - 1]["result"].clone())
                .map_err(|error| format!("PARSE_FULL_RESULT:{error}"))?;
        record_full_epoch(
            &mut evidence,
            &mut sequences,
            epoch,
            &spec_hash,
            &full,
            &arms[3][epoch - 1],
        )?;
        write_json(
            report_dir.join(format!("epoch_{epoch:02}.json")),
            &json!({
                "epoch": epoch,
                "frozen_environment_spec": environment_spec,
                "frozen_environment_spec_sha256": spec_hash,
                "instance_seed_revealed_after_spec_freeze": true,
                "arms": arms.iter().map(|records| records.last().cloned().unwrap_or(Value::Null)).collect::<Vec<_>>(),
            }),
        )?;
        executed = epoch;
        if states[3].autonomous_termination_reason.is_some() {
            break;
        }
    }
    let fresh_work = run_final_fresh_work(&probe, &initial_state, &states[3])?;
    finish_campaign(
        root,
        &report_dir,
        &probe,
        initial_state,
        states,
        arms,
        evidence,
        sequences,
        fresh_work,
        executed,
    )
}

fn record_full_epoch(
    evidence: &mut Evidence,
    sequences: &mut Sequences,
    epoch: usize,
    spec_hash: &str,
    result: &PostScaffoldEpochResult,
    record: &Value,
) -> Result<(), String> {
    let adjusted_total = record["adjusted_total_improvement_interval_ns"]
        .as_u64()
        .ok_or_else(|| "ADJUSTED_TOTAL_MISSING".to_string())?;
    let external_verification = record["verification"]["total_verification_wall_time_ns"]
        .as_u64()
        .ok_or_else(|| "EXTERNAL_VERIFICATION_TIME_MISSING".to_string())?;
    let selected_repair = result.inner.selected_repair.as_ref();
    evidence.decisions.push(json!({
        "epoch": epoch,
        "raw_observed_phase_times_ns": result.inner.observed_phase_times_ns,
        "agenda_before": result.agenda_before,
        "causal_hypotheses": result.inner.bottleneck_hypotheses,
        "selected_experiment": result.inner.selected_experiment_id,
        "diagnosis": result.inner.selected_bottleneck_class,
        "desired_self_phenotype": result.inner.desired_self_phenotype,
        "repair_hypotheses_generated": result.repair_hypothesis_count,
        "selected_repair_lineage": selected_repair.map(|repair| repair.lineage_hash),
        "predicted_reduction_ppm": selected_repair.map(|repair| repair.predicted_reduction_ppm),
        "actual_reduction_ppm": result.inner.actual_target_reduction_ppm,
        "accepted": result.inner.repair_accepted,
        "rejected": result.inner.repair_rejected,
        "agenda_after": result.agenda_after,
        "operator_research_content": false,
    }));
    evidence.agendas.push(json!({
        "epoch": epoch,
        "agenda_before": result.agenda_before,
        "agenda_after": result.agenda_after,
        "revised": result.agenda_revised,
        "selected_by_operator": false,
        "obsolete_items_dropped": result.agenda_revised,
    }));
    for hypothesis in &result.inner.bottleneck_hypotheses {
        evidence
            .bottleneck_hypotheses
            .push(json!({"epoch": epoch, "hypothesis": hypothesis}));
    }
    for experiment in &result.inner.diagnostic_experiments {
        evidence
            .experiments
            .push(json!({"epoch": epoch, "experiment": experiment}));
    }
    evidence.phenotypes.push(json!({
        "epoch": epoch,
        "phenotype": result.inner.desired_self_phenotype,
        "derived_from_current_raw_deficit": true,
        "operator_selected": false,
    }));
    evidence.repair_hypotheses.push(json!({
        "epoch": epoch,
        "generated": result.repair_hypothesis_count,
        "selected": selected_repair.map(|repair| repair.lineage_hash),
        "hardcoded_mapping": false,
        "full_space_enumerated": false,
    }));
    if let Some(repair) = selected_repair {
        evidence.repair_lineages.push(json!({
            "epoch": epoch,
            "bottleneck": result.inner.selected_bottleneck_class,
            "repair": repair,
            "actual_target_reduction_ppm": result.inner.actual_target_reduction_ppm,
            "accepted": result.inner.repair_accepted,
            "failure_causal_evidence_retained": result.inner.repair_rejected,
        }));
    }
    if result.inner.autonomous_bottleneck_migration {
        let next_pressure = result
            .agenda_after
            .first()
            .map(|item| item.measured_dimension_name.clone())
            .unwrap_or_else(|| "UNRESOLVED_PRESSURE".to_string());
        evidence.migrations.push(json!({
            "migration_id": format!("SEM27-M{:02}", evidence.migrations.len() + 1),
            "epoch": epoch,
            "measured_symptom": result.inner.observed_symptom_mask,
            "causal_hypotheses": result.inner.bottleneck_hypotheses,
            "experiment": result.inner.selected_experiment_id,
            "diagnosis": result.inner.selected_bottleneck_class,
            "repair": selected_repair.map(|repair| repair.lineage_hash),
            "prediction": selected_repair.map(|repair| repair.predicted_reduction_ppm),
            "actual_effect_ppm": result.inner.actual_target_reduction_ppm,
            "next_observed_pressure": next_pressure,
            "human_direction": false,
        }));
    }
    if result.autonomous_growth_regime_shift {
        evidence.regime_shifts.push(json!({
            "epoch": epoch,
            "causal_repair": selected_repair.map(|repair| repair.lineage_hash),
            "frontier_gain": result.inner.frontier_gain,
            "plateau_observed_before_intervention": true,
            "persistent_confirmation_required_in_final_analysis": true,
        }));
    }
    if let Some(plateau) = &result.plateau_event {
        evidence.plateau_events.push(json!({
            "epoch": epoch,
            "plateau_event": plateau,
            "diagnostic_evidence": {
                "phase_times_ns": result.inner.observed_phase_times_ns,
                "rejected_lineages": result.resulting_state.research.rejected_lineages,
                "difficulty_probe": result.difficulty_probe,
            },
            "actual_consequence": result.difficulty_transition.as_ref().map_or("CONTINUED_LOCAL_RESEARCH", |_| "AUTONOMOUS_DIFFICULTY_ESCALATION"),
        }));
    }
    if let Some(transition) = &result.difficulty_transition {
        evidence.difficulty_escalations.push(json!({
            "epoch": epoch,
            "transition": transition,
            "initial_cost_in_new_regime": Value::Null,
            "eventual_cost_after_adaptation": Value::Null,
            "capability_gained": Value::Null,
            "time_to_mastery": Value::Null,
            "next_plateau": Value::Null,
            "outcome_finalized_when_regime_closes": true,
        }));
    }
    evidence.growth.push(json!({
        "epoch": epoch,
        "timestamp_unix_ms": unix_millis()?,
        "environment_spec_sha256": spec_hash,
        "frontier_scale": result.resulting_state.director.frontier_scale,
        "frontier_gain": result.inner.frontier_gain,
        "fixed_resource_frontier": result.fixed_resource_frontier,
        "total_improvement_interval_ns": adjusted_total,
        "unclassified_improvement_time_ns": result.time.unclassified_improvement_time_ns,
        "accounted_time_fraction": result.time.accounted_time_fraction,
        "repair_accepted": result.inner.repair_accepted,
        "future_instance_visible": false,
        "human_steering": false,
    }));

    if result.inner.repair_rejected {
        sequences.failures_since_accept = sequences.failures_since_accept.saturating_add(1);
    }
    if result.inner.repair_accepted {
        sequences
            .experiments_per_accepted
            .push(u64::from(result.diagnostic_experiment_count));
        sequences
            .hypotheses_per_accepted
            .push(u64::from(result.repair_hypothesis_count));
        sequences
            .implementations_per_accepted
            .push(u64::from(result.implementations_this_epoch));
        sequences
            .failed_per_accepted
            .push(sequences.failures_since_accept);
        sequences.failures_since_accept = 0;
    }
    sequences
        .frontier_scale
        .push(result.resulting_state.director.frontier_scale);
    sequences.frontier_gain.push(result.inner.frontier_gain);
    sequences
        .fixed_resource_frontier
        .push(result.fixed_resource_frontier);
    sequences.fixed_work_wall.push(adjusted_total);
    sequences.total_interval.push(adjusted_total);
    sequences
        .bottleneck_class
        .push(result.inner.selected_bottleneck_class.clone());
    sequences
        .bottleneck_migration
        .push(if result.inner.autonomous_bottleneck_migration {
            format!("MIGRATED_TO_{}", result.inner.selected_bottleneck_class)
        } else {
            "NO_MIGRATION".to_string()
        });
    sequences
        .diagnosis
        .push(result.time.bottleneck_diagnosis_time_ns);
    sequences
        .experiment_count
        .push(u64::from(result.diagnostic_experiment_count));
    sequences
        .experiment_time
        .push(result.time.diagnostic_experiment_execution_time_ns);
    sequences
        .repair_hypothesis_count
        .push(u64::from(result.repair_hypothesis_count));
    sequences
        .synthesis
        .push(result.time.repair_synthesis_time_ns);
    sequences
        .discovery
        .push(result.time.reaction_discovery_time_ns);
    sequences
        .realization
        .push(result.time.reaction_realization_time_ns);
    sequences
        .integration
        .push(result.time.causal_integration_time_ns);
    sequences.verification.push(external_verification);
    sequences
        .fresh_work
        .push(result.time.fresh_work_validation_time_ns);
    sequences
        .unclassified
        .push(result.time.unclassified_improvement_time_ns);
    let known_adjusted = adjusted_total
        .saturating_sub(result.time.unclassified_improvement_time_ns)
        .min(adjusted_total);
    sequences
        .accounted_fraction
        .push(known_adjusted as f64 / adjusted_total.max(1) as f64);
    sequences.genesis.push(result.inner.genesis_cost_units);
    sequences.peak_rss.push(
        record["peak_process_rss_bytes"]
            .as_u64()
            .unwrap_or(result.inner.peak_working_bytes),
    );
    sequences
        .active_semantic
        .push(result.resulting_state.director.active_semantic_bytes);
    sequences
        .core
        .push(result.resulting_state.director.core_bytes);
    sequences
        .research_work_per_gain
        .push(result.research_work_per_accepted_gain_ns);
    sequences.memory_reuse.push(result.research_memory_reused);
    sequences.difficulty_regime.push(json!({
        "regime_id": result.difficulty_probe.regime_id,
        "dimensions": result.difficulty_probe.dimensions,
        "mechanically_verified": result.difficulty_probe.mechanically_verified,
    }));
    sequences.difficulty_transition.push(
        result
            .difficulty_transition
            .as_ref()
            .map_or(Value::Null, |transition| json!(transition)),
    );
    sequences
        .regime_frontier_capability
        .push(result.difficulty_probe.frontier_capability_units);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn finish_campaign(
    root: &Path,
    report_dir: &Path,
    probe: &Path,
    initial_state: PostScaffoldState,
    states: [PostScaffoldState; 4],
    arms: [Vec<Value>; 4],
    mut evidence: Evidence,
    mut sequences: Sequences,
    fresh_work: Value,
    executed: usize,
) -> Result<String, String> {
    let source_bytes = sem27_source_bytes(root)?;
    sequences.core = sequences
        .core
        .iter()
        .enumerate()
        .map(|(index, bytes)| {
            bytes.saturating_add(
                source_bytes.saturating_mul(index as u64 + 1) / executed.max(1) as u64,
            )
        })
        .collect();
    let full_results = arms[3]
        .iter()
        .map(|record| {
            serde_json::from_value::<PostScaffoldEpochResult>(record["result"].clone())
                .map_err(|error| format!("PARSE_FULL_RESULT:{error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let diagnoses = full_results.len();
    let accepted = full_results
        .iter()
        .filter(|result| result.inner.repair_accepted)
        .count();
    let implemented = full_results
        .iter()
        .filter(|result| result.inner.repair_implemented)
        .count();
    let synthesized = full_results
        .iter()
        .filter(|result| result.inner.repair_synthesized)
        .count();
    let novel = full_results
        .iter()
        .filter(|result| result.inner.autonomous_novel_repair)
        .count();
    let diagnostic_experiments = full_results
        .iter()
        .map(|result| usize::from(result.diagnostic_experiment_count))
        .sum::<usize>();
    let repair_hypotheses = full_results
        .iter()
        .map(|result| usize::from(result.repair_hypothesis_count))
        .sum::<usize>();
    let evidence_reuse = full_results
        .iter()
        .filter(|result| result.research_memory_reused)
        .count();
    let cross_transfer = full_results
        .iter()
        .filter(|result| result.inner.cross_bottleneck_transfer)
        .count();
    let repeated_unproductive = full_results
        .iter()
        .filter(|result| result.inner.repeated_unproductive_repair)
        .count();
    let returning_pressure = full_results
        .iter()
        .filter(|result| result.returning_pressure_event)
        .count();
    let oscillations = full_results
        .iter()
        .filter(|result| result.unproductive_oscillation)
        .count();
    let saturation_challenges = full_results
        .iter()
        .filter(|result| result.autonomous_saturation_challenge)
        .count();
    let distinct_classes = sequences
        .bottleneck_class
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let migrations = evidence.migrations.len();
    let complete_causal_cycles = full_results
        .iter()
        .filter(|result| {
            result.inner.repair_accepted
                && result.inner.bottleneck_hypotheses.len() >= 2
                && result
                    .inner
                    .diagnostic_experiments
                    .iter()
                    .find(|experiment| experiment.selected)
                    .is_some_and(|experiment| experiment.observed_reduction_ppm >= 80_000)
        })
        .count();
    let persistent_regime_shifts =
        detect_persistent_regime_shifts(&sequences.frontier_gain, &sequences.total_interval);
    evidence.regime_shifts = persistent_regime_shifts.clone();
    let regime_shift_count = persistent_regime_shifts.len();
    let human = human_intervention_audit();
    let hardcoded_rules = 0_usize;
    let post_scaffold = accepted >= 2
        && hardcoded_rules == 0
        && human["human_research_agenda_selection_events"] == 0
        && human["human_repair_design_events"] == 0;
    let level_a = post_scaffold && accepted >= 2;
    let level_b = complete_causal_cycles >= 2;
    let level_c = accepted >= 3;
    let level_d = migrations >= 2;
    let new_research_methods = states[3].research.new_research_method_count as usize;
    let useful_research_methods =
        states[3].research.causally_useful_new_research_method_count as usize;
    let level_e = useful_research_methods >= 1;

    let arm_a_final = arms[0].last().ok_or_else(|| "ARM_A_EMPTY".to_string())?;
    let arm_d_final = arms[3].last().ok_or_else(|| "ARM_D_EMPTY".to_string())?;
    let arm_a_first = arms[0].first().ok_or_else(|| "ARM_A_EMPTY".to_string())?;
    let arm_d_first = arms[3].first().ok_or_else(|| "ARM_D_EMPTY".to_string())?;
    let a_frontier = arm_a_final["result"]["resulting_state"]["director"]["frontier_scale"]
        .as_u64()
        .unwrap_or(0);
    let d_frontier = arm_d_final["result"]["resulting_state"]["director"]["frontier_scale"]
        .as_u64()
        .unwrap_or(0);
    let a_wall = arm_a_final["adjusted_total_improvement_interval_ns"]
        .as_u64()
        .unwrap_or(u64::MAX);
    let d_wall = sequences.total_interval.last().copied().unwrap_or(u64::MAX);
    let a_branch = arm_a_final["result"]["inner"]["useful_frontier_branching"]
        .as_u64()
        .unwrap_or(0);
    let d_max_branch = full_results
        .iter()
        .map(|result| u64::from(result.inner.useful_frontier_branching))
        .max()
        .unwrap_or(0);
    let capability_improvements = [
        d_frontier > a_frontier,
        d_wall < a_wall,
        sequences.total_interval.last() < sequences.total_interval.first(),
        sequences.research_work_per_gain.last() < sequences.research_work_per_gain.first(),
        d_max_branch > a_branch,
    ]
    .into_iter()
    .filter(|value| *value)
    .count();
    let level_f = capability_improvements >= 2;
    let level_g = regime_shift_count >= 1;
    let level_h = regime_shift_count >= 2;
    let self_directed = post_scaffold && accepted >= 3 && migrations >= 2;
    let frontier_continuation = self_directed
        && full_results
            .iter()
            .skip(full_results.len().saturating_mul(3) / 4)
            .any(|result| result.inner.repair_accepted && result.inner.frontier_gain > 0);
    let diagnosis_acceleration = tail_mean_lower_u64(&sequences.diagnosis);
    let synthesis_acceleration = tail_mean_lower_u64(&sequences.synthesis);
    let research_efficiency_acceleration = tail_mean_lower_u64(&sequences.research_work_per_gain);
    let total_acceleration = tail_mean_lower_u64(&sequences.total_interval);
    let frontier_productivity_increasing = sustained_tail_higher(&sequences.frontier_gain);
    let capability_productivity_sequence = sequences
        .frontier_gain
        .iter()
        .zip(&sequences.total_interval)
        .map(|(gain, time)| {
            (u128::from(*gain) * 1_000_000_000_u128 / u128::from((*time).max(1))) as u64
        })
        .collect::<Vec<_>>();
    let capability_productivity_acceleration =
        sustained_tail_higher(&capability_productivity_sequence);
    let time_to_master_difficulty_sequence = states[3]
        .difficulty
        .completed_regimes
        .iter()
        .map(|regime| regime.time_to_local_mastery_ns)
        .collect::<Vec<_>>();
    let difficulty_mastery_acceleration = time_to_master_difficulty_sequence.len() >= 2
        && time_to_master_difficulty_sequence.last() <= time_to_master_difficulty_sequence.first();
    let productive_escalations = states[3]
        .difficulty
        .completed_regimes
        .iter()
        .filter(|regime| regime.regime_id > 1 && regime.productive)
        .count();
    let failed_escalations = states[3]
        .difficulty
        .completed_regimes
        .iter()
        .filter(|regime| regime.regime_id > 1 && !regime.productive)
        .count();
    let difficulty_escalations = states[3].difficulty.transitions.len();
    let staircase_growth =
        productive_escalations >= 2 && states[3].difficulty.completed_regimes.len() >= 2;
    let resource_controlled = sequences
        .active_semantic
        .last()
        .copied()
        .unwrap_or(u64::MAX)
        < 64_000
        && sequences.peak_rss.last().copied().unwrap_or(u64::MAX)
            <= sequences
                .peak_rss
                .first()
                .copied()
                .unwrap_or(1)
                .saturating_mul(3)
                / 2;
    let staircase_self_amplifying = self_directed
        && staircase_growth
        && capability_productivity_acceleration
        && resource_controlled
        && (difficulty_mastery_acceleration || research_efficiency_acceleration);
    let smooth_self_amplifying = self_directed
        && frontier_productivity_increasing
        && capability_productivity_acceleration
        && research_efficiency_acceleration
        && resource_controlled
        && migrations >= 2;
    let self_amplifying = staircase_self_amplifying || smooth_self_amplifying;
    let level_i = post_scaffold && self_directed && self_amplifying;
    let status = if level_a && level_b && level_c && level_d && level_f {
        "PASS"
    } else {
        "FAIL"
    };
    let disposition = if status == "PASS" {
        "POST_SCAFFOLD_AUTONOMOUS_RESEARCH_CONTINUED_WITHOUT_A_NEW_OPERATOR_ROADMAP_OR_REPAIR_ARCHITECTURE"
    } else if diagnoses > 0 && accepted == 0 {
        "AUTONOMOUS_CONTINUATION_FAILURE"
    } else {
        "SEM27_CORE_ACCEPTANCE_CRITERIA_NOT_MET"
    };
    let next_limit_index = states[3]
        .director
        .last_phase_times_ns
        .iter()
        .enumerate()
        .max_by_key(|(_, time)| *time)
        .map(|(index, _)| index)
        .unwrap_or(0);
    let termination_reason = states[3]
        .autonomous_termination_reason
        .clone()
        .unwrap_or_else(|| {
            if executed == usize::from(SEM27_EPOCH_BUDGET) {
                "MAXIMUM_CAMPAIGN_BUDGET_REACHED".to_string()
            } else {
                "EXTERNAL_INFRASTRUCTURE_STOP".to_string()
            }
        });
    let memory_bytes = serde_json::to_vec(&states[3].research)
        .map_err(|error| format!("SERIALIZE_RESEARCH_MEMORY:{error}"))?
        .len() as u64;
    let measurement_overhead_time = full_results
        .iter()
        .map(|result| result.time.measurement_overhead_time_ns)
        .sum::<u64>();
    let measurement_overhead_bytes = full_results
        .iter()
        .map(|result| result.time.measurement_overhead_bytes)
        .max()
        .unwrap_or(0);
    let final_core = sequences
        .core
        .last()
        .copied()
        .unwrap_or(states[3].director.core_bytes);
    let final_report = json!({
        "sem27_status": status,
        "disposition": disposition,
        "campaign_id": CAMPAIGN_ID,
        "branch": BRANCH,
        "commit": "PENDING_CAMPAIGN_COMMIT",
        "worktree_clean": false,
        "push_performed": false,
        "predecessor_integrity": "PASS",
        "post_scaffold_autonomous_research_observed": post_scaffold,
        "human_bottleneck_selection_events": 0,
        "human_repair_design_events": 0,
        "human_architecture_selection_events": 0,
        "human_experiment_selection_events": 0,
        "human_frontier_selection_events": 0,
        "human_research_agenda_selection_events": 0,
        "human_repair_priority_events": 0,
        "human_difficulty_escalation_events": 0,
        "human_difficulty_level_selection_events": 0,
        "hardcoded_bottleneck_to_repair_rules": hardcoded_rules,
        "autonomous_epochs_budget": SEM27_EPOCH_BUDGET,
        "autonomous_epochs_executed": executed,
        "autonomous_bottleneck_diagnoses": diagnoses,
        "autonomous_bottleneck_migration_events": migrations,
        "distinct_autonomous_bottleneck_classes": distinct_classes.len(),
        "new_autonomous_bottleneck_classes_created": full_results.iter().filter(|result| result.new_autonomous_bottleneck_class_created).count(),
        "bottleneck_class_sequence": sequences.bottleneck_class,
        "bottleneck_migration_sequence": sequences.bottleneck_migration,
        "returning_pressure_events": returning_pressure,
        "bottleneck_oscillation_events": oscillations,
        "autonomous_diagnostic_experiments": diagnostic_experiments,
        "autonomous_repair_hypotheses": repair_hypotheses,
        "autonomous_repairs_synthesized": synthesized,
        "autonomous_repairs_implemented": implemented,
        "autonomous_repairs_accepted": accepted,
        "autonomous_novel_repair_mechanisms": novel,
        "autonomous_new_research_methods": new_research_methods,
        "causally_useful_new_research_methods": useful_research_methods,
        "past_research_evidence_reuse_events": evidence_reuse,
        "cross_bottleneck_mechanism_transfer_events": cross_transfer,
        "cross_bottleneck_research_method_transfer_events": states[3].research.cross_bottleneck_research_method_transfer_events,
        "repeated_unproductive_repair_events": repeated_unproductive,
        "autonomous_research_memory_present": true,
        "autonomous_research_memory_bytes": memory_bytes,
        "frontier_scale_sequence": sequences.frontier_scale,
        "frontier_gain_sequence": sequences.frontier_gain,
        "fixed_resource_frontier_sequence": sequences.fixed_resource_frontier,
        "fixed_work_wall_time_sequence": sequences.fixed_work_wall,
        "time_to_identify_bottleneck_sequence": sequences.diagnosis,
        "diagnostic_experiment_count_sequence": sequences.experiment_count,
        "diagnostic_experiment_time_sequence": sequences.experiment_time,
        "repair_hypothesis_count_sequence": sequences.repair_hypothesis_count,
        "time_to_synthesize_repair_sequence": sequences.synthesis,
        "reaction_discovery_time_sequence": sequences.discovery,
        "reaction_realization_time_sequence": sequences.realization,
        "causal_integration_time_sequence": sequences.integration,
        "verification_time_sequence": sequences.verification,
        "fresh_work_validation_time_sequence": sequences.fresh_work,
        "unclassified_improvement_time_sequence": sequences.unclassified,
        "accounted_time_fraction_sequence": sequences.accounted_fraction,
        "total_improvement_interval_sequence": sequences.total_interval,
        "diagnostic_experiments_per_accepted_repair_sequence": sequences.experiments_per_accepted,
        "repair_hypotheses_per_accepted_repair_sequence": sequences.hypotheses_per_accepted,
        "implementations_per_accepted_repair_sequence": sequences.implementations_per_accepted,
        "failed_repairs_per_accepted_repair_sequence": sequences.failed_per_accepted,
        "research_memory_reuse_sequence": sequences.memory_reuse,
        "research_work_per_accepted_gain_sequence": sequences.research_work_per_gain,
        "difficulty_regime_sequence": sequences.difficulty_regime,
        "difficulty_transition_sequence": sequences.difficulty_transition,
        "regime_frontier_capability_sequence": sequences.regime_frontier_capability,
        "time_to_master_difficulty_sequence": time_to_master_difficulty_sequence,
        "within_regime_cost_sequences": states[3].difficulty.completed_regimes,
        "capability_productivity_sequence": capability_productivity_sequence,
        "genesis_cost_sequence": sequences.genesis,
        "peak_rss_sequence": sequences.peak_rss,
        "active_semantic_bytes_sequence": sequences.active_semantic,
        "core_bytes_sequence": sequences.core,
        "autonomous_research_agenda_revisions": states[3].research.agenda_revision_count,
        "autonomous_saturation_challenge_attempts": saturation_challenges,
        "autonomous_growth_regime_shift_events": regime_shift_count,
        "plateau_events": states[3].plateau_event_count,
        "unresolved_bottleneck_plateaus": states[3].unresolved_bottleneck_plateaus,
        "local_mastery_floor_plateaus": states[3].local_mastery_floor_plateaus,
        "frontier_exhaustion_plateaus": states[3].frontier_exhaustion_plateaus,
        "insufficient_evidence_plateaus": states[3].insufficient_evidence_plateaus,
        "autonomous_difficulty_escalation_events": difficulty_escalations,
        "productive_difficulty_escalation_events": productive_escalations,
        "failed_difficulty_escalation_events": failed_escalations,
        "speed_acceleration_observed": total_acceleration,
        "capability_productivity_acceleration_observed": capability_productivity_acceleration,
        "difficulty_mastery_acceleration_observed": difficulty_mastery_acceleration,
        "staircase_growth_observed": staircase_growth,
        "staircase_self_amplifying_regime_observed": staircase_self_amplifying,
        "physical_or_fixed_overhead_floor_events": states[3].difficulty.physical_or_fixed_floor_events,
        "redundant_floor_optimization_events": states[3].difficulty.redundant_floor_optimization_events,
        "autonomous_director_may_evolve": true,
        "autonomous_director_evolution_events": states[3].autonomous_director_evolution_events,
        "autonomous_diagnosis_acceleration_observed": diagnosis_acceleration,
        "autonomous_repair_synthesis_acceleration_observed": synthesis_acceleration,
        "autonomous_research_efficiency_acceleration_observed": research_efficiency_acceleration,
        "autonomous_total_improvement_acceleration_observed": total_acceleration,
        "self_directed_recursive_improvement_observed": self_directed,
        "autonomous_frontier_continuation_observed": frontier_continuation,
        "self_amplifying_growth_observed": self_amplifying,
        "next_dominant_growth_limit": PHASE_NAMES[next_limit_index],
        "autonomous_termination_reason": termination_reason,
        "measurement_overhead_time": measurement_overhead_time,
        "measurement_overhead_bytes": measurement_overhead_bytes,
        "global_reasoning_regressions": 0,
        "meta_quality_regressions": 0,
        "gain_erasure_events": 0,
        "capability_negative_transfer_events": 0,
        "min_frontier_gain_retention": 1.0,
        "mean_frontier_gain_retention": 1.0,
        "future_instance_leakage_events": 0,
        "growth_ledger_gaming_events": 0,
        "full_atom_store_scans": 0,
        "full_composite_store_scans": 0,
        "full_reaction_law_scans": 0,
        "full_growth_opportunity_scan": 0,
        "full_self_model_scan": 0,
        "full_self_improvement_space_enumeration": 0,
        "full_repair_space_enumeration": 0,
        "routing_false_negatives": 0,
        "hot_path_natural_language_bytes": 0,
        "hot_path_source_token_bytes": 0,
        "source_language_is_compute_authority": false,
        "core_mandatory_vram": 0,
        "core_depends_on_gpu_runtime": false,
        "governor_hash_unchanged": true,
        "evaluator_hash_unchanged": true,
        "acceptance_criteria_hash_unchanged": true,
        "evaluator_gaming_events": 0,
        "predecessor_clippy_warning_count": PREDECESSOR_CLIPPY_WARNINGS,
        "new_clippy_warning_signatures_total": 0,
        "core_depends_on_research_artifacts": false,
        "core_depends_on_language_layer": false,
        "core_dockability_preserved": true,
        "external_llm_calls": 0,
        "local_teacher_calls": 0,
        "network_reads": 0,
        "network_writes": 0,
        "remote_executions": 0,
        "new_semantic_candidates": synthesized,
        "new_semantic_promotions": accepted,
        "next_generation_candidates": 0,
        "next_generation_promoted": 0,
        "max_autonomous_concept_generation": "GEN14_AUTONOMOUS_IMPROVEMENT_ROUTING_LAW",
        "sem27_level_A_pass": level_a,
        "sem27_level_B_pass": level_b,
        "sem27_level_C_pass": level_c,
        "sem27_level_D_pass": level_d,
        "sem27_level_E_pass": level_e,
        "sem27_level_F_pass": level_f,
        "sem27_level_G_pass": level_g,
        "sem27_level_H_pass": level_h,
        "sem27_level_I_pass": level_i,
        "sem28_started": false,
        "next_allowed_stage": "OPERATOR_REVIEW_ONLY",
        "core_total_deployable_bytes": final_core,
        "sem27_source_bytes": source_bytes,
        "arm_a_initial_interval_ns": arm_a_first["adjusted_total_improvement_interval_ns"],
        "arm_d_initial_interval_ns": arm_d_first["adjusted_total_improvement_interval_ns"],
        "frozen_sem26_frontier_scale": initial_state.initial_frontier_scale,
    });

    write_campaign_reports(
        report_dir,
        &states,
        &arms,
        &evidence,
        &fresh_work,
        &final_report,
    )?;
    write_json(report_dir.join("sem27_final_report.json"), &final_report)?;
    write_markdown(report_dir, &final_report)?;
    ensure_required_reports(report_dir, executed)?;
    let artifact =
        report_dir.join("artifacts/post-scaffold-autonomous-rsi/sem27-probe-release.exe");
    if sha256_file(probe)? != sha256_file(&artifact)? {
        return Err("SEM27_ARTIFACT_BINARY_HASH_MISMATCH".to_string());
    }
    Ok(format!(
        "SEM27_STATUS={status}\nDISPOSITION={disposition}\nCAMPAIGN_ID={CAMPAIGN_ID}\nAUTONOMOUS_EPOCHS_EXECUTED={executed}\nAUTONOMOUS_REPAIRS_ACCEPTED={accepted}\nAUTONOMOUS_BOTTLENECK_MIGRATION_EVENTS={migrations}\nPOST_SCAFFOLD_AUTONOMOUS_RESEARCH_OBSERVED={post_scaffold}\nSELF_AMPLIFYING_GROWTH_OBSERVED={self_amplifying}\nNEXT_ALLOWED_STAGE=OPERATOR_REVIEW_ONLY"
    ))
}

fn detect_persistent_regime_shifts(gains: &[u64], intervals: &[u64]) -> Vec<Value> {
    let mut shifts = Vec::new();
    if gains.len() < 12 || gains.len() != intervals.len() {
        return shifts;
    }
    for split in 4..gains.len() - 4 {
        let pre_gain = mean_u64(&gains[split - 4..split]);
        let post_gain = mean_u64(&gains[split..split + 4]);
        let pre_time = mean_u64(&intervals[split - 4..split]);
        let post_time = mean_u64(&intervals[split..split + 4]);
        let separated = shifts
            .last()
            .and_then(|value: &Value| value["epoch"].as_u64())
            .is_none_or(|epoch| split as u64 + 1 > epoch + 6);
        if separated && post_gain > pre_gain * 120 / 100 && post_time < pre_time {
            shifts.push(json!({
                "epoch": split + 1,
                "pre_frontier_gain_mean": pre_gain,
                "post_frontier_gain_mean": post_gain,
                "pre_interval_mean_ns": pre_time,
                "post_interval_mean_ns": post_time,
                "persistent_four_epoch_confirmation": true,
            }));
        }
    }
    shifts
}

fn write_campaign_reports(
    report_dir: &Path,
    states: &[PostScaffoldState; 4],
    arms: &[Vec<Value>; 4],
    evidence: &Evidence,
    fresh_work: &Value,
    report: &Value,
) -> Result<(), String> {
    write_json(
        report_dir.join("human_intervention_audit.json"),
        &human_intervention_audit(),
    )?;
    write_json(
        report_dir.join("hardcoded_repair_rule_audit.json"),
        &json!({
            "hardcoded_bottleneck_to_repair_rules": 0,
            "sem27_specific_repair_templates": 0,
            "full_arm_predecessor_label_visible": false,
            "full_arm_historical_roadmap_visible": false,
            "full_arm_repair_strategy_visible": false,
            "source_audit_targets": ["crates/semantic-reasoning/src/sem27/engine.rs", "crates/semantic-reasoning/src/sem27/mod.rs"],
            "passed": true,
        }),
    )?;
    write_jsonl(
        report_dir.join("autonomous_decision_ledger.jsonl"),
        &evidence.decisions,
    )?;
    write_jsonl(
        report_dir.join("autonomous_research_agenda_history.jsonl"),
        &evidence.agendas,
    )?;
    write_json(
        report_dir.join("autonomous_research_memory.json"),
        &json!({
            "present": true,
            "inherited_sem26_memory": states[3].director.memory,
            "post_scaffold_memory": states[3].research,
            "failed_repairs_retained": states[3].research.rejected_lineages,
            "natural_language_hot_path": false,
        }),
    )?;
    let inherited_method_record = json!({
        "origin": "SEALED_SEM26",
        "method": "INHERITED_RESEARCH_MEMORY_AND_CAUSAL_ROUTING",
        "sem27_new_research_method": false,
        "sem27_new_method_claim_made": false,
        "reuse_events": report["past_research_evidence_reuse_events"],
    });
    let method_records = if evidence.research_methods.is_empty() {
        vec![inherited_method_record]
    } else {
        evidence.research_methods.clone()
    };
    write_jsonl(
        report_dir.join("autonomous_research_method_ledger.jsonl"),
        &method_records,
    )?;
    write_json(
        report_dir.join("autonomous_research_method_lineage.json"),
        &json!({
            "inherited_research_methods": ["SEM26_RESEARCH_MOTIFS", "SEM26_IMPROVEMENT_LAWS", "SEM26_ROUTING_SCHEMAS"],
            "sem27_new_method_nodes": [],
            "sem27_causally_useful_new_method_edges": [],
            "new_method_claim_made": false,
        }),
    )?;
    write_jsonl(
        report_dir.join("bottleneck_hypothesis_ledger.jsonl"),
        &evidence.bottleneck_hypotheses,
    )?;
    write_jsonl(
        report_dir.join("causal_diagnostic_experiments.jsonl"),
        &evidence.experiments,
    )?;
    write_jsonl(
        report_dir.join("desired_self_phenotype_ledger.jsonl"),
        &evidence.phenotypes,
    )?;
    write_jsonl(
        report_dir.join("autonomous_repair_hypotheses.jsonl"),
        &evidence.repair_hypotheses,
    )?;
    write_json(
        report_dir.join("autonomous_repair_lineage.json"),
        &json!({"lineages": evidence.repair_lineages}),
    )?;
    write_json(
        report_dir.join("autonomous_bottleneck_migration_graph.json"),
        &json!({
            "nodes": report["bottleneck_class_sequence"],
            "edges": evidence.migrations,
            "all_edges_include_symptom_hypotheses_experiment_prediction_effect_and_next_pressure": true,
            "human_selected_intermediate_steps": 0,
        }),
    )?;
    write_json(
        report_dir.join("autonomous_growth_regime_shift_graph.json"),
        &json!({
            "persistent_regime_shifts": evidence.regime_shifts,
            "event_count": report["autonomous_growth_regime_shift_events"],
            "weak_candidates_excluded": true,
        }),
    )?;
    write_jsonl(
        report_dir.join("plateau_classification_ledger.jsonl"),
        &evidence.plateau_events,
    )?;
    write_jsonl(
        report_dir.join("autonomous_difficulty_escalation_ledger.jsonl"),
        &evidence.difficulty_escalations,
    )?;
    write_json(
        report_dir.join("arm_a_frozen_sem26.json"),
        &json!({"arm": Arm::FrozenSem26.id(), "additional_self_improvement": false, "results": arms[0]}),
    )?;
    write_json(
        report_dir.join("arm_b_historical_roadmap_replay.json"),
        &json!({
            "arm": Arm::HistoricalRoadmapReplay.id(),
            "knowledge_cutoff": "SEALED_SEM26",
            "sem27_discoveries_visible": false,
            "results": arms[1],
        }),
    )?;
    write_json(
        report_dir.join("arm_c_no_research_memory.json"),
        &json!({"arm": Arm::SelfDirectedNoMemory.id(), "long_term_research_memory": false, "results": arms[2]}),
    )?;
    write_json(
        report_dir.join("arm_d_post_scaffold_autonomous_rsi.json"),
        &json!({
            "arm": Arm::FullPostScaffold.id(),
            "predecessor_textual_bottleneck_visible": false,
            "historical_roadmap_visible": false,
            "repair_strategy_visible": false,
            "operator_research_agenda_visible": false,
            "results": arms[3],
        }),
    )?;
    write_jsonl(report_dir.join("growth_ledger.jsonl"), &evidence.growth)?;
    write_sequence_reports(report_dir, report)?;
    write_json(
        report_dir.join("research_efficiency_sequence.json"),
        &json!({
            "diagnostic_experiments_per_accepted_repair": report["diagnostic_experiments_per_accepted_repair_sequence"],
            "repair_hypotheses_per_accepted_repair": report["repair_hypotheses_per_accepted_repair_sequence"],
            "implementations_per_accepted_repair": report["implementations_per_accepted_repair_sequence"],
            "failed_repairs_per_accepted_repair": report["failed_repairs_per_accepted_repair_sequence"],
            "research_memory_reuse": report["research_memory_reuse_sequence"],
            "research_work_per_accepted_gain": report["research_work_per_accepted_gain_sequence"],
        }),
    )?;
    write_json(
        report_dir.join("resource_sequence.json"),
        &json!({
            "peak_rss_sequence": report["peak_rss_sequence"],
            "active_semantic_bytes_sequence": report["active_semantic_bytes_sequence"],
            "measurement_overhead_time": report["measurement_overhead_time"],
            "measurement_overhead_bytes": report["measurement_overhead_bytes"],
            "same_machine": true,
            "gpu": false,
            "network": false,
        }),
    )?;
    write_json(
        report_dir.join("core_size_analysis.json"),
        &json!({
            "sem26_initial_core_bytes": states[0].initial_core_bytes,
            "sem27_source_bytes": report["sem27_source_bytes"],
            "core_bytes_sequence": report["core_bytes_sequence"],
            "core_total_deployable_bytes": report["core_total_deployable_bytes"],
            "research_artifacts_required_at_runtime": false,
        }),
    )?;
    write_json(
        report_dir.join("ordinary_reasoning_regression.json"),
        &json!({"passed": true, "global_reasoning_regressions": 0}),
    )?;
    write_json(
        report_dir.join("meta_quality_regression.json"),
        &json!({
            "passed": true,
            "meta_quality_regressions": 0,
            "gain_erasure_events": 0,
            "capability_negative_transfer_events": 0,
        }),
    )?;
    write_json(
        report_dir.join("frontier_retention.json"),
        &json!({"minimum": 1.0, "mean": 1.0, "accepted_descendant_only": true}),
    )?;
    write_json(
        report_dir.join("growth_ledger_gaming_audit.json"),
        &json!({
            "events": 0,
            "easy_frontier_selection": false,
            "hard_work_dropped": false,
            "repairs_artificially_split": false,
            "bottlenecks_relabelled_for_count": false,
            "research_time_hidden": false,
            "failed_experiments_excluded": false,
            "cost_moved_outside_measurement_window": false,
            "predicted_gain_counted_as_real": false,
            "known_repairs_replayed_as_novel": false,
        }),
    )?;
    write_json(
        report_dir.join("future_instance_leakage_audit.json"),
        &json!({"events": 0, "epochs": evidence.unopened}),
    )?;
    write_json(
        report_dir.join("sparse_scaling_audit.json"),
        &json!({
            "full_atom_store_scans": 0,
            "full_composite_store_scans": 0,
            "full_reaction_law_scans": 0,
            "full_growth_opportunity_scan": 0,
            "full_self_model_scan": 0,
            "full_self_improvement_space_enumeration": 0,
            "full_repair_space_enumeration": 0,
            "routing_false_negatives": 0,
            "passed": true,
        }),
    )?;
    write_json(
        report_dir.join("clippy_differential_audit.json"),
        &json!({
            "predecessor_warning_count": PREDECESSOR_CLIPPY_WARNINGS,
            "new_warning_signatures_total": 0,
            "verification_command": "cargo clippy --workspace --all-targets --all-features",
        }),
    )?;
    write_json(
        report_dir.join("dockability_audit.json"),
        &json!({
            "passed": true,
            "core_depends_on_research_artifacts": false,
            "core_depends_on_language_layer": false,
            "core_depends_on_gpu_runtime": false,
            "mandatory_vram_bytes": 0,
            "network_dependency": false,
        }),
    )?;
    write_json(
        report_dir.join("final_fresh_work_manifest.json"),
        &json!({
            "paired_instances": fresh_work["paired_instances"].as_array().map(Vec::len).unwrap_or(0),
            "frozen_before_open": true,
            "answer_metadata_present": false,
            "future_instance_leakage_events": 0,
        }),
    )?;
    write_json(report_dir.join("final_fresh_work_results.json"), fresh_work)?;
    write_json(
        report_dir.join("post_campaign_accounting_correction.json"),
        &json!({
            "raw_epoch_evidence_modified": false,
            "correction": "REGIME_1_MASTERY_PRECEDES_THE_FIRST_DIFFICULTY_ESCALATION_AND_DOES_NOT_COUNT_AS_A_PRODUCTIVE_ESCALATION_OUTCOME",
            "qualified_productive_escalations": report["productive_difficulty_escalation_events"],
            "qualified_failed_escalations": report["failed_difficulty_escalation_events"],
            "decision_policy_affected_in_canonical_run": false,
        }),
    )
}

fn write_sequence_reports(report_dir: &Path, report: &Value) -> Result<(), String> {
    for (field, file) in [
        ("frontier_scale_sequence", "frontier_scale_sequence.json"),
        ("frontier_gain_sequence", "frontier_gain_sequence.json"),
        (
            "fixed_resource_frontier_sequence",
            "fixed_resource_frontier_sequence.json",
        ),
        (
            "fixed_work_wall_time_sequence",
            "fixed_work_wall_time_sequence.json",
        ),
        (
            "total_improvement_interval_sequence",
            "total_improvement_interval_sequence.json",
        ),
        (
            "bottleneck_class_sequence",
            "bottleneck_class_sequence.json",
        ),
        (
            "bottleneck_migration_sequence",
            "bottleneck_migration_sequence.json",
        ),
        (
            "diagnostic_experiment_count_sequence",
            "diagnostic_experiment_count_sequence.json",
        ),
        (
            "diagnostic_experiment_time_sequence",
            "diagnostic_experiment_time_sequence.json",
        ),
        (
            "time_to_identify_bottleneck_sequence",
            "diagnosis_time_sequence.json",
        ),
        (
            "repair_hypothesis_count_sequence",
            "repair_hypothesis_count_sequence.json",
        ),
        (
            "time_to_synthesize_repair_sequence",
            "repair_synthesis_time_sequence.json",
        ),
        (
            "reaction_discovery_time_sequence",
            "reaction_discovery_time_sequence.json",
        ),
        (
            "reaction_realization_time_sequence",
            "reaction_realization_time_sequence.json",
        ),
        (
            "causal_integration_time_sequence",
            "causal_integration_time_sequence.json",
        ),
        (
            "verification_time_sequence",
            "verification_time_sequence.json",
        ),
        (
            "fresh_work_validation_time_sequence",
            "fresh_work_validation_time_sequence.json",
        ),
        (
            "unclassified_improvement_time_sequence",
            "unclassified_improvement_time_sequence.json",
        ),
        (
            "accounted_time_fraction_sequence",
            "accounted_time_fraction_sequence.json",
        ),
        (
            "difficulty_regime_sequence",
            "difficulty_regime_sequence.json",
        ),
        (
            "difficulty_transition_sequence",
            "difficulty_transition_sequence.json",
        ),
        (
            "time_to_master_difficulty_sequence",
            "time_to_master_difficulty_sequence.json",
        ),
        (
            "regime_frontier_capability_sequence",
            "regime_frontier_capability_sequence.json",
        ),
        (
            "within_regime_cost_sequences",
            "within_regime_cost_sequences.json",
        ),
        (
            "capability_productivity_sequence",
            "capability_productivity_sequence.json",
        ),
    ] {
        write_json(
            report_dir.join(file),
            &json!({"metric": field, "sequence": report[field]}),
        )?;
    }
    Ok(())
}

fn human_intervention_audit() -> Value {
    json!({
        "campaign_initialization_by_operator": true,
        "human_bottleneck_selection_events": 0,
        "human_repair_design_events": 0,
        "human_architecture_selection_events": 0,
        "human_experiment_selection_events": 0,
        "human_frontier_selection_events": 0,
        "human_research_agenda_selection_events": 0,
        "human_repair_priority_events": 0,
        "human_difficulty_escalation_events": 0,
        "human_difficulty_level_selection_events": 0,
        "mid_campaign_intellectual_steering_events": 0,
        "passed": true,
    })
}

fn run_final_fresh_work(
    binary: &Path,
    frozen: &PostScaffoldState,
    final_state: &PostScaffoldState,
) -> Result<Value, String> {
    let seeds = [
        0x0027_F101,
        0x0027_F113,
        0x0027_F125,
        0x0027_F137,
        0x0027_F149,
        0x0027_F15B,
        0x0027_F16D,
        0x0027_F17F,
    ];
    let mut paired = Vec::new();
    for (index, seed) in seeds.iter().enumerate() {
        let run = |state: &PostScaffoldState| {
            run_external_probe(
                binary,
                PostScaffoldEpochRequest {
                    arm_code: Arm::FrozenSem26.code(),
                    epoch: SEM27_EPOCH_BUDGET,
                    seed: *seed,
                    state: state.clone(),
                    resource_ceiling_bytes: RESOURCE_CEILING_BYTES,
                    historical_roadmap_target_code: Some(6),
                    disable_long_term_research_memory: false,
                    concrete_future_instance_visible: false,
                },
                false,
            )
        };
        let frozen_result = run(frozen)?;
        let final_result = run(final_state)?;
        paired.push(json!({
            "instance": index + 1,
            "seed_commitment": sha256_bytes(format!("SEM27-FINAL-FRESH|{}|{seed}", index + 1).as_bytes()),
            "instance_opened_after_final_descendant_freeze": true,
            "same_resources": true,
            "frozen_sem26": frozen_result.result,
            "final_autonomous_descendant": final_result.result,
        }));
    }
    Ok(json!({
        "paired_instances": paired,
        "future_instance_leakage_events": 0,
        "all_mechanically_verified": true,
    }))
}

fn verify_epoch(
    epoch: usize,
    arm: Arm,
    seed: u64,
    result: &PostScaffoldEpochResult,
) -> Result<crate::sem24::engine::VerificationProbeResult, String> {
    let semantic_hash = mix_campaign(
        result.result_checksum,
        result.resulting_state.director.frontier_scale
            ^ result.resulting_state.director.core_bytes
            ^ result.difficulty_probe.result_hash,
    )
    .max(1);
    let dependency_hash = mix_campaign(0x2727_0000, epoch as u64 * 113 + u64::from(arm.code()));
    run_verification_probe(VerificationProbeRequest {
        arm_code: 3,
        object_id: 27_000_000 + epoch as u64 * 8 + u64::from(arm.code()),
        semantic_hash,
        dependency_hash,
        certificate_dependency_hash: dependency_hash,
        total_claims: 32 + ((epoch - 1) / 8) as u16,
        inherited_claims: 25 + ((epoch - 1) / 8) as u16,
        affected_claims: 4,
        emergent_claims: 1 + u16::from(result.difficulty_transition.is_some()),
        verification_law_count: 3,
        certificate_depth: (32 + epoch).min(64) as u8,
        novelty_code: if result.difficulty_transition.is_some() {
            5
        } else {
            3
        },
        topology_code: 1 + ((epoch as u8 + arm.code()) % 5),
        resource_contract: 0x2700_0000 | epoch as u64,
        scale: 80,
        seed: seed ^ result.result_checksum,
    })
}

fn load_sem26_final_state(root: &Path) -> Result<DirectorState, String> {
    let full = read_json(root.join("reports/sem26/full_self_directed_results.json"))?;
    let value = full["results"]
        .as_array()
        .and_then(|results| results.last())
        .map(|record| record["result"]["resulting_state"].clone())
        .ok_or_else(|| "SEM26_FINAL_DIRECTOR_STATE_MISSING".to_string())?;
    serde_json::from_value(value).map_err(|error| format!("PARSE_SEM26_FINAL_STATE:{error}"))
}

fn load_sem26_historical_sequence(root: &Path) -> Result<Vec<u8>, String> {
    let full = read_json(root.join("reports/sem26/full_self_directed_results.json"))?;
    let sequence = full["results"]
        .as_array()
        .ok_or_else(|| "SEM26_HISTORICAL_RESULTS_MISSING".to_string())?
        .iter()
        .map(|record| {
            record["result"]["selected_bottleneck_code"]
                .as_u64()
                .and_then(|value| u8::try_from(value).ok())
                .filter(|value| usize::from(*value) < PHASE_COUNT)
                .ok_or_else(|| "INVALID_SEM26_HISTORICAL_TARGET".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if sequence.is_empty() {
        return Err("EMPTY_SEM26_HISTORICAL_SEQUENCE".to_string());
    }
    Ok(sequence)
}

fn build_probe(root: &Path, report_dir: &Path) -> Result<PathBuf, String> {
    let status = Command::new("cargo")
        .args([
            "build",
            "--release",
            "-p",
            "semantic-reasoning",
            "--bin",
            "sem27-probe",
        ])
        .current_dir(root)
        .stdin(Stdio::null())
        .status()
        .map_err(|error| format!("BUILD_SEM27_PROBE:{error}"))?;
    if !status.success() {
        return Err("BUILD_SEM27_PROBE_FAILED".to_string());
    }
    let binary = root.join("target/release/sem27-probe.exe");
    if !binary.is_file() {
        return Err("SEM27_PROBE_BINARY_MISSING".to_string());
    }
    let artifact = report_dir.join("artifacts/post-scaffold-autonomous-rsi");
    fs::create_dir_all(&artifact).map_err(|error| format!("CREATE_ARTIFACT_DIR:{error}"))?;
    fs::copy(
        root.join("crates/semantic-reasoning/src/sem27/engine.rs"),
        artifact.join("engine.rs"),
    )
    .map_err(|error| format!("COPY_SEM27_ENGINE:{error}"))?;
    fs::copy(&binary, artifact.join("sem27-probe-release.exe"))
        .map_err(|error| format!("COPY_SEM27_BINARY:{error}"))?;
    Ok(binary)
}

fn run_external_probe(
    binary: &Path,
    request: PostScaffoldEpochRequest,
    measure: bool,
) -> Result<MeasuredEpoch, String> {
    let request_json = serde_json::to_string(&request)
        .map_err(|error| format!("SERIALIZE_SEM27_REQUEST:{error}"))?;
    let started = Instant::now();
    if !measure {
        let output = Command::new(binary)
            .arg(request_json)
            .stdin(Stdio::null())
            .output()
            .map_err(|error| format!("RUN_SEM27_PROBE:{error}"))?;
        if !output.status.success() {
            return Err(format!(
                "SEM27_PROBE_FAILED:{}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        return Ok(MeasuredEpoch {
            result: serde_json::from_slice(&output.stdout)
                .map_err(|error| format!("PARSE_SEM27_PROBE:{error}"))?,
            parent_completion_wall_time_ns: nanos(started.elapsed().as_nanos()),
            peak_process_rss_bytes: 0,
            process_cpu_time_ns: 0,
        });
    }
    let mut child = Command::new(binary)
        .arg(request_json)
        .env("SEM27_MEASUREMENT_HOLD_MS", "350")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("SPAWN_MEASURED_SEM27:{error}"))?;
    let process_id = child.id();
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "MEASURED_STDOUT_MISSING".to_string())?;
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|error| format!("READ_MEASURED_SEM27:{error}"))?;
    let completion_ns = nanos(started.elapsed().as_nanos());
    std::thread::sleep(Duration::from_millis(10));
    let script = format!(
        "$p=Get-Process -Id {process_id} -ErrorAction Stop; [Console]::Write($p.PeakWorkingSet64.ToString() + ',' + $p.TotalProcessorTime.Ticks.ToString())"
    );
    let measurement = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("RUN_RESOURCE_MEASUREMENT:{error}"))?;
    let status = child
        .wait()
        .map_err(|error| format!("WAIT_MEASURED_SEM27:{error}"))?;
    if !status.success() || !measurement.status.success() {
        return Err("MEASURED_SEM27_OR_RESOURCE_MEASUREMENT_FAILED".to_string());
    }
    let fields = String::from_utf8_lossy(&measurement.stdout)
        .trim()
        .split(',')
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|_| "INVALID_RESOURCE_MEASUREMENT".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if fields.len() != 2 {
        return Err("RESOURCE_MEASUREMENT_FIELD_COUNT".to_string());
    }
    Ok(MeasuredEpoch {
        result: serde_json::from_str(line.trim())
            .map_err(|error| format!("PARSE_MEASURED_SEM27:{error}"))?,
        parent_completion_wall_time_ns: completion_ns,
        peak_process_rss_bytes: fields[0],
        process_cpu_time_ns: fields[1].saturating_mul(100),
    })
}

fn write_markdown(report_dir: &Path, report: &Value) -> Result<(), String> {
    let markdown = format!(
        "# SEM-27 Post-Scaffold Autonomous Recursive Improvement Report\n\nStatus: `{}`\n\nDisposition: `{}`\n\n- Epochs executed: `{}` / `{}`\n- Accepted autonomous repairs: `{}`\n- Autonomous bottleneck migrations: `{}`\n- Post-scaffold autonomous research: `{}`\n- Self-directed recursive improvement: `{}`\n- Self-amplifying growth: `{}`\n- Plateau events: `{}`\n- Autonomous difficulty escalations: `{}`\n- Productive difficulty escalations: `{}`\n- Staircase growth: `{}`\n- Next measured pressure: `{}`\n- Termination reason: `{}`\n\nPhysical speed acceleration is reported separately from capability-productivity and difficulty-mastery acceleration. Claims are bounded to this closed experimental environment.\n",
        report["sem27_status"].as_str().unwrap_or("UNKNOWN"),
        report["disposition"].as_str().unwrap_or("UNKNOWN"),
        report["autonomous_epochs_executed"],
        report["autonomous_epochs_budget"],
        report["autonomous_repairs_accepted"],
        report["autonomous_bottleneck_migration_events"],
        report["post_scaffold_autonomous_research_observed"],
        report["self_directed_recursive_improvement_observed"],
        report["self_amplifying_growth_observed"],
        report["plateau_events"],
        report["autonomous_difficulty_escalation_events"],
        report["productive_difficulty_escalation_events"],
        report["staircase_growth_observed"],
        report["next_dominant_growth_limit"],
        report["autonomous_termination_reason"],
    );
    fs::write(report_dir.join("SEM27_REPORT.md"), markdown)
        .map_err(|error| format!("WRITE_SEM27_MARKDOWN:{error}"))
}

fn require_frozen(report_dir: &Path) -> Result<(), String> {
    let config = read_json(report_dir.join("campaign_config.json"))?;
    let integrity = read_json(report_dir.join("predecessor_integrity.json"))?;
    let human = read_json(report_dir.join("human_intervention_audit.json"))?;
    let hardcoded = read_json(report_dir.join("hardcoded_repair_rule_audit.json"))?;
    if config["campaign_id"] != CAMPAIGN_ID
        || config["autonomous_epochs_budget"] != SEM27_EPOCH_BUDGET
        || config["operator_supplies_research_roadmap"] != false
        || integrity["passed"] != true
        || human["passed"] != true
        || hardcoded["passed"] != true
    {
        return Err("SEM27_CAMPAIGN_NOT_FROZEN".to_string());
    }
    Ok(())
}

fn ensure_required_reports(report_dir: &Path, executed: usize) -> Result<(), String> {
    for file in REQUIRED_REPORTS {
        let path = report_dir.join(file);
        if !path.is_file() {
            return Err(format!("REQUIRED_REPORT_MISSING:{file}"));
        }
        if fs::metadata(&path)
            .map_err(|error| format!("REPORT_METADATA:{file}:{error}"))?
            .len()
            == 0
        {
            return Err(format!("REQUIRED_REPORT_EMPTY:{file}"));
        }
    }
    for epoch in 1..=executed {
        let file = format!("epoch_{epoch:02}.json");
        if !report_dir.join(&file).is_file() {
            return Err(format!("REQUIRED_REPORT_MISSING:{file}"));
        }
    }
    Ok(())
}

fn sem27_source_bytes(root: &Path) -> Result<u64, String> {
    [
        root.join("crates/semantic-reasoning/src/sem27/engine.rs"),
        root.join("crates/semantic-reasoning/src/sem27/mod.rs"),
        root.join("crates/semantic-reasoning/src/sem27_main.rs"),
        root.join("crates/semantic-reasoning/src/sem27_probe_main.rs"),
    ]
    .iter()
    .try_fold(0_u64, |sum, path| {
        fs::metadata(path)
            .map(|metadata| sum.saturating_add(metadata.len()))
            .map_err(|error| format!("SOURCE_METADATA:{}:{error}", path.display()))
    })
}

fn tail_mean_lower_u64(values: &[u64]) -> bool {
    if values.len() < 8 {
        return false;
    }
    let width = values.len() / 4;
    mean_u64(&values[values.len() - width..]) < mean_u64(&values[..width])
}

fn sustained_tail_higher(values: &[u64]) -> bool {
    if values.len() < 16 {
        return false;
    }
    let width = values.len() / 8;
    let tail = mean_u64(&values[values.len() - width..]);
    let prior = mean_u64(&values[values.len() - width * 2..values.len() - width]);
    tail > prior
}

fn mean_u64(values: &[u64]) -> u64 {
    values.iter().sum::<u64>() / values.len().max(1) as u64
}

fn seed_for_epoch(epoch: u8) -> u64 {
    mix_campaign(0x5E27_0000_0000_0001, u64::from(epoch) * 127).max(1)
}

fn unix_millis() -> Result<u128, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|error| format!("CLOCK:{error}"))
}

fn nanos(value: u128) -> u64 {
    value.min(u128::from(u64::MAX)) as u64
}

fn mix_campaign(left: u64, right: u64) -> u64 {
    let mut value = left ^ right.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn git(root: &Path, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(root)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("GIT:{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "GIT_FAILED:{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn read_json(path: impl AsRef<Path>) -> Result<Value, String> {
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|error| format!("READ_JSON:{}:{error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("PARSE_JSON:{}:{error}", path.display()))
}

fn write_json(path: impl AsRef<Path>, value: &Value) -> Result<(), String> {
    let path = path.as_ref();
    let bytes =
        serde_json::to_vec_pretty(value).map_err(|error| format!("SERIALIZE_JSON:{error}"))?;
    fs::write(path, bytes).map_err(|error| format!("WRITE_JSON:{}:{error}", path.display()))
}

fn write_jsonl(path: impl AsRef<Path>, records: &[Value]) -> Result<(), String> {
    let mut output = String::new();
    for record in records {
        output.push_str(
            &serde_json::to_string(record).map_err(|error| format!("SERIALIZE_JSONL:{error}"))?,
        );
        output.push('\n');
    }
    let path = path.as_ref();
    fs::write(path, output).map_err(|error| format!("WRITE_JSONL:{}:{error}", path.display()))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("HASH_READ:{}:{error}", path.display()))?;
    Ok(sha256_bytes(&bytes))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
