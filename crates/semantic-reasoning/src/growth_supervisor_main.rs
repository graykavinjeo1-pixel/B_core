use std::env;
use std::path::PathBuf;

use semantic_reasoning::growth_supervisor::{
    initialize, make_config, record_work_event, request_resume, request_stop, run_daemon,
    self_check, status, supervisor_step, WorkEvent,
};

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
    "USAGE:<self-check|make-config|init|step|run|status|stop|resume|record-event> ...".to_string()
}
