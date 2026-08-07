use std::path::PathBuf;

use semantic_reasoning::sem9r1::{experiment::run_sem9_r1, reporting::write_reports};

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf();
    match run_sem9_r1(&root) {
        Ok(outcome) => {
            if let Err(error) = write_reports(&root, &outcome) {
                eprintln!("SEM9_R1_STATUS=FAIL\nDISPOSITION=REPORTING_FAILURE:{error}");
                std::process::exit(1);
            }
            println!("SEM9_R1_STATUS={}", outcome.final_report.sem9_r1_status);
            println!("DISPOSITION={}", outcome.final_report.disposition);
            println!(
                "VERIFIED_SELF_APPLICATION_CANDIDATES={}",
                outcome.final_report.verified_self_application_candidates
            );
            println!(
                "EXPANSION_REDUCTION={}",
                outcome.final_report.performance.expansion_reduction
            );
            if outcome.final_report.sem9_r1_status != "PASS" {
                std::process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("SEM9_R1_STATUS=FAIL\nDISPOSITION={error}");
            std::process::exit(1);
        }
    }
}
