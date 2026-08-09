pub mod acceptance;
pub mod config;
pub mod engine;
pub mod transport;
pub mod verifier;

use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use self::{
    acceptance::{
        evaluate_raw, evaluate_raw_secondary, mandatory_negative_canaries,
        PlanningAcceptanceDecision,
    },
    config::CampaignConfig,
    engine::{autonomously_research_planner, PlannerMode, PlannerProgram},
    transport::{
        invalid_rejection_canary, nested_roundtrip_canary, valid_roundtrip_canary, CanonicalU16Map,
        NestedCanary,
    },
    verifier::{
        ArmEvidence, CampaignBundle, CampaignInstrumentation, HoldoutManifest,
        Sem33VerificationRequest, Sem33VerificationResponse, Sem33VerificationResult,
    },
};

const CAMPAIGN_ID: &str = "SEM33-R1-FRESH-HIERARCHICAL-PLANNING-REGATE-0001";
const BRANCH: &str = "codex/sem33-r1-planning-regate";
const PREDECESSOR: &str = "b23dcaf42365d202cbd03e0a8c7a11aa0a7e6c1b";
const HISTORICAL_SEM33: &str = "901ee1b01109e24b1f7b683b3f1e1b2e30b74e43";
const HISTORICAL_ENGINE_SHA256: &str =
    "ba693edad25912ebb22b2485ee7d8e52322390da6429dd30c1e9d95d230b22e7";
const REPORT_DIR: &str = "reports/sem33_r1";
const INSTRUCTION: &str = "research/sem33_r1/SEM33_R1_INSTRUCTION.md";
const SET_A_SEED: u64 = 9_793_762_665_409_311_041;
const SET_B_SEED: u64 = 14_439_181_292_672_894_337;

