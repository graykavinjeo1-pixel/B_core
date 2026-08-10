use std::{env, fs, path::PathBuf};

use semantic_reasoning::sem37_r6::{
    acceptance_diff, primary_acceptance, report_dir, secondary_acceptance, FinalResult,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("SEM37_R6_VERIFY_ERROR:{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let root = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or(env::current_dir().map_err(|error| error.to_string())?);
    let report = report_dir(&root);
    let result: FinalResult = serde_json::from_slice(
        &fs::read(report.join("r6_final_k_raw.json")).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let primary = primary_acceptance(&result);
    let secondary = secondary_acceptance(&result);
    let diff = acceptance_diff(&primary, &secondary);
    println!(
        "{{\"status\":\"{}\",\"primary_secondary_acceptance_diff\":{}}}",
        if diff == 0 { "PASS" } else { "FAIL" },
        diff
    );
    if diff != 0 {
        return Err(format!("SEM37_R6_ACCEPTANCE_DIFF:{diff}"));
    }
    Ok(())
}
