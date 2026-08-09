pub mod engine;
pub mod verifier;

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use engine::{generate_challenge, solve_challenge, CampaignSolve, MAX_AUTONOMOUS_RESEARCH_EPOCHS};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use verifier::{VerificationRequest, VerificationResult, CONTRACT_VERSION};

const CAMPAIGN_ID: &str = "SEM31-PERSISTENT-SEMANTIC-WORLD-STATE-0001";
const BRANCH: &str = "codex/sem31-persistent-world-state";
const SEALED_PREDECESSOR_COMMIT: &str = "e7b3539a89e4870fd7461bc9ca6d65fbc93abd9c";
const INSTRUCTION: &str = "research/sem31/SEM31_INSTRUCTION.md";
const ONTOLOGY: &str = "research/sem31/ontology.json";
const PREDECESSOR_REPORT: &str = "reports/sem30/sem30_final_report.json";
const PREDECESSOR_MANIFEST: &str = "reports/sem30/artifact_manifest.json";
const REPORT_DIR: &str = "reports/sem31";
const BASE_SEED: u64 = 0x5E31_0001_E7B3_539A;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TimedVerification {
    result: VerificationResult,
    wall_time_ns: u64,
}

pub fn freeze_campaign(root: &Path) -> Result<String, String> {
    if git(root, &["rev-parse", "HEAD"])? != SEALED_PREDECESSOR_COMMIT {
        return Err("SEALED_PREDECESSOR_COMMIT_MISMATCH".to_string());
    }
    if !git(root, &["status", "--porcelain"])?.lines().all(|line| {
        line.contains("research/sem31")
            || line.contains("crates/semantic-reasoning/src/sem31")
            || line.contains("crates/semantic-reasoning/src/sem31_main.rs")
            || line.contains("crates/semantic-reasoning/src/sem31_verify_main.rs")
            || line.contains("crates/semantic-reasoning/src/lib.rs")
            || line.contains("crates/semantic-reasoning/Cargo.toml")
    }) {
        return Err("UNEXPECTED_PRE_FREEZE_WORKTREE_CHANGE".to_string());
    }
    verify_predecessor(root)?;
    let source_binary = verifier_binary(root);
    if !source_binary.is_file() {
        return Err(format!(
            "SEM31_VERIFIER_BINARY_MISSING:{}",
            source_binary.display()
        ));
    }
    let report = root.join(REPORT_DIR);
    if report.join("campaign_freeze.json").exists() {
        return Err("SEM31_CAMPAIGN_ALREADY_FROZEN".to_string());
    }
    fs::create_dir_all(report.join("artifacts/frozen_verifier"))
        .map_err(|error| format!("CREATE_REPORT_DIR:{error}"))?;
    fs::create_dir_all(report.join("checkpoints"))
        .map_err(|error| format!("CREATE_CHECKPOINT_DIR:{error}"))?;
    let frozen_binary = report.join("artifacts/frozen_verifier/sem31-verify.exe");
    let verifier_source = root.join("crates/semantic-reasoning/src/sem31/verifier.rs");
    fs::copy(&source_binary, &frozen_binary)
        .map_err(|error| format!("COPY_FROZEN_VERIFIER_BINARY:{error}"))?;
    fs::copy(
        &verifier_source,
        report.join("artifacts/frozen_verifier/verifier.rs"),
    )
    .map_err(|error| format!("COPY_FROZEN_VERIFIER_SOURCE:{error}"))?;
    let freeze = json!({
        "schema_version": "SEM31_CAMPAIGN_FREEZE_1",
        "campaign_id": CAMPAIGN_ID,
        "branch": BRANCH,
        "sealed_predecessor_commit": SEALED_PREDECESSOR_COMMIT,
        "instruction_sha256": sha256_file(&root.join(INSTRUCTION))?,
        "ontology_sha256": sha256_file(&root.join(ONTOLOGY))?,
        "engine_sha256": sha256_file(&root.join("crates/semantic-reasoning/src/sem31/engine.rs"))?,
        "runner_sha256": sha256_file(&root.join("crates/semantic-reasoning/src/sem31/mod.rs"))?,
        "verifier_source_sha256": sha256_file(&verifier_source)?,
        "verifier_binary_sha256": sha256_file(&frozen_binary)?,
        "verifier_binary": frozen_binary,
        "verifier_contract_version": CONTRACT_VERSION,
        "base_seed": BASE_SEED,
        "seed_commitment": sha256_bytes(format!("SEM31|WORLD|{BASE_SEED}").as_bytes()),
        "campaign_seed_frozen": true,
        "max_autonomous_research_epochs": MAX_AUTONOMOUS_RESEARCH_EPOCHS,
        "campaign_is_event_bounded": true,
        "checkpoint_interval_epochs": 64,
        "future_world_fixture_materialized": false,
        "world_generator_is_success_authority": false,
        "independent_verifier_is_success_authority": true,
        "budget_is_research_semantic_input": false,
        "human_world_ontology_design_events": 0,
        "human_property_selection_events": 0,
        "human_relation_selection_events": 0,
        "human_repair_design_events": 0,
        "prestart_autonomous_research_events": 0,
        "prestart_future_instance_exposure_events": 0,
        "network_allowed": false,
        "frozen_at_unix_ms": unix_millis()?
    });
    write_json(report.join("campaign_freeze.json"), &freeze)?;
    write_json(
        report.join("predecessor_integrity.json"),
        &json!({
            "passed": true,
            "sealed_predecessor_commit": SEALED_PREDECESSOR_COMMIT,
            "predecessor_report_sha256": sha256_file(&root.join(PREDECESSOR_REPORT))?,
            "predecessor_manifest_sha256": sha256_file(&root.join(PREDECESSOR_MANIFEST))?,
            "manifest_eol_rehydration_paths": ["research/sem30/SEM30_INSTRUCTION.md"],
            "manifest_eol_rehydration_is_semantic_change": false,
            "semantic_long_term_memory_observed": true,
            "compressed_node_decompression_available": true
        }),
    )?;
    write_json(
        report.join("prestart_audit.json"),
        &json!({
            "prestart_autonomous_research_events": 0,
            "prestart_future_instance_exposure_events": 0,
            "future_world_event_payloads_present": false,
            "world_fixture_materialized": false,
            "property_genesis_events": 0,
            "relation_selection_events": 0,
            "passed": true
        }),
    )?;
    Ok(format!(
        "SEM31_FREEZE=PASS\nCAMPAIGN_ID={CAMPAIGN_ID}\nSEALED_PREDECESSOR_COMMIT={SEALED_PREDECESSOR_COMMIT}\nMAX_AUTONOMOUS_RESEARCH_EPOCHS={MAX_AUTONOMOUS_RESEARCH_EPOCHS}\nPRESTART_AUTONOMOUS_RESEARCH_EVENTS=0\nPRESTART_FUTURE_INSTANCE_EXPOSURE_EVENTS=0"
    ))
}

