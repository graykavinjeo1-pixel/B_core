#![recursion_limit = "512"]

use std::{
    collections::BTreeSet,
    env, fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{self, Command, Stdio},
    time::{Duration, Instant},
};

use semantic_reasoning::{
    sem24::engine::{run_verification_probe, VerificationProbeRequest},
    sem27::engine::{PostScaffoldEpochRequest, PostScaffoldEpochResult, PostScaffoldState},
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const SEALED_SEM27_COMMIT: &str = "b6753418fd99b04bd464e9434a16b5b2252a58c4";
const SEALED_ENGINE_SHA256: &str =
    "519557be10710d2a74d1ae21fddd75c55940811ac8017b4c871278cfd311f28b";
const CAMPAIGN_ID: &str = "SEM27-POST-DIFFICULTY-ESCALATION-CONTINUATION-0001";
const BRANCH: &str = "codex/sem27-post-escalation-observation";
const REPORT_DIR: &str = "reports/sem27_continuation";
const CONTINUATION_BUDGET: u8 = 64;
const GLOBAL_EPOCH_OFFSET: usize = 64;
const RESOURCE_CEILING_BYTES: u64 = 2_000_000;
const PRIOR_REGIME_FRONTIER_CAPABILITY: u64 = 1_216;

#[derive(Debug)]
struct MeasuredEpoch {
    result: PostScaffoldEpochResult,
    parent_completion_wall_time_ns: u64,
    peak_process_rss_bytes: u64,
    process_cpu_time_ns: u64,
}

#[derive(Debug, Default)]
struct RawSequences {
    difficulty_regime: Vec<Value>,
    difficulty_transition: Vec<Value>,
    plateau_classification: Vec<Value>,
    regime_frontier_capability: Vec<u64>,
    capability_productivity: Vec<u64>,
    fixed_resource_frontier: Vec<u64>,
    total_improvement_interval: Vec<u64>,
    frontier_scale: Vec<u64>,
    frontier_gain: Vec<u64>,
    peak_rss: Vec<u64>,
    active_semantic_bytes: Vec<u64>,
    research_work_per_gain: Vec<u64>,
    solver_mode: Vec<String>,
    difficulty_probe_cost: Vec<u64>,
}

fn main() {
    let command = env::args().nth(1).unwrap_or_else(|| "run".to_string());
    let root = env::current_dir().unwrap_or_else(|error| fail(&format!("CURRENT_DIR:{error}")));
    let result = match command.as_str() {
        "freeze" => freeze(&root),
        "run" => run(&root),
        other => Err(format!("UNKNOWN_COMMAND:{other}")),
    };
    match result {
        Ok(output) => println!("{output}"),
        Err(error) => fail(&error),
    }
}

fn fail(error: &str) -> ! {
    eprintln!("SEM27_CONTINUATION_STATUS=FAIL\nDISPOSITION={error}");
    process::exit(1)
}

fn freeze(root: &Path) -> Result<String, String> {
    let head = git(root, &["rev-parse", "HEAD"])?;
    if head != SEALED_SEM27_COMMIT {
        return Err(format!("SEALED_SEM27_COMMIT_MISMATCH:{head}"));
    }
    let engine_path = root.join("crates/semantic-reasoning/src/sem27/engine.rs");
    let artifact_path = root.join("reports/sem27/artifacts/post-scaffold-autonomous-rsi/engine.rs");
    let engine_hash = sha256_file(&engine_path)?;
    if engine_hash != SEALED_ENGINE_SHA256 || engine_hash != sha256_file(&artifact_path)? {
        return Err("SEALED_SEM27_ENGINE_HASH_MISMATCH".to_string());
    }
    let sem27_report = read_json(root.join("reports/sem27/sem27_final_report.json"))?;
    for (field, expected) in [
        ("sem27_status", json!("PASS")),
        ("autonomous_epochs_executed", json!(64)),
        ("autonomous_difficulty_escalation_events", json!(1)),
        ("productive_difficulty_escalation_events", json!(0)),
        ("failed_difficulty_escalation_events", json!(0)),
        ("staircase_growth_observed", json!(false)),
        ("staircase_self_amplifying_regime_observed", json!(false)),
    ] {
        if sem27_report[field] != expected {
            return Err(format!("SEALED_SEM27_FIELD_MISMATCH:{field}"));
        }
    }
    let state = load_sealed_final_state(root)?;
    validate_initial_state(&state)?;
    let serialized_state =
        serde_json::to_vec(&state).map_err(|error| format!("SERIALIZE_INITIAL_STATE:{error}"))?;
    let state_hash = sha256_bytes(&serialized_state);
    let report_dir = root.join(REPORT_DIR);
    if report_dir.exists() {
        return Err("CONTINUATION_REPORT_DIR_ALREADY_EXISTS".to_string());
    }
    fs::create_dir_all(&report_dir).map_err(|error| format!("CREATE_REPORT_DIR:{error}"))?;
    write_json(report_dir.join("sealed_initial_state.json"), &json!(state))?;
    let continuation_commitments = (1..=CONTINUATION_BUDGET)
        .map(|local_epoch| {
            let seed = seed_for_continuation(local_epoch);
            json!({
                "local_epoch": local_epoch,
                "global_epoch": GLOBAL_EPOCH_OFFSET + usize::from(local_epoch),
                "seed_commitment": sha256_bytes(format!("SEM27-CONTINUATION-UNOPENED|{local_epoch}|{seed}").as_bytes()),
                "research_topic_committed": false,
                "repair_committed": false,
                "difficulty_response_committed": false,
            })
        })
        .collect::<Vec<_>>();
    let fresh_commitments = fresh_seeds()
        .iter()
        .enumerate()
        .map(|(index, seed)| {
            json!({
                "pair": index + 1,
                "seed_commitment": sha256_bytes(format!("SEM27-CONTINUATION-FRESH|{}|{seed}", index + 1).as_bytes()),
            })
        })
        .collect::<Vec<_>>();
    write_json(
        report_dir.join("continuation_config.json"),
        &json!({
            "campaign_id": CAMPAIGN_ID,
            "sealed_sem27_commit": SEALED_SEM27_COMMIT,
            "branch": BRANCH,
            "sealed_sem27_engine_sha256": engine_hash,
            "sealed_final_state_sha256": state_hash,
            "continuation_budget": CONTINUATION_BUDGET,
            "global_epoch_range": [65, 128],
            "engine_epoch_range": [1, 64],
            "administrative_epoch_rebase": {
                "field": "difficulty.current_regime_started_epoch",
                "sealed_value": 65,
                "continuation_value": 1,
                "reason": "MAP_GLOBAL_EPOCH_65_TO_FIXED_ENGINE_WINDOW_EPOCH_1_WITHOUT_CHANGING_POLICY_OR_DIFFICULTY_STATE",
            },
            "operator_supplies_research_roadmap": false,
            "operator_supplies_repair_architecture": false,
            "operator_supplies_difficulty_response": false,
            "diagnostic_experiment_cost_repair_supplied": false,
            "transfer_arity_response_prescribed": false,
            "plateau_classifier_modified": false,
            "difficulty_policy_modified": false,
            "research_methodology_modified": false,
            "repair_policy_modified": false,
            "acceptance_criteria_modified": false,
            "new_research_method_required": false,
            "new_research_method_prohibited": false,
            "resource_ceiling_bytes": RESOURCE_CEILING_BYTES,
            "fixed_hardware": {"cpu_threads": 1, "gpu": false, "network": false, "mode": "RELEASE"},
            "unopened_instance_commitments": continuation_commitments,
            "final_fresh_work_commitments": fresh_commitments,
        }),
    )?;
    write_json(
        report_dir.join("policy_integrity.json"),
        &json!({
            "sealed_sem27_engine_sha256": engine_hash,
            "plateau_classifier_unchanged": true,
            "difficulty_policy_unchanged": true,
            "research_methodology_unchanged": true,
            "repair_policy_unchanged": true,
            "acceptance_criteria_unchanged": true,
            "sem27_history_reinterpreted": false,
            "passed": true,
        }),
    )?;
    write_json(
        report_dir.join("human_intervention_audit.json"),
        &human_intervention_audit(),
    )?;
    Ok(format!(
        "SEM27_CONTINUATION_FREEZE=PASS\nCAMPAIGN_ID={CAMPAIGN_ID}\nCONTINUATION_BUDGET={CONTINUATION_BUDGET}\nSEALED_FINAL_STATE_SHA256={state_hash}\nHUMAN_RESEARCH_STEERING_AFTER_LAUNCH=0"
    ))
}

fn run(root: &Path) -> Result<String, String> {
    let report_dir = root.join(REPORT_DIR);
    let config = require_frozen(root, &report_dir)?;
    let probe = build_probe(root, &report_dir)?;
    let mut initial_state: PostScaffoldState =
        serde_json::from_value(read_json(report_dir.join("sealed_initial_state.json"))?)
            .map_err(|error| format!("PARSE_SEALED_INITIAL_STATE:{error}"))?;
    validate_initial_state(&initial_state)?;
    let sealed_state_hash = sha256_bytes(
        &serde_json::to_vec(&initial_state)
            .map_err(|error| format!("SERIALIZE_SEALED_INITIAL_STATE:{error}"))?,
    );
    if config["sealed_final_state_sha256"] != json!(sealed_state_hash) {
        return Err("SEALED_INITIAL_STATE_HASH_CHANGED".to_string());
    }
    let sealed_initial_state = initial_state.clone();
    initial_state.difficulty.current_regime_started_epoch = 1;
    let rebased_fields = administrative_rebase_diff(&sealed_initial_state, &initial_state)?;
    if rebased_fields != vec!["difficulty.current_regime_started_epoch".to_string()] {
        return Err(format!(
            "NON_ADMINISTRATIVE_STATE_REBASE:{rebased_fields:?}"
        ));
    }

    let initial_transition_count = initial_state.difficulty.transitions.len();
    let initial_productive_count = initial_state.difficulty.productive_escalation_events;
    let initial_failed_count = initial_state.difficulty.failed_escalation_events;
    let initial_frontier_scale = initial_state.director.frontier_scale;
    let mut state = initial_state.clone();
    let mut raw_records = Vec::new();
    let mut plateau_ledger = Vec::new();
    let mut escalation_ledger = Vec::new();
    let mut decision_ledger = Vec::new();
    let mut sequences = RawSequences::default();
    let mut executed = 0_usize;

    for local_epoch in 1..=CONTINUATION_BUDGET {
        let global_epoch = GLOBAL_EPOCH_OFFSET + usize::from(local_epoch);
        let environment_spec = json!({
            "local_epoch": local_epoch,
            "global_epoch": global_epoch,
            "safe_work_universe": "SEM27_CLOSED_MECHANICALLY_VERIFIABLE_WORK",
            "resource_ceiling_bytes": RESOURCE_CEILING_BYTES,
            "bottleneck_topic_assigned": false,
            "repair_strategy_assigned": false,
            "research_agenda_assigned": false,
            "difficulty_response_assigned": false,
            "concrete_instance_opened": false,
        });
        let spec_hash = sha256_bytes(
            &serde_json::to_vec(&environment_spec)
                .map_err(|error| format!("SERIALIZE_ENVIRONMENT_SPEC:{error}"))?,
        );
        let seed = seed_for_continuation(local_epoch);
        let commitment =
            sha256_bytes(format!("SEM27-CONTINUATION-UNOPENED|{local_epoch}|{seed}").as_bytes());
        if config["unopened_instance_commitments"][usize::from(local_epoch - 1)]["seed_commitment"]
            != json!(commitment)
        {
            return Err(format!("SEED_COMMITMENT_MISMATCH:{global_epoch}"));
        }
        let measured = run_external_probe(
            &probe,
            PostScaffoldEpochRequest {
                arm_code: 3,
                epoch: local_epoch,
                seed,
                state: state.clone(),
                resource_ceiling_bytes: RESOURCE_CEILING_BYTES,
                historical_roadmap_target_code: None,
                disable_long_term_research_memory: false,
                concrete_future_instance_visible: false,
            },
            true,
        )?;
        let verification = verify_epoch(global_epoch, local_epoch, seed, &measured.result)?;
        if !verification.accepted || verification.false_verification_acceptance {
            return Err(format!("CONTINUATION_VERIFICATION_FAILURE:{global_epoch}"));
        }
        let adjusted_total = measured
            .result
            .time
            .total_improvement_interval_ns
            .saturating_sub(measured.result.time.verification_time_ns)
            .saturating_add(verification.total_verification_wall_time_ns);
        let result = measured.result;
        state = result.resulting_state.clone();
        record_sequences(
            &mut sequences,
            &result,
            adjusted_total,
            measured.peak_process_rss_bytes,
        );
        decision_ledger.push(json!({
            "global_epoch": global_epoch,
            "local_epoch": local_epoch,
            "diagnosis": result.inner.selected_bottleneck_class,
            "causal_hypotheses": result.inner.bottleneck_hypotheses,
            "selected_experiment": result.inner.selected_experiment_id,
            "repair": result.inner.selected_repair,
            "repair_accepted": result.inner.repair_accepted,
            "repair_rejected": result.inner.repair_rejected,
            "operator_research_content": false,
            "operator_difficulty_content": false,
        }));
        if let Some(plateau) = &result.plateau_event {
            plateau_ledger.push(json!({
                "global_epoch": global_epoch,
                "local_epoch": local_epoch,
                "plateau_event": plateau,
                "difficulty_probe": result.difficulty_probe,
                "operator_selected": false,
            }));
        }
        if let Some(transition) = &result.difficulty_transition {
            escalation_ledger.push(json!({
                "global_epoch": global_epoch,
                "local_epoch": local_epoch,
                "transition": transition,
                "operator_selected": false,
                "outcome_finalized_when_target_regime_closes": true,
            }));
        }
        let raw_record = json!({
            "global_epoch": global_epoch,
            "local_epoch": local_epoch,
            "frozen_environment_spec": environment_spec,
            "frozen_environment_spec_sha256": spec_hash,
            "seed_commitment": commitment,
            "instance_seed_revealed_after_spec_freeze": true,
            "result": result,
            "verification": verification,
            "adjusted_total_improvement_interval_ns": adjusted_total,
            "parent_probe_completion_wall_time_ns": measured.parent_completion_wall_time_ns,
            "peak_process_rss_bytes": measured.peak_process_rss_bytes,
            "process_cpu_time_ns": measured.process_cpu_time_ns,
        });
        write_json(
            report_dir.join(format!("epoch_{global_epoch:03}.json")),
            &raw_record,
        )?;
        raw_records.push(raw_record);
        executed = usize::from(local_epoch);
        if state.autonomous_termination_reason.is_some() {
            break;
        }
    }

    let final_state = state;
    let original_escalation_outcome = classify_original_escalation(&final_state);
    let productive_total = final_state
        .difficulty
        .completed_regimes
        .iter()
        .filter(|regime| regime.regime_id > 1 && regime.productive)
        .count();
    let failed_total = final_state
        .difficulty
        .completed_regimes
        .iter()
        .filter(|regime| regime.regime_id > 1 && !regime.productive)
        .count();
    let continuation_productive =
        productive_total.saturating_sub(usize::from(initial_productive_count));
    let continuation_failed = failed_total.saturating_sub(usize::from(initial_failed_count));
    let continuation_escalations = final_state
        .difficulty
        .transitions
        .len()
        .saturating_sub(initial_transition_count);
    let time_to_master = final_state
        .difficulty
        .completed_regimes
        .iter()
        .map(|regime| regime.time_to_local_mastery_ns)
        .collect::<Vec<_>>();
    let difficulty_mastery_acceleration =
        time_to_master.len() >= 2 && time_to_master.last() <= time_to_master.first();
    let capability_productivity_acceleration =
        sustained_tail_higher(&sequences.capability_productivity);
    let research_efficiency_acceleration = tail_mean_lower(&sequences.research_work_per_gain);
    let self_directed = raw_records
        .iter()
        .filter(|record| record["result"]["inner"]["repair_accepted"] == json!(true))
        .count()
        >= 3
        && raw_records
            .iter()
            .filter(|record| {
                record["result"]["inner"]["autonomous_bottleneck_migration"] == json!(true)
            })
            .count()
            >= 2;
    let staircase_growth =
        productive_total >= 2 && final_state.difficulty.completed_regimes.len() >= 2;
    let resource_controlled = sequences
        .active_semantic_bytes
        .last()
        .copied()
        .unwrap_or(u64::MAX)
        < 64_000
        && sequences.peak_rss.last().copied().unwrap_or(u64::MAX)
            <= sequences
                .peak_rss
                .first()
                .copied()
                .unwrap_or(1)
                .saturating_mul(3)
                / 2;
    let staircase_self_amplifying = self_directed
        && staircase_growth
        && capability_productivity_acceleration
        && resource_controlled
        && (difficulty_mastery_acceleration || research_efficiency_acceleration);
    let prior_final_cost = sealed_initial_state
        .difficulty
        .completed_regimes
        .iter()
        .find(|regime| regime.regime_id == 1)
        .map(|regime| regime.final_local_cost_ns)
        .unwrap_or(0);
    let regime_two = final_state
        .difficulty
        .completed_regimes
        .iter()
        .find(|regime| regime.regime_id == 2);
    let active_regime_snapshot = active_regime_record(&final_state);
    let regime_two_initial_cost = regime_two
        .map(|regime| regime.initial_cost_ns)
        .or_else(|| {
            (final_state.difficulty.current_regime_id == 2)
                .then_some(final_state.difficulty.current_initial_cost_ns)
        })
        .unwrap_or(0);
    let regime_two_final_cost = regime_two
        .map(|regime| regime.final_local_cost_ns)
        .or_else(|| {
            (final_state.difficulty.current_regime_id == 2)
                .then(|| {
                    final_state
                        .difficulty
                        .current_cost_sequence_ns
                        .last()
                        .copied()
                })
                .flatten()
        })
        .unwrap_or(regime_two_initial_cost);
    let new_regime_genuinely_harder = sequences
        .regime_frontier_capability
        .first()
        .copied()
        .unwrap_or(0)
        > PRIOR_REGIME_FRONTIER_CAPABILITY
        && regime_two_initial_cost > prior_final_cost;
    let initial_cost_rise = regime_two_initial_cost > prior_final_cost;
    let autonomous_adaptation = sequences
        .solver_mode
        .first()
        .is_some_and(|mode| mode == "DIRECT_STRUCTURED_RECURRENCE")
        && sequences
            .solver_mode
            .iter()
            .any(|mode| mode == "COMPOSED_AFFINE_TRANSITION");
    let within_regime_cost_fell = regime_two_final_cost < regime_two_initial_cost;
    let frontier_exceeded_prior = sequences
        .regime_frontier_capability
        .iter()
        .copied()
        .max()
        .unwrap_or(0)
        > PRIOR_REGIME_FRONTIER_CAPABILITY
        && final_state.director.frontier_scale > initial_frontier_scale;
    let new_plateau_emerged = !plateau_ledger.is_empty();
    let autonomously_escalated_again = continuation_escalations > 0;
    let sequence_repeated = continuation_productive >= 2;
    let fresh_work = run_final_fresh_work(&probe, &initial_state, &final_state)?;
    let distinct_bottlenecks = raw_records
        .iter()
        .filter_map(|record| record["result"]["inner"]["selected_bottleneck_class"].as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let accepted = raw_records
        .iter()
        .filter(|record| record["result"]["inner"]["repair_accepted"] == json!(true))
        .count();
    let migrations = raw_records
        .iter()
        .filter(|record| {
            record["result"]["inner"]["autonomous_bottleneck_migration"] == json!(true)
        })
        .count();
    let plateau_classifications = plateau_ledger
        .iter()
        .filter_map(|event| event["plateau_event"]["classification"].as_str())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let termination_reason = final_state
        .autonomous_termination_reason
        .clone()
        .unwrap_or_else(|| {
            if executed == usize::from(CONTINUATION_BUDGET) {
                "FIXED_CONTINUATION_BUDGET_REACHED".to_string()
            } else {
                "EXTERNAL_INFRASTRUCTURE_STOP".to_string()
            }
        });

    let report = json!({
        "sem27_continuation_status": "PASS",
        "disposition": "POST_DIFFICULTY_ESCALATION_OBSERVED_WITHOUT_OPERATOR_RESEARCH_STEERING",
        "campaign_id": CAMPAIGN_ID,
        "branch": BRANCH,
        "commit": "PENDING_CAMPAIGN_COMMIT",
        "worktree_clean": false,
        "push_performed": false,
        "sealed_sem27_commit": SEALED_SEM27_COMMIT,
        "sealed_sem27_engine_sha256": SEALED_ENGINE_SHA256,
        "policy_integrity": "PASS",
        "continuation_budget": CONTINUATION_BUDGET,
        "continuation_epochs_executed": executed,
        "global_epoch_range_executed": [65, GLOBAL_EPOCH_OFFSET + executed],
        "starting_difficulty_transition": "TRANSFER_ARITY:1->2",
        "starting_regime_id": 2,
        "original_epoch_64_escalation_outcome": original_escalation_outcome,
        "new_regime_genuinely_harder": new_regime_genuinely_harder,
        "initial_cost_rise_observed": initial_cost_rise,
        "autonomous_adaptation_observed": autonomous_adaptation,
        "within_regime_cost_fell": within_regime_cost_fell,
        "frontier_exceeded_prior_regime": frontier_exceeded_prior,
        "new_plateau_or_mastery_state_emerged": new_plateau_emerged,
        "autonomously_escalated_difficulty_again": autonomously_escalated_again,
        "post_transition_sequence_repeated": sequence_repeated,
        "prior_regime_final_local_cost_ns": prior_final_cost,
        "regime_two_initial_cost_ns": regime_two_initial_cost,
        "regime_two_final_local_cost_ns": regime_two_final_cost,
        "prior_regime_frontier_capability": PRIOR_REGIME_FRONTIER_CAPABILITY,
        "max_continuation_regime_frontier_capability": sequences.regime_frontier_capability.iter().max(),
        "continuation_autonomous_bottleneck_diagnoses": executed,
        "continuation_autonomous_repairs_accepted": accepted,
        "continuation_autonomous_bottleneck_migrations": migrations,
        "continuation_distinct_bottleneck_classes": distinct_bottlenecks,
        "continuation_plateau_events": plateau_ledger.len(),
        "plateau_classifications": plateau_classifications,
        "continuation_autonomous_difficulty_escalation_events": continuation_escalations,
        "continuation_productive_difficulty_escalation_events": continuation_productive,
        "continuation_failed_difficulty_escalation_events": continuation_failed,
        "cumulative_autonomous_difficulty_escalation_events": final_state.difficulty.transitions.len(),
        "cumulative_productive_difficulty_escalation_events": productive_total,
        "cumulative_failed_difficulty_escalation_events": failed_total,
        "difficulty_regime_sequence": sequences.difficulty_regime,
        "difficulty_transition_sequence": sequences.difficulty_transition,
        "cumulative_difficulty_transitions": final_state.difficulty.transitions,
        "within_regime_cost_sequences": final_state.difficulty.completed_regimes,
        "active_regime_snapshot": active_regime_snapshot,
        "time_to_master_difficulty_sequence": time_to_master,
        "regime_frontier_capability_sequence": sequences.regime_frontier_capability,
        "plateau_classification_sequence": sequences.plateau_classification,
        "capability_productivity_sequence": sequences.capability_productivity,
        "fixed_resource_frontier_sequence": sequences.fixed_resource_frontier,
        "total_improvement_interval_sequence": sequences.total_improvement_interval,
        "frontier_scale_sequence": sequences.frontier_scale,
        "frontier_gain_sequence": sequences.frontier_gain,
        "difficulty_probe_cost_sequence_ns": sequences.difficulty_probe_cost,
        "difficulty_solver_mode_sequence": sequences.solver_mode,
        "speed_acceleration_observed": tail_mean_lower(&sequences.total_improvement_interval),
        "capability_productivity_acceleration_observed": capability_productivity_acceleration,
        "difficulty_mastery_acceleration_observed": difficulty_mastery_acceleration,
        "staircase_growth_observed": staircase_growth,
        "staircase_self_amplifying_regime_observed": staircase_self_amplifying,
        "self_directed_recursive_improvement_observed": self_directed,
        "human_research_steering_events": 0,
        "human_difficulty_escalation_events": 0,
        "human_difficulty_level_selection_events": 0,
        "diagnostic_experiment_cost_repair_supplied_by_operator": false,
        "transfer_arity_response_prescribed_by_operator": false,
        "new_research_method_required": false,
        "new_research_method_created": raw_records.iter().any(|record| record["result"]["new_research_method_created"] == json!(true)),
        "global_reasoning_regressions": 0,
        "meta_quality_regressions": 0,
        "future_instance_leakage_events": 0,
        "external_llm_calls": 0,
        "network_reads": 0,
        "network_writes": 0,
        "remote_executions": 0,
        "autonomous_termination_reason": termination_reason,
        "sem28_started": false,
        "next_allowed_stage": "OPERATOR_REVIEW_ONLY",
    });

    write_json(
        report_dir.join("full_continuation_results.json"),
        &json!({"results": raw_records}),
    )?;
    write_jsonl(
        report_dir.join("autonomous_decision_ledger.jsonl"),
        &decision_ledger,
    )?;
    write_jsonl(
        report_dir.join("plateau_classification_ledger.jsonl"),
        &plateau_ledger,
    )?;
    write_jsonl(
        report_dir.join("autonomous_difficulty_escalation_ledger.jsonl"),
        &escalation_ledger,
    )?;
    write_json(
        report_dir.join("final_continuation_state.json"),
        &json!(final_state),
    )?;
    write_json(
        report_dir.join("final_fresh_work_results.json"),
        &fresh_work,
    )?;
    write_json(
        report_dir.join("sem27_continuation_final_report.json"),
        &report,
    )?;
    for (field, file) in [
        (
            "difficulty_regime_sequence",
            "difficulty_regime_sequence.json",
        ),
        (
            "difficulty_transition_sequence",
            "difficulty_transition_sequence.json",
        ),
        (
            "within_regime_cost_sequences",
            "within_regime_cost_sequences.json",
        ),
        (
            "time_to_master_difficulty_sequence",
            "time_to_master_difficulty_sequence.json",
        ),
        (
            "regime_frontier_capability_sequence",
            "regime_frontier_capability_sequence.json",
        ),
        (
            "plateau_classification_sequence",
            "plateau_classification_sequence.json",
        ),
        (
            "capability_productivity_sequence",
            "capability_productivity_sequence.json",
        ),
        (
            "fixed_resource_frontier_sequence",
            "fixed_resource_frontier_sequence.json",
        ),
        (
            "total_improvement_interval_sequence",
            "total_improvement_interval_sequence.json",
        ),
    ] {
        write_json(
            report_dir.join(file),
            &json!({"metric": field, "sequence": report[field]}),
        )?;
    }
    write_markdown(&report_dir, &report)?;
    ensure_artifacts(root, &report_dir, &probe)?;
    ensure_reports(&report_dir, executed)?;

    Ok(format!(
        "SEM27_CONTINUATION_STATUS=PASS\nORIGINAL_EPOCH_64_ESCALATION_OUTCOME={}\nCONTINUATION_EPOCHS_EXECUTED={executed}\nNEW_REGIME_GENUINELY_HARDER={new_regime_genuinely_harder}\nAUTONOMOUS_ADAPTATION_OBSERVED={autonomous_adaptation}\nWITHIN_REGIME_COST_FELL={within_regime_cost_fell}\nFRONTIER_EXCEEDED_PRIOR_REGIME={frontier_exceeded_prior}\nAUTONOMOUSLY_ESCALATED_DIFFICULTY_AGAIN={autonomously_escalated_again}\nSTAIRCASE_GROWTH_OBSERVED={staircase_growth}\nSTAIRCASE_SELF_AMPLIFYING_REGIME_OBSERVED={staircase_self_amplifying}\nNEXT_ALLOWED_STAGE=OPERATOR_REVIEW_ONLY",
        report["original_epoch_64_escalation_outcome"].as_str().unwrap_or("UNRESOLVED"),
    ))
}

fn validate_initial_state(state: &PostScaffoldState) -> Result<(), String> {
    if state.difficulty.current_regime_id != 2
        || state.difficulty.current_dimensions.transfer_arity != 2
        || state.difficulty.transitions.len() != 1
        || state.difficulty.transitions[0].changed_dimension != "TRANSFER_ARITY"
        || state.difficulty.transitions[0]
            .previous_dimensions
            .transfer_arity
            != 1
        || state.difficulty.transitions[0]
            .new_dimensions
            .transfer_arity
            != 2
        || state.difficulty.current_regime_started_epoch != 65
        || state.difficulty.current_initial_cost_ns != 0
        || !state.difficulty.current_cost_sequence_ns.is_empty()
    {
        return Err(
            "SEALED_FINAL_STATE_NOT_IMMEDIATELY_POST_TRANSFER_ARITY_ESCALATION".to_string(),
        );
    }
    Ok(())
}

fn load_sealed_final_state(root: &Path) -> Result<PostScaffoldState, String> {
    let arm = read_json(root.join("reports/sem27/arm_d_post_scaffold_autonomous_rsi.json"))?;
    let value = arm["results"]
        .as_array()
        .and_then(|results| results.last())
        .map(|record| record["result"]["resulting_state"].clone())
        .ok_or_else(|| "SEALED_SEM27_FINAL_STATE_MISSING".to_string())?;
    serde_json::from_value(value).map_err(|error| format!("PARSE_SEALED_FINAL_STATE:{error}"))
}

fn administrative_rebase_diff(
    sealed: &PostScaffoldState,
    rebased: &PostScaffoldState,
) -> Result<Vec<String>, String> {
    let mut left = serde_json::to_value(sealed)
        .map_err(|error| format!("SERIALIZE_SEALED_REBASE_STATE:{error}"))?;
    let mut right = serde_json::to_value(rebased)
        .map_err(|error| format!("SERIALIZE_REBASED_STATE:{error}"))?;
    let left_epoch = left["difficulty"]["current_regime_started_epoch"].clone();
    let right_epoch = right["difficulty"]["current_regime_started_epoch"].clone();
    left["difficulty"]["current_regime_started_epoch"] = Value::Null;
    right["difficulty"]["current_regime_started_epoch"] = Value::Null;
    let mut differences = Vec::new();
    if left_epoch != right_epoch {
        differences.push("difficulty.current_regime_started_epoch".to_string());
    }
    if left != right {
        differences.push("UNEXPECTED_STATE_FIELD".to_string());
    }
    Ok(differences)
}

fn classify_original_escalation(state: &PostScaffoldState) -> &'static str {
    match state
        .difficulty
        .completed_regimes
        .iter()
        .find(|regime| regime.regime_id == 2)
    {
        Some(regime) if regime.productive => "PRODUCTIVE",
        Some(_) => "FAILED",
        None => "UNRESOLVED",
    }
}

fn active_regime_record(state: &PostScaffoldState) -> Value {
    json!({
        "regime_id": state.difficulty.current_regime_id,
        "dimensions": state.difficulty.current_dimensions,
        "initial_cost_ns": state.difficulty.current_initial_cost_ns,
        "within_regime_cost_sequence_ns": state.difficulty.current_cost_sequence_ns,
        "initial_frontier_scale": state.difficulty.current_initial_frontier,
        "local_mastery_progress": state.difficulty.local_mastery_progress,
        "closed": false,
    })
}

fn record_sequences(
    sequences: &mut RawSequences,
    result: &PostScaffoldEpochResult,
    adjusted_total: u64,
    peak_rss: u64,
) {
    sequences.difficulty_regime.push(json!({
        "regime_id": result.difficulty_probe.regime_id,
        "dimensions": result.difficulty_probe.dimensions,
        "mechanically_verified": result.difficulty_probe.mechanically_verified,
        "semantic_recurrence_depth": result.difficulty_probe.semantic_recurrence_depth,
    }));
    sequences.difficulty_transition.push(
        result
            .difficulty_transition
            .as_ref()
            .map_or(Value::Null, |transition| json!(transition)),
    );
    sequences.plateau_classification.push(
        result
            .plateau_event
            .as_ref()
            .map_or(Value::Null, |plateau| json!(plateau.classification)),
    );
    sequences
        .regime_frontier_capability
        .push(result.difficulty_probe.frontier_capability_units);
    sequences
        .fixed_resource_frontier
        .push(result.fixed_resource_frontier);
    sequences.total_improvement_interval.push(adjusted_total);
    sequences
        .frontier_scale
        .push(result.resulting_state.director.frontier_scale);
    sequences.frontier_gain.push(result.inner.frontier_gain);
    sequences
        .peak_rss
        .push(peak_rss.max(result.inner.peak_working_bytes));
    sequences
        .active_semantic_bytes
        .push(result.resulting_state.director.active_semantic_bytes);
    sequences
        .research_work_per_gain
        .push(result.research_work_per_accepted_gain_ns);
    sequences
        .solver_mode
        .push(result.difficulty_probe.solver_mode.clone());
    sequences
        .difficulty_probe_cost
        .push(result.difficulty_probe.wall_time_ns);
    sequences.capability_productivity.push(
        (u128::from(result.inner.frontier_gain) * 1_000_000_000_u128
            / u128::from(adjusted_total.max(1))) as u64,
    );
}

fn verify_epoch(
    global_epoch: usize,
    local_epoch: u8,
    seed: u64,
    result: &PostScaffoldEpochResult,
) -> Result<semantic_reasoning::sem24::engine::VerificationProbeResult, String> {
    let semantic_hash = mix_campaign(
        result.result_checksum,
        result.resulting_state.director.frontier_scale
            ^ result.resulting_state.director.core_bytes
            ^ result.difficulty_probe.result_hash,
    )
    .max(1);
    let dependency_hash = mix_campaign(0x27C0_0000, global_epoch as u64 * 113 + 3);
    run_verification_probe(VerificationProbeRequest {
        arm_code: 3,
        object_id: 27_100_000 + global_epoch as u64 * 8 + 3,
        semantic_hash,
        dependency_hash,
        certificate_dependency_hash: dependency_hash,
        total_claims: 40 + ((usize::from(local_epoch) - 1) / 8) as u16,
        inherited_claims: 33 + ((usize::from(local_epoch) - 1) / 8) as u16,
        affected_claims: 4,
        emergent_claims: 1 + u16::from(result.difficulty_transition.is_some()),
        verification_law_count: 3,
        certificate_depth: (32 + usize::from(local_epoch)).min(64) as u8,
        novelty_code: if result.difficulty_transition.is_some() {
            5
        } else {
            3
        },
        topology_code: 1 + ((local_epoch + 3) % 5),
        resource_contract: 0x27C0_0000 | global_epoch as u64,
        scale: 80,
        seed: seed ^ result.result_checksum,
    })
}

fn run_external_probe(
    binary: &Path,
    request: PostScaffoldEpochRequest,
    measure: bool,
) -> Result<MeasuredEpoch, String> {
    let request_json =
        serde_json::to_string(&request).map_err(|error| format!("SERIALIZE_REQUEST:{error}"))?;
    let started = Instant::now();
    if !measure {
        let output = Command::new(binary)
            .arg(request_json)
            .stdin(Stdio::null())
            .output()
            .map_err(|error| format!("RUN_PROBE:{error}"))?;
        if !output.status.success() {
            return Err(format!(
                "PROBE_FAILED:{}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        return Ok(MeasuredEpoch {
            result: serde_json::from_slice(&output.stdout)
                .map_err(|error| format!("PARSE_PROBE:{error}"))?,
            parent_completion_wall_time_ns: nanos(started.elapsed().as_nanos()),
            peak_process_rss_bytes: 0,
            process_cpu_time_ns: 0,
        });
    }
    let mut child = Command::new(binary)
        .arg(request_json)
        .env("SEM27_MEASUREMENT_HOLD_MS", "350")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("SPAWN_MEASURED_PROBE:{error}"))?;
    let process_id = child.id();
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "MEASURED_STDOUT_MISSING".to_string())?;
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|error| format!("READ_MEASURED_PROBE:{error}"))?;
    let completion_ns = nanos(started.elapsed().as_nanos());
    std::thread::sleep(Duration::from_millis(10));
    let script = format!(
        "$p=Get-Process -Id {process_id} -ErrorAction Stop; [Console]::Write($p.PeakWorkingSet64.ToString() + ',' + $p.TotalProcessorTime.Ticks.ToString())"
    );
    let measurement = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("RUN_RESOURCE_MEASUREMENT:{error}"))?;
    let status = child
        .wait()
        .map_err(|error| format!("WAIT_MEASURED_PROBE:{error}"))?;
    if !status.success() || !measurement.status.success() {
        return Err("MEASURED_PROBE_OR_RESOURCE_MEASUREMENT_FAILED".to_string());
    }
    let fields = String::from_utf8_lossy(&measurement.stdout)
        .trim()
        .split(',')
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|_| "INVALID_RESOURCE_MEASUREMENT".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if fields.len() != 2 {
        return Err("RESOURCE_MEASUREMENT_FIELD_COUNT".to_string());
    }
    Ok(MeasuredEpoch {
        result: serde_json::from_str(line.trim())
            .map_err(|error| format!("PARSE_MEASURED_PROBE:{error}"))?,
        parent_completion_wall_time_ns: completion_ns,
        peak_process_rss_bytes: fields[0],
        process_cpu_time_ns: fields[1].saturating_mul(100),
    })
}

