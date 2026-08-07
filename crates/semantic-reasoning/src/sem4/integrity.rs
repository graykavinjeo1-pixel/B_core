use std::{fs, path::Path};

use serde::Serialize;
use sha2::{Digest, Sha256};
use synapse_recursive_core::quarantine::RecursiveImprovementQuarantine;

pub const PREDECESSOR_COMMIT: &str = "87633105e088392ee6fa4d72039f3c5de27eb250";
pub const CANONICAL_MANIFEST_HASH: &str =
    "d34b467e86cf9d6da2531b191df2a17a677cf02040a908e2c3467ce2b9c7f61c";
pub const SEM0_TREE_HASH: &str = "6ccf0423ca5c7106d70492f107c23980b9e9f31a807f778f16d66462e2558cbe";
pub const SEM1_TREE_HASH: &str = "b5083b272995fbeabb735608db43b08d289359e5f03601cc91b4eb99756f87f8";
pub const SEM2_TREE_HASH: &str = "9e0f2ee39cd7ea11f60a66842f650c0f269054a6cdcaa061e4de02f2bbb37e0c";
pub const SEM3_TREE_HASH: &str = "90ad74b1a86ee455a3d20ac52d906a6ee82435b163665580c9d8b8ac5720abdd";

#[derive(Debug, Clone, Serialize)]
pub struct PredecessorTreeHash {
    pub relative_path: String,
    pub files: usize,
    pub sha256: String,
    pub expected_sha256: String,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Sem4PredecessorIntegrity {
    pub passed: bool,
    pub predecessor_commit: String,
    pub canonical_manifest_sha256: String,
    pub canonical_files_verified: usize,
    pub predecessor_trees: Vec<PredecessorTreeHash>,
    pub sem1_failed_run_preserved: bool,
    pub sem2_failed_run_preserved: bool,
    pub sem3_passing_run_sealed: bool,
    pub sem3_run_id: String,
    pub sem3_final_report_sha256: String,
    pub sem3_blind_manifest_sha256: String,
    pub promoted_concepts_verified_immutable: Vec<String>,
    pub recursive_improvement_quarantine: RecursiveImprovementQuarantine,
    pub network_enabled: bool,
    pub external_llm_enabled: bool,
    pub local_teacher_enabled: bool,
    pub self_observe: bool,
    pub self_measure: bool,
    pub self_propose: bool,
    pub self_apply: bool,
    pub source_mutation: bool,
    pub auto_patch: bool,
    pub auto_commit: bool,
    pub auto_push: bool,
}

pub fn verify_predecessors(root: &Path) -> Result<Sem4PredecessorIntegrity, String> {
    let canonical_files_verified = verify_canonical_manifest(root)?;
    let inherited = crate::sem3::integrity::verify_predecessors(root)?;
    let expected_trees = [
        ("reports/sem0", SEM0_TREE_HASH),
        ("reports/sem1", SEM1_TREE_HASH),
        ("reports/sem2", SEM2_TREE_HASH),
        ("reports/sem3", SEM3_TREE_HASH),
    ];
    let mut predecessor_trees = Vec::new();
    for (relative, expected) in expected_trees {
        let (sha256, files) = hash_tree(root, relative)?;
        predecessor_trees.push(PredecessorTreeHash {
            relative_path: relative.to_string(),
            files,
            passed: sha256 == expected,
            sha256,
            expected_sha256: expected.to_string(),
        });
    }
    if predecessor_trees.iter().any(|tree| !tree.passed) {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:ARTIFACT_TREE_DRIFT".to_string());
    }
    let sem3_final_bytes =
        fs::read(root.join("reports/sem3/sem3_final_report.json")).map_err(|e| e.to_string())?;
    let sem3_final: serde_json::Value =
        serde_json::from_slice(&sem3_final_bytes).map_err(|e| e.to_string())?;
    let freeze_bytes =
        fs::read(root.join("reports/sem3/freeze_record.json")).map_err(|e| e.to_string())?;
    let freeze: serde_json::Value =
        serde_json::from_slice(&freeze_bytes).map_err(|e| e.to_string())?;
    let sem3_passing_run_sealed = sem3_final["sem3_status"] == "PASS"
        && sem3_final["sem4_started"] == false
        && sem3_final["gates"].as_array().is_some_and(|gates| {
            gates.len() == 9 && gates.iter().all(|gate| gate["passed"] == true)
        })
        && freeze["run_id"] == "SEM3-RUN-0001"
        && freeze["frozen_before_curriculum"] == true
        && freeze["selector_blind_access"] == false
        && freeze["post_blind_tuning"] == false;
    if !sem3_passing_run_sealed {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:SEM3_RUN_RECORD".to_string());
    }
    let quarantine = inherited.recursive_improvement_quarantine;
    if quarantine.network_enabled
        || quarantine.external_llm_enabled
        || quarantine.proposal_generation_enabled
        || quarantine.source_patching_enabled
        || quarantine.auto_apply_enabled
        || quarantine.auto_commit_enabled
        || quarantine.auto_push_enabled
    {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:QUARANTINE".to_string());
    }
    Ok(Sem4PredecessorIntegrity {
        passed: true,
        predecessor_commit: PREDECESSOR_COMMIT.to_string(),
        canonical_manifest_sha256: CANONICAL_MANIFEST_HASH.to_string(),
        canonical_files_verified,
        predecessor_trees,
        sem1_failed_run_preserved: inherited.sem1_failed_run_preserved,
        sem2_failed_run_preserved: inherited.sem2_failed_run_preserved,
        sem3_passing_run_sealed,
        sem3_run_id: freeze["run_id"].as_str().unwrap_or_default().to_string(),
        sem3_final_report_sha256: hash_bytes(&sem3_final_bytes),
        sem3_blind_manifest_sha256: hash_bytes(
            &fs::read(root.join("reports/sem3/frozen_blind_manifest.json"))
                .map_err(|e| e.to_string())?,
        ),
        promoted_concepts_verified_immutable: inherited.promoted_concepts_verified_immutable,
        network_enabled: quarantine.network_enabled,
        external_llm_enabled: quarantine.external_llm_enabled,
        local_teacher_enabled: false,
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

fn verify_canonical_manifest(root: &Path) -> Result<usize, String> {
    let raw = fs::read_to_string(root.join("docs/CANONICAL_MANIFEST.json"))
        .map_err(|error| error.to_string())?;
    let manifest: serde_json::Value =
        serde_json::from_str(&raw).map_err(|error| error.to_string())?;
    let self_hash = manifest["manifest_self_hash_sha256"]
        .as_str()
        .ok_or_else(|| "CANONICAL_SELF_HASH_MISSING".to_string())?;
    let normalized = raw.replacen(self_hash, &"0".repeat(64), 1);
    if self_hash != CANONICAL_MANIFEST_HASH || hash_bytes(normalized.as_bytes()) != self_hash {
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
    Ok(files.len())
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

pub fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    #[test]
    fn sem0_through_sem3_and_quarantine_are_immutable() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("root")
            .to_path_buf();
        let report = super::verify_predecessors(&root).expect("integrity");
        assert!(report.passed);
        assert!(report.sem3_passing_run_sealed);
        assert_eq!(report.predecessor_trees.len(), 4);
        assert_eq!(report.promoted_concepts_verified_immutable.len(), 4);
        assert!(!report.source_mutation);
    }
}
