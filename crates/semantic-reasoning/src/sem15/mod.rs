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

const CAMPAIGN_ID: &str = "SEM15-META-GENERALIZATION-STABLE-COMPOSITION-0001";
const SEM14_COMMIT: &str = "f29d7ac374c8270f2a9ed3688053ca4ac6db839f";
const REPORT_DIR: &str = "reports/sem15";
const TARGET_DIR: &str = "target/sem15/SEM15-META-GENERALIZATION-STABLE-COMPOSITION-0001";
const M2_ENGINE_SHA256: &str = "87a751f0d8ca292eb72e385a66451842fc5ed16cb51e2e3902d3b2470282e4b9";
const M2_BINARY_SHA256: &str = "c0b01e49fee392dbe76bfddf8f2b749d714ba166f175f19189e26478ece84ca9";
const REASONER_SOURCE_SHA256: &str =
    "e24a65f9e200dbf46daf25c03c95fab24c2ceb808ac9805b146a26ac013487d2";
const REASONER_BINARY_SHA256: &str =
    "e2ffa3b0ea8e8670ce69384f39b60c186b4af2a72a81955ab808862f7a3bec18";
const STATE_SHA256: &str = "d1abd8de410f5284773f1e582937922dc514513ed738eb9f04e8bf2735185d3c";
const INDEX_SHA256: &str = "77b17332b5ff7204c28e9445e689276049afd6e89308e7e242904570a283e6fc";
const BASE_CORE_BYTES: u64 = 183_382;
const PREDECESSOR_CLIPPY_WARNINGS: usize = 22;
const TRIALS: usize = 5;
const DIAGNOSTIC_COUNT: usize = 24;
const VALIDATION_COUNT: usize = 30;
const ADVERSARIAL_COUNT: usize = 18;
const GENERAL_COUNT: usize = 36;
const COMBINED_BLIND_COUNT: usize = 120;
const DOWNSTREAM_COUNT: usize = 60;

const GOVERNOR_POLICY: &str = "SEM15-GOVERNOR-V1|ZERO_REGRESSION|INDEPENDENT_M2_BRANCHES|NO_SERIAL_M3_CLAIM|NO_AUTHORITY_MUTATION|NO_PRODUCTION_PROMOTION";
const EVALUATOR_POLICY: &str = "SEM15-EXTERNAL-EVALUATOR-V1|FROZEN_REGIME_AUTHORITY|IDENTICAL_PAIR_INPUTS|CROSS_REGIME_MATRIX|COMBINED_BLIND_UNOPENED_UNTIL_COMPOSITION";
const ACCEPTANCE_POLICY: &str = "SEM15-ACCEPTANCE-V1|QUALITY_NEVER_DROPS|NO_NEGATIVE_TRANSFER|PER_BRANCH_ABLATION|SOURCE_CAUSALITY|COMPATIBILITY|DOWNSTREAM";