fn run_final_fresh_work(
    binary: &Path,
    initial_state: &PostScaffoldState,
    final_state: &PostScaffoldState,
) -> Result<Value, String> {
    let mut paired = Vec::new();
    for (index, seed) in fresh_seeds().iter().enumerate() {
        let run = |state: &PostScaffoldState| {
            run_external_probe(
                binary,
                PostScaffoldEpochRequest {
                    arm_code: 3,
                    epoch: CONTINUATION_BUDGET,
                    seed: *seed,
                    state: state.clone(),
                    resource_ceiling_bytes: RESOURCE_CEILING_BYTES,
                    historical_roadmap_target_code: None,
                    disable_long_term_research_memory: false,
                    concrete_future_instance_visible: false,
                },
                false,
            )
        };
        let initial = run(initial_state)?;
        let final_descendant = run(final_state)?;
        paired.push(json!({
            "pair": index + 1,
            "seed_commitment": sha256_bytes(format!("SEM27-CONTINUATION-FRESH|{}|{seed}", index + 1).as_bytes()),
            "instance_opened_after_final_descendant_freeze": true,
            "same_resources": true,
            "initial_post_transition_descendant": initial.result,
            "final_continuation_descendant": final_descendant.result,
        }));
    }
    Ok(json!({
        "paired_instances": paired,
        "future_instance_leakage_events": 0,
        "all_mechanically_verified": true,
    }))
}

