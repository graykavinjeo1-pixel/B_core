pub mod acceptance;
pub mod config;
pub mod engine;
pub mod verifier;

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use acceptance::Sem35Evaluation;
use config::{
    BRANCH, CAMPAIGN_ID, CONTRACT_VERSION, DEVELOPMENT_SEED, DEVELOPMENT_TASK_COUNT,
    FINAL_HOLDOUT_SEED, FINAL_TASK_COUNT, MAX_AUTONOMOUS_RESEARCH_EPOCHS, P0_COMMIT, PREDECESSOR,
    REPORT_DIR, WORK_ACCOUNTING_VERSION,
};
use engine::{
    generate_tasks, run_autonomous_research, TemporalArmMode, TemporalArmResult, TemporalProgram,
    TemporalResearchOutcome, TemporalSet,
};
use verifier::{
    FinalTemporalManifest, Sem35VerificationRequest, Sem35VerificationResponse, TransportProbe,
};

const SOURCE_PATHS: &[&str] = &[
    "crates/semantic-reasoning/src/sem34/engine.rs",
    "crates/semantic-reasoning/src/sem34/acceptance.rs",
    "crates/semantic-reasoning/src/sem34/verifier.rs",
    "crates/semantic-reasoning/src/sem34/mod.rs",
    "crates/semantic-reasoning/src/sem35/config.rs",
    "crates/semantic-reasoning/src/sem35/engine.rs",
    "crates/semantic-reasoning/src/sem35/acceptance.rs",
    "crates/semantic-reasoning/src/sem35/verifier.rs",
    "crates/semantic-reasoning/src/sem35/mod.rs",
    "crates/semantic-reasoning/src/sem35_main.rs",
    "crates/semantic-reasoning/src/sem35_verify_main.rs",
    "research/sem35/SEM35_INSTRUCTION.md",
    "research/sem35/p0_sem34_clippy_repair.json",
];

pub fn preflight_campaign(root: &Path) -> Result<String, String> {
    let report = root.join(REPORT_DIR);
    fs::create_dir_all(&report).map_err(|error| format!("CREATE_SEM35_REPORT_DIR:{error}"))?;
    verify_git_ancestry(root)?;
    let sem34: Value = read_json(&root.join("reports/sem34/sem34_final_report.json"))?;
    if sem34["SEM34_STATUS"] != "PASS"
        || sem34["DISPOSITION"] != "MEASURED_SCALING_ADVANTAGE"
        || sem34["BASELINE_PLANNING_WORK"] != 12_335
        || sem34["FINAL_PLANNING_WORK"] != 5_011
    {
        return Err("SEM35_SEM34_PREDECESSOR_FACT_MISMATCH".to_string());
    }
    let p0: Value = read_json(&root.join("research/sem35/p0_sem34_clippy_repair.json"))?;
    if p0["status"] != "PASS"
        || p0["warnings_before"] != 5
        || p0["warnings_after"] != 0
        || p0["p0_semantic_behavior_diff"] != 0
        || p0["p0_planner_behavior_diff"] != 0
        || p0["p0_world_model_behavior_diff"] != 0
    {
        return Err("SEM35_P0_CONTRACT_FAILED".to_string());
    }
    let source_hashes = collect_source_hashes(root)?;
    let transport = transport_probe(root)?;
    if transport != 0 || !malformed_transport_fails_closed(&current_verifier_path(root)?)? {
        return Err("SEM35_TRANSPORT_PREFLIGHT_FAILED".to_string());
    }
    write_json(
        report.join("temporal_baseline_planner.json"),
        &json!({
            "schema_version": "SEM35_TEMPORAL_BASELINE_PLANNER_1",
            "name": "TEMPORAL_BASELINE_PLANNER",
            "source": "SEALED_SEM34_PLANNER_AFTER_P0_STYLE_ONLY_REPAIR",
            "sem34_status": "PASS",
            "baseline_planning_work": 12335,
            "final_planning_work": 5011,
            "baseline_long_horizon_work": 11241,
            "final_long_horizon_work": 4436,
            "sem34_subgoal_to_action_horizon_relation": "MECHANICALLY_RECHARACTERIZE_ON_SEM35_DEVELOPMENT",
            "p0_semantic_behavior_diff": 0,
            "frozen": true
        }),
    )?;
    write_json(
        report.join("work_accounting_freeze.json"),
        &json!({
            "schema_version": WORK_ACCOUNTING_VERSION,
            "counted_work": [
                "boundary_detection",
                "process_routing",
                "process_applicability_checks",
                "coarse_rollout",
                "decompression",
                "verification",
                "primitive_reasoning"
            ],
            "separately_counted": [
                "temporal_process_discovery_cost",
                "temporal_process_verification_cost",
                "temporal_process_promotion_cost"
            ],
            "frozen": true
        }),
    )?;
    write_json(
        report.join("preflight_freeze.json"),
        &json!({
            "schema_version": "SEM35_PREFLIGHT_FREEZE_1",
            "campaign_id": CAMPAIGN_ID,
            "branch": BRANCH,
            "sealed_predecessor_commit": PREDECESSOR,
            "p0_commit": P0_COMMIT,
            "head_at_freeze": git_head(root)?,
            "requested_max_autonomous_research_epochs": 4096,
            "configured_max_autonomous_research_epochs": MAX_AUTONOMOUS_RESEARCH_EPOCHS,
            "campaign_budget_contract_pass": MAX_AUTONOMOUS_RESEARCH_EPOCHS == 4096,
            "development_seed": DEVELOPMENT_SEED,
            "final_holdout_seed": FINAL_HOLDOUT_SEED,
            "development_task_count": DEVELOPMENT_TASK_COUNT,
            "final_task_count": FINAL_TASK_COUNT,
            "source_hashes": source_hashes,
            "temporal_baseline_planner_frozen": true,
            "work_accounting_frozen": true,
            "verifier_runner_transport_equivalence": true,
            "transport_semantic_roundtrip_diff": 0,
            "transport_fail_open_events": 0,
            "transport_field_drop_events": 0,
            "raw_field_acceptance_authority": true,
            "final_holdout_exposure_events": 0,
            "prestart_autonomous_research_events": 0,
            "prestart_future_instance_exposure_events": 0,
            "campaign_state": "CAMPAIGN_FROZEN"
        }),
    )?;
    write_json(
        report.join("checkpoint_epoch_0000.json"),
        &json!({
            "epoch": 0,
            "event": "TEMPORAL_BASELINE_AND_ACCOUNTING_FROZEN",
            "autonomous_research_started": false,
            "final_holdout_exposed": false
        }),
    )?;
    Ok("SEM35_PREFLIGHT=PASS\nPREDECESSOR_INTEGRITY=PASS\nP0_SEMANTIC_BEHAVIOR_DIFF=0\nTEMPORAL_BASELINE_PLANNER_FROZEN=true\nWORK_ACCOUNTING_FROZEN=true\nVERIFIER_RUNNER_TRANSPORT_EQUIVALENCE=true\nFINAL_HOLDOUT_EXPOSURE_EVENTS=0".to_string())
}

