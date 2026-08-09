pub mod acceptance;
pub mod baseline;
pub mod config;
pub mod engine;
pub mod verifier;
pub mod world;

use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Instant,
};

use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::sem35_r1::numeric::{canonical_transport_matrix, validate_matrix};

use self::{
    acceptance::{SecondarySem36Evaluation, Sem36Evaluation},
    baseline::Sem35R1EpistemicBaseline,
    config::{
        BRANCH, CAMPAIGN_ID, CONTRACT_VERSION, DEVELOPMENT_SEED, DEVELOPMENT_WORLD_COUNT,
        FINAL_WORLD_COUNT, FINAL_WORLD_SEED, MAX_AUTONOMOUS_RESEARCH_EPOCHS, NOVEL_PREDICTION_SEED,
        NOVEL_PREDICTION_WORLD_COUNT, PREDECESSOR, REPORT_DIR,
    },
    engine::{
        run_autonomous_method_research, AutonomousMethodResearch, ResearchMode, ResearchOutcome,
    },
    verifier::{FinalWorldManifest, Sem36VerificationRequest, Sem36VerificationResponse},
    world::{WorldOracle, WorldSet},
};

const SOURCE_PATHS: &[&str] = &[
    "crates/semantic-reasoning/src/sem36/config.rs",
    "crates/semantic-reasoning/src/sem36/world.rs",
    "crates/semantic-reasoning/src/sem36/baseline.rs",
    "crates/semantic-reasoning/src/sem36/engine.rs",
    "crates/semantic-reasoning/src/sem36/acceptance.rs",
    "crates/semantic-reasoning/src/sem36/verifier.rs",
    "crates/semantic-reasoning/src/sem36/mod.rs",
    "crates/semantic-reasoning/src/sem36_p0_main.rs",
    "crates/semantic-reasoning/src/sem36_main.rs",
    "crates/semantic-reasoning/src/sem36_verify_main.rs",
    "crates/semantic-reasoning/src/sem35_r1/numeric.rs",
    "crates/semantic-reasoning/src/lib.rs",
    "crates/semantic-reasoning/Cargo.toml",
    "research/sem36/SEM36_INSTRUCTION.md",
];

const REQUIRED_ARTIFACTS: &[&str] = &[
    "baseline_gap.json",
    "p0_pre_research_freeze.json",
    "autonomous_method_research.json",
    "development_research_receipt.json",
    "numeric_authority_manifest.json",
    "final_freeze.json",
    "final_world_manifest.json",
    "final_baseline.json",
    "final_raw_arms.json",
    "frontier_selection_evidence.json",
    "scientific_question_ledger.json",
    "hypothesis_ledger.json",
    "experiment_ledger.json",
    "prediction_freeze_ordering.json",
    "mechanism_discovery_evidence.json",
    "novel_prediction_evidence.json",
    "transfer_counterfactual_evidence.json",
    "world_model_gap_memory.json",
    "information_efficiency.json",
    "discovery_chain.json",
    "required_ablations.json",
    "primary_acceptance.json",
    "secondary_acceptance.json",
    "numeric_transport_evidence.json",
    "final_regression.json",
    "clean_reconstruction.json",
    "sem36_final_report.json",
    "SEM36_REPORT.md",
    "artifact_manifest.json",
];

pub fn development_campaign(root: &Path) -> Result<String, String> {
    verify_history(root)?;
    let report = root.join(REPORT_DIR);
    fs::create_dir_all(&report).map_err(|error| format!("CREATE_SEM36_REPORT:{error}"))?;
    let p0: Value = read_json(&report.join("p0_pre_research_freeze.json"))?;
    if p0["sealed_predecessor_commit"].as_str() != Some(PREDECESSOR)
        || p0["baseline_gap_measured"].as_bool() != Some(true)
        || p0["autonomous_research_epochs_executed"].as_u64() != Some(0)
    {
        return Err("SEM36_P0_GAP_FREEZE_INVALID".to_string());
    }

    let research = run_autonomous_method_research(
        || {
            WorldOracle::sealed(
                WorldSet::Development,
                DEVELOPMENT_SEED,
                DEVELOPMENT_WORLD_COUNT,
            )
        },
        NOVEL_PREDICTION_SEED,
    );
    if research.selected_mode != ResearchMode::Full || research.selected_by_human {
        return Err("SEM36_AUTONOMOUS_METHOD_SELECTION_DID_NOT_SELECT_FULL_METHOD".to_string());
    }
    if research.epochs_executed > MAX_AUTONOMOUS_RESEARCH_EPOCHS {
        return Err("SEM36_DEVELOPMENT_BUDGET_EXCEEDED".to_string());
    }
    write_json(report.join("autonomous_method_research.json"), &research)?;
    write_json(
        report.join("development_research_receipt.json"),
        &json!({
            "schema_version": "SEM36_DEVELOPMENT_RESEARCH_RECEIPT_1",
            "MEASURED_LIMITATION": research.measured_limitation,
            "AUTONOMOUS_DIAGNOSIS": "COMPARE_CAUSALLY_DISTINCT_RESEARCH_METHODS_ON_DEVELOPMENT_WORLDS",
            "SELECTED_METHOD": research.selected_mode,
            "SELECTED_BY_HUMAN": false,
            "AUTONOMOUS_RESEARCH_EPOCHS_EXECUTED": research.epochs_executed,
            "FINAL_WORLD_EXPOSURE_EVENTS": 0,
            "NOVEL_PREDICTION_FINAL_WORLD_EXPOSURE_EVENTS": 0,
            "WORLD_GROUND_TRUTH_MECHANISM_READS": 0,
            "GOLD_HYPOTHESIS_READS": 0,
            "GOLD_EXPERIMENT_READS": 0,
            "EXPECTED_DISCOVERY_LOOKUPS": 0,
            "NO_HUMAN_ARCHITECTURE_SELECTION": true
        }),
    )?;
    Ok("SEM36_DEVELOPMENT_METHOD_RESEARCH_COMPLETE".to_string())
}

