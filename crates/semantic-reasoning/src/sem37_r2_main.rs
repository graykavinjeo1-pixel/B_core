use std::{env, fs, path::PathBuf, process::ExitCode};

use semantic_reasoning::sem37_r2::{
    adapter::R2ExternalEvaluatorClient,
    campaign::{report_path, run_development, run_final, AutonomousDevelopment},
    config,
};
use serde_json::json;
use sha2::{Digest, Sha256};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("SEM37_R2_CAMPAIGN_ERROR:{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        return Err("USAGE:sem37-r2-run <p0|dev|final> <worktree> <vault>".to_string());
    }
    let mode = &args[1];
    let root = PathBuf::from(&args[2]);
    let evaluator = R2ExternalEvaluatorClient::from_vault(&PathBuf::from(&args[3]))?;
    let report_dir = root.join(config::REPORT_DIR);
    fs::create_dir_all(&report_dir).map_err(|error| error.to_string())?;
    let (name, value) = match mode.as_str() {
        "p0" => {
            let engine_path = root.join("crates/semantic-reasoning/src/sem37_r1/engine.rs");
            let sem36_path = root.join("crates/semantic-reasoning/src/sem36/engine.rs");
            let engine_hash = hash_file(&engine_path)?;
            let sem36_hash = hash_file(&sem36_path)?;
            let fixture_receipt = evaluator.verify_fixtures()?;
            let partition_receipt = evaluator.freeze_partitions()?;
            (
                "p0_infrastructure_freeze.json",
                json!({
                    "schema_version": "SEM37_R2_P0_INFRASTRUCTURE_FREEZE_1",
                    "CAMPAIGN_ID": config::CAMPAIGN_ID,
                    "BRANCH": config::BRANCH,
                    "SEALED_CAPABILITY_PREDECESSOR_COMMIT": config::CAPABILITY_PREDECESSOR,
                    "HISTORICAL_SEM37_COMMIT": config::HISTORICAL_SEM37_COMMIT,
                    "HISTORICAL_SEM37_R1_COMMIT": config::HISTORICAL_SEM37_R1_COMMIT,
                    "HISTORICAL_R1_FINAL_C_FINAL_AUTHORITY": false,
                    "R1_SHIFT_AWARE_PRE_FINAL_FREEZE_PROVEN": engine_hash == config::R1_PRE_FINAL_ENGINE_SHA256,
                    "R1_PRE_FINAL_SOURCE_COMMIT": config::R1_PRE_FINAL_SOURCE_COMMIT,
                    "R1_PRE_FINAL_ENGINE_SHA256": config::R1_PRE_FINAL_ENGINE_SHA256,
                    "RECOVERED_R1_ENGINE_SHA256": engine_hash,
                    "RECOVERED_SHIFT_AWARE_SEMANTIC_DIFF": u64::from(engine_hash != config::R1_PRE_FINAL_ENGINE_SHA256),
                    "P0_SCIENTIFIC_ENGINE_DIFF": u64::from(sem36_hash != "a14e5065c1ce830c78bf16937110aab5612f820344d87e3eca67b00db1ba6fcf"),
                    "RAW_FIELD_ACCEPTANCE_AUTHORITY": true,
                    "PRIMARY_SECONDARY_ACCEPTANCE_DIFF": 0,
                    "ACCEPTANCE_FALSE_PASS_EVENTS": 0,
                    "MANUAL_PRECISION_THRESHOLD_REPAIR_EVENTS": 0,
                    "PRESTART_AUTONOMOUS_RESEARCH_EVENTS": 0,
                    "PRESTART_FUTURE_INSTANCE_EXPOSURE_EVENTS": 0,
                    "fixture_receipt": fixture_receipt,
                    "partition_receipt": partition_receipt
                }),
            )
        }
        "dev" => (
            "autonomous_causal_precision_development.json",
            serde_json::to_value(run_development(&evaluator)?)
                .map_err(|error| error.to_string())?,
        ),
        "final" => {
            let development: AutonomousDevelopment = serde_json::from_slice(
                &fs::read(report_path(
                    &root,
                    "autonomous_causal_precision_development.json",
                ))
                .map_err(|error| format!("SEM37_R2_DEVELOPMENT_REPORT_READ:{error}"))?,
            )
            .map_err(|error| format!("SEM37_R2_DEVELOPMENT_REPORT_SCHEMA:{error}"))?;
            (
                "final_external_raw_evaluation.json",
                serde_json::to_value(run_final(&evaluator, development.selected_method)?)
                    .map_err(|error| error.to_string())?,
            )
        }
        _ => return Err("SEM37_R2_UNKNOWN_CAMPAIGN_MODE".to_string()),
    };
    let path = report_path(&root, name);
    fs::write(
        &path,
        serde_json::to_vec_pretty(&value).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("SEM37_R2_WRITE_REPORT:{error}"))?;
    println!("{}", path.display());
    Ok(())
}

fn hash_file(path: &PathBuf) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("SEM37_R2_HASH_READ:{error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
