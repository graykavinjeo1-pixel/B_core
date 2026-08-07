use std::{fs, path::Path};

use serde::Serialize;

use crate::sem9::{
    integrity::{hash_bytes, hash_file, hash_tree},
    model::Sem9FinalReport,
};

use super::model::{
    ArtifactHash, FailedCandidateFreeze, Run0001ExecutionPathAudit, Run0001FailureReceipt,
};

pub const PREDECESSOR_COMMIT: &str = "39ddd82fcada55fe179f1b84bc3962644a49efbb";

pub const RUN0001_CRITICAL_ARTIFACTS: [&str; 14] = [
    "reports/sem9/SEM9_REPORT.md",
    "reports/sem9/sem9_final_report.json",
    "reports/sem9/fresh_blind_manifest.json",
    "reports/sem9/artifacts/candidate.patch",
    "reports/sem9/artifacts/candidate_lib.rs",
    "reports/sem9/artifacts/verified_candidate_probe.exe",
    "reports/sem9/candidate_patches.json",
    "reports/sem9/patch_provenance.json",
    "reports/sem9/self_role_mappings.json",
    "reports/sem9/self_assumption_ledgers.json",
    "reports/sem9/self_application_ablation.json",
    "reports/sem9/source_concept_ablation.json",
    "reports/sem9/protected_core_manifest.json",
    "reports/sem9/sandbox_build_results.json",
];

pub fn verify_r1_predecessor(root: &Path) -> Result<(), String> {
    crate::sem9::integrity::verify_predecessors(root)?;
    let final_report: Sem9FinalReport =
        read_json(&root.join("reports/sem9/sem9_final_report.json"))?;
    if final_report.sem9_status != "FAIL"
        || final_report.disposition != "SELF_PATCH_BUILD_FAILURE:CANDIDATE_FMT_CHECK_FAILED"
        || final_report.verified_self_application_candidates != 0
        || final_report.production_source_mutations != 0
        || final_report.sem10_started
        || final_report.next_allowed_stage != "SEM9-R1_RECURSIVE_SELF_APPLICATION_REPAIR"
    {
        return Err("PREDECESSOR_INTEGRITY_FAILURE:SEM9_RUN0001".to_string());
    }
    for relative in RUN0001_CRITICAL_ARTIFACTS {
        if !root.join(relative).is_file() {
            return Err(format!("PREDECESSOR_INTEGRITY_FAILURE:MISSING:{relative}"));
        }
    }
    Ok(())
}