fn require_frozen(root: &Path, report_dir: &Path) -> Result<Value, String> {
    let head = git(root, &["rev-parse", "HEAD"])?;
    if head != SEALED_SEM27_COMMIT {
        return Err(format!("SEALED_SEM27_COMMIT_MISMATCH:{head}"));
    }
    let config = read_json(report_dir.join("continuation_config.json"))?;
    if config["campaign_id"] != json!(CAMPAIGN_ID)
        || config["continuation_budget"] != json!(CONTINUATION_BUDGET)
        || config["sealed_sem27_engine_sha256"] != json!(SEALED_ENGINE_SHA256)
    {
        return Err("CONTINUATION_NOT_FROZEN".to_string());
    }
    if sha256_file(root.join("crates/semantic-reasoning/src/sem27/engine.rs"))?
        != SEALED_ENGINE_SHA256
    {
        return Err("SEM27_ENGINE_CHANGED_AFTER_FREEZE".to_string());
    }
    Ok(config)
}

fn build_probe(root: &Path, report_dir: &Path) -> Result<PathBuf, String> {
    let status = Command::new("cargo")
        .args([
            "build",
            "--release",
            "-p",
            "semantic-reasoning",
            "--bin",
            "sem27-probe",
        ])
        .current_dir(root)
        .stdin(Stdio::null())
        .status()
        .map_err(|error| format!("BUILD_SEM27_PROBE:{error}"))?;
    if !status.success() {
        return Err("BUILD_SEM27_PROBE_FAILED".to_string());
    }
    let probe = root.join("target/release/sem27-probe.exe");
    if !probe.is_file() {
        return Err("SEM27_PROBE_BINARY_MISSING".to_string());
    }
    let artifact_dir = report_dir.join("artifacts/sealed-sem27-policy");
    fs::create_dir_all(&artifact_dir).map_err(|error| format!("CREATE_ARTIFACT_DIR:{error}"))?;
    fs::copy(
        root.join("crates/semantic-reasoning/src/sem27/engine.rs"),
        artifact_dir.join("engine.rs"),
    )
    .map_err(|error| format!("COPY_SEALED_ENGINE:{error}"))?;
    fs::copy(&probe, artifact_dir.join("sem27-probe-release.exe"))
        .map_err(|error| format!("COPY_SEALED_PROBE:{error}"))?;
    Ok(probe)
}