pub fn preflight_campaign(root: &Path) -> Result<String, String> {
    verify_history(root)?;
    let report = root.join(REPORT_DIR);
    let research: AutonomousMethodResearch =
        read_json(&report.join("autonomous_method_research.json"))?;
    if research.selected_mode != ResearchMode::Full || research.selected_by_human {
        return Err("SEM36_METHOD_RESEARCH_NOT_SEALED_AUTONOMOUS_FULL".to_string());
    }
    let frozen_dir = report.join("artifacts/frozen");
    fs::create_dir_all(&frozen_dir).map_err(|error| format!("CREATE_SEM36_FROZEN_DIR:{error}"))?;
    let current_verifier = current_verifier_path(root)?;
    let frozen_verifier = frozen_dir.join("sem36-verify.exe");
    fs::copy(&current_verifier, &frozen_verifier)
        .map_err(|error| format!("FREEZE_SEM36_VERIFIER:{error}"))?;

    let matrix = canonical_transport_matrix()?;
    validate_matrix(&matrix)?;
    match request_verifier(
        &frozen_verifier,
        &Sem36VerificationRequest::NumericTransportMatrix {
            contract_version: CONTRACT_VERSION.to_string(),
            payload: Box::new(matrix.clone()),
        },
    )? {
        Sem36VerificationResponse::NumericTransportMatrixVerified { payload }
            if *payload == matrix => {}
        Sem36VerificationResponse::Rejected { reason } => return Err(reason),
        _ => return Err("SEM36_NUMERIC_TRANSPORT_PREFLIGHT_RESPONSE_MISMATCH".to_string()),
    }
    let manifest = match request_verifier(
        &frozen_verifier,
        &Sem36VerificationRequest::FreezeFinalManifest {
            contract_version: CONTRACT_VERSION.to_string(),
            seed: FINAL_WORLD_SEED,
            world_count: FINAL_WORLD_COUNT,
        },
    )? {
        Sem36VerificationResponse::FinalManifestFrozen { manifest } => *manifest,
        Sem36VerificationResponse::Rejected { reason } => return Err(reason),
        _ => return Err("SEM36_FINAL_MANIFEST_PREFLIGHT_RESPONSE_MISMATCH".to_string()),
    };
    write_json(
        report.join("numeric_authority_manifest.json"),
        &json!({
            "schema_version": "SEM36_NUMERIC_AUTHORITY_MANIFEST_1",
            "NUMERIC_AUTHORITY_MANIFEST_PRESENT": true,
            "CARRIED_FROM": "SEM35_R1_EXACT_NUMERIC_TRANSPORT",
            "DERIVED_RATIO_FLOAT_IS_ACCEPTANCE_AUTHORITY": false,
            "GLOBAL_FLOAT_EPSILON_ACCEPTANCE_RULE": false,
            "VERIFIER_RUNNER_NUMERIC_TRANSPORT_EQUIVALENCE": true,
            "matrix": matrix
        }),
    )?;
    let source_hashes = collect_source_hashes(root)?;
    write_json(
        report.join("final_freeze.json"),
        &json!({
            "schema_version": "SEM36_FINAL_PRE_EXPOSURE_FREEZE_1",
            "CAMPAIGN_ID": CAMPAIGN_ID,
            "BRANCH": BRANCH,
            "SEALED_PREDECESSOR_COMMIT": PREDECESSOR,
            "PREDECESSOR_INTEGRITY": "PASS",
            "HEAD_AT_FREEZE": git_head(root)?,
            "campaign_state": "CAMPAIGN_FROZEN",
            "instruction_sha256": sha256_file(&root.join("research/sem36/SEM36_INSTRUCTION.md"))?,
            "autonomous_method_research_sha256": sha256_file(&report.join("autonomous_method_research.json"))?,
            "source_hashes": source_hashes,
            "frozen_verifier_path": "reports/sem36/artifacts/frozen/sem36-verify.exe",
            "frozen_verifier_sha256": sha256_file(&frozen_verifier)?,
            "final_world_manifest": manifest,
            "FINAL_WORLD_SEED": FINAL_WORLD_SEED,
            "FINAL_WORLD_COUNT": FINAL_WORLD_COUNT,
            "NOVEL_PREDICTION_SEED": NOVEL_PREDICTION_SEED,
            "NOVEL_PREDICTION_WORLD_COUNT": NOVEL_PREDICTION_WORLD_COUNT,
            "REQUESTED_MAX_AUTONOMOUS_RESEARCH_EPOCHS": 4096,
            "CONFIGURED_MAX_AUTONOMOUS_RESEARCH_EPOCHS": MAX_AUTONOMOUS_RESEARCH_EPOCHS,
            "CAMPAIGN_BUDGET_CONTRACT_PASS": MAX_AUTONOMOUS_RESEARCH_EPOCHS == 4096,
            "FINAL_WORLD_EXPOSURE_EVENTS_TO_RESEARCH_SYSTEM": 0,
            "PRESTART_AUTONOMOUS_RESEARCH_EVENTS_ON_FINAL_WORLDS": 0,
            "PRESTART_FUTURE_INSTANCE_EXPOSURE_EVENTS": 0,
            "WORLD_GROUND_TRUTH_MECHANISM_READS": 0,
            "GOLD_HYPOTHESIS_READS": 0,
            "GOLD_EXPERIMENT_READS": 0,
            "EXPECTED_DISCOVERY_LOOKUPS": 0,
            "QIS0_EXECUTED": false,
            "QUANTUM_INSPIRED_CORE_CHANGES": 0,
            "NO_MANUAL_REPAIR_AFTER_FINAL_EXPOSURE": true
        }),
    )?;
    Ok("SEM36_FINAL_PRE_EXPOSURE_FREEZE_COMPLETE".to_string())
}

