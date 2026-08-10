use std::path::PathBuf;

fn main() {
    let report_dir = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("reports").join("b-core-code-graft-01"));
    if let Err(error) = semantic_reasoning::code_graft_acceptance::verify_and_report(&report_dir) {
        eprintln!("B_CORE_CODE_GRAFT_VERIFY_ERROR:{error}");
        std::process::exit(1);
    }
}
