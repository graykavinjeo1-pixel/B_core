use std::env;
use std::path::PathBuf;

use semantic_reasoning::compound_growth::{
    run_compound_growth_cycle, CompoundGrowthCycleRequestIR, CompoundGrowthInputIR,
};
use semantic_reasoning::growth_supervisor::{
    cleanup_source_staging, compound_growth_status, continue_lineage, initialize, make_config,
    preview_source_repair, record_compound_growth_input, record_repository_issue,
    record_work_event, request_resume, request_stop, run_daemon, self_check, status,
    supervisor_step, WorkEvent,
};
use semantic_reasoning::repository_experience::RepositoryIssueIntakeRequestIR;

const MAX_COMPOUND_INPUT_BYTES: u64 = 16 * 1024 * 1024;
const MAX_REPOSITORY_ISSUE_INPUT_BYTES: u64 = 1024 * 1024;

fn main() {
    if let Err(error) = run() {
        eprintln!("B_CORE_GROWTH_SUPERVISOR_ERROR:{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args_os().skip(1);
    let operation = args
        .next()
        .and_then(|value| value.into_string().ok())
        .ok_or_else(usage)?;
    match operation.as_str() {
        "self-check" => {
            ensure_no_more(&mut args)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&self_check()).map_err(|e| e.to_string())?
            );
        }
        "make-config" => {
            let config = next_path(&mut args, "CONFIG_PATH_MISSING")?;
            let watch_root = next_path(&mut args, "WATCH_ROOT_MISSING")?;
            let state_dir = next_path(&mut args, "STATE_DIR_MISSING")?;
            ensure_no_more(&mut args)?;
            let created = make_config(&config, &watch_root, &state_dir)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&created).map_err(|e| e.to_string())?
            );
        }
        "init" => {
            let config = next_path(&mut args, "CONFIG_PATH_MISSING")?;
            ensure_no_more(&mut args)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&initialize(&config)?).map_err(|e| e.to_string())?
            );
        }
        "continue-lineage" => {
            let predecessor = next_path(&mut args, "PREDECESSOR_CONFIG_PATH_MISSING")?;
            let successor = next_path(&mut args, "SUCCESSOR_CONFIG_PATH_MISSING")?;
            ensure_no_more(&mut args)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&continue_lineage(&predecessor, &successor)?)
                    .map_err(|e| e.to_string())?
            );
        }
        "step" => {
            let config = next_path(&mut args, "CONFIG_PATH_MISSING")?;
            ensure_no_more(&mut args)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&supervisor_step(&config)?)
                    .map_err(|e| e.to_string())?
            );
        }
        "run" => {
            let config = next_path(&mut args, "CONFIG_PATH_MISSING")?;
            ensure_no_more(&mut args)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&run_daemon(&config)?).map_err(|e| e.to_string())?
            );
        }
        "status" => {
            let config = next_path(&mut args, "CONFIG_PATH_MISSING")?;
            ensure_no_more(&mut args)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&status(&config)?).map_err(|e| e.to_string())?
            );
        }
        "cleanup-source-staging" => {
            let config = next_path(&mut args, "CONFIG_PATH_MISSING")?;
            ensure_no_more(&mut args)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&cleanup_source_staging(&config)?)
                    .map_err(|e| e.to_string())?
            );
        }
        "preview-source-repair" => {
            let config = next_path(&mut args, "CONFIG_PATH_MISSING")?;
            ensure_no_more(&mut args)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&preview_source_repair(&config)?)
                    .map_err(|e| e.to_string())?
            );
        }
        "stop" => {
            let config = next_path(&mut args, "CONFIG_PATH_MISSING")?;
            ensure_no_more(&mut args)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&request_stop(&config)?).map_err(|e| e.to_string())?
            );
        }
        "resume" => {
            let config = next_path(&mut args, "CONFIG_PATH_MISSING")?;
            ensure_no_more(&mut args)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&request_resume(&config)?)
                    .map_err(|e| e.to_string())?
            );
        }
        "record-event" => {
            let config = next_path(&mut args, "CONFIG_PATH_MISSING")?;
            let event_path = next_path(&mut args, "EVENT_PATH_MISSING")?;
            ensure_no_more(&mut args)?;
            let bytes = std::fs::read(&event_path)
                .map_err(|e| format!("EVENT_READ:{}:{e}", event_path.display()))?;
            let event: WorkEvent =
                serde_json::from_slice(&bytes).map_err(|e| format!("EVENT_JSON:{e}"))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&record_work_event(&config, event)?)
                    .map_err(|e| e.to_string())?
            );
        }
        "record-repository-issue" => {
            let config = next_path(&mut args, "CONFIG_PATH_MISSING")?;
            let request_path = next_path(&mut args, "REPOSITORY_ISSUE_PATH_MISSING")?;
            ensure_no_more(&mut args)?;
            let metadata = std::fs::symlink_metadata(&request_path)
                .map_err(|e| format!("REPOSITORY_ISSUE_METADATA:{}:{e}", request_path.display()))?;
            if !metadata.file_type().is_file()
                || metadata.file_type().is_symlink()
                || metadata.len() > MAX_REPOSITORY_ISSUE_INPUT_BYTES
            {
                return Err(format!(
                    "REPOSITORY_ISSUE_NOT_BOUNDED_REGULAR_FILE:{}",
                    request_path.display()
                ));
            }
            let bytes = std::fs::read(&request_path)
                .map_err(|e| format!("REPOSITORY_ISSUE_READ:{}:{e}", request_path.display()))?;
            let request: RepositoryIssueIntakeRequestIR =
                serde_json::from_slice(&bytes).map_err(|e| format!("REPOSITORY_ISSUE_JSON:{e}"))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&record_repository_issue(&config, request)?)
                    .map_err(|e| e.to_string())?
            );
        }
        "record-compound-input" => {
            let config = next_path(&mut args, "CONFIG_PATH_MISSING")?;
            let input_path = next_path(&mut args, "COMPOUND_INPUT_PATH_MISSING")?;
            ensure_no_more(&mut args)?;
            let metadata = std::fs::symlink_metadata(&input_path)
                .map_err(|e| format!("COMPOUND_INPUT_METADATA:{}:{e}", input_path.display()))?;
            if !metadata.file_type().is_file()
                || metadata.file_type().is_symlink()
                || metadata.len() > MAX_COMPOUND_INPUT_BYTES
            {
                return Err(format!(
                    "COMPOUND_INPUT_NOT_BOUNDED_REGULAR_FILE:{}",
                    input_path.display()
                ));
            }
            let bytes = std::fs::read(&input_path)
                .map_err(|e| format!("COMPOUND_INPUT_READ:{}:{e}", input_path.display()))?;
            if bytes.len() as u64 > MAX_COMPOUND_INPUT_BYTES {
                return Err("COMPOUND_INPUT_TOO_LARGE".to_string());
            }
            let input: CompoundGrowthInputIR =
                serde_json::from_slice(&bytes).map_err(|e| format!("COMPOUND_INPUT_JSON:{e}"))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&record_compound_growth_input(&config, input)?)
                    .map_err(|e| e.to_string())?
            );
        }
        "compound-status" => {
            let config = next_path(&mut args, "CONFIG_PATH_MISSING")?;
            ensure_no_more(&mut args)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&compound_growth_status(&config)?)
                    .map_err(|e| e.to_string())?
            );
        }
        "compound-cycle" => {
            let request_path = next_path(&mut args, "COMPOUND_REQUEST_PATH_MISSING")?;
            ensure_no_more(&mut args)?;
            let bytes = std::fs::read(&request_path)
                .map_err(|e| format!("COMPOUND_REQUEST_READ:{}:{e}", request_path.display()))?;
            let request: CompoundGrowthCycleRequestIR =
                serde_json::from_slice(&bytes).map_err(|e| format!("COMPOUND_REQUEST_JSON:{e}"))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&run_compound_growth_cycle(&request)?)
                    .map_err(|e| e.to_string())?
            );
        }
        _ => return Err(usage()),
    }
    Ok(())
}

fn next_path(
    args: &mut impl Iterator<Item = std::ffi::OsString>,
    error: &str,
) -> Result<PathBuf, String> {
    args.next()
        .map(PathBuf::from)
        .ok_or_else(|| error.to_string())
}

fn ensure_no_more(args: &mut impl Iterator<Item = std::ffi::OsString>) -> Result<(), String> {
    if args.next().is_some() {
        Err("UNEXPECTED_ARGUMENT".to_string())
    } else {
        Ok(())
    }
}

fn usage() -> String {
    "USAGE:<self-check|make-config|init|continue-lineage|step|run|status|cleanup-source-staging|preview-source-repair|stop|resume|record-event|record-repository-issue|record-compound-input|compound-status|compound-cycle> ...".to_string()
}
