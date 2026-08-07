use std::{fs, path::Path};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{
    model::{ProtectedCoreEntry, ProtectedCoreManifest},
    tasks::hash_serializable,
};

pub const PREDECESSOR_COMMIT: &str = "8028670e9870afd2f5961116f5e7d7d52ff8339d";
pub const SEM8_TREE_HASH: &str = "0b61431f13f428d47f0ec482816791008e79bc62c0d224110aba97c8439bbb35";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeSeal {
    pub relative_path: String,
    pub files: usize,
    pub sha256: String,
    pub expected_sha256: Option<String>,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sem9PredecessorIntegrity {
    pub passed: bool,
    pub predecessor_commit: String,
    pub canonical_manifest_sha256: String,
    pub canonical_files_verified: usize,
    pub sem8_tree: TreeSeal,
    pub sem8_run_id: String,
    pub sem8_final_report_sha256: String,
    pub sem8_blind_manifest_sha256: String,
    pub sem8_transfer_artifacts_verified: usize,
    pub sem8_gates_verified: usize,
    pub sem7_failed_run_history_preserved: bool,
    pub promoted_concepts_verified_immutable: Vec<String>,
    pub cross_domain_concept_c000013_sealed: bool,
    pub predecessor_semantic_hash_changes: usize,
    pub sparse_routing_preserved: bool,
    pub recursive_improvement_quarantine_preserved: bool,
}

pub fn verify_predecessors(root: &Path) -> Result<Sem9PredecessorIntegrity, String> {
    let inherited = crate::sem8::integrity::verify_predecessors(root)?;
    let (sem8_sha256, files) = hash_tree(root, "reports/sem8")?;
    let sem8_tree = TreeSeal {
        relative_path: "reports/sem8".to_string(),
        files,
        sha256: sem8_sha256.clone(),
        expected_sha256: Some(SEM8_TREE_HASH.to_string()),
        passed: files == 27 && sem8_sha256 == SEM8_TREE_HASH,
    };
    if !sem8_tree.passed {
        return Err(format!(
            "PREDECESSOR_INTEGRITY_FAILURE:SEM8_TREE:{files}:{sem8_sha256}"
        ));
    }
    let final_bytes = fs::read(root.join("reports/sem8/sem8_final_report.json"))
        .map_err(|error| error.to_string())?;
    let final_report: crate::sem8::model::Sem8FinalReport =
        serde_json::from_slice(&final_bytes).map_err(|error| error.to_string())?;
    let sem8_pass = final_report.sem8_status == "PASS"
        && final_report.run_id == "SEM8-RUN-0001"
        && final_report.gates.len() == 12
        && final_report.gates.values().all(|passed| *passed)
        && final_report.full_catalog_scans == 0
        && final_report.routing_false_negatives == 0
        && final_report.recursive_source_mutations == 0
        && !final_report.sem9_started;
    if !sem8_pass {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:SEM8_PASS_RECORD".to_string());
    }
    let promotions: Vec<crate::sem8::model::CrossDomainPromotion> =
        read_json(&root.join("reports/sem8/cross_domain_promotions.json"))?;
    let c000013 = promotions.iter().any(|promotion| {
        promotion.promoted
            && promotion.promoted_concept_id.as_deref() == Some("C000013")
            && promotion.predecessor_concepts_overwritten == 0
    });
    if !c000013 {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:C000013".to_string());
    }
    let sparse: crate::sem8::model::SparseTransferAudit =
        read_json(&root.join("reports/sem8/sparse_activation_audit.json"))?;
    if !sparse.passed || sparse.full_catalog_scans != 0 || sparse.routing_false_negatives != 0 {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:SPARSE_ROUTING".to_string());
    }
    let transfer_artifacts = [
        "source_mechanism_catalog.json",
        "role_mapping_results.json",
        "assumption_ledger.json",
        "transfer_ablation.json",
        "transfer_leakage_audit.json",
    ];
    for artifact in transfer_artifacts {
        if !root.join("reports/sem8").join(artifact).is_file() {
            return Err(format!(
                "PREDECESSOR_INTEGRITY_FAILURE:MISSING_SEM8_ARTIFACT:{artifact}"
            ));
        }
    }
    let quarantine = inherited.recursive_improvement_quarantine;
    let quarantine_preserved = !quarantine.external_llm_enabled
        && !quarantine.proposal_generation_enabled
        && !quarantine.source_patching_enabled
        && !quarantine.auto_apply_enabled
        && !quarantine.auto_commit_enabled
        && !quarantine.auto_push_enabled;
    if !quarantine_preserved {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:QUARANTINE".to_string());
    }
    Ok(Sem9PredecessorIntegrity {
        passed: true,
        predecessor_commit: PREDECESSOR_COMMIT.to_string(),
        canonical_manifest_sha256: inherited.canonical_manifest_sha256,
        canonical_files_verified: inherited.canonical_files_verified,
        sem8_tree,
        sem8_run_id: final_report.run_id,
        sem8_final_report_sha256: hash_bytes(&final_bytes),
        sem8_blind_manifest_sha256: hash_file(
            &root.join("reports/sem8/blind_target_manifest.json"),
        )?,
        sem8_transfer_artifacts_verified: transfer_artifacts.len(),
        sem8_gates_verified: final_report.gates.len(),
        sem7_failed_run_history_preserved: inherited.sem7_failed_run_0001_preserved,
        promoted_concepts_verified_immutable: inherited.promoted_concepts_verified_immutable,
        cross_domain_concept_c000013_sealed: c000013,
        predecessor_semantic_hash_changes: 0,
        sparse_routing_preserved: true,
        recursive_improvement_quarantine_preserved: quarantine_preserved,
    })
}

pub fn build_protected_core_manifest(
    root: &Path,
    run_id: &str,
) -> Result<ProtectedCoreManifest, String> {
    let protected = [
        ("CONSTITUTION", "CONSTITUTION.md"),
        ("RESEARCH_HYPOTHESIS", "RESEARCH_HYPOTHESIS.md"),
        ("CANONICAL_MANIFEST", "docs/CANONICAL_MANIFEST.json"),
        (
            "PROOF_KERNEL_AUTHORITY",
            "crates/semantic-reasoning/src/sem4/kernel.rs",
        ),
        (
            "PROMOTION_GATES",
            "crates/semantic-reasoning/src/sem8/experiment.rs",
        ),
        (
            "BLIND_EVALUATOR",
            "crates/semantic-reasoning/src/sem9/tasks.rs",
        ),
        (
            "LEAKAGE_EVALUATOR",
            "crates/semantic-reasoning/src/sem9/experiment.rs",
        ),
        (
            "SECURITY_SANDBOX_BOUNDARY",
            "crates/synapse-recursive-core/src/quarantine.rs",
        ),
    ];
    let entries = protected
        .into_iter()
        .map(|(component_id, relative_path)| {
            Ok(ProtectedCoreEntry {
                component_id: component_id.to_string(),
                relative_path: relative_path.to_string(),
                sha256: hash_file(&root.join(relative_path))?,
                mutation_allowed: false,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let (source_tree_sha256, _) = hash_tree(root, "crates/semantic-reasoning/src")?;
    let (evaluator_tree_sha256, _) = hash_tree(root, "crates/semantic-reasoning/src/sem9")?;
    #[derive(Serialize)]
    struct Commitment<'a> {
        run_id: &'a str,
        entries: &'a [ProtectedCoreEntry],
        source_tree_sha256: &'a str,
        evaluator_tree_sha256: &'a str,
        mutation_authority_enabled: bool,
        frozen_before_proposals: bool,
    }
    let commitment = Commitment {
        run_id,
        entries: &entries,
        source_tree_sha256: &source_tree_sha256,
        evaluator_tree_sha256: &evaluator_tree_sha256,
        mutation_authority_enabled: false,
        frozen_before_proposals: true,
    };
    let manifest_sha256 = hash_serializable(&commitment);
    Ok(ProtectedCoreManifest {
        run_id: run_id.to_string(),
        entries,
        source_tree_sha256,
        evaluator_tree_sha256,
        mutation_authority_enabled: false,
        frozen_before_proposals: true,
        manifest_sha256,
    })
}

pub fn verify_protected_core_manifest(
    root: &Path,
    frozen: &ProtectedCoreManifest,
) -> Result<(), String> {
    let current = build_protected_core_manifest(root, &frozen.run_id)?;
    if current != *frozen {
        return Err("PROTECTED_CORE_MUTATION_ATTEMPT:MANIFEST_MISMATCH".to_string());
    }
    Ok(())
}

pub fn hash_tree(root: &Path, relative: &str) -> Result<(String, usize), String> {
    let mut paths = Vec::new();
    collect_files(&root.join(relative), &mut paths).map_err(|error| error.to_string())?;
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

pub fn hash_file(path: &Path) -> Result<String, String> {
    fs::read(path)
        .map(|bytes| hash_bytes(&bytes))
        .map_err(|error| error.to_string())
}

pub fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    serde_json::from_slice(&fs::read(path).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("root")
            .to_path_buf()
    }

    #[test]
    fn sem0_through_sem8_transfer_and_quarantine_are_sealed() {
        let integrity = verify_predecessors(&root()).expect("integrity");
        assert!(integrity.passed);
        assert_eq!(integrity.sem8_tree.files, 27);
        assert!(integrity.cross_domain_concept_c000013_sealed);
        assert!(integrity.sem7_failed_run_history_preserved);
        assert_eq!(integrity.predecessor_semantic_hash_changes, 0);
        assert!(integrity.recursive_improvement_quarantine_preserved);
    }

    #[test]
    fn protected_core_manifest_is_immutable_and_disables_mutation_authority() {
        let manifest = build_protected_core_manifest(&root(), "test").expect("manifest");
        assert!(!manifest.mutation_authority_enabled);
        assert!(manifest.entries.iter().all(|entry| !entry.mutation_allowed));
        verify_protected_core_manifest(&root(), &manifest).expect("verify");
    }
}
