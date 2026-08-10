use std::{env, fs, path::PathBuf};

use semantic_reasoning::sem37_r5::{
    adapter::ExternalEvaluatorClient,
    campaign::{run_development, run_final, AutonomousDevelopment},
};

fn main() {
    if let Err(error) = run() {
        eprintln!("SEM37_R5_ERROR:{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let phase = args.next().unwrap_or_else(|| "dev".to_string());
    let root = args
        .next()
        .map(PathBuf::from)
        .unwrap_or(env::current_dir().map_err(|error| error.to_string())?);
    let vault = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"D:\B_Core_SEM37_R5_EVALUATOR_VAULT"));
    let evaluator = ExternalEvaluatorClient::from_vault(&vault)?;
    match phase.as_str() {
        "dev" => {
            let result = run_development(&root, &evaluator)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&result).map_err(|error| error.to_string())?
            );
        }
        "final" => {
            let path = root.join("reports/sem37-r5/development_result.json");
            let development: AutonomousDevelopment =
                serde_json::from_slice(&fs::read(path).map_err(|error| error.to_string())?)
                    .map_err(|error| error.to_string())?;
            let result = run_final(&root, &evaluator, &development)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&result).map_err(|error| error.to_string())?
            );
        }
        _ => return Err(format!("SEM37_R5_UNKNOWN_PHASE:{phase}")),
    }
    Ok(())
}
