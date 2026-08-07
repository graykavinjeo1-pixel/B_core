use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf();
    let outcome = match semantic_reasoning::run_sem5(&root) {
        Ok(outcome) => outcome,
        Err(disposition) => {
            eprintln!("SEM5_STATUS=FAIL");
            eprintln!("DISPOSITION={disposition}");
            std::process::exit(1);
        }
    };
    if let Err(error) = semantic_reasoning::sem5::reporting::write_reports(&root, &outcome) {
        eprintln!("SEM5_STATUS=FAIL");
        eprintln!("DISPOSITION=REPORT_WRITE_FAILURE:{error}");
        std::process::exit(1);
    }
    println!("SEM5_STATUS={}", outcome.final_report.sem5_status);
    println!("DISPOSITION={}", outcome.final_report.disposition);
    println!(
        "PROPERTY_GENERALIZATION={:.6}",
        outcome.final_report.property_generalization_pass_rate
    );
}