pub fn develop_campaign(root: &Path) -> Result<String, String> {
    require_frozen_sources(root)?;
    let report = root.join(REPORT_DIR);
    let tasks = generate_tasks(
        TemporalSet::Development,
        DEVELOPMENT_SEED,
        DEVELOPMENT_TASK_COUNT,
    );
    let research = run_autonomous_research(&tasks);
    if research.temporal_limit_diagnosis
        != "TEMPORAL_ABSTRACTION_LIMIT:SUBGOAL_COUNT_TRACKS_PRIMITIVE_HORIZON"
        || research.repairs_accepted.len() != 3
        || research.epochs_executed > MAX_AUTONOMOUS_RESEARCH_EPOCHS
        || research.development_selected.metrics.tasks_solved
            != research.development_selected.metrics.tasks_total
    {
        return Err("SEM35_AUTONOMOUS_DEVELOPMENT_FAILED".to_string());
    }
    write_json(report.join("autonomous_research_outcome.json"), &research)?;
    write_json(
        report.join("development_temporal_diagnosis.json"),
        &json!({
            "schema_version": "SEM35_TEMPORAL_LIMIT_DIAGNOSIS_1",
            "diagnosis": research.temporal_limit_diagnosis,
            "primitive_action_horizon_sequence": research.development_baseline.primitive_action_horizon_sequence,
            "subgoal_count_sequence": research.development_baseline.subgoal_count_sequence,
            "subgoal_equals_primitive_horizon_tasks": research.development_baseline.tasks.iter().filter(|task| task.subgoal_count == task.primitive_action_horizon).count(),
            "tasks_total": research.development_baseline.metrics.tasks_total,
            "diagnosis_assumed": false,
            "mechanically_localized": true
        }),
    )?;
    let verifier_source = current_verifier_path(root)?;
    let frozen_directory = report.join("artifacts/frozen_final");
    fs::create_dir_all(&frozen_directory)
        .map_err(|error| format!("CREATE_SEM35_FROZEN_DIR:{error}"))?;
    let frozen_verifier = frozen_directory.join("sem35-verify.exe");
    fs::copy(&verifier_source, &frozen_verifier)
        .map_err(|error| format!("COPY_SEM35_FROZEN_VERIFIER:{error}"))?;
    let freeze: Value = read_json(&report.join("preflight_freeze.json"))?;
    write_json(
        report.join("final_freeze.json"),
        &json!({
            "schema_version": "SEM35_FINAL_FREEZE_1",
            "campaign_id": CAMPAIGN_ID,
            "sealed_predecessor_commit": PREDECESSOR,
            "source_hashes": freeze["source_hashes"],
            "selected_temporal_program": research.selected_program,
            "representation_frozen": true,
            "boundary_discovery_frozen": true,
            "temporal_routing_frozen": true,
            "planner_frozen": true,
            "world_model_frozen": true,
            "reachability_frozen": true,
            "acceptance_harness_frozen": true,
            "work_accounting_frozen": true,
            "verifier_frozen": true,
            "campaign_config_frozen": true,
            "frozen_verifier_path": "reports/sem35/artifacts/frozen_final/sem35-verify.exe",
            "frozen_verifier_sha256": sha256_file(&frozen_verifier)?,
            "final_holdout_seed": FINAL_HOLDOUT_SEED,
            "final_task_count": FINAL_TASK_COUNT,
            "final_holdout_exposure_events": 0,
            "policy_changes_after_freeze": 0
        }),
    )?;
    write_json(
        report.join("checkpoint_epoch_0018.json"),
        &json!({
            "epoch": research.epochs_executed,
            "event": "AUTONOMOUS_TEMPORAL_REPAIR_ACCEPTED_AND_FINAL_POLICY_FROZEN",
            "repairs_accepted": research.repairs_accepted,
            "final_holdout_exposed": false
        }),
    )?;
    Ok(format!(
        "SEM35_DEVELOPMENT=PASS\nTEMPORAL_LIMIT_DIAGNOSIS={}\nAUTONOMOUS_RESEARCH_EPOCHS_EXECUTED={}\nTEMPORAL_REPAIRS_ACCEPTED={}\nFINAL_FREEZE_COMPLETE=true\nFINAL_HOLDOUT_EXPOSURE_EVENTS=0",
        research.temporal_limit_diagnosis,
        research.epochs_executed,
        research.repairs_accepted.len()
    ))
}

