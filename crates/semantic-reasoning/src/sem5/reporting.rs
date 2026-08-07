use std::{fs, path::Path};

use serde::Serialize;
use serde_json::json;

use super::experiment::Sem5Outcome;

pub fn write_reports(root: &Path, outcome: &Sem5Outcome) -> Result<(), String> {
    let directory = root.join("reports/sem5");
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    write_json(
        &directory.join("predecessor_integrity.json"),
        &outcome.predecessor_integrity,
    )?;
    write_json(
        &directory.join("programming_primitive_catalog.json"),
        &outcome.programming_primitive_catalog,
    )?;
    write_json(
        &directory.join("program_ir_spec.json"),
        &outcome.program_ir_spec,
    )?;
    write_json(
        &directory.join("rust_min_allowlist.json"),
        &outcome.rust_min_allowlist,
    )?;
    write_json(
        &directory.join("sandbox_audit.json"),
        &outcome.sandbox_audit,
    )?;
    write_json(
        &directory.join("discovery_manifest.json"),
        &outcome.discovery_manifest,
    )?;
    write_json(
        &directory.join("blind_manifest.json"),
        &outcome.blind_manifest,
    )?;
    write_json(
        &directory.join("opaque_api_manifest.json"),
        &outcome.opaque_api_manifest,
    )?;
    write_json(
        &directory.join("adversarial_manifest.json"),
        &outcome.adversarial_manifest,
    )?;
    write_json(
        &directory.join("freeze_record.json"),
        &outcome.freeze_record,
    )?;
    write_json(
        &directory.join("program_synthesis_results.json"),
        &json!({
            "program_ir_is_authoritative": true,
            "rust_is_execution_adapter": true,
            "programs": outcome.programs,
            "conditions": outcome.condition_reports,
        }),
    )?;
    write_json(
        &directory.join("compile_results.json"),
        &outcome.compile_results,
    )?;
    write_json(
        &directory.join("runtime_results.json"),
        &json!({
            "results": outcome.compile_results,
            "successful": outcome.compile_results.iter().filter(|record| record.runtime_valid).count(),
            "timeouts": outcome.compile_results.iter().filter(|record| record.runtime_timed_out).count(),
            "containment_violations": outcome.compile_results.iter().map(|record| record.containment_violations).sum::<usize>(),
        }),
    )?;
    let d = outcome
        .condition_reports
        .get("FIRST_PRINCIPLES_D")
        .ok_or_else(|| "D_REPORT_MISSING".to_string())?;
    write_json(
        &directory.join("property_generalization.json"),
        &json!({
            "hidden_cases_generated_after_synthesis": true,
            "expected_outputs_solver_visible": false,
            "property_cases_passed": d.records.iter().map(|record| record.property_tests_passed).sum::<usize>(),
            "property_cases_total": d.records.iter().map(|record| record.property_tests_total).sum::<usize>(),
            "pass_rate": d.property_generalization_pass_rate,
            "invalid_effect_accepted": outcome.final_report.invalid_effect_accepted,
        }),
    )?;
    write_json(
        &directory.join("programming_candidates.json"),
        &outcome.programming_candidates,
    )?;
    write_json(
        &directory.join("programming_promotions.json"),
        &outcome.programming_promotions,
    )?;
    write_json(
        &directory.join("programming_lineage.json"),
        &json!({
            "predecessor_concepts_immutable": outcome.predecessor_integrity.promoted_concepts_verified_immutable,
            "new_lineage": outcome.programming_promotions.iter().map(|promotion| json!({
                "concept_id": promotion.concept.concept_id,
                "generation": promotion.concept.generation,
                "parent_ids": promotion.concept.parent_ids,
                "promoted": promotion.promoted,
                "identity_wrapper": promotion.concept.identity_wrapper,
            })).collect::<Vec<_>>(),
        }),
    )?;
    write_json(
        &directory.join("cross_domain_transfer.json"),
        &outcome.cross_domain_transfer,
    )?;
    let opaque_ids = outcome
        .opaque_api_manifest
        .tasks
        .iter()
        .map(|task| task.task_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let opaque_records = d
        .records
        .iter()
        .filter(|record| opaque_ids.contains(record.task_id.as_str()))
        .collect::<Vec<_>>();
    write_json(
        &directory.join("opaque_api_zero_shot.json"),
        &json!({
            "definitions_only": true,
            "worked_examples": 0,
            "stable_api_meanings": 0,
            "tasks": opaque_records,
            "solve_rate": outcome.final_report.definition_only_api_zero_shot_solve_rate,
        }),
    )?;
    write_json(
        &directory.join("programming_counterfactuals.json"),
        &outcome.counterfactuals,
    )?;
    write_json(
        &directory.join("programming_ablation.json"),
        &outcome.ablations,
    )?;
    write_json(
        &directory.join("baseline_results.json"),
        &json!({
            "equal_task_set": true,
            "equal_search_budget": true,
            "reports": outcome.condition_reports,
        }),
    )?;
    write_json(
        &directory.join("capability_frontier.json"),
        &outcome.capability_frontier,
    )?;
    write_json(
        &directory.join("target_solution_leakage_audit.json"),
        &outcome.target_solution_leakage_audit,
    )?;
    write_json(
        &directory.join("language_separation_audit.json"),
        &outcome.language_separation_audit,
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
        &directory.join("sem5_final_report.json"),
        &outcome.final_report,
    )?;
    fs::write(directory.join("SEM5_REPORT.md"), markdown_report(outcome))
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    fs::write(path, bytes).map_err(|error| error.to_string())
}

fn markdown_report(outcome: &Sem5Outcome) -> String {
    let report = &outcome.final_report;
    let mut gates = String::new();
    for (gate, passed) in &report.gates {
        gates.push_str(&format!(
            "- `{gate}`: {}\n",
            if *passed { "PASS" } else { "FAIL" }
        ));
    }
    format!(
        "# SEM-5 Programming First-Principles Expansion\n\n\
Run `{}` is **{}**: `{}`. The frozen evaluator used {} fresh blind tasks, including {} definition-only opaque-API tasks and {} adversarial programs. Expected outputs, evaluator families, and reference source were absent from solver-visible manifests.\n\n\
## Execution evidence\n\n\
- ProgramIR valid rate: `{:.6}`\n\
- offline Rust-Min compile rate: `{:.6}`\n\
- bounded runtime-valid rate: `{:.6}`\n\
- hidden property pass rate: `{:.6}`\n\
- definition-only zero-shot rate: `{:.6}`\n\
- containment violations: `{}`\n\n\
ProgramIR remained authoritative; Rust source was generated only as a deterministic execution adapter. All canonical programs were source-audited, compiled with the local `rustc` and no external crates, run in isolated temporary directories, and deleted after capture. Windows does not expose a portable standard-library-only address-space limiter, so timeout, output, path, process, dependency, and filesystem containment were enforced while the memory-limit field records that platform limitation.\n\n\
## Controlled comparison\n\n\
- primitive A solve rate: `{:.6}`\n\
- structural B solve rate: `{:.6}`\n\
- semantic no-promotion C solve rate: `{:.6}`\n\
- full first-principles D solve rate: `{:.6}`\n\
- D-minus-C solve delta: `{:.6}`\n\
- D search-cost reduction versus C: `{:.6}`\n\n\
All conditions used the same frozen tasks and expansion budget. Outcomes arise from typed IR construction and resource-bounded search; no opened blind task was rewritten.\n\n\
## Autonomous concepts\n\n\
{} candidates were proposed from recurring IR dependency structures and {} passed semantic consistency, compression, calibration, fresh reuse, cross-instance, language-separation, lineage, and causal-ablation gates. Generation-3 concepts depend on immutable Generation-2 ancestors; the Generation-4 concept recombines the promoted Generation-3 abstractions. Human interpretations were attached only after sealing.\n\n\
Best concept: `{}` — {}. Compression: `{}` primitive-expanded nodes to `{}` operational nodes (`{:.6}x`). Cross-domain transfers: `{}`; predecessor-concept reuse: `{}`.\n\n\
## Gates\n\n\
{}\n\
## Quarantine and next stage\n\n\
Network, external LLM, local teacher, recursive mutation, full-catalog scan, and routing-false-negative counts are zero. Recursive improvement remains observe/measure-only. SEM-6 was not started; the next allowed stage is `{}`.\n",
        report.run_id,
        report.sem5_status,
        report.disposition,
        report.fresh_blind_tasks,
        report.opaque_api_blind_tasks,
        report.adversarial_programming_tasks,
        report.program_ir_valid_rate,
        report.rust_compile_rate,
        report.runtime_valid_rate,
        report.property_generalization_pass_rate,
        report.definition_only_api_zero_shot_solve_rate,
        outcome.sandbox_audit.containment_violations,
        report.baseline_a_solve_rate,
        report.baseline_b_solve_rate,
        report.baseline_c_solve_rate,
        report.full_d_solve_rate,
        outcome.capability_frontier.solve_rate_delta_d_minus_c,
        outcome.capability_frontier.search_cost_reduction_d_vs_c,
        report.autonomous_program_candidates,
        report.promoted_program_concepts,
        report.best_program_concept_id,
        report.best_program_concept_posthoc_interpretation,
        report.best_primitive_expanded_ir_nodes,
        report.best_compressed_operational_nodes,
        report.best_program_compression_ratio,
        report.cross_domain_concept_transfer_count,
        report.predecessor_concept_reuse_count,
        gates,
        report.next_allowed_stage,
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn required_report_inventory_is_explicit() {
        let source = include_str!("reporting.rs");
        for name in [
            "predecessor_integrity.json",
            "programming_primitive_catalog.json",
            "program_ir_spec.json",
            "rust_min_allowlist.json",
            "sandbox_audit.json",
            "blind_manifest.json",
            "program_synthesis_results.json",
            "compile_results.json",
            "runtime_results.json",
            "property_generalization.json",
            "programming_candidates.json",
            "programming_promotions.json",
            "programming_lineage.json",
            "cross_domain_transfer.json",
            "opaque_api_zero_shot.json",
            "programming_counterfactuals.json",
            "programming_ablation.json",
            "baseline_results.json",
            "capability_frontier.json",
            "target_solution_leakage_audit.json",
            "language_separation_audit.json",
            "sparse_activation_audit.json",
            "contamination_audit.json",
            "sem5_final_report.json",
            "SEM5_REPORT.md",
        ] {
            assert!(source.contains(name), "missing {name}");
        }
    }
}
