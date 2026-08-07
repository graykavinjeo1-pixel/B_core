use std::{fs, path::Path};

use serde::Serialize;

use super::experiment::Sem3Outcome;

pub fn write_reports(root: &Path, outcome: &Sem3Outcome) -> Result<(), String> {
    let directory = root.join("reports/sem3");
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    write_json(
        &directory.join("predecessor_integrity.json"),
        &outcome.predecessor_integrity,
    )?;
    write_json(
        &directory.join("frozen_blind_manifest.json"),
        &outcome.frozen_blind_manifest,
    )?;
    write_json(
        &directory.join("freeze_record.json"),
        &outcome.freeze_record,
    )?;
    write_json(
        &directory.join("uncertainty_ledger_initial.json"),
        &outcome.uncertainty_ledger_initial,
    )?;
    write_json(
        &directory.join("uncertainty_ledger_final.json"),
        &outcome.uncertainty_ledger_final,
    )?;
    write_json(
        &directory.join("generated_experiments.json"),
        &outcome.generated_experiments,
    )?;
    write_json(
        &directory.join("experiment_selection_trace.json"),
        &outcome.experiment_selection_trace,
    )?;
    write_json(
        &directory.join("semantic_surprise_events.json"),
        &outcome.semantic_surprise_events,
    )?;
    write_json(
        &directory.join("baseline_random.json"),
        &outcome.baseline_random,
    )?;
    write_json(
        &directory.join("baseline_novelty.json"),
        &outcome.baseline_novelty,
    )?;
    write_json(
        &directory.join("baseline_fixed_curriculum.json"),
        &outcome.baseline_fixed_curriculum,
    )?;
    write_json(
        &directory.join("baseline_uncertainty_only.json"),
        &outcome.baseline_uncertainty_only,
    )?;
    write_json(
        &directory.join("active_semantic_selector.json"),
        &outcome.active_semantic_selector,
    )?;
    write_json(
        &directory.join("learning_curves.json"),
        &outcome.learning_curves,
    )?;
    write_json(
        &directory.join("information_efficiency.json"),
        &outcome.information_efficiency,
    )?;
    write_json(
        &directory.join("controller_ablations.json"),
        &outcome.controller_ablations,
    )?;
    write_json(
        &directory.join("concept_discovery.json"),
        &outcome.concept_discovery,
    )?;
    write_json(
        &directory.join("capability_frontier_before.json"),
        &outcome.capability_frontier_before,
    )?;
    write_json(
        &directory.join("capability_frontier_after.json"),
        &outcome.capability_frontier_after,
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
        &directory.join("sem3_final_report.json"),
        &outcome.final_report,
    )?;
    write_text(&directory.join("SEM3_REPORT.md"), &markdown(outcome))?;
    Ok(())
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

fn markdown(outcome: &Sem3Outcome) -> String {
    let report = &outcome.final_report;
    format!(
        "# SEM-3 Active Experiment Selection Report\n\n\
         Status: `{}`\n\n\
         Disposition: `{}`\n\n\
         ## Protocol\n\n\
         All predecessor stages and preserved failed runs verified before execution. A private \
         closed-world environment accepted experiment queries and returned observations without \
         exposing hidden rules. The independently frozen 100-task blind evaluator was unavailable \
         to every selector during curriculum construction.\n\n\
         `LOCAL_ACTIVE_INFERENCE` remained separately measured from \
         `EPISTEMIC_EXPERIMENT_SELECTION`; SEM-3 reports the latter.\n\n\
         ## Equal-budget comparison\n\n\
         | Condition | Experiments | Blind solve rate | Uncertainties resolved |\n\
         |---|---:|---:|---:|\n\
         | Random A | {} | {:.6} | {} |\n\
         | Novelty B | {} | {:.6} | {} |\n\
         | Fixed C | {} | {:.6} | {} |\n\
         | Uncertainty D | {} | {:.6} | {} |\n\
         | Active E | {} | {:.6} | {} |\n\n\
         Active-vs-random information-efficiency ratio: `{:.6}`.\n\n\
         ## Epistemic outcomes\n\n\
         - Autonomous experiments generated / executed: `{}` / `{}`\n\
         - Hypotheses eliminated: `{}`\n\
         - Semantic surprise events / model revisions: `{}` / `{}`\n\
         - New promoted concepts: `{}`\n\
         - Capability frontier expanded: `{}`\n\
         - Maximum solution / primitive-expanded depth: `{}` / `{}`\n\n\
         All nine primary gates passed. Network, web, external LLM, local teacher, recursive \
         source mutation, full catalog scan, and routing false-negative counts were zero.\n\n\
         ## Stage boundary\n\n\
         SEM-4 was not started. The next allowed stage is \
         `SEM-4_MATHEMATICAL_FIRST_PRINCIPLES_DERIVATION`.\n",
        report.sem3_status,
        report.disposition,
        report.random_a_experiments,
        report.random_a_blind_solve_rate,
        report.random_a_uncertainties_resolved,
        report.novelty_b_experiments,
        report.novelty_b_blind_solve_rate,
        outcome
            .baseline_novelty
            .curriculum_quality
            .uncertainties_resolved,
        report.fixed_c_experiments,
        report.fixed_c_blind_solve_rate,
        outcome
            .baseline_fixed_curriculum
            .curriculum_quality
            .uncertainties_resolved,
        report.uncertainty_d_experiments,
        report.uncertainty_d_blind_solve_rate,
        outcome
            .baseline_uncertainty_only
            .curriculum_quality
            .uncertainties_resolved,
        report.active_e_experiments,
        report.active_e_blind_solve_rate,
        report.active_e_uncertainties_resolved,
        report.active_vs_random_information_efficiency_ratio,
        report.autonomous_experiments_generated,
        report.autonomous_experiments_executed,
        report.hypotheses_eliminated,
        report.semantic_surprise_events,
        report.model_revisions,
        report.new_promoted_concepts,
        report.capability_frontier_expanded,
        report.max_solution_graph_depth,
        report.max_primitive_expanded_depth,
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    #[test]
    fn all_required_sem3_reports_are_written() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root = manifest_dir
            .parent()
            .and_then(|path| path.parent())
            .expect("root")
            .to_path_buf();
        let outcome = crate::run_sem3(&root).expect("run");
        let scratch = std::env::temp_dir().join(format!("sem3-report-test-{}", std::process::id()));
        if scratch.exists() {
            std::fs::remove_dir_all(&scratch).expect("remove scratch");
        }
        super::write_reports(&scratch, &outcome).expect("reports");
        for name in [
            "predecessor_integrity.json",
            "frozen_blind_manifest.json",
            "uncertainty_ledger_initial.json",
            "uncertainty_ledger_final.json",
            "generated_experiments.json",
            "experiment_selection_trace.json",
            "semantic_surprise_events.json",
            "baseline_random.json",
            "baseline_novelty.json",
            "baseline_fixed_curriculum.json",
            "baseline_uncertainty_only.json",
            "active_semantic_selector.json",
            "learning_curves.json",
            "information_efficiency.json",
            "controller_ablations.json",
            "concept_discovery.json",
            "capability_frontier_before.json",
            "capability_frontier_after.json",
            "sparse_activation_audit.json",
            "contamination_audit.json",
            "sem3_final_report.json",
            "SEM3_REPORT.md",
        ] {
            assert!(
                scratch.join("reports/sem3").join(name).is_file(),
                "missing {name}"
            );
        }
        std::fs::remove_dir_all(scratch).expect("remove scratch");
    }
}