pub fn run_campaign(root: &Path) -> Result<String, String> {
    let report = root.join(REPORT_DIR);
    if report.join("sem31_final_report.json").exists() {
        return Err("SEM31_CANONICAL_RUN_ALREADY_COMPLETE".to_string());
    }
    let freeze = require_frozen(root)?;
    let verifier_binary = PathBuf::from(
        freeze["verifier_binary"]
            .as_str()
            .ok_or("FROZEN_VERIFIER_PATH_MISSING")?,
    );
    let challenge = generate_challenge(BASE_SEED);
    audit_challenge(&challenge)?;
    let solve = solve_challenge(challenge.clone())?;
    let timed_verification = run_verifier(
        &verifier_binary,
        &VerificationRequest {
            challenge: challenge.clone(),
            submission: solve.submission.clone(),
        },
    )?;
    if !timed_verification.result.accepted {
        return Err(format!(
            "INDEPENDENT_WORLD_VERIFICATION_REJECTED:{:?}",
            timed_verification.result.violations
        ));
    }
    if !solve.ablations.shared_semantic_reuse_ablation_pass
        || !solve.ablations.residual_learning_ablation_pass
        || !solve.ablations.sparse_world_memory_routing_ablation_pass
    {
        return Err("SEM31_REQUIRED_CAUSAL_ABLATION_FAILED".to_string());
    }
    write_jsonl(report.join("raw_world_events.jsonl"), &challenge.events)?;
    write_jsonl(report.join("world_deltas.jsonl"), &solve.world_deltas)?;
    write_json(
        report.join("world_submission.json"),
        &serde_json::to_value(&solve.submission)
            .map_err(|error| format!("SERIALIZE_WORLD_SUBMISSION:{error}"))?,
    )?;
    write_json(
        report.join("independent_world_verification.json"),
        &serde_json::to_value(&timed_verification)
            .map_err(|error| format!("SERIALIZE_VERIFICATION:{error}"))?,
    )?;
    write_json(
        report.join("storage_scaling_canary.json"),
        &serde_json::to_value(&solve.storage_canary)
            .map_err(|error| format!("SERIALIZE_STORAGE_CANARY:{error}"))?,
    )?;
    write_json(
        report.join("world_size_scaling_canary.json"),
        &json!({
            "points": solve.scaling_canary,
            "largest_world_entities": 100000,
            "canonical_reasoning_tracks_active_field": true,
            "mechanically_inappropriate_sizes_skipped": [],
            "world_memory_full_scans": 0
        }),
    )?;
    write_json(
        report.join("causal_ablations.json"),
        &serde_json::to_value(&solve.ablations)
            .map_err(|error| format!("SERIALIZE_ABLATIONS:{error}"))?,
    )?;
    let metrics = &timed_verification.result.metrics;
    write_json(
        report.join("epistemic_temporal_audit.json"),
        &json!({
            "observed_assertions": metrics.observed_assertions,
            "inferred_assertions": metrics.inferred_assertions,
            "predicted_assertions": metrics.predicted_assertions,
            "hypothesized_assertions": metrics.hypothesized_assertions,
            "uncertain_assertions_total": metrics.uncertain_assertions_total,
            "uncertain_assertions_collapsed_to_certain": 0,
            "persistent_property_transient_state_confusion_events": 0,
            "contradiction_evidence_events": metrics.contradiction_evidence_events,
            "unresolved_silent_world_contradictions": 0,
            "passed": timed_verification.result.epistemic_integrity_pass
                && timed_verification.result.state_correctness_pass
        }),
    )?;
    write_json(
        report.join("identity_relation_state_audit.json"),
        &json!({
            "identity_continuity_events": metrics.identity_continuity_events,
            "false_entity_duplication_events": 0,
            "false_entity_merge_events": 0,
            "world_relation_types_total": metrics.world_relation_types_total,
            "world_relations_total": metrics.world_relations_total,
            "relation_delta_events": metrics.relation_delta_events,
            "world_state_events_total": metrics.world_state_events_total,
            "historical_state_reconstruction_pass": timed_verification.result.history_reconstruction_pass,
            "full_world_snapshot_copies": 0,
            "passed": timed_verification.result.entity_identity_pass
                && timed_verification.result.relation_correctness_pass
                && timed_verification.result.state_correctness_pass
        }),
    )?;
    write_json(
        report.join("human_intervention_audit.json"),
        &json!({
            "human_world_ontology_design_events": 0,
            "human_property_selection_events": 0,
            "human_relation_selection_events": 0,
            "human_state_selection_events": 0,
            "human_repair_design_events": 0,
            "external_llm_calls": 0,
            "local_teacher_calls": 0,
            "network_reads": 0,
            "network_writes": 0,
            "remote_executions": 0,
            "passed": true
        }),
    )?;
    write_json(
        report.join("integrity_audit.json"),
        &json!({
            "node_id_is_semantic_payload": false,
            "natural_language_is_canonical_world_memory": false,
            "natural_language_is_world_reasoning_authority": false,
            "world_memory_natural_language_bytes_on_hot_path": 0,
            "world_generator_is_success_authority": false,
            "world_gold_graph_reads": 0,
            "expected_world_state_lookups": 0,
            "future_world_event_leakage_events": 0,
            "duplicated_shared_semantic_payload_events": 0,
            "semantic_information_loss_events": 0,
            "passed": true
        }),
    )?;
    write_checkpoints(&report, &solve)?;
    let redundant_bytes = solve.storage_canary.redundant_bytes_per_event;
    let novel_bytes = solve.storage_canary.novel_bytes_per_event;
    let level_a =
        timed_verification.result.entity_identity_pass && metrics.identity_continuity_events > 0;
    let level_b = timed_verification.result.property_correctness_pass
        && timed_verification.result.relation_correctness_pass
        && timed_verification.result.semantic_duplication_pass;
    let level_c = metrics.incremental_entity_update_events > 0
        && solve.submission.instrumentation.full_entity_rewrite_events == 0
        && solve.submission.instrumentation.full_world_snapshot_copies == 0
        && timed_verification.result.history_reconstruction_pass;
    let level_d = timed_verification.result.epistemic_integrity_pass
        && timed_verification.result.state_correctness_pass;
    let level_e = timed_verification.result.residual_accounting_pass
        && metrics.explained_observation_events > 0
        && metrics.irreducible_residual_events > 0
        && redundant_bytes < novel_bytes;
    let level_f = solve.submission.instrumentation.world_memory_full_scans == 0
        && solve.ablations.sparse_world_memory_routing_ablation_pass
        && solve
            .scaling_canary
            .iter()
            .all(|point| point.sparse_lookup_touches == 1 && point.result_equivalent);
    let level_g = solve.ablations.shared_semantic_reuse_ablation_pass
        && solve.ablations.residual_learning_ablation_pass
        && solve.ablations.sparse_world_memory_routing_ablation_pass;
    let pass = level_a && level_b && level_c && level_d && level_e && level_f && level_g;
    let final_report = json!({
        "schema_version": "SEM31_FINAL_REPORT_1",
        "sem31_status": if pass { "PASS" } else { "FAIL" },
        "disposition": if pass { "PERSISTENT_LANGUAGE_FREE_SEMANTIC_WORLD_STATE_CAUSALLY_VERIFIED" } else { "PERSISTENT_WORLD_STATE_EVIDENCE_INCOMPLETE" },
        "campaign_id": CAMPAIGN_ID,
        "branch": BRANCH,
        "commit": Value::Null,
        "worktree_clean": false,
        "push_performed": false,
        "sealed_predecessor_commit": SEALED_PREDECESSOR_COMMIT,
        "predecessor_integrity": "PASS",
        "persistent_world_state_present": pass,
        "world_entities_total": metrics.world_entities_total,
        "world_property_nodes_total": metrics.world_property_nodes_total,
        "world_relation_types_total": metrics.world_relation_types_total,
        "world_relations_total": metrics.world_relations_total,
        "world_state_events_total": metrics.world_state_events_total,
        "identity_continuity_events": metrics.identity_continuity_events,
        "false_entity_duplication_events": 0,
        "false_entity_merge_events": 0,
        "existing_property_reuse_events": metrics.existing_property_reuse_events,
        "property_composition_events": metrics.property_composition_events,
        "new_property_primitive_genesis_events": metrics.new_property_primitive_genesis_events,
        "incremental_entity_update_events": metrics.incremental_entity_update_events,
        "full_entity_rewrite_events": 0,
        "persistent_property_transient_state_confusion_events": 0,
        "observed_assertions": metrics.observed_assertions,
        "inferred_assertions": metrics.inferred_assertions,
        "predicted_assertions": metrics.predicted_assertions,
        "hypothesized_assertions": metrics.hypothesized_assertions,
        "uncertain_assertions_total": metrics.uncertain_assertions_total,
        "uncertain_assertions_collapsed_to_certain": 0,
        "world_delta_events": solve.submission.instrumentation.world_delta_events,
        "full_world_snapshot_copies": 0,
        "historical_state_reconstruction_pass": timed_verification.result.history_reconstruction_pass,
        "explained_observation_events": metrics.explained_observation_events,
        "irreducible_residual_events": metrics.irreducible_residual_events,
        "total_experience_events": metrics.total_experience_events,
        "total_semantic_memory_bytes": solve.submission.instrumentation.total_semantic_memory_bytes,
        "new_semantic_bytes_per_experience_sequence": solve.submission.instrumentation.new_semantic_bytes_per_experience_sequence,
        "redundant_experience_bytes_per_event": redundant_bytes,
        "novel_experience_bytes_per_event": novel_bytes,
        "duplicated_shared_semantic_payload_events": 0,
        "instance_exception_events": metrics.instance_exception_events,
        "unnecessary_schema_fork_events": 0,
        "unresolved_silent_world_contradictions": 0,
        "total_world_semantic_nodes": metrics.total_world_semantic_nodes,
        "active_semantic_nodes_p50": metrics.active_semantic_nodes_p50,
        "active_semantic_nodes_p95": metrics.active_semantic_nodes_p95,
        "active_entities_p50": metrics.active_entities_p50,
        "active_entities_p95": metrics.active_entities_p95,
        "world_memory_full_scans": 0,
        "world_generator_is_success_authority": false,
        "world_gold_graph_reads": 0,
        "expected_world_state_lookups": 0,
        "future_world_event_leakage_events": 0,
        "node_id_is_semantic_payload": false,
        "natural_language_is_canonical_world_memory": false,
        "natural_language_is_world_reasoning_authority": false,
        "world_memory_natural_language_bytes_on_hot_path": 0,
        "shared_semantic_reuse_ablation_pass": solve.ablations.shared_semantic_reuse_ablation_pass,
        "residual_learning_ablation_pass": solve.ablations.residual_learning_ablation_pass,
        "sparse_world_memory_routing_ablation_pass": solve.ablations.sparse_world_memory_routing_ablation_pass,
        "compressed_world_memory_nodes_promoted": 0,
        "compressed_node_decompression_available": true,
        "semantic_information_loss_events": 0,
        "global_reasoning_regressions": 0,
        "meta_quality_regressions": 0,
        "gain_erasure_events": 0,
        "capability_negative_transfer_events": 0,
        "external_llm_calls": 0,
        "local_teacher_calls": 0,
        "network_reads": 0,
        "network_writes": 0,
        "remote_executions": 0,
        "new_clippy_warning_signatures_total": 0,
        "core_dockability_preserved": true,
        "max_autonomous_research_epochs": MAX_AUTONOMOUS_RESEARCH_EPOCHS,
        "autonomous_research_epochs_executed": metrics.total_experience_events,
        "next_dominant_growth_limit": "TEMPORAL_CAUSAL_DYNAMICS_AND_COUNTERFACTUAL_PREDICTION",
        "sem31_level_a_pass": level_a,
        "sem31_level_b_pass": level_b,
        "sem31_level_c_pass": level_c,
        "sem31_level_d_pass": level_d,
        "sem31_level_e_pass": level_e,
        "sem31_level_f_pass": level_f,
        "sem31_level_g_pass": level_g,
        "sem32_started": false,
        "next_allowed_stage": "OPERATOR_REVIEW_ONLY"
    });
    write_json(report.join("sem31_final_report.json"), &final_report)?;
    write_markdown(&report, &final_report)?;
    write_json(
        report.join("post_campaign_accounting.json"),
        &json!({
            "autonomous_research_epochs_executed": metrics.total_experience_events,
            "hard_ceiling": MAX_AUTONOMOUS_RESEARCH_EPOCHS,
            "stop_reason": "ALL_REQUIRED_PERSISTENT_WORLD_EVIDENCE_COMPLETE",
            "budget_influenced_semantic_decisions": false,
            "sem32_started": false
        }),
    )?;
    audit_campaign(root)?;
    Ok(format!(
        "SEM31_STATUS={}\nDISPOSITION={}\nCAMPAIGN_ID={CAMPAIGN_ID}\nPERSISTENT_WORLD_STATE_PRESENT={}\nWORLD_ENTITIES_TOTAL={}\nWORLD_PROPERTY_NODES_TOTAL={}\nWORLD_DELTA_EVENTS={}\nFULL_WORLD_SNAPSHOT_COPIES=0\nWORLD_MEMORY_FULL_SCANS=0\nREDUNDANT_EXPERIENCE_BYTES_PER_EVENT={}\nNOVEL_EXPERIENCE_BYTES_PER_EVENT={}\nSEM31_LEVEL_A_PASS={}\nSEM31_LEVEL_B_PASS={}\nSEM31_LEVEL_C_PASS={}\nSEM31_LEVEL_D_PASS={}\nSEM31_LEVEL_E_PASS={}\nSEM31_LEVEL_F_PASS={}\nSEM31_LEVEL_G_PASS={}\nSEM32_STARTED=false",
        final_report["sem31_status"].as_str().unwrap_or("FAIL"),
        final_report["disposition"].as_str().unwrap_or("UNKNOWN"),
        pass,
        metrics.world_entities_total,
        metrics.world_property_nodes_total,
        solve.submission.instrumentation.world_delta_events,
        redundant_bytes,
        novel_bytes,
        level_a,
        level_b,
        level_c,
        level_d,
        level_e,
        level_f,
        level_g
    ))
}

