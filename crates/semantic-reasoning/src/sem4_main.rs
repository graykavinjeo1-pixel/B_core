use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf();
    let outcome = match semantic_reasoning::run_sem4(&root) {
        Ok(outcome) => outcome,
        Err(disposition) => {
            eprintln!("SEM4_STATUS=FAIL");
            eprintln!("DISPOSITION={disposition}");
            std::process::exit(1);
        }
    };
    if let Err(error) = semantic_reasoning::sem4::reporting::write_reports(&root, &outcome) {
        eprintln!("SEM4_STATUS=FAIL");
        eprintln!("DISPOSITION=REPORT_WRITE_FAILURE:{error}");
        std::process::exit(1);
    }
    println!("SEM4_STATUS={}", outcome.final_report.sem4_status);
    println!("DISPOSITION={}", outcome.final_report.disposition);
    println!(
        "DEFINITION_ONLY_ZERO_SHOT={:.6}",
        outcome.final_report.definition_only_zero_shot_solve_rate
    );
}
