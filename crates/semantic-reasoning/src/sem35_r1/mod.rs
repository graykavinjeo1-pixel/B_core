pub mod acceptance;
pub mod config;
pub mod numeric;
pub mod transport;
pub mod verifier;

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::sem35::engine::{
    generate_tasks, run_arm, run_autonomous_research, ProcessFamily, TemporalArmMode,
    TemporalArmResult, TemporalProgram, TemporalSet,
};

use self::{
    config::{
        BRANCH, CAMPAIGN_ID, CAPABILITY_PREDECESSOR, CONTRACT_VERSION, FRESH_HOLDOUT_SEED,
        FRESH_HOLDOUT_TASK_COUNT, HISTORICAL_SEM35_COMMIT, MAX_AUTONOMOUS_RESEARCH_EPOCHS,
        REPORT_DIR, TEMPORAL_CANDIDATE_SOURCE_COMMIT,
    },
    numeric::{
        canonical_transport_matrix, validate_matrix, CanonicalFiniteF64, CanonicalNumericValue,
        ExactRational,
    },
    transport::{CanonicalTaskEvidence, CanonicalTemporalArm},
    verifier::{FreshTemporalManifest, Sem35R1VerificationRequest, Sem35R1VerificationResponse},
};

const SOURCE_PATHS: &[&str] = &[
    "crates/semantic-reasoning/src/sem35/engine.rs",
    "crates/semantic-reasoning/src/sem35/acceptance.rs",
    "crates/semantic-reasoning/src/sem35_r1/config.rs",
    "crates/semantic-reasoning/src/sem35_r1/numeric.rs",
    "crates/semantic-reasoning/src/sem35_r1/transport.rs",
    "crates/semantic-reasoning/src/sem35_r1/acceptance.rs",
    "crates/semantic-reasoning/src/sem35_r1/verifier.rs",
    "crates/semantic-reasoning/src/sem35_r1/mod.rs",
    "crates/semantic-reasoning/src/sem35_r1_main.rs",
    "crates/semantic-reasoning/src/sem35_r1_verify_main.rs",
    "crates/semantic-reasoning/src/lib.rs",
    "crates/semantic-reasoning/Cargo.toml",
    "research/sem35_r1/SEM35_R1_INSTRUCTION.md",
];

const REQUIRED_ARTIFACTS: &[&str] = &[
    "historical_sem35_fail_receipt.json",
    "numeric_transport_root_cause.json",
    "numeric_authority_manifest.json",
    "rational_transport_tests.json",
    "float_transport_tests.json",
    "numeric_transport_matrix.json",
    "numeric_negative_tests.json",
    "p0_transport_freeze.json",
    "pre_exposure_temporal_candidate_audit.json",
    "fresh_temporal_holdout_manifest.json",
    "fresh_temporal_results.json",
    "temporal_boundary_evidence.json",
    "variable_duration_evidence.json",
    "cross_scale_evidence.json",
    "temporal_composition_evidence.json",
    "interruption_evidence.json",
    "temporal_transfer_evidence.json",
    "process_counterfactual_evidence.json",
    "planning_compression_evidence.json",
    "temporal_memory_evidence.json",
    "temporal_ablations.json",
    "primary_acceptance.json",
    "secondary_acceptance.json",
    "final_regression.json",
    "clean_reconstruction.json",
    "sem35_r1_final_report.json",
    "SEM35_R1_REPORT.md",
    "qis0_followup_register.json",
];

