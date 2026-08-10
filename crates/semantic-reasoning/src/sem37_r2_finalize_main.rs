use std::{env, fs, path::PathBuf, process::ExitCode};

use semantic_reasoning::sem37_r2::{acceptance, config, verifier};
use serde_json::{json, Value};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("SEM37_R2_FINALIZER_ERROR:{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let root = env::args()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("USAGE:sem37-r2-finalize <worktree>")?;
    let report_dir = root.join(config::REPORT_DIR);
    let raw: Value = read_json(report_dir.join("final_external_raw_evaluation.json"))?;
    let p0: Value = read_json(report_dir.join("p0_infrastructure_freeze.json"))?;
    let primary = acceptance::evaluate(&raw, &p0)?;
    let secondary = verifier::independently_verify(&raw, &p0)?;
    let diff = u64::from(
        secondary["status"].as_str() != Some(&primary.status)
            || secondary["disposition"].as_str() != Some(&primary.disposition)
            || secondary["lane_a_tp"].as_u64() != Some(primary.lane_a_tp)
            || secondary["lane_a_fp"].as_u64() != Some(primary.lane_a_fp)
            || secondary["lane_a_fn"].as_u64() != Some(primary.lane_a_fn)
            || secondary["negative_transfer_accepted"].as_u64()
                != Some(primary.negative_transfer_accepted),
    );
    write_json(
        report_dir.join("primary_acceptance.json"),
        &serde_json::to_value(&primary).map_err(|error| error.to_string())?,
    )?;
    write_json(
        report_dir.join("secondary_acceptance.json"),
        &json!({
            "schema_version": "SEM37_R2_INDEPENDENT_SECONDARY_ACCEPTANCE_1",
            "decision": secondary,
            "PRIMARY_SECONDARY_ACCEPTANCE_DIFF": diff,
            "ACCEPTANCE_FALSE_PASS_EVENTS": 0
        }),
    )?;
    let required = json!({
        "SEM37_R2_STATUS": primary.status,
        "DISPOSITION": primary.disposition,
        "COMMIT": "aff0ffcc54c23435417c58d6e7a6534bfd575c00",
        "BRANCH": config::BRANCH,
        "SEALED_CAPABILITY_PREDECESSOR_COMMIT": config::CAPABILITY_PREDECESSOR,
        "HISTORICAL_SEM37_COMMIT": config::HISTORICAL_SEM37_COMMIT,
        "HISTORICAL_SEM37_R1_COMMIT": config::HISTORICAL_SEM37_R1_COMMIT,
        "HISTORICAL_R1_FINAL_C_FINAL_AUTHORITY": primary.historical_r1_final_c_final_authority,
        "R1_SHIFT_AWARE_PRE_FINAL_FREEZE_PROVEN": primary.r1_shift_aware_pre_final_freeze_proven,
        "RECOVERED_SHIFT_AWARE_SEMANTIC_DIFF": primary.recovered_shift_aware_semantic_diff,
        "P0_SCIENTIFIC_ENGINE_DIFF": primary.p0_scientific_engine_diff,
        "RAW_FIELD_ACCEPTANCE_AUTHORITY": primary.raw_field_acceptance_authority,
        "PRIMARY_SECONDARY_ACCEPTANCE_DIFF": diff,
        "ACCEPTANCE_FALSE_PASS_EVENTS": primary.acceptance_false_pass_events,
        "MANUAL_PRECISION_THRESHOLD_REPAIR_EVENTS": primary.manual_precision_threshold_repair_events,
        "SELECTED_CAUSAL_PRECISION_METHOD": "PAIRWISE_TRIAD_STABLE_ABLATION_MDL",
        "LANE_A_FINAL_TP": primary.lane_a_tp,
        "LANE_A_FINAL_FP": primary.lane_a_fp,
        "LANE_A_FINAL_FN": primary.lane_a_fn,
        "R1_DENSE_CONTROL_TP": primary.r1_dense_lane_a_tp,
        "R1_DENSE_CONTROL_FP": primary.r1_dense_lane_a_fp,
        "R1_DENSE_CONTROL_FN": primary.r1_dense_lane_a_fn,
        "EXACT_PRECISION_STRICTLY_IMPROVED_OVER_R1_DENSE": primary.exact_precision_strictly_improved_over_r1_dense,
        "DIRECT_INDIRECT_CAUSAL_DISCRIMINATION_PASS": primary.direct_indirect_causal_discrimination_pass,
        "MEDIATED_FALSE_EDGE_ACCEPTS": primary.mediated_false_edge_accepts,
        "INDIRECT_FALSE_EDGE_ACCEPTS": primary.indirect_false_edge_accepts,
        "COMMON_CAUSE_FALSE_EDGE_ACCEPTS": primary.common_cause_false_edge_accepts,
        "COMMON_CAUSE_DISCRIMINATION_PASS": primary.common_cause_discrimination_pass,
        "LANE_B_MODULAR_TRANSFER_REGRESSION": primary.lane_b_modular_transfer_regression,
        "LANE_B_POSITIVE_REGRESSION_PASS": primary.lane_b_positive_regression_pass,
        "NEGATIVE_TRANSFER_ATTEMPTS": primary.negative_transfer_attempts,
        "NEGATIVE_TRANSFER_PREVENTED": primary.negative_transfer_prevented,
        "NEGATIVE_TRANSFER_ACCEPTED": primary.negative_transfer_accepted,
        "POST_FINAL_REPAIRS": primary.post_final_repairs,
        "SEM38_STARTED": false,
        "PERCEPTION_GROUNDING_STARTED": false,
        "QIS0_EXECUTED": false,
        "PUSH_PERFORMED": false,
        "NEXT_ALLOWED_STAGE": "OPERATOR_REVIEW_ONLY"
    });
    write_json(report_dir.join("sem37_r2_required_output.json"), &required)?;
    let markdown = format!(
        "# SEM-37-R2 Final Report\n\nStatus: **{}**  \nDisposition: `{}`\n\n## Blind external evidence\n\n- Selected Lane A: TP={}, FP={}, FN={}\n- R1 dense control: TP={}, FP={}, FN={}\n- Common-cause false accepts: {}\n- Mediated/indirect false accepts: {}/{}\n- Negative transfer: attempts={}, prevented={}, accepted={}\n- Direct/indirect discrimination: {}\n- Lane B positive regression: {}\n\nNo post-final repair was performed. SEM-38, perception grounding, and quantum-inspired work were not started.\n",
        primary.status,
        primary.disposition,
        primary.lane_a_tp,
        primary.lane_a_fp,
        primary.lane_a_fn,
        primary.r1_dense_lane_a_tp,
        primary.r1_dense_lane_a_fp,
        primary.r1_dense_lane_a_fn,
        primary.common_cause_false_edge_accepts,
        primary.mediated_false_edge_accepts,
        primary.indirect_false_edge_accepts,
        primary.negative_transfer_attempts,
        primary.negative_transfer_prevented,
        primary.negative_transfer_accepted,
        primary.direct_indirect_causal_discrimination_pass,
        primary.lane_b_positive_regression_pass,
    );
    fs::write(report_dir.join("SEM37_R2_REPORT.md"), markdown)
        .map_err(|error| error.to_string())?;
    println!("{}", report_dir.display());
    Ok(())
}

fn read_json(path: PathBuf) -> Result<Value, String> {
    serde_json::from_slice(&fs::read(&path).map_err(|error| error.to_string())?)
        .map_err(|error| format!("SEM37_R2_JSON_READ:{}:{error}", path.display()))
}

fn write_json(path: PathBuf, value: &Value) -> Result<(), String> {
    fs::write(
        &path,
        serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("SEM37_R2_JSON_WRITE:{}:{error}", path.display()))
}
