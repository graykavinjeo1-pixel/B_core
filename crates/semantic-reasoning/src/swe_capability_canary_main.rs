use std::path::PathBuf;

use semantic_reasoning::benchmark_capability_canary::{
    run_benchmark_capability_canary, write_benchmark_capability_report,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let root = std::env::current_dir().map_err(|error| format!("CANARY_ROOT:{error}"))?;
    let node = PathBuf::from(r"C:\Program Files\nodejs\node.exe");
    let go = PathBuf::from(r"C:\Program Files\Go\bin\go.exe");
    let report = run_benchmark_capability_canary(&node, &go);
    let markdown = write_benchmark_capability_report(&root, &report)?;
    println!("REPORT={}", markdown.display());
    println!("DISPOSITION={}", report.disposition);
    if !report.pass {
        return Err(format!(
            "CANARY_FAILED:{}",
            report.failed_boundaries.join(",")
        ));
    }
    Ok(())
}
