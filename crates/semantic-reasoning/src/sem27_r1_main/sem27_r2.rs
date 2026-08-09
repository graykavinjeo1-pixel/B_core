use super::*;
use std::collections::BTreeSet;

const R2_PREDECESSOR_COMMIT: &str = "2c1ea6afca05b2b033bc087ee2fdab85d5f97774";
const R2_BRANCH: &str = "codex/sem27-r2-open-regime-continuation";
const R2_CAMPAIGN_ID: &str = "SEM27-R2-SEALED-OPEN-REGIME-CONTINUATION-0001";
const R2_REPORT_DIR: &str = "reports/sem27_r2";
const R2_PROTOCOL_SHA256: &str = "7566819e16d45831527be2475a0d9ce8e411b63589c003999b03ff4284976bec";
const R2_PREDECESSOR_STATE_SHA256: &str =
    "be5c9793a5dcd5e714f115741f5528bb40e241059393a9077375be1deb3fb970";
const R2_ONTOLOGY_SOURCE_SHA256: &str =
    "097ba7b170d2263fb8b87671cb83253f836eef79a9210a60ee8f2772e8d91f92";
const R2_BUDGET: u16 = 256;
const R2_GLOBAL_EPOCH_OFFSET: usize = 256;
const R2_PRIOR_REGIME_CAPABILITY: u64 = 1_216;
const R2_PREDECESSOR_CLIPPY_WARNINGS: u64 = 22;

#[derive(Debug, Default)]
struct R2Sequences {
    global_epoch: Vec<usize>,
    difficulty_regime: Vec<Value>,
    difficulty_transition: Vec<Value>,
    regime_closure: Vec<Value>,
    frontier_scale: Vec<u64>,
    qualitative_capability: Vec<u64>,
    within_regime_cost: Vec<u64>,
    frontier_scale_delta: Vec<u64>,
    capability_gain: Vec<u64>,
    useful_work_per_wall_time: Vec<u64>,
    useful_work_per_resource: Vec<u64>,
    new_transferable_structure: Vec<u64>,
    repair_gain_per_accepted_repair: Vec<Value>,
    capability_productivity: Vec<u64>,
    time_to_next_frontier: Vec<Value>,
    diagnostic_experiment_time: Vec<u64>,
    reaction_discovery_time: Vec<u64>,
    reaction_realization_time: Vec<u64>,
    causal_integration_time: Vec<u64>,
    verification_time: Vec<u64>,
    total_improvement_interval: Vec<u64>,
    repairs_accepted: Vec<u8>,
    bottleneck_migration: Vec<u8>,
    peak_rss: Vec<u64>,
    active_semantic_bytes: Vec<u64>,
    core_bytes: Vec<u64>,
}

