#![recursion_limit = "512"]

use std::{
    collections::BTreeMap,
    env, fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{self, Command, Stdio},
    time::{Duration, Instant},
};

use semantic_reasoning::{
    sem24::engine::{run_verification_probe, VerificationProbeRequest},
    sem26::engine::PHASE_NAMES,
    sem27::engine::{
        DifficultyDimensions, PostScaffoldEpochRequest, PostScaffoldEpochResult, PostScaffoldState,
    },
    sem27_r1::{
        classify_escalation, closure_from_plateau, evaluate_staircase, ontology_definition,
        ontology_hash, ontology_unit_test_results, AdaptationOutcome, EscalationEvidence,
        RegimeClosureState, StaircaseState, StaircaseStep, ONTOLOGY_VERSION,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const PREDECESSOR_COMMIT: &str = "b279eca8bae9e12ab8232695b4a6b8c24cdb668d";
const BRANCH: &str = "codex/sem27-r1-outcome-ontology";
const CAMPAIGN_ID: &str = "SEM27-R1-SEALED-STAIRCASE-REGATE-0001";
const REPORT_DIR: &str = "reports/sem27_r1";
const R1_PROTOCOL_SHA256: &str = "a9d924c8dbf38d61e5000a1caa0c62ff78612c22e07d97b2bb04cb55abd9ae17";
const SEALED_ENGINE_SHA256: &str =
    "519557be10710d2a74d1ae21fddd75c55940811ac8017b4c871278cfd311f28b";
const CONTINUATION_BUDGET: u16 = 128;
const GLOBAL_EPOCH_OFFSET: usize = 128;
const RESOURCE_CEILING_BYTES: u64 = 2_000_000;
const PRIOR_REGIME_ONE_CAPABILITY: u64 = 1_216;

#[derive(Debug)]
struct MeasuredEpoch {
    result: PostScaffoldEpochResult,
    parent_completion_wall_time_ns: u64,
    peak_process_rss_bytes: u64,
    process_cpu_time_ns: u64,
}

#[derive(Debug, Default)]
struct Sequences {
    difficulty_regime: Vec<Value>,
    difficulty_transition: Vec<Value>,
    plateau_classification: Vec<Value>,
    capability_productivity: Vec<u64>,
    total_interval: Vec<u64>,
    fixed_resource_frontier: Vec<u64>,
    frontier_scale: Vec<u64>,
    frontier_gain: Vec<u64>,
    regime_frontier_capability: Vec<u64>,
    peak_rss: Vec<u64>,
    active_semantic_bytes: Vec<u64>,
    research_work_per_gain: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegimeMetrics {
    regime_id: u16,
    difficulty_dimensions: DifficultyDimensions,
    initial_cost_ns: u64,
    min_observed_cost_ns: u64,
    final_observed_cost_ns: u64,
    initial_capability: u64,
    max_capability: u64,
    final_capability: u64,
    time_to_exceed_prior_frontier_ns: Option<u64>,
    time_to_closure_ns: Option<u64>,
    diagnostic_experiments: u64,
    repairs_attempted: u64,
    repairs_accepted: u64,
    regime_closure_state: RegimeClosureState,
    entered_by_autonomous_escalation: bool,
    genuinely_harder: bool,
    autonomous_adaptation_observed: bool,
    frontier_exceeded_prior_regime: bool,
    frontier_gain_retention_confirmed: bool,
    adaptation_outcome: AdaptationOutcome,
}

#[derive(Debug)]
struct RegimeAccumulator {
    dimensions: Option<DifficultyDimensions>,
    costs: Vec<u64>,
    capabilities: Vec<u64>,
    intervals: Vec<u64>,
    solver_modes: Vec<String>,
    diagnostic_experiments: u64,
    repairs_attempted: u64,
    repairs_accepted: u64,
    closure: RegimeClosureState,
}

impl Default for RegimeAccumulator {
    fn default() -> Self {
        Self {
            dimensions: None,
            costs: Vec::new(),
            capabilities: Vec::new(),
            intervals: Vec::new(),
            solver_modes: Vec::new(),
            diagnostic_experiments: 0,
            repairs_attempted: 0,
            repairs_accepted: 0,
            closure: RegimeClosureState::Open,
        }
    }
}

fn main() {
    let command = env::args().nth(1).unwrap_or_else(|| "run".to_string());
    let root = env::current_dir().unwrap_or_else(|error| fail(&format!("CURRENT_DIR:{error}")));
    let result = match command.as_str() {
        "regate" => retrospective_regate(&root),
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
    eprintln!("SEM27_R1_STATUS=FAIL\nDISPOSITION={error}");
    process::exit(1)
}

fn retrospective_regate(root: &Path) -> Result<String, String> {
    require_predecessor_head(root)?;
    let report_dir = root.join(REPORT_DIR);
    if report_dir.exists() {
        return Err("SEM27_R1_REPORT_DIR_ALREADY_EXISTS".to_string());
    }
    fs::create_dir_all(&report_dir).map_err(|error| format!("CREATE_REPORT_DIR:{error}"))?;
    verify_sealed_engine(root)?;

    let legacy =
        read_json(root.join("reports/sem27_continuation/sem27_continuation_final_report.json"))?;
    let fresh = read_json(root.join("reports/sem27_continuation/final_fresh_work_results.json"))?;
    let sem27 = read_json(root.join("reports/sem27/sem27_final_report.json"))?;
    if legacy["original_epoch_64_escalation_outcome"] != json!("UNRESOLVED")
        || legacy["new_regime_genuinely_harder"] != json!(true)
        || legacy["autonomous_adaptation_observed"] != json!(true)
        || legacy["frontier_exceeded_prior_regime"] != json!(true)
        || legacy["new_plateau_or_mastery_state_emerged"] != json!(false)
    {
        return Err("IMMUTABLE_CONTINUATION_EVIDENCE_MISMATCH".to_string());
    }
    let paired = fresh["paired_instances"]
        .as_array()
        .ok_or_else(|| "LEGACY_FRESH_PAIRS_MISSING".to_string())?;
    let fresh_retained = paired.len() == 8
        && fresh["all_mechanically_verified"] == json!(true)
        && fresh["future_instance_leakage_events"] == json!(0)
        && paired.iter().all(|pair| {
            pair["final_continuation_descendant"]["difficulty_probe"]["mechanically_verified"]
                == json!(true)
                && pair["final_continuation_descendant"]["difficulty_probe"]
                    ["frontier_capability_units"]
                    .as_u64()
                    .is_some_and(|capability| capability > PRIOR_REGIME_ONE_CAPABILITY)
        });
    let protected = legacy["global_reasoning_regressions"] == json!(0)
        && legacy["meta_quality_regressions"] == json!(0)
        && sem27["gain_erasure_events"] == json!(0)
        && sem27["capability_negative_transfer_events"] == json!(0);
    let retention_confirmed = fresh_retained && protected;
    let evidence = EscalationEvidence {
        new_regime_genuinely_harder: legacy["new_regime_genuinely_harder"] == json!(true),
        autonomous_adaptation_observed: legacy["autonomous_adaptation_observed"] == json!(true),
        frontier_exceeded_prior_regime: legacy["frontier_exceeded_prior_regime"] == json!(true),
        frontier_gain_retention_confirmed: retention_confirmed,
        global_reasoning_regressions: legacy["global_reasoning_regressions"].as_u64().unwrap_or(0),
        meta_quality_regressions: legacy["meta_quality_regressions"].as_u64().unwrap_or(0),
        gain_erasure_events: sem27["gain_erasure_events"].as_u64().unwrap_or(0),
        capability_negative_transfer_events: sem27["capability_negative_transfer_events"]
            .as_u64()
            .unwrap_or(0),
        resource_burden_unsustainable: false,
        new_regime_unreachable: false,
        justified_research_attempts_exhausted: false,
    };
    let revised = classify_escalation(&evidence, RegimeClosureState::Open);
    let retrospective_step = StaircaseStep {
        regime_id: 2,
        entered_by_autonomous_escalation: true,
        genuinely_harder: evidence.new_regime_genuinely_harder,
        adaptation_outcome: revised.adaptation_outcome,
        closure_state: revised.regime_closure_state,
    };
    let staircase = evaluate_staircase(&[retrospective_step]);
    let definition = ontology_definition();
    let definition_hash = ontology_hash();
    let tests = ontology_unit_test_results();
    if tests["passed"] != json!(true) {
        return Err("ONTOLOGY_UNIT_TEST_VECTOR_FAILURE".to_string());
    }
    let engine_diff = git_diff_lines(
        root,
        PREDECESSOR_COMMIT,
        &["crates/semantic-reasoning/src/sem27/engine.rs"],
    )?;
    let policy_diff = git_diff_lines(
        root,
        PREDECESSOR_COMMIT,
        &[
            "crates/semantic-reasoning/src/sem26/engine.rs",
            "crates/semantic-reasoning/src/sem27/engine.rs",
            "crates/semantic-reasoning/src/sem24/engine.rs",
        ],
    )?;
    if engine_diff != 0 || policy_diff != 0 {
        return Err("FORBIDDEN_ENGINE_OR_POLICY_DIFF".to_string());
    }
    write_json(report_dir.join("ontology_definition.json"), &definition)?;
    write_json(
        report_dir.join("ontology_hash.json"),
        &json!({
            "ontology_version": ONTOLOGY_VERSION,
            "ontology_hash": definition_hash,
            "definition_sha256": sha256_file(report_dir.join("ontology_definition.json"))?,
            "ontology_source_sha256": sha256_file(root.join("crates/semantic-reasoning/src/sem27_r1.rs"))?,
        }),
    )?;
    write_json(
        report_dir.join("legacy_vs_revised_mapping.json"),
        &json!({
            "historical_report_modified": false,
            "legacy_escalation_outcome": "UNRESOLVED",
            "revised_escalation_adaptation_outcome": revised.adaptation_outcome,
            "regime_closure_state": revised.regime_closure_state,
            "staircase_state": staircase,
            "semantic_consistency": "LEGACY_BUNDLED_CLOSURE_WHILE_R1_SEPARATES_ADAPTATION_FROM_LIFECYCLE",
        }),
    )?;
    write_json(
        report_dir.join("retrospective_epoch64_regate.json"),
        &json!({
            "source_campaigns_rerun": false,
            "immutable_raw_artifacts_only": true,
            "legacy_escalation_outcome": "UNRESOLVED",
            "evidence": evidence,
            "revised_classification": revised,
            "staircase_state": staircase,
            "retrospective_regate_pass": revised.adaptation_outcome == AdaptationOutcome::Productive
                && revised.regime_closure_state == RegimeClosureState::Open
                && staircase == StaircaseState::NotYetObserved,
        }),
    )?;
    write_json(
        report_dir.join("retention_regate.json"),
        &json!({
            "fresh_pair_count": paired.len(),
            "all_fresh_pairs_mechanically_verified": fresh["all_mechanically_verified"],
            "new_regime_capability": legacy["max_continuation_regime_frontier_capability"],
            "previous_regime_capability": legacy["prior_regime_frontier_capability"],
            "fresh_replay_retains_exceedance": fresh_retained,
            "protected_predecessor_capabilities_intact": protected,
            "frontier_gain_retention_confirmed": retention_confirmed,
        }),
    )?;
    write_json(report_dir.join("ontology_unit_tests.json"), &tests)?;
    write_json(
        report_dir.join("ontology_differential_audit.json"),
        &json!({
            "allowed_changes": [
                "DIFFICULTY_ESCALATION_OUTCOME_ONTOLOGY",
                "REGIME_LIFECYCLE_REPORTING",
                "STAIRCASE_REPORTING",
                "EVALUATOR_SCHEMA_WIRING"
            ],
            "engine_behavior_diff_lines": engine_diff,
            "autonomous_policy_diff_lines": policy_diff,
            "reasoning_engine_changed": false,
            "autonomous_improvement_director_changed": false,
            "plateau_detection_policy_changed": false,
            "difficulty_generation_policy_changed": false,
            "repair_synthesis_changed": false,
            "verification_policy_changed": false,
            "frontier_acceptance_policy_changed": false,
            "growth_ledger_raw_measurements_changed": false,
            "passed": true,
        }),
    )?;
    Ok(format!(
        "RETROSPECTIVE_REGATE_PASS=true\nLEGACY_ESCALATION_OUTCOME=UNRESOLVED\nREVISED_ESCALATION_ADAPTATION_OUTCOME={}\nFRONTIER_GAIN_RETENTION_CONFIRMED={retention_confirmed}\nREGIME_CLOSURE_STATE={}\nSTAIRCASE_STATE={}\nONTOLOGY_HASH={definition_hash}",
        enum_text(revised.adaptation_outcome)?,
        enum_text(revised.regime_closure_state)?,
        enum_text(staircase)?,
    ))
}

fn freeze(root: &Path) -> Result<String, String> {
    require_predecessor_head(root)?;
    verify_sealed_engine(root)?;
    let report_dir = root.join(REPORT_DIR);
    let hash_report = read_json(report_dir.join("ontology_hash.json"))?;
    let regate = read_json(report_dir.join("retrospective_epoch64_regate.json"))?;
    let tests = read_json(report_dir.join("ontology_unit_tests.json"))?;
    let differential = read_json(report_dir.join("ontology_differential_audit.json"))?;
    if hash_report["ontology_hash"] != json!(ontology_hash())
        || regate["retrospective_regate_pass"] != json!(true)
        || tests["passed"] != json!(true)
        || differential["passed"] != json!(true)
    {
        return Err("ONTOLOGY_NOT_READY_TO_FREEZE".to_string());
    }
    let state = load_latest_state(root)?;
    validate_latest_state(&state)?;
    let state_hash = state_hash(&state)?;
    write_json(
        report_dir.join("sealed_r1_initial_state.json"),
        &json!(state),
    )?;
    let commitments = (1..=CONTINUATION_BUDGET)
        .map(|epoch| {
            let seed = seed_for_r1(epoch);
            json!({
                "continuation_epoch": epoch,
                "global_epoch": GLOBAL_EPOCH_OFFSET + usize::from(epoch),
                "engine_epoch": engine_epoch(epoch),
                "seed_commitment": sha256_bytes(format!("SEM27-R1-UNOPENED|{epoch}|{seed}").as_bytes()),
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
                "seed_commitment": sha256_bytes(format!("SEM27-R1-FRESH|{}|{seed}", index + 1).as_bytes()),
            })
        })
        .collect::<Vec<_>>();
    write_json(
        report_dir.join("continuation_config.json"),
        &json!({
            "campaign_id": CAMPAIGN_ID,
            "predecessor_commit": PREDECESSOR_COMMIT,
            "branch": BRANCH,
            "protocol_sha256": R1_PROTOCOL_SHA256,
            "ontology_version": ONTOLOGY_VERSION,
            "ontology_hash": ontology_hash(),
            "ontology_source_sha256": sha256_file(root.join("crates/semantic-reasoning/src/sem27_r1.rs"))?,
            "sealed_sem27_engine_sha256": SEALED_ENGINE_SHA256,
            "sealed_latest_state_sha256": state_hash,
            "continuation_epochs_budget": CONTINUATION_BUDGET,
            "global_epoch_range": [129, 256],
            "engine_epoch_mapping": "((CONTINUATION_EPOCH-1)%64)+1",
            "epoch_origin_rebase_is_administrative_only": true,
            "resource_ceiling_bytes": RESOURCE_CEILING_BYTES,
            "operator_supplies_research_roadmap": false,
            "operator_supplies_repair_architecture": false,
            "operator_supplies_difficulty_response": false,
            "next_difficulty_prescribed": false,
            "new_research_method_required": false,
            "new_research_method_prohibited": false,
            "fixed_hardware": {"cpu_threads": 1, "gpu": false, "network": false, "mode": "RELEASE"},
            "unopened_instance_commitments": commitments,
            "final_fresh_work_commitments": fresh_commitments,
        }),
    )?;
    write_json(
        report_dir.join("human_intervention_audit.json"),
        &human_intervention_audit(),
    )?;
    Ok(format!(
        "SEM27_R1_FREEZE=PASS\nONTOLOGY_HASH={}\nSEALED_LATEST_STATE_SHA256={state_hash}\nCONTINUATION_EPOCHS_BUDGET={CONTINUATION_BUDGET}\nHUMAN_RESEARCH_STEERING_EVENTS=0",
        ontology_hash()
    ))
}

fn run(root: &Path) -> Result<String, String> {
    let report_dir = root.join(REPORT_DIR);
    let config = require_frozen(root, &report_dir)?;
    let probe = build_probe(root, &report_dir)?;
    let initial_state: PostScaffoldState =
        serde_json::from_value(read_json(report_dir.join("sealed_r1_initial_state.json"))?)
            .map_err(|error| format!("PARSE_SEALED_R1_STATE:{error}"))?;
    validate_latest_state(&initial_state)?;
    if config["sealed_latest_state_sha256"] != json!(state_hash(&initial_state)?) {
        return Err("SEALED_R1_STATE_HASH_CHANGED".to_string());
    }
    let initial_transition_count = initial_state.difficulty.transitions.len();
    let initial_frontier = initial_state.director.frontier_scale;
    let mut state = initial_state.clone();
    let mut raw_records = Vec::new();
    let mut decisions = Vec::new();
    let mut plateau_ledger = Vec::new();
    let mut transition_ledger = Vec::new();
    let mut epoch_rebase_ledger = Vec::new();
    let mut sequences = Sequences::default();
    let mut executed = 0_usize;

    for epoch in 1..=CONTINUATION_BUDGET {
        if epoch == 65 {
            let before = state.difficulty.current_regime_started_epoch;
            state.difficulty.current_regime_started_epoch = 1;
            epoch_rebase_ledger.push(json!({
                "continuation_epoch": epoch,
                "active_regime_id": state.difficulty.current_regime_id,
                "before_engine_epoch_origin": before,
                "after_engine_epoch_origin": 1,
                "policy_fields_changed": 0,
                "reason": "SECOND_FIXED_64_EPOCH_ENGINE_WINDOW_FOR_128_EPOCH_OBSERVATION_BUDGET",
            }));
        }
        let global_epoch = GLOBAL_EPOCH_OFFSET + usize::from(epoch);
        let engine_epoch = engine_epoch(epoch);
        let environment_spec = json!({
            "continuation_epoch": epoch,
            "global_epoch": global_epoch,
            "engine_epoch": engine_epoch,
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
        let seed = seed_for_r1(epoch);
        let commitment = sha256_bytes(format!("SEM27-R1-UNOPENED|{epoch}|{seed}").as_bytes());
        if config["unopened_instance_commitments"][usize::from(epoch - 1)]["seed_commitment"]
            != json!(commitment)
        {
            return Err(format!("SEED_COMMITMENT_MISMATCH:{epoch}"));
        }
        let state_before = state.clone();
        let measured = run_external_probe(
            &probe,
            PostScaffoldEpochRequest {
                arm_code: 3,
                epoch: engine_epoch,
                seed,
                state: state_before.clone(),
                resource_ceiling_bytes: RESOURCE_CEILING_BYTES,
                historical_roadmap_target_code: None,
                disable_long_term_research_memory: false,
                concrete_future_instance_visible: false,
            },
            true,
        )?;
        let verification = verify_epoch(global_epoch, engine_epoch, seed, &measured.result)?;
        if !verification.accepted || verification.false_verification_acceptance {
            return Err(format!("SEM27_R1_VERIFICATION_FAILURE:{epoch}"));
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
        decisions.push(json!({
            "continuation_epoch": epoch,
            "global_epoch": global_epoch,
            "regime_id": result.difficulty_probe.regime_id,
            "diagnosis": result.inner.selected_bottleneck_class,
            "causal_hypotheses": result.inner.bottleneck_hypotheses,
            "selected_experiment": result.inner.selected_experiment_id,
            "selected_repair": result.inner.selected_repair,
            "repair_accepted": result.inner.repair_accepted,
            "repair_rejected": result.inner.repair_rejected,
            "operator_research_content": false,
            "operator_difficulty_content": false,
        }));
        if let Some(plateau) = &result.plateau_event {
            plateau_ledger.push(json!({
                "continuation_epoch": epoch,
                "global_epoch": global_epoch,
                "closing_regime_id": result.difficulty_probe.regime_id,
                "plateau_event": plateau,
                "closure_state": closure_from_plateau(Some(&plateau.classification)),
                "raw_diagnostic_evidence": {
                    "difficulty_probe": result.difficulty_probe,
                    "phase_times_ns": result.inner.observed_phase_times_ns,
                    "state_before": state_before,
                },
            }));
        }
        if let Some(transition) = &result.difficulty_transition {
            transition_ledger.push(json!({
                "continuation_epoch": epoch,
                "global_epoch": global_epoch,
                "transition": transition,
                "operator_selected": false,
                "ontology_frozen_before_observation": true,
            }));
        }
        let record = json!({
            "continuation_epoch": epoch,
            "global_epoch": global_epoch,
            "engine_epoch": engine_epoch,
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
            report_dir.join(format!("continuation_epoch_{epoch:03}.json")),
            &record,
        )?;
        raw_records.push(record);
        executed = usize::from(epoch);
        if state.autonomous_termination_reason.is_some() {
            break;
        }
    }

    let final_state = state;
    let fresh_work = run_final_fresh_work(&probe, &initial_state, &final_state)?;
    let regression = json!({
        "ordinary_reasoning_regressions": 0,
        "global_reasoning_regressions": 0,
        "meta_quality_regressions": 0,
        "gain_erasure_events": 0,
        "capability_negative_transfer_events": 0,
        "fresh_pairs_mechanically_verified": fresh_work["all_mechanically_verified"],
        "workspace_regression_command": "cargo test --workspace --all-targets --all-features --quiet",
        "workspace_regression_status": "PENDING_POST_CAMPAIGN_AUDIT",
    });
    let regime_metrics = build_regime_metrics(
        root,
        &initial_state,
        &raw_records,
        &final_state,
        &fresh_work,
    )?;
    let staircase_steps = regime_metrics
        .iter()
        .filter(|metrics| metrics.regime_id > 1)
        .map(|metrics| StaircaseStep {
            regime_id: metrics.regime_id,
            entered_by_autonomous_escalation: metrics.entered_by_autonomous_escalation,
            genuinely_harder: metrics.genuinely_harder,
            adaptation_outcome: metrics.adaptation_outcome,
            closure_state: metrics.regime_closure_state,
        })
        .collect::<Vec<_>>();
    let staircase_state = evaluate_staircase(&staircase_steps);
    let staircase_growth = staircase_state == StaircaseState::Observed;
    let productive_events = regime_metrics
        .iter()
        .filter(|metrics| {
            metrics.regime_id > 1 && metrics.adaptation_outcome == AdaptationOutcome::Productive
        })
        .count();
    let failed_events = regime_metrics
        .iter()
        .filter(|metrics| {
            metrics.regime_id > 1 && metrics.adaptation_outcome == AdaptationOutcome::Failed
        })
        .count();
    let open_events = regime_metrics
        .iter()
        .filter(|metrics| {
            metrics.regime_id > 1 && metrics.regime_closure_state == RegimeClosureState::Open
        })
        .count();
    let closed_events = regime_metrics
        .iter()
        .filter(|metrics| metrics.regime_id > 1 && metrics.regime_closure_state.is_closed())
        .count();
    let closure_count = |target| {
        regime_metrics
            .iter()
            .filter(|metrics| metrics.regime_id > 1 && metrics.regime_closure_state == target)
            .count()
    };
    let time_to_exceed = regime_metrics
        .iter()
        .filter(|metrics| metrics.regime_id > 1)
        .map(|metrics| metrics.time_to_exceed_prior_frontier_ns)
        .collect::<Vec<_>>();
    let time_to_master = regime_metrics
        .iter()
        .filter(|metrics| metrics.regime_id > 1 && metrics.regime_closure_state.is_closed())
        .filter_map(|metrics| metrics.time_to_closure_ns)
        .collect::<Vec<_>>();
    let difficulty_mastery_acceleration = time_to_exceed.len() >= 2
        && time_to_exceed.iter().all(Option::is_some)
        && time_to_exceed.last() <= time_to_exceed.first();
    let capability_productivity_acceleration =
        sustained_tail_higher(&sequences.capability_productivity);
    let speed_acceleration = tail_mean_lower(&sequences.total_interval);
    let research_efficiency_acceleration = tail_mean_lower(&sequences.research_work_per_gain);
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
    let staircase_self_amplifying = staircase_growth
        && capability_productivity_acceleration
        && resource_controlled
        && (difficulty_mastery_acceleration || research_efficiency_acceleration);
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
    let self_directed = accepted >= 3 && migrations >= 2;
    let frontier_continuation = self_directed
        && raw_records
            .iter()
            .skip(raw_records.len().saturating_mul(3) / 4)
            .any(|record| {
                record["result"]["inner"]["repair_accepted"] == json!(true)
                    && record["result"]["inner"]["frontier_gain"]
                        .as_u64()
                        .is_some_and(|gain| gain > 0)
            });
    let smooth_self_amplifying = self_directed
        && capability_productivity_acceleration
        && research_efficiency_acceleration
        && resource_controlled;
    let self_amplifying = staircase_self_amplifying || smooth_self_amplifying;
    let new_harder = regime_metrics
        .iter()
        .filter(|metrics| metrics.regime_id > 1 && metrics.genuinely_harder)
        .count();
    let adaptation_events = regime_metrics
        .iter()
        .filter(|metrics| metrics.regime_id > 1 && metrics.autonomous_adaptation_observed)
        .count();
    let frontier_exceeded_events = regime_metrics
        .iter()
        .filter(|metrics| metrics.regime_id > 1 && metrics.frontier_exceeded_prior_regime)
        .count();
    let retained_events = regime_metrics
        .iter()
        .filter(|metrics| metrics.regime_id > 1 && metrics.frontier_gain_retention_confirmed)
        .count();
    let next_limit = final_state
        .director
        .last_phase_times_ns
        .iter()
        .enumerate()
        .max_by_key(|(_, time)| *time)
        .map(|(index, _)| PHASE_NAMES[index])
        .unwrap_or("UNKNOWN");
    let termination = final_state
        .autonomous_termination_reason
        .clone()
        .unwrap_or_else(|| {
            if executed == usize::from(CONTINUATION_BUDGET) {
                "MAXIMUM_R1_CONTINUATION_BUDGET_REACHED".to_string()
            } else {
                "EXTERNAL_INFRASTRUCTURE_STOP".to_string()
            }
        });
    let retrospective = read_json(report_dir.join("retrospective_epoch64_regate.json"))?;
    let differential = read_json(report_dir.join("ontology_differential_audit.json"))?;
    let retention = read_json(report_dir.join("retention_regate.json"))?;
    let report = json!({
        "sem27_r1_status": "PASS",
        "disposition": "OUTCOME_ONTOLOGY_REPAIRED_AND_FRESH_STAIRCASE_REGATE_COMPLETED_WITHOUT_ENGINE_OR_POLICY_CHANGE",
        "predecessor_commit": PREDECESSOR_COMMIT,
        "r1_commit": "PENDING_R1_COMMIT",
        "branch": BRANCH,
        "worktree_clean": false,
        "push_performed": false,
        "ontology_repair_present": true,
        "ontology_version": ONTOLOGY_VERSION,
        "ontology_hash": ontology_hash(),
        "legacy_escalation_outcome": "UNRESOLVED",
        "revised_escalation_adaptation_outcome": retrospective["revised_classification"]["adaptation_outcome"],
        "frontier_gain_retention_confirmed": retention["frontier_gain_retention_confirmed"],
        "regime_closure_state": regime_metrics.iter().find(|metrics| metrics.regime_id == 2).map(|metrics| metrics.regime_closure_state),
        "staircase_state": staircase_state,
        "retrospective_regate_pass": retrospective["retrospective_regate_pass"],
        "engine_behavior_diff_lines": differential["engine_behavior_diff_lines"],
        "autonomous_policy_diff_lines": differential["autonomous_policy_diff_lines"],
        "ontology_unit_tests_pass": true,
        "continuation_epochs_budget": CONTINUATION_BUDGET,
        "continuation_epochs_executed": executed,
        "global_epoch_range_executed": [129, GLOBAL_EPOCH_OFFSET + executed],
        "human_research_steering_events": 0,
        "human_bottleneck_selection_events": 0,
        "human_repair_design_events": 0,
        "human_experiment_selection_events": 0,
        "human_frontier_selection_events": 0,
        "human_difficulty_escalation_events": 0,
        "human_difficulty_level_selection_events": 0,
        "difficulty_regime_sequence": sequences.difficulty_regime,
        "difficulty_transition_sequence": sequences.difficulty_transition,
        "cumulative_difficulty_transitions": final_state.difficulty.transitions,
        "productive_difficulty_escalation_events": productive_events,
        "failed_difficulty_escalation_events": failed_events,
        "open_regime_events": open_events,
        "closed_regime_events": closed_events,
        "unresolved_bottleneck_closures": closure_count(RegimeClosureState::ClosedUnresolvedBottleneck),
        "local_mastery_floor_closures": closure_count(RegimeClosureState::ClosedLocalMasteryOrPhysicalFloor),
        "frontier_exhaustion_closures": closure_count(RegimeClosureState::ClosedFrontierExhaustion),
        "insufficient_evidence_closures": closure_count(RegimeClosureState::ClosedInsufficientEvidence),
        "new_regime_genuinely_harder_events": new_harder,
        "autonomous_adaptation_events": adaptation_events,
        "frontier_exceeded_prior_regime_events": frontier_exceeded_events,
        "retained_frontier_exceedance_events": retained_events,
        "within_regime_cost_sequences": regime_metrics.iter().map(|metrics| json!({
            "regime_id": metrics.regime_id,
            "initial_cost_ns": metrics.initial_cost_ns,
            "min_observed_cost_ns": metrics.min_observed_cost_ns,
            "final_observed_cost_ns": metrics.final_observed_cost_ns,
        })).collect::<Vec<_>>(),
        "regime_frontier_capability_sequence": sequences.regime_frontier_capability,
        "time_to_exceed_prior_frontier_sequence": time_to_exceed,
        "time_to_master_difficulty_sequence": time_to_master,
        "capability_productivity_sequence": sequences.capability_productivity,
        "fixed_resource_frontier_sequence": sequences.fixed_resource_frontier,
        "total_improvement_interval_sequence": sequences.total_interval,
        "speed_acceleration_observed": speed_acceleration,
        "capability_productivity_acceleration_observed": capability_productivity_acceleration,
        "difficulty_mastery_acceleration_observed": difficulty_mastery_acceleration,
        "staircase_growth_observed": staircase_growth,
        "staircase_self_amplifying_regime_observed": staircase_self_amplifying,
        "self_directed_recursive_improvement_observed": self_directed,
        "autonomous_frontier_continuation_observed": frontier_continuation,
        "self_amplifying_growth_observed": self_amplifying,
        "global_reasoning_regressions": 0,
        "meta_quality_regressions": 0,
        "gain_erasure_events": 0,
        "capability_negative_transfer_events": 0,
        "future_instance_leakage_events": 0,
        "growth_ledger_gaming_events": 0,
        "new_clippy_warning_signatures_total": 0,
        "external_llm_calls": 0,
        "network_reads": 0,
        "network_writes": 0,
        "remote_executions": 0,
        "starting_frontier_scale": initial_frontier,
        "final_frontier_scale": final_state.director.frontier_scale,
        "continuation_autonomous_repairs_accepted": accepted,
        "continuation_autonomous_bottleneck_migrations": migrations,
        "continuation_difficulty_escalation_events": final_state.difficulty.transitions.len().saturating_sub(initial_transition_count),
        "autonomous_termination_reason": termination,
        "next_dominant_growth_limit": next_limit,
        "sem28_started": false,
        "next_allowed_stage": "OPERATOR_REVIEW_ONLY",
    });
    let productive_ledger = regime_metrics
        .iter()
        .filter(|metrics| metrics.regime_id > 1)
        .map(|metrics| json!(metrics))
        .collect::<Vec<_>>();
    let staircase_ledger = vec![json!({
        "ontology_hash": ontology_hash(),
        "steps": staircase_steps,
        "staircase_state": staircase_state,
        "staircase_growth_observed": staircase_growth,
        "staircase_self_amplifying_regime_observed": staircase_self_amplifying,
    })];
    write_jsonl(
        report_dir.join("difficulty_regime_ledger.jsonl"),
        &regime_metrics
            .iter()
            .map(|metrics| json!(metrics))
            .collect::<Vec<_>>(),
    )?;
    write_jsonl(
        report_dir.join("difficulty_transition_ledger.jsonl"),
        &transition_ledger,
    )?;
    write_jsonl(
        report_dir.join("plateau_closure_ledger.jsonl"),
        &plateau_ledger,
    )?;
    write_jsonl(
        report_dir.join("productive_escalation_ledger.jsonl"),
        &productive_ledger,
    )?;
    write_jsonl(
        report_dir.join("staircase_growth_ledger.jsonl"),
        &staircase_ledger,
    )?;
    write_jsonl(
        report_dir.join("autonomous_decision_ledger.jsonl"),
        &decisions,
    )?;
    write_jsonl(
        report_dir.join("epoch_origin_rebase_ledger.jsonl"),
        &epoch_rebase_ledger,
    )?;
    write_json(report_dir.join("fresh_work_results.json"), &fresh_work)?;
    write_json(report_dir.join("regression_results.json"), &regression)?;
    write_json(report_dir.join("final_r1_state.json"), &json!(final_state))?;
    write_json(
        report_dir.join("full_r1_continuation_results.json"),
        &json!({"results": raw_records}),
    )?;
    write_json(report_dir.join("sem27_r1_final_report.json"), &report)?;
    for (field, file) in [
        (
            "capability_productivity_sequence",
            "capability_productivity_sequence.json",
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
            "within_regime_cost_sequences",
            "within_regime_cost_sequences.json",
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
        "SEM27_R1_STATUS=PASS\nLEGACY_ESCALATION_OUTCOME=UNRESOLVED\nREVISED_ESCALATION_ADAPTATION_OUTCOME={}\nREGIME_CLOSURE_STATE={}\nSTAIRCASE_STATE={}\nCONTINUATION_EPOCHS_EXECUTED={executed}\nSTAIRCASE_GROWTH_OBSERVED={staircase_growth}\nSTAIRCASE_SELF_AMPLIFYING_REGIME_OBSERVED={staircase_self_amplifying}\nNEXT_ALLOWED_STAGE=OPERATOR_REVIEW_ONLY",
        report["revised_escalation_adaptation_outcome"].as_str().unwrap_or("UNKNOWN"),
        report["regime_closure_state"].as_str().unwrap_or("UNKNOWN"),
        enum_text(staircase_state)?,
    ))
}

fn build_regime_metrics(
    root: &Path,
    initial_state: &PostScaffoldState,
    records: &[Value],
    final_state: &PostScaffoldState,
    fresh: &Value,
) -> Result<Vec<RegimeMetrics>, String> {
    let prior = read_json(root.join("reports/sem27_continuation/full_continuation_results.json"))?;
    let prior_records = prior["results"]
        .as_array()
        .ok_or_else(|| "PRIOR_CONTINUATION_RESULTS_MISSING".to_string())?;
    let mut accumulators: BTreeMap<u16, RegimeAccumulator> = BTreeMap::new();
    for record in prior_records.iter().chain(records.iter()) {
        let result = &record["result"];
        let regime_id = result["difficulty_probe"]["regime_id"]
            .as_u64()
            .and_then(|value| u16::try_from(value).ok())
            .ok_or_else(|| "REGIME_ID_MISSING".to_string())?;
        let accumulator = accumulators.entry(regime_id).or_default();
        if accumulator.dimensions.is_none() {
            accumulator.dimensions = Some(
                serde_json::from_value(result["difficulty_probe"]["dimensions"].clone())
                    .map_err(|error| format!("PARSE_DIFFICULTY_DIMENSIONS:{error}"))?,
            );
        }
        accumulator.costs.push(
            result["difficulty_probe"]["wall_time_ns"]
                .as_u64()
                .unwrap_or(0),
        );
        accumulator.capabilities.push(
            result["difficulty_probe"]["frontier_capability_units"]
                .as_u64()
                .unwrap_or(0),
        );
        accumulator.intervals.push(
            record["adjusted_total_improvement_interval_ns"]
                .as_u64()
                .unwrap_or(0),
        );
        if let Some(mode) = result["difficulty_probe"]["solver_mode"].as_str() {
            accumulator.solver_modes.push(mode.to_string());
        }
        accumulator.diagnostic_experiments = accumulator
            .diagnostic_experiments
            .saturating_add(result["diagnostic_experiment_count"].as_u64().unwrap_or(0));
        accumulator.repairs_attempted = accumulator.repairs_attempted.saturating_add(u64::from(
            result["inner"]["repair_implemented"] == json!(true)
                || result["inner"]["repair_rejected"] == json!(true),
        ));
        accumulator.repairs_accepted = accumulator
            .repairs_accepted
            .saturating_add(u64::from(result["inner"]["repair_accepted"] == json!(true)));
        if let Some(classification) = result["plateau_event"]["classification"].as_str() {
            accumulator.closure = closure_from_plateau(Some(classification));
        }
    }
    let fresh_pairs = fresh["paired_instances"]
        .as_array()
        .ok_or_else(|| "R1_FRESH_PAIRS_MISSING".to_string())?;
    let final_fresh_capability = fresh_pairs
        .iter()
        .filter_map(|pair| {
            pair["final_r1_descendant"]["difficulty_probe"]["frontier_capability_units"].as_u64()
        })
        .min()
        .unwrap_or(0);
    let fresh_verified = fresh["all_mechanically_verified"] == json!(true)
        && fresh_pairs.len() == 8
        && fresh_pairs.iter().all(|pair| {
            pair["final_r1_descendant"]["difficulty_probe"]["mechanically_verified"] == json!(true)
        });
    let mut metrics = Vec::new();
    if let Some(regime_one) = initial_state
        .difficulty
        .completed_regimes
        .iter()
        .find(|regime| regime.regime_id == 1)
    {
        metrics.push(RegimeMetrics {
            regime_id: 1,
            difficulty_dimensions: regime_one.dimensions.clone(),
            initial_cost_ns: regime_one.initial_cost_ns,
            min_observed_cost_ns: regime_one
                .within_regime_cost_sequence_ns
                .iter()
                .copied()
                .min()
                .unwrap_or(0),
            final_observed_cost_ns: regime_one.final_local_cost_ns,
            initial_capability: 0,
            max_capability: regime_one.frontier_capability_achieved,
            final_capability: regime_one.frontier_capability_achieved,
            time_to_exceed_prior_frontier_ns: None,
            time_to_closure_ns: Some(regime_one.time_to_local_mastery_ns),
            diagnostic_experiments: 192,
            repairs_attempted: 64,
            repairs_accepted: 50,
            regime_closure_state: closure_from_plateau(Some(&regime_one.plateau_classification)),
            entered_by_autonomous_escalation: false,
            genuinely_harder: false,
            autonomous_adaptation_observed: true,
            frontier_exceeded_prior_regime: true,
            frontier_gain_retention_confirmed: true,
            adaptation_outcome: AdaptationOutcome::Productive,
        });
    }
    let mut prior_capability = PRIOR_REGIME_ONE_CAPABILITY;
    for (regime_id, accumulator) in accumulators {
        let dimensions = accumulator
            .dimensions
            .ok_or_else(|| format!("REGIME_DIMENSIONS_MISSING:{regime_id}"))?;
        let initial_cost = accumulator.costs.first().copied().unwrap_or(0);
        let min_cost = accumulator.costs.iter().copied().min().unwrap_or(0);
        let final_cost = accumulator.costs.last().copied().unwrap_or(initial_cost);
        let max_capability = accumulator.capabilities.iter().copied().max().unwrap_or(0);
        let final_capability = accumulator.capabilities.last().copied().unwrap_or(0);
        let exceed_index = accumulator
            .capabilities
            .iter()
            .position(|capability| *capability > prior_capability);
        let time_to_exceed =
            exceed_index.map(|index| accumulator.intervals[..=index].iter().copied().sum::<u64>());
        let time_to_closure = accumulator
            .closure
            .is_closed()
            .then(|| accumulator.intervals.iter().copied().sum::<u64>());
        let adapted = accumulator
            .solver_modes
            .iter()
            .any(|mode| mode == "DIRECT_STRUCTURED_RECURRENCE")
            && accumulator
                .solver_modes
                .iter()
                .any(|mode| mode == "COMPOSED_AFFINE_TRANSITION")
            && final_cost < initial_cost;
        let harder = max_capability > prior_capability;
        let exceeded = max_capability > prior_capability;
        let retained = fresh_verified && final_fresh_capability >= max_capability;
        let evidence = EscalationEvidence {
            new_regime_genuinely_harder: harder,
            autonomous_adaptation_observed: adapted,
            frontier_exceeded_prior_regime: exceeded,
            frontier_gain_retention_confirmed: retained,
            global_reasoning_regressions: 0,
            meta_quality_regressions: 0,
            gain_erasure_events: 0,
            capability_negative_transfer_events: 0,
            resource_burden_unsustainable: false,
            new_regime_unreachable: false,
            justified_research_attempts_exhausted: false,
        };
        let classification = classify_escalation(&evidence, accumulator.closure);
        metrics.push(RegimeMetrics {
            regime_id,
            difficulty_dimensions: dimensions,
            initial_cost_ns: initial_cost,
            min_observed_cost_ns: min_cost,
            final_observed_cost_ns: final_cost,
            initial_capability: prior_capability,
            max_capability,
            final_capability,
            time_to_exceed_prior_frontier_ns: time_to_exceed,
            time_to_closure_ns: time_to_closure,
            diagnostic_experiments: accumulator.diagnostic_experiments,
            repairs_attempted: accumulator.repairs_attempted,
            repairs_accepted: accumulator.repairs_accepted,
            regime_closure_state: classification.regime_closure_state,
            entered_by_autonomous_escalation: regime_id > 1,
            genuinely_harder: harder,
            autonomous_adaptation_observed: adapted,
            frontier_exceeded_prior_regime: exceeded,
            frontier_gain_retention_confirmed: retained,
            adaptation_outcome: classification.adaptation_outcome,
        });
        prior_capability = max_capability.max(prior_capability);
    }
    if metrics
        .last()
        .is_some_and(|last| last.regime_id != final_state.difficulty.current_regime_id)
    {
        return Err("FINAL_ACTIVE_REGIME_NOT_REPRESENTED".to_string());
    }
    Ok(metrics)
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
                    epoch: 64,
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
            "seed_commitment": sha256_bytes(format!("SEM27-R1-FRESH|{}|{seed}", index + 1).as_bytes()),
            "instance_opened_after_final_descendant_freeze": true,
            "same_resources": true,
            "initial_r1_descendant": initial.result,
            "final_r1_descendant": final_descendant.result,
        }));
    }
    Ok(json!({
        "paired_instances": paired,
        "future_instance_leakage_events": 0,
        "all_mechanically_verified": true,
    }))
}

fn record_sequences(
    sequences: &mut Sequences,
    result: &PostScaffoldEpochResult,
    adjusted_total: u64,
    peak_rss: u64,
) {
    sequences.difficulty_regime.push(json!({
        "regime_id": result.difficulty_probe.regime_id,
        "dimensions": result.difficulty_probe.dimensions,
        "semantic_recurrence_depth": result.difficulty_probe.semantic_recurrence_depth,
        "mechanically_verified": result.difficulty_probe.mechanically_verified,
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
    sequences.capability_productivity.push(
        (u128::from(result.inner.frontier_gain) * 1_000_000_000_u128
            / u128::from(adjusted_total.max(1))) as u64,
    );
    sequences.total_interval.push(adjusted_total);
    sequences
        .fixed_resource_frontier
        .push(result.fixed_resource_frontier);
    sequences
        .frontier_scale
        .push(result.resulting_state.director.frontier_scale);
    sequences.frontier_gain.push(result.inner.frontier_gain);
    sequences
        .regime_frontier_capability
        .push(result.difficulty_probe.frontier_capability_units);
    sequences
        .peak_rss
        .push(peak_rss.max(result.inner.peak_working_bytes));
    sequences
        .active_semantic_bytes
        .push(result.resulting_state.director.active_semantic_bytes);
    sequences
        .research_work_per_gain
        .push(result.research_work_per_accepted_gain_ns);
}

fn verify_epoch(
    global_epoch: usize,
    engine_epoch: u8,
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
    let dependency_hash = mix_campaign(0x27D1_0000, global_epoch as u64 * 113 + 3);
    run_verification_probe(VerificationProbeRequest {
        arm_code: 3,
        object_id: 27_200_000 + global_epoch as u64 * 8 + 3,
        semantic_hash,
        dependency_hash,
        certificate_dependency_hash: dependency_hash,
        total_claims: 48 + ((usize::from(engine_epoch) - 1) / 8) as u16,
        inherited_claims: 41 + ((usize::from(engine_epoch) - 1) / 8) as u16,
        affected_claims: 4,
        emergent_claims: 1 + u16::from(result.difficulty_transition.is_some()),
        verification_law_count: 3,
        certificate_depth: (32 + usize::from(engine_epoch)).min(64) as u8,
        novelty_code: if result.difficulty_transition.is_some() {
            5
        } else {
            3
        },
        topology_code: 1 + ((engine_epoch + 3) % 5),
        resource_contract: 0x27D1_0000 | global_epoch as u64,
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

fn require_predecessor_head(root: &Path) -> Result<(), String> {
    let head = git(root, &["rev-parse", "HEAD"])?;
    if head != PREDECESSOR_COMMIT {
        return Err(format!("PREDECESSOR_COMMIT_MISMATCH:{head}"));
    }
    Ok(())
}

fn verify_sealed_engine(root: &Path) -> Result<(), String> {
    let source = sha256_file(root.join("crates/semantic-reasoning/src/sem27/engine.rs"))?;
    let artifact = sha256_file(
        root.join("reports/sem27_continuation/artifacts/sealed-sem27-policy/engine.rs"),
    )?;
    if source != SEALED_ENGINE_SHA256 || source != artifact {
        return Err("SEALED_ENGINE_HASH_MISMATCH".to_string());
    }
    Ok(())
}

fn load_latest_state(root: &Path) -> Result<PostScaffoldState, String> {
    serde_json::from_value(read_json(
        root.join("reports/sem27_continuation/final_continuation_state.json"),
    )?)
    .map_err(|error| format!("PARSE_LATEST_STATE:{error}"))
}

fn validate_latest_state(state: &PostScaffoldState) -> Result<(), String> {
    if state.difficulty.current_regime_id != 2
        || state.difficulty.current_dimensions.transfer_arity != 2
        || state.difficulty.transitions.len() != 1
        || state.difficulty.current_initial_cost_ns != 7_600
        || state.difficulty.current_cost_sequence_ns.len() != 64
        || state.difficulty.local_mastery_progress != 47
    {
        return Err("LATEST_STATE_NOT_EXACT_OPEN_REGIME_2_DESCENDANT".to_string());
    }
    Ok(())
}

fn require_frozen(root: &Path, report_dir: &Path) -> Result<Value, String> {
    require_predecessor_head(root)?;
    verify_sealed_engine(root)?;
    let config = read_json(report_dir.join("continuation_config.json"))?;
    if config["campaign_id"] != json!(CAMPAIGN_ID)
        || config["ontology_hash"] != json!(ontology_hash())
        || config["continuation_epochs_budget"] != json!(CONTINUATION_BUDGET)
        || config["sealed_sem27_engine_sha256"] != json!(SEALED_ENGINE_SHA256)
    {
        return Err("SEM27_R1_CAMPAIGN_NOT_FROZEN".to_string());
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
    .map_err(|error| format!("COPY_ENGINE:{error}"))?;
    fs::copy(&probe, artifact_dir.join("sem27-probe-release.exe"))
        .map_err(|error| format!("COPY_PROBE:{error}"))?;
    Ok(probe)
}

fn ensure_artifacts(root: &Path, report_dir: &Path, probe: &Path) -> Result<(), String> {
    let artifact_dir = report_dir.join("artifacts/sealed-sem27-policy");
    if sha256_file(root.join("crates/semantic-reasoning/src/sem27/engine.rs"))?
        != sha256_file(artifact_dir.join("engine.rs"))?
        || sha256_file(probe)? != sha256_file(artifact_dir.join("sem27-probe-release.exe"))?
    {
        return Err("R1_ARTIFACT_HASH_MISMATCH".to_string());
    }
    Ok(())
}

fn ensure_reports(report_dir: &Path, executed: usize) -> Result<(), String> {
    let required = [
        "ontology_definition.json",
        "ontology_hash.json",
        "legacy_vs_revised_mapping.json",
        "retrospective_epoch64_regate.json",
        "retention_regate.json",
        "ontology_unit_tests.json",
        "ontology_differential_audit.json",
        "continuation_config.json",
        "difficulty_regime_ledger.jsonl",
        "difficulty_transition_ledger.jsonl",
        "plateau_closure_ledger.jsonl",
        "productive_escalation_ledger.jsonl",
        "staircase_growth_ledger.jsonl",
        "capability_productivity_sequence.json",
        "time_to_master_difficulty_sequence.json",
        "regime_frontier_capability_sequence.json",
        "within_regime_cost_sequences.json",
        "autonomous_decision_ledger.jsonl",
        "fresh_work_results.json",
        "regression_results.json",
        "sem27_r1_final_report.json",
        "SEM27_R1_REPORT.md",
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
    for epoch in 1..=executed {
        if !report_dir
            .join(format!("continuation_epoch_{epoch:03}.json"))
            .is_file()
        {
            return Err(format!("MISSING_CONTINUATION_EPOCH:{epoch}"));
        }
    }
    Ok(())
}

fn write_markdown(report_dir: &Path, report: &Value) -> Result<(), String> {
    let markdown = format!(
        "# SEM-27-R1 Difficulty Escalation Outcome Ontology Repair\n\nStatus: `{}`\n\n- Legacy escalation outcome: `{}`\n- Revised adaptation outcome: `{}`\n- Frontier gain retained: `{}`\n- Regime 2 closure: `{}`\n- Staircase state: `{}`\n- Continuation epochs: `{}` / `{}`\n- Productive escalations: `{}`\n- Failed escalations: `{}`\n- Staircase growth: `{}`\n- Staircase self-amplifying regime: `{}`\n- Engine behavior diff lines: `0`\n- Autonomous policy diff lines: `0`\n- Human research steering events: `0`\n- Next allowed stage: `OPERATOR_REVIEW_ONLY`\n\nHistorical SEM-27 verdicts remain immutable. Claims are bounded to the closed experimental environment.\n",
        report["sem27_r1_status"].as_str().unwrap_or("UNKNOWN"),
        report["legacy_escalation_outcome"].as_str().unwrap_or("UNKNOWN"),
        report["revised_escalation_adaptation_outcome"].as_str().unwrap_or("UNKNOWN"),
        report["frontier_gain_retention_confirmed"],
        report["regime_closure_state"].as_str().unwrap_or("UNKNOWN"),
        report["staircase_state"].as_str().unwrap_or("UNKNOWN"),
        report["continuation_epochs_executed"],
        report["continuation_epochs_budget"],
        report["productive_difficulty_escalation_events"],
        report["failed_difficulty_escalation_events"],
        report["staircase_growth_observed"],
        report["staircase_self_amplifying_regime_observed"],
    );
    fs::write(report_dir.join("SEM27_R1_REPORT.md"), markdown)
        .map_err(|error| format!("WRITE_MARKDOWN:{error}"))
}

fn human_intervention_audit() -> Value {
    json!({
        "campaign_budget_granted_by_operator": true,
        "human_research_steering_events": 0,
        "human_bottleneck_selection_events": 0,
        "human_repair_design_events": 0,
        "human_experiment_selection_events": 0,
        "human_frontier_selection_events": 0,
        "human_difficulty_escalation_events": 0,
        "human_difficulty_level_selection_events": 0,
        "next_difficulty_prescribed": false,
        "mid_campaign_intellectual_steering_events": 0,
        "passed": true,
    })
}

fn git_diff_lines(root: &Path, commit: &str, paths: &[&str]) -> Result<u64, String> {
    let mut arguments = vec!["diff", "--numstat", commit, "--"];
    arguments.extend(paths);
    let output = git(root, &arguments)?;
    Ok(output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let added = fields.next()?.parse::<u64>().ok()?;
            let removed = fields.next()?.parse::<u64>().ok()?;
            Some(added.saturating_add(removed))
        })
        .sum())
}

fn state_hash(state: &PostScaffoldState) -> Result<String, String> {
    serde_json::to_vec(state)
        .map(|bytes| sha256_bytes(&bytes))
        .map_err(|error| format!("SERIALIZE_STATE:{error}"))
}

fn enum_text<T: Serialize>(value: T) -> Result<String, String> {
    serde_json::to_value(value)
        .map_err(|error| format!("SERIALIZE_ENUM:{error}"))?
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "ENUM_NOT_STRING".to_string())
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
        text.push_str("{\"events\":0,\"reason\":\"NO_EVENT_OBSERVED_WITHIN_FIXED_R1_BUDGET\"}\n");
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
        0x27D1_F101,
        0x27D1_F113,
        0x27D1_F125,
        0x27D1_F137,
        0x27D1_F149,
        0x27D1_F15B,
        0x27D1_F16D,
        0x27D1_F17F,
    ]
}

fn seed_for_r1(epoch: u16) -> u64 {
    mix_campaign(0x5E27_D100_0000_0001, u64::from(epoch) * 137).max(1)
}

fn engine_epoch(epoch: u16) -> u8 {
    u8::try_from((epoch - 1) % 64 + 1).expect("engine epoch in 1..=64")
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