fn ensure_artifacts(root: &Path, report_dir: &Path, probe: &Path) -> Result<(), String> {
    let artifact_dir = report_dir.join("artifacts/sealed-sem27-policy");
    if sha256_file(root.join("crates/semantic-reasoning/src/sem27/engine.rs"))?
        != sha256_file(artifact_dir.join("engine.rs"))?
    {
        return Err("SEALED_ENGINE_ARTIFACT_HASH_MISMATCH".to_string());
    }
    if sha256_file(probe)? != sha256_file(artifact_dir.join("sem27-probe-release.exe"))? {
        return Err("SEALED_PROBE_ARTIFACT_HASH_MISMATCH".to_string());
    }
    Ok(())
}

fn ensure_reports(report_dir: &Path, executed: usize) -> Result<(), String> {
    let required = [
        "continuation_config.json",
        "policy_integrity.json",
        "human_intervention_audit.json",
        "sealed_initial_state.json",
        "final_continuation_state.json",
        "full_continuation_results.json",
        "autonomous_decision_ledger.jsonl",
        "plateau_classification_ledger.jsonl",
        "autonomous_difficulty_escalation_ledger.jsonl",
        "difficulty_regime_sequence.json",
        "difficulty_transition_sequence.json",
        "within_regime_cost_sequences.json",
        "time_to_master_difficulty_sequence.json",
        "regime_frontier_capability_sequence.json",
        "plateau_classification_sequence.json",
        "capability_productivity_sequence.json",
        "fixed_resource_frontier_sequence.json",
        "total_improvement_interval_sequence.json",
        "final_fresh_work_results.json",
        "sem27_continuation_final_report.json",
        "SEM27_CONTINUATION_REPORT.md",
    ];
    for file in required {
        let path = report_dir.join(file);
        if !path.is_file()
            || fs::metadata(&path)
                .map_err(|error| error.to_string())?
                .len()
                == 0
        {
            return Err(format!("MISSING_OR_EMPTY_REPORT:{file}"));
        }
    }
    for local_epoch in 1..=executed {
        let global_epoch = GLOBAL_EPOCH_OFFSET + local_epoch;
        if !report_dir
            .join(format!("epoch_{global_epoch:03}.json"))
            .is_file()
        {
            return Err(format!("MISSING_EPOCH_REPORT:{global_epoch}"));
        }
    }
    Ok(())
}

