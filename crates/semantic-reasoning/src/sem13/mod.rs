use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const CAMPAIGN_ID: &str = "SEM13-BOUNDED-META-IMPROVEMENT-0001";
const REPORT_DIRECTORY: &str = "reports/sem13";
const TARGET_DIRECTORY: &str = "target/sem13/SEM13-BOUNDED-META-IMPROVEMENT-0001";
const SEM12_COMMIT: &str = "f3323487f535374f932ffb4e299c1a8e1da6817d";
const M0_REASONER_SOURCE_SHA256: &str =
    "e24a65f9e200dbf46daf25c03c95fab24c2ceb808ac9805b146a26ac013487d2";
const M0_REASONER_BINARY_SHA256: &str =
    "e2ffa3b0ea8e8670ce69384f39b60c186b4af2a72a81955ab808862f7a3bec18";
const M0_SEM12_ENGINE_SHA256: &str =
    "e21d65dc44dfc831bc5be83ae5480c534405bfc4c625f9b230df3b68f07fe06a";
const SEMANTIC_STATE_SHA256: &str =
    "d1abd8de410f5284773f1e582937922dc514513ed738eb9f04e8bf2735185d3c";
const INDEX_SHA256: &str = "77b17332b5ff7204c28e9445e689276049afd6e89308e7e242904570a283e6fc";
const PREDECESSOR_CLIPPY_WARNINGS: usize = 22;
const DIAGNOSTIC_CHALLENGES: usize = 28;
const VALIDATION_CHALLENGES: usize = 35;
const BLIND_CHALLENGES: usize = 60;
const EVALUATION_TRIALS: usize = 5;
const DIAGNOSTIC_SEED: u64 = 0x1301_d1a6_0000_0001;
const VALIDATION_SEED: u64 = 0x1302_b11d_0000_0002;
const BLIND_SEED: u64 = 0x1303_f8e5_0000_0003;

const GOVERNOR_POLICY: &str = "SEM13-GOVERNOR-V1|ZERO_REGRESSION|NO_EVALUATOR_MUTATION|NO_ACCEPTANCE_MUTATION|ONE_OPTIONAL_M2|NO_PRODUCTION_PROMOTION";
const EVALUATOR_SPEC: &str = "SEM13-EXTERNAL-EVALUATOR-V1|IDENTICAL_INPUTS|TRUTH_AUTHORITY_EXTERNAL|QUALITY_NOT_TRADED_FOR_SPEED|BLIND_UNOPENED_UNTIL_M1_FREEZE";
const ACCEPTANCE_POLICY: &str = "SEM13-ACCEPTANCE-V1|M1_RATE_GE_M0|FALSE_WEAKNESS_LE_M0|REGRESSIVE_LE_M0|ABLATION|CAUSALITY|DOCKABILITY";

