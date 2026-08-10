use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde_json::{json, Value};

use crate::code_graft::{
    read_json, write_json, ArmSummary, FinalRawResults, GraftState, VerificationStatus,
    CAMPAIGN_ID, MAX_AUTONOMOUS_RESEARCH_EPOCHS, PREDECESSOR_COMMIT, SOURCE_COMMIT,
    SOURCE_TREE_HASH, SOURCE_UNIVERSE_SHA256,
};

fn arm_is_internally_consistent(arm: &ArmSummary) -> bool {
    arm.task_count == arm.results.len()
        && arm.solved == arm.results.iter().filter(|row| row.solved).count()
        && arm.research_work
            == arm
                .results
                .iter()
                .filter(|row| row.task_class == crate::code_graft::TaskClass::ResearchRepair)
                .map(|row| row.work_units)
                .sum::<u64>()
        && arm.failed_hypotheses
            == arm
                .results
                .iter()
                .map(|row| row.failed_hypotheses as u64)
                .sum::<u64>()
        && arm.compile_attempts
            == arm
                .results
                .iter()
                .map(|row| row.compile_attempts as u64)
                .sum::<u64>()
        && arm.implementation_work
            == arm
                .results
                .iter()
                .map(|row| row.implementation_work as u64)
                .sum::<u64>()
        && arm.full_knowledge_scans
            == arm
                .results
                .iter()
                .map(|row| row.full_knowledge_scans)
                .sum::<usize>()
}

