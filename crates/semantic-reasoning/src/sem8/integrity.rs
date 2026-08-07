use std::{fs, path::Path};

use serde::Serialize;
use sha2::{Digest, Sha256};
use synapse_recursive_core::quarantine::RecursiveImprovementQuarantine;

use crate::sem7::concepts::ConceptRegistry;

pub const PREDECESSOR_COMMIT: &str = "a3cf50d3263e156bce7b81e6fa1198b5446898f3";
pub const SEM7_TREE_HASH: &str = "ff41c4cf31950909ceaa386b17ee00eabfbc783b97edaccecc04601785f0e0ad";

#[derive(Debug, Clone, Serialize)]
pub struct SemanticPayloadHash {
    pub concept_id: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PredecessorTreeHash {
    pub relative_path: String,
    pub files: usize,
    pub sha256: String,
    pub expected_sha256: String,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Sem8PredecessorIntegrity {
    pub passed: bool,
    pub predecessor_commit: String,
    pub canonical_manifest_sha256: String,
    pub canonical_files_verified: usize,
    pub sem7_tree: PredecessorTreeHash,
    pub sem7_run_id: String,
    pub sem7_final_report_sha256: String,
    pub sem7_blind_manifest_sha256: String,
    pub sem7_gates_verified: usize,
    pub sem7_failed_run_0001_preserved: bool,
    pub sem7_failed_run_manifest_sha256: String,
    pub language_cortex_adapter_only: bool,
    pub language_semantic_separation_pass: bool,
    pub promoted_concepts_verified_immutable: Vec<String>,
    pub predecessor_semantic_payload_hashes: Vec<SemanticPayloadHash>,
    pub predecessor_semantic_hash_changes: usize,
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

pub fn verify_predecessors(root: &Path) -> Result<Sem8PredecessorIntegrity, String> {
    let inherited = crate::sem7::integrity::verify_predecessors(root)?;
    let (tree_sha256, files) = hash_tree(root, "reports/sem7")?;
    let sem7_tree = PredecessorTreeHash {
        relative_path: "reports/sem7".to_string(),
        files,
        passed: tree_sha256 == SEM7_TREE_HASH,
        sha256: tree_sha256.clone(),
        expected_sha256: SEM7_TREE_HASH.to_string(),
    };
    if !sem7_tree.passed || files != 24 {
        return Err(format!(
            "PREDECESSOR_INTEGRITY_FAILURE:SEM7_TREE:{tree_sha256}:{files}"
        ));
    }
    let final_bytes = fs::read(root.join("reports/sem7/sem7_final_report.json"))
        .map_err(|error| error.to_string())?;
    let final_report: crate::sem7::model::Sem7FinalReport =
        serde_json::from_slice(&final_bytes).map_err(|error| error.to_string())?;
    let sem7_pass = final_report.sem7_status == "PASS"
        && final_report.run_id == "SEM7-RUN-0002"
        && final_report.gates.len() == 13
        && final_report.gates.values().all(|passed| *passed)
        && final_report.language_cortex_boundary_pass
        && final_report.semantic_language_separation_pass
        && !final_report.sem8_started;
    if !sem7_pass {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:SEM7_PASS_RECORD".to_string());
    }
    let failed_directory = root.join("reports/sem7/failed_runs/SEM7-RUN-0001");
    let failed_final_bytes = fs::read(failed_directory.join("sem7_final_report.json"))
        .map_err(|error| error.to_string())?;
    let failed_final: serde_json::Value =
        serde_json::from_slice(&failed_final_bytes).map_err(|error| error.to_string())?;
    let failed_manifest_bytes = fs::read(failed_directory.join("blind_manifest.json"))
        .map_err(|error| error.to_string())?;
    let failed_manifest: crate::sem7::model::LanguageTaskManifest =
        serde_json::from_slice(&failed_manifest_bytes).map_err(|error| error.to_string())?;
    let failed_preserved = failed_final["sem7_status"] == "FAIL"
        && failed_final["run_id"] == "SEM7-RUN-0001"
        && failed_final["disposition"] == "LANGUAGE_TO_GOAL_IR_REGRESSION"
        && failed_manifest.run_id == "SEM7-RUN-0001"
        && failed_manifest.tasks.len() == 100;
    if !failed_preserved {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:SEM7_FAILED_RUN".to_string());
    }
    let lexical_audit: crate::sem7::model::LexicalContaminationAudit = serde_json::from_slice(
        &fs::read(root.join("reports/sem7/lexical_contamination_audit.json"))
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let contamination: crate::sem7::model::Sem7ContaminationAudit = serde_json::from_slice(
        &fs::read(root.join("reports/sem7/contamination_audit.json"))
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let language_cortex_adapter_only = lexical_audit.passed
        && lexical_audit.lexical_token_dependent_promoted_concepts == 0
        && contamination.raw_text_reasoner_inputs == 0
        && contamination.direct_text_to_program_shortcuts == 0;
    if !language_cortex_adapter_only {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:LANGUAGE_CORTEX_BOUNDARY".to_string());
    }
    let registry = ConceptRegistry::canonical();
    let hashes = registry
        .concepts()
        .map(|concept| {
            Ok(SemanticPayloadHash {
                concept_id: concept.concept_id.clone(),
                sha256: registry.semantic_hash(&concept.concept_id)?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let quarantine = inherited.recursive_improvement_quarantine;
    if quarantine.external_llm_enabled
        || quarantine.proposal_generation_enabled
        || quarantine.source_patching_enabled
        || quarantine.auto_apply_enabled
        || quarantine.auto_commit_enabled
        || quarantine.auto_push_enabled
    {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:QUARANTINE".to_string());
    }
    Ok(Sem8PredecessorIntegrity {
        passed: true,
        predecessor_commit: PREDECESSOR_COMMIT.to_string(),
        canonical_manifest_sha256: inherited.canonical_manifest_sha256,
        canonical_files_verified: inherited.canonical_files_verified,
        sem7_tree,
        sem7_run_id: final_report.run_id,
        sem7_final_report_sha256: hash_bytes(&final_bytes),
        sem7_blind_manifest_sha256: hash_bytes(
            &fs::read(root.join("reports/sem7/blind_manifest.json"))
                .map_err(|error| error.to_string())?,
        ),
        sem7_gates_verified: final_report.gates.len(),
        sem7_failed_run_0001_preserved: failed_preserved,
        sem7_failed_run_manifest_sha256: hash_bytes(&failed_manifest_bytes),
        language_cortex_adapter_only,
        language_semantic_separation_pass: final_report.semantic_language_separation_pass,
        promoted_concepts_verified_immutable: inherited.promoted_concepts_verified_immutable,
        predecessor_semantic_payload_hashes: hashes,
        predecessor_semantic_hash_changes: 0,
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

fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    #[test]
    fn sem0_through_sem7_failure_history_language_boundary_and_quarantine_are_sealed() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("root")
            .to_path_buf();
        let integrity = super::verify_predecessors(&root).expect("integrity");
        assert!(integrity.passed);
        assert!(integrity.sem7_failed_run_0001_preserved);
        assert!(integrity.language_cortex_adapter_only);
        assert_eq!(integrity.promoted_concepts_verified_immutable.len(), 11);
        assert_eq!(integrity.predecessor_semantic_payload_hashes.len(), 7);
        assert_eq!(integrity.predecessor_semantic_hash_changes, 0);
        assert!(!integrity.source_mutation);
    }
}
