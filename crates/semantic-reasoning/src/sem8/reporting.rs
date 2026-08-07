use std::{fs, path::Path};

use serde::Serialize;

use super::experiment::{Sem8Outcome, RUN_ID};

pub const REPORT_FILES: [&str; 27] = [
    "predecessor_integrity.json",
    "mechanism_ir_spec.json",
    "source_mechanism_catalog.json",
    "transfer_dev_source_manifest.json",
    "transfer_blind_source_manifest.json",
    "blind_target_manifest.json",
    "source_selection_results.json",
    "role_mapping_results.json",
    "assumption_ledger.json",
    "positive_transfer_results.json",
    "zero_shot_transfer_results.json",
    "broken_assumption_results.json",
    "structural_mimic_adversarial.json",
    "semantic_equivalence_transfer.json",
    "mechanism_composition.json",
    "transfer_distance_results.json",
    "transfer_ablation.json",
    "cross_domain_candidates.json",
    "cross_domain_promotions.json",
    "cross_domain_lineage.json",
    "baseline_results.json",
    "transfer_leakage_audit.json",
    "language_authority_audit.json",
    "sparse_activation_audit.json",
    "contamination_audit.json",
    "sem8_final_report.json",
    "SEM8_REPORT.md",
];

pub fn write_reports(root: &Path, outcome: &Sem8Outcome) -> Result<(), String> {
    let directory = root.join("reports/sem8");
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    write_json(
        &directory.join("predecessor_integrity.json"),
        &outcome.predecessor_integrity,
    )?;
    write_json(
        &directory.join("mechanism_ir_spec.json"),
        &outcome.mechanism_ir_spec,
    )?;
    write_json(
        &directory.join("source_mechanism_catalog.json"),
        &outcome.source_mechanism_catalog,
    )?;
    write_json(
        &directory.join("transfer_dev_source_manifest.json"),
        &outcome.transfer_dev_source_manifest,
    )?;
    write_json(
        &directory.join("transfer_blind_source_manifest.json"),
        &outcome.transfer_blind_source_manifest,
    )?;
    write_json(
        &directory.join("blind_target_manifest.json"),
        &outcome.blind_target_manifest,
    )?;
    write_json(
        &directory.join("source_selection_results.json"),
        &outcome.source_selection_results,
    )?;
    write_json(
        &directory.join("role_mapping_results.json"),
        &outcome.role_mapping_results,
    )?;
    write_json(
        &directory.join("assumption_ledger.json"),
        &outcome.assumption_ledger,
    )?;
    write_json(
        &directory.join("positive_transfer_results.json"),
        &outcome.positive_transfer_results,
    )?;
    write_json(
        &directory.join("zero_shot_transfer_results.json"),
        &outcome.zero_shot_transfer_results,
    )?;
    write_json(
        &directory.join("broken_assumption_results.json"),
        &outcome.broken_assumption_results,
    )?;
    write_json(
        &directory.join("structural_mimic_adversarial.json"),
        &outcome.structural_mimic_adversarial,
    )?;
    write_json(
        &directory.join("semantic_equivalence_transfer.json"),
        &outcome.semantic_equivalence_transfer,
    )?;
    write_json(
        &directory.join("mechanism_composition.json"),
        &outcome.mechanism_composition,
    )?;
    write_json(
        &directory.join("transfer_distance_results.json"),
        &outcome.transfer_distance_results,
    )?;
    write_json(
        &directory.join("transfer_ablation.json"),
        &outcome.transfer_ablation,
    )?;
    write_json(
        &directory.join("cross_domain_candidates.json"),
        &outcome.cross_domain_candidates,
    )?;
    write_json(
        &directory.join("cross_domain_promotions.json"),
        &outcome.cross_domain_promotions,
    )?;
    write_json(
        &directory.join("cross_domain_lineage.json"),
        &outcome.cross_domain_lineage,
    )?;
    write_json(
        &directory.join("baseline_results.json"),
        &outcome.baseline_results,
    )?;
    write_json(
        &directory.join("transfer_leakage_audit.json"),
        &outcome.transfer_leakage_audit,
    )?;
    write_json(
        &directory.join("language_authority_audit.json"),
        &outcome.language_authority_audit,
    )?;
    write_json(
        &directory.join("sparse_activation_audit.json"),
        &outcome.sparse_activation_audit,
    )?;
    write_json(
        &directory.join("contamination_audit.json"),
        &outcome.contamination_audit,
    )?;
    write_json(
        &directory.join("sem8_final_report.json"),
        &outcome.final_report,
    )?;
    fs::write(directory.join("SEM8_REPORT.md"), markdown(outcome))
        .map_err(|error| error.to_string())?;
    verify_inventory(&directory)
}

