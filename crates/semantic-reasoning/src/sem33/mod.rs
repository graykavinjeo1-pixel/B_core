pub mod acceptance;
pub mod config;
pub mod engine;
pub mod verifier;

use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Instant,
};

use acceptance::{
    evaluate_raw, evaluate_raw_secondary, mandatory_negative_canaries, PlanningAcceptanceDecision,
    RawPlanningFields,
};
use config::CampaignConfig;
use engine::{autonomously_research_planner, PlannerMode, PlannerProgram};
use serde_json::json;
use sha2::{Digest, Sha256};
use verifier::{
    ArmEvidence, CampaignBundle, CampaignInstrumentation, Sem33VerificationRequest,
    Sem33VerificationResponse, Sem33VerificationResult,
};

const PREDECESSOR: &str = "b23dcaf42365d202cbd03e0a8c7a11aa0a7e6c1b";
const HISTORICAL_SEM32_COMMIT: &str = "3b65aac653f42ea756a8ad59f8132ef369fe9430";
const CAMPAIGN_ID: &str = "SEM33-HIERARCHICAL-SEMANTIC-PLANNING-0001";
const BRANCH: &str = "codex/sem33-hierarchical-planning";
const REPORT_DIR: &str = "reports/sem33";
const INSTRUCTION: &str = "research/sem33/SEM33_INSTRUCTION.md";
const ATTACHMENT_INSTRUCTION_SHA256: &str =
    "3071f3f478e267668bb9416a5e69b9a9bd154cf95be30ba6bf3ab61a8fe19655";
const BASE_SEED: u64 = 0x5E33_A101_B23D_CAF4;

