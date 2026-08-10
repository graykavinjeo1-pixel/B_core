use std::{env, fs, path::PathBuf, process::ExitCode};

use semantic_reasoning::sem37_r1::{
    adapter::R1ExternalEvaluatorClient,
    campaign::{report_path, run_development_research, run_final_external_evaluation},
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("SEM37_R1_CAMPAIGN_ERROR:{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        return Err("USAGE:sem37-r1-campaign <dev|final> <worktree> <vault>".to_string());
    }
    let mode = &args[1];
    let root = PathBuf::from(&args[2]);
    let evaluator = R1ExternalEvaluatorClient::from_vault(&PathBuf::from(&args[3]))?;
    let (name, value) = match mode.as_str() {
        "dev" => (
            "autonomous_shift_aware_development.json",
            serde_json::to_value(run_development_research(&evaluator)?)
                .map_err(|error| error.to_string())?,
        ),
        "final" => (
            "final_external_raw_evaluation.json",
            serde_json::to_value(run_final_external_evaluation(&evaluator)?)
                .map_err(|error| error.to_string())?,
        ),
        _ => return Err("SEM37_R1_UNKNOWN_CAMPAIGN_MODE".to_string()),
    };
    let path = report_path(&root, name);
    let bytes = serde_json::to_vec_pretty(&value).map_err(|error| error.to_string())?;
    fs::write(&path, bytes).map_err(|error| format!("SEM37_R1_WRITE_REPORT:{error}"))?;
    println!("{}", path.display());
    Ok(())
}
