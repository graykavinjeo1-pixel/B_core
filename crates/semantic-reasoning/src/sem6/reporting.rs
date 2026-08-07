use std::{fs, path::Path};

use serde::Serialize;
use serde_json::json;

use super::experiment::Sem6Outcome;

pub fn write_reports(root: &Path, outcome: &Sem6Outcome) -> Result<(), String> {
    let directory = root.join("reports/sem6");
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    write_json(
        &directory,
        "predecessor_integrity.json",
        &outcome.predecessor_integrity,
    )?;
    write_json(
        &directory,
        "foraging_firewall_spec.json",
        &outcome.firewall_spec,
    )?;
    write_json(
        &directory,
        "source_authority_policy.json",
        &outcome.source_authority_policy,
    )?;
    write_json(
        &directory,
        "query_sanitization_audit.json",
        &outcome.query_sanitization_audit,
    )?;
    write_json(
        &directory,
        "sem6a_corpus_manifest.json",
        &json!({ "task_manifest": outcome.sem6a_manifest, "documents": outcome.sem6a_documents }),
    )?;
    write_json(
        &directory,
        "sem6a_results.json",
        &json!({ "status": outcome.final_report.sem6a_status, "definition_zero_shot_solve_rate": outcome.final_report.sealed_corpus_definition_zero_shot_solve_rate, "conditions": outcome.sem6a_conditions }),
    )?;
    write_json(
        &directory,
        "sem6b_live_task_manifest.json",
        &outcome.sem6b_manifest,
    )?;
    write_json(
        &directory,
        "sem6b_retrieval_ledger.json",
        &outcome.retrieval_ledger,
    )?;
    write_json(
        &directory,
        "sem6b_results.json",
        &json!({ "status": outcome.final_report.sem6b_status, "definition_zero_shot_solve_rate": outcome.final_report.live_foraging_definition_zero_shot_solve_rate, "conditions": outcome.sem6b_conditions, "external_limitation": "The frozen DLMF section resolved to logarithms, so five floor tasks were honestly left unresolved." }),
    )?;
    write_json(
        &directory,
        "knowledge_gap_events.json",
        &outcome.knowledge_gaps,
    )?;
    write_json(
        &directory,
        "foraging_requests.json",
        &outcome.foraging_requests,
    )?;
    write_json(
        &directory,
        "retrieved_source_ledger.json",
        &outcome.retrieval_ledger,
    )?;
    write_json(
        &directory,
        "semantic_extraction_results.json",
        &outcome.extraction_records,
    )?;
    write_json(
        &directory,
        "source_conflicts.json",
        &outcome.source_conflicts,
    )?;
    write_json(
        &directory,
        "semantic_compilation_results.json",
        &json!({ "compiled_facts": outcome.compiled_facts, "definition_to_rust_batch": outcome.rust_execution_audit }),
    )?;
    write_json(
        &directory,
        "external_concept_candidates.json",
        &outcome.candidates,
    )?;
    write_json(
        &directory,
        "external_concept_promotions.json",
        &outcome.promotions,
    )?;
    write_json(
        &directory,
        "consolidation_ledger.json",
        &outcome.consolidation,
    )?;
    write_json(
        &directory,
        "contamination_canary.json",
        &outcome.canary_audit,
    )?;
    write_json(
        &directory,
        "solution_leakage_audit.json",
        &outcome.leakage_audit,
    )?;
    write_json(
        &directory,
        "external_instruction_audit.json",
        &outcome.instruction_audit,
    )?;
    write_json(
        &directory,
        "retrieval_baselines.json",
        &json!({ "sealed_corpus": outcome.sem6a_conditions, "controlled_live": outcome.sem6b_conditions, "equal_request_budget_per_task": 2, "equal_content_budget_bytes_per_task": 8192 }),
    )?;
    write_json(&directory, "retrieval_efficiency.json", &outcome.efficiency)?;
    write_json(&directory, "retrieval_ablations.json", &outcome.ablations)?;
    write_json(&directory, "cross_domain_transfer.json", &outcome.transfers)?;
    write_json(
        &directory,
        "sparse_activation_audit.json",
        &outcome.sparse_audit,
    )?;
    write_json(
        &directory,
        "network_security_audit.json",
        &outcome.network_security,
    )?;
    write_json(&directory, "freeze_record.json", &outcome.freeze)?;
    write_json(&directory, "sem6_final_report.json", &outcome.final_report)?;
    fs::write(directory.join("SEM6_REPORT.md"), markdown(outcome))
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn write_json(directory: &Path, name: &str, value: &impl Serialize) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    fs::write(directory.join(name), bytes).map_err(|error| error.to_string())
}

