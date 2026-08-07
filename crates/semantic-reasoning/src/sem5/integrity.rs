use std::{fs, path::Path};

use serde::Serialize;
use sha2::{Digest, Sha256};
use synapse_recursive_core::quarantine::RecursiveImprovementQuarantine;

pub const PREDECESSOR_COMMIT: &str = "79367ade96e457848819461e55cc983329ffab52";
pub const SEM0_TREE_HASH: &str = "6ccf0423ca5c7106d70492f107c23980b9e9f31a807f778f16d66462e2558cbe";
pub const SEM1_TREE_HASH: &str = "b5083b272995fbeabb735608db43b08d289359e5f03601cc91b4eb99756f87f8";
pub const SEM2_TREE_HASH: &str = "9e0f2ee39cd7ea11f60a66842f650c0f269054a6cdcaa061e4de02f2bbb37e0c";
pub const SEM3_TREE_HASH: &str = "90ad74b1a86ee455a3d20ac52d906a6ee82435b163665580c9d8b8ac5720abdd";
pub const SEM4_TREE_HASH: &str = "1abcd9cbe97ffa8bdeb28444137d7b5a7348868e632cb47defac0f73c2d0c11e";

#[derive(Debug, Clone, Serialize)]
pub struct PredecessorTreeHash {
    pub relative_path: String,
    pub files: usize,
    pub sha256: String,
    pub expected_sha256: String,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Sem5PredecessorIntegrity {
    pub passed: bool,
    pub predecessor_commit: String,
    pub canonical_manifest_sha256: String,
    pub canonical_files_verified: usize,
    pub predecessor_trees: Vec<PredecessorTreeHash>,
    pub sem1_failed_run_preserved: bool,
    pub sem2_failed_run_preserved: bool,
    pub sem4_passing_run_sealed: bool,
    pub sem4_run_id: String,
    pub sem4_final_report_sha256: String,
    pub sem4_blind_manifest_sha256: String,
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

pub fn verify_predecessors(root: &Path) -> Result<Sem5PredecessorIntegrity, String> {
    let inherited = crate::sem4::integrity::verify_predecessors(root)?;
    let expected_trees = [
        ("reports/sem0", SEM0_TREE_HASH),
        ("reports/sem1", SEM1_TREE_HASH),
        ("reports/sem2", SEM2_TREE_HASH),
        ("reports/sem3", SEM3_TREE_HASH),
        ("reports/sem4", SEM4_TREE_HASH),
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
    let final_bytes =
        fs::read(root.join("reports/sem4/sem4_final_report.json")).map_err(|e| e.to_string())?;
    let final_report: serde_json::Value =
        serde_json::from_slice(&final_bytes).map_err(|e| e.to_string())?;
    let freeze_bytes =
        fs::read(root.join("reports/sem4/freeze_record.json")).map_err(|e| e.to_string())?;
    let freeze: serde_json::Value =
        serde_json::from_slice(&freeze_bytes).map_err(|e| e.to_string())?;
    let sem4_passing_run_sealed = final_report["sem4_status"] == "PASS"
        && final_report["sem5_started"] == false
        && final_report["gates"].as_array().is_some_and(|gates| {
            gates.len() == 9 && gates.iter().all(|gate| gate["passed"] == true)
        })
        && freeze["run_id"] == "SEM4-RUN-0001"
        && freeze["frozen_before_final_tuning"] == true
        && freeze["reasoner_blind_access_before_freeze"] == false
        && freeze["post_blind_tuning"] == false;
    if !sem4_passing_run_sealed {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:SEM4_RUN_RECORD".to_string());
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
    promoted.extend(["C000006".to_string(), "C000007".to_string()]);
    Ok(Sem5PredecessorIntegrity {
        passed: true,
        predecessor_commit: PREDECESSOR_COMMIT.to_string(),
        canonical_manifest_sha256: inherited.canonical_manifest_sha256,
        canonical_files_verified: inherited.canonical_files_verified,
        predecessor_trees,
        sem1_failed_run_preserved: inherited.sem1_failed_run_preserved,
        sem2_failed_run_preserved: inherited.sem2_failed_run_preserved,
        sem4_passing_run_sealed,
        sem4_run_id: freeze["run_id"].as_str().unwrap_or_default().to_string(),
        sem4_final_report_sha256: hash_bytes(&final_bytes),
        sem4_blind_manifest_sha256: hash_bytes(
            &fs::read(root.join("reports/sem4/blind_manifest.json"))
                .map_err(|error| error.to_string())?,
        ),
        promoted_concepts_verified_immutable: promoted,
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
    fn sem0_through_sem4_and_quarantine_are_immutable() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("root")
            .to_path_buf();
        let report = super::verify_predecessors(&root).expect("integrity");
        assert!(report.passed);
        assert!(report.sem4_passing_run_sealed);
        assert_eq!(report.predecessor_trees.len(), 5);
        assert_eq!(report.promoted_concepts_verified_immutable.len(), 6);
        assert!(!report.source_mutation);
    }
}
