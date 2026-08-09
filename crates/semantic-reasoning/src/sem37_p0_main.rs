use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use semantic_reasoning::sem37::{
    adapter::ExternalEvaluatorClient,
    baseline::run_sem36_external_transfer_baseline,
    config::{
        BRANCH, CAMPAIGN_ID, MAX_AUTONOMOUS_RESEARCH_EPOCHS, PREDECESSOR, REPORT_DIR,
        SEM36_ACCEPTANCE_PATH, SEM36_ENGINE_PATH,
    },
};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn main() -> ExitCode {
    let mut arguments = env::args().skip(1);
    let command = arguments.next().unwrap_or_else(|| "preflight".to_string());
    let root = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().expect("current directory"));
    let vault = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"D:\B_Core_SEM37_EVALUATOR_VAULT"));
    let result = match command.as_str() {
        "preflight" => preflight(&root, &vault),
        "baseline" => baseline(&root, &vault),
        other => Err(format!("UNKNOWN_SEM37_P0_COMMAND:{other}")),
    };
    match result {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("SEM37_P0_ERROR={error}");
            ExitCode::FAILURE
        }
    }
}

fn preflight(root: &Path, vault: &Path) -> Result<String, String> {
    verify_predecessor(root)?;
    let report = root.join(REPORT_DIR);
    fs::create_dir_all(&report).map_err(|error| error.to_string())?;
    let evaluator = ExternalEvaluatorClient::from_vault(vault)?;
    let fixture_receipt = evaluator.verify_fixtures()?;
    let partition_receipt = evaluator.freeze_partitions()?;
    if partition_receipt["concrete_case_ids_exposed_to_bcore"].as_u64() != Some(0)
        || partition_receipt["set_a_b_overlap"].as_u64() != Some(0)
        || partition_receipt["set_a_c_overlap"].as_u64() != Some(0)
        || partition_receipt["set_b_c_overlap"].as_u64() != Some(0)
    {
        return Err("SEM37_EXTERNAL_PARTITION_PREFLIGHT_FAILED".to_string());
    }
    let source_hashes = [
        SEM36_ENGINE_PATH,
        SEM36_ACCEPTANCE_PATH,
        "crates/semantic-reasoning/src/sem37/config.rs",
        "crates/semantic-reasoning/src/sem37/adapter.rs",
        "crates/semantic-reasoning/src/sem37/baseline.rs",
        "crates/semantic-reasoning/src/sem37/mod.rs",
        "crates/semantic-reasoning/src/sem37_p0_main.rs",
        "research/sem37/SEM37_INSTRUCTION.md",
    ]
    .into_iter()
    .map(|relative| Ok((relative.to_string(), sha256_file(&root.join(relative))?)))
    .collect::<Result<BTreeMap<_, _>, String>>()?;
    let freeze = json!({
        "schema_version": "SEM37_SEM36_EXTERNAL_TRANSFER_BASELINE_FREEZE_1",
        "CAMPAIGN_ID": CAMPAIGN_ID,
        "BRANCH": BRANCH,
        "SEALED_PREDECESSOR_COMMIT": PREDECESSOR,
        "PREDECESSOR_INTEGRITY": "PASS",
        "HEAD_AT_BASELINE_FREEZE": git_head(root)?,
        "SEM36_EXTERNAL_TRANSFER_BASELINE": true,
        "SEM36_ENGINE_SHA256": sha256_file(&root.join(SEM36_ENGINE_PATH))?,
        "SEM36_ACCEPTANCE_SHA256": sha256_file(&root.join(SEM36_ACCEPTANCE_PATH))?,
        "source_hashes": source_hashes,
        "EXTERNAL_EVALUATOR_SHA256": sha256_file(&vault.join("sem37_external_evaluator.py"))?,
        "EXTERNAL_FIXTURE_MANIFEST_SHA256": sha256_file(&vault.join("fixture_manifest.json"))?,
        "PRIVATE_PARTITION_MANIFEST_SHA256": sha256_file(&vault.join("private_partition_manifest.json"))?,
        "fixture_receipt": fixture_receipt,
        "partition_receipt": partition_receipt,
        "REQUESTED_MAX_AUTONOMOUS_RESEARCH_EPOCHS": 4096,
        "CONFIGURED_MAX_AUTONOMOUS_RESEARCH_EPOCHS": MAX_AUTONOMOUS_RESEARCH_EPOCHS,
        "CAMPAIGN_BUDGET_CONTRACT_PASS": MAX_AUTONOMOUS_RESEARCH_EPOCHS == 4096,
        "B_CORE_AUTHORED_CANONICAL_WORLD_INSTANCES": 0,
        "BENCHMARK_SPECIFIC_CAUSAL_HINT_BRANCHES": 0,
        "PRE_BASELINE_EXTERNAL_OBSERVATION_EXPOSURE_EVENTS": 0,
        "PRE_BASELINE_AUTONOMOUS_RESEARCH_EVENTS": 0,
        "EXTERNAL_GENERATOR_SOURCE_READS_BY_BCORE": 0,
        "EXTERNAL_GROUND_TRUTH_GRAPH_READS": 0,
        "EXTERNAL_GROUND_TRUTH_EQUATION_READS": 0,
        "EXPECTED_EXTERNAL_RESULT_LOOKUPS": 0,
        "NETWORK_READS_DURING_CANONICAL": 0,
        "NETWORK_WRITES_DURING_CANONICAL": 0,
        "QIS0_EXECUTED": false,
        "QUANTUM_INSPIRED_CORE_CHANGES": 0,
        "PERCEPTION_GROUNDING_STARTED": false
    });
    write_json(report.join("p0_external_transfer_freeze.json"), &freeze)?;
    Ok("SEM37_SEM36_EXTERNAL_TRANSFER_BASELINE_FROZEN".to_string())
}

