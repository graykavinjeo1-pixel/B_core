use std::{fs, path::Path};

use serde::Serialize;

use super::experiment::Sem1Outcome;

pub fn write_reports(root: &Path, outcome: &Sem1Outcome) -> Result<(), String> {
    let report_dir = root.join("reports/sem1");
    fs::create_dir_all(&report_dir).map_err(|error| error.to_string())?;

    write_json(
        &report_dir.join("predecessor_integrity.json"),
        &outcome.predecessor_integrity,
    )?;
    write_json(
        &report_dir.join("curriculum_manifest.json"),
        &outcome.curriculum_manifest,
    )?;
    write_json(
        &report_dir.join("blind_manifest.json"),
        &outcome.blind_manifest,
    )?;
    write_json(
        &report_dir.join("freeze_record.json"),
        &outcome.freeze_record,
    )?;
    write_json(
        &report_dir.join("concept_generation_ledger.json"),
        &outcome.concept_generation_ledger,
    )?;
    write_json(
        &report_dir.join("concept_lineage.json"),
        &outcome.concept_lineage,
    )?;
    write_json(
        &report_dir.join("adaptive_reasoning_metrics.json"),
        &outcome.adaptive_reasoning_metrics,
    )?;
    write_json(
        &report_dir.join("structural_macro_baseline.json"),
        &outcome.structural_macro_baseline,
    )?;
    write_json(
        &report_dir.join("semantic_baseline.json"),
        &outcome.semantic_baseline,
    )?;
    write_json(
        &report_dir.join("all_conditions.json"),
        &outcome.all_conditions,
    )?;
    write_json(
        &report_dir.join("semantic_vs_macro.json"),
        &outcome.semantic_vs_macro,
    )?;
    write_json(
        &report_dir.join("counterfactual_results.json"),
        &outcome.counterfactual_results,
    )?;
    write_json(
        &report_dir.join("adversarial_transfer_results.json"),
        &outcome.adversarial_transfer_results,
    )?;
    write_json(
        &report_dir.join("causal_ladder_ablation.json"),
        &outcome.causal_ladder_ablation,
    )?;
    write_json(
        &report_dir.join("compression_across_generations.json"),
        &outcome.compression_across_generations,
    )?;
    write_json(
        &report_dir.join("sparse_activation_audit.json"),
        &outcome.sparse_activation_audit,
    )?;
    write_json(
        &report_dir.join("leakage_audit.json"),
        &outcome.leakage_audit,
    )?;
    write_json(
        &report_dir.join("sem1_final_report.json"),
        &outcome.final_report,
    )?;
    write_text(
        &report_dir.join("SEM1_REPORT.md"),
        &markdown_report(outcome),
    )?;
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
            .and_then(|extension| extension.to_str())
            .unwrap_or("report")
    ));
    fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    if path.exists() {
        fs::remove_file(path).map_err(|error| error.to_string())?;
    }
    fs::rename(&temporary, path).map_err(|error| error.to_string())
}

