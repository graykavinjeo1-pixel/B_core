use std::env;
use std::fs;
use std::path::PathBuf;

use semantic_reasoning::integrated_development::{
    run_integrated_development_epoch, IntegratedDevelopmentEpochRequest,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("B_CORE_INTEGRATED_DEVELOPMENT_ERROR:{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args_os().skip(1);
    let request_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "USAGE:<request.json> <result.json>".to_string())?;
    let result_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "USAGE:<request.json> <result.json>".to_string())?;
    if args.next().is_some() {
        return Err("UNEXPECTED_ARGUMENT".to_string());
    }
    if request_path == result_path {
        return Err("REQUEST_AND_RESULT_PATH_COLLISION".to_string());
    }

    let request_bytes = fs::read(&request_path)
        .map_err(|error| format!("REQUEST_READ:{}:{error}", request_path.display()))?;
    let request: IntegratedDevelopmentEpochRequest =
        serde_json::from_slice(&request_bytes).map_err(|error| format!("REQUEST_JSON:{error}"))?;
    let result = run_integrated_development_epoch(request)?;
    let result_bytes =
        serde_json::to_vec_pretty(&result).map_err(|error| format!("RESULT_JSON:{error}"))?;

    let parent = result_path
        .parent()
        .ok_or_else(|| "RESULT_PARENT_MISSING".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("RESULT_PARENT_CREATE:{error}"))?;
    let temporary = result_path.with_extension("json.tmp");
    fs::write(&temporary, result_bytes)
        .map_err(|error| format!("RESULT_TEMP_WRITE:{}:{error}", temporary.display()))?;
    fs::rename(&temporary, &result_path)
        .map_err(|error| format!("RESULT_ATOMIC_RENAME:{}:{error}", result_path.display()))?;
    println!("{}", result_path.display());
    Ok(())
}