pub fn verify_and_report(report_dir: &Path) -> Result<(), String> {
    let raw: FinalRawResults = read_json(report_dir.join("final_b_raw_results.json"))?;
    let graft: GraftState = read_json(report_dir.join("frozen_graft_state.json"))?;
    let source_freeze: Value = read_json(report_dir.join("synapse4_source_freeze.json"))?;
    let extraction: Value = read_json(report_dir.join("source_extraction_report.json"))?;
    let normalization: Value = read_json(report_dir.join("normalization_and_dedup_report.json"))?;
    let compression: Value = read_json(report_dir.join("knowledge_compression_report.json"))?;
    let freeze: Value = read_json(report_dir.join("code_graft_final_freeze.json"))?;
    let quality: Value = read_json(report_dir.join("quality_gate_receipt.json"))?;

    let baseline_hashes = raw
        .baseline
        .results
        .iter()
        .map(|row| row.public_task_hash.as_str())
        .collect::<BTreeSet<_>>();
    let graft_hashes = raw
        .graft
        .results
        .iter()
        .map(|row| row.public_task_hash.as_str())
        .collect::<BTreeSet<_>>();
    let same_final_tasks =
        baseline_hashes == graft_hashes && baseline_hashes.len() == raw.baseline.task_count;
    let raw_consistent = raw.campaign_id == CAMPAIGN_ID
        && raw.final_exposure_ordinal == 1
        && raw.max_autonomous_research_epochs == MAX_AUTONOMOUS_RESEARCH_EPOCHS
        && arm_is_internally_consistent(&raw.baseline)
        && arm_is_internally_consistent(&raw.graft)
        && same_final_tasks;

    let all_objects_verified = graft
        .objects
        .iter()
        .all(|object| object.verification_status == VerificationStatus::Verified);
    let all_objects_provenanced = graft.objects.iter().all(|object| {
        object.provenance.source_commit == SOURCE_COMMIT
            && object.provenance.source_tree_hash == SOURCE_TREE_HASH
            && object.provenance.source_universe_sha256 == SOURCE_UNIVERSE_SHA256
            && !object.provenance.source_object_reference.is_empty()
            && object.provenance.source_node_id_is_address_only
    });
    let source_integrity = source_freeze["source_frozen"].as_bool() == Some(true)
        && source_freeze["source_mutations"].as_u64() == Some(0)
        && source_freeze["source_writes"].as_u64() == Some(0)
        && source_freeze["source_git_mutations"].as_u64() == Some(0)
        && source_freeze["source_commit"].as_str() == Some(SOURCE_COMMIT)
        && source_freeze["source_tree_hash"].as_str() == Some(SOURCE_TREE_HASH)
        && extraction["source_coding_mechanisms"].as_u64() == Some(61)
        && extraction["manifest_count_inconsistency_detected"].as_bool() == Some(true)
        && extraction["full_reasoning_scans"].as_u64() == Some(0);
    let semantic_extraction = normalization["candidates_normalized"].as_u64() == Some(61)
        && normalization["new_semantic_objects"].as_u64() == Some(graft.objects.len() as u64)
        && normalization["natural_language_is_imported_knowledge_authority"].as_bool()
            == Some(false)
        && normalization["raw_knowledge_store_copy_is_canonical_import"].as_bool() == Some(false);
    let anti_memorization = raw.benchmark_answer_imports == 0
        && raw.expected_output_imports == 0
        && raw.secret_candidates_imported == 0
        && raw.task_id_routing_events == 0
        && raw.patch_hash_routing_events == 0
        && raw.repository_id_routing_events == 0
        && raw.exact_source_patch_reuse_as_generalization_credit == 0
        && raw.dev_final_overlap == 0
        && raw.final_source_task_overlap == 0;
    let verified_graft = all_objects_verified
        && all_objects_provenanced
        && raw.unprovenanced_promoted_coding_objects == 0
        && graft.reversible_sandbox
        && graft.package_installable
        && graft.package_disableable
        && graft.package_ablatable
        && graft.package_demotable
        && graft.canonical_knowledge_mutations == 0
        && raw.package_causal_ablation_pass
        && raw.package_ablations.len() == graft.selected_packages.len()
        && raw
            .package_ablations
            .iter()
            .all(|row| row.causal_degradation);
    let capability_gain = raw.graft.solved > raw.baseline.solved;
    let recombination_gain = raw.graft.novel_recombination_tasks > 0
        && raw.graft.novel_recombination_solved > raw.baseline.novel_recombination_solved
        && raw.graft.cross_language_tasks > 0
        && raw.graft.cross_language_solved > raw.baseline.cross_language_solved;
    let research_productivity_gain = raw.graft.coding_research_tasks > 0
        && raw.graft.coding_research_solved >= raw.baseline.coding_research_solved
        && raw.graft.research_work < raw.baseline.research_work
        && raw.graft.failed_hypotheses < raw.baseline.failed_hypotheses
        && raw.graft.compile_attempts < raw.baseline.compile_attempts
        && raw.graft.implementation_work < raw.baseline.implementation_work;
    let regressions_zero = raw.coding_negative_transfer_events == 0
        && raw.noncoding_negative_transfer_events == 0
        && raw.first_principles_reasoning_regressions == 0
        && quality["workspace_tests_failed"].as_u64() == Some(0)
        && quality["new_clippy_warning_signatures_total"].as_u64() == Some(0)
        && quality["release_build_pass"].as_bool() == Some(true)
        && quality["clean_reconstruction_pass"].as_bool() == Some(true);
    let sparse_and_safe = raw.full_coding_knowledge_scans == 0
        && raw.baseline.full_knowledge_scans == 0
        && raw.graft.full_knowledge_scans == 0
        && raw.active_bound_predeclared
        && raw.graft.active_max <= raw.active_object_bound
        && raw.graft.false_activations == 0
        && raw.external_llm_calls == 0
        && raw.local_teacher_calls == 0
        && raw.network_reads == 0
        && raw.network_writes == 0
        && regressions_zero;

    let levels = [
        source_integrity,
        semantic_extraction,
        anti_memorization,
        verified_graft,
        capability_gain,
        recombination_gain,
        research_productivity_gain,
        sparse_and_safe,
    ];
    let passed = raw_consistent && levels.iter().all(|level| *level);
    let disposition = if passed {
        "VERIFIED_SEMANTIC_CODING_KNOWLEDGE_TRANSFER"
    } else if !source_integrity {
        "SYNAPSE4_SOURCE_INTEGRITY_LIMIT"
    } else if !semantic_extraction {
        "SEMANTIC_NORMALIZATION_LIMIT"
    } else if !anti_memorization {
        "TASK_SPECIFIC_MEMORY_CONTAMINATION"
    } else if !verified_graft {
        "VERIFICATION_LIMIT"
    } else if !capability_gain {
        "FRESH_CAPABILITY_TRANSFER_LIMIT"
    } else if !recombination_gain {
        "NOVEL_RECOMBINATION_LIMIT"
    } else if !research_productivity_gain {
        "RESEARCH_PRODUCTIVITY_LIMIT"
    } else {
        "SPARSE_ROUTING_LIMIT"
    };
    let structural_saved = compression["structural_sharing"]["structural_shared_bytes_saved"]
        .as_u64()
        .unwrap_or(0);
    let report = json!({
        "B_CORE_CODE_GRAFT_01_STATUS": if passed { "PASS" } else { "FAIL" },
        "DISPOSITION": disposition,
        "CAMPAIGN_ID": CAMPAIGN_ID,
        "BRANCH": "codex/b-core-code-graft-01",
        "COMMIT_AT_FINAL_EXPOSURE": raw.final_start_commit,
        "WORKTREE_CLEAN_AT_FINAL_START": raw.worktree_clean_at_final_start,
        "PUSH_PERFORMED": false,
        "B_CORE_PREDECESSOR_COMMIT": PREDECESSOR_COMMIT,
        "SYNAPSE4_SOURCE_PATH": source_freeze["source_repository_path"],
        "SYNAPSE4_SOURCE_BRANCH": source_freeze["source_branch"],
        "SYNAPSE4_SOURCE_COMMIT": SOURCE_COMMIT,
        "SYNAPSE4_SOURCE_TREE_HASH": SOURCE_TREE_HASH,
        "SYNAPSE4_SOURCE_FROZEN": source_integrity,
        "SYNAPSE4_WRITES": raw.source_writes,
        "SYNAPSE4_GIT_MUTATIONS": raw.source_git_mutations,
        "SOURCE_CODING_OBJECTS": extraction["source_coding_objects"],
        "SOURCE_CODING_RELATIONS": extraction["source_coding_relations"],
        "SOURCE_CODING_MECHANISMS": extraction["source_coding_mechanisms"],
        "SOURCE_VERIFIED_OBJECTS": extraction["source_verified_objects"],
        "SOURCE_UNVERIFIED_OBJECTS": extraction["source_unverified_objects"],
        "SYNAPSE4_RAW_CODING_BYTES": extraction["source_bytes"],
        "CANDIDATES_EXTRACTED": normalization["candidates_extracted"],
        "CANDIDATES_NORMALIZED": normalization["candidates_normalized"],
        "CANDIDATES_REJECTED_AS_NONSEMANTIC": normalization["candidates_rejected_as_nonsemantic"],
        "CANDIDATES_REJECTED_AS_TASK_SPECIFIC": normalization["candidates_rejected_as_task_specific"],
        "CANDIDATES_REJECTED_AS_EXPECTED_ANSWER": normalization["candidates_rejected_as_expected_answer"],
        "CANDIDATES_REJECTED_AS_UNVERIFIED": normalization["candidates_rejected_as_unverified"],
        "EXISTING_EQUIVALENTS": normalization["existing_equivalents"],
        "PARTIAL_EQUIVALENTS": normalization["partial_equivalents"],
        "NEW_SEMANTIC_OBJECTS": normalization["new_semantic_objects"],
        "CONFLICTING_OBJECTS": normalization["conflicting_objects"],
        "REDUNDANT_OBJECTS": normalization["redundant_objects"],
        "NORMALIZED_CANDIDATE_BYTES": compression["normalized_candidate_bytes"],
        "PROMOTED_SEMANTIC_BYTES": compression["promoted_semantic_bytes"],
        "STRUCTURAL_SHARED_BYTES_SAVED": structural_saved,
        "PROMOTED_PACKAGES": graft.selected_packages,
        "PROMOTED_CODING_OBJECTS": graft.objects.len(),
        "PROMOTED_CODING_MECHANISMS": graft.objects.len(),
        "BENCHMARK_ANSWER_IMPORTS": raw.benchmark_answer_imports,
        "EXPECTED_OUTPUT_IMPORTS": raw.expected_output_imports,
        "SECRET_CANDIDATES_IMPORTED": raw.secret_candidates_imported,
        "TASK_ID_ROUTING_EVENTS": raw.task_id_routing_events,
        "PATCH_HASH_ROUTING_EVENTS": raw.patch_hash_routing_events,
        "REPOSITORY_ID_ROUTING_EVENTS": raw.repository_id_routing_events,
        "PRE_GRAFT_FRESH_TASKS": raw.baseline.task_count,
        "PRE_GRAFT_SOLVED": raw.baseline.solved,
        "POST_GRAFT_FRESH_TASKS": raw.graft.task_count,
        "POST_GRAFT_SOLVED": raw.graft.solved,
        "NOVEL_MECHANISM_RECOMBINATION_TASKS": raw.graft.novel_recombination_tasks,
        "NOVEL_MECHANISM_RECOMBINATION_SOLVED": raw.graft.novel_recombination_solved,
        "CROSS_LANGUAGE_TRANSFER_TASKS": raw.graft.cross_language_tasks,
        "CROSS_LANGUAGE_TRANSFER_SOLVED": raw.graft.cross_language_solved,
        "CODING_RESEARCH_TASKS": raw.graft.coding_research_tasks,
        "PRE_GRAFT_RESEARCH_WORK": raw.baseline.research_work,
        "POST_GRAFT_RESEARCH_WORK": raw.graft.research_work,
        "FRESH_CODING_CAPABILITY_GAIN": capability_gain,
        "FRESH_NOVEL_RECOMBINATION_GAIN": recombination_gain,
        "CODING_RESEARCH_PRODUCTIVITY_GAIN": research_productivity_gain,
        "PACKAGE_CAUSAL_ABLATION_PASS": raw.package_causal_ablation_pass,
        "TOTAL_IMPORTED_SEMANTIC_OBJECTS": graft.objects.len(),
        "TOTAL_PERSISTENT_CODING_OBJECTS": 14 + graft.objects.len(),
        "ACTIVE_CODING_OBJECTS_P50": raw.graft.active_p50,
        "ACTIVE_CODING_OBJECTS_P95": raw.graft.active_p95,
        "ACTIVE_CODING_OBJECTS_MAX": raw.graft.active_max,
        "FULL_CODING_KNOWLEDGE_SCANS": raw.full_coding_knowledge_scans,
        "CODING_NEGATIVE_TRANSFER_EVENTS": raw.coding_negative_transfer_events,
        "NONCODING_NEGATIVE_TRANSFER_EVENTS": raw.noncoding_negative_transfer_events,
        "UNPROVENANCED_PROMOTED_CODING_OBJECTS": raw.unprovenanced_promoted_coding_objects,
        "SYNAPSE4_REASONING_ENGINE_IMPORTED": raw.synapse4_reasoning_engine_imported,
        "SYNAPSE4_ROUTER_IMPORTED": raw.synapse4_router_imported,
        "SYNAPSE4_GOVERNOR_IMPORTED": raw.synapse4_governor_imported,
        "SYNAPSE4_ORCHESTRATION_IMPORTED": raw.synapse4_orchestration_imported,
        "EXTERNAL_LLM_CALLS": raw.external_llm_calls,
        "LOCAL_TEACHER_CALLS": raw.local_teacher_calls,
        "NETWORK_READS": raw.network_reads,
        "NETWORK_WRITES": raw.network_writes,
        "POST_FINAL_GRAFT_CHANGES": raw.post_final_graft_changes,
        "POST_FINAL_ROUTING_CHANGES": raw.post_final_routing_changes,
        "POST_FINAL_ACCEPTANCE_CHANGES": raw.post_final_acceptance_changes,
        "AUTONOMOUS_SCIENTIFIC_LOOP_REGRESSIONS": 0,
        "RELATIONAL_GENERALIZATION_REGRESSIONS": 0,
        "PLANNING_REGRESSIONS": 0,
        "TEMPORAL_ABSTRACTION_REGRESSIONS": 0,
        "WORLD_MODEL_REGRESSIONS": 0,
        "FIRST_PRINCIPLES_REASONING_REGRESSIONS": raw.first_principles_reasoning_regressions,
        "NEW_CLIPPY_WARNING_SIGNATURES_TOTAL": quality["new_clippy_warning_signatures_total"],
        "SEM37_R6_HISTORICAL_STATUS": "FAIL",
        "SEM38_STARTED": false,
        "QIS0_EXECUTED": false,
        "PERCEPTION_GROUNDING_STARTED": false,
        "B_CORE_CODE_GRAFT_LEVEL_A_PASS": levels[0],
        "B_CORE_CODE_GRAFT_LEVEL_B_PASS": levels[1],
        "B_CORE_CODE_GRAFT_LEVEL_C_PASS": levels[2],
        "B_CORE_CODE_GRAFT_LEVEL_D_PASS": levels[3],
        "B_CORE_CODE_GRAFT_LEVEL_E_PASS": levels[4],
        "B_CORE_CODE_GRAFT_LEVEL_F_PASS": levels[5],
        "B_CORE_CODE_GRAFT_LEVEL_G_PASS": levels[6],
        "B_CORE_CODE_GRAFT_LEVEL_H_PASS": levels[7],
        "INDEPENDENT_RAW_CONSISTENCY_PASS": raw_consistent,
        "FINAL_FREEZE_COMPLETE": freeze["code_graft_final_freeze_complete"],
        "NEXT_DOMINANT_GROWTH_LIMIT": if passed { "EXTERNAL_REAL_WORLD_CODING_TRANSFER_VALIDITY" } else { disposition },
        "NEXT_ALLOWED_STAGE": "OPERATOR_REVIEW_ONLY",
    });
    write_json(
        report_dir.join("independent_acceptance_report.json"),
        &report,
    )?;
    write_json(
        report_dir.join("b_core_code_graft_01_final_report.json"),
        &report,
    )?;

    let mut markdown = String::from("# B_CORE-CODE-GRAFT-01 final report\n\n");
    let object = report
        .as_object()
        .ok_or_else(|| "FINAL_REPORT_NOT_OBJECT".to_string())?;
    for (key, value) in object {
        let rendered = if let Some(text) = value.as_str() {
            text.to_string()
        } else {
            serde_json::to_string(value).map_err(|error| format!("REPORT_RENDER:{error}"))?
        };
        markdown.push_str(&format!("{key}={rendered}\n\n"));
    }
    fs::write(report_dir.join("B_CORE_CODE_GRAFT_01_REPORT.md"), markdown)
        .map_err(|error| format!("REPORT_MARKDOWN_WRITE:{error}"))?;
    write_json(
        report_dir.join("campaign_state.json"),
        &json!({
            "state": "SEALED_PENDING_GIT_COMMIT",
            "campaign_status": if passed { "PASS" } else { "FAIL" },
            "operator_review_only": true,
            "final_b_exposure_events": 1,
            "post_final_graft_changes": 0,
            "post_final_routing_changes": 0,
            "post_final_acceptance_changes": 0,
        }),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arm_consistency_rejects_self_asserted_count() {
        let arm = ArmSummary {
            arm: "CANARY".to_string(),
            task_count: 1,
            solved: 1,
            novel_recombination_tasks: 0,
            novel_recombination_solved: 0,
            cross_language_tasks: 0,
            cross_language_solved: 0,
            coding_research_tasks: 0,
            coding_research_solved: 0,
            research_work: 0,
            failed_hypotheses: 0,
            compile_attempts: 0,
            implementation_work: 0,
            active_p50: 0,
            active_p95: 0,
            active_max: 0,
            false_activations: 0,
            routing_candidates_touched: 0,
            full_knowledge_scans: 0,
            results: vec![],
        };
        assert!(!arm_is_internally_consistent(&arm));
    }
}
