use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf();
    let outcome = match semantic_reasoning::run_sem6(&root) {
        Ok(outcome) => outcome,
        Err(disposition) => {
            eprintln!("SEM6_STATUS=FAIL");
            eprintln!("DISPOSITION={disposition}");
            std::process::exit(1);
        }
    };
    if let Err(error) = semantic_reasoning::sem6::reporting::write_reports(&root, &outcome) {
        eprintln!("SEM6_STATUS=FAIL");
        eprintln!("DISPOSITION=REPORT_WRITE_FAILURE:{error}");
        std::process::exit(1);
    }
    println!("SEM6_STATUS={}", outcome.final_report.sem6_status);
    println!("DISPOSITION={}", outcome.final_report.disposition);
    println!(
        "SEALED_ZERO_SHOT={:.6}",
        outcome
            .final_report
            .sealed_corpus_definition_zero_shot_solve_rate
    );
    println!(
        "LIVE_ZERO_SHOT={:.6}",
        outcome
            .final_report
            .live_foraging_definition_zero_shot_solve_rate
    );
}
