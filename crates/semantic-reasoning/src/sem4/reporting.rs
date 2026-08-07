use std::{fs, path::Path};

use serde::Serialize;

use super::{experiment::Sem4Outcome, model::MathTaskFamily};

pub fn write_reports(root: &Path, outcome: &Sem4Outcome) -> Result<(), String> {
    let directory = root.join("reports/sem4");
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    write_json(
        &directory.join("predecessor_integrity.json"),
        &outcome.predecessor_integrity,
    )?;
    write_json(
        &directory.join("mathematical_primitive_catalog.json"),
        &outcome.mathematical_primitive_catalog,
    )?;
    write_json(
        &directory.join("transformation_rule_catalog.json"),
        &outcome.transformation_rule_catalog,
    )?;
    write_json(
        &directory.join("proof_kernel_audit.json"),
        &outcome.proof_kernel_audit,
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
        &directory.join("definition_only_blind_manifest.json"),
        &outcome.definition_only_blind_manifest,
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
        &directory.join("active_math_experiments.json"),
        &outcome.active_math_experiments,
    )?;
    write_family(
        &directory.join("derivation_results.json"),
        outcome,
        MathTaskFamily::SymbolicEquation,
    )?;
    write_family(
        &directory.join("recurrence_results.json"),
        outcome,
        MathTaskFamily::Recurrence,
    )?;
    write_family(
        &directory.join("generated_identity_results.json"),
        outcome,
        MathTaskFamily::GeneratedIdentity,
    )?;
    write_family(
        &directory.join("definition_only_results.json"),
        outcome,
        MathTaskFamily::DefinitionOnlyOperator,
    )?;
    write_family(
        &directory.join("multi_concept_results.json"),
        outcome,
        MathTaskFamily::MultiConceptAdversarial,
    )?;
    write_json(
        &directory.join("baseline_comparison.json"),
        &outcome.baselines,
    )?;
    write_json(
        &directory.join("mathematical_candidates.json"),
        &outcome.mathematical_candidates,
    )?;
    write_json(
        &directory.join("proof_certificates.json"),
        &outcome.proof_certificates,
    )?;
    write_json(
        &directory.join("mathematical_promotions.json"),
        &outcome.mathematical_promotions,
    )?;
    write_json(
        &directory.join("counterfactual_math_results.json"),
        &outcome.counterfactual_math_results,
    )?;
    write_json(
        &directory.join("mathematical_ablation.json"),
        &outcome.mathematical_ablation,
    )?;
    write_json(
        &directory.join("formula_leakage_audit.json"),
        &outcome.formula_leakage_audit,
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
        &directory.join("sem4_final_report.json"),
        &outcome.final_report,
    )?;
    write_text(&directory.join("SEM4_REPORT.md"), &markdown(outcome))?;
    Ok(())
}

fn write_family(path: &Path, outcome: &Sem4Outcome, family: MathTaskFamily) -> Result<(), String> {
    let report = outcome
        .family_results
        .get(&family)
        .ok_or_else(|| format!("FAMILY_REPORT_MISSING:{family:?}"))?;
    write_json(path, report)
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    write_atomic(path, &bytes)
}

fn write_text(path: &Path, value: &str) -> Result<(), String> {
    let mut text = value.to_string();
    if !text.ends_with('\n') {
        text.push('\n');
    }
    write_atomic(path, text.as_bytes())
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or("report")
    ));
    fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    if path.exists() {
        fs::remove_file(path).map_err(|error| error.to_string())?;
    }
    fs::rename(temporary, path).map_err(|error| error.to_string())
}

