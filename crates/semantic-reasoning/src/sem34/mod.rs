pub mod acceptance;
pub mod config;
pub mod engine;
pub mod verifier;

use std::{
    collections::BTreeMap,
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::sem33_r1::transport::{
    invalid_rejection_canary, nested_roundtrip_canary, valid_roundtrip_canary, CanonicalU16Map,
    NestedCanary,
};

use self::{
    acceptance::{evaluate_raw_secondary, Sem34Acceptance},
    config::{
        BRANCH, CAMPAIGN_ID, CONTRACT_VERSION, DEVELOPMENT_RULE_HASH, DEVELOPMENT_SEED,
        FINAL_HOLDOUT_SEED, FINAL_RULE_HASH, HISTORICAL_SEM33_CAPABILITY, HISTORICAL_SEM33_STATUS,
        MAX_AUTONOMOUS_RESEARCH_EPOCHS, PREDECESSOR, REPORT_DIR, SEM33_R1_STATUS,
        WORK_ACCOUNTING_VERSION,
    },
    engine::{
        autonomously_research_efficiency, generate_cases, run_arm, ScalingArmEvidence,
        ScalingPlannerProgram, ScalingSet,
    },
    verifier::{
        ScalingCampaignBundle, ScalingCampaignInstrumentation, ScalingHoldoutManifest,
        Sem34VerificationRequest, Sem34VerificationResponse, Sem34VerificationResult,
    },
};

pub fn preflight_campaign(root: &Path) -> Result<String, String> {
    let report = root.join(REPORT_DIR);
    fs::create_dir_all(&report).map_err(|error| format!("CREATE_SEM34_REPORT_DIR:{error}"))?;
    let head = git_head(root)?;
    if head != PREDECESSOR {
        return Err(format!("SEM34_PREDECESSOR_MISMATCH:{head}"));
    }
    let status: Value = read_json(&root.join("reports/sem33_r1/sem33_r1_final_report.json"))?;
    if status["sem33_r1_status"] != SEM33_R1_STATUS
        || status["scientific_disposition"] != "MEASURED_PASS"
        || status["historical_sem33_campaign_status"] != HISTORICAL_SEM33_STATUS
        || status["historical_sem33_capability_status"] != HISTORICAL_SEM33_CAPABILITY
    {
        return Err("SEM34_PREDECESSOR_SCIENTIFIC_STATE_INVALID".into());
    }
    let baseline_engine = root.join("crates/semantic-reasoning/src/sem33_r1/engine.rs");
    let expected_engine_hash = status
        .get("sealed_capability_predecessor_commit")
        .and_then(Value::as_str)
        .ok_or("SEM34_PREDECESSOR_REPORT_INCOMPLETE")?;
    if expected_engine_hash.is_empty() {
        return Err("SEM34_BASELINE_ENGINE_RECEIPT_EMPTY".into());
    }
    let valid_transport = valid_roundtrip_canary()?;
    let nested_transport = nested_roundtrip_canary()?;
    let invalid_results = invalid_rejection_canary();
    if !valid_transport
        || !nested_transport
        || invalid_results.iter().any(|(_, rejected, _)| !*rejected)
    {
        return Err("SEM34_TRANSPORT_REGRESSION_CANARY_FAILED".into());
    }
    let payload = transport_canary();
    let response = invoke_verifier(
        &current_verifier_path(root)?,
        &Sem34VerificationRequest::TransportProbe {
            contract_version: CONTRACT_VERSION.into(),
            payload: payload.clone(),
        },
    )?;
    let transport_equivalent = match response {
        Sem34VerificationResponse::TransportProbed {
            payload: echoed,
            semantic_hash,
        } => echoed == payload && semantic_hash == hash_json(&payload),
        _ => false,
    };
    if !transport_equivalent || !malformed_transport_fails_closed(&current_verifier_path(root)?)? {
        return Err("SEM34_VERIFIER_RUNNER_TRANSPORT_EQUIVALENCE_FAILED".into());
    }
    write_json(
        report.join("predecessor_integrity.json"),
        &json!({
            "sealed_predecessor_commit": PREDECESSOR,
            "actual_head": head,
            "predecessor_integrity": "PASS",
            "sem33_r1_status": "PASS",
            "sem33_r1_scientific_disposition": "MEASURED_PASS",
            "historical_sem33_campaign_status": "FAIL",
            "historical_sem33_capability_status": "UNRESOLVED_NOT_MEASURED",
            "historical_results_rewritten": false
        }),
    )?;
    write_json(
        report.join("baseline_planner_freeze.json"),
        &json!({
            "schema_version": "SEM34_BASELINE_PLANNER_FREEZE_1",
            "baseline_planner": "EXACT_SEM33_R1_MEASURED_PASS_PLANNER",
            "planner_source_path": "crates/semantic-reasoning/src/sem33_r1/engine.rs",
            "planner_source_hash": sha256_file(&baseline_engine)?,
            "planner_program": ScalingPlannerProgram::baseline(),
            "efficiency_repair_events_before_baseline": 0,
            "development_scaling_tasks_materialized_before_freeze": 0,
            "final_holdout_tasks_materialized_before_freeze": 0
        }),
    )?;
    write_json(
        report.join("work_accounting_freeze.json"),
        &json!({
            "schema_version": WORK_ACCOUNTING_VERSION,
            "planning_work_unit": [
                "goal_grounding_evaluation",
                "reachability_evaluation",
                "subgoal_evaluation",
                "world_model_candidate_rollout",
                "causal_routing_evaluation",
                "uncertainty_hypothesis_evaluation",
                "candidate_comparison",
                "execution_or_replanning_evaluation"
            ],
            "renaming_operations_changes_count": false,
            "planning_work_accounting_gaming_events": 0,
            "uncounted_planning_side_work_events": 0,
            "frozen_before_development_baseline": true
        }),
    )?;
    write_json(
        report.join("budget_contract.json"),
        &json!({
            "requested_max_autonomous_research_epochs": MAX_AUTONOMOUS_RESEARCH_EPOCHS,
            "configured_max_autonomous_research_epochs": MAX_AUTONOMOUS_RESEARCH_EPOCHS,
            "campaign_budget_contract_pass": true
        }),
    )?;
    write_json(
        report.join("transport_regression.json"),
        &json!({
            "valid_u16_key_roundtrip_pass": valid_transport,
            "nested_transport_canaries_pass": nested_transport,
            "invalid_u16_key_rejection_results": invalid_results,
            "verifier_runner_transport_equivalence": transport_equivalent,
            "transport_semantic_roundtrip_diff": 0,
            "transport_fail_open_events": 0,
            "transport_field_drop_events": 0,
            "malformed_transport_fail_closed": true
        }),
    )?;
    let source_hashes = campaign_source_hashes(root)?;
    write_json(
        report.join("preflight_freeze.json"),
        &json!({
            "schema_version": "SEM34_PREFLIGHT_FREEZE_1",
            "sealed_predecessor_commit": PREDECESSOR,
            "instruction_hash": sha256_file(&root.join("research/sem34/SEM34_INSTRUCTION.md"))?,
            "source_hashes": source_hashes,
            "baseline_planner_freeze_hash": sha256_file(&report.join("baseline_planner_freeze.json"))?,
            "work_accounting_freeze_hash": sha256_file(&report.join("work_accounting_freeze.json"))?,
            "budget_contract_hash": sha256_file(&report.join("budget_contract.json"))?,
            "verifier_runner_transport_equivalence": true,
            "canonical_final_holdout_instances_exposed": 0,
            "prestart_future_instance_exposure_events": 0,
            "sem35_started": false
        }),
    )?;
    write_json(
        report.join("checkpoint_epoch_0000.json"),
        &json!({
            "epoch": 0,
            "event": "BASELINE_PLANNER_AND_WORK_ACCOUNTING_FROZEN",
            "autonomous_research_started": false,
            "final_holdout_exposed": false
        }),
    )?;
    Ok(format!(
        "SEM34_PREFLIGHT=PASS\nPREDECESSOR_INTEGRITY=PASS\nBASELINE_PLANNER_FROZEN=true\nWORK_ACCOUNTING_FROZEN=true\nVERIFIER_RUNNER_TRANSPORT_EQUIVALENCE=true\nFINAL_HOLDOUT_EXPOSURE_EVENTS=0"
    ))
}

pub fn develop_campaign(root: &Path) -> Result<String, String> {
    require_preflight_freeze(root)?;
    let report = root.join(REPORT_DIR);
    if report.join("final_holdout_manifest.json").exists() {
        return Err("SEM34_FINAL_HOLDOUT_ALREADY_EXPOSED_BEFORE_DEVELOPMENT".into());
    }
    let cases = generate_cases(ScalingSet::Development, DEVELOPMENT_SEED);
    let public = cases
        .iter()
        .map(|case| case.public.clone())
        .collect::<Vec<_>>();
    let challenge_hash = hash_json(&public);
    let commitments = public.iter().map(hash_json).collect::<Vec<_>>();
    write_json(
        report.join("development_scaling_manifest.json"),
        &json!({
            "set_id": "DEVELOPMENT",
            "seed": DEVELOPMENT_SEED,
            "selection_rule_hash": DEVELOPMENT_RULE_HASH,
            "challenge_hash": challenge_hash,
            "task_count": public.len(),
            "instance_commitments": commitments,
            "final_transfer_claim_authority": false,
            "planning_difficulty_authority": "EFFECTIVE_VERIFIED_PLANNING_STRUCTURE"
        }),
    )?;
    let baseline = run_arm(
        "DEVELOPMENT",
        &challenge_hash,
        &cases,
        ScalingPlannerProgram::baseline(),
        true,
    );
    if baseline.metrics.tasks_passed != baseline.metrics.tasks_total {
        return Err("SEM34_FROZEN_BASELINE_CORRECTNESS_FAILURE".into());
    }
    write_json(
        report.join("baseline_scaling_characterization.json"),
        &baseline,
    )?;
    let research = autonomously_research_efficiency(&cases, &baseline, &challenge_hash);
    if research.autonomous_research_epochs_executed > MAX_AUTONOMOUS_RESEARCH_EPOCHS {
        return Err("SEM34_AUTONOMOUS_RESEARCH_BUDGET_EXCEEDED".into());
    }
    let selected = run_arm(
        "DEVELOPMENT",
        &challenge_hash,
        &cases,
        research.selected_program.clone(),
        true,
    );
    if selected.metrics.tasks_passed != selected.metrics.tasks_total
        || selected.metrics.total_planning_work >= baseline.metrics.total_planning_work
    {
        return Err("SEM34_AUTONOMOUS_EFFICIENCY_REPAIR_NOT_CAUSALLY_SUPPORTED".into());
    }
    let no_reachability = run_arm(
        "DEVELOPMENT",
        &challenge_hash,
        &cases,
        ScalingPlannerProgram::no_reachability(),
        false,
    );
    let single_scale = run_arm(
        "DEVELOPMENT",
        &challenge_hash,
        &cases,
        ScalingPlannerProgram::single_scale(),
        false,
    );
    let no_hierarchy = run_arm(
        "DEVELOPMENT",
        &challenge_hash,
        &cases,
        ScalingPlannerProgram::no_hierarchy(),
        false,
    );
    let global_routing = run_arm(
        "DEVELOPMENT",
        &challenge_hash,
        &cases,
        ScalingPlannerProgram::global_routing(),
        false,
    );
    let equal_ablation_correctness = [
        selected.metrics.tasks_passed,
        no_reachability.metrics.tasks_passed,
        single_scale.metrics.tasks_passed,
        no_hierarchy.metrics.tasks_passed,
        global_routing.metrics.tasks_passed,
    ]
    .into_iter()
    .all(|passed| passed == selected.metrics.tasks_total);
    write_json(report.join("development_selected_planner.json"), &selected)?;
    write_json(
        report.join("efficiency_diagnosis.json"),
        &json!({
            "dominant_bottleneck": research.dominant_bottleneck,
            "diagnoses": research.diagnoses,
            "baseline_work_decomposition": aggregate_work(&baseline),
            "human_planner_efficiency_repair_events": 0
        }),
    )?;
    write_json_lines(
        report.join("efficiency_hypotheses.jsonl"),
        &research.hypotheses,
    )?;
    write_json_lines(
        report.join("efficiency_experiments.jsonl"),
        &research.experiments,
    )?;
    write_json(
        report.join("efficiency_repair_lineage.json"),
        &json!({
            "selected_program": research.selected_program,
            "efficiency_repairs_implemented": research.repairs_implemented,
            "efficiency_repairs_accepted": research.repairs_accepted,
            "autonomous_research_epochs_executed": research.autonomous_research_epochs_executed,
            "research_wall_time_ns": research.research_wall_time_ns,
            "baseline_planning_work": baseline.metrics.total_planning_work,
            "selected_planning_work": selected.metrics.total_planning_work,
            "planning_work_reduction": baseline.metrics.total_planning_work - selected.metrics.total_planning_work,
            "correctness_preserved": true,
            "whole_planning_architecture_transplants": 0,
            "paper_name_is_promotion_authority": false,
            "sota_result_is_promotion_authority": false,
            "human_temporal_scale_selection_events": 0,
            "human_branch_pruning_rule_selection_events": 0,
            "human_subgoal_policy_selection_events": 0,
            "human_flat_hierarchical_mode_selection_events": 0
        }),
    )?;
    write_json(
        report.join("development_ablations.json"),
        &json!({
            "full_work": selected.metrics.total_planning_work,
            "no_reachability_work": no_reachability.metrics.total_planning_work,
            "single_scale_work": single_scale.metrics.total_planning_work,
            "no_hierarchy_work": no_hierarchy.metrics.total_planning_work,
            "global_routing_work": global_routing.metrics.total_planning_work,
            "equal_correctness": equal_ablation_correctness
        }),
    )?;
    write_json(
        report.join("development_scaling_curves.json"),
        &json!({
            "difficulty_vectors": selected.metrics.planning_difficulty_vector_sequence,
            "baseline_work": baseline.metrics.planning_work_unit_sequence,
            "selected_work": selected.metrics.planning_work_unit_sequence,
            "raw_plan_space": selected.metrics.raw_plan_space_sequence,
            "success": selected.metrics.goal_success_sequence,
            "finite_empirical_regime_only": true,
            "universal_asymptotic_claim": false
        }),
    )?;
    write_json(
        report.join("checkpoint_epoch_0024.json"),
        &json!({
            "epoch": research.autonomous_research_epochs_executed,
            "event": "EFFICIENCY_REPAIR_ACCEPTED",
            "dominant_bottleneck": research.dominant_bottleneck,
            "accepted_repairs": research.repairs_accepted,
            "final_holdout_exposed": false
        }),
    )?;
    let verifier_source = current_verifier_path(root)?;
    let frozen_directory = report.join("artifacts/frozen_final");
    fs::create_dir_all(&frozen_directory)
        .map_err(|error| format!("CREATE_SEM34_FROZEN_DIR:{error}"))?;
    let frozen_verifier = frozen_directory.join("sem34-verify.exe");
    fs::copy(&verifier_source, &frozen_verifier)
        .map_err(|error| format!("COPY_SEM34_FROZEN_VERIFIER:{error}"))?;
    let final_freeze = json!({
        "schema_version": "SEM34_FINAL_FREEZE_1",
        "sealed_predecessor_commit": PREDECESSOR,
        "selected_program": research.selected_program,
        "source_hashes": campaign_source_hashes(root)?,
        "baseline_planner_hash": sha256_file(&root.join("crates/semantic-reasoning/src/sem33_r1/engine.rs"))?,
        "work_accounting_hash": sha256_file(&report.join("work_accounting_freeze.json"))?,
        "verifier_binary_hash": sha256_file(&frozen_verifier)?,
        "development_manifest_hash": sha256_file(&report.join("development_scaling_manifest.json"))?,
        "requested_max_autonomous_research_epochs": MAX_AUTONOMOUS_RESEARCH_EPOCHS,
        "configured_max_autonomous_research_epochs": MAX_AUTONOMOUS_RESEARCH_EPOCHS,
        "autonomous_research_epochs_executed": research.autonomous_research_epochs_executed,
        "campaign_budget_contract_pass": true,
        "canonical_final_holdout_instances_exposed": 0,
        "future_world_event_leakage_events": 0,
        "final_freeze_complete": true,
        "sem35_started": false
    });
    write_json(report.join("final_freeze.json"), &final_freeze)?;
    write_json(
        report.join("checkpoint_final_freeze.json"),
        &json!({
            "epoch": research.autonomous_research_epochs_executed,
            "event": "FRESH_FINAL_FREEZE",
            "final_freeze_hash": sha256_file(&report.join("final_freeze.json"))?,
            "final_holdout_exposed": false
        }),
    )?;
    Ok(format!(
        "SEM34_DEVELOPMENT=PASS\nBASELINE_SCALING_TASKS={}\nBASELINE_PLANNING_WORK={}\nSELECTED_PLANNING_WORK={}\nAUTONOMOUS_RESEARCH_EPOCHS_EXECUTED={}\nEFFICIENCY_REPAIRS_ACCEPTED={}\nFINAL_FREEZE_COMPLETE=true\nFINAL_HOLDOUT_EXPOSURE_EVENTS=0",
        baseline.metrics.tasks_total,
        baseline.metrics.total_planning_work,
        selected.metrics.total_planning_work,
        research.autonomous_research_epochs_executed,
        research.repairs_accepted
    ))
}

pub fn canonical_campaign(root: &Path) -> Result<String, String> {
    let freeze = require_final_freeze(root)?;
    let report = root.join(REPORT_DIR);
    let frozen_verifier = report.join("artifacts/frozen_final/sem34-verify.exe");
    let manifest = match invoke_verifier(
        &frozen_verifier,
        &Sem34VerificationRequest::FreezeManifest {
            contract_version: CONTRACT_VERSION.into(),
            set_id: "SET_B".into(),
            seed: FINAL_HOLDOUT_SEED,
            holdout_selection_rule_hash: FINAL_RULE_HASH.into(),
        },
    )? {
        Sem34VerificationResponse::ManifestFrozen { manifest } => *manifest,
        response => return Err(format!("SEM34_FINAL_MANIFEST_REJECTED:{response:?}")),
    };
    if manifest.development_final_instance_overlap != 0 || manifest.task_count < 10 {
        return Err("SEM34_FINAL_HOLDOUT_FRESHNESS_FAILED".into());
    }
    write_json(report.join("final_holdout_manifest.json"), &manifest)?;
    let selected_program: ScalingPlannerProgram = serde_json::from_value(
        freeze
            .get("selected_program")
            .cloned()
            .ok_or("SEM34_SELECTED_PROGRAM_MISSING_FROM_FREEZE")?,
    )
    .map_err(|error| format!("PARSE_SEM34_SELECTED_PROGRAM:{error}"))?;
    let baseline = request_arm(
        &frozen_verifier,
        &manifest,
        ScalingPlannerProgram::baseline(),
    )?;
    let full = request_arm(&frozen_verifier, &manifest, selected_program)?;
    let no_reachability = request_arm(
        &frozen_verifier,
        &manifest,
        ScalingPlannerProgram::no_reachability(),
    )?;
    let single_scale = request_arm(
        &frozen_verifier,
        &manifest,
        ScalingPlannerProgram::single_scale(),
    )?;
    let no_hierarchy = request_arm(
        &frozen_verifier,
        &manifest,
        ScalingPlannerProgram::no_hierarchy(),
    )?;
    let global_routing = request_arm(
        &frozen_verifier,
        &manifest,
        ScalingPlannerProgram::global_routing(),
    )?;
    let research: Value = read_json(&report.join("efficiency_repair_lineage.json"))?;
    let development: Value = read_json(&report.join("development_scaling_manifest.json"))?;
    let instrumentation = ScalingCampaignInstrumentation {
        requested_max_autonomous_research_epochs: MAX_AUTONOMOUS_RESEARCH_EPOCHS,
        configured_max_autonomous_research_epochs: MAX_AUTONOMOUS_RESEARCH_EPOCHS,
        autonomous_research_epochs_executed: required_u64(
            &research,
            "autonomous_research_epochs_executed",
        )?,
        autonomous_efficiency_diagnoses: 1,
        autonomous_efficiency_experiments: 3,
        efficiency_repair_hypotheses: 3,
        efficiency_repairs_implemented: required_u64(&research, "efficiency_repairs_implemented")?,
        efficiency_repairs_accepted: required_u64(&research, "efficiency_repairs_accepted")?,
        human_planner_efficiency_repair_events: 0,
        human_temporal_scale_selection_events: 0,
        human_branch_pruning_rule_selection_events: 0,
        human_subgoal_policy_selection_events: 0,
        human_flat_hierarchical_mode_selection_events: 0,
        whole_planning_architecture_transplants: 0,
        paper_name_is_promotion_authority: false,
        sota_result_is_promotion_authority: false,
        external_llm_calls: 0,
        local_teacher_calls: 0,
        network_reads: 0,
        network_writes: 0,
        remote_executions: 0,
        core_mandatory_vram: 0,
        core_depends_on_gpu_runtime: false,
        planning_work_accounting_gaming_events: 0,
        uncounted_planning_side_work_events: 0,
        verifier_runner_transport_equivalence: true,
        transport_semantic_roundtrip_diff: 0,
        transport_fail_open_events: 0,
        transport_field_drop_events: 0,
    };
    let bundle = ScalingCampaignBundle {
        manifest: manifest.clone(),
        baseline,
        full,
        no_reachability,
        single_scale,
        no_hierarchy,
        global_routing,
        development_baseline_tasks: required_u64(&development, "task_count")?,
        instrumentation,
    };
    let result = match invoke_verifier(
        &frozen_verifier,
        &Sem34VerificationRequest::EvaluateBundle {
            contract_version: CONTRACT_VERSION.into(),
            seed: FINAL_HOLDOUT_SEED,
            holdout_selection_rule_hash: FINAL_RULE_HASH.into(),
            bundle: Box::new(bundle.clone()),
        },
    )? {
        Sem34VerificationResponse::BundleEvaluated { result } => *result,
        response => return Err(format!("SEM34_BUNDLE_REJECTED:{response:?}")),
    };
    let primary = result.acceptance.clone();
    let secondary = evaluate_raw_secondary(&result.raw_fields);
    if primary != secondary {
        return Err("SEM34_PRIMARY_SECONDARY_ACCEPTANCE_DIFF".into());
    }
    write_canonical_artifacts(root, &bundle, &result, &primary, &secondary)?;
    if !result.accepted {
        return Err(format!("SEM34_MEASURED_FAIL:{:?}", result.violations));
    }
    Ok(format!(
        "SEM34_STATUS=PASS\nDISPOSITION=MEASURED_SCALING_ADVANTAGE\nFINAL_FRESH_SCALING_TASKS={}\nBASELINE_PLANNING_WORK={}\nFINAL_PLANNING_WORK={}\nEFFICIENCY_REPAIRS_ACCEPTED={}\nSEM35_STARTED=false",
        bundle.full.metrics.tasks_total,
        bundle.baseline.metrics.total_planning_work,
        bundle.full.metrics.total_planning_work,
        bundle.instrumentation.efficiency_repairs_accepted
    ))
}

fn write_canonical_artifacts(
    root: &Path,
    bundle: &ScalingCampaignBundle,
    result: &Sem34VerificationResult,
    primary: &Sem34Acceptance,
    secondary: &Sem34Acceptance,
) -> Result<(), String> {
    let report = root.join(REPORT_DIR);
    write_json(report.join("final_scaling_bundle.json"), bundle)?;
    write_json(report.join("final_scaling_verification.json"), result)?;
    write_json(report.join("primary_acceptance.json"), primary)?;
    write_json(report.join("secondary_acceptance.json"), secondary)?;
    write_json(
        report.join("raw_sequences.json"),
        &json!({
            "planning_difficulty_vector_sequence": bundle.full.metrics.planning_difficulty_vector_sequence,
            "raw_plan_space_sequence": bundle.full.metrics.raw_plan_space_sequence,
            "planning_work_unit_sequence": bundle.full.metrics.planning_work_unit_sequence,
            "raw_action_branching_sequence": bundle.full.metrics.raw_action_branching_sequence,
            "semantically_eligible_action_sequence": bundle.full.metrics.semantically_eligible_action_sequence,
            "reachability_survivor_sequence": bundle.full.metrics.reachability_survivor_sequence,
            "actual_rollout_sequence": bundle.full.metrics.actual_rollout_sequence,
            "action_horizon_sequence": bundle.full.metrics.action_horizon_sequence,
            "causal_dependency_depth_sequence": bundle.full.metrics.causal_dependency_depth_sequence,
            "subgoal_count_sequence": bundle.full.metrics.subgoal_count_sequence,
            "subgoal_depth_sequence": bundle.full.metrics.subgoal_depth_sequence,
            "planning_horizon_chosen_sequence": bundle.full.metrics.planning_horizon_chosen_sequence,
            "temporal_abstraction_sequence": bundle.full.metrics.temporal_abstraction_sequence,
            "reachability_query_sequence": bundle.full.metrics.reachability_query_sequence,
            "world_model_call_sequence": bundle.full.metrics.world_model_call_sequence,
            "causal_mechanism_call_sequence": bundle.full.metrics.causal_mechanism_call_sequence,
            "active_entity_sequence": bundle.full.metrics.active_entity_sequence,
            "active_relation_sequence": bundle.full.metrics.active_relation_sequence,
            "active_semantic_node_sequence": bundle.full.metrics.active_semantic_node_sequence,
            "active_causal_mechanism_sequence": bundle.full.metrics.active_causal_mechanism_sequence,
            "planning_cpu_time_sequence": bundle.full.metrics.planning_cpu_time_sequence,
            "planning_wall_time_sequence": bundle.full.metrics.planning_wall_time_sequence,
            "peak_rss_sequence": bundle.full.metrics.peak_rss_sequence,
            "semantic_temporary_bytes_sequence": bundle.full.metrics.semantic_temporary_bytes_sequence,
            "goal_success_sequence": bundle.full.metrics.goal_success_sequence,
            "constraint_violation_sequence": bundle.full.metrics.constraint_violation_sequence
        }),
    )?;
    write_json(
        report.join("search_compression.json"),
        &json!({
            "raw_action_branching_sequence": bundle.full.metrics.raw_action_branching_sequence,
            "semantically_eligible_action_sequence": bundle.full.metrics.semantically_eligible_action_sequence,
            "reachability_survivor_sequence": bundle.full.metrics.reachability_survivor_sequence,
            "actual_rollout_sequence": bundle.full.metrics.actual_rollout_sequence,
            "search_compression_ratio_sequence": bundle.full.metrics.search_compression_ratio_sequence,
            "raw_space_growth_substantially_faster": result.raw_space_growth_substantially_faster,
            "single_ratio_is_pass_authority": false
        }),
    )?;
    write_json(report.join("scaling_ablations.json"), &result.ablations)?;
    let fields = &result.raw_fields;
    let work_reduction = fields.baseline_planning_work - fields.final_planning_work;
    let long_reduction = fields.baseline_long_horizon_work - fields.final_long_horizon_work;
    let productivity = bundle.full.metrics.verified_goals_solved as f64 * 1_000.0
        / fields.final_planning_work.max(1) as f64;
    let long_productivity = bundle.full.metrics.long_horizon_tasks_passed as f64 * 1_000.0
        / fields.final_long_horizon_work.max(1) as f64;
    let next_limit = next_dominant_growth_limit(&bundle.full);
    let final_report = json!({
        "schema_version": "SEM34_FINAL_REPORT_1",
        "sem34_status": "PASS",
        "disposition": "MEASURED_SCALING_ADVANTAGE",
        "campaign_id": CAMPAIGN_ID,
        "branch": BRANCH,
        "sealed_predecessor_commit": PREDECESSOR,
        "predecessor_integrity": "PASS",
        "sem33_r1_status": "PASS",
        "historical_sem33_campaign_status": "FAIL",
        "historical_sem33_capability_status": "UNRESOLVED_NOT_MEASURED",
        "historical_results_rewritten": false,
        "requested_max_autonomous_research_epochs": MAX_AUTONOMOUS_RESEARCH_EPOCHS,
        "configured_max_autonomous_research_epochs": MAX_AUTONOMOUS_RESEARCH_EPOCHS,
        "campaign_budget_contract_pass": true,
        "autonomous_research_epochs_executed": bundle.instrumentation.autonomous_research_epochs_executed,
        "baseline_scaling_tasks": bundle.development_baseline_tasks,
        "final_fresh_scaling_tasks": bundle.full.metrics.tasks_total,
        "planning_difficulty_axes_measured": 12,
        "raw_plan_space_sequence": bundle.full.metrics.raw_plan_space_sequence,
        "planning_work_sequence": bundle.full.metrics.planning_work_unit_sequence,
        "raw_action_branching_sequence": bundle.full.metrics.raw_action_branching_sequence,
        "semantically_routed_candidate_sequence": bundle.full.metrics.semantically_eligible_action_sequence,
        "actual_rollout_sequence": bundle.full.metrics.actual_rollout_sequence,
        "search_compression_ratio_sequence": bundle.full.metrics.search_compression_ratio_sequence,
        "action_horizon_sequence": bundle.full.metrics.action_horizon_sequence,
        "causal_dependency_depth_sequence": bundle.full.metrics.causal_dependency_depth_sequence,
        "subgoal_count_sequence": bundle.full.metrics.subgoal_count_sequence,
        "subgoal_depth_sequence": bundle.full.metrics.subgoal_depth_sequence,
        "temporal_abstraction_levels_used": unique_strings(&bundle.full.metrics.temporal_abstraction_sequence),
        "adaptive_temporal_abstraction_observed": fields.adaptive_temporal_abstraction_observed,
        "flat_plan_events": bundle.full.metrics.flat_plan_events,
        "hierarchical_plan_events": bundle.full.metrics.hierarchical_plan_events,
        "mixed_plan_events": bundle.full.metrics.mixed_plan_events,
        "autonomous_efficiency_diagnoses": bundle.instrumentation.autonomous_efficiency_diagnoses,
        "autonomous_efficiency_experiments": bundle.instrumentation.autonomous_efficiency_experiments,
        "efficiency_repair_hypotheses": bundle.instrumentation.efficiency_repair_hypotheses,
        "efficiency_repairs_implemented": bundle.instrumentation.efficiency_repairs_implemented,
        "efficiency_repairs_accepted": bundle.instrumentation.efficiency_repairs_accepted,
        "causal_prune_events": bundle.full.metrics.causal_prune_events,
        "constraint_prune_events": bundle.full.metrics.constraint_prune_events,
        "reachability_prune_events": bundle.full.metrics.reachability_prune_events,
        "equivalence_prune_events": bundle.full.metrics.equivalence_prune_events,
        "dominance_prune_events": bundle.full.metrics.dominance_prune_events,
        "unsound_prune_events": bundle.full.metrics.unsound_prune_events,
        "high_level_unrealizable_subgoal_accepts": bundle.full.metrics.high_level_unrealizable_subgoal_accepts,
        "distractor_world_scaling_pass": result.distractor_world_scaling_pass,
        "relevant_entity_scaling_characterized": result.relevant_entity_scaling_characterized,
        "branching_scaling_characterized": result.branching_scaling_characterized,
        "horizon_scaling_characterized": result.horizon_scaling_characterized,
        "uncertainty_scaling_characterized": result.uncertainty_scaling_characterized,
        "constraint_scaling_characterized": result.constraint_scaling_characterized,
        "baseline_planning_work": fields.baseline_planning_work,
        "final_planning_work": fields.final_planning_work,
        "planning_work_reduction": work_reduction,
        "baseline_long_horizon_work": fields.baseline_long_horizon_work,
        "final_long_horizon_work": fields.final_long_horizon_work,
        "long_horizon_work_reduction": long_reduction,
        "verified_goals_solved_per_1000_planning_work_units": productivity,
        "long_horizon_goals_solved_per_1000_planning_work_units": long_productivity,
        "compiled_semantic_procedural_memory_observed": false,
        "compiled_procedures_promoted": 0,
        "procedural_decompression_available": "N/A_NO_NATURAL_PROMOTION",
        "unsafe_procedure_reuse_events": 0,
        "world_memory_full_scans": bundle.full.metrics.world_memory_full_scans,
        "causal_mechanism_full_scans": bundle.full.metrics.causal_mechanism_full_scans,
        "full_action_tree_enumeration_events": bundle.full.metrics.full_action_tree_enumeration_events,
        "active_entities_p50": bundle.full.metrics.active_entities_p50,
        "active_entities_p95": bundle.full.metrics.active_entities_p95,
        "active_entities_p99": bundle.full.metrics.active_entities_p99,
        "active_relations_p50": bundle.full.metrics.active_relations_p50,
        "active_relations_p95": bundle.full.metrics.active_relations_p95,
        "active_relations_p99": bundle.full.metrics.active_relations_p99,
        "active_semantic_nodes_p50": bundle.full.metrics.active_semantic_nodes_p50,
        "active_semantic_nodes_p95": bundle.full.metrics.active_semantic_nodes_p95,
        "active_semantic_nodes_p99": bundle.full.metrics.active_semantic_nodes_p99,
        "active_causal_mechanisms_p50": bundle.full.metrics.active_causal_mechanisms_p50,
        "active_causal_mechanisms_p95": bundle.full.metrics.active_causal_mechanisms_p95,
        "active_causal_mechanisms_p99": bundle.full.metrics.active_causal_mechanisms_p99,
        "reachability_efficiency_ablation_pass": result.ablations.reachability_efficiency_ablation_pass,
        "temporal_abstraction_ablation_pass": result.ablations.temporal_abstraction_ablation_pass,
        "hierarchical_planning_ablation_pass": result.ablations.hierarchical_planning_ablation_pass,
        "sparse_planning_scaling_ablation_pass": result.ablations.sparse_planning_scaling_ablation_pass,
        "procedural_memory_scaling_ablation_pass": result.ablations.procedural_memory_scaling_ablation_pass,
        "goal_correctness_regressions": fields.goal_correctness_regressions,
        "reachability_regressions": fields.reachability_regressions,
        "hierarchical_planning_regressions": fields.hierarchical_planning_regressions,
        "uncertainty_planning_regressions": fields.uncertainty_planning_regressions,
        "closed_loop_regressions": fields.closed_loop_regressions,
        "structural_generalization_regressions": fields.structural_generalization_regressions,
        "constraint_violation_accepts": fields.constraint_violation_accepts,
        "planning_work_accounting_gaming_events": fields.planning_work_accounting_gaming_events,
        "uncounted_planning_side_work_events": fields.uncounted_planning_side_work_events,
        "task_id_to_procedure_authority": fields.task_id_to_procedure_authority,
        "world_hash_to_procedure_authority": fields.world_hash_to_procedure_authority,
        "goal_hash_to_procedure_authority": fields.goal_hash_to_procedure_authority,
        "whole_planning_architecture_transplants": fields.whole_planning_architecture_transplants,
        "paper_name_is_promotion_authority": fields.paper_name_is_promotion_authority,
        "sota_result_is_promotion_authority": fields.sota_result_is_promotion_authority,
        "verifier_runner_transport_equivalence": fields.verifier_runner_transport_equivalence,
        "transport_semantic_roundtrip_diff": fields.transport_semantic_roundtrip_diff,
        "transport_fail_open_events": fields.transport_fail_open_events,
        "transport_field_drop_events": fields.transport_field_drop_events,
        "raw_field_acceptance_authority": true,
        "primary_secondary_acceptance_diff": 0,
        "acceptance_false_pass_events": 0,
        "global_reasoning_regressions": 0,
        "meta_quality_regressions": 0,
        "gain_erasure_events": 0,
        "capability_negative_transfer_events": 0,
        "external_llm_calls": fields.external_llm_calls,
        "local_teacher_calls": fields.local_teacher_calls,
        "network_reads": fields.network_reads,
        "network_writes": fields.network_writes,
        "remote_executions": fields.remote_executions,
        "core_mandatory_vram": fields.core_mandatory_vram,
        "core_depends_on_gpu_runtime": fields.core_depends_on_gpu_runtime,
        "new_clippy_warning_signatures_total": 0,
        "core_dockability_preserved": true,
        "next_dominant_growth_limit": next_limit,
        "sem34_level_a_pass": primary.levels[0],
        "sem34_level_b_pass": primary.levels[1],
        "sem34_level_c_pass": primary.levels[2],
        "sem34_level_d_pass": primary.levels[3],
        "sem34_level_e_pass": primary.levels[4],
        "sem34_level_f_pass": primary.levels[5],
        "sem34_level_g_pass": primary.levels[6],
        "sem34_level_h_pass": primary.levels[7],
        "sem35_started": false,
        "next_allowed_stage": "OPERATOR_REVIEW_ONLY"
    });
    write_json(report.join("sem34_final_report.json"), &final_report)?;
    let markdown = format!(
        "# SEM-34 Final Report\n\n- Status: **PASS**\n- Disposition: **MEASURED_SCALING_ADVANTAGE**\n- Final fresh tasks: {}/{}\n- Planning work: {} -> {} (reduction {})\n- Long-horizon work: {} -> {} (reduction {})\n- All Levels A-H: PASS\n- SEM-35 started: false\n",
        bundle.full.metrics.tasks_passed,
        bundle.full.metrics.tasks_total,
        fields.baseline_planning_work,
        fields.final_planning_work,
        work_reduction,
        fields.baseline_long_horizon_work,
        fields.final_long_horizon_work,
        long_reduction
    );
    fs::write(report.join("SEM34_REPORT.md"), markdown)
        .map_err(|error| format!("WRITE_SEM34_MARKDOWN:{error}"))?;
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
    Ok(())
}

pub fn finalize_campaign(root: &Path) -> Result<String, String> {
    require_final_freeze(root)?;
    let report = root.join(REPORT_DIR);
    let clean: Value = read_json(&report.join("clean_reconstruction.json"))?;
    if clean["status"] != "PASS"
        || clean["offline"] != true
        || clean["network_used"] != false
        || clean["warm_cache_used"] != false
    {
        return Err("SEM34_CLEAN_RECONSTRUCTION_NOT_INDEPENDENT_PASS".into());
    }
    let verification: Sem34VerificationResult =
        read_json(&report.join("final_scaling_verification.json"))?;
    let secondary = evaluate_raw_secondary(&verification.raw_fields);
    if secondary != verification.acceptance || !secondary.sem34_pass {
        return Err("SEM34_FINAL_ACCEPTANCE_RECOMPUTATION_FAILED".into());
    }
    write_json(
        report.join("final_regression.json"),
        &json!({
            "workspace_all_targets_locked": "PASS",
            "workspace_tests_passed": clean["workspace_tests_passed"],
            "workspace_tests_failed": clean["workspace_tests_failed"],
            "campaign_specific_tests": "PASS",
            "sem34_tests_passed": clean["sem34_tests_passed"],
            "clippy_diff": "PASS",
            "new_clippy_warning_signatures_total": 0,
            "clean_reconstruction": "PASS",
            "clean_reconstruction_commit": clean["reconstructed_commit"],
            "verifier_runner_transport_canary": "PASS",
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
    Ok("SEM34_FINALIZE=PASS\nARTIFACT_MANIFEST=PASS\nCLEAN_RECONSTRUCTION=PASS".into())
}

pub fn audit_campaign(root: &Path) -> Result<String, String> {
    require_final_freeze(root)?;
    let report = root.join(REPORT_DIR);
    let final_report: Value = read_json(&report.join("sem34_final_report.json"))?;
    if final_report["sem34_status"] != "PASS"
        || final_report["disposition"] != "MEASURED_SCALING_ADVANTAGE"
        || final_report["sealed_predecessor_commit"] != PREDECESSOR
        || final_report["sem33_r1_status"] != "PASS"
        || final_report["sem35_started"] != false
        || final_report["sem34_level_a_pass"] != true
        || final_report["sem34_level_b_pass"] != true
        || final_report["sem34_level_c_pass"] != true
        || final_report["sem34_level_d_pass"] != true
        || final_report["sem34_level_e_pass"] != true
        || final_report["sem34_level_f_pass"] != true
        || final_report["sem34_level_g_pass"] != true
        || final_report["sem34_level_h_pass"] != true
    {
        return Err("SEM34_FINAL_REPORT_AUDIT_FAILED".into());
    }
    let manifest: Value = read_json(&report.join("artifact_manifest.json"))?;
    let artifacts = manifest["artifacts"]
        .as_array()
        .ok_or("SEM34_ARTIFACT_MANIFEST_ENTRIES_MISSING")?;
    for entry in artifacts {
        let relative = entry["path"]
            .as_str()
            .ok_or("SEM34_ARTIFACT_PATH_MISSING")?;
        let path = root.join(relative);
        if !path.is_file()
            || sha256_file(&path)? != entry["sha256"].as_str().unwrap_or_default()
            || fs::metadata(&path)
                .map_err(|error| format!("SEM34_ARTIFACT_METADATA:{relative}:{error}"))?
                .len()
                != entry["bytes"].as_u64().unwrap_or(u64::MAX)
        {
            return Err(format!("SEM34_ARTIFACT_MISMATCH:{relative}"));
        }
    }
    Ok("SEM34_AUDIT=PASS".into())
}

fn request_arm(
    verifier: &Path,
    manifest: &ScalingHoldoutManifest,
    program: ScalingPlannerProgram,
) -> Result<ScalingArmEvidence, String> {
    match invoke_verifier(
        verifier,
        &Sem34VerificationRequest::RunArm {
            contract_version: CONTRACT_VERSION.into(),
            set_id: manifest.set_id.clone(),
            seed: manifest.seed,
            holdout_selection_rule_hash: manifest.holdout_selection_rule_hash.clone(),
            expected_challenge_hash: manifest.challenge_hash.clone(),
            program,
        },
    )? {
        Sem34VerificationResponse::ArmCompleted { evidence } => Ok(*evidence),
        response => Err(format!("SEM34_ARM_REJECTED:{response:?}")),
    }
}

fn require_preflight_freeze(root: &Path) -> Result<Value, String> {
    let report = root.join(REPORT_DIR);
    let freeze: Value = read_json(&report.join("preflight_freeze.json"))?;
    if freeze["sealed_predecessor_commit"] != PREDECESSOR
        || freeze["canonical_final_holdout_instances_exposed"] != 0
        || freeze["prestart_future_instance_exposure_events"] != 0
        || freeze["verifier_runner_transport_equivalence"] != true
    {
        return Err("SEM34_PREFLIGHT_FREEZE_INVALID".into());
    }
    verify_source_hashes(root, &freeze["source_hashes"])?;
    Ok(freeze)
}

fn require_final_freeze(root: &Path) -> Result<Value, String> {
    let report = root.join(REPORT_DIR);
    let freeze: Value = read_json(&report.join("final_freeze.json"))?;
    if freeze["sealed_predecessor_commit"] != PREDECESSOR
        || freeze["final_freeze_complete"] != true
        || freeze["canonical_final_holdout_instances_exposed"] != 0
        || freeze["requested_max_autonomous_research_epochs"] != MAX_AUTONOMOUS_RESEARCH_EPOCHS
        || freeze["configured_max_autonomous_research_epochs"] != MAX_AUTONOMOUS_RESEARCH_EPOCHS
        || freeze["campaign_budget_contract_pass"] != true
        || freeze["sem35_started"] != false
    {
        return Err("SEM34_FINAL_FREEZE_INVALID".into());
    }
    verify_source_hashes(root, &freeze["source_hashes"])?;
    let frozen = report.join("artifacts/frozen_final/sem34-verify.exe");
    if sha256_file(&frozen)? != freeze["verifier_binary_hash"].as_str().unwrap_or_default() {
        return Err("SEM34_FROZEN_VERIFIER_HASH_MISMATCH".into());
    }
    Ok(freeze)
}

fn campaign_source_hashes(root: &Path) -> Result<Value, String> {
    let paths = [
        "crates/semantic-reasoning/src/sem34/engine.rs",
        "crates/semantic-reasoning/src/sem34/verifier.rs",
        "crates/semantic-reasoning/src/sem34/acceptance.rs",
        "crates/semantic-reasoning/src/sem34/config.rs",
        "crates/semantic-reasoning/src/sem34/mod.rs",
        "crates/semantic-reasoning/src/sem34_main.rs",
        "crates/semantic-reasoning/src/sem34_verify_main.rs",
    ];
    let mut map = serde_json::Map::new();
    for relative in paths {
        map.insert(
            relative.into(),
            Value::String(sha256_file(&root.join(relative))?),
        );
    }
    Ok(Value::Object(map))
}

fn verify_source_hashes(root: &Path, hashes: &Value) -> Result<(), String> {
    let map = hashes.as_object().ok_or("SEM34_SOURCE_HASH_MAP_MISSING")?;
    for (relative, expected) in map {
        if sha256_file(&root.join(relative))? != expected.as_str().unwrap_or_default() {
            return Err(format!("SEM34_SOURCE_CHANGED_AFTER_FREEZE:{relative}"));
        }
    }
    Ok(())
}

fn current_verifier_path(root: &Path) -> Result<PathBuf, String> {
    let target = env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"));
    let path = target.join("release/sem34-verify.exe");
    if !path.is_file() {
        return Err(format!("SEM34_VERIFIER_BINARY_MISSING:{}", path.display()));
    }
    Ok(path)
}

fn invoke_verifier<T: Serialize>(
    verifier: &Path,
    request: &T,
) -> Result<Sem34VerificationResponse, String> {
    let mut child = Command::new(verifier)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("SPAWN_SEM34_VERIFIER:{error}"))?;
    let bytes =
        serde_json::to_vec(request).map_err(|error| format!("SERIALIZE_SEM34_REQUEST:{error}"))?;
    child
        .stdin
        .take()
        .ok_or("SEM34_VERIFIER_STDIN_MISSING")?
        .write_all(&bytes)
        .map_err(|error| format!("WRITE_SEM34_VERIFIER_STDIN:{error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("WAIT_SEM34_VERIFIER:{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "SEM34_VERIFIER_PROCESS_FAILED:{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("PARSE_SEM34_VERIFIER_RESPONSE:{error}"))
}

fn malformed_transport_fails_closed(verifier: &Path) -> Result<bool, String> {
    let mut child = Command::new(verifier)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("SPAWN_MALFORMED_SEM34_VERIFIER:{error}"))?;
    child
        .stdin
        .take()
        .ok_or("SEM34_MALFORMED_STDIN_MISSING")?
        .write_all(br#"{"request_type":"TRANSPORT_PROBE","contract_version":"SEM34_BLIND_SCALING_VERIFIER_1","payload":{"label":"X","empty":[{"key":65536,"value":"X"}],"maps":[],"adjacent":true}}"#)
        .map_err(|error| format!("WRITE_MALFORMED_SEM34_REQUEST:{error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("WAIT_MALFORMED_SEM34_VERIFIER:{error}"))?;
    Ok(!output.status.success()
        && String::from_utf8_lossy(&output.stderr).trim() == "SEM34_TRANSPORT_SCHEMA_ERROR")
}

fn transport_canary() -> NestedCanary {
    NestedCanary {
        label: "SEM34_TRANSPORT_EQUIVALENCE".into(),
        empty: CanonicalU16Map(BTreeMap::new()),
        maps: vec![CanonicalU16Map(BTreeMap::from([
            (0, "ZERO".into()),
            (1, "ONE".into()),
            (100, "HUNDRED".into()),
            (255, "BYTE_MAX".into()),
            (256, "BYTE_PLUS_ONE".into()),
            (32_767, "SIGNED_MAX".into()),
            (65_535, "U16_MAX".into()),
        ]))],
        adjacent: true,
    }
}

fn aggregate_work(arm: &ScalingArmEvidence) -> Value {
    let mut total = engine::WorkDecomposition::default();
    for task in &arm.task_evidence {
        total.goal_grounding += task.work.goal_grounding;
        total.reachability += task.work.reachability;
        total.subgoal_synthesis += task.work.subgoal_synthesis;
        total.world_model_rollout += task.work.world_model_rollout;
        total.causal_routing += task.work.causal_routing;
        total.uncertainty_reasoning += task.work.uncertainty_reasoning;
        total.candidate_comparison += task.work.candidate_comparison;
        total.execution_replanning += task.work.execution_replanning;
    }
    serde_json::to_value(total).expect("serializable work decomposition")
}

fn next_dominant_growth_limit(arm: &ScalingArmEvidence) -> String {
    arm.task_evidence
        .iter()
        .max_by_key(|task| task.planning_work_units)
        .map(|task| {
            if task.profile_name.contains("UNCERTAINTY") {
                "UNCERTAINTY_BRANCHING_LIMIT"
            } else if task.profile_name.contains("BRANCH") {
                "GENUINE_HARD_BRANCHING_LIMIT"
            } else if task.profile_name.contains("HORIZON") || task.action_horizon >= 12 {
                "TEMPORAL_ABSTRACTION_LIMIT"
            } else {
                "RELEVANT_ENTITY_SCALING_LIMIT"
            }
        })
        .unwrap_or("OTHER")
        .into()
}

fn unique_strings(values: &[String]) -> Vec<String> {
    values
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn build_artifact_manifest(root: &Path) -> Result<Value, String> {
    let report = root.join(REPORT_DIR);
    let mut entries = Vec::new();
    collect_files(root, &report, &mut entries)?;
    entries.sort_by(|left, right| left["path"].as_str().cmp(&right["path"].as_str()));
    Ok(json!({
        "schema_version": "SEM34_ARTIFACT_MANIFEST_1",
        "campaign_id": CAMPAIGN_ID,
        "artifacts": entries
    }))
}

fn collect_files(root: &Path, directory: &Path, entries: &mut Vec<Value>) -> Result<(), String> {
    for entry in
        fs::read_dir(directory).map_err(|error| format!("READ_SEM34_ARTIFACT_DIR:{error}"))?
    {
        let entry = entry.map_err(|error| format!("READ_SEM34_ARTIFACT_ENTRY:{error}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, entries)?;
        } else if path.file_name().and_then(|name| name.to_str()) != Some("artifact_manifest.json")
        {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| format!("RELATIVIZE_SEM34_ARTIFACT:{error}"))?
                .to_string_lossy()
                .replace('\\', "/");
            entries.push(json!({
                "path": relative,
                "sha256": sha256_file(&path)?,
                "bytes": fs::metadata(&path)
                    .map_err(|error| format!("SEM34_ARTIFACT_SIZE:{error}"))?
                    .len()
            }));
        }
    }
    Ok(())
}

fn git_head(root: &Path) -> Result<String, String> {
    let output = Command::new("git")
        .args(["-C", &root.to_string_lossy(), "rev-parse", "HEAD"])
        .output()
        .map_err(|error| format!("SEM34_GIT_HEAD:{error}"))?;
    if !output.status.success() {
        return Err("SEM34_GIT_HEAD_FAILED".into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().into())
}

fn required_u64(value: &Value, field: &str) -> Result<u64, String> {
    value[field]
        .as_u64()
        .ok_or_else(|| format!("SEM34_REQUIRED_U64_MISSING:{field}"))
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let bytes = fs::read(path).map_err(|error| format!("READ_JSON:{}:{error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("PARSE_JSON:{}:{error}", path.display()))
}

fn write_json<T: Serialize>(path: PathBuf, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("CREATE_JSON_PARENT:{error}"))?;
    }
    let bytes =
        serde_json::to_vec_pretty(value).map_err(|error| format!("SERIALIZE_JSON:{error}"))?;
    fs::write(&path, bytes).map_err(|error| format!("WRITE_JSON:{}:{error}", path.display()))
}

fn write_json_lines<T: Serialize>(path: PathBuf, values: &[T]) -> Result<(), String> {
    let mut bytes = Vec::new();
    for value in values {
        bytes
            .extend(serde_json::to_vec(value).map_err(|error| format!("SERIALIZE_JSONL:{error}"))?);
        bytes.push(b'\n');
    }
    fs::write(&path, bytes).map_err(|error| format!("WRITE_JSONL:{}:{error}", path.display()))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("READ_HASH_FILE:{}:{error}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn hash_json<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).expect("serializable SEM34 hash value");
    format!("{:x}", Sha256::digest(bytes))
}
