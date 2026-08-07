use std::{fs, path::Path};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use synapse_recursive_core::quarantine::{
    status as quarantine_status, RecursiveImprovementQuarantine,
};

use crate::{
    sem1::model::ConceptRecord,
    substrate::{ConceptIR, ConceptKind, PromotionState},
};

pub const PREDECESSOR_COMMIT: &str = "148865c7b8b9b1a54d29bcffdf86d7c4ebe7e143";
pub const SEM0_TREE_HASH: &str = "6ccf0423ca5c7106d70492f107c23980b9e9f31a807f778f16d66462e2558cbe";
pub const SEM1_TREE_HASH: &str = "b5083b272995fbeabb735608db43b08d289359e5f03601cc91b4eb99756f87f8";
pub const SEM2_TREE_HASH: &str = "9e0f2ee39cd7ea11f60a66842f650c0f269054a6cdcaa061e4de02f2bbb37e0c";

#[derive(Debug, Clone, Serialize)]
pub struct PredecessorTreeHash {
    pub relative_path: String,
    pub files: usize,
    pub sha256: String,
    pub expected_sha256: String,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PredecessorIntegrityReport {
    pub passed: bool,
    pub predecessor_commit: String,
    pub canonical_manifest_sha256: String,
    pub canonical_files_verified: usize,
    pub predecessor_trees: Vec<PredecessorTreeHash>,
    pub sem1_failed_run_preserved: bool,
    pub sem2_failed_run_preserved: bool,
    pub sem2_passing_run_sealed: bool,
    pub sem2_passing_run_id: String,
    pub sem2_final_report_sha256: String,
    pub sem2_blind_manifest_sha256: String,
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
struct Sem0Candidates {
    concepts: Vec<ConceptIR>,
}

#[derive(Debug, Deserialize)]
struct Sem1Ledger {
    candidates: Vec<Sem1LedgerEntry>,
}

#[derive(Debug, Deserialize)]
struct Sem1LedgerEntry {
    concept: ConceptRecord,
    promoted: bool,
}

pub fn verify_predecessors(root: &Path) -> Result<PredecessorIntegrityReport, String> {
    let (canonical_files_verified, canonical_manifest_sha256) = verify_canonical_manifest(root)?;
    let mut predecessor_trees = Vec::new();
    for (relative, expected) in [
        ("reports/sem0", SEM0_TREE_HASH),
        ("reports/sem1", SEM1_TREE_HASH),
        ("reports/sem2", SEM2_TREE_HASH),
    ] {
        let (sha256, files) = hash_tree(root, relative)?;
        let passed = sha256 == expected;
        predecessor_trees.push(PredecessorTreeHash {
            relative_path: relative.to_string(),
            files,
            sha256,
            expected_sha256: expected.to_string(),
            passed,
        });
    }
    if predecessor_trees.iter().any(|tree| !tree.passed) {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:ARTIFACT_TREE_DRIFT".to_string());
    }
    let sem1_failed =
        read_json(root.join("reports/sem1/runs/SEM1-RUN-0001/sem1_final_report.json"))?;
    let sem2_failed =
        read_json(root.join("reports/sem2/runs/SEM2-RUN-0001/sem2_final_report.json"))?;
    let sem2_final_path = root.join("reports/sem2/sem2_final_report.json");
    let sem2_final_bytes = fs::read(&sem2_final_path).map_err(|error| error.to_string())?;
    let sem2_final: serde_json::Value =
        serde_json::from_slice(&sem2_final_bytes).map_err(|error| error.to_string())?;
    let freeze = read_json(root.join("reports/sem2/freeze_record.json"))?;
    let sem1_failed_run_preserved = sem1_failed["sem1_status"] == "FAIL";
    let sem2_failed_run_preserved = sem2_failed["sem2_status"] == "FAIL";
    let sem2_passing_run_sealed = sem2_final["sem2_status"] == "PASS"
        && sem2_final["gates"].as_array().is_some_and(|gates| {
            gates.len() == 8 && gates.iter().all(|gate| gate["passed"] == true)
        })
        && freeze["run_id"] == "SEM2-RUN-0002"
        && freeze["frozen_before_blind"] == true
        && freeze["post_blind_tuning"] == false;
    if !sem1_failed_run_preserved || !sem2_failed_run_preserved || !sem2_passing_run_sealed {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:HISTORICAL_RUN_RECORD".to_string());
    }
    let promoted_concepts_verified_immutable = verify_promoted_concepts(root)?;
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
        canonical_manifest_sha256,
        canonical_files_verified,
        predecessor_trees,
        sem1_failed_run_preserved,
        sem2_failed_run_preserved,
        sem2_passing_run_sealed,
        sem2_passing_run_id: freeze["run_id"].as_str().unwrap_or_default().to_string(),
        sem2_final_report_sha256: hash_bytes(&sem2_final_bytes),
        sem2_blind_manifest_sha256: hash_bytes(
            &fs::read(root.join("reports/sem2/blind_manifest.json"))
                .map_err(|error| error.to_string())?,
        ),
        promoted_concepts_verified_immutable,
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

fn verify_promoted_concepts(root: &Path) -> Result<Vec<String>, String> {
    let sem0: Sem0Candidates = serde_json::from_slice(
        &fs::read(root.join("reports/sem0/candidate_concepts.json"))
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let predecessor = sem0
        .concepts
        .into_iter()
        .find(|concept| concept.concept_id == "C000001")
        .ok_or_else(|| "C000001_MISSING".to_string())?;
    let stored = predecessor.content_hash_sha256.clone();
    let mut recomputed = predecessor.clone();
    recomputed
        .freeze_hash()
        .map_err(|error| error.to_string())?;
    if predecessor.kind != ConceptKind::Promoted
        || predecessor.promotion_state != PromotionState::Promoted
        || recomputed.content_hash_sha256 != stored
    {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:C000001_MUTATED".to_string());
    }
    let ledger: Sem1Ledger = serde_json::from_slice(
        &fs::read(root.join("reports/sem1/concept_generation_ledger.json"))
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let mut immutable = vec!["C000001".to_string()];
    for entry in ledger.candidates.into_iter().filter(|entry| entry.promoted) {
        let stored = entry.concept.content_hash_sha256.clone();
        let mut concept = entry.concept;
        concept.freeze_hash()?;
        if concept.content_hash_sha256 != stored
            || concept.promotion_state != "PROMOTED"
            || !concept.derived_autonomously
        {
            return Err("PREDECESSOR_INTEGRITY_FAILURE:PROMOTED_CONCEPT_MUTATED".to_string());
        }
        immutable.push(concept.concept_id);
    }
    immutable.sort();
    if immutable != ["C000001", "C000002", "C000004", "C000005"] {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:PROMOTED_CONCEPT_SET".to_string());
    }
    Ok(immutable)
}

fn verify_canonical_manifest(root: &Path) -> Result<(usize, String), String> {
    let raw = fs::read_to_string(root.join("docs/CANONICAL_MANIFEST.json"))
        .map_err(|error| error.to_string())?;
    let manifest: serde_json::Value =
        serde_json::from_str(&raw).map_err(|error| error.to_string())?;
    let self_hash = manifest["manifest_self_hash_sha256"]
        .as_str()
        .ok_or_else(|| "CANONICAL_SELF_HASH_MISSING".to_string())?;
    let normalized = raw.replacen(self_hash, &"0".repeat(64), 1);
    if hash_bytes(normalized.as_bytes()) != self_hash {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:CANONICAL_MANIFEST".to_string());
    }
    let files = manifest["canonical_files"]
        .as_array()
        .ok_or_else(|| "CANONICAL_FILES_MISSING".to_string())?;
    for file in files {
        let relative = file["relative_path"]
            .as_str()
            .ok_or_else(|| "CANONICAL_PATH_MISSING".to_string())?;
        let bytes = fs::read(root.join(relative)).map_err(|error| error.to_string())?;
        if bytes.len() != file["byte_length"].as_u64().unwrap_or_default() as usize
            || hash_bytes(&bytes) != file["sha256"].as_str().unwrap_or_default()
        {
            return Err(format!("PREDECESSOR_INTEGRITY_FAILURE:{relative}"));
        }
    }
    Ok((files.len(), self_hash.to_string()))
}

fn read_json(path: std::path::PathBuf) -> Result<serde_json::Value, String> {
    serde_json::from_slice(&fs::read(path).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())
}

fn hash_tree(root: &Path, relative: &str) -> Result<(String, usize), String> {
    let mut paths = Vec::new();
    collect_files(&root.join(relative), &mut paths).map_err(|error| error.to_string())?;
    paths.sort();
    let mut manifest = String::new();
    for path in &paths {
        let bytes = fs::read(path).map_err(|error| error.to_string())?;
        let path = path
            .strip_prefix(root)
            .map_err(|error| error.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        manifest.push_str(&format!("{path}|{}|{}\n", bytes.len(), hash_bytes(&bytes)));
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
    fn all_predecessor_stages_failures_concepts_and_quarantine_verify() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("root")
            .to_path_buf();
        let report = super::verify_predecessors(&root).expect("integrity");
        assert!(report.passed);
        assert!(report.sem1_failed_run_preserved);
        assert!(report.sem2_failed_run_preserved);
        assert!(report.sem2_passing_run_sealed);
        assert_eq!(report.promoted_concepts_verified_immutable.len(), 4);
        assert!(!report.source_mutation);
    }
}