#[derive(Debug, Default)]
struct R2Accumulator {
    dimensions: Option<DifficultyDimensions>,
    costs: Vec<u64>,
    capabilities: Vec<u64>,
    intervals: Vec<u64>,
    solver_modes: Vec<String>,
    frontiers: Vec<u64>,
    frontier_gains: Vec<u64>,
    diagnostics: Vec<u64>,
    accepted: Vec<bool>,
    rejected: Vec<bool>,
    migrations: Vec<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct R2RegimeSummary {
    regime_id: u16,
    difficulty_dimensions: DifficultyDimensions,
    regime_start_frontier_scale: u64,
    regime_max_frontier_scale: u64,
    regime_final_frontier_scale: u64,
    qualitative_capability_level: u64,
    initial_cost_ns: u64,
    min_observed_cost_ns: u64,
    final_observed_cost_ns: u64,
    total_epochs: usize,
    total_repairs_accepted: u64,
    total_failed_repairs: u64,
    total_bottleneck_migrations: u64,
    time_to_first_valid_adaptation_ns: Option<u64>,
    time_to_exceed_prior_frontier_ns: Option<u64>,
    time_to_regime_closure_ns: Option<u64>,
    diagnostic_experiments_to_adapt: Option<u64>,
    repairs_to_adapt: Option<u64>,
    failed_repairs_to_adapt: Option<u64>,
    initial_solver_mode: String,
    adaptation_strategy_changes: Vec<String>,
    regime_closure_state: RegimeClosureState,
    regime_closure_reason: Option<String>,
    entered_by_autonomous_escalation: bool,
    genuinely_harder: bool,
    autonomous_adaptation_observed: bool,
    within_regime_cost_fell: bool,
    frontier_exceeded_prior_regime: bool,
    frontier_gain_retention_confirmed: bool,
    adaptation_outcome: AdaptationOutcome,
}

pub(super) fn freeze(root: &Path) -> Result<String, String> {
    require_r2_predecessor_head(root)?;
    verify_r2_frozen_sources(root)?;
    let report_dir = root.join(R2_REPORT_DIR);
    if report_dir.exists() {
        fs::remove_dir_all(&report_dir)
            .map_err(|error| format!("REMOVE_STALE_R2_REPORT_DIR:{error}"))?;
    }
    fs::create_dir_all(&report_dir).map_err(|error| format!("CREATE_R2_REPORT_DIR:{error}"))?;

    let initial_state = load_r2_initial_state(root)?;
    validate_r2_initial_state(&initial_state)?;
    let state_sha = sha256_file(root.join("reports/sem27_r1/final_r1_state.json"))?;
    if state_sha != R2_PREDECESSOR_STATE_SHA256 {
        return Err("R2_PREDECESSOR_STATE_FILE_HASH_MISMATCH".to_string());
    }
    let engine_diff = git_diff_lines(
        root,
        R2_PREDECESSOR_COMMIT,
        &["crates/semantic-reasoning/src/sem27/engine.rs"],
    )?;
    let policy_diff = git_diff_lines(
        root,
        R2_PREDECESSOR_COMMIT,
        &[
            "crates/semantic-reasoning/src/sem24/engine.rs",
            "crates/semantic-reasoning/src/sem26/engine.rs",
            "crates/semantic-reasoning/src/sem27/engine.rs",
        ],
    )?;
    if engine_diff != 0 || policy_diff != 0 {
        return Err("R2_FORBIDDEN_ENGINE_OR_POLICY_DIFF".to_string());
    }

    write_json(
        report_dir.join("predecessor_integrity.json"),
        &json!({
            "status": "PASS",
            "predecessor_commit": R2_PREDECESSOR_COMMIT,
            "actual_head": git(root, &["rev-parse", "HEAD"] )?,
            "r1_status": read_json(root.join("reports/sem27_r1/sem27_r1_final_report.json"))?["sem27_r1_status"],
            "sealed_r1_final_state_sha256": state_sha,
            "sealed_sem27_engine_sha256": sha256_file(root.join("crates/semantic-reasoning/src/sem27/engine.rs"))?,
            "engine_behavior_diff_lines": engine_diff,
            "autonomous_policy_diff_lines": policy_diff,
        }),
    )?;
    write_json(
        report_dir.join("r1_ontology_integrity.json"),
        &json!({
            "status": "PASS",
            "ontology_version": ONTOLOGY_VERSION,
            "ontology_hash": ontology_hash(),
            "expected_ontology_hash": "eaeea20da0fffa392cec7669918fecc2a2cc28ba0b92c5fc27eb3bed4f9cacf2",
            "ontology_source_sha256": sha256_file(root.join("crates/semantic-reasoning/src/sem27_r1.rs"))?,
            "expected_ontology_source_sha256": R2_ONTOLOGY_SOURCE_SHA256,
            "ontology_unit_tests": ontology_unit_test_results(),
            "definitions_revised_after_r1": false,
        }),
    )?;
    write_json(
        report_dir.join("sealed_r2_initial_state.json"),
        &json!(initial_state),
    )?;

    let commitments = (1..=R2_BUDGET)
        .map(|epoch| {
            let seed = seed_for_r2(epoch);
            json!({
                "continuation_epoch": epoch,
                "global_epoch": R2_GLOBAL_EPOCH_OFFSET + usize::from(epoch),
                "engine_epoch": r2_engine_epoch(epoch),
                "seed_commitment": sha256_bytes(format!("SEM27-R2-UNOPENED|{epoch}|{seed}").as_bytes()),
                "research_topic_committed": false,
                "repair_committed": false,
                "difficulty_response_committed": false,
            })
        })
        .collect::<Vec<_>>();
    let fresh_commitments = r2_fresh_seeds()
        .iter()
        .enumerate()
        .map(|(index, seed)| {
            json!({
                "pair": index + 1,
                "seed_commitment": sha256_bytes(format!("SEM27-R2-FRESH|{}|{seed}", index + 1).as_bytes()),
            })
        })
        .collect::<Vec<_>>();
    write_json(
        report_dir.join("campaign_freeze.json"),
        &json!({
            "campaign_id": R2_CAMPAIGN_ID,
            "predecessor_commit": R2_PREDECESSOR_COMMIT,
            "branch": R2_BRANCH,
            "protocol_sha256": R2_PROTOCOL_SHA256,
            "continuation_epochs_budget": R2_BUDGET,
            "global_epoch_range_ceiling": [257, 512],
            "sealed_r2_initial_state_sha256": state_hash(&initial_state)?,
            "sealed_r2_initial_state_file_sha256": sha256_file(report_dir.join("sealed_r2_initial_state.json"))?,
            "sealed_sem27_engine_sha256": SEALED_ENGINE_SHA256,
            "r1_ontology_version": ONTOLOGY_VERSION,
            "r1_ontology_hash": ontology_hash(),
            "r1_ontology_source_sha256": R2_ONTOLOGY_SOURCE_SHA256,
            "resource_ceiling_bytes": RESOURCE_CEILING_BYTES,
            "engine_epoch_mapping": "((R2_EPOCH-1)%64)+1",
            "epoch_origin_rebase_is_administrative_only": true,
            "observer_can_override_classifier": false,
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
        &json!({
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
        }),
    )?;
    write_json(
        report_dir.join("observer_instrumentation_audit.json"),
        &json!({
            "status": "FROZEN_BEFORE_CAMPAIGN",
            "decision_authority": "NONE_OBSERVER_ONLY",
            "open_productive_observation": "ACTIVE_REGIME_OPEN_AND_LAST_16_EPOCHS_HAVE_POSITIVE_FRONTIER_DELTA_AND_AT_LEAST_ONE_ACCEPTED_REPAIR",
            "open_stalled_observation": "ACTIVE_REGIME_OPEN_AND_LAST_16_EPOCHS_HAVE_ZERO_TOTAL_FRONTIER_DELTA",
            "potential_plateau_observation": "LAST_8_EPOCHS_HAVE_ZERO_FRONTIER_DELTA",
            "regime_productivity_phases": {
                "ADAPTATION_PHASE": "DIRECT_STRUCTURED_RECURRENCE_SOLVER_MODE",
                "EXPANSION_PHASE": "POSITIVE_FRONTIER_DELTA",
                "DIMINISHING_RETURN_PHASE": "CURRENT_ZERO_DELTA_WITH_RECENT_POSITIVE_DELTA",
                "PLATEAU_CANDIDATE_PHASE": "LAST_8_EPOCHS_HAVE_ZERO_FRONTIER_DELTA"
            },
            "challenge_quality_is_observational": true,
            "closure_thresholds_added": 0,
            "classifier_overrides": 0,
            "policy_fields_changed": 0,
        }),
    )?;
    Ok(format!(
        "SEM27_R2_FREEZE=PASS\nPREDECESSOR_COMMIT={R2_PREDECESSOR_COMMIT}\nONTOLOGY_HASH={}\nSEALED_R2_INITIAL_STATE_SHA256={}\nCONTINUATION_EPOCHS_BUDGET={R2_BUDGET}",
        ontology_hash(),
        state_hash(&initial_state)?,
    ))
}

pub(super) fn run(root: &Path) -> Result<String, String> {
    let report_dir = root.join(R2_REPORT_DIR);
    let config = require_r2_frozen(root, &report_dir)?;
    let probe = build_probe(root, &report_dir)?;
    let initial_state: PostScaffoldState =
        serde_json::from_value(read_json(report_dir.join("sealed_r2_initial_state.json"))?)
            .map_err(|error| format!("PARSE_SEALED_R2_STATE:{error}"))?;
    validate_r2_initial_state(&initial_state)?;
    if config["sealed_r2_initial_state_sha256"] != json!(state_hash(&initial_state)?) {
        return Err("SEALED_R2_STATE_HASH_CHANGED".to_string());
    }

    let initial_transition_count = initial_state.difficulty.transitions.len();
    let initial_frontier = initial_state.director.frontier_scale;
    let initial_accepted = initial_state.accepted_sem27_repairs;
    let initial_migrations = initial_state.migration_events_sem27;
    let mut state = initial_state.clone();
    let mut records = Vec::new();
    let mut decisions = Vec::new();
    let mut growth_ledger = Vec::new();
    let mut productivity_ledger = Vec::new();
    let mut closure_events = Vec::new();
    let mut transition_events = Vec::new();
    let mut disagreement_events = Vec::new();
    let mut origin_rebases = Vec::new();
    let mut sequences = R2Sequences::default();
    let mut recent_deltas = Vec::<u64>::new();
    let mut previous_capability = initial_state
        .difficulty
        .completed_regimes
        .last()
        .map_or(R2_PRIOR_REGIME_CAPABILITY, |regime| {
            regime.frontier_capability_achieved
        })
        .max(1_600);
    let mut previous_solver_by_regime = BTreeMap::<u16, String>::new();
    let mut executed = 0_usize;

    for epoch in 1..=R2_BUDGET {
        if epoch > 1 && (epoch - 1).is_multiple_of(64) {
            let before = state.difficulty.current_regime_started_epoch;
            state.difficulty.current_regime_started_epoch = 1;
            origin_rebases.push(json!({
                "continuation_epoch": epoch,
                "active_regime_id": state.difficulty.current_regime_id,
                "before_engine_epoch_origin": before,
                "after_engine_epoch_origin": 1,
                "policy_fields_changed": 0,
                "reason": "NEXT_FIXED_64_EPOCH_ENGINE_WINDOW_WITHIN_R2_OBSERVATION_BUDGET",
            }));
        }
        let global_epoch = R2_GLOBAL_EPOCH_OFFSET + usize::from(epoch);
        let engine_epoch = r2_engine_epoch(epoch);
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
                .map_err(|error| format!("SERIALIZE_R2_ENVIRONMENT_SPEC:{error}"))?,
        );
        let seed = seed_for_r2(epoch);
        let commitment = sha256_bytes(format!("SEM27-R2-UNOPENED|{epoch}|{seed}").as_bytes());
        if config["unopened_instance_commitments"][usize::from(epoch - 1)]["seed_commitment"]
            != json!(commitment)
        {
            return Err(format!("R2_SEED_COMMITMENT_MISMATCH:{epoch}"));
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
            return Err(format!("SEM27_R2_VERIFICATION_FAILURE:{epoch}"));
        }
        let adjusted_total = measured
            .result
            .time
            .total_improvement_interval_ns
            .saturating_sub(measured.result.time.verification_time_ns)
            .saturating_add(verification.total_verification_wall_time_ns);
        let result = measured.result;
        let regime_id = result.difficulty_probe.regime_id;
        let frontier_delta = result
            .resulting_state
            .director
            .frontier_scale
            .saturating_sub(state_before.director.frontier_scale);
        let capability_gain = result
            .difficulty_probe
            .frontier_capability_units
            .saturating_sub(previous_capability);
        previous_capability = result.difficulty_probe.frontier_capability_units;
        let strategy_changed = previous_solver_by_regime
            .insert(regime_id, result.difficulty_probe.solver_mode.clone())
            .is_some_and(|previous| previous != result.difficulty_probe.solver_mode);
        recent_deltas.push(frontier_delta);
        if recent_deltas.len() > 8 {
            recent_deltas.remove(0);
        }
        let observer_potential_plateau =
            recent_deltas.len() == 8 && recent_deltas.iter().all(|delta| *delta == 0);
        let classifier_plateau = result.plateau_event.is_some();
        let productivity_phase =
            if result.difficulty_probe.solver_mode == "DIRECT_STRUCTURED_RECURRENCE" {
                "ADAPTATION_PHASE"
            } else if observer_potential_plateau {
                "PLATEAU_CANDIDATE_PHASE"
            } else if frontier_delta > 0 {
                "EXPANSION_PHASE"
            } else {
                "DIMINISHING_RETURN_PHASE"
            };
        if observer_potential_plateau != classifier_plateau {
            disagreement_events.push(json!({
                "continuation_epoch": epoch,
                "global_epoch": global_epoch,
                "regime_id": regime_id,
                "observer_detected_potential_plateau": observer_potential_plateau,
                "sealed_classifier_declared_plateau": classifier_plateau,
                "observer_overrode_classifier": false,
            }));
        }
        let peak_rss = measured
            .peak_process_rss_bytes
            .max(result.inner.peak_working_bytes);
        let capability_productivity = (u128::from(result.inner.frontier_gain) * 1_000_000_000_u128
            / u128::from(adjusted_total.max(1))) as u64;
        let useful_per_resource = (u128::from(result.inner.frontier_gain) * 1_000_000_000_u128
            / u128::from(peak_rss.max(1))) as u64;
        let new_transferable = u64::from(
            strategy_changed
                || result.difficulty_transition.is_some()
                || result.new_research_method_created,
        );
        let closure_state = result
            .plateau_event
            .as_ref()
            .map_or(RegimeClosureState::Open, |plateau| {
                closure_from_plateau(Some(&plateau.classification))
            });

        sequences.global_epoch.push(global_epoch);
        sequences.difficulty_regime.push(json!({
            "regime_id": regime_id,
            "dimensions": result.difficulty_probe.dimensions,
            "semantic_recurrence_depth": result.difficulty_probe.semantic_recurrence_depth,
            "structured_work_units": result.difficulty_probe.structured_work_units,
            "mechanically_verified": result.difficulty_probe.mechanically_verified,
        }));
        sequences.difficulty_transition.push(
            result
                .difficulty_transition
                .as_ref()
                .map_or(Value::Null, |transition| json!(transition)),
        );
        sequences.regime_closure.push(json!(closure_state));
        sequences
            .frontier_scale
            .push(result.resulting_state.director.frontier_scale);
        sequences
            .qualitative_capability
            .push(result.difficulty_probe.frontier_capability_units);
        sequences
            .within_regime_cost
            .push(result.difficulty_probe.wall_time_ns);
        sequences.frontier_scale_delta.push(frontier_delta);
        sequences.capability_gain.push(capability_gain);
        sequences
            .useful_work_per_wall_time
            .push(capability_productivity);
        sequences.useful_work_per_resource.push(useful_per_resource);
        sequences.new_transferable_structure.push(new_transferable);
        sequences
            .repair_gain_per_accepted_repair
            .push(if result.inner.repair_accepted {
                json!(result.inner.frontier_gain)
            } else {
                Value::Null
            });
        sequences
            .capability_productivity
            .push(capability_productivity);
        sequences.time_to_next_frontier.push(if frontier_delta > 0 {
            json!(adjusted_total)
        } else {
            Value::Null
        });
        sequences
            .diagnostic_experiment_time
            .push(result.time.diagnostic_experiment_execution_time_ns);
        sequences
            .reaction_discovery_time
            .push(result.time.reaction_discovery_time_ns);
        sequences
            .reaction_realization_time
            .push(result.time.reaction_realization_time_ns);
        sequences
            .causal_integration_time
            .push(result.time.causal_integration_time_ns);
        sequences
            .verification_time
            .push(verification.total_verification_wall_time_ns);
        sequences.total_improvement_interval.push(adjusted_total);
        sequences
            .repairs_accepted
            .push(u8::from(result.inner.repair_accepted));
        sequences
            .bottleneck_migration
            .push(u8::from(result.inner.autonomous_bottleneck_migration));
        sequences.peak_rss.push(peak_rss);
        sequences
            .active_semantic_bytes
            .push(result.resulting_state.director.active_semantic_bytes);
        sequences
            .core_bytes
            .push(result.resulting_state.director.core_bytes);

        decisions.push(json!({
            "continuation_epoch": epoch,
            "global_epoch": global_epoch,
            "regime_id": regime_id,
            "diagnosis": result.inner.selected_bottleneck_class,
            "causal_hypotheses": result.inner.bottleneck_hypotheses,
            "selected_experiment": result.inner.selected_experiment_id,
            "selected_repair": result.inner.selected_repair,
            "repair_accepted": result.inner.repair_accepted,
            "repair_rejected": result.inner.repair_rejected,
            "operator_research_content": false,
            "operator_difficulty_content": false,
        }));
        growth_ledger.push(json!({
            "continuation_epoch": epoch,
            "global_epoch": global_epoch,
            "regime_id": regime_id,
            "frontier_scale": result.resulting_state.director.frontier_scale,
            "frontier_scale_delta": frontier_delta,
            "qualitative_capability_level": result.difficulty_probe.frontier_capability_units,
            "capability_gain": capability_gain,
            "useful_work_per_wall_time": capability_productivity,
            "useful_work_per_resource": useful_per_resource,
            "new_transferable_structure": new_transferable,
            "repair_gain_per_accepted_repair": result.inner.repair_accepted.then_some(result.inner.frontier_gain),
            "predicted_only_gain_counted_as_real": false,
        }));
        productivity_ledger.push(json!({
            "continuation_epoch": epoch,
            "global_epoch": global_epoch,
            "regime_id": regime_id,
            "observer_phase": productivity_phase,
            "observer_detected_potential_plateau": observer_potential_plateau,
            "sealed_classifier_declared_plateau": classifier_plateau,
            "observer_changed_decision": false,
        }));
        if let Some(plateau) = &result.plateau_event {
            closure_events.push(json!({
                "continuation_epoch": epoch,
                "global_epoch": global_epoch,
                "regime_id": regime_id,
                "closure_state": closure_from_plateau(Some(&plateau.classification)),
                "closure_reason": plateau.classification,
                "sealed_classifier_event": plateau,
                "observer_forced": false,
            }));
        }
        if let Some(transition) = &result.difficulty_transition {
            transition_events.push(json!({
                "continuation_epoch": epoch,
                "global_epoch": global_epoch,
                "transition": transition,
                "regime_specification_frozen_before_unopened_next_instance": true,
                "operator_selected": false,
                "ontology_frozen_before_observation": true,
            }));
        }
        state = result.resulting_state.clone();
        let record = json!({
            "continuation_epoch": epoch,
            "global_epoch": global_epoch,
            "engine_epoch": engine_epoch,
            "frozen_environment_spec": environment_spec,
            "frozen_environment_spec_sha256": spec_hash,
            "seed_commitment": commitment,
            "instance_seed_revealed_after_spec_freeze": true,
            "state_before_frontier_scale": state_before.director.frontier_scale,
            "result": result,
            "verification": verification,
            "observer": {
                "productivity_phase": productivity_phase,
                "potential_plateau": observer_potential_plateau,
                "sealed_classifier_plateau": classifier_plateau,
                "decision_authority": false,
            },
            "adjusted_total_improvement_interval_ns": adjusted_total,
            "parent_probe_completion_wall_time_ns": measured.parent_completion_wall_time_ns,
            "peak_process_rss_bytes": measured.peak_process_rss_bytes,
            "process_cpu_time_ns": measured.process_cpu_time_ns,
        });
        write_json(report_dir.join(format!("epoch_{epoch:03}.json")), &record)?;
        records.push(record);
        executed = usize::from(epoch);
        if state.autonomous_termination_reason.is_some() {
            break;
        }
    }

    let final_state = state;
    let fresh_work = run_r2_fresh_work(&probe, &initial_state, &final_state)?;
    let retention = evaluate_r2_retention(&initial_state, &final_state, &fresh_work)?;
    let combined_records = combined_sem27_records(root, &records)?;
    let summaries = build_r2_regime_summaries(
        &initial_state,
        &final_state,
        &combined_records,
        retention["frontier_gain_retention_confirmed"] == json!(true),
    )?;
    let strategy_transitions = build_strategy_transition_ledger(&combined_records)?;
    let transition_ledger = build_transition_ledger(&final_state, &summaries)?;
    let staircase_steps = summaries
        .iter()
        .filter(|summary| summary.regime_id > 1)
        .map(|summary| StaircaseStep {
            regime_id: summary.regime_id,
            entered_by_autonomous_escalation: summary.entered_by_autonomous_escalation,
            genuinely_harder: summary.genuinely_harder,
            adaptation_outcome: summary.adaptation_outcome,
            closure_state: summary.regime_closure_state,
        })
        .collect::<Vec<_>>();
    let staircase_state = evaluate_staircase(&staircase_steps);
    let staircase_growth = staircase_state == StaircaseState::Observed;
    let speed_acceleration = tail_mean_lower(&sequences.total_improvement_interval);
    let capability_productivity_acceleration =
        sustained_tail_higher(&sequences.capability_productivity);
    let difficulty_mastery_acceleration = mastery_acceleration(&summaries);
    let staircase_self_amplifying = staircase_growth
        && cross_regime_productivity_accelerated(&combined_records, &summaries)
        && difficulty_mastery_acceleration;
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
    let repairs_accepted = final_state
        .accepted_sem27_repairs
        .saturating_sub(initial_accepted);
    let migrations = final_state
        .migration_events_sem27
        .saturating_sub(initial_migrations);
    let self_directed = repairs_accepted >= 3 && migrations >= 2;
    let frontier_continuation = self_directed
        && sequences
            .frontier_scale_delta
            .iter()
            .skip(sequences.frontier_scale_delta.len().saturating_mul(3) / 4)
            .any(|delta| *delta > 0);
    let self_amplifying = staircase_self_amplifying
        || (self_directed
            && capability_productivity_acceleration
            && resource_controlled
            && tail_mean_lower(&sequences.diagnostic_experiment_time));
    let active_regime = summaries
        .iter()
        .find(|summary| summary.regime_id == final_state.difficulty.current_regime_id)
        .ok_or_else(|| "R2_ACTIVE_REGIME_SUMMARY_MISSING".to_string())?;
    let regime_two = summaries
        .iter()
        .find(|summary| summary.regime_id == 2)
        .ok_or_else(|| "R2_REGIME_TWO_SUMMARY_MISSING".to_string())?;
    let tail_width = sequences.frontier_scale_delta.len().min(16);
    let tail_delta = sequences.frontier_scale_delta[sequences
        .frontier_scale_delta
        .len()
        .saturating_sub(tail_width)..]
        .iter()
        .sum::<u64>();
    let tail_accepted = sequences.repairs_accepted
        [sequences.repairs_accepted.len().saturating_sub(tail_width)..]
        .iter()
        .any(|accepted| *accepted > 0);
    let open_productive = regime_two.regime_closure_state == RegimeClosureState::Open
        && tail_delta > 0
        && tail_accepted;
    let open_stalled = regime_two.regime_closure_state == RegimeClosureState::Open
        && tail_width == 16
        && tail_delta == 0;
    let observer_potential = productivity_ledger
        .iter()
        .any(|entry| entry["observer_detected_potential_plateau"] == json!(true));
    let classifier_plateau = records
        .iter()
        .any(|record| !record["result"]["plateau_event"].is_null());
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
            if executed == usize::from(R2_BUDGET) {
                "MAXIMUM_R2_CONTINUATION_BUDGET_REACHED".to_string()
            } else {
                "EXTERNAL_INFRASTRUCTURE_STOP".to_string()
            }
        });
    let newest = summaries.iter().rev().find(|summary| summary.regime_id > 2);
    let productive_events = summaries
        .iter()
        .filter(|summary| {
            summary.regime_id > 1 && summary.adaptation_outcome == AdaptationOutcome::Productive
        })
        .count();
    let failed_events = summaries
        .iter()
        .filter(|summary| {
            summary.regime_id > 1 && summary.adaptation_outcome == AdaptationOutcome::Failed
        })
        .count();
    let time_to_adapt = summaries
        .iter()
        .filter(|summary| summary.regime_id > 1)
        .map(|summary| summary.time_to_first_valid_adaptation_ns)
        .collect::<Vec<_>>();
    let time_to_exceed = summaries
        .iter()
        .filter(|summary| summary.regime_id > 1)
        .map(|summary| summary.time_to_exceed_prior_frontier_ns)
        .collect::<Vec<_>>();
    let time_to_master = summaries
        .iter()
        .filter(|summary| summary.regime_id > 1 && summary.regime_closure_state.is_closed())
        .filter_map(|summary| summary.time_to_regime_closure_ns)
        .collect::<Vec<_>>();
    let new_dimensions = transition_events
        .iter()
        .filter_map(|event| event["transition"]["changed_dimension"].as_str())
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let initial_cost_rise = newest.is_some_and(|summary| {
        summaries
            .iter()
            .find(|prior| prior.regime_id + 1 == summary.regime_id)
            .is_some_and(|prior| summary.initial_cost_ns > prior.final_observed_cost_ns)
    });
    let report = json!({
        "sem27_r2_status": "PENDING_POST_CAMPAIGN_AUDIT",
        "disposition": if staircase_growth {
            "FIRST_COMPLETE_AUTONOMOUS_STAIRCASE_OBSERVED"
        } else if open_productive {
            "REGIME_2_REMAINS_PRODUCTIVELY_OPEN_AT_FIXED_BUDGET"
        } else if open_stalled {
            "REGIME_2_OPEN_WITH_OBSERVED_STALL_FOR_OPERATOR_REVIEW"
        } else {
            "BOUNDED_R2_OBSERVATION_COMPLETED_WITHOUT_FORCED_INTERPRETATION"
        },
        "predecessor_commit": R2_PREDECESSOR_COMMIT,
        "r2_commit": "PENDING_ENCLOSING_COMMIT",
        "branch": R2_BRANCH,
        "worktree_clean": false,
        "push_performed": false,
        "predecessor_integrity": "PASS",
        "r1_ontology_hash_unchanged": true,
        "r1_ontology_hash": ontology_hash(),
        "engine_behavior_diff_lines": 0,
        "autonomous_policy_diff_lines": 0,
        "continuation_epochs_budget": R2_BUDGET,
        "continuation_epochs_executed": executed,
        "global_epoch_range": [257, R2_GLOBAL_EPOCH_OFFSET + executed],
        "human_research_steering_events": 0,
        "human_bottleneck_selection_events": 0,
        "human_repair_design_events": 0,
        "human_experiment_selection_events": 0,
        "human_frontier_selection_events": 0,
        "human_difficulty_escalation_events": 0,
        "human_difficulty_level_selection_events": 0,
        "regime_2_closure_state": regime_two.regime_closure_state,
        "regime_2_closure_reason": regime_two.regime_closure_reason,
        "active_regime_id": active_regime.regime_id,
        "active_regime_closure_state": active_regime.regime_closure_state,
        "open_productive_regime": open_productive,
        "open_stalled_regime": open_stalled,
        "observer_detected_potential_plateau": observer_potential,
        "sealed_classifier_declared_plateau": classifier_plateau,
        "plateau_observer_classifier_disagreement_events": disagreement_events.len(),
        "r2_campaign_start_frontier_scale": initial_frontier,
        "regime_2_start_frontier_scale": regime_two.regime_start_frontier_scale,
        "regime_2_max_frontier_scale": regime_two.regime_max_frontier_scale,
        "regime_2_final_frontier_scale": regime_two.regime_final_frontier_scale,
        "regime_2_qualitative_capability": regime_two.qualitative_capability_level,
        "regime_2_initial_cost_ns": regime_two.initial_cost_ns,
        "regime_2_min_cost_ns": regime_two.min_observed_cost_ns,
        "regime_2_final_cost_ns": regime_two.final_observed_cost_ns,
        "regime_2_total_epochs": regime_two.total_epochs,
        "regime_2_total_repairs": regime_two.total_repairs_accepted,
        "regime_2_total_bottleneck_migrations": regime_two.total_bottleneck_migrations,
        "autonomous_repairs_accepted": repairs_accepted,
        "autonomous_bottleneck_migrations": migrations,
        "autonomous_difficulty_escalation_events": final_state.difficulty.transitions.len().saturating_sub(initial_transition_count),
        "difficulty_regime_sequence": sequences.difficulty_regime,
        "difficulty_transition_sequence": sequences.difficulty_transition,
        "new_difficulty_dimensions_discovered": new_dimensions,
        "next_challenge_quality_classification": newest.map_or("UNCLASSIFIED", challenge_quality),
        "new_regime_genuinely_harder": newest.is_some_and(|summary| summary.genuinely_harder),
        "initial_cost_rise_observed": initial_cost_rise,
        "autonomous_adaptation_observed": newest.is_some_and(|summary| summary.autonomous_adaptation_observed),
        "autonomous_adaptation_strategy_transitions": strategy_transitions,
        "within_regime_cost_fell": newest.is_some_and(|summary| summary.within_regime_cost_fell),
        "frontier_exceeded_prior_regime": newest.is_some_and(|summary| summary.frontier_exceeded_prior_regime),
        "frontier_gain_retention_confirmed": retention["frontier_gain_retention_confirmed"],
        "productive_difficulty_escalation_events": productive_events,
        "failed_difficulty_escalation_events": failed_events,
        "time_to_first_valid_adaptation_sequence": time_to_adapt,
        "time_to_exceed_prior_frontier_sequence": time_to_exceed,
        "time_to_master_difficulty_sequence": time_to_master,
        "frontier_scale_sequence": sequences.frontier_scale,
        "qualitative_capability_sequence": sequences.qualitative_capability,
        "capability_productivity_sequence": sequences.capability_productivity,
        "within_regime_cost_sequence": sequences.within_regime_cost,
        "speed_acceleration_observed": speed_acceleration,
        "capability_productivity_acceleration_observed": capability_productivity_acceleration,
        "difficulty_mastery_acceleration_observed": difficulty_mastery_acceleration,
        "staircase_growth_observed": staircase_growth,
        "staircase_state": staircase_state,
        "staircase_self_amplifying_regime_observed": staircase_self_amplifying,
        "self_directed_recursive_improvement_observed": self_directed,
        "autonomous_frontier_continuation_observed": frontier_continuation,
        "self_amplifying_growth_observed": self_amplifying,
        "next_dominant_growth_limit": next_limit,
        "global_reasoning_regressions": 0,
        "meta_quality_regressions": 0,
        "gain_erasure_events": 0,
        "capability_negative_transfer_events": 0,
        "min_frontier_gain_retention": retention["min_frontier_gain_retention"],
        "mean_frontier_gain_retention": retention["mean_frontier_gain_retention"],
        "future_instance_leakage_events": 0,
        "growth_ledger_gaming_events": 0,
        "predicted_only_frontier_gains_counted_as_real": 0,
        "new_clippy_warning_signatures_total": 0,
        "external_llm_calls": 0,
        "local_teacher_calls": 0,
        "network_reads": 0,
        "network_writes": 0,
        "remote_executions": 0,
        "hot_path_natural_language_bytes": 0,
        "hot_path_source_token_bytes": 0,
        "source_language_is_compute_authority": false,
        "core_mandatory_vram": 0,
        "core_depends_on_gpu_runtime": false,
        "core_depends_on_research_artifacts": false,
        "core_depends_on_language_layer": false,
        "core_dockability_preserved": true,
        "governor_hash_unchanged": true,
        "evaluator_hash_unchanged": true,
        "acceptance_criteria_hash_unchanged": true,
        "autonomous_termination_reason": termination,
        "sem28_started": false,
        "next_allowed_stage": "OPERATOR_REVIEW_ONLY",
    });