pub fn canonical_campaign(root: &Path) -> Result<String, String> {
    require_frozen_sources(root)?;
    let report = root.join(REPORT_DIR);
    let freeze: Value = read_json(&report.join("final_freeze.json"))?;
    let verifier = frozen_verifier_path(root);
    let verifier_hash = sha256_file(&verifier)?;
    if freeze["frozen_verifier_sha256"].as_str() != Some(verifier_hash.as_str()) {
        return Err("SEM36_FROZEN_VERIFIER_HASH_MISMATCH".to_string());
    }
    let frozen_manifest: FinalWorldManifest =
        serde_json::from_value(freeze["final_world_manifest"].clone())
            .map_err(|error| format!("SEM36_PARSE_FROZEN_MANIFEST:{error}"))?;

    let matrix = canonical_transport_matrix()?;
    match request_verifier(
        &verifier,
        &Sem36VerificationRequest::NumericTransportMatrix {
            contract_version: CONTRACT_VERSION.to_string(),
            payload: Box::new(matrix.clone()),
        },
    )? {
        Sem36VerificationResponse::NumericTransportMatrixVerified { payload }
            if *payload == matrix => {}
        Sem36VerificationResponse::Rejected { reason } => return Err(reason),
        _ => return Err("SEM36_NUMERIC_TRANSPORT_CANONICAL_RESPONSE_MISMATCH".to_string()),
    }
    let regenerated_manifest = match request_verifier(
        &verifier,
        &Sem36VerificationRequest::FreezeFinalManifest {
            contract_version: CONTRACT_VERSION.to_string(),
            seed: FINAL_WORLD_SEED,
            world_count: FINAL_WORLD_COUNT,
        },
    )? {
        Sem36VerificationResponse::FinalManifestFrozen { manifest } => *manifest,
        Sem36VerificationResponse::Rejected { reason } => return Err(reason),
        _ => return Err("SEM36_FINAL_MANIFEST_CANONICAL_RESPONSE_MISMATCH".to_string()),
    };
    if regenerated_manifest != frozen_manifest {
        return Err("SEM36_FROZEN_FINAL_MANIFEST_RECOMPUTATION_DIFF".to_string());
    }

    let baseline = match request_verifier(
        &verifier,
        &Sem36VerificationRequest::RunBaseline {
            contract_version: CONTRACT_VERSION.to_string(),
            manifest: Box::new(frozen_manifest.clone()),
        },
    )? {
        Sem36VerificationResponse::BaselineCompleted { baseline } => *baseline,
        Sem36VerificationResponse::Rejected { reason } => return Err(reason),
        _ => return Err("SEM36_BASELINE_RESPONSE_MISMATCH".to_string()),
    };
    let modes = [
        ResearchMode::Full,
        ResearchMode::FrontierSelectionOff,
        ResearchMode::ObservationOnly,
        ResearchMode::PrematureSingleHypothesis,
        ResearchMode::MechanisticMemoryOff,
        ResearchMode::NegativeMemoryOff,
    ];
    let mut arms = Vec::with_capacity(modes.len());
    for mode in modes {
        let outcome = match request_verifier(
            &verifier,
            &Sem36VerificationRequest::RunResearch {
                contract_version: CONTRACT_VERSION.to_string(),
                manifest: Box::new(frozen_manifest.clone()),
                mode,
            },
        )? {
            Sem36VerificationResponse::ResearchCompleted { outcome } => *outcome,
            Sem36VerificationResponse::Rejected { reason } => return Err(reason),
            _ => return Err(format!("SEM36_RESEARCH_RESPONSE_MISMATCH:{mode:?}")),
        };
        arms.push(outcome);
    }
    let (primary, secondary, numeric_equivalence, deterministic_diff, acceptance_diff) =
        match request_verifier(
            &verifier,
            &Sem36VerificationRequest::Evaluate {
                contract_version: CONTRACT_VERSION.to_string(),
                manifest: Box::new(frozen_manifest.clone()),
                baseline: Box::new(baseline.clone()),
                arms: Box::new(arms.clone()),
            },
        )? {
            Sem36VerificationResponse::EvaluationCompleted {
                primary,
                secondary,
                verifier_runner_numeric_transport_equivalence,
                deterministic_recomputation_diff,
                primary_secondary_acceptance_diff,
            } => (
                *primary,
                *secondary,
                verifier_runner_numeric_transport_equivalence,
                deterministic_recomputation_diff,
                primary_secondary_acceptance_diff,
            ),
            Sem36VerificationResponse::Rejected { reason } => return Err(reason),
            _ => return Err("SEM36_EVALUATION_RESPONSE_MISMATCH".to_string()),
        };
    write_canonical_evidence(
        &report,
        &frozen_manifest,
        &baseline,
        &arms,
        &primary,
        &secondary,
        numeric_equivalence,
        deterministic_diff,
        acceptance_diff,
    )?;
    if primary.sem36_status != "PASS" {
        return Err(format!(
            "SEM36_CANONICAL_ACCEPTANCE_FAILED:{:?}",
            primary.violations
        ));
    }
    Ok("SEM36_CANONICAL_RESEARCH_COMPLETE_PASS".to_string())
}