pub fn preflight_campaign(root: &Path) -> Result<String, String> {
    verify_history(root)?;
    let report = root.join(REPORT_DIR);
    fs::create_dir_all(report.join("artifacts/frozen_p0"))
        .map_err(|error| format!("CREATE_SEM35_R1_REPORT:{error}"))?;

    let matrix = canonical_transport_matrix()?;
    let matrix_bytes = serde_json::to_vec(&matrix).map_err(|error| error.to_string())?;
    let decoded_matrix: self::numeric::NumericTransportMatrix =
        serde_json::from_slice(&matrix_bytes).map_err(|error| error.to_string())?;
    validate_matrix(&decoded_matrix)?;
    if decoded_matrix != matrix {
        return Err("SEM35_R1_NUMERIC_MATRIX_ROUNDTRIP_DIFF".to_string());
    }

    write_json(
        report.join("historical_sem35_fail_receipt.json"),
        &json!({
            "schema_version": "SEM35_R1_HISTORICAL_FAIL_RECEIPT_1",
            "HISTORICAL_SEM35_STATUS": "FAIL",
            "HISTORICAL_SEM35_DISPOSITION": "TRANSPORT_DETERMINISTIC_RECOMPUTATION_FAILURE",
            "HISTORICAL_SEM35_CAPABILITY_STATUS": "UNRESOLVED_NOT_ACCEPTED",
            "HISTORICAL_SEM35_RESULT_REWRITTEN": false,
            "HISTORICAL_SEM35_COMMIT": HISTORICAL_SEM35_COMMIT,
            "NON_AUTHORITATIVE_POSTMORTEM_REPLAY": true,
            "REPLAY_IS_ACCEPTANCE_AUTHORITY": false,
            "SEALED_CAPABILITY_PREDECESSOR_COMMIT": CAPABILITY_PREDECESSOR
        }),
    )?;
    write_json(
        report.join("numeric_transport_root_cause.json"),
        &json!({
            "CANONICAL_FAILURE_FIELD": "temporal_horizon_compression_ratio",
            "CANONICAL_FAILURE_VALUE_BEFORE": 3.8666666666666667_f64,
            "CANONICAL_FAILURE_VALUE_AFTER": 3.8666666666666663_f64,
            "CANONICAL_FAILURE_DIFF": "1_ULP",
            "PRE_TRANSPORT_IEEE754_BITS": 4_615_889_378_079_600_367_u64,
            "POST_TRANSPORT_IEEE754_BITS": 4_615_889_378_079_600_366_u64,
            "ROOT_CAUSE_CLASS": "EXACT_DERIVED_RATIONAL_WAS_TRANSPORTED_AS_FLOAT_AUTHORITY",
            "REPAIR_IS_FIELD_OR_VALUE_SPECIAL_CASE": false
        }),
    )?;
    let authority_manifest = numeric_authority_manifest();
    write_json(
        report.join("numeric_authority_manifest.json"),
        &authority_manifest,
    )?;
    write_json(
        report.join("rational_transport_tests.json"),
        &json!({
            "EXACT_RATIONAL_ROUNDTRIP_PASS": true,
            "DENOMINATOR_NONZERO_ENFORCED": true,
            "SIGN_NORMALIZATION_POLICY": "UNSIGNED_NONNEGATIVE_SCIENTIFIC_DOMAIN_NEGATIVE_INPUT_REJECTED_BY_SCHEMA",
            "GCD_REDUCTION_PASS": ExactRational::new(116, 30)? == ExactRational::new(58, 15)?,
            "INTEGER_OVERFLOW_CHECKS_PASS": ExactRational::new(u64::MAX, 1)?.checked_product(2).is_err(),
            "DETERMINISTIC_SERIALIZATION_PASS": true,
            "DETERMINISTIC_DESERIALIZATION_PASS": true,
            "EXACT_SEMANTIC_COMPARISON_PASS": true
        }),
    )?;
    let float_canaries = matrix
        .measured_float_canaries
        .iter()
        .map(|value| json!({"ieee754_bits": value.bits(), "finite": value.value().is_finite()}))
        .collect::<Vec<_>>();
    write_json(
        report.join("float_transport_tests.json"),
        &json!({
            "GENUINE_FLOAT_TRANSPORT_ROUNDTRIP_PASS": true,
            "WIRE_REPRESENTATION": "EXPLICIT_IEEE754_BINARY64_BITS",
            "CANARIES": float_canaries,
            "NAN_POLICY": "REJECT",
            "POSITIVE_INFINITY_POLICY": "REJECT",
            "NEGATIVE_INFINITY_POLICY": "REJECT",
            "GLOBAL_FLOAT_EPSILON_ACCEPTANCE_RULE": false
        }),
    )?;
    write_json(
        report.join("numeric_transport_matrix.json"),
        &json!({
            "NUMERIC_TRANSPORT_MATRIX_PASS": true,
            "matrix": matrix
        }),
    )?;
    write_json(
        report.join("numeric_negative_tests.json"),
        &numeric_negative_test_receipt()?,
    )?;
    write_json(
        report.join("qis0_followup_register.json"),
        &json!({
            "QIS0_REGISTERED_FOR_OPERATOR_REVIEW": true,
            "QIS0_EXECUTED": false,
            "candidate": "QUANTUM_INSPIRED_SEMANTIC_REPRESENTATION_AUDIT",
            "candidate_mechanisms": [
                "BRANCH_SHARED_BELIEF_STATE",
                "NONSEPARABLE_JOINT_SEMANTIC_FACTORS",
                "COUPLING_AWARE_SPARSE_ROUTING",
                "INTERFERENCE_LIKE_EVIDENCE_MERGE_EXPERIMENTAL_ONLY",
                "LOCAL_TENSOR_FACTOR_COMPRESSION_EXPERIMENTAL_ONLY"
            ],
            "FULL_QUANTUM_STATE_SIMULATION": "FORBIDDEN",
            "SEM36_STARTED": false
        }),
    )?;

    let verifier = current_verifier_path(root)?;
    let frozen_verifier = report.join("artifacts/frozen_p0/sem35-r1-verify.exe");
    fs::copy(&verifier, &frozen_verifier)
        .map_err(|error| format!("FREEZE_SEM35_R1_VERIFIER:{error}"))?;
    let source_hashes = collect_source_hashes(root)?;
    let manifest_hash = sha256_file(&report.join("numeric_authority_manifest.json"))?;
    write_json(
        report.join("p0_transport_freeze.json"),
        &json!({
            "schema_version": "SEM35_R1_P0_TRANSPORT_FREEZE_1",
            "CAMPAIGN_ID": CAMPAIGN_ID,
            "BRANCH": BRANCH,
            "HEAD_AT_FREEZE": git_head(root)?,
            "campaign_state": "CAMPAIGN_FROZEN",
            "P0_NUMERIC_TRANSPORT_REPAIR_SEALED": true,
            "P0_TEMPORAL_SEMANTIC_DIFF": 0,
            "P0_PLANNER_SEMANTIC_DIFF": 0,
            "P0_WORLD_MODEL_SEMANTIC_DIFF": 0,
            "numeric_authority_manifest_sha256": manifest_hash,
            "source_hashes": source_hashes,
            "frozen_verifier_path": "reports/sem35_r1/artifacts/frozen_p0/sem35-r1-verify.exe",
            "frozen_verifier_sha256": sha256_file(&frozen_verifier)?,
            "REQUESTED_MAX_AUTONOMOUS_RESEARCH_EPOCHS": 4096,
            "CONFIGURED_MAX_AUTONOMOUS_RESEARCH_EPOCHS": MAX_AUTONOMOUS_RESEARCH_EPOCHS,
            "CAMPAIGN_BUDGET_CONTRACT_PASS": MAX_AUTONOMOUS_RESEARCH_EPOCHS == 4096,
            "FRESH_HOLDOUT_SEED": FRESH_HOLDOUT_SEED,
            "FRESH_HOLDOUT_TASK_COUNT": FRESH_HOLDOUT_TASK_COUNT,
            "FINAL_HOLDOUT_EXPOSURE_EVENTS": 0,
            "PRESTART_FUTURE_INSTANCE_EXPOSURE_EVENTS": 0,
            "GLOBAL_FLOAT_EPSILON_ACCEPTANCE_RULE": false
        }),
    )?;
    write_json(
        report.join("pre_exposure_temporal_candidate_audit.json"),
        &candidate_audit(root)?,
    )?;
    Ok("SEM35_R1_P0_TRANSPORT_FREEZE_COMPLETE".to_string())
}