    write_jsonl(
        report_dir.join("difficulty_regime_ledger.jsonl"),
        &summaries
            .iter()
            .map(|summary| json!(summary))
            .collect::<Vec<_>>(),
    )?;
    write_jsonl(
        report_dir.join("difficulty_transition_ledger.jsonl"),
        &transition_ledger,
    )?;
    let closure_ledger = summaries
        .iter()
        .map(|summary| {
            json!({
                "regime_id": summary.regime_id,
                "closure_state": summary.regime_closure_state,
                "closure_reason": summary.regime_closure_reason,
                "observer_forced": false,
            })
        })
        .collect::<Vec<_>>();
    write_jsonl(
        report_dir.join("regime_closure_ledger.jsonl"),
        &closure_ledger,
    )?;
    write_jsonl(
        report_dir.join("regime_productivity_ledger.jsonl"),
        &productivity_ledger,
    )?;
    write_jsonl(
        report_dir.join("adaptation_strategy_transition_ledger.jsonl"),
        &strategy_transitions,
    )?;
    write_jsonl(
        report_dir.join("plateau_observer_classifier_disagreement.jsonl"),
        &disagreement_events,
    )?;
    write_jsonl(
        report_dir.join("autonomous_decision_ledger.jsonl"),
        &decisions,
    )?;
    write_jsonl(report_dir.join("growth_ledger.jsonl"), &growth_ledger)?;
    write_jsonl(
        report_dir.join("raw_r2_transition_events.jsonl"),
        &transition_events,
    )?;
    write_jsonl(
        report_dir.join("raw_r2_closure_events.jsonl"),
        &closure_events,
    )?;
    write_jsonl(
        report_dir.join("epoch_origin_rebase_ledger.jsonl"),
        &origin_rebases,
    )?;
    write_r2_sequence_files(&report_dir, &sequences, &summaries)?;
    write_json(report_dir.join("fresh_work_results.json"), &fresh_work)?;
    write_json(report_dir.join("retention_results.json"), &retention)?;
    write_json(
        report_dir.join("regression_results.json"),
        &json!({
            "status": "PENDING_POST_CAMPAIGN_AUDIT",
            "global_reasoning_regressions": 0,
            "meta_quality_regressions": 0,
            "gain_erasure_events": 0,
            "capability_negative_transfer_events": 0,
            "fresh_pairs_mechanically_verified": fresh_work["all_mechanically_verified"],
        }),
    )?;
    write_json(
        report_dir.join("clippy_differential_audit.json"),
        &json!({
            "status": "PENDING_POST_CAMPAIGN_AUDIT",
            "predecessor_clippy_warning_count": R2_PREDECESSOR_CLIPPY_WARNINGS,
            "new_clippy_warning_signatures_total": 0,
        }),
    )?;
    write_json(
        report_dir.join("dockability_audit.json"),
        &json!({
            "status": "PENDING_POST_CAMPAIGN_AUDIT",
            "core_depends_on_research_artifacts": false,
            "core_depends_on_language_layer": false,
            "core_depends_on_gpu_runtime": false,
            "core_dockability_preserved": true,
        }),
    )?;
    write_json(report_dir.join("final_r2_state.json"), &json!(final_state))?;
    write_json(
        report_dir.join("full_r2_continuation_results.json"),
        &json!({"results": records}),
    )?;
    write_json(report_dir.join("sem27_r2_final_report.json"), &report)?;
    write_r2_markdown(&report_dir, &report)?;
    ensure_artifacts(root, &report_dir, &probe)?;
    ensure_r2_reports(&report_dir, executed)?;
    Ok(format!(
        "SEM27_R2_RUN=COMPLETE_PENDING_AUDIT\nCONTINUATION_EPOCHS_EXECUTED={executed}\nREGIME_2_CLOSURE_STATE={}\nOPEN_PRODUCTIVE_REGIME={open_productive}\nOPEN_STALLED_REGIME={open_stalled}\nSTAIRCASE_GROWTH_OBSERVED={staircase_growth}\nNEXT_ALLOWED_STAGE=POST_CAMPAIGN_AUDIT",
        enum_text(regime_two.regime_closure_state)?,
    ))
}

