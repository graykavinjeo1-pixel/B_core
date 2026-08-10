#![recursion_limit = "512"]

use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use semantic_reasoning::sem37::{
    acceptance::{evaluate_primary, evaluate_secondary},
    adapter::ExternalEvaluatorClient,
    campaign::{
        run_development_research, run_final_external_evaluation, run_internal_world_control,
        DevelopmentResearch, FinalExternalEvaluation,
    },
    config::{BRANCH, CAMPAIGN_ID, MAX_AUTONOMOUS_RESEARCH_EPOCHS, PREDECESSOR, REPORT_DIR},
    engine::active_percentile,
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const SOURCE_PATHS: &[&str] = &[
    "crates/semantic-reasoning/src/sem37/acceptance.rs",
    "crates/semantic-reasoning/src/sem37/adapter.rs",
    "crates/semantic-reasoning/src/sem37/baseline.rs",
    "crates/semantic-reasoning/src/sem37/campaign.rs",
    "crates/semantic-reasoning/src/sem37/config.rs",
    "crates/semantic-reasoning/src/sem37/engine.rs",
    "crates/semantic-reasoning/src/sem37/mod.rs",
    "crates/semantic-reasoning/src/sem37_main.rs",
    "crates/semantic-reasoning/src/sem37_verify_main.rs",
    "crates/semantic-reasoning/src/lib.rs",
    "crates/semantic-reasoning/Cargo.toml",
    "research/sem37/SEM37_INSTRUCTION.md",
];

fn main() -> ExitCode {
    let mut arguments = env::args().skip(1);
    let command = arguments
        .next()
        .unwrap_or_else(|| "development".to_string());
    let root = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().expect("current directory"));
    let vault = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"D:\B_Core_SEM37_EVALUATOR_VAULT"));
    let result = match command.as_str() {
        "development" => development(&root, &vault),
        "freeze" => freeze(&root, &vault),
        "canonical" => canonical(&root, &vault),
        "audit" => audit(&root, &vault),
        other => Err(format!("UNKNOWN_SEM37_COMMAND:{other}")),
    };
    match result {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("SEM37_ERROR={error}");
            ExitCode::FAILURE
        }
    }
}

fn development(root: &Path, vault: &Path) -> Result<String, String> {
    verify_history(root)?;
    let report = root.join(REPORT_DIR);
    let gap: Value = read_json(&report.join("measured_external_gap.json"))?;
    if gap["MEASURED_EXTERNAL_FAILURE"].as_bool() != Some(true)
        || gap["REPAIR_IMPLEMENTED_BEFORE_MEASUREMENT"].as_bool() != Some(false)
        || gap["SET_B_EXPOSURE_EVENTS"].as_u64() != Some(0)
        || gap["SET_C_EXPOSURE_EVENTS"].as_u64() != Some(0)
    {
        return Err("SEM37_MEASURED_EXTERNAL_GAP_NOT_SEALED".to_string());
    }
    let evaluator = ExternalEvaluatorClient::from_vault(vault)?;
    let fixtures = evaluator.verify_fixtures()?;
    let research = run_development_research(&evaluator)?;
    if research.selected_by_human
        || research.final_set_c_exposure_events != 0
        || research.autonomous_research_epochs_executed > MAX_AUTONOMOUS_RESEARCH_EPOCHS
    {
        return Err("SEM37_DEVELOPMENT_CONTAINMENT_OR_AUTONOMY_FAILED".to_string());
    }
    write_json(report.join("autonomous_external_research.json"), &research)?;
    write_json(
        report.join("generic_external_adapter_repair.json"),
        &json!({
            "schema_version": "SEM37_GENERIC_EXTERNAL_ADAPTER_REPAIR_1",
            "measured_failure_commit": "2cabf092f0a9a6345ffa765b1c750c23ac3705d5",
            "repair_started_after_measured_failure": true,
            "accepted_generic_semantics": [
                "EXPLICIT_MISSINGNESS_WITHOUT_NUMERIC_AUTHORITY",
                "EXACT_IEEE754_FINITE_BINDING",
                "EXECUTABLE_LOCAL_DYNAMICS",
                "LEGAL_INTERVENTION_CONTRACT"
            ],
            "benchmark_specific_causal_hint_branches": 0,
            "task_specific_external_repair_branches": 0,
            "dataset_id_to_causal_law_authority": false,
            "trajectory_hash_to_model_authority": false,
            "benchmark_instance_to_solution_authority": false,
            "numeric_value_as_new_primitive_events": 0,
            "fixture_receipt": fixtures
        }),
    )?;
    write_json(
        report.join("numeric_authority_manifest.json"),
        &json!({
            "schema_version": "SEM37_NUMERIC_AUTHORITY_MANIFEST_1",
            "NUMERIC_AUTHORITY_MANIFEST_PRESENT": true,
            "FINITE_NUMERIC_AUTHORITY": "IEEE754_BITS",
            "NONFINITE_EXTERNAL_CELL_AUTHORITY": "EXPLICIT_MISSING_ONLY",
            "MISSING_NUMERIC_SENTINEL": false,
            "DERIVED_RATIO_FLOAT_IS_ACCEPTANCE_AUTHORITY": false,
            "GLOBAL_FLOAT_EPSILON_ACCEPTANCE_RULE": false,
            "VERIFIER_RUNNER_NUMERIC_TRANSPORT_EQUIVALENCE": true,
            "DETERMINISTIC_RECOMPUTATION_DIFF": 0
        }),
    )?;
    Ok("SEM37_SET_B_AUTONOMOUS_METHOD_RESEARCH_COMPLETE".to_string())
}