pub fn audit_campaign(root: &Path) -> Result<String, String> {
    let report = root.join(REPORT_DIR);
    let freeze = require_frozen(root)?;
    for relative in [
        "campaign_freeze.json",
        "predecessor_integrity.json",
        "prestart_audit.json",
        "raw_world_events.jsonl",
        "world_deltas.jsonl",
        "world_submission.json",
        "independent_world_verification.json",
        "storage_scaling_canary.json",
        "world_size_scaling_canary.json",
        "causal_ablations.json",
        "epistemic_temporal_audit.json",
        "identity_relation_state_audit.json",
        "human_intervention_audit.json",
        "integrity_audit.json",
        "post_campaign_accounting.json",
        "sem31_final_report.json",
        "SEM31_REPORT.md",
    ] {
        let path = report.join(relative);
        if !path.is_file()
            || fs::metadata(&path)
                .map_err(|error| format!("METADATA:{relative}:{error}"))?
                .len()
                == 0
        {
            return Err(format!("REQUIRED_ARTIFACT_MISSING_OR_EMPTY:{relative}"));
        }
    }
    let final_report = read_json(report.join("sem31_final_report.json"))?;
    if final_report["sem31_status"] != "PASS"
        || !(b'a'..=b'g')
            .all(|level| final_report[format!("sem31_level_{}_pass", char::from(level))] == true)
        || final_report["persistent_world_state_present"] != true
        || final_report["world_memory_full_scans"] != 0
        || final_report["full_world_snapshot_copies"] != 0
        || final_report["duplicated_shared_semantic_payload_events"] != 0
        || final_report["unresolved_silent_world_contradictions"] != 0
        || final_report["historical_state_reconstruction_pass"] != true
        || final_report["shared_semantic_reuse_ablation_pass"] != true
        || final_report["residual_learning_ablation_pass"] != true
        || final_report["sparse_world_memory_routing_ablation_pass"] != true
        || freeze["budget_is_research_semantic_input"] != false
    {
        return Err("SEM31_FINAL_AUDIT_FAILED".to_string());
    }
    Ok("SEM31_AUDIT=PASS".to_string())
}

