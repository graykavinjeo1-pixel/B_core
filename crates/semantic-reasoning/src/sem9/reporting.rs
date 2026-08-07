use std::{collections::BTreeSet, fs, path::Path};

use serde::Serialize;
use serde_json::json;

use super::experiment::Sem9Outcome;

pub const REPORT_FILES: [&str; 26] = [
    "predecessor_integrity.json",
    "protected_core_manifest.json",
    "self_mechanism_ir_spec.json",
    "self_component_catalog.json",
    "self_weaknesses.json",
    "self_application_proposals.json",
    "self_role_mappings.json",
    "self_assumption_ledgers.json",
    "rejected_self_applications.json",
    "candidate_patch_plans.json",
    "candidate_patches.json",
    "patch_provenance.json",
    "sandbox_build_results.json",
    "sandbox_test_results.json",
    "fresh_blind_manifest.json",
    "fresh_blind_results.json",
    "regression_matrix.json",
    "performance_results.json",
    "self_application_ablation.json",
    "source_concept_ablation.json",
    "patch_ablation.json",
    "safety_gate_results.json",
    "leakage_audit.json",
    "sparse_activation_audit.json",
    "sem9_final_report.json",
    "SEM9_REPORT.md",
];

pub fn write_reports(root: &Path, outcome: &Sem9Outcome) -> Result<(), String> {
    let directory = root.join("reports/sem9");
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    write_json(
        &directory.join("predecessor_integrity.json"),
        &outcome.predecessor_integrity,
    )?;
    write_json(
        &directory.join("protected_core_manifest.json"),
        &outcome.protected_core_manifest,
    )?;
    write_json(
        &directory.join("self_mechanism_ir_spec.json"),
        &json!({
            "version": "SEM9-SELF-MECHANISM-IR-1.0.0",
            "reason_directly_from_arbitrary_raw_source": false,
            "fields": [
                "component_id", "role", "inputs", "outputs", "state",
                "transformations", "preconditions", "invariants", "dependencies",
                "resource_cost", "failure_modes", "externally_visible_behavior",
                "protected_status", "provenance"
            ],
            "semantic_bridge": "SEM8 MechanismIR roles -> SelfMechanismIR roles",
            "protected_components_eligible": false,
        }),
    )?;
    write_json(
        &directory.join("self_component_catalog.json"),
        &outcome.self_components,
    )?;
    write_json(&directory.join("self_weaknesses.json"), &outcome.weaknesses)?;
    write_json(
        &directory.join("self_application_proposals.json"),
        &outcome.proposals,
    )?;
    write_json(
        &directory.join("self_role_mappings.json"),
        &outcome.role_mappings,
    )?;
    write_json(
        &directory.join("self_assumption_ledgers.json"),
        &outcome.assumption_ledgers,
    )?;
    write_json(
        &directory.join("rejected_self_applications.json"),
        &outcome.rejected_proposals,
    )?;
    write_json(
        &directory.join("candidate_patch_plans.json"),
        &outcome.patch_plans,
    )?;
    write_json(&directory.join("candidate_patches.json"), &outcome.patches)?;
    write_json(
        &directory.join("patch_provenance.json"),
        &outcome.patch_provenance,
    )?;
    write_json(
        &directory.join("sandbox_build_results.json"),
        &outcome.sandbox_build_results,
    )?;
    write_json(
        &directory.join("sandbox_test_results.json"),
        &outcome.sandbox_test_results,
    )?;
    write_json(
        &directory.join("fresh_blind_manifest.json"),
        &outcome.fresh_manifest,
    )?;
    write_json(
        &directory.join("fresh_blind_results.json"),
        &json!({
            "fresh_conditions": outcome.baselines,
            "adversarial_conditions": outcome.adversarial_results,
            "candidate_frozen_before_hidden_state_generation": true,
        }),
    )?;
    write_json(
        &directory.join("regression_matrix.json"),
        &outcome.regression_matrix,
    )?;
    write_json(
        &directory.join("performance_results.json"),
        &outcome.performance,
    )?;
    write_json(
        &directory.join("self_application_ablation.json"),
        &outcome.self_application_ablation,
    )?;
    write_json(
        &directory.join("source_concept_ablation.json"),
        &outcome.source_concept_ablation,
    )?;
    write_json(
        &directory.join("patch_ablation.json"),
        &outcome.patch_ablation,
    )?;
    write_json(
        &directory.join("safety_gate_results.json"),
        &outcome.safety_gate,
    )?;
    write_json(
        &directory.join("leakage_audit.json"),
        &outcome.leakage_audit,
    )?;
    write_json(
        &directory.join("sparse_activation_audit.json"),
        &outcome.sparse_audit,
    )?;
    write_json(
        &directory.join("sem9_final_report.json"),
        &outcome.final_report,
    )?;
    fs::write(directory.join("SEM9_REPORT.md"), markdown(outcome))
        .map_err(|error| error.to_string())?;
    verify_inventory(&directory)
}