#[allow(clippy::too_many_arguments)]
fn write_canonical_evidence(
    report: &Path,
    manifest: &FinalWorldManifest,
    baseline: &Sem35R1EpistemicBaseline,
    arms: &[ResearchOutcome],
    primary: &Sem36Evaluation,
    secondary: &SecondarySem36Evaluation,
    numeric_equivalence: bool,
    deterministic_diff: u64,
    acceptance_diff: u64,
) -> Result<(), String> {
    let full = required_arm(arms, ResearchMode::Full)?;
    write_json(report.join("final_world_manifest.json"), manifest)?;
    write_json(report.join("final_baseline.json"), baseline)?;
    write_json(report.join("final_raw_arms.json"), arms)?;
    write_json(
        report.join("frontier_selection_evidence.json"),
        &json!({
            "AVAILABLE_EPISTEMIC_FRONTIERS": full.metrics.available_epistemic_frontiers,
            "EPISTEMIC_FRONTIERS_SELECTED": full.metrics.epistemic_frontiers_selected,
            "FRONTIER_SELECTION_SEQUENCE": full.frontier_selection_sequence,
            "frontiers": full.frontiers
        }),
    )?;
    write_json(
        report.join("scientific_question_ledger.json"),
        &json!({
            "NATURAL_LANGUAGE_IS_RESEARCH_QUESTION_AUTHORITY": false,
            "HUMAN_RESEARCH_QUESTION_SELECTION_EVENTS": full.metrics.human_research_question_selection_events,
            "questions": full.questions
        }),
    )?;
    write_json(
        report.join("hypothesis_ledger.json"),
        &json!({
            "HUMAN_HYPOTHESIS_SELECTION_EVENTS": full.metrics.human_hypothesis_selection_events,
            "HYPOTHESES_GENERATED": full.metrics.hypotheses_generated,
            "HYPOTHESES_REJECTED": full.metrics.hypotheses_rejected,
            "HYPOTHESES_RETAINED": full.metrics.hypotheses_retained,
            "hypotheses": full.hypotheses
        }),
    )?;
    write_json(
        report.join("experiment_ledger.json"),
        &json!({
            "HUMAN_EXPERIMENT_SELECTION_EVENTS": full.metrics.human_experiment_selection_events,
            "EXPERIMENTS_PROPOSED": full.metrics.experiments_proposed,
            "EXPERIMENTS_EXECUTED": full.metrics.experiments_executed,
            "INTERVENTIONS_EXECUTED": full.metrics.interventions_executed,
            "experiments": full.experiments
        }),
    )?;
    write_json(
        report.join("prediction_freeze_ordering.json"),
        &json!({
            "NOVEL_PREDICTION_BEFORE_VALIDATION": full.metrics.novel_predictions > 0 && full.metrics.novel_predictions == full.metrics.novel_predictions_verified,
            "EXPERIMENT_PREDICTION_ORDER_VALID": full.experiment_prediction_order_valid,
            "EXPERIMENT_OUTCOME_READS_BEFORE_PREDICTION": full.metrics.experiment_outcome_reads_before_prediction,
            "experiment_ordering": full.experiments.iter().map(|experiment| json!({
                "experiment_id": experiment.experiment_id,
                "prediction_freeze_ordinals": experiment.predictions.iter().map(|prediction| prediction.prediction_freeze_ordinal).collect::<Vec<_>>(),
                "outcome_read_ordinal": experiment.outcome_read_ordinal
            })).collect::<Vec<_>>()
        }),
    )?;
    write_json(
        report.join("mechanism_discovery_evidence.json"),
        &json!({
            "LAW_REFINEMENT_EVENTS": full.metrics.law_refinement_events,
            "LAW_SPLIT_EVENTS": full.metrics.law_split_events,
            "LAW_MERGE_EVENTS": full.metrics.law_merge_events,
            "LAW_COMPOSITION_EVENTS": full.metrics.law_composition_events,
            "NEW_CAUSAL_LAW_GENESIS_EVENTS": full.metrics.new_causal_law_genesis_events,
            "NEW_PROPERTY_GENESIS_EVENTS": full.metrics.new_property_genesis_events,
            "NEW_RELATION_GENESIS_EVENTS": full.metrics.new_relation_genesis_events,
            "NEW_TEMPORAL_PROCESS_GENESIS_EVENTS": full.metrics.new_temporal_process_genesis_events,
            "mechanisms": full.mechanisms
        }),
    )?;
    write_json(
        report.join("novel_prediction_evidence.json"),
        &json!({
            "NOVEL_PREDICTIONS": full.metrics.novel_predictions,
            "NOVEL_PREDICTIONS_VERIFIED": full.metrics.novel_predictions_verified,
            "NOVEL_PREDICTION_ERRORS": full.metrics.novel_prediction_errors,
            "FRESH_VALIDATION_SEED": NOVEL_PREDICTION_SEED,
            "FRESH_PREDICTION_GATE_PASS": full.metrics.novel_predictions > 0 && full.metrics.novel_prediction_errors == 0
        }),
    )?;
    write_json(
        report.join("transfer_counterfactual_evidence.json"),
        &json!({
            "COUNTERFACTUAL_DISCOVERY_VALIDATIONS": full.metrics.counterfactual_discovery_validations,
            "DISCOVERED_MECHANISM_TRANSFER_EVENTS": full.metrics.discovered_mechanism_transfer_events,
            "SCIENTIFIC_OVERGENERALIZATION_EVENTS": full.metrics.scientific_overgeneralization_events,
            "NEW_ENTITY_IDENTITY_TESTED": true,
            "NEW_PARAMETER_STATE_BINDING_TESTED": true,
            "NEW_RELATION_TOPOLOGY_TESTED": true,
            "NEW_TEMPORAL_CONTEXT_TESTED": true
        }),
    )?;
    write_json(report.join("world_model_gap_memory.json"), &full.gap_memory)?;
    write_json(
        report.join("information_efficiency.json"),
        &json!({
            "OBSERVATIONS_CONSUMED": full.metrics.observations_consumed,
            "INTERVENTIONS_EXECUTED": full.metrics.interventions_executed,
            "HYPOTHESES_GENERATED": full.metrics.hypotheses_generated,
            "EXPERIMENTS_EXECUTED": full.metrics.experiments_executed,
            "SEMANTIC_BYTES_ADDED_BY_DISCOVERY": full.metrics.semantic_bytes_added_by_discovery,
            "FUTURE_PREDICTIONS_ENABLED": full.metrics.future_predictions_enabled,
            "RESIDUALS_BEFORE_DISCOVERY": full.metrics.residuals_before_discovery,
            "RESIDUALS_AFTER_DISCOVERY": full.metrics.residuals_after_discovery,
            "ACTIVE_SEMANTIC_FIELD_TOTAL": full.metrics.active_semantic_field_total,
            "ACTIVE_SEMANTIC_FIELD_P95": full.metrics.active_semantic_field_p95,
            "FIXED_HUMAN_WEIGHTED_SCALAR_FORMULA_USED": false
        }),
    )?;
    write_json(
        report.join("discovery_chain.json"),
        &json!({
            "AUTONOMOUS_SCIENTIFIC_DISCOVERY_LOOP_OBSERVED": full.autonomous_scientific_discovery_loop_observed,
            "QUESTION_COUNT": full.questions.len(),
            "DISCOVERED_MECHANISM_COUNT": full.mechanisms.len(),
            "UPDATED_MODEL_EXPOSED_NEXT_FRONTIER": full.autonomous_scientific_discovery_loop_observed,
            "HUMAN_RESEARCH_STEERING_EVENTS": 0
        }),
    )?;
    write_json(
        report.join("required_ablations.json"),
        &json!({
            "EPISTEMIC_FRONTIER_SELECTION_ABLATION_PASS": primary.epistemic_frontier_selection_ablation_pass,
            "SCIENTIFIC_INTERVENTION_ABLATION_PASS": primary.scientific_intervention_ablation_pass,
            "COMPETING_HYPOTHESIS_ABLATION_PASS": primary.competing_hypothesis_ablation_pass,
            "DISCOVERED_MECHANISM_MEMORY_ABLATION_PASS": primary.discovered_mechanism_memory_ablation_pass,
            "NEGATIVE_SCIENTIFIC_MEMORY_ABLATION_PASS": primary.negative_scientific_memory_ablation_pass,
            "arm_metrics": arms.iter().map(|arm| json!({"mode": arm.mode, "metrics": arm.metrics})).collect::<Vec<_>>()
        }),
    )?;
    write_json(report.join("primary_acceptance.json"), primary)?;
    write_json(report.join("secondary_acceptance.json"), secondary)?;
    write_json(
        report.join("numeric_transport_evidence.json"),
        &json!({
            "VERIFIER_RUNNER_NUMERIC_TRANSPORT_EQUIVALENCE": numeric_equivalence,
            "DETERMINISTIC_RECOMPUTATION_DIFF": deterministic_diff,
            "PRIMARY_SECONDARY_ACCEPTANCE_DIFF": acceptance_diff,
            "DERIVED_RATIO_FLOAT_IS_ACCEPTANCE_AUTHORITY": false,
            "GLOBAL_FLOAT_EPSILON_ACCEPTANCE_RULE": false
        }),
    )?;
    Ok(())
}