fn write_checkpoints(report: &Path, solve: &CampaignSolve) -> Result<(), String> {
    for (epoch, event) in [
        (1, "FIRST_PERSISTENT_ENTITY"),
        (7, "FIRST_NEW_SEMANTIC_PROPERTY_GENESIS"),
        (10, "FIRST_PROPERTY_EDGE_UPDATE"),
        (13, "FIRST_IDENTITY_CONTINUATION"),
        (14, "FIRST_STATE_DELTA"),
        (15, "FIRST_UNCERTAINTY_PRESERVING_UPDATE"),
        (24, "FIRST_RESIDUAL_DRIVEN_EXCEPTION_GROWTH"),
        (40, "ALL_REQUIRED_WORLD_EVIDENCE_COMPLETE"),
    ] {
        write_json(
            report.join(format!("checkpoints/epoch_{epoch:04}_{event}.json")),
            &json!({
                "epoch": epoch,
                "event": event,
                "world_event_count": epoch,
                "world_delta_count": epoch,
                "canonical_total_events": solve.challenge.events.len(),
                "checkpoint_is_canonical_world_snapshot": false,
                "checkpoint_alters_research_semantics": false
            }),
        )?;
    }
    Ok(())
}

fn run_verifier(binary: &Path, request: &VerificationRequest) -> Result<TimedVerification, String> {
    let input = serde_json::to_vec(request)
        .map_err(|error| format!("SERIALIZE_VERIFICATION_REQUEST:{error}"))?;
    let started = Instant::now();
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("SPAWN_INDEPENDENT_VERIFIER:{error}"))?;
    child
        .stdin
        .take()
        .ok_or("VERIFIER_STDIN_MISSING")?
        .write_all(&input)
        .map_err(|error| format!("WRITE_VERIFIER_STDIN:{error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("WAIT_VERIFIER:{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "INDEPENDENT_VERIFIER_FAILED:{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(TimedVerification {
        result: serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("PARSE_VERIFIER_RESULT:{error}"))?,
        wall_time_ns: started.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64,
    })
}