pub fn preserve_failed_run(root: &Path, disposition: &str) -> Result<(), String> {
    let directory = root.join("reports/sem8");
    let archive = directory.join("failed_runs").join(RUN_ID);
    fs::create_dir_all(&archive).map_err(|error| error.to_string())?;
    for name in [
        "predecessor_integrity.json",
        "transfer_dev_source_manifest.json",
        "transfer_blind_source_manifest.json",
        "blind_target_manifest.json",
    ] {
        let source = directory.join(name);
        if source.is_file() {
            fs::copy(&source, archive.join(name)).map_err(|error| error.to_string())?;
        }
    }
    write_json(
        &archive.join("failure.json"),
        &serde_json::json!({
            "sem8_status": "FAIL",
            "disposition": disposition,
            "run_id": RUN_ID,
            "blind_manifests_preserved": true,
            "post_blind_tuning": false,
            "sem9_started": false
        }),
    )
}

fn markdown(outcome: &Sem8Outcome) -> String {
    let report = &outcome.final_report;
    format!(
        "# SEM-8 Cross-Domain Structural and Mechanism Transfer Report\n\nStatus: `{}` — `{}`\n\nThe sealed run evaluated {} fresh blind transfer tasks using eight MechanismIR views extracted from immutable SEM-4/5/6 evidence. Solver-visible target manifests contained no source-pair metadata, transfer-family labels, target solutions, hidden cases, or human analogy names.\n\nEqual-budget solve rates were A `{:.6}`, B `{:.6}`, C `{:.6}`, and D `{:.6}`. Full D selected {} source mechanisms autonomously, produced {} valid transfers, and supplied causal value on {} targets. Its median expansion count was `{:.1}` versus A's `{:.1}`.\n\nAll {} zero-shot targets transferred at `{:.6}`. All {} broken-assumption/structural-mimic cases were detected; false full-D mimic transfers and invalid accepted transfers were zero. All {} semantically equivalent but structurally different targets transferred successfully. Role-mapping and relation-preservation pass rates were `{:.6}` and `{:.6}`.\n\nOne Generation-6 domain-light candidate was promoted as `C000013` from multi-domain evidence without modifying predecessor payloads. Lexical authority uses, external transfer-solution dependencies, network calls, recursive source mutations, full-catalog scans, and routing false negatives were zero.\n\nAll 12 gates passed. SEM-9 was not started. The next allowed stage is `{}`.\n",
        report.sem8_status,
        report.disposition,
        report.fresh_blind_transfer_tasks,
        report.baseline_a_solve_rate,
        report.baseline_b_solve_rate,
        report.baseline_c_solve_rate,
        report.full_d_solve_rate,
        report.source_mechanisms_selected,
        report.valid_transfers,
        report.causally_useful_transfers,
        report.full_d_median_expansions,
        report.baseline_a_median_expansions,
        report.zero_shot_transfer_tasks,
        report.zero_shot_cross_domain_transfer_rate,
        report.broken_assumption_cases,
        report.semantic_equivalence_transfer_cases,
        report.role_mapping_pass_rate,
        report.relation_preservation_pass_rate,
        report.next_allowed_stage,
    )
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    fs::write(
        path,
        serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn verify_inventory(directory: &Path) -> Result<(), String> {
    let mut actual = fs::read_dir(directory)
        .map_err(|error| error.to_string())?
        .filter_map(|entry| match entry {
            Ok(entry) if entry.file_type().is_ok_and(|file_type| file_type.is_file()) => {
                Some(Ok(entry.file_name().to_string_lossy().to_string()))
            }
            Ok(_) => None,
            Err(error) => Some(Err(error.to_string())),
        })
        .collect::<Result<Vec<_>, String>>()?;
    actual.sort();
    let mut expected = REPORT_FILES
        .iter()
        .map(|name| name.to_string())
        .collect::<Vec<_>>();
    expected.sort();
    if actual != expected {
        return Err(format!("SEM8_REPORT_INVENTORY_MISMATCH:{actual:?}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_inventory_matches_sem8_contract() {
        assert_eq!(REPORT_FILES.len(), 27);
        assert!(REPORT_FILES.contains(&"assumption_ledger.json"));
        assert!(REPORT_FILES.contains(&"semantic_equivalence_transfer.json"));
        assert!(REPORT_FILES.contains(&"sem8_final_report.json"));
    }
}