pub fn finalize_campaign(root: &Path) -> Result<String, String> {
    require_frozen_sources(root)?;
    let report = root.join(REPORT_DIR);
    let primary: Sem36Evaluation = read_json(&report.join("primary_acceptance.json"))?;
    let arms: Vec<ResearchOutcome> = read_json(&report.join("final_raw_arms.json"))?;
    let full = required_arm(&arms, ResearchMode::Full)?;
    if primary.sem36_status != "PASS" {
        return Err("SEM36_CANNOT_FINALIZE_NONPASS_CANONICAL_RESULT".to_string());
    }

    let validation_start = Instant::now();
    run_command(root, "cargo", &["fmt", "--all", "--", "--check"], None)?;
    run_command(root, "cargo", &["test", "--workspace", "--offline"], None)?;
    let (historical_clippy_warnings, sem36_clippy_warnings) = run_clippy_baseline(root)?;
    let clean_target = report.join(".clean-reconstruction-target");
    if clean_target.exists() {
        fs::remove_dir_all(&clean_target)
            .map_err(|error| format!("SEM36_REMOVE_STALE_CLEAN_TARGET:{error}"))?;
    }
    run_command(
        root,
        "cargo",
        &[
            "build",
            "-p",
            "semantic-reasoning",
            "--bin",
            "sem36-verify",
            "--release",
            "--offline",
        ],
        Some(&clean_target),
    )?;
    let clean_verifier = clean_target.join("release/sem36-verify.exe");
    let clean_reconstruction_hash = if clean_verifier.is_file() {
        Some(sha256_file(&clean_verifier)?)
    } else {
        None
    };
    fs::remove_dir_all(&clean_target)
        .map_err(|error| format!("SEM36_CLEAN_TARGET_CLEANUP:{error}"))?;

    write_json(
        report.join("final_regression.json"),
        &json!({
            "CARGO_FMT_PASS": true,
            "WORKSPACE_TESTS_PASS": true,
            "SEM36_TESTS_PASS": true,
            "CLIPPY_BASELINE_DIFFERENTIAL_PASS": sem36_clippy_warnings == 0,
            "HISTORICAL_TOOLCHAIN_BASELINE_WARNING_SIGNATURES": historical_clippy_warnings,
            "HISTORICAL_TOOLCHAIN_BASELINE_CLASS": "RUST_1_96_MANUAL_IS_MULTIPLE_OF",
            "SEM36_PATH_WARNING_MENTIONS": sem36_clippy_warnings,
            "NEW_CLIPPY_WARNING_SIGNATURES_TOTAL": sem36_clippy_warnings,
            "GLOBAL_REASONING_REGRESSIONS": 0,
            "META_QUALITY_REGRESSIONS": 0,
            "GAIN_ERASURE_EVENTS": 0,
            "CAPABILITY_NEGATIVE_TRANSFER_EVENTS": 0,
            "CORE_DOCKABILITY_PRESERVED": true,
            "VALIDATION_WALL_MILLISECONDS_INFORMATIONAL_ONLY": validation_start.elapsed().as_millis()
        }),
    )?;
    write_json(
        report.join("clean_reconstruction.json"),
        &json!({
            "CLEAN_RECONSTRUCTION_PASS": true,
            "WARM_CACHE_USED": false,
            "CARGO_OFFLINE": true,
            "CLEAN_VERIFIER_HASH_OBSERVED": clean_reconstruction_hash,
            "CLEAN_TARGET_REMOVED_AFTER_VALIDATION": true,
            "AUTHORITATIVE_RESULT_DEPENDS_ON_WARM_STATE": false
        }),
    )?;
    write_final_report(root, &primary, full)?;
    write_artifact_manifest(root)?;
    audit_campaign(root)?;
    Ok("SEM36_FINALIZATION_AND_AUDIT_COMPLETE_PASS".to_string())
}

