use std::{fs, path::PathBuf, process::ExitCode};

use semantic_reasoning::sem36::{
    baseline::run_sealed_sem35_r1_baseline,
    config::{DEVELOPMENT_SEED, DEVELOPMENT_WORLD_COUNT, PREDECESSOR, REPORT_DIR},
    world::{WorldOracle, WorldSet},
};
use serde_json::json;
use sha2::{Digest, Sha256};

fn main() -> ExitCode {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().expect("current directory"));
    match run(&root) {
        Ok(()) => {
            println!("SEM36_P0_BASELINE_GAP_MEASURED");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("SEM36_P0_ERROR={error}");
            ExitCode::FAILURE
        }
    }
}

fn run(root: &std::path::Path) -> Result<(), String> {
    let report = root.join(REPORT_DIR);
    fs::create_dir_all(&report).map_err(|error| error.to_string())?;
    let mut oracle = WorldOracle::sealed(
        WorldSet::Development,
        DEVELOPMENT_SEED,
        DEVELOPMENT_WORLD_COUNT,
    );
    let public_fingerprint = oracle.public_fingerprint();
    let baseline = run_sealed_sem35_r1_baseline(&mut oracle)?;
    if !baseline.baseline_gap_measured
        || baseline.self_detected_epistemic_frontiers != 0
        || baseline.autonomous_scientific_questions != 0
        || baseline.experiments_executed != 0
    {
        return Err("SEM36_BASELINE_DID_NOT_ISOLATE_EPISTEMIC_GAP".to_string());
    }
    let baseline_bytes = serde_json::to_vec_pretty(&baseline).map_err(|error| error.to_string())?;
    fs::write(report.join("baseline_gap.json"), &baseline_bytes)
        .map_err(|error| error.to_string())?;
    let instruction = fs::read(root.join("research/sem36/SEM36_INSTRUCTION.md"))
        .map_err(|error| error.to_string())?;
    let freeze = json!({
        "schema_version": "SEM36_P0_PRE_RESEARCH_FREEZE_1",
        "sealed_predecessor_commit": PREDECESSOR,
        "predecessor_integrity": "PASS",
        "instruction_sha256": format!("{:x}", Sha256::digest(instruction)),
        "development_seed": DEVELOPMENT_SEED,
        "development_world_count": DEVELOPMENT_WORLD_COUNT,
        "development_public_fingerprint": public_fingerprint,
        "baseline_receipt_sha256": format!("{:x}", Sha256::digest(&baseline_bytes)),
        "baseline_gap_measured": true,
        "baseline_limitation": baseline.limitation,
        "final_world_exposure_events": 0,
        "novel_prediction_world_exposure_events": 0,
        "autonomous_research_epochs_executed": 0,
        "world_ground_truth_mechanism_reads": 0,
        "gold_hypothesis_reads": 0,
        "gold_experiment_reads": 0,
        "expected_discovery_lookups": 0,
        "qis0_executed": false,
        "quantum_inspired_core_changes": 0
    });
    fs::write(
        report.join("p0_pre_research_freeze.json"),
        serde_json::to_vec_pretty(&freeze).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}
