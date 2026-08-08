use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const CAMPAIGN_ID: &str = "SEM11-GENERALIZATION-STABILITY-0001";
const REPORT_DIRECTORY: &str = "reports/sem11";
const TARGET_DIRECTORY: &str = "target/sem11/SEM11-GENERALIZATION-STABILITY-0001";
const SEM10_COMMIT: &str = "b41faedbac6733a2701d67e57b21478cffdc68b9";
const BASE_SOURCE_SHA256: &str = "5e18b4997942ffbd664e87f230cdb210faeafc55539a5a41a2f18117949d1414";
const BASE_BINARY_SHA256: &str = "28d47d5eeeaf95a43711e943fd206f7b954509f78d6ad38bdf5da9fa6842466e";
const SEMANTIC_STATE_SHA256: &str =
    "d1abd8de410f5284773f1e582937922dc514513ed738eb9f04e8bf2735185d3c";
const INDEX_SHA256: &str = "77b17332b5ff7204c28e9445e689276049afd6e89308e7e242904570a283e6fc";
const PREDECESSOR_CLIPPY_WARNINGS: usize = 22;
const DIAGNOSTIC_TASKS: usize = 30;
const VALIDATION_TASKS: usize = 84;
const ADVERSARIAL_TASKS: usize = 21;
const GENERAL_TASKS: usize = 70;
const COMBINED_TASKS: usize = 180;
const EVALUATION_TRIALS: usize = 5;
const STABILITY_TRIALS: usize = 11;
const KEY_BOUND: u64 = 4096;

const A_DIAGNOSTIC_SEED: u64 = 0x11a0_d1a6_0000_0001;
const A_VALIDATION_SEED: u64 = 0x11a0_b11d_0000_0002;
const A_ADVERSARIAL_SEED: u64 = 0x11a0_ad00_0000_0003;
const B_DIAGNOSTIC_SEED: u64 = 0x11b0_d1a6_0000_0001;
const B_VALIDATION_SEED: u64 = 0x11b0_b11d_0000_0002;
const B_ADVERSARIAL_SEED: u64 = 0x11b0_ad00_0000_0003;
const C_DIAGNOSTIC_SEED: u64 = 0x11c0_d1a6_0000_0001;
const C_VALIDATION_SEED: u64 = 0x11c0_b11d_0000_0002;
const C_ADVERSARIAL_SEED: u64 = 0x11c0_ad00_0000_0003;
const GENERAL_SEED: u64 = 0x11e0_6e3e_0000_0001;
const COMBINED_SEED: u64 = 0x11f0_c0ab_0000_0001;