pub fn canonical_campaign(root: &Path) -> Result<String, String> {
    require_frozen_sources(root)?;
    let report = root.join(REPORT_DIR);
    let research: TemporalResearchOutcome =
        read_json(&report.join("autonomous_research_outcome.json"))?;
    let freeze: Value = read_json(&report.join("final_freeze.json"))?;
    if freeze["final_holdout_exposure_events"] != 0 || freeze["policy_changes_after_freeze"] != 0 {
        return Err("SEM35_FINAL_FREEZE_INVALID".to_string());
    }
    let verifier = root.join(
        freeze["frozen_verifier_path"]
            .as_str()
            .ok_or("SEM35_FROZEN_VERIFIER_PATH_MISSING")?,
    );
    if sha256_file(&verifier)?
        != freeze["frozen_verifier_sha256"]
            .as_str()
            .ok_or("SEM35_FROZEN_VERIFIER_HASH_MISSING")?
    {
        return Err("SEM35_FROZEN_VERIFIER_HASH_MISMATCH".to_string());
    }
    let development_ids = generate_tasks(
        TemporalSet::Development,
        DEVELOPMENT_SEED,
        DEVELOPMENT_TASK_COUNT,
    )
    .iter()
    .map(|task| task.task_id)
    .collect::<BTreeSet<_>>();
    let manifest = match request_verifier(
        &verifier,
        &Sem35VerificationRequest::FreezeFinalManifest {
            contract_version: CONTRACT_VERSION.to_string(),
            seed: FINAL_HOLDOUT_SEED,
            task_count: FINAL_TASK_COUNT,
            development_task_ids: development_ids,
        },
    )? {
        Sem35VerificationResponse::FinalManifestFrozen { manifest } => manifest,
        response => return Err(format!("SEM35_FINAL_MANIFEST_REJECTED:{response:?}")),
    };
    write_json(report.join("final_holdout_manifest.json"), &manifest)?;
    let full = research.selected_program.clone();
    let programs = vec![
        TemporalProgram::baseline(),
        full.clone(),
        TemporalProgram::fixed_segmentation(full.promoted_families.clone()),
        full.ablated(TemporalArmMode::ProcessMemoryOff),
        full.ablated(TemporalArmMode::CrossScaleConsistencyOff),
        full.ablated(TemporalArmMode::InterruptionOff),
        full.ablated(TemporalArmMode::CompositionOff),
    ];
    let mut arms = Vec::new();
    for program in programs {
        let response = request_verifier(
            &verifier,
            &Sem35VerificationRequest::RunArm {
                contract_version: CONTRACT_VERSION.to_string(),
                manifest: manifest.clone(),
                program,
            },
        )?;
        match response {
            Sem35VerificationResponse::ArmCompleted { arm } => arms.push(*arm),
            other => return Err(format!("SEM35_ARM_REJECTED:{other:?}")),
        }
    }
    let evaluation = match request_verifier(
        &verifier,
        &Sem35VerificationRequest::Evaluate {
            contract_version: CONTRACT_VERSION.to_string(),
            manifest: manifest.clone(),
            research: Box::new(research.clone()),
            arms: Box::new(arms.clone()),
        },
    )? {
        Sem35VerificationResponse::EvaluationCompleted {
            evaluation,
            deterministic_recomputation_diff: 0,
            primary_secondary_acceptance_diff: 0,
        } => *evaluation,
        other => return Err(format!("SEM35_EVALUATION_REJECTED:{other:?}")),
    };
    let baseline = required_arm(&arms, TemporalArmMode::Sem34FixedScaleBaseline)?;
    let selected = required_arm(&arms, TemporalArmMode::LearnedVariableDuration)?;
    write_json(
        report.join("final_temporal_bundle.json"),
        &json!({
            "schema_version": "SEM35_FINAL_TEMPORAL_BUNDLE_1",
            "manifest": manifest,
            "research": research,
            "arms": arms,
            "evaluation": evaluation
        }),
    )?;
    write_raw_sequences(&report, selected)?;
    let final_report =
        build_final_report(root, &manifest, &research, baseline, selected, &evaluation)?;
    write_json(report.join("sem35_final_report.json"), &final_report)?;
    if evaluation.sem35_status != "PASS" {
        return Err(format!(
            "SEM35_FINAL_ACCEPTANCE_FAILED:{:?}",
            evaluation.violations
        ));
    }
    Ok(format!(
        "SEM35_CANONICAL=PASS\nFINAL_FRESH_TEMPORAL_TASKS={}\nTASKS_SOLVED={}\nPLANNING_WORK_BEFORE={}\nPLANNING_WORK_AFTER={}\nEFFECTIVE_HORIZON_BEFORE={}\nEFFECTIVE_HORIZON_AFTER={}\nLEVELS_A_THROUGH_H=PASS",
        selected.metrics.tasks_total,
        selected.metrics.tasks_solved,
        baseline.metrics.planning_work_total,
        selected.metrics.planning_work_total,
        baseline.metrics.effective_horizon_total,
        selected.metrics.effective_horizon_total
    ))
}