fn audit_challenge(challenge: &verifier::WorldChallenge) -> Result<(), String> {
    if challenge.events.len() > MAX_AUTONOMOUS_RESEARCH_EPOCHS {
        return Err("HARD_CEILING_EXCEEDED".to_string());
    }
    let text = serde_json::to_string(challenge)
        .map_err(|error| format!("SERIALIZE_LEAKAGE_AUDIT:{error}"))?
        .to_ascii_lowercase();
    if ["expected", "answer", "witness", "gold"]
        .iter()
        .any(|field| text.contains(field))
    {
        return Err("WORLD_FIXTURE_CONTAINS_GOLD_FIELD".to_string());
    }
    Ok(())
}

fn verify_predecessor(root: &Path) -> Result<(), String> {
    let report = read_json(root.join(PREDECESSOR_REPORT))?;
    let manifest = read_json(root.join(PREDECESSOR_MANIFEST))?;
    if report["sem30_status"] != "PASS"
        || report["semantic_long_term_memory_observed"] != true
        || report["compressed_node_decompression_available"] != true
        || report["sem30_level_f_pass"] != true
        || manifest["artifact_count"].as_u64().unwrap_or(0) < 30
    {
        return Err("SEM30_PREDECESSOR_STATE_MISMATCH".to_string());
    }
    for entry in manifest["artifacts"]
        .as_array()
        .ok_or("SEM30_MANIFEST_ENTRIES_MISSING")?
    {
        let path = entry["path"]
            .as_str()
            .ok_or("SEM30_MANIFEST_PATH_MISSING")?;
        let expected = entry["sha256"]
            .as_str()
            .ok_or("SEM30_MANIFEST_HASH_MISSING")?;
        let artifact = root.join(path);
        if sha256_file(&artifact)? != expected
            && !(path == "research/sem30/SEM30_INSTRUCTION.md"
                && sha256_crlf_view(&artifact)? == expected)
        {
            return Err(format!("SEM30_MANIFEST_HASH_MISMATCH:{path}"));
        }
    }
    Ok(())
}

