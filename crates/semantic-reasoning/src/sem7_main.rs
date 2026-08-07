use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf();
    let outcome = match semantic_reasoning::run_sem7(&root) {
        Ok(outcome) => outcome,
        Err(disposition) => {
            eprintln!("SEM7_STATUS=FAIL");
            eprintln!("DISPOSITION={disposition}");
            std::process::exit(1);
        }
    };
    if let Err(error) = semantic_reasoning::sem7::reporting::write_reports(&root, &outcome) {
        eprintln!("SEM7_STATUS=FAIL");
        eprintln!("DISPOSITION=REPORT_WRITE_FAILURE:{error}");
        std::process::exit(1);
    }
    println!("SEM7_STATUS={}", outcome.final_report.sem7_status);
    println!("DISPOSITION={}", outcome.final_report.disposition);
    println!(
        "LANGUAGE_TO_GOAL_IR_ACCURACY={:.6}",
        outcome.final_report.language_to_goal_ir_accuracy
    );
    println!(
        "SEMANTIC_LANGUAGE_SEPARATION_PASS={}",
        outcome.final_report.semantic_language_separation_pass
    );
}