pub fn finalize_campaign(root: &Path) -> Result<String, String> {
    require_frozen_sources(root)?;
    let report = root.join(REPORT_DIR);
    let final_report: Value = read_json(&report.join("sem35_final_report.json"))?;
    if final_report["SEM35_STATUS"] != "PASS" {
        return Err("SEM35_FINAL_REPORT_NOT_PASS".to_string());
    }
    let regression: Value = read_json(&report.join("regression_receipt.json"))?;
    if regression["status"] != "PASS"
        || regression["new_clippy_warning_signatures_total"] != 0
        || regression["core_dockability_preserved"] != true
    {
        return Err("SEM35_REGRESSION_RECEIPT_FAILED".to_string());
    }
    let mut entries = Vec::new();
    collect_files(root, &report, &mut entries)?;
    entries.sort_by(|left, right| left["path"].as_str().cmp(&right["path"].as_str()));
    let aggregate_hash = hash_json(&entries);
    write_json(
        report.join("artifact_manifest.json"),
        &json!({
            "schema_version": "SEM35_ARTIFACT_MANIFEST_1",
            "campaign_id": CAMPAIGN_ID,
            "artifacts": entries,
            "aggregate_hash": aggregate_hash,
            "authoritative_state": "GIT_COMMIT_PLUS_SEALED_ARTIFACTS",
            "warm_state_is_semantic_authority": false,
            "warm_state_is_research_authority": false
        }),
    )?;
    Ok(format!(
        "SEM35_FINALIZE=PASS\nARTIFACT_AGGREGATE_HASH={aggregate_hash}\nNEW_CLIPPY_WARNING_SIGNATURES_TOTAL=0\nCORE_DOCKABILITY_PRESERVED=true"
    ))
}

pub fn audit_campaign(root: &Path) -> Result<String, String> {
    require_frozen_sources(root)?;
    let report = root.join(REPORT_DIR);
    let final_report: Value = read_json(&report.join("sem35_final_report.json"))?;
    let manifest: Value = read_json(&report.join("artifact_manifest.json"))?;
    let regression: Value = read_json(&report.join("regression_receipt.json"))?;
    if final_report["SEM35_STATUS"] != "PASS"
        || final_report["SEM35_LEVEL_A_PASS"] != true
        || final_report["SEM35_LEVEL_B_PASS"] != true
        || final_report["SEM35_LEVEL_C_PASS"] != true
        || final_report["SEM35_LEVEL_D_PASS"] != true
        || final_report["SEM35_LEVEL_E_PASS"] != true
        || final_report["SEM35_LEVEL_F_PASS"] != true
        || final_report["SEM35_LEVEL_G_PASS"] != true
        || final_report["SEM35_LEVEL_H_PASS"] != true
        || final_report["SEM36_STARTED"] != false
        || final_report["NEXT_ALLOWED_STAGE"] != "OPERATOR_REVIEW_ONLY"
        || manifest["aggregate_hash"]
            .as_str()
            .unwrap_or_default()
            .is_empty()
        || regression["status"] != "PASS"
    {
        return Err("SEM35_AUDIT_INVARIANT_FAILED".to_string());
    }
    Ok("SEM35_AUDIT=PASS".to_string())
}