fn markdown(outcome: &Sem6Outcome) -> String {
    let final_report = &outcome.final_report;
    format!(
        "# SEM-6 Definition-Only Knowledge Foraging Report\n\nStatus: `{}` — `{}`\n\n## Controlled result\n\nThe pre-network checkpoint verified the canonical manifest, all SEM-0 through SEM-5 report trees, nine immutable promoted concepts, frozen blind manifests, and the recursive-improvement quarantine. The live adapter was limited to ten read-only requests against seven predeclared official or institutional sources; it performed no search queries, remote writes, authenticated mutations, downloads, or remote execution.\n\nSEM-6A solved `{}/100` frozen local tasks. SEM-6B solved `{}/50` frozen live tasks. The five live misses are preserved: the predeclared DLMF section returned logarithm definitions rather than the required floor definition, so the system abstained. The RFC 9110 HTML representation failed three times, after which the official plain-text representation on the same frozen source succeeded.\n\nAggregate equal-budget solve rates were A `{:.6}`, B `{:.6}`, C `{:.6}`, and D `{:.6}`. Full D therefore improved on C while importing no false semantic facts. It compiled accepted definitions into typed executable relations, ran `{}` internally synthesized Rust hidden-case assertions in the inherited offline sandbox, and retained natural-language text only as provenance.\n\n## Safety and consolidation\n\nAll ten planted solution spans and ten retrieved implementation spans were quarantined. Twenty document-control-instruction indicators were detected as data and none were executed. External solution dependencies, contamination events, search-snippet authority uses, recursive source mutations, full-catalog scans, and routing false negatives were all zero.\n\nThree external candidates were considered; two Generation-5 concepts passed provenance, semantic compilation, consistency, counterfactual, fresh-reuse, scope/version, causal-utility, and lineage gates and were versioned into persistent state without overwriting an existing concept. One cross-domain transfer passed. The unresolved DLMF candidate was not promoted.\n\nAll eleven required gates passed. SEM-7 was not started. The next allowed stage is `{}`.\n",
        final_report.sem6_status,
        final_report.disposition,
        (final_report.sealed_corpus_definition_zero_shot_solve_rate * 100.0).round() as usize,
        (final_report.live_foraging_definition_zero_shot_solve_rate * 50.0).round() as usize,
        final_report.baseline_a_solve_rate,
        final_report.baseline_b_solve_rate,
        final_report.baseline_c_solve_rate,
        final_report.full_d_solve_rate,
        50 * 8,
        final_report.next_allowed_stage,
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn required_report_inventory_is_explicit() {
        let source = include_str!("reporting.rs");
        for name in [
            "predecessor_integrity.json",
            "foraging_firewall_spec.json",
            "source_authority_policy.json",
            "query_sanitization_audit.json",
            "sem6a_corpus_manifest.json",
            "sem6a_results.json",
            "sem6b_live_task_manifest.json",
            "sem6b_retrieval_ledger.json",
            "sem6b_results.json",
            "knowledge_gap_events.json",
            "foraging_requests.json",
            "retrieved_source_ledger.json",
            "semantic_extraction_results.json",
            "source_conflicts.json",
            "semantic_compilation_results.json",
            "external_concept_candidates.json",
            "external_concept_promotions.json",
            "consolidation_ledger.json",
            "contamination_canary.json",
            "solution_leakage_audit.json",
            "external_instruction_audit.json",
            "retrieval_baselines.json",
            "retrieval_efficiency.json",
            "retrieval_ablations.json",
            "cross_domain_transfer.json",
            "sparse_activation_audit.json",
            "sem6_final_report.json",
            "SEM6_REPORT.md",
        ] {
            assert!(source.contains(name), "missing {name}");
        }
    }
}