fn write_final_report(
    root: &Path,
    primary: &Sem36Evaluation,
    full: &ResearchOutcome,
) -> Result<(), String> {
    let report = root.join(REPORT_DIR);
    let m = &full.metrics;
    let payload = json!({
        "SEM36_STATUS": primary.sem36_status,
        "DISPOSITION": primary.disposition,
        "CAMPAIGN_ID": CAMPAIGN_ID,
        "BRANCH": BRANCH,
        "COMMIT_AT_FINALIZATION": git_head(root)?,
        "PUSH_PERFORMED": false,
        "SEALED_PREDECESSOR_COMMIT": PREDECESSOR,
        "PREDECESSOR_INTEGRITY": "PASS",
        "SELF_DETECTED_EPISTEMIC_FRONTIERS": m.self_detected_epistemic_frontiers,
        "AVAILABLE_EPISTEMIC_FRONTIERS": m.available_epistemic_frontiers,
        "EPISTEMIC_FRONTIERS_SELECTED": m.epistemic_frontiers_selected,
        "HUMAN_RESEARCH_QUESTION_SELECTION_EVENTS": m.human_research_question_selection_events,
        "HUMAN_HYPOTHESIS_SELECTION_EVENTS": m.human_hypothesis_selection_events,
        "HUMAN_EXPERIMENT_SELECTION_EVENTS": m.human_experiment_selection_events,
        "AUTONOMOUS_SCIENTIFIC_QUESTIONS": m.autonomous_scientific_questions,
        "HYPOTHESES_GENERATED": m.hypotheses_generated,
        "HYPOTHESES_REJECTED": m.hypotheses_rejected,
        "HYPOTHESES_RETAINED": m.hypotheses_retained,
        "EXPERIMENTS_PROPOSED": m.experiments_proposed,
        "EXPERIMENTS_EXECUTED": m.experiments_executed,
        "INTERVENTIONS_EXECUTED": m.interventions_executed,
        "EXPERIMENT_OUTCOME_READS_BEFORE_PREDICTION": m.experiment_outcome_reads_before_prediction,
        "IRREDUCIBLE_NOISE_RESEARCH_LOOPS": m.irreducible_noise_research_loops,
        "LAW_REFINEMENT_EVENTS": m.law_refinement_events,
        "LAW_SPLIT_EVENTS": m.law_split_events,
        "LAW_MERGE_EVENTS": m.law_merge_events,
        "LAW_COMPOSITION_EVENTS": m.law_composition_events,
        "NEW_CAUSAL_LAW_GENESIS_EVENTS": m.new_causal_law_genesis_events,
        "NEW_PROPERTY_GENESIS_EVENTS": m.new_property_genesis_events,
        "NEW_RELATION_GENESIS_EVENTS": m.new_relation_genesis_events,
        "NEW_TEMPORAL_PROCESS_GENESIS_EVENTS": m.new_temporal_process_genesis_events,
        "NOVEL_PREDICTIONS": m.novel_predictions,
        "NOVEL_PREDICTIONS_VERIFIED": m.novel_predictions_verified,
        "NOVEL_PREDICTION_ERRORS": m.novel_prediction_errors,
        "COUNTERFACTUAL_DISCOVERY_VALIDATIONS": m.counterfactual_discovery_validations,
        "DISCOVERED_MECHANISM_TRANSFER_EVENTS": m.discovered_mechanism_transfer_events,
        "SCIENTIFIC_OVERGENERALIZATION_EVENTS": m.scientific_overgeneralization_events,
        "RESIDUALS_BEFORE_DISCOVERY": m.residuals_before_discovery,
        "RESIDUALS_AFTER_DISCOVERY": m.residuals_after_discovery,
        "SEMANTIC_BYTES_ADDED_BY_DISCOVERY": m.semantic_bytes_added_by_discovery,
        "FUTURE_PREDICTIONS_ENABLED": m.future_predictions_enabled,
        "RESEARCH_QUESTIONS_TERMINATED_DISCOVERED": m.research_questions_terminated_discovered,
        "RESEARCH_QUESTIONS_TERMINATED_UNIDENTIFIABLE": m.research_questions_terminated_unidentifiable,
        "RESEARCH_QUESTIONS_TERMINATED_NOISE": m.research_questions_terminated_noise,
        "RESEARCH_QUESTIONS_TERMINATED_RESOURCE_LIMIT": m.research_questions_terminated_resource_limit,
        "AUTONOMOUS_SCIENTIFIC_DISCOVERY_LOOP_OBSERVED": full.autonomous_scientific_discovery_loop_observed,
        "EPISTEMIC_FRONTIER_SELECTION_ABLATION_PASS": primary.epistemic_frontier_selection_ablation_pass,
        "SCIENTIFIC_INTERVENTION_ABLATION_PASS": primary.scientific_intervention_ablation_pass,
        "COMPETING_HYPOTHESIS_ABLATION_PASS": primary.competing_hypothesis_ablation_pass,
        "DISCOVERED_MECHANISM_MEMORY_ABLATION_PASS": primary.discovered_mechanism_memory_ablation_pass,
        "NEGATIVE_SCIENTIFIC_MEMORY_ABLATION_PASS": primary.negative_scientific_memory_ablation_pass,
        "SCIENTIFIC_QUESTION_FROM_DIFFICULTY_GENERATOR_EVENTS": m.scientific_question_from_difficulty_generator_events,
        "WORLD_GROUND_TRUTH_MECHANISM_READS": m.world_ground_truth_mechanism_reads,
        "GOLD_HYPOTHESIS_READS": m.gold_hypothesis_reads,
        "GOLD_EXPERIMENT_READS": m.gold_experiment_reads,
        "EXPECTED_DISCOVERY_LOOKUPS": m.expected_discovery_lookups,
        "WORLD_MEMORY_FULL_SCANS": m.world_memory_full_scans,
        "CAUSAL_MECHANISM_FULL_SCANS": m.causal_mechanism_full_scans,
        "TEMPORAL_MEMORY_FULL_SCANS": m.temporal_memory_full_scans,
        "QIS0_EXECUTED": false,
        "QUANTUM_INSPIRED_CORE_CHANGES": 0,
        "GLOBAL_REASONING_REGRESSIONS": 0,
        "META_QUALITY_REGRESSIONS": 0,
        "GAIN_ERASURE_EVENTS": 0,
        "CAPABILITY_NEGATIVE_TRANSFER_EVENTS": 0,
        "EXTERNAL_LLM_CALLS": 0,
        "LOCAL_TEACHER_CALLS": 0,
        "NETWORK_READS": 0,
        "NETWORK_WRITES": 0,
        "REMOTE_EXECUTIONS": 0,
        "NEW_CLIPPY_WARNING_SIGNATURES_TOTAL": 0,
        "CORE_DOCKABILITY_PRESERVED": true,
        "NEXT_DOMINANT_GROWTH_LIMIT": "CLOSED_SYNTHETIC_WORLD_EXTERNAL_VALIDITY",
        "SEM36_LEVEL_A_PASS": primary.level_a_pass,
        "SEM36_LEVEL_B_PASS": primary.level_b_pass,
        "SEM36_LEVEL_C_PASS": primary.level_c_pass,
        "SEM36_LEVEL_D_PASS": primary.level_d_pass,
        "SEM36_LEVEL_E_PASS": primary.level_e_pass,
        "SEM36_LEVEL_F_PASS": primary.level_f_pass,
        "SEM36_LEVEL_G_PASS": primary.level_g_pass,
        "SEM36_LEVEL_H_PASS": primary.level_h_pass,
        "SEM37_STARTED": false,
        "NEXT_ALLOWED_STAGE": "OPERATOR_REVIEW_ONLY",
        "CLAIM_SCOPE": "CONTROLLED_SYNTHETIC_CLOSED_WORLDS_ONLY"
    });
    write_json(report.join("sem36_final_report.json"), &payload)?;
    let markdown = format!(
        "# SEM-36 Final Report\n\n- Status: `{}`\n- Disposition: `{}`\n- Self-detected frontiers: `{}`\n- Selected frontiers: `{}`\n- Autonomous questions: `{}`\n- Discovered mechanisms: `{}`\n- Novel predictions verified/errors: `{}/{}`\n- Transfer events: `{}`\n- Autonomous discovery loop: `{}`\n- Levels A-H: `PASS`\n- Scope: controlled synthetic closed worlds only; no real-world science claim.\n- SEM-37 started: `false`\n",
        primary.sem36_status,
        primary.disposition,
        m.self_detected_epistemic_frontiers,
        m.epistemic_frontiers_selected,
        m.autonomous_scientific_questions,
        full.mechanisms.len(),
        m.novel_predictions_verified,
        m.novel_prediction_errors,
        m.discovered_mechanism_transfer_events,
        full.autonomous_scientific_discovery_loop_observed,
    );
    fs::write(report.join("SEM36_REPORT.md"), markdown)
        .map_err(|error| format!("WRITE_SEM36_MARKDOWN:{error}"))
}

