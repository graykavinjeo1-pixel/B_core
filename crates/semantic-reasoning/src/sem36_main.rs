use std::{env, path::PathBuf, process::ExitCode};

use semantic_reasoning::sem36::{
    audit_campaign, canonical_campaign, development_campaign, finalize_campaign, preflight_campaign,
};

fn main() -> ExitCode {
    let mut arguments = env::args().skip(1);
    let command = arguments.next().unwrap_or_else(|| "canonical".to_string());
    let root = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().expect("current directory"));
    let result = match command.as_str() {
        "develop" => development_campaign(&root),
        "preflight" => preflight_campaign(&root),
        "canonical" => canonical_campaign(&root),
        "finalize" => finalize_campaign(&root),
        "audit" => audit_campaign(&root),
        other => Err(format!("UNKNOWN_SEM36_COMMAND:{other}")),
    };
    match result {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("SEM36_ERROR={error}");
            ExitCode::FAILURE
        }
    }
}