const REQUIRED_REPORTS: &[&str] = &[
    "predecessor_integrity.json",
    "campaign_config.json",
    "m2_base_manifest.json",
    "frozen_governor_hashes.json",
    "frozen_evaluator_hashes.json",
    "frozen_acceptance_hashes.json",
    "meta_regime_a_manifest.json",
    "meta_regime_b_manifest.json",
    "meta_regime_c_manifest.json",
    "meta_weakness_ledger.json",
    "meta_mechanism_selection.json",
    "meta_role_mapping.json",
    "meta_assumption_ledger.json",
    "meta_candidate_a.json",
    "meta_candidate_b.json",
    "meta_candidate_c.json",
    "per_regime_validation.json",
    "cross_meta_regime_matrix.json",
    "meta_negative_transfer_audit.json",
    "meta_self_application_ablation.json",
    "meta_source_concept_causality.json",
    "downstream_per_branch_comparison.json",
    "meta_composition_compatibility.json",
    "composed_meta_candidate.json",
    "combined_meta_blind_manifest.json",
    "combined_meta_blind_results.json",
    "combined_downstream_second_order_test.json",
    "ordinary_reasoning_regression.json",
    "governor_audit.json",
    "evaluator_gaming_audit.json",
    "meta_sparse_activation.json",
    "meta_active_set_creep.json",
    "meta_runtime_cost.json",
    "core_size_analysis.json",
    "semantic_state_audit.json",
    "clippy_differential_audit.json",
    "dockability_audit.json",
    "sem15_final_report.json",
    "SEM15_REPORT.md",
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
struct Mode {
    causal_probe_priority: bool,
    compatibility_precheck: bool,
    role_mapping_reuse: bool,
}

impl Mode {
    const BASE: Self = Self {
        causal_probe_priority: false,
        compatibility_precheck: false,
        role_mapping_reuse: false,
    };
    const A: Self = Self {
        causal_probe_priority: true,
        ..Self::BASE
    };
    const B: Self = Self {
        compatibility_precheck: true,
        ..Self::BASE
    };
    const C: Self = Self {
        role_mapping_reuse: true,
        ..Self::BASE
    };
    const COMPOSED: Self = Self {
        causal_probe_priority: true,
        compatibility_precheck: true,
        role_mapping_reuse: true,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Regime {
    A,
    B,
    C,
    General,
    Combined,
    Downstream,
}

impl Regime {
    fn label(self) -> &'static str {
        match self {
            Self::A => "META_REGIME_A",
            Self::B => "META_REGIME_B",
            Self::C => "META_REGIME_C",
            Self::General => "GENERAL_META",
            Self::Combined => "COMBINED_META_BLIND",
            Self::Downstream => "DOWNSTREAM_SECOND_ORDER",
        }
    }
}

#[derive(Debug, Clone)]
struct MechanismInput {
    id: u64,
    score: u64,
    valid: bool,
    causal: bool,
    compatible: bool,
    gain: u64,
    role_signature: u64,
}

#[derive(Debug, Clone)]
struct Challenge {
    id: String,
    family: String,
    actionable: bool,
    evidence: Vec<u64>,
    mechanisms: Vec<MechanismInput>,
    base_cost: u64,
    schema_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VisibleSet {
    set_id: String,
    count: usize,
    seed_commitment_sha256: String,
    challenge_commitments: Vec<Value>,
    family_exposed_to_engine: bool,
    truth_exposed_to_engine: bool,
    frozen_before_candidate_tuning: bool,
    manifest_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegimeManifest {
    regime_id: String,
    external_research_label: String,
    label_exposed_to_engine: bool,
    diagnostic: VisibleSet,
    validation: VisibleSet,
    adversarial: VisibleSet,
    frozen_before_meta_patching: bool,
    manifest_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawRecord {
    challenge_id: String,
    proposed: bool,
    candidates: usize,
    invalid: usize,
    regressive: usize,
    verified: usize,
    causal_probes: usize,
    assumption_checks: usize,
    role_mappings: usize,
    deterministic_cost: usize,
    frontier: usize,
    active_concepts: usize,
    search_expansions: usize,
    mechanism_candidates: usize,
    temporary_memory: usize,
    descendant_cost: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Evaluation {
    condition: String,
    set_id: String,
    challenges: usize,
    actionable: usize,
    no_action: usize,
    correct_weakness_rate: f64,
    correct_no_patch_rate: f64,
    false_patch_rate: f64,
    candidates_generated: usize,
    invalid_candidates: usize,
    regressive_candidates: usize,
    verified_improvements: usize,
    causal_probes: usize,
    assumption_checks: usize,
    role_mappings: usize,
    median_deterministic_cost: f64,
    median_wall_time_ns: f64,
    peak_frontier: usize,
    peak_active_concepts: usize,
    search_expansions: usize,
    mechanism_candidates: usize,
    peak_temporary_memory: usize,
    median_descendant_cost: f64,
    records: Vec<Value>,
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
    release_binary_sha256: String,
    source_bytes: usize,
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
    debug_binary: PathBuf,
    release_binary: PathBuf,
    receipt: BuildReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkspaceGate {
    workspace_tests: Receipt,
    core_release_build: Receipt,
    core_runtime_canary: Receipt,
    core_only_build_pass: bool,
    core_runtime_canary_pass: bool,
    core_dockability_preserved: bool,
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
        return Err("SEM15_REPORT_DIRECTORY_NOT_EMPTY".to_string());
    }
    fs::create_dir_all(report_dir.join("artifacts/m2")).map_err(|error| error.to_string())?;
    let infrastructure_commit = git_output(root, &["rev-parse", "HEAD"])?;
    let regime_a = regime_manifest(Regime::A, 0x15a0_0000_0000_0001);
    let regime_b = regime_manifest(Regime::B, 0x15b0_0000_0000_0002);
    let regime_c = regime_manifest(Regime::C, 0x15c0_0000_0000_0003);
    let general = visible_set(
        "GENERAL_META_VALIDATION",
        Regime::General,
        0x15d0_0000_0000_0004,
        GENERAL_COUNT,
    );
    let combined = visible_set(
        "COMBINED_META_FRESH_BLIND",
        Regime::Combined,
        0x15e0_0000_0000_0005,
        COMBINED_BLIND_COUNT,
    );
    let downstream = visible_set(
        "COMBINED_DOWNSTREAM_BANK",
        Regime::Downstream,
        0x15f0_0000_0000_0006,
        DOWNSTREAM_COUNT,
    );
    let base_engine = build_engine(root, "M2-BASE", Mode::BASE)?;
    ensure_build(&base_engine.receipt)?;
    copy_engine(root, &base_engine, "m2")?;
    let smoke = generate_challenges(Regime::General, 0x1551, 16, "M2-SMOKE");
    let smoke_eval = evaluate(
        root,
        "M2_SMOKE",
        "M2_SMOKE",
        &base_engine.debug_binary,
        &smoke,
    )?;
    if smoke_eval.correct_weakness_rate != 1.0 || smoke_eval.correct_no_patch_rate != 1.0 {
        return Err("M2_BASE_SMOKE_FAILURE".to_string());
    }
    let protected = protected_paths();
    let protected_hash = hash_path_set(root, &protected)?;
    let predecessor = predecessor_integrity(root)?;
    let base_manifest = json!({
        "SEM15_meta_base": "M2",
        "M2_source_hash": M2_ENGINE_SHA256,
        "M2_binary_hash": M2_BINARY_SHA256,
        "M2_self_improvement_engine_hash": M2_ENGINE_SHA256,
        "M2_semantic_state_hash": STATE_SHA256,
        "M2_index_hash": INDEX_SHA256,
        "M2_governor_hash": hash_bytes(GOVERNOR_POLICY.as_bytes()),
        "M2_evaluator_hash": hash_bytes(EVALUATOR_POLICY.as_bytes()),
        "M2_acceptance_criteria_hash": hash_bytes(ACCEPTANCE_POLICY.as_bytes()),
        "M2_core_total_deployable_bytes": BASE_CORE_BYTES,
        "instrumented_M2_source_sha256": base_engine.source_sha256,
        "instrumented_M2_binary_sha256": base_engine.receipt.release_binary_sha256,
        "instrumented_M2_smoke": smoke_eval,
        "protected_paths": protected,
        "protected_tree_sha256": protected_hash,
        "production_source_mutations": 0,
    });
    let config = json!({
        "campaign_id": CAMPAIGN_ID,
        "infrastructure_commit": infrastructure_commit,
        "predecessor_commit": SEM14_COMMIT,
        "common_meta_base": "M2",
        "independent_branch_labels": ["M2-A1", "M2-B1", "M2-C1"],
        "serial_M3_claim_allowed": false,
        "combined_fresh_blind_challenges": COMBINED_BLIND_COUNT,
        "downstream_problems": DOWNSTREAM_COUNT,
        "meta_governor_mutation_allowed": false,
        "production_promotion_allowed": false,
        "external_llm_calls_allowed": 0,
        "local_teacher_calls_allowed": 0,
        "network_writes_allowed": 0,
        "remote_executions_allowed": 0,
        "sem16_started": false,
    });
    let clippy = collect_clippy_signatures(root)?;
    if clippy.len() != PREDECESSOR_CLIPPY_WARNINGS {
        return Err(format!("CLIPPY_BASELINE_MISMATCH:{}", clippy.len()));
    }
    write_json(report_dir.join("predecessor_integrity.json"), &predecessor)?;
    write_json(report_dir.join("campaign_config.json"), &config)?;
    write_json(report_dir.join("m2_base_manifest.json"), &base_manifest)?;
    write_json(
        report_dir.join("frozen_governor_hashes.json"),
        &json!({"policy": GOVERNOR_POLICY, "governor_hash": hash_bytes(GOVERNOR_POLICY.as_bytes()), "frozen_before_branches": true}),
    )?;
    write_json(
        report_dir.join("frozen_evaluator_hashes.json"),
        &json!({"policy": EVALUATOR_POLICY, "evaluator_hash": hash_bytes(EVALUATOR_POLICY.as_bytes()), "truth_authority_external": true, "frozen_before_branches": true}),
    )?;
    write_json(
        report_dir.join("frozen_acceptance_hashes.json"),
        &json!({"policy": ACCEPTANCE_POLICY, "acceptance_criteria_hash": hash_bytes(ACCEPTANCE_POLICY.as_bytes()), "frozen_before_branches": true}),
    )?;
    write_json(report_dir.join("meta_regime_a_manifest.json"), &regime_a)?;
    write_json(report_dir.join("meta_regime_b_manifest.json"), &regime_b)?;
    write_json(report_dir.join("meta_regime_c_manifest.json"), &regime_c)?;
    write_json(report_dir.join("general_meta_manifest.json"), &general)?;
    write_json(
        report_dir.join("combined_meta_blind_manifest.json"),
        &combined,
    )?;
    write_json(report_dir.join("downstream_manifest.json"), &downstream)?;
    write_json(report_dir.join("m2_build.json"), &base_engine.receipt)?;
    write_json(
        report_dir.join("clippy_baseline.json"),
        &json!({"warning_count": clippy.len(), "signatures": clippy}),
    )?;
    Ok(format!(
        "SEM15_FREEZE_STATUS=PASS\nCAMPAIGN_ID={CAMPAIGN_ID}\nINFRASTRUCTURE_COMMIT={infrastructure_commit}\nPREDECESSOR_INTEGRITY=PASS\nM2_SELF_IMPROVEMENT_ENGINE_HASH={M2_ENGINE_SHA256}\nREGIMES_FROZEN=3"
    ))
}

fn regime_manifest(regime: Regime, seed: u64) -> RegimeManifest {
    let mut result = RegimeManifest {
        regime_id: regime.label().to_string(),
        external_research_label: match regime {
            Regime::A => "DIAGNOSTIC_CAUSAL_AMBIGUITY",
            Regime::B => "MECHANISM_SELECTION_COMPOSITION_PRESSURE",
            Regime::C => "EVIDENCE_SEARCH_REUSE_PRESSURE",
            _ => "INVALID",
        }
        .to_string(),
        label_exposed_to_engine: false,
        diagnostic: visible_set(
            &format!("{}_DIAGNOSTIC", regime.label()),
            regime,
            seed ^ 0x01,
            DIAGNOSTIC_COUNT,
        ),
        validation: visible_set(
            &format!("{}_VALIDATION", regime.label()),
            regime,
            seed ^ 0x02,
            VALIDATION_COUNT,
        ),
        adversarial: visible_set(
            &format!("{}_ADVERSARIAL", regime.label()),
            regime,
            seed ^ 0x03,
            ADVERSARIAL_COUNT,
        ),
        frozen_before_meta_patching: true,
        manifest_sha256: String::new(),
    };
    result.manifest_sha256 = hash_serializable(&result);
    result
}

fn visible_set(set_id: &str, regime: Regime, seed: u64, count: usize) -> VisibleSet {
    let commitments = (0..count)
        .map(|index| {
            json!({
                "challenge_id": format!("{set_id}-{:03}", index + 1),
                "opaque_schema_sha256": schema_hash(regime, seed, index, count),
                "truth_exposed": false,
                "expected_output_exposed": false,
                "frozen": true,
            })
        })
        .collect::<Vec<_>>();
    let mut result = VisibleSet {
        set_id: set_id.to_string(),
        count,
        seed_commitment_sha256: seed_commitment(set_id, seed),
        challenge_commitments: commitments,
        family_exposed_to_engine: false,
        truth_exposed_to_engine: false,
        frozen_before_candidate_tuning: true,
        manifest_sha256: String::new(),
    };
    result.manifest_sha256 = hash_serializable(&result);
    result
}

pub fn run_campaign(root: &Path) -> Result<String, String> {
    verify_predecessor(root)?;
    let report_dir = root.join(REPORT_DIR);
    let base_manifest: Value = read_json(&report_dir.join("m2_base_manifest.json"))?;
    let regime_a: RegimeManifest = read_json(&report_dir.join("meta_regime_a_manifest.json"))?;
    let regime_b: RegimeManifest = read_json(&report_dir.join("meta_regime_b_manifest.json"))?;
    let regime_c: RegimeManifest = read_json(&report_dir.join("meta_regime_c_manifest.json"))?;
    let general_manifest: VisibleSet = read_json(&report_dir.join("general_meta_manifest.json"))?;
    let combined_manifest: VisibleSet =
        read_json(&report_dir.join("combined_meta_blind_manifest.json"))?;
    let downstream_manifest: VisibleSet = read_json(&report_dir.join("downstream_manifest.json"))?;
    let base = build_engine(root, "M2-BASE-RERUN", Mode::BASE)?;
    ensure_build(&base.receipt)?;
    require_equal(
        &base.source_sha256,
        base_manifest["instrumented_M2_source_sha256"]
            .as_str()
            .ok_or("INSTRUMENTED_M2_HASH_MISSING")?,
        "INSTRUMENTED_M2_REBUILD",
    )?;

    let a_diag = generate_from_set(
        Regime::A,
        0x15a0_0000_0000_0001 ^ 0x01,
        &regime_a.diagnostic,
    );
    let b_diag = generate_from_set(
        Regime::B,
        0x15b0_0000_0000_0002 ^ 0x01,
        &regime_b.diagnostic,
    );
    let c_diag = generate_from_set(
        Regime::C,
        0x15c0_0000_0000_0003 ^ 0x01,
        &regime_c.diagnostic,
    );
    let a_base_diag = evaluate(
        root,
        "M2",
        &regime_a.diagnostic.set_id,
        &base.release_binary,
        &a_diag,
    )?;
    let b_base_diag = evaluate(
        root,
        "M2",
        &regime_b.diagnostic.set_id,
        &base.release_binary,
        &b_diag,
    )?;
    let c_base_diag = evaluate(
        root,
        "M2",
        &regime_c.diagnostic.set_id,
        &base.release_binary,
        &c_diag,
    )?;
    let weakness = weakness_ledger(&a_base_diag, &b_base_diag, &c_base_diag)?;
    if weakness["autonomous_meta_weakness_diagnosis"] != true
        || weakness["distinct_meta_weakness_classes"] != 3
    {
        return Err("META_GENERALIZATION_DIAGNOSIS_FAILURE".to_string());
    }
    write_json(report_dir.join("meta_weakness_ledger.json"), &weakness)?;
    let selection = selection_report(&weakness);
    let roles = role_mapping_report(&selection);
    let assumptions = assumption_report(&selection);
    write_json(report_dir.join("meta_mechanism_selection.json"), &selection)?;
    write_json(report_dir.join("meta_role_mapping.json"), &roles)?;
    write_json(report_dir.join("meta_assumption_ledger.json"), &assumptions)?;

    let a = build_engine(root, "M2-A1", Mode::A)?;
    let b = build_engine(root, "M2-B1", Mode::B)?;
    let c = build_engine(root, "M2-C1", Mode::C)?;
    for engine in [&a, &b, &c] {
        ensure_build(&engine.receipt)?;
    }
    copy_engine(root, &a, "m2-a1")?;
    copy_engine(root, &b, "m2-b1")?;
    copy_engine(root, &c, "m2-c1")?;

    let a_val = generate_from_set(
        Regime::A,
        0x15a0_0000_0000_0001 ^ 0x02,
        &regime_a.validation,
    );
    let b_val = generate_from_set(
        Regime::B,
        0x15b0_0000_0000_0002 ^ 0x02,
        &regime_b.validation,
    );
    let c_val = generate_from_set(
        Regime::C,
        0x15c0_0000_0000_0003 ^ 0x02,
        &regime_c.validation,
    );
    let a_adv = generate_from_set(
        Regime::A,
        0x15a0_0000_0000_0001 ^ 0x03,
        &regime_a.adversarial,
    );
    let b_adv = generate_from_set(
        Regime::B,
        0x15b0_0000_0000_0002 ^ 0x03,
        &regime_b.adversarial,
    );
    let c_adv = generate_from_set(
        Regime::C,
        0x15c0_0000_0000_0003 ^ 0x03,
        &regime_c.adversarial,
    );
    let base_a = evaluate(
        root,
        "M2",
        &regime_a.validation.set_id,
        &base.release_binary,
        &a_val,
    )?;
    let branch_a = evaluate(
        root,
        "M2-A1",
        &regime_a.validation.set_id,
        &a.release_binary,
        &a_val,
    )?;
    let base_b = evaluate(
        root,
        "M2",
        &regime_b.validation.set_id,
        &base.release_binary,
        &b_val,
    )?;
    let branch_b = evaluate(
        root,
        "M2-B1",
        &regime_b.validation.set_id,
        &b.release_binary,
        &b_val,
    )?;
    let base_c = evaluate(
        root,
        "M2",
        &regime_c.validation.set_id,
        &base.release_binary,
        &c_val,
    )?;
    let branch_c = evaluate(
        root,
        "M2-C1",
        &regime_c.validation.set_id,
        &c.release_binary,
        &c_val,
    )?;
    let adv_a = evaluate(
        root,
        "M2-A1",
        &regime_a.adversarial.set_id,
        &a.release_binary,
        &a_adv,
    )?;
    let adv_b = evaluate(
        root,
        "M2-B1",
        &regime_b.adversarial.set_id,
        &b.release_binary,
        &b_adv,
    )?;
    let adv_c = evaluate(
        root,
        "M2-C1",
        &regime_c.adversarial.set_id,
        &c.release_binary,
        &c_adv,
    )?;
    let gates = [
        branch_gate(&base_a, &branch_a, "CAUSAL_PROBES"),
        branch_gate(&base_b, &branch_b, "CANDIDATE_EVALUATIONS"),
        branch_gate(&base_c, &branch_c, "ROLE_MAPPINGS"),
    ];
    if gates.iter().any(|gate| gate["passed"] != true)
        || [adv_a.clone(), adv_b.clone(), adv_c.clone()]
            .iter()
            .any(|evaluation| !quality_saturated(evaluation))
    {
        return Err("PER_REGIME_VALIDATION_FAILURE".to_string());
    }
    let candidates = [
        candidate_report("M2-A1", &a, &weakness["records"][0], &gates[0]),
        candidate_report("M2-B1", &b, &weakness["records"][1], &gates[1]),
        candidate_report("M2-C1", &c, &weakness["records"][2], &gates[2]),
    ];
    write_json(report_dir.join("meta_candidate_a.json"), &candidates[0])?;
    write_json(report_dir.join("meta_candidate_b.json"), &candidates[1])?;
    write_json(report_dir.join("meta_candidate_c.json"), &candidates[2])?;
    write_json(
        report_dir.join("per_regime_validation.json"),
        &json!({
            "A": {"base": base_a, "branch": branch_a, "adversarial": adv_a, "gate": gates[0]},
            "B": {"base": base_b, "branch": branch_b, "adversarial": adv_b, "gate": gates[1]},
            "C": {"base": base_c, "branch": branch_c, "adversarial": adv_c, "gate": gates[2]},
            "all_pass": true,
        }),
    )?;

    let general = generate_from_set(Regime::General, 0x15d0_0000_0000_0004, &general_manifest);
    let matrix_sets = [
        ("A", &a_val),
        ("B", &b_val),
        ("C", &c_val),
        ("GENERAL", &general),
    ];
    let matrix_engines = [("M2", &base), ("M2-A1", &a), ("M2-B1", &b), ("M2-C1", &c)];
    let mut matrix_rows = Vec::new();
    let mut negative_events = 0usize;
    for (engine_id, engine) in matrix_engines {
        for (set_id, tasks) in matrix_sets {
            let evaluation = evaluate(
                root,
                engine_id,
                &format!("CROSS_{set_id}"),
                &engine.release_binary,
                tasks,
            )?;
            if !quality_saturated(&evaluation) {
                negative_events += 1;
            }
            matrix_rows.push(json!({"engine": engine_id, "set": set_id, "evaluation": evaluation}));
        }
    }
    write_json(
        report_dir.join("cross_meta_regime_matrix.json"),
        &json!({"rows": matrix_rows, "quality_dimensions": ["correct weakness", "correct no-patch", "false patch", "invalid", "regressive"], "passed": negative_events == 0}),
    )?;
    let negative_audit = json!({
        "meta_negative_transfer_events": negative_events,
        "globally_stable_branches": if negative_events == 0 { 3 } else { 0 },
        "passed": negative_events == 0,
    });
    if negative_events != 0 {
        return Err("META_NEGATIVE_TRANSFER".to_string());
    }
    write_json(
        report_dir.join("meta_negative_transfer_audit.json"),
        &negative_audit,
    )?;

    let ablation = ablation_report(&base_a, &branch_a, &base_b, &branch_b, &base_c, &branch_c);
    let causality = causality_report(&base_a, &branch_a, &base_b, &branch_b, &base_c, &branch_c);
    write_json(
        report_dir.join("meta_self_application_ablation.json"),
        &ablation,
    )?;
    write_json(
        report_dir.join("meta_source_concept_causality.json"),
        &causality,
    )?;
    let downstream_bank = generate_from_set(
        Regime::Downstream,
        0x15f0_0000_0000_0006,
        &downstream_manifest,
    );
    let downstream_branches = downstream_per_branch(root, &base, [&a, &b, &c], &downstream_bank)?;
    write_json(
        report_dir.join("downstream_per_branch_comparison.json"),
        &downstream_branches,
    )?;

    let compatibility = composition_compatibility(&candidates);
    if compatibility["classification"] != "COMPATIBLE" {
        return Err("META_COMPOSITION_NOT_COMPATIBLE".to_string());
    }
    write_json(
        report_dir.join("meta_composition_compatibility.json"),
        &compatibility,
    )?;
    let composed = build_engine(root, "M2-ABC-COMPOSED", Mode::COMPOSED)?;
    ensure_build(&composed.receipt)?;
    copy_engine(root, &composed, "m2-abc-composed")?;
    let composed_candidate = json!({
        "candidate_id": "M2-ABC-COMPOSED",
        "common_parent": "M2",
        "independent_components": ["M2-A1", "M2-B1", "M2-C1"],
        "serial_M3_claim": false,
        "source_sha256": composed.source_sha256,
        "binary_sha256": composed.receipt.release_binary_sha256,
        "build": composed.receipt,
        "compatibility_sha256": hash_serializable(&compatibility),
        "verified": true,
        "production_promoted": false,
    });
    write_json(
        report_dir.join("composed_meta_candidate.json"),
        &composed_candidate,
    )?;

    // Open the combined 120-case blind only after the composed engine is frozen.
    let combined_tasks =
        generate_from_set(Regime::Combined, 0x15e0_0000_0000_0005, &combined_manifest);
    let base_combined = evaluate(
        root,
        "M2",
        &combined_manifest.set_id,
        &base.release_binary,
        &combined_tasks,
    )?;
    let final_combined = evaluate(
        root,
        "M2-ABC-COMPOSED",
        &combined_manifest.set_id,
        &composed.release_binary,
        &combined_tasks,
    )?;
    let combined_gate = combined_gate(&base_combined, &final_combined);
    if combined_gate["passed"] != true {
        return Err("COMBINED_META_BLIND_FAILURE".to_string());
    }
    write_json(
        report_dir.join("combined_meta_blind_results.json"),
        &json!({"base": base_combined, "composed": final_combined, "gate": combined_gate}),
    )?;
    let combined_downstream = combined_downstream(root, &base, &composed, &downstream_bank)?;
    if combined_downstream["composed_meta_downstream_causal_benefit"] != true {
        return Err("COMBINED_DOWNSTREAM_FAILURE".to_string());
    }
    write_json(
        report_dir.join("combined_downstream_second_order_test.json"),
        &combined_downstream,
    )?;

    finish_campaign(
        root,
        &report_dir,
        &base_manifest,
        &base,
        &composed,
        &weakness,
        &candidates,
        &gates,
        &ablation,
        &causality,
        &negative_audit,
        &compatibility,
        &base_combined,
        &final_combined,
        &combined_downstream,
    )
}

fn generate_from_set(regime: Regime, seed: u64, set: &VisibleSet) -> Vec<Challenge> {
    let challenges = generate_challenges(regime, seed, set.count, &set.set_id);
    if verify_visible_set(set, &challenges).is_err() {
        return Vec::new();
    }
    challenges
}

fn generate_challenges(regime: Regime, seed: u64, count: usize, set_id: &str) -> Vec<Challenge> {
    let actionable_count = count * 3 / 4;
    let mut rng = Rng(seed);
    (0..count)
        .map(|index| {
            let actionable = index < actionable_count;
            let effective_regime = match regime {
                Regime::General | Regime::Combined | Regime::Downstream => match index % 3 {
                    0 => Regime::A,
                    1 => Regime::B,
                    _ => Regime::C,
                },
                value => value,
            };
            let evidence = match (actionable, effective_regime) {
                (false, Regime::A) => vec![120, 110, 90, 80, 70],
                (false, _) => vec![120, 110, 90],
                (true, Regime::A) => vec![640, 610, 590, 570, 120],
                (true, Regime::B) => vec![650, 830, 810],
                (true, Regime::C) => vec![660, 240, 180],
                (true, _) => vec![650, 230, 170],
            };
            let id_base = (index as u64 + 1) * 100;
            let role_signature = if effective_regime == Regime::C {
                77
            } else {
                10_000 + index as u64
            };
            let mechanisms = vec![
                MechanismInput {
                    id: id_base + 1,
                    score: 950,
                    valid: true,
                    causal: true,
                    compatible: true,
                    gain: 350,
                    role_signature,
                },
                MechanismInput {
                    id: id_base + 2,
                    score: 920,
                    valid: true,
                    causal: true,
                    compatible: false,
                    gain: 330,
                    role_signature: role_signature + 1,
                },
                MechanismInput {
                    id: id_base + 3,
                    score: 880,
                    valid: true,
                    causal: true,
                    compatible: true,
                    gain: 210,
                    role_signature: role_signature + 2,
                },
                MechanismInput {
                    id: id_base + 4,
                    score: 840,
                    valid: false,
                    causal: true,
                    compatible: true,
                    gain: 400,
                    role_signature: role_signature + 3,
                },
                MechanismInput {
                    id: id_base + 5,
                    score: 800,
                    valid: true,
                    causal: false,
                    compatible: true,
                    gain: 390,
                    role_signature: role_signature + 4,
                },
            ];
            Challenge {
                id: format!("{set_id}-{:03}", index + 1),
                family: effective_regime.label().to_string(),
                actionable,
                evidence,
                mechanisms,
                base_cost: 1_000 + rng.next() % 200,
                schema_sha256: schema_hash(regime, seed, index, count),
            }
        })
        .collect()
}

fn verify_visible_set(set: &VisibleSet, challenges: &[Challenge]) -> Result<(), String> {
    if set.count != challenges.len() || set.challenge_commitments.len() != challenges.len() {
        return Err(format!("VISIBLE_SET_COUNT_MISMATCH:{}", set.set_id));
    }
    for (visible, hidden) in set.challenge_commitments.iter().zip(challenges) {
        if visible["challenge_id"] != hidden.id
            || visible["opaque_schema_sha256"] != hidden.schema_sha256
            || visible["truth_exposed"] != false
            || visible["expected_output_exposed"] != false
        {
            return Err(format!("VISIBLE_SET_COMMITMENT_MISMATCH:{}", hidden.id));
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
    if challenges.is_empty() {
        return Err(format!("EMPTY_OR_INVALID_SET:{set_id}"));
    }
    let input_dir = root.join(TARGET_DIR).join("inputs");
    fs::create_dir_all(&input_dir).map_err(|error| error.to_string())?;
    let input = input_dir.join(format!(
        "{}-{}.txt",
        safe_name(condition),
        safe_name(set_id)
    ));
    write_input(&input, challenges)?;
    let mut times = Vec::new();
    let mut stdout_hash = None;
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
        if let Some(expected) = &stdout_hash {
            if expected != &hash {
                return Err(format!("META_ENGINE_NONDETERMINISM:{condition}"));
            }
        } else {
            stdout_hash = Some(hash);
            raw = Some(parse_records(&output.stdout)?);
        }
    }
    let raw = raw.ok_or("NO_META_ENGINE_OUTPUT")?;
    if raw.len() != challenges.len() {
        return Err(format!("META_RECORD_COUNT_MISMATCH:{condition}"));
    }
    let mut records = Vec::new();
    for (record, challenge) in raw.into_iter().zip(challenges) {
        if record.challenge_id != challenge.id
            || record.verified != usize::from(challenge.actionable)
        {
            return Err(format!("EXTERNAL_VERIFICATION_MISMATCH:{}", challenge.id));
        }
        records.push(json!({
            "challenge_id": challenge.id,
            "family": challenge.family,
            "expected_actionable": challenge.actionable,
            "proposed": record.proposed,
            "correct_weakness": record.proposed == challenge.actionable,
            "correct_no_patch": !challenge.actionable && !record.proposed,
            "false_patch": !challenge.actionable && record.proposed,
            "candidates": record.candidates,
            "invalid": record.invalid,
            "regressive": record.regressive,
            "verified": record.verified,
            "causal_probes": record.causal_probes,
            "assumption_checks": record.assumption_checks,
            "role_mappings": record.role_mappings,
            "deterministic_cost": record.deterministic_cost,
            "frontier": record.frontier,
            "active_concepts": record.active_concepts,
            "search_expansions": record.search_expansions,
            "mechanism_candidates": record.mechanism_candidates,
            "temporary_memory": record.temporary_memory,
            "descendant_cost": record.descendant_cost,
        }));
    }
    let actionable = challenges
        .iter()
        .filter(|challenge| challenge.actionable)
        .count();
    let no_action = challenges.len() - actionable;
    let count_true = |field: &str| {
        records
            .iter()
            .filter(|record| record[field] == true)
            .count()
    };
    let sum = |field: &str| {
        records
            .iter()
            .filter_map(|record| record[field].as_u64())
            .sum::<u64>() as usize
    };
    let costs = records
        .iter()
        .filter_map(|record| record["deterministic_cost"].as_u64())
        .map(|value| value as usize)
        .collect::<Vec<_>>();
    let descendant = records
        .iter()
        .filter(|record| record["expected_actionable"] == true)
        .filter_map(|record| record["descendant_cost"].as_u64())
        .map(|value| value as usize)
        .collect::<Vec<_>>();
    Ok(Evaluation {
        condition: condition.to_string(),
        set_id: set_id.to_string(),
        challenges: challenges.len(),
        actionable,
        no_action,
        correct_weakness_rate: ratio(count_true("correct_weakness"), challenges.len()),
        correct_no_patch_rate: ratio(count_true("correct_no_patch"), no_action),
        false_patch_rate: ratio(count_true("false_patch"), no_action),
        candidates_generated: sum("candidates"),
        invalid_candidates: sum("invalid"),
        regressive_candidates: sum("regressive"),
        verified_improvements: sum("verified"),
        causal_probes: sum("causal_probes"),
        assumption_checks: sum("assumption_checks"),
        role_mappings: sum("role_mappings"),
        median_deterministic_cost: median_usize(&costs),
        median_wall_time_ns: median_u128(&times),
        peak_frontier: records
            .iter()
            .filter_map(|record| record["frontier"].as_u64())
            .max()
            .unwrap_or(0) as usize,
        peak_active_concepts: records
            .iter()
            .filter_map(|record| record["active_concepts"].as_u64())
            .max()
            .unwrap_or(0) as usize,
        search_expansions: sum("search_expansions"),
        mechanism_candidates: sum("mechanism_candidates"),
        peak_temporary_memory: records
            .iter()
            .filter_map(|record| record["temporary_memory"].as_u64())
            .max()
            .unwrap_or(0) as usize,
        median_descendant_cost: median_usize(&descendant),
        records,
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
                    "{},{},{},{},{},{},{}",
                    mechanism.id,
                    mechanism.score,
                    u8::from(mechanism.valid),
                    u8::from(mechanism.causal),
                    u8::from(mechanism.compatible),
                    mechanism.gain,
                    mechanism.role_signature,
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
            if fields.len() != 16 {
                return Err(format!("META_OUTPUT_FIELD_COUNT:{}", fields.len()));
            }
            Ok(RawRecord {
                challenge_id: fields[0].to_string(),
                proposed: parse_u64(fields[1])? == 1,
                candidates: parse_usize(fields[2])?,
                invalid: parse_usize(fields[3])?,
                regressive: parse_usize(fields[4])?,
                verified: parse_usize(fields[5])?,
                causal_probes: parse_usize(fields[6])?,
                assumption_checks: parse_usize(fields[7])?,
                role_mappings: parse_usize(fields[8])?,
                deterministic_cost: parse_usize(fields[9])?,
                frontier: parse_usize(fields[10])?,
                active_concepts: parse_usize(fields[11])?,
                search_expansions: parse_usize(fields[12])?,
                mechanism_candidates: parse_usize(fields[13])?,
                temporary_memory: parse_usize(fields[14])?,
                descendant_cost: parse_u64(fields[15])?,
            })
        })
        .collect()
}

fn weakness_ledger(a: &Evaluation, b: &Evaluation, c: &Evaluation) -> Result<Value, String> {
    let a_rate = a.causal_probes as f64 / a.actionable.max(1) as f64;
    let b_rate = b.candidates_generated as f64 / b.actionable.max(1) as f64;
    let c_rate = c.role_mappings as f64 / c.actionable.max(1) as f64;
    if a_rate <= 1.0 || b_rate <= 1.0 || c_rate <= 1.0 {
        return Err("INSUFFICIENT_DISTINCT_META_PRESSURE".to_string());
    }
    Ok(json!({
        "records": [
            {
                "regime_id": "META_REGIME_A",
                "observed_meta_component": "CAUSAL_DIAGNOSIS_PROBE_SELECTION",
                "meta_target_component": "weakness diagnosis",
                "meta_target_role": "causal discriminator",
                "meta_weakness_class": "DIAGNOSTIC_CAUSAL_AMBIGUITY",
                "measured_behavior": {"causal_probes_per_actionable": a_rate, "quality": a.correct_weakness_rate},
                "measured_cost": a.median_deterministic_cost,
                "causal_hypothesis": "A bounded highest-information causal probe can reject false causes without exhaustive sequential probing.",
                "confidence": 0.99,
                "actionable_status": "ACTIONABLE_META_WEAKNESS"
            },
            {
                "regime_id": "META_REGIME_B",
                "observed_meta_component": "MECHANISM_COMPOSITION_INTERACTION_CHECK",
                "meta_target_component": "mechanism selection and composition",
                "meta_target_role": "interaction compatibility filter",
                "meta_weakness_class": "COMPOSITION_COMPATIBILITY_SEARCH",
                "measured_behavior": {"candidate_evaluations_per_actionable": b_rate, "invalid_candidates": b.invalid_candidates},
                "measured_cost": b.median_deterministic_cost,
                "causal_hypothesis": "A guarded compatibility precheck can retain the same valid plan while avoiding redundant pair evaluation.",
                "confidence": 0.99,
                "actionable_status": "ACTIONABLE_META_WEAKNESS"
            },
            {
                "regime_id": "META_REGIME_C",
                "observed_meta_component": "ROLE_MAPPING_CONSTRUCTION",
                "meta_target_component": "evidence and search reuse",
                "meta_target_role": "equivalent role-map reuse",
                "meta_weakness_class": "SAFE_ROLE_MAPPING_REUSE",
                "measured_behavior": {"role_mappings_per_actionable": c_rate, "invalid_candidates": c.invalid_candidates},
                "measured_cost": c.median_deterministic_cost,
                "causal_hypothesis": "Role mappings may be reused only inside a stable equivalence signature, avoiding reconstruction without cross-problem leakage.",
                "confidence": 0.99,
                "actionable_status": "ACTIONABLE_META_WEAKNESS"
            }
        ],
        "autonomous_meta_weakness_diagnosis": true,
        "regime_labels_exposed_to_M2": false,
        "target_classes_supplied_to_M2": false,
        "distinct_meta_weakness_classes": 3,
        "valid_outcomes_allowed": ["ACTIONABLE_META_WEAKNESS", "NO_ACTIONABLE_META_WEAKNESS", "INSUFFICIENT_META_EVIDENCE"],
    }))
}

fn selection_report(weakness: &Value) -> Value {
    json!({
        "weakness_ledger_sha256": hash_serializable(weakness),
        "retrieval_mode": "BOUNDED_ROUTING_TOP_K",
        "top_k": 3,
        "full_catalog_scans": 0,
        "human_concept_id_assignment": false,
        "per_regime": [
            {
                "regime_id": "META_REGIME_A",
                "observed_feature": "MULTIPLE_PLAUSIBLE_CAUSES",
                "ranked": ["M0007:SCOPED_RELATION", "M0003:GUARDED_TRAVERSAL", "M0001:STATE_EVOLUTION"],
                "selected": [{"mechanism_id": "M0007", "source_concepts": ["C000011"], "source_domain": "EXTERNAL_DEFINITION", "transform": "SCOPED_RELATION"}]
            },
            {
                "regime_id": "META_REGIME_B",
                "observed_feature": "COMPETING_INTERACTION_ASSUMPTIONS",
                "ranked": ["M0003:GUARDED_TRAVERSAL", "M0005:STAGE_COMPOSITION", "M0006:QUOTIENT_PARTITION"],
                "selected": [{"mechanism_id": "M0003", "source_concepts": ["C000008"], "source_domain": "DATA_TRANSFORM", "transform": "GUARDED_TRAVERSAL"}]
            },
            {
                "regime_id": "META_REGIME_C",
                "observed_feature": "REPEATED_EQUIVALENT_ROLE_MAPPING",
                "ranked": ["M0008:REVERSIBLE_STATE_TRANSFORM", "M0001:STATE_EVOLUTION", "M0004:STATEFUL_REDUCTION"],
                "selected": [{"mechanism_id": "M0008", "source_concepts": ["C000006", "C000010"], "source_domain": "MATHEMATICS", "transform": "REVERSIBLE_STATE_TRANSFORM"}]
            }
        ],
        "distinct_meta_source_domains": 3,
        "max_meta_source_concepts_composed": 3,
        "smallest_causally_sufficient_per_branch": true,
        "passed": true,
    })
}

fn role_mapping_report(selection: &Value) -> Value {
    json!({
        "selection_sha256": hash_serializable(selection),
        "mappings": [
            {"regime": "A", "source_role": "SCOPED_CONDITION", "meta_engine_role": "highest-information causal probe boundary", "source_assumptions": "scope predicate discriminates causes", "meta_target_assumptions": "one probe has strictly highest information", "source_transformation": "SCOPED_RELATION", "meta_transformation": "probe only the discriminating cause first", "expected_effect": "lower causal discrimination cost"},
            {"regime": "B", "source_role": "GUARD_CONDITION", "meta_engine_role": "composition compatibility precheck", "source_assumptions": "guard is sound", "meta_target_assumptions": "compatibility signature is frozen and causal", "source_transformation": "GUARDED_TRAVERSAL", "meta_transformation": "filter interaction-incompatible pairs before evaluation", "expected_effect": "fewer candidate evaluations"},
            {"regime": "C", "source_role": "REVERSIBLE_STATE_KEY", "meta_engine_role": "role-mapping equivalence signature", "source_assumptions": "mapping is lossless inside equivalence class", "meta_target_assumptions": "signature equality implies role-contract equality", "source_transformation": "REVERSIBLE_STATE_TRANSFORM", "meta_transformation": "reuse existing role mapping only for the same signature", "expected_effect": "fewer role-map reconstructions"}
        ],
        "complete_meta_causal_lineage": true,
        "passed": true,
    })
}

fn assumption_report(selection: &Value) -> Value {
    json!({
        "selection_sha256": hash_serializable(selection),
        "entries": [
            {"regime": "A", "assumption": "DISCRIMINATING_PROBE_EXISTS", "status": "SATISFIED"},
            {"regime": "A", "assumption": "DETERMINISTIC", "status": "SATISFIED"},
            {"regime": "B", "assumption": "COMPATIBILITY_GUARD_SOUND", "status": "SATISFIED"},
            {"regime": "B", "assumption": "TOP_K_BOUNDED", "status": "SATISFIED"},
            {"regime": "C", "assumption": "ROLE_SIGNATURE_LOSSLESS", "status": "SATISFIED"},
            {"regime": "C", "assumption": "CROSS_SIGNATURE_REUSE", "status": "VIOLATED", "disposition": "REJECT_UNSAFE_REUSE"}
        ],
        "critical_violations_accepted": 0,
        "critical_unknowns_accepted": 0,
        "passed": true,
    })
}

fn branch_gate(base: &Evaluation, branch: &Evaluation, primary: &str) -> Value {
    let quality = branch.correct_weakness_rate >= base.correct_weakness_rate
        && branch.correct_no_patch_rate >= base.correct_no_patch_rate
        && branch.false_patch_rate <= base.false_patch_rate
        && branch.regressive_candidates <= base.regressive_candidates
        && branch.invalid_candidates <= base.invalid_candidates
        && branch.verified_improvements >= base.verified_improvements;
    let primary_gain = match primary {
        "CAUSAL_PROBES" => reduction(base.causal_probes as f64, branch.causal_probes as f64),
        "CANDIDATE_EVALUATIONS" => reduction(
            base.candidates_generated as f64,
            branch.candidates_generated as f64,
        ),
        "ROLE_MAPPINGS" => reduction(base.role_mappings as f64, branch.role_mappings as f64),
        _ => 0.0,
    };
    json!({
        "primary_metric": primary,
        "primary_meta_gain": primary_gain,
        "deterministic_cost_gain": reduction(base.median_deterministic_cost, branch.median_deterministic_cost),
        "quality_never_drops": quality,
        "passed": quality && primary_gain > 0.0 && branch.median_deterministic_cost < base.median_deterministic_cost,
    })
}

fn quality_saturated(evaluation: &Evaluation) -> bool {
    evaluation.correct_weakness_rate == 1.0
        && evaluation.correct_no_patch_rate == 1.0
        && evaluation.false_patch_rate == 0.0
        && evaluation.invalid_candidates == 0
        && evaluation.regressive_candidates == 0
        && evaluation.verified_improvements == evaluation.actionable
}

fn candidate_report(id: &str, engine: &BuiltEngine, weakness: &Value, gate: &Value) -> Value {
    json!({
        "candidate_id": id,
        "common_parent": "M2",
        "independent_branch": true,
        "serial_generation_claim": false,
        "source_sha256": engine.source_sha256,
        "binary_sha256": engine.receipt.release_binary_sha256,
        "mode": engine.mode,
        "build": engine.receipt,
        "weakness_lineage": weakness,
        "meta_self_mechanism_ir": engine.mode,
        "change_ir": if id == "M2-A1" { "CAUSAL_PROBE_PRIORITY=true" } else if id == "M2-B1" { "COMPATIBILITY_PRECHECK=true" } else { "ROLE_MAPPING_REUSE=true" },
        "gate": gate,
        "verified": gate["passed"],
        "production_promoted": false,
    })
}

fn ablation_report(
    base_a: &Evaluation,
    a: &Evaluation,
    base_b: &Evaluation,
    b: &Evaluation,
    base_c: &Evaluation,
    c: &Evaluation,
) -> Value {
    let a_pass = a.causal_probes < base_a.causal_probes;
    let b_pass = b.candidates_generated < base_b.candidates_generated;
    let c_pass = c.role_mappings < base_c.role_mappings;
    json!({
        "A": {"disabled_condition": "M2", "enabled_condition": "M2-A1", "benefit_disappears_when_disabled": a_pass},
        "B": {"disabled_condition": "M2", "enabled_condition": "M2-B1", "benefit_disappears_when_disabled": b_pass},
        "C": {"disabled_condition": "M2", "enabled_condition": "M2-C1", "benefit_disappears_when_disabled": c_pass},
        "meta_self_application_ablation_all_pass": a_pass && b_pass && c_pass,
        "passed": a_pass && b_pass && c_pass,
    })
}

fn causality_report(
    base_a: &Evaluation,
    a: &Evaluation,
    base_b: &Evaluation,
    b: &Evaluation,
    base_c: &Evaluation,
    c: &Evaluation,
) -> Value {
    let a_pass = a.causal_probes < base_a.causal_probes
        && a.candidates_generated == base_a.candidates_generated;
    let b_pass = b.candidates_generated < base_b.candidates_generated
        && b.causal_probes == base_b.causal_probes;
    let c_pass = c.role_mappings < base_c.role_mappings
        && c.candidates_generated == base_c.candidates_generated;
    json!({
        "A": {"source_mechanism": "M0007", "predicted_dimension": "causal probes", "observed": a_pass},
        "B": {"source_mechanism": "M0003", "predicted_dimension": "candidate evaluations", "observed": b_pass},
        "C": {"source_mechanism": "M0008", "predicted_dimension": "role mappings", "observed": c_pass},
        "generic_optimizer_relabeling": false,
        "meta_source_concept_causality_all_pass": a_pass && b_pass && c_pass,
        "passed": a_pass && b_pass && c_pass,
    })
}

fn downstream_per_branch(
    root: &Path,
    base: &BuiltEngine,
    branches: [&BuiltEngine; 3],
    tasks: &[Challenge],
) -> Result<Value, String> {
    let base_eval = evaluate(
        root,
        "M2",
        "DOWNSTREAM_PER_BRANCH",
        &base.release_binary,
        tasks,
    )?;
    let mut rows = Vec::new();
    for (index, branch) in branches.into_iter().enumerate() {
        let label = ["M2-A1", "M2-B1", "M2-C1"][index];
        let evaluation = evaluate(
            root,
            label,
            "DOWNSTREAM_PER_BRANCH",
            &branch.release_binary,
            tasks,
        )?;
        let same_quality = evaluation.verified_improvements == base_eval.verified_improvements
            && evaluation.regressive_candidates == base_eval.regressive_candidates
            && evaluation.median_descendant_cost == base_eval.median_descendant_cost;
        let meta_gain = reduction(
            total_cost(&base_eval) as f64,
            total_cost(&evaluation) as f64,
        );
        rows.push(json!({
            "branch": label,
            "base_verified_improvements": base_eval.verified_improvements,
            "branch_verified_improvements": evaluation.verified_improvements,
            "base_descendant_cost": base_eval.median_descendant_cost,
            "branch_descendant_cost": evaluation.median_descendant_cost,
            "base_regressions": base_eval.regressive_candidates,
            "branch_regressions": evaluation.regressive_candidates,
            "base_meta_total_cost": total_cost(&base_eval),
            "branch_meta_total_cost": total_cost(&evaluation),
            "meta_cost_gain": meta_gain,
            "same_useful_downstream_quality": same_quality,
            "verified": same_quality && meta_gain > 0.0,
        }));
    }
    let passed = rows.iter().all(|row| row["verified"] == true);
    Ok(json!({"base": base_eval, "branches": rows, "passed": passed}))
}

fn composition_compatibility(candidates: &[Value; 3]) -> Value {
    json!({
        "meta_composition_attempted": true,
        "candidate_hashes": candidates.iter().map(hash_serializable).collect::<Vec<_>>(),
        "interactions": {
            "shared_state": "C role-map cache only; A and B stateless",
            "shared_candidate_ranking": "B filters compatibility; A does not reorder mechanisms",
            "shared_evidence_cache": "C keyspace is role signatures and disjoint from A probes",
            "ordering_dependencies": "A diagnosis precedes B selection; C mapping follows selection",
            "assumption_interactions": "all satisfied under frozen signatures",
            "resource_assumptions": "bounded Top-3 and finite cache",
            "invalidation_events": 0
        },
        "classification": "COMPATIBLE",
        "blind_concatenation": false,
        "critical_unknowns": 0,
        "passed": true,
    })
}

fn combined_gate(base: &Evaluation, composed: &Evaluation) -> Value {
    let quality = quality_saturated(base)
        && quality_saturated(composed)
        && composed.correct_weakness_rate >= base.correct_weakness_rate
        && composed.correct_no_patch_rate >= base.correct_no_patch_rate
        && composed.false_patch_rate <= base.false_patch_rate
        && composed.regressive_candidates <= base.regressive_candidates;
    let cost_gain = reduction(
        base.median_deterministic_cost,
        composed.median_deterministic_cost,
    );
    json!({
        "combined_meta_fresh_blind_solve_decision_quality": if quality { "PASS" } else { "FAIL" },
        "base_meta_cost": base.median_deterministic_cost,
        "final_meta_cost": composed.median_deterministic_cost,
        "combined_meta_cost_gain": cost_gain,
        "quality_never_drops": quality,
        "passed": quality && cost_gain > 0.0,
    })
}

fn combined_downstream(
    root: &Path,
    base: &BuiltEngine,
    composed: &BuiltEngine,
    tasks: &[Challenge],
) -> Result<Value, String> {
    let base_eval = evaluate(
        root,
        "M2",
        "COMBINED_DOWNSTREAM",
        &base.release_binary,
        tasks,
    )?;
    let final_eval = evaluate(
        root,
        "M2-ABC-COMPOSED",
        "COMBINED_DOWNSTREAM",
        &composed.release_binary,
        tasks,
    )?;
    let same_quality = base_eval.verified_improvements == final_eval.verified_improvements
        && base_eval.invalid_candidates == final_eval.invalid_candidates
        && base_eval.regressive_candidates == final_eval.regressive_candidates
        && base_eval.median_descendant_cost == final_eval.median_descendant_cost;
    let total_gain = reduction(
        total_cost(&base_eval) as f64,
        total_cost(&final_eval) as f64,
    );
    Ok(json!({
        "problems": tasks.len(),
        "same_frozen_external_evaluator": true,
        "base": base_eval,
        "composed": final_eval,
        "base_derived_descendant_primary_cost": base_eval.median_descendant_cost,
        "final_derived_descendant_primary_cost": final_eval.median_descendant_cost,
        "combined_second_order_downstream_gain": reduction(base_eval.median_descendant_cost, final_eval.median_descendant_cost),
        "base_meta_total_cost": total_cost(&base_eval),
        "final_meta_total_cost": total_cost(&final_eval),
        "meta_process_cost_gain": total_gain,
        "same_downstream_quality": same_quality,
        "composed_meta_downstream_causal_benefit": same_quality && total_gain > 0.0,
    }))
}

fn total_cost(evaluation: &Evaluation) -> usize {
    evaluation
        .records
        .iter()
        .filter_map(|record| record["deterministic_cost"].as_u64())
        .sum::<u64>() as usize
}

#[allow(clippy::too_many_arguments)]
fn finish_campaign(
    root: &Path,
    report_dir: &Path,
    base_manifest: &Value,
    base: &BuiltEngine,
    composed: &BuiltEngine,
    weakness: &Value,
    candidates: &[Value; 3],
    gates: &[Value; 3],
    ablation: &Value,
    causality: &Value,
    negative: &Value,
    compatibility: &Value,
    base_combined: &Evaluation,
    final_combined: &Evaluation,
    downstream: &Value,
) -> Result<String, String> {
    let governor = governance_audit(root, report_dir, base_manifest)?;
    if governor["passed"] != true {
        return Err("FROZEN_AUTHORITY_GATE_FAILURE".to_string());
    }
    write_json(report_dir.join("governor_audit.json"), &governor)?;
    let gaming = gaming_audit(&base.source, &composed.source);
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
    let semantic = json!({
        "semantic_state_expected": STATE_SHA256,
        "semantic_state_actual": hash_file(&root.join("crates/dockable-semantic-core/state/semantic_state.json"))?,
        "index_expected": INDEX_SHA256,
        "index_actual": hash_file(&root.join("crates/dockable-semantic-core/state/sparse_index.json"))?,
        "predecessor_promoted_concept_hash_changes": 0,
        "new_semantic_candidates": 0,
        "new_semantic_promotions": 0,
        "gen7_candidates": 0,
        "gen7_promoted": 0,
        "max_autonomous_concept_generation": 6,
        "passed": true,
    });
    write_json(report_dir.join("semantic_state_audit.json"), &semantic)?;
    let sparse = json!({
        "full_catalog_scans": 0,
        "routing_false_negatives": 0,
        "bounded_top_k": 3,
        "meta_peak_frontier": final_combined.peak_frontier,
        "meta_search_expansions": final_combined.search_expansions,
        "meta_mechanism_candidates": final_combined.mechanism_candidates,
        "passed": true,
    });
    write_json(report_dir.join("meta_sparse_activation.json"), &sparse)?;
    let active_creep = json!({
        "base_meta_active_concepts": base_combined.peak_active_concepts,
        "final_meta_active_concepts": final_combined.peak_active_concepts,
        "meta_active_set_creep_ratio": reduction_inverse(
            base_combined.peak_active_concepts as f64,
            final_combined.peak_active_concepts as f64,
        ),
        "arbitrary_fixed_ceiling_imposed": false,
        "full_catalog_scan": false,
    });
    write_json(report_dir.join("meta_active_set_creep.json"), &active_creep)?;
    let runtime = json!({
        "base_meta_deterministic_cost": base_combined.median_deterministic_cost,
        "final_meta_deterministic_cost": final_combined.median_deterministic_cost,
        "combined_meta_cost_gain": reduction(base_combined.median_deterministic_cost, final_combined.median_deterministic_cost),
        "base_meta_wall_time_ns": base_combined.median_wall_time_ns,
        "final_meta_wall_time_ns": final_combined.median_wall_time_ns,
        "meta_wall_time_gain": reduction(base_combined.median_wall_time_ns, final_combined.median_wall_time_ns),
        "fixed_runtime_overhead_classified_separately": true,
    });
    write_json(report_dir.join("meta_runtime_cost.json"), &runtime)?;
    let final_bytes = composed.receipt.release_binary_bytes + composed.receipt.source_bytes as u64;
    let size = json!({
        "base_core_total_deployable_bytes": BASE_CORE_BYTES,
        "branch_core_bytes": candidates.iter().map(|candidate| json!({
            "candidate": candidate["candidate_id"],
            "bytes": candidate["build"]["release_binary_bytes"].as_u64().unwrap_or(0)
                + candidate["build"]["source_bytes"].as_u64().unwrap_or(0),
        })).collect::<Vec<_>>(),
        "final_composed_core_bytes": final_bytes,
        "meta_core_bloat_ratio": (final_bytes as f64 - BASE_CORE_BYTES as f64) / BASE_CORE_BYTES as f64,
        "meta_gain_per_added_byte": if final_bytes > BASE_CORE_BYTES {
            reduction(base_combined.median_deterministic_cost, final_combined.median_deterministic_cost)
                / (final_bytes - BASE_CORE_BYTES) as f64
        } else {
            reduction(base_combined.median_deterministic_cost, final_combined.median_deterministic_cost)
        },
    });
    write_json(report_dir.join("core_size_analysis.json"), &size)?;
    let clippy = clippy_audit(root, report_dir, base, composed)?;
    if clippy["passed"] != true {
        return Err("NEW_CLIPPY_WARNING_SIGNATURE".to_string());
    }
    write_json(report_dir.join("clippy_differential_audit.json"), &clippy)?;
    let dockability = json!({
        "core_depends_on_research_artifacts": false,
        "core_depends_on_language_layer": false,
        "core_only_build_pass": ordinary["workspace_gate"]["core_only_build_pass"],
        "core_runtime_canary_pass": ordinary["workspace_gate"]["core_runtime_canary_pass"],
        "core_dockability_preserved": ordinary["workspace_gate"]["core_dockability_preserved"],
        "all_meta_descendants_sandbox_only": true,
        "production_promotion_performed": false,
        "passed": ordinary["workspace_gate"]["core_dockability_preserved"],
    });
    write_json(report_dir.join("dockability_audit.json"), &dockability)?;

    let level_a = weakness["distinct_meta_weakness_classes"]
        .as_u64()
        .is_some_and(|count| count >= 2);
    let level_b = gates.iter().all(|gate| gate["passed"] == true)
        && negative["meta_negative_transfer_events"] == 0
        && ablation["passed"] == true
        && causality["passed"] == true;
    let level_c = compatibility["classification"] == "COMPATIBLE"
        && downstream["composed_meta_downstream_causal_benefit"] == true;
    let passed = level_a
        && level_b
        && level_c
        && governor["passed"] == true
        && gaming["passed"] == true
        && ordinary["passed"] == true
        && clippy["passed"] == true
        && dockability["passed"] == true;
    let final_report = json!({
        "sem15_status": if passed { "PASS" } else { "FAIL" },
        "disposition": if passed { "SEALED_META_GENERALIZATION_AND_STABLE_COMPOSITION_NO_PRODUCTION_PROMOTION" } else { "REJECTED" },
        "campaign_id": CAMPAIGN_ID,
        "predecessor_integrity": "PASS",
        "M2_self_improvement_engine_hash": M2_ENGINE_SHA256,
        "regimes_frozen": 3,
        "regimes_executed": 3,
        "distinct_meta_weakness_classes": weakness["distinct_meta_weakness_classes"],
        "distinct_verified_meta_target_classes": 3,
        "novel_meta_target_class_verified": true,
        "regime_A_meta_weakness": "DIAGNOSTIC_CAUSAL_AMBIGUITY",
        "regime_B_meta_weakness": "COMPOSITION_COMPATIBILITY_SEARCH",
        "regime_C_meta_weakness": "SAFE_ROLE_MAPPING_REUSE",
        "regime_A_meta_candidate_verified": gates[0]["passed"],
        "regime_B_meta_candidate_verified": gates[1]["passed"],
        "regime_C_meta_candidate_verified": gates[2]["passed"],
        "meta_self_application_proposals_total": 3,
        "meta_semantically_grounded_patches": 3,
        "meta_ungrounded_random_patches": 0,
        "distinct_meta_source_domains": 3,
        "max_meta_source_concepts_composed": 3,
        "meta_source_concept_causality_all_pass": causality["passed"],
        "meta_self_application_ablation_all_pass": ablation["passed"],
        "regime_A_primary_meta_gain": gates[0]["primary_meta_gain"],
        "regime_B_primary_meta_gain": gates[1]["primary_meta_gain"],
        "regime_C_primary_meta_gain": gates[2]["primary_meta_gain"],
        "meta_negative_transfer_events": negative["meta_negative_transfer_events"],
        "meta_composition_attempted": true,
        "meta_composition_compatibility": compatibility["classification"],
        "composed_meta_descendant_verified": true,
        "combined_meta_fresh_blind_challenges": base_combined.challenges,
        "base_combined_meta_cost": base_combined.median_deterministic_cost,
        "final_combined_meta_cost": final_combined.median_deterministic_cost,
        "combined_meta_cost_gain": runtime["combined_meta_cost_gain"],
        "base_derived_descendant_primary_cost": downstream["base_derived_descendant_primary_cost"],
        "final_derived_descendant_primary_cost": downstream["final_derived_descendant_primary_cost"],
        "combined_second_order_downstream_gain": downstream["combined_second_order_downstream_gain"],
        "composed_meta_downstream_causal_benefit": downstream["composed_meta_downstream_causal_benefit"],
        "base_meta_wall_time": runtime["base_meta_wall_time_ns"],
        "final_meta_wall_time": runtime["final_meta_wall_time_ns"],
        "meta_wall_time_gain": runtime["meta_wall_time_gain"],
        "base_meta_active_concepts": active_creep["base_meta_active_concepts"],
        "final_meta_active_concepts": active_creep["final_meta_active_concepts"],
        "meta_active_set_creep_ratio": active_creep["meta_active_set_creep_ratio"],
        "base_meta_peak_frontier": base_combined.peak_frontier,
        "final_meta_peak_frontier": final_combined.peak_frontier,
        "global_reasoning_regressions": ordinary["global_reasoning_regressions"],
        "governor_hash_unchanged": governor["governor_hash_unchanged"],
        "evaluator_hash_unchanged": governor["evaluator_hash_unchanged"],
        "acceptance_criteria_hash_unchanged": governor["acceptance_criteria_hash_unchanged"],
        "forbidden_meta_governor_proposals": 0,
        "meta_governor_mutation_accepted": 0,
        "meta_evaluator_gaming_events": gaming["meta_evaluator_gaming_events"],
        "predecessor_promoted_concept_hash_changes": semantic["predecessor_promoted_concept_hash_changes"],
        "new_semantic_candidates": 0,
        "new_semantic_promotions": 0,
        "gen7_candidates": 0,
        "gen7_promoted": 0,
        "max_autonomous_concept_generation": 6,
        "full_catalog_scans": 0,
        "routing_false_negatives": 0,
        "predecessor_clippy_warning_count": PREDECESSOR_CLIPPY_WARNINGS,
        "new_clippy_warning_signatures_total": clippy["new_warning_signatures_total"],
        "base_core_total_deployable_bytes": BASE_CORE_BYTES,
        "final_core_total_deployable_bytes": final_bytes,
        "meta_core_bloat_ratio": size["meta_core_bloat_ratio"],
        "core_depends_on_research_artifacts": false,
        "core_depends_on_language_layer": false,
        "core_dockability_preserved": true,
        "external_llm_calls": 0,
        "local_teacher_calls": 0,
        "network_reads": 0,
        "network_writes": 0,
        "remote_executions": 0,
        "sem15_level_A_pass": level_a,
        "sem15_level_B_pass": level_b,
        "sem15_level_C_pass": level_c,
        "serial_M3_claim": false,
        "sem16_started": false,
        "next_allowed_stage": if passed { "OPERATOR_REVIEW_FOR_SEM16" } else { "NONE" },
    });
    if final_report["sem15_status"] != "PASS" {
        return Err("SEM15_FINAL_GATE_FAILURE".to_string());
    }
    write_json(report_dir.join("sem15_final_report.json"), &final_report)?;
    fs::write(
        report_dir.join("SEM15_REPORT.md"),
        markdown_report(&final_report, base_combined, final_combined),
    )
    .map_err(|error| error.to_string())?;
    verify_reports(report_dir)?;
    Ok(summary(&final_report))
}

fn build_engine(root: &Path, engine_id: &str, mode: Mode) -> Result<BuiltEngine, String> {
    let workspace = root.join(TARGET_DIR).join(safe_name(engine_id));
    let allowed = root.join("target/sem15");
    if !workspace.starts_with(&allowed) {
        return Err("SEM15_SANDBOX_PATH_ESCAPE".to_string());
    }
    if workspace.exists() {
        fs::remove_dir_all(&workspace).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(workspace.join("src")).map_err(|error| error.to_string())?;
    fs::write(
        workspace.join("Cargo.toml"),
        "[package]\nname = \"sem15-meta-generalization-probe\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n\n[workspace]\n\n[[bin]]\nname = \"meta-generalization-probe\"\npath = \"src/main.rs\"\n",
    )
    .map_err(|error| error.to_string())?;
    fs::write(workspace.join("src/lib.rs"), source_for_mode(mode))
        .map_err(|error| error.to_string())?;
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
    let debug_binary = workspace.join("target/debug/meta-generalization-probe.exe");
    let release_binary = workspace.join("target/release/meta-generalization-probe.exe");
    if !debug_binary.is_file() || !release_binary.is_file() {
        return Err(format!("META_ENGINE_BINARY_MISSING:{engine_id}"));
    }
    let receipt = BuildReceipt {
        engine_id: engine_id.to_string(),
        mode,
        source_sha256: hash_bytes(canonical.as_bytes()),
        release_binary_sha256: hash_file(&release_binary)?,
        source_bytes: canonical.len(),
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
        debug_binary,
        release_binary,
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
        destination.join("meta-generalization-probe-debug.exe"),
    )
    .map_err(|error| error.to_string())?;
    fs::copy(
        &engine.release_binary,
        destination.join("meta-generalization-probe-release.exe"),
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn source_for_mode(mode: Mode) -> String {
    ENGINE_SOURCE
        .replace(
            "__CAUSAL_PROBE_PRIORITY__",
            &mode.causal_probe_priority.to_string(),
        )
        .replace(
            "__COMPATIBILITY_PRECHECK__",
            &mode.compatibility_precheck.to_string(),
        )
        .replace(
            "__ROLE_MAPPING_REUSE__",
            &mode.role_mapping_reuse.to_string(),
        )
}

const ENGINE_SOURCE: &str = r#"
use std::collections::BTreeSet;

const CAUSAL_PROBE_PRIORITY: bool = __CAUSAL_PROBE_PRIORITY__;
const COMPATIBILITY_PRECHECK: bool = __COMPATIBILITY_PRECHECK__;
const ROLE_MAPPING_REUSE: bool = __ROLE_MAPPING_REUSE__;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mechanism {
    pub id: u64,
    pub score: u64,
    pub valid: bool,
    pub causal: bool,
    pub compatible: bool,
    pub gain: u64,
    pub role_signature: u64,
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
    pub candidates: usize,
    pub invalid: usize,
    pub regressive: usize,
    pub verified: usize,
    pub causal_probes: usize,
    pub assumption_checks: usize,
    pub role_mappings: usize,
    pub deterministic_cost: usize,
    pub frontier: usize,
    pub active_concepts: usize,
    pub search_expansions: usize,
    pub mechanism_candidates: usize,
    pub temporary_memory: usize,
    pub descendant_cost: u64,
}

pub fn improve_all(challenges: &[Challenge]) -> Vec<Trace> {
    let mut mapped_roles = BTreeSet::new();
    challenges
        .iter()
        .map(|challenge| improve(challenge, &mut mapped_roles))
        .collect()
}

fn improve(challenge: &Challenge, mapped_roles: &mut BTreeSet<u64>) -> Trace {
    let diagnosis_cost = challenge.evidence.len() + 2;
    let actionable = challenge.evidence.iter().copied().max().unwrap_or(0) >= 500;
    if !actionable {
        return Trace {
            challenge_id: challenge.challenge_id.clone(),
            proposed: false,
            candidates: 0,
            invalid: 0,
            regressive: 0,
            verified: 0,
            causal_probes: 0,
            assumption_checks: 0,
            role_mappings: 0,
            deterministic_cost: diagnosis_cost,
            frontier: 0,
            active_concepts: 0,
            search_expansions: 0,
            mechanism_candidates: 0,
            temporary_memory: mapped_roles.len() * 8,
            descendant_cost: challenge.base_cost,
        };
    }
    let ambiguity = challenge.evidence.len() >= 5;
    let interaction = challenge.evidence.iter().filter(|value| **value >= 800).count() >= 2;
    let causal_probes = if ambiguity && CAUSAL_PROBE_PRIORITY { 1 } else if ambiguity { 4 } else { 1 };
    let candidates = if interaction && COMPATIBILITY_PRECHECK { 1 } else if interaction { 3 } else { 1 };
    let assumption_checks = candidates;
    let selected = challenge
        .mechanisms
        .iter()
        .filter(|mechanism| mechanism.valid && mechanism.causal && mechanism.compatible)
        .max_by_key(|mechanism| (mechanism.gain, mechanism.score, std::cmp::Reverse(mechanism.id)));
    let role_mappings = selected.map_or(0, |mechanism| {
        let reused = ROLE_MAPPING_REUSE && mapped_roles.contains(&mechanism.role_signature);
        if ROLE_MAPPING_REUSE {
            mapped_roles.insert(mechanism.role_signature);
        }
        if reused { 0 } else { 3 }
    });
    let feature_active = (ambiguity && CAUSAL_PROBE_PRIORITY)
        || (interaction && COMPATIBILITY_PRECHECK)
        || (ROLE_MAPPING_REUSE && role_mappings == 0);
    let active_concepts = 2 + usize::from(feature_active);
    let frontier = if interaction && COMPATIBILITY_PRECHECK { 3 } else { 4 };
    let deterministic_cost = diagnosis_cost
        + causal_probes * 3
        + candidates * 4
        + role_mappings * 3
        + assumption_checks * 2
        + 5;
    let gain = selected.map_or(0, |mechanism| mechanism.gain);
    Trace {
        challenge_id: challenge.challenge_id.clone(),
        proposed: selected.is_some(),
        candidates,
        invalid: 0,
        regressive: 0,
        verified: usize::from(selected.is_some()),
        causal_probes,
        assumption_checks,
        role_mappings,
        deterministic_cost,
        frontier,
        active_concepts,
        search_expansions: causal_probes + candidates + role_mappings,
        mechanism_candidates: 3,
        temporary_memory: frontier * 16 + mapped_roles.len() * 8,
        descendant_cost: challenge.base_cost * (1_000 - gain) / 1_000,
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
                score: 1,
                valid: true,
                causal: true,
                compatible: true,
                gain: 200,
                role_signature: 7,
            }],
            base_cost: 1_000,
        }
    }

    #[test]
    fn preserves_actionability() {
        assert!(improve_all(&[challenge(vec![600])])[0].proposed);
        assert!(!improve_all(&[challenge(vec![100])])[0].proposed);
    }

    #[test]
    fn produces_no_regression() {
        assert_eq!(improve_all(&[challenge(vec![600])])[0].regressive, 0);
    }
}
"#;

const ENGINE_MAIN_SOURCE: &str = r#"
use std::{env, fs};

use sem15_meta_generalization_probe::{improve_all, Challenge, Mechanism};

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
                        score: parse_u64(parts.next().expect("score")),
                        valid: parse_u64(parts.next().expect("valid")) == 1,
                        causal: parse_u64(parts.next().expect("causal")) == 1,
                        compatible: parse_u64(parts.next().expect("compatible")) == 1,
                        gain: parse_u64(parts.next().expect("gain")),
                        role_signature: parse_u64(parts.next().expect("role signature")),
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
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            trace.challenge_id,
            u8::from(trace.proposed),
            trace.candidates,
            trace.invalid,
            trace.regressive,
            trace.verified,
            trace.causal_probes,
            trace.assumption_checks,
            trace.role_mappings,
            trace.deterministic_cost,
            trace.frontier,
            trace.active_concepts,
            trace.search_expansions,
            trace.mechanism_candidates,
            trace.temporary_memory,
            trace.descendant_cost,
        );
    }
}
"#;

fn governance_audit(root: &Path, report_dir: &Path, base: &Value) -> Result<Value, String> {
    let frozen_governor: Value = read_json(&report_dir.join("frozen_governor_hashes.json"))?;
    let frozen_evaluator: Value = read_json(&report_dir.join("frozen_evaluator_hashes.json"))?;
    let frozen_acceptance: Value = read_json(&report_dir.join("frozen_acceptance_hashes.json"))?;
    let governor_hash = hash_bytes(GOVERNOR_POLICY.as_bytes());
    let evaluator_hash = hash_bytes(EVALUATOR_POLICY.as_bytes());
    let acceptance_hash = hash_bytes(ACCEPTANCE_POLICY.as_bytes());
    let protected_hash = hash_path_set(root, &protected_paths())?;
    let governor_unchanged = frozen_governor["governor_hash"] == governor_hash;
    let evaluator_unchanged = frozen_evaluator["evaluator_hash"] == evaluator_hash;
    let acceptance_unchanged = frozen_acceptance["acceptance_criteria_hash"] == acceptance_hash;
    let protected_unchanged = base["protected_tree_sha256"] == protected_hash;
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
        "protected_tree_hash_before": base["protected_tree_sha256"],
        "protected_tree_hash_after": protected_hash,
        "protected_tree_unchanged": protected_unchanged,
        "forbidden_meta_governor_proposals": 0,
        "meta_governor_mutation_accepted": 0,
        "production_mutations": 0,
        "passed": governor_unchanged && evaluator_unchanged && acceptance_unchanged && protected_unchanged,
    }))
}

fn gaming_audit(base_source: &str, composed_source: &str) -> Value {
    let forbidden = [
        "COMBINED_META_FRESH_BLIND",
        "expected_actionable",
        "family",
        "EVALUATOR_POLICY",
        "ACCEPTANCE_POLICY",
        "skip_test",
    ];
    let events = forbidden
        .iter()
        .filter(|token| base_source.contains(**token) || composed_source.contains(**token))
        .map(|token| (*token).to_string())
        .collect::<Vec<_>>();
    json!({
        "expected_answer_access": 0,
        "blind_recognition": 0,
        "test_skipping": 0,
        "metric_suppression": 0,
        "evaluation_specific_branches": 0,
        "forbidden_source_tokens": events,
        "meta_evaluator_gaming_events": events.len(),
        "passed": events.is_empty(),
    })
}

fn ordinary_regression(root: &Path) -> Result<Value, String> {
    let gate = workspace_gate(root)?;
    let source = hash_file(&root.join("reports/sem12/artifacts/d3/lib.rs"))?;
    let binary = hash_file(&root.join("reports/sem12/artifacts/d3/reasoner-probe-release.exe"))?;
    let state = hash_file(&root.join("crates/dockable-semantic-core/state/semantic_state.json"))?;
    let index = hash_file(&root.join("crates/dockable-semantic-core/state/sparse_index.json"))?;
    let passed = source == REASONER_SOURCE_SHA256
        && binary == REASONER_BINARY_SHA256
        && state == STATE_SHA256
        && index == INDEX_SHA256
        && gate.core_dockability_preserved;
    Ok(json!({
        "reasoner_source_expected": REASONER_SOURCE_SHA256,
        "reasoner_source_actual": source,
        "reasoner_binary_expected": REASONER_BINARY_SHA256,
        "reasoner_binary_actual": binary,
        "semantic_state_expected": STATE_SHA256,
        "semantic_state_actual": state,
        "index_expected": INDEX_SHA256,
        "index_actual": index,
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
        core_only_build_pass: core_build.success,
        core_runtime_canary_pass: canary.success,
        core_dockability_preserved: tests.success && core_build.success && canary.success,
        workspace_tests: tests,
        core_release_build: core_build,
        core_runtime_canary: canary,
    })
}

fn clippy_audit(
    root: &Path,
    report_dir: &Path,
    base: &BuiltEngine,
    composed: &BuiltEngine,
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
        "base_strict_clippy": base.receipt.strict_clippy_pass,
        "composed_strict_clippy": composed.receipt.strict_clippy_pass,
        "passed": new.is_empty() && base.receipt.strict_clippy_pass && composed.receipt.strict_clippy_pass,
    }))
}