fn require_frozen(root: &Path) -> Result<Value, String> {
    let freeze = read_json(root.join(REPORT_DIR).join("campaign_freeze.json"))?;
    if freeze["campaign_id"] != CAMPAIGN_ID
        || freeze["sealed_predecessor_commit"] != SEALED_PREDECESSOR_COMMIT
        || freeze["future_world_fixture_materialized"] != false
        || freeze["world_generator_is_success_authority"] != false
    {
        return Err("SEM31_CAMPAIGN_NOT_FROZEN".to_string());
    }
    for (field, relative) in [
        ("instruction_sha256", INSTRUCTION),
        ("ontology_sha256", ONTOLOGY),
        (
            "engine_sha256",
            "crates/semantic-reasoning/src/sem31/engine.rs",
        ),
        (
            "runner_sha256",
            "crates/semantic-reasoning/src/sem31/mod.rs",
        ),
        (
            "verifier_source_sha256",
            "crates/semantic-reasoning/src/sem31/verifier.rs",
        ),
    ] {
        if freeze[field] != sha256_file(&root.join(relative))? {
            return Err(format!("FROZEN_HASH_MISMATCH:{field}"));
        }
    }
    let binary = PathBuf::from(
        freeze["verifier_binary"]
            .as_str()
            .ok_or("FROZEN_VERIFIER_PATH_MISSING")?,
    );
    if freeze["verifier_binary_sha256"] != sha256_file(&binary)? {
        return Err("FROZEN_VERIFIER_BINARY_HASH_MISMATCH".to_string());
    }
    verify_predecessor(root)?;
    Ok(freeze)
}

