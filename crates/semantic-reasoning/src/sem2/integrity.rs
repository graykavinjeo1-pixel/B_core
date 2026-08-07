use std::{fs, path::Path};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use synapse_recursive_core::quarantine::{
    status as quarantine_status, RecursiveImprovementQuarantine,
};

use crate::sem1::{integrity::verify_and_load, model::ConceptRecord};

pub const PREDECESSOR_COMMIT: &str = "52e4937cb39c78120e5767948046cc9dab44d23b";
pub const SEALED_SEM1_TREE_HASH: &str =
    "b5083b272995fbeabb735608db43b08d289359e5f03601cc91b4eb99756f87f8";

#[derive(Debug, Clone, Serialize)]
pub struct PredecessorIntegrityReport {
    pub passed: bool,
    pub predecessor_commit: String,
    pub canonical_manifest_sha256: String,
    pub sem0_integrity_passed: bool,
    pub sem0_artifacts_verified: usize,
    pub sem1_tree_sha256: String,
    pub sem1_tree_expected_sha256: String,
    pub sem1_files_verified: usize,
    pub run_0001_preserved_failure: bool,
    pub run_0001_final_sha256: String,
    pub run_0002_sealed_success: bool,
    pub run_0002_final_sha256: String,
    pub run_0002_blind_manifest_sha256: String,
    pub promoted_concepts_verified_immutable: Vec<String>,
    pub recursive_improvement_quarantine: RecursiveImprovementQuarantine,
    pub self_observe: bool,
    pub self_measure: bool,
    pub self_propose: bool,
    pub self_apply: bool,
    pub source_mutation: bool,
    pub auto_patch: bool,
    pub auto_commit: bool,
    pub auto_push: bool,
}

#[derive(Debug, Deserialize)]
struct Ledger {
    candidates: Vec<LedgerEntry>,
}

#[derive(Debug, Deserialize)]
struct LedgerEntry {
    concept: ConceptRecord,
    promoted: bool,
}