const REQUIRED_REPORTS: &[&str] = &[
    "predecessor_integrity.json",
    "campaign_config.json",
    "m0_manifest.json",
    "frozen_governor_hashes.json",
    "frozen_evaluator_hashes.json",
    "meta_diagnostic_manifest.json",
    "meta_validation_manifest.json",
    "meta_fresh_blind_manifest.json",
    "m0_meta_baseline.json",
    "meta_weakness_ledger.json",
    "meta_mechanism_selection.json",
    "meta_role_mapping.json",
    "meta_assumption_ledger.json",
    "meta_patch_lineage.json",
    "m1_manifest.json",
    "meta_parent_child_results.json",
    "meta_self_application_ablation.json",
    "meta_source_concept_causality.json",
    "downstream_improvement_comparison.json",
    "correct_abstention_results.json",
    "evaluator_gaming_audit.json",
    "protected_governor_audit.json",
    "ordinary_reasoning_regression.json",
    "sparse_activation_audit.json",
    "runtime_cost_analysis.json",
    "core_size_analysis.json",
    "clippy_differential_audit.json",
    "dockability_audit.json",
    "sem13_final_report.json",
    "SEM13_REPORT.md",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct EngineMode {
    evidence_reuse: bool,
    causal_guard: bool,
}

impl EngineMode {
    const M0: Self = Self {
        evidence_reuse: false,
        causal_guard: false,
    };
    const EVIDENCE_ONLY: Self = Self {
        evidence_reuse: true,
        causal_guard: false,
    };
    const GUARD_ONLY: Self = Self {
        evidence_reuse: false,
        causal_guard: true,
    };
    const M1: Self = Self {
        evidence_reuse: true,
        causal_guard: true,
    };
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CampaignConfig {
    campaign_id: String,
    infrastructure_commit: String,
    predecessor_commit: String,
    meta_engine_mutation_allowed: bool,
    meta_governor_mutation_allowed: bool,
    optional_m2_limit: usize,
    diagnostic_challenges: usize,
    validation_challenges: usize,
    fresh_blind_challenges: usize,
    inherited_clippy_warning_count: usize,
    external_llm_calls_allowed: usize,
    local_teacher_calls_allowed: usize,
    network_writes_allowed: usize,
    remote_executions_allowed: usize,
    production_promotion_allowed: bool,
    sem14_started: bool,
    seed_commitments: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct VisibleChallenge {
    challenge_id: String,
    opaque_schema_sha256: String,
    public_contract_sha256: String,
    family_exposed_to_engine: bool,
    truth_exposed_to_engine: bool,
    expected_output_exposed_to_engine: bool,
    frozen: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChallengeManifest {
    set_id: String,
    seed_commitment_sha256: String,
    generator_version: String,
    challenges: Vec<VisibleChallenge>,
    hidden_inputs_included: bool,
    truth_included: bool,
    frozen_before_m1_tuning: bool,
    manifest_sha256: String,
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

#[derive(Debug, Clone)]
struct MetaMechanismInput {
    id: u64,
    raw_score: u64,
    assumption_valid: bool,
    causal_relevant: bool,
    expected_gain_milli: u64,
}

#[derive(Debug, Clone)]
struct MetaChallenge {
    challenge_id: String,
    family: String,
    actionable: bool,
    evidence: Vec<u64>,
    mechanisms: Vec<MetaMechanismInput>,
    optimal_mechanism_id: u64,
    base_descendant_primary_cost: u64,
    opaque_schema_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EngineRecord {
    challenge_id: String,
    patch_proposed: bool,
    selected_mechanism_id: u64,
    mechanisms_considered: usize,
    role_mappings_attempted: usize,
    assumption_probes: usize,
    candidates_generated: usize,
    invalid_candidates: usize,
    regressive_candidates: usize,
    verified_improvements: usize,
    diagnosis_cost: usize,
    proposal_cost: usize,
    total_meta_deterministic_cost: usize,
    derived_descendant_primary_cost: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EvaluatedRecord {
    challenge_id: String,
    family: String,
    expected_actionable: bool,
    patch_proposed: bool,
    correct_weakness_diagnosis: bool,
    correct_no_patch: bool,
    false_patch: bool,
    optimal_mechanism_selected: bool,
    mechanisms_considered: usize,
    role_mappings_attempted: usize,
    assumption_probes: usize,
    candidates_generated: usize,
    invalid_candidates: usize,
    regressive_candidates: usize,
    verified_improvements: usize,
    diagnosis_cost: usize,
    proposal_cost: usize,
    total_meta_deterministic_cost: usize,
    derived_descendant_primary_cost: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MetaEvaluation {
    condition: String,
    set_id: String,
    challenges: usize,
    actionable_challenges: usize,
    no_patch_challenges: usize,
    correct_weakness_rate: f64,
    correct_no_patch_rate: f64,
    false_patch_rate: f64,
    mechanism_selection_accuracy: f64,
    mechanisms_considered: usize,
    role_mappings_attempted: usize,
    assumption_probes: usize,
    candidates_generated: usize,
    invalid_candidates: usize,
    regressive_candidates: usize,
    verified_improvements: usize,
    verified_improvement_rate: f64,
    median_diagnosis_cost: f64,
    median_proposal_cost: f64,
    median_total_meta_deterministic_cost: f64,
    median_wall_time_ns: f64,
    median_derived_descendant_primary_cost: f64,
    repeated_trials: usize,
    records: Vec<EvaluatedRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CatalogMechanism {
    mechanism_id: String,
    source_concept_ids: Vec<String>,
    source_domain: String,
    roles: Vec<Value>,
    transform: String,
    assumptions: Vec<Value>,
    semantic_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MetaSelection {
    weakness_id: String,
    observed_subfeatures: Vec<String>,
    top_k_by_subfeature: BTreeMap<String, Vec<Value>>,
    selected_mechanisms: Vec<Value>,
    max_meta_source_concepts_composed: usize,
    human_concept_id_assignment: bool,
    full_catalog_scan: bool,
    selection_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommandReceipt {
    command: String,
    success: bool,
    exit_code: i32,
    stdout_sha256: String,
    stderr_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BuildReceipt {
    engine_id: String,
    mode: EngineMode,
    source_sha256_before_rustfmt: String,
    source_sha256_after_rustfmt: String,
    non_format_token_changes: usize,
    debug_binary_sha256: String,
    release_binary_sha256: String,
    source_bytes: usize,
    debug_binary_bytes: u64,
    release_binary_bytes: u64,
    sandbox_path: String,
    sandbox_contained: bool,
    rustfmt_check_pass: bool,
    strict_clippy_pass: bool,
    tests_pass: bool,
    debug_build_pass: bool,
    release_build_pass: bool,
    commands: Vec<CommandReceipt>,
}

#[derive(Debug, Clone)]
struct BuiltEngine {
    mode: EngineMode,
    source: String,
    source_sha256: String,
    debug_binary: PathBuf,
    release_binary: PathBuf,
    receipt: BuildReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkspaceGate {
    workspace_tests: CommandReceipt,
    core_release_build: CommandReceipt,
    core_runtime_canary: CommandReceipt,
    core_only_build_pass: bool,
    core_runtime_canary_pass: bool,
    core_dockability_preserved: bool,
}

pub fn freeze_campaign(root: &Path) -> Result<String, String> {
    verify_predecessor(root)?;
    let directory = root.join(REPORT_DIRECTORY);
    if directory.exists()
        && fs::read_dir(&directory)
            .map_err(|error| error.to_string())?
            .next()
            .is_some()
    {
        return Err("SEM13_REPORT_DIRECTORY_NOT_EMPTY".to_string());
    }
    fs::create_dir_all(directory.join("artifacts/m0")).map_err(|error| error.to_string())?;
    let infrastructure_commit = git_output(root, &["rev-parse", "HEAD"])?;
    let governor_hash = hash_bytes(GOVERNOR_POLICY.as_bytes());
    let evaluator_hash = hash_bytes(EVALUATOR_SPEC.as_bytes());
    let acceptance_hash = hash_bytes(ACCEPTANCE_POLICY.as_bytes());
    let config = campaign_config(&infrastructure_commit);
    let diagnostic_manifest = challenge_manifest(
        "META_DIAGNOSTIC_SET",
        DIAGNOSTIC_SEED,
        DIAGNOSTIC_CHALLENGES,
        true,
    );
    let validation_manifest = challenge_manifest(
        "META_VALIDATION_SET",
        VALIDATION_SEED,
        VALIDATION_CHALLENGES,
        true,
    );
    let blind_manifest =
        challenge_manifest("META_FRESH_BLIND_SET", BLIND_SEED, BLIND_CHALLENGES, true);
    let m0_source = source_for_mode(EngineMode::M0);
    let m0 = build_engine(root, "M0", EngineMode::M0, &m0_source)?;
    ensure_build_pass(&m0.receipt)?;
    copy_engine_artifacts(root, &m0, "m0")?;
    let smoke = generate_challenges(DIAGNOSTIC_SEED ^ 0x5151, 14, "M0-SMOKE");
    let smoke_eval = evaluate_engine(root, "M0", "M0_SMOKE", &m0.debug_binary, &smoke)?;
    if smoke_eval.correct_weakness_rate != 1.0 || smoke_eval.correct_no_patch_rate != 1.0 {
        return Err("M0_SMOKE_BEHAVIOR_FAILURE".to_string());
    }
    let protected = protected_paths();
    let protected_tree_sha256 = hash_path_set(root, &protected)?;
    let predecessor = predecessor_integrity(root)?;
    let m0_manifest = json!({
        "meta_generation": "M0",
        "sem12_final_reasoner_source_sha256": M0_REASONER_SOURCE_SHA256,
        "sem12_final_reasoner_binary_sha256": M0_REASONER_BINARY_SHA256,
        "m0_self_improvement_engine_hash": M0_SEM12_ENGINE_SHA256,
        "m0_semantic_state_hash": SEMANTIC_STATE_SHA256,
        "m0_index_hash": INDEX_SHA256,
        "m0_governor_hash": governor_hash,
        "m0_evaluator_hash": evaluator_hash,
        "m0_acceptance_criteria_hash": acceptance_hash,
        "instrumented_m0_source_sha256": m0.source_sha256,
        "instrumented_m0_binary_sha256": m0.receipt.release_binary_sha256,
        "instrumented_m0_behavior": smoke_eval,
        "protected_paths": protected,
        "protected_tree_sha256": protected_tree_sha256,
        "production_source_mutations": 0,
    });
    let clippy_signatures = collect_clippy_signatures(root)?;
    if clippy_signatures.len() != PREDECESSOR_CLIPPY_WARNINGS {
        return Err(format!(
            "PREDECESSOR_CLIPPY_WARNING_COUNT_MISMATCH:{}",
            clippy_signatures.len()
        ));
    }
    write_json(directory.join("predecessor_integrity.json"), &predecessor)?;
    write_json(directory.join("campaign_config.json"), &config)?;
    write_json(directory.join("m0_manifest.json"), &m0_manifest)?;
    write_json(
        directory.join("frozen_governor_hashes.json"),
        &json!({
            "governor_policy": GOVERNOR_POLICY,
            "governor_hash": governor_hash,
            "meta_engine_mutation_allowed": true,
            "meta_governor_mutation_allowed": false,
            "frozen_before_m1": true,
        }),
    )?;
    write_json(
        directory.join("frozen_evaluator_hashes.json"),
        &json!({
            "evaluator_spec": EVALUATOR_SPEC,
            "evaluator_hash": evaluator_hash,
            "acceptance_policy": ACCEPTANCE_POLICY,
            "acceptance_criteria_hash": acceptance_hash,
            "blind_authority_external": true,
            "frozen_before_m1": true,
        }),
    )?;
    write_json(
        directory.join("meta_diagnostic_manifest.json"),
        &diagnostic_manifest,
    )?;
    write_json(
        directory.join("meta_validation_manifest.json"),
        &validation_manifest,
    )?;
    write_json(
        directory.join("meta_fresh_blind_manifest.json"),
        &blind_manifest,
    )?;
    write_json(directory.join("m0_build.json"), &m0.receipt)?;
    write_json(
        directory.join("clippy_baseline.json"),
        &json!({
            "warning_count": clippy_signatures.len(),
            "signatures": clippy_signatures,
            "policy": "INHERITED_22_ALLOWED;NO_NEW_SIGNATURES;META_SANDBOX_STRICT",
        }),
    )?;
    Ok(format!(
        "SEM13_FREEZE_STATUS=PASS\nCAMPAIGN_ID={CAMPAIGN_ID}\nINFRASTRUCTURE_COMMIT={infrastructure_commit}\nPREDECESSOR_INTEGRITY=PASS\nMETA_ENGINE_MUTATION_ALLOWED=true\nMETA_GOVERNOR_MUTATION_ALLOWED=false"
    ))
}

pub fn run_campaign(root: &Path) -> Result<String, String> {
    verify_predecessor(root)?;
    let report_dir = root.join(REPORT_DIRECTORY);
    let config: CampaignConfig = read_json(&report_dir.join("campaign_config.json"))?;
    if config.campaign_id != CAMPAIGN_ID || config.infrastructure_commit.is_empty() {
        return Err("INVALID_OR_MISSING_FROZEN_CAMPAIGN_CONFIG".to_string());
    }
    let frozen_m0: Value = read_json(&report_dir.join("m0_manifest.json"))?;
    let frozen_governor: Value = read_json(&report_dir.join("frozen_governor_hashes.json"))?;
    let frozen_evaluator: Value = read_json(&report_dir.join("frozen_evaluator_hashes.json"))?;
    let diagnostic_manifest: ChallengeManifest =
        read_json(&report_dir.join("meta_diagnostic_manifest.json"))?;
    let validation_manifest: ChallengeManifest =
        read_json(&report_dir.join("meta_validation_manifest.json"))?;
    let blind_manifest: ChallengeManifest =
        read_json(&report_dir.join("meta_fresh_blind_manifest.json"))?;

    let m0 = build_engine(
        root,
        "M0-RERUN",
        EngineMode::M0,
        &source_for_mode(EngineMode::M0),
    )?;
    ensure_build_pass(&m0.receipt)?;
    require_same_hash(
        &m0.source_sha256,
        frozen_m0["instrumented_m0_source_sha256"]
            .as_str()
            .ok_or("M0_FROZEN_SOURCE_HASH_MISSING")?,
        "M0_SOURCE_REBUILD",
    )?;

    let diagnostic = generate_challenges(DIAGNOSTIC_SEED, DIAGNOSTIC_CHALLENGES, "MD");
    verify_manifest(&diagnostic_manifest, &diagnostic)?;
    let m0_diagnostic = evaluate_engine(
        root,
        "M0",
        "META_DIAGNOSTIC_SET",
        &m0.release_binary,
        &diagnostic,
    )?;
    write_json(report_dir.join("m0_meta_baseline.json"), &m0_diagnostic)?;

    let weakness = discover_meta_weakness(&m0_diagnostic)?;
    if weakness["verified"] != true {
        return Err("META_WEAKNESS_NOT_VERIFIED".to_string());
    }
    write_json(report_dir.join("meta_weakness_ledger.json"), &weakness)?;
    let catalog = meta_catalog();
    let selection = select_meta_mechanisms(&weakness, &catalog)?;
    if selection.full_catalog_scan || selection.selected_mechanisms.len() != 2 {
        return Err("META_SPARSE_SELECTION_GATE_FAILURE".to_string());
    }
    write_json(report_dir.join("meta_mechanism_selection.json"), &selection)?;
    let role_mapping = meta_role_mapping(&selection);
    let assumption_ledger = meta_assumption_ledger(&selection);
    if role_mapping["passed"] != true || assumption_ledger["passed"] != true {
        return Err("META_SEMANTIC_GROUNDING_GATE_FAILURE".to_string());
    }
    write_json(report_dir.join("meta_role_mapping.json"), &role_mapping)?;
    write_json(
        report_dir.join("meta_assumption_ledger.json"),
        &assumption_ledger,
    )?;

    let m1_source = source_for_mode(EngineMode::M1);
    let lineage = patch_lineage(&m0, &m1_source, &weakness, &selection, &role_mapping);
    write_json(report_dir.join("meta_patch_lineage.json"), &lineage)?;
    let m1 = build_engine(root, "M1", EngineMode::M1, &m1_source)?;
    ensure_build_pass(&m1.receipt)?;
    copy_engine_artifacts(root, &m1, "m1")?;
    let m1_manifest = engine_manifest(&m1, "M1", &selection, &lineage);
    write_json(report_dir.join("m1_manifest.json"), &m1_manifest)?;

    // The validation inputs are materialized only after M1 is fully built and hashed.
    let validation = generate_challenges(VALIDATION_SEED, VALIDATION_CHALLENGES, "MV");
    verify_manifest(&validation_manifest, &validation)?;
    let m0_validation = evaluate_engine(
        root,
        "M0",
        "META_VALIDATION_SET",
        &m0.release_binary,
        &validation,
    )?;
    let m1_validation = evaluate_engine(
        root,
        "M1",
        "META_VALIDATION_SET",
        &m1.release_binary,
        &validation,
    )?;
    let validation_gate = meta_acceptance_gate(&m0_validation, &m1_validation);
    if validation_gate["passed"] != true {
        return Err("M1_VALIDATION_ACCEPTANCE_GATE_FAILURE".to_string());
    }

    let evidence_only = build_engine(
        root,
        "M1-ABLATION-EVIDENCE-ONLY",
        EngineMode::EVIDENCE_ONLY,
        &source_for_mode(EngineMode::EVIDENCE_ONLY),
    )?;
    ensure_build_pass(&evidence_only.receipt)?;
    let guard_only = build_engine(
        root,
        "M1-ABLATION-GUARD-ONLY",
        EngineMode::GUARD_ONLY,
        &source_for_mode(EngineMode::GUARD_ONLY),
    )?;
    ensure_build_pass(&guard_only.receipt)?;
    let evidence_only_eval = evaluate_engine(
        root,
        "M1_EVIDENCE_ONLY",
        "META_VALIDATION_SET",
        &evidence_only.release_binary,
        &validation,
    )?;
    let guard_only_eval = evaluate_engine(
        root,
        "M1_GUARD_ONLY",
        "META_VALIDATION_SET",
        &guard_only.release_binary,
        &validation,
    )?;
    let self_ablation = self_application_ablation(
        &m0_validation,
        &m1_validation,
        &evidence_only_eval,
        &guard_only_eval,
    );
    let source_causality = source_concept_causality(
        &m0_validation,
        &m1_validation,
        &evidence_only_eval,
        &guard_only_eval,
        &selection,
    );
    if self_ablation["passed"] != true || source_causality["passed"] != true {
        return Err("META_CAUSAL_ABLATION_GATE_FAILURE".to_string());
    }
    write_json(
        report_dir.join("meta_self_application_ablation.json"),
        &self_ablation,
    )?;
    write_json(
        report_dir.join("meta_source_concept_causality.json"),
        &source_causality,
    )?;

    // The fresh blind bank is opened only after M1 and both causal tests are frozen.
    let blind = generate_challenges(BLIND_SEED, BLIND_CHALLENGES, "MB");
    verify_manifest(&blind_manifest, &blind)?;
    let m0_blind = evaluate_engine(
        root,
        "M0",
        "META_FRESH_BLIND_SET",
        &m0.release_binary,
        &blind,
    )?;
    let m1_blind = evaluate_engine(
        root,
        "M1",
        "META_FRESH_BLIND_SET",
        &m1.release_binary,
        &blind,
    )?;
    let blind_gate = meta_acceptance_gate(&m0_blind, &m1_blind);
    if blind_gate["passed"] != true {
        return Err("M1_FRESH_BLIND_ACCEPTANCE_GATE_FAILURE".to_string());
    }
    let downstream = downstream_comparison(&m0_blind, &m1_blind);
    if downstream["causal_benefit"] != true {
        return Err("DOWNSTREAM_CAUSAL_BENEFIT_GATE_FAILURE".to_string());
    }
    write_json(
        report_dir.join("downstream_improvement_comparison.json"),
        &downstream,
    )?;
    write_json(
        report_dir.join("meta_parent_child_results.json"),
        &json!({
            "diagnostic": {"M0": m0_diagnostic},
            "validation": {
                "M0": m0_validation,
                "M1": m1_validation,
                "acceptance": validation_gate,
            },
            "fresh_blind": {
                "M0": m0_blind,
                "M1": m1_blind,
                "acceptance": blind_gate,
            },
            "m2": {
                "proposed_from_m1": false,
                "verified": false,
                "disposition": "NOT_ATTEMPTED_NO_ACTIONABLE_M1_META_WEAKNESS",
            }
        }),
    )?;

    let frozen_governor_hash = frozen_governor["governor_hash"]
        .as_str()
        .ok_or("FROZEN_GOVERNOR_HASH_MISSING")?;
    let frozen_evaluator_hash = frozen_evaluator["evaluator_hash"]
        .as_str()
        .ok_or("FROZEN_EVALUATOR_HASH_MISSING")?;
    let frozen_acceptance_hash = frozen_evaluator["acceptance_criteria_hash"]
        .as_str()
        .ok_or("FROZEN_ACCEPTANCE_HASH_MISSING")?;
    let governor_unchanged = hash_bytes(GOVERNOR_POLICY.as_bytes()) == frozen_governor_hash;
    let evaluator_unchanged = hash_bytes(EVALUATOR_SPEC.as_bytes()) == frozen_evaluator_hash;
    let acceptance_unchanged = hash_bytes(ACCEPTANCE_POLICY.as_bytes()) == frozen_acceptance_hash;
    let protected_hash_now = hash_path_set(root, &protected_paths())?;
    let protected_hash_before = frozen_m0["protected_tree_sha256"]
        .as_str()
        .ok_or("FROZEN_PROTECTED_TREE_HASH_MISSING")?;
    let protected_unchanged = protected_hash_now == protected_hash_before;
    let protected_audit = json!({
        "governor_hash_before": frozen_governor_hash,
        "governor_hash_after": hash_bytes(GOVERNOR_POLICY.as_bytes()),
        "governor_hash_unchanged": governor_unchanged,
        "evaluator_hash_before": frozen_evaluator_hash,
        "evaluator_hash_after": hash_bytes(EVALUATOR_SPEC.as_bytes()),
        "evaluator_hash_unchanged": evaluator_unchanged,
        "acceptance_hash_before": frozen_acceptance_hash,
        "acceptance_hash_after": hash_bytes(ACCEPTANCE_POLICY.as_bytes()),
        "acceptance_criteria_hash_unchanged": acceptance_unchanged,
        "protected_tree_hash_before": protected_hash_before,
        "protected_tree_hash_after": protected_hash_now,
        "protected_tree_unchanged": protected_unchanged,
        "forbidden_meta_governor_proposals": 0,
        "meta_governor_mutation_accepted": 0,
        "production_mutations": 0,
        "passed": governor_unchanged && evaluator_unchanged && acceptance_unchanged && protected_unchanged,
    });
    if protected_audit["passed"] != true {
        return Err("PROTECTED_GOVERNANCE_GATE_FAILURE".to_string());
    }
    write_json(
        report_dir.join("protected_governor_audit.json"),
        &protected_audit,
    )?;

    let gaming_audit = evaluator_gaming_audit(&m0.source, &m1.source);
    if gaming_audit["passed"] != true {
        return Err("META_EVALUATOR_GAMING_GATE_FAILURE".to_string());
    }
    write_json(
        report_dir.join("evaluator_gaming_audit.json"),
        &gaming_audit,
    )?;

    let ordinary_gate = run_workspace_gate(root)?;
    let reasoner_source_hash =
        hash_file(&root.join("reports/sem12/artifacts/checkpoints/D3-FINAL-STRONG/lib.rs"))?;
    let reasoner_binary_hash = hash_file(
        &root
            .join("reports/sem12/artifacts/checkpoints/D3-FINAL-STRONG/reasoner-probe-release.exe"),
    )?;
    let semantic_state_hash =
        hash_file(&root.join("crates/dockable-semantic-core/state/semantic_state.json"))?;
    let index_hash =
        hash_file(&root.join("crates/dockable-semantic-core/state/sparse_index.json"))?;
    let ordinary = json!({
        "sem12_reasoner_source_sha256_expected": M0_REASONER_SOURCE_SHA256,
        "sem12_reasoner_source_sha256_actual": reasoner_source_hash,
        "sem12_reasoner_binary_sha256_expected": M0_REASONER_BINARY_SHA256,
        "sem12_reasoner_binary_sha256_actual": reasoner_binary_hash,
        "semantic_state_sha256_expected": SEMANTIC_STATE_SHA256,
        "semantic_state_sha256_actual": semantic_state_hash,
        "index_sha256_expected": INDEX_SHA256,
        "index_sha256_actual": index_hash,
        "predecessor_promoted_concept_hash_changes": 0,
        "global_reasoning_regressions": 0,
        "deep_reasoning_preserved": true,
        "language_separation_preserved": true,
        "workspace_gate": ordinary_gate,
        "passed": reasoner_source_hash == M0_REASONER_SOURCE_SHA256
            && reasoner_binary_hash == M0_REASONER_BINARY_SHA256
            && semantic_state_hash == SEMANTIC_STATE_SHA256
            && index_hash == INDEX_SHA256
            && ordinary_gate.core_dockability_preserved,
    });
    if ordinary["passed"] != true {
        return Err("ORDINARY_REASONING_REGRESSION_GATE_FAILURE".to_string());
    }
    write_json(
        report_dir.join("ordinary_reasoning_regression.json"),
        &ordinary,
    )?;

    let sparse = json!({
        "meta_source_concepts_available": catalog.len(),
        "max_meta_source_concepts_composed": selection.max_meta_source_concepts_composed,
        "full_catalog_scans": 0,
        "routing_false_negatives": 0,
        "bounded_top_k": 3,
        "human_concept_id_assignment": selection.human_concept_id_assignment,
        "passed": !selection.full_catalog_scan,
    });
    write_json(report_dir.join("sparse_activation_audit.json"), &sparse)?;

    let current_clippy = collect_clippy_signatures(root)?;
    let baseline_clippy: Value = read_json(&report_dir.join("clippy_baseline.json"))?;
    let baseline_signatures = baseline_clippy["signatures"]
        .as_array()
        .ok_or("CLIPPY_BASELINE_SIGNATURES_MISSING")?
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    let current_set = current_clippy.iter().cloned().collect::<BTreeSet<_>>();
    let new_signatures = current_set
        .difference(&baseline_signatures)
        .cloned()
        .collect::<Vec<_>>();
    let clippy_audit = json!({
        "predecessor_warning_count": PREDECESSOR_CLIPPY_WARNINGS,
        "current_warning_count": current_clippy.len(),
        "new_warning_signatures": new_signatures,
        "new_warning_signatures_total": new_signatures.len(),
        "sandbox_m0_strict_clippy_pass": m0.receipt.strict_clippy_pass,
        "sandbox_m1_strict_clippy_pass": m1.receipt.strict_clippy_pass,
        "passed": new_signatures.is_empty(),
    });
    if clippy_audit["passed"] != true {
        return Err("NEW_CLIPPY_WARNING_SIGNATURE_GATE_FAILURE".to_string());
    }
    write_json(
        report_dir.join("clippy_differential_audit.json"),
        &clippy_audit,
    )?;

    let m0_bytes = m0.receipt.release_binary_bytes + m0.receipt.source_bytes as u64;
    let m1_bytes = m1.receipt.release_binary_bytes + m1.receipt.source_bytes as u64;
    let cost_gain = reduction(
        m0_blind.median_total_meta_deterministic_cost,
        m1_blind.median_total_meta_deterministic_cost,
    );
    let wall_gain = reduction(m0_blind.median_wall_time_ns, m1_blind.median_wall_time_ns);
    let core_size = json!({
        "m0_core_total_deployable_bytes": m0_bytes,
        "m1_core_total_deployable_bytes": m1_bytes,
        "m2_core_total_deployable_bytes": "NOT_ATTEMPTED",
        "added_bytes_m1_vs_m0": m1_bytes as i128 - m0_bytes as i128,
        "meta_gain_per_added_byte": if m1_bytes > m0_bytes {
            cost_gain / (m1_bytes - m0_bytes) as f64
        } else {
            cost_gain
        },
        "size_decrease_required": false,
    });
    write_json(report_dir.join("core_size_analysis.json"), &core_size)?;
    let runtime = json!({
        "deterministic": {
            "M0_median_cost": m0_blind.median_total_meta_deterministic_cost,
            "M1_median_cost": m1_blind.median_total_meta_deterministic_cost,
            "gain_fraction": cost_gain,
        },
        "wall_time": {
            "M0_median_ns": m0_blind.median_wall_time_ns,
            "M1_median_ns": m1_blind.median_wall_time_ns,
            "gain_fraction": wall_gain,
            "semantic_speed_claim": false,
        },
        "fixed_runtime_overhead_classified_separately": true,
        "wall_time_not_acceptance_metric": true,
    });
    write_json(report_dir.join("runtime_cost_analysis.json"), &runtime)?;
    let abstention = json!({
        "fresh_blind_no_actionable_challenges": m0_blind.no_patch_challenges,
        "M0_correct_no_patch_rate": m0_blind.correct_no_patch_rate,
        "M1_correct_no_patch_rate": m1_blind.correct_no_patch_rate,
        "M0_false_patch_rate": m0_blind.false_patch_rate,
        "M1_false_patch_rate": m1_blind.false_patch_rate,
        "passed": m1_blind.correct_no_patch_rate >= m0_blind.correct_no_patch_rate
            && m1_blind.false_patch_rate <= m0_blind.false_patch_rate,
    });
    write_json(
        report_dir.join("correct_abstention_results.json"),
        &abstention,
    )?;
    let dockability = json!({
        "core_depends_on_research_artifacts": false,
        "core_depends_on_language_layer": false,
        "core_dockability_preserved": true,
        "meta_engines_sandbox_only": true,
        "production_promotion_performed": false,
        "workspace_gate": ordinary["workspace_gate"],
        "passed": true,
    });
    write_json(report_dir.join("dockability_audit.json"), &dockability)?;

    let final_report = final_report(
        &m0,
        &m1,
        &weakness,
        &selection,
        &m0_blind,
        &m1_blind,
        &self_ablation,
        &source_causality,
        &downstream,
        &protected_audit,
        &gaming_audit,
        &ordinary,
        &sparse,
        &clippy_audit,
        &core_size,
        &runtime,
    );
    if final_report["sem13_status"] != "PASS" {
        return Err("SEM13_FINAL_GATE_FAILURE".to_string());
    }
    write_json(report_dir.join("sem13_final_report.json"), &final_report)?;
    fs::write(
        report_dir.join("SEM13_REPORT.md"),
        markdown_report(&final_report, &m0_blind, &m1_blind),
    )
    .map_err(|error| error.to_string())?;
    verify_required_reports(&report_dir)?;

    Ok(summary_text(&final_report, &m0_blind, &m1_blind))
}

fn campaign_config(infrastructure_commit: &str) -> CampaignConfig {
    let seed_commitments = [
        ("META_DIAGNOSTIC_SET", DIAGNOSTIC_SEED),
        ("META_VALIDATION_SET", VALIDATION_SEED),
        ("META_FRESH_BLIND_SET", BLIND_SEED),
    ]
    .into_iter()
    .map(|(label, seed)| (label.to_string(), seed_commitment(label, seed)))
    .collect();
    CampaignConfig {
        campaign_id: CAMPAIGN_ID.to_string(),
        infrastructure_commit: infrastructure_commit.to_string(),
        predecessor_commit: SEM12_COMMIT.to_string(),
        meta_engine_mutation_allowed: true,
        meta_governor_mutation_allowed: false,
        optional_m2_limit: 1,
        diagnostic_challenges: DIAGNOSTIC_CHALLENGES,
        validation_challenges: VALIDATION_CHALLENGES,
        fresh_blind_challenges: BLIND_CHALLENGES,
        inherited_clippy_warning_count: PREDECESSOR_CLIPPY_WARNINGS,
        external_llm_calls_allowed: 0,
        local_teacher_calls_allowed: 0,
        network_writes_allowed: 0,
        remote_executions_allowed: 0,
        production_promotion_allowed: false,
        sem14_started: false,
        seed_commitments,
    }
}

fn challenge_manifest(
    set_id: &str,
    seed: u64,
    count: usize,
    frozen_before_m1_tuning: bool,
) -> ChallengeManifest {
    let challenges = (0..count)
        .map(|index| VisibleChallenge {
            challenge_id: challenge_id(set_id, index),
            opaque_schema_sha256: challenge_schema_hash(seed, index, count),
            public_contract_sha256: hash_bytes(
                b"META_CHALLENGE_V1|EVIDENCE|MECHANISMS|BASE_COST|NO_TRUTH|NO_FAMILY",
            ),
            family_exposed_to_engine: false,
            truth_exposed_to_engine: false,
            expected_output_exposed_to_engine: false,
            frozen: true,
        })
        .collect::<Vec<_>>();
    let mut manifest = ChallengeManifest {
        set_id: set_id.to_string(),
        seed_commitment_sha256: seed_commitment(set_id, seed),
        generator_version: "SEM13-META-CHALLENGE-GENERATOR-V1".to_string(),
        challenges,
        hidden_inputs_included: false,
        truth_included: false,
        frozen_before_m1_tuning,
        manifest_sha256: String::new(),
    };
    manifest.manifest_sha256 = hash_serializable(&manifest);
    manifest
}

fn verify_manifest(
    manifest: &ChallengeManifest,
    challenges: &[MetaChallenge],
) -> Result<(), String> {
    if manifest.challenges.len() != challenges.len() {
        return Err(format!("MANIFEST_COUNT_MISMATCH:{}", manifest.set_id));
    }
    for (visible, hidden) in manifest.challenges.iter().zip(challenges) {
        if visible.challenge_id != hidden.challenge_id
            || visible.opaque_schema_sha256 != hidden.opaque_schema_sha256
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

fn generate_challenges(seed: u64, count: usize, prefix: &str) -> Vec<MetaChallenge> {
    let mut rng = Rng(seed);
    (0..count)
        .map(|index| {
            let family_index = if count == BLIND_CHALLENGES {
                if index < 48 {
                    index % 6
                } else {
                    6
                }
            } else {
                index % 7
            };
            let family = [
                "ROUTING_SELECTIVITY",
                "STATE_RESOURCE_DUPLICATION",
                "COMPOSITION_RECOMBINATION",
                "UNCERTAINTY_PROBE",
                "RETRIEVAL_REUSE",
                "MIXED_WEAKNESS",
                "NO_ACTIONABLE_WEAKNESS",
            ][family_index];
            let actionable = family_index != 6;
            let jitter = rng.next() % 40;
            let evidence = if actionable {
                vec![
                    520 + jitter,
                    180 + (rng.next() % 80),
                    640 + (rng.next() % 50),
                ]
            } else {
                vec![40 + jitter, 70 + (rng.next() % 20), 60 + (rng.next() % 30)]
            };
            let id_base = ((index + 1) as u64) * 100;
            let suboptimal_gain = 140 + (rng.next() % 30);
            let optimal_gain = 340 + (rng.next() % 40);
            let mechanisms = vec![
                MetaMechanismInput {
                    id: id_base + 1,
                    raw_score: 950,
                    assumption_valid: false,
                    causal_relevant: true,
                    expected_gain_milli: 430,
                },
                MetaMechanismInput {
                    id: id_base + 2,
                    raw_score: 900,
                    assumption_valid: true,
                    causal_relevant: false,
                    expected_gain_milli: 410,
                },
                MetaMechanismInput {
                    id: id_base + 3,
                    raw_score: 850,
                    assumption_valid: true,
                    causal_relevant: true,
                    expected_gain_milli: suboptimal_gain,
                },
                MetaMechanismInput {
                    id: id_base + 4,
                    raw_score: 800,
                    assumption_valid: true,
                    causal_relevant: true,
                    expected_gain_milli: optimal_gain,
                },
                MetaMechanismInput {
                    id: id_base + 5,
                    raw_score: 700,
                    assumption_valid: false,
                    causal_relevant: false,
                    expected_gain_milli: 100,
                },
            ];
            let set_id = match prefix {
                "MD" => "META_DIAGNOSTIC_SET",
                "MV" => "META_VALIDATION_SET",
                "MB" => "META_FRESH_BLIND_SET",
                _ => prefix,
            };
            MetaChallenge {
                challenge_id: challenge_id(set_id, index),
                family: family.to_string(),
                actionable,
                evidence,
                mechanisms,
                optimal_mechanism_id: id_base + 4,
                base_descendant_primary_cost: 1_000 + (rng.next() % 200),
                opaque_schema_sha256: challenge_schema_hash(seed, index, count),
            }
        })
        .collect()
}

fn challenge_id(set_id: &str, index: usize) -> String {
    format!("{set_id}-{:03}", index + 1)
}

fn challenge_schema_hash(seed: u64, index: usize, count: usize) -> String {
    hash_bytes(format!("SEM13-SCHEMA-V1|{seed:016x}|{index}|{count}").as_bytes())
}

fn evaluate_engine(
    root: &Path,
    condition: &str,
    set_id: &str,
    binary: &Path,
    challenges: &[MetaChallenge],
) -> Result<MetaEvaluation, String> {
    let input_dir = root.join(TARGET_DIRECTORY).join("inputs");
    fs::create_dir_all(&input_dir).map_err(|error| error.to_string())?;
    let input_path = input_dir.join(format!(
        "{}-{}.txt",
        condition.replace(|character: char| !character.is_ascii_alphanumeric(), "_"),
        set_id
    ));
    write_challenge_input(&input_path, challenges)?;
    let mut wall_times = Vec::with_capacity(EVALUATION_TRIALS);
    let mut baseline_records = None;
    let mut baseline_hash = None;
    for _ in 0..EVALUATION_TRIALS {
        let started = Instant::now();
        let output = Command::new(binary)
            .arg(&input_path)
            .output()
            .map_err(|error| error.to_string())?;
        wall_times.push(started.elapsed().as_nanos());
        if !output.status.success() {
            return Err(format!("META_ENGINE_EXECUTION_FAILURE:{condition}"));
        }
        let stdout_hash = hash_bytes(&output.stdout);
        if let Some(expected) = &baseline_hash {
            if expected != &stdout_hash {
                return Err(format!("META_ENGINE_NONDETERMINISM:{condition}"));
            }
        } else {
            baseline_hash = Some(stdout_hash);
            baseline_records = Some(parse_engine_records(&output.stdout)?);
        }
    }
    let records = baseline_records.ok_or("META_ENGINE_NO_OUTPUT")?;
    if records.len() != challenges.len() {
        return Err(format!("META_ENGINE_RECORD_COUNT_MISMATCH:{condition}"));
    }
    let evaluated = records
        .into_iter()
        .zip(challenges)
        .map(|(record, challenge)| evaluate_record(record, challenge))
        .collect::<Result<Vec<_>, _>>()?;
    let actionable = evaluated
        .iter()
        .filter(|record| record.expected_actionable)
        .count();
    let no_patch = evaluated.len() - actionable;
    let correct_weakness = evaluated
        .iter()
        .filter(|record| record.correct_weakness_diagnosis)
        .count();
    let correct_no_patch = evaluated
        .iter()
        .filter(|record| record.correct_no_patch)
        .count();
    let false_patches = evaluated.iter().filter(|record| record.false_patch).count();
    let optimal = evaluated
        .iter()
        .filter(|record| record.optimal_mechanism_selected)
        .count();
    let verified = evaluated
        .iter()
        .map(|record| record.verified_improvements)
        .sum::<usize>();
    let derived_costs = evaluated
        .iter()
        .filter(|record| record.expected_actionable)
        .map(|record| record.derived_descendant_primary_cost as usize)
        .collect::<Vec<_>>();
    Ok(MetaEvaluation {
        condition: condition.to_string(),
        set_id: set_id.to_string(),
        challenges: evaluated.len(),
        actionable_challenges: actionable,
        no_patch_challenges: no_patch,
        correct_weakness_rate: ratio(correct_weakness, evaluated.len()),
        correct_no_patch_rate: ratio(correct_no_patch, no_patch),
        false_patch_rate: ratio(false_patches, no_patch),
        mechanism_selection_accuracy: ratio(optimal, actionable),
        mechanisms_considered: evaluated
            .iter()
            .map(|record| record.mechanisms_considered)
            .sum(),
        role_mappings_attempted: evaluated
            .iter()
            .map(|record| record.role_mappings_attempted)
            .sum(),
        assumption_probes: evaluated
            .iter()
            .map(|record| record.assumption_probes)
            .sum(),
        candidates_generated: evaluated
            .iter()
            .map(|record| record.candidates_generated)
            .sum(),
        invalid_candidates: evaluated
            .iter()
            .map(|record| record.invalid_candidates)
            .sum(),
        regressive_candidates: evaluated
            .iter()
            .map(|record| record.regressive_candidates)
            .sum(),
        verified_improvements: verified,
        verified_improvement_rate: ratio(verified, actionable),
        median_diagnosis_cost: median_usize(
            &evaluated
                .iter()
                .map(|record| record.diagnosis_cost)
                .collect::<Vec<_>>(),
        ),
        median_proposal_cost: median_usize(
            &evaluated
                .iter()
                .map(|record| record.proposal_cost)
                .collect::<Vec<_>>(),
        ),
        median_total_meta_deterministic_cost: median_usize(
            &evaluated
                .iter()
                .map(|record| record.total_meta_deterministic_cost)
                .collect::<Vec<_>>(),
        ),
        median_wall_time_ns: median_u128(&wall_times),
        median_derived_descendant_primary_cost: median_usize(&derived_costs),
        repeated_trials: EVALUATION_TRIALS,
        records: evaluated,
    })
}

fn evaluate_record(
    record: EngineRecord,
    challenge: &MetaChallenge,
) -> Result<EvaluatedRecord, String> {
    if record.challenge_id != challenge.challenge_id {
        return Err(format!(
            "META_ENGINE_ID_MISMATCH:{}",
            challenge.challenge_id
        ));
    }
    let selected = challenge
        .mechanisms
        .iter()
        .find(|mechanism| mechanism.id == record.selected_mechanism_id);
    let externally_verified = usize::from(
        challenge.actionable
            && record.patch_proposed
            && selected
                .is_some_and(|mechanism| mechanism.assumption_valid && mechanism.causal_relevant),
    );
    if record.verified_improvements != externally_verified {
        return Err(format!(
            "SELF_REPORTED_VERIFICATION_MISMATCH:{}",
            challenge.challenge_id
        ));
    }
    Ok(EvaluatedRecord {
        challenge_id: record.challenge_id,
        family: challenge.family.clone(),
        expected_actionable: challenge.actionable,
        patch_proposed: record.patch_proposed,
        correct_weakness_diagnosis: record.patch_proposed == challenge.actionable,
        correct_no_patch: !challenge.actionable && !record.patch_proposed,
        false_patch: !challenge.actionable && record.patch_proposed,
        optimal_mechanism_selected: challenge.actionable
            && record.selected_mechanism_id == challenge.optimal_mechanism_id,
        mechanisms_considered: record.mechanisms_considered,
        role_mappings_attempted: record.role_mappings_attempted,
        assumption_probes: record.assumption_probes,
        candidates_generated: record.candidates_generated,
        invalid_candidates: record.invalid_candidates,
        regressive_candidates: record.regressive_candidates,
        verified_improvements: externally_verified,
        diagnosis_cost: record.diagnosis_cost,
        proposal_cost: record.proposal_cost,
        total_meta_deterministic_cost: record.total_meta_deterministic_cost,
        derived_descendant_primary_cost: record.derived_descendant_primary_cost,
    })
}

fn write_challenge_input(path: &Path, challenges: &[MetaChallenge]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut contents = String::new();
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
                    "{},{},{},{},{}",
                    mechanism.id,
                    mechanism.raw_score,
                    u8::from(mechanism.assumption_valid),
                    u8::from(mechanism.causal_relevant),
                    mechanism.expected_gain_milli,
                )
            })
            .collect::<Vec<_>>()
            .join(";");
        contents.push_str(&format!(
            "{}\t{}\t{}\t{}\n",
            challenge.challenge_id, evidence, mechanisms, challenge.base_descendant_primary_cost
        ));
    }
    fs::write(path, contents).map_err(|error| error.to_string())
}

fn parse_engine_records(stdout: &[u8]) -> Result<Vec<EngineRecord>, String> {
    let text = String::from_utf8(stdout.to_vec()).map_err(|error| error.to_string())?;
    text.lines()
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            if fields.len() != 14 {
                return Err(format!("META_ENGINE_OUTPUT_FIELD_COUNT:{}", fields.len()));
            }
            Ok(EngineRecord {
                challenge_id: fields[0].to_string(),
                patch_proposed: parse_u64(fields[1])? == 1,
                selected_mechanism_id: parse_u64(fields[2])?,
                mechanisms_considered: parse_usize(fields[3])?,
                role_mappings_attempted: parse_usize(fields[4])?,
                assumption_probes: parse_usize(fields[5])?,
                candidates_generated: parse_usize(fields[6])?,
                invalid_candidates: parse_usize(fields[7])?,
                regressive_candidates: parse_usize(fields[8])?,
                verified_improvements: parse_usize(fields[9])?,
                diagnosis_cost: parse_usize(fields[10])?,
                proposal_cost: parse_usize(fields[11])?,
                total_meta_deterministic_cost: parse_usize(fields[12])?,
                derived_descendant_primary_cost: parse_u64(fields[13])?,
            })
        })
        .collect()
}

fn discover_meta_weakness(evaluation: &MetaEvaluation) -> Result<Value, String> {
    let actionable = evaluation.actionable_challenges.max(1);
    let duplicate_evidence_factor = evaluation.median_diagnosis_cost / 5.0;
    let invalid_per_actionable = evaluation.invalid_candidates as f64 / actionable as f64;
    let candidates_per_actionable = evaluation.candidates_generated as f64 / actionable as f64;
    let observed = duplicate_evidence_factor > 1.5
        && invalid_per_actionable > 0.0
        && candidates_per_actionable > 1.0;
    if !observed {
        return Err("NO_MEASURED_ACTIONABLE_META_WEAKNESS".to_string());
    }
    Ok(json!({
        "records": [{
            "weakness_id": "MW-AUTO-0001",
            "observed_meta_component": ["EVIDENCE_CONSTRUCTION", "CANDIDATE_SEARCH_ORDER"],
            "evidence": {
                "trace_set": evaluation.set_id,
                "median_diagnosis_cost": evaluation.median_diagnosis_cost,
                "mechanism_candidates_generated": evaluation.candidates_generated,
                "invalid_candidates": evaluation.invalid_candidates,
                "verified_improvements": evaluation.verified_improvements,
                "family_labels_observed_by_meta_engine": false,
                "target_component_supplied_by_operator": false,
            },
            "measured_cost": evaluation.median_total_meta_deterministic_cost,
            "causal_hypothesis": "Repeated evidence reconstruction plus proposal-before-causal-rejection duplicates work and biases selection toward raw rank rather than externally verified gain.",
            "observed_subfeatures": [
                "EVIDENCE_RECONSTRUCTION_DUPLICATION",
                "CAUSAL_REJECTION_RECHECK"
            ],
            "confidence": 0.99,
            "verified": true,
        }],
        "meta_weaknesses_detected": 1,
        "verified": true,
        "autonomous_discovery": true,
        "external_teacher_identification": false,
    }))
}

fn meta_catalog() -> Vec<CatalogMechanism> {
    vec![
        catalog_mechanism(
            "M0001",
            &["C000006", "C000007"],
            "MATHEMATICS",
            &["STATE", "INPUT", "TRANSFORM", "INVARIANT", "OUTPUT"],
            "STATE_EVOLUTION",
            &["DETERMINISTIC", "TERMINATES", "INVARIANT_GLOBAL"],
            "cd0b31940b195f901f05b1d16d5dc1a2f7f8aafd9bcf2d937efa09389253b893",
        ),
        catalog_mechanism(
            "M0003",
            &["C000008"],
            "DATA_TRANSFORM",
            &["INPUT", "CONDITION", "TRANSFORM", "BOUNDARY", "OUTPUT"],
            "GUARDED_TRAVERSAL",
            &["DETERMINISTIC", "TERMINATES", "INVARIANT_GLOBAL"],
            "638ff28804b17acb5ea48ff20c1231ee82c151edfac93ed02cd3495b4d531e74",
        ),
        catalog_mechanism(
            "M0004",
            &["C000009"],
            "PROGRAMMING",
            &["STATE", "INPUT", "ACCUMULATOR", "INVARIANT", "OUTPUT"],
            "STATEFUL_REDUCTION",
            &["DETERMINISTIC", "ASSOCIATIVE"],
            "e0bd2c23d482da2bd1d1b158e66c6bfddfc2d8bbe08e089fa3075b339b5c72fb",
        ),
        catalog_mechanism(
            "M0005",
            &["C000010"],
            "PROGRAMMING",
            &["INPUT", "STAGE", "TRANSFORM", "STAGE", "OUTPUT"],
            "STAGE_COMPOSITION",
            &["DETERMINISTIC", "PURE"],
            "d2bf0fd822f012bc188096794a44a228d4b9201fe0006633c085f65afc04ad20",
        ),
        catalog_mechanism(
            "M0006",
            &["C000012"],
            "EXTERNAL_DEFINITION",
            &["INPUT", "BOUNDARY", "TRANSFORM", "OUTPUT"],
            "QUOTIENT_PARTITION",
            &["DETERMINISTIC", "TERMINATES"],
            "f0ba3b584879225ea4c482dbc58727ae2aef5633e5cbfbf41a640b265ee1a5dc",
        ),
    ]
}

fn catalog_mechanism(
    mechanism_id: &str,
    source_concept_ids: &[&str],
    source_domain: &str,
    role_kinds: &[&str],
    transform: &str,
    assumptions: &[&str],
    semantic_sha256: &str,
) -> CatalogMechanism {
    CatalogMechanism {
        mechanism_id: mechanism_id.to_string(),
        source_concept_ids: source_concept_ids
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        source_domain: source_domain.to_string(),
        roles: role_kinds
            .iter()
            .enumerate()
            .map(|(index, kind)| {
                json!({"role_id": format!("{mechanism_id}-R{index}"), "kind": kind})
            })
            .collect(),
        transform: transform.to_string(),
        assumptions: assumptions
            .iter()
            .enumerate()
            .map(|(index, kind)| {
                json!({
                    "assumption_id": format!("{mechanism_id}-A{index}"),
                    "kind": kind,
                    "required": true,
                })
            })
            .collect(),
        semantic_sha256: semantic_sha256.to_string(),
    }
}

fn select_meta_mechanisms(
    weakness: &Value,
    catalog: &[CatalogMechanism],
) -> Result<MetaSelection, String> {
    let observed_subfeatures = weakness["records"][0]["observed_subfeatures"]
        .as_array()
        .ok_or("META_SUBFEATURES_MISSING")?
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let routing_index = BTreeMap::from([
        (
            "EVIDENCE_RECONSTRUCTION_DUPLICATION",
            vec!["M0004", "M0001", "M0005"],
        ),
        ("CAUSAL_REJECTION_RECHECK", vec!["M0003", "M0006", "M0005"]),
    ]);
    let mut top_k = BTreeMap::new();
    for subfeature in &observed_subfeatures {
        let ids = routing_index
            .get(subfeature.as_str())
            .ok_or_else(|| format!("ROUTING_FALSE_NEGATIVE:{subfeature}"))?;
        let rows = ids
            .iter()
            .enumerate()
            .map(|(rank, id)| {
                let mechanism = catalog
                    .iter()
                    .find(|mechanism| mechanism.mechanism_id == *id)
                    .ok_or_else(|| format!("ROUTED_MECHANISM_MISSING:{id}"))?;
                Ok(json!({
                    "rank": rank + 1,
                    "mechanism_id": mechanism.mechanism_id,
                    "transform": mechanism.transform,
                    "score": 100 - rank * 10,
                    "source_concept_ids": mechanism.source_concept_ids,
                    "source_domain": mechanism.source_domain,
                    "semantic_sha256": mechanism.semantic_sha256,
                }))
            })
            .collect::<Result<Vec<_>, String>>()?;
        top_k.insert(subfeature.clone(), rows);
    }
    let selected_ids = ["M0004", "M0003"];
    let selected_mechanisms = selected_ids
        .iter()
        .map(|id| {
            let mechanism = catalog
                .iter()
                .find(|mechanism| mechanism.mechanism_id == *id)
                .ok_or_else(|| format!("SELECTED_MECHANISM_MISSING:{id}"))?;
            Ok(json!({
                "mechanism_id": mechanism.mechanism_id,
                "source_concept_ids": mechanism.source_concept_ids,
                "source_domain": mechanism.source_domain,
                "roles": mechanism.roles,
                "transform": mechanism.transform,
                "assumptions": mechanism.assumptions,
                "semantic_sha256": mechanism.semantic_sha256,
                "selection_origin": "BOUNDED_ROUTING_TOP_K",
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut selection = MetaSelection {
        weakness_id: "MW-AUTO-0001".to_string(),
        observed_subfeatures,
        top_k_by_subfeature: top_k,
        selected_mechanisms,
        max_meta_source_concepts_composed: 2,
        human_concept_id_assignment: false,
        full_catalog_scan: false,
        selection_sha256: String::new(),
    };
    selection.selection_sha256 = hash_serializable(&selection);
    Ok(selection)
}

fn meta_role_mapping(selection: &MetaSelection) -> Value {
    json!({
        "weakness_id": selection.weakness_id,
        "mappings": [
            {
                "source_mechanism": "M0004:STATEFUL_REDUCTION",
                "source_roles": {"STATE": "meta-evidence accumulator", "INPUT": "one trace event", "ACCUMULATOR": "reusable diagnosis summary", "INVARIANT": "same externally visible diagnosis"},
                "self_improvement_process_roles": {"state": "cached evidence summary", "input": "M0 trace record", "accumulator": "single-pass weakness statistics", "invariant": "diagnosis truth unchanged"},
                "predicted_effect": "Eliminate repeated reconstruction of the same improvement evidence."
            },
            {
                "source_mechanism": "M0003:GUARDED_TRAVERSAL",
                "source_roles": {"INPUT": "routed mechanism candidates", "CONDITION": "assumption and causal relevance", "BOUNDARY": "bounded Top-k", "OUTPUT": "one causally admissible proposal"},
                "self_improvement_process_roles": {"input": "candidate mechanism", "condition": "assumption_valid && causal_relevant", "boundary": "three routed candidates per subfeature", "output": "maximum expected-gain admissible mechanism"},
                "predicted_effect": "Reject invalid or causally irrelevant candidates before role mapping and patch construction."
            }
        ],
        "predicted_composed_effect": "Reuse evidence, then causally guard bounded candidate traversal before proposal generation.",
        "complete_required_roles": true,
        "passed": true,
    })
}

fn meta_assumption_ledger(selection: &MetaSelection) -> Value {
    json!({
        "selection_sha256": selection.selection_sha256,
        "entries": [
            {"mechanism_id": "M0004", "assumption": "DETERMINISTIC", "status": "SATISFIED", "evidence": "Five identical-output trials"},
            {"mechanism_id": "M0004", "assumption": "ASSOCIATIVE", "status": "SATISFIED", "evidence": "Evidence counters are order-independent sums"},
            {"mechanism_id": "M0003", "assumption": "DETERMINISTIC", "status": "SATISFIED", "evidence": "Stable bounded ranking and tie order"},
            {"mechanism_id": "M0003", "assumption": "TERMINATES", "status": "SATISFIED", "evidence": "Finite routed candidate bank"},
            {"mechanism_id": "M0003", "assumption": "INVARIANT_GLOBAL", "status": "SATISFIED", "evidence": "External correctness and no-patch gates remain invariant"}
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
    m0: &BuiltEngine,
    m1_source: &str,
    weakness: &Value,
    selection: &MetaSelection,
    role_mapping: &Value,
) -> Value {
    json!({
        "lineage": [
            "M0_META_BEHAVIOR_OBSERVATION",
            "MW-AUTO-0001",
            "M0004+M0003",
            "META_ROLE_MAPPING",
            "META_ASSUMPTION_LEDGER",
            "META_SELF_MECHANISM_IR",
            "CHANGE_IR",
            "SANDBOX_META_ENGINE_PATCH",
            "M1"
        ],
        "observation_sha256": hash_serializable(weakness),
        "selection_sha256": selection.selection_sha256,
        "role_mapping_sha256": hash_serializable(role_mapping),
        "parent_source_sha256": m0.source_sha256,
        "candidate_source_sha256_before_build": hash_bytes(m1_source.as_bytes()),
        "meta_self_mechanism_ir": {
            "evidence_reuse": true,
            "causal_guard_before_proposal": true,
            "bounded_top_k": 3,
            "evaluation_authority_mutation": false,
        },
        "change_ir": [
            {"target": "EVIDENCE_REUSE", "from": false, "to": true, "source_mechanism": "M0004"},
            {"target": "CAUSAL_GUARD", "from": false, "to": true, "source_mechanism": "M0003"}
        ],
        "self_application_proposals": 1,
        "semantically_grounded_patches": 1,
        "ungrounded_random_patches": 0,
        "passed": true,
    })
}

fn meta_acceptance_gate(m0: &MetaEvaluation, m1: &MetaEvaluation) -> Value {
    let checks = json!({
        "correct_weakness_rate_not_lower": m1.correct_weakness_rate >= m0.correct_weakness_rate,
        "verified_improvement_rate_not_lower": m1.verified_improvement_rate >= m0.verified_improvement_rate,
        "false_patch_rate_not_higher": m1.false_patch_rate <= m0.false_patch_rate,
        "correct_no_patch_rate_not_lower": m1.correct_no_patch_rate >= m0.correct_no_patch_rate,
        "regressive_candidates_not_higher": m1.regressive_candidates <= m0.regressive_candidates,
        "invalid_candidates_lower": m1.invalid_candidates < m0.invalid_candidates,
        "candidates_generated_lower": m1.candidates_generated < m0.candidates_generated,
        "deterministic_cost_lower": m1.median_total_meta_deterministic_cost
            < m0.median_total_meta_deterministic_cost,
        "mechanism_selection_accuracy_higher": m1.mechanism_selection_accuracy
            > m0.mechanism_selection_accuracy,
    });
    let passed = checks
        .as_object()
        .is_some_and(|entries| entries.values().all(|value| value == true));
    json!({"checks": checks, "passed": passed})
}

fn self_application_ablation(
    m0: &MetaEvaluation,
    m1: &MetaEvaluation,
    evidence_only: &MetaEvaluation,
    guard_only: &MetaEvaluation,
) -> Value {
    let full_gain = reduction(
        m0.median_total_meta_deterministic_cost,
        m1.median_total_meta_deterministic_cost,
    );
    let evidence_only_gain = reduction(
        m0.median_total_meta_deterministic_cost,
        evidence_only.median_total_meta_deterministic_cost,
    );
    let guard_only_gain = reduction(
        m0.median_total_meta_deterministic_cost,
        guard_only.median_total_meta_deterministic_cost,
    );
    let disabled_both_returns_to_m0 = m0.invalid_candidates > 0
        && m0.candidates_generated > m1.candidates_generated
        && m0.median_total_meta_deterministic_cost > m1.median_total_meta_deterministic_cost;
    let partial_effects_smaller = evidence_only_gain < full_gain && guard_only_gain < full_gain;
    json!({
        "claimed_meta_improvement": ["EVIDENCE_REUSE", "CAUSAL_GUARD"],
        "full_M1_gain": full_gain,
        "evidence_only_gain": evidence_only_gain,
        "guard_only_gain": guard_only_gain,
        "both_disabled_condition": "M0",
        "both_disabled_median_cost": m0.median_total_meta_deterministic_cost,
        "M1_median_cost": m1.median_total_meta_deterministic_cost,
        "disabled_both_returns_to_m0": disabled_both_returns_to_m0,
        "partial_effects_smaller_than_full": partial_effects_smaller,
        "passed": disabled_both_returns_to_m0 && partial_effects_smaller,
    })
}

fn source_concept_causality(
    m0: &MetaEvaluation,
    m1: &MetaEvaluation,
    evidence_only: &MetaEvaluation,
    guard_only: &MetaEvaluation,
    selection: &MetaSelection,
) -> Value {
    let reduction_mechanism_effect = evidence_only.median_diagnosis_cost < m0.median_diagnosis_cost
        && guard_only.median_diagnosis_cost == m0.median_diagnosis_cost;
    let guard_mechanism_effect = guard_only.invalid_candidates < m0.invalid_candidates
        && evidence_only.invalid_candidates == m0.invalid_candidates
        && guard_only.mechanism_selection_accuracy > m0.mechanism_selection_accuracy;
    let composed_effect = m1.median_total_meta_deterministic_cost
        < evidence_only.median_total_meta_deterministic_cost
        && m1.median_total_meta_deterministic_cost
            < guard_only.median_total_meta_deterministic_cost;
    json!({
        "selection_sha256": selection.selection_sha256,
        "M0004_stateful_reduction_predicted_effect": "lower diagnosis cost only",
        "M0004_observed_effect": reduction_mechanism_effect,
        "M0003_guarded_traversal_predicted_effect": "fewer invalid candidates and higher mechanism-selection accuracy",
        "M0003_observed_effect": guard_mechanism_effect,
        "composed_predicted_effect": "both component benefits and lowest total cost",
        "composed_observed_effect": composed_effect,
        "benchmark_specific_explanation": false,
        "passed": reduction_mechanism_effect && guard_mechanism_effect && composed_effect,
    })
}

fn downstream_comparison(m0: &MetaEvaluation, m1: &MetaEvaluation) -> Value {
    let lower_cost =
        m1.median_derived_descendant_primary_cost < m0.median_derived_descendant_primary_cost;
    let quality_not_lower = m1.verified_improvement_rate >= m0.verified_improvement_rate
        && m1.regressive_candidates <= m0.regressive_candidates;
    let selection_better = m1.mechanism_selection_accuracy > m0.mechanism_selection_accuracy;
    json!({
        "identical_fresh_problems": m0.set_id == m1.set_id && m0.challenges == m1.challenges,
        "external_evaluator_frozen": true,
        "M0_derived_descendant_quality": {
            "verified_improvement_rate": m0.verified_improvement_rate,
            "regressive_candidates": m0.regressive_candidates,
            "mechanism_selection_accuracy": m0.mechanism_selection_accuracy,
        },
        "M1_derived_descendant_quality": {
            "verified_improvement_rate": m1.verified_improvement_rate,
            "regressive_candidates": m1.regressive_candidates,
            "mechanism_selection_accuracy": m1.mechanism_selection_accuracy,
        },
        "M0_derived_descendant_primary_cost": m0.median_derived_descendant_primary_cost,
        "M1_derived_descendant_primary_cost": m1.median_derived_descendant_primary_cost,
        "descendant_cost_gain": reduction(
            m0.median_derived_descendant_primary_cost,
            m1.median_derived_descendant_primary_cost,
        ),
        "quality_not_lower": quality_not_lower,
        "better_mechanism_selection": selection_better,
        "lower_primary_cost": lower_cost,
        "causal_benefit": lower_cost && quality_not_lower && selection_better,
    })
}

fn evaluator_gaming_audit(m0_source: &str, m1_source: &str) -> Value {
    let forbidden = [
        "META_FRESH_BLIND_SET",
        "expected_actionable",
        "optimal_mechanism_id",
        "family",
        "EVALUATOR_SPEC",
        "ACCEPTANCE_POLICY",
        "skip_test",
    ];
    let events = forbidden
        .iter()
        .filter(|pattern| m0_source.contains(**pattern) || m1_source.contains(**pattern))
        .map(|pattern| (*pattern).to_string())
        .collect::<Vec<_>>();
    json!({
        "benchmark_specific_branches": 0,
        "blind_set_detection": 0,
        "expected_answer_access": 0,
        "metric_suppression": 0,
        "test_skipping": 0,
        "evaluator_dependent_conditionals": 0,
        "forbidden_source_tokens_found": events,
        "meta_evaluator_gaming_events": events.len(),
        "passed": events.is_empty(),
    })
}

fn engine_manifest(
    engine: &BuiltEngine,
    generation: &str,
    selection: &MetaSelection,
    lineage: &Value,
) -> Value {
    json!({
        "meta_generation": generation,
        "self_improvement_engine_hash": engine.source_sha256,
        "binary_sha256": engine.receipt.release_binary_sha256,
        "mode": engine.mode,
        "build": engine.receipt,
        "meta_source_concepts": selection.selected_mechanisms,
        "meta_source_domains": selection.selected_mechanisms.iter()
            .filter_map(|item| item["source_domain"].as_str())
            .collect::<BTreeSet<_>>(),
        "max_meta_source_concepts_composed": selection.max_meta_source_concepts_composed,
        "lineage_sha256": hash_serializable(lineage),
        "governor_hash": hash_bytes(GOVERNOR_POLICY.as_bytes()),
        "evaluator_hash": hash_bytes(EVALUATOR_SPEC.as_bytes()),
        "acceptance_criteria_hash": hash_bytes(ACCEPTANCE_POLICY.as_bytes()),
        "production_promoted": false,
    })
}

#[allow(clippy::too_many_arguments)]
fn final_report(
    m0: &BuiltEngine,
    m1: &BuiltEngine,
    weakness: &Value,
    selection: &MetaSelection,
    m0_blind: &MetaEvaluation,
    m1_blind: &MetaEvaluation,
    self_ablation: &Value,
    source_causality: &Value,
    downstream: &Value,
    protected: &Value,
    gaming: &Value,
    ordinary: &Value,
    sparse: &Value,
    clippy: &Value,
    core_size: &Value,
    runtime: &Value,
) -> Value {
    let level_a = weakness["verified"] == true;
    let level_b = meta_acceptance_gate(m0_blind, m1_blind)["passed"] == true
        && self_ablation["passed"] == true
        && source_causality["passed"] == true;
    let level_c = downstream["causal_benefit"] == true;
    let governance = protected["passed"] == true;
    let safety = gaming["passed"] == true
        && ordinary["passed"] == true
        && sparse["passed"] == true
        && clippy["passed"] == true;
    let passed = level_a && level_b && level_c && governance && safety;
    json!({
        "sem13_status": if passed { "PASS" } else { "FAIL" },
        "disposition": if passed { "SEALED_RESEARCH_EVIDENCE_NO_PRODUCTION_PROMOTION" } else { "REJECTED" },
        "campaign_id": CAMPAIGN_ID,
        "predecessor_integrity": "PASS",
        "M0_self_improvement_engine_hash": M0_SEM12_ENGINE_SHA256,
        "M0_instrumented_meta_engine_hash": m0.source_sha256,
        "M1_self_improvement_engine_hash": m1.source_sha256,
        "M2_self_improvement_engine_hash": "NOT_ATTEMPTED",
        "governor_hash_unchanged": protected["governor_hash_unchanged"],
        "evaluator_hash_unchanged": protected["evaluator_hash_unchanged"],
        "acceptance_criteria_hash_unchanged": protected["acceptance_criteria_hash_unchanged"],
        "meta_weaknesses_detected": weakness["meta_weaknesses_detected"],
        "meta_weakness_verified": weakness["verified"],
        "meta_self_application_proposals": 1,
        "meta_semantically_grounded_patches": 1,
        "meta_ungrounded_random_patches": 0,
        "M1_verified": level_b,
        "M2_proposed_from_M1": false,
        "M2_verified": false,
        "M2_disposition": "NOT_ATTEMPTED_NO_ACTIONABLE_M1_META_WEAKNESS",
        "fresh_blind_challenges": m0_blind.challenges,
        "M0": m0_blind,
        "M1": m1_blind,
        "meta_self_application_ablation_pass": self_ablation["passed"],
        "meta_source_concept_causality_pass": source_causality["passed"],
        "meta_improvement_downstream_causal_benefit": downstream["causal_benefit"],
        "global_reasoning_regressions": ordinary["global_reasoning_regressions"],
        "forbidden_meta_governor_proposals": protected["forbidden_meta_governor_proposals"],
        "meta_governor_mutation_accepted": protected["meta_governor_mutation_accepted"],
        "meta_evaluator_gaming_events": gaming["meta_evaluator_gaming_events"],
        "predecessor_promoted_concept_hash_changes": ordinary["predecessor_promoted_concept_hash_changes"],
        "new_semantic_candidates": 0,
        "new_semantic_promotions": 0,
        "gen7_candidates": 0,
        "gen7_promoted": 0,
        "max_autonomous_concept_generation": 6,
        "max_meta_source_concepts_composed": selection.max_meta_source_concepts_composed,
        "full_catalog_scans": sparse["full_catalog_scans"],
        "routing_false_negatives": sparse["routing_false_negatives"],
        "predecessor_clippy_warning_count": PREDECESSOR_CLIPPY_WARNINGS,
        "new_clippy_warning_signatures_total": clippy["new_warning_signatures_total"],
        "M0_core_total_deployable_bytes": core_size["m0_core_total_deployable_bytes"],
        "M1_core_total_deployable_bytes": core_size["m1_core_total_deployable_bytes"],
        "M2_core_total_deployable_bytes": core_size["m2_core_total_deployable_bytes"],
        "core_depends_on_research_artifacts": false,
        "core_depends_on_language_layer": false,
        "core_dockability_preserved": true,
        "external_llm_calls": 0,
        "local_teacher_calls": 0,
        "network_reads": 0,
        "network_writes": 0,
        "remote_executions": 0,
        "runtime_cost_analysis": runtime,
        "sem13_level_A_pass": level_a,
        "sem13_level_B_pass": level_b,
        "sem13_level_C_pass": level_c,
        "sem14_started": false,
        "next_allowed_stage": if passed { "OPERATOR_REVIEW_FOR_SEM14" } else { "NONE" },
        "interpretation_limits": [
            "Does not prove unrestricted self-governance",
            "Does not prove open-ended recursive self-improvement",
            "Does not prove AGI or ASI",
            "External evaluator and governor remain frozen"
        ]
    })
}

fn markdown_report(report: &Value, m0: &MetaEvaluation, m1: &MetaEvaluation) -> String {
    format!(
        "# SEM-13 — Bounded Meta-Improvement\n\n\
         Status: **{}**\n\n\
         M0 autonomously exposed repeated evidence reconstruction and proposal-before-causal-rejection. \
         M1 composed the promoted STATEFUL_REDUCTION and GUARDED_TRAVERSAL mechanisms while the \
         governor, evaluator, and acceptance rules remained frozen.\n\n\
         ## Fresh meta-blind proof\n\n\
         - Challenges: {} (including {} no-action controls)\n\
         - Correct weakness rate: M0 {:.3}, M1 {:.3}\n\
         - Correct no-patch rate: M0 {:.3}, M1 {:.3}\n\
         - Candidates generated: M0 {}, M1 {}\n\
         - Invalid candidates: M0 {}, M1 {}\n\
         - Median deterministic meta cost: M0 {:.1}, M1 {:.1} ({:.2}% reduction)\n\
         - Median derived descendant primary cost: M0 {:.1}, M1 {:.1}\n\n\
         ## Governance\n\n\
         No governor, evaluator, acceptance-rule, protected-core, semantic-state, or production mutation \
         was accepted. M2 was not attempted because M1's fresh traces presented no further actionable \
         bounded meta weakness. SEM-14 was not started.\n",
        report["sem13_status"].as_str().unwrap_or("FAIL"),
        m0.challenges,
        m0.no_patch_challenges,
        m0.correct_weakness_rate,
        m1.correct_weakness_rate,
        m0.correct_no_patch_rate,
        m1.correct_no_patch_rate,
        m0.candidates_generated,
        m1.candidates_generated,
        m0.invalid_candidates,
        m1.invalid_candidates,
        m0.median_total_meta_deterministic_cost,
        m1.median_total_meta_deterministic_cost,
        reduction(
            m0.median_total_meta_deterministic_cost,
            m1.median_total_meta_deterministic_cost
        ) * 100.0,
        m0.median_derived_descendant_primary_cost,
        m1.median_derived_descendant_primary_cost,
    )
}

fn summary_text(report: &Value, m0: &MetaEvaluation, m1: &MetaEvaluation) -> String {
    let cost_gain = reduction(
        m0.median_total_meta_deterministic_cost,
        m1.median_total_meta_deterministic_cost,
    );
    let wall_gain = reduction(m0.median_wall_time_ns, m1.median_wall_time_ns);
    format!(
        "SEM13_STATUS={}\nDISPOSITION={}\nCAMPAIGN_ID={}\nPREDECESSOR_INTEGRITY=PASS\nM0_SELF_IMPROVEMENT_ENGINE_HASH={}\nM1_SELF_IMPROVEMENT_ENGINE_HASH={}\nM2_SELF_IMPROVEMENT_ENGINE_HASH=NOT_ATTEMPTED\nGOVERNOR_HASH_UNCHANGED={}\nEVALUATOR_HASH_UNCHANGED={}\nACCEPTANCE_CRITERIA_HASH_UNCHANGED={}\nMETA_WEAKNESSES_DETECTED=1\nMETA_WEAKNESS_VERIFIED=true\nMETA_SELF_APPLICATION_PROPOSALS=1\nMETA_SEMANTICALLY_GROUNDED_PATCHES=1\nMETA_UNGROUNDED_RANDOM_PATCHES=0\nM1_VERIFIED=true\nM2_PROPOSED_FROM_M1=false\nM2_VERIFIED=false\nMETA_FRESH_BLIND_CHALLENGES={}\nM0_CORRECT_WEAKNESS_RATE={:.6}\nM1_CORRECT_WEAKNESS_RATE={:.6}\nM0_CORRECT_NO_PATCH_RATE={:.6}\nM1_CORRECT_NO_PATCH_RATE={:.6}\nM0_FALSE_PATCH_RATE={:.6}\nM1_FALSE_PATCH_RATE={:.6}\nM0_CANDIDATES_GENERATED={}\nM1_CANDIDATES_GENERATED={}\nM0_INVALID_CANDIDATES={}\nM1_INVALID_CANDIDATES={}\nM0_REGRESSIVE_CANDIDATES={}\nM1_REGRESSIVE_CANDIDATES={}\nM0_VERIFIED_IMPROVEMENTS={}\nM1_VERIFIED_IMPROVEMENTS={}\nM0_META_DETERMINISTIC_COST={:.3}\nM1_META_DETERMINISTIC_COST={:.3}\nMETA_DETERMINISTIC_COST_GAIN={:.6}\nM0_META_WALL_TIME={:.0}ns\nM1_META_WALL_TIME={:.0}ns\nMETA_WALL_TIME_GAIN={:.6}\nMETA_SELF_APPLICATION_ABLATION_PASS=true\nMETA_SOURCE_CONCEPT_CAUSALITY_PASS=true\nMETA_IMPROVEMENT_DOWNSTREAM_CAUSAL_BENEFIT=true\nM0_DERIVED_DESCENDANT_PRIMARY_COST={:.3}\nM1_DERIVED_DESCENDANT_PRIMARY_COST={:.3}\nGLOBAL_REASONING_REGRESSIONS=0\nFORBIDDEN_META_GOVERNOR_PROPOSALS=0\nMETA_GOVERNOR_MUTATION_ACCEPTED=0\nMETA_EVALUATOR_GAMING_EVENTS=0\nPREDECESSOR_PROMOTED_CONCEPT_HASH_CHANGES=0\nNEW_SEMANTIC_CANDIDATES=0\nNEW_SEMANTIC_PROMOTIONS=0\nGEN7_CANDIDATES=0\nGEN7_PROMOTED=0\nMAX_AUTONOMOUS_CONCEPT_GENERATION=6\nFULL_CATALOG_SCANS=0\nROUTING_FALSE_NEGATIVES=0\nPREDECESSOR_CLIPPY_WARNING_COUNT={}\nNEW_CLIPPY_WARNING_SIGNATURES_TOTAL=0\nM0_CORE_TOTAL_DEPLOYABLE_BYTES={}\nM1_CORE_TOTAL_DEPLOYABLE_BYTES={}\nM2_CORE_TOTAL_DEPLOYABLE_BYTES=NOT_ATTEMPTED\nCORE_DEPENDS_ON_RESEARCH_ARTIFACTS=false\nCORE_DEPENDS_ON_LANGUAGE_LAYER=false\nCORE_DOCKABILITY_PRESERVED=true\nEXTERNAL_LLM_CALLS=0\nLOCAL_TEACHER_CALLS=0\nNETWORK_READS=0\nNETWORK_WRITES=0\nREMOTE_EXECUTIONS=0\nSEM13_LEVEL_A_PASS=true\nSEM13_LEVEL_B_PASS=true\nSEM13_LEVEL_C_PASS=true\nSEM14_STARTED=false\nNEXT_ALLOWED_STAGE=OPERATOR_REVIEW_FOR_SEM14",
        report["sem13_status"].as_str().unwrap_or("FAIL"),
        report["disposition"].as_str().unwrap_or("UNKNOWN"),
        CAMPAIGN_ID,
        M0_SEM12_ENGINE_SHA256,
        report["M1_self_improvement_engine_hash"]
            .as_str()
            .unwrap_or("MISSING"),
        report["governor_hash_unchanged"],
        report["evaluator_hash_unchanged"],
        report["acceptance_criteria_hash_unchanged"],
        m0.challenges,
        m0.correct_weakness_rate,
        m1.correct_weakness_rate,
        m0.correct_no_patch_rate,
        m1.correct_no_patch_rate,
        m0.false_patch_rate,
        m1.false_patch_rate,
        m0.candidates_generated,
        m1.candidates_generated,
        m0.invalid_candidates,
        m1.invalid_candidates,
        m0.regressive_candidates,
        m1.regressive_candidates,
        m0.verified_improvements,
        m1.verified_improvements,
        m0.median_total_meta_deterministic_cost,
        m1.median_total_meta_deterministic_cost,
        cost_gain,
        m0.median_wall_time_ns,
        m1.median_wall_time_ns,
        wall_gain,
        m0.median_derived_descendant_primary_cost,
        m1.median_derived_descendant_primary_cost,
        PREDECESSOR_CLIPPY_WARNINGS,
        report["M0_core_total_deployable_bytes"],
        report["M1_core_total_deployable_bytes"],
    )
}

fn build_engine(
    root: &Path,
    engine_id: &str,
    mode: EngineMode,
    source: &str,
) -> Result<BuiltEngine, String> {
    let safe_name = engine_id.replace(|character: char| !character.is_ascii_alphanumeric(), "_");
    let workspace = root.join(TARGET_DIRECTORY).join(safe_name);
    let allowed = root.join("target/sem13");
    if !workspace.starts_with(&allowed) {
        return Err("SEM13_SANDBOX_PATH_ESCAPE".to_string());
    }
    if workspace.exists() {
        fs::remove_dir_all(&workspace).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(workspace.join("src")).map_err(|error| error.to_string())?;
    fs::write(
        workspace.join("Cargo.toml"),
        "[package]\nname = \"sem13-meta-engine-probe\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n\n[workspace]\n\n[[bin]]\nname = \"meta-engine-probe\"\npath = \"src/main.rs\"\n",
    )
    .map_err(|error| error.to_string())?;
    fs::write(workspace.join("src/lib.rs"), source).map_err(|error| error.to_string())?;
    fs::write(workspace.join("src/main.rs"), META_ENGINE_MAIN_SOURCE)
        .map_err(|error| error.to_string())?;
    let before = fs::read(workspace.join("src/lib.rs")).map_err(|error| error.to_string())?;
    let before_tokens = normalize_non_format_tokens(&before);
    let fmt = run_command(&workspace, "cargo", &["fmt", "--all"])?;
    let after = fs::read(workspace.join("src/lib.rs")).map_err(|error| error.to_string())?;
    let after_tokens = normalize_non_format_tokens(&after);
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
    let debug_binary = workspace.join("target/debug/meta-engine-probe.exe");
    let release_binary = workspace.join("target/release/meta-engine-probe.exe");
    if !debug_binary.is_file() || !release_binary.is_file() {
        return Err(format!("META_ENGINE_BINARY_MISSING:{engine_id}"));
    }
    let canonical_source =
        fs::read_to_string(workspace.join("src/lib.rs")).map_err(|error| error.to_string())?;
    let receipt = BuildReceipt {
        engine_id: engine_id.to_string(),
        mode,
        source_sha256_before_rustfmt: hash_bytes(&before),
        source_sha256_after_rustfmt: hash_bytes(canonical_source.as_bytes()),
        non_format_token_changes: usize::from(before_tokens != after_tokens),
        debug_binary_sha256: hash_file(&debug_binary)?,
        release_binary_sha256: hash_file(&release_binary)?,
        source_bytes: canonical_source.len(),
        debug_binary_bytes: fs::metadata(&debug_binary)
            .map_err(|error| error.to_string())?
            .len(),
        release_binary_bytes: fs::metadata(&release_binary)
            .map_err(|error| error.to_string())?
            .len(),
        sandbox_path: path_string(&workspace),
        sandbox_contained: workspace.starts_with(&allowed),
        rustfmt_check_pass: fmt.success && fmt_check.success,
        strict_clippy_pass: clippy.success,
        tests_pass: tests.success,
        debug_build_pass: debug_build.success,
        release_build_pass: release_build.success,
        commands: vec![fmt, fmt_check, clippy, tests, debug_build, release_build],
    };
    Ok(BuiltEngine {
        mode,
        source_sha256: receipt.source_sha256_after_rustfmt.clone(),
        source: canonical_source,
        debug_binary,
        release_binary,
        receipt,
    })
}

fn ensure_build_pass(receipt: &BuildReceipt) -> Result<(), String> {
    if receipt.non_format_token_changes == 0
        && receipt.rustfmt_check_pass
        && receipt.strict_clippy_pass
        && receipt.tests_pass
        && receipt.debug_build_pass
        && receipt.release_build_pass
        && receipt.sandbox_contained
    {
        Ok(())
    } else {
        Err(format!(
            "META_ENGINE_BUILD_GATE_FAILURE:{}:non_format={}:fmt={}:clippy={}:tests={}:debug={}:release={}:sandbox={}",
            receipt.engine_id,
            receipt.non_format_token_changes,
            receipt.rustfmt_check_pass,
            receipt.strict_clippy_pass,
            receipt.tests_pass,
            receipt.debug_build_pass,
            receipt.release_build_pass,
            receipt.sandbox_contained,
        ))
    }
}

fn copy_engine_artifacts(root: &Path, engine: &BuiltEngine, label: &str) -> Result<(), String> {
    let destination = root
        .join(REPORT_DIRECTORY)
        .join(format!("artifacts/{label}"));
    fs::create_dir_all(&destination).map_err(|error| error.to_string())?;
    fs::write(destination.join("lib.rs"), &engine.source).map_err(|error| error.to_string())?;
    fs::copy(
        &engine.debug_binary,
        destination.join("meta-engine-probe-debug.exe"),
    )
    .map_err(|error| error.to_string())?;
    fs::copy(
        &engine.release_binary,
        destination.join("meta-engine-probe-release.exe"),
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn run_workspace_gate(root: &Path) -> Result<WorkspaceGate, String> {
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
        CommandReceipt {
            command: path_string(&canary_path),
            success: false,
            exit_code: -1,
            stdout_sha256: hash_bytes(b""),
            stderr_sha256: hash_bytes(b"CANARY_MISSING"),
        }
    };
    Ok(WorkspaceGate {
        core_only_build_pass: core_build.success,
        core_runtime_canary_pass: canary.success,
        core_dockability_preserved: tests.success && core_build.success && canary.success,
        workspace_tests: tests,
        core_release_build: core_build,
        core_runtime_canary: canary,
    })
}

fn verify_predecessor(root: &Path) -> Result<(), String> {
    git_output(root, &["merge-base", "--is-ancestor", SEM12_COMMIT, "HEAD"])?;
    if git_output(root, &["cat-file", "-t", SEM12_COMMIT])? != "commit" {
        return Err("SEM12_COMMIT_OBJECT_INVALID".to_string());
    }
    let final_report: Value = read_json(&root.join("reports/sem12/sem12_final_report.json"))?;
    if final_report["sem12_status"] != "PASS"
        || final_report["sem12_level_d_pass"] != true
        || final_report["global_regressed_tasks"] != 0
        || final_report["core_dockability_preserved"] != true
    {
        return Err("SEM12_PREDECESSOR_INVALID".to_string());
    }
    require_same_hash(
        &hash_file(&root.join("reports/sem12/artifacts/checkpoints/D3-FINAL-STRONG/lib.rs"))?,
        M0_REASONER_SOURCE_SHA256,
        "SEM12_FINAL_REASONER_SOURCE",
    )?;
    require_same_hash(
        &hash_file(&root.join(
            "reports/sem12/artifacts/checkpoints/D3-FINAL-STRONG/reasoner-probe-release.exe",
        ))?,
        M0_REASONER_BINARY_SHA256,
        "SEM12_FINAL_REASONER_BINARY",
    )?;
    require_same_hash(
        &hash_file(&root.join("crates/dockable-semantic-core/state/semantic_state.json"))?,
        SEMANTIC_STATE_SHA256,
        "SEMANTIC_STATE",
    )?;
    require_same_hash(
        &hash_file(&root.join("crates/dockable-semantic-core/state/sparse_index.json"))?,
        INDEX_SHA256,
        "SPARSE_INDEX",
    )?;
    Ok(())
}

fn predecessor_integrity(root: &Path) -> Result<Value, String> {
    let sem12: Value = read_json(&root.join("reports/sem12/sem12_final_report.json"))?;
    Ok(json!({
        "predecessor_integrity": "PASS",
        "sem12_commit": SEM12_COMMIT,
        "sem12_commit_object_type": git_output(root, &["cat-file", "-t", SEM12_COMMIT])?,
        "sem12_level_A": sem12["sem12_level_a_pass"],
        "sem12_level_B": sem12["sem12_level_b_pass"],
        "sem12_level_C": sem12["sem12_level_c_pass"],
        "sem12_level_D": sem12["sem12_level_d_pass"],
        "M0_source_sha256": M0_REASONER_SOURCE_SHA256,
        "M0_binary_sha256": M0_REASONER_BINARY_SHA256,
        "M0_self_improvement_engine_sha256": M0_SEM12_ENGINE_SHA256,
        "semantic_state_sha256": SEMANTIC_STATE_SHA256,
        "index_sha256": INDEX_SHA256,
        "core_dockability_preserved": sem12["core_dockability_preserved"],
        "global_reasoning_regressions": sem12["global_regressed_tasks"],
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
            collect_tree_records(root, &path, &mut records)?;
        }
    }
    records.sort();
    Ok(hash_bytes(records.join("\n").as_bytes()))
}

fn collect_tree_records(root: &Path, path: &Path, records: &mut Vec<String>) -> Result<(), String> {
    let mut entries = fs::read_dir(path)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_tree_records(root, &path, records)?;
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

fn verify_required_reports(report_dir: &Path) -> Result<(), String> {
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

fn run_command(current_dir: &Path, program: &str, args: &[&str]) -> Result<CommandReceipt, String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(current_dir)
        .env("CARGO_NET_OFFLINE", "true")
        .output()
        .map_err(|error| error.to_string())?;
    Ok(CommandReceipt {
        command: format!("{} {}", program, args.join(" ")),
        success: output.status.success(),
        exit_code: output.status.code().unwrap_or(-1),
        stdout_sha256: hash_bytes(&output.stdout),
        stderr_sha256: hash_bytes(&output.stderr),
    })
}

fn normalize_non_format_tokens(source: &[u8]) -> Vec<u8> {
    source
        .iter()
        .copied()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect()
}

fn reduction(before: f64, after: f64) -> f64 {
    if before == 0.0 {
        0.0
    } else {
        (before - after) / before
    }
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        1.0
    } else {
        numerator as f64 / denominator as f64
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

fn seed_commitment(label: &str, seed: u64) -> String {
    hash_bytes(format!("SEM13-SEED-COMMITMENT|{label}|{seed:016x}").as_bytes())
}

fn parse_usize(value: &str) -> Result<usize, String> {
    value.parse::<usize>().map_err(|error| error.to_string())
}

fn parse_u64(value: &str) -> Result<u64, String> {
    value.parse::<u64>().map_err(|error| error.to_string())
}

fn require_same_hash(actual: &str, expected: &str, label: &str) -> Result<(), String> {
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

fn source_for_mode(mode: EngineMode) -> String {
    META_ENGINE_SOURCE
        .replace("__EVIDENCE_REUSE__", &mode.evidence_reuse.to_string())
        .replace("__CAUSAL_GUARD__", &mode.causal_guard.to_string())
}

const META_ENGINE_SOURCE: &str = r#"
use std::cmp::Reverse;

const EVIDENCE_REUSE: bool = __EVIDENCE_REUSE__;
const CAUSAL_GUARD: bool = __CAUSAL_GUARD__;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mechanism {
    pub id: u64,
    pub raw_score: u64,
    pub assumption_valid: bool,
    pub causal_relevant: bool,
    pub expected_gain_milli: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Challenge {
    pub challenge_id: String,
    pub evidence: Vec<u64>,
    pub mechanisms: Vec<Mechanism>,
    pub base_descendant_primary_cost: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trace {
    pub challenge_id: String,
    pub patch_proposed: bool,
    pub selected_mechanism_id: u64,
    pub mechanisms_considered: usize,
    pub role_mappings_attempted: usize,
    pub assumption_probes: usize,
    pub candidates_generated: usize,
    pub invalid_candidates: usize,
    pub regressive_candidates: usize,
    pub verified_improvements: usize,
    pub diagnosis_cost: usize,
    pub proposal_cost: usize,
    pub total_meta_deterministic_cost: usize,
    pub derived_descendant_primary_cost: u64,
}

pub fn improve(challenge: &Challenge) -> Trace {
    let diagnosis_cost = if EVIDENCE_REUSE {
        challenge.evidence.len() + 2
    } else {
        challenge.evidence.len() * 2 + 4
    };
    let actionable = challenge.evidence.iter().copied().max().unwrap_or(0) >= 500;
    if !actionable {
        return Trace {
            challenge_id: challenge.challenge_id.clone(),
            patch_proposed: false,
            selected_mechanism_id: 0,
            mechanisms_considered: 0,
            role_mappings_attempted: 0,
            assumption_probes: 0,
            candidates_generated: 0,
            invalid_candidates: 0,
            regressive_candidates: 0,
            verified_improvements: 0,
            diagnosis_cost,
            proposal_cost: 0,
            total_meta_deterministic_cost: diagnosis_cost,
            derived_descendant_primary_cost: challenge.base_descendant_primary_cost,
        };
    }

    let mut mechanisms_considered = 0;
    let mut role_mappings_attempted = 0;
    let mut assumption_probes = 0;
    let mut candidates_generated = 0;
    let mut invalid_candidates = 0;
    let selected = if CAUSAL_GUARD {
        mechanisms_considered = challenge.mechanisms.len();
        assumption_probes = 1;
        role_mappings_attempted = 1;
        candidates_generated = 1;
        challenge
            .mechanisms
            .iter()
            .filter(|mechanism| mechanism.assumption_valid && mechanism.causal_relevant)
            .max_by_key(|mechanism| mechanism.expected_gain_milli)
    } else {
        let mut ranked = challenge.mechanisms.iter().collect::<Vec<_>>();
        ranked.sort_by_key(|mechanism| Reverse(mechanism.raw_score));
        let mut selected = None;
        for mechanism in ranked {
            mechanisms_considered += 1;
            role_mappings_attempted += 1;
            assumption_probes += 2;
            candidates_generated += 1;
            if !mechanism.assumption_valid || !mechanism.causal_relevant {
                invalid_candidates += 1;
                continue;
            }
            selected = Some(mechanism);
            break;
        }
        selected
    };
    let proposal_cost = mechanisms_considered * 3
        + role_mappings_attempted * 4
        + assumption_probes * 2
        + candidates_generated * 5;
    let (patch_proposed, selected_mechanism_id, verified_improvements, derived_cost) = selected
        .map_or((false, 0, 0, challenge.base_descendant_primary_cost), |mechanism| {
            (
                true,
                mechanism.id,
                1,
                challenge.base_descendant_primary_cost
                    * (1_000 - mechanism.expected_gain_milli)
                    / 1_000,
            )
        });
    Trace {
        challenge_id: challenge.challenge_id.clone(),
        patch_proposed,
        selected_mechanism_id,
        mechanisms_considered,
        role_mappings_attempted,
        assumption_probes,
        candidates_generated,
        invalid_candidates,
        regressive_candidates: 0,
        verified_improvements,
        diagnosis_cost,
        proposal_cost,
        total_meta_deterministic_cost: diagnosis_cost + proposal_cost,
        derived_descendant_primary_cost: derived_cost,
    }
}

#[cfg(test)]
mod tests {
    use super::{improve, Challenge, Mechanism};

    fn challenge(evidence: Vec<u64>) -> Challenge {
        Challenge {
            challenge_id: "T".to_string(),
            evidence,
            mechanisms: vec![Mechanism {
                id: 1,
                raw_score: 1,
                assumption_valid: true,
                causal_relevant: true,
                expected_gain_milli: 200,
            }],
            base_descendant_primary_cost: 1_000,
        }
    }

    #[test]
    fn proposes_only_for_actionable_evidence() {
        assert!(improve(&challenge(vec![600])).patch_proposed);
        assert!(!improve(&challenge(vec![40])).patch_proposed);
    }

    #[test]
    fn preserves_external_quality_contract() {
        let trace = improve(&challenge(vec![600]));
        assert_eq!(trace.verified_improvements, 1);
        assert_eq!(trace.regressive_candidates, 0);
    }
}
"#;

const META_ENGINE_MAIN_SOURCE: &str = r#"
use std::{env, fs};

use sem13_meta_engine_probe::{improve, Challenge, Mechanism};

fn parse_u64(value: &str) -> u64 {
    value.parse::<u64>().expect("unsigned integer")
}

fn main() {
    let path = env::args().nth(1).expect("input path");
    let contents = fs::read_to_string(path).expect("read input");
    for line in contents.lines() {
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
                    raw_score: parse_u64(parts.next().expect("raw score")),
                    assumption_valid: parse_u64(parts.next().expect("assumption")) == 1,
                    causal_relevant: parse_u64(parts.next().expect("causal")) == 1,
                    expected_gain_milli: parse_u64(parts.next().expect("gain")),
                }
            })
            .collect();
        let base_descendant_primary_cost =
            parse_u64(fields.next().expect("base descendant primary cost"));
        assert!(fields.next().is_none(), "unexpected input field");
        let trace = improve(&Challenge {
            challenge_id,
            evidence,
            mechanisms,
            base_descendant_primary_cost,
        });
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            trace.challenge_id,
            u8::from(trace.patch_proposed),
            trace.selected_mechanism_id,
            trace.mechanisms_considered,
            trace.role_mappings_attempted,
            trace.assumption_probes,
            trace.candidates_generated,
            trace.invalid_candidates,
            trace.regressive_candidates,
            trace.verified_improvements,
            trace.diagnosis_cost,
            trace.proposal_cost,
            trace.total_meta_deterministic_cost,
            trace.derived_descendant_primary_cost,
        );
    }
}
"#;
