use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf();
    let outcome = match semantic_reasoning::run_sem3(&root) {
        Ok(outcome) => outcome,
        Err(disposition) => {
            eprintln!("SEM3_STATUS=FAIL");
            eprintln!("DISPOSITION={disposition}");
            std::process::exit(1);
        }
    };
    if let Err(error) = semantic_reasoning::sem3::reporting::write_reports(&root, &outcome) {
        eprintln!("SEM3_STATUS=FAIL");
        eprintln!("DISPOSITION=REPORT_WRITE_FAILURE:{error}");
        std::process::exit(1);
    }
    println!("SEM3_STATUS={}", outcome.final_report.sem3_status);
    println!("DISPOSITION={}", outcome.final_report.disposition);
    println!(
        "ACTIVE_INFORMATION_EFFICIENCY={:.6}",
        outcome
            .final_report
            .active_e_information_gain_per_experiment
    );
}