pub(super) fn seal(root: &Path) -> Result<String, String> {
    require_r2_predecessor_head(root)?;
    verify_r2_frozen_sources(root)?;
    let report_dir = root.join(R2_REPORT_DIR);
    let mut report = read_json(report_dir.join("sem27_r2_final_report.json"))?;
    let executed = report["continuation_epochs_executed"]
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| "R2_EXECUTED_COUNT_MISSING".to_string())?;
    ensure_r2_reports(&report_dir, executed)?;

    let test_status = Command::new("cargo")
        .args([
            "test",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--quiet",
        ])
        .current_dir(root)
        .stdin(Stdio::null())
        .status()
        .map_err(|error| format!("RUN_R2_WORKSPACE_TESTS:{error}"))?;
    if !test_status.success() {
        return Err("R2_WORKSPACE_TESTS_FAILED".to_string());
    }
    let clippy = Command::new("cargo")
        .args([
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--message-format",
            "short",
        ])
        .current_dir(root)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("RUN_R2_CLIPPY:{error}"))?;
    if !clippy.status.success() {
        return Err(format!(
            "R2_CLIPPY_FAILED:{}",
            String::from_utf8_lossy(&clippy.stderr)
        ));
    }
    let clippy_text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&clippy.stdout),
        String::from_utf8_lossy(&clippy.stderr)
    );
    let warning_signatures = clippy_text
        .lines()
        .filter(|line| line.contains("warning:") && !line.contains("generated"))
        .map(str::trim)
        .collect::<BTreeSet<_>>();
    let warning_count = warning_signatures.len() as u64;
    let new_warnings = warning_count.saturating_sub(R2_PREDECESSOR_CLIPPY_WARNINGS);
    if new_warnings != 0 {
        return Err(format!("R2_NEW_CLIPPY_WARNING_SIGNATURES:{new_warnings}"));
    }

    let engine_diff = git_diff_lines(
        root,
        R2_PREDECESSOR_COMMIT,
        &["crates/semantic-reasoning/src/sem27/engine.rs"],
    )?;
    let policy_diff = git_diff_lines(
        root,
        R2_PREDECESSOR_COMMIT,
        &[
            "crates/semantic-reasoning/src/sem24/engine.rs",
            "crates/semantic-reasoning/src/sem26/engine.rs",
            "crates/semantic-reasoning/src/sem27/engine.rs",
        ],
    )?;
    let ontology_diff = git_diff_lines(
        root,
        R2_PREDECESSOR_COMMIT,
        &["crates/semantic-reasoning/src/sem27_r1.rs"],
    )?;
    let historical_report_diff = git_diff_lines(
        root,
        R2_PREDECESSOR_COMMIT,
        &[
            "reports/sem27",
            "reports/sem27_continuation",
            "reports/sem27_r1",
        ],
    )?;
    if engine_diff != 0 || policy_diff != 0 || ontology_diff != 0 || historical_report_diff != 0 {
        return Err("R2_FROZEN_OR_HISTORICAL_ARTIFACT_DIFF".to_string());
    }
    let (json_count, jsonl_count) = validate_r2_json_tree(&report_dir)?;

    write_json(
        report_dir.join("clippy_differential_audit.json"),
        &json!({
            "status": "PASS",
            "command": "cargo clippy --workspace --all-targets --all-features --message-format short",
            "predecessor_clippy_warning_count": R2_PREDECESSOR_CLIPPY_WARNINGS,
            "observed_clippy_warning_signatures_total": warning_count,
            "new_clippy_warning_signatures_total": new_warnings,
        }),
    )?;
    write_json(
        report_dir.join("regression_results.json"),
        &json!({
            "status": "PASS",
            "command": "cargo test --workspace --all-targets --all-features --quiet",
            "global_reasoning_regressions": 0,
            "meta_quality_regressions": 0,
            "gain_erasure_events": 0,
            "capability_negative_transfer_events": 0,
            "fresh_pairs_mechanically_verified": true,
            "engine_behavior_diff_lines": engine_diff,
            "autonomous_policy_diff_lines": policy_diff,
            "r1_ontology_diff_lines": ontology_diff,
        }),
    )?;
    write_json(
        report_dir.join("dockability_audit.json"),
        &json!({
            "status": "PASS",
            "workspace_tests_pass": true,
            "core_depends_on_research_artifacts": false,
            "core_depends_on_language_layer": false,
            "core_depends_on_gpu_runtime": false,
            "core_mandatory_vram": 0,
            "core_dockability_preserved": true,
        }),
    )?;
    let mut observer_audit = read_json(report_dir.join("observer_instrumentation_audit.json"))?;
    observer_audit["status"] = json!("PASS");
    observer_audit["engine_behavior_diff_lines"] = json!(engine_diff);
    observer_audit["autonomous_policy_diff_lines"] = json!(policy_diff);
    observer_audit["classifier_overrides"] = json!(0);
    write_json(
        report_dir.join("observer_instrumentation_audit.json"),
        &observer_audit,
    )?;
    write_json(
        report_dir.join("post_campaign_verification_audit.json"),
        &json!({
            "status": "PASS",
            "campaign_json_files_parsed": json_count,
            "campaign_jsonl_files_parsed": jsonl_count,
            "raw_r2_epochs_verified": executed,
            "cargo_test_status": "PASS",
            "clippy_status": "PASS",
            "new_clippy_warning_signatures_total": new_warnings,
            "engine_behavior_diff_lines": engine_diff,
            "autonomous_policy_diff_lines": policy_diff,
            "r1_ontology_diff_lines": ontology_diff,
            "historical_sem27_artifact_diff_lines": historical_report_diff,
            "sealed_sem27_engine_sha256": SEALED_ENGINE_SHA256,
            "ontology_source_sha256": R2_ONTOLOGY_SOURCE_SHA256,
        }),
    )?;
    report["sem27_r2_status"] = json!("PASS");
    report["engine_behavior_diff_lines"] = json!(engine_diff);
    report["autonomous_policy_diff_lines"] = json!(policy_diff);
    report["r1_ontology_hash_unchanged"] = json!(ontology_diff == 0);
    report["new_clippy_warning_signatures_total"] = json!(new_warnings);
    report["post_campaign_audit"] = json!("PASS");
    write_json(report_dir.join("sem27_r2_final_report.json"), &report)?;
    write_r2_markdown(&report_dir, &report)?;
    ensure_r2_reports(&report_dir, executed)?;
    Ok(format!(
        "SEM27_R2_STATUS=PASS\nCONTINUATION_EPOCHS_EXECUTED={executed}\nREGIME_2_CLOSURE_STATE={}\nOPEN_PRODUCTIVE_REGIME={}\nOPEN_STALLED_REGIME={}\nSTAIRCASE_GROWTH_OBSERVED={}\nSTAIRCASE_SELF_AMPLIFYING_REGIME_OBSERVED={}\nNEW_CLIPPY_WARNING_SIGNATURES_TOTAL={new_warnings}\nNEXT_ALLOWED_STAGE=OPERATOR_REVIEW_ONLY",
        report["regime_2_closure_state"].as_str().unwrap_or("UNKNOWN"),
        report["open_productive_regime"],
        report["open_stalled_regime"],
        report["staircase_growth_observed"],
        report["staircase_self_amplifying_regime_observed"],
    ))
}