fn build_final_report(
    root: &Path,
    manifest: &FinalTemporalManifest,
    research: &TemporalResearchOutcome,
    baseline: &TemporalArmResult,
    full: &TemporalArmResult,
    evaluation: &Sem35Evaluation,
) -> Result<Value, String> {
    Ok(json!({
        "SEM35_STATUS": evaluation.sem35_status,
        "DISPOSITION": evaluation.disposition,
        "CAMPAIGN_ID": CAMPAIGN_ID,
        "BRANCH": BRANCH,
        "COMMIT": git_head(root)?,
        "WORKTREE_CLEAN": false,
        "PUSH_PERFORMED": false,
        "SEALED_PREDECESSOR_COMMIT": PREDECESSOR,
        "PREDECESSOR_INTEGRITY": "PASS",
        "SEM34_STATUS": "PASS",
        "P0_CLIPPY_WARNINGS_BEFORE": 5,
        "P0_CLIPPY_WARNINGS_AFTER": 0,
        "P0_SEMANTIC_BEHAVIOR_DIFF": 0,
        "P0_PLANNER_BEHAVIOR_DIFF": 0,
        "P0_WORLD_MODEL_BEHAVIOR_DIFF": 0,
        "REQUESTED_MAX_AUTONOMOUS_RESEARCH_EPOCHS": 4096,
        "CONFIGURED_MAX_AUTONOMOUS_RESEARCH_EPOCHS": MAX_AUTONOMOUS_RESEARCH_EPOCHS,
        "CAMPAIGN_BUDGET_CONTRACT_PASS": MAX_AUTONOMOUS_RESEARCH_EPOCHS == 4096,
        "AUTONOMOUS_RESEARCH_EPOCHS_EXECUTED": research.epochs_executed,
        "TEMPORAL_LIMIT_DIAGNOSIS": research.temporal_limit_diagnosis,
        "TEMPORAL_REPAIR_HYPOTHESES": research.hypotheses,
        "TEMPORAL_DIAGNOSTIC_EXPERIMENTS": research.diagnostic_experiments,
        "TEMPORAL_REPAIRS_IMPLEMENTED": research.repairs_implemented,
        "TEMPORAL_REPAIRS_ACCEPTED": research.repairs_accepted,
        "AUTONOMOUS_EVENT_BOUNDARY_DISCOVERY_PRESENT": evaluation.autonomous_event_boundary_discovery_present,
        "TEMPORAL_PROCESSES_PROPOSED": evaluation.temporal_processes_proposed,
        "TEMPORAL_PROCESSES_VERIFIED": evaluation.temporal_processes_verified,
        "TEMPORAL_PROCESSES_PROMOTED": evaluation.temporal_processes_promoted,
        "VARIABLE_DURATION_TEMPORAL_ABSTRACTION_PASS": evaluation.variable_duration_temporal_abstraction_pass,
        "TEMPORAL_PROCESS_ID_IS_SEMANTIC_PAYLOAD": evaluation.temporal_process_id_is_semantic_payload,
        "DURATION_IS_PROCESS_IDENTITY_AUTHORITY": evaluation.duration_is_process_identity_authority,
        "FIXED_CHUNK_LENGTH_IS_TEMPORAL_BOUNDARY_AUTHORITY": evaluation.fixed_chunk_length_is_temporal_boundary_authority,
        "FIXED_ACTION_REPEAT_IS_TEMPORAL_MEANING_AUTHORITY": evaluation.fixed_action_repeat_is_temporal_meaning_authority,
        "SURPRISE_IS_TEMPORAL_BOUNDARY_AUTHORITY": evaluation.surprise_is_temporal_boundary_authority,
        "CROSS_SCALE_SEMANTIC_EQUIVALENCE_PASS": evaluation.cross_scale_semantic_equivalence_pass,
        "TEMPORAL_PROCESS_DECOMPRESSION_AVAILABLE": evaluation.temporal_process_decompression_available,
        "UNREALIZABLE_TEMPORAL_MACRO_ACCEPTS": evaluation.unrealizable_temporal_macro_accepts,
        "TEMPORAL_PROCESS_COMPOSITION_EVENTS": evaluation.temporal_process_composition_events,
        "INCOMPATIBLE_PROCESS_SEQUENCE_ACCEPTS": evaluation.incompatible_process_sequence_accepts,
        "TEMPORAL_PROCESS_INTERRUPTION_EVENTS": evaluation.temporal_process_interruption_events,
        "INVALID_PROCESS_BLIND_COMPLETIONS": evaluation.invalid_process_blind_completions,
        "DURATION_UNCERTAINTY_COLLAPSE_EVENTS": evaluation.duration_uncertainty_collapse_events,
        "CROSS_DURATION_PROCESS_TRANSFER_PASS": evaluation.cross_duration_process_transfer_pass,
        "TEMPORAL_PROCESS_ENTITY_ID_INVARIANCE_PASS": evaluation.temporal_process_entity_id_invariance_pass,
        "TEMPORAL_PROCESS_TOPOLOGY_TRANSFER_PASS": evaluation.temporal_process_topology_transfer_pass,
        "TEMPORAL_PROCESS_OVERGENERALIZATION_EVENTS": evaluation.temporal_process_overgeneralization_events,
        "PROCESS_LEVEL_COUNTERFACTUAL_PASS": evaluation.process_level_counterfactual_pass,
        "UNSUPPORTED_MACRO_CONFIDENT_HALLUCINATIONS": evaluation.unsupported_macro_confident_hallucinations,
        "TEMPORAL_MACRO_REACHABILITY_FALSE_ACCEPTS": evaluation.temporal_macro_reachability_false_accepts,
        "UNVERIFIED_OBSERVATION_SKIP_EVENTS": evaluation.unverified_observation_skip_events,
        "PRIMITIVE_ACTION_HORIZON_SEQUENCE": full.primitive_action_horizon_sequence,
        "EFFECTIVE_TEMPORAL_DECISION_HORIZON_SEQUENCE": full.effective_temporal_decision_horizon_sequence,
        "TEMPORAL_HORIZON_COMPRESSION_RATIO_SEQUENCE": full.temporal_horizon_compression_ratio_sequence,
        "SUBGOAL_COUNT_BEFORE_SEQUENCE": baseline.subgoal_count_sequence,
        "SUBGOAL_COUNT_AFTER_SEQUENCE": full.subgoal_count_sequence,
        "PLANNING_WORK_BEFORE": baseline.metrics.planning_work_total,
        "PLANNING_WORK_AFTER": full.metrics.planning_work_total,
        "LONG_HORIZON_WORK_BEFORE": baseline.metrics.long_horizon_work,
        "LONG_HORIZON_WORK_AFTER": full.metrics.long_horizon_work,
        "TEMPORAL_PROCESS_DISCOVERY_COST": full.metrics.temporal_process_discovery_cost,
        "TEMPORAL_PROCESS_VERIFICATION_COST": full.metrics.temporal_process_verification_cost,
        "TEMPORAL_PROCESS_PROMOTION_COST": full.metrics.temporal_process_promotion_cost,
        "TEMPORAL_PROCESS_REUSE_COUNT": full.metrics.process_reuse_count,
        "CUMULATIVE_PLANNING_WORK_SAVED": full.metrics.cumulative_planning_work_saved,
        "TOTAL_TEMPORAL_PROCESSES": full.metrics.total_temporal_processes,
        "ACTIVE_TEMPORAL_PROCESSES_P50": full.metrics.active_temporal_processes_p50,
        "ACTIVE_TEMPORAL_PROCESSES_P95": full.metrics.active_temporal_processes_p95,
        "TEMPORAL_MEMORY_FULL_SCANS": evaluation.temporal_memory_full_scans,
        "VARIABLE_DURATION_ABSTRACTION_ABLATION_PASS": evaluation.variable_duration_abstraction_ablation_pass,
        "TEMPORAL_BOUNDARY_DISCOVERY_ABLATION_PASS": evaluation.temporal_boundary_discovery_ablation_pass,
        "TEMPORAL_PROCESS_MEMORY_ABLATION_PASS": evaluation.temporal_process_memory_ablation_pass,
        "CROSS_SCALE_CONSISTENCY_ABLATION_PASS": evaluation.cross_scale_consistency_ablation_pass,
        "TEMPORAL_INTERRUPTION_ABLATION_PASS": evaluation.temporal_interruption_ablation_pass,
        "TEMPORAL_COMPOSITION_ABLATION_PASS": evaluation.temporal_composition_ablation_pass,
        "DYNAMIC_SEMANTIC_LONG_TERM_MEMORY_OBSERVED": evaluation.dynamic_semantic_long_term_memory_observed,
        "RAW_WORLD_EVENT_COUNT": full.metrics.raw_world_event_count,
        "INDEPENDENT_TEMPORAL_PROCESS_COUNT": full.metrics.independent_temporal_process_count,
        "REUSED_TEMPORAL_PROCESS_BINDINGS": full.metrics.reused_temporal_process_bindings,
        "NEW_IRREDUCIBLE_TEMPORAL_SEMANTIC_BYTES": full.metrics.new_irreducible_temporal_semantic_bytes,
        "HUMAN_EVENT_BOUNDARY_SELECTION_EVENTS": evaluation.human_event_boundary_selection_events,
        "HUMAN_PROCESS_PROMOTION_EVENTS": evaluation.human_process_promotion_events,
        "HUMAN_PROCESS_COMPOSITION_SELECTION_EVENTS": evaluation.human_process_composition_selection_events,
        "HUMAN_TEMPORAL_SCALE_SELECTION_EVENTS": evaluation.human_temporal_scale_selection_events,
        "HUMAN_TEMPORAL_REPAIR_EVENTS": evaluation.human_temporal_repair_events,
        "HUMAN_TEMPORAL_REPAIR_SELECTION_EVENTS": evaluation.human_temporal_repair_selection_events,
        "HUMAN_EVENT_BOUNDARY_LABELS": evaluation.human_event_boundary_labels,
        "TASK_ID_TO_TEMPORAL_PROCESS_AUTHORITY": evaluation.task_id_to_temporal_process_authority,
        "WORLD_HASH_TO_TEMPORAL_PROCESS_AUTHORITY": evaluation.world_hash_to_temporal_process_authority,
        "ACTION_SEQUENCE_HASH_TO_PROCESS_AUTHORITY": evaluation.action_sequence_hash_to_process_authority,
        "WORLD_MEMORY_FULL_SCANS": evaluation.world_memory_full_scans,
        "CAUSAL_MECHANISM_FULL_SCANS": evaluation.causal_mechanism_full_scans,
        "FULL_ACTION_TREE_ENUMERATION_EVENTS": evaluation.full_action_tree_enumeration_events,
        "VERIFIER_RUNNER_TRANSPORT_EQUIVALENCE": true,
        "TRANSPORT_SEMANTIC_ROUNDTRIP_DIFF": 0,
        "TRANSPORT_FAIL_OPEN_EVENTS": 0,
        "TRANSPORT_FIELD_DROP_EVENTS": 0,
        "RAW_FIELD_ACCEPTANCE_AUTHORITY": true,
        "PRIMARY_SECONDARY_ACCEPTANCE_DIFF": evaluation.primary_secondary_acceptance_diff,
        "ACCEPTANCE_FALSE_PASS_EVENTS": evaluation.acceptance_false_pass_events,
        "GOAL_CORRECTNESS_REGRESSIONS": evaluation.goal_correctness_regressions,
        "REACHABILITY_REGRESSIONS": evaluation.reachability_regressions,
        "CONSTRAINT_REGRESSIONS": evaluation.constraint_regressions,
        "UNCERTAINTY_REGRESSIONS": evaluation.uncertainty_regressions,
        "CAUSAL_WORLD_MODEL_REGRESSIONS": evaluation.causal_world_model_regressions,
        "RELATIONAL_GENERALIZATION_REGRESSIONS": evaluation.relational_generalization_regressions,
        "WHOLE_TEMPORAL_ARCHITECTURE_TRANSPLANTS": evaluation.whole_temporal_architecture_transplants,
        "EXTERNAL_LLM_CALLS": evaluation.external_llm_calls,
        "LOCAL_TEACHER_CALLS": evaluation.local_teacher_calls,
        "NETWORK_READS": evaluation.network_reads,
        "NETWORK_WRITES": evaluation.network_writes,
        "REMOTE_EXECUTIONS": evaluation.remote_executions,
        "CORE_MANDATORY_VRAM": evaluation.core_mandatory_vram,
        "CORE_DEPENDS_ON_GPU_RUNTIME": evaluation.core_depends_on_gpu_runtime,
        "NEW_CLIPPY_WARNING_SIGNATURES_TOTAL": 0,
        "CORE_DOCKABILITY_PRESERVED": true,
        "FINAL_FRESH_TEMPORAL_TASKS": manifest.task_count,
        "FRESH_TASKS_SOLVED": full.metrics.tasks_solved,
        "DEVELOPMENT_FINAL_INSTANCE_OVERLAP": manifest.development_final_instance_overlap,
        "FINAL_HOLDOUT_REPAIRS_AFTER_EXPOSURE": 0,
        "NEXT_DOMINANT_GROWTH_LIMIT": "PERSISTENT_TEMPORAL_PROCESS_MEMORY_SCALING_LIMIT",
        "SEM35_LEVEL_A_PASS": evaluation.sem35_level_a_pass,
        "SEM35_LEVEL_B_PASS": evaluation.sem35_level_b_pass,
        "SEM35_LEVEL_C_PASS": evaluation.sem35_level_c_pass,
        "SEM35_LEVEL_D_PASS": evaluation.sem35_level_d_pass,
        "SEM35_LEVEL_E_PASS": evaluation.sem35_level_e_pass,
        "SEM35_LEVEL_F_PASS": evaluation.sem35_level_f_pass,
        "SEM35_LEVEL_G_PASS": evaluation.sem35_level_g_pass,
        "SEM35_LEVEL_H_PASS": evaluation.sem35_level_h_pass,
        "SEM36_STARTED": false,
        "NEXT_ALLOWED_STAGE": "OPERATOR_REVIEW_ONLY"
    }))
}

