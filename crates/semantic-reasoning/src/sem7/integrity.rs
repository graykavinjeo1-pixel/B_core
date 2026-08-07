use std::{fs, path::Path};

use serde::Serialize;
use sha2::{Digest, Sha256};
use synapse_recursive_core::quarantine::RecursiveImprovementQuarantine;

pub const PREDECESSOR_COMMIT: &str = "94cf12299fa3142afe73fb88cc6a880d7ec61c39";
pub const SEM6_TREE_HASH: &str = "29deb5012dc736504ebeaf020faac8b73693701016bba922172ed91a3842c143";

#[derive(Debug, Clone, Serialize)]
pub struct PredecessorTreeHash {
    pub relative_path: String,
    pub files: usize,
    pub sha256: String,
    pub expected_sha256: String,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Sem7PredecessorIntegrity {
    pub passed: bool,
    pub predecessor_commit: String,
    pub canonical_manifest_sha256: String,
    pub canonical_files_verified: usize,
    pub sem6_tree: PredecessorTreeHash,
    pub sem6_run_id: String,
    pub sem6_final_report_sha256: String,
    pub sem6_blind_manifest_sha256: String,
    pub sem6_gates_verified: usize,
    pub sem6_frozen_without_post_tuning: bool,
    pub promoted_concepts_verified_immutable: Vec<String>,
    pub external_concepts_with_provenance: Vec<String>,
    pub existing_concepts_overwritten: usize,
    pub recursive_improvement_quarantine: RecursiveImprovementQuarantine,
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

pub fn verify_predecessors(root: &Path) -> Result<Sem7PredecessorIntegrity, String> {
    let inherited = crate::sem6::integrity::verify_predecessors(root)?;
    let (tree_sha256, files) = hash_tree(root, "reports/sem6")?;
    let sem6_tree = PredecessorTreeHash {
        relative_path: "reports/sem6".to_string(),
        files,
        passed: tree_sha256 == SEM6_TREE_HASH,
        sha256: tree_sha256.clone(),
        expected_sha256: SEM6_TREE_HASH.to_string(),
    };
    if !sem6_tree.passed || files != 31 {
        return Err(format!(
            "PREDECESSOR_INTEGRITY_FAILURE:SEM6_TREE:{tree_sha256}:{files}"
        ));
    }

    let final_bytes = fs::read(root.join("reports/sem6/sem6_final_report.json"))
        .map_err(|error| error.to_string())?;
    let final_report: crate::sem6::model::Sem6FinalReport =
        serde_json::from_slice(&final_bytes).map_err(|error| error.to_string())?;
    let freeze_bytes = fs::read(root.join("reports/sem6/freeze_record.json"))
        .map_err(|error| error.to_string())?;
    let freeze: crate::sem6::model::FreezeRecord =
        serde_json::from_slice(&freeze_bytes).map_err(|error| error.to_string())?;
    let sem6_frozen_without_post_tuning = final_report.sem6_status == "PASS"
        && !final_report.sem7_started
        && final_report.gates.len() == 11
        && final_report.gates.values().all(|passed| *passed)
        && freeze.run_id == "SEM6-RUN-0001"
        && freeze.frozen_before_final_tuning
        && !freeze.post_blind_tuning;
    if !sem6_frozen_without_post_tuning {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:SEM6_RUN_RECORD".to_string());
    }

    let promotion_bytes = fs::read(root.join("reports/sem6/external_concept_promotions.json"))
        .map_err(|error| error.to_string())?;
    let promotions: Vec<crate::sem6::model::ExternalConceptPromotion> =
        serde_json::from_slice(&promotion_bytes).map_err(|error| error.to_string())?;
    let external_concepts = promotions
        .iter()
        .filter(|promotion| promotion.promoted)
        .map(|promotion| {
            if !promotion.source_provenance_pass
                || promotion.candidate.provenance.is_empty()
                || !promotion.scope_version_validity_pass
            {
                return Err(format!(
                    "PREDECESSOR_INTEGRITY_FAILURE:EXTERNAL_PROVENANCE:{}",
                    promotion.candidate.concept_id
                ));
            }
            Ok(promotion.candidate.concept_id.clone())
        })
        .collect::<Result<Vec<_>, String>>()?;
    if external_concepts != ["C000011".to_string(), "C000012".to_string()] {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:EXTERNAL_PROMOTIONS".to_string());
    }
    let consolidation: Vec<crate::sem6::model::ConsolidationRecord> = serde_json::from_slice(
        &fs::read(root.join("reports/sem6/consolidation_ledger.json"))
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let overwritten = consolidation
        .iter()
        .map(|record| record.existing_concepts_overwritten)
        .sum();
    if overwritten != 0 || consolidation.iter().any(|record| !record.versioned_change) {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:SEM6_CONSOLIDATION".to_string());
    }

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
    let mut promoted = inherited.promoted_concepts_verified_immutable;
    promoted.extend(external_concepts.clone());
    Ok(Sem7PredecessorIntegrity {
        passed: true,
        predecessor_commit: PREDECESSOR_COMMIT.to_string(),
        canonical_manifest_sha256: inherited.canonical_manifest_sha256,
        canonical_files_verified: inherited.canonical_files_verified,
        sem6_tree,
        sem6_run_id: final_report.run_id,
        sem6_final_report_sha256: hash_bytes(&final_bytes),
        sem6_blind_manifest_sha256: hash_bytes(
            &fs::read(root.join("reports/sem6/sem6b_live_task_manifest.json"))
                .map_err(|error| error.to_string())?,
        ),
        sem6_gates_verified: final_report.gates.len(),
        sem6_frozen_without_post_tuning,
        promoted_concepts_verified_immutable: promoted,
        external_concepts_with_provenance: external_concepts,
        existing_concepts_overwritten: overwritten,
        recursive_improvement_quarantine: quarantine,
        external_llm_enabled: false,
        local_teacher_enabled: false,
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
    fn sem0_through_sem6_concepts_provenance_and_quarantine_are_immutable() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("root")
            .to_path_buf();
        let integrity = super::verify_predecessors(&root).expect("integrity");
        assert!(integrity.passed);
        assert_eq!(integrity.promoted_concepts_verified_immutable.len(), 11);
        assert_eq!(integrity.external_concepts_with_provenance.len(), 2);
        assert_eq!(integrity.existing_concepts_overwritten, 0);
        assert!(!integrity.source_mutation);
    }
}