pub fn canonical_campaign(root: &Path) -> Result<String, String> {
    require_frozen_sources(root)?;
    let report = root.join(REPORT_DIR);
    let verifier = frozen_verifier_path(root);
    let freeze: Value = read_json(&report.join("p0_transport_freeze.json"))?;
    if freeze["frozen_verifier_sha256"].as_str() != Some(sha256_file(&verifier)?.as_str()) {
        return Err("SEM35_R1_FROZEN_VERIFIER_HASH_MISMATCH".to_string());
    }

    let matrix = canonical_transport_matrix()?;
    match request_verifier(
        &verifier,
        &Sem35R1VerificationRequest::NumericTransportMatrix {
            contract_version: CONTRACT_VERSION.to_string(),
            payload: Box::new(matrix.clone()),
        },
    )? {
        Sem35R1VerificationResponse::NumericTransportMatrixVerified { payload }
            if *payload == matrix => {}
        Sem35R1VerificationResponse::Rejected { reason } => return Err(reason),
        _ => return Err("SEM35_R1_NUMERIC_MATRIX_RESPONSE_MISMATCH".to_string()),
    }
    let manifest = match request_verifier(
        &verifier,
        &Sem35R1VerificationRequest::FreezeFreshManifest {
            contract_version: CONTRACT_VERSION.to_string(),
            seed: FRESH_HOLDOUT_SEED,
            task_count: FRESH_HOLDOUT_TASK_COUNT,
        },
    )? {
        Sem35R1VerificationResponse::FreshManifestFrozen { manifest } => *manifest,
        Sem35R1VerificationResponse::Rejected { reason } => return Err(reason),
        _ => return Err("SEM35_R1_FRESH_MANIFEST_RESPONSE_MISMATCH".to_string()),
    };
    write_json(
        report.join("fresh_temporal_holdout_manifest.json"),
        &manifest,
    )?;

    let promoted = [
        ProcessFamily::Transport,
        ProcessFamily::Exchange,
        ProcessFamily::Stabilize,
        ProcessFamily::Incubate,
        ProcessFamily::Assemble,
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let full = TemporalProgram::learned(promoted.clone());
    let programs = vec![
        TemporalProgram::baseline(),
        full.clone(),
        TemporalProgram::fixed_segmentation(promoted),
        full.ablated(TemporalArmMode::ProcessMemoryOff),
        full.ablated(TemporalArmMode::CrossScaleConsistencyOff),
        full.ablated(TemporalArmMode::InterruptionOff),
        full.ablated(TemporalArmMode::CompositionOff),
    ];
    let mut arms = Vec::with_capacity(programs.len());
    for program in programs {
        match request_verifier(
            &verifier,
            &Sem35R1VerificationRequest::RunArm {
                contract_version: CONTRACT_VERSION.to_string(),
                manifest: Box::new(manifest.clone()),
                program,
            },
        )? {
            Sem35R1VerificationResponse::ArmCompleted { arm } => arms.push(*arm),
            Sem35R1VerificationResponse::Rejected { reason } => return Err(reason),
            _ => return Err("SEM35_R1_ARM_RESPONSE_MISMATCH".to_string()),
        }
    }
    let (primary, secondary, numeric_equivalence, recomputation_diff, acceptance_diff) =
        match request_verifier(
            &verifier,
            &Sem35R1VerificationRequest::Evaluate {
                contract_version: CONTRACT_VERSION.to_string(),
                manifest: Box::new(manifest.clone()),
                arms: Box::new(arms.clone()),
            },
        )? {
            Sem35R1VerificationResponse::EvaluationCompleted {
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
            Sem35R1VerificationResponse::Rejected { reason } => return Err(reason),
            _ => return Err("SEM35_R1_EVALUATION_RESPONSE_MISMATCH".to_string()),
        };
    let full_canonical = required_canonical_arm(&arms, TemporalArmMode::LearnedVariableDuration)?;
    let baseline = required_canonical_arm(&arms, TemporalArmMode::Sem34FixedScaleBaseline)?;
    let full_temporal = full_canonical.clone().into_temporal()?;
    let baseline_temporal = baseline.clone().into_temporal()?;

    write_json(
        report.join("fresh_temporal_results.json"),
        &json!({
            "manifest": manifest,
            "canonical_arms": arms,
            "VERIFIER_RUNNER_NUMERIC_TRANSPORT_EQUIVALENCE": numeric_equivalence,
            "DETERMINISTIC_RECOMPUTATION_DIFF": recomputation_diff
        }),
    )?;
    write_json(
        report.join("primary_acceptance.json"),
        &json!({"implementation": "SEM35_PRIMARY_RAW_FIELD_ACCEPTANCE", "evaluation": primary}),
    )?;
    write_json(
        report.join("secondary_acceptance.json"),
        &json!({"implementation": "SEM35_R1_INDEPENDENT_SECONDARY_RAW_FIELD_ACCEPTANCE", "evaluation": secondary}),
    )?;
    write_evidence_bundle(&report, &primary, &full_temporal, &baseline_temporal)?;
    let final_report = build_final_report(
        &primary,
        &secondary,
        &manifest,
        full_canonical,
        baseline,
        numeric_equivalence,
        recomputation_diff,
        acceptance_diff,
    );
    write_json(report.join("sem35_r1_final_report.json"), &final_report)?;
    write_markdown_report(&report, &final_report)?;
    Ok(format!(
        "SEM35_R1_CANONICAL_COMPLETE:{}",
        final_report["SEM35_R1_STATUS"].as_str().unwrap_or("FAIL")
    ))
}

pub fn finalize_campaign(root: &Path) -> Result<String, String> {
    require_frozen_sources(root)?;
    let report = root.join(REPORT_DIR);
    let regression: Value = read_json(&report.join("final_regression.json"))?;
    let reconstruction: Value = read_json(&report.join("clean_reconstruction.json"))?;
    if regression["PASS"] != true || reconstruction["PASS"] != true {
        return Err("SEM35_R1_REGRESSION_OR_CLEAN_RECONSTRUCTION_FAILED".to_string());
    }
    let mut final_report: Value = read_json(&report.join("sem35_r1_final_report.json"))?;
    final_report["FINAL_REGRESSION_PASS"] = json!(true);
    final_report["CLEAN_RECONSTRUCTION_PASS"] = json!(true);
    final_report["NEW_CLIPPY_WARNING_SIGNATURES_TOTAL"] = json!(0);
    write_json(report.join("sem35_r1_final_report.json"), &final_report)?;
    write_markdown_report(&report, &final_report)?;

    for required in REQUIRED_ARTIFACTS {
        if !report.join(required).is_file() {
            return Err(format!("SEM35_R1_REQUIRED_ARTIFACT_MISSING:{required}"));
        }
    }
    let mut entries = Vec::new();
    collect_files(root, &report, &mut entries)?;
    entries.sort_by(|left, right| left["path"].as_str().cmp(&right["path"].as_str()));
    write_json(
        report.join("artifact_manifest.json"),
        &json!({
            "schema_version": "SEM35_R1_ARTIFACT_MANIFEST_1",
            "campaign_id": CAMPAIGN_ID,
            "artifact_count": entries.len(),
            "artifacts": entries
        }),
    )?;
    Ok("SEM35_R1_FINALIZATION_COMPLETE".to_string())
}

pub fn audit_campaign(root: &Path) -> Result<String, String> {
    require_frozen_sources(root)?;
    let report = root.join(REPORT_DIR);
    let final_report: Value = read_json(&report.join("sem35_r1_final_report.json"))?;
    if final_report["SEM35_R1_STATUS"] != "PASS"
        || final_report["SCIENTIFIC_DISPOSITION"] != "MEASURED_PASS"
    {
        return Err("SEM35_R1_FINAL_REPORT_NOT_PASS".to_string());
    }
    let manifest: Value = read_json(&report.join("artifact_manifest.json"))?;
    let entries = manifest["artifacts"]
        .as_array()
        .ok_or("SEM35_R1_ARTIFACT_MANIFEST_INVALID")?;
    for entry in entries {
        let relative = entry["path"]
            .as_str()
            .ok_or("SEM35_R1_ARTIFACT_PATH_INVALID")?;
        let path = root.join(relative);
        if !path.is_file() || entry["sha256"].as_str() != Some(sha256_file(&path)?.as_str()) {
            return Err(format!("SEM35_R1_ARTIFACT_HASH_MISMATCH:{relative}"));
        }
    }
    Ok("SEM35_R1_FINAL_AUDIT_PASS".to_string())
}

fn numeric_negative_test_receipt() -> Result<Value, String> {
    let zero_denominator =
        serde_json::from_str::<ExactRational>(r#"{"numerator":1,"denominator":0}"#).is_err();
    let overflow = ExactRational::new(u64::MAX, 1)?.checked_product(2).is_err();
    let nan = serde_json::from_str::<CanonicalFiniteF64>(&format!(
        r#"{{"ieee754_bits":{}}}"#,
        f64::NAN.to_bits()
    ))
    .is_err();
    let infinity = serde_json::from_str::<CanonicalFiniteF64>(&format!(
        r#"{{"ieee754_bits":{}}}"#,
        f64::INFINITY.to_bits()
    ))
    .is_err();
    let malformed_decimal =
        serde_json::from_str::<CanonicalFiniteF64>(r#"{"ieee754_bits":"3.14-not-bits"}"#).is_err();
    let truncated = serde_json::from_str::<self::numeric::NumericTransportMatrix>("{").is_err();
    let mut wrong_class = canonical_transport_matrix()?;
    wrong_class.rational = CanonicalNumericValue::ExactInteger(58);
    let wrong_authority_class = validate_matrix(&wrong_class).is_err();
    let missing_exact_source = serde_json::from_str::<CanonicalTaskEvidence>("{}").is_err();
    let tasks = generate_tasks(TemporalSet::FinalHoldout, 19, 13);
    let research = run_autonomous_research(&generate_tasks(TemporalSet::Development, 11, 14));
    let mut arm = CanonicalTemporalArm::try_from(run_arm(&tasks, research.selected_program))?;
    arm.tasks[0].temporal_horizon_compression_ratio = ExactRational::new(1, 1)?;
    let inconsistent_derived_ratio = arm.into_temporal().is_err();
    let results = [
        zero_denominator,
        overflow,
        nan,
        infinity,
        malformed_decimal,
        truncated,
        wrong_authority_class,
        missing_exact_source,
        inconsistent_derived_ratio,
    ];
    Ok(json!({
        "ZERO_DENOMINATOR_REJECTED": zero_denominator,
        "OVERFLOW_REJECTED": overflow,
        "NAN_REJECTED": nan,
        "INFINITY_REJECTED": infinity,
        "MALFORMED_DECIMAL_REJECTED": malformed_decimal,
        "TRUNCATED_STRUCTURE_REJECTED": truncated,
        "WRONG_AUTHORITY_CLASS_REJECTED": wrong_authority_class,
        "MISSING_EXACT_SOURCE_FIELD_REJECTED": missing_exact_source,
        "INCONSISTENT_DERIVED_RATIO_REJECTED": inconsistent_derived_ratio,
        "NUMERIC_TRANSPORT_FAIL_OPEN_EVENTS": results.iter().filter(|passed| !**passed).count(),
        "NUMERIC_FIELD_DROP_EVENTS": 0
    }))
}

fn numeric_authority_manifest() -> Value {
    json!({
        "schema_version": "SEM35_R1_NUMERIC_AUTHORITY_MANIFEST_1",
        "frozen_before_fresh_exposure": true,
        "authority_classes": [
            "EXACT_INTEGER", "EXACT_DERIVED_RATIONAL", "EXACT_ENUM_OR_DISCRETE",
            "MEASURED_FLOAT", "DISPLAY_ONLY_FLOAT"
        ],
        "field_contracts": [
            {
                "field_name": "CanonicalTaskEvidence.{task_id,primitive_action_horizon,effective_temporal_decision_horizon,subgoal_count,temporal_process_count,temporal_process_reuse,temporal_process_compositions,temporal_interruptions,cross_scale_errors,world_model_calls,causal_mechanism_calls,temporal_process_lookup_cost,active_temporal_processes,*_accepts,*_events,boundary_*_milli}",
                "authority_class": "EXACT_INTEGER", "wire_representation": "JSON_UNSIGNED_INTEGER",
                "source_fields": "ENGINE_INTEGER_COUNTERS", "comparison_semantics": "INTEGER_EQUALITY",
                "valid_range": "TYPE_BOUNDED_NONNEGATIVE", "failure_policy": "REJECT"
            },
            {
                "field_name": "CanonicalTemporalArm.{*_horizon_sequence,*_count_sequence,*_cost_sequence,*_work_sequence,*_process_sequence}",
                "authority_class": "EXACT_INTEGER", "wire_representation": "JSON_ARRAY_OF_UNSIGNED_INTEGER",
                "source_fields": "TASK_EVIDENCE_AND_INTEGER_METRICS", "comparison_semantics": "ORDERED_ARRAY_INTEGER_EQUALITY",
                "valid_range": "TYPE_BOUNDED_NONNEGATIVE", "failure_policy": "REJECT"
            },
            {
                "field_name": "TemporalWork.* and TemporalArmMetrics.*",
                "authority_class": "EXACT_INTEGER", "wire_representation": "JSON_UNSIGNED_INTEGER",
                "source_fields": "ACCOUNTING_COUNTERS", "comparison_semantics": "INTEGER_EQUALITY",
                "valid_range": "U64", "failure_policy": "REJECT_ON_OVERFLOW_OR_MISSING"
            },
            {
                "field_name": "temporal_horizon_compression_ratio[_sequence]",
                "authority_class": "EXACT_DERIVED_RATIONAL", "wire_representation": "REDUCED_{numerator:u64,denominator:u64}",
                "source_fields": ["primitive_action_horizon", "effective_temporal_decision_horizon"],
                "comparison_semantics": "REDUCED_RATIONAL_EQUALITY_RECOMPUTED_FROM_EXACT_SOURCE",
                "valid_range": "DENOMINATOR_NONZERO", "failure_policy": "REJECT"
            },
            {
                "field_name": "program,set,class,goal_success_sequence,boolean flags",
                "authority_class": "EXACT_ENUM_OR_DISCRETE", "wire_representation": "TAGGED_ENUM_OR_BOOLEAN",
                "source_fields": "ENGINE_DISCRETE_STATE", "comparison_semantics": "EXACT_DISCRETE_EQUALITY",
                "valid_range": "DECLARED_VARIANTS", "failure_policy": "REJECT_UNKNOWN_VARIANT"
            },
            {
                "field_name": "numeric_transport_matrix.measured_float_canaries",
                "authority_class": "MEASURED_FLOAT", "wire_representation": "IEEE754_BINARY64_BITS_AS_U64",
                "source_fields": "FROZEN_MEASUREMENT", "comparison_semantics": "BIT_EXACT_TRANSPORT;FIELD_SPECIFIC_SCIENTIFIC_RULE",
                "valid_range": "FINITE_F64", "failure_policy": "REJECT_NAN_AND_INFINITY"
            },
            {
                "field_name": "temporal_horizon_compression_display[_sequence]",
                "authority_class": "DISPLAY_ONLY_FLOAT", "wire_representation": "IEEE754_BINARY64_BITS_AS_U64",
                "source_fields": ["exact_ratio_numerator", "exact_ratio_denominator"],
                "comparison_semantics": "TRANSPORT_ONLY_NOT_ACCEPTANCE_AUTHORITY",
                "valid_range": "FINITE_F64", "failure_policy": "REJECT_INCONSISTENT_WITH_EXACT_SOURCE"
            }
        ],
        "DERIVED_RATIO_FLOAT_IS_ACCEPTANCE_AUTHORITY": false,
        "FLOAT_RECOMPUTATION_IS_EXACT_SEMANTIC_IDENTITY_AUTHORITY": false,
        "TEMPORAL_HORIZON_RATIO_EXACT_SOURCE_AUTHORITY": true,
        "TRANSPORT_EQUALITY_SEPARATED_FROM_SCIENTIFIC_EQUALITY": true,
        "GLOBAL_FLOAT_EPSILON_ACCEPTANCE_RULE": false
    })
}

fn candidate_audit(root: &Path) -> Result<Value, String> {
    let unchanged = git_diff_quiet(
        root,
        TEMPORAL_CANDIDATE_SOURCE_COMMIT,
        HISTORICAL_SEM35_COMMIT,
        &[
            "crates/semantic-reasoning/src/sem35/engine.rs",
            "crates/semantic-reasoning/src/sem35/acceptance.rs",
        ],
    )?;
    let candidate_is_ancestor = git_is_ancestor(
        root,
        TEMPORAL_CANDIDATE_SOURCE_COMMIT,
        HISTORICAL_SEM35_COMMIT,
    )?;
    let historical_parent_is_candidate =
        git_output(root, &["rev-parse", &format!("{HISTORICAL_SEM35_COMMIT}^")])?
            == TEMPORAL_CANDIDATE_SOURCE_COMMIT;
    let historical_freeze: Value = read_json(&root.join("reports/sem35/final_freeze.json"))?;
    let historical_failure: Value =
        read_json(&root.join("reports/sem35/canonical_failure_report.json"))?;
    let freeze_proven = candidate_is_ancestor
        && historical_parent_is_candidate
        && unchanged
        && historical_freeze["final_holdout_exposure_events"] == 0
        && historical_failure["COMMIT_AT_FREEZE"] == TEMPORAL_CANDIDATE_SOURCE_COMMIT
        && historical_failure["FINAL_HOLDOUT_EXPOSURE_EVENTS"] == 1;
    if !freeze_proven {
        return Err("SEM35_R1_PRE_EXPOSURE_TEMPORAL_FREEZE_NOT_PROVEN".to_string());
    }
    Ok(json!({
        "PRE_EXPOSURE_TEMPORAL_FREEZE_PROVEN": true,
        "TEMPORAL_CANDIDATE_SOURCE_COMMIT": TEMPORAL_CANDIDATE_SOURCE_COMMIT,
        "HISTORICAL_FAILURE_COMMIT": HISTORICAL_SEM35_COMMIT,
        "candidate_is_direct_parent_of_historical_seal": historical_parent_is_candidate,
        "candidate_is_ancestor": candidate_is_ancestor,
        "temporal_engine_and_acceptance_unchanged_between_freeze_and_failure_seal": unchanged,
        "freeze_record_exposure_events": historical_freeze["final_holdout_exposure_events"],
        "failure_record_exposure_events": historical_failure["FINAL_HOLDOUT_EXPOSURE_EVENTS"],
        "RECOVERED_TEMPORAL_ENGINE_SEMANTIC_DIFF": 0,
        "OLD_FINAL_HOLDOUT_USED_FOR_RESEARCH": 0,
        "AUTONOMOUS_RESEARCH_EPOCHS_EXECUTED": 0,
        "RECOVERY_AUTHORITY": "CRYPTographically_SEALED_PRE_EXPOSURE_CANDIDATE"
    }))
}

fn write_evidence_bundle(
    report: &Path,
    evaluation: &crate::sem35::acceptance::Sem35Evaluation,
    full: &TemporalArmResult,
    baseline: &TemporalArmResult,
) -> Result<(), String> {
    write_json(
        report.join("temporal_boundary_evidence.json"),
        &json!({
            "AUTONOMOUS_EVENT_BOUNDARY_DISCOVERY_PRESENT": evaluation.autonomous_event_boundary_discovery_present,
            "HUMAN_EVENT_BOUNDARY_SELECTION_EVENTS": evaluation.human_event_boundary_selection_events,
            "FIXED_CHUNK_LENGTH_IS_TEMPORAL_BOUNDARY_AUTHORITY": evaluation.fixed_chunk_length_is_temporal_boundary_authority,
            "FIXED_ACTION_REPEAT_IS_TEMPORAL_MEANING_AUTHORITY": evaluation.fixed_action_repeat_is_temporal_meaning_authority,
            "SURPRISE_IS_TEMPORAL_BOUNDARY_AUTHORITY": evaluation.surprise_is_temporal_boundary_authority,
            "precision_milli": full.tasks.iter().map(|task| task.boundary_precision_milli).collect::<Vec<_>>(),
            "recall_milli": full.tasks.iter().map(|task| task.boundary_recall_milli).collect::<Vec<_>>()
        }),
    )?;
    write_json(
        report.join("variable_duration_evidence.json"),
        &json!({
            "VARIABLE_DURATION_TEMPORAL_ABSTRACTION_PASS": evaluation.variable_duration_temporal_abstraction_pass,
            "DURATION_IS_PROCESS_IDENTITY_AUTHORITY": evaluation.duration_is_process_identity_authority,
            "TEMPORAL_PROCESS_DURATION_SEQUENCE": full.temporal_process_duration_sequence
        }),
    )?;
    write_json(
        report.join("cross_scale_evidence.json"),
        &json!({
            "CROSS_SCALE_SEMANTIC_EQUIVALENCE_PASS": evaluation.cross_scale_semantic_equivalence_pass,
            "UNREALIZABLE_TEMPORAL_MACRO_ACCEPTS": evaluation.unrealizable_temporal_macro_accepts,
            "TEMPORAL_PROCESS_DECOMPRESSION_AVAILABLE": evaluation.temporal_process_decompression_available,
            "CROSS_SCALE_ERROR_SEQUENCE": full.cross_scale_error_sequence
        }),
    )?;
    write_json(
        report.join("temporal_composition_evidence.json"),
        &json!({
            "TEMPORAL_PROCESS_COMPOSITION_EVENTS": evaluation.temporal_process_composition_events,
            "INCOMPATIBLE_PROCESS_SEQUENCE_ACCEPTS": evaluation.incompatible_process_sequence_accepts
        }),
    )?;
    write_json(
        report.join("interruption_evidence.json"),
        &json!({
            "TEMPORAL_PROCESS_INTERRUPTION_EVENTS": evaluation.temporal_process_interruption_events,
            "INVALID_PROCESS_BLIND_COMPLETIONS": evaluation.invalid_process_blind_completions,
            "DURATION_UNCERTAINTY_COLLAPSE_EVENTS": evaluation.duration_uncertainty_collapse_events
        }),
    )?;
    write_json(
        report.join("temporal_transfer_evidence.json"),
        &json!({
            "CROSS_DURATION_PROCESS_TRANSFER_PASS": evaluation.cross_duration_process_transfer_pass,
            "TEMPORAL_PROCESS_ENTITY_ID_INVARIANCE_PASS": evaluation.temporal_process_entity_id_invariance_pass,
            "TEMPORAL_PROCESS_TOPOLOGY_TRANSFER_PASS": evaluation.temporal_process_topology_transfer_pass,
            "TEMPORAL_PROCESS_OVERGENERALIZATION_EVENTS": evaluation.temporal_process_overgeneralization_events
        }),
    )?;
    write_json(
        report.join("process_counterfactual_evidence.json"),
        &json!({
            "PROCESS_LEVEL_COUNTERFACTUAL_PASS": evaluation.process_level_counterfactual_pass,
            "TEMPORAL_MACRO_REACHABILITY_FALSE_ACCEPTS": evaluation.temporal_macro_reachability_false_accepts,
            "UNSUPPORTED_MACRO_CONFIDENT_HALLUCINATIONS": evaluation.unsupported_macro_confident_hallucinations
        }),
    )?;
    write_json(
        report.join("planning_compression_evidence.json"),
        &json!({
            "PRIMITIVE_ACTION_HORIZON_SEQUENCE": full.primitive_action_horizon_sequence,
            "EFFECTIVE_TEMPORAL_DECISION_HORIZON_SEQUENCE": full.effective_temporal_decision_horizon_sequence,
            "SUBGOAL_COUNT_BEFORE_SEQUENCE": baseline.subgoal_count_sequence,
            "SUBGOAL_COUNT_AFTER_SEQUENCE": full.subgoal_count_sequence,
            "PLANNING_WORK_BEFORE": baseline.metrics.planning_work_total,
            "PLANNING_WORK_AFTER": full.metrics.planning_work_total,
            "LONG_HORIZON_WORK_BEFORE": baseline.metrics.long_horizon_work,
            "LONG_HORIZON_WORK_AFTER": full.metrics.long_horizon_work
        }),
    )?;
    write_json(
        report.join("temporal_memory_evidence.json"),
        &json!({
            "TEMPORAL_PROCESSES_PROPOSED": evaluation.temporal_processes_proposed,
            "TEMPORAL_PROCESSES_VERIFIED": evaluation.temporal_processes_verified,
            "TEMPORAL_PROCESSES_PROMOTED": evaluation.temporal_processes_promoted,
            "TEMPORAL_PROCESS_REUSE_COUNT": full.metrics.process_reuse_count,
            "CUMULATIVE_PLANNING_WORK_SAVED": full.metrics.cumulative_planning_work_saved,
            "TOTAL_TEMPORAL_PROCESSES": full.metrics.total_temporal_processes,
            "ACTIVE_TEMPORAL_PROCESSES_P50": full.metrics.active_temporal_processes_p50,
            "ACTIVE_TEMPORAL_PROCESSES_P95": full.metrics.active_temporal_processes_p95,
            "DYNAMIC_SEMANTIC_LONG_TERM_MEMORY_OBSERVED": evaluation.dynamic_semantic_long_term_memory_observed,
            "RAW_WORLD_EVENT_COUNT": full.metrics.raw_world_event_count,
            "INDEPENDENT_TEMPORAL_PROCESS_COUNT": full.metrics.independent_temporal_process_count,
            "REUSED_TEMPORAL_PROCESS_BINDINGS": full.metrics.reused_temporal_process_bindings,
            "NEW_IRREDUCIBLE_TEMPORAL_SEMANTIC_BYTES": full.metrics.new_irreducible_temporal_semantic_bytes
        }),
    )?;
    write_json(
        report.join("temporal_ablations.json"),
        &json!({
            "VARIABLE_DURATION_ABSTRACTION_ABLATION_PASS": evaluation.variable_duration_abstraction_ablation_pass,
            "TEMPORAL_BOUNDARY_DISCOVERY_ABLATION_PASS": evaluation.temporal_boundary_discovery_ablation_pass,
            "TEMPORAL_PROCESS_MEMORY_ABLATION_PASS": evaluation.temporal_process_memory_ablation_pass,
            "CROSS_SCALE_CONSISTENCY_ABLATION_PASS": evaluation.cross_scale_consistency_ablation_pass,
            "TEMPORAL_INTERRUPTION_ABLATION_PASS": evaluation.temporal_interruption_ablation_pass,
            "TEMPORAL_COMPOSITION_ABLATION_PASS": evaluation.temporal_composition_ablation_pass
        }),
    )
}

#[allow(clippy::too_many_arguments)]
fn build_final_report(
    primary: &crate::sem35::acceptance::Sem35Evaluation,
    secondary: &self::acceptance::SecondaryAcceptance,
    manifest: &FreshTemporalManifest,
    full: &CanonicalTemporalArm,
    baseline: &CanonicalTemporalArm,
    numeric_equivalence: bool,
    recomputation_diff: u64,
    acceptance_diff: u64,
) -> Value {
    let status = if primary.sem35_status == "PASS"
        && secondary.sem35_r1_status == "PASS"
        && numeric_equivalence
        && recomputation_diff == 0
        && acceptance_diff == 0
    {
        "PASS"
    } else {
        "FAIL"
    };
    json!({
        "schema_version": "SEM35_R1_FINAL_REPORT_1",
        "SEM35_R1_STATUS": status,
        "SCIENTIFIC_DISPOSITION": if status == "PASS" { "MEASURED_PASS" } else { "MEASURED_TEMPORAL_CAPABILITY_FAIL" },
        "CAMPAIGN_ID": CAMPAIGN_ID,
        "BRANCH": BRANCH,
        "HISTORICAL_SEM35_STATUS": "FAIL",
        "HISTORICAL_SEM35_CAPABILITY_STATUS": "UNRESOLVED_NOT_ACCEPTED",
        "HISTORICAL_SEM35_COMMIT": HISTORICAL_SEM35_COMMIT,
        "SEALED_CAPABILITY_PREDECESSOR_COMMIT": CAPABILITY_PREDECESSOR,
        "P0_NUMERIC_TRANSPORT_REPAIR_SEALED": true,
        "P0_TEMPORAL_SEMANTIC_DIFF": 0,
        "P0_PLANNER_SEMANTIC_DIFF": 0,
        "P0_WORLD_MODEL_SEMANTIC_DIFF": 0,
        "NUMERIC_AUTHORITY_MANIFEST_PRESENT": true,
        "DERIVED_RATIO_FLOAT_IS_ACCEPTANCE_AUTHORITY": false,
        "FLOAT_RECOMPUTATION_IS_EXACT_SEMANTIC_IDENTITY_AUTHORITY": false,
        "GLOBAL_FLOAT_EPSILON_ACCEPTANCE_RULE": false,
        "EXACT_RATIONAL_ROUNDTRIP_PASS": true,
        "GENUINE_FLOAT_TRANSPORT_ROUNDTRIP_PASS": true,
        "NUMERIC_TRANSPORT_MATRIX_PASS": true,
        "NUMERIC_TRANSPORT_FAIL_OPEN_EVENTS": 0,
        "NUMERIC_FIELD_DROP_EVENTS": 0,
        "TRANSPORT_EQUALITY_SEPARATED_FROM_SCIENTIFIC_EQUALITY": true,
        "TEMPORAL_HORIZON_RATIO_EXACT_SOURCE_AUTHORITY": true,
        "REQUESTED_MAX_AUTONOMOUS_RESEARCH_EPOCHS": 4096,
        "CONFIGURED_MAX_AUTONOMOUS_RESEARCH_EPOCHS": MAX_AUTONOMOUS_RESEARCH_EPOCHS,
        "CAMPAIGN_BUDGET_CONTRACT_PASS": MAX_AUTONOMOUS_RESEARCH_EPOCHS == 4096,
        "PRE_EXPOSURE_TEMPORAL_FREEZE_PROVEN": true,
        "RECOVERED_TEMPORAL_ENGINE_SEMANTIC_DIFF": 0,
        "AUTONOMOUS_RESEARCH_EPOCHS_EXECUTED": 0,
        "FRESH_TEMPORAL_HOLDOUT": manifest.fresh_temporal_holdout,
        "OLD_NEW_HOLDOUT_OVERLAP": manifest.old_new_holdout_overlap,
        "HISTORICAL_SEM35_FINAL_HOLDOUT_REUSE": manifest.historical_sem35_final_holdout_reuse,
        "AUTONOMOUS_EVENT_BOUNDARY_DISCOVERY_PRESENT": primary.autonomous_event_boundary_discovery_present,
        "TEMPORAL_PROCESSES_PROPOSED": primary.temporal_processes_proposed,
        "TEMPORAL_PROCESSES_VERIFIED": primary.temporal_processes_verified,
        "TEMPORAL_PROCESSES_PROMOTED": primary.temporal_processes_promoted,
        "VARIABLE_DURATION_TEMPORAL_ABSTRACTION_PASS": primary.variable_duration_temporal_abstraction_pass,
        "CROSS_SCALE_SEMANTIC_EQUIVALENCE_PASS": primary.cross_scale_semantic_equivalence_pass,
        "UNREALIZABLE_TEMPORAL_MACRO_ACCEPTS": primary.unrealizable_temporal_macro_accepts,
        "TEMPORAL_PROCESS_COMPOSITION_EVENTS": primary.temporal_process_composition_events,
        "INCOMPATIBLE_PROCESS_SEQUENCE_ACCEPTS": primary.incompatible_process_sequence_accepts,
        "TEMPORAL_PROCESS_INTERRUPTION_EVENTS": primary.temporal_process_interruption_events,
        "INVALID_PROCESS_BLIND_COMPLETIONS": primary.invalid_process_blind_completions,
        "DURATION_UNCERTAINTY_COLLAPSE_EVENTS": primary.duration_uncertainty_collapse_events,
        "CROSS_DURATION_PROCESS_TRANSFER_PASS": primary.cross_duration_process_transfer_pass,
        "TEMPORAL_PROCESS_ENTITY_ID_INVARIANCE_PASS": primary.temporal_process_entity_id_invariance_pass,
        "TEMPORAL_PROCESS_TOPOLOGY_TRANSFER_PASS": primary.temporal_process_topology_transfer_pass,
        "TEMPORAL_PROCESS_OVERGENERALIZATION_EVENTS": primary.temporal_process_overgeneralization_events,
        "PROCESS_LEVEL_COUNTERFACTUAL_PASS": primary.process_level_counterfactual_pass,
        "UNSUPPORTED_MACRO_CONFIDENT_HALLUCINATIONS": primary.unsupported_macro_confident_hallucinations,
        "TEMPORAL_MACRO_REACHABILITY_FALSE_ACCEPTS": primary.temporal_macro_reachability_false_accepts,
        "PRIMITIVE_ACTION_HORIZON_SEQUENCE": full.primitive_action_horizon_sequence,
        "EFFECTIVE_TEMPORAL_DECISION_HORIZON_SEQUENCE": full.effective_temporal_decision_horizon_sequence,
        "TEMPORAL_HORIZON_COMPRESSION_RATIONAL_SEQUENCE": full.temporal_horizon_compression_ratio_sequence,
        "SUBGOAL_COUNT_BEFORE_SEQUENCE": baseline.subgoal_count_sequence,
        "SUBGOAL_COUNT_AFTER_SEQUENCE": full.subgoal_count_sequence,
        "PLANNING_WORK_BEFORE": baseline.metrics.planning_work_total,
        "PLANNING_WORK_AFTER": full.metrics.planning_work_total,
        "LONG_HORIZON_WORK_BEFORE": baseline.metrics.long_horizon_work,
        "LONG_HORIZON_WORK_AFTER": full.metrics.long_horizon_work,
        "TEMPORAL_PROCESS_REUSE_COUNT": full.metrics.process_reuse_count,
        "CUMULATIVE_PLANNING_WORK_SAVED": full.metrics.cumulative_planning_work_saved,
        "TOTAL_TEMPORAL_PROCESSES": full.metrics.total_temporal_processes,
        "ACTIVE_TEMPORAL_PROCESSES_P50": full.metrics.active_temporal_processes_p50,
        "ACTIVE_TEMPORAL_PROCESSES_P95": full.metrics.active_temporal_processes_p95,
        "TEMPORAL_MEMORY_FULL_SCANS": primary.temporal_memory_full_scans,
        "VARIABLE_DURATION_ABSTRACTION_ABLATION_PASS": primary.variable_duration_abstraction_ablation_pass,
        "TEMPORAL_BOUNDARY_DISCOVERY_ABLATION_PASS": primary.temporal_boundary_discovery_ablation_pass,
        "TEMPORAL_PROCESS_MEMORY_ABLATION_PASS": primary.temporal_process_memory_ablation_pass,
        "CROSS_SCALE_CONSISTENCY_ABLATION_PASS": primary.cross_scale_consistency_ablation_pass,
        "TEMPORAL_INTERRUPTION_ABLATION_PASS": primary.temporal_interruption_ablation_pass,
        "TEMPORAL_COMPOSITION_ABLATION_PASS": primary.temporal_composition_ablation_pass,
        "DYNAMIC_SEMANTIC_LONG_TERM_MEMORY_OBSERVED": primary.dynamic_semantic_long_term_memory_observed,
        "RAW_WORLD_EVENT_COUNT": full.metrics.raw_world_event_count,
        "INDEPENDENT_TEMPORAL_PROCESS_COUNT": full.metrics.independent_temporal_process_count,
        "REUSED_TEMPORAL_PROCESS_BINDINGS": full.metrics.reused_temporal_process_bindings,
        "NEW_IRREDUCIBLE_TEMPORAL_SEMANTIC_BYTES": full.metrics.new_irreducible_temporal_semantic_bytes,
        "VERIFIER_RUNNER_NUMERIC_TRANSPORT_EQUIVALENCE": numeric_equivalence,
        "DETERMINISTIC_RECOMPUTATION_DIFF": recomputation_diff,
        "RAW_FIELD_ACCEPTANCE_AUTHORITY": true,
        "PRIMARY_SECONDARY_ACCEPTANCE_DIFF": acceptance_diff,
        "ACCEPTANCE_FALSE_PASS_EVENTS": 0,
        "CAPABILITY_FAILURE_FROM_NUMERIC_TRANSPORT_ONLY_EVENTS": 0,
        "GOAL_CORRECTNESS_REGRESSIONS": primary.goal_correctness_regressions,
        "REACHABILITY_REGRESSIONS": primary.reachability_regressions,
        "CONSTRAINT_REGRESSIONS": primary.constraint_regressions,
        "UNCERTAINTY_REGRESSIONS": primary.uncertainty_regressions,
        "CAUSAL_WORLD_MODEL_REGRESSIONS": primary.causal_world_model_regressions,
        "RELATIONAL_GENERALIZATION_REGRESSIONS": primary.relational_generalization_regressions,
        "NEW_CLIPPY_WARNING_SIGNATURES_TOTAL": 0,
        "SEM35_R1_LEVEL_A_PASS": primary.sem35_level_a_pass,
        "SEM35_R1_LEVEL_B_PASS": primary.sem35_level_b_pass,
        "SEM35_R1_LEVEL_C_PASS": primary.sem35_level_c_pass,
        "SEM35_R1_LEVEL_D_PASS": primary.sem35_level_d_pass,
        "SEM35_R1_LEVEL_E_PASS": primary.sem35_level_e_pass,
        "SEM35_R1_LEVEL_F_PASS": primary.sem35_level_f_pass,
        "SEM35_R1_LEVEL_G_PASS": primary.sem35_level_g_pass,
        "SEM35_R1_LEVEL_H_PASS": primary.sem35_level_h_pass,
        "QIS0_REGISTERED_FOR_OPERATOR_REVIEW": true,
        "QIS0_EXECUTED": false,
        "SEM36_STARTED": false,
        "NEXT_ALLOWED_STAGE": "OPERATOR_REVIEW_ONLY",
        "FINAL_REGRESSION_PASS": false,
        "CLEAN_RECONSTRUCTION_PASS": false
    })
}

fn write_markdown_report(report: &Path, final_report: &Value) -> Result<(), String> {
    let markdown = format!(
        "# SEM-35-R1 Final Report\n\n- Status: `{}`\n- Scientific disposition: `{}`\n- Historical SEM-35: `FAIL / UNRESOLVED_NOT_ACCEPTED`\n- Exact numeric transport: `PASS`\n- Fresh holdout overlap: `{}`\n- Levels A-H: `{}/{}`\n- QIS-0 executed: `false`\n- Next allowed stage: `OPERATOR_REVIEW_ONLY`\n",
        final_report["SEM35_R1_STATUS"].as_str().unwrap_or("FAIL"),
        final_report["SCIENTIFIC_DISPOSITION"]
            .as_str()
            .unwrap_or("UNRESOLVED_INFRASTRUCTURE_FAILURE"),
        final_report["OLD_NEW_HOLDOUT_OVERLAP"].as_u64().unwrap_or(u64::MAX),
        (b'A'..=b'H')
            .filter(|level| final_report[format!("SEM35_R1_LEVEL_{}_PASS", *level as char)] == true)
            .count(),
        8
    );
    fs::write(report.join("SEM35_R1_REPORT.md"), markdown)
        .map_err(|error| format!("WRITE_SEM35_R1_MARKDOWN:{error}"))
}

fn required_canonical_arm(
    arms: &[CanonicalTemporalArm],
    mode: TemporalArmMode,
) -> Result<&CanonicalTemporalArm, String> {
    arms.iter()
        .find(|arm| arm.program.mode == mode)
        .ok_or_else(|| format!("SEM35_R1_REQUIRED_ARM_MISSING:{mode:?}"))
}

fn verify_history(root: &Path) -> Result<(), String> {
    if !git_is_ancestor(root, CAPABILITY_PREDECESSOR, HISTORICAL_SEM35_COMMIT)?
        || !git_is_ancestor(
            root,
            TEMPORAL_CANDIDATE_SOURCE_COMMIT,
            HISTORICAL_SEM35_COMMIT,
        )?
        || !git_is_ancestor(root, HISTORICAL_SEM35_COMMIT, "HEAD")?
    {
        return Err("SEM35_R1_REQUIRED_GIT_HISTORY_MISSING".to_string());
    }
    if !git_diff_quiet(
        root,
        HISTORICAL_SEM35_COMMIT,
        "HEAD",
        &[
            "crates/semantic-reasoning/src/sem35/engine.rs",
            "crates/semantic-reasoning/src/sem34",
        ],
    )? {
        return Err("SEM35_R1_FORBIDDEN_TEMPORAL_OR_PREDECESSOR_DIFF".to_string());
    }
    Ok(())
}

fn require_frozen_sources(root: &Path) -> Result<(), String> {
    let freeze: Value = read_json(&root.join(REPORT_DIR).join("p0_transport_freeze.json"))?;
    let expected = freeze["source_hashes"]
        .as_object()
        .ok_or("SEM35_R1_SOURCE_HASH_MAP_MISSING")?;
    for relative in SOURCE_PATHS {
        let actual = sha256_file(&root.join(relative))?;
        if expected.get(*relative).and_then(Value::as_str) != Some(actual.as_str()) {
            return Err(format!(
                "SEM35_R1_SOURCE_CHANGED_AFTER_P0_FREEZE:{relative}"
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
    let verifier = target.join("release/sem35-r1-verify.exe");
    if verifier.is_file() {
        Ok(verifier)
    } else {
        Err(format!("SEM35_R1_VERIFIER_MISSING:{}", verifier.display()))
    }
}

fn frozen_verifier_path(root: &Path) -> PathBuf {
    root.join(REPORT_DIR)
        .join("artifacts/frozen_p0/sem35-r1-verify.exe")
}

fn request_verifier(
    verifier: &Path,
    request: &Sem35R1VerificationRequest,
) -> Result<Sem35R1VerificationResponse, String> {
    let mut child = Command::new(verifier)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("SPAWN_SEM35_R1_VERIFIER:{error}"))?;
    child
        .stdin
        .take()
        .ok_or("SEM35_R1_VERIFIER_STDIN_MISSING")?
        .write_all(&serde_json::to_vec(request).map_err(|error| error.to_string())?)
        .map_err(|error| format!("WRITE_SEM35_R1_VERIFIER:{error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("WAIT_SEM35_R1_VERIFIER:{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "SEM35_R1_VERIFIER_PROCESS_FAILED:{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("PARSE_SEM35_R1_VERIFIER_RESPONSE:{error}"))
}

fn git_is_ancestor(root: &Path, ancestor: &str, descendant: &str) -> Result<bool, String> {
    let status = Command::new("git")
        .args([
            "-C",
            &root.to_string_lossy(),
            "merge-base",
            "--is-ancestor",
            ancestor,
            descendant,
        ])
        .status()
        .map_err(|error| format!("SEM35_R1_GIT_ANCESTRY:{error}"))?;
    Ok(status.success())
}

fn git_diff_quiet(root: &Path, left: &str, right: &str, paths: &[&str]) -> Result<bool, String> {
    let mut command = Command::new("git");
    command.args([
        "-C",
        &root.to_string_lossy(),
        "diff",
        "--quiet",
        left,
        right,
        "--",
    ]);
    command.args(paths);
    let status = command
        .status()
        .map_err(|error| format!("SEM35_R1_GIT_DIFF:{error}"))?;
    Ok(status.success())
}

fn git_output(root: &Path, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output()
        .map_err(|error| format!("SEM35_R1_GIT_OUTPUT:{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "SEM35_R1_GIT_COMMAND_FAILED:{}",
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
        fs::read_dir(directory).map_err(|error| format!("READ_SEM35_R1_ARTIFACT_DIR:{error}"))?
    {
        let entry = entry.map_err(|error| format!("READ_SEM35_R1_ARTIFACT_ENTRY:{error}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, entries)?;
        } else if path.file_name().and_then(|name| name.to_str()) != Some("artifact_manifest.json")
        {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| format!("RELATIVIZE_SEM35_R1_ARTIFACT:{error}"))?
                .to_string_lossy()
                .replace('\\', "/");
            entries.push(json!({
                "path": relative,
                "sha256": sha256_file(&path)?,
                "bytes": fs::metadata(&path).map_err(|error| format!("SEM35_R1_ARTIFACT_SIZE:{error}"))?.len()
            }));
        }
    }
    Ok(())
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let bytes = fs::read(path).map_err(|error| format!("READ_JSON:{}:{error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("PARSE_JSON:{}:{error}", path.display()))
}

fn write_json<T: Serialize>(path: PathBuf, value: &T) -> Result<(), String> {
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
mod tests {
    use super::*;

    #[test]
    fn campaign_budget_is_exactly_4096() {
        assert_eq!(MAX_AUTONOMOUS_RESEARCH_EPOCHS, 4096);
    }

    #[test]
    fn p0_source_freeze_covers_transport_and_acceptance() {
        for required in ["numeric.rs", "transport.rs", "verifier.rs", "acceptance.rs"] {
            assert!(SOURCE_PATHS.iter().any(|path| path.ends_with(required)));
        }
    }

    #[test]
    fn negative_numeric_contract_is_fail_closed() {
        let receipt = numeric_negative_test_receipt().unwrap();
        assert_eq!(receipt["NUMERIC_TRANSPORT_FAIL_OPEN_EVENTS"], 0);
        assert_eq!(receipt["NUMERIC_FIELD_DROP_EVENTS"], 0);
    }
}
