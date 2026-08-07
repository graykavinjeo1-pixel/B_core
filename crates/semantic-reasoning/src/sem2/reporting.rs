use std::{fs, path::Path};

use serde::Serialize;

use super::{experiment::Sem2Outcome, model::TaskClass};

pub fn write_reports(root: &Path, outcome: &Sem2Outcome) -> Result<(), String> {
    let directory = root.join("reports/sem2");
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    write_json(
        &directory.join("predecessor_integrity.json"),
        &outcome.predecessor_integrity,
    )?;
    write_json(
        &directory.join("metric_semantics_audit.json"),
        &outcome.metric_semantics_audit,
    )?;
    write_json(
        &directory.join("complexity_curriculum.json"),
        &outcome.complexity_curriculum,
    )?;
    write_json(
        &directory.join("blind_manifest.json"),
        &outcome.blind_manifest,
    )?;
    write_json(
        &directory.join("freeze_record.json"),
        &outcome.freeze_record,
    )?;
    write_json(
        &directory.join("equal_resource_results.json"),
        &outcome.equal_resource_results,
    )?;
    write_json(
        &directory.join("equal_accuracy_results.json"),
        &outcome.equal_accuracy_results,
    )?;
    write_json(
        &directory.join("depth_tasks.json"),
        &outcome.class_results[&TaskClass::Depth],
    )?;
    write_json(
        &directory.join("width_tasks.json"),
        &outcome.class_results[&TaskClass::Width],
    )?;
    write_json(
        &directory.join("recombination_tasks.json"),
        &outcome.class_results[&TaskClass::Recombination],
    )?;
    write_json(
        &directory.join("composition_tasks.json"),
        &outcome.class_results[&TaskClass::Composition],
    )?;
    write_json(
        &directory.join("mixed_tasks.json"),
        &outcome.class_results[&TaskClass::Mixed],
    )?;
    write_json(
        &directory.join("frontier_metrics.json"),
        &outcome.frontier_metrics,
    )?;
    write_json(
        &directory.join("adaptive_controller_trace.json"),
        &outcome.adaptive_controller_trace,
    )?;
    write_json(
        &directory.join("information_gain_results.json"),
        &outcome.information_gain_results,
    )?;
    write_json(
        &directory.join("semantic_pruning_results.json"),
        &outcome.semantic_pruning_results,
    )?;
    write_json(
        &directory.join("decomposition_results.json"),
        &outcome.decomposition_results,
    )?;
    write_json(
        &directory.join("recombination_results.json"),
        &outcome.recombination_results,
    )?;
    write_json(
        &directory.join("controller_ablations.json"),
        &outcome.controller_ablations,
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
        &directory.join("reasoning_complexity_frontier.json"),
        &outcome.reasoning_complexity_frontier,
    )?;
    write_json(
        &directory.join("sem2_final_report.json"),
        &outcome.final_report,
    )?;
    write_text(&directory.join("SEM2_REPORT.md"), &markdown(outcome))?;
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

fn markdown(outcome: &Sem2Outcome) -> String {
    let report = &outcome.final_report;
    format!(
        "# SEM-2 Adaptive Reasoning Complexity Report\n\n\
         Status: `{}`\n\n\
         Disposition: `{}`\n\n\
         ## Integrity and protocol\n\n\
         Canonical and predecessor integrity passed. The failed `SEM1-RUN-0001` and sealed \
         successful `SEM1-RUN-0002` were verified before implementation. Four promoted \
         concepts remained immutable. The blind matrix was frozen before evaluation and no \
         post-blind tuning occurred. Network, external LLM, local teacher, and recursive source \
         mutation counts were all zero.\n\n\
         ## Metric semantics audit\n\n\
         SEM-1's `MAX_REASONING_WIDTH=28540` and corresponding live-branch field represented \
         cumulative candidate-plan generation, not instantaneous concurrency. SEM-2 reports \
         instantaneous frontier width, simultaneous live branches, cumulative branches, and \
         cumulative expansions separately. SEM-1 depth 56 counted dynamic execution work, \
         whereas primitive-expanded depth 17 counted static derivation nodes.\n\n\
         ## Equal-resource result\n\n\
         | Metric | Baseline B | Adaptive D |\n\
         |---|---:|---:|\n\
         | Strict solve rate | {:.6} | {:.6} |\n\
         | Median hard WIDTH/MIXED expansions | {:.3} | {:.3} |\n\
         | Peak simultaneous live branches | {} | {} |\n\n\
         Expansion reduction: `{:.6}`. Live-branch reduction: `{:.6}`.\n\n\
         ## Adaptive reasoning evidence\n\n\
         - Maximum solution graph depth: `{}`\n\
         - Maximum primitive-expanded depth: `{}`\n\
         - Maximum search trajectory depth: `{}`\n\
         - Maximum concepts composed: `{}`\n\
         - Maximum simultaneous subproblems: `{}`\n\
         - Information probes executed: `{}`\n\
         - Hypotheses eliminated: `{}`\n\
         - Semantic prunes / false prunes: `{}` / `{}`\n\
         - Semantic state merges / false merges: `{}` / `{}`\n\n\
         Deep reasoning, dynamic allocation, decomposition, recombination, semantic pruning, \
         frontier control, sparse routing, and all eight primary gates passed.\n\n\
         ## Stage boundary\n\n\
         SEM-3 was not started. The next allowed stage is \
         `SEM-3_ACTIVE_EXPERIMENT_SELECTION`.\n",
        report.sem2_status,
        report.disposition,
        report.baseline_b_solve_rate_equal_resource,
        report.adaptive_d_solve_rate_equal_resource,
        report.baseline_b_median_expansions_hard,
        report.adaptive_d_median_expansions_hard,
        report.baseline_b_peak_live_branches,
        report.adaptive_d_peak_live_branches,
        report.expansion_reduction,
        report.live_branch_reduction,
        report.max_solution_graph_depth,
        report.max_primitive_expanded_depth,
        report.max_search_trajectory_depth,
        report.max_concepts_composed,
        report.max_simultaneous_subproblems,
        report.information_probes_executed,
        report.hypotheses_eliminated,
        report.semantic_prunes,
        report.false_prunes,
        report.semantic_state_merges,
        report.false_merges,
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    #[test]
    fn all_required_reports_are_written() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root = manifest_dir
            .parent()
            .and_then(|path| path.parent())
            .expect("root")
            .to_path_buf();
        let outcome = crate::run_sem2(&root).expect("run");
        let scratch = std::env::temp_dir().join(format!("sem2-report-test-{}", std::process::id()));
        if scratch.exists() {
            std::fs::remove_dir_all(&scratch).expect("remove old scratch");
        }
        super::write_reports(&scratch, &outcome).expect("reports");
        for name in [
            "predecessor_integrity.json",
            "metric_semantics_audit.json",
            "complexity_curriculum.json",
            "blind_manifest.json",
            "equal_resource_results.json",
            "equal_accuracy_results.json",
            "depth_tasks.json",
            "width_tasks.json",
            "recombination_tasks.json",
            "composition_tasks.json",
            "mixed_tasks.json",
            "frontier_metrics.json",
            "adaptive_controller_trace.json",
            "information_gain_results.json",
            "semantic_pruning_results.json",
            "decomposition_results.json",
            "recombination_results.json",
            "controller_ablations.json",
            "sparse_activation_audit.json",
            "contamination_audit.json",
            "reasoning_complexity_frontier.json",
            "sem2_final_report.json",
            "SEM2_REPORT.md",
        ] {
            assert!(
                scratch.join("reports/sem2").join(name).is_file(),
                "missing {name}"
            );
        }
        std::fs::remove_dir_all(scratch).expect("remove scratch");
    }
}