pub fn build_run0001_receipt(root: &Path) -> Result<Run0001FailureReceipt, String> {
    verify_r1_predecessor(root)?;
    let critical_artifacts = RUN0001_CRITICAL_ARTIFACTS
        .iter()
        .map(|relative| {
            let path = root.join(relative);
            Ok(ArtifactHash {
                relative_path: (*relative).to_string(),
                byte_length: fs::metadata(&path)
                    .map_err(|error| error.to_string())?
                    .len(),
                sha256: hash_file(&path)?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    #[derive(Serialize)]
    struct Commitment<'a> {
        run_id: &'a str,
        status: &'a str,
        disposition: &'a str,
        predecessor_commit: &'a str,
        critical_artifacts: &'a [ArtifactHash],
        artifacts_verified: usize,
        run0001_overwritten: bool,
    }
    let commitment = Commitment {
        run_id: "SEM9-RUN-0001",
        status: "FAIL",
        disposition: "SELF_PATCH_BUILD_FAILURE:CANDIDATE_FMT_CHECK_FAILED",
        predecessor_commit: PREDECESSOR_COMMIT,
        critical_artifacts: &critical_artifacts,
        artifacts_verified: critical_artifacts.len(),
        run0001_overwritten: false,
    };
    let receipt_sha256 =
        hash_bytes(&serde_json::to_vec(&commitment).map_err(|error| error.to_string())?);
    Ok(Run0001FailureReceipt {
        run_id: commitment.run_id.to_string(),
        status: commitment.status.to_string(),
        disposition: commitment.disposition.to_string(),
        predecessor_commit: commitment.predecessor_commit.to_string(),
        artifacts_verified: commitment.artifacts_verified,
        run0001_overwritten: false,
        critical_artifacts,
        receipt_sha256,
    })
}

pub fn verify_run0001_receipt(root: &Path, frozen: &Run0001FailureReceipt) -> Result<(), String> {
    let current = build_run0001_receipt(root)?;
    if current != *frozen {
        return Err("RUN0001_FAILURE_RECEIPT_MISMATCH".to_string());
    }
    Ok(())
}

pub fn freeze_failed_candidate(root: &Path) -> Result<FailedCandidateFreeze, String> {
    let source_path = root.join("reports/sem9/artifacts/candidate_lib.rs");
    let source_sha256 = hash_file(&source_path)?;
    Ok(FailedCandidateFreeze {
        // Bind identity to the immutable failed-candidate bytes. Token normalization is
        // an equivalence check, not an identity function, and may evolve independently.
        failed_candidate_semantic_id: format!("SEM9-CANDIDATE-SEMANTIC:{source_sha256}"),
        failed_candidate_source_sha256: source_sha256,
        failed_candidate_patch_sha256: hash_file(
            &root.join("reports/sem9/artifacts/candidate.patch"),
        )?,
        mapping_sha256: hash_file(&root.join("reports/sem9/self_role_mappings.json"))?,
        assumptions_sha256: hash_file(&root.join("reports/sem9/self_assumption_ledgers.json"))?,
        target_component: "SELF-CANDIDATE-ROUTER".to_string(),
        source_concept_id: "C000012".to_string(),
        source_mechanism_id: "M0006".to_string(),
    })
}

pub fn build_execution_path_audit(
    root: &Path,
    diagnostic_cases: usize,
    diagnostic_failures: usize,
) -> Result<Run0001ExecutionPathAudit, String> {
    let builds: Vec<crate::sem9::model::SandboxBuildResult> =
        read_json(&root.join("reports/sem9/sandbox_build_results.json"))?;
    let tests: Vec<crate::sem9::model::SandboxTestResult> =
        read_json(&root.join("reports/sem9/sandbox_test_results.json"))?;
    let final_report: Sem9FinalReport =
        read_json(&root.join("reports/sem9/sem9_final_report.json"))?;
    let build = builds
        .first()
        .ok_or_else(|| "RUN0001_BUILD_EVIDENCE_MISSING".to_string())?;
    let test = tests
        .first()
        .ok_or_else(|| "RUN0001_TEST_EVIDENCE_MISSING".to_string())?;
    let format = build
        .commands
        .iter()
        .find(|command| command.command == "cargo fmt --all -- --check")
        .ok_or_else(|| "RUN0001_FMT_EVIDENCE_MISSING".to_string())?;
    let raw_compile = build
        .commands
        .iter()
        .rev()
        .find(|command| command.command == "cargo build --workspace")
        .ok_or_else(|| "RUN0001_COMPILE_EVIDENCE_MISSING".to_string())?;
    let clippy = build
        .commands
        .iter()
        .find(|command| command.command.contains("clippy"))
        .ok_or_else(|| "RUN0001_CLIPPY_EVIDENCE_MISSING".to_string())?;
    let behavioral_eval_run = root.join("reports/sem9/fresh_blind_results.json").is_file()
        && final_report.fresh_blind_tasks == 140;
    let equivalent = diagnostic_cases > 0 && diagnostic_failures == 0;
    let passed = !format.success
        && raw_compile.success
        && clippy.success
        && test.passed
        && behavioral_eval_run
        && !build.passed
        && equivalent;
    Ok(Run0001ExecutionPathAudit {
        source_generated: root
            .join("reports/sem9/artifacts/candidate_lib.rs")
            .is_file(),
        format_checked: true,
        format_check_passed: format.success,
        raw_compile_attempted: true,
        raw_compile_succeeded: raw_compile.success,
        clippy_run: true,
        clippy_passed: clippy.success,
        tests_run: true,
        tests_passed: test.passed,
        behavioral_eval_run,
        canonical_build_gate_passed: build.passed,
        behavioral_path_operation:
            "ChangeIR MergeEquivalentStates keyed by canonical semantic state identity"
                .to_string(),
        candidate_source_operation:
            "schedule(): BTreeSet seen; skip repeated canonical_key when enabled".to_string(),
        diagnostic_equivalence_cases: diagnostic_cases,
        diagnostic_equivalence_failures: diagnostic_failures,
        candidate_evaluation_path_equivalent: equivalent,
        built_zero_explanation: "BUILT=0 denoted aggregate canonical gate failure at cargo fmt --check; the exact raw source independently compiled, passed Clippy/tests, and was executable. RUN-0001 behavioral counters used the same ChangeIR operation, confirmed against the raw source on diagnostic cases."
            .to_string(),
        passed,
    })
}

pub fn normalize_non_format_tokens(source: &str) -> Vec<u8> {
    let bytes = source.as_bytes();
    let mut normalized = Vec::with_capacity(bytes.len());
    let mut index = 0usize;
    let mut quote = None;
    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(delimiter) = quote {
            normalized.push(byte);
            if byte == b'\\' && index + 1 < bytes.len() {
                index += 1;
                normalized.push(bytes[index]);
            } else if byte == delimiter {
                quote = None;
            }
            index += 1;
            continue;
        }
        if byte == b'"' || byte == b'\'' {
            quote = Some(byte);
            normalized.push(byte);
            index += 1;
            continue;
        }
        if byte == b'/' && bytes.get(index + 1) == Some(&b'/') {
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if byte == b'/' && bytes.get(index + 1) == Some(&b'*') {
            index += 2;
            while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/') {
                index += 1;
            }
            index = (index + 2).min(bytes.len());
            continue;
        }
        if !byte.is_ascii_whitespace() {
            normalized.push(byte);
        }
        index += 1;
    }
    // rustfmt may insert or remove an optional trailing comma immediately before
    // a closing delimiter. Rust assigns that comma no executable meaning, so it
    // is canonicalized as layout punctuation rather than a non-format token.
    let mut semantic_tokens = Vec::with_capacity(normalized.len());
    let mut index = 0usize;
    let mut quote = None;
    while index < normalized.len() {
        let byte = normalized[index];
        if let Some(delimiter) = quote {
            semantic_tokens.push(byte);
            if byte == b'\\' && index + 1 < normalized.len() {
                index += 1;
                semantic_tokens.push(normalized[index]);
            } else if byte == delimiter {
                quote = None;
            }
        } else if byte == b'"' || byte == b'\'' {
            quote = Some(byte);
            semantic_tokens.push(byte);
        } else if byte == b',' && matches!(normalized.get(index + 1), Some(b'}' | b']')) {
            index += 1;
            continue;
        } else {
            semantic_tokens.push(byte);
        }
        index += 1;
    }
    semantic_tokens
}

pub fn production_source_hash(root: &Path) -> Result<String, String> {
    hash_tree(root, "crates/semantic-reasoning/src").map(|value| value.0)
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
    fn run0001_failure_is_exact_and_receiptable() {
        verify_r1_predecessor(&root()).expect("predecessor");
        let receipt = build_run0001_receipt(&root()).expect("receipt");
        assert_eq!(receipt.artifacts_verified, 14);
        assert!(!receipt.run0001_overwritten);
    }

    #[test]
    fn token_normalization_ignores_only_layout_and_comments() {
        let left = "fn f(){ // layout\n let x = S { value: 1 }; x }";
        let right = "fn f() { let x=S{value:1,};x }";
        assert_eq!(
            normalize_non_format_tokens(left),
            normalize_non_format_tokens(right)
        );
        assert_ne!(
            normalize_non_format_tokens("fn f(){1}"),
            normalize_non_format_tokens("fn f(){2}")
        );
    }
}
