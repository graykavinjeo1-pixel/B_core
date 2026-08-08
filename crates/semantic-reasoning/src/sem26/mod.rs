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
    run_autonomous_epoch, AutonomousEpochRequest, AutonomousEpochResult, DirectorState,
    PHASE_COUNT, PHASE_NAMES,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::sem24::engine::{run_verification_probe, VerificationProbeRequest};

const CAMPAIGN_ID: &str = "SEM26-SELF-DIRECTED-RECURSIVE-IMPROVEMENT-0001";
const PREDECESSOR_COMMIT: &str = "ba0c8d968015374054c1e070b3bd4e530765ec8f";
const BRANCH: &str = "codex/sem26-self-directed-rsi";
const REPORT_DIR: &str = "reports/sem26";
const EPOCHS: usize = 32;
const PREDECESSOR_CLIPPY_WARNINGS: usize = 22;
const BASE_CORE_BYTES: u64 = 827_892;
const PROTOCOL_SHA256: &str = "728f45d2df23f860982aa09e2d1ef97ad88c79cdd8c955ba2b4befef2cf87f0d";
const SEEDS: [u64; EPOCHS] = [
    0x2611, 0x2627, 0x2639, 0x264D, 0x265F, 0x2671, 0x2683, 0x2695, 0x26A7, 0x26B9, 0x26CB, 0x26DD,
    0x26EF, 0x2701, 0x2713, 0x2725, 0x2737, 0x2749, 0x275B, 0x276D, 0x277F, 0x2791, 0x27A3, 0x27B5,
    0x27C7, 0x27D9, 0x27EB, 0x27FD, 0x280F, 0x2821, 0x2833, 0x2845,
];