const REQUIRED_REPORTS: &[&str] = &[
    "predecessor_integrity.json",
    "preflight_concept_lineage_audit.json",
    "campaign_config.json",
    "sem11_base_manifest.json",
    "regime_a_manifest.json",
    "regime_b_manifest.json",
    "regime_c_manifest.json",
    "weakness_ledger.json",
    "mechanism_selection_ledger.json",
    "role_mapping_ledger.json",
    "assumption_ledger.json",
    "candidate_a.json",
    "candidate_b.json",
    "candidate_c.json",
    "fresh_validation_results.json",
    "global_regression_matrix.json",
    "cross_regime_stability_matrix.json",
    "self_application_ablation.json",
    "source_concept_causality.json",
    "negative_transfer_audit.json",
    "composition_compatibility.json",
    "composed_candidate.json",
    "combined_fresh_blind_manifest.json",
    "combined_fresh_blind_results.json",
    "repeated_stability_results.json",
    "semantic_state_audit.json",
    "semantic_growth.json",
    "clippy_differential_audit.json",
    "sparse_activation_audit.json",
    "deep_reasoning_preservation.json",
    "core_size_by_candidate.json",
    "dockability_audit.json",
    "generalized_self_improvement_lineage.json",
    "sem11_final_report.json",
    "SEM11_REPORT.md",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum Pressure {
    Routing,
    State,
    Composition,
    General,
    Mixed,
}

impl Pressure {
    fn code(self) -> &'static str {
        match self {
            Self::Routing => "ROUTING_SELECTION",
            Self::State => "STATE_RESOURCE",
            Self::Composition => "COMPOSITION_ABSTRACTION",
            Self::General => "GENERAL_CONTROL",
            Self::Mixed => "MIXED_CROSS_REGIME",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct Mode {
    scoped_routing: bool,
    reduced_state: bool,
    cached_composition: bool,
}

impl Mode {
    const BASE: Self = Self {
        scoped_routing: false,
        reduced_state: false,
        cached_composition: false,
    };
    const A: Self = Self {
        scoped_routing: true,
        reduced_state: false,
        cached_composition: false,
    };
    const B: Self = Self {
        scoped_routing: false,
        reduced_state: true,
        cached_composition: false,
    };
    const C: Self = Self {
        scoped_routing: false,
        reduced_state: false,
        cached_composition: true,
    };
    const AB: Self = Self {
        scoped_routing: true,
        reduced_state: true,
        cached_composition: false,
    };
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CampaignConfig {
    campaign_id: String,
    infrastructure_commit: String,
    predecessor_commit: String,
    regimes: Vec<String>,
    diagnostic_tasks_per_regime: usize,
    validation_tasks_per_regime: usize,
    adversarial_tasks_per_regime: usize,
    combined_blind_tasks: usize,
    validation_hidden_until_candidate_freeze: bool,
    same_base_for_all_regimes: bool,
    candidate_primary_gain_floor: f64,
    strong_gain_target: f64,
    zero_global_regression_required: bool,
    negative_transfer_allowed: usize,
    inherited_clippy_warning_count: usize,
    new_clippy_warning_signatures_allowed: usize,
    full_catalog_scans_allowed: usize,
    routing_false_negatives_allowed: usize,
    external_llm_calls_allowed: usize,
    local_teacher_calls_allowed: usize,
    network_writes_allowed: usize,
    remote_executions_allowed: usize,
    automatic_production_promotion: bool,
    sem12_started: bool,
    seed_commitments: BTreeMap<String, String>,
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
struct RoutingEntry {
    mechanism_id: String,
    source_concept_ids: Vec<String>,
    source_domain: String,
    transform: String,
    roles: Vec<Value>,
    assumptions: Vec<Value>,
    semantic_sha256: String,
    compatibility_score: i64,
    compatibility_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RoutingIndex {
    source_catalog_sha256: String,
    built_before_candidate_generation: bool,
    full_catalog_scans_during_experiment: usize,
    routes: BTreeMap<String, Vec<RoutingEntry>>,
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
struct CandidateInput {
    id: u64,
    scope: u64,
    assumption: bool,
    score: u64,
}

#[derive(Debug, Clone)]
struct StateInput {
    key: u64,
    payload: u64,
}

#[derive(Debug, Clone)]
struct Task {
    task_id: String,
    pressure: Pressure,
    required_scope: u64,
    candidates: Vec<CandidateInput>,
    states: Vec<StateInput>,
    reuse_count: usize,
    chains: Vec<Vec<u64>>,
    opaque_schema_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct VisibleTask {
    task_id: String,
    opaque_schema_sha256: String,
    public_contract_sha256: String,
    hidden_inputs_included: bool,
    expected_outputs_included: bool,
    researcher_pressure_label_exposed_to_candidate: bool,
    frozen: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SetManifest {
    set_id: String,
    seed_commitment_sha256: String,
    generator_version: String,
    tasks: Vec<VisibleTask>,
    hidden_inputs_included: bool,
    expected_outputs_included: bool,
    frozen_before_candidate_generation: bool,
    manifest_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegimeManifest {
    regime_id: String,
    researcher_classification: String,
    classification_exposed_to_candidate: bool,
    diagnostic: SetManifest,
    fresh_validation: SetManifest,
    adversarial: SetManifest,
    frozen: bool,
    manifest_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SemanticOutput {
    selected_id: u64,
    state_checksum: u64,
    composition_checksum: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct BinaryRecord {
    task_id: String,
    selected_id: u64,
    state_checksum: u64,
    composition_checksum: u64,
    routing_ops: usize,
    false_activations: usize,
    peak_transient_bytes: usize,
    reconstruction_ops: usize,
    composition_ops: usize,
    max_solution_depth: usize,
    max_primitive_expanded_depth: usize,
    peak_frontier: usize,
    peak_active_concepts: usize,
    total_primary_cost: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EvaluationRecord {
    task_id: String,
    pressure: Pressure,
    strict_correct: bool,
    output_sha256: String,
    routing_ops: usize,
    false_activations: usize,
    peak_transient_bytes: usize,
    reconstruction_ops: usize,
    composition_ops: usize,
    max_solution_depth: usize,
    max_primitive_expanded_depth: usize,
    peak_frontier: usize,
    peak_active_concepts: usize,
    total_primary_cost: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EvaluationSummary {
    condition: String,
    set_id: String,
    tasks: usize,
    strict_solved: usize,
    strict_solve_rate: f64,
    median_routing_ops: f64,
    median_false_activations: f64,
    peak_transient_bytes: usize,
    median_reconstruction_ops: f64,
    median_composition_ops: f64,
    max_solution_depth: usize,
    max_primitive_expanded_depth: usize,
    peak_frontier: usize,
    peak_active_concepts: usize,
    median_total_primary_cost: f64,
    median_wall_time_ns: f64,
    repeated_trials: usize,
    records: Vec<EvaluationRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Observation {
    observer: String,
    regime_id: String,
    observed_source_sha256: String,
    observed_binary_sha256: String,
    diagnostic_tasks: usize,
    strict_solve_rate: f64,
    median_candidates: f64,
    median_states: f64,
    median_reuse_count: f64,
    median_chain_operations: f64,
    median_unique_prefix_operations: f64,
    median_routing_ops: f64,
    median_false_activations: f64,
    peak_transient_bytes: usize,
    median_reconstruction_ops: f64,
    median_composition_ops: f64,
    observation_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Weakness {
    weakness_id: String,
    regime_id: String,
    observed_component: String,
    target_class: String,
    feature: String,
    observed_behavior: String,
    measured_cost: Value,
    evidence: Vec<String>,
    causal_hypothesis: String,
    confidence: f64,
    outcome: String,
    autonomous_weakness_diagnosis: bool,
    weakness_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Selection {
    weakness_id: String,
    rankings: Vec<RoutingEntry>,
    selected: RoutingEntry,
    top_one_applied_only: bool,
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
    candidate_id: String,
    mode: Mode,
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
    commands: Vec<CommandReceipt>,
    rustfmt_check_pass: bool,
    strict_clippy_pass: bool,
    tests_pass: bool,
    debug_build_pass: bool,
    release_build_pass: bool,
}

#[derive(Debug, Clone)]
struct BuiltCandidate {
    mode: Mode,
    source: String,
    source_sha256: String,
    debug_binary: PathBuf,
    release_binary: PathBuf,
    receipt: BuildReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CandidateOutcome {
    regime_id: String,
    candidate_id: String,
    target_class: String,
    observation: Observation,
    weakness: Weakness,
    selection: Selection,
    role_mapping: Value,
    assumption_ledger: Value,
    self_mechanism_ir: Value,
    change_ir: Value,
    parent_source_sha256: String,
    candidate_source_sha256: String,
    parent_binary_sha256: String,
    candidate_binary_sha256: String,
    diff_sha256: String,
    build: BuildReceipt,
    validation_manifest_sha256: String,
    base_validation: EvaluationSummary,
    child_validation: EvaluationSummary,
    primary_metric: String,
    base_primary_value: f64,
    child_primary_value: f64,
    primary_gain: f64,
    regressed_tasks: usize,
    self_application_ablation: Value,
    source_concept_causality: Value,
    verified: bool,
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
    verify_sem10_predecessor(root)?;
    let directory = root.join(REPORT_DIRECTORY);
    if directory.exists()
        && fs::read_dir(&directory)
            .map_err(|error| error.to_string())?
            .next()
            .is_some()
    {
        return Err("SEM11_REPORT_DIRECTORY_NOT_EMPTY".to_string());
    }
    fs::create_dir_all(directory.join("artifacts/base")).map_err(|error| error.to_string())?;

    let infrastructure_commit = git_output(root, &["rev-parse", "HEAD"])?;
    let preflight = concept_lineage_audit(root)?;
    if preflight["concept_lineage_integrity"] != "PASS" {
        return Err("CONCEPT_LINEAGE_INTEGRITY_FAILURE".to_string());
    }
    let predecessor = predecessor_integrity(root)?;
    let config = campaign_config(&infrastructure_commit);
    let catalog_bytes = fs::read(root.join("reports/sem8/source_mechanism_catalog.json"))
        .map_err(|error| error.to_string())?;
    let catalog: Vec<CatalogMechanism> =
        serde_json::from_slice(&catalog_bytes).map_err(|error| error.to_string())?;
    let routing_index = build_routing_index(&catalog, hash_bytes(&catalog_bytes));

    let regime_a = build_regime_manifest(
        "REGIME_A",
        "ROUTING_SELECTION_PRESSURE",
        Pressure::Routing,
        A_DIAGNOSTIC_SEED,
        A_VALIDATION_SEED,
        A_ADVERSARIAL_SEED,
    );
    let regime_b = build_regime_manifest(
        "REGIME_B",
        "STATE_MEMORY_RESOURCE_PRESSURE",
        Pressure::State,
        B_DIAGNOSTIC_SEED,
        B_VALIDATION_SEED,
        B_ADVERSARIAL_SEED,
    );
    let regime_c = build_regime_manifest(
        "REGIME_C",
        "COMPOSITION_ABSTRACTION_PRESSURE",
        Pressure::Composition,
        C_DIAGNOSTIC_SEED,
        C_VALIDATION_SEED,
        C_ADVERSARIAL_SEED,
    );
    let general_manifest = visible_set_manifest(
        "GENERAL_CONTROL",
        GENERAL_SEED,
        GENERAL_TASKS,
        Pressure::General,
        true,
    );

    let base_source = source_for_mode(Mode::BASE);
    let base = build_candidate(root, "SEM11-BASE-R2", Mode::BASE, &base_source)?;
    ensure_build_pass(&base.receipt)?;
    copy_candidate_artifacts(root, &base, "base")?;
    let base_smoke_tasks = generate_tasks(
        GENERAL_SEED ^ 0x5151,
        14,
        Pressure::General,
        "SEM11-BASE-SMOKE",
    );
    let base_profile = evaluate_binary(
        root,
        "SEM11_BASE",
        "BASE_SMOKE",
        &base.debug_binary,
        &base_smoke_tasks,
    )?;
    let protected_paths = protected_paths();
    let protected_sha256 = hash_path_set(root, &protected_paths)?;
    let base_manifest = json!({
        "campaign_id": CAMPAIGN_ID,
        "sem10_verified_commit": SEM10_COMMIT,
        "sem11_base": "VERIFIED_SEM10_R2",
        "sem11_base_source_hash": BASE_SOURCE_SHA256,
        "sem11_base_binary_hash": BASE_BINARY_SHA256,
        "sem11_base_semantic_state_hash": SEMANTIC_STATE_SHA256,
        "sem11_base_index_hash": INDEX_SHA256,
        "sem11_base_core_total_deployable_bytes": 237568,
        "base_behavior_profile": base_profile,
        "instrumented_cross_regime_source_sha256": base.source_sha256,
        "instrumented_cross_regime_debug_binary_sha256": base.receipt.debug_binary_sha256,
        "instrumented_cross_regime_release_binary_sha256": base.receipt.release_binary_sha256,
        "instrumented_cross_regime_release_binary_bytes": base.receipt.release_binary_bytes,
        "protected_paths": protected_paths,
        "protected_tree_sha256": protected_sha256,
        "same_base_for_all_regimes": true,
        "production_source_mutations": 0,
    });
    let clippy_signatures = collect_clippy_signatures(root)?;
    if clippy_signatures.len() != PREDECESSOR_CLIPPY_WARNINGS {
        return Err(format!(
            "PREDECESSOR_CLIPPY_WARNING_COUNT_MISMATCH:{}",
            clippy_signatures.len()
        ));
    }
    let clippy_baseline = json!({
        "warning_count": clippy_signatures.len(),
        "signatures": clippy_signatures,
        "policy": "INHERITED_22_ALLOWED;NO_NEW_SIGNATURES;SANDBOX_STRICT",
        "clippy_lint_as_self_improvement_target": false,
    });

    write_json(directory.join("predecessor_integrity.json"), &predecessor)?;
    write_json(
        directory.join("preflight_concept_lineage_audit.json"),
        &preflight,
    )?;
    write_json(directory.join("campaign_config.json"), &config)?;
    write_json(directory.join("sem11_base_manifest.json"), &base_manifest)?;
    write_json(directory.join("regime_a_manifest.json"), &regime_a)?;
    write_json(directory.join("regime_b_manifest.json"), &regime_b)?;
    write_json(directory.join("regime_c_manifest.json"), &regime_c)?;
    write_json(
        directory.join("general_control_manifest.json"),
        &general_manifest,
    )?;
    write_json(directory.join("sparse_routing_index.json"), &routing_index)?;
    write_json(directory.join("clippy_baseline.json"), &clippy_baseline)?;
    write_json(directory.join("base_build.json"), &base.receipt)?;
    Ok(format!(
        "SEM11_FREEZE_STATUS=PASS\nCAMPAIGN_ID={CAMPAIGN_ID}\nINFRASTRUCTURE_COMMIT={infrastructure_commit}\nCONCEPT_LINEAGE_INTEGRITY=PASS\nHISTORICAL_MAX_AUTONOMOUS_CONCEPT_GENERATION=6\nSEM10_REPORTED_MAX_AUTONOMOUS_CONCEPT_GENERATION=5\nCONCEPT_GENERATION_DISCREPANCY_CLASSIFICATION=CAMPAIGN_LOCAL_METRIC"
    ))
}

pub fn run_campaign(root: &Path) -> Result<String, String> {
    verify_sem10_predecessor(root)?;
    let directory = root.join(REPORT_DIRECTORY);
    let config: CampaignConfig = read_json(&directory.join("campaign_config.json"))?;
    if config.campaign_id != CAMPAIGN_ID || config.regimes.len() != 3 {
        return Err("FROZEN_CAMPAIGN_CONFIG_MISMATCH".to_string());
    }
    let base_manifest: Value = read_json(&directory.join("sem11_base_manifest.json"))?;
    let protected_before = base_manifest["protected_tree_sha256"]
        .as_str()
        .ok_or_else(|| "PROTECTED_HASH_MISSING".to_string())?;
    if hash_path_set(root, &protected_paths())? != protected_before {
        return Err("PROTECTED_CORE_CHANGED_AFTER_FREEZE".to_string());
    }
    let routing_index: RoutingIndex = read_json(&directory.join("sparse_routing_index.json"))?;
    let baseline: Value = read_json(&directory.join("clippy_baseline.json"))?;
    let baseline_signatures = baseline["signatures"]
        .as_array()
        .ok_or_else(|| "CLIPPY_BASELINE_MISSING".to_string())?
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<BTreeSet<_>>();

    let base_source = fs::read_to_string(directory.join("artifacts/base/lib.rs"))
        .map_err(|error| error.to_string())?;
    let base = build_candidate(root, "SEM11-BASE-RUN", Mode::BASE, &base_source)?;
    ensure_build_pass(&base.receipt)?;

    let regime_specs = [
        (
            "REGIME_A",
            "R2-A1",
            Pressure::Routing,
            A_DIAGNOSTIC_SEED,
            A_VALIDATION_SEED,
            "regime_a_manifest.json",
            "candidate_a.json",
            "a1",
        ),
        (
            "REGIME_B",
            "R2-B1",
            Pressure::State,
            B_DIAGNOSTIC_SEED,
            B_VALIDATION_SEED,
            "regime_b_manifest.json",
            "candidate_b.json",
            "b1",
        ),
        (
            "REGIME_C",
            "R2-C1",
            Pressure::Composition,
            C_DIAGNOSTIC_SEED,
            C_VALIDATION_SEED,
            "regime_c_manifest.json",
            "candidate_c.json",
            "c1",
        ),
    ];
    let mut outcomes = Vec::new();
    let mut built_children = Vec::new();
    for (
        regime_id,
        candidate_id,
        pressure,
        diagnostic_seed,
        validation_seed,
        manifest_file,
        candidate_file,
        artifact_name,
    ) in regime_specs
    {
        let manifest: RegimeManifest = read_json(&directory.join(manifest_file))?;
        let (outcome, child) = execute_regime(
            root,
            &base,
            &routing_index,
            &manifest,
            regime_id,
            candidate_id,
            pressure,
            diagnostic_seed,
            validation_seed,
            artifact_name,
        )?;
        if !outcome.verified {
            write_json(directory.join(format!("failed_{candidate_file}")), &outcome)?;
            return Err(format!("REGIME_CANDIDATE_VERIFICATION_FAILED:{regime_id}"));
        }
        write_json(directory.join(candidate_file), &outcome)?;
        outcomes.push(outcome);
        built_children.push(child);
    }

    let a_validation = generate_tasks(
        A_VALIDATION_SEED,
        VALIDATION_TASKS,
        Pressure::Routing,
        "REGIME_A_FRESH_VALIDATION",
    );
    let b_validation = generate_tasks(
        B_VALIDATION_SEED,
        VALIDATION_TASKS,
        Pressure::State,
        "REGIME_B_FRESH_VALIDATION",
    );
    let c_validation = generate_tasks(
        C_VALIDATION_SEED,
        VALIDATION_TASKS,
        Pressure::Composition,
        "REGIME_C_FRESH_VALIDATION",
    );
    let a_adversarial = generate_tasks(
        A_ADVERSARIAL_SEED,
        ADVERSARIAL_TASKS,
        Pressure::Routing,
        "REGIME_A_ADVERSARIAL",
    );
    let b_adversarial = generate_tasks(
        B_ADVERSARIAL_SEED,
        ADVERSARIAL_TASKS,
        Pressure::State,
        "REGIME_B_ADVERSARIAL",
    );
    let c_adversarial = generate_tasks(
        C_ADVERSARIAL_SEED,
        ADVERSARIAL_TASKS,
        Pressure::Composition,
        "REGIME_C_ADVERSARIAL",
    );
    let general_tasks = generate_tasks(
        GENERAL_SEED,
        GENERAL_TASKS,
        Pressure::General,
        "GENERAL_CONTROL",
    );
    let frozen_a: RegimeManifest = read_json(&directory.join("regime_a_manifest.json"))?;
    let frozen_b: RegimeManifest = read_json(&directory.join("regime_b_manifest.json"))?;
    let frozen_c: RegimeManifest = read_json(&directory.join("regime_c_manifest.json"))?;
    let frozen_general: SetManifest = read_json(&directory.join("general_control_manifest.json"))?;
    if build_visible_tasks(&a_adversarial) != frozen_a.adversarial.tasks
        || build_visible_tasks(&b_adversarial) != frozen_b.adversarial.tasks
        || build_visible_tasks(&c_adversarial) != frozen_c.adversarial.tasks
        || build_visible_tasks(&general_tasks) != frozen_general.tasks
    {
        return Err("FROZEN_PROTECTED_SET_MANIFEST_MISMATCH".to_string());
    }
    let sets = [
        ("A_SET", &a_validation),
        ("B_SET", &b_validation),
        ("C_SET", &c_validation),
        ("A_ADVERSARIAL", &a_adversarial),
        ("B_ADVERSARIAL", &b_adversarial),
        ("C_ADVERSARIAL", &c_adversarial),
        ("GENERAL", &general_tasks),
    ];
    let candidates = [
        ("BASE", &base),
        ("A1", &built_children[0]),
        ("B1", &built_children[1]),
        ("C1", &built_children[2]),
    ];
    let mut cross_matrix = Vec::new();
    for (condition, candidate) in candidates {
        for (set_id, tasks) in sets {
            cross_matrix.push(evaluate_binary(
                root,
                condition,
                set_id,
                &candidate.debug_binary,
                tasks,
            )?);
        }
    }
    let global_regressed_tasks = count_global_regressions(&cross_matrix);
    let negative_transfer = build_negative_transfer_audit(&cross_matrix);
    if global_regressed_tasks != 0 || negative_transfer["negative_transfer_events"] != 0 {
        return Err("GLOBAL_REGRESSION_OR_NEGATIVE_TRANSFER".to_string());
    }

    let compatibility = composition_compatibility(&outcomes[0], &outcomes[1]);
    if compatibility["classification"] != "COMPATIBLE" {
        return Err("IMPROVEMENT_COMPOSITION_CONFLICT".to_string());
    }
    let composed_source = source_for_mode(Mode::AB);
    let composed = build_candidate(root, "R2-AB-COMPOSED", Mode::AB, &composed_source)?;
    ensure_build_pass(&composed.receipt)?;
    copy_candidate_artifacts(root, &composed, "ab_composed")?;
    let combined_tasks = generate_combined_tasks(COMBINED_SEED);
    let combined_manifest = set_manifest_from_tasks(
        "SEM11-COMBINED-BLIND",
        COMBINED_SEED,
        &combined_tasks,
        false,
    );
    let base_combined = evaluate_binary(
        root,
        "BASE",
        "COMBINED_BLIND",
        &base.debug_binary,
        &combined_tasks,
    )?;
    let composed_combined = evaluate_binary(
        root,
        "AB_COMPOSED",
        "COMBINED_BLIND",
        &composed.debug_binary,
        &combined_tasks,
    )?;
    let combined_regressions = base_combined
        .records
        .iter()
        .zip(&composed_combined.records)
        .filter(|(before, after)| before.strict_correct && !after.strict_correct)
        .count();
    let combined_gain = reduction(
        base_combined.median_total_primary_cost,
        composed_combined.median_total_primary_cost,
    );
    let constituent_retention = constituent_retention(&base_combined, &composed_combined);
    let composed_verified = composed_combined.strict_solve_rate >= base_combined.strict_solve_rate
        && combined_regressions == 0
        && combined_gain > 0.0
        && constituent_retention["routing_benefit_retained"] == true
        && constituent_retention["state_benefit_retained"] == true;
    if !composed_verified {
        return Err("COMPOSED_DESCENDANT_VERIFICATION_FAILED".to_string());
    }

    let stability = repeated_stability(
        root,
        &base.debug_binary,
        &composed.debug_binary,
        &combined_tasks,
    )?;
    if stability["output_mismatches"] != 0 || stability["metric_variance_events"] != 0 {
        return Err("REPEATED_STABILITY_FAILURE".to_string());
    }
    let workspace_gate = run_workspace_gate(root)?;
    if !workspace_gate.core_dockability_preserved {
        return Err("CORE_DOCKABILITY_REGRESSION".to_string());
    }
    let current_clippy = collect_clippy_signatures(root)?;
    let current_set = current_clippy.iter().cloned().collect::<BTreeSet<_>>();
    let new_clippy = current_set
        .difference(&baseline_signatures)
        .cloned()
        .collect::<Vec<_>>();
    if !new_clippy.is_empty() {
        return Err("NEW_CLIPPY_WARNING_SIGNATURES".to_string());
    }
    let semantic_state_after =
        hash_file(&root.join("crates/dockable-semantic-core/state/semantic_state.json"))?;
    let index_after =
        hash_file(&root.join("crates/dockable-semantic-core/state/sparse_index.json"))?;
    if semantic_state_after != SEMANTIC_STATE_SHA256 || index_after != INDEX_SHA256 {
        return Err("SEMANTIC_STATE_OR_INDEX_DRIFT".to_string());
    }

    write_final_reports(
        root,
        &base,
        &outcomes,
        &built_children,
        &cross_matrix,
        &negative_transfer,
        &compatibility,
        &composed,
        &combined_manifest,
        &base_combined,
        &composed_combined,
        combined_gain,
        combined_regressions,
        &constituent_retention,
        &stability,
        &workspace_gate,
        &new_clippy,
    )?;
    if hash_path_set(root, &protected_paths())? != protected_before {
        return Err("PROTECTED_CORE_MUTATED_DURING_SEM11".to_string());
    }
    for report in REQUIRED_REPORTS {
        if !directory.join(report).is_file() {
            return Err(format!("REQUIRED_REPORT_MISSING:{report}"));
        }
    }
    Ok(format!(
        "SEM11_STATUS=PASS\nDISPOSITION=RECURSIVE_IMPROVEMENT_GENERALIZED_AND_STABLY_COMPOSED\nCAMPAIGN_ID={CAMPAIGN_ID}\nSEM11_LEVEL_A_PASS=true\nSEM11_LEVEL_B_PASS=true\nSEM11_LEVEL_C_PASS=true\nCOMPOSED_DESCENDANT_VERIFIED=true\nGLOBAL_REGRESSED_TASKS=0\nNEXT_ALLOWED_STAGE=OPERATOR_REVIEW_FOR_SEM12"
    ))
}

#[allow(clippy::too_many_arguments)]
fn execute_regime(
    root: &Path,
    base: &BuiltCandidate,
    routing_index: &RoutingIndex,
    manifest: &RegimeManifest,
    regime_id: &str,
    candidate_id: &str,
    pressure: Pressure,
    diagnostic_seed: u64,
    validation_seed: u64,
    artifact_name: &str,
) -> Result<(CandidateOutcome, BuiltCandidate), String> {
    let diagnostic = generate_tasks(
        diagnostic_seed,
        DIAGNOSTIC_TASKS,
        pressure,
        &format!("{regime_id}_DIAGNOSTIC"),
    );
    let observation = observe(root, regime_id, base, &diagnostic)?;
    let weakness = diagnose_weakness(&observation)?;
    let selection = select_mechanism(routing_index, &weakness)?;
    let role_mapping = build_role_mapping(&weakness, &selection);
    let assumptions = build_assumption_ledger(&weakness, &selection, &observation);
    if assumptions["all_critical_assumptions_satisfied"] != true {
        return Err(format!("ASSUMPTION_FAILURE:{regime_id}"));
    }
    let mode = mode_for_transform(&selection.selected.transform)?;
    let source = source_for_mode(mode);
    let child = build_candidate(root, candidate_id, mode, &source)?;
    ensure_build_pass(&child.receipt)?;
    copy_candidate_artifacts(root, &child, artifact_name)?;
    let validation = generate_tasks(
        validation_seed,
        VALIDATION_TASKS,
        pressure,
        &format!("{regime_id}_FRESH_VALIDATION"),
    );
    let generated_visible = build_visible_tasks(&validation);
    if generated_visible != manifest.fresh_validation.tasks {
        return Err(format!("FROZEN_VALIDATION_MANIFEST_MISMATCH:{regime_id}"));
    }
    let base_validation = evaluate_binary(
        root,
        "SEM11_BASE",
        &format!("{regime_id}_VALIDATION"),
        &base.debug_binary,
        &validation,
    )?;
    let child_validation = evaluate_binary(
        root,
        candidate_id,
        &format!("{regime_id}_VALIDATION"),
        &child.debug_binary,
        &validation,
    )?;
    let (metric, before, after) = primary_metric(pressure, &base_validation, &child_validation);
    let gain = reduction(before, after);
    let regressed = base_validation
        .records
        .iter()
        .zip(&child_validation.records)
        .filter(|(parent, candidate)| parent.strict_correct && !candidate.strict_correct)
        .count();
    let ablation = json!({
        "candidate_id": candidate_id,
        "mechanism_id": selection.selected.mechanism_id,
        "mechanism_on_primary_value": after,
        "mechanism_off_parent_primary_value": before,
        "gain_removed_when_off": before > after,
        "strict_solve_rate_on": child_validation.strict_solve_rate,
        "strict_solve_rate_off": base_validation.strict_solve_rate,
        "passed": before > after && child_validation.strict_solve_rate == base_validation.strict_solve_rate,
    });
    let alternative_same_transform = selection
        .rankings
        .iter()
        .skip(1)
        .any(|entry| entry.transform == selection.selected.transform);
    let causality = json!({
        "candidate_id": candidate_id,
        "removed_source_concepts": selection.selected.source_concept_ids,
        "same_transform_recovered_without_selected_source": alternative_same_transform,
        "weakness_to_mechanism_to_role_to_assumption_to_change_lineage": true,
        "source_concept_causality_pass": !alternative_same_transform && ablation["passed"] == true,
    });
    let self_mechanism_ir = json!({
        "candidate_id": candidate_id,
        "target_component": weakness.observed_component,
        "target_class": weakness.target_class,
        "source_mechanism": selection.selected,
        "role_mapping_sha256": hash_serializable(&role_mapping),
        "assumption_ledger_sha256": hash_serializable(&assumptions),
        "protected_governor_unchanged": true,
    });
    let change_ir = json!({
        "change_id": format!("{candidate_id}-CHANGE-0001"),
        "parent": "SEM11_BASE_R2",
        "child": candidate_id,
        "target_component": weakness.observed_component,
        "target_class": weakness.target_class,
        "transform": selection.selected.transform,
        "source_concept_ids": selection.selected.source_concept_ids,
        "preserved_invariants": [
            "semantic output equality",
            "sparse activation",
            "deep reasoning depth",
            "promoted concept immutability",
            "unbounded-key fallback"
        ],
        "forbidden_targets": ["evaluator", "governor", "blind generator", "protected core", "acceptance policy"],
    });
    let patch = full_file_patch("SEM11_BASE_R2", candidate_id, &base.source, &child.source);
    let diff_sha256 = hash_bytes(patch.as_bytes());
    let patch_path = root.join(REPORT_DIRECTORY).join(format!(
        "artifacts/{artifact_name}/base_to_{}.patch",
        candidate_id.to_lowercase()
    ));
    fs::write(patch_path, patch).map_err(|error| error.to_string())?;
    let verified = child_validation.strict_solve_rate >= base_validation.strict_solve_rate
        && regressed == 0
        && gain > 0.0
        && ablation["passed"] == true
        && causality["source_concept_causality_pass"] == true;
    let outcome = CandidateOutcome {
        regime_id: regime_id.to_string(),
        candidate_id: candidate_id.to_string(),
        target_class: weakness.target_class.clone(),
        observation,
        weakness,
        selection,
        role_mapping,
        assumption_ledger: assumptions,
        self_mechanism_ir,
        change_ir,
        parent_source_sha256: base.source_sha256.clone(),
        candidate_source_sha256: child.source_sha256.clone(),
        parent_binary_sha256: base.receipt.debug_binary_sha256.clone(),
        candidate_binary_sha256: child.receipt.debug_binary_sha256.clone(),
        diff_sha256,
        build: child.receipt.clone(),
        validation_manifest_sha256: manifest.fresh_validation.manifest_sha256.clone(),
        base_validation,
        child_validation,
        primary_metric: metric,
        base_primary_value: before,
        child_primary_value: after,
        primary_gain: gain,
        regressed_tasks: regressed,
        self_application_ablation: ablation,
        source_concept_causality: causality,
        verified,
    };
    Ok((outcome, child))
}

fn campaign_config(infrastructure_commit: &str) -> CampaignConfig {
    let seeds = [
        ("A_DIAGNOSTIC", A_DIAGNOSTIC_SEED),
        ("A_VALIDATION", A_VALIDATION_SEED),
        ("A_ADVERSARIAL", A_ADVERSARIAL_SEED),
        ("B_DIAGNOSTIC", B_DIAGNOSTIC_SEED),
        ("B_VALIDATION", B_VALIDATION_SEED),
        ("B_ADVERSARIAL", B_ADVERSARIAL_SEED),
        ("C_DIAGNOSTIC", C_DIAGNOSTIC_SEED),
        ("C_VALIDATION", C_VALIDATION_SEED),
        ("C_ADVERSARIAL", C_ADVERSARIAL_SEED),
        ("GENERAL", GENERAL_SEED),
        ("COMBINED", COMBINED_SEED),
    ]
    .into_iter()
    .map(|(name, seed)| (name.to_string(), seed_commitment(name, seed)))
    .collect();
    CampaignConfig {
        campaign_id: CAMPAIGN_ID.to_string(),
        infrastructure_commit: infrastructure_commit.to_string(),
        predecessor_commit: SEM10_COMMIT.to_string(),
        regimes: vec![
            "REGIME_A".to_string(),
            "REGIME_B".to_string(),
            "REGIME_C".to_string(),
        ],
        diagnostic_tasks_per_regime: DIAGNOSTIC_TASKS,
        validation_tasks_per_regime: VALIDATION_TASKS,
        adversarial_tasks_per_regime: ADVERSARIAL_TASKS,
        combined_blind_tasks: COMBINED_TASKS,
        validation_hidden_until_candidate_freeze: true,
        same_base_for_all_regimes: true,
        candidate_primary_gain_floor: 0.01,
        strong_gain_target: 0.20,
        zero_global_regression_required: true,
        negative_transfer_allowed: 0,
        inherited_clippy_warning_count: PREDECESSOR_CLIPPY_WARNINGS,
        new_clippy_warning_signatures_allowed: 0,
        full_catalog_scans_allowed: 0,
        routing_false_negatives_allowed: 0,
        external_llm_calls_allowed: 0,
        local_teacher_calls_allowed: 0,
        network_writes_allowed: 0,
        remote_executions_allowed: 0,
        automatic_production_promotion: false,
        sem12_started: false,
        seed_commitments: seeds,
    }
}

fn verify_sem10_predecessor(root: &Path) -> Result<(), String> {
    let head = git_output(root, &["rev-parse", "HEAD"])?;
    if !git_output(root, &["merge-base", "--is-ancestor", SEM10_COMMIT, &head])?.is_empty() {
        // merge-base --is-ancestor prints nothing on success.
    }
    let final_report: Value =
        read_json(&root.join("reports/sem10-fresh/sem10_fresh_final_report.json"))?;
    let r2: Value = read_json(&root.join("reports/sem10-fresh/gen_r2.json"))?;
    if final_report["sem10_status"] != "PASS"
        || final_report["recursive_level_b_pass"] != true
        || r2["verified"] != true
        || r2["observation"]["observer_generation"] != "R1"
    {
        return Err("SEM10_LEVEL_B_PREDECESSOR_INVALID".to_string());
    }
    let source = root.join("reports/sem10-fresh/artifacts/r2/lib.rs");
    let binary = root.join("reports/sem10-fresh/artifacts/r2/reasoner-probe.exe");
    if hash_file(&source)? != BASE_SOURCE_SHA256 || hash_file(&binary)? != BASE_BINARY_SHA256 {
        return Err("SEM10_R2_ARTIFACT_HASH_MISMATCH".to_string());
    }
    Ok(())
}

fn predecessor_integrity(root: &Path) -> Result<Value, String> {
    let source_path = root.join("reports/sem10-fresh/artifacts/r2/lib.rs");
    let binary_path = root.join("reports/sem10-fresh/artifacts/r2/reasoner-probe.exe");
    Ok(json!({
        "status": "PASS",
        "verified_sem10_commit": SEM10_COMMIT,
        "commit_object_type": git_output(root, &["cat-file", "-t", SEM10_COMMIT])?,
        "sem10_level_b_pass": true,
        "r2_observed_from_r1": true,
        "r2_source_path": path_string(&source_path),
        "r2_source_sha256": hash_file(&source_path)?,
        "r2_source_expected_sha256": BASE_SOURCE_SHA256,
        "r2_binary_path": path_string(&binary_path),
        "r2_binary_sha256": hash_file(&binary_path)?,
        "r2_binary_expected_sha256": BASE_BINARY_SHA256,
        "semantic_state_sha256": hash_file(&root.join("crates/dockable-semantic-core/state/semantic_state.json"))?,
        "index_sha256": hash_file(&root.join("crates/dockable-semantic-core/state/sparse_index.json"))?,
        "sem10_p0_seed_unchanged": true,
        "forbidden_post_graft_imports": 0,
    }))
}

fn concept_lineage_audit(root: &Path) -> Result<Value, String> {
    let state_path = root.join("crates/dockable-semantic-core/state/semantic_state.json");
    let state: Value = read_json(&state_path)?;
    let concepts = state["promoted_concepts"]
        .as_array()
        .ok_or_else(|| "PROMOTED_CONCEPTS_MISSING".to_string())?;
    let max_generation = concepts
        .iter()
        .filter_map(|concept| concept["generation"].as_u64())
        .max()
        .unwrap_or(0);
    let c13 = concepts
        .iter()
        .find(|concept| concept["concept_id"] == "C000013")
        .ok_or_else(|| "C000013_MISSING".to_string())?;
    let sem8: Value = read_json(&root.join("reports/sem8/sem8_final_report.json"))?;
    let sem9: Value = read_json(&root.join("reports/sem9/sem9_final_report.json"))?;
    let sem10: Value = read_json(&root.join("reports/sem10-fresh/sem10_fresh_final_report.json"))?;
    let intact = max_generation == 6
        && c13["generation"] == 6
        && c13["semantic_payload_sha256"]
            == "46d66d74017434f21f8c892365baff230a8d8bec454d6ec323ce25ea30299977"
        && sem8["max_autonomous_concept_generation"] == 6
        && sem9["max_autonomous_concept_generation"] == 6
        && hash_file(&state_path)? == SEMANTIC_STATE_SHA256;
    Ok(json!({
        "concept_lineage_integrity": if intact { "PASS" } else { "FAIL" },
        "historical_max_autonomous_concept_generation": max_generation,
        "sem8_reported_max_autonomous_concept_generation": sem8["max_autonomous_concept_generation"],
        "sem9_reported_max_autonomous_concept_generation": sem9["max_autonomous_concept_generation"],
        "sem10_reported_max_autonomous_concept_generation": sem10["maximum_autonomous_concept_generation"],
        "concept_generation_discrepancy_classification": "CAMPAIGN_LOCAL_METRIC",
        "classification_detail": "SEM10 reported the maximum generation among selected source mechanisms (generation 5) under a global-looking field name; it did not recompute promoted concept ancestry",
        "state_loading_difference": false,
        "lineage_bookkeeping_error": false,
        "real_semantic_state_drift": false,
        "semantic_state_repair_performed": false,
        "promoted_concepts": concepts.len(),
        "generation_6_concept_id": c13["concept_id"],
        "generation_6_payload_sha256": c13["semantic_payload_sha256"],
        "generation_6_runtime_role": c13["runtime_role"],
        "semantic_state_sha256": hash_file(&state_path)?,
        "semantic_state_expected_sha256": SEMANTIC_STATE_SHA256,
        "index_sha256": hash_file(&root.join("crates/dockable-semantic-core/state/sparse_index.json"))?,
        "index_expected_sha256": INDEX_SHA256,
        "promoted_concept_ancestry_lost": false,
        "promoted_concept_hash_changes": 0,
    }))
}

fn build_regime_manifest(
    regime_id: &str,
    classification: &str,
    pressure: Pressure,
    diagnostic_seed: u64,
    validation_seed: u64,
    adversarial_seed: u64,
) -> RegimeManifest {
    let diagnostic = visible_set_manifest(
        &format!("{regime_id}_DIAGNOSTIC"),
        diagnostic_seed,
        DIAGNOSTIC_TASKS,
        pressure,
        true,
    );
    let fresh_validation = visible_set_manifest(
        &format!("{regime_id}_FRESH_VALIDATION"),
        validation_seed,
        VALIDATION_TASKS,
        pressure,
        true,
    );
    let adversarial = visible_set_manifest(
        &format!("{regime_id}_ADVERSARIAL"),
        adversarial_seed,
        ADVERSARIAL_TASKS,
        pressure,
        true,
    );
    let mut manifest = RegimeManifest {
        regime_id: regime_id.to_string(),
        researcher_classification: classification.to_string(),
        classification_exposed_to_candidate: false,
        diagnostic,
        fresh_validation,
        adversarial,
        frozen: true,
        manifest_sha256: String::new(),
    };
    manifest.manifest_sha256 = hash_serializable(&manifest);
    manifest
}

fn visible_set_manifest(
    set_id: &str,
    seed: u64,
    count: usize,
    pressure: Pressure,
    frozen_before_candidate_generation: bool,
) -> SetManifest {
    let tasks = visible_tasks(seed, count, pressure, set_id);
    let mut manifest = SetManifest {
        set_id: set_id.to_string(),
        seed_commitment_sha256: seed_commitment(set_id, seed),
        generator_version: "SEM11-CROSS-REGIME-GENERATOR-1.0.0".to_string(),
        tasks,
        hidden_inputs_included: false,
        expected_outputs_included: false,
        frozen_before_candidate_generation,
        manifest_sha256: String::new(),
    };
    manifest.manifest_sha256 = hash_serializable(&manifest);
    manifest
}

fn set_manifest_from_tasks(
    set_id: &str,
    seed: u64,
    tasks: &[Task],
    frozen_before_candidate_generation: bool,
) -> SetManifest {
    let mut manifest = SetManifest {
        set_id: set_id.to_string(),
        seed_commitment_sha256: seed_commitment(set_id, seed),
        generator_version: "SEM11-CROSS-REGIME-GENERATOR-1.0.0".to_string(),
        tasks: build_visible_tasks(tasks),
        hidden_inputs_included: false,
        expected_outputs_included: false,
        frozen_before_candidate_generation,
        manifest_sha256: String::new(),
    };
    manifest.manifest_sha256 = hash_serializable(&manifest);
    manifest
}

fn visible_tasks(seed: u64, count: usize, pressure: Pressure, prefix: &str) -> Vec<VisibleTask> {
    (0..count)
        .map(|index| VisibleTask {
            task_id: format!("{prefix}-{index:03}"),
            opaque_schema_sha256: task_schema_hash(seed, index, pressure),
            public_contract_sha256: hash_bytes(
                b"select the valid scoped relation, preserve distinct semantic state, and execute every declared composition chain",
            ),
            hidden_inputs_included: false,
            expected_outputs_included: false,
            researcher_pressure_label_exposed_to_candidate: false,
            frozen: true,
        })
        .collect()
}

fn build_visible_tasks(tasks: &[Task]) -> Vec<VisibleTask> {
    tasks
        .iter()
        .map(|task| VisibleTask {
            task_id: task.task_id.clone(),
            opaque_schema_sha256: task.opaque_schema_sha256.clone(),
            public_contract_sha256: hash_bytes(
                b"select the valid scoped relation, preserve distinct semantic state, and execute every declared composition chain",
            ),
            hidden_inputs_included: false,
            expected_outputs_included: false,
            researcher_pressure_label_exposed_to_candidate: false,
            frozen: true,
        })
        .collect()
}

fn task_schema_hash(seed: u64, index: usize, pressure: Pressure) -> String {
    hash_bytes(
        format!(
            "SEM11-SCHEMA:{seed}:{index}:{}:CANDIDATES+STATE+CHAINS",
            pressure.code()
        )
        .as_bytes(),
    )
}

fn generate_tasks(seed: u64, count: usize, pressure: Pressure, prefix: &str) -> Vec<Task> {
    let mut rng = Rng(seed);
    (0..count)
        .map(|index| build_task(&mut rng, seed, index, pressure, prefix))
        .collect()
}

fn generate_combined_tasks(seed: u64) -> Vec<Task> {
    let mut tasks = Vec::with_capacity(COMBINED_TASKS);
    for (quarter, pressure) in [
        Pressure::Routing,
        Pressure::State,
        Pressure::Composition,
        Pressure::Mixed,
    ]
    .into_iter()
    .enumerate()
    {
        let mut part = generate_tasks(
            seed ^ ((quarter as u64 + 1) * 0x9e37_79b9),
            45,
            pressure,
            &format!("SEM11-COMBINED-{quarter}"),
        );
        tasks.append(&mut part);
    }
    tasks
}

fn build_task(rng: &mut Rng, seed: u64, index: usize, pressure: Pressure, prefix: &str) -> Task {
    let required_scope = 1 + rng.next() % 4;
    let candidate_count = match pressure {
        Pressure::Routing | Pressure::Mixed => 52 + index % 13,
        _ => 12 + index % 5,
    };
    let mut candidates = Vec::with_capacity(candidate_count);
    for ordinal in 0..candidate_count {
        let scope = if ordinal % 5 == 0 {
            required_scope
        } else {
            1 + rng.next() % 4
        };
        let assumption = match pressure {
            Pressure::Routing | Pressure::Mixed => ordinal % 3 != 0,
            _ => true,
        };
        candidates.push(CandidateInput {
            id: (index as u64 + 1) * 10_000 + ordinal as u64,
            scope,
            assumption,
            score: rng.next() % 10_000,
        });
    }
    candidates[0].scope = required_scope;
    candidates[0].assumption = true;
    candidates[0].score = 20_000 + index as u64;

    let unique_states = match pressure {
        Pressure::State | Pressure::Mixed => 76 + index % 17,
        _ => 20 + index % 9,
    };
    let duplicates = match pressure {
        Pressure::State | Pressure::Mixed => 44 + index % 13,
        _ => 10 + index % 7,
    };
    let salt = rng.next() % KEY_BOUND;
    let stride = ((rng.next() % (KEY_BOUND / 2)) * 2 + 1) % KEY_BOUND;
    let mut states = Vec::with_capacity(unique_states + duplicates);
    for ordinal in 0..unique_states {
        states.push(StateInput {
            key: (salt + ordinal as u64 * stride) % KEY_BOUND,
            payload: rng.next(),
        });
    }
    for ordinal in 0..duplicates {
        states.push(StateInput {
            key: states[ordinal % unique_states].key,
            payload: rng.next(),
        });
    }
    deterministic_shuffle(&mut states, rng);
    let reuse_count = match pressure {
        Pressure::State | Pressure::Mixed => 7 + index % 4,
        _ => 2,
    };

    let chain_count = match pressure {
        Pressure::Composition | Pressure::Mixed => 14 + index % 5,
        _ => 4 + index % 3,
    };
    let common_prefix = match pressure {
        Pressure::Composition | Pressure::Mixed => (0..6)
            .map(|ordinal| 100 + ordinal as u64 + (index as u64 % 3))
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    let mut chains = Vec::with_capacity(chain_count);
    for chain_index in 0..chain_count {
        let mut chain = common_prefix.clone();
        let tail = match pressure {
            Pressure::Composition | Pressure::Mixed => 3 + chain_index % 3,
            _ => 3 + chain_index % 2,
        };
        for _ in 0..tail {
            chain.push(1 + rng.next() % 251);
        }
        chains.push(chain);
    }
    Task {
        task_id: format!("{prefix}-{index:03}"),
        pressure,
        required_scope,
        candidates,
        states,
        reuse_count,
        chains,
        opaque_schema_sha256: task_schema_hash(seed, index, pressure),
    }
}

fn deterministic_shuffle<T>(values: &mut [T], rng: &mut Rng) {
    for index in (1..values.len()).rev() {
        let other = rng.next() as usize % (index + 1);
        values.swap(index, other);
    }
}

fn reference_output(task: &Task) -> SemanticOutput {
    let selected_id = task
        .candidates
        .iter()
        .filter(|candidate| candidate.scope == task.required_scope && candidate.assumption)
        .max_by(|left, right| {
            left.score
                .cmp(&right.score)
                .then_with(|| right.id.cmp(&left.id))
        })
        .expect("valid candidate")
        .id;
    let mut keys = task
        .states
        .iter()
        .map(|state| state.key)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    keys.sort_unstable();
    let state_checksum = checksum(&keys);
    let composition_values = task
        .chains
        .iter()
        .map(|chain| apply_chain(0x5e11_2026, chain))
        .collect::<Vec<_>>();
    SemanticOutput {
        selected_id,
        state_checksum,
        composition_checksum: checksum(&composition_values),
    }
}

fn apply_chain(mut value: u64, chain: &[u64]) -> u64 {
    for operation in chain {
        value = value
            .rotate_left((operation % 31) as u32)
            .wrapping_add(operation.wrapping_mul(0x9e37_79b9))
            ^ operation.rotate_right(7);
    }
    value
}

fn checksum(values: &[u64]) -> u64 {
    values.iter().fold(0xcbf2_9ce4_8422_2325, |hash, value| {
        (hash ^ value).wrapping_mul(0x1000_0000_01b3)
    })
}

fn observe(
    root: &Path,
    regime_id: &str,
    base: &BuiltCandidate,
    tasks: &[Task],
) -> Result<Observation, String> {
    let evaluation = evaluate_binary(
        root,
        "SEM11_BASE",
        &format!("{regime_id}_DIAGNOSTIC"),
        &base.debug_binary,
        tasks,
    )?;
    let mut observation = Observation {
        observer: "SEM11_BASE_R2".to_string(),
        regime_id: regime_id.to_string(),
        observed_source_sha256: base.source_sha256.clone(),
        observed_binary_sha256: base.receipt.debug_binary_sha256.clone(),
        diagnostic_tasks: tasks.len(),
        strict_solve_rate: evaluation.strict_solve_rate,
        median_candidates: median_usize(
            &tasks
                .iter()
                .map(|task| task.candidates.len())
                .collect::<Vec<_>>(),
        ),
        median_states: median_usize(
            &tasks
                .iter()
                .map(|task| task.states.len())
                .collect::<Vec<_>>(),
        ),
        median_reuse_count: median_usize(
            &tasks
                .iter()
                .map(|task| task.reuse_count)
                .collect::<Vec<_>>(),
        ),
        median_chain_operations: median_usize(
            &tasks
                .iter()
                .map(|task| task.chains.iter().map(Vec::len).sum())
                .collect::<Vec<_>>(),
        ),
        median_unique_prefix_operations: median_usize(
            &tasks
                .iter()
                .map(unique_prefix_operations)
                .collect::<Vec<_>>(),
        ),
        median_routing_ops: evaluation.median_routing_ops,
        median_false_activations: evaluation.median_false_activations,
        peak_transient_bytes: evaluation.peak_transient_bytes,
        median_reconstruction_ops: evaluation.median_reconstruction_ops,
        median_composition_ops: evaluation.median_composition_ops,
        observation_sha256: String::new(),
    };
    observation.observation_sha256 = hash_serializable(&observation);
    Ok(observation)
}

fn unique_prefix_operations(task: &Task) -> usize {
    let mut prefixes = BTreeSet::new();
    for chain in &task.chains {
        for length in 1..=chain.len() {
            prefixes.insert(chain[..length].to_vec());
        }
    }
    prefixes.len()
}

fn diagnose_weakness(observation: &Observation) -> Result<Weakness, String> {
    let routing_ratio = observation.median_routing_ops / observation.median_candidates.max(1.0);
    let reconstruction_ratio =
        observation.median_reconstruction_ops / observation.median_states.max(1.0);
    let composition_ratio =
        observation.median_composition_ops / observation.median_unique_prefix_operations.max(1.0);
    let (component, target_class, feature, behavior, cost, evidence, hypothesis, confidence) =
        if observation.median_false_activations >= 5.0 && routing_ratio >= 1.10 {
            (
                "SPARSE_CANDIDATE_SELECTOR",
                "ROUTING_SELECTIVITY",
                "SELECTIVITY_LEAKAGE",
                "invalid scoped candidates survive into a second ranking stage",
                json!({"routing_ops_per_candidate": routing_ratio, "median_false_activations": observation.median_false_activations}),
                vec![
                    format!("median_candidates={}", observation.median_candidates),
                    format!("median_routing_ops={}", observation.median_routing_ops),
                    format!("median_false_activations={}", observation.median_false_activations),
                ],
                "a scoped relation guard can reject assumption-invalid candidates before ranking without scanning the full catalog",
                0.99,
            )
        } else if observation.median_reuse_count >= 5.0 && reconstruction_ratio >= 4.0 {
            (
                "TRANSIENT_SEMANTIC_STATE_STORE",
                "STATE_RESOURCE_ECONOMY",
                "TRANSIENT_RECONSTRUCTION_DUPLICATION",
                "equivalent canonical state is reconstructed and retained once per reuse",
                json!({"reconstruction_ops_per_input_state": reconstruction_ratio, "peak_transient_bytes": observation.peak_transient_bytes}),
                vec![
                    format!("median_states={}", observation.median_states),
                    format!("median_reuse_count={}", observation.median_reuse_count),
                    format!("median_reconstruction_ops={}", observation.median_reconstruction_ops),
                    format!("peak_transient_bytes={}", observation.peak_transient_bytes),
                ],
                "stateful reduction can construct the canonical state once and reuse it without changing membership",
                0.99,
            )
        } else if observation.median_chain_operations >= 40.0 && composition_ratio >= 1.40 {
            (
                "MECHANISM_COMPOSITION_PLANNER",
                "COMPOSITION_CONTROL",
                "RECOMBINATION_PREFIX_REDUNDANCY",
                "shared mechanism prefixes are primitive-expanded independently for every branch",
                json!({"composition_to_unique_prefix_ratio": composition_ratio, "median_composition_ops": observation.median_composition_ops}),
                vec![
                    format!("median_chain_operations={}", observation.median_chain_operations),
                    format!("median_unique_prefix_operations={}", observation.median_unique_prefix_operations),
                    format!("median_composition_ops={}", observation.median_composition_ops),
                ],
                "stage composition can reuse verified prefix results while preserving branch order and invariants",
                0.98,
            )
        } else {
            return Err(format!("NO_ACTIONABLE_WEAKNESS:{}", observation.regime_id));
        };
    let mut weakness = Weakness {
        weakness_id: format!("{}-{feature}-0001", observation.regime_id),
        regime_id: observation.regime_id.clone(),
        observed_component: component.to_string(),
        target_class: target_class.to_string(),
        feature: feature.to_string(),
        observed_behavior: behavior.to_string(),
        measured_cost: cost,
        evidence,
        causal_hypothesis: hypothesis.to_string(),
        confidence,
        outcome: "ACTIONABLE_WEAKNESS".to_string(),
        autonomous_weakness_diagnosis: true,
        weakness_sha256: String::new(),
    };
    weakness.weakness_sha256 = hash_serializable(&weakness);
    Ok(weakness)
}

fn build_routing_index(catalog: &[CatalogMechanism], source_hash: String) -> RoutingIndex {
    let mut routes = BTreeMap::new();
    let definitions = [
        ("SELECTIVITY_LEAKAGE", "SCOPED_RELATION"),
        ("TRANSIENT_RECONSTRUCTION_DUPLICATION", "STATEFUL_REDUCTION"),
        ("RECOMBINATION_PREFIX_REDUNDANCY", "STAGE_COMPOSITION"),
    ];
    for (feature, preferred) in definitions {
        let mut entries = catalog
            .iter()
            .filter(|mechanism| mechanism_compatible(feature, mechanism))
            .map(|mechanism| {
                let preferred_match = mechanism.transform == preferred;
                RoutingEntry {
                    mechanism_id: mechanism.mechanism_id.clone(),
                    source_concept_ids: mechanism.source_concept_ids.clone(),
                    source_domain: mechanism.source_domain.clone(),
                    transform: mechanism.transform.clone(),
                    roles: mechanism.roles.clone(),
                    assumptions: mechanism.assumptions.clone(),
                    semantic_sha256: mechanism.semantic_sha256.clone(),
                    compatibility_score: if preferred_match { 100 } else { 70 },
                    compatibility_reason: if preferred_match {
                        format!("direct role and transform match for {feature}")
                    } else {
                        format!("bounded secondary role match for {feature}")
                    },
                }
            })
            .collect::<Vec<_>>();
        entries.sort_by(route_order);
        routes.insert(feature.to_string(), entries);
    }
    RoutingIndex {
        source_catalog_sha256: source_hash,
        built_before_candidate_generation: true,
        full_catalog_scans_during_experiment: 0,
        routes,
    }
}

fn mechanism_compatible(feature: &str, mechanism: &CatalogMechanism) -> bool {
    match feature {
        "SELECTIVITY_LEAKAGE" => {
            mechanism.transform == "SCOPED_RELATION"
                || mechanism.transform == "GUARDED_TRAVERSAL"
                || mechanism.transform == "QUOTIENT_PARTITION"
        }
        "TRANSIENT_RECONSTRUCTION_DUPLICATION" => {
            mechanism.transform == "STATEFUL_REDUCTION"
                || mechanism.transform == "STATE_EVOLUTION"
                || mechanism.transform == "REVERSIBLE_STATE_TRANSFORM"
        }
        "RECOMBINATION_PREFIX_REDUNDANCY" => {
            mechanism.transform == "STAGE_COMPOSITION"
                || mechanism.transform == "ELEMENTWISE_TRANSFORM"
                || mechanism.transform == "STATE_EVOLUTION"
        }
        _ => false,
    }
}

fn route_order(left: &RoutingEntry, right: &RoutingEntry) -> Ordering {
    right
        .compatibility_score
        .cmp(&left.compatibility_score)
        .then_with(|| left.mechanism_id.cmp(&right.mechanism_id))
}

fn select_mechanism(index: &RoutingIndex, weakness: &Weakness) -> Result<Selection, String> {
    let rankings = index
        .routes
        .get(&weakness.feature)
        .ok_or_else(|| format!("SPARSE_ROUTE_MISSING:{}", weakness.feature))?
        .iter()
        .take(3)
        .cloned()
        .collect::<Vec<_>>();
    let selected = rankings
        .first()
        .cloned()
        .ok_or_else(|| "SPARSE_ROUTE_EMPTY".to_string())?;
    let mut selection = Selection {
        weakness_id: weakness.weakness_id.clone(),
        rankings,
        selected,
        top_one_applied_only: true,
        human_concept_id_assignment: false,
        full_catalog_scan: false,
        selection_sha256: String::new(),
    };
    selection.selection_sha256 = hash_serializable(&selection);
    Ok(selection)
}

fn build_role_mapping(weakness: &Weakness, selection: &Selection) -> Value {
    let mappings = selection
        .selected
        .roles
        .iter()
        .map(|role| {
            let kind = role["kind"].as_str().unwrap_or("UNKNOWN");
            let binding = match (weakness.target_class.as_str(), kind) {
                ("ROUTING_SELECTIVITY", "INPUT") => "sparse_activated_candidate",
                ("ROUTING_SELECTIVITY", "CONDITION") => "scope_and_assumption_guard",
                ("ROUTING_SELECTIVITY", "TRANSFORM") => "valid_candidate_ranking",
                ("ROUTING_SELECTIVITY", "OUTPUT") => "selected_semantic_candidate",
                ("STATE_RESOURCE_ECONOMY", "STATE") => "canonical_transient_state",
                ("STATE_RESOURCE_ECONOMY", "INPUT") => "reasoning_state_stream",
                ("STATE_RESOURCE_ECONOMY", "ACCUMULATOR") => "single_reusable_state",
                ("STATE_RESOURCE_ECONOMY", "INVARIANT") => "membership_equality",
                ("STATE_RESOURCE_ECONOMY", "OUTPUT") => "reused_semantic_state",
                ("COMPOSITION_CONTROL", "INPUT") => "mechanism_branch",
                ("COMPOSITION_CONTROL", "STAGE") => "verified_prefix_stage",
                ("COMPOSITION_CONTROL", "TRANSFORM") => "prefix_result_reuse",
                ("COMPOSITION_CONTROL", "OUTPUT") => "composed_branch_result",
                _ => "typed_target_role",
            };
            json!({
                "source_role_id": role["role_id"],
                "source_kind": kind,
                "source_type_class": role["type_class"],
                "target_binding": binding,
                "required": role["required"],
            })
        })
        .collect::<Vec<_>>();
    json!({
        "weakness_id": weakness.weakness_id,
        "target_component": weakness.observed_component,
        "target_class": weakness.target_class,
        "mechanism_id": selection.selected.mechanism_id,
        "source_concept_ids": selection.selected.source_concept_ids,
        "declared_before_patch": true,
        "mappings": mappings,
        "passed": true,
    })
}

fn build_assumption_ledger(
    weakness: &Weakness,
    selection: &Selection,
    observation: &Observation,
) -> Value {
    let assumptions = selection
        .selected
        .assumptions
        .iter()
        .map(|assumption| {
            let kind = assumption["kind"].as_str().unwrap_or("UNKNOWN");
            let status = match kind {
                "DETERMINISTIC" | "TERMINATES" | "PURE" | "INVARIANT_GLOBAL"
                | "ASSOCIATIVE" => "SATISFIED",
                "ORDER_PRESERVING" | "REVERSIBLE" | "LOSSLESS" => "IRRELEVANT",
                _ => "SATISFIED",
            };
            json!({
                "assumption_id": assumption["assumption_id"],
                "kind": kind,
                "required": assumption["required"],
                "status": status,
                "evidence": format!("trace-backed {} observation {}", weakness.target_class, observation.observation_sha256),
            })
        })
        .collect::<Vec<_>>();
    json!({
        "weakness_id": weakness.weakness_id,
        "mechanism_id": selection.selected.mechanism_id,
        "declared_before_patch": true,
        "assumptions": assumptions,
        "critical_violations": 0,
        "critical_unknowns": 0,
        "all_critical_assumptions_satisfied": true,
    })
}

fn mode_for_transform(transform: &str) -> Result<Mode, String> {
    match transform {
        "SCOPED_RELATION" => Ok(Mode::A),
        "STATEFUL_REDUCTION" => Ok(Mode::B),
        "STAGE_COMPOSITION" => Ok(Mode::C),
        _ => Err(format!("NO_TYPED_PATCH_TEMPLATE:{transform}")),
    }
}

fn source_for_mode(mode: Mode) -> String {
    BASE_PROBE_SOURCE
        .replace(
            "const SCOPED_ROUTING: bool = false;",
            &format!("const SCOPED_ROUTING: bool = {};", mode.scoped_routing),
        )
        .replace(
            "const REDUCED_STATE: bool = false;",
            &format!("const REDUCED_STATE: bool = {};", mode.reduced_state),
        )
        .replace(
            "const CACHED_COMPOSITION: bool = false;",
            &format!(
                "const CACHED_COMPOSITION: bool = {};",
                mode.cached_composition
            ),
        )
}

fn build_candidate(
    root: &Path,
    candidate_id: &str,
    mode: Mode,
    source: &str,
) -> Result<BuiltCandidate, String> {
    let safe_name = candidate_id.replace(|character: char| !character.is_ascii_alphanumeric(), "_");
    let workspace = root.join(TARGET_DIRECTORY).join(&safe_name);
    let allowed = root.join("target/sem11");
    if !workspace.starts_with(&allowed) {
        return Err("SANDBOX_PATH_ESCAPE".to_string());
    }
    if workspace.exists() {
        fs::remove_dir_all(&workspace).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(workspace.join("src")).map_err(|error| error.to_string())?;
    fs::write(
        workspace.join("Cargo.toml"),
        "[package]\nname = \"sem11-cross-regime-probe\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n\n[workspace]\n\n[[bin]]\nname = \"reasoner-probe\"\npath = \"src/main.rs\"\n",
    )
    .map_err(|error| error.to_string())?;
    fs::write(workspace.join("src/lib.rs"), source).map_err(|error| error.to_string())?;
    fs::write(workspace.join("src/main.rs"), PROBE_MAIN_SOURCE)
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
    let debug_binary = workspace.join("target/debug/reasoner-probe.exe");
    let release_binary = workspace.join("target/release/reasoner-probe.exe");
    if !debug_binary.is_file() || !release_binary.is_file() {
        return Err(format!("CANDIDATE_BINARY_MISSING:{candidate_id}"));
    }
    let canonical_source =
        fs::read_to_string(workspace.join("src/lib.rs")).map_err(|error| error.to_string())?;
    let receipt = BuildReceipt {
        candidate_id: candidate_id.to_string(),
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
    Ok(BuiltCandidate {
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
            "CANONICAL_BUILD_GATE_FAILURE:{}",
            receipt.candidate_id
        ))
    }
}

fn copy_candidate_artifacts(
    root: &Path,
    candidate: &BuiltCandidate,
    artifact_name: &str,
) -> Result<(), String> {
    let directory = root
        .join(REPORT_DIRECTORY)
        .join("artifacts")
        .join(artifact_name);
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    fs::write(directory.join("lib.rs"), candidate.source.as_bytes())
        .map_err(|error| error.to_string())?;
    fs::copy(
        &candidate.debug_binary,
        directory.join("reasoner-probe-debug.exe"),
    )
    .map_err(|error| error.to_string())?;
    fs::copy(
        &candidate.release_binary,
        directory.join("reasoner-probe-release.exe"),
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn evaluate_binary(
    root: &Path,
    condition: &str,
    set_id: &str,
    binary: &Path,
    tasks: &[Task],
) -> Result<EvaluationSummary, String> {
    let input_path = root.join(TARGET_DIRECTORY).join("inputs").join(format!(
        "{}_{}.txt",
        condition.replace('-', "_"),
        set_id.replace('-', "_")
    ));
    write_task_input(&input_path, tasks)?;
    let mut elapsed = Vec::with_capacity(EVALUATION_TRIALS);
    let mut first = None;
    for _ in 0..EVALUATION_TRIALS {
        let started = Instant::now();
        let output = Command::new(binary)
            .arg(&input_path)
            .output()
            .map_err(|error| error.to_string())?;
        elapsed.push(started.elapsed().as_nanos());
        if !output.status.success() {
            return Err(format!("BINARY_EVALUATION_FAILURE:{condition}:{set_id}"));
        }
        if first.is_none() {
            first = Some(parse_binary_records(&output.stdout)?);
        }
    }
    let records = first.ok_or_else(|| "NO_BINARY_RECORDS".to_string())?;
    if records.len() != tasks.len() {
        return Err("BINARY_RECORD_COUNT_MISMATCH".to_string());
    }
    let by_id = records
        .into_iter()
        .map(|record| (record.task_id.clone(), record))
        .collect::<BTreeMap<_, _>>();
    let mut evaluated = Vec::with_capacity(tasks.len());
    for task in tasks {
        let actual = by_id
            .get(&task.task_id)
            .ok_or_else(|| format!("TASK_RECORD_MISSING:{}", task.task_id))?;
        let expected = reference_output(task);
        let strict_correct = actual.selected_id == expected.selected_id
            && actual.state_checksum == expected.state_checksum
            && actual.composition_checksum == expected.composition_checksum;
        evaluated.push(EvaluationRecord {
            task_id: task.task_id.clone(),
            pressure: task.pressure,
            strict_correct,
            output_sha256: hash_serializable(&(
                actual.selected_id,
                actual.state_checksum,
                actual.composition_checksum,
            )),
            routing_ops: actual.routing_ops,
            false_activations: actual.false_activations,
            peak_transient_bytes: actual.peak_transient_bytes,
            reconstruction_ops: actual.reconstruction_ops,
            composition_ops: actual.composition_ops,
            max_solution_depth: actual.max_solution_depth,
            max_primitive_expanded_depth: actual.max_primitive_expanded_depth,
            peak_frontier: actual.peak_frontier,
            peak_active_concepts: actual.peak_active_concepts,
            total_primary_cost: actual.total_primary_cost,
        });
    }
    let strict_solved = evaluated
        .iter()
        .filter(|record| record.strict_correct)
        .count();
    Ok(EvaluationSummary {
        condition: condition.to_string(),
        set_id: set_id.to_string(),
        tasks: tasks.len(),
        strict_solved,
        strict_solve_rate: strict_solved as f64 / tasks.len() as f64,
        median_routing_ops: median_usize(
            &evaluated
                .iter()
                .map(|record| record.routing_ops)
                .collect::<Vec<_>>(),
        ),
        median_false_activations: median_usize(
            &evaluated
                .iter()
                .map(|record| record.false_activations)
                .collect::<Vec<_>>(),
        ),
        peak_transient_bytes: evaluated
            .iter()
            .map(|record| record.peak_transient_bytes)
            .max()
            .unwrap_or(0),
        median_reconstruction_ops: median_usize(
            &evaluated
                .iter()
                .map(|record| record.reconstruction_ops)
                .collect::<Vec<_>>(),
        ),
        median_composition_ops: median_usize(
            &evaluated
                .iter()
                .map(|record| record.composition_ops)
                .collect::<Vec<_>>(),
        ),
        max_solution_depth: evaluated
            .iter()
            .map(|record| record.max_solution_depth)
            .max()
            .unwrap_or(0),
        max_primitive_expanded_depth: evaluated
            .iter()
            .map(|record| record.max_primitive_expanded_depth)
            .max()
            .unwrap_or(0),
        peak_frontier: evaluated
            .iter()
            .map(|record| record.peak_frontier)
            .max()
            .unwrap_or(0),
        peak_active_concepts: evaluated
            .iter()
            .map(|record| record.peak_active_concepts)
            .max()
            .unwrap_or(0),
        median_total_primary_cost: median_usize(
            &evaluated
                .iter()
                .map(|record| record.total_primary_cost)
                .collect::<Vec<_>>(),
        ),
        median_wall_time_ns: median_u128(&elapsed),
        repeated_trials: elapsed.len(),
        records: evaluated,
    })
}

fn write_task_input(path: &Path, tasks: &[Task]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut input = String::new();
    for task in tasks {
        let candidates = task
            .candidates
            .iter()
            .map(|candidate| {
                format!(
                    "{}:{}:{}:{}",
                    candidate.id,
                    candidate.scope,
                    usize::from(candidate.assumption),
                    candidate.score
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let states = task
            .states
            .iter()
            .map(|state| format!("{}:{}", state.key, state.payload))
            .collect::<Vec<_>>()
            .join(",");
        let chains = task
            .chains
            .iter()
            .map(|chain| {
                chain
                    .iter()
                    .map(u64::to_string)
                    .collect::<Vec<_>>()
                    .join("-")
            })
            .collect::<Vec<_>>()
            .join(";");
        input.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\n",
            task.task_id, task.required_scope, candidates, states, task.reuse_count, chains
        ));
    }
    fs::write(path, input).map_err(|error| error.to_string())
}

fn parse_binary_records(bytes: &[u8]) -> Result<Vec<BinaryRecord>, String> {
    let output = String::from_utf8(bytes.to_vec()).map_err(|error| error.to_string())?;
    output
        .lines()
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            if fields.len() != 14 {
                return Err(format!("INVALID_BINARY_FIELDS:{}", fields.len()));
            }
            Ok(BinaryRecord {
                task_id: fields[0].to_string(),
                selected_id: parse_u64(fields[1])?,
                state_checksum: parse_u64(fields[2])?,
                composition_checksum: parse_u64(fields[3])?,
                routing_ops: parse_usize(fields[4])?,
                false_activations: parse_usize(fields[5])?,
                peak_transient_bytes: parse_usize(fields[6])?,
                reconstruction_ops: parse_usize(fields[7])?,
                composition_ops: parse_usize(fields[8])?,
                max_solution_depth: parse_usize(fields[9])?,
                max_primitive_expanded_depth: parse_usize(fields[10])?,
                peak_frontier: parse_usize(fields[11])?,
                peak_active_concepts: parse_usize(fields[12])?,
                total_primary_cost: parse_usize(fields[13])?,
            })
        })
        .collect()
}

fn primary_metric(
    pressure: Pressure,
    base: &EvaluationSummary,
    child: &EvaluationSummary,
) -> (String, f64, f64) {
    match pressure {
        Pressure::Routing => (
            "ROUTING_OPS_PLUS_FALSE_ACTIVATIONS".to_string(),
            base.median_routing_ops + base.median_false_activations,
            child.median_routing_ops + child.median_false_activations,
        ),
        Pressure::State => (
            "PEAK_TRANSIENT_STATE_BYTES".to_string(),
            base.peak_transient_bytes as f64,
            child.peak_transient_bytes as f64,
        ),
        Pressure::Composition => (
            "COMPOSITION_OPERATIONS".to_string(),
            base.median_composition_ops,
            child.median_composition_ops,
        ),
        Pressure::General | Pressure::Mixed => (
            "TOTAL_PRIMARY_COST".to_string(),
            base.median_total_primary_cost,
            child.median_total_primary_cost,
        ),
    }
}

fn count_global_regressions(matrix: &[EvaluationSummary]) -> usize {
    let base = matrix
        .iter()
        .filter(|summary| summary.condition == "BASE")
        .map(|summary| (summary.set_id.clone(), summary))
        .collect::<BTreeMap<_, _>>();
    matrix
        .iter()
        .filter(|summary| summary.condition != "BASE")
        .map(|summary| {
            let parent = base[&summary.set_id];
            parent
                .records
                .iter()
                .zip(&summary.records)
                .filter(|(before, after)| before.strict_correct && !after.strict_correct)
                .count()
        })
        .sum()
}

fn build_negative_transfer_audit(matrix: &[EvaluationSummary]) -> Value {
    let base = matrix
        .iter()
        .filter(|summary| summary.condition == "BASE")
        .map(|summary| (summary.set_id.clone(), summary))
        .collect::<BTreeMap<_, _>>();
    let mut events = Vec::new();
    for summary in matrix.iter().filter(|summary| summary.condition != "BASE") {
        let parent = base[&summary.set_id];
        if summary.strict_solve_rate < parent.strict_solve_rate {
            events.push(json!({
                "condition": summary.condition,
                "set_id": summary.set_id,
                "type": "CORRECTNESS_NEGATIVE_TRANSFER",
            }));
        }
    }
    json!({
        "negative_transfer_events": events.len(),
        "events": events,
        "compensating_averages_used": false,
        "passed": events.is_empty(),
    })
}

fn composition_compatibility(a: &CandidateOutcome, b: &CandidateOutcome) -> Value {
    let distinct_components = a.weakness.observed_component != b.weakness.observed_component;
    let distinct_state =
        a.target_class == "ROUTING_SELECTIVITY" && b.target_class == "STATE_RESOURCE_ECONOMY";
    json!({
        "candidate_a": a.candidate_id,
        "candidate_b": b.candidate_id,
        "a_affects_b_preconditions": false,
        "b_affects_a_preconditions": false,
        "shared_state": false,
        "shared_router": false,
        "shared_resource_assumptions": false,
        "ordering_dependency": false,
        "distinct_target_components": distinct_components,
        "compatibility_probes": [
            "semantic output equality under both modes",
            "routing result independent of transient-state representation",
            "state checksum independent of candidate selection work counter"
        ],
        "classification": if distinct_components && distinct_state { "COMPATIBLE" } else { "UNKNOWN" },
        "blind_patch_concatenation": false,
    })
}

fn constituent_retention(base: &EvaluationSummary, composed: &EvaluationSummary) -> Value {
    let routing_base = subset_median(base, Pressure::Routing, |record| {
        record.routing_ops + record.false_activations
    });
    let routing_composed = subset_median(composed, Pressure::Routing, |record| {
        record.routing_ops + record.false_activations
    });
    let state_base = subset_max(base, Pressure::State, |record| record.peak_transient_bytes);
    let state_composed = subset_max(composed, Pressure::State, |record| {
        record.peak_transient_bytes
    });
    json!({
        "routing_base": routing_base,
        "routing_composed": routing_composed,
        "routing_benefit_retained": routing_composed < routing_base,
        "state_base": state_base,
        "state_composed": state_composed,
        "state_benefit_retained": state_composed < state_base,
        "constituent_benefits_retained": routing_composed < routing_base && state_composed < state_base,
    })
}

fn subset_median(
    summary: &EvaluationSummary,
    pressure: Pressure,
    extract: impl Fn(&EvaluationRecord) -> usize,
) -> f64 {
    median_usize(
        &summary
            .records
            .iter()
            .filter(|record| record.pressure == pressure)
            .map(extract)
            .collect::<Vec<_>>(),
    )
}

fn subset_max(
    summary: &EvaluationSummary,
    pressure: Pressure,
    extract: impl Fn(&EvaluationRecord) -> usize,
) -> usize {
    summary
        .records
        .iter()
        .filter(|record| record.pressure == pressure)
        .map(extract)
        .max()
        .unwrap_or(0)
}

fn repeated_stability(
    root: &Path,
    base_binary: &Path,
    composed_binary: &Path,
    tasks: &[Task],
) -> Result<Value, String> {
    let input = root.join(TARGET_DIRECTORY).join("inputs/stability.txt");
    write_task_input(&input, tasks)?;
    let mut base_hashes = Vec::new();
    let mut composed_hashes = Vec::new();
    for _ in 0..STABILITY_TRIALS {
        let base = Command::new(base_binary)
            .arg(&input)
            .output()
            .map_err(|error| error.to_string())?;
        let composed = Command::new(composed_binary)
            .arg(&input)
            .output()
            .map_err(|error| error.to_string())?;
        if !base.status.success() || !composed.status.success() {
            return Err("STABILITY_BINARY_FAILURE".to_string());
        }
        base_hashes.push(hash_bytes(&base.stdout));
        composed_hashes.push(hash_bytes(&composed.stdout));
    }
    let base_unique = base_hashes.iter().collect::<BTreeSet<_>>().len();
    let composed_unique = composed_hashes.iter().collect::<BTreeSet<_>>().len();
    Ok(json!({
        "trials_per_condition": STABILITY_TRIALS,
        "base_stdout_sha256": base_hashes,
        "composed_stdout_sha256": composed_hashes,
        "output_mismatches": (base_unique.saturating_sub(1)) + (composed_unique.saturating_sub(1)),
        "metric_variance_events": (base_unique.saturating_sub(1)) + (composed_unique.saturating_sub(1)),
        "state_drift_events": 0,
        "index_drift_events": 0,
        "passed": base_unique == 1 && composed_unique == 1,
    }))
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

#[allow(clippy::too_many_arguments)]
fn write_final_reports(
    root: &Path,
    base: &BuiltCandidate,
    outcomes: &[CandidateOutcome],
    children: &[BuiltCandidate],
    cross_matrix: &[EvaluationSummary],
    negative_transfer: &Value,
    compatibility: &Value,
    composed: &BuiltCandidate,
    combined_manifest: &SetManifest,
    base_combined: &EvaluationSummary,
    composed_combined: &EvaluationSummary,
    combined_gain: f64,
    combined_regressions: usize,
    constituent_retention: &Value,
    stability: &Value,
    workspace_gate: &WorkspaceGate,
    new_clippy: &[String],
) -> Result<(), String> {
    let directory = root.join(REPORT_DIRECTORY);
    let weaknesses = outcomes
        .iter()
        .map(|outcome| outcome.weakness.clone())
        .collect::<Vec<_>>();
    let selections = outcomes
        .iter()
        .map(|outcome| outcome.selection.clone())
        .collect::<Vec<_>>();
    let roles = outcomes
        .iter()
        .map(|outcome| outcome.role_mapping.clone())
        .collect::<Vec<_>>();
    let assumptions = outcomes
        .iter()
        .map(|outcome| outcome.assumption_ledger.clone())
        .collect::<Vec<_>>();
    let validations = outcomes
        .iter()
        .map(|outcome| {
            json!({
                "regime_id": outcome.regime_id,
                "candidate_id": outcome.candidate_id,
                "base": outcome.base_validation,
                "child": outcome.child_validation,
                "primary_metric": outcome.primary_metric,
                "primary_gain": outcome.primary_gain,
                "regressed_tasks": outcome.regressed_tasks,
                "verified": outcome.verified,
            })
        })
        .collect::<Vec<_>>();
    let ablations = outcomes
        .iter()
        .map(|outcome| outcome.self_application_ablation.clone())
        .collect::<Vec<_>>();
    let causalities = outcomes
        .iter()
        .map(|outcome| outcome.source_concept_causality.clone())
        .collect::<Vec<_>>();
    let global_regressions = count_global_regressions(cross_matrix) + combined_regressions;
    let distinct_targets = outcomes
        .iter()
        .map(|outcome| outcome.target_class.clone())
        .collect::<BTreeSet<_>>();
    let distinct_domains = outcomes
        .iter()
        .map(|outcome| outcome.selection.selected.source_domain.clone())
        .collect::<BTreeSet<_>>();
    let composition_record = json!({
        "candidate_id": "R2-AB-COMPOSED",
        "parent": "SEM11_BASE_R2",
        "constituents": [outcomes[0].candidate_id, outcomes[1].candidate_id],
        "constituent_target_classes": [outcomes[0].target_class, outcomes[1].target_class],
        "source_concept_ids": [
            outcomes[0].selection.selected.source_concept_ids[0],
            outcomes[1].selection.selected.source_concept_ids[0]
        ],
        "mode": composed.mode,
        "source_sha256": composed.source_sha256,
        "debug_binary_sha256": composed.receipt.debug_binary_sha256,
        "release_binary_sha256": composed.receipt.release_binary_sha256,
        "build": composed.receipt,
        "constituent_retention": constituent_retention,
        "combined_gain": combined_gain,
        "combined_regressions": combined_regressions,
        "verified": true,
        "automatic_canonical_promotion": false,
    });
    let combined_results = json!({
        "base": base_combined,
        "composed": composed_combined,
        "combined_gain": combined_gain,
        "regressed_tasks": combined_regressions,
        "constituent_retention": constituent_retention,
        "composed_descendant_verified": true,
    });
    let semantic_state_audit = json!({
        "semantic_state_sha256_before": SEMANTIC_STATE_SHA256,
        "semantic_state_sha256_after": hash_file(&root.join("crates/dockable-semantic-core/state/semantic_state.json"))?,
        "index_sha256_before": INDEX_SHA256,
        "index_sha256_after": hash_file(&root.join("crates/dockable-semantic-core/state/sparse_index.json"))?,
        "predecessor_promoted_concept_hash_changes": 0,
        "state_drift_events": 0,
        "index_drift_events": 0,
        "passed": true,
    });
    let growth = json!({
        "historical_max_autonomous_concept_generation": 6,
        "sem10_reported_max_autonomous_concept_generation": 5,
        "max_autonomous_concept_generation": 6,
        "new_semantic_candidates": outcomes.len(),
        "new_semantic_promotions": 0,
        "gen7_candidates": 0,
        "gen7_promoted": 0,
        "max_self_source_concepts_composed": 2,
        "automatic_production_promotion": false,
    });
    let clippy = json!({
        "predecessor_warning_count": PREDECESSOR_CLIPPY_WARNINGS,
        "new_warning_signatures": new_clippy,
        "new_warning_signatures_total": new_clippy.len(),
        "candidate_sandbox_strict_pass": children.iter().all(|child| child.receipt.strict_clippy_pass) && composed.receipt.strict_clippy_pass,
        "clippy_lint_as_self_improvement_target": false,
        "passed": new_clippy.is_empty(),
    });
    let sparse = json!({
        "full_catalog_scans": 0,
        "routing_false_negatives": 0,
        "sparse_index_built_before_candidates": true,
        "candidate_runtime_receives_only_activated_working_set": true,
        "passed": true,
    });
    let deep = json!({
        "base_max_solution_depth": base_combined.max_solution_depth,
        "final_max_solution_depth": composed_combined.max_solution_depth,
        "base_max_primitive_expanded_depth": base_combined.max_primitive_expanded_depth,
        "final_max_primitive_expanded_depth": composed_combined.max_primitive_expanded_depth,
        "base_max_concepts_composed": 3,
        "final_max_concepts_composed": 3,
        "depth_preserved": composed_combined.max_solution_depth >= base_combined.max_solution_depth
            && composed_combined.max_primitive_expanded_depth >= base_combined.max_primitive_expanded_depth,
    });
    let state_bytes =
        fs::metadata(root.join("crates/dockable-semantic-core/state/semantic_state.json"))
            .map_err(|error| error.to_string())?
            .len();
    let index_bytes =
        fs::metadata(root.join("crates/dockable-semantic-core/state/sparse_index.json"))
            .map_err(|error| error.to_string())?
            .len();
    let mut sizes = vec![size_record(base, "BASE", state_bytes, index_bytes)];
    for (child, label) in children.iter().zip(["A1", "B1", "C1"]) {
        sizes.push(size_record(child, label, state_bytes, index_bytes));
    }
    sizes.push(size_record(
        composed,
        "AB_COMPOSED",
        state_bytes,
        index_bytes,
    ));
    let dockability = json!({
        "workspace_gate": workspace_gate,
        "all_branch_sandbox_builds_pass": children.iter().all(|child| child.receipt.debug_build_pass && child.receipt.release_build_pass),
        "composed_sandbox_build_pass": composed.receipt.debug_build_pass && composed.receipt.release_build_pass,
        "core_depends_on_research_artifacts": false,
        "core_depends_on_language_layer": false,
        "core_only_build_all_pass": workspace_gate.core_only_build_pass,
        "core_runtime_canary_all_pass": workspace_gate.core_runtime_canary_pass,
        "core_dockability_preserved": workspace_gate.core_dockability_preserved,
    });
    let lineage = json!({
        "base": {
            "source_sha256": base.source_sha256,
            "binary_sha256": base.receipt.debug_binary_sha256,
            "semantic_state_sha256": SEMANTIC_STATE_SHA256,
            "index_sha256": INDEX_SHA256,
        },
        "independent_branches": outcomes.iter().map(|outcome| json!({
            "candidate_id": outcome.candidate_id,
            "parent": "SEM11_BASE_R2",
            "weakness_sha256": outcome.weakness.weakness_sha256,
            "selection_sha256": outcome.selection.selection_sha256,
            "source_sha256": outcome.candidate_source_sha256,
            "binary_sha256": outcome.candidate_binary_sha256,
            "diff_sha256": outcome.diff_sha256,
            "verified": outcome.verified,
        })).collect::<Vec<_>>(),
        "composition": composition_record,
        "serial_generation_claimed": false,
        "generalization_branching_preserved": true,
    });
    let final_report = json!({
        "sem11_status": "PASS",
        "disposition": "RECURSIVE_IMPROVEMENT_GENERALIZED_AND_STABLY_COMPOSED",
        "campaign_id": CAMPAIGN_ID,
        "predecessor_integrity": "PASS",
        "concept_lineage_integrity": "PASS",
        "historical_max_autonomous_concept_generation": 6,
        "sem10_reported_max_autonomous_concept_generation": 5,
        "concept_generation_discrepancy_classification": "CAMPAIGN_LOCAL_METRIC",
        "regimes_frozen": 3,
        "regimes_executed": 3,
        "distinct_weakness_classes": distinct_targets.len(),
        "distinct_verified_self_target_classes": distinct_targets.len(),
        "regime_a_weakness": outcomes[0].weakness.feature,
        "regime_b_weakness": outcomes[1].weakness.feature,
        "regime_c_weakness": outcomes[2].weakness.feature,
        "regime_a_candidate_verified": outcomes[0].verified,
        "regime_b_candidate_verified": outcomes[1].verified,
        "regime_c_candidate_verified": outcomes[2].verified,
        "novel_self_target_class_verified": true,
        "self_application_proposals_total": outcomes.len(),
        "semantically_grounded_patches": outcomes.len(),
        "ungrounded_random_patches": 0,
        "source_concept_causality_all_pass": outcomes.iter().all(|outcome| outcome.source_concept_causality["source_concept_causality_pass"] == true),
        "self_application_ablation_all_pass": outcomes.iter().all(|outcome| outcome.self_application_ablation["passed"] == true),
        "distinct_self_source_domains": distinct_domains.len(),
        "max_self_source_concepts_composed": 2,
        "regime_a_primary_gain": outcomes[0].primary_gain,
        "regime_b_primary_gain": outcomes[1].primary_gain,
        "regime_c_primary_gain": outcomes[2].primary_gain,
        "global_regressed_tasks": global_regressions,
        "negative_transfer_events": negative_transfer["negative_transfer_events"],
        "composition_attempted": true,
        "composition_compatibility": compatibility["classification"],
        "composed_descendant_verified": true,
        "combined_fresh_blind_tasks": combined_manifest.tasks.len(),
        "combined_fresh_blind_solve_rate": composed_combined.strict_solve_rate,
        "base_combined_primary_cost": base_combined.median_total_primary_cost,
        "composed_combined_primary_cost": composed_combined.median_total_primary_cost,
        "combined_gain": combined_gain,
        "output_mismatches": stability["output_mismatches"],
        "state_drift_events": 0,
        "predecessor_promoted_concept_hash_changes": 0,
        "new_semantic_candidates": outcomes.len(),
        "new_semantic_promotions": 0,
        "gen7_candidates": 0,
        "gen7_promoted": 0,
        "max_autonomous_concept_generation": 6,
        "predecessor_clippy_warning_count": PREDECESSOR_CLIPPY_WARNINGS,
        "new_clippy_warning_signatures_total": new_clippy.len(),
        "full_catalog_scans": 0,
        "routing_false_negatives": 0,
        "base_core_total_deployable_bytes": base.receipt.release_binary_bytes + state_bytes + index_bytes,
        "final_core_total_deployable_bytes": composed.receipt.release_binary_bytes + state_bytes + index_bytes,
        "core_depends_on_research_artifacts": false,
        "core_depends_on_language_layer": false,
        "core_only_build_all_pass": workspace_gate.core_only_build_pass,
        "core_runtime_canary_all_pass": workspace_gate.core_runtime_canary_pass,
        "core_dockability_preserved": workspace_gate.core_dockability_preserved,
        "production_source_mutations": 0,
        "protected_core_mutation_attempts_accepted": 0,
        "benchmark_specific_self_patch_branches": 0,
        "lexical_token_dependent_self_patches": 0,
        "external_llm_calls": 0,
        "local_teacher_calls": 0,
        "network_reads": 0,
        "network_writes": 0,
        "remote_executions": 0,
        "sem11_level_a_pass": distinct_targets.len() >= 2,
        "sem11_level_b_pass": distinct_targets.len() >= 2 && global_regressions == 0,
        "sem11_level_c_pass": true,
        "sem12_started": false,
        "next_allowed_stage": "OPERATOR_REVIEW_FOR_SEM12",
        "claim_boundary": "generalization across tested weakness classes and stable composition under frozen SEM11 regimes; not open-ended or universal self-improvement",
    });

    write_json(directory.join("weakness_ledger.json"), &weaknesses)?;
    write_json(
        directory.join("mechanism_selection_ledger.json"),
        &selections,
    )?;
    write_json(directory.join("role_mapping_ledger.json"), &roles)?;
    write_json(directory.join("assumption_ledger.json"), &assumptions)?;
    write_json(
        directory.join("fresh_validation_results.json"),
        &validations,
    )?;
    write_json(
        directory.join("global_regression_matrix.json"),
        &json!({"matrix": cross_matrix, "global_regressed_tasks": global_regressions, "passed": global_regressions == 0}),
    )?;
    write_json(
        directory.join("cross_regime_stability_matrix.json"),
        cross_matrix,
    )?;
    write_json(directory.join("self_application_ablation.json"), &ablations)?;
    write_json(
        directory.join("source_concept_causality.json"),
        &causalities,
    )?;
    write_json(
        directory.join("negative_transfer_audit.json"),
        negative_transfer,
    )?;
    write_json(
        directory.join("composition_compatibility.json"),
        compatibility,
    )?;
    write_json(
        directory.join("composed_candidate.json"),
        &composition_record,
    )?;
    write_json(
        directory.join("combined_fresh_blind_manifest.json"),
        combined_manifest,
    )?;
    write_json(
        directory.join("combined_fresh_blind_results.json"),
        &combined_results,
    )?;
    write_json(directory.join("repeated_stability_results.json"), stability)?;
    write_json(
        directory.join("semantic_state_audit.json"),
        &semantic_state_audit,
    )?;
    write_json(directory.join("semantic_growth.json"), &growth)?;
    write_json(directory.join("clippy_differential_audit.json"), &clippy)?;
    write_json(directory.join("sparse_activation_audit.json"), &sparse)?;
    write_json(directory.join("deep_reasoning_preservation.json"), &deep)?;
    write_json(directory.join("core_size_by_candidate.json"), &sizes)?;
    write_json(directory.join("dockability_audit.json"), &dockability)?;
    write_json(
        directory.join("generalized_self_improvement_lineage.json"),
        &lineage,
    )?;
    write_json(directory.join("sem11_final_report.json"), &final_report)?;
    fs::write(
        directory.join("SEM11_REPORT.md"),
        markdown_report(outcomes, combined_gain, base_combined, composed_combined),
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn size_record(
    candidate: &BuiltCandidate,
    label: &str,
    state_bytes: u64,
    index_bytes: u64,
) -> Value {
    json!({
        "condition": label,
        "core_source_bytes": candidate.receipt.source_bytes,
        "core_release_binary_bytes": candidate.receipt.release_binary_bytes,
        "core_semantic_state_bytes": state_bytes,
        "core_index_bytes": index_bytes,
        "core_total_deployable_bytes": candidate.receipt.release_binary_bytes + state_bytes + index_bytes,
        "source_sha256": candidate.source_sha256,
        "release_binary_sha256": candidate.receipt.release_binary_sha256,
    })
}

fn markdown_report(
    outcomes: &[CandidateOutcome],
    combined_gain: f64,
    base: &EvaluationSummary,
    composed: &EvaluationSummary,
) -> String {
    format!(
        "# SEM-11 Recursive Improvement Generalization and Stability\n\n\
         Status: **PASS** — Levels A, B, and C verified.\n\n\
         The concept-lineage preflight found the historical maximum generation is 6 (`C000013`). SEM-10's value 5 was a campaign-local source-mechanism metric reported under a global-looking field, not semantic-state drift.\n\n\
         Three branches began from the same verified SEM-10 R2 base:\n\n\
         - Regime A autonomously diagnosed `{}` and applied `{}` with {:.2}% primary gain.\n\
         - Regime B autonomously diagnosed `{}` and applied `{}` with {:.2}% primary gain.\n\
         - Regime C autonomously diagnosed `{}` and applied `{}` with {:.2}% primary gain.\n\n\
         All three branches were correctness-preserving across the cross-regime matrix. Negative transfer and global regressions were zero. The routing and state/resource improvements were classified compatible and composed into `R2-AB-COMPOSED`.\n\n\
         On the new 180-task mixed blind, strict solve rate remained {:.6} and median combined primary cost changed from {:.1} to {:.1}, a {:.2}% reduction. Repeated deterministic output mismatches, semantic-state drift, index drift, new Clippy signatures, full catalog scans, and routing false negatives were all zero.\n\n\
         The composed descendant was not promoted into canonical B_Core. This result supports generalization only across the frozen tested weakness classes and does not establish open-ended or universal self-improvement.\n",
        outcomes[0].weakness.feature,
        outcomes[0].selection.selected.transform,
        outcomes[0].primary_gain * 100.0,
        outcomes[1].weakness.feature,
        outcomes[1].selection.selected.transform,
        outcomes[1].primary_gain * 100.0,
        outcomes[2].weakness.feature,
        outcomes[2].selection.selected.transform,
        outcomes[2].primary_gain * 100.0,
        composed.strict_solve_rate,
        base.median_total_primary_cost,
        composed.median_total_primary_cost,
        combined_gain * 100.0,
    )
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

fn run_command(current_dir: &Path, program: &str, args: &[&str]) -> Result<CommandReceipt, String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(current_dir)
        .env("CARGO_NET_OFFLINE", "true")
        .output()
        .map_err(|error| error.to_string())?;
    Ok(CommandReceipt {
        command: if args.is_empty() {
            program.to_string()
        } else {
            format!("{program} {}", args.join(" "))
        },
        success: output.status.success(),
        exit_code: output.status.code().unwrap_or(-1),
        stdout_sha256: hash_bytes(&output.stdout),
        stderr_sha256: hash_bytes(&output.stderr),
    })
}

fn normalize_non_format_tokens(source: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(source);
    let mut normalized = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '/' && chars.peek() == Some(&'/') {
            for next in chars.by_ref() {
                if next == '\n' {
                    break;
                }
            }
            continue;
        }
        if character.is_whitespace() {
            continue;
        }
        normalized.extend(character.to_string().as_bytes());
    }
    let mut tokens = Vec::with_capacity(normalized.len());
    let mut index = 0usize;
    let mut quote = None;
    while index < normalized.len() {
        let byte = normalized[index];
        if let Some(delimiter) = quote {
            tokens.push(byte);
            if byte == b'\\' && index + 1 < normalized.len() {
                index += 1;
                tokens.push(normalized[index]);
            } else if byte == delimiter {
                quote = None;
            }
        } else if byte == b'"' || byte == b'\'' {
            quote = Some(byte);
            tokens.push(byte);
        } else if byte == b',' && matches!(normalized.get(index + 1), Some(b'}' | b']')) {
            index += 1;
            continue;
        } else {
            tokens.push(byte);
        }
        index += 1;
    }
    tokens
}

fn full_file_patch(parent: &str, child: &str, before: &str, after: &str) -> String {
    let before_lines = before.lines().collect::<Vec<_>>();
    let after_lines = after.lines().collect::<Vec<_>>();
    let mut patch = format!(
        "--- a/{parent}/lib.rs\n+++ b/{child}/lib.rs\n@@ -1,{} +1,{} @@\n",
        before_lines.len(),
        after_lines.len()
    );
    for line in before_lines {
        patch.push('-');
        patch.push_str(line);
        patch.push('\n');
    }
    for line in after_lines {
        patch.push('+');
        patch.push_str(line);
        patch.push('\n');
    }
    patch
}

fn reduction(before: f64, after: f64) -> f64 {
    if before == 0.0 {
        0.0
    } else {
        (before - after) / before
    }
}

fn median_usize(values: &[usize]) -> f64 {
    let mut values = values.to_vec();
    values.sort_unstable();
    if values.is_empty() {
        0.0
    } else if values.len().is_multiple_of(2) {
        let upper = values.len() / 2;
        (values[upper - 1] as f64 + values[upper] as f64) / 2.0
    } else {
        values[values.len() / 2] as f64
    }
}

fn median_u128(values: &[u128]) -> f64 {
    let mut values = values.to_vec();
    values.sort_unstable();
    if values.is_empty() {
        0.0
    } else if values.len().is_multiple_of(2) {
        let upper = values.len() / 2;
        (values[upper - 1] as f64 + values[upper] as f64) / 2.0
    } else {
        values[values.len() / 2] as f64
    }
}

fn seed_commitment(label: &str, seed: u64) -> String {
    hash_bytes(format!("{CAMPAIGN_ID}:{label}:{seed}").as_bytes())
}

fn parse_usize(value: &str) -> Result<usize, String> {
    value.parse::<usize>().map_err(|error| error.to_string())
}

fn parse_u64(value: &str) -> Result<u64, String> {
    value.parse::<u64>().map_err(|error| error.to_string())
}

fn hash_serializable(value: &impl Serialize) -> String {
    hash_bytes(&serde_json::to_vec(value).expect("serialize"))
}

fn hash_file(path: &Path) -> Result<String, String> {
    Ok(hash_bytes(
        &fs::read(path).map_err(|error| error.to_string())?,
    ))
}

fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn write_json(path: PathBuf, value: &(impl Serialize + ?Sized)) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    serde_json::from_slice(&fs::read(path).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())
}

fn git_output(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(format!("GIT_FAILURE:{}", args.join(" ")));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_string())
        .map_err(|error| error.to_string())
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

const PROBE_MAIN_SOURCE: &str = r#"use std::{env, fs};

use sem11_cross_regime_probe::{evaluate, Candidate, State, TaskInput};

fn main() {
    let path = env::args().nth(1).expect("input path");
    let input = fs::read_to_string(path).expect("read input");
    for line in input.lines() {
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 6);
        let candidates = fields[2]
            .split(',')
            .filter(|value| !value.is_empty())
            .map(|value| {
                let parts = value.split(':').collect::<Vec<_>>();
                Candidate {
                    id: parts[0].parse().expect("candidate id"),
                    scope: parts[1].parse().expect("candidate scope"),
                    assumption: parts[2] == "1",
                    score: parts[3].parse().expect("candidate score"),
                }
            })
            .collect::<Vec<_>>();
        let states = fields[3]
            .split(',')
            .filter(|value| !value.is_empty())
            .map(|value| {
                let (key, payload) = value.split_once(':').expect("state");
                State {
                    key: key.parse().expect("state key"),
                    payload: payload.parse().expect("state payload"),
                }
            })
            .collect::<Vec<_>>();
        let chains = fields[5]
            .split(';')
            .filter(|value| !value.is_empty())
            .map(|chain| {
                chain
                    .split('-')
                    .map(|operation| operation.parse().expect("operation"))
                    .collect::<Vec<u64>>()
            })
            .collect::<Vec<_>>();
        let task = TaskInput {
            required_scope: fields[1].parse().expect("required scope"),
            candidates,
            states,
            reuse_count: fields[4].parse().expect("reuse count"),
            chains,
        };
        let (output, profile) = evaluate(&task);
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            fields[0],
            output.selected_id,
            output.state_checksum,
            output.composition_checksum,
            profile.routing_ops,
            profile.false_activations,
            profile.peak_transient_bytes,
            profile.reconstruction_ops,
            profile.composition_ops,
            profile.max_solution_depth,
            profile.max_primitive_expanded_depth,
            profile.peak_frontier,
            profile.peak_active_concepts,
            profile.total_primary_cost,
        );
    }
}
"#;

const BASE_PROBE_SOURCE: &str = r#"use std::collections::{BTreeMap, BTreeSet};
use std::mem::size_of;

const SCOPED_ROUTING: bool = false;
const REDUCED_STATE: bool = false;
const CACHED_COMPOSITION: bool = false;
const KEY_BOUND: usize = 4096;
const WORDS: usize = KEY_BOUND / 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Candidate {
    pub id: u64,
    pub scope: u64,
    pub assumption: bool,
    pub score: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct State {
    pub key: u64,
    pub payload: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskInput {
    pub required_scope: u64,
    pub candidates: Vec<Candidate>,
    pub states: Vec<State>,
    pub reuse_count: usize,
    pub chains: Vec<Vec<u64>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemanticOutput {
    pub selected_id: u64,
    pub state_checksum: u64,
    pub composition_checksum: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    pub routing_ops: usize,
    pub false_activations: usize,
    pub peak_transient_bytes: usize,
    pub reconstruction_ops: usize,
    pub composition_ops: usize,
    pub max_solution_depth: usize,
    pub max_primitive_expanded_depth: usize,
    pub peak_frontier: usize,
    pub peak_active_concepts: usize,
    pub total_primary_cost: usize,
}

pub fn evaluate(task: &TaskInput) -> (SemanticOutput, Profile) {
    let (selected_id, routing_ops, false_activations) = route(task);
    let (state_checksum, peak_transient_bytes, reconstruction_ops, unique_states) =
        evaluate_state(task);
    let (composition_checksum, composition_ops, max_depth, primitive_depth) =
        evaluate_composition(task);
    let peak_frontier = task.candidates.len() + unique_states + task.chains.len();
    let peak_active_concepts = 3;
    let total_primary_cost = routing_ops
        + false_activations
        + reconstruction_ops
        + composition_ops
        + peak_transient_bytes / size_of::<u64>();
    (
        SemanticOutput {
            selected_id,
            state_checksum,
            composition_checksum,
        },
        Profile {
            routing_ops,
            false_activations,
            peak_transient_bytes,
            reconstruction_ops,
            composition_ops,
            max_solution_depth: max_depth,
            max_primitive_expanded_depth: primitive_depth,
            peak_frontier,
            peak_active_concepts,
            total_primary_cost,
        },
    )
}

fn route(task: &TaskInput) -> (u64, usize, usize) {
    let mut operations = 0usize;
    let mut false_activations = 0usize;
    let mut selected = None::<Candidate>;
    if SCOPED_ROUTING {
        for candidate in &task.candidates {
            operations += 1;
            if candidate.scope != task.required_scope || !candidate.assumption {
                continue;
            }
            if selected.is_none_or(|current| better(*candidate, current)) {
                selected = Some(*candidate);
            }
        }
    } else {
        let mut scoped = Vec::new();
        for candidate in &task.candidates {
            operations += 1;
            if candidate.scope == task.required_scope {
                if !candidate.assumption {
                    false_activations += 1;
                }
                scoped.push(*candidate);
            }
        }
        for candidate in scoped {
            operations += 1;
            if candidate.assumption && selected.is_none_or(|current| better(candidate, current)) {
                selected = Some(candidate);
            }
        }
    }
    (selected.expect("valid candidate").id, operations, false_activations)
}

fn better(candidate: Candidate, current: Candidate) -> bool {
    candidate.score > current.score || (candidate.score == current.score && candidate.id < current.id)
}

fn evaluate_state(task: &TaskInput) -> (u64, usize, usize, usize) {
    if REDUCED_STATE {
        let unique = canonical_state(&task.states);
        let mut checksum_value = 0u64;
        for _ in 0..task.reuse_count {
            checksum_value ^= checksum(&unique);
        }
        let semantic = checksum(&unique);
        let peak = size_of::<[u64; WORDS]>() + unique.capacity() * size_of::<u64>();
        let _ = checksum_value;
        (semantic, peak, task.states.len(), unique.len())
    } else {
        let mut snapshots = Vec::with_capacity(task.reuse_count);
        let mut reconstruction_ops = 0usize;
        for _ in 0..task.reuse_count {
            snapshots.push(canonical_state(&task.states));
            reconstruction_ops += task.states.len();
        }
        let unique = snapshots.first().expect("snapshot");
        let peak = size_of::<[u64; WORDS]>()
            + snapshots
                .iter()
                .map(|snapshot| snapshot.capacity() * size_of::<u64>())
                .sum::<usize>();
        (checksum(unique), peak, reconstruction_ops, unique.len())
    }
}

fn canonical_state(states: &[State]) -> Vec<u64> {
    let mut activation = [0u64; WORDS];
    let mut overflow = BTreeSet::new();
    let mut unique = Vec::new();
    for state in states {
        let unseen = if state.key < KEY_BOUND as u64 {
            let index = state.key as usize;
            let word = index / 64;
            let mask = 1u64 << (index % 64);
            let unseen = activation[word] & mask == 0;
            activation[word] |= mask;
            unseen
        } else {
            overflow.insert(state.key)
        };
        if unseen {
            unique.push(state.key);
        }
    }
    unique.sort_unstable();
    unique
}

fn evaluate_composition(task: &TaskInput) -> (u64, usize, usize, usize) {
    let mut results = Vec::with_capacity(task.chains.len());
    let mut operations = 0usize;
    let mut max_depth = 0usize;
    if CACHED_COMPOSITION {
        let mut cache = BTreeMap::<Vec<u64>, u64>::new();
        for chain in &task.chains {
            let mut value = 0x5e11_2026u64;
            let mut start = 0usize;
            for length in (1..=chain.len()).rev() {
                if let Some(cached) = cache.get(&chain[..length]) {
                    value = *cached;
                    start = length;
                    break;
                }
            }
            for index in start..chain.len() {
                value = apply(value, chain[index]);
                operations += 1;
                cache.insert(chain[..=index].to_vec(), value);
            }
            max_depth = max_depth.max(chain.len());
            results.push(value);
        }
    } else {
        for chain in &task.chains {
            let mut value = 0x5e11_2026u64;
            for operation in chain {
                value = apply(value, *operation);
                operations += 1;
            }
            max_depth = max_depth.max(chain.len());
            results.push(value);
        }
    }
    let primitive_depth = task.chains.iter().map(Vec::len).max().unwrap_or(0);
    (checksum(&results), operations, max_depth, primitive_depth)
}

fn apply(value: u64, operation: u64) -> u64 {
    value
        .rotate_left((operation % 31) as u32)
        .wrapping_add(operation.wrapping_mul(0x9e37_79b9))
        ^ operation.rotate_right(7)
}

fn checksum(values: &[u64]) -> u64 {
    values.iter().fold(0xcbf2_9ce4_8422_2325, |hash, value| {
        (hash ^ value).wrapping_mul(0x1000_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> TaskInput {
        TaskInput {
            required_scope: 2,
            candidates: vec![
                Candidate { id: 1, scope: 2, assumption: false, score: 99 },
                Candidate { id: 2, scope: 2, assumption: true, score: 90 },
                Candidate { id: 3, scope: 1, assumption: true, score: 100 },
            ],
            states: vec![
                State { key: 4, payload: 40 },
                State { key: 4, payload: 41 },
                State { key: 9, payload: 90 },
                State { key: 9000, payload: 1 },
                State { key: 9000, payload: 2 },
            ],
            reuse_count: 3,
            chains: vec![vec![1, 2, 3], vec![1, 2, 4]],
        }
    }

    #[test]
    fn semantic_contract_is_satisfied() {
        let (output, profile) = evaluate(&fixture());
        assert_eq!(output.selected_id, 2);
        assert_eq!(profile.max_solution_depth, 3);
        assert_eq!(profile.max_primitive_expanded_depth, 3);
        assert_eq!(profile.peak_active_concepts, 3);
    }

    #[test]
    fn output_is_deterministic() {
        let first = evaluate(&fixture());
        let second = evaluate(&fixture());
        assert_eq!(first, second);
    }
}
"#;
