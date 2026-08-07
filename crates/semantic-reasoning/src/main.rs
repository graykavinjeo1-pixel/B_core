use std::path::PathBuf;

fn main() {
    let repository_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf();
    let outcome = match semantic_reasoning::run_sem0(&repository_root) {
        Ok(outcome) => outcome,
        Err(disposition) => {
            eprintln!("SEM0_STATUS=FAIL");
            eprintln!("DISPOSITION={disposition}");
            std::process::exit(1);
        }
    };
    if let Err(error) = semantic_reasoning::reporting::write_reports(&repository_root, &outcome) {
        eprintln!("SEM0_STATUS=FAIL");
        eprintln!("DISPOSITION=REPORT_WRITE_FAILURE:{error}");
        std::process::exit(1);
    }
    println!("SEM0_STATUS={}", outcome.final_report.sem0_status);
    println!("DISPOSITION={}", outcome.final_report.disposition);
    println!(
        "PROMOTED_CONCEPTS={}",
        outcome.final_report.promoted_concepts
    );
}