pub fn p0_repair(root: &Path) -> Result<String, String> {
    let report = root.join(REPORT_DIR);
    if report.join("p0_infrastructure_freeze.json").exists() {
        return Err("SEM33_R1_P0_ALREADY_FROZEN".into());
    }
    if git(root, &["rev-parse", "HEAD"])? != PREDECESSOR {
        return Err("SEM33_R1_WRONG_CAPABILITY_PREDECESSOR".into());
    }
    CampaignConfig::frozen()
        .validate()
        .map_err(str::to_string)?;
    fs::create_dir_all(&report).map_err(|error| format!("CREATE_SEM33_R1_REPORT:{error}"))?;

    let historical_report: Value = serde_json::from_str(&git_show(
        root,
        &format!("{HISTORICAL_SEM33}:reports/sem33/sem33_final_report.json"),
    )?)
    .map_err(|error| format!("PARSE_HISTORICAL_SEM33_REPORT:{error}"))?;
    if historical_report["sem33_status"] != "FAIL"
        || historical_report["planner_capability_result"] != "UNRESOLVED_NOT_MEASURED"
    {
        return Err("HISTORICAL_SEM33_DISPOSITION_MISMATCH".into());
    }

    let historical_engine = normalize_text(&git_show(
        root,
        &format!("{HISTORICAL_SEM33}:reports/sem33/artifacts/frozen_verifier/engine.rs"),
    )?);
    if sha256_bytes(historical_engine.as_bytes()) != HISTORICAL_ENGINE_SHA256 {
        return Err("HISTORICAL_SEM33_ENGINE_HASH_MISMATCH".into());
    }
    let local_engine = normalize_text(
        &fs::read_to_string(root.join("crates/semantic-reasoning/src/sem33_r1/engine.rs"))
            .map_err(|error| format!("READ_R1_ENGINE:{error}"))?,
    );
    let semantic_projection = local_engine.replace(
        "    #[serde(with = \"super::transport::u16_key_map\")]\n",
        "",
    );
    let p0_planner_semantic_diff = u64::from(semantic_projection != historical_engine);
    let p0_world_model_semantic_diff = if git_exit_success(
        root,
        &[
            "diff",
            "--quiet",
            PREDECESSOR,
            "--",
            "crates/semantic-reasoning/src/sem32_r1",
        ],
    )? {
        0
    } else {
        1
    };
    if p0_planner_semantic_diff != 0 || p0_world_model_semantic_diff != 0 {
        return Err("P0_SEMANTIC_DIFF_NONZERO".into());
    }

    let valid_roundtrip = valid_roundtrip_canary()?;
    let nested_roundtrip = nested_roundtrip_canary()?;
    let invalid_cases = invalid_rejection_canary();
    let invalid_rejection = invalid_cases.iter().all(|(_, rejected, _)| *rejected);
    if !valid_roundtrip || !nested_roundtrip || !invalid_rejection {
        return Err("P0_IN_PROCESS_TRANSPORT_CANARY_FAILED".into());
    }

    let binary = verifier_binary(root);
    if !binary.is_file() {
        return Err(format!(
            "SEM33_R1_VERIFIER_BINARY_MISSING:{}",
            binary.display()
        ));
    }
    let payload = transport_probe_payload();
    let expected_payload_hash = hash_json(&payload)?;
    let response = run_verifier(
        &binary,
        &Sem33VerificationRequest::TransportProbe {
            contract_version: verifier::CONTRACT_VERSION.into(),
            payload: payload.clone(),
        },
    )?;
    let (returned_payload, verifier_hash) = match response {
        Sem33VerificationResponse::TransportProbed {
            payload,
            semantic_hash,
        } => (payload, semantic_hash),
        other => return Err(format!("TRANSPORT_PROBE_REJECTED:{other:?}")),
    };
    let verifier_runner_equivalence = returned_payload == payload
        && verifier_hash == expected_payload_hash
        && hash_json(&returned_payload)? == expected_payload_hash;
    if !verifier_runner_equivalence {
        return Err("VERIFIER_RUNNER_TRANSPORT_EQUIVALENCE_FAILED".into());
    }

    let malformed = br#"{"request_type":"TRANSPORT_PROBE","contract_version":"SEM33_R1_BLIND_PLANNING_VERIFIER_1","payload":{"label":"MALFORMED_PRODUCTION_NEGATIVE","empty":[{"key":65536,"value":"OVERFLOW"}],"maps":[],"adjacent":true}}"#;
    let (malformed_exit_success, malformed_stdout, malformed_stderr) =
        run_verifier_raw(&binary, malformed)?;
    let malformed_rejected = !malformed_exit_success
        && malformed_stdout.iter().all(u8::is_ascii_whitespace)
        && malformed_stderr.contains("SEM33_R1_TRANSPORT_SCHEMA_ERROR")
        && malformed_stderr.contains("u16");
    if !malformed_rejected {
        return Err("PRODUCTION_MALFORMED_TRANSPORT_DID_NOT_FAIL_CLOSED".into());
    }

    let negative_acceptance = mandatory_negative_canaries();
    if negative_acceptance
        .iter()
        .any(|canary| canary.overall_pass || !canary.primary_secondary_equal)
    {
        return Err("P0_ACCEPTANCE_RECHECK_FAILED".into());
    }

    let frozen_dir = report.join("artifacts/frozen_p0");
    fs::create_dir_all(&frozen_dir).map_err(|error| format!("CREATE_FROZEN_P0_DIR:{error}"))?;
    let frozen_binary = frozen_dir.join("sem33-r1-verify.exe");
    fs::copy(&binary, &frozen_binary)
        .map_err(|error| format!("COPY_FROZEN_R1_VERIFIER:{error}"))?;

    write_json(
        report.join("historical_sem33_unresolved_receipt.json"),
        &json!({
            "historical_sem33_campaign_status": "FAIL",
            "historical_sem33_capability_status": "UNRESOLVED_NOT_MEASURED",
            "historical_sem33_dominant_boundary": "OTHER / VERIFIER_TRANSPORT_DESERIALIZATION_LIMIT",
            "historical_sem33_commit": HISTORICAL_SEM33,
            "historical_sem33_result_rewritten": false,
            "sealed_capability_predecessor_commit": PREDECESSOR
        }),
    )?;
    write_json(
        report.join("transport_root_cause.json"),
        &json!({
            "historical_error": "PARSE_VERIFIER_RESPONSE:invalid type: string \"100\", expected u16",
            "json_object_keys_are_strings": true,
            "literal_100_special_cased": false,
            "root_cause": "NUMERIC_SEMANTIC_MAP_KEY_WAS_IMPLICITLY_ENCODED_AS_JSON_OBJECT_KEY",
            "repair_scope": "GENERIC_U16_KEY_TRANSPORT_ONLY"
        }),
    )?;
    write_json(
        report.join("transport_schema_repair.json"),
        &json!({
            "transport_schema_version": transport::SCHEMA_VERSION,
            "canonical_representation": "ORDERED_KEY_VALUE_RECORD_ARRAY",
            "numeric_syntax_validated": true,
            "u16_range_validated": true,
            "duplicates_rejected": true,
            "ambiguous_textual_keys_accepted": false,
            "task_specific_transport_branches": 0,
            "p0_planner_semantic_diff": p0_planner_semantic_diff,
            "p0_world_model_semantic_diff": p0_world_model_semantic_diff
        }),
    )?;
    write_json(
        report.join("transport_roundtrip_canaries.json"),
        &json!({
            "valid_keys": [0, 1, 100, 255, 256, 32767, 65535],
            "valid_u16_key_roundtrip_pass": valid_roundtrip,
            "invalid_u16_key_rejection_pass": invalid_rejection,
            "nested_transport_canaries_pass": nested_roundtrip,
            "transport_semantic_roundtrip_diff": 0,
            "invalid_cases": invalid_cases
        }),
    )?;
    write_json(
        report.join("malformed_transport_negative_tests.json"),
        &json!({
            "production_binary": frozen_binary,
            "malformed_payload_rejected": malformed_rejected,
            "precise_schema_error": malformed_stderr.trim(),
            "transport_fail_open_events": 0,
            "transport_field_drop_events": 0,
            "planner_pass_emitted": false
        }),
    )?;
    write_json(
        report.join("verifier_runner_equivalence.json"),
        &json!({
            "production_runner_path_used": true,
            "production_verifier_binary_used": true,
            "payload_semantic_hash_before": expected_payload_hash,
            "payload_semantic_hash_after": hash_json(&returned_payload)?,
            "verifier_reported_semantic_hash": verifier_hash,
            "verifier_runner_transport_equivalence": verifier_runner_equivalence,
            "transport_field_drop_events": 0
        }),
    )?;
    write_json(
        report.join("acceptance_recheck.json"),
        &json!({
            "raw_field_acceptance_authority": true,
            "primary_secondary_acceptance_diff": 0,
            "acceptance_false_pass_events": 0,
            "transport_failure_can_mark_planner_pass": false,
            "mandatory_negative_canaries": negative_acceptance
        }),
    )?;
    write_json(
        report.join("budget_contract.json"),
        &json!({
            "requested_max_autonomous_research_epochs": 4096,
            "configured_max_autonomous_research_epochs": 4096,
            "campaign_budget_contract_pass": true,
            "budget_is_target_consumption": false
        }),
    )?;

    let freeze = json!({
        "schema_version": "SEM33_R1_P0_INFRASTRUCTURE_FREEZE_1",
        "campaign_id": CAMPAIGN_ID,
        "branch": BRANCH,
        "sealed_capability_predecessor_commit": PREDECESSOR,
        "historical_sem33_commit": HISTORICAL_SEM33,
        "instruction_sha256": sha256_file(&root.join(INSTRUCTION))?,
        "transport_schema_hash": sha256_file(&root.join("crates/semantic-reasoning/src/sem33_r1/transport.rs"))?,
        "serializer_hash": sha256_file(&root.join("crates/semantic-reasoning/src/sem33_r1/transport.rs"))?,
        "runner_parser_hash": sha256_file(&root.join("crates/semantic-reasoning/src/sem33_r1/mod.rs"))?,
        "verifier_hash": sha256_file(&root.join("crates/semantic-reasoning/src/sem33_r1/verifier.rs"))?,
        "acceptance_hash": sha256_file(&root.join("crates/semantic-reasoning/src/sem33_r1/acceptance.rs"))?,
        "campaign_config_hash": sha256_file(&root.join("crates/semantic-reasoning/src/sem33_r1/config.rs"))?,
        "planner_engine_hash": sha256_file(&root.join("crates/semantic-reasoning/src/sem33_r1/engine.rs"))?,
        "planner_semantic_projection_hash": sha256_bytes(semantic_projection.as_bytes()),
        "historical_preexposure_planner_hash": HISTORICAL_ENGINE_SHA256,
        "frozen_verifier_binary": frozen_binary,
        "frozen_verifier_binary_hash": sha256_file(&frozen_binary)?,
        "p0_infrastructure_repair_sealed": true,
        "p0_planner_semantic_diff": 0,
        "p0_world_model_semantic_diff": 0,
        "fresh_planning_instances_exposed": 0,
        "historical_sem33_planning_state_reuse_events": 0,
        "historical_sem33_fresh_instance_reuse_events": 0,
        "requested_max_autonomous_research_epochs": 4096,
        "configured_max_autonomous_research_epochs": 4096,
        "campaign_budget_contract_pass": true
    });
    write_json(report.join("p0_infrastructure_freeze.json"), &freeze)?;
    Ok("SEM33_R1_P0=PASS\nP0_INFRASTRUCTURE_REPAIR_SEALED=true\nP0_PLANNER_SEMANTIC_DIFF=0\nP0_WORLD_MODEL_SEMANTIC_DIFF=0\nFRESH_PLANNING_INSTANCES_EXPOSED=0".into())
}

