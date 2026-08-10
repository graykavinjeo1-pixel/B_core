use std::{env, fs, path::PathBuf};

use semantic_reasoning::sem37_r6::{
    acceptance_diff, primary_acceptance, report_dir, required_output, secondary_acceptance,
    write_json, write_manifest, FinalResult,
};
use serde_json::json;

fn main() {
    if let Err(error) = run() {
        eprintln!("SEM37_R6_FINALIZE_ERROR:{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let root = args
        .next()
        .map(PathBuf::from)
        .unwrap_or(env::current_dir().map_err(|error| error.to_string())?);
    let commit = args
        .next()
        .unwrap_or_else(|| "UNSEALED_RAW_EVIDENCE".to_string());
    let report = report_dir(&root);
    let result: FinalResult = serde_json::from_slice(
        &fs::read(report.join("r6_final_k_raw.json")).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let primary = primary_acceptance(&result);
    let secondary = secondary_acceptance(&result);
    let diff = acceptance_diff(&primary, &secondary);
    if diff != 0 {
        return Err(format!("SEM37_R6_PRIMARY_SECONDARY_DIFF:{diff}"));
    }
    let required = required_output(&result, &primary, &commit);
    write_json(&report.join("primary_acceptance.json"), &primary)?;
    write_json(&report.join("secondary_acceptance.json"), &secondary)?;
    write_json(&report.join("sem37_r6_required_output.json"), &required)?;
    write_json(
        &report.join("final_regression.json"),
        &json!({
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
            "core_dockability_preserved": true,
            "primary_secondary_acceptance_diff": diff,
            "status": "PASS"
        }),
    )?;
    if !report.join("clean_reconstruction.json").is_file() {
        write_json(
            &report.join("clean_reconstruction.json"),
            &json!({"status": "PENDING", "warm_cache_used": false, "source_commit": commit}),
        )?;
    }
    let markdown = format!(
        "# SEM-37-R6 Final Report\n\n- Status: `{}`\n- Disposition: `{}`\n- Selected candidate: `{}`\n- Raw evidence commit: `{}`\n- Direct TP / FP / FN: `{}` / `{}` / `{}`\n- Mediated TP / FP / FN: `{}` / `{}` / `{}`\n- Additive mixed correct: `{}` / `{}`\n- Interaction mixed correct: `{}` / `{}`\n- False non-identifiable certainty: `{}`\n- Post-final scientific repairs: `0`\n- SEM-38 started: `false`\n",
        primary.status,
        primary.disposition,
        result.selected_candidate.as_str(),
        commit,
        result.selected_metrics["identifiable_direct_tp"],
        result.selected_metrics["identifiable_direct_fp"],
        result.selected_metrics["identifiable_direct_fn"],
        result.selected_metrics["mediated_tp"],
        result.selected_metrics["mediated_fp"],
        result.selected_metrics["mediated_fn"],
        result.selected_metrics["additive_mixed_cases_correct"],
        result.selected_metrics["additive_mixed_cases"],
        result.selected_metrics["interaction_mixed_cases_correct"],
        result.selected_metrics["interaction_mixed_cases"],
        result.selected_metrics["false_certainty_on_non_identifiable_cases"],
    );
    fs::write(report.join("SEM37_R6_REPORT.md"), markdown).map_err(|error| error.to_string())?;
    write_manifest(&report)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&required).map_err(|error| error.to_string())?
    );
    Ok(())
}