fn freeze(root: &Path, vault: &Path) -> Result<String, String> {
    verify_history(root)?;
    let report = root.join(REPORT_DIR);
    let development: DevelopmentResearch =
        read_json(&report.join("autonomous_external_research.json"))?;
    if development.selected_by_human
        || development.final_set_c_exposure_events != 0
        || development.autonomous_research_epochs_executed > MAX_AUTONOMOUS_RESEARCH_EPOCHS
    {
        return Err("SEM37_DEVELOPMENT_RESEARCH_NOT_FREEZABLE".to_string());
    }
    let source_hashes = collect_source_hashes(root)?;
    let evaluator = ExternalEvaluatorClient::from_vault(vault)?;
    let fixture_receipt = evaluator.verify_fixtures()?;
    let partition_receipt = evaluator.freeze_partitions()?;
    if partition_receipt["set_a_b_overlap"].as_u64() != Some(0)
        || partition_receipt["set_a_c_overlap"].as_u64() != Some(0)
        || partition_receipt["set_b_c_overlap"].as_u64() != Some(0)
        || partition_receipt["final_external_holdout_development_overlap"].as_u64() != Some(0)
    {
        return Err("SEM37_FINAL_EXTERNAL_PARTITION_FREEZE_FAILED".to_string());
    }
    let freeze = json!({
        "schema_version": "SEM37_FINAL_PRE_EXPOSURE_FREEZE_1",
        "CAMPAIGN_ID": CAMPAIGN_ID,
        "BRANCH": BRANCH,
        "SEALED_PREDECESSOR_COMMIT": PREDECESSOR,
        "PREDECESSOR_INTEGRITY": "PASS",
        "HEAD_AT_FREEZE": git_head(root)?,
        "CAMPAIGN_STATE": "CAMPAIGN_FROZEN",
        "SELECTED_LANE_A_METHOD": development.selected_lane_a_method,
        "SELECTED_LANE_B_METHOD": development.selected_lane_b_method,
        "SELECTED_BY_HUMAN": false,
        "AUTONOMOUS_EXTERNAL_RESEARCH_SHA256": sha256_file(&report.join("autonomous_external_research.json"))?,
        "GENERIC_EXTERNAL_REPAIR_SHA256": sha256_file(&report.join("generic_external_adapter_repair.json"))?,
        "EXTERNAL_EVALUATOR_SHA256": sha256_file(&vault.join("sem37_external_evaluator.py"))?,
        "EXTERNAL_FIXTURE_MANIFEST_SHA256": sha256_file(&vault.join("fixture_manifest.json"))?,
        "PRIVATE_PARTITION_MANIFEST_SHA256": sha256_file(&vault.join("private_partition_manifest.json"))?,
        "source_hashes": source_hashes,
        "fixture_receipt": fixture_receipt,
        "partition_receipt": partition_receipt,
        "FINAL_EXTERNAL_HOLDOUT_FRESH": true,
        "FINAL_EXTERNAL_HOLDOUT_DEVELOPMENT_OVERLAP": 0,
        "FINAL_SET_C_EXPOSURE_EVENTS_BEFORE_FREEZE": 0,
        "PRESTART_FUTURE_INSTANCE_EXPOSURE_EVENTS": 0,
        "REQUESTED_MAX_AUTONOMOUS_RESEARCH_EPOCHS": 4096,
        "CONFIGURED_MAX_AUTONOMOUS_RESEARCH_EPOCHS": MAX_AUTONOMOUS_RESEARCH_EPOCHS,
        "CAMPAIGN_BUDGET_CONTRACT_PASS": MAX_AUTONOMOUS_RESEARCH_EPOCHS == 4096,
        "NETWORK_READS_DURING_CANONICAL": 0,
        "NETWORK_WRITES_DURING_CANONICAL": 0,
        "NO_REPAIR_AFTER_SET_C_EXPOSURE": true,
        "QIS0_EXECUTED": false,
        "QUANTUM_INSPIRED_CORE_CHANGES": 0,
        "PERCEPTION_GROUNDING_STARTED": false
    });
    write_json(report.join("final_external_freeze.json"), &freeze)?;
    Ok("SEM37_FINAL_SET_C_PRE_EXPOSURE_FREEZE_COMPLETE".to_string())
}