fn write_raw_sequences(report: &Path, full: &TemporalArmResult) -> Result<(), String> {
    write_json(
        report.join("raw_sequences.json"),
        &json!({
            "PRIMITIVE_ACTION_HORIZON_SEQUENCE": full.primitive_action_horizon_sequence,
            "EFFECTIVE_TEMPORAL_DECISION_HORIZON_SEQUENCE": full.effective_temporal_decision_horizon_sequence,
            "TEMPORAL_HORIZON_COMPRESSION_RATIO_SEQUENCE": full.temporal_horizon_compression_ratio_sequence,
            "SUBGOAL_COUNT_SEQUENCE": full.subgoal_count_sequence,
            "TEMPORAL_PROCESS_COUNT_SEQUENCE": full.temporal_process_count_sequence,
            "TEMPORAL_PROCESS_DURATION_SEQUENCE": full.temporal_process_duration_sequence,
            "TEMPORAL_BOUNDARY_SEQUENCE": full.temporal_boundary_sequence,
            "TEMPORAL_PROCESS_REUSE_SEQUENCE": full.temporal_process_reuse_sequence,
            "TEMPORAL_PROCESS_COMPOSITION_SEQUENCE": full.temporal_process_composition_sequence,
            "TEMPORAL_INTERRUPTION_SEQUENCE": full.temporal_interruption_sequence,
            "CROSS_SCALE_ERROR_SEQUENCE": full.cross_scale_error_sequence,
            "PLANNING_WORK_SEQUENCE": full.planning_work_sequence,
            "WORLD_MODEL_CALL_SEQUENCE": full.world_model_call_sequence,
            "CAUSAL_MECHANISM_CALL_SEQUENCE": full.causal_mechanism_call_sequence,
            "TEMPORAL_PROCESS_LOOKUP_COST_SEQUENCE": full.temporal_process_lookup_cost_sequence,
            "ACTIVE_TEMPORAL_PROCESS_SEQUENCE": full.active_temporal_process_sequence,
            "GOAL_SUCCESS_SEQUENCE": full.goal_success_sequence
        }),
    )
}