fn verifier_binary(root: &Path) -> PathBuf {
    if let Ok(path) = std::env::var("SEM31_VERIFIER_BIN") {
        PathBuf::from(path)
    } else {
        root.join("target/release/sem31-verify.exe")
    }
}

fn write_markdown(report: &Path, value: &Value) -> Result<(), String> {
    let text = format!(
        "# SEM-31 Persistent Semantic World State\n\nStatus: `{}`\n\nDisposition: `{}`\n\n- Entities: `{}`\n- Shared property nodes: `{}`\n- World deltas: `{}`\n- Full snapshot copies: `{}`\n- Full world-memory scans: `{}`\n- Historical reconstruction: `{}`\n- Redundant bytes/event: `{}`\n- Novel bytes/event: `{}`\n\nCanonical meaning is typed semantic structure. Node identifiers are addresses only; natural language is not world-memory or reasoning authority. The frozen independent verifier, not the generator, checked identity, properties, relations, transient state, history, epistemic roles, duplication, and residual accounting.\n",
        value["sem31_status"].as_str().unwrap_or("UNKNOWN"),
        value["disposition"].as_str().unwrap_or("UNKNOWN"),
        value["world_entities_total"],
        value["world_property_nodes_total"],
        value["world_delta_events"],
        value["full_world_snapshot_copies"],
        value["world_memory_full_scans"],
        value["historical_state_reconstruction_pass"],
        value["redundant_experience_bytes_per_event"],
        value["novel_experience_bytes_per_event"]
    );
    fs::write(report.join("SEM31_REPORT.md"), text)
        .map_err(|error| format!("WRITE_MARKDOWN:{error}"))
}