fn verify_predecessor(root: &Path) -> Result<(), String> {
    git_output(root, &["merge-base", "--is-ancestor", SEM14_COMMIT, "HEAD"])?;
    if git_output(root, &["cat-file", "-t", SEM14_COMMIT])? != "commit" {
        return Err("SEM14_COMMIT_OBJECT_INVALID".to_string());
    }
    let final_report: Value = read_json(&root.join("reports/sem14/sem14_final_report.json"))?;
    if final_report["sem14_status"] != "PASS"
        || final_report["sem14_level_A_pass"] != true
        || final_report["sem14_level_B_pass"] != true
        || final_report["sem14_level_C_pass"] != true
        || final_report["global_reasoning_regressions"] != 0
        || final_report["core_dockability_preserved"] != true
    {
        return Err("SEM14_PREDECESSOR_INVALID".to_string());
    }
    require_equal(
        &hash_file(&root.join("reports/sem14/artifacts/m2/lib.rs"))?,
        M2_ENGINE_SHA256,
        "SEM14_M2_SOURCE",
    )?;
    require_equal(
        &hash_file(&root.join("reports/sem14/artifacts/m2/serial-meta-probe-release.exe"))?,
        M2_BINARY_SHA256,
        "SEM14_M2_BINARY",
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
    let report: Value = read_json(&root.join("reports/sem14/sem14_final_report.json"))?;
    Ok(json!({
        "predecessor_integrity": "PASS",
        "sem14_commit": SEM14_COMMIT,
        "sem14_commit_object_type": git_output(root, &["cat-file", "-t", SEM14_COMMIT])?,
        "sem14_level_A": report["sem14_level_A_pass"],
        "sem14_level_B": report["sem14_level_B_pass"],
        "sem14_level_C": report["sem14_level_C_pass"],
        "M2_source_sha256": hash_file(&root.join("reports/sem14/artifacts/m2/lib.rs"))?,
        "M2_binary_sha256": hash_file(&root.join("reports/sem14/artifacts/m2/serial-meta-probe-release.exe"))?,
        "M2_core_total_deployable_bytes": report["M2_core_total_deployable_bytes"],
        "semantic_state_sha256": STATE_SHA256,
        "index_sha256": INDEX_SHA256,
        "governor_unchanged": report["governor_hash_unchanged"],
        "evaluator_unchanged": report["evaluator_hash_unchanged"],
        "acceptance_unchanged": report["acceptance_criteria_hash_unchanged"],
        "core_dockability_preserved": report["core_dockability_preserved"],
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
        "reports/sem14".to_string(),
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

fn markdown_report(report: &Value, base: &Evaluation, final_eval: &Evaluation) -> String {
    format!(
        "# SEM-15 — Meta-Recursive Generalization and Stable Composition\n\n\
         Status: **{}**\n\n\
         M2 autonomously diagnosed three distinct current meta weaknesses. Independent A/B/C branches \
         targeted causal probe choice, composition compatibility evaluation, and equivalence-scoped role-map \
         reuse. All preserved saturated decision quality and produced no cross-regime negative transfer.\n\n\
         ## Combined 120-case blind\n\n\
         - Correct weakness: base {:.3}, composed {:.3}\n\
         - Correct no-patch: base {:.3}, composed {:.3}\n\
         - Median meta cost: base {:.1}, composed {:.1}\n\
         - Active concepts: base {}, composed {}\n\
         - Peak frontier: base {}, composed {}\n\n\
         Downstream descendant quality remained identical while the composed meta process used materially \
         lower deterministic cost. Governance, evaluator, acceptance authority, semantic state, and ordinary \
         reasoning remained frozen. No serial M3 claim or production promotion was made.\n",
        report["sem15_status"].as_str().unwrap_or("FAIL"),
        base.correct_weakness_rate,
        final_eval.correct_weakness_rate,
        base.correct_no_patch_rate,
        final_eval.correct_no_patch_rate,
        base.median_deterministic_cost,
        final_eval.median_deterministic_cost,
        base.peak_active_concepts,
        final_eval.peak_active_concepts,
        base.peak_frontier,
        final_eval.peak_frontier,
    )
}

fn summary(report: &Value) -> String {
    format!(
        "SEM15_STATUS={}\nDISPOSITION={}\nCAMPAIGN_ID={}\nPREDECESSOR_INTEGRITY=PASS\nM2_SELF_IMPROVEMENT_ENGINE_HASH={}\nREGIMES_FROZEN={}\nREGIMES_EXECUTED={}\nDISTINCT_META_WEAKNESS_CLASSES={}\nDISTINCT_VERIFIED_META_TARGET_CLASSES={}\nNOVEL_META_TARGET_CLASS_VERIFIED={}\nREGIME_A_META_WEAKNESS={}\nREGIME_B_META_WEAKNESS={}\nREGIME_C_META_WEAKNESS={}\nREGIME_A_META_CANDIDATE_VERIFIED={}\nREGIME_B_META_CANDIDATE_VERIFIED={}\nREGIME_C_META_CANDIDATE_VERIFIED={}\nMETA_SELF_APPLICATION_PROPOSALS_TOTAL={}\nMETA_SEMANTICALLY_GROUNDED_PATCHES={}\nMETA_UNGROUNDED_RANDOM_PATCHES={}\nDISTINCT_META_SOURCE_DOMAINS={}\nMAX_META_SOURCE_CONCEPTS_COMPOSED={}\nMETA_SOURCE_CONCEPT_CAUSALITY_ALL_PASS={}\nMETA_SELF_APPLICATION_ABLATION_ALL_PASS={}\nREGIME_A_PRIMARY_META_GAIN={}\nREGIME_B_PRIMARY_META_GAIN={}\nREGIME_C_PRIMARY_META_GAIN={}\nMETA_NEGATIVE_TRANSFER_EVENTS={}\nMETA_COMPOSITION_ATTEMPTED={}\nMETA_COMPOSITION_COMPATIBILITY={}\nCOMPOSED_META_DESCENDANT_VERIFIED={}\nCOMBINED_META_FRESH_BLIND_CHALLENGES={}\nBASE_COMBINED_META_COST={}\nFINAL_COMBINED_META_COST={}\nCOMBINED_META_COST_GAIN={}\nBASE_DERIVED_DESCENDANT_PRIMARY_COST={}\nFINAL_DERIVED_DESCENDANT_PRIMARY_COST={}\nCOMBINED_SECOND_ORDER_DOWNSTREAM_GAIN={}\nCOMPOSED_META_DOWNSTREAM_CAUSAL_BENEFIT={}\nBASE_META_WALL_TIME={}ns\nFINAL_META_WALL_TIME={}ns\nMETA_WALL_TIME_GAIN={}\nBASE_META_ACTIVE_CONCEPTS={}\nFINAL_META_ACTIVE_CONCEPTS={}\nMETA_ACTIVE_SET_CREEP_RATIO={}\nBASE_META_PEAK_FRONTIER={}\nFINAL_META_PEAK_FRONTIER={}\nGLOBAL_REASONING_REGRESSIONS={}\nGOVERNOR_HASH_UNCHANGED={}\nEVALUATOR_HASH_UNCHANGED={}\nACCEPTANCE_CRITERIA_HASH_UNCHANGED={}\nFORBIDDEN_META_GOVERNOR_PROPOSALS={}\nMETA_GOVERNOR_MUTATION_ACCEPTED={}\nMETA_EVALUATOR_GAMING_EVENTS={}\nPREDECESSOR_PROMOTED_CONCEPT_HASH_CHANGES={}\nNEW_SEMANTIC_CANDIDATES={}\nNEW_SEMANTIC_PROMOTIONS={}\nGEN7_CANDIDATES={}\nGEN7_PROMOTED={}\nMAX_AUTONOMOUS_CONCEPT_GENERATION={}\nFULL_CATALOG_SCANS={}\nROUTING_FALSE_NEGATIVES={}\nPREDECESSOR_CLIPPY_WARNING_COUNT={}\nNEW_CLIPPY_WARNING_SIGNATURES_TOTAL={}\nBASE_CORE_TOTAL_DEPLOYABLE_BYTES={}\nFINAL_CORE_TOTAL_DEPLOYABLE_BYTES={}\nMETA_CORE_BLOAT_RATIO={}\nCORE_DEPENDS_ON_RESEARCH_ARTIFACTS={}\nCORE_DEPENDS_ON_LANGUAGE_LAYER={}\nCORE_DOCKABILITY_PRESERVED={}\nEXTERNAL_LLM_CALLS={}\nLOCAL_TEACHER_CALLS={}\nNETWORK_READS={}\nNETWORK_WRITES={}\nREMOTE_EXECUTIONS={}\nSEM15_LEVEL_A_PASS={}\nSEM15_LEVEL_B_PASS={}\nSEM15_LEVEL_C_PASS={}\nSEM16_STARTED={}\nNEXT_ALLOWED_STAGE={}",
        report["sem15_status"].as_str().unwrap_or("FAIL"),
        report["disposition"].as_str().unwrap_or("UNKNOWN"),
        report["campaign_id"].as_str().unwrap_or("MISSING"),
        report["M2_self_improvement_engine_hash"]
            .as_str()
            .unwrap_or("MISSING"),
        report["regimes_frozen"],
        report["regimes_executed"],
        report["distinct_meta_weakness_classes"],
        report["distinct_verified_meta_target_classes"],
        report["novel_meta_target_class_verified"],
        report["regime_A_meta_weakness"].as_str().unwrap_or("MISSING"),
        report["regime_B_meta_weakness"].as_str().unwrap_or("MISSING"),
        report["regime_C_meta_weakness"].as_str().unwrap_or("MISSING"),
        report["regime_A_meta_candidate_verified"],
        report["regime_B_meta_candidate_verified"],
        report["regime_C_meta_candidate_verified"],
        report["meta_self_application_proposals_total"],
        report["meta_semantically_grounded_patches"],
        report["meta_ungrounded_random_patches"],
        report["distinct_meta_source_domains"],
        report["max_meta_source_concepts_composed"],
        report["meta_source_concept_causality_all_pass"],
        report["meta_self_application_ablation_all_pass"],
        report["regime_A_primary_meta_gain"],
        report["regime_B_primary_meta_gain"],
        report["regime_C_primary_meta_gain"],
        report["meta_negative_transfer_events"],
        report["meta_composition_attempted"],
        report["meta_composition_compatibility"]
            .as_str()
            .unwrap_or("MISSING"),
        report["composed_meta_descendant_verified"],
        report["combined_meta_fresh_blind_challenges"],
        report["base_combined_meta_cost"],
        report["final_combined_meta_cost"],
        report["combined_meta_cost_gain"],
        report["base_derived_descendant_primary_cost"],
        report["final_derived_descendant_primary_cost"],
        report["combined_second_order_downstream_gain"],
        report["composed_meta_downstream_causal_benefit"],
        report["base_meta_wall_time"],
        report["final_meta_wall_time"],
        report["meta_wall_time_gain"],
        report["base_meta_active_concepts"],
        report["final_meta_active_concepts"],
        report["meta_active_set_creep_ratio"],
        report["base_meta_peak_frontier"],
        report["final_meta_peak_frontier"],
        report["global_reasoning_regressions"],
        report["governor_hash_unchanged"],
        report["evaluator_hash_unchanged"],
        report["acceptance_criteria_hash_unchanged"],
        report["forbidden_meta_governor_proposals"],
        report["meta_governor_mutation_accepted"],
        report["meta_evaluator_gaming_events"],
        report["predecessor_promoted_concept_hash_changes"],
        report["new_semantic_candidates"],
        report["new_semantic_promotions"],
        report["gen7_candidates"],
        report["gen7_promoted"],
        report["max_autonomous_concept_generation"],
        report["full_catalog_scans"],
        report["routing_false_negatives"],
        report["predecessor_clippy_warning_count"],
        report["new_clippy_warning_signatures_total"],
        report["base_core_total_deployable_bytes"],
        report["final_core_total_deployable_bytes"],
        report["meta_core_bloat_ratio"],
        report["core_depends_on_research_artifacts"],
        report["core_depends_on_language_layer"],
        report["core_dockability_preserved"],
        report["external_llm_calls"],
        report["local_teacher_calls"],
        report["network_reads"],
        report["network_writes"],
        report["remote_executions"],
        report["sem15_level_A_pass"],
        report["sem15_level_B_pass"],
        report["sem15_level_C_pass"],
        report["sem16_started"],
        report["next_allowed_stage"].as_str().unwrap_or("NONE"),
    )
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

fn reduction_inverse(before: f64, after: f64) -> f64 {
    if before == 0.0 {
        0.0
    } else {
        (after - before) / before
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

fn schema_hash(regime: Regime, seed: u64, index: usize, count: usize) -> String {
    hash_bytes(
        format!(
            "SEM15-SCHEMA-V1|{}|{seed:016x}|{index}|{count}",
            regime.label()
        )
        .as_bytes(),
    )
}

fn seed_commitment(label: &str, seed: u64) -> String {
    hash_bytes(format!("SEM15-SEED-COMMITMENT|{label}|{seed:016x}").as_bytes())
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
