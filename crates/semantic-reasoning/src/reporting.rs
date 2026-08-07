use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::experiment::{Condition, Sem0Outcome};

pub fn write_reports(root: &Path, outcome: &Sem0Outcome) -> Result<(), String> {
    let report_root = root.join("reports/sem0");
    fs::create_dir_all(&report_root).map_err(|error| error.to_string())?;

    write_json(&report_root.join("environment.json"), &outcome.environment)?;
    write_json(
        &report_root.join("primitive_catalog.json"),
        &outcome.primitive_catalog,
    )?;
    write_json(
        &report_root.join("task_manifest_train.json"),
        &outcome.train_manifest,
    )?;
    write_json(
        &report_root.join("task_manifest_blind.json"),
        &outcome.blind_manifest,
    )?;
    write_json(
        &report_root.join("baseline_results.json"),
        &outcome.baseline_results,
    )?;
    write_json(
        &report_root.join("derivation_metrics.json"),
        &serde_json::json!({
            "train": outcome.train_results,
            "calibration": outcome.calibration_results,
            "fresh_blind": outcome.fresh_blind_results.per_condition,
        }),
    )?;
    write_json(
        &report_root.join("candidate_concepts.json"),
        &serde_json::json!({
            "mining": outcome.mining.report,
            "concepts": outcome.candidate_concepts,
            "structural_macros": outcome.structural_macros,
        }),
    )?;
    write_json(
        &report_root.join("semantic_gate_results.json"),
        &outcome.gate_results,
    )?;
    write_json(
        &report_root.join("counterfactual_results.json"),
        &outcome.counterfactual_results,
    )?;
    write_json(
        &report_root.join("fresh_blind_results.json"),
        &outcome.fresh_blind_results,
    )?;
    write_json(
        &report_root.join("ablation_results.json"),
        &outcome.ablation_results,
    )?;
    write_json(
        &report_root.join("leakage_audit.json"),
        &outcome.leakage_audit,
    )?;
    write_json(
        &report_root.join("lineage_graph.json"),
        &outcome.lineage_graph,
    )?;
    write_json(
        &report_root.join("sem0_final_report.json"),
        &outcome.final_report,
    )?;
    fs::write(report_root.join("SEM0_REPORT.md"), markdown(outcome))
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    fs::write(path, bytes).map_err(|error| error.to_string())
}

fn markdown(outcome: &Sem0Outcome) -> String {
    let summary = |condition| {
        outcome
            .baseline_results
            .summaries
            .iter()
            .find(|summary| summary.condition == condition)
            .expect("all conditions present")
    };
    let a = summary(Condition::PrimitiveOnly);
    let b = summary(Condition::SolutionCache);
    let c = summary(Condition::StructuralMacro);
    let d = summary(Condition::SemanticEvolution);
    format!(
        "# SEM-0 Report\n\n\
         Status: `{}`  \n\
         Disposition: `{}`  \n\
         Canonical pre-run self-hash: `{}`\n\n\
         ## Result\n\n\
         One opaque candidate (`C000001`) was mined by typed anti-unification of {} independently solved primitive derivations. No lexical alias was available to the engine. It passed all eight gates and was promoted only after frozen blind evaluation and causal ablation.\n\n\
         The structural-macro control had the same typed execution power and matched D on blind solve rate and expansions. Therefore this experiment does **not** claim a performance advantage of D over C. D differs through executable predictions, counterfactual validation, immutable provenance, promotion gates, and causal ablation.\n\n\
         ## Frozen controls\n\n\
         | Condition | Solved / attempted | Strict rate | Expansions | Max depth | Macro uses | Concept uses |\n\
         |---|---:|---:|---:|---:|---:|---:|\n\
         | A | {} / {} | {:.3} | {} | {} | {} | {} |\n\
         | B | {} / {} | {:.3} | {} | {} | {} | {} |\n\
         | C | {} / {} | {:.3} | {} | {} | {} | {} |\n\
         | D | {} / {} | {:.3} | {} | {} | {} | {} |\n\n\
         ## Semantic evidence\n\n\
         - Counterfactual pass rate: `{:.3}` ({} / {})\n\
         - Compression ratio: `{:.3}`\n\
         - Ablation solve-rate delta: `{:.3}`\n\
         - Ablation expansion delta (disabled minus enabled): `{}`\n\
         - Blind task manifest SHA-256: `{}`\n\
         - Candidate semantics SHA-256 before blind: `{}`\n\
         - Full catalog scans: `{}`\n\n\
         ## Contamination controls\n\n\
         Network, external LLM, local teacher, solution retrieval, expected-query lookup during solving, and recursive source mutation counts were all zero. The inherited recursive stack remained observe/measure-only. Blind expected outputs and hidden generator metadata were absent from the reasoner-visible manifest.\n\n\
         ## Scope\n\n\
         This is a single-generation, closed-world SEM-0 result. It does not establish general intelligence, does not validate later hypotheses, and does not start SEM-1. Human lexical interpretation is intentionally absent from the sealed canonical metrics and may be attached only afterward as forensic metadata.\n",
        outcome.final_report.sem0_status,
        outcome.final_report.disposition,
        outcome.environment.canonical_integrity.pre_run_manifest_self_hash_sha256,
        outcome.mining.report.aligned_occurrences,
        a.tasks_solved, a.tasks_attempted, a.strict_solve_rate, a.search_expansions, a.max_successful_reasoning_depth, a.macro_uses, a.concept_uses,
        b.tasks_solved, b.tasks_attempted, b.strict_solve_rate, b.search_expansions, b.max_successful_reasoning_depth, b.macro_uses, b.concept_uses,
        c.tasks_solved, c.tasks_attempted, c.strict_solve_rate, c.search_expansions, c.max_successful_reasoning_depth, c.macro_uses, c.concept_uses,
        d.tasks_solved, d.tasks_attempted, d.strict_solve_rate, d.search_expansions, d.max_successful_reasoning_depth, d.macro_uses, d.concept_uses,
        outcome.counterfactual_results.pass_rate,
        outcome.counterfactual_results.passed,
        outcome.counterfactual_results.attempted,
        outcome.final_report.compression_ratio,
        outcome.ablation_results.solve_rate_delta,
        outcome.ablation_results.search_expansion_delta,
        outcome.blind_manifest.manifest_sha256,
        outcome.fresh_blind_results.candidate_semantics_sha256_before_blind,
        outcome.leakage_audit.full_catalog_scans,
    )
}