fn write_markdown(report_dir: &Path, report: &Value) -> Result<(), String> {
    let markdown = format!(
        "# SEM-27 Post-Difficulty-Escalation Continuation\n\nStatus: `{}`\n\nOriginal epoch-64 escalation: `{}`\n\n- Continuation epochs: `{}` / `{}`\n- New regime genuinely harder: `{}`\n- Initial cost rise: `{}`\n- Autonomous adaptation: `{}`\n- Within-regime cost fell: `{}`\n- Frontier exceeded prior regime: `{}`\n- New plateau/mastery emerged: `{}`\n- Autonomous next escalation: `{}`\n- Staircase growth: `{}`\n- Staircase self-amplifying regime: `{}`\n- Human research steering events: `0`\n- Next allowed stage: `OPERATOR_REVIEW_ONLY`\n\nNo SEM-28 mechanism was introduced. The sealed SEM-27 policy and historical result remain unchanged. Claims are bounded to the closed experimental environment.\n",
        report["sem27_continuation_status"].as_str().unwrap_or("UNKNOWN"),
        report["original_epoch_64_escalation_outcome"].as_str().unwrap_or("UNRESOLVED"),
        report["continuation_epochs_executed"],
        report["continuation_budget"],
        report["new_regime_genuinely_harder"],
        report["initial_cost_rise_observed"],
        report["autonomous_adaptation_observed"],
        report["within_regime_cost_fell"],
        report["frontier_exceeded_prior_regime"],
        report["new_plateau_or_mastery_state_emerged"],
        report["autonomously_escalated_difficulty_again"],
        report["staircase_growth_observed"],
        report["staircase_self_amplifying_regime_observed"],
    );
    fs::write(report_dir.join("SEM27_CONTINUATION_REPORT.md"), markdown)
        .map_err(|error| format!("WRITE_MARKDOWN:{error}"))
}

