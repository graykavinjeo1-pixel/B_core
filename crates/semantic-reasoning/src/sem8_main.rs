use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf();
    let outcome = match semantic_reasoning::run_sem8(&root) {
        Ok(outcome) => outcome,
        Err(disposition) => {
            let _ = semantic_reasoning::sem8::reporting::preserve_failed_run(&root, &disposition);
            eprintln!("SEM8_STATUS=FAIL");
            eprintln!("DISPOSITION={disposition}");
            std::process::exit(1);
        }
    };
    if let Err(error) = semantic_reasoning::sem8::reporting::write_reports(&root, &outcome) {
        eprintln!("SEM8_STATUS=FAIL");
        eprintln!("DISPOSITION=REPORT_WRITE_FAILURE:{error}");
        std::process::exit(1);
    }
    println!("SEM8_STATUS={}", outcome.final_report.sem8_status);
    println!("DISPOSITION={}", outcome.final_report.disposition);
    println!(
        "FULL_D_SOLVE_RATE={:.6}",
        outcome.final_report.full_d_solve_rate
    );
    println!(
        "BROKEN_ASSUMPTION_DETECTION_RATE={:.6}",
        outcome.final_report.broken_assumption_detection_rate
    );
    println!(
        "STRUCTURAL_MIMIC_FALSE_TRANSFER_RATE={:.6}",
        outcome.final_report.structural_mimic_false_transfer_rate
    );
}
