use std::{env, path::PathBuf, process::ExitCode};

use semantic_reasoning::sem32_r1::{audit_r1, finalize_r1, freeze_repair, run_regate};

fn main() -> ExitCode {
    let mut arguments = env::args().skip(1);
    let command = arguments.next().unwrap_or_else(|| "run".into());
    let root = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().expect("current directory"));
    let result = match command.as_str() {
        "freeze" => freeze_repair(&root),
        "run" => run_regate(&root),
        "finalize" => finalize_r1(&root),
        "audit" => audit_r1(&root),
        other => Err(format!("UNKNOWN_SEM32_R1_COMMAND:{other}")),
    };
    match result {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("SEM32_R1_ERROR={error}");
            ExitCode::FAILURE
        }
    }
}
