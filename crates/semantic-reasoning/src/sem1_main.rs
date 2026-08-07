use std::path::PathBuf;

fn main() {
    let repository_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf();
    let outcome = match semantic_reasoning::run_sem1(&repository_root) {
        Ok(outcome) => outcome,
        Err(disposition) => {
            eprintln!("SEM1_STATUS=FAIL");
            eprintln!("DISPOSITION={disposition}");
            std::process::exit(1);
        }
    };
    if let Err(error) =
        semantic_reasoning::sem1::reporting::write_reports(&repository_root, &outcome)
    {
        eprintln!("SEM1_STATUS=FAIL");
        eprintln!("DISPOSITION=REPORT_WRITE_FAILURE:{error}");
        std::process::exit(1);
    }
    println!("SEM1_STATUS={}", outcome.final_report.sem1_status);
    println!("DISPOSITION={}", outcome.final_report.disposition);
    println!(
        "MAX_AUTONOMOUS_CONCEPT_GENERATION={}",
        outcome.final_report.max_autonomous_concept_generation
    );
    println!(
        "SEMANTIC_SEPARATION_PASS={}",
        outcome.final_report.semantic_separation_pass
    );
}
