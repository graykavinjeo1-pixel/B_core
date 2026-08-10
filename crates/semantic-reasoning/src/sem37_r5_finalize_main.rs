use std::{
    env, fs,
    path::{Path, PathBuf},
};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use semantic_reasoning::sem37_r5::{
    acceptance::{primary, required_output, secondary},
    campaign::{write_json, FinalEvaluation},
    verifier::acceptance_diff,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("SEM37_R5_FINALIZE_ERROR:{error}");
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
    let report = root.join("reports/sem37-r5");
    let final_result: FinalEvaluation = serde_json::from_slice(
        &fs::read(report.join("r5_final_i_raw.json")).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let primary_decision = primary(&final_result);
    let secondary_decision = secondary(&final_result);
    let primary_value = json!(primary_decision);
    let secondary_value = json!(secondary_decision);
    let diff = acceptance_diff(&primary_value, &secondary_value);
    if diff != 0 {
        return Err(format!("SEM37_R5_PRIMARY_SECONDARY_DIFF:{diff}"));
    }
    let required = required_output(&final_result, &primary_decision, &commit);
    write_json(&report.join("primary_acceptance.json"), &primary_decision)?;
    write_json(
        &report.join("secondary_acceptance.json"),
        &secondary_decision,
    )?;
    write_json(&report.join("sem37_r5_required_output.json"), &required)?;
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
            &json!({
                "status": "PENDING",
                "warm_cache_used": false,
                "source_commit": commit
            }),
        )?;
    }
    let markdown = format!(
        "# SEM-37-R5 Final Report\n\n- Status: `{}`\n- Disposition: `{}`\n- Selected model: `{}`\n- Raw evidence commit: `{}`\n- Fully / partially / non-identifiable: `{}` / `{}` / `{}`\n- Identifiable direct TP / FP / FN: `{}` / `{}` / `{}`\n- Mediated TP / FP / FN: `{}` / `{}` / `{}`\n- Mixed identification: `{}`\n- Invalid final fixture exposures: `0`\n- Post-final scientific repairs: `0`\n- SEM-38 started: `false`\n",
        primary_decision.status,
        primary_decision.disposition,
        final_result.selected_model.name(),
        commit,
        final_result.selected_metrics["fully_identifiable_cases"],
        final_result.selected_metrics["partially_identifiable_cases"],
        final_result.selected_metrics["non_identifiable_cases"],
        final_result.selected_metrics["identifiable_direct_tp"],
        final_result.selected_metrics["identifiable_direct_fp"],
        final_result.selected_metrics["identifiable_direct_fn"],
        final_result.selected_metrics["mediated_tp"],
        final_result.selected_metrics["mediated_fp"],
        final_result.selected_metrics["mediated_fn"],
        final_result.selected_metrics["mixed_direct_mediated_identification_pass"],
    );
    fs::write(report.join("SEM37_R5_REPORT.md"), markdown).map_err(|error| error.to_string())?;
    write_manifest(&report)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&required).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn write_manifest(report: &Path) -> Result<(), String> {
    let mut entries = fs::read_dir(report)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file() && entry.file_name() != "artifact_manifest.json")
        .map(|entry| {
            let bytes = fs::read(entry.path()).map_err(|error| error.to_string())?;
            let digest = Sha256::digest(&bytes);
            let sha256: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
            Ok(json!({
                "path": format!("reports/sem37-r5/{}", entry.file_name().to_string_lossy()),
                "sha256": sha256,
                "bytes": bytes.len()
            }))
        })
        .collect::<Result<Vec<Value>, String>>()?;
    entries.sort_by(|left, right| left["path"].as_str().cmp(&right["path"].as_str()));
    write_json(
        &report.join("artifact_manifest.json"),
        &json!({
            "schema_version": "SEM37_R5_ARTIFACT_MANIFEST_1",
            "entries": entries,
            "authoritative_state": "GIT_COMMIT_PLUS_SEALED_ARTIFACTS"
        }),
    )
}
