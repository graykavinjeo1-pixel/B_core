use std::{fs, path::Path};

use serde::Serialize;
use sha2::{Digest, Sha256};
use synapse_recursive_core::quarantine::RecursiveImprovementQuarantine;

pub const PREDECESSOR_COMMIT: &str = "e61adb6f94ede876b7ddabd467473c848c7c0155";
pub const SEM5_TREE_HASH: &str = "b47dd3a0f174bb460000a45b115f4c48f3f1c034a22cceb0e9efae1a954ba8cc";

#[derive(Debug, Clone, Serialize)]
pub struct PredecessorTreeHash {
    pub relative_path: String,
    pub files: usize,
    pub sha256: String,
    pub expected_sha256: String,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Sem6PredecessorIntegrity {
    pub passed: bool,
    pub predecessor_commit: String,
    pub canonical_manifest_sha256: String,
    pub canonical_files_verified: usize,
    pub predecessor_trees: Vec<PredecessorTreeHash>,
    pub sem1_failed_run_preserved: bool,
    pub sem2_failed_run_preserved: bool,
    pub sem5_passing_run_sealed: bool,
    pub sem5_run_id: String,
    pub sem5_final_report_sha256: String,
    pub sem5_blind_manifest_sha256: String,
    pub promoted_concepts_verified_immutable: Vec<String>,
    pub recursive_improvement_quarantine: RecursiveImprovementQuarantine,
    pub network_enabled_before_firewall: bool,
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

pub fn verify_predecessors(root: &Path) -> Result<Sem6PredecessorIntegrity, String> {
    let inherited = crate::sem5::integrity::verify_predecessors(root)?;
    let (sha256, files) = hash_tree(root, "reports/sem5")?;
    let tree = PredecessorTreeHash {
        relative_path: "reports/sem5".to_string(),
        files,
        passed: sha256 == SEM5_TREE_HASH,
        sha256: sha256.clone(),
        expected_sha256: SEM5_TREE_HASH.to_string(),
    };
    if !tree.passed {
        return Err(format!("PREDECESSOR_INTEGRITY_FAILURE:SEM5_TREE:{sha256}"));
    }
    let final_bytes =
        fs::read(root.join("reports/sem5/sem5_final_report.json")).map_err(|e| e.to_string())?;
    let final_report: serde_json::Value =
        serde_json::from_slice(&final_bytes).map_err(|e| e.to_string())?;
    let freeze_bytes =
        fs::read(root.join("reports/sem5/freeze_record.json")).map_err(|e| e.to_string())?;
    let freeze: serde_json::Value =
        serde_json::from_slice(&freeze_bytes).map_err(|e| e.to_string())?;
    let sem5_passing_run_sealed = final_report["sem5_status"] == "PASS"
        && final_report["sem6_started"] == false
        && final_report["gates"]
            .as_object()
            .is_some_and(|gates| gates.len() == 12 && gates.values().all(|passed| passed == true))
        && freeze["run_id"] == "SEM5-RUN-0002"
        && freeze["frozen_before_final_tuning"] == true
        && freeze["solver_blind_access_before_freeze"] == false
        && freeze["post_blind_tuning"] == false;
    if !sem5_passing_run_sealed {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:SEM5_RUN_RECORD".to_string());
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
    let mut promoted = inherited.promoted_concepts_verified_immutable;
    promoted.extend([
        "C000008".to_string(),
        "C000009".to_string(),
        "C000010".to_string(),
    ]);
    Ok(Sem6PredecessorIntegrity {
        passed: true,
        predecessor_commit: PREDECESSOR_COMMIT.to_string(),
        canonical_manifest_sha256: inherited.canonical_manifest_sha256,
        canonical_files_verified: inherited.canonical_files_verified,
        predecessor_trees: vec![tree],
        sem1_failed_run_preserved: inherited.sem1_failed_run_preserved,
        sem2_failed_run_preserved: inherited.sem2_failed_run_preserved,
        sem5_passing_run_sealed,
        sem5_run_id: freeze["run_id"].as_str().unwrap_or_default().to_string(),
        sem5_final_report_sha256: hash_bytes(&final_bytes),
        sem5_blind_manifest_sha256: hash_bytes(
            &fs::read(root.join("reports/sem5/blind_manifest.json"))
                .map_err(|error| error.to_string())?,
        ),
        promoted_concepts_verified_immutable: promoted,
        network_enabled_before_firewall: false,
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
    fn sem0_through_sem5_and_quarantine_are_immutable_before_network() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("root")
            .to_path_buf();
        let report = super::verify_predecessors(&root).expect("integrity");
        assert!(report.passed);
        assert!(report.sem5_passing_run_sealed);
        assert_eq!(report.promoted_concepts_verified_immutable.len(), 9);
        assert!(!report.network_enabled_before_firewall);
        assert!(!report.source_mutation);
    }
}
