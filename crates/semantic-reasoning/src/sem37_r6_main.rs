use std::{env, fs, path::PathBuf};

use semantic_reasoning::sem37_r6::{
    run_development, run_final, DevelopmentResult, ExternalEvaluatorClient,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("SEM37_R6_ERROR:{error}");
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
        .unwrap_or_else(|| PathBuf::from(r"D:\B_Core_SEM37_R6_EVALUATOR_VAULT"));
    let evaluator = ExternalEvaluatorClient::from_vault(&vault)?;
    let value = match phase.as_str() {
        "dev" => serde_json::to_value(run_development(&root, &evaluator)?),
        "final" => {
            let development: DevelopmentResult = serde_json::from_slice(
                &fs::read(root.join("reports/sem37-r6/development_result.json"))
                    .map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
            serde_json::to_value(run_final(&root, &evaluator, &development)?)
        }
        _ => return Err(format!("SEM37_R6_UNKNOWN_PHASE:{phase}")),
    }
    .map_err(|error| error.to_string())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?
    );
    Ok(())
}