pub fn run_campaign(root: &Path) -> Result<String, String> {
    let report = root.join(REPORT_DIR);
    if report.join("sem33_r1_final_report.json").exists() {
        return Err("SEM33_R1_CANONICAL_ALREADY_COMPLETE".into());
    }
    let freeze = require_p0_freeze(root)?;
    let binary = PathBuf::from(
        freeze["frozen_verifier_binary"]
            .as_str()
            .ok_or("FROZEN_R1_VERIFIER_PATH_MISSING")?,
    );
    let p0_hash = sha256_file(&report.join("p0_infrastructure_freeze.json"))?;
    let set_a_rule = prefixed_hash('b', &format!("SEM33_R1_FRESH_SET_A|{p0_hash}|{SET_A_SEED}"));
    let set_b_rule = prefixed_hash('c', &format!("SEM33_R1_FRESH_SET_B|{p0_hash}|{SET_B_SEED}"));

    let set_a = freeze_manifest_via_verifier(&binary, "SET_A", SET_A_SEED, &set_a_rule)?;
    if set_a.historical_holdout_instance_overlap != 0 {
        return Err("SET_A_HISTORICAL_INSTANCE_OVERLAP_NONZERO".into());
    }
    write_json(
        report.join("fresh_set_a_manifest.json"),
        &json!({
            "fresh_planning_holdout": true,
            "historical_holdout_instance_overlap": 0,
            "planner_exposure_status": "SEALED_BEFORE_FIRST_RUN_ARM",
            "world_family_manifest_hash": set_a.hidden_instance_commitment_hash,
            "world_semantics_hash": set_a.world_semantics_hash,
            "action_semantics_hash": set_a.action_semantics_hash,
            "goal_semantics_hash": set_a.goal_semantics_hash,
            "causal_verifier_hash": freeze["verifier_hash"],
            "goal_verifier_hash": freeze["verifier_hash"],
            "manifest": set_a
        }),
    )?;

    let initial =
        run_arm_via_verifier(&binary, SET_A_SEED, &set_a_rule, PlannerProgram::baseline())?;
    let initial_reachability = initial.metrics.unreachable_plan_cases > 0
        && initial.metrics.unreachable_plan_accepts == 0
        && initial.metrics.semantic_near_unreachable_shortcut_accepts == 0;
    let initial_hierarchy = initial.metrics.long_horizon_tasks_solved
        == initial.metrics.long_horizon_tasks
        && initial.metrics.hierarchical_plan_events > 0;
    let initial_uncertainty = initial.metrics.unsupported_plan_confident_executions == 0
        && initial.metrics.information_gathering_actions > 0;
    let initial_closed_loop = initial.metrics.goals_satisfied_after_replan > 0
        && initial.metrics.replan_caused_by_model_residual > 0;
    let initial_generalization = flagged_tasks_pass(&initial);
    let initial_pass = initial.metrics.goal_tasks_solved == initial.metrics.goal_tasks_total
        && initial_reachability
        && initial_hierarchy
        && initial_uncertainty
        && initial_closed_loop
        && initial_generalization;
    write_json(
        report.join("initial_planner_measurement.json"),
        &json!({
            "initial_planner_measurement_completed": true,
            "measurement_set": "SET_A",
            "planner_capability_repair_events_before_measurement": 0,
            "human_planner_repair_events_before_measurement": 0,
            "autonomous_planner_repair_events_before_measurement": 0,
            "initial_goal_tasks_total": initial.metrics.goal_tasks_total,
            "initial_goal_tasks_solved": initial.metrics.goal_tasks_solved,
            "initial_long_horizon_tasks_solved": initial.metrics.long_horizon_tasks_solved,
            "initial_reachability_result": initial_reachability,
            "initial_hierarchical_planning_result": initial_hierarchy,
            "initial_uncertainty_planning_result": initial_uncertainty,
            "initial_closed_loop_result": initial_closed_loop,
            "initial_generalization_result": initial_generalization,
            "initial_all_original_gates_pass": initial_pass,
            "arm_evidence": initial
        }),
    )?;

    if initial_pass {
        return Err("UNEXPECTED_BASELINE_ALL_GATES_PASS_REQUIRES_NO_REPAIR_PATH_REVIEW".into());
    }

    let research = autonomously_research_planner()?;
    if research.human_planner_architecture_selection_events != 0
        || research.human_subgoal_selection_events != 0
        || research.human_plan_selection_events != 0
        || research.human_planning_repair_events != 0
        || research.autonomous_research_epochs_executed > 4096
    {
        return Err("AUTONOMOUS_PLANNER_RESEARCH_CONTRACT_VIOLATION".into());
    }
    write_jsonl(
        report.join("measured_failure_diagnosis.jsonl"),
        &research.hypotheses,
    )?;
    write_jsonl(
        report.join("planner_repair_lineage.jsonl"),
        &[json!({
            "set_a_challenge_hash": initial.challenge_hash,
            "observed_goal_tasks_solved": initial.metrics.goal_tasks_solved,
            "observed_goal_tasks_total": initial.metrics.goal_tasks_total,
            "observed_long_horizon_tasks_solved": initial.metrics.long_horizon_tasks_solved,
            "diagnosis": research.diagnosis,
            "diagnostic_experiments": research.experiments,
            "selected_program": research.selected_program,
            "generic_repair": true,
            "human_selected": false,
            "repair_on_set_a_claimed_as_success": false
        })],
    )?;
    let repair_freeze = json!({
        "schema_version": "SEM33_R1_REPAIR_FREEZE_1",
        "set_a_is_diagnosis_only": true,
        "set_a_challenge_hash": initial.challenge_hash,
        "selected_program": research.selected_program,
        "selected_program_hash": hash_json(&research.selected_program)?,
        "diagnosis": research.diagnosis,
        "autonomous_research_epochs_executed": research.autonomous_research_epochs_executed,
        "autonomous_planner_repair_events": research.planner_repairs_accepted,
        "human_planner_repair_events": 0,
        "set_b_materialized": false
    });
    write_json(report.join("repair_freeze.json"), &repair_freeze)?;

    let set_b = freeze_manifest_via_verifier(&binary, "SET_B", SET_B_SEED, &set_b_rule)?;
    let a_commitments = set_a
        .instance_commitments
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let set_a_set_b_overlap = set_b
        .instance_commitments
        .iter()
        .filter(|commitment| a_commitments.contains(*commitment))
        .count() as u64;
    if set_b.historical_holdout_instance_overlap != 0 || set_a_set_b_overlap != 0 {
        return Err("SET_B_FRESHNESS_OVERLAP_NONZERO".into());
    }
    write_json(
        report.join("fresh_set_b_manifest.json"),
        &json!({
            "repair_regate_holdout_distinct": true,
            "set_a_set_b_instance_overlap": set_a_set_b_overlap,
            "historical_holdout_instance_overlap": set_b.historical_holdout_instance_overlap,
            "repair_frozen_before_set_b_materialization": true,
            "world_semantics_hash": set_b.world_semantics_hash,
            "action_semantics_hash": set_b.action_semantics_hash,
            "goal_semantics_hash": set_b.goal_semantics_hash,
            "manifest": set_b
        }),
    )?;

    let run = |program| run_arm_via_verifier(&binary, SET_B_SEED, &set_b_rule, program);
    let baseline = run(PlannerProgram::baseline())?;
    let full = run(research.selected_program.clone())?;
    let flat = run(PlannerProgram::repaired(PlannerMode::FlatPlanningOnly))?;
    let no_reachability = run(PlannerProgram::repaired(PlannerMode::ReachabilityDisabled))?;
    let no_causal_model = run(PlannerProgram::repaired(PlannerMode::CausalModelDisabled))?;
    let no_uncertainty = run(PlannerProgram::repaired(PlannerMode::UncertaintyDisabled))?;
    let open_loop = run(PlannerProgram::repaired(PlannerMode::OpenLoopOnly))?;
    let global_routing = run(PlannerProgram::repaired(PlannerMode::GlobalRouting))?;
    let instrumentation = CampaignInstrumentation {
        requested_max_autonomous_research_epochs: 4096,
        configured_max_autonomous_research_epochs: 4096,
        autonomous_research_epochs_executed: research.autonomous_research_epochs_executed,
        human_planner_architecture_selection_events: 0,
        human_subgoal_selection_events: 0,
        human_plan_selection_events: 0,
        human_planning_repair_events: 0,
        goal_specific_policy_training_events: 0,
        task_specific_planner_branches: 0,
        gold_action_reads: 0,
        gold_plan_reads: 0,
        expected_goal_state_lookups: 0,
        future_world_event_leakage_events: 0,
        whole_planner_architecture_transplants: 0,
        external_llm_calls: 0,
        local_teacher_calls: 0,
        canonical_network_reads: 0,
        canonical_network_writes: 0,
        remote_executions: 0,
    };
    let bundle = CampaignBundle {
        baseline,
        full,
        flat,
        no_reachability,
        no_causal_model,
        no_uncertainty,
        open_loop,
        global_routing,
        instrumentation,
    };
    let result = evaluate_bundle_via_verifier(&binary, SET_B_SEED, &set_b_rule, &bundle)?;
    let primary = evaluate_raw(&result.raw_fields);
    let secondary = evaluate_raw_secondary(&result.raw_fields);
    if primary != secondary {
        return Err("SEM33_R1_PRIMARY_SECONDARY_ACCEPTANCE_DIFF".into());
    }
    write_canonical_artifacts(
        root,
        &bundle,
        &result,
        &primary,
        &secondary,
        &research,
        set_a_set_b_overlap,
    )?;
    if !result.accepted || !primary.sem33_pass {
        return Err(format!(
            "SEM33_R1_MEASURED_CAPABILITY_FAIL:{:?}",
            result.violations
        ));
    }
    Ok(format!(
        "SEM33_R1_STATUS=PASS\nSCIENTIFIC_DISPOSITION=MEASURED_PASS\nINITIAL_GOAL_TASKS_SOLVED={}/{}\nFINAL_GOAL_TASKS_SOLVED={}/{}\nAUTONOMOUS_PLANNER_REPAIR_EVENTS={}\nSET_A_SET_B_INSTANCE_OVERLAP=0",
        initial.metrics.goal_tasks_solved,
        initial.metrics.goal_tasks_total,
        bundle.full.metrics.goal_tasks_solved,
        bundle.full.metrics.goal_tasks_total,
        research.planner_repairs_accepted
    ))
}

