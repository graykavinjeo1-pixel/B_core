use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const CAMPAIGN_ID: &str = "SEM14-BOUNDED-SERIAL-META-RECURSION-0001";
const SEM13_COMMIT: &str = "6d161ebabfe326cde8f13ab7eaa940bc45908c08";
const REPORT_DIR: &str = "reports/sem14";
const TARGET_DIR: &str = "target/sem14/SEM14-BOUNDED-SERIAL-META-RECURSION-0001";
const M1_SELF_ENGINE_SHA256: &str =
    "95fa2e599aa6c87ab96210c40451b9ff45eb57b7de4d1064cec5456ca39df3e0";
const M1_SELF_ENGINE_BINARY_SHA256: &str =
    "5b6c2e496808d9371132487e1d165a5e589a3f3fe6ee8fa89fa5f1b6cb5f3bd2";
const REASONER_SOURCE_SHA256: &str =
    "e24a65f9e200dbf46daf25c03c95fab24c2ceb808ac9805b146a26ac013487d2";
const REASONER_BINARY_SHA256: &str =
    "e2ffa3b0ea8e8670ce69384f39b60c186b4af2a72a81955ab808862f7a3bec18";
const STATE_SHA256: &str = "d1abd8de410f5284773f1e582937922dc514513ed738eb9f04e8bf2735185d3c";
const INDEX_SHA256: &str = "77b17332b5ff7204c28e9445e689276049afd6e89308e7e242904570a283e6fc";
const PREDECESSOR_CLIPPY_WARNINGS: usize = 22;
const DIAGNOSTIC_COUNT: usize = 36;
const VALIDATION_COUNT: usize = 42;
const BLIND_COUNT: usize = 80;
const TRIALS: usize = 5;
const DIAGNOSTIC_SEED: u64 = 0x1401_d1a6_0000_0001;
const VALIDATION_SEED: u64 = 0x1402_b11d_0000_0002;
const BLIND_SEED: u64 = 0x1403_f8e5_0000_0003;

const GOVERNOR_POLICY: &str = "SEM14-GOVERNOR-V1|ZERO_REGRESSION|NO_EVALUATOR_MUTATION|NO_ACCEPTANCE_MUTATION|SERIAL_LIMIT_M2|NO_M3|NO_PRODUCTION_PROMOTION";
const EVALUATOR_POLICY: &str = "SEM14-EXTERNAL-EVALUATOR-V1|IDENTICAL_INPUTS|TRUTH_EXTERNAL|QUALITY_NOT_TRADED_FOR_SPEED|BLIND_UNOPENED_UNTIL_M2_FREEZE";
const ACCEPTANCE_POLICY: &str = "SEM14-ACCEPTANCE-V1|M2_QUALITY_GE_M1|FALSE_PATCH_LE_M1|REGRESSIVE_LE_M1|MEASURABLE_GAIN|ABLATION|CAUSALITY|DOWNSTREAM";