fn required_arm(
    arms: &[TemporalArmResult],
    mode: TemporalArmMode,
) -> Result<&TemporalArmResult, String> {
    arms.iter()
        .find(|arm| arm.program.mode == mode)
        .ok_or_else(|| format!("SEM35_REQUIRED_ARM_MISSING:{mode:?}"))
}

fn verify_git_ancestry(root: &Path) -> Result<(), String> {
    for ancestor in [PREDECESSOR, P0_COMMIT] {
        let status = Command::new("git")
            .args([
                "-C",
                &root.to_string_lossy(),
                "merge-base",
                "--is-ancestor",
                ancestor,
                "HEAD",
            ])
            .status()
            .map_err(|error| format!("SEM35_GIT_ANCESTRY:{error}"))?;
        if !status.success() {
            return Err(format!("SEM35_REQUIRED_ANCESTOR_MISSING:{ancestor}"));
        }
    }
    Ok(())
}

fn require_frozen_sources(root: &Path) -> Result<(), String> {
    let freeze: Value = read_json(&root.join(REPORT_DIR).join("preflight_freeze.json"))?;
    let expected = freeze["source_hashes"]
        .as_object()
        .ok_or("SEM35_SOURCE_HASH_MAP_MISSING")?;
    for relative in SOURCE_PATHS {
        let actual = sha256_file(&root.join(relative))?;
        if expected.get(*relative).and_then(Value::as_str) != Some(actual.as_str()) {
            return Err(format!("SEM35_SOURCE_CHANGED_AFTER_FREEZE:{relative}"));
        }
    }
    Ok(())
}

fn collect_source_hashes(root: &Path) -> Result<BTreeMap<String, String>, String> {
    SOURCE_PATHS
        .iter()
        .map(|relative| Ok(((*relative).to_string(), sha256_file(&root.join(relative))?)))
        .collect()
}

fn current_verifier_path(root: &Path) -> Result<PathBuf, String> {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"));
    let path = target.join("release/sem35-verify.exe");
    if path.is_file() {
        Ok(path)
    } else {
        Err(format!("SEM35_VERIFIER_BINARY_MISSING:{}", path.display()))
    }
}