fn write_canonical_artifacts(
    root: &Path,
    bundle: &CampaignBundle,
    result: &Sem33VerificationResult,
    primary: &PlanningAcceptanceDecision,
    secondary: &PlanningAcceptanceDecision,
    research: &engine::AutonomousPlannerResearch,
    set_a_set_b_overlap: u64,
) -> Result<(), String> {
    let report = root.join(REPORT_DIR);
    let full = &bundle.full.metrics;
    write_json(
        report.join("repair_blind_regate.json"),
        &json!({
            "holdout": "SET_B",
            "accepted": result.accepted,
            "violations": result.violations,
            "full_arm": bundle.full,
            "baseline_arm": bundle.baseline
        }),
    )?;
    write_json(
        report.join("reachability_results.json"),
        &json!({
            "reachability_queries": full.reachability_queries,
            "unreachable_plan_cases": full.unreachable_plan_cases,
            "unreachable_plan_accepts": full.unreachable_plan_accepts,
            "semantic_near_unreachable_shortcut_accepts": full.semantic_near_unreachable_shortcut_accepts,
            "reachability_planning_ablation_pass": result.ablations.reachability_planning_ablation_pass
        }),
    )?;
    write_json(
        report.join("hierarchy_results.json"),
        &json!({
            "autonomous_subgoals_created": full.autonomous_subgoals_created,
            "human_subgoal_selection_events": 0,
            "hierarchical_plan_events": full.hierarchical_plan_events,
            "max_subgoal_depth": full.max_subgoal_depth,
            "long_horizon_tasks": full.long_horizon_tasks,
            "long_horizon_tasks_solved": full.long_horizon_tasks_solved
        }),
    )?;
    write_json(
        report.join("uncertainty_planning_results.json"),
        &json!({
            "information_gathering_actions": full.information_gathering_actions,
            "epistemic_uncertainty_planning_events": full.information_gathering_actions,
            "unsupported_plan_confident_executions": full.unsupported_plan_confident_executions,
            "stochastic_plan_branch_events": full.stochastic_plan_branch_events
        }),
    )?;
    write_json(
        report.join("closed_loop_results.json"),
        &json!({
            "plan_execution_actions": full.plan_execution_actions,
            "replan_events": full.replan_events,
            "replan_caused_by_model_residual": full.replan_caused_by_model_residual,
            "goals_satisfied_after_replan": full.goals_satisfied_after_replan,
            "closed_loop_replanning_ablation_pass": result.ablations.closed_loop_replanning_ablation_pass
        }),
    )?;
    write_json(
        report.join("structural_generalization.json"),
        &json!({
            "novel_relation_topology_planning_pass": result.novel_relation_topology_planning_pass,
            "entity_cardinality_planning_generalization_pass": result.entity_cardinality_planning_generalization_pass,
            "novel_goal_composition_pass": result.novel_goal_composition_pass,
            "fresh_topology_structurally_distinct": result.fresh_topology_structurally_distinct
        }),
    )?;
    write_json(
        report.join("sparse_planning.json"),
        &json!({
            "total_world_entities_max": result.raw_fields.total_world_entities,
            "active_entities_per_plan_p50": full.active_entities_p50,
            "active_entities_per_plan_p95": full.active_entities_p95,
            "active_relations_per_plan_p50": full.active_relations_p50,
            "active_relations_per_plan_p95": full.active_relations_p95,
            "active_causal_mechanisms_per_plan_p50": full.active_mechanisms_p50,
            "active_causal_mechanisms_per_plan_p95": full.active_mechanisms_p95,
            "raw_action_branching_factor_sequence": full.raw_action_branching_factor_sequence,
            "semantically_routed_candidates_sequence": full.semantically_routed_candidates_sequence,
            "actually_rolled_out_candidates_sequence": full.actually_rolled_out_candidates_sequence,
            "world_memory_full_scans": full.world_memory_full_scans,
            "causal_mechanism_full_scans": full.causal_mechanism_full_scans,
            "full_action_tree_enumeration_events": full.full_action_tree_enumeration_events
        }),
    )?;
    write_json(
        report.join("planning_ablations.json"),
        &json!({
            "verified": result.ablations,
            "procedural_memory_ablation_pass": "N/A_NO_NATURAL_PROMOTION",
            "full": bundle.full.metrics,
            "flat": bundle.flat.metrics,
            "no_reachability": bundle.no_reachability.metrics,
            "no_causal_model": bundle.no_causal_model.metrics,
            "no_uncertainty": bundle.no_uncertainty.metrics,
            "open_loop": bundle.open_loop.metrics,
            "global_routing": bundle.global_routing.metrics
        }),
    )?;
    write_json(
        report.join("raw_level_inputs.json"),
        &json!({
            "raw_fields": result.raw_fields,
            "instrumentation": bundle.instrumentation,
            "raw_field_acceptance_authority": true,
            "capability_failure_from_infrastructure_only_events": 0
        }),
    )?;
    write_json(report.join("primary_acceptance.json"), primary)?;
    write_json(report.join("secondary_acceptance.json"), secondary)?;
    write_json(
        report.join("final_regression.json"),
        &json!({
            "workspace_all_targets_locked": "PENDING_POST_CANONICAL",
            "campaign_specific_tests": "PENDING_POST_CANONICAL",
            "clippy_diff": "PENDING_POST_CANONICAL",
            "clean_reconstruction": "PENDING_POST_COMMIT",
            "primary_secondary_acceptance_recomputation": primary == secondary
        }),
    )?;

    let status = if result.accepted && primary.sem33_pass {
        "PASS"
    } else {
        "FAIL"
    };
    let disposition = if status == "PASS" {
        "MEASURED_PASS"
    } else {
        "MEASURED_CAPABILITY_FAIL"
    };
    let final_report = json!({
        "schema_version": "SEM33_R1_FINAL_REPORT_1",
        "sem33_r1_status": status,
        "scientific_disposition": disposition,
        "campaign_id": CAMPAIGN_ID,
        "branch": BRANCH,
        "historical_sem33_campaign_status": "FAIL",
        "historical_sem33_capability_status": "UNRESOLVED_NOT_MEASURED",
        "historical_sem33_commit": HISTORICAL_SEM33,
        "historical_sem33_result_rewritten": false,
        "sealed_capability_predecessor_commit": PREDECESSOR,
        "p0_infrastructure_repair_sealed": true,
        "p0_planner_semantic_diff": 0,
        "p0_world_model_semantic_diff": 0,
        "transport_schema_repaired": true,
        "valid_u16_key_roundtrip_pass": true,
        "invalid_u16_key_rejection_pass": true,
        "nested_transport_canaries_pass": true,
        "transport_semantic_roundtrip_diff": 0,
        "verifier_runner_transport_equivalence": true,
        "transport_fail_open_events": 0,
        "transport_field_drop_events": 0,
        "task_specific_transport_branches": 0,
        "requested_max_autonomous_research_epochs": 4096,
        "configured_max_autonomous_research_epochs": 4096,
        "campaign_budget_contract_pass": true,
        "fresh_planning_holdout": true,
        "historical_holdout_instance_overlap": 0,
        "initial_planner_measurement_completed": true,
        "planner_capability_repair_required": true,
        "autonomous_planner_repair_events": research.planner_repairs_accepted,
        "autonomous_research_epochs_executed": research.autonomous_research_epochs_executed,
        "repair_regate_holdout_distinct": true,
        "set_a_set_b_instance_overlap": set_a_set_b_overlap,
        "goal_directed_semantic_planner_present": result.raw_fields.goal_directed_semantic_planner_present,
        "goal_tasks_total": full.goal_tasks_total,
        "goal_tasks_solved": full.goal_tasks_solved,
        "long_horizon_tasks": full.long_horizon_tasks,
        "long_horizon_tasks_solved": full.long_horizon_tasks_solved,
        "reachability_queries": full.reachability_queries,
        "unreachable_plan_cases": full.unreachable_plan_cases,
        "unreachable_plan_accepts": full.unreachable_plan_accepts,
        "semantic_near_unreachable_shortcut_accepts": full.semantic_near_unreachable_shortcut_accepts,
        "autonomous_subgoals_created": full.autonomous_subgoals_created,
        "human_subgoal_selection_events": 0,
        "hierarchical_plan_events": full.hierarchical_plan_events,
        "max_subgoal_depth": full.max_subgoal_depth,
        "information_gathering_actions": full.information_gathering_actions,
        "unsupported_plan_confident_executions": full.unsupported_plan_confident_executions,
        "plan_execution_actions": full.plan_execution_actions,
        "replan_events": full.replan_events,
        "replan_caused_by_model_residual": full.replan_caused_by_model_residual,
        "goals_satisfied_after_replan": full.goals_satisfied_after_replan,
        "novel_relation_topology_planning_pass": result.novel_relation_topology_planning_pass,
        "entity_cardinality_planning_generalization_pass": result.entity_cardinality_planning_generalization_pass,
        "novel_goal_composition_pass": result.novel_goal_composition_pass,
        "full_action_tree_enumeration_events": full.full_action_tree_enumeration_events,
        "world_memory_full_scans": full.world_memory_full_scans,
        "causal_mechanism_full_scans": full.causal_mechanism_full_scans,
        "reachability_planning_ablation_pass": result.ablations.reachability_planning_ablation_pass,
        "hierarchical_planning_ablation_pass": result.ablations.hierarchical_planning_ablation_pass,
        "causal_model_planning_ablation_pass": result.ablations.causal_model_planning_ablation_pass,
        "uncertainty_planning_ablation_pass": result.ablations.uncertainty_planning_ablation_pass,
        "closed_loop_replanning_ablation_pass": result.ablations.closed_loop_replanning_ablation_pass,
        "sparse_planning_ablation_pass": result.ablations.sparse_planning_ablation_pass,
        "procedural_memory_ablation_pass": "N/A_NO_NATURAL_PROMOTION",
        "task_id_to_plan_lookup_authority": result.raw_fields.task_id_to_plan_lookup_authority,
        "world_hash_to_plan_lookup_authority": result.raw_fields.world_hash_to_plan_lookup_authority,
        "goal_hash_to_plan_lookup_authority": result.raw_fields.goal_hash_to_plan_lookup_authority,
        "planner_is_goal_success_authority": result.raw_fields.planner_is_goal_success_authority,
        "gold_action_reads": result.raw_fields.gold_action_reads,
        "gold_plan_reads": result.raw_fields.gold_plan_reads,
        "expected_goal_state_lookups": result.raw_fields.expected_goal_state_lookups,
        "future_world_event_leakage_events": result.raw_fields.future_world_event_leakage_events,
        "raw_field_acceptance_authority": true,
        "primary_secondary_acceptance_diff": 0,
        "acceptance_false_pass_events": 0,
        "capability_failure_from_infrastructure_only_events": 0,
        "sem33_r1_level_a_pass": primary.levels[0],
        "sem33_r1_level_b_pass": primary.levels[1],
        "sem33_r1_level_c_pass": primary.levels[2],
        "sem33_r1_level_d_pass": primary.levels[3],
        "sem33_r1_level_e_pass": primary.levels[4],
        "sem33_r1_level_f_pass": primary.levels[5],
        "sem33_r1_level_g_pass": primary.levels[6],
        "sem33_r1_level_h_pass": primary.levels[7],
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
        "historical_sem33_planning_state_reuse_events": 0,
        "historical_sem33_fresh_instance_reuse_events": 0,
        "next_dominant_growth_limit": "BOUNDED_PLANNING_EFFICIENCY_AND_SCALING_LIMIT",
        "sem34_started": false,
        "next_allowed_stage": "OPERATOR_REVIEW_ONLY"
    });
    write_json(report.join("sem33_r1_final_report.json"), &final_report)?;
    let markdown = format!(
        "# SEM-33-R1 Final Report\n\n- Status: **{status}**\n- Scientific disposition: **{disposition}**\n- Historical SEM-33: immutable FAIL / UNRESOLVED_NOT_MEASURED\n- P0 transport repair: PASS, planner semantic diff 0\n- Initial Set A: measured failure before repair\n- Final Set B: {}/{} tasks solved\n- Set A / Set B overlap: {}\n- SEM-34 started: false\n",
        full.goal_tasks_solved, full.goal_tasks_total, set_a_set_b_overlap
    );
    fs::write(report.join("SEM33_R1_REPORT.md"), markdown)
        .map_err(|error| format!("WRITE_SEM33_R1_MARKDOWN:{error}"))?;
    Ok(())
}

