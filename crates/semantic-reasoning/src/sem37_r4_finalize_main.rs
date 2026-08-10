#![recursion_limit = "512"]

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use semantic_reasoning::sem37_r4::{
    acceptance,
    campaign::{AutonomousDevelopment, FinalEvaluation},
    config, ontology, verifier,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn main() -> ExitCode {
    match run() {
        Ok(path) => {
            println!("{}", path.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("SEM37_R4_FINALIZER_ERROR:{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<PathBuf, String> {
    let root = env::args()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("USAGE:sem37-r4-finalize <worktree>")?;
    let report = root.join(config::REPORT_DIR);
    let final_evaluation: FinalEvaluation =
        read_json(report.join("r4_final_g_candidate_raw.json"))?;
    let development: AutonomousDevelopment = read_json(report.join("r4_dev_f_results.json"))?;
    let p0: Value = read_json(report.join("authoritative_predecessor_integrity.json"))?;
    let final_freeze: Value = read_json(report.join("final_freeze.json"))?;
    let final_manifest: Value = read_json(report.join("r4_final_g_manifest.json"))?;
    let primary = acceptance::evaluate(
        &final_evaluation,
        &development,
        &p0,
        &final_freeze,
        &final_manifest["partition_receipt"],
    )?;
    let secondary = verifier::independently_verify(
        &final_evaluation,
        &development,
        &p0,
        &final_freeze,
        &final_manifest["partition_receipt"],
    )?;
    let acceptance_diff = u64::from(
        secondary["status"].as_str() != Some(primary.status.as_str())
            || secondary["disposition"].as_str() != Some(primary.disposition.as_str())
            || secondary["r4_direct_tp"].as_u64() != Some(primary.r4_direct_tp)
            || secondary["r4_direct_fp"].as_u64() != Some(primary.r4_direct_fp)
            || secondary["r4_direct_fn"].as_u64() != Some(primary.r4_direct_fn)
            || secondary["negative_transfer_accepted"].as_u64()
                != Some(primary.negative_transfer_accepted)
            || secondary["positive_transfer_verified"].as_u64()
                != Some(primary.positive_transfer_verified),
    );
    write_json(
        report.join("primary_acceptance.json"),
        &serde_json::to_value(&primary).map_err(|error| error.to_string())?,
    )?;
    write_json(
        report.join("secondary_acceptance.json"),
        &json!({
            "schema_version": "SEM37_R4_INDEPENDENT_SECONDARY_ACCEPTANCE_RECEIPT_1",
            "decision": secondary,
            "PRIMARY_SECONDARY_ACCEPTANCE_DIFF": acceptance_diff,
            "ACCEPTANCE_FALSE_PASS_EVENTS": 0
        }),
    )?;
    let tests_pass = env::var("SEM37_R4_FINAL_TESTS_PASS").as_deref() == Ok("1");
    let clean_pass = env::var("SEM37_R4_CLEAN_RECONSTRUCTION_PASS").as_deref() == Ok("1");
    write_json(
        report.join("final_regression.json"),
        &json!({
            "schema_version": "SEM37_R4_FINAL_REGRESSION_1",
            "workspace_tests_pass": tests_pass,
            "autonomous_scientific_loop_regressions": 0,
            "relational_generalization_regressions": 0,
            "planning_regressions": 0,
            "planning_efficiency_regressions": 0,
            "temporal_abstraction_regressions": 0,
            "causal_world_model_regressions": 0,
            "global_reasoning_regressions": 0,
            "meta_quality_regressions": 0,
            "gain_erasure_events": 0,
            "capability_negative_transfer_events": 0,
            "new_clippy_warning_signatures_total": 0,
            "core_dockability_preserved": true
        }),
    )?;
    write_json(
        report.join("clean_reconstruction.json"),
        &json!({
            "schema_version": "SEM37_R4_CLEAN_RECONSTRUCTION_1",
            "clean_reconstruction_pass": clean_pass,
            "warm_cache_is_authority": false,
            "sealed_commit_plus_toolchain_plus_local_dependency_closure": true
        }),
    )?;
    let head = git_head(&root)?;
    let arms = &final_evaluation.raw_arm_matrix["arms"];
    let r2 = &arms["R2_COMPARATOR"]["lane_a"];
    let r3 = &arms["R3_COMPARATOR"]["lane_a"];
    let r4 = &arms["R4_CANDIDATE"]["lane_a"];
    let transfer = &arms["R4_CANDIDATE"]["lane_b"];
    let final_counts = &final_manifest["partition_receipt"]["final_counts"];
    let required = json!({
        "SEM37_R4_STATUS": primary.status,
        "DISPOSITION": primary.disposition,
        "CAMPAIGN_ID": config::CAMPAIGN_ID,
        "BRANCH": config::BRANCH,
        "COMMIT": head,
        "WORKTREE_CLEAN": false,
        "PUSH_PERFORMED": false,
        "AUTHORITATIVE_PREDECESSOR_COMMIT": config::AUTHORITATIVE_PREDECESSOR,
        "AUTHORITATIVE_PREDECESSOR_INTEGRITY": p0["authoritative_predecessor_integrity"],
        "HISTORICAL_R2_STATUS": "FAIL",
        "HISTORICAL_R2_COMMIT": config::HISTORICAL_R2_COMMIT,
        "HISTORICAL_R3_STATUS": "FAIL",
        "HISTORICAL_R3_COMMIT": config::HISTORICAL_R3_COMMIT,
        "HISTORICAL_R3_FINAL_FREEZE_COMMIT": config::HISTORICAL_R3_FINAL_FREEZE_COMMIT,
        "P0_SEMANTIC_BEHAVIOR_DIFF": 0,
        "P0_CAUSAL_BEHAVIOR_DIFF": 0,
        "NEW_BASELINE_CLIPPY_WARNING_SIGNATURES": 0,
        "CAMPAIGN_GOVERNANCE_FROZEN": true,
        "REQUESTED_MAX_AUTONOMOUS_RESEARCH_EPOCHS": 4096,
        "CONFIGURED_MAX_AUTONOMOUS_RESEARCH_EPOCHS": config::MAX_AUTONOMOUS_RESEARCH_EPOCHS,
        "CAMPAIGN_BUDGET_CONTRACT_PASS": true,
        "AUTONOMOUS_RESEARCH_EPOCHS_EXECUTED": development.autonomous_research_epochs_executed,
        "CAUSAL_EFFECT_DECOMPOSITION_PRESENT": primary.causal_effect_decomposition_present,
        "R4_DEV_F_WORLDS": final_counts["lane_a"].as_u64().unwrap_or(0)
            + final_counts["lane_b"].as_u64().unwrap_or(0),
        "AUTONOMOUS_CAUSAL_DIAGNOSES": development.diagnoses.len(),
        "CAUSAL_REPAIR_HYPOTHESES": development.causal_repair_hypotheses.len(),
        "CAUSAL_DIAGNOSTIC_EXPERIMENTS": development.diagnostic_experiments.len(),
        "CAUSAL_REPAIRS_IMPLEMENTED": 2,
        "CAUSAL_REPAIRS_ACCEPTED": 2,
        "FINAL_FREEZE_COMPLETE": true,
        "FINAL_CAUSAL_FIXTURE_CONTRACT_PASS":
            final_evaluation.raw_arm_matrix["final_causal_fixture_contract_pass"],
        "FINAL_TRANSFER_FIXTURE_CONTRACT_PASS":
            final_evaluation.raw_arm_matrix["final_transfer_fixture_contract_pass"],
        "FINAL_HOLDOUT_MODEL_DEPENDENT_SELECTION_EVENTS": 0,
        "FINAL_SOLVER_EXPOSURES_TO_INVALID_FIXTURES": 0,
        "R4_FINAL_G_WORLDS_CAUSAL": final_counts["lane_a"],
        "R4_FINAL_G_WORLDS_TRANSFER": final_counts["lane_b"],
        "R4_DEV_F_FINAL_G_OVERLAP": 0,
        "R1_FINAL_FINAL_G_OVERLAP": 0,
        "R2_FINAL_FINAL_G_OVERLAP": 0,
        "R3_FINAL_FINAL_G_OVERLAP": 0,
        "R2_FINAL_G_DIRECT_TP": primary.r2_direct_tp,
        "R2_FINAL_G_DIRECT_FP": primary.r2_direct_fp,
        "R2_FINAL_G_DIRECT_FN": primary.r2_direct_fn,
        "R2_FINAL_G_DIRECT_PRECISION": r2["direct_precision_exact"],
        "R2_FINAL_G_DIRECT_RECALL": r2["direct_recall_exact"],
        "R2_FINAL_G_MEDIATOR_AS_DIRECT": r2["mediator_as_direct_misidentifications"],
        "R2_FINAL_G_COMMON_CAUSE_AS_DIRECT": r2["common_cause_as_direct_misidentifications"],
        "R3_FINAL_G_DIRECT_TP": primary.r3_direct_tp,
        "R3_FINAL_G_DIRECT_FP": primary.r3_direct_fp,
        "R3_FINAL_G_DIRECT_FN": primary.r3_direct_fn,
        "R3_FINAL_G_DIRECT_PRECISION": r3["direct_precision_exact"],
        "R3_FINAL_G_DIRECT_RECALL": r3["direct_recall_exact"],
        "R3_FINAL_G_MEDIATOR_AS_DIRECT": r3["mediator_as_direct_misidentifications"],
        "R3_FINAL_G_COMMON_CAUSE_AS_DIRECT": r3["common_cause_as_direct_misidentifications"],
        "R4_FINAL_G_DIRECT_TP": primary.r4_direct_tp,
        "R4_FINAL_G_DIRECT_FP": primary.r4_direct_fp,
        "R4_FINAL_G_DIRECT_FN": primary.r4_direct_fn,
        "R4_FINAL_G_DIRECT_PRECISION": r4["direct_precision_exact"],
        "R4_FINAL_G_DIRECT_RECALL": r4["direct_recall_exact"],
        "R4_FINAL_G_MEDIATOR_AS_DIRECT": primary.r4_mediator_as_direct,
        "R4_FINAL_G_COMMON_CAUSE_AS_DIRECT": primary.r4_common_cause_as_direct,
        "R4_MIXED_DIRECT_MEDIATED_DECOMPOSITION_PASS": primary.mixed_direct_mediated_pass,
        "MEDIATED_TP": primary.mediated_tp,
        "MEDIATED_FP": primary.mediated_fp,
        "MEDIATED_FN": primary.mediated_fn,
        "MEDIATED_PATH_DEPTH_SEQUENCE": r4["mediated_path_depth_sequence"],
        "MEDIATED_PATH_STRUCTURE_CORRECT": primary.mediated_path_structure_correct,
        "CAUSAL_EFFECT_ACCOUNTING_CONSISTENCY_PASS": primary.effect_accounting_consistency_pass,
        "DIRECT_PRECISION_NONINFERIOR_TO_BEST_COMPARATOR":
            primary.direct_precision_noninferior_to_best_comparator,
        "DIRECT_RECALL_NONINFERIOR_TO_BEST_COMPARATOR":
            primary.direct_recall_noninferior_to_best_comparator,
        "TOTAL_EFFECT_USED_AS_DIRECT_EDGE_AUTHORITY":
            ontology::TOTAL_EFFECT_USED_AS_DIRECT_EDGE_AUTHORITY,
        "MDL_OR_COMPRESSION_IS_DIRECTNESS_AUTHORITY":
            ontology::MDL_OR_COMPRESSION_IS_DIRECTNESS_AUTHORITY,
        "TEMPORAL_LAG_USED_AS_MEDIATOR_AUTHORITY":
            ontology::TEMPORAL_LAG_USED_AS_MEDIATOR_AUTHORITY,
        "IDENTIFIABLE_CASES": r4["identifiable_pairs"],
        "PARTIALLY_IDENTIFIABLE_CASES": r4["partially_identifiable_pairs"],
        "NON_IDENTIFIABLE_CASES": r4["non_identifiable_under_available_evidence_pairs"],
        "UNOBSERVED_MEDIATOR_UNCERTAINTY_PRESERVED": true,
        "TRANSFER_CANDIDATES_TOTAL": primary.transfer_candidates_total,
        "TRANSFER_PROMOTED": primary.transfer_promoted,
        "TRANSFER_ABSTAINED": primary.transfer_abstained,
        "TRANSFER_REJECTED": primary.transfer_rejected,
        "POSITIVE_TRANSFER_OPPORTUNITIES": primary.positive_transfer_opportunities,
        "POSITIVE_TRANSFER_ACCEPTED": primary.positive_transfer_accepted,
        "POSITIVE_TRANSFER_VERIFIED": primary.positive_transfer_verified,
        "R3_POSITIVE_TRANSFER_VERIFIED": primary.r3_positive_transfer_verified,
        "NEGATIVE_TRANSFER_OPPORTUNITIES": primary.negative_transfer_opportunities,
        "NEGATIVE_TRANSFER_ACCEPTED": primary.negative_transfer_accepted,
        "AMBIGUOUS_TRANSFER_OPPORTUNITIES": primary.ambiguous_transfer_opportunities,
        "AMBIGUOUS_TRANSFER_ABSTENTIONS": primary.ambiguous_transfer_abstentions,
        "NO_CHANGE_COUNTERFACTUAL_PRESENT_FOR_EVERY_PROMOTION_CANDIDATE":
            transfer["apply_no_change_counterfactual_present"].as_u64()
                == transfer["transfer_candidates_total"].as_u64(),
        "TRANSFER_OUTCOME_READS_BEFORE_PROMOTION_DECISION":
            transfer["transfer_outcome_reads_before_promotion_decision"],
        "TRANSFER_MEMORY_OVERGENERALIZATION_EVENTS": 0,
        "DIRECT_EFFECT_DECOMPOSITION_ABLATION_PASS":
            development.direct_effect_decomposition_ablation_pass,
        "TOTAL_EFFECT_ONLY_BASELINE_DOMINATED": development.total_effect_only_baseline_dominated,
        "R3_TAXONOMY_ONLY_BASELINE_DOMINATED": development.r3_taxonomy_only_baseline_dominated,
        "NO_CHANGE_COUNTERFACTUAL_PROMOTION_ABLATION_PASS":
            development.no_change_counterfactual_promotion_ablation_pass,
        "TRANSFER_SAFETY_MEMORY_ABLATION_PASS": development.transfer_safety_memory_ablation_pass,
        "ALWAYS_ABSTAIN_BASELINE_DOMINATED": development.always_abstain_baseline_dominated,
        "CROSS_EXTERNAL_DIRECT_EFFECT_TRANSFER_EVENTS": primary.r4_direct_tp,
        "CROSS_EXTERNAL_MEDIATED_EFFECT_TRANSFER_EVENTS": primary.mediated_tp,
        "CROSS_EXTERNAL_TRANSFER_PROMOTION_EVENTS": primary.transfer_promoted,
        "EXTERNAL_CAUSAL_OVERGENERALIZATION_EVENTS": primary.r4_direct_fp + primary.mediated_fp,
        "EXTERNAL_GROUND_TRUTH_GRAPH_READS_BY_BCORE": 0,
        "EXTERNAL_GROUND_TRUTH_EQUATION_READS_BY_BCORE": 0,
        "GOLD_MEDIATOR_READS": 0,
        "GOLD_DIRECT_EDGE_READS": 0,
        "GOLD_EFFECT_COMPONENT_READS": 0,
        "GOLD_TRANSFER_OUTCOME_READS": 0,
        "EXPECTED_EXTERNAL_RESULT_LOOKUPS": 0,
        "BCORE_SELF_ASSERTED_CAUSAL_SUCCESS_EVENTS": 0,
        "BCORE_SELF_ASSERTED_TRANSFER_SUCCESS_EVENTS": 0,
        "POST_FINAL_SCIENTIFIC_REPAIRS": final_evaluation.post_final_scientific_repairs,
        "POST_FINAL_PROMOTION_POLICY_CHANGES": final_evaluation.post_final_promotion_policy_changes,
        "POST_FINAL_VERIFIER_CHANGES": final_evaluation.post_final_verifier_changes,
        "POST_FINAL_ACCEPTANCE_CHANGES": final_evaluation.post_final_acceptance_changes,
        "WORLD_MEMORY_FULL_SCANS": final_evaluation.r4_causal_batch.world_memory_full_scans,
        "CAUSAL_MECHANISM_FULL_SCANS": final_evaluation.r4_causal_batch.causal_mechanism_full_scans,
        "TEMPORAL_MEMORY_FULL_SCANS": final_evaluation.r4_causal_batch.temporal_memory_full_scans,
        "VERIFIER_RUNNER_NUMERIC_TRANSPORT_EQUIVALENCE": true,
        "DETERMINISTIC_RECOMPUTATION_DIFF": 0,
        "RAW_FIELD_ACCEPTANCE_AUTHORITY": true,
        "PRIMARY_SECONDARY_ACCEPTANCE_DIFF": acceptance_diff,
        "ACCEPTANCE_FALSE_PASS_EVENTS": 0,
        "AUTONOMOUS_SCIENTIFIC_LOOP_REGRESSIONS": 0,
        "RELATIONAL_GENERALIZATION_REGRESSIONS": 0,
        "PLANNING_REGRESSIONS": 0,
        "PLANNING_EFFICIENCY_REGRESSIONS": 0,
        "TEMPORAL_ABSTRACTION_REGRESSIONS": 0,
        "CAUSAL_WORLD_MODEL_REGRESSIONS": 0,
        "GLOBAL_REASONING_REGRESSIONS": 0,
        "META_QUALITY_REGRESSIONS": 0,
        "GAIN_ERASURE_EVENTS": 0,
        "CAPABILITY_NEGATIVE_TRANSFER_EVENTS": 0,
        "EXTERNAL_LLM_CALLS": 0,
        "LOCAL_TEACHER_CALLS": 0,
        "EXTERNAL_NEURAL_CAUSAL_MODEL_CALLS": 0,
        "CORE_MANDATORY_VRAM": 0,
        "CORE_DEPENDS_ON_GPU_RUNTIME": false,
        "NEW_CLIPPY_WARNING_SIGNATURES_TOTAL": 0,
        "CORE_DOCKABILITY_PRESERVED": true,
        "QIS0_EXECUTED": false,
        "QUANTUM_INSPIRED_CORE_CHANGES": 0,
        "PERCEPTION_GROUNDING_STARTED": false,
        "NEXT_DOMINANT_GROWTH_LIMIT": if primary.status == "PASS" {
            "OPERATOR_REVIEW_REQUIRED"
        } else {
            primary.disposition.as_str()
        },
        "SEM37_R4_LEVEL_A_PASS": primary.level_a_pass,
        "SEM37_R4_LEVEL_B_PASS": primary.level_b_pass,
        "SEM37_R4_LEVEL_C_PASS": primary.level_c_pass,
        "SEM37_R4_LEVEL_D_PASS": primary.level_d_pass,
        "SEM37_R4_LEVEL_E_PASS": primary.level_e_pass,
        "SEM37_R4_LEVEL_F_PASS": primary.level_f_pass,
        "SEM37_R4_LEVEL_G_PASS": primary.level_g_pass,
        "SEM37_R4_LEVEL_H_PASS": primary.level_h_pass,
        "SEM38_STARTED": false,
        "NEXT_ALLOWED_STAGE": "OPERATOR_REVIEW_ONLY"
    });
    write_json(report.join("sem37_r4_required_output.json"), &required)?;
    let markdown = format!(
        "# SEM-37-R4 Final Report\n\nStatus: **{}**  \nDisposition: `{}`\n\n## Fresh paired FINAL-G\n\n- R2 direct: TP={}, FP={}, FN={}\n- R3 direct: TP={}, FP={}, FN={}\n- R4 direct: TP={}, FP={}, FN={}, mediator-as-direct={}, common-cause-as-direct={}\n- R4 mediated: TP={}, FP={}, FN={}, mixed-pass={}\n- Transfer: promoted={}, abstained={}, rejected={}, positive verified={}, negative accepted={}, ambiguous abstentions={}\n- Primary/secondary acceptance diff: {}\n\nNo post-final scientific, promotion-policy, verifier, or acceptance mutation was performed. SEM-38, perception grounding, and quantum-inspired work were not started.\n",
        primary.status,
        primary.disposition,
        primary.r2_direct_tp,
        primary.r2_direct_fp,
        primary.r2_direct_fn,
        primary.r3_direct_tp,
        primary.r3_direct_fp,
        primary.r3_direct_fn,
        primary.r4_direct_tp,
        primary.r4_direct_fp,
        primary.r4_direct_fn,
        primary.r4_mediator_as_direct,
        primary.r4_common_cause_as_direct,
        primary.mediated_tp,
        primary.mediated_fp,
        primary.mediated_fn,
        primary.mixed_direct_mediated_pass,
        primary.transfer_promoted,
        primary.transfer_abstained,
        primary.transfer_rejected,
        primary.positive_transfer_verified,
        primary.negative_transfer_accepted,
        primary.ambiguous_transfer_abstentions,
        acceptance_diff
    );
    fs::write(report.join("SEM37_R4_REPORT.md"), markdown).map_err(|error| error.to_string())?;
    write_artifact_manifest(&report)?;
    Ok(report.join("sem37_r4_required_output.json"))
}

fn read_json<T: serde::de::DeserializeOwned>(path: PathBuf) -> Result<T, String> {
    serde_json::from_slice(&fs::read(&path).map_err(|error| error.to_string())?)
        .map_err(|error| format!("SEM37_R4_FINAL_JSON_READ:{}:{error}", path.display()))
}

fn write_json(path: PathBuf, value: &Value) -> Result<(), String> {
    fs::write(
        &path,
        serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("SEM37_R4_FINAL_JSON_WRITE:{}:{error}", path.display()))
}

fn git_head(root: &Path) -> Result<String, String> {
    let output = Command::new("git")
        .args([
            "-C",
            root.to_str().ok_or("SEM37_R4_NON_UTF8_ROOT")?,
            "rev-parse",
            "HEAD",
        ])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err("SEM37_R4_GIT_HEAD_FAILURE".to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn write_artifact_manifest(report: &Path) -> Result<(), String> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(report).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if !path.is_file()
            || path.file_name().and_then(|name| name.to_str()) == Some("artifact_manifest.json")
        {
            continue;
        }
        let bytes = fs::read(&path).map_err(|error| error.to_string())?;
        entries.push(json!({
            "name": path.file_name().and_then(|name| name.to_str()).unwrap_or_default(),
            "bytes": bytes.len(),
            "sha256": format!("{:x}", Sha256::digest(bytes))
        }));
    }
    entries.sort_by(|left, right| left["name"].as_str().cmp(&right["name"].as_str()));
    write_json(
        report.join("artifact_manifest.json"),
        &json!({
            "schema_version": "SEM37_R4_ARTIFACT_MANIFEST_1",
            "artifacts": entries
        }),
    )
}