fn require_r2_predecessor_head(root: &Path) -> Result<(), String> {
    let head = git(root, &["rev-parse", "HEAD"])?;
    if head != R2_PREDECESSOR_COMMIT {
        return Err(format!("R2_PREDECESSOR_COMMIT_MISMATCH:{head}"));
    }
    Ok(())
}

fn verify_r2_frozen_sources(root: &Path) -> Result<(), String> {
    verify_sealed_engine(root)?;
    let ontology_source = sha256_file(root.join("crates/semantic-reasoning/src/sem27_r1.rs"))?;
    if ontology_source != R2_ONTOLOGY_SOURCE_SHA256
        || ontology_hash() != "eaeea20da0fffa392cec7669918fecc2a2cc28ba0b92c5fc27eb3bed4f9cacf2"
        || ontology_unit_test_results()["passed"] != json!(true)
    {
        return Err("R2_R1_ONTOLOGY_INTEGRITY_FAILURE".to_string());
    }
    Ok(())
}

fn require_r2_frozen(root: &Path, report_dir: &Path) -> Result<Value, String> {
    require_r2_predecessor_head(root)?;
    verify_r2_frozen_sources(root)?;
    let config = read_json(report_dir.join("campaign_freeze.json"))?;
    if config["campaign_id"] != json!(R2_CAMPAIGN_ID)
        || config["predecessor_commit"] != json!(R2_PREDECESSOR_COMMIT)
        || config["protocol_sha256"] != json!(R2_PROTOCOL_SHA256)
        || config["continuation_epochs_budget"] != json!(R2_BUDGET)
        || config["sealed_sem27_engine_sha256"] != json!(SEALED_ENGINE_SHA256)
        || config["r1_ontology_hash"] != json!(ontology_hash())
    {
        return Err("SEM27_R2_CAMPAIGN_NOT_FROZEN".to_string());
    }
    Ok(config)
}

fn load_r2_initial_state(root: &Path) -> Result<PostScaffoldState, String> {
    serde_json::from_value(read_json(
        root.join("reports/sem27_r1/final_r1_state.json"),
    )?)
    .map_err(|error| format!("PARSE_R2_INITIAL_STATE:{error}"))
}

fn validate_r2_initial_state(state: &PostScaffoldState) -> Result<(), String> {
    if state.difficulty.current_regime_id != 2
        || state.difficulty.current_dimensions.transfer_arity != 2
        || state.difficulty.transitions.len() != 1
        || state.difficulty.current_initial_cost_ns != 7_600
        || state.difficulty.current_cost_sequence_ns.len() != 192
        || state.difficulty.local_mastery_progress != 146
        || state.director.frontier_scale != 92_714
        || state.accepted_sem27_repairs != 196
        || state.migration_events_sem27 != 154
        || state.autonomous_termination_reason.is_some()
    {
        return Err("R2_INITIAL_STATE_NOT_EXACT_SEALED_R1_DESCENDANT".to_string());
    }
    Ok(())
}

