use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use semantic_reasoning::sem37_r1::{
    adapter::R1ExternalEvaluatorClient,
    config::{
        BRANCH, CAMPAIGN_ID, CAPABILITY_PREDECESSOR, HISTORICAL_SEM37_COMMIT,
        MAX_AUTONOMOUS_RESEARCH_EPOCHS, REPORT_DIR, SEM36_ACCEPTANCE_PATH, SEM36_ENGINE_PATH,
        SEM36_ENGINE_SHA256,
    },
};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};

fn main() -> ExitCode {
    let mut arguments = env::args().skip(1);
    let root = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().expect("current directory"));
    let vault = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"D:\B_Core_SEM37_R1_EVALUATOR_VAULT"));
    match preflight(&root, &vault) {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("SEM37_R1_P0_ERROR={error}");
            ExitCode::FAILURE
        }
    }
}

fn preflight(root: &Path, vault: &Path) -> Result<String, String> {
    require_exact_history(root)?;
    let report = root.join(REPORT_DIR);
    fs::create_dir_all(&report).map_err(|error| error.to_string())?;
    let actual_engine_hash = sha256_file(&root.join(SEM36_ENGINE_PATH))?;
    if actual_engine_hash != SEM36_ENGINE_SHA256 {
        return Err("SEM37_R1_P0_SEM36_ENGINE_DIFF_NONZERO".to_string());
    }
    let evaluator = R1ExternalEvaluatorClient::from_vault(vault)?;
    let fixture_receipt = evaluator.verify_fixtures()?;
    let partition_receipt = evaluator.freeze_partitions()?;
    for field in [
        "historical_sem37_set_c_overlap",
        "r1_dev_final_overlap",
        "r1_dev_a_dev_b_overlap",
        "r1_dev_a_final_overlap",
        "r1_dev_b_final_overlap",
        "concrete_case_ids_exposed_to_bcore",
        "prestart_future_instance_exposure_events",
    ] {
        if partition_receipt[field].as_u64() != Some(0) {
            return Err(format!("SEM37_R1_P0_PARTITION_BOUNDARY_FAILED:{field}"));
        }
    }
    let source_paths = [
        SEM36_ENGINE_PATH,
        SEM36_ACCEPTANCE_PATH,
        "crates/semantic-reasoning/src/sem37_r1/config.rs",
        "crates/semantic-reasoning/src/sem37_r1/adapter.rs",
        "crates/semantic-reasoning/src/sem37_r1/mod.rs",
        "crates/semantic-reasoning/src/sem37_r1_p0_main.rs",
        "research/sem37_r1/SEM37_R1_INSTRUCTION.md",
        "reports/sem37-r1/historical_sem37_failure.json",
    ];
    let source_hashes = source_paths
        .into_iter()
        .map(|relative| Ok((relative.to_string(), sha256_file(&root.join(relative))?)))
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    let freeze = json!({
        "schema_version": "SEM37_R1_P0_INFRASTRUCTURE_FREEZE_1",
        "CAMPAIGN_ID": CAMPAIGN_ID,
        "BRANCH": BRANCH,
        "SEALED_CAPABILITY_PREDECESSOR_COMMIT": CAPABILITY_PREDECESSOR,
        "HISTORICAL_SEM37_COMMIT": HISTORICAL_SEM37_COMMIT,
        "HISTORICAL_SEM37_STATUS": "FAIL",
        "HISTORICAL_SEM37_DISPOSITION": "EXTERNAL_MECHANISM_TRANSFER_LIMIT",
        "HISTORICAL_SET_C_FINAL_AUTHORITY": false,
        "FAILED_SEM37_ADAPTIVE_STATE_IS_PREDECESSOR": false,
        "HEAD_AT_P0_FREEZE": git_head(root)?,
        "PREDECESSOR_INTEGRITY": "PASS",
        "P0_SCIENTIFIC_ENGINE_DIFF_FROM_SEM36": 0,
        "P0_SEM36_ENGINE_SHA256": actual_engine_hash,
        "P0_SEM36_ACCEPTANCE_SHA256": sha256_file(&root.join(SEM36_ACCEPTANCE_PATH))?,
        "P0_BENCHMARK_SPECIFIC_CAUSAL_HINTS": 0,
        "GENERIC_EXTERNAL_DYNAMICAL_ADAPTER_PRESENT": true,
        "source_hashes": source_hashes,
        "R1_EXTERNAL_EVALUATOR_SHA256": sha256_file(&vault.join("sem37_r1_external_evaluator.py"))?,
        "R1_FIXTURE_MANIFEST_SHA256": sha256_file(&vault.join("r1_fixture_manifest.json"))?,
        "R1_PRIVATE_PARTITION_MANIFEST_SHA256": sha256_file(&vault.join("r1_private_partition_manifest.json"))?,
        "fixture_receipt": fixture_receipt,
        "partition_receipt": partition_receipt,
        "PRE_P0_R1_ADAPTIVE_EXTERNAL_EXPOSURE_EVENTS": 0,
        "PRESTART_FUTURE_INSTANCE_EXPOSURE_EVENTS": 0,
        "REQUESTED_MAX_AUTONOMOUS_RESEARCH_EPOCHS": 4096,
        "CONFIGURED_MAX_AUTONOMOUS_RESEARCH_EPOCHS": MAX_AUTONOMOUS_RESEARCH_EPOCHS,
        "CAMPAIGN_BUDGET_CONTRACT_PASS": MAX_AUTONOMOUS_RESEARCH_EPOCHS == 4096,
        "NETWORK_READS_DURING_CANONICAL": 0,
        "NETWORK_WRITES_DURING_CANONICAL": 0,
        "QIS0_EXECUTED": false,
        "QUANTUM_INSPIRED_CORE_CHANGES": 0,
        "PERCEPTION_GROUNDING_STARTED": false
    });
    write_json(report.join("p0_infrastructure_freeze.json"), &freeze)?;
    Ok("SEM37_R1_P0_INFRASTRUCTURE_FROZEN".to_string())
}

fn require_exact_history(root: &Path) -> Result<(), String> {
    let head = git_head(root)?;
    if head != CAPABILITY_PREDECESSOR && !git_is_ancestor(root, CAPABILITY_PREDECESSOR, &head)? {
        return Err("SEM37_R1_CAPABILITY_PREDECESSOR_MISSING".to_string());
    }
    let historical_exists = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "cat-file",
            "-e",
            &format!("{HISTORICAL_SEM37_COMMIT}^{{commit}}"),
        ])
        .status()
        .map_err(|error| error.to_string())?;
    if !historical_exists.success() {
        return Err("SEM37_R1_HISTORICAL_SEM37_COMMIT_MISSING".to_string());
    }
    Ok(())
}

fn git_is_ancestor(root: &Path, ancestor: &str, descendant: &str) -> Result<bool, String> {
    Ok(Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["merge-base", "--is-ancestor", ancestor, descendant])
        .status()
        .map_err(|error| error.to_string())?
        .success())
}

fn git_head(root: &Path) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err("SEM37_R1_GIT_HEAD_FAILED".to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn write_json<T: Serialize + ?Sized>(path: PathBuf, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn sha256_file(path: &Path) -> Result<String, String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(fs::read(path).map_err(|error| error.to_string())?)
    ))
}
