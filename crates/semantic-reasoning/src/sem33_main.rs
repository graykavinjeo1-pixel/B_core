use std::{env, path::PathBuf, process::ExitCode};

use semantic_reasoning::sem33::{audit_campaign, finalize_campaign, freeze_campaign, run_campaign};

fn main() -> ExitCode {
    let mut arguments = env::args().skip(1);
    let command = arguments.next().unwrap_or_else(|| "run".into());
    let root = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().expect("current directory"));
    let result = match command.as_str() {
        "freeze" => freeze_campaign(&root),
        "run" => run_campaign(&root),
        "finalize" => finalize_campaign(&root),
        "audit" => audit_campaign(&root),
        other => Err(format!("UNKNOWN_SEM33_COMMAND:{other}")),
    };
    match result {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("SEM33_ERROR={error}");
            ExitCode::FAILURE
        }
    }
}