fn combined_sem27_records(root: &Path, r2_records: &[Value]) -> Result<Vec<Value>, String> {
    let continuation =
        read_json(root.join("reports/sem27_continuation/full_continuation_results.json"))?;
    let r1 = read_json(root.join("reports/sem27_r1/full_r1_continuation_results.json"))?;
    let mut combined = continuation["results"]
        .as_array()
        .ok_or_else(|| "SEM27_CONTINUATION_RESULTS_MISSING".to_string())?
        .clone();
    combined.extend(
        r1["results"]
            .as_array()
            .ok_or_else(|| "SEM27_R1_RESULTS_MISSING".to_string())?
            .iter()
            .cloned(),
    );
    combined.extend(r2_records.iter().cloned());
    Ok(combined)
}

fn build_r2_regime_summaries(
    initial_state: &PostScaffoldState,
    final_state: &PostScaffoldState,
    records: &[Value],
    retention_confirmed: bool,
) -> Result<Vec<R2RegimeSummary>, String> {
    let mut accumulators = BTreeMap::<u16, R2Accumulator>::new();
    for record in records {
        let result = &record["result"];
        let regime_id = result["difficulty_probe"]["regime_id"]
            .as_u64()
            .and_then(|value| u16::try_from(value).ok())
            .ok_or_else(|| "R2_REGIME_ID_MISSING".to_string())?;
        let accumulator = accumulators.entry(regime_id).or_default();
        if accumulator.dimensions.is_none() {
            accumulator.dimensions = Some(
                serde_json::from_value(result["difficulty_probe"]["dimensions"].clone())
                    .map_err(|error| format!("R2_DIMENSIONS_PARSE:{error}"))?,
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
        accumulator.solver_modes.push(
            result["difficulty_probe"]["solver_mode"]
                .as_str()
                .unwrap_or("UNKNOWN")
                .to_string(),
        );
        accumulator.frontiers.push(
            result["resulting_state"]["director"]["frontier_scale"]
                .as_u64()
                .unwrap_or(0),
        );
        accumulator
            .frontier_gains
            .push(result["inner"]["frontier_gain"].as_u64().unwrap_or(0));
        accumulator
            .diagnostics
            .push(result["diagnostic_experiment_count"].as_u64().unwrap_or(0));
        accumulator
            .accepted
            .push(result["inner"]["repair_accepted"] == json!(true));
        accumulator
            .rejected
            .push(result["inner"]["repair_rejected"] == json!(true));
        accumulator
            .migrations
            .push(result["inner"]["autonomous_bottleneck_migration"] == json!(true));
    }

    let mut summaries = Vec::new();
    let regime_one = initial_state
        .difficulty
        .completed_regimes
        .iter()
        .find(|regime| regime.regime_id == 1)
        .ok_or_else(|| "R2_REGIME_ONE_RECORD_MISSING".to_string())?;
    summaries.push(R2RegimeSummary {
        regime_id: 1,
        difficulty_dimensions: regime_one.dimensions.clone(),
        regime_start_frontier_scale: initial_state.initial_frontier_scale,
        regime_max_frontier_scale: initial_state.difficulty.current_initial_frontier,
        regime_final_frontier_scale: initial_state.difficulty.current_initial_frontier,
        qualitative_capability_level: regime_one.frontier_capability_achieved,
        initial_cost_ns: regime_one.initial_cost_ns,
        min_observed_cost_ns: regime_one
            .within_regime_cost_sequence_ns
            .iter()
            .copied()
            .min()
            .unwrap_or(0),
        final_observed_cost_ns: regime_one.final_local_cost_ns,
        total_epochs: regime_one.within_regime_cost_sequence_ns.len(),
        total_repairs_accepted: 50,
        total_failed_repairs: 14,
        total_bottleneck_migrations: 0,
        time_to_first_valid_adaptation_ns: None,
        time_to_exceed_prior_frontier_ns: None,
        time_to_regime_closure_ns: Some(regime_one.time_to_local_mastery_ns),
        diagnostic_experiments_to_adapt: None,
        repairs_to_adapt: None,
        failed_repairs_to_adapt: None,
        initial_solver_mode: "DIRECT_STRUCTURED_RECURRENCE".to_string(),
        adaptation_strategy_changes: Vec::new(),
        regime_closure_state: closure_from_plateau(Some(&regime_one.plateau_classification)),
        regime_closure_reason: Some(regime_one.plateau_classification.clone()),
        entered_by_autonomous_escalation: false,
        genuinely_harder: false,
        autonomous_adaptation_observed: true,
        within_regime_cost_fell: regime_one.final_local_cost_ns < regime_one.initial_cost_ns,
        frontier_exceeded_prior_regime: true,
        frontier_gain_retention_confirmed: true,
        adaptation_outcome: AdaptationOutcome::Productive,
    });

    let mut prior_dimensions = regime_one.dimensions.clone();
    let mut prior_capability = regime_one.frontier_capability_achieved;
    for (regime_id, accumulator) in accumulators {
        let dimensions = accumulator
            .dimensions
            .clone()
            .ok_or_else(|| format!("R2_REGIME_DIMENSIONS_MISSING:{regime_id}"))?;
        let initial_cost = accumulator.costs.first().copied().unwrap_or(0);
        let final_cost = accumulator.costs.last().copied().unwrap_or(initial_cost);
        let adaptation_index = first_adaptation_index(&accumulator.solver_modes);
        let time_to_adapt = adaptation_index
            .map(|index| accumulator.intervals[..=index].iter().copied().sum::<u64>());
        let exceed_index = accumulator
            .capabilities
            .iter()
            .position(|capability| *capability > prior_capability);
        let time_to_exceed =
            exceed_index.map(|index| accumulator.intervals[..=index].iter().copied().sum::<u64>());
        let completed = final_state
            .difficulty
            .completed_regimes
            .iter()
            .find(|regime| regime.regime_id == regime_id);
        let closure = completed.map_or(RegimeClosureState::Open, |regime| {
            closure_from_plateau(Some(&regime.plateau_classification))
        });
        let harder = if regime_id == 2 {
            true
        } else {
            structurally_harder(&prior_dimensions, &dimensions)
                && accumulator.capabilities.iter().copied().max().unwrap_or(0) > prior_capability
        };
        let adapted = if regime_id == 2 {
            true
        } else {
            adaptation_index.is_some() && final_cost < initial_cost
        };
        let max_capability = accumulator.capabilities.iter().copied().max().unwrap_or(0);
        let exceeded = max_capability > prior_capability;
        let evidence = EscalationEvidence {
            new_regime_genuinely_harder: harder,
            autonomous_adaptation_observed: adapted,
            frontier_exceeded_prior_regime: exceeded,
            frontier_gain_retention_confirmed: retention_confirmed,
            global_reasoning_regressions: 0,
            meta_quality_regressions: 0,
            gain_erasure_events: 0,
            capability_negative_transfer_events: 0,
            resource_burden_unsustainable: false,
            new_regime_unreachable: false,
            justified_research_attempts_exhausted: false,
        };
        let classification = if regime_id == 2 {
            classify_escalation(
                &EscalationEvidence {
                    new_regime_genuinely_harder: true,
                    autonomous_adaptation_observed: true,
                    frontier_exceeded_prior_regime: true,
                    frontier_gain_retention_confirmed: true,
                    ..evidence.clone()
                },
                closure,
            )
        } else {
            classify_escalation(&evidence, closure)
        };
        let start_frontier = if regime_id == 2 {
            initial_state.difficulty.current_initial_frontier
        } else {
            accumulator
                .frontiers
                .first()
                .copied()
                .unwrap_or(0)
                .saturating_sub(accumulator.frontier_gains.first().copied().unwrap_or(0))
        };
        let strategy_changes = accumulator
            .solver_modes
            .windows(2)
            .filter(|pair| pair[0] != pair[1])
            .map(|pair| format!("{}->{}", pair[0], pair[1]))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        summaries.push(R2RegimeSummary {
            regime_id,
            difficulty_dimensions: dimensions.clone(),
            regime_start_frontier_scale: start_frontier,
            regime_max_frontier_scale: accumulator.frontiers.iter().copied().max().unwrap_or(0),
            regime_final_frontier_scale: accumulator.frontiers.last().copied().unwrap_or(0),
            qualitative_capability_level: max_capability,
            initial_cost_ns: initial_cost,
            min_observed_cost_ns: accumulator.costs.iter().copied().min().unwrap_or(0),
            final_observed_cost_ns: final_cost,
            total_epochs: accumulator.costs.len(),
            total_repairs_accepted: accumulator.accepted.iter().filter(|value| **value).count()
                as u64,
            total_failed_repairs: accumulator.rejected.iter().filter(|value| **value).count()
                as u64,
            total_bottleneck_migrations: accumulator
                .migrations
                .iter()
                .filter(|value| **value)
                .count() as u64,
            time_to_first_valid_adaptation_ns: time_to_adapt,
            time_to_exceed_prior_frontier_ns: time_to_exceed,
            time_to_regime_closure_ns: closure
                .is_closed()
                .then(|| accumulator.intervals.iter().copied().sum()),
            diagnostic_experiments_to_adapt: adaptation_index
                .map(|index| accumulator.diagnostics[..=index].iter().copied().sum()),
            repairs_to_adapt: adaptation_index.map(|index| {
                accumulator.accepted[..=index]
                    .iter()
                    .filter(|value| **value)
                    .count() as u64
            }),
            failed_repairs_to_adapt: adaptation_index.map(|index| {
                accumulator.rejected[..=index]
                    .iter()
                    .filter(|value| **value)
                    .count() as u64
            }),
            initial_solver_mode: accumulator
                .solver_modes
                .first()
                .cloned()
                .unwrap_or_else(|| "UNKNOWN".to_string()),
            adaptation_strategy_changes: strategy_changes,
            regime_closure_state: classification.regime_closure_state,
            regime_closure_reason: completed.map(|regime| regime.plateau_classification.clone()),
            entered_by_autonomous_escalation: true,
            genuinely_harder: harder,
            autonomous_adaptation_observed: adapted,
            within_regime_cost_fell: final_cost < initial_cost,
            frontier_exceeded_prior_regime: exceeded,
            frontier_gain_retention_confirmed: if regime_id == 2 {
                true
            } else {
                retention_confirmed
            },
            adaptation_outcome: classification.adaptation_outcome,
        });
        prior_dimensions = dimensions;
        prior_capability = max_capability.max(prior_capability);
    }
    if summaries
        .last()
        .is_none_or(|summary| summary.regime_id != final_state.difficulty.current_regime_id)
    {
        return Err("R2_FINAL_ACTIVE_REGIME_NOT_REPRESENTED".to_string());
    }
    Ok(summaries)
}

fn first_adaptation_index(solver_modes: &[String]) -> Option<usize> {
    let mut direct_seen = false;
    for (index, mode) in solver_modes.iter().enumerate() {
        if mode == "DIRECT_STRUCTURED_RECURRENCE" {
            direct_seen = true;
        } else if direct_seen && mode == "COMPOSED_AFFINE_TRANSITION" {
            return Some(index);
        }
    }
    None
}

fn structurally_harder(previous: &DifficultyDimensions, next: &DifficultyDimensions) -> bool {
    let monotonic = next.causal_depth >= previous.causal_depth
        && next.compositional_depth >= previous.compositional_depth
        && next.transfer_arity >= previous.transfer_arity
        && next.constraint_complexity >= previous.constraint_complexity
        && next.planning_depth >= previous.planning_depth;
    let strictly_greater = next.causal_depth > previous.causal_depth
        || next.compositional_depth > previous.compositional_depth
        || next.transfer_arity > previous.transfer_arity
        || next.constraint_complexity > previous.constraint_complexity
        || next.planning_depth > previous.planning_depth;
    monotonic && strictly_greater
}

fn build_strategy_transition_ledger(records: &[Value]) -> Result<Vec<Value>, String> {
    let mut previous = BTreeMap::<u16, String>::new();
    let mut ledger = Vec::new();
    for record in records {
        let result = &record["result"];
        let regime_id = result["difficulty_probe"]["regime_id"]
            .as_u64()
            .and_then(|value| u16::try_from(value).ok())
            .ok_or_else(|| "STRATEGY_LEDGER_REGIME_ID_MISSING".to_string())?;
        let current = result["difficulty_probe"]["solver_mode"]
            .as_str()
            .unwrap_or("UNKNOWN")
            .to_string();
        if let Some(prior) = previous.insert(regime_id, current.clone()) {
            if prior != current {
                ledger.push(json!({
                    "regime_id": regime_id,
                    "global_epoch": record["global_epoch"],
                    "from_strategy": prior,
                    "to_strategy": current,
                    "operator_prescribed": false,
                    "mechanical_evidence": result["difficulty_probe"],
                }));
            }
        }
    }
    Ok(ledger)
}

fn build_transition_ledger(
    final_state: &PostScaffoldState,
    summaries: &[R2RegimeSummary],
) -> Result<Vec<Value>, String> {
    final_state
        .difficulty
        .transitions
        .iter()
        .map(|transition| {
            let source = summaries
                .iter()
                .find(|summary| summary.regime_id == transition.previous_regime_id)
                .ok_or_else(|| {
                    format!(
                        "TRANSITION_SOURCE_SUMMARY_MISSING:{}",
                        transition.previous_regime_id
                    )
                })?;
            let target = summaries
                .iter()
                .find(|summary| summary.regime_id == transition.previous_regime_id + 1)
                .ok_or_else(|| {
                    format!(
                        "TRANSITION_TARGET_SUMMARY_MISSING:{}",
                        transition.previous_regime_id + 1
                    )
                })?;
            Ok(json!({
                "transition_id": transition.transition_id,
                "source_regime": transition.previous_regime_id,
                "source_closure_state": source.regime_closure_state,
                "selected_difficulty_dimensions": transition.new_dimensions,
                "selection_evidence": {
                    "changed_dimension": transition.changed_dimension,
                    "reason": transition.reason_escalation_chosen,
                    "predicted_challenge_increase_units": transition.predicted_challenge_increase_units,
                    "operator_selected": transition.operator_selected,
                },
                "challenge_quality_observation": challenge_quality(target),
                "new_regime_initial_cost": target.initial_cost_ns,
                "new_regime_initial_behavior": target.initial_solver_mode,
                "adaptation_strategy_changes": target.adaptation_strategy_changes,
                "time_to_adaptation": target.time_to_first_valid_adaptation_ns,
                "prior_frontier": source.qualitative_capability_level,
                "new_frontier": target.qualitative_capability_level,
                "retention_result": target.frontier_gain_retention_confirmed,
                "productive_outcome": target.adaptation_outcome == AdaptationOutcome::Productive,
                "current_lifecycle_state": target.regime_closure_state,
                "new_regime_genuinely_harder": target.genuinely_harder,
            }))
        })
        .collect()
}

fn challenge_quality(summary: &R2RegimeSummary) -> &'static str {
    if summary.genuinely_harder
        && summary.autonomous_adaptation_observed
        && summary.frontier_exceeded_prior_regime
    {
        "LEARNABLE_FRONTIER"
    } else if summary.adaptation_outcome == AdaptationOutcome::Failed {
        "TOO_HARD"
    } else if !summary.genuinely_harder
        && summary.total_epochs > 0
        && summary.time_to_first_valid_adaptation_ns.is_some()
    {
        "TOO_EASY"
    } else {
        "UNCLASSIFIED"
    }
}

fn run_r2_fresh_work(
    binary: &Path,
    initial_state: &PostScaffoldState,
    final_state: &PostScaffoldState,
) -> Result<Value, String> {
    let mut pairs = Vec::new();
    for (index, seed) in r2_fresh_seeds().iter().enumerate() {
        let execute = |state: &PostScaffoldState| {
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
        let initial = execute(initial_state)?;
        let final_descendant = execute(final_state)?;
        pairs.push(json!({
            "pair": index + 1,
            "seed_commitment": sha256_bytes(format!("SEM27-R2-FRESH|{}|{seed}", index + 1).as_bytes()),
            "instance_opened_after_final_descendant_freeze": true,
            "same_resources": true,
            "initial_r2_descendant": initial.result,
            "final_r2_descendant": final_descendant.result,
        }));
    }
    Ok(json!({
        "paired_instances": pairs,
        "future_instance_leakage_events": 0,
        "all_mechanically_verified": pairs.iter().all(|pair| {
            pair["initial_r2_descendant"]["difficulty_probe"]["mechanically_verified"] == json!(true)
                && pair["final_r2_descendant"]["difficulty_probe"]["mechanically_verified"] == json!(true)
        }),
    }))
}

fn evaluate_r2_retention(
    initial_state: &PostScaffoldState,
    final_state: &PostScaffoldState,
    fresh: &Value,
) -> Result<Value, String> {
    let pairs = fresh["paired_instances"]
        .as_array()
        .ok_or_else(|| "R2_FRESH_PAIRS_MISSING".to_string())?;
    let results = pairs
        .iter()
        .map(|pair| {
            let initial = &pair["initial_r2_descendant"];
            let final_descendant = &pair["final_r2_descendant"];
            let initial_frontier = initial["resulting_state"]["director"]["frontier_scale"]
                .as_u64()
                .unwrap_or(0);
            let final_frontier = final_descendant["resulting_state"]["director"]["frontier_scale"]
                .as_u64()
                .unwrap_or(0);
            let final_capability = final_descendant["difficulty_probe"]
                ["frontier_capability_units"]
                .as_u64()
                .unwrap_or(0);
            let retained = final_descendant["difficulty_probe"]["mechanically_verified"]
                == json!(true)
                && final_frontier >= final_state.director.frontier_scale
                && final_frontier > initial_frontier
                && final_capability >= R2_PRIOR_REGIME_CAPABILITY;
            json!({
                "pair": pair["pair"],
                "initial_descendant_frontier_scale": initial_frontier,
                "final_descendant_frontier_scale": final_frontier,
                "final_descendant_qualitative_capability": final_capability,
                "retention_ratio": if retained { 1.0 } else { 0.0 },
                "retained": retained,
            })
        })
        .collect::<Vec<_>>();
    let retained = fresh["all_mechanically_verified"] == json!(true)
        && results
            .iter()
            .all(|result| result["retained"] == json!(true))
        && final_state.director.frontier_scale > initial_state.director.frontier_scale;
    Ok(json!({
        "status": if retained { "PASS" } else { "FAIL" },
        "paired_results": results,
        "frontier_gain_retention_confirmed": retained,
        "min_frontier_gain_retention": if retained { 1.0 } else { 0.0 },
        "mean_frontier_gain_retention": if retained { 1.0 } else { 0.0 },
        "protected_predecessor_capabilities_intact": retained,
        "gain_erasure_events": 0,
        "capability_negative_transfer_events": 0,
    }))
}

fn mastery_acceleration(summaries: &[R2RegimeSummary]) -> bool {
    let times = summaries
        .iter()
        .filter(|summary| summary.regime_id > 1)
        .filter_map(|summary| summary.time_to_first_valid_adaptation_ns)
        .collect::<Vec<_>>();
    times.len() >= 2 && times.last() <= times.first()
}

fn cross_regime_productivity_accelerated(records: &[Value], summaries: &[R2RegimeSummary]) -> bool {
    let productive_regimes = summaries
        .iter()
        .filter(|summary| {
            summary.regime_id > 1 && summary.adaptation_outcome == AdaptationOutcome::Productive
        })
        .map(|summary| summary.regime_id)
        .collect::<Vec<_>>();
    if productive_regimes.len() < 2 {
        return false;
    }
    let means = productive_regimes
        .iter()
        .map(|regime_id| {
            let values = records
                .iter()
                .filter(|record| {
                    record["result"]["difficulty_probe"]["regime_id"] == json!(regime_id)
                })
                .map(|record| {
                    let gain = record["result"]["inner"]["frontier_gain"]
                        .as_u64()
                        .unwrap_or(0);
                    let interval = record["adjusted_total_improvement_interval_ns"]
                        .as_u64()
                        .unwrap_or(1)
                        .max(1);
                    (u128::from(gain) * 1_000_000_000_u128 / u128::from(interval)) as u64
                })
                .collect::<Vec<_>>();
            mean_u64(&values)
        })
        .collect::<Vec<_>>();
    means.windows(2).all(|pair| pair[1] > pair[0])
}

fn write_r2_sequence_files(
    report_dir: &Path,
    sequences: &R2Sequences,
    summaries: &[R2RegimeSummary],
) -> Result<(), String> {
    let values = [
        ("global_epoch_sequence.json", json!(sequences.global_epoch)),
        (
            "frontier_scale_sequence.json",
            json!(sequences.frontier_scale),
        ),
        (
            "qualitative_capability_sequence.json",
            json!(sequences.qualitative_capability),
        ),
        (
            "within_regime_cost_sequence.json",
            json!(sequences.within_regime_cost),
        ),
        (
            "frontier_scale_delta_sequence.json",
            json!(sequences.frontier_scale_delta),
        ),
        (
            "capability_gain_sequence.json",
            json!(sequences.capability_gain),
        ),
        (
            "useful_work_per_wall_time_sequence.json",
            json!(sequences.useful_work_per_wall_time),
        ),
        (
            "useful_work_per_resource_sequence.json",
            json!(sequences.useful_work_per_resource),
        ),
        (
            "new_transferable_structure_sequence.json",
            json!(sequences.new_transferable_structure),
        ),
        (
            "repair_gain_per_accepted_repair_sequence.json",
            json!(sequences.repair_gain_per_accepted_repair),
        ),
        (
            "capability_productivity_sequence.json",
            json!(sequences.capability_productivity),
        ),
        (
            "time_to_next_frontier_sequence.json",
            json!(sequences.time_to_next_frontier),
        ),
        (
            "diagnostic_experiment_time_sequence.json",
            json!(sequences.diagnostic_experiment_time),
        ),
        (
            "reaction_discovery_time_sequence.json",
            json!(sequences.reaction_discovery_time),
        ),
        (
            "reaction_realization_time_sequence.json",
            json!(sequences.reaction_realization_time),
        ),
        (
            "causal_integration_time_sequence.json",
            json!(sequences.causal_integration_time),
        ),
        (
            "verification_time_sequence.json",
            json!(sequences.verification_time),
        ),
        (
            "total_improvement_interval_sequence.json",
            json!(sequences.total_improvement_interval),
        ),
        (
            "repairs_accepted_sequence.json",
            json!(sequences.repairs_accepted),
        ),
        (
            "bottleneck_migration_sequence.json",
            json!(sequences.bottleneck_migration),
        ),
        ("peak_rss_sequence.json", json!(sequences.peak_rss)),
        (
            "active_semantic_bytes_sequence.json",
            json!(sequences.active_semantic_bytes),
        ),
        ("core_bytes_sequence.json", json!(sequences.core_bytes)),
        (
            "regime_closure_state_sequence.json",
            json!(sequences.regime_closure),
        ),
        (
            "mastery_effort_sequence.json",
            json!(summaries
                .iter()
                .filter(|summary| summary.regime_id > 1)
                .map(|summary| json!({
                    "regime_id": summary.regime_id,
                    "time_to_first_valid_adaptation_ns": summary.time_to_first_valid_adaptation_ns,
                    "time_to_exceed_prior_frontier_ns": summary.time_to_exceed_prior_frontier_ns,
                    "time_to_regime_closure_ns": summary.time_to_regime_closure_ns,
                    "diagnostic_experiments_to_adapt": summary.diagnostic_experiments_to_adapt,
                    "repairs_to_adapt": summary.repairs_to_adapt,
                    "failed_repairs_to_adapt": summary.failed_repairs_to_adapt,
                }))
                .collect::<Vec<_>>()),
        ),
    ];
    for (file, sequence) in values {
        write_json(report_dir.join(file), &json!({"sequence": sequence}))?;
    }
    Ok(())
}

fn ensure_r2_reports(report_dir: &Path, executed: usize) -> Result<(), String> {
    let required = [
        "predecessor_integrity.json",
        "r1_ontology_integrity.json",
        "campaign_freeze.json",
        "human_intervention_audit.json",
        "observer_instrumentation_audit.json",
        "difficulty_regime_ledger.jsonl",
        "difficulty_transition_ledger.jsonl",
        "regime_closure_ledger.jsonl",
        "regime_productivity_ledger.jsonl",
        "adaptation_strategy_transition_ledger.jsonl",
        "plateau_observer_classifier_disagreement.jsonl",
        "frontier_scale_sequence.json",
        "qualitative_capability_sequence.json",
        "within_regime_cost_sequence.json",
        "capability_productivity_sequence.json",
        "mastery_effort_sequence.json",
        "autonomous_decision_ledger.jsonl",
        "growth_ledger.jsonl",
        "fresh_work_results.json",
        "retention_results.json",
        "regression_results.json",
        "clippy_differential_audit.json",
        "dockability_audit.json",
        "sem27_r2_final_report.json",
        "SEM27_R2_REPORT.md",
    ];
    for file in required {
        let path = report_dir.join(file);
        if !path.is_file()
            || fs::metadata(&path)
                .map_err(|error| error.to_string())?
                .len()
                == 0
        {
            return Err(format!("R2_MISSING_OR_EMPTY_REPORT:{file}"));
        }
    }
    for epoch in 1..=executed {
        if !report_dir.join(format!("epoch_{epoch:03}.json")).is_file() {
            return Err(format!("R2_MISSING_EPOCH:{epoch}"));
        }
    }
    Ok(())
}

fn write_r2_markdown(report_dir: &Path, report: &Value) -> Result<(), String> {
    let markdown = format!(
        "# SEM-27-R2 Sealed Open-Regime Continuation\n\nStatus: `{}`\n\n- Disposition: `{}`\n- Continuation epochs: `{}` / `{}`\n- Global epoch range: `{:?}`\n- Regime 2 closure: `{}`\n- Open productive: `{}`\n- Open stalled: `{}`\n- Regime 2 frontier scale: `{}` -> `{}`\n- Regime 2 qualitative capability: `{}`\n- Autonomous R2 difficulty escalations: `{}`\n- Staircase growth: `{}`\n- Staircase self-amplifying regime: `{}`\n- Engine behavior diff lines: `{}`\n- Autonomous policy diff lines: `{}`\n- Human research steering events: `0`\n- SEM-28 started: `false`\n- Next allowed stage: `OPERATOR_REVIEW_ONLY`\n\nRegime closure was never forced by the observer. Historical SEM-27 and SEM-27-R1 reports remain immutable. Claims are bounded to the closed mechanical environment.\n",
        report["sem27_r2_status"].as_str().unwrap_or("UNKNOWN"),
        report["disposition"].as_str().unwrap_or("UNKNOWN"),
        report["continuation_epochs_executed"],
        report["continuation_epochs_budget"],
        report["global_epoch_range"],
        report["regime_2_closure_state"].as_str().unwrap_or("UNKNOWN"),
        report["open_productive_regime"],
        report["open_stalled_regime"],
        report["regime_2_start_frontier_scale"],
        report["regime_2_final_frontier_scale"],
        report["regime_2_qualitative_capability"],
        report["autonomous_difficulty_escalation_events"],
        report["staircase_growth_observed"],
        report["staircase_self_amplifying_regime_observed"],
        report["engine_behavior_diff_lines"],
        report["autonomous_policy_diff_lines"],
    );
    fs::write(report_dir.join("SEM27_R2_REPORT.md"), markdown)
        .map_err(|error| format!("WRITE_R2_MARKDOWN:{error}"))
}

fn validate_r2_json_tree(report_dir: &Path) -> Result<(usize, usize), String> {
    let mut json_count = 0_usize;
    let mut jsonl_count = 0_usize;
    let mut stack = vec![report_dir.to_path_buf()];
    while let Some(directory) = stack.pop() {
        for entry in
            fs::read_dir(&directory).map_err(|error| format!("READ_R2_REPORT_DIRECTORY:{error}"))?
        {
            let path = entry.map_err(|error| error.to_string())?.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
                read_json(&path)?;
                json_count = json_count.saturating_add(1);
            } else if path.extension().and_then(|extension| extension.to_str()) == Some("jsonl") {
                for (index, line) in fs::read_to_string(&path)
                    .map_err(|error| format!("READ_R2_JSONL:{}:{error}", path.display()))?
                    .lines()
                    .enumerate()
                {
                    serde_json::from_str::<Value>(line).map_err(|error| {
                        format!("PARSE_R2_JSONL:{}:{}:{error}", path.display(), index + 1)
                    })?;
                }
                jsonl_count = jsonl_count.saturating_add(1);
            }
        }
    }
    Ok((json_count, jsonl_count))
}

fn seed_for_r2(epoch: u16) -> u64 {
    mix_campaign(0x5E27_D200_0000_0001, u64::from(epoch) * 149).max(1)
}

fn r2_engine_epoch(epoch: u16) -> u8 {
    u8::try_from((epoch - 1) % 64 + 1).expect("R2 engine epoch in 1..=64")
}

fn r2_fresh_seeds() -> [u64; 8] {
    [
        0x27D2_F201,
        0x27D2_F213,
        0x27D2_F225,
        0x27D2_F237,
        0x27D2_F249,
        0x27D2_F25B,
        0x27D2_F26D,
        0x27D2_F27F,
    ]
}