pub fn preserve_failed_run(root: &Path, disposition: &str) -> Result<(), String> {
    let directory = root.join("reports/sem9");
    if !directory.exists() {
        return Ok(());
    }
    let failed_root = directory.join("failed_runs");
    fs::create_dir_all(&failed_root).map_err(|error| error.to_string())?;
    let mut ordinal = 1usize;
    loop {
        let target = failed_root.join(format!("SEM9-FAILED-{ordinal:04}"));
        if !target.exists() {
            fs::create_dir_all(&target).map_err(|error| error.to_string())?;
            for entry in fs::read_dir(&directory).map_err(|error| error.to_string())? {
                let entry = entry.map_err(|error| error.to_string())?;
                if entry.file_name() == "failed_runs" {
                    continue;
                }
                let destination = target.join(entry.file_name());
                fs::rename(entry.path(), destination).map_err(|error| error.to_string())?;
            }
            fs::write(target.join("DISPOSITION.txt"), disposition)
                .map_err(|error| error.to_string())?;
            return Ok(());
        }
        ordinal += 1;
    }
}

fn markdown(outcome: &Sem9Outcome) -> String {
    let final_report = &outcome.final_report;
    format!(
        "# SEM-9 Recursive Self-Application Sandbox\n\n\
         - Status: `{}`\n\
         - Disposition: `{}`\n\
         - Autonomous source: `{}` from `{:?}`\n\
         - Self target: `{}`\n\
         - Fresh blind tasks: `{}`\n\
         - Strict correctness: `{:.3}` -> `{:.3}`\n\
         - Median expansions: `{:.1}` -> `{:.1}`\n\
         - Expansion reduction: `{:.3}`\n\
         - Regressed tasks: `{}`\n\
         - Production source mutations: `{}`\n\
         - Verified sandbox candidates: `{}`\n\
         - SEM-10 started: `false`\n\n\
         The verified candidate remains a sandbox artifact and was not merged into the canonical runtime.\n",
        final_report.sem9_status,
        final_report.disposition,
        final_report.best_self_source_concept_id,
        final_report.best_self_source_concept_origin_domain,
        final_report.best_self_target_component,
        final_report.fresh_blind_tasks,
        final_report.predecessor_strict_solve_rate,
        final_report.best_candidate_strict_solve_rate,
        final_report.performance.predecessor_median_expansions,
        final_report.performance.candidate_median_expansions,
        final_report.performance.expansion_reduction,
        final_report.performance.regressed_tasks,
        final_report.production_source_mutations,
        final_report.verified_self_application_candidates,
    )
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    fs::write(path, bytes).map_err(|error| error.to_string())
}

fn verify_inventory(directory: &Path) -> Result<(), String> {
    let expected = REPORT_FILES.iter().copied().collect::<BTreeSet<_>>();
    let actual = fs::read_dir(directory)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect::<BTreeSet<_>>();
    let actual_refs = actual.iter().map(String::as_str).collect::<BTreeSet<_>>();
    if expected != actual_refs {
        return Err(format!(
            "SEM9_REPORT_INVENTORY_MISMATCH:expected={expected:?}:actual={actual_refs:?}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_inventory_matches_sem9_contract() {
        assert_eq!(REPORT_FILES.len(), 26);
        assert!(REPORT_FILES.contains(&"self_application_ablation.json"));
        assert!(REPORT_FILES.contains(&"source_concept_ablation.json"));
        assert!(REPORT_FILES.contains(&"patch_ablation.json"));
        assert!(REPORT_FILES.contains(&"SEM9_REPORT.md"));
    }
}