fn markdown_report(outcome: &Sem1Outcome) -> String {
    let report = &outcome.final_report;
    let semantic = &outcome.semantic_vs_macro;
    let lineage = &outcome.concept_lineage;
    let counterfactual = &outcome.counterfactual_results;

    format!(
        "# SEM-1 Recursive Concept Ladder Report\n\n\
         ## Disposition\n\n\
         - Status: `{status}`\n\
         - Disposition: `{disposition}`\n\
         - Predecessor integrity: `{predecessor}`\n\
         - Canonical integrity: `{canonical}`\n\
         - Blind set frozen before evaluation: `{frozen}`\n\
         - Post-blind tuning: `{post_blind}`\n\n\
         ## Recursive Ladder\n\n\
         `C000001` remained immutable and was executed as an actual ancestor during discovery. \
         The miner produced {gen2_candidates} Generation-2 candidates and promoted {gen2_promoted}.\n\n\
         - Maximum autonomous concept generation: `{max_generation}`\n\
         - Best Generation-2 concept: `{best_id}`\n\
         - Post-hoc interpretation: {best_interpretation}\n\
         - Generation-2 ablation pass: `{gen2_ablation}`\n\
         - Generation-1 ancestor ablation pass: `{gen1_ablation}`\n\
         - Expanded derivations preserved: `{expanded}`\n\n\
         ## Semantic Separation\n\n\
         Baseline C is a typed, parameterized structural graph-macro system with structural \
         matching, macro composition, and macro-on-macro reuse. It was not intentionally \
         weakened. Condition D adds explicit semantic preconditions, safe abstention, \
         relation-based equivalence, and counterfactual applicability checks.\n\n\
         | Metric | Structural C | Semantic D | D minus C |\n\
         |---|---:|---:|---:|\n\
         | Strict solve rate | {c_rate:.6} | {d_rate:.6} | {solve_delta:.6} |\n\
         | Search expansions | {c_expansions} | {d_expansions} | {expansion_delta} |\n\
         | False-transfer rate | {c_false:.6} | {d_false:.6} | {false_delta:.6} |\n\
         | Invalid abstention rate | {c_abstain:.6} | {d_abstain:.6} | {abstain_delta:.6} |\n\n\
         Semantic separation pass: `{semantic_pass}`.\n\n\
         ## Generalization And Counterfactuals\n\n\
         - Frozen fresh-blind tasks: `{blind_tasks}`\n\
         - Counterfactual probes: `{counterfactual_tests}` ({counterfactual_passed} passed)\n\
         - Valid counterfactual prediction accuracy: `{valid_accuracy:.6}`\n\
         - Invalid-case rejection accuracy: `{invalid_accuracy:.6}`\n\
         - Adversarial transfer tests: `{adversarial_tests}`\n\n\
         ## Adaptive Complexity\n\n\
         - Maximum successful reasoning depth: `{max_depth}`\n\
         - Maximum primitive-expanded depth: `{primitive_depth}`\n\
         - Maximum reasoning width: `{width}`\n\
         - Maximum live branches: `{branches}`\n\
         - Maximum concepts composed: `{composed}`\n\
         - Maximum graph nodes / edges: `{nodes}` / `{edges}`\n\
         - Peak active concepts: `{active}`\n\
         - Best multi-generation compression ratio: `{compression:.6}`\n\n\
         ## Sparse Activation And Quarantine\n\n\
         - Full catalog scans: `{scans}`\n\
         - Routing false negatives: `{routing_false_negatives}`\n\
         - Network / external LLM / local teacher calls: `0 / 0 / 0`\n\
         - Recursive source mutations: `0`\n\
         - `SELF_OBSERVE=true`\n\
         - `SELF_MEASURE=true`\n\
         - `SELF_PROPOSE=false`\n\
         - `SELF_APPLY=false`\n\
         - `SOURCE_MUTATION=false`\n\n\
         ## Lineage\n\n\
         The exact lineage DAG is serialized in `concept_lineage.json`; it contains \
         {lineage_nodes} nodes and {lineage_edges} edges. Primitive expansion is reconstructable.\n\n\
         ## Next Stage\n\n\
         SEM-2 was not started. The next allowed stage is \
         `SEM-2_ADAPTIVE_REASONING_COMPLEXITY`.\n",
        status = report.sem1_status,
        disposition = report.disposition,
        predecessor = report.predecessor_integrity,
        canonical = report.canonical_integrity,
        frozen = outcome.freeze_record.frozen_before_blind,
        post_blind = outcome.freeze_record.post_blind_tuning,
        gen2_candidates = report.gen2_candidates,
        gen2_promoted = report.gen2_promoted,
        max_generation = report.max_autonomous_concept_generation,
        best_id = report.best_gen2_concept_id,
        best_interpretation = report.best_gen2_posthoc_interpretation,
        gen2_ablation = report.gen2_ablation_pass,
        gen1_ablation = report.gen1_ancestor_ablation_pass,
        expanded = outcome.compression_across_generations.expanded_derivations_preserved,
        c_rate = semantic.baseline_c_solve_rate,
        d_rate = semantic.semantic_d_solve_rate,
        solve_delta = semantic.d_vs_c_solve_delta,
        c_expansions = outcome.structural_macro_baseline.search_expansions,
        d_expansions = outcome.semantic_baseline.search_expansions,
        expansion_delta = semantic.d_vs_c_search_expansion_delta,
        c_false = outcome.structural_macro_baseline.false_transfer_rate,
        d_false = outcome.semantic_baseline.false_transfer_rate,
        false_delta = semantic.d_vs_c_false_transfer_delta,
        c_abstain = outcome.structural_macro_baseline.invalid_abstention_rate,
        d_abstain = outcome.semantic_baseline.invalid_abstention_rate,
        abstain_delta = semantic.d_vs_c_invalid_abstention_delta,
        semantic_pass = report.semantic_separation_pass,
        blind_tasks = report.fresh_blind_tasks,
        counterfactual_tests = report.counterfactual_tests,
        counterfactual_passed = counterfactual.passed,
        valid_accuracy = counterfactual.valid_counterfactual_prediction_accuracy,
        invalid_accuracy = counterfactual.invalid_case_rejection_accuracy,
        adversarial_tests = report.adversarial_transfer_tests,
        max_depth = report.max_successful_reasoning_depth,
        primitive_depth = report.max_primitive_expanded_depth,
        width = report.max_reasoning_width,
        branches = report.max_live_branches,
        composed = report.max_concepts_composed,
        nodes = report.max_reasoning_graph_nodes,
        edges = report.max_reasoning_graph_edges,
        active = report.peak_active_concepts,
        compression = report.best_multi_generation_compression_ratio,
        scans = report.full_catalog_scans,
        routing_false_negatives = report.routing_false_negatives,
        lineage_nodes = lineage.nodes.len(),
        lineage_edges = lineage.edges.len(),
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    #[test]
    fn required_report_names_are_written() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root = manifest_dir
            .parent()
            .and_then(|path| path.parent())
            .expect("workspace root");
        let outcome = crate::run_sem1(root).expect("SEM-1 run");
        let scratch = std::env::temp_dir().join(format!("sem1-report-test-{}", std::process::id()));
        if scratch.exists() {
            std::fs::remove_dir_all(&scratch).expect("remove old scratch");
        }
        super::write_reports(&scratch, &outcome).expect("write reports");
        for name in [
            "predecessor_integrity.json",
            "curriculum_manifest.json",
            "blind_manifest.json",
            "concept_generation_ledger.json",
            "concept_lineage.json",
            "adaptive_reasoning_metrics.json",
            "structural_macro_baseline.json",
            "semantic_baseline.json",
            "semantic_vs_macro.json",
            "counterfactual_results.json",
            "adversarial_transfer_results.json",
            "causal_ladder_ablation.json",
            "compression_across_generations.json",
            "sparse_activation_audit.json",
            "leakage_audit.json",
            "sem1_final_report.json",
            "SEM1_REPORT.md",
        ] {
            assert!(
                scratch.join("reports/sem1").join(name).is_file(),
                "missing {name}"
            );
        }
        std::fs::remove_dir_all(scratch).expect("remove scratch");
    }
}