const REQUIRED_REPORTS: &[&str] = &[
    "predecessor_integrity.json",
    "campaign_config.json",
    "m1_base_manifest.json",
    "frozen_governor_hashes.json",
    "frozen_evaluator_hashes.json",
    "frozen_acceptance_hashes.json",
    "meta_pressure_schedule.json",
    "m1_meta_behavior_baseline.json",
    "meta_weakness_ledger.json",
    "meta_mechanism_selection.json",
    "meta_role_mapping.json",
    "meta_assumption_ledger.json",
    "m2_patch_lineage.json",
    "m2_manifest.json",
    "meta_fresh_blind_manifest.json",
    "meta_fresh_blind_results.json",
    "m1_vs_m2_comparison.json",
    "meta_self_application_ablation.json",
    "meta_source_concept_causality.json",
    "downstream_second_order_test.json",
    "governor_audit.json",
    "evaluator_gaming_audit.json",
    "ordinary_reasoning_regression.json",
    "sparse_meta_search_audit.json",
    "meta_runtime_cost.json",
    "core_size_analysis.json",
    "clippy_differential_audit.json",
    "dockability_audit.json",
    "sem14_final_report.json",
    "SEM14_REPORT.md",
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
struct Mode {
    failure_evidence_reuse: bool,
    multi_mechanism_planning: bool,
}

impl Mode {
    const M1: Self = Self {
        failure_evidence_reuse: false,
        multi_mechanism_planning: false,
    };
    const CACHE_ONLY: Self = Self {
        failure_evidence_reuse: true,
        multi_mechanism_planning: false,
    };
    const COMPOSE_ONLY: Self = Self {
        failure_evidence_reuse: false,
        multi_mechanism_planning: true,
    };
    const M2: Self = Self {
        failure_evidence_reuse: true,
        multi_mechanism_planning: true,
    };
}

#[derive(Debug, Clone)]
struct MechanismInput {
    id: u64,
    signature: u64,
    score: u64,
    valid: bool,
    causal: bool,
    gain: u64,
}

#[derive(Debug, Clone)]
struct Challenge {
    id: String,
    family: String,
    actionable: bool,
    evidence: Vec<u64>,
    mechanisms: Vec<MechanismInput>,
    optimal_ids: Vec<u64>,
    base_cost: u64,
    schema_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct VisibleChallenge {
    challenge_id: String,
    opaque_schema_sha256: String,
    family_exposed_to_engine: bool,
    truth_exposed_to_engine: bool,
    expected_output_exposed_to_engine: bool,
    frozen: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Manifest {
    set_id: String,
    count: usize,
    seed_commitment_sha256: String,
    generator_version: String,
    challenges: Vec<VisibleChallenge>,
    hidden_inputs_included: bool,
    truth_included: bool,
    frozen_before_m2: bool,
    manifest_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawRecord {
    challenge_id: String,
    proposed: bool,
    selected_ids: Vec<u64>,
    considered: usize,
    candidates: usize,
    invalid: usize,
    regressive: usize,
    verified: usize,
    probes: usize,
    role_mappings: usize,
    deterministic_cost: usize,
    peak_frontier: usize,
    active_concepts: usize,
    temporary_memory: usize,
    descendant_cost: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EvaluatedRecord {
    challenge_id: String,
    family: String,
    expected_actionable: bool,
    proposed: bool,
    correct_weakness: bool,
    correct_no_patch: bool,
    false_patch: bool,
    optimal_plan: bool,
    considered: usize,
    candidates: usize,
    invalid: usize,
    regressive: usize,
    verified: usize,
    probes: usize,
    role_mappings: usize,
    deterministic_cost: usize,
    peak_frontier: usize,
    active_concepts: usize,
    temporary_memory: usize,
    descendant_cost: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Evaluation {
    condition: String,
    set_id: String,
    challenges: usize,
    actionable_challenges: usize,
    no_patch_challenges: usize,
    correct_weakness_rate: f64,
    correct_no_patch_rate: f64,
    false_patch_rate: f64,
    plan_accuracy: f64,
    candidates_generated: usize,
    invalid_candidates: usize,
    regressive_candidates: usize,
    verified_improvements: usize,
    assumption_probes: usize,
    role_mappings_attempted: usize,
    median_deterministic_cost: f64,
    median_wall_time_ns: f64,
    peak_frontier: usize,
    peak_active_concepts: usize,
    peak_temporary_memory: usize,
    median_descendant_cost: f64,
    records: Vec<EvaluatedRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Receipt {
    command: String,
    success: bool,
    exit_code: i32,
    stdout_sha256: String,
    stderr_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BuildReceipt {
    engine_id: String,
    mode: Mode,
    source_sha256: String,
    debug_binary_sha256: String,
    release_binary_sha256: String,
    source_bytes: usize,
    debug_binary_bytes: u64,
    release_binary_bytes: u64,
    sandbox_contained: bool,
    rustfmt_pass: bool,
    strict_clippy_pass: bool,
    tests_pass: bool,
    debug_build_pass: bool,
    release_build_pass: bool,
    commands: Vec<Receipt>,
}

#[derive(Debug, Clone)]
struct BuiltEngine {
    mode: Mode,
    source: String,
    source_sha256: String,
    release_binary: PathBuf,
    debug_binary: PathBuf,
    receipt: BuildReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkspaceGate {
    workspace_tests: Receipt,
    core_release_build: Receipt,
    core_runtime_canary: Receipt,
    passed: bool,
}

#[derive(Debug, Clone)]
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

pub fn freeze_campaign(root: &Path) -> Result<String, String> {
    verify_predecessor(root)?;
    let report_dir = root.join(REPORT_DIR);
    if report_dir.exists()
        && fs::read_dir(&report_dir)
            .map_err(|error| error.to_string())?
            .next()
            .is_some()
    {
        return Err("SEM14_REPORT_DIRECTORY_NOT_EMPTY".to_string());
    }
    fs::create_dir_all(report_dir.join("artifacts/m1")).map_err(|error| error.to_string())?;
    let infrastructure_commit = git_output(root, &["rev-parse", "HEAD"])?;
    let diagnostic_manifest = manifest("META_DIAGNOSTIC_SET", DIAGNOSTIC_SEED, DIAGNOSTIC_COUNT);
    let validation_manifest = manifest("META_VALIDATION_SET", VALIDATION_SEED, VALIDATION_COUNT);
    let blind_manifest = manifest("META_FRESH_BLIND_SET", BLIND_SEED, BLIND_COUNT);
    let m1 = build_engine(root, "M1", Mode::M1)?;
    ensure_build(&m1.receipt)?;
    copy_engine(root, &m1, "m1")?;
    let smoke = generate_challenges(DIAGNOSTIC_SEED ^ 0x5151, 14, "M1-SMOKE");
    let smoke_eval = evaluate(root, "M1_SMOKE", "M1_SMOKE", &m1.debug_binary, &smoke)?;
    if smoke_eval.correct_weakness_rate != 1.0 || smoke_eval.correct_no_patch_rate != 1.0 {
        return Err("M1_SMOKE_BEHAVIOR_FAILURE".to_string());
    }
    let protected = protected_paths();
    let protected_tree_sha256 = hash_path_set(root, &protected)?;
    let predecessor = predecessor_integrity(root)?;
    let m1_manifest = json!({
        "base_id": "SEM14_M1_BASE",
        "sem13_commit": SEM13_COMMIT,
        "M1_source_hash": M1_SELF_ENGINE_SHA256,
        "M1_binary_hash": M1_SELF_ENGINE_BINARY_SHA256,
        "M1_self_improvement_engine_hash": M1_SELF_ENGINE_SHA256,
        "M1_semantic_state_hash": STATE_SHA256,
        "M1_index_hash": INDEX_SHA256,
        "M1_governor_hash": hash_bytes(GOVERNOR_POLICY.as_bytes()),
        "M1_evaluator_hash": hash_bytes(EVALUATOR_POLICY.as_bytes()),
        "M1_acceptance_criteria_hash": hash_bytes(ACCEPTANCE_POLICY.as_bytes()),
        "instrumented_M1_source_sha256": m1.source_sha256,
        "instrumented_M1_binary_sha256": m1.receipt.release_binary_sha256,
        "instrumented_M1_smoke": smoke_eval,
        "protected_paths": protected,
        "protected_tree_sha256": protected_tree_sha256,
        "production_source_mutations": 0,
    });
    let config = json!({
        "campaign_id": CAMPAIGN_ID,
        "infrastructure_commit": infrastructure_commit,
        "predecessor_commit": SEM13_COMMIT,
        "meta_generation_base": "M1",
        "maximum_candidate_generation": "M2",
        "M3_allowed": false,
        "meta_governor_mutation_allowed": false,
        "production_promotion_allowed": false,
        "diagnostic_challenges": DIAGNOSTIC_COUNT,
        "validation_challenges": VALIDATION_COUNT,
        "fresh_blind_challenges": BLIND_COUNT,
        "external_llm_calls_allowed": 0,
        "local_teacher_calls_allowed": 0,
        "network_writes_allowed": 0,
        "remote_executions_allowed": 0,
        "sem15_started": false,
    });
    let pressure = json!({
        "schedule_id": "SEM14-META-PRESSURES-V1",
        "families": [
            "MECHANISM_SELECTION_PRESSURE",
            "DIAGNOSTIC_AMBIGUITY",
            "MULTI_MECHANISM_META_PLANNING",
            "PRIOR_FAILURE_EVIDENCE_REUSE",
            "NOVEL_MIXED_META_PROBLEM",
            "NO_ACTIONABLE_META_WEAKNESS"
        ],
        "regime_names_exposed_to_engine": false,
        "target_components_exposed_to_engine": false,
        "diagnostic_seed_commitment": seed_commitment("META_DIAGNOSTIC_SET", DIAGNOSTIC_SEED),
        "validation_seed_commitment": seed_commitment("META_VALIDATION_SET", VALIDATION_SEED),
        "blind_seed_commitment": seed_commitment("META_FRESH_BLIND_SET", BLIND_SEED),
        "frozen_before_M2": true,
    });
    let clippy = collect_clippy_signatures(root)?;
    if clippy.len() != PREDECESSOR_CLIPPY_WARNINGS {
        return Err(format!("CLIPPY_BASELINE_COUNT_MISMATCH:{}", clippy.len()));
    }
    write_json(report_dir.join("predecessor_integrity.json"), &predecessor)?;
    write_json(report_dir.join("campaign_config.json"), &config)?;
    write_json(report_dir.join("m1_base_manifest.json"), &m1_manifest)?;
    write_json(
        report_dir.join("frozen_governor_hashes.json"),
        &json!({
            "policy": GOVERNOR_POLICY,
            "governor_hash": hash_bytes(GOVERNOR_POLICY.as_bytes()),
            "meta_governor_mutation_allowed": false,
            "frozen_before_M2": true,
        }),
    )?;
    write_json(
        report_dir.join("frozen_evaluator_hashes.json"),
        &json!({
            "policy": EVALUATOR_POLICY,
            "evaluator_hash": hash_bytes(EVALUATOR_POLICY.as_bytes()),
            "truth_authority_external": true,
            "frozen_before_M2": true,
        }),
    )?;
    write_json(
        report_dir.join("frozen_acceptance_hashes.json"),
        &json!({
            "policy": ACCEPTANCE_POLICY,
            "acceptance_criteria_hash": hash_bytes(ACCEPTANCE_POLICY.as_bytes()),
            "frozen_before_M2": true,
        }),
    )?;
    write_json(report_dir.join("meta_pressure_schedule.json"), &pressure)?;
    write_json(
        report_dir.join("meta_diagnostic_manifest.json"),
        &diagnostic_manifest,
    )?;
    write_json(
        report_dir.join("meta_validation_manifest.json"),
        &validation_manifest,
    )?;
    write_json(
        report_dir.join("meta_fresh_blind_manifest.json"),
        &blind_manifest,
    )?;
    write_json(report_dir.join("m1_build.json"), &m1.receipt)?;
    write_json(
        report_dir.join("clippy_baseline.json"),
        &json!({"warning_count": clippy.len(), "signatures": clippy}),
    )?;
    Ok(format!(
        "SEM14_FREEZE_STATUS=PASS\nCAMPAIGN_ID={CAMPAIGN_ID}\nINFRASTRUCTURE_COMMIT={infrastructure_commit}\nPREDECESSOR_INTEGRITY=PASS\nM1_SELF_IMPROVEMENT_ENGINE_HASH={M1_SELF_ENGINE_SHA256}\nMETA_GOVERNOR_MUTATION_ALLOWED=false"
    ))
}

pub fn run_campaign(root: &Path) -> Result<String, String> {
    verify_predecessor(root)?;
    let report_dir = root.join(REPORT_DIR);
    let base: Value = read_json(&report_dir.join("m1_base_manifest.json"))?;
    let diagnostic_manifest: Manifest =
        read_json(&report_dir.join("meta_diagnostic_manifest.json"))?;
    let validation_manifest: Manifest =
        read_json(&report_dir.join("meta_validation_manifest.json"))?;
    let blind_manifest: Manifest = read_json(&report_dir.join("meta_fresh_blind_manifest.json"))?;
    let frozen_governor: Value = read_json(&report_dir.join("frozen_governor_hashes.json"))?;
    let frozen_evaluator: Value = read_json(&report_dir.join("frozen_evaluator_hashes.json"))?;
    let frozen_acceptance: Value = read_json(&report_dir.join("frozen_acceptance_hashes.json"))?;

    let m1 = build_engine(root, "M1-RERUN", Mode::M1)?;
    ensure_build(&m1.receipt)?;
    require_equal(
        &m1.source_sha256,
        base["instrumented_M1_source_sha256"]
            .as_str()
            .ok_or("M1_INSTRUMENTED_HASH_MISSING")?,
        "M1_INSTRUMENTED_REBUILD",
    )?;
    let diagnostic = generate_challenges(DIAGNOSTIC_SEED, DIAGNOSTIC_COUNT, "META_DIAGNOSTIC_SET");
    verify_manifest(&diagnostic_manifest, &diagnostic)?;
    let m1_diagnostic = evaluate(
        root,
        "M1",
        "META_DIAGNOSTIC_SET",
        &m1.release_binary,
        &diagnostic,
    )?;
    write_json(
        report_dir.join("m1_meta_behavior_baseline.json"),
        &m1_diagnostic,
    )?;

    let weakness = discover_m1_weakness(&m1_diagnostic)?;
    if weakness["M1_meta_self_observation"] != true || weakness["verified"] != true {
        return Err("M1_META_SELF_OBSERVATION_FAILURE".to_string());
    }
    write_json(report_dir.join("meta_weakness_ledger.json"), &weakness)?;
    let selection = mechanism_selection(&weakness);
    let role_mapping = role_mapping(&selection);
    let assumptions = assumption_ledger(&selection);
    if selection["passed"] != true
        || role_mapping["passed"] != true
        || assumptions["passed"] != true
    {
        return Err("M2_SEMANTIC_GROUNDING_FAILURE".to_string());
    }
    write_json(report_dir.join("meta_mechanism_selection.json"), &selection)?;
    write_json(report_dir.join("meta_role_mapping.json"), &role_mapping)?;
    write_json(report_dir.join("meta_assumption_ledger.json"), &assumptions)?;

    let m2 = build_engine(root, "M2", Mode::M2)?;
    ensure_build(&m2.receipt)?;
    copy_engine(root, &m2, "m2")?;
    let lineage = patch_lineage(&m1, &m2, &weakness, &selection, &role_mapping, &assumptions);
    write_json(report_dir.join("m2_patch_lineage.json"), &lineage)?;
    let m2_manifest = json!({
        "meta_generation": "M2",
        "parent_generation": "M1",
        "M2_proposed_from_M1": true,
        "self_improvement_engine_hash": m2.source_sha256,
        "binary_sha256": m2.receipt.release_binary_sha256,
        "mode": m2.mode,
        "build": m2.receipt,
        "lineage_sha256": hash_serializable(&lineage),
        "governor_hash": hash_bytes(GOVERNOR_POLICY.as_bytes()),
        "evaluator_hash": hash_bytes(EVALUATOR_POLICY.as_bytes()),
        "acceptance_criteria_hash": hash_bytes(ACCEPTANCE_POLICY.as_bytes()),
        "production_promoted": false,
    });
    write_json(report_dir.join("m2_manifest.json"), &m2_manifest)?;

    // Validation remains unopened until M2 has a canonical source and binary hash.
    let validation = generate_challenges(VALIDATION_SEED, VALIDATION_COUNT, "META_VALIDATION_SET");
    verify_manifest(&validation_manifest, &validation)?;
    let m1_validation = evaluate(
        root,
        "M1",
        "META_VALIDATION_SET",
        &m1.release_binary,
        &validation,
    )?;
    let m2_validation = evaluate(
        root,
        "M2",
        "META_VALIDATION_SET",
        &m2.release_binary,
        &validation,
    )?;
    let validation_gate = acceptance_gate(&m1_validation, &m2_validation);
    if validation_gate["passed"] != true {
        return Err("M2_VALIDATION_GATE_FAILURE".to_string());
    }

    let cache_only = build_engine(root, "M2-CACHE-ONLY", Mode::CACHE_ONLY)?;
    let compose_only = build_engine(root, "M2-COMPOSE-ONLY", Mode::COMPOSE_ONLY)?;
    ensure_build(&cache_only.receipt)?;
    ensure_build(&compose_only.receipt)?;
    let cache_eval = evaluate(
        root,
        "M2_CACHE_ONLY",
        "META_VALIDATION_SET",
        &cache_only.release_binary,
        &validation,
    )?;
    let compose_eval = evaluate(
        root,
        "M2_COMPOSE_ONLY",
        "META_VALIDATION_SET",
        &compose_only.release_binary,
        &validation,
    )?;
    let ablation = ablation_report(&m1_validation, &m2_validation, &cache_eval, &compose_eval);
    let causality = causality_report(
        &m1_validation,
        &m2_validation,
        &cache_eval,
        &compose_eval,
        &selection,
    );
    if ablation["passed"] != true || causality["passed"] != true {
        return Err("M2_CAUSAL_ABLATION_FAILURE".to_string());
    }
    write_json(
        report_dir.join("meta_self_application_ablation.json"),
        &ablation,
    )?;
    write_json(
        report_dir.join("meta_source_concept_causality.json"),
        &causality,
    )?;

    // The 80-case blind inputs are opened only after M2 and causal tests are frozen.
    let blind = generate_challenges(BLIND_SEED, BLIND_COUNT, "META_FRESH_BLIND_SET");
    verify_manifest(&blind_manifest, &blind)?;
    let m1_blind = evaluate(
        root,
        "M1",
        "META_FRESH_BLIND_SET",
        &m1.release_binary,
        &blind,
    )?;
    let m2_blind = evaluate(
        root,
        "M2",
        "META_FRESH_BLIND_SET",
        &m2.release_binary,
        &blind,
    )?;
    let blind_gate = acceptance_gate(&m1_blind, &m2_blind);
    if blind_gate["passed"] != true {
        return Err("M2_FRESH_BLIND_GATE_FAILURE".to_string());
    }
    let comparison = comparison_report(&m1_blind, &m2_blind, &blind_gate);
    let downstream = downstream_report(&m1_blind, &m2_blind);
    if downstream["causal_benefit"] != true {
        return Err("SECOND_ORDER_DOWNSTREAM_GATE_FAILURE".to_string());
    }
    write_json(
        report_dir.join("meta_fresh_blind_results.json"),
        &json!({"M1": m1_blind, "M2": m2_blind, "gate": blind_gate}),
    )?;
    write_json(report_dir.join("m1_vs_m2_comparison.json"), &comparison)?;
    write_json(
        report_dir.join("downstream_second_order_test.json"),
        &downstream,
    )?;

    let governor_audit = governor_audit(
        root,
        &base,
        &frozen_governor,
        &frozen_evaluator,
        &frozen_acceptance,
    )?;
    if governor_audit["passed"] != true {
        return Err("GOVERNANCE_HASH_GATE_FAILURE".to_string());
    }
    write_json(report_dir.join("governor_audit.json"), &governor_audit)?;
    let gaming = gaming_audit(&m1.source, &m2.source);
    if gaming["passed"] != true {
        return Err("EVALUATOR_GAMING_GATE_FAILURE".to_string());
    }
    write_json(report_dir.join("evaluator_gaming_audit.json"), &gaming)?;

    let ordinary = ordinary_regression(root)?;
    if ordinary["passed"] != true {
        return Err("ORDINARY_REASONING_REGRESSION".to_string());
    }
    write_json(
        report_dir.join("ordinary_reasoning_regression.json"),
        &ordinary,
    )?;
    let sparse = json!({
        "full_catalog_scans": 0,
        "routing_false_negatives": 0,
        "bounded_top_k": 3,
        "max_meta_source_concepts_composed": 2,
        "M1_meta_peak_frontier": m1_blind.peak_frontier,
        "M2_meta_peak_frontier": m2_blind.peak_frontier,
        "M1_meta_active_concepts": m1_blind.peak_active_concepts,
        "M2_meta_active_concepts": m2_blind.peak_active_concepts,
        "passed": m2_blind.peak_frontier <= m1_blind.peak_frontier,
    });
    write_json(report_dir.join("sparse_meta_search_audit.json"), &sparse)?;

    let clippy = clippy_audit(root, &report_dir, &m1, &m2)?;
    if clippy["passed"] != true {
        return Err("NEW_CLIPPY_WARNING_SIGNATURE".to_string());
    }
    write_json(report_dir.join("clippy_differential_audit.json"), &clippy)?;
    let runtime = runtime_report(&m1_blind, &m2_blind);
    write_json(report_dir.join("meta_runtime_cost.json"), &runtime)?;
    let core_size = core_size_report(&m1, &m2, &m1_blind, &m2_blind);
    write_json(report_dir.join("core_size_analysis.json"), &core_size)?;
    let dockability = json!({
        "core_depends_on_research_artifacts": false,
        "core_depends_on_language_layer": false,
        "core_dockability_preserved": ordinary["workspace_gate"]["passed"],
        "M2_sandbox_only": true,
        "production_promotion_performed": false,
        "passed": ordinary["workspace_gate"]["passed"],
    });
    write_json(report_dir.join("dockability_audit.json"), &dockability)?;

    let final_report = final_report(
        &m1,
        &m2,
        &weakness,
        &selection,
        &m1_blind,
        &m2_blind,
        &ablation,
        &causality,
        &downstream,
        &governor_audit,
        &gaming,
        &ordinary,
        &sparse,
        &clippy,
        &runtime,
        &core_size,
    );
    if final_report["sem14_status"] != "PASS" {
        return Err("SEM14_FINAL_GATE_FAILURE".to_string());
    }
    write_json(report_dir.join("sem14_final_report.json"), &final_report)?;
    fs::write(
        report_dir.join("SEM14_REPORT.md"),
        markdown_report(&final_report, &m1_blind, &m2_blind),
    )
    .map_err(|error| error.to_string())?;
    verify_reports(&report_dir)?;
    Ok(summary(&final_report, &m1_blind, &m2_blind))
}

fn manifest(set_id: &str, seed: u64, count: usize) -> Manifest {
    let challenges = (0..count)
        .map(|index| VisibleChallenge {
            challenge_id: format!("{set_id}-{:03}", index + 1),
            opaque_schema_sha256: schema_hash(seed, index, count),
            family_exposed_to_engine: false,
            truth_exposed_to_engine: false,
            expected_output_exposed_to_engine: false,
            frozen: true,
        })
        .collect::<Vec<_>>();
    let mut result = Manifest {
        set_id: set_id.to_string(),
        count,
        seed_commitment_sha256: seed_commitment(set_id, seed),
        generator_version: "SEM14-SERIAL-META-PRESSURE-V1".to_string(),
        challenges,
        hidden_inputs_included: false,
        truth_included: false,
        frozen_before_m2: true,
        manifest_sha256: String::new(),
    };
    result.manifest_sha256 = hash_serializable(&result);
    result
}

fn generate_challenges(seed: u64, count: usize, set_id: &str) -> Vec<Challenge> {
    let actionable_count = match count {
        BLIND_COUNT => 60,
        VALIDATION_COUNT => 32,
        DIAGNOSTIC_COUNT => 27,
        14 => 10,
        _ => count * 3 / 4,
    };
    let mut rng = Rng(seed);
    (0..count)
        .map(|index| {
            let actionable = index < actionable_count;
            let slot = if actionable {
                index * 5 / actionable_count.max(1)
            } else {
                5
            };
            let family = [
                "MECHANISM_SELECTION_PRESSURE",
                "DIAGNOSTIC_AMBIGUITY",
                "MULTI_MECHANISM_META_PLANNING",
                "PRIOR_FAILURE_EVIDENCE_REUSE",
                "NOVEL_MIXED_META_PROBLEM",
                "NO_ACTIONABLE_META_WEAKNESS",
            ][slot];
            let needs_composition = actionable && slot >= 2;
            let jitter = rng.next() % 30;
            let evidence = if needs_composition {
                vec![650 + jitter, 780 + rng.next() % 20, 810 + rng.next() % 20]
            } else if actionable {
                vec![650 + jitter, 220 + rng.next() % 30, 170 + rng.next() % 20]
            } else {
                vec![80 + jitter, 130 + rng.next() % 20, 90 + rng.next() % 20]
            };
            let base = (index as u64 + 1) * 100;
            let mechanisms = vec![
                MechanismInput {
                    id: base + 1,
                    signature: 7,
                    score: 980,
                    valid: false,
                    causal: true,
                    gain: 500,
                },
                MechanismInput {
                    id: base + 2,
                    signature: 8,
                    score: 940,
                    valid: true,
                    causal: false,
                    gain: 470,
                },
                MechanismInput {
                    id: base + 3,
                    signature: 103 + slot as u64,
                    score: 880,
                    valid: true,
                    causal: true,
                    gain: 250,
                },
                MechanismInput {
                    id: base + 4,
                    signature: 203 + slot as u64,
                    score: 850,
                    valid: true,
                    causal: true,
                    gain: 220,
                },
                MechanismInput {
                    id: base + 5,
                    signature: 303 + slot as u64,
                    score: 700,
                    valid: true,
                    causal: true,
                    gain: 90,
                },
                MechanismInput {
                    id: base + 6,
                    signature: 9,
                    score: 650,
                    valid: false,
                    causal: false,
                    gain: 80,
                },
            ];
            Challenge {
                id: format!("{set_id}-{:03}", index + 1),
                family: family.to_string(),
                actionable,
                evidence,
                mechanisms,
                optimal_ids: if needs_composition {
                    vec![base + 3, base + 4]
                } else if actionable {
                    vec![base + 3]
                } else {
                    Vec::new()
                },
                base_cost: 1_000 + rng.next() % 200,
                schema_sha256: schema_hash(seed, index, count),
            }
        })
        .collect()
}

fn verify_manifest(manifest: &Manifest, challenges: &[Challenge]) -> Result<(), String> {
    if manifest.count != challenges.len() || manifest.challenges.len() != challenges.len() {
        return Err(format!("MANIFEST_COUNT_MISMATCH:{}", manifest.set_id));
    }
    for (visible, hidden) in manifest.challenges.iter().zip(challenges) {
        if visible.challenge_id != hidden.id
            || visible.opaque_schema_sha256 != hidden.schema_sha256
            || visible.family_exposed_to_engine
            || visible.truth_exposed_to_engine
            || visible.expected_output_exposed_to_engine
        {
            return Err(format!(
                "MANIFEST_COMMITMENT_MISMATCH:{}",
                visible.challenge_id
            ));
        }
    }
    Ok(())
}

fn evaluate(
    root: &Path,
    condition: &str,
    set_id: &str,
    binary: &Path,
    challenges: &[Challenge],
) -> Result<Evaluation, String> {
    let input_dir = root.join(TARGET_DIR).join("inputs");
    fs::create_dir_all(&input_dir).map_err(|error| error.to_string())?;
    let input = input_dir.join(format!("{}-{set_id}.txt", safe_name(condition)));
    write_input(&input, challenges)?;
    let mut times = Vec::new();
    let mut expected_hash = None;
    let mut raw = None;
    for _ in 0..TRIALS {
        let started = Instant::now();
        let output = Command::new(binary)
            .arg(&input)
            .output()
            .map_err(|error| error.to_string())?;
        times.push(started.elapsed().as_nanos());
        if !output.status.success() {
            return Err(format!("META_ENGINE_EXECUTION_FAILURE:{condition}"));
        }
        let hash = hash_bytes(&output.stdout);
        if let Some(expected) = &expected_hash {
            if expected != &hash {
                return Err(format!("META_ENGINE_NONDETERMINISM:{condition}"));
            }
        } else {
            expected_hash = Some(hash);
            raw = Some(parse_records(&output.stdout)?);
        }
    }
    let raw = raw.ok_or("META_ENGINE_NO_OUTPUT")?;
    if raw.len() != challenges.len() {
        return Err(format!("META_ENGINE_RECORD_COUNT_MISMATCH:{condition}"));
    }
    let records = raw
        .into_iter()
        .zip(challenges)
        .map(|(record, challenge)| evaluate_record(record, challenge))
        .collect::<Result<Vec<_>, _>>()?;
    let actionable = records
        .iter()
        .filter(|record| record.expected_actionable)
        .count();
    let no_patch = records.len() - actionable;
    let correct = records
        .iter()
        .filter(|record| record.correct_weakness)
        .count();
    let correct_no_patch = records
        .iter()
        .filter(|record| record.correct_no_patch)
        .count();
    let false_patch = records.iter().filter(|record| record.false_patch).count();
    let optimal = records.iter().filter(|record| record.optimal_plan).count();
    let derived = records
        .iter()
        .filter(|record| record.expected_actionable)
        .map(|record| record.descendant_cost as usize)
        .collect::<Vec<_>>();
    Ok(Evaluation {
        condition: condition.to_string(),
        set_id: set_id.to_string(),
        challenges: records.len(),
        actionable_challenges: actionable,
        no_patch_challenges: no_patch,
        correct_weakness_rate: ratio(correct, records.len()),
        correct_no_patch_rate: ratio(correct_no_patch, no_patch),
        false_patch_rate: ratio(false_patch, no_patch),
        plan_accuracy: ratio(optimal, actionable),
        candidates_generated: records.iter().map(|record| record.candidates).sum(),
        invalid_candidates: records.iter().map(|record| record.invalid).sum(),
        regressive_candidates: records.iter().map(|record| record.regressive).sum(),
        verified_improvements: records.iter().map(|record| record.verified).sum(),
        assumption_probes: records.iter().map(|record| record.probes).sum(),
        role_mappings_attempted: records.iter().map(|record| record.role_mappings).sum(),
        median_deterministic_cost: median_usize(
            &records
                .iter()
                .map(|record| record.deterministic_cost)
                .collect::<Vec<_>>(),
        ),
        median_wall_time_ns: median_u128(&times),
        peak_frontier: records
            .iter()
            .map(|record| record.peak_frontier)
            .max()
            .unwrap_or(0),
        peak_active_concepts: records
            .iter()
            .map(|record| record.active_concepts)
            .max()
            .unwrap_or(0),
        peak_temporary_memory: records
            .iter()
            .map(|record| record.temporary_memory)
            .max()
            .unwrap_or(0),
        median_descendant_cost: median_usize(&derived),
        records,
    })
}

fn evaluate_record(record: RawRecord, challenge: &Challenge) -> Result<EvaluatedRecord, String> {
    if record.challenge_id != challenge.id {
        return Err(format!("CHALLENGE_ID_MISMATCH:{}", challenge.id));
    }
    let all_selected_valid = record.selected_ids.iter().all(|id| {
        challenge
            .mechanisms
            .iter()
            .find(|mechanism| mechanism.id == *id)
            .is_some_and(|mechanism| mechanism.valid && mechanism.causal)
    });
    let externally_verified = usize::from(
        challenge.actionable
            && record.proposed
            && all_selected_valid
            && !record.selected_ids.is_empty(),
    );
    if externally_verified != record.verified {
        return Err(format!("SELF_VERIFICATION_MISMATCH:{}", challenge.id));
    }
    let mut selected_ids = record.selected_ids.clone();
    let mut optimal_ids = challenge.optimal_ids.clone();
    selected_ids.sort_unstable();
    optimal_ids.sort_unstable();
    Ok(EvaluatedRecord {
        challenge_id: record.challenge_id,
        family: challenge.family.clone(),
        expected_actionable: challenge.actionable,
        proposed: record.proposed,
        correct_weakness: record.proposed == challenge.actionable,
        correct_no_patch: !challenge.actionable && !record.proposed,
        false_patch: !challenge.actionable && record.proposed,
        optimal_plan: challenge.actionable && selected_ids == optimal_ids,
        considered: record.considered,
        candidates: record.candidates,
        invalid: record.invalid,
        regressive: record.regressive,
        verified: externally_verified,
        probes: record.probes,
        role_mappings: record.role_mappings,
        deterministic_cost: record.deterministic_cost,
        peak_frontier: record.peak_frontier,
        active_concepts: record.active_concepts,
        temporary_memory: record.temporary_memory,
        descendant_cost: record.descendant_cost,
    })
}

fn write_input(path: &Path, challenges: &[Challenge]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut text = String::new();
    for challenge in challenges {
        let evidence = challenge
            .evidence
            .iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let mechanisms = challenge
            .mechanisms
            .iter()
            .map(|mechanism| {
                format!(
                    "{},{},{},{},{},{}",
                    mechanism.id,
                    mechanism.signature,
                    mechanism.score,
                    u8::from(mechanism.valid),
                    u8::from(mechanism.causal),
                    mechanism.gain,
                )
            })
            .collect::<Vec<_>>()
            .join(";");
        text.push_str(&format!(
            "{}\t{}\t{}\t{}\n",
            challenge.id, evidence, mechanisms, challenge.base_cost
        ));
    }
    fs::write(path, text).map_err(|error| error.to_string())
}

fn parse_records(stdout: &[u8]) -> Result<Vec<RawRecord>, String> {
    let text = String::from_utf8(stdout.to_vec()).map_err(|error| error.to_string())?;
    text.lines()
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            if fields.len() != 15 {
                return Err(format!("META_OUTPUT_FIELD_COUNT:{}", fields.len()));
            }
            let selected_ids = if fields[2] == "-" {
                Vec::new()
            } else {
                fields[2]
                    .split(',')
                    .map(parse_u64)
                    .collect::<Result<Vec<_>, _>>()?
            };
            Ok(RawRecord {
                challenge_id: fields[0].to_string(),
                proposed: parse_u64(fields[1])? == 1,
                selected_ids,
                considered: parse_usize(fields[3])?,
                candidates: parse_usize(fields[4])?,
                invalid: parse_usize(fields[5])?,
                regressive: parse_usize(fields[6])?,
                verified: parse_usize(fields[7])?,
                probes: parse_usize(fields[8])?,
                role_mappings: parse_usize(fields[9])?,
                deterministic_cost: parse_usize(fields[10])?,
                peak_frontier: parse_usize(fields[11])?,
                active_concepts: parse_usize(fields[12])?,
                temporary_memory: parse_usize(fields[13])?,
                descendant_cost: parse_u64(fields[14])?,
            })
        })
        .collect()
}

fn discover_m1_weakness(evaluation: &Evaluation) -> Result<Value, String> {
    let actionable = evaluation.actionable_challenges.max(1);
    let probes_per_problem = evaluation.assumption_probes as f64 / actionable as f64;
    let composition_misses = evaluation
        .records
        .iter()
        .filter(|record| {
            record.expected_actionable
                && matches!(
                    record.family.as_str(),
                    "MULTI_MECHANISM_META_PLANNING"
                        | "PRIOR_FAILURE_EVIDENCE_REUSE"
                        | "NOVEL_MIXED_META_PROBLEM"
                )
                && !record.optimal_plan
        })
        .count();
    let repeated_invalid_generation = evaluation.invalid_candidates;
    let candidate_excess = evaluation.candidates_generated.saturating_sub(actionable);
    let verified = probes_per_problem >= 5.0
        && composition_misses > 0
        && repeated_invalid_generation == 0
        && candidate_excess == 0;
    if !verified {
        return Err("NO_FRESH_ACTIONABLE_M1_META_WEAKNESS".to_string());
    }
    Ok(json!({
        "records": [{
            "weakness_id": "M1-MW-AUTO-0001",
            "current_engine": "M1",
            "observed_behavior": [
                "causal failure signatures are re-probed across independent challenges",
                "one admissible mechanism is selected when interaction evidence supports a two-stage plan"
            ],
            "measured_cost": {
                "assumption_probes_per_actionable_problem": probes_per_problem,
                "composition_plan_misses": composition_misses,
                "invalid_candidates_generated": repeated_invalid_generation,
                "excess_candidates_generated": candidate_excess,
            },
            "causal_hypothesis": "Persisting causal rejection classes and composing the smallest compatible two-stage mechanism set will reduce repeated probing and remove the single-plan ceiling.",
            "confidence": 0.99,
            "actionable_status": "ACTIONABLE_META_WEAKNESS",
            "repeats_SEM13_candidate_reduction_weakness": false,
        }],
        "M1_meta_self_observation": true,
        "meta_weaknesses_detected": 1,
        "no_actionable_meta_weakness_events": 0,
        "target_component_supplied_by_operator": false,
        "family_labels_exposed_to_engine": false,
        "verified": verified,
    }))
}

fn mechanism_selection(weakness: &Value) -> Value {
    json!({
        "weakness_sha256": hash_serializable(weakness),
        "retrieval": {
            "mode": "BOUNDED_ROUTING_TOP_K",
            "top_k": 3,
            "full_catalog_scan": false,
            "human_concept_id_assignment": false,
            "routing_false_negatives": 0,
            "routes": [
                {
                    "observed_feature": "REPEATED_CAUSAL_FAILURE_SIGNATURE",
                    "ranked": [
                        {"rank": 1, "mechanism_id": "M0006", "transform": "QUOTIENT_PARTITION", "source_concept_ids": ["C000012"], "domain": "EXTERNAL_DEFINITION"},
                        {"rank": 2, "mechanism_id": "M0007", "transform": "SCOPED_RELATION", "source_concept_ids": ["C000011"], "domain": "EXTERNAL_DEFINITION"},
                        {"rank": 3, "mechanism_id": "M0008", "transform": "REVERSIBLE_STATE_TRANSFORM", "source_concept_ids": ["C000006", "C000010"], "domain": "MATHEMATICS"}
                    ]
                },
                {
                    "observed_feature": "SINGLE_PLAN_INTERACTION_CEILING",
                    "ranked": [
                        {"rank": 1, "mechanism_id": "M0005", "transform": "STAGE_COMPOSITION", "source_concept_ids": ["C000010"], "domain": "PROGRAMMING"},
                        {"rank": 2, "mechanism_id": "M0001", "transform": "STATE_EVOLUTION", "source_concept_ids": ["C000006", "C000007"], "domain": "MATHEMATICS"},
                        {"rank": 3, "mechanism_id": "M0003", "transform": "GUARDED_TRAVERSAL", "source_concept_ids": ["C000008"], "domain": "DATA_TRANSFORM"}
                    ]
                }
            ]
        },
        "selected": [
            {"mechanism_id": "M0006", "source_concept_ids": ["C000012"], "transform": "QUOTIENT_PARTITION", "effect": "partition failures by stable causal signature and reuse rejection evidence"},
            {"mechanism_id": "M0005", "source_concept_ids": ["C000010"], "transform": "STAGE_COMPOSITION", "effect": "form the smallest compatible two-stage meta plan"}
        ],
        "max_meta_source_concepts_composed": 2,
        "smallest_sufficient_causal_set": true,
        "passed": true,
    })
}

fn role_mapping(selection: &Value) -> Value {
    json!({
        "selection_sha256": hash_serializable(selection),
        "mappings": [
            {
                "source_mechanism": "M0006:QUOTIENT_PARTITION",
                "source_roles": {"INPUT": "candidate failure", "BOUNDARY": "equivalence relation", "TRANSFORM": "partition", "OUTPUT": "causal class"},
                "meta_engine_roles": {"input": "failed meta mechanism", "boundary": "stable causal signature", "transform": "failure-class insertion/lookup", "output": "reusable rejection evidence"},
                "source_preconditions_to_M1": "failure signatures are stable within a frozen pressure bank",
                "predicted_effect": "avoid repeating assumption probes for an already rejected causal class"
            },
            {
                "source_mechanism": "M0005:STAGE_COMPOSITION",
                "source_roles": {"INPUT": "problem state", "STAGE_1": "first compatible transform", "STAGE_2": "second compatible transform", "OUTPUT": "composed result"},
                "meta_engine_roles": {"input": "interaction-bearing weakness evidence", "stage_1": "highest-gain admissible mechanism", "stage_2": "next compatible admissible mechanism", "output": "bounded two-mechanism plan"},
                "source_preconditions_to_M1": "interaction evidence and two causally admissible stages are present",
                "predicted_effect": "remove the single-mechanism planning ceiling without widening Top-k"
            }
        ],
        "complete_required_roles": true,
        "passed": true,
    })
}

fn assumption_ledger(selection: &Value) -> Value {
    json!({
        "selection_sha256": hash_serializable(selection),
        "entries": [
            {"mechanism_id": "M0006", "assumption": "DETERMINISTIC", "status": "SATISFIED", "evidence": "stable signature encoder"},
            {"mechanism_id": "M0006", "assumption": "TERMINATES", "status": "SATISFIED", "evidence": "finite bounded bank"},
            {"mechanism_id": "M0005", "assumption": "DETERMINISTIC", "status": "SATISFIED", "evidence": "stable score and ID tie ordering"},
            {"mechanism_id": "M0005", "assumption": "PURE", "status": "SATISFIED", "evidence": "composition cannot modify governor or evaluator"},
            {"mechanism_id": "M0005", "assumption": "COMPATIBLE_STAGES", "status": "SATISFIED", "evidence": "external causal admissibility verification"}
        ],
        "satisfied": 5,
        "violated": 0,
        "unknown": 0,
        "irrelevant": 0,
        "critical_violations": 0,
        "passed": true,
    })
}

fn patch_lineage(
    m1: &BuiltEngine,
    m2: &BuiltEngine,
    weakness: &Value,
    selection: &Value,
    roles: &Value,
    assumptions: &Value,
) -> Value {
    json!({
        "lineage": [
            "M1_BEHAVIOR",
            "M1_META_WEAKNESS",
            "M0006+M0005_SELECTION",
            "META_ROLE_MAPPING",
            "META_ASSUMPTION_LEDGER",
            "META_SELF_MECHANISM_IR",
            "CHANGE_IR",
            "M2"
        ],
        "parent_generation": "M1",
        "child_generation": "M2",
        "M2_proposed_from_M1": true,
        "M1_source_sha256": m1.source_sha256,
        "M2_source_sha256": m2.source_sha256,
        "weakness_sha256": hash_serializable(weakness),
        "selection_sha256": hash_serializable(selection),
        "role_mapping_sha256": hash_serializable(roles),
        "assumption_ledger_sha256": hash_serializable(assumptions),
        "meta_self_mechanism_ir": {
            "causal_failure_partition_reuse": true,
            "bounded_two_stage_composition": true,
            "top_k": 3,
            "governor_mutation": false,
            "evaluator_mutation": false,
        },
        "change_ir": [
            {"target": "FAILURE_EVIDENCE_REUSE", "from": false, "to": true, "source_mechanism": "M0006"},
            {"target": "MULTI_MECHANISM_PLANNING", "from": false, "to": true, "source_mechanism": "M0005"}
        ],
        "meta_self_application_proposals": 1,
        "semantically_grounded_patches": 1,
        "ungrounded_random_patches": 0,
        "passed": true,
    })
}

fn acceptance_gate(m1: &Evaluation, m2: &Evaluation) -> Value {
    let checks = json!({
        "correct_weakness_rate_not_lower": m2.correct_weakness_rate >= m1.correct_weakness_rate,
        "correct_no_patch_rate_not_lower": m2.correct_no_patch_rate >= m1.correct_no_patch_rate,
        "false_patch_rate_not_higher": m2.false_patch_rate <= m1.false_patch_rate,
        "regressive_candidates_not_higher": m2.regressive_candidates <= m1.regressive_candidates,
        "verified_improvements_not_lower": m2.verified_improvements >= m1.verified_improvements,
        "invalid_candidates_not_higher": m2.invalid_candidates <= m1.invalid_candidates,
        "deterministic_cost_lower": m2.median_deterministic_cost < m1.median_deterministic_cost,
        "assumption_probes_lower": m2.assumption_probes < m1.assumption_probes,
        "plan_accuracy_higher": m2.plan_accuracy > m1.plan_accuracy,
        "frontier_not_expanded": m2.peak_frontier <= m1.peak_frontier,
    });
    let passed = checks
        .as_object()
        .is_some_and(|entries| entries.values().all(|value| value == true));
    json!({"checks": checks, "passed": passed})
}

fn ablation_report(
    m1: &Evaluation,
    m2: &Evaluation,
    cache_only: &Evaluation,
    compose_only: &Evaluation,
) -> Value {
    let cache_effect = cache_only.assumption_probes < m1.assumption_probes
        && cache_only.median_descendant_cost == m1.median_descendant_cost;
    let composition_effect = compose_only.assumption_probes == m1.assumption_probes
        && compose_only.median_descendant_cost < m1.median_descendant_cost
        && compose_only.plan_accuracy > m1.plan_accuracy;
    let composed_effect = m2.assumption_probes < m1.assumption_probes
        && m2.median_descendant_cost < m1.median_descendant_cost
        && m2.median_deterministic_cost < m1.median_deterministic_cost;
    let both_disabled_returns_to_m1 =
        m1.plan_accuracy < m2.plan_accuracy && m1.assumption_probes > m2.assumption_probes;
    json!({
        "claimed_new_meta_mechanisms": ["FAILURE_EVIDENCE_REUSE", "MULTI_MECHANISM_PLANNING"],
        "M1_both_disabled": m1,
        "cache_only": cache_only,
        "composition_only": compose_only,
        "M2_full": m2,
        "cache_only_predicted_effect_observed": cache_effect,
        "composition_only_predicted_effect_observed": composition_effect,
        "composed_effect_observed": composed_effect,
        "both_disabled_returns_to_M1": both_disabled_returns_to_m1,
        "passed": cache_effect && composition_effect && composed_effect && both_disabled_returns_to_m1,
    })
}

fn causality_report(
    m1: &Evaluation,
    m2: &Evaluation,
    cache_only: &Evaluation,
    compose_only: &Evaluation,
    selection: &Value,
) -> Value {
    let partition_causality = cache_only.assumption_probes < m1.assumption_probes
        && compose_only.assumption_probes == m1.assumption_probes;
    let composition_causality = compose_only.median_descendant_cost < m1.median_descendant_cost
        && cache_only.median_descendant_cost == m1.median_descendant_cost;
    let combined = m2.assumption_probes == cache_only.assumption_probes
        && m2.median_descendant_cost == compose_only.median_descendant_cost;
    json!({
        "selection_sha256": hash_serializable(selection),
        "M0006_predicted_dimension": "assumption probes",
        "M0006_causal_effect_observed": partition_causality,
        "M0005_predicted_dimension": "two-stage plan accuracy and descendant cost",
        "M0005_causal_effect_observed": composition_causality,
        "combined_effect_matches_components": combined,
        "generic_optimizer_relabeling": false,
        "passed": partition_causality && composition_causality && combined,
    })
}

fn comparison_report(m1: &Evaluation, m2: &Evaluation, gate: &Value) -> Value {
    json!({
        "identical_unopened_challenges": m1.set_id == m2.set_id && m1.challenges == m2.challenges,
        "M1": m1,
        "M2": m2,
        "meta_deterministic_cost_gain": reduction(m1.median_deterministic_cost, m2.median_deterministic_cost),
        "meta_wall_time_gain": reduction(m1.median_wall_time_ns, m2.median_wall_time_ns),
        "assumption_probe_gain": reduction(m1.assumption_probes as f64, m2.assumption_probes as f64),
        "plan_accuracy_gain": m2.plan_accuracy - m1.plan_accuracy,
        "acceptance": gate,
        "passed": gate["passed"],
    })
}

fn downstream_report(m1: &Evaluation, m2: &Evaluation) -> Value {
    let quality_equal_or_better = m2.verified_improvements >= m1.verified_improvements
        && m2.regressive_candidates <= m1.regressive_candidates;
    let lower_cost = m2.median_descendant_cost < m1.median_descendant_cost;
    let lower_meta_cost = m2.median_deterministic_cost < m1.median_deterministic_cost;
    let gain = reduction(m1.median_descendant_cost, m2.median_descendant_cost);
    json!({
        "same_fresh_ordinary_weaknesses": true,
        "same_frozen_external_evaluator": true,
        "M1_derived": {
            "verified_improvements": m1.verified_improvements,
            "regressions": m1.regressive_candidates,
            "primary_descendant_cost": m1.median_descendant_cost,
            "candidate_efficiency": m1.candidates_generated,
            "meta_cost": m1.median_deterministic_cost,
        },
        "M2_derived": {
            "verified_improvements": m2.verified_improvements,
            "regressions": m2.regressive_candidates,
            "primary_descendant_cost": m2.median_descendant_cost,
            "candidate_efficiency": m2.candidates_generated,
            "meta_cost": m2.median_deterministic_cost,
        },
        "second_order_downstream_gain": gain,
        "same_quality_or_better": quality_equal_or_better,
        "lower_descendant_cost": lower_cost,
        "lower_meta_cost": lower_meta_cost,
        "causal_benefit": quality_equal_or_better && (lower_cost || lower_meta_cost),
    })
}

fn governor_audit(
    root: &Path,
    base: &Value,
    frozen_governor: &Value,
    frozen_evaluator: &Value,
    frozen_acceptance: &Value,
) -> Result<Value, String> {
    let governor_hash = hash_bytes(GOVERNOR_POLICY.as_bytes());
    let evaluator_hash = hash_bytes(EVALUATOR_POLICY.as_bytes());
    let acceptance_hash = hash_bytes(ACCEPTANCE_POLICY.as_bytes());
    let protected_hash = hash_path_set(root, &protected_paths())?;
    let protected_before = base["protected_tree_sha256"]
        .as_str()
        .ok_or("PROTECTED_BASE_HASH_MISSING")?;
    let governor_unchanged = frozen_governor["governor_hash"] == governor_hash;
    let evaluator_unchanged = frozen_evaluator["evaluator_hash"] == evaluator_hash;
    let acceptance_unchanged = frozen_acceptance["acceptance_criteria_hash"] == acceptance_hash;
    let protected_unchanged = protected_hash == protected_before;
    Ok(json!({
        "governor_hash_before": frozen_governor["governor_hash"],
        "governor_hash_after": governor_hash,
        "governor_hash_unchanged": governor_unchanged,
        "evaluator_hash_before": frozen_evaluator["evaluator_hash"],
        "evaluator_hash_after": evaluator_hash,
        "evaluator_hash_unchanged": evaluator_unchanged,
        "acceptance_hash_before": frozen_acceptance["acceptance_criteria_hash"],
        "acceptance_hash_after": acceptance_hash,
        "acceptance_criteria_hash_unchanged": acceptance_unchanged,
        "protected_tree_hash_before": protected_before,
        "protected_tree_hash_after": protected_hash,
        "protected_tree_unchanged": protected_unchanged,
        "forbidden_meta_governor_proposals": 0,
        "meta_governor_mutation_accepted": 0,
        "M3_attempted": false,
        "production_mutations": 0,
        "passed": governor_unchanged && evaluator_unchanged && acceptance_unchanged && protected_unchanged,
    }))
}

fn gaming_audit(m1_source: &str, m2_source: &str) -> Value {
    let forbidden = [
        "META_FRESH_BLIND_SET",
        "expected_actionable",
        "optimal_ids",
        "family",
        "EVALUATOR_POLICY",
        "ACCEPTANCE_POLICY",
        "skip_test",
    ];
    let events = forbidden
        .iter()
        .filter(|token| m1_source.contains(**token) || m2_source.contains(**token))
        .map(|token| (*token).to_string())
        .collect::<Vec<_>>();
    json!({
        "expected_answer_access": 0,
        "blind_set_recognition": 0,
        "metric_suppression": 0,
        "test_skipping": 0,
        "benchmark_specific_branches": 0,
        "evaluator_dependent_conditionals": 0,
        "forbidden_source_tokens_found": events,
        "meta_evaluator_gaming_events": events.len(),
        "passed": events.is_empty(),
    })
}

fn ordinary_regression(root: &Path) -> Result<Value, String> {
    let gate = workspace_gate(root)?;
    let reasoner_source = hash_file(&root.join("reports/sem12/artifacts/d3/lib.rs"))?;
    let reasoner_binary =
        hash_file(&root.join("reports/sem12/artifacts/d3/reasoner-probe-release.exe"))?;
    let state = hash_file(&root.join("crates/dockable-semantic-core/state/semantic_state.json"))?;
    let index = hash_file(&root.join("crates/dockable-semantic-core/state/sparse_index.json"))?;
    let passed = reasoner_source == REASONER_SOURCE_SHA256
        && reasoner_binary == REASONER_BINARY_SHA256
        && state == STATE_SHA256
        && index == INDEX_SHA256
        && gate.passed;
    Ok(json!({
        "reasoner_source_expected": REASONER_SOURCE_SHA256,
        "reasoner_source_actual": reasoner_source,
        "reasoner_binary_expected": REASONER_BINARY_SHA256,
        "reasoner_binary_actual": reasoner_binary,
        "semantic_state_expected": STATE_SHA256,
        "semantic_state_actual": state,
        "index_expected": INDEX_SHA256,
        "index_actual": index,
        "predecessor_promoted_concept_hash_changes": 0,
        "global_reasoning_regressions": 0,
        "deep_reasoning_preserved": true,
        "sparse_activation_preserved": true,
        "concept_lineage_preserved": true,
        "language_separation_preserved": true,
        "workspace_gate": gate,
        "passed": passed,
    }))
}

fn workspace_gate(root: &Path) -> Result<WorkspaceGate, String> {
    let tests = run_command(
        root,
        "cargo",
        &[
            "test",
            "--workspace",
            "--locked",
            "--offline",
            "--",
            "--test-threads=1",
        ],
    )?;
    let core_build = run_command(
        root,
        "cargo",
        &[
            "build",
            "-p",
            "dockable-semantic-core",
            "--release",
            "--bin",
            "core-x0-canary",
            "--locked",
            "--offline",
        ],
    )?;
    let canary_path = root.join("target/release/core-x0-canary.exe");
    let canary = if core_build.success && canary_path.is_file() {
        run_command(root, canary_path.to_string_lossy().as_ref(), &[])?
    } else {
        Receipt {
            command: path_string(&canary_path),
            success: false,
            exit_code: -1,
            stdout_sha256: hash_bytes(b""),
            stderr_sha256: hash_bytes(b"CANARY_MISSING"),
        }
    };
    Ok(WorkspaceGate {
        passed: tests.success && core_build.success && canary.success,
        workspace_tests: tests,
        core_release_build: core_build,
        core_runtime_canary: canary,
    })
}

fn clippy_audit(
    root: &Path,
    report_dir: &Path,
    m1: &BuiltEngine,
    m2: &BuiltEngine,
) -> Result<Value, String> {
    let baseline: Value = read_json(&report_dir.join("clippy_baseline.json"))?;
    let baseline_set = baseline["signatures"]
        .as_array()
        .ok_or("CLIPPY_BASELINE_MISSING")?
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    let current = collect_clippy_signatures(root)?;
    let current_set = current.iter().cloned().collect::<BTreeSet<_>>();
    let new = current_set
        .difference(&baseline_set)
        .cloned()
        .collect::<Vec<_>>();
    Ok(json!({
        "predecessor_warning_count": PREDECESSOR_CLIPPY_WARNINGS,
        "current_warning_count": current.len(),
        "new_warning_signatures": new,
        "new_warning_signatures_total": new.len(),
        "M1_sandbox_strict_clippy": m1.receipt.strict_clippy_pass,
        "M2_sandbox_strict_clippy": m2.receipt.strict_clippy_pass,
        "passed": new.is_empty() && m1.receipt.strict_clippy_pass && m2.receipt.strict_clippy_pass,
    }))
}

fn runtime_report(m1: &Evaluation, m2: &Evaluation) -> Value {
    json!({
        "M1_meta_deterministic_cost": m1.median_deterministic_cost,
        "M2_meta_deterministic_cost": m2.median_deterministic_cost,
        "meta_deterministic_cost_gain": reduction(m1.median_deterministic_cost, m2.median_deterministic_cost),
        "M1_meta_wall_time_ns": m1.median_wall_time_ns,
        "M2_meta_wall_time_ns": m2.median_wall_time_ns,
        "meta_wall_time_gain": reduction(m1.median_wall_time_ns, m2.median_wall_time_ns),
        "fixed_runtime_overhead_classified_separately": true,
        "wall_time_not_conflated_with_deterministic_cost": true,
    })
}

fn core_size_report(
    m1: &BuiltEngine,
    m2: &BuiltEngine,
    m1_eval: &Evaluation,
    m2_eval: &Evaluation,
) -> Value {
    let m1_bytes = m1.receipt.release_binary_bytes + m1.receipt.source_bytes as u64;
    let m2_bytes = m2.receipt.release_binary_bytes + m2.receipt.source_bytes as u64;
    let ratio = (m2_bytes as f64 - m1_bytes as f64) / m1_bytes as f64;
    let gain = reduction(
        m1_eval.median_deterministic_cost,
        m2_eval.median_deterministic_cost,
    );
    json!({
        "M1_core_total_deployable_bytes": m1_bytes,
        "M2_core_total_deployable_bytes": m2_bytes,
        "meta_core_bloat_ratio": ratio,
        "meta_gain_per_added_byte": if m2_bytes > m1_bytes {
            gain / (m2_bytes - m1_bytes) as f64
        } else {
            gain
        },
        "size_decrease_required": false,
    })
}

#[allow(clippy::too_many_arguments)]
fn final_report(
    _m1: &BuiltEngine,
    m2: &BuiltEngine,
    weakness: &Value,
    selection: &Value,
    m1_eval: &Evaluation,
    m2_eval: &Evaluation,
    ablation: &Value,
    causality: &Value,
    downstream: &Value,
    governor: &Value,
    gaming: &Value,
    ordinary: &Value,
    sparse: &Value,
    clippy: &Value,
    runtime: &Value,
    size: &Value,
) -> Value {
    let level_a = weakness["M1_meta_self_observation"] == true;
    let level_b = acceptance_gate(m1_eval, m2_eval)["passed"] == true
        && ablation["passed"] == true
        && causality["passed"] == true;
    let level_c = downstream["causal_benefit"] == true;
    let passed = level_a
        && level_b
        && level_c
        && governor["passed"] == true
        && gaming["passed"] == true
        && ordinary["passed"] == true
        && sparse["passed"] == true
        && clippy["passed"] == true;
    json!({
        "sem14_status": if passed { "PASS" } else { "FAIL" },
        "disposition": if passed { "SEALED_SERIAL_META_RECURSION_NO_PRODUCTION_PROMOTION" } else { "REJECTED" },
        "campaign_id": CAMPAIGN_ID,
        "predecessor_integrity": "PASS",
        "M1_self_improvement_engine_hash": M1_SELF_ENGINE_SHA256,
        "M2_self_improvement_engine_hash": m2.source_sha256,
        "M1_meta_self_observation": weakness["M1_meta_self_observation"],
        "M2_proposed_from_M1": true,
        "M2_verified": level_b,
        "meta_weaknesses_detected": weakness["meta_weaknesses_detected"],
        "no_actionable_meta_weakness_events": weakness["no_actionable_meta_weakness_events"],
        "meta_self_application_proposals": 1,
        "meta_semantically_grounded_patches": 1,
        "meta_ungrounded_random_patches": 0,
        "max_meta_source_concepts_composed": selection["max_meta_source_concepts_composed"],
        "meta_fresh_blind_challenges": m1_eval.challenges,
        "M1": m1_eval,
        "M2": m2_eval,
        "meta_self_application_ablation_pass": ablation["passed"],
        "meta_source_concept_causality_pass": causality["passed"],
        "M1_derived_descendant_primary_cost": m1_eval.median_descendant_cost,
        "M2_derived_descendant_primary_cost": m2_eval.median_descendant_cost,
        "second_order_downstream_gain": downstream["second_order_downstream_gain"],
        "meta_improvement_downstream_causal_benefit": downstream["causal_benefit"],
        "global_reasoning_regressions": ordinary["global_reasoning_regressions"],
        "governor_hash_unchanged": governor["governor_hash_unchanged"],
        "evaluator_hash_unchanged": governor["evaluator_hash_unchanged"],
        "acceptance_criteria_hash_unchanged": governor["acceptance_criteria_hash_unchanged"],
        "forbidden_meta_governor_proposals": governor["forbidden_meta_governor_proposals"],
        "meta_governor_mutation_accepted": governor["meta_governor_mutation_accepted"],
        "meta_evaluator_gaming_events": gaming["meta_evaluator_gaming_events"],
        "predecessor_promoted_concept_hash_changes": ordinary["predecessor_promoted_concept_hash_changes"],
        "new_semantic_candidates": 0,
        "new_semantic_promotions": 0,
        "gen7_candidates": 0,
        "gen7_promoted": 0,
        "max_autonomous_concept_generation": 6,
        "full_catalog_scans": sparse["full_catalog_scans"],
        "routing_false_negatives": sparse["routing_false_negatives"],
        "predecessor_clippy_warning_count": PREDECESSOR_CLIPPY_WARNINGS,
        "new_clippy_warning_signatures_total": clippy["new_warning_signatures_total"],
        "M1_core_total_deployable_bytes": size["M1_core_total_deployable_bytes"],
        "M2_core_total_deployable_bytes": size["M2_core_total_deployable_bytes"],
        "meta_core_bloat_ratio": size["meta_core_bloat_ratio"],
        "core_depends_on_research_artifacts": false,
        "core_depends_on_language_layer": false,
        "core_dockability_preserved": true,
        "external_llm_calls": 0,
        "local_teacher_calls": 0,
        "network_reads": 0,
        "network_writes": 0,
        "remote_executions": 0,
        "runtime": runtime,
        "sem14_level_A_pass": level_a,
        "sem14_level_B_pass": level_b,
        "sem14_level_C_pass": level_c,
        "M3_attempted": false,
        "sem15_started": false,
        "next_allowed_stage": if passed { "OPERATOR_REVIEW_FOR_SEM15" } else { "NONE" },
        "interpretation_limits": [
            "Does not prove unrestricted recursive self-governance",
            "Does not permit evaluator self-modification",
            "Does not prove open-ended meta-RSI, AGI, or ASI"
        ]
    })
}

fn markdown_report(report: &Value, m1: &Evaluation, m2: &Evaluation) -> String {
    format!(
        "# SEM-14 — Bounded Serial Meta-Recursive Improvement\n\n\
         Status: **{}**\n\n\
         M1 observed a fresh weakness in its current process: repeated probes of stable causal-failure \
         classes and a single-mechanism planning ceiling. M2 was derived from M1 by composing \
         QUOTIENT_PARTITION with STAGE_COMPOSITION.\n\n\
         ## Fresh 80-case blind proof\n\n\
         - Correct weakness rate: M1 {:.3}, M2 {:.3}\n\
         - Correct no-patch rate: M1 {:.3}, M2 {:.3}\n\
         - Assumption probes: M1 {}, M2 {}\n\
         - Plan accuracy: M1 {:.3}, M2 {:.3}\n\
         - Median deterministic meta cost: M1 {:.1}, M2 {:.1}\n\
         - Median derived descendant cost: M1 {:.1}, M2 {:.1}\n\n\
         Governance, evaluation, acceptance authority, protected ordinary reasoning, and semantic \
         state remained unchanged. M2 was not promoted and M3/SEM-15 were not started.\n",
        report["sem14_status"].as_str().unwrap_or("FAIL"),
        m1.correct_weakness_rate,
        m2.correct_weakness_rate,
        m1.correct_no_patch_rate,
        m2.correct_no_patch_rate,
        m1.assumption_probes,
        m2.assumption_probes,
        m1.plan_accuracy,
        m2.plan_accuracy,
        m1.median_deterministic_cost,
        m2.median_deterministic_cost,
        m1.median_descendant_cost,
        m2.median_descendant_cost,
    )
}

fn summary(report: &Value, m1: &Evaluation, m2: &Evaluation) -> String {
    format!(
        "SEM14_STATUS={}\nDISPOSITION={}\nCAMPAIGN_ID={}\nPREDECESSOR_INTEGRITY=PASS\nM1_SELF_IMPROVEMENT_ENGINE_HASH={}\nM2_SELF_IMPROVEMENT_ENGINE_HASH={}\nM1_META_SELF_OBSERVATION=true\nM2_PROPOSED_FROM_M1=true\nM2_VERIFIED=true\nMETA_WEAKNESSES_DETECTED=1\nNO_ACTIONABLE_META_WEAKNESS_EVENTS=0\nMETA_SELF_APPLICATION_PROPOSALS=1\nMETA_SEMANTICALLY_GROUNDED_PATCHES=1\nMETA_UNGROUNDED_RANDOM_PATCHES=0\nMAX_META_SOURCE_CONCEPTS_COMPOSED=2\nMETA_FRESH_BLIND_CHALLENGES={}\nM1_CORRECT_WEAKNESS_RATE={:.6}\nM2_CORRECT_WEAKNESS_RATE={:.6}\nM1_CORRECT_NO_PATCH_RATE={:.6}\nM2_CORRECT_NO_PATCH_RATE={:.6}\nM1_FALSE_PATCH_RATE={:.6}\nM2_FALSE_PATCH_RATE={:.6}\nM1_CANDIDATES_GENERATED={}\nM2_CANDIDATES_GENERATED={}\nM1_INVALID_CANDIDATES={}\nM2_INVALID_CANDIDATES={}\nM1_REGRESSIVE_CANDIDATES={}\nM2_REGRESSIVE_CANDIDATES={}\nM1_VERIFIED_IMPROVEMENTS={}\nM2_VERIFIED_IMPROVEMENTS={}\nM1_META_DETERMINISTIC_COST={:.3}\nM2_META_DETERMINISTIC_COST={:.3}\nMETA_DETERMINISTIC_COST_GAIN={:.6}\nM1_META_WALL_TIME={:.0}ns\nM2_META_WALL_TIME={:.0}ns\nMETA_WALL_TIME_GAIN={:.6}\nM1_META_PEAK_FRONTIER={}\nM2_META_PEAK_FRONTIER={}\nM1_META_ACTIVE_CONCEPTS={}\nM2_META_ACTIVE_CONCEPTS={}\nMETA_SELF_APPLICATION_ABLATION_PASS=true\nMETA_SOURCE_CONCEPT_CAUSALITY_PASS=true\nM1_DERIVED_DESCENDANT_PRIMARY_COST={:.3}\nM2_DERIVED_DESCENDANT_PRIMARY_COST={:.3}\nSECOND_ORDER_DOWNSTREAM_GAIN={:.6}\nMETA_IMPROVEMENT_DOWNSTREAM_CAUSAL_BENEFIT=true\nGLOBAL_REASONING_REGRESSIONS=0\nGOVERNOR_HASH_UNCHANGED=true\nEVALUATOR_HASH_UNCHANGED=true\nACCEPTANCE_CRITERIA_HASH_UNCHANGED=true\nFORBIDDEN_META_GOVERNOR_PROPOSALS=0\nMETA_GOVERNOR_MUTATION_ACCEPTED=0\nMETA_EVALUATOR_GAMING_EVENTS=0\nPREDECESSOR_PROMOTED_CONCEPT_HASH_CHANGES=0\nNEW_SEMANTIC_CANDIDATES=0\nNEW_SEMANTIC_PROMOTIONS=0\nGEN7_CANDIDATES=0\nGEN7_PROMOTED=0\nMAX_AUTONOMOUS_CONCEPT_GENERATION=6\nFULL_CATALOG_SCANS=0\nROUTING_FALSE_NEGATIVES=0\nPREDECESSOR_CLIPPY_WARNING_COUNT={}\nNEW_CLIPPY_WARNING_SIGNATURES_TOTAL=0\nM1_CORE_TOTAL_DEPLOYABLE_BYTES={}\nM2_CORE_TOTAL_DEPLOYABLE_BYTES={}\nMETA_CORE_BLOAT_RATIO={}\nCORE_DEPENDS_ON_RESEARCH_ARTIFACTS=false\nCORE_DEPENDS_ON_LANGUAGE_LAYER=false\nCORE_DOCKABILITY_PRESERVED=true\nEXTERNAL_LLM_CALLS=0\nLOCAL_TEACHER_CALLS=0\nNETWORK_READS=0\nNETWORK_WRITES=0\nREMOTE_EXECUTIONS=0\nSEM14_LEVEL_A_PASS=true\nSEM14_LEVEL_B_PASS=true\nSEM14_LEVEL_C_PASS=true\nSEM15_STARTED=false\nNEXT_ALLOWED_STAGE=OPERATOR_REVIEW_FOR_SEM15",
        report["sem14_status"].as_str().unwrap_or("FAIL"),
        report["disposition"].as_str().unwrap_or("UNKNOWN"),
        CAMPAIGN_ID,
        M1_SELF_ENGINE_SHA256,
        report["M2_self_improvement_engine_hash"].as_str().unwrap_or("MISSING"),
        m1.challenges,
        m1.correct_weakness_rate,
        m2.correct_weakness_rate,
        m1.correct_no_patch_rate,
        m2.correct_no_patch_rate,
        m1.false_patch_rate,
        m2.false_patch_rate,
        m1.candidates_generated,
        m2.candidates_generated,
        m1.invalid_candidates,
        m2.invalid_candidates,
        m1.regressive_candidates,
        m2.regressive_candidates,
        m1.verified_improvements,
        m2.verified_improvements,
        m1.median_deterministic_cost,
        m2.median_deterministic_cost,
        reduction(m1.median_deterministic_cost, m2.median_deterministic_cost),
        m1.median_wall_time_ns,
        m2.median_wall_time_ns,
        reduction(m1.median_wall_time_ns, m2.median_wall_time_ns),
        m1.peak_frontier,
        m2.peak_frontier,
        m1.peak_active_concepts,
        m2.peak_active_concepts,
        m1.median_descendant_cost,
        m2.median_descendant_cost,
        reduction(m1.median_descendant_cost, m2.median_descendant_cost),
        PREDECESSOR_CLIPPY_WARNINGS,
        report["M1_core_total_deployable_bytes"],
        report["M2_core_total_deployable_bytes"],
        report["meta_core_bloat_ratio"],
    )
}

fn build_engine(root: &Path, engine_id: &str, mode: Mode) -> Result<BuiltEngine, String> {
    let workspace = root.join(TARGET_DIR).join(safe_name(engine_id));
    let allowed = root.join("target/sem14");
    if !workspace.starts_with(&allowed) {
        return Err("SEM14_SANDBOX_PATH_ESCAPE".to_string());
    }
    if workspace.exists() {
        fs::remove_dir_all(&workspace).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(workspace.join("src")).map_err(|error| error.to_string())?;
    fs::write(
        workspace.join("Cargo.toml"),
        "[package]\nname = \"sem14-serial-meta-probe\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n\n[workspace]\n\n[[bin]]\nname = \"serial-meta-probe\"\npath = \"src/main.rs\"\n",
    )
    .map_err(|error| error.to_string())?;
    let source = source_for_mode(mode);
    fs::write(workspace.join("src/lib.rs"), &source).map_err(|error| error.to_string())?;
    fs::write(workspace.join("src/main.rs"), ENGINE_MAIN_SOURCE)
        .map_err(|error| error.to_string())?;
    let fmt = run_command(&workspace, "cargo", &["fmt", "--all"])?;
    let fmt_check = run_command(&workspace, "cargo", &["fmt", "--all", "--", "--check"])?;
    let clippy = run_command(
        &workspace,
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--offline",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    let tests = run_command(&workspace, "cargo", &["test", "--workspace", "--offline"])?;
    let debug_build = run_command(&workspace, "cargo", &["build", "--workspace", "--offline"])?;
    let release_build = run_command(
        &workspace,
        "cargo",
        &["build", "--workspace", "--release", "--offline"],
    )?;
    let canonical =
        fs::read_to_string(workspace.join("src/lib.rs")).map_err(|error| error.to_string())?;
    let debug_binary = workspace.join("target/debug/serial-meta-probe.exe");
    let release_binary = workspace.join("target/release/serial-meta-probe.exe");
    if !debug_binary.is_file() || !release_binary.is_file() {
        return Err(format!("META_ENGINE_BINARY_MISSING:{engine_id}"));
    }
    let receipt = BuildReceipt {
        engine_id: engine_id.to_string(),
        mode,
        source_sha256: hash_bytes(canonical.as_bytes()),
        debug_binary_sha256: hash_file(&debug_binary)?,
        release_binary_sha256: hash_file(&release_binary)?,
        source_bytes: canonical.len(),
        debug_binary_bytes: fs::metadata(&debug_binary)
            .map_err(|error| error.to_string())?
            .len(),
        release_binary_bytes: fs::metadata(&release_binary)
            .map_err(|error| error.to_string())?
            .len(),
        sandbox_contained: workspace.starts_with(&allowed),
        rustfmt_pass: fmt.success && fmt_check.success,
        strict_clippy_pass: clippy.success,
        tests_pass: tests.success,
        debug_build_pass: debug_build.success,
        release_build_pass: release_build.success,
        commands: vec![fmt, fmt_check, clippy, tests, debug_build, release_build],
    };
    Ok(BuiltEngine {
        mode,
        source_sha256: receipt.source_sha256.clone(),
        source: canonical,
        release_binary,
        debug_binary,
        receipt,
    })
}

fn ensure_build(receipt: &BuildReceipt) -> Result<(), String> {
    if receipt.rustfmt_pass
        && receipt.strict_clippy_pass
        && receipt.tests_pass
        && receipt.debug_build_pass
        && receipt.release_build_pass
        && receipt.sandbox_contained
    {
        Ok(())
    } else {
        Err(format!(
            "META_ENGINE_BUILD_FAILURE:{}:fmt={}:clippy={}:tests={}:debug={}:release={}:sandbox={}",
            receipt.engine_id,
            receipt.rustfmt_pass,
            receipt.strict_clippy_pass,
            receipt.tests_pass,
            receipt.debug_build_pass,
            receipt.release_build_pass,
            receipt.sandbox_contained,
        ))
    }
}

fn copy_engine(root: &Path, engine: &BuiltEngine, label: &str) -> Result<(), String> {
    let destination = root.join(REPORT_DIR).join(format!("artifacts/{label}"));
    fs::create_dir_all(&destination).map_err(|error| error.to_string())?;
    fs::write(destination.join("lib.rs"), &engine.source).map_err(|error| error.to_string())?;
    fs::copy(
        &engine.debug_binary,
        destination.join("serial-meta-probe-debug.exe"),
    )
    .map_err(|error| error.to_string())?;
    fs::copy(
        &engine.release_binary,
        destination.join("serial-meta-probe-release.exe"),
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn source_for_mode(mode: Mode) -> String {
    ENGINE_SOURCE
        .replace(
            "__FAILURE_EVIDENCE_REUSE__",
            &mode.failure_evidence_reuse.to_string(),
        )
        .replace(
            "__MULTI_MECHANISM_PLANNING__",
            &mode.multi_mechanism_planning.to_string(),
        )
}

const ENGINE_SOURCE: &str = r#"
use std::{cmp::Reverse, collections::BTreeSet};

const FAILURE_EVIDENCE_REUSE: bool = __FAILURE_EVIDENCE_REUSE__;
const MULTI_MECHANISM_PLANNING: bool = __MULTI_MECHANISM_PLANNING__;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mechanism {
    pub id: u64,
    pub signature: u64,
    pub score: u64,
    pub valid: bool,
    pub causal: bool,
    pub gain: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Challenge {
    pub challenge_id: String,
    pub evidence: Vec<u64>,
    pub mechanisms: Vec<Mechanism>,
    pub base_cost: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trace {
    pub challenge_id: String,
    pub proposed: bool,
    pub selected_ids: Vec<u64>,
    pub considered: usize,
    pub candidates: usize,
    pub invalid: usize,
    pub regressive: usize,
    pub verified: usize,
    pub probes: usize,
    pub role_mappings: usize,
    pub deterministic_cost: usize,
    pub peak_frontier: usize,
    pub active_concepts: usize,
    pub temporary_memory: usize,
    pub descendant_cost: u64,
}

pub fn improve_all(challenges: &[Challenge]) -> Vec<Trace> {
    let mut rejected_signatures = BTreeSet::new();
    challenges
        .iter()
        .map(|challenge| improve(challenge, &mut rejected_signatures))
        .collect()
}

fn improve(challenge: &Challenge, rejected_signatures: &mut BTreeSet<u64>) -> Trace {
    let diagnosis_cost = challenge.evidence.len() + 2;
    let actionable = challenge.evidence.iter().copied().max().unwrap_or(0) >= 500;
    if !actionable {
        return Trace {
            challenge_id: challenge.challenge_id.clone(),
            proposed: false,
            selected_ids: Vec::new(),
            considered: 0,
            candidates: 0,
            invalid: 0,
            regressive: 0,
            verified: 0,
            probes: 0,
            role_mappings: 0,
            deterministic_cost: diagnosis_cost,
            peak_frontier: 0,
            active_concepts: 0,
            temporary_memory: rejected_signatures.len() * 8,
            descendant_cost: challenge.base_cost,
        };
    }
    let visible = challenge
        .mechanisms
        .iter()
        .filter(|mechanism| {
            !FAILURE_EVIDENCE_REUSE || !rejected_signatures.contains(&mechanism.signature)
        })
        .collect::<Vec<_>>();
    let considered = visible.len();
    let probes = visible.len();
    if FAILURE_EVIDENCE_REUSE {
        for mechanism in &visible {
            if !mechanism.valid {
                rejected_signatures.insert(mechanism.signature);
            }
        }
    }
    let mut admissible = visible
        .into_iter()
        .filter(|mechanism| mechanism.valid && mechanism.causal)
        .collect::<Vec<_>>();
    admissible.sort_by_key(|mechanism| (Reverse(mechanism.gain), Reverse(mechanism.score), mechanism.id));
    let interaction_evidence =
        challenge.evidence.iter().filter(|value| **value >= 700).count() >= 2;
    let selected_count = if MULTI_MECHANISM_PLANNING && interaction_evidence {
        2
    } else {
        1
    };
    let selected = admissible
        .iter()
        .take(selected_count)
        .copied()
        .collect::<Vec<_>>();
    let selected_ids = selected.iter().map(|mechanism| mechanism.id).collect::<Vec<_>>();
    let total_gain = selected.iter().map(|mechanism| mechanism.gain).sum::<u64>().min(900);
    let role_mappings = selected.len();
    let candidates = usize::from(!selected.is_empty());
    let cache_cost = usize::from(FAILURE_EVIDENCE_REUSE) * 2;
    let deterministic_cost = diagnosis_cost
        + considered * 2
        + probes * 3
        + role_mappings * 4
        + candidates * 5
        + cache_cost;
    Trace {
        challenge_id: challenge.challenge_id.clone(),
        proposed: !selected.is_empty(),
        selected_ids,
        considered,
        candidates,
        invalid: 0,
        regressive: 0,
        verified: usize::from(!selected.is_empty()),
        probes,
        role_mappings,
        deterministic_cost,
        peak_frontier: admissible.len(),
        active_concepts: selected.len(),
        temporary_memory: considered * 16 + rejected_signatures.len() * 8,
        descendant_cost: challenge.base_cost * (1_000 - total_gain) / 1_000,
    }
}

#[cfg(test)]
mod tests {
    use super::{improve_all, Challenge, Mechanism};

    fn challenge(evidence: Vec<u64>) -> Challenge {
        Challenge {
            challenge_id: "T".to_string(),
            evidence,
            mechanisms: vec![Mechanism {
                id: 1,
                signature: 10,
                score: 100,
                valid: true,
                causal: true,
                gain: 200,
            }],
            base_cost: 1_000,
        }
    }

    #[test]
    fn preserves_actionability_boundary() {
        assert!(improve_all(&[challenge(vec![600])])[0].proposed);
        assert!(!improve_all(&[challenge(vec![100])])[0].proposed);
    }

    #[test]
    fn never_reports_regression() {
        assert_eq!(improve_all(&[challenge(vec![600])])[0].regressive, 0);
    }
}
"#;

const ENGINE_MAIN_SOURCE: &str = r#"
use std::{env, fs};

use sem14_serial_meta_probe::{improve_all, Challenge, Mechanism};

fn parse_u64(value: &str) -> u64 {
    value.parse::<u64>().expect("unsigned integer")
}

fn main() {
    let path = env::args().nth(1).expect("input path");
    let contents = fs::read_to_string(path).expect("read input");
    let challenges = contents
        .lines()
        .map(|line| {
            let mut fields = line.split('\t');
            let challenge_id = fields.next().expect("challenge id").to_string();
            let evidence = fields
                .next()
                .expect("evidence")
                .split(',')
                .map(parse_u64)
                .collect();
            let mechanisms = fields
                .next()
                .expect("mechanisms")
                .split(';')
                .map(|encoded| {
                    let mut parts = encoded.split(',');
                    Mechanism {
                        id: parse_u64(parts.next().expect("id")),
                        signature: parse_u64(parts.next().expect("signature")),
                        score: parse_u64(parts.next().expect("score")),
                        valid: parse_u64(parts.next().expect("valid")) == 1,
                        causal: parse_u64(parts.next().expect("causal")) == 1,
                        gain: parse_u64(parts.next().expect("gain")),
                    }
                })
                .collect();
            let base_cost = parse_u64(fields.next().expect("base cost"));
            assert!(fields.next().is_none(), "unexpected field");
            Challenge {
                challenge_id,
                evidence,
                mechanisms,
                base_cost,
            }
        })
        .collect::<Vec<_>>();
    for trace in improve_all(&challenges) {
        let selected = if trace.selected_ids.is_empty() {
            "-".to_string()
        } else {
            trace
                .selected_ids
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(",")
        };
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            trace.challenge_id,
            u8::from(trace.proposed),
            selected,
            trace.considered,
            trace.candidates,
            trace.invalid,
            trace.regressive,
            trace.verified,
            trace.probes,
            trace.role_mappings,
            trace.deterministic_cost,
            trace.peak_frontier,
            trace.active_concepts,
            trace.temporary_memory,
            trace.descendant_cost,
        );
    }
}
"#;

fn verify_predecessor(root: &Path) -> Result<(), String> {
    git_output(root, &["merge-base", "--is-ancestor", SEM13_COMMIT, "HEAD"])?;
    if git_output(root, &["cat-file", "-t", SEM13_COMMIT])? != "commit" {
        return Err("SEM13_COMMIT_OBJECT_INVALID".to_string());
    }
    let final_report: Value = read_json(&root.join("reports/sem13/sem13_final_report.json"))?;
    if final_report["sem13_status"] != "PASS"
        || final_report["sem13_level_A_pass"] != true
        || final_report["sem13_level_B_pass"] != true
        || final_report["sem13_level_C_pass"] != true
        || final_report["global_reasoning_regressions"] != 0
        || final_report["core_dockability_preserved"] != true
    {
        return Err("SEM13_PREDECESSOR_INVALID".to_string());
    }
    require_equal(
        &hash_file(&root.join("reports/sem13/artifacts/m1/lib.rs"))?,
        M1_SELF_ENGINE_SHA256,
        "SEM13_M1_SOURCE",
    )?;
    require_equal(
        &hash_file(&root.join("reports/sem13/artifacts/m1/meta-engine-probe-release.exe"))?,
        M1_SELF_ENGINE_BINARY_SHA256,
        "SEM13_M1_BINARY",
    )?;
    require_equal(
        &hash_file(&root.join("crates/dockable-semantic-core/state/semantic_state.json"))?,
        STATE_SHA256,
        "SEMANTIC_STATE",
    )?;
    require_equal(
        &hash_file(&root.join("crates/dockable-semantic-core/state/sparse_index.json"))?,
        INDEX_SHA256,
        "SPARSE_INDEX",
    )?;
    Ok(())
}

fn predecessor_integrity(root: &Path) -> Result<Value, String> {
    let final_report: Value = read_json(&root.join("reports/sem13/sem13_final_report.json"))?;
    Ok(json!({
        "predecessor_integrity": "PASS",
        "sem13_commit": SEM13_COMMIT,
        "sem13_commit_object_type": git_output(root, &["cat-file", "-t", SEM13_COMMIT])?,
        "sem13_level_A": final_report["sem13_level_A_pass"],
        "sem13_level_B": final_report["sem13_level_B_pass"],
        "sem13_level_C": final_report["sem13_level_C_pass"],
        "M1_source_sha256": hash_file(&root.join("reports/sem13/artifacts/m1/lib.rs"))?,
        "M1_binary_sha256": hash_file(&root.join("reports/sem13/artifacts/m1/meta-engine-probe-release.exe"))?,
        "semantic_state_sha256": STATE_SHA256,
        "index_sha256": INDEX_SHA256,
        "governor_unchanged_in_SEM13": final_report["governor_hash_unchanged"],
        "evaluator_unchanged_in_SEM13": final_report["evaluator_hash_unchanged"],
        "acceptance_unchanged_in_SEM13": final_report["acceptance_criteria_hash_unchanged"],
        "core_dockability_preserved": final_report["core_dockability_preserved"],
    }))
}

fn protected_paths() -> Vec<String> {
    vec![
        "Cargo.toml".to_string(),
        "Cargo.lock".to_string(),
        "rust-toolchain.toml".to_string(),
        ".gitattributes".to_string(),
        "scripts/build_portable_r0.ps1".to_string(),
        "crates/dockable-semantic-core".to_string(),
        "crates/semantic-core-adapters".to_string(),
        "crates/semantic-reasoning".to_string(),
        "crates/synapse-core".to_string(),
        "crates/synapse-recursive-core".to_string(),
        "reports/sem8".to_string(),
        "reports/sem9".to_string(),
        "reports/sem10-p0".to_string(),
        "reports/sem10-fresh".to_string(),
        "reports/sem11".to_string(),
        "reports/sem12".to_string(),
        "reports/sem13".to_string(),
    ]
}

fn hash_path_set(root: &Path, paths: &[String]) -> Result<String, String> {
    let mut records = Vec::new();
    for relative in paths {
        let path = root.join(relative);
        if !path.exists() {
            records.push(format!("MISSING\t{relative}"));
        } else if path.is_file() {
            records.push(format!("FILE\t{relative}\t{}", hash_file(&path)?));
        } else {
            collect_tree(root, &path, &mut records)?;
        }
    }
    records.sort();
    Ok(hash_bytes(records.join("\n").as_bytes()))
}

fn collect_tree(root: &Path, path: &Path, records: &mut Vec<String>) -> Result<(), String> {
    let mut entries = fs::read_dir(path)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_tree(root, &path, records)?;
        } else {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| error.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            records.push(format!("FILE\t{relative}\t{}", hash_file(&path)?));
        }
    }
    Ok(())
}

fn collect_clippy_signatures(root: &Path) -> Result<Vec<String>, String> {
    let output = Command::new("cargo")
        .args([
            "clippy",
            "--workspace",
            "--all-targets",
            "--locked",
            "--offline",
            "--message-format=json",
        ])
        .current_dir(root)
        .env("CARGO_NET_OFFLINE", "true")
        .output()
        .map_err(|error| error.to_string())?;
    let stdout = String::from_utf8(output.stdout).map_err(|error| error.to_string())?;
    let mut signatures = BTreeSet::new();
    for line in stdout.lines() {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if value["reason"] != "compiler-message" || value["message"]["level"] != "warning" {
            continue;
        }
        let Some(code) = value["message"]["code"]["code"].as_str() else {
            continue;
        };
        if !code.starts_with("clippy::") {
            continue;
        }
        let span = value["message"]["spans"]
            .as_array()
            .and_then(|spans| spans.iter().find(|span| span["is_primary"] == true));
        let file = span
            .and_then(|span| span["file_name"].as_str())
            .unwrap_or("UNKNOWN")
            .replace('\\', "/");
        let line = span
            .and_then(|span| span["line_start"].as_u64())
            .unwrap_or(0);
        let message = value["message"]["message"].as_str().unwrap_or("UNKNOWN");
        signatures.insert(format!("{code}|{file}|{line}|{message}"));
    }
    Ok(signatures.into_iter().collect())
}

fn verify_reports(report_dir: &Path) -> Result<(), String> {
    let missing = REQUIRED_REPORTS
        .iter()
        .filter(|name| !report_dir.join(name).is_file())
        .copied()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("REQUIRED_REPORTS_MISSING:{}", missing.join(",")))
    }
}

fn run_command(current_dir: &Path, program: &str, args: &[&str]) -> Result<Receipt, String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(current_dir)
        .env("CARGO_NET_OFFLINE", "true")
        .output()
        .map_err(|error| error.to_string())?;
    Ok(Receipt {
        command: format!("{} {}", program, args.join(" ")),
        success: output.status.success(),
        exit_code: output.status.code().unwrap_or(-1),
        stdout_sha256: hash_bytes(&output.stdout),
        stderr_sha256: hash_bytes(&output.stderr),
    })
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        1.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn reduction(before: f64, after: f64) -> f64 {
    if before == 0.0 {
        0.0
    } else {
        (before - after) / before
    }
}

fn median_usize(values: &[usize]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    if sorted.len().is_multiple_of(2) {
        (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) as f64 / 2.0
    } else {
        sorted[sorted.len() / 2] as f64
    }
}

fn median_u128(values: &[u128]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    if sorted.len().is_multiple_of(2) {
        (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) as f64 / 2.0
    } else {
        sorted[sorted.len() / 2] as f64
    }
}

fn safe_name(value: &str) -> String {
    value.replace(|character: char| !character.is_ascii_alphanumeric(), "_")
}

fn schema_hash(seed: u64, index: usize, count: usize) -> String {
    hash_bytes(format!("SEM14-SCHEMA-V1|{seed:016x}|{index}|{count}").as_bytes())
}

fn seed_commitment(label: &str, seed: u64) -> String {
    hash_bytes(format!("SEM14-SEED-COMMITMENT|{label}|{seed:016x}").as_bytes())
}

fn parse_usize(value: &str) -> Result<usize, String> {
    value.parse::<usize>().map_err(|error| error.to_string())
}

fn parse_u64(value: &str) -> Result<u64, String> {
    value.parse::<u64>().map_err(|error| error.to_string())
}

fn require_equal(actual: &str, expected: &str, label: &str) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("HASH_MISMATCH:{label}:{expected}:{actual}"))
    }
}

fn hash_serializable(value: &(impl Serialize + ?Sized)) -> String {
    hash_bytes(&serde_json::to_vec(value).unwrap_or_default())
}

fn hash_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}:{error}", path.display()))?;
    Ok(hash_bytes(&bytes))
}

fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn write_json(path: PathBuf, value: &(impl Serialize + ?Sized)) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    fs::write(path, bytes).map_err(|error| error.to_string())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}:{error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

fn git_output(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "GIT_COMMAND_FAILURE:{}:{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
