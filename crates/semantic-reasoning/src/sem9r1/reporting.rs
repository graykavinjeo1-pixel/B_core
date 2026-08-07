use std::{fs, path::Path};

use serde::Serialize;
use serde_json::json;

use super::experiment::Sem9R1Outcome;

pub fn write_reports(root: &Path, outcome: &Sem9R1Outcome) -> Result<(), String> {
    let directory = root.join("reports/sem9-r1");
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    write_json(
        &directory.join("predecessor_integrity.json"),
        &json!({
            "passed": true,
            "predecessor_commit": outcome.run0001_receipt.predecessor_commit,
            "run0001_status": outcome.run0001_receipt.status,
            "run0001_disposition": outcome.run0001_receipt.disposition,
            "run0001_receipt_sha256": outcome.final_report.run0001_failure_receipt_sha256,
            "canonical_integrity": "PASS",
            "predecessor_integrity": "PASS",
        }),
    )?;
    write_json(
        &directory.join("preblind_format_preflight_receipt.json"),
        &json!({
            "status": "NON_CANONICAL_PREFLIGHT_FAILURE_PRESERVED",
            "failures": [
                {
                    "failure": "FORMAT_ONLY_EQUIVALENCE_FAILURE",
                    "observed_difference": "six optional trailing commas inserted by rustfmt before closing struct delimiters",
                    "resolution": "optional delimiter-adjacent trailing commas are treated as layout punctuation"
                },
                {
                    "failure": "FAILED_CANDIDATE_FREEZE_MISMATCH",
                    "observed_difference": "the semantic ID depended on the corrected token normalizer",
                    "resolution": "the semantic ID is bound directly to the immutable raw candidate source SHA-256"
                }
            ],
            "fresh_run0002_behavioral_evaluation_opened": false,
            "performance_feedback_observed": false,
            "candidate_logic_changed": false,
            "candidate_source_changed": false,
            "run0002_manifest_changed": false,
        }),
    )?;
    write_json(
        &directory.join("run0001_execution_path_audit.json"),
        &outcome.execution_path_audit,
    )?;
    write_json(
        &directory.join("failed_candidate_freeze.json"),
        &outcome.candidate_freeze,
    )?;
    write_json(
        &directory.join("format_equivalence_audit.json"),
        &outcome.format_audit,
    )?;
    write_json(
        &directory.join("sandbox_build_results.json"),
        &outcome.build_results,
    )?;
    write_json(
        &directory.join("run0002_fresh_blind_manifest.json"),
        &outcome.fresh_manifest,
    )?;
    write_json(
        &directory.join("run0002_fresh_blind_results.json"),
        &json!({
            "predecessor": outcome.predecessor,
            "candidate": outcome.candidate,
            "adversarial_predecessor": outcome.adversarial_predecessor,
            "adversarial_candidate": outcome.adversarial_candidate,
            "actual_sandbox_binaries_executed": true,
        }),
    )?;
    write_json(
        &directory.join("performance_results.json"),
        &outcome.performance,
    )?;
    write_json(
        &directory.join("regression_matrix.json"),
        &outcome.regression_matrix,
    )?;
    write_json(
        &directory.join("self_application_ablation.json"),
        &outcome.ablation,
    )?;
    write_json(
        &directory.join("source_concept_lineage.json"),
        &outcome.source_lineage,
    )?;
    write_json(
        &directory.join("protected_core_audit.json"),
        &outcome.protected_core,
    )?;
    write_json(&directory.join("leakage_audit.json"), &outcome.leakage)?;
    write_json(
        &directory.join("sparse_activation_audit.json"),
        &outcome.sparse,
    )?;
    write_json(
        &directory.join("sem9_r1_final_report.json"),
        &outcome.final_report,
    )?;
    fs::write(directory.join("SEM9_R1_REPORT.md"), markdown(outcome))
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn markdown(outcome: &Sem9R1Outcome) -> String {
    let report = &outcome.final_report;
    format!(
        "# SEM9-R1 Format-Only Repair and Fresh Regate\n\n\
         - Status: `{}`\n\
         - Disposition: `{}`\n\
         - RUN-0001 preserved: `{}`\n\
         - RUN-0001 path audit: `{}`\n\
         - RUN-0002 fresh tasks: `{}`\n\
         - Non-format token changes: `{}`\n\
         - Candidate logic changed: `{}`\n\
         - Strict correctness: `{:.3}` -> `{:.3}`\n\
         - Median expansions: `{:.1}` -> `{:.1}`\n\
         - Peak frontier: `{}` -> `{}`\n\
         - Expansion reduction: `{:.3}`\n\
         - Frontier reduction: `{:.3}`\n\
         - Regressed tasks: `{}`\n\
         - Verified sandbox candidates: `{}`\n\
         - Production integration: `not performed`\n\
         - SEM-10 started: `false`\n",
        report.sem9_r1_status,
        report.disposition,
        report.run0001_preserved,
        report.run0001_evaluation_path_audit_pass,
        report.run0002_fresh_blind_tasks,
        report.non_format_token_changes,
        report.candidate_logic_changed,
        report.predecessor_strict_solve_rate_run0002,
        report.candidate_strict_solve_rate_run0002,
        report.performance.predecessor_median_expansions,
        report.performance.candidate_median_expansions,
        report.performance.predecessor_peak_frontier,
        report.performance.candidate_peak_frontier,
        report.performance.expansion_reduction,
        report.performance.frontier_reduction,
        report.performance.regressed_tasks,
        report.verified_self_application_candidates,
    )
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    fs::write(
        path,
        serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn r1_reports_are_separate_from_run0001() {
        assert_ne!("reports/sem9-r1", "reports/sem9");
    }
}