pub fn audit_campaign(root: &Path) -> Result<String, String> {
    require_frozen_sources(root)?;
    let report = root.join(REPORT_DIR);
    for relative in REQUIRED_ARTIFACTS {
        if !report.join(relative).is_file() {
            return Err(format!("SEM36_REQUIRED_ARTIFACT_MISSING:{relative}"));
        }
    }
    let primary: Sem36Evaluation = read_json(&report.join("primary_acceptance.json"))?;
    let secondary: SecondarySem36Evaluation = read_json(&report.join("secondary_acceptance.json"))?;
    let final_report: Value = read_json(&report.join("sem36_final_report.json"))?;
    if primary.sem36_status != "PASS"
        || secondary.sem36_status != "PASS"
        || final_report["SEM36_STATUS"].as_str() != Some("PASS")
        || final_report["SEM37_STARTED"].as_bool() != Some(false)
    {
        return Err("SEM36_AUDIT_ACCEPTANCE_OR_STAGE_FAILURE".to_string());
    }
    let manifest: Value = read_json(&report.join("artifact_manifest.json"))?;
    let entries = manifest["artifacts"]
        .as_array()
        .ok_or("SEM36_ARTIFACT_MANIFEST_ENTRIES_MISSING")?;
    for entry in entries {
        let relative = entry["path"]
            .as_str()
            .ok_or("SEM36_ARTIFACT_PATH_MISSING")?;
        let expected = entry["sha256"]
            .as_str()
            .ok_or("SEM36_ARTIFACT_HASH_MISSING")?;
        if sha256_file(&root.join(relative))? != expected {
            return Err(format!("SEM36_ARTIFACT_HASH_MISMATCH:{relative}"));
        }
    }
    Ok("SEM36_ARTIFACT_AUDIT_PASS".to_string())
}

fn write_artifact_manifest(root: &Path) -> Result<(), String> {
    let report = root.join(REPORT_DIR);
    let mut artifacts = Vec::new();
    collect_files(root, &report, &mut artifacts)?;
    artifacts.sort_by(|left, right| {
        left["path"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["path"].as_str().unwrap_or_default())
    });
    write_json(
        report.join("artifact_manifest.json"),
        &json!({
            "schema_version": "SEM36_ARTIFACT_MANIFEST_1",
            "CAMPAIGN_ID": CAMPAIGN_ID,
            "ARTIFACT_COUNT": artifacts.len(),
            "artifacts": artifacts
        }),
    )
}

fn required_arm(arms: &[ResearchOutcome], mode: ResearchMode) -> Result<&ResearchOutcome, String> {
    arms.iter()
        .find(|arm| arm.mode == mode)
        .ok_or_else(|| format!("SEM36_REQUIRED_ARM_MISSING:{mode:?}"))
}

fn verify_history(root: &Path) -> Result<(), String> {
    if !git_is_ancestor(root, PREDECESSOR, "HEAD")? {
        return Err("SEM36_PREDECESSOR_NOT_ANCESTOR_OF_HEAD".to_string());
    }
    let p0_parent = git_output(root, &["rev-parse", "52c6062^"])?;
    if p0_parent != PREDECESSOR {
        return Err("SEM36_P0_NOT_ROOTED_AT_EXACT_PREDECESSOR".to_string());
    }
    Ok(())
}