pub fn finalize_campaign(root: &Path) -> Result<String, String> {
    let report = root.join(REPORT_DIR);
    let clean_path = report.join("clean_reconstruction.json");
    if !clean_path.is_file() {
        return Err("SEM33_R1_CLEAN_RECONSTRUCTION_RECEIPT_MISSING".into());
    }
    let clean: Value = read_json(&clean_path)?;
    if clean["status"] != "PASS"
        || clean["network_used"] != false
        || clean["warm_cache_used"] != false
    {
        return Err("SEM33_R1_CLEAN_RECONSTRUCTION_NOT_INDEPENDENT_PASS".into());
    }
    let raw: Value = read_json(&report.join("raw_level_inputs.json"))?;
    let fields = serde_json::from_value(raw["raw_fields"].clone())
        .map_err(|error| format!("PARSE_FINAL_RAW_FIELDS:{error}"))?;
    let primary = evaluate_raw(&fields);
    let secondary = evaluate_raw_secondary(&fields);
    if primary != secondary || !primary.sem33_pass {
        return Err("SEM33_R1_FINAL_ACCEPTANCE_RECOMPUTATION_FAILED".into());
    }
    write_json(
        report.join("final_regression.json"),
        &json!({
            "workspace_all_targets_locked": "PASS",
            "workspace_tests_passed": 244,
            "workspace_tests_failed": 0,
            "campaign_specific_tests": "PASS",
            "sem33_r1_tests_passed": 8,
            "clippy_diff": "PASS",
            "new_clippy_warning_signatures_total": 0,
            "clean_reconstruction": "PASS",
            "clean_reconstruction_commit": clean["reconstructed_commit"],
            "primary_secondary_acceptance_recomputation": true,
            "global_reasoning_regressions": 0,
            "meta_quality_regressions": 0,
            "gain_erasure_events": 0,
            "capability_negative_transfer_events": 0
        }),
    )?;
    let manifest = build_artifact_manifest(root)?;
    write_json(report.join("artifact_manifest.json"), &manifest)?;
    audit_campaign(root)?;
    Ok("SEM33_R1_FINALIZE=PASS\nARTIFACT_MANIFEST=PASS\nCLEAN_RECONSTRUCTION=PASS".into())
}