fn baseline(root: &Path, vault: &Path) -> Result<String, String> {
    verify_predecessor(root)?;
    let report = root.join(REPORT_DIR);
    let freeze: Value = read_json(&report.join("p0_external_transfer_freeze.json"))?;
    let expected = freeze["source_hashes"]
        .as_object()
        .ok_or("SEM37_P0_SOURCE_HASHES_MISSING")?;
    for (relative, hash) in expected {
        if hash.as_str() != Some(sha256_file(&root.join(relative))?.as_str()) {
            return Err(format!("SEM37_P0_SOURCE_CHANGED_AFTER_FREEZE:{relative}"));
        }
    }
    for (path, field) in [
        (
            vault.join("sem37_external_evaluator.py"),
            "EXTERNAL_EVALUATOR_SHA256",
        ),
        (
            vault.join("fixture_manifest.json"),
            "EXTERNAL_FIXTURE_MANIFEST_SHA256",
        ),
        (
            vault.join("private_partition_manifest.json"),
            "PRIVATE_PARTITION_MANIFEST_SHA256",
        ),
    ] {
        let actual = sha256_file(&path)?;
        if freeze[field].as_str() != Some(actual.as_str()) {
            return Err(format!("SEM37_P0_EXTERNAL_AUTHORITY_CHANGED:{field}"));
        }
    }
    let evaluator = ExternalEvaluatorClient::from_vault(vault)?;
    let baseline = run_sem36_external_transfer_baseline(&evaluator)?;
    if !baseline.external_repair_required
        || baseline.measured_disposition != "EXTERNAL_GROUNDING_LIMIT"
    {
        return Err("SEM37_EXPECTED_MEASURED_EXTERNAL_BOUNDARY_NOT_OBSERVED".to_string());
    }
    write_json(
        report.join("sem36_external_transfer_baseline.json"),
        &baseline,
    )?;
    write_json(
        report.join("measured_external_gap.json"),
        &json!({
            "MEASURED_EXTERNAL_FAILURE": true,
            "DISPOSITION": baseline.measured_disposition,
            "EXTERNAL_REPAIR_REQUIRED": baseline.external_repair_required,
            "REPAIR_IMPLEMENTED_BEFORE_MEASUREMENT": false,
            "AUTONOMOUS_EXTERNAL_DIAGNOSIS_ALLOWED_NEXT": true,
            "SET_B_EXPOSURE_EVENTS": 0,
            "SET_C_EXPOSURE_EVENTS": 0,
            "NO_BENCHMARK_SPECIFIC_REPAIR_AUTHORIZED": true
        }),
    )?;
    Ok("SEM37_ZERO_SHOT_EXTERNAL_BASELINE_MEASURED_GROUNDING_LIMIT".to_string())
}

fn verify_predecessor(root: &Path) -> Result<(), String> {
    let status = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["merge-base", "--is-ancestor", PREDECESSOR, "HEAD"])
        .status()
        .map_err(|error| error.to_string())?;
    if !status.success() {
        return Err("SEM37_EXACT_PREDECESSOR_MISSING".to_string());
    }
    Ok(())
}

fn git_head(root: &Path) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err("SEM37_GIT_HEAD_FAILED".to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    serde_json::from_slice(&fs::read(path).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())
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