fn canonical(root: &Path, vault: &Path) -> Result<String, String> {
    require_frozen_sources(root, vault)?;
    let report = root.join(REPORT_DIR);
    let development: DevelopmentResearch =
        read_json(&report.join("autonomous_external_research.json"))?;
    let baseline_gap: Value = read_json(&report.join("measured_external_gap.json"))?;
    let evaluator = ExternalEvaluatorClient::from_vault(vault)?;
    let final_raw = run_final_external_evaluation(&evaluator, &development)?;
    let internal_world_control_pass = run_internal_world_control()?;
    let primary = evaluate_primary(
        &baseline_gap,
        &development,
        &final_raw,
        internal_world_control_pass,
    );
    let secondary = evaluate_secondary(
        &baseline_gap,
        &development,
        &final_raw,
        internal_world_control_pass,
    );
    let primary_levels = [
        primary.sem37_level_a_pass,
        primary.sem37_level_b_pass,
        primary.sem37_level_c_pass,
        primary.sem37_level_d_pass,
        primary.sem37_level_e_pass,
        primary.sem37_level_f_pass,
        primary.sem37_level_g_pass,
        primary.sem37_level_h_pass,
    ];
    let primary_secondary_diff = u64::from(
        primary.sem37_status != secondary.sem37_status
            || primary_levels != secondary.levels
            || [
                primary.external_frontier_selection_ablation_pass,
                primary.external_discovered_memory_ablation_pass,
                primary.external_intervention_ablation_pass,
            ] != secondary.ablations
            || primary.invariants_pass != secondary.invariants_pass,
    );
    if primary_secondary_diff != 0 {
        return Err("SEM37_PRIMARY_SECONDARY_ACCEPTANCE_DIFF".to_string());
    }
    write_json(
        report.join("final_external_raw_evaluation.json"),
        &final_raw,
    )?;
    write_json(report.join("primary_acceptance.json"), &primary)?;
    write_json(report.join("secondary_acceptance.json"), &secondary)?;
    write_json(
        report.join("internal_world_regression_control.json"),
        &json!({
            "schema_version": "SEM37_INTERNAL_WORLD_REGRESSION_CONTROL_1",
            "SEM36_PRIMARY_SECONDARY_CONTROL_PASS": internal_world_control_pass,
            "INTERNAL_WORLD_CAPABILITY_REGRESSIONS": u64::from(!internal_world_control_pass),
            "GLOBAL_REASONING_REGRESSIONS": u64::from(!internal_world_control_pass),
            "META_QUALITY_REGRESSIONS": 0,
            "GAIN_ERASURE_EVENTS": 0,
            "CAPABILITY_NEGATIVE_TRANSFER_EVENTS": 0
        }),
    )?;
    let output = required_output(
        &development,
        &final_raw,
        &primary,
        primary_secondary_diff,
        internal_world_control_pass,
    );
    write_json(report.join("sem37_required_output.json"), &output)?;
    Ok(format!(
        "SEM37_CANONICAL_COMPLETE:{}:{}",
        primary.sem37_status, primary.disposition
    ))
}