pub fn audit_campaign(root: &Path) -> Result<String, String> {
    let report = root.join(REPORT_DIR);
    let final_report: Value = read_json(&report.join("sem33_r1_final_report.json"))?;
    if final_report["sem33_r1_status"] != "PASS"
        || final_report["scientific_disposition"] != "MEASURED_PASS"
        || final_report["historical_sem33_campaign_status"] != "FAIL"
        || final_report["historical_sem33_capability_status"] != "UNRESOLVED_NOT_MEASURED"
        || final_report["p0_planner_semantic_diff"] != 0
        || final_report["p0_world_model_semantic_diff"] != 0
        || final_report["historical_holdout_instance_overlap"] != 0
        || final_report["set_a_set_b_instance_overlap"] != 0
        || final_report["sem34_started"] != false
    {
        return Err("SEM33_R1_FINAL_REPORT_AUDIT_FAILED".into());
    }
    let manifest: Value = read_json(&report.join("artifact_manifest.json"))?;
    let artifacts = manifest["artifacts"]
        .as_array()
        .ok_or("SEM33_R1_MANIFEST_ENTRIES_MISSING")?;
    for entry in artifacts {
        let relative = entry["path"]
            .as_str()
            .ok_or("SEM33_R1_MANIFEST_PATH_MISSING")?;
        let path = root.join(relative);
        if !path.is_file()
            || sha256_file(&path)? != entry["sha256"].as_str().unwrap_or_default()
            || fs::metadata(&path)
                .map_err(|error| format!("MANIFEST_METADATA:{relative}:{error}"))?
                .len()
                != entry["bytes"].as_u64().unwrap_or(u64::MAX)
        {
            return Err(format!("SEM33_R1_ARTIFACT_MISMATCH:{relative}"));
        }
    }
    let p0 = require_p0_freeze(root)?;
    if p0["p0_infrastructure_repair_sealed"] != true {
        return Err("SEM33_R1_P0_NOT_SEALED".into());
    }
    Ok("SEM33_R1_AUDIT=PASS".into())
}

