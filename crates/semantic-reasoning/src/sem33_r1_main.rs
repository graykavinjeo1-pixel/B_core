use std::{env, path::PathBuf, process::ExitCode};

use semantic_reasoning::sem33_r1::{audit_campaign, finalize_campaign, p0_repair, run_campaign};

fn main() -> ExitCode {
    let mut arguments = env::args().skip(1);
    let command = arguments.next().unwrap_or_else(|| "run".into());
    let root = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().expect("current directory"));
    let result = match command.as_str() {
        "p0" => p0_repair(&root),
        "run" => run_campaign(&root),
        "finalize" => finalize_campaign(&root),
        "audit" => audit_campaign(&root),
        other => Err(format!("UNKNOWN_SEM33_R1_COMMAND:{other}")),
    };
    match result {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("SEM33_R1_ERROR={error}");
            ExitCode::FAILURE
        }
    }
}