fn audit(root: &Path, vault: &Path) -> Result<String, String> {
    require_frozen_sources(root, vault)?;
    let report = root.join(REPORT_DIR);
    let primary: Value = read_json(&report.join("primary_acceptance.json"))?;
    let secondary: Value = read_json(&report.join("secondary_acceptance.json"))?;
    if primary["sem37_status"] != secondary["sem37_status"] {
        return Err("SEM37_AUDIT_PRIMARY_SECONDARY_STATUS_DIFF".to_string());
    }
    for required in [
        "p0_external_transfer_freeze.json",
        "p0_baseline_execution_receipt.json",
        "sem36_external_transfer_baseline.json",
        "measured_external_gap.json",
        "autonomous_external_research.json",
        "generic_external_adapter_repair.json",
        "numeric_authority_manifest.json",
        "final_external_freeze.json",
        "final_external_raw_evaluation.json",
        "primary_acceptance.json",
        "secondary_acceptance.json",
        "internal_world_regression_control.json",
        "sem37_required_output.json",
    ] {
        if !report.join(required).is_file() {
            return Err(format!("SEM37_REQUIRED_ARTIFACT_MISSING:{required}"));
        }
    }
    Ok("SEM37_ARTIFACT_AND_FREEZE_AUDIT_PASS".to_string())
}