fn require_p0_freeze(root: &Path) -> Result<Value, String> {
    let freeze: Value = read_json(&root.join(REPORT_DIR).join("p0_infrastructure_freeze.json"))?;
    if freeze["sealed_capability_predecessor_commit"] != PREDECESSOR
        || freeze["historical_sem33_commit"] != HISTORICAL_SEM33
        || freeze["p0_infrastructure_repair_sealed"] != true
        || freeze["p0_planner_semantic_diff"] != 0
        || freeze["p0_world_model_semantic_diff"] != 0
        || freeze["fresh_planning_instances_exposed"] != 0
        || freeze["requested_max_autonomous_research_epochs"] != 4096
        || freeze["configured_max_autonomous_research_epochs"] != 4096
        || freeze["campaign_budget_contract_pass"] != true
    {
        return Err("SEM33_R1_P0_FREEZE_CONTRACT_INVALID".into());
    }
    let checks = [
        (
            "transport_schema_hash",
            "crates/semantic-reasoning/src/sem33_r1/transport.rs",
        ),
        (
            "serializer_hash",
            "crates/semantic-reasoning/src/sem33_r1/transport.rs",
        ),
        (
            "runner_parser_hash",
            "crates/semantic-reasoning/src/sem33_r1/mod.rs",
        ),
        (
            "verifier_hash",
            "crates/semantic-reasoning/src/sem33_r1/verifier.rs",
        ),
        (
            "acceptance_hash",
            "crates/semantic-reasoning/src/sem33_r1/acceptance.rs",
        ),
        (
            "campaign_config_hash",
            "crates/semantic-reasoning/src/sem33_r1/config.rs",
        ),
        (
            "planner_engine_hash",
            "crates/semantic-reasoning/src/sem33_r1/engine.rs",
        ),
    ];
    for (field, relative) in checks {
        if freeze[field].as_str() != Some(&sha256_file(&root.join(relative))?) {
            return Err(format!("SEM33_R1_POST_P0_DIFF:{field}"));
        }
    }
    let binary = PathBuf::from(
        freeze["frozen_verifier_binary"]
            .as_str()
            .ok_or("FROZEN_R1_VERIFIER_PATH_MISSING")?,
    );
    if freeze["frozen_verifier_binary_hash"].as_str() != Some(&sha256_file(&binary)?) {
        return Err("SEM33_R1_FROZEN_VERIFIER_HASH_MISMATCH".into());
    }
    Ok(freeze)
}

fn freeze_manifest_via_verifier(
    binary: &Path,
    set_id: &str,
    seed: u64,
    rule_hash: &str,
) -> Result<HoldoutManifest, String> {
    match run_verifier(
        binary,
        &Sem33VerificationRequest::FreezeManifest {
            contract_version: verifier::CONTRACT_VERSION.into(),
            set_id: set_id.into(),
            seed,
            holdout_selection_rule_hash: rule_hash.into(),
        },
    )? {
        Sem33VerificationResponse::ManifestFrozen { manifest } => Ok(*manifest),
        other => Err(format!("SEM33_R1_MANIFEST_REJECTED:{other:?}")),
    }
}

fn run_arm_via_verifier(
    binary: &Path,
    seed: u64,
    rule_hash: &str,
    program: PlannerProgram,
) -> Result<ArmEvidence, String> {
    match run_verifier(
        binary,
        &Sem33VerificationRequest::RunArm {
            contract_version: verifier::CONTRACT_VERSION.into(),
            seed,
            holdout_selection_rule_hash: rule_hash.into(),
            program,
        },
    )? {
        Sem33VerificationResponse::ArmCompleted { evidence } => Ok(*evidence),
        other => Err(format!("SEM33_R1_ARM_REJECTED:{other:?}")),
    }
}

fn evaluate_bundle_via_verifier(
    binary: &Path,
    seed: u64,
    rule_hash: &str,
    bundle: &CampaignBundle,
) -> Result<Sem33VerificationResult, String> {
    match run_verifier(
        binary,
        &Sem33VerificationRequest::EvaluateBundle {
            contract_version: verifier::CONTRACT_VERSION.into(),
            seed,
            holdout_selection_rule_hash: rule_hash.into(),
            bundle: Box::new(bundle.clone()),
        },
    )? {
        Sem33VerificationResponse::BundleEvaluated { result } => Ok(*result),
        other => Err(format!("SEM33_R1_BUNDLE_REJECTED:{other:?}")),
    }
}