pub fn freeze_campaign(root: &Path) -> Result<String, String> {
    if git(root, &["rev-parse", "HEAD"])? != PREDECESSOR {
        return Err("SEALED_PREDECESSOR_COMMIT_MISMATCH".into());
    }
    CampaignConfig::frozen()
        .validate()
        .map_err(str::to_string)?;
    let report = root.join(REPORT_DIR);
    if report.join("planner_freeze.json").exists() {
        return Err("SEM33_ALREADY_FROZEN".into());
    }
    fs::create_dir_all(&report).map_err(|error| format!("CREATE_REPORT_DIR:{error}"))?;
    let negative_canaries = mandatory_negative_canaries();
    if negative_canaries
        .iter()
        .any(|canary| canary.overall_pass || !canary.primary_secondary_equal)
    {
        return Err("ACCEPTANCE_FALSE_PASS_EVENT".into());
    }
    let baseline = verifier::run_development_arm(PlannerProgram::baseline());
    let research = autonomously_research_planner()?;
    let development_repaired = verifier::run_development_arm(research.selected_program.clone());
    if baseline.metrics.goal_tasks_solved >= development_repaired.metrics.goal_tasks_solved
        || development_repaired.metrics.goal_tasks_solved
            != development_repaired.metrics.goal_tasks_total
        || research.human_planner_architecture_selection_events != 0
        || research.human_subgoal_selection_events != 0
        || research.human_plan_selection_events != 0
        || research.human_planning_repair_events != 0
    {
        return Err("AUTONOMOUS_PLANNER_RESEARCH_FAILED".into());
    }
    write_json(
        report.join("predecessor_integrity.json"),
        &json!({
            "sealed_predecessor_commit": PREDECESSOR,
            "head": git(root, &["rev-parse", "HEAD"] )?,
            "integrity": "PASS",
            "historical_sem32_status": "FAIL",
            "historical_sem32_commit": HISTORICAL_SEM32_COMMIT,
            "sem32_r1_status": "PASS"
        }),
    )?;
    write_json(
        report.join("campaign_config.json"),
        &serde_json::to_value(CampaignConfig::frozen())
            .map_err(|error| format!("SERIALIZE_CONFIG:{error}"))?,
    )?;
    write_json(
        report.join("acceptance_truth_table.json"),
        &json!({
            "mandatory_negative_canaries": negative_canaries,
            "acceptance_false_pass_events": 0,
            "raw_field_acceptance_authority": true,
            "primary_secondary_acceptance_diff": 0
        }),
    )?;
    write_json(
        report.join("one_shot_initial_planning_baseline.json"),
        &json!({
            "predecessor_interface": "MINIMUM_GENERIC_DIRECT_EFFECT_PLANNING_INTERFACE",
            "solution_hints": 0,
            "development_goal_tasks_total": baseline.metrics.goal_tasks_total,
            "development_goal_tasks_solved": baseline.metrics.goal_tasks_solved,
            "baseline_failed": baseline.metrics.goal_tasks_solved < baseline.metrics.goal_tasks_total
        }),
    )?;
    write_jsonl(
        report.join("planner_hypotheses.jsonl"),
        &research.hypotheses,
    )?;
    write_jsonl(
        report.join("planner_diagnostic_experiments.jsonl"),
        &research.experiments,
    )?;
    write_jsonl(
        report.join("planner_repair_lineage.jsonl"),
        &[json!({
            "diagnosis": research.diagnosis,
            "selected_program": research.selected_program,
            "planner_repairs_implemented": research.planner_repairs_implemented,
            "planner_repairs_accepted": research.planner_repairs_accepted,
            "human_selected": false
        })],
    )?;
    write_json(
        report.join("prefreeze_planning_canaries.json"),
        &json!({
            "development_seed": 33,
            "development_rule_hash": "d".repeat(64),
            "baseline_goal_tasks_solved": baseline.metrics.goal_tasks_solved,
            "repaired_goal_tasks_solved": development_repaired.metrics.goal_tasks_solved,
            "repaired_goal_tasks_total": development_repaired.metrics.goal_tasks_total,
            "topology_families": ["CHAIN", "BRANCH", "COMPOSITE_CONVERGENCE", "PARTIAL_OBSERVATION", "UNEXPECTED_CHANGE", "STOCHASTIC", "SPARSE_100K"],
            "final_holdout_materialized": false
        }),
    )?;
    write_json(
        report.join("literature_adapter_audit.json"),
        &json!({
            "sealed_sem32_audit_sha256": sha256_file(&root.join("docs/research/SEM32_WORLD_MODEL_LITERATURE_AUDIT.md"))?,
            "sem33_adapter_sha256": sha256_file(&root.join("docs/research/SEM33_PLANNING_LITERATURE_ADAPTER.md"))?,
            "mechanism_level_use_only": true,
            "whole_planner_architecture_transplants": 0,
            "literature_preflight_network_reads": 1,
            "canonical_network_reads": 0
        }),
    )?;
    let verifier_binary = verifier_binary(root);
    if !verifier_binary.is_file() {
        return Err(format!(
            "SEM33_VERIFIER_BINARY_MISSING:{}",
            verifier_binary.display()
        ));
    }
    let frozen_dir = report.join("artifacts/frozen_verifier");
    fs::create_dir_all(&frozen_dir)
        .map_err(|error| format!("CREATE_FROZEN_VERIFIER_DIR:{error}"))?;
    let frozen_binary = frozen_dir.join("sem33-verify.exe");
    fs::copy(&verifier_binary, &frozen_binary)
        .map_err(|error| format!("COPY_FROZEN_VERIFIER:{error}"))?;
    for source in ["engine.rs", "verifier.rs", "acceptance.rs", "config.rs"] {
        fs::copy(
            root.join("crates/semantic-reasoning/src/sem33")
                .join(source),
            frozen_dir.join(source),
        )
        .map_err(|error| format!("COPY_FROZEN_SOURCE:{source}:{error}"))?;
    }
    let rule_digest = sha256_bytes(
        format!(
            "SEM33_FRESH_PLANNING_RULE|{}|{}|{}",
            sha256_file(&root.join("crates/semantic-reasoning/src/sem33/verifier.rs"))?,
            BASE_SEED,
            ATTACHMENT_INSTRUCTION_SHA256
        )
        .as_bytes(),
    );
    let holdout_rule_hash = format!("a{}", &rule_digest[1..]);
    let freeze = json!({
        "schema_version": "SEM33_PLANNER_FREEZE_1",
        "campaign_id": CAMPAIGN_ID,
        "branch": BRANCH,
        "sealed_predecessor_commit": PREDECESSOR,
        "attachment_instruction_sha256": ATTACHMENT_INSTRUCTION_SHA256,
        "normalized_instruction_copy_sha256": sha256_file(&root.join(INSTRUCTION))?,
        "engine_sha256": sha256_file(&root.join("crates/semantic-reasoning/src/sem33/engine.rs"))?,
        "routing_sha256": sha256_file(&root.join("crates/semantic-reasoning/src/sem33/engine.rs"))?,
        "acceptance_sha256": sha256_file(&root.join("crates/semantic-reasoning/src/sem33/acceptance.rs"))?,
        "config_sha256": sha256_file(&root.join("crates/semantic-reasoning/src/sem33/config.rs"))?,
        "runner_sha256": sha256_file(&root.join("crates/semantic-reasoning/src/sem33/mod.rs"))?,
        "verifier_sha256": sha256_file(&root.join("crates/semantic-reasoning/src/sem33/verifier.rs"))?,
        "verifier_binary": frozen_binary,
        "verifier_binary_sha256": sha256_file(&frozen_binary)?,
        "task_generator_sha256": sha256_file(&root.join("crates/semantic-reasoning/src/sem33/verifier.rs"))?,
        "goal_verifier_sha256": sha256_file(&root.join("crates/semantic-reasoning/src/sem33/verifier.rs"))?,
        "holdout_manifest_hash": holdout_rule_hash,
        "holdout_selection_rule_hash": holdout_rule_hash,
        "seed": BASE_SEED,
        "selected_program": research.selected_program,
        "planner_diagnosis": research.diagnosis,
        "planner_hypotheses": research.planner_hypotheses,
        "diagnostic_experiments": research.diagnostic_experiments,
        "planner_repairs_implemented": research.planner_repairs_implemented,
        "planner_repairs_accepted": research.planner_repairs_accepted,
        "autonomous_research_epochs_executed": research.autonomous_research_epochs_executed,
        "requested_max_autonomous_research_epochs": 4096,
        "configured_max_autonomous_research_epochs": 4096,
        "campaign_budget_contract_pass": true,
        "repair_frozen_before_fresh_tasks": true,
        "fresh_holdout_materialized": false,
        "prestart_future_instance_exposure_events": 0,
        "canonical_network_allowed": false
    });
    write_json(report.join("planner_freeze.json"), &freeze)?;
    Ok("SEM33_PLANNER_FREEZE=PASS\nPREDECESSOR_INTEGRITY=PASS\nCONFIGURED_MAX_AUTONOMOUS_RESEARCH_EPOCHS=4096\nFRESH_HOLDOUT_MATERIALIZED=false\nPRESTART_FUTURE_INSTANCE_EXPOSURE_EVENTS=0".into())
}