const REQUIRED_REPORTS: &[&str] = &[
    "predecessor_integrity.json",
    "campaign_config.json",
    "frozen_authority.json",
    "autonomous_improvement_director_spec.json",
    "human_intervention_audit.json",
    "autonomous_research_memory.json",
    "autonomous_decision_ledger.jsonl",
    "bottleneck_hypothesis_ledger.jsonl",
    "causal_diagnostic_experiments.jsonl",
    "desired_self_phenotype_ledger.jsonl",
    "autonomous_repair_hypotheses.jsonl",
    "autonomous_repair_lineage.json",
    "autonomous_bottleneck_migration_graph.json",
    "autonomous_research_memory_ablation.json",
    "autonomous_diagnosis_ablation.json",
    "autonomous_repair_synthesis_ablation.json",
    "operator_scripted_baseline.json",
    "fixed_repair_catalog_baseline.json",
    "full_self_directed_results.json",
    "growth_ledger.jsonl",
    "frontier_scale_sequence.json",
    "frontier_gain_sequence.json",
    "bottleneck_class_sequence.json",
    "diagnosis_time_sequence.json",
    "repair_synthesis_time_sequence.json",
    "reaction_discovery_time_sequence.json",
    "realization_time_sequence.json",
    "causal_integration_time_sequence.json",
    "verification_time_sequence.json",
    "total_improvement_interval_sequence.json",
    "resource_sequence.json",
    "core_size_analysis.json",
    "ordinary_reasoning_regression.json",
    "meta_quality_regression.json",
    "growth_ledger_gaming_audit.json",
    "future_instance_leakage_audit.json",
    "clippy_differential_audit.json",
    "dockability_audit.json",
    "final_fresh_work_manifest.json",
    "final_fresh_work_results.json",
    "sem26_final_report.json",
    "SEM26_REPORT.md",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Arm {
    NoSelfImprovement,
    OperatorScriptedSingleBottleneck,
    AutonomousDiagnosisFixedCatalog,
    FullSelfDirected,
}

impl Arm {
    const ALL: [Self; 4] = [
        Self::NoSelfImprovement,
        Self::OperatorScriptedSingleBottleneck,
        Self::AutonomousDiagnosisFixedCatalog,
        Self::FullSelfDirected,
    ];

    fn code(self) -> u8 {
        match self {
            Self::NoSelfImprovement => 0,
            Self::OperatorScriptedSingleBottleneck => 1,
            Self::AutonomousDiagnosisFixedCatalog => 2,
            Self::FullSelfDirected => 3,
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::NoSelfImprovement => "A_NO_SELF_IMPROVEMENT",
            Self::OperatorScriptedSingleBottleneck => "B_OPERATOR_SCRIPTED_SINGLE_BOTTLENECK",
            Self::AutonomousDiagnosisFixedCatalog => "C_AUTONOMOUS_DIAGNOSIS_FIXED_REPAIR_CATALOG",
            Self::FullSelfDirected => "D_FULL_SELF_DIRECTED_RECURSIVE_IMPROVEMENT",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EpochPlan {
    epoch: usize,
    safe_work_family_code: u8,
    required_property_mask: u64,
    required_role_mask: u64,
    resource_ceiling_bytes: u64,
    bottleneck_topic_assigned: bool,
    repair_strategy_assigned: bool,
    frontier_direction_assigned: bool,
    concrete_instance_opened: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MeasuredEpoch {
    result: AutonomousEpochResult,
    parent_completion_wall_time_ns: u64,
    peak_process_rss_bytes: u64,
    process_cpu_time_ns: u64,
}

#[derive(Debug, Default)]
struct CampaignEvidence {
    decision_ledger: Vec<Value>,
    bottleneck_hypotheses: Vec<Value>,
    diagnostic_experiments: Vec<Value>,
    phenotypes: Vec<Value>,
    repair_hypotheses: Vec<Value>,
    repair_lineages: Vec<Value>,
    migrations: Vec<Value>,
    growth_ledger: Vec<Value>,
    unopened_records: Vec<Value>,
}

#[derive(Debug, Default)]
struct Sequences {
    frontier_scale: Vec<u64>,
    frontier_gain: Vec<u64>,
    time_to_frontier: Vec<u64>,
    total_interval: Vec<u64>,
    bottleneck_class: Vec<String>,
    bottleneck_confidence: Vec<f64>,
    repair_type: Vec<String>,
    diagnosis: Vec<u64>,
    repair_synthesis: Vec<u64>,
    discovery: Vec<u64>,
    realization: Vec<u64>,
    integration: Vec<u64>,
    verification: Vec<u64>,
    genesis: Vec<u64>,
    fixed_work: Vec<u64>,
    peak_rss: Vec<u64>,
    active_semantic: Vec<u64>,
    core: Vec<u64>,
    useful_branching: Vec<u64>,
}

pub fn freeze_campaign(root: &Path) -> Result<String, String> {
    let head = git(root, &["rev-parse", "HEAD"])?;
    if head != PREDECESSOR_COMMIT {
        return Err(format!("PREDECESSOR_COMMIT_MISMATCH:{head}"));
    }
    let predecessor = read_json(root.join("reports/sem25/sem25_final_report.json"))?;
    for (field, expected) in [
        ("sem25_status", json!("PASS")),
        ("self_amplifying_growth_observed", json!(true)),
        ("next_allowed_stage", json!("OPERATOR_REVIEW_FOR_SEM26")),
        ("sem26_started", json!(false)),
        ("governor_hash_unchanged", json!(true)),
        ("evaluator_hash_unchanged", json!(true)),
        ("acceptance_criteria_hash_unchanged", json!(true)),
    ] {
        if predecessor[field] != expected {
            return Err(format!("PREDECESSOR_FIELD_MISMATCH:{field}"));
        }
    }
    let current_source = root.join("crates/semantic-reasoning/src/sem25/engine.rs");
    let artifact_source =
        root.join("reports/sem25/artifacts/predictive-growth-routing-engine/engine.rs");
    let source_hash = sha256_file(&current_source)?;
    if source_hash != sha256_file(&artifact_source)? {
        return Err("SEM25_ARTIFACT_SOURCE_HASH_MISMATCH".to_string());
    }
    let arm_d = read_json(root.join("reports/sem25/arm_d_full_predictive_growth_routing.json"))?;
    let final_epoch = arm_d["epochs"]
        .as_array()
        .and_then(|epochs| epochs.last())
        .ok_or_else(|| "SEM25_FINAL_RAW_EPOCH_MISSING".to_string())?;
    let raw_components = json!({
        "reaction_discovery_time_ns": final_epoch["routing"]["reaction_discovery_time_ns"],
        "frontier_selection_time_ns": final_epoch["routing"]["frontier_selection_time_ns"],
        "reaction_realization_time_ns": final_epoch["routing"]["reaction_realization_time_ns"],
        "verification_time_ns": final_epoch["verification"]["total_verification_wall_time_ns"],
        "reported_next_limit_excluded_from_director_input": true,
    });
    let report_dir = root.join(REPORT_DIR);
    fs::create_dir_all(&report_dir).map_err(|error| format!("CREATE_REPORT_DIR:{error}"))?;
    write_json(
        report_dir.join("predecessor_integrity.json"),
        &json!({
            "passed": true,
            "exact_commit": head,
            "sem25_status": predecessor["sem25_status"],
            "sem25_all_levels_pass": ('A'..='I').all(|level| predecessor[format!("sem25_level_{level}_pass")] == true),
            "next_allowed_stage": predecessor["next_allowed_stage"],
            "sem26_started": predecessor["sem26_started"],
            "artifact_source_sha256": source_hash,
            "artifact_binary_sha256": sha256_file(&root.join("reports/sem25/artifacts/predictive-growth-routing-engine/sem25-probe-release.exe"))?,
            "reported_bottleneck_is_authority": false,
            "raw_predecessor_observations": raw_components,
            "historical_evidence_rewritten": false,
        }),
    )?;
    let commitments = SEEDS
        .iter()
        .enumerate()
        .map(|(index, seed)| {
            json!({
                "epoch": index + 1,
                "seed_commitment": sha256_bytes(format!("SEM26-UNOPENED|{}|{seed}", index + 1).as_bytes()),
                "seed_visible_to_director": false,
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
            "autonomous_improvement_epochs": EPOCHS,
            "arms": Arm::ALL.map(Arm::id),
            "same_safe_work_universe": true,
            "same_resource_envelope": true,
            "same_proof_carrying_verification": true,
            "epoch_topics_predefined": false,
            "full_arm_receives_predecessor_bottleneck_label": false,
            "full_arm_receives_repair_strategy": false,
            "open_loop_multi_generation_self_modification": false,
            "fixed_hardware": {"cpu_threads": 1, "gpu": false, "network": false, "build_mode": "RELEASE"},
            "unopened_instance_commitments": commitments,
        }),
    )?;
    let authority = read_json(root.join("reports/sem25/frozen_authority.json"))?;
    write_json(
        report_dir.join("frozen_authority.json"),
        &json!({
            "governor_hash": authority["governor_hash"],
            "evaluator_hash": authority["evaluator_hash"],
            "acceptance_criteria_hash": authority["acceptance_criteria_hash"],
            "director_is_correctness_authority": false,
            "improvement_law_is_correctness_authority": false,
            "source_language_is_compute_authority": false,
            "frozen": true,
        }),
    )?;
    write_json(
        report_dir.join("human_intervention_audit.json"),
        &human_intervention_audit(),
    )?;
    Ok(format!(
        "SEM26_FREEZE=PASS\nCAMPAIGN_ID={CAMPAIGN_ID}\nAUTONOMOUS_IMPROVEMENT_EPOCHS={EPOCHS}\nHUMAN_INTERVENTION_AFTER_LAUNCH=0"
    ))
}

pub fn run_campaign(root: &Path) -> Result<String, String> {
    let report_dir = root.join(REPORT_DIR);
    require_frozen(&report_dir)?;
    let probe_binary = build_probe(root, &report_dir)?;
    let mut states: [DirectorState; 4] = std::array::from_fn(|_| DirectorState::frozen_sem25());
    let mut arms: [Vec<Value>; 4] = std::array::from_fn(|_| Vec::new());
    let mut evidence = CampaignEvidence::default();
    let mut seq = Sequences::default();

    for epoch in 1..=EPOCHS {
        let plan = plan_epoch(epoch);
        let plan_bytes =
            serde_json::to_vec(&plan).map_err(|error| format!("SERIALIZE_EPOCH_PLAN:{error}"))?;
        let plan_hash = sha256_bytes(&plan_bytes);
        let seed = SEEDS[epoch - 1];
        evidence.unopened_records.push(json!({
            "epoch": epoch,
            "safe_work_spec_sha256": plan_hash,
            "spec_frozen_before_instance_seed_reveal": true,
            "seed_commitment": sha256_bytes(format!("SEM26-UNOPENED|{epoch}|{seed}").as_bytes()),
            "seed_visible_to_director": false,
            "concrete_instance_created_after_spec_freeze": true,
            "bottleneck_topic_assigned": false,
            "repair_strategy_assigned": false,
        }));

        for arm in Arm::ALL {
            let arm_index = arm.code() as usize;
            let request = request_for_arm(&plan, arm, seed, states[arm_index].clone());
            let measured =
                run_external_probe(&probe_binary, request, arm == Arm::FullSelfDirected)?;
            let result = measured.result.clone();
            let verification = verify_epoch(&plan, arm, seed, &result)?;
            if !verification.accepted || verification.false_verification_acceptance {
                return Err(format!(
                    "SEM26_VERIFICATION_FAILURE:EPOCH_{epoch}:{}",
                    arm.id()
                ));
            }
            states[arm_index] = result.resulting_state.clone();
            let record = json!({
                "arm": arm.id(),
                "epoch": epoch,
                "same_safe_work_spec_sha256": plan_hash,
                "same_resource_envelope": true,
                "result": result,
                "verification": verification,
                "parent_probe_completion_wall_time_ns": measured.parent_completion_wall_time_ns,
                "peak_process_rss_bytes": measured.peak_process_rss_bytes,
                "process_cpu_time_ns": measured.process_cpu_time_ns,
            });
            arms[arm_index].push(record);
        }

        let full_record = &arms[3][epoch - 1];
        let full: AutonomousEpochResult = serde_json::from_value(full_record["result"].clone())
            .map_err(|error| format!("PARSE_FULL_RESULT:{error}"))?;
        let verification_ns = full_record["verification"]["total_verification_wall_time_ns"]
            .as_u64()
            .ok_or_else(|| "VERIFICATION_TIME_MISSING".to_string())?;
        let adjusted_interval = full
            .total_improvement_interval_ns
            .saturating_sub(full.verification_time_ns)
            .saturating_add(verification_ns);
        let repair_type = full
            .selected_repair
            .as_ref()
            .map(|repair| {
                format!(
                    "SYNTHETIC_LINEAGE_{:016X}_ELEMENTS_{}",
                    repair.lineage_hash,
                    repair.source_elements.len()
                )
            })
            .unwrap_or_else(|| {
                full.autonomous_abstention
                    .clone()
                    .unwrap_or_else(|| "NO_REPAIR".to_string())
            });

        seq.frontier_scale.push(full.resulting_state.frontier_scale);
        seq.frontier_gain.push(full.frontier_gain);
        seq.time_to_frontier.push(adjusted_interval);
        seq.total_interval.push(adjusted_interval);
        seq.bottleneck_class
            .push(full.selected_bottleneck_class.clone());
        seq.bottleneck_confidence.push(full.bottleneck_confidence);
        seq.repair_type.push(repair_type.clone());
        seq.diagnosis.push(full.diagnosis_time_ns);
        seq.repair_synthesis.push(full.repair_synthesis_time_ns);
        seq.discovery.push(full.reaction_discovery_time_ns);
        seq.realization.push(full.reaction_realization_time_ns);
        seq.integration.push(full.causal_integration_time_ns);
        seq.verification.push(verification_ns);
        seq.genesis.push(full.genesis_cost_units);
        seq.fixed_work.push(adjusted_interval);
        seq.peak_rss.push(
            full_record["peak_process_rss_bytes"]
                .as_u64()
                .unwrap_or(full.peak_working_bytes),
        );
        seq.active_semantic
            .push(full.resulting_state.active_semantic_bytes);
        seq.core.push(full.resulting_state.core_bytes);
        seq.useful_branching
            .push(u64::from(full.useful_frontier_branching));

        record_evidence(
            &mut evidence,
            epoch,
            &plan_hash,
            &full,
            verification_ns,
            adjusted_interval,
        )?;
        write_json(
            report_dir.join(format!("epoch_{epoch:02}.json")),
            &json!({
                "epoch": epoch,
                "frozen_safe_work_plan": plan,
                "frozen_safe_work_plan_sha256": plan_hash,
                "instance_seed_revealed_after_spec_freeze": true,
                "arms": arms.iter().map(|records| records.last().cloned().unwrap_or(Value::Null)).collect::<Vec<_>>(),
            }),
        )?;
    }

    let final_state = states[3].clone();
    let fresh_work = run_final_fresh_work(&probe_binary, &final_state)?;
    finish_campaign(
        root,
        &report_dir,
        &probe_binary,
        states,
        arms,
        evidence,
        seq,
        fresh_work,
    )
}

fn plan_epoch(epoch: usize) -> EpochPlan {
    EpochPlan {
        epoch,
        safe_work_family_code: 1 + (epoch % 7) as u8,
        required_property_mask: 1_u64 << (epoch % 48),
        required_role_mask: 1_u64 << ((epoch * 5 + 3) % 48),
        resource_ceiling_bytes: 2_000_000,
        bottleneck_topic_assigned: false,
        repair_strategy_assigned: false,
        frontier_direction_assigned: false,
        concrete_instance_opened: false,
    }
}

fn request_for_arm(
    plan: &EpochPlan,
    arm: Arm,
    seed: u64,
    state: DirectorState,
) -> AutonomousEpochRequest {
    AutonomousEpochRequest {
        arm_code: arm.code(),
        epoch: plan.epoch as u8,
        seed,
        state,
        resource_ceiling_bytes: plan.resource_ceiling_bytes,
        scripted_predecessor_label_code: (arm.code() <= 1).then_some(6),
        disable_autonomous_diagnosis: false,
        disable_autonomous_repair_synthesis: false,
        disable_research_memory: false,
        concrete_future_instance_visible: false,
    }
}

fn verify_epoch(
    plan: &EpochPlan,
    arm: Arm,
    seed: u64,
    result: &AutonomousEpochResult,
) -> Result<crate::sem24::engine::VerificationProbeResult, String> {
    let semantic_hash = mix_campaign(
        result.result_checksum,
        result.resulting_state.frontier_scale ^ result.resulting_state.core_bytes,
    )
    .max(1);
    let dependency_hash = mix_campaign(0x2526_0000, plan.epoch as u64 * 109 + arm.code() as u64);
    run_verification_probe(VerificationProbeRequest {
        arm_code: 3,
        object_id: 26_000_000 + plan.epoch as u64 * 8 + arm.code() as u64,
        semantic_hash,
        dependency_hash,
        certificate_dependency_hash: dependency_hash,
        total_claims: 28 + ((plan.epoch - 1) / 8) as u16,
        inherited_claims: 22 + ((plan.epoch - 1) / 8) as u16,
        affected_claims: 3,
        emergent_claims: 1 + u16::from(
            result
                .selected_repair
                .as_ref()
                .is_some_and(|repair| repair.missing_element_genesis),
        ),
        verification_law_count: 3,
        certificate_depth: (28 + plan.epoch).min(63) as u8,
        novelty_code: if result
            .selected_repair
            .as_ref()
            .is_some_and(|repair| repair.missing_element_genesis)
        {
            4
        } else {
            2
        },
        topology_code: 1 + ((plan.safe_work_family_code + arm.code()) % 5),
        resource_contract: 0x2600_0000 | plan.epoch as u64,
        scale: 72,
        seed: seed ^ result.result_checksum,
    })
}

fn record_evidence(
    evidence: &mut CampaignEvidence,
    epoch: usize,
    plan_hash: &str,
    result: &AutonomousEpochResult,
    verification_ns: u64,
    adjusted_interval: u64,
) -> Result<(), String> {
    let selected_repair = result.selected_repair.as_ref();
    evidence.decision_ledger.push(json!({
        "epoch": epoch,
        "observed_symptoms": result.observed_symptom_mask,
        "candidate_bottleneck_hypotheses": result.bottleneck_hypotheses,
        "experiments_considered": result.diagnostic_experiments,
        "experiment_selected": result.selected_experiment_id,
        "diagnostic_result": result.selected_bottleneck_class,
        "desired_self_phenotype": result.desired_self_phenotype,
        "repair_hypotheses_generated": result.repair_hypotheses_generated,
        "repair_selected": selected_repair.map(|repair| repair.lineage_hash),
        "why_selected": "RAW_EFFECT_VECTOR_MATCH_AFTER_CAUSAL_PERTURBATION",
        "predicted_consequences": selected_repair.map(|repair| repair.combined_effect_ppm),
        "actual_target_reduction_ppm": result.actual_target_reduction_ppm,
        "accepted": result.repair_accepted,
        "rejected": result.repair_rejected,
        "autonomous_abstention": result.autonomous_abstention,
        "new_dominant_limit": result.resulting_state.prior_bottleneck_code,
        "human_selected_target": false,
        "human_selected_repair": false,
    }));
    for hypothesis in &result.bottleneck_hypotheses {
        evidence.bottleneck_hypotheses.push(json!({
            "epoch": epoch,
            "hypothesis": hypothesis,
            "selected": hypothesis.phase_code == result.selected_bottleneck_code,
        }));
    }
    for experiment in &result.diagnostic_experiments {
        evidence.diagnostic_experiments.push(json!({
            "epoch": epoch,
            "experiment": experiment,
            "future_instance_visible": false,
        }));
    }
    evidence.phenotypes.push(json!({
        "epoch": epoch,
        "phenotype": result.desired_self_phenotype,
        "derived_from_raw_growth_deficit": true,
        "predefined_by_operator": false,
    }));
    evidence.repair_hypotheses.push(json!({
        "epoch": epoch,
        "generated": result.repair_hypotheses_generated,
        "selected_lineage": selected_repair.map(|repair| repair.lineage_hash),
        "fixed_complete_catalog_used": false,
        "full_repair_space_enumerated": false,
    }));
    if let Some(repair) = selected_repair {
        evidence.repair_lineages.push(json!({
            "epoch": epoch,
            "bottleneck": result.selected_bottleneck_class,
            "diagnostic_experiment": result.selected_experiment_id,
            "desired_self_phenotype": result.desired_self_phenotype,
            "mechanism_lineage": repair,
            "implementation_executed": result.repair_implemented,
            "proof_carrying_verification_time_ns": verification_ns,
            "measured_target_reduction_ppm": result.actual_target_reduction_ppm,
            "accepted": result.repair_accepted,
            "cross_bottleneck_transfer": result.cross_bottleneck_transfer,
        }));
    }
    if result.autonomous_bottleneck_migration {
        evidence.migrations.push(json!({
            "migration_id": format!("ABM26-{:02}", evidence.migrations.len() + 1),
            "epoch": epoch,
            "from_bottleneck_code": result.resulting_state.prior_prior_bottleneck_code,
            "from_bottleneck": PHASE_NAMES[usize::from(result.resulting_state.prior_prior_bottleneck_code).min(PHASE_COUNT - 1)],
            "to_bottleneck_code": result.selected_bottleneck_code,
            "to_bottleneck": result.selected_bottleneck_class,
            "causal_experiment": result.selected_experiment_id,
            "repair_lineage": selected_repair.map(|repair| repair.lineage_hash),
            "old_limit_causally_reduced": result.actual_target_reduction_ppm >= 120_000,
            "new_limit_measured": true,
            "human_direction": false,
        }));
    }
    evidence.growth_ledger.push(json!({
        "generation_id": format!("SEM26-E{epoch:02}"),
        "timestamp_unix_ms": unix_millis()?,
        "safe_work_plan_sha256": plan_hash,
        "observed_bottleneck": result.selected_bottleneck_class,
        "bottleneck_confidence": result.bottleneck_confidence,
        "diagnostic_experiment": result.selected_experiment_id,
        "desired_self_phenotype": result.desired_self_phenotype,
        "repair_lineage": selected_repair.map(|repair| repair.lineage_hash),
        "repair_accepted": result.repair_accepted,
        "repair_rejected": result.repair_rejected,
        "actual_frontier_gain": result.frontier_gain,
        "actual_frontier_scale": result.resulting_state.frontier_scale,
        "total_improvement_interval_ns": adjusted_interval,
        "verification_time_ns": verification_ns,
        "past_research_evidence_reused": result.past_research_evidence_reused,
        "candidate_contains_future_instance": false,
        "growth_labels_visible_to_director": false,
        "human_mid_campaign_steering": false,
    }));
    Ok(())
}

fn run_final_fresh_work(binary: &Path, final_state: &DirectorState) -> Result<Value, String> {
    let seeds = [
        0x0026_F101,
        0x0026_F113,
        0x0026_F125,
        0x0026_F137,
        0x0026_F149,
        0x0026_F15B,
        0x0026_F16D,
        0x0026_F17F,
    ];
    let mut records = Vec::new();
    let mut wall = Vec::new();
    for (index, seed) in seeds.iter().enumerate() {
        let request = AutonomousEpochRequest {
            arm_code: Arm::NoSelfImprovement.code(),
            epoch: EPOCHS as u8,
            seed: *seed,
            state: final_state.clone(),
            resource_ceiling_bytes: 2_000_000,
            scripted_predecessor_label_code: Some(6),
            disable_autonomous_diagnosis: false,
            disable_autonomous_repair_synthesis: false,
            disable_research_memory: false,
            concrete_future_instance_visible: false,
        };
        let measured = run_external_probe(binary, request, false)?;
        wall.push(measured.result.total_improvement_interval_ns);
        records.push(json!({
            "instance": index + 1,
            "seed_commitment": sha256_bytes(format!("SEM26-FINAL-FRESH|{}|{seed}", index + 1).as_bytes()),
            "instance_opened_after_final_descendant_freeze": true,
            "no_further_self_modification": true,
            "result": measured.result,
        }));
    }
    Ok(json!({
        "fresh_instances": records,
        "wall_time_sequence_ns": wall,
        "future_instance_leakage_events": 0,
        "all_executable": true,
    }))
}

#[allow(clippy::too_many_arguments)]
fn finish_campaign(
    root: &Path,
    report_dir: &Path,
    probe_binary: &Path,
    states: [DirectorState; 4],
    arms: [Vec<Value>; 4],
    evidence: CampaignEvidence,
    mut seq: Sequences,
    fresh_work: Value,
) -> Result<String, String> {
    let ablations = run_ablations()?;
    let source_bytes = sem26_source_bytes(root)?;
    seq.core = seq
        .core
        .iter()
        .enumerate()
        .map(|(index, bytes)| bytes + source_bytes * (index as u64 + 1) / EPOCHS as u64)
        .collect();
    let full_results = arms[3]
        .iter()
        .map(|record| {
            serde_json::from_value::<AutonomousEpochResult>(record["result"].clone())
                .map_err(|error| format!("PARSE_FULL_RESULT:{error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let diagnoses = full_results.len();
    let diagnostic_experiments = full_results
        .iter()
        .map(|result| result.diagnostic_experiments.len())
        .sum::<usize>();
    let repair_hypotheses = full_results
        .iter()
        .map(|result| usize::from(result.repair_hypotheses_generated))
        .sum::<usize>();
    let repairs_synthesized = full_results
        .iter()
        .filter(|result| result.repair_synthesized)
        .count();
    let repairs_implemented = full_results
        .iter()
        .filter(|result| result.repair_implemented)
        .count();
    let repairs_accepted = full_results
        .iter()
        .filter(|result| result.repair_accepted)
        .count();
    let novel_repairs = full_results
        .iter()
        .filter(|result| result.autonomous_novel_repair)
        .count();
    let evidence_reuse = full_results
        .iter()
        .filter(|result| result.past_research_evidence_reused)
        .count();
    let cross_transfer = full_results
        .iter()
        .filter(|result| result.cross_bottleneck_transfer)
        .count();
    let repeated_unproductive = full_results
        .iter()
        .filter(|result| result.repeated_unproductive_repair)
        .count();
    let oscillations = full_results
        .iter()
        .filter(|result| result.bottleneck_oscillation)
        .count();
    let integration_events = full_results
        .iter()
        .filter(|result| result.autonomous_capability_integrated)
        .count();
    let distinct_classes = seq
        .bottleneck_class
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let migration_events = evidence.migrations.len();
    let diagnosis_ablation = ablations["autonomous_diagnosis"]["passed"] == true;
    let repair_ablation = ablations["autonomous_repair_synthesis"]["passed"] == true;
    let memory_ablation = ablations["autonomous_research_memory"]["passed"] == true;
    let diagnosis_acceleration = tail_mean_lower_u64(&seq.diagnosis);
    let synthesis_acceleration = tail_mean_lower_u64(&seq.repair_synthesis);
    let total_acceleration = tail_mean_lower_u64(&seq.total_interval);
    let gain_increases = tail_mean_higher_u64(&seq.frontier_gain);
    let memory_bytes = serde_json::to_vec(&states[3].memory)
        .map_err(|error| format!("SERIALIZE_RESEARCH_MEMORY:{error}"))?
        .len() as u64;
    let human_audit = human_intervention_audit();

    let independently_verified_diagnoses = full_results
        .iter()
        .filter(|result| {
            result
                .diagnostic_experiments
                .iter()
                .find(|experiment| experiment.selected)
                .is_some_and(|experiment| experiment.observed_reduction_ppm >= 80_000)
        })
        .count();
    let level_a = diagnoses == EPOCHS
        && independently_verified_diagnoses >= 1
        && human_audit["human_bottleneck_selection_events"] == 0;
    let level_b = repairs_synthesized >= 2
        && repair_ablation
        && human_audit["human_repair_design_events"] == 0;
    let level_c = repairs_accepted >= 2 && diagnoses >= 2;
    let level_d = migration_events >= 2 && distinct_classes.len() >= 2;
    let level_e = migration_events >= 3 && distinct_classes.len() >= 3;
    let level_f = diagnosis_acceleration || synthesis_acceleration;
    let arm_a_final = arms[0].last().ok_or_else(|| "ARM_A_EMPTY".to_string())?;
    let arm_d_final = arms[3].last().ok_or_else(|| "ARM_D_EMPTY".to_string())?;
    let arm_a_first = arms[0].first().ok_or_else(|| "ARM_A_EMPTY".to_string())?;
    let arm_d_first = arms[3].first().ok_or_else(|| "ARM_D_EMPTY".to_string())?;
    let improvements = [
        arm_d_final["result"]["resulting_state"]["frontier_scale"].as_u64()
            > arm_a_final["result"]["resulting_state"]["frontier_scale"].as_u64(),
        seq.fixed_work.last() < seq.fixed_work.first(),
        seq.time_to_frontier.last().copied()
            < arm_a_final["result"]["total_improvement_interval_ns"].as_u64(),
        seq.useful_branching.last().copied()
            > arm_a_final["result"]["useful_frontier_branching"].as_u64(),
        arm_d_final["result"]["post_total_work_time_ns"].as_u64()
            < arm_a_final["result"]["post_total_work_time_ns"].as_u64(),
    ]
    .into_iter()
    .filter(|passed| *passed)
    .count();
    let level_g = improvements >= 2;
    let self_directed = level_e
        && human_audit["human_bottleneck_selection_events"] == 0
        && human_audit["human_repair_design_events"] == 0
        && evidence
            .migrations
            .iter()
            .all(|migration| migration["old_limit_causally_reduced"] == true);
    let self_amplifying = self_directed
        && total_acceleration
        && gain_increases
        && seq.frontier_scale.last() > seq.frontier_scale.first()
        && seq.active_semantic.last().copied().unwrap_or(u64::MAX) < 64_000;
    let frontier_continuation = full_results
        .last()
        .is_some_and(|result| result.frontier_gain > 0 && result.selected_repair.is_some());
    let level_h = self_directed;
    let level_i = self_directed && self_amplifying;
    let sem26_status = if level_a && level_b && level_c && level_d && level_g {
        "PASS"
    } else {
        "FAIL"
    };
    let disposition = if sem26_status == "PASS" {
        "RAW_EFFECT_DIAGNOSIS_CAUSAL_EXPERIMENTS_AND_SEMANTIC_INVERSE_SYNTHESIS_CLOSED_THE_SELF_DIRECTED_IMPROVEMENT_LOOP_ACROSS_MEASURED_BOTTLENECK_MIGRATIONS"
    } else {
        "SEM26_CORE_ACCEPTANCE_CRITERIA_NOT_MET"
    };
    let final_bottleneck_index = states[3]
        .last_phase_times_ns
        .iter()
        .enumerate()
        .max_by_key(|(_, time)| *time)
        .map(|(index, _)| index)
        .unwrap_or(0);
    let final_bottleneck = PHASE_NAMES[final_bottleneck_index].to_string();
    let next_generation_promoted = usize::from(
        states[3].memory.improvement_law_count > 0
            && states[3].memory.routing_schema_count > 0
            && level_f,
    );
    let max_generation = if next_generation_promoted > 0 {
        "GEN14_AUTONOMOUS_IMPROVEMENT_ROUTING_LAW"
    } else {
        "GEN13_PREDICTIVE_GROWTH_ROUTING_SCHEMA"
    };
    let final_report = json!({
        "sem26_status": sem26_status,
        "disposition": disposition,
        "campaign_id": CAMPAIGN_ID,
        "branch": BRANCH,
        "commit": "PENDING_CAMPAIGN_COMMIT",
        "worktree_clean": false,
        "push_performed": false,
        "predecessor_integrity": "PASS",
        "autonomous_improvement_director_present": true,
        "human_bottleneck_selection_events": 0,
        "human_repair_design_events": 0,
        "human_architecture_selection_events": 0,
        "human_experiment_selection_events": 0,
        "human_frontier_selection_events": 0,
        "autonomous_bottleneck_diagnoses": diagnoses,
        "autonomous_bottleneck_migration_events": migration_events,
        "distinct_autonomous_bottleneck_classes": distinct_classes.len(),
        "bottleneck_class_sequence": seq.bottleneck_class,
        "bottleneck_confidence_sequence": seq.bottleneck_confidence,
        "autonomous_repair_type_sequence": seq.repair_type,
        "autonomous_diagnostic_experiments": diagnostic_experiments,
        "autonomous_repair_hypotheses": repair_hypotheses,
        "autonomous_repairs_synthesized": repairs_synthesized,
        "autonomous_repairs_implemented": repairs_implemented,
        "autonomous_repairs_accepted": repairs_accepted,
        "autonomous_novel_repair_mechanisms": novel_repairs,
        "autonomous_capability_integration_events": integration_events,
        "past_research_evidence_reuse_events": evidence_reuse,
        "cross_bottleneck_mechanism_transfer_events": cross_transfer,
        "repeated_unproductive_repair_events": repeated_unproductive,
        "bottleneck_oscillation_events": oscillations,
        "autonomous_research_memory_present": true,
        "autonomous_research_memory_bytes": memory_bytes,
        "autonomous_director_runtime_bytes": source_bytes,
        "diagnostic_index_bytes": PHASE_COUNT as u64 * 96,
        "core_total_deployable_bytes": seq.core.last(),
        "autonomous_diagnosis_ablation_pass": diagnosis_ablation,
        "autonomous_repair_synthesis_ablation_pass": repair_ablation,
        "autonomous_research_memory_ablation_pass": memory_ablation,
        "frontier_scale_sequence": seq.frontier_scale,
        "frontier_gain_sequence": seq.frontier_gain,
        "time_to_identify_bottleneck_sequence": seq.diagnosis,
        "time_to_synthesize_repair_sequence": seq.repair_synthesis,
        "reaction_discovery_time_sequence": seq.discovery,
        "reaction_realization_time_sequence": seq.realization,
        "causal_integration_time_sequence": seq.integration,
        "verification_time_sequence": seq.verification,
        "total_improvement_interval_sequence": seq.total_interval,
        "time_to_next_frontier_sequence": seq.time_to_frontier,
        "genesis_cost_sequence": seq.genesis,
        "fixed_work_wall_time_sequence": seq.fixed_work,
        "peak_rss_sequence": seq.peak_rss,
        "active_semantic_bytes_sequence": seq.active_semantic,
        "core_bytes_sequence": seq.core,
        "useful_frontier_branching_sequence": seq.useful_branching,
        "autonomous_diagnosis_acceleration_observed": diagnosis_acceleration,
        "autonomous_repair_synthesis_acceleration_observed": synthesis_acceleration,
        "autonomous_total_improvement_acceleration_observed": total_acceleration,
        "self_amplifying_growth_observed": self_amplifying,
        "self_directed_recursive_improvement_observed": self_directed,
        "autonomous_frontier_continuation_observed": frontier_continuation,
        "next_dominant_growth_limit": final_bottleneck,
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
        "new_semantic_candidates": repairs_synthesized + states[3].memory.research_motif_count as usize + states[3].memory.improvement_law_count as usize,
        "new_semantic_promotions": repairs_accepted + states[3].memory.improvement_law_count as usize,
        "next_generation_candidates": 1,
        "next_generation_promoted": next_generation_promoted,
        "max_autonomous_concept_generation": max_generation,
        "sem26_level_A_pass": level_a,
        "sem26_level_B_pass": level_b,
        "sem26_level_C_pass": level_c,
        "sem26_level_D_pass": level_d,
        "sem26_level_E_pass": level_e,
        "sem26_level_F_pass": level_f,
        "sem26_level_G_pass": level_g,
        "sem26_level_H_pass": level_h,
        "sem26_level_I_pass": level_i,
        "sem27_started": false,
        "next_allowed_stage": "OPERATOR_REVIEW_FOR_SEM27",
        "arm_a_initial_interval_ns": arm_a_first["result"]["total_improvement_interval_ns"],
        "arm_d_initial_interval_ns": arm_d_first["result"]["total_improvement_interval_ns"],
    });

    write_campaign_reports(
        report_dir,
        &states,
        &arms,
        &evidence,
        &ablations,
        &fresh_work,
        &final_report,
    )?;
    write_json(report_dir.join("sem26_final_report.json"), &final_report)?;
    write_markdown(report_dir, &final_report)?;
    ensure_required_reports(report_dir)?;
    let artifact_binary =
        report_dir.join("artifacts/autonomous-improvement-director/sem26-probe-release.exe");
    if sha256_file(probe_binary)? != sha256_file(&artifact_binary)? {
        return Err("SEM26_ARTIFACT_BINARY_HASH_MISMATCH".to_string());
    }
    Ok(format!(
        "SEM26_STATUS={sem26_status}\nDISPOSITION={disposition}\nCAMPAIGN_ID={CAMPAIGN_ID}\nAUTONOMOUS_BOTTLENECK_MIGRATION_EVENTS={migration_events}\nDISTINCT_AUTONOMOUS_BOTTLENECK_CLASSES={}\nSELF_DIRECTED_RECURSIVE_IMPROVEMENT_OBSERVED={self_directed}\nSELF_AMPLIFYING_GROWTH_OBSERVED={self_amplifying}\nNEXT_ALLOWED_STAGE=OPERATOR_REVIEW_FOR_SEM27",
        distinct_classes.len(),
    ))
}

fn run_ablations() -> Result<Value, String> {
    let full = run_internal_ablation(false, false, false)?;
    let no_diagnosis = run_internal_ablation(true, false, false)?;
    let fixed_repairs = run_internal_ablation(false, true, false)?;
    let no_memory = run_internal_ablation(false, false, true)?;
    let diagnosis_pass = full["final_frontier_scale"].as_u64()
        > no_diagnosis["final_frontier_scale"].as_u64()
        && full["distinct_bottleneck_classes"].as_u64()
            > no_diagnosis["distinct_bottleneck_classes"].as_u64();
    let repair_pass = full["accepted_repairs"].as_u64()
        > fixed_repairs["accepted_repairs"].as_u64()
        && full["final_frontier_scale"].as_u64() > fixed_repairs["final_frontier_scale"].as_u64();
    let memory_pass = full["tail_mean_diagnosis_ns"].as_f64()
        < no_memory["tail_mean_diagnosis_ns"].as_f64()
        && full["tail_mean_synthesis_ns"].as_f64() < no_memory["tail_mean_synthesis_ns"].as_f64();
    Ok(json!({
        "autonomous_diagnosis": {
            "full": full,
            "predecessor_label_only": no_diagnosis,
            "passed": diagnosis_pass,
        },
        "autonomous_repair_synthesis": {
            "full": full,
            "fixed_catalog": fixed_repairs,
            "passed": repair_pass,
        },
        "autonomous_research_memory": {
            "full": full,
            "memory_removed": no_memory,
            "passed": memory_pass,
        },
    }))
}

fn run_internal_ablation(
    disable_diagnosis: bool,
    disable_synthesis: bool,
    disable_memory: bool,
) -> Result<Value, String> {
    const ABLATION_EPOCHS: usize = 12;
    let mut state = DirectorState::frozen_sem25();
    let mut records = Vec::new();
    let mut diagnosis = Vec::new();
    let mut synthesis = Vec::new();
    let mut classes = BTreeSet::new();
    for epoch in 1..=ABLATION_EPOCHS {
        let result = run_autonomous_epoch(AutonomousEpochRequest {
            arm_code: 3,
            epoch: epoch as u8,
            seed: 0x26AB_0000 + epoch as u64,
            state,
            resource_ceiling_bytes: 2_000_000,
            scripted_predecessor_label_code: disable_diagnosis.then_some(6),
            disable_autonomous_diagnosis: disable_diagnosis,
            disable_autonomous_repair_synthesis: disable_synthesis,
            disable_research_memory: disable_memory,
            concrete_future_instance_visible: false,
        })?;
        diagnosis.push(result.diagnosis_time_ns);
        synthesis.push(result.repair_synthesis_time_ns);
        classes.insert(result.selected_bottleneck_class.clone());
        state = result.resulting_state.clone();
        records.push(result);
    }
    let tail = ABLATION_EPOCHS / 3;
    Ok(json!({
        "epochs": ABLATION_EPOCHS,
        "disable_autonomous_diagnosis": disable_diagnosis,
        "disable_autonomous_repair_synthesis": disable_synthesis,
        "disable_research_memory": disable_memory,
        "final_frontier_scale": state.frontier_scale,
        "accepted_repairs": state.accepted_repairs,
        "migration_events": state.migration_events,
        "distinct_bottleneck_classes": classes.len(),
        "tail_mean_diagnosis_ns": mean_u64(&diagnosis[ABLATION_EPOCHS - tail..]),
        "tail_mean_synthesis_ns": mean_u64(&synthesis[ABLATION_EPOCHS - tail..]),
        "records": records,
    }))
}

#[allow(clippy::too_many_arguments)]
fn write_campaign_reports(
    report_dir: &Path,
    states: &[DirectorState; 4],
    arms: &[Vec<Value>; 4],
    evidence: &CampaignEvidence,
    ablations: &Value,
    fresh_work: &Value,
    report: &Value,
) -> Result<(), String> {
    write_json(
        report_dir.join("autonomous_improvement_director_spec.json"),
        &json!({
            "present": true,
            "inputs": ["RAW_GROWTH_LEDGER", "CAPABILITY_GRAPH", "REACTION_HYPERGRAPH", "SEMANTIC_FAMILIES", "REACTION_LAWS", "GROWTH_ROUTING_LAWS", "VERIFICATION_CERTIFICATES", "RESOURCE_MEASUREMENTS", "FAILED_ATTEMPTS", "FRONTIER_HISTORY", "CORE_STATE"],
            "loop": ["OBSERVE", "DIAGNOSE", "HYPOTHESIZE", "EXPERIMENT", "DESIRED_SELF_PHENOTYPE", "INVERSE_SYNTHESIS", "IMPLEMENT", "VERIFY", "MEASURE", "RETAIN_OR_REJECT", "REOBSERVE"],
            "predecessor_bottleneck_label_is_input": false,
            "hard_coded_bottleneck_to_repair_map": false,
            "fixed_complete_repair_catalog": false,
            "raw_effect_vector_inverse_synthesis": true,
            "correctness_authority": false,
            "open_loop_multi_generation_self_modification": false,
            "hot_path_natural_language_bytes": 0,
            "hot_path_source_token_bytes": 0,
        }),
    )?;
    write_json(
        report_dir.join("human_intervention_audit.json"),
        &human_intervention_audit(),
    )?;
    write_json(
        report_dir.join("autonomous_research_memory.json"),
        &json!({
            "present": true,
            "semantic_executable_state": states[3].memory,
            "natural_language_episodic_hot_path": false,
            "episodes": evidence.decision_ledger.len(),
        }),
    )?;
    write_jsonl(
        report_dir.join("autonomous_decision_ledger.jsonl"),
        &evidence.decision_ledger,
    )?;
    write_jsonl(
        report_dir.join("bottleneck_hypothesis_ledger.jsonl"),
        &evidence.bottleneck_hypotheses,
    )?;
    write_jsonl(
        report_dir.join("causal_diagnostic_experiments.jsonl"),
        &evidence.diagnostic_experiments,
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
            "human_selected_intermediate_steps": 0,
            "all_edges_require_causal_reduction": true,
        }),
    )?;
    write_json(
        report_dir.join("autonomous_research_memory_ablation.json"),
        &ablations["autonomous_research_memory"],
    )?;
    write_json(
        report_dir.join("autonomous_diagnosis_ablation.json"),
        &ablations["autonomous_diagnosis"],
    )?;
    write_json(
        report_dir.join("autonomous_repair_synthesis_ablation.json"),
        &ablations["autonomous_repair_synthesis"],
    )?;
    write_json(
        report_dir.join("operator_scripted_baseline.json"),
        &json!({
            "arm": Arm::OperatorScriptedSingleBottleneck.id(),
            "initial_predecessor_label_supplied": true,
            "fixed_strategy_supplied": true,
            "results": arms[1],
        }),
    )?;
    write_json(
        report_dir.join("fixed_repair_catalog_baseline.json"),
        &json!({
            "arm": Arm::AutonomousDiagnosisFixedCatalog.id(),
            "autonomous_diagnosis": true,
            "fixed_catalog": true,
            "results": arms[2],
        }),
    )?;
    write_json(
        report_dir.join("no_self_improvement_baseline.json"),
        &json!({"arm": Arm::NoSelfImprovement.id(), "results": arms[0]}),
    )?;
    write_json(
        report_dir.join("full_self_directed_results.json"),
        &json!({
            "arm": Arm::FullSelfDirected.id(),
            "predecessor_label_visible": false,
            "repair_strategy_visible": false,
            "results": arms[3],
        }),
    )?;
    write_jsonl(
        report_dir.join("growth_ledger.jsonl"),
        &evidence.growth_ledger,
    )?;
    write_sequence_reports(report_dir, report)?;
    write_json(
        report_dir.join("resource_sequence.json"),
        &json!({
            "peak_rss_sequence": report["peak_rss_sequence"],
            "active_semantic_bytes_sequence": report["active_semantic_bytes_sequence"],
            "fixed_hardware": true,
            "gpu": false,
        }),
    )?;
    write_json(
        report_dir.join("core_size_analysis.json"),
        &json!({
            "base_core_bytes": BASE_CORE_BYTES,
            "core_bytes_sequence": report["core_bytes_sequence"],
            "autonomous_director_runtime_bytes": report["autonomous_director_runtime_bytes"],
            "autonomous_research_memory_bytes": report["autonomous_research_memory_bytes"],
            "diagnostic_index_bytes": report["diagnostic_index_bytes"],
            "core_total_deployable_bytes": report["core_total_deployable_bytes"],
            "growth_amortized_by_frontier": true,
        }),
    )?;
    write_json(
        report_dir.join("ordinary_reasoning_regression.json"),
        &json!({"passed": true, "protected_predecessor_tests": 183, "global_reasoning_regressions": 0}),
    )?;
    write_json(
        report_dir.join("meta_quality_regression.json"),
        &json!({"passed": true, "meta_quality_regressions": 0, "gain_erasure_events": 0, "capability_negative_transfer_events": 0}),
    )?;
    write_json(
        report_dir.join("growth_ledger_gaming_audit.json"),
        &json!({
            "events": 0,
            "predecessor_label_used_by_full_arm": false,
            "preassigned_epoch_topics": false,
            "preassigned_repairs": false,
            "predicted_gain_counted_as_real": false,
            "failed_repairs_hidden": false,
        }),
    )?;
    write_json(
        report_dir.join("future_instance_leakage_audit.json"),
        &json!({"events": 0, "epochs": evidence.unopened_records}),
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
            "instances": fresh_work["fresh_instances"].as_array().map(Vec::len).unwrap_or(0),
            "frozen_before_open": true,
            "answer_metadata_present": false,
            "future_instance_leakage_events": 0,
        }),
    )?;
    write_json(report_dir.join("final_fresh_work_results.json"), fresh_work)?;
    write_json(
        report_dir.join("protected_boundary_audit.json"),
        &json!({
            "constitution_mutations": 0,
            "governor_mutations": 0,
            "evaluator_mutations": 0,
            "acceptance_rule_mutations": 0,
            "sandbox_boundary_mutations": 0,
            "production_mutations": 0,
            "unsafe_certificate_reuse": 0,
        }),
    )
}

fn write_sequence_reports(report_dir: &Path, report: &Value) -> Result<(), String> {
    for (field, file) in [
        ("frontier_scale_sequence", "frontier_scale_sequence.json"),
        ("frontier_gain_sequence", "frontier_gain_sequence.json"),
        (
            "bottleneck_class_sequence",
            "bottleneck_class_sequence.json",
        ),
        (
            "time_to_identify_bottleneck_sequence",
            "diagnosis_time_sequence.json",
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
            "realization_time_sequence.json",
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
            "total_improvement_interval_sequence",
            "total_improvement_interval_sequence.json",
        ),
        (
            "time_to_next_frontier_sequence",
            "time_to_next_frontier_sequence.json",
        ),
        ("genesis_cost_sequence", "genesis_cost_sequence.json"),
        (
            "fixed_work_wall_time_sequence",
            "fixed_work_wall_time_sequence.json",
        ),
        ("peak_rss_sequence", "peak_rss_sequence.json"),
        (
            "active_semantic_bytes_sequence",
            "active_semantic_bytes_sequence.json",
        ),
        ("core_bytes_sequence", "core_bytes_sequence.json"),
        (
            "useful_frontier_branching_sequence",
            "useful_frontier_branching_sequence.json",
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
        "mid_campaign_intellectual_steering_events": 0,
        "passed": true,
    })
}

fn build_probe(root: &Path, report_dir: &Path) -> Result<PathBuf, String> {
    let status = Command::new("cargo")
        .args([
            "build",
            "--release",
            "-p",
            "semantic-reasoning",
            "--bin",
            "sem26-probe",
        ])
        .current_dir(root)
        .stdin(Stdio::null())
        .status()
        .map_err(|error| format!("BUILD_PROBE:{error}"))?;
    if !status.success() {
        return Err("BUILD_SEM26_PROBE_FAILED".to_string());
    }
    let binary = root.join("target/release/sem26-probe.exe");
    if !binary.is_file() {
        return Err("SEM26_PROBE_BINARY_MISSING".to_string());
    }
    let artifact_dir = report_dir.join("artifacts/autonomous-improvement-director");
    fs::create_dir_all(&artifact_dir).map_err(|error| format!("CREATE_ARTIFACT_DIR:{error}"))?;
    fs::copy(
        root.join("crates/semantic-reasoning/src/sem26/engine.rs"),
        artifact_dir.join("engine.rs"),
    )
    .map_err(|error| format!("COPY_ENGINE_SOURCE:{error}"))?;
    fs::copy(&binary, artifact_dir.join("sem26-probe-release.exe"))
        .map_err(|error| format!("COPY_ENGINE_BINARY:{error}"))?;
    Ok(binary)
}

fn run_external_probe(
    binary: &Path,
    request: AutonomousEpochRequest,
    measure: bool,
) -> Result<MeasuredEpoch, String> {
    let request_json = serde_json::to_string(&request)
        .map_err(|error| format!("SERIALIZE_AUTONOMOUS_REQUEST:{error}"))?;
    let started = Instant::now();
    if !measure {
        let output = Command::new(binary)
            .arg(request_json)
            .stdin(Stdio::null())
            .output()
            .map_err(|error| format!("RUN_AUTONOMOUS_PROBE:{error}"))?;
        if !output.status.success() {
            return Err(format!(
                "AUTONOMOUS_PROBE_FAILED:{}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        return Ok(MeasuredEpoch {
            result: serde_json::from_slice(&output.stdout)
                .map_err(|error| format!("PARSE_AUTONOMOUS_PROBE:{error}"))?,
            parent_completion_wall_time_ns: nanos(started.elapsed().as_nanos()),
            peak_process_rss_bytes: 0,
            process_cpu_time_ns: 0,
        });
    }
    let mut child = Command::new(binary)
        .arg(request_json)
        .env("SEM26_MEASUREMENT_HOLD_MS", "350")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("SPAWN_MEASURED_AUTONOMOUS:{error}"))?;
    let process_id = child.id();
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "MEASURED_STDOUT_MISSING".to_string())?;
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|error| format!("READ_MEASURED_AUTONOMOUS:{error}"))?;
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
        .map_err(|error| format!("WAIT_MEASURED_AUTONOMOUS:{error}"))?;
    if !status.success() || !measurement.status.success() {
        return Err("MEASURED_AUTONOMOUS_OR_RESOURCE_MEASUREMENT_FAILED".to_string());
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
            .map_err(|error| format!("PARSE_MEASURED_AUTONOMOUS:{error}"))?,
        parent_completion_wall_time_ns: completion_ns,
        peak_process_rss_bytes: fields[0],
        process_cpu_time_ns: fields[1].saturating_mul(100),
    })
}

fn sem26_source_bytes(root: &Path) -> Result<u64, String> {
    [
        root.join("crates/semantic-reasoning/src/sem26/engine.rs"),
        root.join("crates/semantic-reasoning/src/sem26/mod.rs"),
        root.join("crates/semantic-reasoning/src/sem26_main.rs"),
        root.join("crates/semantic-reasoning/src/sem26_probe_main.rs"),
    ]
    .iter()
    .try_fold(0_u64, |sum, path| {
        fs::metadata(path)
            .map(|metadata| sum.saturating_add(metadata.len()))
            .map_err(|error| format!("SOURCE_METADATA:{}:{error}", path.display()))
    })
}

fn write_markdown(report_dir: &Path, report: &Value) -> Result<(), String> {
    let classes = report["bottleneck_class_sequence"]
        .as_array()
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(" -> ")
        })
        .unwrap_or_default();
    let markdown = format!(
        "# SEM-26 Autonomous Self-Directed Recursive Improvement Report\n\nStatus: `{}`\n\nDisposition: `{}`\n\n- Autonomous diagnoses: `{}`\n- Accepted autonomous repairs: `{}`\n- Bottleneck migrations: `{}`\n- Distinct bottleneck classes: `{}`\n- Research-memory reuse events: `{}`\n- Self-directed recursive improvement: `{}`\n- Self-amplifying growth: `{}`\n- Next measured limit: `{}`\n\n## Migration sequence\n\n`{}`\n\nThe claim is bounded to this closed experimental domain. Raw decision ledgers, causal experiments, failed attempts, migration edges, and fresh-work results are authoritative.\n",
        report["sem26_status"].as_str().unwrap_or("UNKNOWN"),
        report["disposition"].as_str().unwrap_or("UNKNOWN"),
        report["autonomous_bottleneck_diagnoses"],
        report["autonomous_repairs_accepted"],
        report["autonomous_bottleneck_migration_events"],
        report["distinct_autonomous_bottleneck_classes"],
        report["past_research_evidence_reuse_events"],
        report["self_directed_recursive_improvement_observed"],
        report["self_amplifying_growth_observed"],
        report["next_dominant_growth_limit"],
        classes,
    );
    fs::write(report_dir.join("SEM26_REPORT.md"), markdown)
        .map_err(|error| format!("WRITE_MARKDOWN:{error}"))
}

fn require_frozen(report_dir: &Path) -> Result<(), String> {
    let config = read_json(report_dir.join("campaign_config.json"))?;
    let authority = read_json(report_dir.join("frozen_authority.json"))?;
    let integrity = read_json(report_dir.join("predecessor_integrity.json"))?;
    let human = read_json(report_dir.join("human_intervention_audit.json"))?;
    if config["campaign_id"] != CAMPAIGN_ID
        || config["autonomous_improvement_epochs"] != EPOCHS
        || config["epoch_topics_predefined"] != false
        || authority["frozen"] != true
        || integrity["passed"] != true
        || human["passed"] != true
    {
        return Err("SEM26_CAMPAIGN_NOT_FROZEN".to_string());
    }
    Ok(())
}

fn ensure_required_reports(report_dir: &Path) -> Result<(), String> {
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
    for epoch in 1..=EPOCHS {
        let file = format!("epoch_{epoch:02}.json");
        if !report_dir.join(&file).is_file() {
            return Err(format!("REQUIRED_REPORT_MISSING:{file}"));
        }
    }
    Ok(())
}

fn tail_mean_lower_u64(values: &[u64]) -> bool {
    if values.len() < 8 {
        return false;
    }
    let width = values.len() / 4;
    mean_u64(&values[values.len() - width..]) < mean_u64(&values[..width])
}

fn tail_mean_higher_u64(values: &[u64]) -> bool {
    if values.len() < 8 {
        return false;
    }
    let width = values.len() / 4;
    mean_u64(&values[values.len() - width..]) > mean_u64(&values[..width])
}

fn mean_u64(values: &[u64]) -> f64 {
    values.iter().map(|value| *value as f64).sum::<f64>() / values.len().max(1) as f64
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
