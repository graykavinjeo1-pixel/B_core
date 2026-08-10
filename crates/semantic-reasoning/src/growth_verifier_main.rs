use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use semantic_reasoning::growth_supervisor::{run_verifier_request, VerifierRequest};

fn main() {
    if let Err(error) = run() {
        eprintln!("B_CORE_GROWTH_VERIFIER_ERROR:{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args_os().skip(1);
    let request_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "REQUEST_PATH_MISSING".to_string())?;
    let result_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "RESULT_PATH_MISSING".to_string())?;
    if args.next().is_some() {
        return Err("UNEXPECTED_ARGUMENT".to_string());
    }
    if result_path.exists() {
        return Err("IMMUTABLE_RESULT_ALREADY_EXISTS".to_string());
    }
    let request: VerifierRequest = serde_json::from_slice(
        &fs::read(&request_path)
            .map_err(|error| format!("REQUEST_READ:{}:{error}", request_path.display()))?,
    )
    .map_err(|error| format!("REQUEST_JSON:{error}"))?;
    let receipt = run_verifier_request(&request)?;
    let bytes = serde_json::to_vec_pretty(&receipt).map_err(|error| error.to_string())?;
    let parent = result_path
        .parent()
        .ok_or_else(|| "RESULT_PARENT_MISSING".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = parent.join(format!(".growth-verifier-{}.tmp", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| format!("RESULT_CREATE:{}:{error}", temporary.display()))?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("RESULT_WRITE:{}:{error}", temporary.display()))?;
    drop(file);
    fs::rename(&temporary, &result_path)
        .map_err(|error| format!("RESULT_RENAME:{}:{error}", result_path.display()))?;
    println!("{}", result_path.display());
    Ok(())
}