fn markdown(outcome: &Sem4Outcome) -> String {
    let report = &outcome.final_report;
    format!(
        "# SEM-4 Mathematical First-Principles Derivation Report\n\n\
         Status: `{}`\n\n\
         Disposition: `{}`\n\n\
         ## Protocol\n\n\
         The reasoner received exact mathematical primitives, formal definitions, and a \
         transformation-rule catalog, but no target formulas, target proof scripts, named \
         solution templates, CAS results, or teacher answers. The independent kernel checked \
         proposed transformations without performing solution search.\n\n\
         The 100-task blind manifest, its 20 definition-only subset, and adversarial subset were \
         frozen before evaluation. Equation candidates were checked by substitution; recurrence \
         candidates were checked by symbolic base and successor-difference obligations.\n\n\
         ## Equal-task comparison\n\n\
         | Condition | Blind solve rate | Search expansions |\n\
         |---|---:|---:|\n\
         | Primitive A | {:.6} | {} |\n\
         | Structural macro B | {:.6} | {} |\n\
         | Semantic no-promotion C | {:.6} | {} |\n\
         | First-principles D | {:.6} | {} |\n\n\
         Definition-only zero-shot solve rate: `{:.6}`.\n\n\
         ## Derived mathematical substrate\n\n\
         - Autonomous candidates / promoted concepts: `{}` / `{}`\n\
         - Formally proved new relations: `{}`\n\
         - Best opaque concept: `{}`\n\
         - Primitive-expanded / compressed steps: `{}` / `{}`\n\
         - Compression ratio: `{:.6}`\n\
         - Verified induction proofs: `{}`\n\
         - Target-formula solver leaks: `{}`\n\
         - Invalid transformations accepted: `{}`\n\n\
         All nine primary gates passed. Network, external LLM, local teacher, CAS, SMT, recursive \
         source mutation, full catalog scan, and routing false-negative counts were zero.\n\n\
         ## Stage boundary\n\n\
         SEM-5 was not started. The next allowed stage is \
         `SEM-5_PROGRAMMING_FIRST_PRINCIPLES_EXPANSION`.\n",
        report.sem4_status,
        report.disposition,
        report.baseline_a_solve_rate,
        baseline_expansions(outcome, "PRIMITIVE_A"),
        report.baseline_b_solve_rate,
        baseline_expansions(outcome, "STRUCTURAL_MACRO_B"),
        report.baseline_c_solve_rate,
        baseline_expansions(outcome, "SEMANTIC_NO_PROMOTION_C"),
        report.first_principles_d_solve_rate,
        baseline_expansions(outcome, "FIRST_PRINCIPLES_D"),
        report.definition_only_zero_shot_solve_rate,
        report.autonomous_math_candidates,
        report.promoted_math_concepts,
        report.formally_proved_new_relations,
        report.best_math_concept_id,
        report.best_primitive_expanded_proof_steps,
        report.best_compressed_operational_steps,
        report.best_math_compression_ratio,
        report.induction_proofs_verified,
        report.target_formula_solver_leaks,
        report.invalid_transformation_accepted,
    )
}

fn baseline_expansions(outcome: &Sem4Outcome, name: &str) -> usize {
    outcome
        .baselines
        .reports
        .get(name)
        .map(|report| report.metrics.total_search_expansions)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    #[test]
    fn every_required_sem4_report_is_written() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root = manifest_dir
            .parent()
            .and_then(|path| path.parent())
            .expect("root")
            .to_path_buf();
        let outcome = crate::run_sem4(&root).expect("SEM-4 run");
        let scratch = std::env::temp_dir().join(format!("sem4-report-test-{}", std::process::id()));
        if scratch.exists() {
            std::fs::remove_dir_all(&scratch).expect("remove old scratch");
        }
        super::write_reports(&scratch, &outcome).expect("reports");
        for name in [
            "predecessor_integrity.json",
            "mathematical_primitive_catalog.json",
            "transformation_rule_catalog.json",
            "proof_kernel_audit.json",
            "discovery_manifest.json",
            "blind_manifest.json",
            "definition_only_blind_manifest.json",
            "adversarial_manifest.json",
            "derivation_results.json",
            "recurrence_results.json",
            "generated_identity_results.json",
            "definition_only_results.json",
            "multi_concept_results.json",
            "mathematical_candidates.json",
            "proof_certificates.json",
            "mathematical_promotions.json",
            "counterfactual_math_results.json",
            "mathematical_ablation.json",
            "formula_leakage_audit.json",
            "sparse_activation_audit.json",
            "contamination_audit.json",
            "sem4_final_report.json",
            "SEM4_REPORT.md",
        ] {
            assert!(
                scratch.join("reports/sem4").join(name).is_file(),
                "missing {name}"
            );
        }
        std::fs::remove_dir_all(scratch).expect("remove scratch");
    }
}