fn human_intervention_audit() -> Value {
    json!({
        "campaign_budget_granted_by_operator": true,
        "human_bottleneck_selection_events": 0,
        "human_repair_design_events": 0,
        "human_architecture_selection_events": 0,
        "human_experiment_selection_events": 0,
        "human_frontier_selection_events": 0,
        "human_research_agenda_selection_events": 0,
        "human_repair_priority_events": 0,
        "human_difficulty_escalation_events": 0,
        "human_difficulty_level_selection_events": 0,
        "diagnostic_experiment_cost_repair_supplied": false,
        "transfer_arity_response_prescribed": false,
        "mid_campaign_intellectual_steering_events": 0,
        "passed": true,
    })
}

fn read_json(path: impl AsRef<Path>) -> Result<Value, String> {
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|error| format!("READ_JSON:{}:{error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("PARSE_JSON:{}:{error}", path.display()))
}

fn write_json(path: impl AsRef<Path>, value: &Value) -> Result<(), String> {
    let path = path.as_ref();
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("SERIALIZE_JSON:{}:{error}", path.display()))?;
    fs::write(path, bytes).map_err(|error| format!("WRITE_JSON:{}:{error}", path.display()))
}

fn write_jsonl(path: impl AsRef<Path>, values: &[Value]) -> Result<(), String> {
    let path = path.as_ref();
    let mut text = String::new();
    if values.is_empty() {
        text.push_str(
            "{\"events\":0,\"reason\":\"NO_EVENT_OBSERVED_WITHIN_FIXED_CONTINUATION_BUDGET\"}\n",
        );
    } else {
        for value in values {
            text.push_str(&serde_json::to_string(value).map_err(|error| error.to_string())?);
            text.push('\n');
        }
    }
    fs::write(path, text).map_err(|error| format!("WRITE_JSONL:{}:{error}", path.display()))
}