pub fn verify_predecessors(root: &Path) -> Result<PredecessorIntegrityReport, String> {
    let (sem0, c000001) =
        verify_and_load(root).map_err(|error| format!("PREDECESSOR_INTEGRITY_FAILURE:{error}"))?;
    let (sem1_tree_sha256, sem1_files_verified) = hash_tree(root, "reports/sem1")?;
    if sem1_tree_sha256 != SEALED_SEM1_TREE_HASH {
        return Err(format!(
            "PREDECESSOR_INTEGRITY_FAILURE:SEM1_ARTIFACT_DRIFT:{sem1_tree_sha256}"
        ));
    }
    let run1_path = root.join("reports/sem1/runs/SEM1-RUN-0001/sem1_final_report.json");
    let run2_path = root.join("reports/sem1/sem1_final_report.json");
    let run1_bytes = fs::read(&run1_path).map_err(|error| error.to_string())?;
    let run2_bytes = fs::read(&run2_path).map_err(|error| error.to_string())?;
    let run1: serde_json::Value =
        serde_json::from_slice(&run1_bytes).map_err(|error| error.to_string())?;
    let run2: serde_json::Value =
        serde_json::from_slice(&run2_bytes).map_err(|error| error.to_string())?;
    let run_0001_preserved_failure =
        run1["sem1_status"] == "FAIL" && run1["gen1_ancestor_ablation_pass"] == false;
    let run_0002_sealed_success = run2["sem1_status"] == "PASS"
        && run2["fresh_blind_tasks"] == 20
        && run2["gen1_ancestor_ablation_pass"] == true
        && run2["semantic_separation_pass"] == true;
    if !run_0001_preserved_failure || !run_0002_sealed_success {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:SEM1_RUN_RECORDS".to_string());
    }
    let freeze: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("reports/sem1/freeze_record.json"))
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    if freeze["run_id"] != "SEM1-RUN-0002"
        || freeze["frozen_before_blind"] != true
        || freeze["post_blind_tuning"] != false
    {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:SEM1_FREEZE".to_string());
    }
    let ledger: Ledger = serde_json::from_slice(
        &fs::read(root.join("reports/sem1/concept_generation_ledger.json"))
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let mut immutable = vec![c000001.concept_id];
    for entry in ledger.candidates.into_iter().filter(|entry| entry.promoted) {
        let stored = entry.concept.content_hash_sha256.clone();
        let mut recomputed = entry.concept;
        recomputed.freeze_hash()?;
        if recomputed.content_hash_sha256 != stored
            || recomputed.promotion_state != "PROMOTED"
            || !recomputed.derived_autonomously
        {
            return Err("PREDECESSOR_INTEGRITY_FAILURE:PROMOTED_CONCEPT_MUTATION".to_string());
        }
        immutable.push(recomputed.concept_id);
    }
    immutable.sort();
    if immutable != ["C000001", "C000002", "C000004", "C000005"] {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:PROMOTED_CONCEPT_SET".to_string());
    }
    let quarantine = quarantine_status();
    let quarantine_pass = quarantine.observe_enabled
        && quarantine.measure_enabled
        && !quarantine.proposal_generation_enabled
        && !quarantine.source_patching_enabled
        && !quarantine.sandbox_apply_enabled
        && !quarantine.auto_apply_enabled
        && !quarantine.auto_merge_enabled
        && !quarantine.auto_commit_enabled
        && !quarantine.auto_push_enabled
        && !quarantine.external_provider_repair_enabled
        && !quarantine.recursive_benchmark_mutation_enabled
        && !quarantine.network_enabled
        && !quarantine.external_llm_enabled;
    if !quarantine_pass {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:QUARANTINE".to_string());
    }
    Ok(PredecessorIntegrityReport {
        passed: true,
        predecessor_commit: PREDECESSOR_COMMIT.to_string(),
        canonical_manifest_sha256: sem0.canonical_manifest_sha256,
        sem0_integrity_passed: sem0.passed,
        sem0_artifacts_verified: sem0.sem0_artifacts_verified,
        sem1_tree_sha256,
        sem1_tree_expected_sha256: SEALED_SEM1_TREE_HASH.to_string(),
        sem1_files_verified,
        run_0001_preserved_failure,
        run_0001_final_sha256: hash_bytes(&run1_bytes),
        run_0002_sealed_success,
        run_0002_final_sha256: hash_bytes(&run2_bytes),
        run_0002_blind_manifest_sha256: freeze["blind_manifest_sha256"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        promoted_concepts_verified_immutable: immutable,
        recursive_improvement_quarantine: quarantine,
        self_observe: true,
        self_measure: true,
        self_propose: false,
        self_apply: false,
        source_mutation: false,
        auto_patch: false,
        auto_commit: false,
        auto_push: false,
    })
}

fn hash_tree(root: &Path, relative: &str) -> Result<(String, usize), String> {
    let base = root.join(relative);
    let mut paths = Vec::new();
    collect_files(&base, &mut paths).map_err(|error| error.to_string())?;
    paths.sort();
    let mut manifest = String::new();
    for path in &paths {
        let bytes = fs::read(path).map_err(|error| error.to_string())?;
        let relative_path = path
            .strip_prefix(root)
            .map_err(|error| error.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        manifest.push_str(&format!(
            "{relative_path}|{}|{}\n",
            bytes.len(),
            hash_bytes(&bytes)
        ));
    }
    Ok((hash_bytes(manifest.as_bytes()), paths.len()))
}

fn collect_files(directory: &Path, target: &mut Vec<std::path::PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            collect_files(&entry.path(), target)?;
        } else {
            target.push(entry.path());
        }
    }
    Ok(())
}

pub fn hash_serializable<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|bytes| hash_bytes(&bytes))
        .map_err(|error| error.to_string())
}

pub fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    #[test]
    fn sem0_sem1_runs_concepts_and_quarantine_are_immutable() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("root")
            .to_path_buf();
        let report = super::verify_predecessors(&root).expect("integrity");
        assert!(report.passed);
        assert!(report.run_0001_preserved_failure);
        assert!(report.run_0002_sealed_success);
        assert_eq!(report.promoted_concepts_verified_immutable.len(), 4);
        assert!(!report.source_mutation);
    }
}
