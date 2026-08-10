use std::{env, fs, path::PathBuf, process::ExitCode};

use semantic_reasoning::sem37::{
    acceptance::evaluate_secondary,
    campaign::{DevelopmentResearch, FinalExternalEvaluation},
};
use serde_json::Value;

fn main() -> ExitCode {
    let root = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().expect("current directory"));
    let report = root.join("reports/sem37");
    let result = (|| -> Result<String, String> {
        let baseline: Value = read(report.join("measured_external_gap.json"))?;
        let development: DevelopmentResearch =
            read(report.join("autonomous_external_research.json"))?;
        let final_raw: FinalExternalEvaluation =
            read(report.join("final_external_raw_evaluation.json"))?;
        let internal: Value = read(report.join("internal_world_regression_control.json"))?;
        let recorded: Value = read(report.join("secondary_acceptance.json"))?;
        let recomputed = evaluate_secondary(
            &baseline,
            &development,
            &final_raw,
            internal["SEM36_PRIMARY_SECONDARY_CONTROL_PASS"].as_bool() == Some(true),
        );
        let recomputed_value =
            serde_json::to_value(recomputed).map_err(|error| error.to_string())?;
        if recomputed_value != recorded {
            return Err("SEM37_INDEPENDENT_RECOMPUTATION_DIFF".to_string());
        }
        Ok("SEM37_INDEPENDENT_RAW_ACCEPTANCE_RECOMPUTATION_PASS".to_string())
    })();
    match result {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("SEM37_VERIFY_ERROR={error}");
            ExitCode::FAILURE
        }
    }
}

fn read<T: serde::de::DeserializeOwned>(path: PathBuf) -> Result<T, String> {
    serde_json::from_slice(&fs::read(path).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())
}
