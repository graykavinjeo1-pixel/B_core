use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use semantic_reasoning::self_healing_pipeline::{
    begin_operator_teaching, ingest_operator_teaching, ingest_provisional_teaching,
    run_provisional_transfer_request, run_self_healing_request, BeginOperatorTeachingRequest,
    ProvisionalPromotionRequest, ProvisionalTransferRunnerRequest, RepairLearningPromotionRequest,
    SelfHealingRunnerRequest,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("B_CORE_SELF_HEALING_ERROR:{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args_os().skip(1);
    let operation = args
        .next()
        .and_then(|value| value.into_string().ok())
        .ok_or_else(|| {
            "USAGE:<--attempt|--begin-teaching|--attempt-transfer|--promote-provisional|--promote> <request.json> <immutable-result.json>".to_string()
        })?;
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
    if request_path == result_path {
        return Err("REQUEST_AND_RESULT_PATH_COLLISION".to_string());
    }
    if result_path.exists() {
        return Err("IMMUTABLE_RESULT_ALREADY_EXISTS".to_string());
    }

    let request_bytes = fs::read(&request_path)
        .map_err(|error| format!("REQUEST_READ:{}:{error}", request_path.display()))?;
    let result_bytes = match operation.as_str() {
        "--attempt" => {
            let request: SelfHealingRunnerRequest = serde_json::from_slice(&request_bytes)
                .map_err(|error| format!("REQUEST_JSON:{error}"))?;
            serde_json::to_vec_pretty(&run_self_healing_request(request)?)
                .map_err(|error| format!("RESULT_JSON:{error}"))?
        }
        "--begin-teaching" => {
            let request: BeginOperatorTeachingRequest = serde_json::from_slice(&request_bytes)
                .map_err(|error| format!("REQUEST_JSON:{error}"))?;
            serde_json::to_vec_pretty(
                &begin_operator_teaching(request.lesson, &request.teaching)
                    .map_err(|error| format!("LESSON_BEGIN:{error:?}"))?,
            )
            .map_err(|error| format!("RESULT_JSON:{error}"))?
        }
        "--attempt-transfer" => {
            let request: ProvisionalTransferRunnerRequest = serde_json::from_slice(&request_bytes)
                .map_err(|error| format!("REQUEST_JSON:{error}"))?;
            serde_json::to_vec_pretty(&run_provisional_transfer_request(request)?)
                .map_err(|error| format!("RESULT_JSON:{error}"))?
        }
        "--promote-provisional" => {
            let request: ProvisionalPromotionRequest = serde_json::from_slice(&request_bytes)
                .map_err(|error| format!("REQUEST_JSON:{error}"))?;
            serde_json::to_vec_pretty(&ingest_provisional_teaching(request)?)
                .map_err(|error| format!("RESULT_JSON:{error}"))?
        }
        "--promote" => {
            let request: RepairLearningPromotionRequest = serde_json::from_slice(&request_bytes)
                .map_err(|error| format!("REQUEST_JSON:{error}"))?;
            serde_json::to_vec_pretty(&ingest_operator_teaching(request)?)
                .map_err(|error| format!("RESULT_JSON:{error}"))?
        }
        _ => return Err("UNKNOWN_OPERATION".to_string()),
    };
    write_immutable_atomic(&result_path, &result_bytes)?;
    println!("{}", result_path.display());
    Ok(())
}

fn write_immutable_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "RESULT_PARENT_MISSING".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("RESULT_PARENT_CREATE:{error}"))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "RESULT_FILE_NAME_INVALID".to_string())?;
    let temporary = parent.join(format!(".{file_name}.{}.tmp", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| format!("RESULT_TEMP_CREATE:{}:{error}", temporary.display()))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("RESULT_TEMP_WRITE:{}:{error}", temporary.display()))?;
    drop(file);
    if path.exists() {
        let _ = fs::remove_file(&temporary);
        return Err("IMMUTABLE_RESULT_RACE".to_string());
    }
    fs::rename(&temporary, path)
        .map_err(|error| format!("RESULT_ATOMIC_RENAME:{}:{error}", path.display()))?;
    Ok(())
}