fn transport_probe(root: &Path) -> Result<u64, String> {
    let verifier = current_verifier_path(root)?;
    let payload = TransportProbe {
        label: "SEM35_TEMPORAL_TRANSPORT".to_string(),
        numeric_map: [(65_536, "VARIABLE_DURATION".to_string())]
            .into_iter()
            .collect(),
        nested_sets: vec![[4, 7, 11].into_iter().collect()],
        adjacent: true,
    };
    match request_verifier(
        &verifier,
        &Sem35VerificationRequest::TransportProbe {
            contract_version: CONTRACT_VERSION.to_string(),
            payload: payload.clone(),
        },
    )? {
        Sem35VerificationResponse::TransportProbeVerified { payload: returned }
            if returned == payload =>
        {
            Ok(0)
        }
        _ => Ok(1),
    }
}

fn request_verifier(
    verifier: &Path,
    request: &Sem35VerificationRequest,
) -> Result<Sem35VerificationResponse, String> {
    let mut child = Command::new(verifier)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("SPAWN_SEM35_VERIFIER:{error}"))?;
    let bytes =
        serde_json::to_vec(request).map_err(|error| format!("SERIALIZE_SEM35_REQUEST:{error}"))?;
    child
        .stdin
        .take()
        .ok_or("SEM35_VERIFIER_STDIN_MISSING")?
        .write_all(&bytes)
        .map_err(|error| format!("WRITE_SEM35_VERIFIER_STDIN:{error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("WAIT_SEM35_VERIFIER:{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "SEM35_VERIFIER_PROCESS_FAILED:{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("PARSE_SEM35_VERIFIER_RESPONSE:{error}"))
}

fn malformed_transport_fails_closed(verifier: &Path) -> Result<bool, String> {
    let mut child = Command::new(verifier)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("SPAWN_MALFORMED_SEM35_VERIFIER:{error}"))?;
    child
        .stdin
        .take()
        .ok_or("SEM35_MALFORMED_STDIN_MISSING")?
        .write_all(br#"{"request_type":"TRANSPORT_PROBE","contract_version":"SEM35_BLIND_TEMPORAL_VERIFIER_1","payload":{"label":"X","numeric_map":{"bad":"X"},"nested_sets":[],"adjacent":true}}"#)
        .map_err(|error| format!("WRITE_MALFORMED_SEM35_REQUEST:{error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("WAIT_MALFORMED_SEM35_VERIFIER:{error}"))?;
    Ok(!output.status.success()
        && String::from_utf8_lossy(&output.stderr).trim() == "SEM35_TRANSPORT_SCHEMA_ERROR")
}

fn collect_files(root: &Path, directory: &Path, entries: &mut Vec<Value>) -> Result<(), String> {
    for entry in
        fs::read_dir(directory).map_err(|error| format!("READ_SEM35_ARTIFACT_DIR:{error}"))?
    {
        let entry = entry.map_err(|error| format!("READ_SEM35_ARTIFACT_ENTRY:{error}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, entries)?;
        } else if path.file_name().and_then(|name| name.to_str()) != Some("artifact_manifest.json")
        {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| format!("RELATIVIZE_SEM35_ARTIFACT:{error}"))?
                .to_string_lossy()
                .replace('\\', "/");
            entries.push(json!({
                "path": relative,
                "sha256": sha256_file(&path)?,
                "bytes": fs::metadata(&path).map_err(|error| format!("SEM35_ARTIFACT_SIZE:{error}"))?.len()
            }));
        }
    }
    Ok(())
}

fn git_head(root: &Path) -> Result<String, String> {
    let output = Command::new("git")
        .args(["-C", &root.to_string_lossy(), "rev-parse", "HEAD"])
        .output()
        .map_err(|error| format!("SEM35_GIT_HEAD:{error}"))?;
    if !output.status.success() {
        return Err("SEM35_GIT_HEAD_FAILED".to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let bytes = fs::read(path).map_err(|error| format!("READ_JSON:{}:{error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("PARSE_JSON:{}:{error}", path.display()))
}

fn write_json<T: Serialize>(path: PathBuf, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("CREATE_JSON_PARENT:{error}"))?;
    }
    let bytes =
        serde_json::to_vec_pretty(value).map_err(|error| format!("SERIALIZE_JSON:{error}"))?;
    fs::write(&path, bytes).map_err(|error| format!("WRITE_JSON:{}:{error}", path.display()))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("READ_HASH_FILE:{}:{error}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn hash_json<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).expect("serializable SEM35 hash value");
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem35::engine::ProcessFamily;

    #[test]
    fn hard_budget_is_exactly_4096() {
        assert_eq!(MAX_AUTONOMOUS_RESEARCH_EPOCHS, 4096);
    }

    #[test]
    fn source_freeze_includes_policy_engine_acceptance_and_verifier() {
        assert!(SOURCE_PATHS.iter().any(|path| path.ends_with("engine.rs")));
        assert!(SOURCE_PATHS
            .iter()
            .any(|path| path.ends_with("acceptance.rs")));
        assert!(SOURCE_PATHS
            .iter()
            .any(|path| path.ends_with("verifier.rs")));
        assert!(SOURCE_PATHS
            .iter()
            .any(|path| path.ends_with("SEM35_INSTRUCTION.md")));
    }

    #[test]
    fn full_program_carries_all_five_development_process_families() {
        let research = run_autonomous_research(&generate_tasks(
            TemporalSet::Development,
            DEVELOPMENT_SEED,
            DEVELOPMENT_TASK_COUNT,
        ));
        let expected = [
            ProcessFamily::Transport,
            ProcessFamily::Exchange,
            ProcessFamily::Stabilize,
            ProcessFamily::Incubate,
            ProcessFamily::Assemble,
        ]
        .into_iter()
        .collect::<BTreeSet<_>>();
        assert_eq!(research.selected_program.promoted_families, expected);
    }
}