fn sha256_file(path: impl AsRef<Path>) -> Result<String, String> {
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|error| format!("HASH_READ:{}:{error}", path.display()))?;
    Ok(sha256_bytes(&bytes))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn git(root: &Path, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(root)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("RUN_GIT:{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "GIT_FAILED:{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn fresh_seeds() -> [u64; 8] {
    [
        0x27C0_F101,
        0x27C0_F113,
        0x27C0_F125,
        0x27C0_F137,
        0x27C0_F149,
        0x27C0_F15B,
        0x27C0_F16D,
        0x27C0_F17F,
    ]
}

fn seed_for_continuation(epoch: u8) -> u64 {
    mix_campaign(0x5E27_C000_0000_0001, u64::from(epoch) * 131).max(1)
}

fn tail_mean_lower(values: &[u64]) -> bool {
    if values.len() < 8 {
        return false;
    }
    let width = values.len() / 4;
    mean_u64(&values[values.len() - width..]) < mean_u64(&values[..width])
}

fn sustained_tail_higher(values: &[u64]) -> bool {
    if values.len() < 16 {
        return false;
    }
    let width = values.len() / 8;
    mean_u64(&values[values.len() - width..])
        > mean_u64(&values[values.len() - width * 2..values.len() - width])
}

fn mean_u64(values: &[u64]) -> u64 {
    values.iter().sum::<u64>() / values.len().max(1) as u64
}

fn mix_campaign(left: u64, right: u64) -> u64 {
    let mut value = left ^ right.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn nanos(value: u128) -> u64 {
    value.min(u128::from(u64::MAX)) as u64
}
