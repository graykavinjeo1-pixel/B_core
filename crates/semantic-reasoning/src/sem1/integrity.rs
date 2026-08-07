use std::{fs, path::Path};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use synapse_recursive_core::quarantine::{
    status as quarantine_status, RecursiveImprovementQuarantine,
};

use crate::substrate::{ConceptIR, PromotionState};

pub const PREDECESSOR_COMMIT: &str = "bae05f1023d7abd65bcfd2272cbf9d0a6dda9e48";
pub const CANONICAL_MANIFEST_HASH: &str =
    "ea9b6cc8fafb9f6f40960a508b12686b948af9f8bb977bc2e53a7b97471a0b0b";

const SEALED_SEM0_FILES: &[(&str, &str)] = &[
    (
        "ablation_results.json",
        "806c67d227c76323148f720e7d62b5bc9574bf952e44b7fc65dffa2b2bf71d97",
    ),
    (
        "baseline_results.json",
        "5fd4e814c39c40f48e9e13b0d0c9b89967f3bb7ed13d4b6c4a404ad09175f54f",
    ),
    (
        "candidate_concepts.json",
        "75d7052e2720b68e0fb0c867e7e271137332091ef4b5d0cc088c7f11f409ac9b",
    ),
    (
        "counterfactual_results.json",
        "c849c45663982ee63819a5929c2b779136e84314b6711980179586df5a112312",
    ),
    (
        "derivation_metrics.json",
        "5c03e52bdf65a1f8aab607f669716c97de2f3f15c3647e59e8d8e5f1f58a1bc5",
    ),
    (
        "environment.json",
        "f1829b19a3ec5ff66d619226d786b09bd10f35a269dfc0493655e661c0ff4028",
    ),
    (
        "fresh_blind_results.json",
        "2c5291211354a933f44f69d6641c85379a9ee32455468e9bdeac15732a0ba1bb",
    ),
    (
        "leakage_audit.json",
        "2a484756b1c0e87fe8a2e2df17c0ef3ec6cbfff8e35cf062dc0f740ca085353b",
    ),
    (
        "lineage_graph.json",
        "0752530f590312c296998b894e2ff115721b62b1df01504546ecd87327145e1b",
    ),
    (
        "posthoc_interpretation.json",
        "1269382421dcf945afcc133d6a41d05e899194ebe28bd57d5ae1b4fcd9c24559",
    ),
    (
        "primitive_catalog.json",
        "45a33012276385d23b64dd7e1238c24270b6e7025b05f4e3dc302110530732e3",
    ),
    (
        "sem0_final_report.json",
        "bc90cd83389c3df9be53d558b0bb040947a8724255ff27a704a1c20b21960aa1",
    ),
    (
        "SEM0_REPORT.md",
        "7cba0a030385fb6799f554c86dd5c797f4ab1beadb9df2d36d8c6b10f9a68717",
    ),
    (
        "semantic_gate_results.json",
        "b6f35cd833238c34008e659fe677875194e89dd9d2a44356b9ea736e37eb4d2f",
    ),
    (
        "task_manifest_blind.json",
        "e552d30671b149145baf13648f79456e66fc75638cecf19f55ca3581d60de9a9",
    ),
    (
        "task_manifest_train.json",
        "563643065f9e952b8792586368dfff9f29c819d975d5a5407e0e62719621bf5f",
    ),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactHash {
    pub relative_path: String,
    pub byte_length: usize,
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
    pub sem0_artifacts_verified: usize,
    pub sem0_artifact_hashes: Vec<ArtifactHash>,
    pub c000001_record_sha256: String,
    pub c000001_content_hash_sha256: String,
    pub c000001_recomputed_content_hash_sha256: String,
    pub c000001_immutable_promotion_record: bool,
    pub recursive_improvement_quarantine: RecursiveImprovementQuarantine,
    pub self_observe: bool,
    pub self_measure: bool,
    pub self_propose: bool,
    pub self_apply: bool,
    pub source_mutation: bool,
}

#[derive(Debug, Deserialize)]
struct CandidateFile {
    concepts: Vec<ConceptIR>,
}

pub fn verify_and_load(root: &Path) -> Result<(PredecessorIntegrityReport, ConceptIR), String> {
    let canonical_files_verified = verify_canonical_manifest(root)?;
    let (sem0_artifact_hashes, sem0_artifacts_verified) = verify_sem0_artifacts(root)?;
    let candidate_path = root.join("reports/sem0/candidate_concepts.json");
    let candidate_bytes = fs::read(&candidate_path).map_err(|error| error.to_string())?;
    let candidate_file: CandidateFile =
        serde_json::from_slice(&candidate_bytes).map_err(|error| error.to_string())?;
    let predecessor = candidate_file
        .concepts
        .into_iter()
        .find(|concept| concept.concept_id == "C000001")
        .ok_or_else(|| "C000001_MISSING".to_string())?;
    let stored_hash = predecessor.content_hash_sha256.clone();
    let mut recomputed = predecessor.clone();
    recomputed
        .freeze_hash()
        .map_err(|error| error.to_string())?;
    let immutable = predecessor.promotion_state == PromotionState::Promoted
        && predecessor.kind == crate::substrate::ConceptKind::Promoted
        && stored_hash == recomputed.content_hash_sha256
        && predecessor.version == 2
        && predecessor.provenance.parent_concept_ids.is_empty()
        && !predecessor.provenance.lexical_information_used
        && !predecessor.provenance.supplied_by_teacher;
    if !immutable {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:C000001".to_string());
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

    Ok((
        PredecessorIntegrityReport {
            passed: true,
            predecessor_commit: PREDECESSOR_COMMIT.to_string(),
            canonical_manifest_sha256: CANONICAL_MANIFEST_HASH.to_string(),
            canonical_files_verified,
            sem0_artifacts_verified,
            sem0_artifact_hashes,
            c000001_record_sha256: hash_bytes(&candidate_bytes),
            c000001_content_hash_sha256: stored_hash,
            c000001_recomputed_content_hash_sha256: recomputed.content_hash_sha256,
            c000001_immutable_promotion_record: true,
            recursive_improvement_quarantine: quarantine,
            self_observe: true,
            self_measure: true,
            self_propose: false,
            self_apply: false,
            source_mutation: false,
        },
        predecessor,
    ))
}

fn verify_sem0_artifacts(root: &Path) -> Result<(Vec<ArtifactHash>, usize), String> {
    let mut reports = Vec::new();
    for (name, expected) in SEALED_SEM0_FILES {
        let path = root.join("reports/sem0").join(name);
        let bytes = fs::read(&path).map_err(|error| format!("{}:{error}", path.display()))?;
        let actual = hash_bytes(&bytes);
        let passed = actual == *expected;
        reports.push(ArtifactHash {
            relative_path: format!("reports/sem0/{name}"),
            byte_length: bytes.len(),
            sha256: actual,
            expected_sha256: (*expected).to_string(),
            passed,
        });
    }
    if reports.iter().any(|report| !report.passed) {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:SEM0_ARTIFACT_DRIFT".to_string());
    }
    let count = reports.len();
    Ok((reports, count))
}

fn verify_canonical_manifest(root: &Path) -> Result<usize, String> {
    let path = root.join("docs/CANONICAL_MANIFEST.json");
    let raw = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let manifest: serde_json::Value =
        serde_json::from_str(&raw).map_err(|error| error.to_string())?;
    let self_hash = manifest["manifest_self_hash_sha256"]
        .as_str()
        .ok_or_else(|| "CANONICAL_SELF_HASH_MISSING".to_string())?;
    let normalized = raw.replacen(self_hash, &"0".repeat(64), 1);
    if hash_bytes(normalized.as_bytes()) != self_hash || self_hash != CANONICAL_MANIFEST_HASH {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:CANONICAL_MANIFEST".to_string());
    }
    let entries = manifest["canonical_files"]
        .as_array()
        .ok_or_else(|| "CANONICAL_FILES_MISSING".to_string())?;
    for entry in entries {
        let relative = entry["relative_path"]
            .as_str()
            .ok_or_else(|| "CANONICAL_PATH_MISSING".to_string())?;
        let expected_length = entry["byte_length"]
            .as_u64()
            .ok_or_else(|| "CANONICAL_LENGTH_MISSING".to_string())?
            as usize;
        let expected_hash = entry["sha256"]
            .as_str()
            .ok_or_else(|| "CANONICAL_HASH_MISSING".to_string())?;
        let bytes = fs::read(root.join(relative)).map_err(|error| error.to_string())?;
        if bytes.len() != expected_length || hash_bytes(&bytes) != expected_hash {
            return Err(format!("PREDECESSOR_INTEGRITY_FAILURE:{relative}"));
        }
    }
    Ok(entries.len())
}

pub fn hash_serializable<T: Serialize>(value: &T) -> Result<String, String> {
    let bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    Ok(hash_bytes(&bytes))
}

pub fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    #[test]
    fn sealed_predecessor_verifies_and_recomputes_c000001_hash() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root = manifest_dir
            .parent()
            .and_then(|path| path.parent())
            .expect("workspace root");
        let (report, concept) = super::verify_and_load(root).expect("sealed predecessor");
        assert!(report.passed);
        assert!(report.c000001_immutable_promotion_record);
        assert_eq!(concept.concept_id, "C000001");
        assert_eq!(report.sem0_artifacts_verified, 16);
    }
}