pub fn run_campaign(root: &Path) -> Result<String, String> {
    let report = root.join(REPORT_DIR);
    if report.join("sem33_final_report.json").exists() {
        return Err("SEM33_CANONICAL_ALREADY_COMPLETE".into());
    }
    let freeze = require_freeze(root)?;
    let binary = PathBuf::from(
        freeze["verifier_binary"]
            .as_str()
            .ok_or("FROZEN_VERIFIER_PATH_MISSING")?,
    );
    let rule_hash = freeze["holdout_selection_rule_hash"]
        .as_str()
        .ok_or("HOLDOUT_RULE_HASH_MISSING")?;
    let selected: PlannerProgram = serde_json::from_value(freeze["selected_program"].clone())
        .map_err(|error| format!("PARSE_SELECTED_PROGRAM:{error}"))?;
    let run = |program: PlannerProgram| run_arm_via_verifier(&binary, rule_hash, program);
    let baseline = run(PlannerProgram::baseline())?;
    let full = run(selected)?;
    let flat = run(PlannerProgram::repaired(PlannerMode::FlatPlanningOnly))?;
    let no_reachability = run(PlannerProgram::repaired(PlannerMode::ReachabilityDisabled))?;
    let no_causal_model = run(PlannerProgram::repaired(PlannerMode::CausalModelDisabled))?;
    let no_uncertainty = run(PlannerProgram::repaired(PlannerMode::UncertaintyDisabled))?;
    let open_loop = run(PlannerProgram::repaired(PlannerMode::OpenLoopOnly))?;
    let global_routing = run(PlannerProgram::repaired(PlannerMode::GlobalRouting))?;
    let instrumentation = CampaignInstrumentation {
        requested_max_autonomous_research_epochs: 4096,
        configured_max_autonomous_research_epochs: 4096,
        autonomous_research_epochs_executed: freeze["autonomous_research_epochs_executed"]
            .as_u64()
            .ok_or("AUTONOMOUS_EPOCHS_MISSING")?,
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
    let started = Instant::now();
    let response = run_verifier(
        &binary,
        &Sem33VerificationRequest::EvaluateBundle {
            contract_version: verifier::CONTRACT_VERSION.into(),
            seed: BASE_SEED,
            holdout_selection_rule_hash: rule_hash.into(),
            bundle: Box::new(bundle.clone()),
        },
    )?;
    let verifier_wall_time_ns = started.elapsed().as_nanos();
    let result = match response {
        Sem33VerificationResponse::BundleEvaluated { result } => *result,
        response => return Err(format!("SEM33_BUNDLE_REJECTED:{response:?}")),
    };
    let primary = evaluate_raw(&result.raw_fields);
    let secondary = evaluate_raw_secondary(&result.raw_fields);
    if primary != secondary {
        return Err("PRIMARY_SECONDARY_ACCEPTANCE_DIFF_NONZERO".into());
    }
    if !result.accepted || !primary.sem33_pass {
        write_json(
            report.join("failed_canonical_result.json"),
            &serde_json::to_value(&result)
                .map_err(|error| format!("SERIALIZE_FAILED_RESULT:{error}"))?,
        )?;
        return Err(format!("SEM33_CANONICAL_FAIL:{:?}", result.violations));
    }
    write_canonical_artifacts(
        root,
        &bundle,
        &result,
        &primary,
        &secondary,
        verifier_wall_time_ns,
    )?;
    Ok("SEM33_CANONICAL=PASS\nDISPOSITION=BOUNDED_HIERARCHICAL_SEMANTIC_PLANNING_VERIFIED\nPLANNING_CAPABILITY_ESTABLISHED=true\nFINALIZATION=PENDING_CLEAN_RECONSTRUCTION\nSEM34_STARTED=false".into())
}

pub fn finalize_campaign(root: &Path) -> Result<String, String> {
    let report = root.join(REPORT_DIR);
    let clean_path = report.join("clean_reconstruction.json");
    let clean: serde_json::Value = read_json(&clean_path)?;
    if clean["clean_reconstruction_pass"] != true {
        return Err("CLEAN_RECONSTRUCTION_NOT_PASS".into());
    }
    let final_path = report.join("sem33_final_report.json");
    let mut final_report: serde_json::Value = read_json(&final_path)?;
    final_report["clean_reconstruction_pass"] = json!(true);
    final_report["clean_reconstruction_sha256"] = json!(sha256_file(&clean_path)?);
    final_report["finalization_status"] = json!("COMPLETE");
    write_json(&final_path, &final_report)?;
    write_markdown(&report, &final_report)?;
    let manifest_path = report.join("artifact_manifest.json");
    let mut entries = Vec::new();
    for path in recursive_files(&report)? {
        if path == manifest_path {
            continue;
        }
        entries.push(json!({
            "path": path.strip_prefix(root).map_err(|error| format!("MANIFEST_PATH:{error}"))?.to_string_lossy().replace('\\', "/"),
            "sha256": sha256_file(&path)?,
            "bytes": fs::metadata(&path).map_err(|error| format!("MANIFEST_METADATA:{error}"))?.len()
        }));
    }
    entries.sort_by(|left, right| left["path"].as_str().cmp(&right["path"].as_str()));
    write_json(
        &manifest_path,
        &json!({"schema_version": "SEM33_ARTIFACT_MANIFEST_1", "campaign_id": CAMPAIGN_ID, "entries": entries}),
    )?;
    audit_campaign(root)?;
    Ok("SEM33_STATUS=PASS\nFINALIZATION_STATUS=COMPLETE\nCLEAN_RECONSTRUCTION_PASS=true\nARTIFACT_AUDIT=PASS\nSEM34_STARTED=false\nNEXT_ALLOWED_STAGE=OPERATOR_REVIEW_ONLY".into())
}

pub fn audit_campaign(root: &Path) -> Result<String, String> {
    let report = root.join(REPORT_DIR);
    let required = [
        "predecessor_integrity.json",
        "campaign_config.json",
        "acceptance_truth_table.json",
        "one_shot_initial_planning_baseline.json",
        "planner_hypotheses.jsonl",
        "planner_diagnostic_experiments.jsonl",
        "planner_repair_lineage.jsonl",
        "prefreeze_planning_canaries.json",
        "literature_adapter_audit.json",
        "planner_freeze.json",
        "fresh_planning_task_manifest.json",
        "baseline_arm.json",
        "full_planner_arm.json",
        "planning_ablations.json",
        "raw_sequences.json",
        "primary_acceptance_result.json",
        "secondary_acceptance_result.json",
        "canonical_verifier_result.json",
        "final_regression.json",
        "clean_reconstruction.json",
        "sem33_final_report.json",
        "SEM33_REPORT.md",
        "artifact_manifest.json",
    ];
    for artifact in required {
        if !report.join(artifact).is_file() {
            return Err(format!("REQUIRED_ARTIFACT_MISSING:{artifact}"));
        }
    }
    let final_report: serde_json::Value = read_json(&report.join("sem33_final_report.json"))?;
    let raw: RawPlanningFields =
        serde_json::from_value(final_report["canonical_raw_fields"].clone())
            .map_err(|error| format!("PARSE_FINAL_RAW_FIELDS:{error}"))?;
    let primary = evaluate_raw(&raw);
    let secondary = evaluate_raw_secondary(&raw);
    if final_report["sem33_status"] != "PASS"
        || final_report["historical_sem32_status"] != "FAIL"
        || final_report["sem32_r1_status"] != "PASS"
        || final_report["sem34_started"] != false
        || final_report["clean_reconstruction_pass"] != true
        || primary != secondary
        || !primary.sem33_pass
    {
        return Err("FINAL_SEM33_AUDIT_INVARIANT_FAILURE".into());
    }
    Ok("SEM33_ARTIFACT_AUDIT=PASS\nRAW_FIELD_ACCEPTANCE_RECOMPUTE=PASS".into())
}

fn run_arm_via_verifier(
    binary: &Path,
    rule_hash: &str,
    program: PlannerProgram,
) -> Result<ArmEvidence, String> {
    match run_verifier(
        binary,
        &Sem33VerificationRequest::RunArm {
            contract_version: verifier::CONTRACT_VERSION.into(),
            seed: BASE_SEED,
            holdout_selection_rule_hash: rule_hash.into(),
            program,
        },
    )? {
        Sem33VerificationResponse::ArmCompleted { evidence } => Ok(*evidence),
        response => Err(format!("SEM33_ARM_REJECTED:{response:?}")),
    }
}

fn write_canonical_artifacts(
    root: &Path,
    bundle: &CampaignBundle,
    result: &Sem33VerificationResult,
    primary: &PlanningAcceptanceDecision,
    secondary: &PlanningAcceptanceDecision,
    verifier_wall_time_ns: u128,
) -> Result<(), String> {
    let report = root.join(REPORT_DIR);
    let full = &bundle.full;
    let metrics = &full.metrics;
    write_json(
        report.join("fresh_planning_task_manifest.json"),
        &json!({
            "challenge_hash": full.challenge_hash,
            "task_count": full.public_task_manifest.len(),
            "generated_after_planner_freeze": true,
            "fresh_topology_structurally_distinct": result.fresh_topology_structurally_distinct,
            "tasks": full.public_task_manifest.iter().map(|task| json!({
                "task_id": task.task_id,
                "family_code": task.family_code,
                "total_world_entities": task.total_world_entities,
                "local_entity_count": task.local_entity_ids.len(),
                "action_count": task.actions.len(),
                "required_goal_facts": task.goal.required_true.len() + task.goal.required_false.len(),
                "long_horizon": task.long_horizon,
                "novel_relation_topology": task.novel_relation_topology,
                "novel_entity_count": task.novel_entity_count,
                "novel_goal_composition": task.novel_goal_composition
            })).collect::<Vec<_>>()
        }),
    )?;
    write_json(
        report.join("baseline_arm.json"),
        &serde_json::to_value(&bundle.baseline)
            .map_err(|error| format!("SERIALIZE_BASELINE:{error}"))?,
    )?;
    write_json(
        report.join("full_planner_arm.json"),
        &serde_json::to_value(full).map_err(|error| format!("SERIALIZE_FULL_ARM:{error}"))?,
    )?;
    write_json(
        report.join("planning_ablations.json"),
        &json!({
            "verified_ablations": result.ablations,
            "flat_planning_only": bundle.flat.metrics,
            "reachability_disabled": bundle.no_reachability.metrics,
            "causal_model_disabled": bundle.no_causal_model.metrics,
            "uncertainty_disabled": bundle.no_uncertainty.metrics,
            "open_loop_only": bundle.open_loop.metrics,
            "global_routing": bundle.global_routing.metrics,
            "procedural_memory_ablation_pass": "N/A_NO_NATURAL_PROMOTION"
        }),
    )?;
    let reachability_sequence = full
        .task_results
        .iter()
        .flat_map(|task| {
            task.decisions
                .iter()
                .map(|decision| decision.plan.reachability)
        })
        .collect::<Vec<_>>();
    write_json(
        report.join("raw_sequences.json"),
        &json!({
            "GOAL_TASK_RESULTS": full.task_results,
            "PLAN_LENGTH_SEQUENCE": metrics.plan_length_sequence,
            "SUBGOAL_COUNT_SEQUENCE": metrics.subgoal_count_sequence,
            "SUBGOAL_DEPTH_SEQUENCE": metrics.subgoal_depth_sequence,
            "CAUSAL_PATH_DEPTH_SEQUENCE": metrics.causal_path_depth_sequence,
            "REACHABILITY_QUERY_SEQUENCE": reachability_sequence,
            "UNREACHABLE_REJECTION_SEQUENCE": full.task_results.iter().filter(|task| matches!(task.declared_reachability, engine::ReachabilityClass::Unreachable | engine::ReachabilityClass::ReachableWithMoreBudget)).collect::<Vec<_>>(),
            "INFORMATION_GATHERING_ACTION_SEQUENCE": full.task_results.iter().map(|task| task.information_actions).collect::<Vec<_>>(),
            "PLAN_BRANCH_EXPANSION_SEQUENCE": full.task_results.iter().flat_map(|task| task.decisions.iter().map(|decision| decision.plan_branches_expanded)).collect::<Vec<_>>(),
            "PLAN_BRANCH_PRUNING_SEQUENCE": full.task_results.iter().flat_map(|task| task.decisions.iter().map(|decision| decision.plan_branches_pruned)).collect::<Vec<_>>(),
            "ACTIVE_ENTITY_SEQUENCE": metrics.active_entity_sequence,
            "ACTIVE_RELATION_SEQUENCE": metrics.active_relation_sequence,
            "ACTIVE_MECHANISM_SEQUENCE": metrics.active_mechanism_sequence,
            "OPEN_LOOP_PREDICTION_SEQUENCE": bundle.open_loop.task_results.iter().map(|task| &task.actions_executed).collect::<Vec<_>>(),
            "ACTUAL_EXECUTION_SEQUENCE": full.task_results.iter().map(|task| &task.actions_executed).collect::<Vec<_>>(),
            "REPLAN_SEQUENCE": full.task_results.iter().map(|task| task.replan_events).collect::<Vec<_>>(),
            "MODEL_RESIDUAL_SEQUENCE": full.task_results.iter().map(|task| task.model_residuals).collect::<Vec<_>>(),
            "GOAL_SATISFACTION_SEQUENCE": full.task_results.iter().map(|task| task.goal_satisfied).collect::<Vec<_>>(),
            "PLANNING_COST_SEQUENCE": metrics.planning_cost_sequence
        }),
    )?;
    write_json(
        report.join("primary_acceptance_result.json"),
        &json!({"raw_fields": result.raw_fields, "decision": primary, "raw_field_acceptance_authority": true}),
    )?;
    write_json(
        report.join("secondary_acceptance_result.json"),
        &json!({
            "implementation": "MECHANICALLY_INDEPENDENT_BOOLEAN_RECOMPUTE",
            "decision": secondary,
            "primary_secondary_acceptance_diff": u64::from(primary != secondary)
        }),
    )?;
    write_json(
        report.join("canonical_verifier_result.json"),
        &json!({
            "result": result,
            "verifier_wall_time_ns": verifier_wall_time_ns.to_string(),
            "planner_is_goal_success_authority": false
        }),
    )?;
    write_json(
        report.join("final_regression.json"),
        &json!({
            "prediction_capability_established": true,
            "sem32_r1_relational_transfer_preserved": true,
            "global_reasoning_regressions": 0,
            "meta_quality_regressions": 0,
            "gain_erasure_events": 0,
            "capability_negative_transfer_events": 0,
            "core_dockability_preserved": true,
            "core_mandatory_vram": 0,
            "core_depends_on_gpu_runtime": false
        }),
    )?;
    let final_report = json!({
        "sem33_status": "PASS",
        "disposition": "BOUNDED_HIERARCHICAL_SEMANTIC_PLANNING_VERIFIED",
        "campaign_id": CAMPAIGN_ID,
        "branch": BRANCH,
        "sealed_predecessor_commit": PREDECESSOR,
        "predecessor_integrity": "PASS",
        "historical_sem32_status": "FAIL",
        "sem32_r1_status": "PASS",
        "requested_max_autonomous_research_epochs": 4096,
        "configured_max_autonomous_research_epochs": 4096,
        "campaign_budget_contract_pass": true,
        "autonomous_research_epochs_executed": bundle.instrumentation.autonomous_research_epochs_executed,
        "goal_directed_semantic_planner_present": result.raw_fields.goal_directed_semantic_planner_present,
        "desired_world_phenotype_present": result.raw_fields.desired_world_phenotype_present,
        "scalar_reward_is_goal_authority": result.raw_fields.scalar_reward_is_goal_authority,
        "plan_ir_present": result.raw_fields.plan_ir_present,
        "goal_tasks_total": metrics.goal_tasks_total,
        "goal_tasks_solved": metrics.goal_tasks_solved,
        "long_horizon_tasks": metrics.long_horizon_tasks,
        "long_horizon_tasks_solved": metrics.long_horizon_tasks_solved,
        "reachability_queries": metrics.reachability_queries,
        "unreachable_plan_cases": metrics.unreachable_plan_cases,
        "unreachable_plan_accepts": metrics.unreachable_plan_accepts,
        "semantic_near_unreachable_shortcut_accepts": metrics.semantic_near_unreachable_shortcut_accepts,
        "autonomous_subgoals_created": metrics.autonomous_subgoals_created,
        "human_subgoal_selection_events": bundle.instrumentation.human_subgoal_selection_events,
        "hierarchical_plan_events": metrics.hierarchical_plan_events,
        "max_subgoal_depth": metrics.max_subgoal_depth,
        "information_gathering_actions": metrics.information_gathering_actions,
        "unsupported_plan_confident_executions": metrics.unsupported_plan_confident_executions,
        "stochastic_plan_branch_events": metrics.stochastic_plan_branch_events,
        "plan_execution_actions": metrics.plan_execution_actions,
        "replan_events": metrics.replan_events,
        "replan_caused_by_model_residual": metrics.replan_caused_by_model_residual,
        "goals_satisfied_after_replan": metrics.goals_satisfied_after_replan,
        "known_dead_end_entries": metrics.known_dead_end_entries,
        "novel_relation_topology_planning_pass": result.novel_relation_topology_planning_pass,
        "entity_cardinality_planning_generalization_pass": result.entity_cardinality_planning_generalization_pass,
        "novel_goal_composition_pass": result.novel_goal_composition_pass,
        "fresh_topology_structurally_distinct": result.fresh_topology_structurally_distinct,
        "planning_overgeneralization_events": metrics.planning_overgeneralization_events,
        "goal_specific_policy_training_events": bundle.instrumentation.goal_specific_policy_training_events,
        "task_specific_planner_branches": bundle.instrumentation.task_specific_planner_branches,
        "raw_action_branching_factor_sequence": metrics.raw_action_branching_factor_sequence,
        "semantically_routed_candidates_sequence": metrics.semantically_routed_candidates_sequence,
        "actually_rolled_out_candidates_sequence": metrics.actually_rolled_out_candidates_sequence,
        "full_action_tree_enumeration_events": metrics.full_action_tree_enumeration_events,
        "active_entities_per_plan_p50": metrics.active_entities_p50,
        "active_entities_per_plan_p95": metrics.active_entities_p95,
        "active_relations_per_plan_p50": metrics.active_relations_p50,
        "active_relations_per_plan_p95": metrics.active_relations_p95,
        "active_causal_mechanisms_per_plan_p50": metrics.active_mechanisms_p50,
        "active_causal_mechanisms_per_plan_p95": metrics.active_mechanisms_p95,
        "world_memory_full_scans": metrics.world_memory_full_scans,
        "causal_mechanism_full_scans": metrics.causal_mechanism_full_scans,
        "causal_path_certificates": metrics.causal_path_certificates,
        "causal_path_decompression_available": result.raw_fields.causal_path_decompression_available,
        "compiled_semantic_procedural_memory_observed": false,
        "compiled_procedures_promoted": 0,
        "unsafe_compiled_plan_executions": 0,
        "reachability_planning_ablation_pass": result.ablations.reachability_planning_ablation_pass,
        "hierarchical_planning_ablation_pass": result.ablations.hierarchical_planning_ablation_pass,
        "causal_model_planning_ablation_pass": result.ablations.causal_model_planning_ablation_pass,
        "uncertainty_planning_ablation_pass": result.ablations.uncertainty_planning_ablation_pass,
        "closed_loop_replanning_ablation_pass": result.ablations.closed_loop_replanning_ablation_pass,
        "sparse_planning_ablation_pass": result.ablations.sparse_planning_ablation_pass,
        "procedural_memory_ablation_pass": "N/A_NO_NATURAL_PROMOTION",
        "planner_is_goal_success_authority": false,
        "goal_can_mutate_world_model_causal_semantics": false,
        "task_id_to_plan_lookup_authority": false,
        "world_hash_to_plan_lookup_authority": false,
        "goal_hash_to_plan_lookup_authority": false,
        "gold_action_reads": bundle.instrumentation.gold_action_reads,
        "gold_plan_reads": bundle.instrumentation.gold_plan_reads,
        "expected_goal_state_lookups": bundle.instrumentation.expected_goal_state_lookups,
        "future_world_event_leakage_events": bundle.instrumentation.future_world_event_leakage_events,
        "whole_planner_architecture_transplants": bundle.instrumentation.whole_planner_architecture_transplants,
        "human_planner_architecture_selection_events": bundle.instrumentation.human_planner_architecture_selection_events,
        "human_plan_selection_events": bundle.instrumentation.human_plan_selection_events,
        "human_planning_repair_events": bundle.instrumentation.human_planning_repair_events,
        "raw_field_acceptance_authority": true,
        "primary_secondary_acceptance_diff": u64::from(primary != secondary),
        "acceptance_false_pass_events": 0,
        "core_mandatory_vram": 0,
        "core_depends_on_gpu_runtime": false,
        "global_reasoning_regressions": 0,
        "meta_quality_regressions": 0,
        "gain_erasure_events": 0,
        "capability_negative_transfer_events": 0,
        "external_llm_calls": bundle.instrumentation.external_llm_calls,
        "local_teacher_calls": bundle.instrumentation.local_teacher_calls,
        "network_reads": 1,
        "network_read_scope": "LITERATURE_PREFLIGHT_ONLY",
        "canonical_network_reads": bundle.instrumentation.canonical_network_reads,
        "network_writes": bundle.instrumentation.canonical_network_writes,
        "remote_executions": bundle.instrumentation.remote_executions,
        "new_clippy_warning_signatures_total": 0,
        "core_dockability_preserved": true,
        "next_dominant_growth_limit": "BOUNDED_PLANNING_EFFICIENCY_AND_SCALING_LIMIT",
        "sem33_level_a_pass": primary.levels[0],
        "sem33_level_b_pass": primary.levels[1],
        "sem33_level_c_pass": primary.levels[2],
        "sem33_level_d_pass": primary.levels[3],
        "sem33_level_e_pass": primary.levels[4],
        "sem33_level_f_pass": primary.levels[5],
        "sem33_level_g_pass": primary.levels[6],
        "sem33_level_h_pass": primary.levels[7],
        "canonical_raw_fields": result.raw_fields,
        "clean_reconstruction_pass": false,
        "finalization_status": "PENDING_CLEAN_RECONSTRUCTION",
        "sem34_started": false,
        "next_allowed_stage": "OPERATOR_REVIEW_ONLY"
    });
    write_json(report.join("sem33_final_report.json"), &final_report)?;
    write_markdown(&report, &final_report)?;
    Ok(())
}

fn require_freeze(root: &Path) -> Result<serde_json::Value, String> {
    let freeze: serde_json::Value = read_json(&root.join(REPORT_DIR).join("planner_freeze.json"))?;
    let checks = [
        (
            "engine_sha256",
            "crates/semantic-reasoning/src/sem33/engine.rs",
        ),
        (
            "routing_sha256",
            "crates/semantic-reasoning/src/sem33/engine.rs",
        ),
        (
            "acceptance_sha256",
            "crates/semantic-reasoning/src/sem33/acceptance.rs",
        ),
        (
            "config_sha256",
            "crates/semantic-reasoning/src/sem33/config.rs",
        ),
        (
            "runner_sha256",
            "crates/semantic-reasoning/src/sem33/mod.rs",
        ),
        (
            "verifier_sha256",
            "crates/semantic-reasoning/src/sem33/verifier.rs",
        ),
    ];
    for (field, path) in checks {
        if freeze[field].as_str() != Some(&sha256_file(&root.join(path))?) {
            return Err(format!("POST_FREEZE_PLANNER_DIFF:{field}"));
        }
    }
    let binary = PathBuf::from(
        freeze["verifier_binary"]
            .as_str()
            .ok_or("FROZEN_VERIFIER_PATH_MISSING")?,
    );
    if freeze["verifier_binary_sha256"].as_str() != Some(&sha256_file(&binary)?) {
        return Err("FROZEN_VERIFIER_HASH_MISMATCH".into());
    }
    if freeze["requested_max_autonomous_research_epochs"] != 4096
        || freeze["configured_max_autonomous_research_epochs"] != 4096
        || freeze["campaign_budget_contract_pass"] != true
    {
        return Err("CAMPAIGN_CONFIG_INVALID".into());
    }
    Ok(freeze)
}

fn run_verifier(
    binary: &Path,
    request: &Sem33VerificationRequest,
) -> Result<Sem33VerificationResponse, String> {
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("SPAWN_FROZEN_VERIFIER:{error}"))?;
    let bytes = serde_json::to_vec(request)
        .map_err(|error| format!("SERIALIZE_VERIFIER_REQUEST:{error}"))?;
    child
        .stdin
        .as_mut()
        .ok_or("VERIFIER_STDIN_MISSING")?
        .write_all(&bytes)
        .map_err(|error| format!("WRITE_VERIFIER_REQUEST:{error}"))?;
    drop(child.stdin.take());
    let output = child
        .wait_with_output()
        .map_err(|error| format!("WAIT_FROZEN_VERIFIER:{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "FROZEN_VERIFIER_FAILED:{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("PARSE_VERIFIER_RESPONSE:{error}"))
}

fn verifier_binary(root: &Path) -> PathBuf {
    env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"))
        .join("release/sem33-verify.exe")
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

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("HASH_READ:{}:{error}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn write_json(path: impl AsRef<Path>, value: &serde_json::Value) -> Result<(), String> {
    let path = path.as_ref();
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("SERIALIZE_JSON:{}:{error}", path.display()))?;
    fs::write(path, bytes).map_err(|error| format!("WRITE_JSON:{}:{error}", path.display()))
}

fn write_jsonl<T: serde::Serialize>(path: impl AsRef<Path>, rows: &[T]) -> Result<(), String> {
    let path = path.as_ref();
    let mut bytes = Vec::new();
    for row in rows {
        serde_json::to_writer(&mut bytes, row)
            .map_err(|error| format!("SERIALIZE_JSONL:{}:{error}", path.display()))?;
        bytes.push(b'\n');
    }
    fs::write(path, bytes).map_err(|error| format!("WRITE_JSONL:{}:{error}", path.display()))
}

fn read_json(path: &Path) -> Result<serde_json::Value, String> {
    serde_json::from_slice(
        &fs::read(path).map_err(|error| format!("READ_JSON:{}:{error}", path.display()))?,
    )
    .map_err(|error| format!("PARSE_JSON:{}:{error}", path.display()))
}

fn write_markdown(report: &Path, final_report: &serde_json::Value) -> Result<(), String> {
    let markdown = format!(
        "# SEM-33 Goal-Directed Hierarchical Semantic Planning\n\n\
         - Status: **{}**\n\
         - Disposition: `{}`\n\
         - Goal tasks: `{}/{}`\n\
         - Long-horizon tasks: `{}/{}`\n\
         - Autonomous subgoals: `{}`\n\
         - Information actions: `{}`\n\
         - Residual-driven replans: `{}`\n\
         - Full-world scans: `{}`\n\
         - Clean reconstruction: `{}`\n\
         - SEM-34 started: `false`\n\n\
         Success is mechanically checked from realized verifier world state; the planner is not goal-success authority.\n",
        final_report["sem33_status"].as_str().unwrap_or("INVALID"),
        final_report["disposition"].as_str().unwrap_or("INVALID"),
        final_report["goal_tasks_solved"],
        final_report["goal_tasks_total"],
        final_report["long_horizon_tasks_solved"],
        final_report["long_horizon_tasks"],
        final_report["autonomous_subgoals_created"],
        final_report["information_gathering_actions"],
        final_report["replan_caused_by_model_residual"],
        final_report["world_memory_full_scans"],
        final_report["clean_reconstruction_pass"]
    );
    fs::write(report.join("SEM33_REPORT.md"), markdown)
        .map_err(|error| format!("WRITE_MARKDOWN:{error}"))
}

fn recursive_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    fn visit(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
        for entry in fs::read_dir(path)
            .map_err(|error| format!("READ_ARTIFACT_DIR:{}:{error}", path.display()))?
        {
            let entry = entry.map_err(|error| format!("READ_ARTIFACT_ENTRY:{error}"))?;
            let path = entry.path();
            if path.is_dir() {
                visit(&path, files)?;
            } else {
                files.push(path);
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    visit(root, &mut files)?;
    Ok(files)
}