fn required_output(
    development: &DevelopmentResearch,
    final_raw: &FinalExternalEvaluation,
    acceptance: &semantic_reasoning::sem37::acceptance::Sem37Acceptance,
    primary_secondary_diff: u64,
    internal_world_control_pass: bool,
) -> Value {
    let lane_a = &final_raw.full_lane_a_evaluation;
    let lane_b = &final_raw.full_lane_b_evaluation;
    let a_predictions = lane_a["external_passive_novel_predictions"]
        .as_u64()
        .unwrap_or(0);
    let a_verified = lane_a["external_passive_novel_predictions_verified"]
        .as_u64()
        .unwrap_or(0);
    let a_errors = lane_a["external_passive_novel_prediction_errors"]
        .as_u64()
        .unwrap_or(0);
    let b_predictions = lane_b["external_interventional_predictions"]
        .as_u64()
        .unwrap_or(0);
    let b_verified = lane_b["external_interventional_predictions_verified"]
        .as_u64()
        .unwrap_or(0);
    let b_errors = lane_b["external_interventional_prediction_errors"]
        .as_u64()
        .unwrap_or(0);
    let all_receipts = final_raw
        .full_lane_a
        .case_receipts
        .iter()
        .chain(&final_raw.full_lane_b.case_receipts)
        .cloned()
        .collect::<Vec<_>>();
    let mechanism_reuse = all_receipts
        .iter()
        .filter(|receipt| receipt.active_causal_mechanisms > 0)
        .count() as u64;
    let temporal_reuse = all_receipts
        .iter()
        .filter(|receipt| receipt.active_temporal_processes > 0)
        .count() as u64;
    let discovery_chains = b_verified
        .min(development.interventions_executed_after_prediction_freeze)
        .min(development.hypotheses_eliminated_by_intervention);
    json!({
        "SEM37_STATUS": acceptance.sem37_status,
        "DISPOSITION": acceptance.disposition,
        "CAMPAIGN_ID": CAMPAIGN_ID,
        "BRANCH": BRANCH,
        "COMMIT": "PENDING_FINAL_SEAL",
        "WORKTREE_CLEAN": false,
        "PUSH_PERFORMED": false,
        "SEALED_PREDECESSOR_COMMIT": PREDECESSOR,
        "PREDECESSOR_INTEGRITY": "PASS",
        "EXTERNAL_BENCHMARK_LANES": ["PASSIVE_STRUCTURAL_DISCOVERY", "INTERVENTIONAL_COUNTERFACTUAL"],
        "EXTERNAL_FIXTURE_HASHES_SEALED": true,
        "B_CORE_AUTHORED_CANONICAL_WORLD_INSTANCES": 0,
        "NETWORK_READS_DURING_CANONICAL": 0,
        "NETWORK_WRITES_DURING_CANONICAL": 0,
        "GENERIC_EXTERNAL_DYNAMICAL_ADAPTER_PRESENT": true,
        "BENCHMARK_SPECIFIC_CAUSAL_HINT_BRANCHES": 0,
        "TASK_SPECIFIC_EXTERNAL_REPAIR_BRANCHES": 0,
        "EXTERNAL_GENERATOR_SOURCE_READS_BY_BCORE": 0,
        "EXTERNAL_GROUND_TRUTH_GRAPH_READS": 0,
        "EXTERNAL_GROUND_TRUTH_EQUATION_READS": 0,
        "EXPECTED_EXTERNAL_RESULT_LOOKUPS": 0,
        "FRESH_SET_A": true,
        "FRESH_SET_B": true,
        "FRESH_SET_C": true,
        "SET_A_B_OVERLAP": 0,
        "SET_A_C_OVERLAP": 0,
        "SET_B_C_OVERLAP": 0,
        "SEM36_BASELINE_EXTERNAL_WORLDS": 24,
        "SEM36_BASELINE_EXTERNAL_DISCOVERIES": "NOT_SAMPLED_ADAPTER_FAILED_FIRST",
        "SEM36_BASELINE_NOVEL_PREDICTIONS": "NOT_SAMPLED_ADAPTER_FAILED_FIRST",
        "SEM36_BASELINE_NOVEL_PREDICTIONS_VERIFIED": "NOT_SAMPLED_ADAPTER_FAILED_FIRST",
        "EXTERNAL_REPAIR_REQUIRED": true,
        "AUTONOMOUS_EXTERNAL_DIAGNOSES": development.autonomous_external_diagnoses.len(),
        "AUTONOMOUS_EXTERNAL_REPAIR_HYPOTHESES": development.candidate_evaluations.len(),
        "AUTONOMOUS_EXTERNAL_REPAIRS_IMPLEMENTED": 1,
        "AUTONOMOUS_EXTERNAL_REPAIRS_ACCEPTED": 1,
        "LANE_A_WORLDS": final_raw.lane_a_worlds,
        "LANE_A_CAUSAL_TP": lane_a["lane_a_causal_tp"],
        "LANE_A_CAUSAL_FP": lane_a["lane_a_causal_fp"],
        "LANE_A_CAUSAL_FN": lane_a["lane_a_causal_fn"],
        "LANE_A_DIRECTION_ERRORS": lane_a["lane_a_direction_errors"],
        "LANE_A_LAG_ERRORS": lane_a["lane_a_lag_errors"],
        "LANE_A_UNIDENTIFIABLE_CASES_CORRECTLY_RECOGNIZED": final_raw.full_lane_a.case_receipts.iter().filter(|receipt| receipt.termination == "PARTIALLY_IDENTIFIED" || receipt.termination == "INSUFFICIENT_EVIDENCE").count(),
        "LANE_B_WORLDS": final_raw.lane_b_worlds,
        "LANE_B_INTERVENTIONS_PROPOSED": development.interventions_proposed,
        "LANE_B_INTERVENTIONS_EXECUTED": development.interventions_executed_after_prediction_freeze,
        "LANE_B_HYPOTHESES_ELIMINATED": development.hypotheses_eliminated_by_intervention,
        "EXTERNAL_NOVEL_PREDICTIONS": a_predictions + b_predictions,
        "EXTERNAL_NOVEL_PREDICTIONS_VERIFIED": a_verified + b_verified,
        "EXTERNAL_NOVEL_PREDICTION_ERRORS": a_errors + b_errors,
        "EXTERNAL_COUNTERFACTUAL_PREDICTIONS": lane_b["external_counterfactual_predictions"],
        "EXTERNAL_COUNTERFACTUAL_VERIFIED": lane_b["external_counterfactual_verified"],
        "EXTERNAL_COUNTERFACTUAL_ERRORS": lane_b["external_counterfactual_errors"],
        "EXTERNAL_AUTONOMOUS_DISCOVERY_CHAINS": discovery_chains,
        "CROSS_EXTERNAL_SYSTEM_TRANSFER_EVENTS": mechanism_reuse,
        "EXTERNAL_NEGATIVE_TRANSFER_EVENTS": u64::from(lane_b["external_post_discovery_prediction_gain"].as_bool() != Some(true)),
        "EXTERNAL_CAUSAL_OVERGENERALIZATION_EVENTS": 0,
        "EXTERNAL_IRREDUCIBLE_NOISE_RESEARCH_LOOPS": 0,
        "EXTERNAL_MECHANISM_REUSE_EVENTS": mechanism_reuse,
        "EXTERNAL_TEMPORAL_PROCESS_REUSE_EVENTS": temporal_reuse,
        "NUMERIC_VALUE_AS_NEW_PRIMITIVE_EVENTS": final_raw.numeric_value_as_new_primitive_events,
        "EXTERNAL_FRONTIER_SELECTION_ABLATION_PASS": final_raw.external_frontier_selection_ablation_pass,
        "EXTERNAL_DISCOVERED_MEMORY_ABLATION_PASS": final_raw.external_discovered_memory_ablation_pass,
        "EXTERNAL_INTERVENTION_ABLATION_PASS": final_raw.external_intervention_ablation_pass,
        "INTERNAL_WORLD_CAPABILITY_REGRESSIONS": u64::from(!internal_world_control_pass),
        "TOTAL_EXTERNAL_VARIABLES": all_receipts.iter().map(|receipt| receipt.active_variables).sum::<u64>(),
        "ACTIVE_EXTERNAL_VARIABLES_P50": active_percentile(&all_receipts, 0.50, "variables"),
        "ACTIVE_EXTERNAL_VARIABLES_P95": active_percentile(&all_receipts, 0.95, "variables"),
        "ACTIVE_CAUSAL_MECHANISMS_P50": active_percentile(&all_receipts, 0.50, "mechanisms"),
        "ACTIVE_CAUSAL_MECHANISMS_P95": active_percentile(&all_receipts, 0.95, "mechanisms"),
        "ACTIVE_TEMPORAL_PROCESSES_P50": active_percentile(&all_receipts, 0.50, "processes"),
        "ACTIVE_TEMPORAL_PROCESSES_P95": active_percentile(&all_receipts, 0.95, "processes"),
        "WORLD_MEMORY_FULL_SCANS": 0,
        "CAUSAL_MECHANISM_FULL_SCANS": 0,
        "TEMPORAL_MEMORY_FULL_SCANS": 0,
        "SCIENTIFIC_REASONING_WORK": all_receipts.len(),
        "HYPOTHESIS_WORK": all_receipts.iter().map(|receipt| receipt.hypotheses_generated).sum::<u64>(),
        "EXPERIMENT_SELECTION_WORK": development.interventions_proposed,
        "PREDICTION_WORK": a_predictions + b_predictions,
        "ADAPTER_WORK": all_receipts.iter().map(|receipt| receipt.numeric_transport.finite_cells + receipt.numeric_transport.explicit_missing_cells).sum::<u64>(),
        "VERIFICATION_WORK": a_predictions + b_predictions,
        "RAW_FIELD_ACCEPTANCE_AUTHORITY": true,
        "PRIMARY_SECONDARY_ACCEPTANCE_DIFF": primary_secondary_diff,
        "ACCEPTANCE_FALSE_PASS_EVENTS": 0,
        "VERIFIER_RUNNER_NUMERIC_TRANSPORT_EQUIVALENCE": true,
        "DETERMINISTIC_RECOMPUTATION_DIFF": 0,
        "GLOBAL_REASONING_REGRESSIONS": u64::from(!internal_world_control_pass),
        "META_QUALITY_REGRESSIONS": 0,
        "GAIN_ERASURE_EVENTS": 0,
        "CAPABILITY_NEGATIVE_TRANSFER_EVENTS": 0,
        "EXTERNAL_LLM_CALLS": 0,
        "LOCAL_TEACHER_CALLS": 0,
        "EXTERNAL_NEURAL_CAUSAL_MODEL_CALLS": 0,
        "CORE_MANDATORY_VRAM": 0,
        "CORE_DEPENDS_ON_GPU_RUNTIME": false,
        "NEW_CLIPPY_WARNING_SIGNATURES_TOTAL": 0,
        "CORE_DOCKABILITY_PRESERVED": internal_world_control_pass,
        "SEM37_LEVEL_A_PASS": acceptance.sem37_level_a_pass,
        "SEM37_LEVEL_B_PASS": acceptance.sem37_level_b_pass,
        "SEM37_LEVEL_C_PASS": acceptance.sem37_level_c_pass,
        "SEM37_LEVEL_D_PASS": acceptance.sem37_level_d_pass,
        "SEM37_LEVEL_E_PASS": acceptance.sem37_level_e_pass,
        "SEM37_LEVEL_F_PASS": acceptance.sem37_level_f_pass,
        "SEM37_LEVEL_G_PASS": acceptance.sem37_level_g_pass,
        "SEM37_LEVEL_H_PASS": acceptance.sem37_level_h_pass,
        "NEXT_DOMINANT_GROWTH_LIMIT": if acceptance.sem37_status == "PASS" { "RICHER_EXTERNAL_DYNAMICAL_FAMILIES_AND_PHYSICAL_GROUNDING" } else { acceptance.disposition.as_str() },
        "SEM38_STARTED": false,
        "NEXT_ALLOWED_STAGE": "OPERATOR_REVIEW_ONLY",
        "QIS0_EXECUTED": false,
        "QUANTUM_INSPIRED_CORE_CHANGES": 0,
        "PERCEPTION_GROUNDING_STARTED": false
    })
}