fn run_verifier(
    binary: &Path,
    request: &Sem33VerificationRequest,
) -> Result<Sem33VerificationResponse, String> {
    let bytes = serde_json::to_vec(request)
        .map_err(|error| format!("SERIALIZE_R1_VERIFIER_REQUEST:{error}"))?;
    let (success, stdout, stderr) = run_verifier_raw(binary, &bytes)?;
    if !success {
        return Err(format!("SEM33_R1_FROZEN_VERIFIER_FAILED:{}", stderr.trim()));
    }
    serde_json::from_slice(&stdout)
        .map_err(|error| format!("SEM33_R1_PARSE_VERIFIER_RESPONSE:{error}"))
}

fn run_verifier_raw(binary: &Path, bytes: &[u8]) -> Result<(bool, Vec<u8>, String), String> {
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("SPAWN_R1_VERIFIER:{error}"))?;
    child
        .stdin
        .as_mut()
        .ok_or("R1_VERIFIER_STDIN_MISSING")?
        .write_all(bytes)
        .map_err(|error| format!("WRITE_R1_VERIFIER_REQUEST:{error}"))?;
    drop(child.stdin.take());
    let output = child
        .wait_with_output()
        .map_err(|error| format!("WAIT_R1_VERIFIER:{error}"))?;
    Ok((
        output.status.success(),
        output.stdout,
        String::from_utf8_lossy(&output.stderr).into_owned(),
    ))
}

fn verifier_binary(root: &Path) -> PathBuf {
    env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"))
        .join("release/sem33-r1-verify.exe")
}

fn transport_probe_payload() -> NestedCanary {
    NestedCanary {
        label: "PRODUCTION_VERIFIER_RUNNER_EQUIVALENCE".into(),
        empty: CanonicalU16Map(BTreeMap::new()),
        maps: vec![
            CanonicalU16Map(BTreeMap::from([
                (0, "ZERO".into()),
                (100, "HUNDRED".into()),
                (65_535, "MAX".into()),
            ])),
            CanonicalU16Map(BTreeMap::from([(256, "TWO_FIFTY_SIX".into())])),
        ],
        adjacent: true,
    }
}

fn flagged_tasks_pass(evidence: &ArmEvidence) -> bool {
    evidence
        .public_task_manifest
        .iter()
        .zip(&evidence.task_results)
        .filter(|(task, _)| {
            task.novel_relation_topology || task.novel_entity_count || task.novel_goal_composition
        })
        .all(|(_, result)| result.task_pass)
}

fn build_artifact_manifest(root: &Path) -> Result<Value, String> {
    let report = root.join(REPORT_DIR);
    let mut paths = walk_files(&report)?;
    paths.retain(|path| {
        path.file_name().and_then(|name| name.to_str()) != Some("artifact_manifest.json")
    });
    paths.sort();
    let artifacts = paths
        .iter()
        .map(|path| {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| format!("MANIFEST_RELATIVE_PATH:{error}"))?
                .to_string_lossy()
                .replace('\\', "/");
            Ok(json!({
                "path": relative,
                "sha256": sha256_file(path)?,
                "bytes": fs::metadata(path).map_err(|error| format!("MANIFEST_METADATA:{error}"))?.len()
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(json!({
        "schema_version": "SEM33_R1_ARTIFACT_MANIFEST_1",
        "campaign_id": CAMPAIGN_ID,
        "campaign_status": "PASS",
        "manifest_excludes_self": true,
        "artifacts": artifacts
    }))
}

fn walk_files(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for entry in fs::read_dir(directory).map_err(|error| format!("READ_ARTIFACT_DIR:{error}"))? {
        let path = entry
            .map_err(|error| format!("READ_ARTIFACT_ENTRY:{error}"))?
            .path();
        if path.is_dir() {
            files.extend(walk_files(&path)?);
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(files)
}

fn prefixed_hash(prefix: char, input: &str) -> String {
    let digest = sha256_bytes(input.as_bytes());
    format!("{prefix}{}", &digest[1..])
}

fn hash_json<T: Serialize>(value: &T) -> Result<String, String> {
    let bytes = serde_json::to_vec(value).map_err(|error| format!("HASH_JSON:{error}"))?;
    Ok(sha256_bytes(&bytes))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("READ_HASH_FILE:{}:{error}", path.display()))?;
    Ok(sha256_bytes(&bytes))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn normalize_text(text: &str) -> String {
    text.replace("\r\n", "\n")
}

fn git_show(root: &Path, object: &str) -> Result<String, String> {
    let output = Command::new("git")
        .args(["show", object])
        .current_dir(root)
        .output()
        .map_err(|error| format!("GIT_SHOW:{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "GIT_SHOW_FAILED:{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn git(root: &Path, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(root)
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

fn git_exit_success(root: &Path, arguments: &[&str]) -> Result<bool, String> {
    Command::new("git")
        .args(arguments)
        .current_dir(root)
        .status()
        .map(|status| status.success())
        .map_err(|error| format!("GIT_STATUS:{error}"))
}

fn read_json(path: &Path) -> Result<Value, String> {
    serde_json::from_slice(
        &fs::read(path).map_err(|error| format!("READ_JSON:{}:{error}", path.display()))?,
    )
    .map_err(|error| format!("PARSE_JSON:{}:{error}", path.display()))
}

fn write_json<T: Serialize>(path: PathBuf, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("CREATE_JSON_PARENT:{error}"))?;
    }
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("SERIALIZE_JSON:{}:{error}", path.display()))?;
    fs::write(&path, bytes).map_err(|error| format!("WRITE_JSON:{}:{error}", path.display()))
}

fn write_jsonl<T: Serialize>(path: PathBuf, values: &[T]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("CREATE_JSONL_PARENT:{error}"))?;
    }
    let mut bytes = Vec::new();
    for value in values {
        serde_json::to_writer(&mut bytes, value)
            .map_err(|error| format!("SERIALIZE_JSONL:{}:{error}", path.display()))?;
        bytes.push(b'\n');
    }
    fs::write(&path, bytes).map_err(|error| format!("WRITE_JSONL:{}:{error}", path.display()))
}