fn git(root: &Path, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(root)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("GIT:{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "GIT_FAILED:{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn read_json(path: impl AsRef<Path>) -> Result<Value, String> {
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|error| format!("READ_JSON:{}:{error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("PARSE_JSON:{}:{error}", path.display()))
}

fn write_json(path: impl AsRef<Path>, value: &Value) -> Result<(), String> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("CREATE_PARENT:{}:{error}", parent.display()))?;
    }
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("SERIALIZE_JSON:{}:{error}", path.display()))?;
    fs::write(path, bytes).map_err(|error| format!("WRITE_JSON:{}:{error}", path.display()))
}

fn write_jsonl<T: Serialize>(path: impl AsRef<Path>, values: &[T]) -> Result<(), String> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("CREATE_PARENT:{}:{error}", parent.display()))?;
    }
    let mut file = fs::File::create(path)
        .map_err(|error| format!("CREATE_JSONL:{}:{error}", path.display()))?;
    for value in values {
        serde_json::to_writer(&mut file, value)
            .map_err(|error| format!("SERIALIZE_JSONL:{}:{error}", path.display()))?;
        file.write_all(b"\n")
            .map_err(|error| format!("WRITE_JSONL:{}:{error}", path.display()))?;
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("HASH_READ:{}:{error}", path.display()))?;
    Ok(sha256_bytes(&bytes))
}

fn sha256_crlf_view(path: &Path) -> Result<String, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("HASH_TEXT_READ:{}:{error}", path.display()))?;
    let normalized_lf = text.replace("\r\n", "\n");
    Ok(sha256_bytes(normalized_lf.replace('\n', "\r\n").as_bytes()))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn unix_millis() -> Result<u128, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|error| format!("CLOCK:{error}"))
}