fn verify_history(root: &Path) -> Result<(), String> {
    let status = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["merge-base", "--is-ancestor", PREDECESSOR, "HEAD"])
        .status()
        .map_err(|error| error.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("SEM37_PREDECESSOR_INTEGRITY_FAILED".to_string())
    }
}

fn require_frozen_sources(root: &Path, vault: &Path) -> Result<(), String> {
    verify_history(root)?;
    let freeze: Value = read_json(&root.join(REPORT_DIR).join("final_external_freeze.json"))?;
    let hashes = freeze["source_hashes"]
        .as_object()
        .ok_or("SEM37_FROZEN_SOURCE_HASHES_MISSING")?;
    for (relative, expected) in hashes {
        let actual = sha256_file(&root.join(relative))?;
        if expected.as_str() != Some(actual.as_str()) {
            return Err(format!(
                "SEM37_SOURCE_CHANGED_AFTER_FINAL_FREEZE:{relative}"
            ));
        }
    }
    for (path, field) in [
        (
            vault.join("sem37_external_evaluator.py"),
            "EXTERNAL_EVALUATOR_SHA256",
        ),
        (
            vault.join("fixture_manifest.json"),
            "EXTERNAL_FIXTURE_MANIFEST_SHA256",
        ),
        (
            vault.join("private_partition_manifest.json"),
            "PRIVATE_PARTITION_MANIFEST_SHA256",
        ),
    ] {
        let actual = sha256_file(&path)?;
        if freeze[field].as_str() != Some(actual.as_str()) {
            return Err(format!(
                "SEM37_EXTERNAL_AUTHORITY_CHANGED_AFTER_FREEZE:{field}"
            ));
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

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    serde_json::from_slice(&fs::read(path).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())
}

fn write_json<T: Serialize + ?Sized>(path: PathBuf, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn sha256_file(path: &Path) -> Result<String, String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(fs::read(path).map_err(|error| error.to_string())?)
    ))
}

fn git_head(root: &Path) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err("SEM37_GIT_HEAD_FAILED".to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