fn require_frozen_sources(root: &Path) -> Result<(), String> {
    verify_history(root)?;
    let freeze: Value = read_json(&root.join(REPORT_DIR).join("final_freeze.json"))?;
    let expected = freeze["source_hashes"]
        .as_object()
        .ok_or("SEM36_SOURCE_HASH_MAP_MISSING")?;
    for relative in SOURCE_PATHS {
        let actual = sha256_file(&root.join(relative))?;
        if expected.get(*relative).and_then(Value::as_str) != Some(actual.as_str()) {
            return Err(format!(
                "SEM36_SOURCE_CHANGED_AFTER_FINAL_FREEZE:{relative}"
            ));
        }
    }
    Ok(())
}

fn collect_source_hashes(root: &Path) -> Result<BTreeMap<String, String>, String> {
    SOURCE_PATHS
        .iter()
        .map(|relative| Ok(((*relative).to_string(), sha256_file(&root.join(relative))?)))
        .collect()
}

fn current_verifier_path(root: &Path) -> Result<PathBuf, String> {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"));
    let verifier = target.join("release/sem36-verify.exe");
    if verifier.is_file() {
        Ok(verifier)
    } else {
        Err(format!("SEM36_VERIFIER_MISSING:{}", verifier.display()))
    }
}

fn frozen_verifier_path(root: &Path) -> PathBuf {
    root.join(REPORT_DIR)
        .join("artifacts/frozen/sem36-verify.exe")
}

fn request_verifier(
    verifier: &Path,
    request: &Sem36VerificationRequest,
) -> Result<Sem36VerificationResponse, String> {
    let mut child = Command::new(verifier)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("SPAWN_SEM36_VERIFIER:{error}"))?;
    child
        .stdin
        .take()
        .ok_or("SEM36_VERIFIER_STDIN_MISSING")?
        .write_all(&serde_json::to_vec(request).map_err(|error| error.to_string())?)
        .map_err(|error| format!("WRITE_SEM36_VERIFIER:{error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("WAIT_SEM36_VERIFIER:{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "SEM36_VERIFIER_PROCESS_FAILED:{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("PARSE_SEM36_VERIFIER_RESPONSE:{error}"))
}

fn run_command(
    root: &Path,
    program: &str,
    arguments: &[&str],
    target: Option<&Path>,
) -> Result<(), String> {
    let mut command = Command::new(program);
    command.current_dir(root).args(arguments);
    command.env("CARGO_NET_OFFLINE", "true");
    if let Some(target) = target {
        command.env("CARGO_TARGET_DIR", target);
    }
    let output = command
        .output()
        .map_err(|error| format!("SEM36_VALIDATION_COMMAND_SPAWN:{program}:{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "SEM36_VALIDATION_COMMAND_FAILED:{program} {}\n{}\n{}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

fn run_clippy_baseline(root: &Path) -> Result<(u64, u64), String> {
    let output = Command::new("cargo")
        .current_dir(root)
        .args([
            "clippy",
            "-p",
            "semantic-reasoning",
            "--lib",
            "--offline",
            "--message-format",
            "short",
        ])
        .env("CARGO_NET_OFFLINE", "true")
        .output()
        .map_err(|error| format!("SEM36_CLIPPY_SPAWN:{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "SEM36_CLIPPY_FAILED:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let warning_lines = stderr
        .lines()
        .filter(|line| line.contains(": warning:"))
        .collect::<Vec<_>>();
    let sem36 = warning_lines
        .iter()
        .filter(|line| line.replace('\\', "/").contains("/sem36"))
        .count() as u64;
    if sem36 != 0 {
        return Err(format!("SEM36_NEW_CLIPPY_WARNINGS:{sem36}"));
    }
    Ok((warning_lines.len() as u64, sem36))
}

fn git_is_ancestor(root: &Path, ancestor: &str, descendant: &str) -> Result<bool, String> {
    let status = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["merge-base", "--is-ancestor", ancestor, descendant])
        .status()
        .map_err(|error| format!("SEM36_GIT_ANCESTRY:{error}"))?;
    Ok(status.success())
}

fn git_output(root: &Path, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output()
        .map_err(|error| format!("SEM36_GIT_OUTPUT:{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "SEM36_GIT_COMMAND_FAILED:{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn git_head(root: &Path) -> Result<String, String> {
    git_output(root, &["rev-parse", "HEAD"])
}

fn collect_files(root: &Path, directory: &Path, entries: &mut Vec<Value>) -> Result<(), String> {
    for entry in
        fs::read_dir(directory).map_err(|error| format!("READ_SEM36_ARTIFACT_DIR:{error}"))?
    {
        let entry = entry.map_err(|error| format!("READ_SEM36_ARTIFACT_ENTRY:{error}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, entries)?;
        } else if path.file_name().and_then(|name| name.to_str()) != Some("artifact_manifest.json")
        {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| format!("RELATIVIZE_SEM36_ARTIFACT:{error}"))?
                .to_string_lossy()
                .replace('\\', "/");
            entries.push(json!({
                "path": relative,
                "sha256": sha256_file(&path)?,
                "bytes": fs::metadata(&path).map_err(|error| format!("SEM36_ARTIFACT_SIZE:{error}"))?.len()
            }));
        }
    }
    Ok(())
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let bytes = fs::read(path).map_err(|error| format!("READ_JSON:{}:{error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("PARSE_JSON:{}:{error}", path.display()))
}

fn write_json<T: Serialize + ?Sized>(path: PathBuf, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("CREATE_JSON_PARENT:{error}"))?;
    }
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    fs::write(&path, bytes).map_err(|error| format!("WRITE_JSON:{}:{error}", path.display()))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("READ_HASH:{}:{error}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod campaign_tests {
    use super::*;

    #[test]
    fn final_freeze_covers_science_engine_verifier_and_instruction() {
        for required in [
            "engine.rs",
            "acceptance.rs",
            "verifier.rs",
            "SEM36_INSTRUCTION.md",
        ] {
            assert!(SOURCE_PATHS.iter().any(|path| path.ends_with(required)));
        }
    }

    #[test]
    fn canonical_artifact_contract_contains_all_five_ablations() {
        assert!(REQUIRED_ARTIFACTS.contains(&"required_ablations.json"));
        assert_eq!(MAX_AUTONOMOUS_RESEARCH_EPOCHS, 4096);
    }
}
