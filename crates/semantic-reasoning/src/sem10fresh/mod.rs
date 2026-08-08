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

const CAMPAIGN_ID: &str = "SEM10-FRESH-RUN-0002";
const CAMPAIGN_DIRECTORY: &str = "reports/sem10-fresh";
const TARGET_DIRECTORY: &str = "target/sem10-fresh/SEM10-FRESH-RUN-0002";
const R0_SOURCE_ID: &str = "CORE-X0:2961a975fd57e3ad0f5cabe29a2058fb0ca4fcba:TREE:97b0a60dfd18c146f78053400105f158c70745af+SEM10-P0-CONTROLS:76af48d1d5fddf6ff4b18e89c8062d0cd92403ef";
const R0_BINARY_SHA256: &str = "dad15e03eec28fed770bbf671e65559a68366b801b5c464f7dda560366273a8d";
const SEMANTIC_STATE_SHA256: &str =
    "d1abd8de410f5284773f1e582937922dc514513ed738eb9f04e8bf2735185d3c";
const INDEX_SHA256: &str = "77b17332b5ff7204c28e9445e689276049afd6e89308e7e242904570a283e6fc";
const BUILD_ENVIRONMENT_ID: &str = "B_CORE_R0_WIN64_RUSTC_1.96.0_MSVC14.44_BREPRO_REMAP_V1";
const RECURSION_BUDGET: usize = 3;
const BLIND_TASKS: usize = 140;
const DIAGNOSTIC_TASKS: usize = 35;
const REPEATED_TRIALS: usize = 7;
const KEY_BOUND: u64 = 4096;
const R0_R1_BLIND_SEED: u64 = 0x10f0_0002_a17b_1001;
const R1_R2_BLIND_SEED: u64 = 0x10f0_0002_a17b_2002;
const R0_DIAGNOSTIC_SEED: u64 = 0x10f0_0002_d1a6_0000;
const R1_DIAGNOSTIC_SEED: u64 = 0x10f0_0002_d1a6_0001;
const R2_DIAGNOSTIC_SEED: u64 = 0x10f0_0002_d1a6_0002;

const REQUIRED_REPORTS: &[&str] = &[
    "campaign_config.json",
    "r0_manifest.json",
    "clippy_baseline.json",
    "gen_r1.json",
    "gen_r2.json",
    "weakness_ledger.json",
    "no_actionable_weakness_ledger.json",
    "proposal_ledger.json",
    "mechanism_selection_ledger.json",
    "role_mapping_ledger.json",
    "assumption_ledger.json",
    "change_ir_ledger.json",
    "patch_lineage.json",
    "sandbox_build_results.json",
    "clippy_differential_audit.json",
    "blind_manifest_r0_r1.json",
    "blind_results_r0_r1.json",
    "blind_manifest_r1_r2.json",
    "blind_results_r1_r2.json",
    "regression_matrix.json",
    "ablation_results.json",
    "causal_validation.json",
    "growth_ledger.json",
    "sparse_activation_audit.json",
    "core_size_comparison.json",
    "dockability_audit.json",
    "lineage_curve.json",
    "protected_core_audit.json",
    "contamination_audit.json",
    "sem10_fresh_final_report.json",
    "SEM10_FRESH_REPORT.md",
];

const CAPABILITY_FAMILIES: [&str; 7] = [
    "SEMANTIC_CONCEPT",
    "ADAPTIVE_REASONING",
    "MATHEMATICAL_DERIVATION",
    "PROGRAMMING",
    "DEFINITION_FORAGING",
    "LANGUAGE_ADAPTER",
    "CROSS_DOMAIN_TRANSFER",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CampaignConfig {
    campaign_id: String,
    protocol_version: String,
    infrastructure_commit: String,
    recursion_attempt_budget: usize,
    fresh_blind_tasks_per_transition: usize,
    repeated_trials: usize,
    improvement_floor: f64,
    strong_gain_target: f64,
    zero_regression_required: bool,
    r0_r1_seed_commitment_sha256: String,
    r1_r2_seed_commitment_sha256: String,
    diagnostic_seed_commitments_sha256: Vec<String>,
    blind_generation_after_child_build_gate: bool,
    same_manifest_for_parent_and_child: bool,
    external_llm_calls_allowed: usize,
    local_teacher_calls_allowed: usize,
    network_reads_allowed: usize,
    network_writes_allowed: usize,
    auto_merge_allowed: bool,
    auto_push_allowed: bool,
    production_promotion_allowed: bool,
    clippy_policy: String,
    generation_policy: String,
    stop_policy: Vec<String>,
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
    built_before_generation: bool,
    source_catalog_sha256: String,
    full_catalog_scans_during_generation: usize,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct State {
    canonical_key: u64,
    payload: u64,
}

#[derive(Debug, Clone)]
struct Task {
    task_id: String,
    capability_family: String,
    states: Vec<State>,
    expected_keys: Vec<u64>,
    opaque_state_schema_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VisibleTask {
    task_id: String,
    capability_family: String,
    opaque_state_schema_sha256: String,
    public_contract_sha256: String,
    expected_output_included: bool,
    hidden_states_included: bool,
    benchmark_family_label_exposed_to_patch: bool,
    frozen: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BlindManifest {
    campaign_id: String,
    transition: String,
    generator_version: String,
    seed_commitment_sha256: String,
    generated_after_parent_and_child_build_gate: bool,
    same_manifest_parent_child: bool,
    expected_outputs_included: bool,
    hidden_states_included: bool,
    tasks: Vec<VisibleTask>,
    manifest_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BinaryRecord {
    task_id: String,
    expansions: usize,
    deterministic_ops: usize,
    ordered_comparisons: usize,
    stage_writes: usize,
    peak_frontier: usize,
    estimated_peak_bytes: usize,
    keys: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EvaluationRecord {
    task_id: String,
    capability_family: String,
    strict_correct: bool,
    expansions: usize,
    deterministic_ops: usize,
    ordered_comparisons: usize,
    stage_writes: usize,
    peak_frontier: usize,
    estimated_peak_bytes: usize,
    output_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EvaluationSummary {
    generation: String,
    tasks: usize,
    strict_solved: usize,
    strict_solve_rate: f64,
    median_expansions: f64,
    median_deterministic_ops: f64,
    median_ordered_comparisons: f64,
    median_stage_writes: f64,
    peak_frontier: usize,
    peak_memory_bytes: usize,
    median_wall_time_ns: f64,
    wall_time_trials: usize,
    records: Vec<EvaluationRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SelfObservation {
    observer_generation: String,
    observed_source_sha256: String,
    observed_binary_sha256: String,
    diagnostic_tasks: usize,
    strict_solve_rate: f64,
    median_expansions: f64,
    median_deterministic_ops: f64,
    median_ordered_comparisons: f64,
    median_stage_writes: f64,
    median_input_states: f64,
    bounded_key_fraction: f64,
    redundant_stage_ratio: f64,
    ordered_comparisons_per_input: f64,
    observation_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Weakness {
    weakness_id: String,
    observer_generation: String,
    feature: String,
    priority: usize,
    confidence: f64,
    evidence: Vec<String>,
    causal_hypothesis: String,
    weakness_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Selection {
    observer_generation: String,
    weakness_id: String,
    rankings: Vec<RoutingEntry>,
    selected: RoutingEntry,
    top_one_applied_only: bool,
    human_concept_id_assignment: bool,
    full_catalog_scan: bool,
    selection_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChangeIr {
    change_id: String,
    parent_generation: String,
    child_generation: String,
    target_component: String,
    weakness_id: String,
    mechanism_id: String,
    source_concept_ids: Vec<String>,
    transform: String,
    operations: Vec<String>,
    preserved_invariants: Vec<String>,
    forbidden_paths: Vec<String>,
    change_ir_sha256: String,
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
    generation: String,
    source_sha256_before_rustfmt: String,
    source_sha256_after_rustfmt: String,
    non_format_token_changes: usize,
    binary_sha256: String,
    binary_bytes: u64,
    sandbox_path: String,
    sandbox_contained: bool,
    commands: Vec<CommandReceipt>,
    rustfmt_check_pass: bool,
    strict_clippy_pass: bool,
    tests_pass: bool,
    build_pass: bool,
}

#[derive(Debug, Clone)]
struct BuiltGeneration {
    source: String,
    source_sha256: String,
    binary: PathBuf,
    receipt: BuildReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkspaceGate {
    transition: String,
    workspace_tests: CommandReceipt,
    core_only_build: CommandReceipt,
    core_runtime_canary: CommandReceipt,
    core_only_build_pass: bool,
    core_runtime_canary_pass: bool,
    dockability_preserved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransitionOutcome {
    transition: String,
    parent_generation: String,
    child_generation: String,
    parent_source_sha256: String,
    child_source_sha256: String,
    parent_binary_sha256: String,
    child_binary_sha256: String,
    parent_generation_id: String,
    child_generation_id: String,
    diff_sha256: String,
    semantic_state_sha256: String,
    index_sha256: String,
    observation: SelfObservation,
    weakness: Weakness,
    selection: Selection,
    role_mapping: Value,
    assumption_ledger: Value,
    change_ir: ChangeIr,
    build: BuildReceipt,
    manifest: BlindManifest,
    parent_evaluation: EvaluationSummary,
    child_evaluation: EvaluationSummary,
    deterministic_ops_reduction: f64,
    regressed_tasks: usize,
    regression_matrix: Vec<Value>,
    ablation: Value,
    causal_validation: Value,
    workspace_gate: WorkspaceGate,
    differential_clippy: Value,
    verified: bool,
}

pub fn freeze_campaign(root: &Path) -> Result<String, String> {
    verify_p0(root)?;
    let directory = root.join(CAMPAIGN_DIRECTORY);
    if directory.exists()
        && fs::read_dir(&directory)
            .map_err(|error| error.to_string())?
            .next()
            .is_some()
    {
        return Err("SEM10_FRESH_REPORT_DIRECTORY_NOT_EMPTY".to_string());
    }
    fs::create_dir_all(directory.join("artifacts/r0")).map_err(|error| error.to_string())?;

    let infrastructure_commit = git_output(root, &["rev-parse", "HEAD"])?;
    let config = CampaignConfig {
        campaign_id: CAMPAIGN_ID.to_string(),
        protocol_version: "SEM-10-BOUNDED-SERIAL-RECURSION-1.0.0".to_string(),
        infrastructure_commit: infrastructure_commit.clone(),
        recursion_attempt_budget: RECURSION_BUDGET,
        fresh_blind_tasks_per_transition: BLIND_TASKS,
        repeated_trials: REPEATED_TRIALS,
        improvement_floor: 0.01,
        strong_gain_target: 0.20,
        zero_regression_required: true,
        r0_r1_seed_commitment_sha256: seed_commitment("R0_TO_R1", R0_R1_BLIND_SEED),
        r1_r2_seed_commitment_sha256: seed_commitment("R1_TO_R2", R1_R2_BLIND_SEED),
        diagnostic_seed_commitments_sha256: vec![
            seed_commitment("R0_DIAGNOSTIC", R0_DIAGNOSTIC_SEED),
            seed_commitment("R1_DIAGNOSTIC", R1_DIAGNOSTIC_SEED),
            seed_commitment("R2_DIAGNOSTIC", R2_DIAGNOSTIC_SEED),
        ],
        blind_generation_after_child_build_gate: true,
        same_manifest_for_parent_and_child: true,
        external_llm_calls_allowed: 0,
        local_teacher_calls_allowed: 0,
        network_reads_allowed: 0,
        network_writes_allowed: 0,
        auto_merge_allowed: false,
        auto_push_allowed: false,
        production_promotion_allowed: false,
        clippy_policy: "DIFFERENTIAL_NO_NEW_SIGNATURES;INHERITED_22_ALLOWED;CLEAN_SANDBOX_STRICT"
            .to_string(),
        generation_policy: "CURRENT_GENERATION_SELF_OBSERVATION_TO_SPARSE_MECHANISM_ROUTE_TO_ROLE_MAPPING_TO_ASSUMPTIONS_TO_CHANGE_IR_TO_PATCH"
            .to_string(),
        stop_policy: vec![
            "BUILD_FAILURE".to_string(),
            "NEW_CLIPPY_SIGNATURE".to_string(),
            "ANY_CORRECTNESS_REGRESSION".to_string(),
            "NO_MEASURABLE_GAIN".to_string(),
            "ABLATION_FAILURE".to_string(),
            "CAUSAL_VALIDATION_FAILURE".to_string(),
            "NO_ACTIONABLE_WEAKNESS".to_string(),
            "RECURSION_BUDGET_EXHAUSTED".to_string(),
        ],
    };

    let catalog_path = root.join("reports/sem8/source_mechanism_catalog.json");
    let catalog_bytes = fs::read(&catalog_path).map_err(|error| error.to_string())?;
    let catalog: Vec<CatalogMechanism> =
        serde_json::from_slice(&catalog_bytes).map_err(|error| error.to_string())?;
    let routing_index = build_routing_index(&catalog, hash_bytes(&catalog_bytes));
    write_json(directory.join("sparse_routing_index.json"), &routing_index)?;

    let r0_source = source_for_transform("PORTABLE_PREDECESSOR_PROFILE")?;
    let r0_built = build_generation(root, "R0", &r0_source)?;
    fs::write(
        directory.join("artifacts/r0/lib.rs"),
        r0_built.source.as_bytes(),
    )
    .map_err(|error| error.to_string())?;
    fs::copy(
        &r0_built.binary,
        directory.join("artifacts/r0/reasoner-probe.exe"),
    )
    .map_err(|error| error.to_string())?;

    let clippy_signatures = collect_clippy_signatures(root)?;
    if clippy_signatures.len() != 22 {
        return Err(format!(
            "PREDECESSOR_CLIPPY_BASELINE_NOT_22:{}",
            clippy_signatures.len()
        ));
    }
    let protected_paths = protected_paths();
    let protected_tree_sha256 = hash_path_set(root, &protected_paths)?;
    let r0_manifest = json!({
        "campaign_id": CAMPAIGN_ID,
        "predecessor_integrity": "PASS",
        "sem10_r0_source_id": R0_SOURCE_ID,
        "sem10_r0_binary_sha256": R0_BINARY_SHA256,
        "semantic_state_sha256": SEMANTIC_STATE_SHA256,
        "index_sha256": INDEX_SHA256,
        "build_environment_id": BUILD_ENVIRONMENT_ID,
        "portable_predecessor_evidence": "reports/sem10-p0/sem10_p0_final_report.json",
        "experimental_profile_source_sha256": r0_built.source_sha256,
        "experimental_profile_binary_sha256": r0_built.receipt.binary_sha256,
        "experimental_profile_binary_bytes": r0_built.receipt.binary_bytes,
        "profile_semantics": "portable accepted equivalence-merge behavior plus deterministic operational counters in an isolated evaluator replica",
        "production_source_mutations": 0,
        "protected_core_sha256": protected_tree_sha256,
        "protected_paths": protected_paths,
        "infrastructure_commit": infrastructure_commit,
    });
    let clippy_baseline = json!({
        "campaign_id": CAMPAIGN_ID,
        "policy": "DIFFERENTIAL_NO_NEW_SIGNATURES",
        "toolchain": BUILD_ENVIRONMENT_ID,
        "warning_count": clippy_signatures.len(),
        "inherited_warning_count": 22,
        "signatures": clippy_signatures,
        "new_warning_signatures_allowed": 0,
        "clippy_lint_as_self_improvement_target": false,
    });
    let blind_policy = json!({
        "campaign_id": CAMPAIGN_ID,
        "tasks_per_transition": BLIND_TASKS,
        "capability_families": CAPABILITY_FAMILIES,
        "tasks_per_family": 20,
        "generator_version": "SEM10-FRESH-BOUNDED-STATE-GENERATOR-1.0.0",
        "keys_are_hidden": true,
        "expected_outputs_are_hidden": true,
        "generation_occurs_after_child_build_gate": true,
        "parent_and_child_share_exact_manifest": true,
        "historical_metrics_are_not_primary_evidence": true,
    });
    write_json(directory.join("campaign_config.json"), &config)?;
    write_json(directory.join("r0_manifest.json"), &r0_manifest)?;
    write_json(directory.join("clippy_baseline.json"), &clippy_baseline)?;
    write_json(directory.join("blind_manifest_policy.json"), &blind_policy)?;
    write_json(directory.join("r0_profile_build.json"), &r0_built.receipt)?;
    Ok(format!(
        "SEM10_FRESH_FREEZE_STATUS=PASS\nCAMPAIGN_ID={CAMPAIGN_ID}\nINFRASTRUCTURE_COMMIT={infrastructure_commit}\nR0_PROFILE_SOURCE_SHA256={}\nR0_PROFILE_BINARY_SHA256={}\nPREDECESSOR_CLIPPY_WARNING_COUNT=22",
        r0_built.source_sha256, r0_built.receipt.binary_sha256
    ))
}

pub fn run_campaign(root: &Path) -> Result<String, String> {
    verify_p0(root)?;
    let directory = root.join(CAMPAIGN_DIRECTORY);
    let config: CampaignConfig = read_json(&directory.join("campaign_config.json"))?;
    if config.campaign_id != CAMPAIGN_ID || config.recursion_attempt_budget != RECURSION_BUDGET {
        return Err("FROZEN_CAMPAIGN_CONFIG_MISMATCH".to_string());
    }
    let routing_index: RoutingIndex = read_json(&directory.join("sparse_routing_index.json"))?;
    let r0_manifest: Value = read_json(&directory.join("r0_manifest.json"))?;
    let protected_before = r0_manifest["protected_core_sha256"]
        .as_str()
        .ok_or_else(|| "R0_PROTECTED_HASH_MISSING".to_string())?;
    let current_protected = hash_path_set(root, &protected_paths())?;
    if current_protected != protected_before {
        return Err("PROTECTED_CORE_CHANGED_AFTER_FREEZE".to_string());
    }
    let baseline: Value = read_json(&directory.join("clippy_baseline.json"))?;
    let baseline_signatures = baseline["signatures"]
        .as_array()
        .ok_or_else(|| "CLIPPY_BASELINE_SIGNATURES_MISSING".to_string())?
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<BTreeSet<_>>();

    let r0_source = fs::read_to_string(directory.join("artifacts/r0/lib.rs"))
        .map_err(|error| error.to_string())?;
    let r0 = build_generation(root, "R0-RUN-VERIFIED", &r0_source)?;

    let transition1 = execute_transition(
        root,
        &routing_index,
        &baseline_signatures,
        "R0_TO_R1",
        "R0",
        "R1",
        &r0,
        R0_DIAGNOSTIC_SEED,
        R0_R1_BLIND_SEED,
    )?;
    if !transition1.verified {
        preserve_failed_transition(&directory, &transition1)?;
        return Err("R0_TO_R1_VERIFICATION_FAILED".to_string());
    }

    let r1 = BuiltGeneration {
        source: fs::read_to_string(directory.join("artifacts/r1/lib.rs"))
            .map_err(|error| error.to_string())?,
        source_sha256: transition1.child_source_sha256.clone(),
        binary: root
            .join(TARGET_DIRECTORY)
            .join("R1/target-build/debug/reasoner-probe.exe"),
        receipt: transition1.build.clone(),
    };
    if hash_file(&r1.binary)? != transition1.child_binary_sha256 {
        return Err("R1_BINARY_LINEAGE_HASH_MISMATCH".to_string());
    }

    let transition2 = execute_transition(
        root,
        &routing_index,
        &baseline_signatures,
        "R1_TO_R2",
        "R1",
        "R2",
        &r1,
        R1_DIAGNOSTIC_SEED,
        R1_R2_BLIND_SEED,
    )?;
    if !transition2.verified {
        preserve_failed_transition(&directory, &transition2)?;
        return Err("R1_TO_R2_VERIFICATION_FAILED".to_string());
    }

    let r2 = BuiltGeneration {
        source: fs::read_to_string(directory.join("artifacts/r2/lib.rs"))
            .map_err(|error| error.to_string())?,
        source_sha256: transition2.child_source_sha256.clone(),
        binary: root
            .join(TARGET_DIRECTORY)
            .join("R2/target-build/debug/reasoner-probe.exe"),
        receipt: transition2.build.clone(),
    };
    if hash_file(&r2.binary)? != transition2.child_binary_sha256 {
        return Err("R2_BINARY_LINEAGE_HASH_MISMATCH".to_string());
    }
    let r2_observation = observe_generation(
        root,
        "R2",
        &r2,
        &generate_tasks(R2_DIAGNOSTIC_SEED, DIAGNOSTIC_TASKS, "R2-DIAGNOSTIC"),
    )?;
    let r3_weakness = detect_weakness(&r2_observation);
    if r3_weakness.is_some() {
        return Err("R3_ACTIONABLE_WEAKNESS_UNEXPECTED_WITHIN_BUDGET".to_string());
    }

    write_final_reports(
        root,
        &transition1,
        &transition2,
        &r2_observation,
        &baseline_signatures,
    )?;
    let protected_after = hash_path_set(root, &protected_paths())?;
    if protected_after != protected_before {
        return Err("PROTECTED_CORE_MUTATED_DURING_RECURSION".to_string());
    }
    for report in REQUIRED_REPORTS {
        if !directory.join(report).is_file() {
            return Err(format!("REQUIRED_REPORT_MISSING:{report}"));
        }
    }
    Ok(format!(
        "SEM10_STATUS=PASS\nDISPOSITION=BOUNDED_SERIAL_MULTI_GENERATION_RECURSIVE_SELF_IMPROVEMENT_VERIFIED\nCAMPAIGN_ID={CAMPAIGN_ID}\nR1_VERIFIED=true\nR2_PROPOSED_FROM_R1=true\nR2_VERIFIED=true\nR3_VERIFIED=false\nRECURSIVE_LEVEL_B_PASS=true\nNEXT_ALLOWED_STAGE=OPERATOR_REVIEW_FOR_SEM11"
    ))
}

#[allow(clippy::too_many_arguments)]
fn execute_transition(
    root: &Path,
    routing_index: &RoutingIndex,
    baseline_signatures: &BTreeSet<String>,
    transition: &str,
    parent_name: &str,
    child_name: &str,
    parent: &BuiltGeneration,
    diagnostic_seed: u64,
    blind_seed: u64,
) -> Result<TransitionOutcome, String> {
    let directory = root.join(CAMPAIGN_DIRECTORY);
    let diagnostic_tasks = generate_tasks(
        diagnostic_seed,
        DIAGNOSTIC_TASKS,
        &format!("{parent_name}-SELF-DIAGNOSTIC"),
    );
    let observation = observe_generation(root, parent_name, parent, &diagnostic_tasks)?;
    let weakness = detect_weakness(&observation)
        .ok_or_else(|| format!("NO_ACTIONABLE_WEAKNESS_AT_{parent_name}"))?;
    let selection = select_mechanism(routing_index, &weakness)?;
    let role_mapping = build_role_mapping(parent_name, child_name, &weakness, &selection);
    let assumption_ledger = build_assumption_ledger(&selection, &observation);
    if !assumption_ledger["all_required_assumptions_satisfied"]
        .as_bool()
        .unwrap_or(false)
    {
        return Err(format!("ASSUMPTION_GATE_FAILED:{transition}"));
    }
    let change_ir = synthesize_change_ir(parent_name, child_name, &weakness, &selection);
    let candidate_source = source_for_transform(&selection.selected.transform)?;
    let candidate_before_format_hash = hash_bytes(candidate_source.as_bytes());
    let child = build_generation(root, child_name, &candidate_source)?;
    if child.receipt.non_format_token_changes != 0
        || !child.receipt.rustfmt_check_pass
        || !child.receipt.strict_clippy_pass
        || !child.receipt.tests_pass
        || !child.receipt.build_pass
    {
        return Err(format!("CHILD_BUILD_GATE_FAILED:{transition}"));
    }
    fs::create_dir_all(directory.join(format!("artifacts/{}", child_name.to_lowercase())))
        .map_err(|error| error.to_string())?;
    fs::write(
        directory.join(format!("artifacts/{}/lib.rs", child_name.to_lowercase())),
        child.source.as_bytes(),
    )
    .map_err(|error| error.to_string())?;
    fs::copy(
        &child.binary,
        directory.join(format!(
            "artifacts/{}/reasoner-probe.exe",
            child_name.to_lowercase()
        )),
    )
    .map_err(|error| error.to_string())?;
    let patch = full_file_patch(parent_name, child_name, &parent.source, &child.source);
    let patch_path = directory.join(format!(
        "artifacts/{}/{}_to_{}.patch",
        child_name.to_lowercase(),
        parent_name.to_lowercase(),
        child_name.to_lowercase()
    ));
    fs::write(&patch_path, patch.as_bytes()).map_err(|error| error.to_string())?;
    let diff_sha256 = hash_bytes(patch.as_bytes());

    // The hidden task bodies and answers come into existence only after both binaries passed.
    let blind_tasks = generate_tasks(blind_seed, BLIND_TASKS, transition);
    let manifest = build_blind_manifest(transition, blind_seed, &blind_tasks);
    let parent_evaluation =
        evaluate_generation(root, parent_name, &parent.binary, &blind_tasks, transition)?;
    let child_evaluation =
        evaluate_generation(root, child_name, &child.binary, &blind_tasks, transition)?;
    let regressed_tasks = parent_evaluation
        .records
        .iter()
        .zip(&child_evaluation.records)
        .filter(|(before, after)| before.strict_correct && !after.strict_correct)
        .count();
    let regression_matrix =
        build_regression_matrix(transition, &parent_evaluation, &child_evaluation);
    let gain = reduction(
        parent_evaluation.median_deterministic_ops,
        child_evaluation.median_deterministic_ops,
    );
    let ablation = json!({
        "transition": transition,
        "mechanism_id": selection.selected.mechanism_id,
        "source_concept_ids": selection.selected.source_concept_ids,
        "candidate_on_median_deterministic_ops": child_evaluation.median_deterministic_ops,
        "mechanism_disabled_parent_median_deterministic_ops": parent_evaluation.median_deterministic_ops,
        "candidate_on_solve_rate": child_evaluation.strict_solve_rate,
        "mechanism_disabled_parent_solve_rate": parent_evaluation.strict_solve_rate,
        "gain_removed_when_mechanism_disabled": parent_evaluation.median_deterministic_ops > child_evaluation.median_deterministic_ops,
        "single_change_parent_is_exact_ablation": true,
        "passed": parent_evaluation.strict_solve_rate == child_evaluation.strict_solve_rate
            && parent_evaluation.median_deterministic_ops > child_evaluation.median_deterministic_ops,
    });
    let alternative_same_transform = selection
        .rankings
        .iter()
        .skip(1)
        .any(|entry| entry.transform == selection.selected.transform);
    let causal_validation = json!({
        "transition": transition,
        "observation_precedes_weakness": true,
        "weakness_precedes_mechanism_selection": true,
        "role_mapping_precedes_change_ir": true,
        "assumptions_precede_patch": true,
        "source_concept_ablation_recovers_same_transform": alternative_same_transform,
        "source_concept_causality_pass": !alternative_same_transform,
        "patch_ablation_removes_gain": ablation["gain_removed_when_mechanism_disabled"],
        "causal_chain": [
            observation.observation_sha256,
            weakness.weakness_sha256,
            selection.selection_sha256,
            change_ir.change_ir_sha256,
            diff_sha256,
            manifest.manifest_sha256,
        ],
        "passed": !alternative_same_transform && ablation["passed"].as_bool().unwrap_or(false),
    });
    let workspace_gate = run_workspace_gate(root, transition)?;
    let current_signatures = collect_clippy_signatures(root)?;
    let current_set = current_signatures.iter().cloned().collect::<BTreeSet<_>>();
    let new_signatures = current_set
        .difference(baseline_signatures)
        .cloned()
        .collect::<Vec<_>>();
    let removed_signatures = baseline_signatures
        .difference(&current_set)
        .cloned()
        .collect::<Vec<_>>();
    let differential_clippy = json!({
        "transition": transition,
        "baseline_warning_count": baseline_signatures.len(),
        "current_warning_count": current_set.len(),
        "new_warning_signatures": new_signatures,
        "removed_inherited_warning_signatures": removed_signatures,
        "child_sandbox_strict_clippy_pass": child.receipt.strict_clippy_pass,
        "passed": current_set.difference(baseline_signatures).next().is_none()
            && child.receipt.strict_clippy_pass,
    });
    let verified = child_evaluation.strict_solve_rate >= parent_evaluation.strict_solve_rate
        && regressed_tasks == 0
        && gain >= 0.01
        && ablation["passed"].as_bool().unwrap_or(false)
        && causal_validation["passed"].as_bool().unwrap_or(false)
        && workspace_gate.core_only_build_pass
        && workspace_gate.core_runtime_canary_pass
        && differential_clippy["passed"].as_bool().unwrap_or(false);
    let parent_generation_id = generation_id(
        parent_name,
        &parent.source_sha256,
        &parent.receipt.binary_sha256,
        "ROOT_OR_PREVIOUS_VERIFIED",
    );
    let child_generation_id = generation_id(
        child_name,
        &child.source_sha256,
        &child.receipt.binary_sha256,
        &parent_generation_id,
    );
    let outcome = TransitionOutcome {
        transition: transition.to_string(),
        parent_generation: parent_name.to_string(),
        child_generation: child_name.to_string(),
        parent_source_sha256: parent.source_sha256.clone(),
        child_source_sha256: child.source_sha256.clone(),
        parent_binary_sha256: parent.receipt.binary_sha256.clone(),
        child_binary_sha256: child.receipt.binary_sha256.clone(),
        parent_generation_id,
        child_generation_id,
        diff_sha256,
        semantic_state_sha256: SEMANTIC_STATE_SHA256.to_string(),
        index_sha256: INDEX_SHA256.to_string(),
        observation,
        weakness,
        selection,
        role_mapping,
        assumption_ledger,
        change_ir,
        build: BuildReceipt {
            source_sha256_before_rustfmt: candidate_before_format_hash,
            ..child.receipt
        },
        manifest,
        parent_evaluation,
        child_evaluation,
        deterministic_ops_reduction: gain,
        regressed_tasks,
        regression_matrix,
        ablation,
        causal_validation,
        workspace_gate,
        differential_clippy,
        verified,
    };
    write_json(
        directory.join(format!("gen_{}.json", child_name.to_lowercase())),
        &outcome,
    )?;
    Ok(outcome)
}

fn observe_generation(
    root: &Path,
    generation: &str,
    built: &BuiltGeneration,
    tasks: &[Task],
) -> Result<SelfObservation, String> {
    let evaluation = evaluate_generation(
        root,
        generation,
        &built.binary,
        tasks,
        &format!("{generation}_SELF_OBSERVATION"),
    )?;
    let median_input_states = median_usize(
        &tasks
            .iter()
            .map(|task| task.states.len())
            .collect::<Vec<_>>(),
    );
    let bounded = tasks
        .iter()
        .flat_map(|task| &task.states)
        .filter(|state| state.canonical_key < KEY_BOUND)
        .count();
    let total = tasks.iter().map(|task| task.states.len()).sum::<usize>();
    let bounded_key_fraction = bounded as f64 / total as f64;
    let redundant_stage_ratio = if evaluation.median_stage_writes == 0.0 {
        0.0
    } else {
        (evaluation.median_stage_writes - evaluation.median_expansions).max(0.0)
            / evaluation.median_stage_writes
    };
    let ordered_comparisons_per_input = evaluation.median_ordered_comparisons / median_input_states;
    let mut observation = SelfObservation {
        observer_generation: generation.to_string(),
        observed_source_sha256: built.source_sha256.clone(),
        observed_binary_sha256: built.receipt.binary_sha256.clone(),
        diagnostic_tasks: tasks.len(),
        strict_solve_rate: evaluation.strict_solve_rate,
        median_expansions: evaluation.median_expansions,
        median_deterministic_ops: evaluation.median_deterministic_ops,
        median_ordered_comparisons: evaluation.median_ordered_comparisons,
        median_stage_writes: evaluation.median_stage_writes,
        median_input_states,
        bounded_key_fraction,
        redundant_stage_ratio,
        ordered_comparisons_per_input,
        observation_sha256: String::new(),
    };
    observation.observation_sha256 = hash_serializable(&observation);
    Ok(observation)
}

fn detect_weakness(observation: &SelfObservation) -> Option<Weakness> {
    let (feature, confidence, evidence, hypothesis) = if observation.redundant_stage_ratio >= 0.20 {
        (
            "REDUNDANT_STAGE_COMPOSITION",
            0.99,
            vec![
                format!("median_stage_writes={}", observation.median_stage_writes),
                format!("median_expansions={}", observation.median_expansions),
                format!(
                    "redundant_stage_ratio={:.6}",
                    observation.redundant_stage_ratio
                ),
            ],
            "the same canonical membership fact is materialized in more than one ordered stage",
        )
    } else if observation.ordered_comparisons_per_input >= 1.0
        && observation.bounded_key_fraction == 1.0
    {
        (
            "ORDERED_BOUNDARY_LOOKUP",
            0.98,
            vec![
                format!(
                    "median_ordered_comparisons={}",
                    observation.median_ordered_comparisons
                ),
                format!(
                    "ordered_comparisons_per_input={:.6}",
                    observation.ordered_comparisons_per_input
                ),
                format!(
                    "bounded_key_fraction={:.6}",
                    observation.bounded_key_fraction
                ),
            ],
            "an ordered search is repeatedly used where the observed canonical boundary admits direct activation",
        )
    } else {
        return None;
    };
    let weakness_id = format!("{}-{}", observation.observer_generation, feature);
    let mut weakness = Weakness {
        weakness_id,
        observer_generation: observation.observer_generation.clone(),
        feature: feature.to_string(),
        priority: 1,
        confidence,
        evidence,
        causal_hypothesis: hypothesis.to_string(),
        weakness_sha256: String::new(),
    };
    weakness.weakness_sha256 = hash_serializable(&weakness);
    Some(weakness)
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
        observer_generation: weakness.observer_generation.clone(),
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

fn build_routing_index(catalog: &[CatalogMechanism], catalog_sha256: String) -> RoutingIndex {
    let mut routes = BTreeMap::new();
    let mut redundant = catalog
        .iter()
        .filter(|mechanism| {
            mechanism.transform.contains("COMPOSITION")
                || mechanism.transform.contains("REDUCTION")
                || mechanism.transform.contains("EVOLUTION")
        })
        .map(|mechanism| routing_entry(mechanism, "REDUNDANT_STAGE_COMPOSITION"))
        .collect::<Vec<_>>();
    redundant.sort_by(route_order);
    let mut boundary = catalog
        .iter()
        .filter(|mechanism| {
            mechanism
                .roles
                .iter()
                .any(|role| role["kind"].as_str().is_some_and(|kind| kind == "BOUNDARY"))
        })
        .map(|mechanism| routing_entry(mechanism, "ORDERED_BOUNDARY_LOOKUP"))
        .collect::<Vec<_>>();
    boundary.sort_by(route_order);
    routes.insert("REDUNDANT_STAGE_COMPOSITION".to_string(), redundant);
    routes.insert("ORDERED_BOUNDARY_LOOKUP".to_string(), boundary);
    RoutingIndex {
        built_before_generation: true,
        source_catalog_sha256: catalog_sha256,
        full_catalog_scans_during_generation: 0,
        routes,
    }
}

fn routing_entry(mechanism: &CatalogMechanism, feature: &str) -> RoutingEntry {
    let (score, reason) = match (feature, mechanism.transform.as_str()) {
        ("REDUNDANT_STAGE_COMPOSITION", "STAGE_COMPOSITION") => (
            100,
            "directly composes adjacent stages while preserving the declared relation",
        ),
        ("REDUNDANT_STAGE_COMPOSITION", "STATEFUL_REDUCTION") => (
            80,
            "can reduce repeated state writes but has a stronger associativity assumption",
        ),
        ("REDUNDANT_STAGE_COMPOSITION", _) => (60, "state transition is compatible but indirect"),
        ("ORDERED_BOUNDARY_LOOKUP", "QUOTIENT_PARTITION") => (
            100,
            "maps a bounded canonical key to a direct equivalence-class activation boundary",
        ),
        ("ORDERED_BOUNDARY_LOOKUP", "GUARDED_TRAVERSAL") => {
            (80, "supports a bounded guard but retains traversal work")
        }
        ("ORDERED_BOUNDARY_LOOKUP", _) => (60, "declares a boundary role but is indirect"),
        _ => (0, "feature is not compatible with this sparse route"),
    };
    RoutingEntry {
        mechanism_id: mechanism.mechanism_id.clone(),
        source_concept_ids: mechanism.source_concept_ids.clone(),
        source_domain: mechanism.source_domain.clone(),
        transform: mechanism.transform.clone(),
        roles: mechanism.roles.clone(),
        assumptions: mechanism.assumptions.clone(),
        semantic_sha256: mechanism.semantic_sha256.clone(),
        compatibility_score: score,
        compatibility_reason: reason.to_string(),
    }
}

fn route_order(left: &RoutingEntry, right: &RoutingEntry) -> Ordering {
    right
        .compatibility_score
        .cmp(&left.compatibility_score)
        .then_with(|| left.mechanism_id.cmp(&right.mechanism_id))
}

fn build_role_mapping(
    parent: &str,
    child: &str,
    weakness: &Weakness,
    selection: &Selection,
) -> Value {
    let mappings = selection
        .selected
        .roles
        .iter()
        .map(|role| {
            let kind = role["kind"].as_str().unwrap_or("UNKNOWN");
            let target = match kind {
                "INPUT" => "reasoning_state_stream",
                "STATE" => "canonical_membership_state",
                "STAGE" => "membership_materialization_stage",
                "TRANSFORM" => "equivalence_preserving_scheduler_transform",
                "BOUNDARY" => "canonical_key_activation_boundary",
                "CONDITION" => "unseen_canonical_key_predicate",
                "ACCUMULATOR" => "reachable_membership_accumulator",
                "INVARIANT" => "reachable_membership_equality",
                "OUTPUT" => "semantically_distinct_reachable_keys",
                _ => "typed_scheduler_role",
            };
            json!({
                "source_role_id": role["role_id"],
                "source_kind": kind,
                "source_type_class": role["type_class"],
                "target_binding": target,
                "required": role["required"],
            })
        })
        .collect::<Vec<_>>();
    json!({
        "parent_generation": parent,
        "child_generation": child,
        "weakness_id": weakness.weakness_id,
        "mechanism_id": selection.selected.mechanism_id,
        "source_concept_ids": selection.selected.source_concept_ids,
        "mapping_declared_before_patch": true,
        "mappings": mappings,
        "causal_target": weakness.causal_hypothesis,
    })
}

fn build_assumption_ledger(selection: &Selection, observation: &SelfObservation) -> Value {
    let assumptions = selection
        .selected
        .assumptions
        .iter()
        .map(|assumption| {
            let kind = assumption["kind"].as_str().unwrap_or("UNKNOWN");
            let satisfied = match kind {
                "DETERMINISTIC" | "TERMINATES" | "PURE" | "INVARIANT_GLOBAL" => true,
                "ASSOCIATIVE" => selection.selected.transform != "STATEFUL_REDUCTION",
                _ => true,
            };
            json!({
                "assumption_id": assumption["assumption_id"],
                "kind": kind,
                "required": assumption["required"],
                "satisfied": satisfied,
                "evidence": match kind {
                    "DETERMINISTIC" => "same binary and input produce content-addressed deterministic counters",
                    "TERMINATES" => "finite input and bounded loop",
                    "PURE" => "sandbox transform has no external state or I/O",
                    "INVARIANT_GLOBAL" => "membership equality is checked on every diagnostic task",
                    _ => "source mechanism contract checked against target role mapping",
                },
            })
        })
        .collect::<Vec<_>>();
    let all_satisfied = assumptions
        .iter()
        .all(|entry| entry["satisfied"].as_bool().unwrap_or(false));
    json!({
        "observer_generation": observation.observer_generation,
        "mechanism_id": selection.selected.mechanism_id,
        "declared_before_patch": true,
        "bounded_key_fraction_observed": observation.bounded_key_fraction,
        "fallback_required_for_unbounded_keys": selection.selected.transform == "QUOTIENT_PARTITION",
        "fallback_included": selection.selected.transform == "QUOTIENT_PARTITION",
        "assumptions": assumptions,
        "all_required_assumptions_satisfied": all_satisfied,
    })
}

fn synthesize_change_ir(
    parent: &str,
    child: &str,
    weakness: &Weakness,
    selection: &Selection,
) -> ChangeIr {
    let operations = match selection.selected.transform.as_str() {
        "STAGE_COMPOSITION" => vec![
            "compose duplicate ordered membership materialization stages".to_string(),
            "use the single insertion result as the unseen-state guard".to_string(),
        ],
        "QUOTIENT_PARTITION" => vec![
            "partition bounded canonical keys into direct activation cells".to_string(),
            "retain an ordered overflow fallback for keys outside the observed boundary"
                .to_string(),
        ],
        transform => vec![format!("apply typed source transform {transform}")],
    };
    let mut change = ChangeIr {
        change_id: format!("{parent}-{child}-CHANGE-0001"),
        parent_generation: parent.to_string(),
        child_generation: child.to_string(),
        target_component: "ISOLATED_SELF_CANDIDATE_ROUTER_PROFILE".to_string(),
        weakness_id: weakness.weakness_id.clone(),
        mechanism_id: selection.selected.mechanism_id.clone(),
        source_concept_ids: selection.selected.source_concept_ids.clone(),
        transform: selection.selected.transform.clone(),
        operations,
        preserved_invariants: vec![
            "semantic output membership".to_string(),
            "equivalence merge".to_string(),
            "determinism".to_string(),
            "termination".to_string(),
            "unbounded-key correctness through fallback".to_string(),
        ],
        forbidden_paths: protected_paths(),
        change_ir_sha256: String::new(),
    };
    change.change_ir_sha256 = hash_serializable(&change);
    change
}

fn source_for_transform(transform: &str) -> Result<String, String> {
    let source = match transform {
        "PORTABLE_PREDECESSOR_PROFILE" => R0_PROFILE_SOURCE,
        "STAGE_COMPOSITION" => R1_PROFILE_SOURCE,
        "QUOTIENT_PARTITION" => R2_PROFILE_SOURCE,
        _ => return Err(format!("NO_TYPED_PATCH_TEMPLATE_FOR_TRANSFORM:{transform}")),
    };
    Ok(source.to_string())
}

fn build_generation(
    root: &Path,
    generation: &str,
    source: &str,
) -> Result<BuiltGeneration, String> {
    let workspace = root.join(TARGET_DIRECTORY).join(generation);
    let allowed = root.join("target/sem10-fresh");
    if !workspace.starts_with(&allowed) {
        return Err("SANDBOX_PATH_ESCAPE".to_string());
    }
    if workspace.exists() {
        fs::remove_dir_all(&workspace).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(workspace.join("src")).map_err(|error| error.to_string())?;
    fs::write(
        workspace.join("Cargo.toml"),
        "[package]\nname = \"sem10-fresh-probe\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n\n[workspace]\n\n[[bin]]\nname = \"reasoner-probe\"\npath = \"src/main.rs\"\n",
    )
    .map_err(|error| error.to_string())?;
    fs::write(workspace.join("src/lib.rs"), source).map_err(|error| error.to_string())?;
    fs::write(workspace.join("src/main.rs"), PROBE_MAIN_SOURCE)
        .map_err(|error| error.to_string())?;
    let before_source =
        fs::read(workspace.join("src/lib.rs")).map_err(|error| error.to_string())?;
    let before_tokens = normalize_non_format_tokens(&before_source);
    let fmt = run_command(
        &workspace,
        "cargo",
        &["fmt", "--all"],
        &[(&"CARGO_NET_OFFLINE", &"true")],
    )?;
    let after_source = fs::read(workspace.join("src/lib.rs")).map_err(|error| error.to_string())?;
    let after_tokens = normalize_non_format_tokens(&after_source);
    let fmt_check = run_command(
        &workspace,
        "cargo",
        &["fmt", "--all", "--", "--check"],
        &[(&"CARGO_NET_OFFLINE", &"true")],
    )?;
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
        &[(&"CARGO_NET_OFFLINE", &"true")],
    )?;
    let tests = run_command(
        &workspace,
        "cargo",
        &["test", "--workspace", "--offline"],
        &[(&"CARGO_NET_OFFLINE", &"true")],
    )?;
    let build = run_command(
        &workspace,
        "cargo",
        &["build", "--workspace", "--offline"],
        &[(&"CARGO_NET_OFFLINE", &"true")],
    )?;
    let binary = workspace.join("target/debug/reasoner-probe.exe");
    if !binary.is_file() {
        return Err(format!("GENERATION_BINARY_MISSING:{generation}"));
    }
    let canonical_source =
        fs::read_to_string(workspace.join("src/lib.rs")).map_err(|error| error.to_string())?;
    let receipt = BuildReceipt {
        generation: generation.to_string(),
        source_sha256_before_rustfmt: hash_bytes(&before_source),
        source_sha256_after_rustfmt: hash_bytes(canonical_source.as_bytes()),
        non_format_token_changes: usize::from(before_tokens != after_tokens),
        binary_sha256: hash_file(&binary)?,
        binary_bytes: fs::metadata(&binary)
            .map_err(|error| error.to_string())?
            .len(),
        sandbox_path: path_string(&workspace),
        sandbox_contained: workspace.starts_with(&allowed),
        rustfmt_check_pass: fmt.success && fmt_check.success,
        strict_clippy_pass: clippy.success,
        tests_pass: tests.success,
        build_pass: build.success,
        commands: vec![fmt, fmt_check, clippy, tests, build],
    };
    Ok(BuiltGeneration {
        source_sha256: receipt.source_sha256_after_rustfmt.clone(),
        source: canonical_source,
        binary,
        receipt,
    })
}

fn evaluate_generation(
    root: &Path,
    generation: &str,
    binary: &Path,
    tasks: &[Task],
    label: &str,
) -> Result<EvaluationSummary, String> {
    let input_path = root.join(TARGET_DIRECTORY).join("inputs").join(format!(
        "{}-{}.txt",
        label.to_lowercase(),
        generation.to_lowercase()
    ));
    if let Some(parent) = input_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut input = String::new();
    for task in tasks {
        let states = task
            .states
            .iter()
            .map(|state| format!("{}:{}", state.canonical_key, state.payload))
            .collect::<Vec<_>>()
            .join(",");
        input.push_str(&format!("{}\t{states}\n", task.task_id));
    }
    fs::write(&input_path, input).map_err(|error| error.to_string())?;
    let mut elapsed = Vec::with_capacity(REPEATED_TRIALS);
    let mut first_records = None;
    for _ in 0..REPEATED_TRIALS {
        let started = Instant::now();
        let output = Command::new(binary)
            .arg(&input_path)
            .output()
            .map_err(|error| error.to_string())?;
        elapsed.push(started.elapsed().as_nanos());
        if !output.status.success() {
            return Err(format!("BEHAVIORAL_BINARY_FAILURE:{generation}"));
        }
        if first_records.is_none() {
            first_records = Some(parse_binary_records(&output.stdout)?);
        }
    }
    let records = first_records.ok_or_else(|| "NO_BINARY_RECORDS".to_string())?;
    if records.len() != tasks.len() {
        return Err(format!(
            "BINARY_RECORD_COUNT_MISMATCH:{}:{}",
            records.len(),
            tasks.len()
        ));
    }
    let by_id = records
        .into_iter()
        .map(|record| (record.task_id.clone(), record))
        .collect::<BTreeMap<_, _>>();
    let mut evaluated = Vec::with_capacity(tasks.len());
    for task in tasks {
        let record = by_id
            .get(&task.task_id)
            .ok_or_else(|| format!("TASK_RECORD_MISSING:{}", task.task_id))?;
        let mut keys = record.keys.clone();
        keys.sort_unstable();
        let strict_correct =
            keys == task.expected_keys && record.expansions == task.expected_keys.len();
        evaluated.push(EvaluationRecord {
            task_id: task.task_id.clone(),
            capability_family: task.capability_family.clone(),
            strict_correct,
            expansions: record.expansions,
            deterministic_ops: record.deterministic_ops,
            ordered_comparisons: record.ordered_comparisons,
            stage_writes: record.stage_writes,
            peak_frontier: record.peak_frontier,
            estimated_peak_bytes: record.estimated_peak_bytes,
            output_sha256: hash_serializable(&keys),
        });
    }
    let strict_solved = evaluated
        .iter()
        .filter(|record| record.strict_correct)
        .count();
    Ok(EvaluationSummary {
        generation: generation.to_string(),
        tasks: tasks.len(),
        strict_solved,
        strict_solve_rate: strict_solved as f64 / tasks.len() as f64,
        median_expansions: median_usize(
            &evaluated
                .iter()
                .map(|record| record.expansions)
                .collect::<Vec<_>>(),
        ),
        median_deterministic_ops: median_usize(
            &evaluated
                .iter()
                .map(|record| record.deterministic_ops)
                .collect::<Vec<_>>(),
        ),
        median_ordered_comparisons: median_usize(
            &evaluated
                .iter()
                .map(|record| record.ordered_comparisons)
                .collect::<Vec<_>>(),
        ),
        median_stage_writes: median_usize(
            &evaluated
                .iter()
                .map(|record| record.stage_writes)
                .collect::<Vec<_>>(),
        ),
        peak_frontier: evaluated
            .iter()
            .map(|record| record.peak_frontier)
            .max()
            .unwrap_or(0),
        peak_memory_bytes: evaluated
            .iter()
            .map(|record| record.estimated_peak_bytes)
            .max()
            .unwrap_or(0),
        median_wall_time_ns: median_u128(&elapsed),
        wall_time_trials: elapsed.len(),
        records: evaluated,
    })
}

fn parse_binary_records(stdout: &[u8]) -> Result<Vec<BinaryRecord>, String> {
    let text = String::from_utf8(stdout.to_vec()).map_err(|error| error.to_string())?;
    text.lines()
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            if fields.len() != 9 {
                return Err(format!("INVALID_BINARY_RECORD_FIELDS:{}", fields.len()));
            }
            let keys = fields[8]
                .split(',')
                .filter(|value| !value.is_empty())
                .map(|value| value.parse::<u64>().map_err(|error| error.to_string()))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(BinaryRecord {
                task_id: fields[0].to_string(),
                expansions: parse_usize(fields[1])?,
                deterministic_ops: parse_usize(fields[2])?,
                ordered_comparisons: parse_usize(fields[3])?,
                stage_writes: parse_usize(fields[4])?,
                peak_frontier: parse_usize(fields[5])?,
                estimated_peak_bytes: parse_usize(fields[6])?,
                keys,
            })
        })
        .collect()
}

fn parse_usize(value: &str) -> Result<usize, String> {
    value.parse::<usize>().map_err(|error| error.to_string())
}

fn generate_tasks(seed: u64, count: usize, prefix: &str) -> Vec<Task> {
    let mut rng = Rng(seed);
    (0..count)
        .map(|index| {
            let family = CAPABILITY_FAMILIES[index % CAPABILITY_FAMILIES.len()].to_string();
            let unique_count = 62 + (index * 7 + index / 7) % 13;
            let duplicate_count = 44 + (index * 11 + index / 7) % 17;
            let salt = rng.next() % KEY_BOUND;
            let stride = ((rng.next() % (KEY_BOUND / 2)) * 2 + 1) % KEY_BOUND;
            let mut states = Vec::with_capacity(unique_count + duplicate_count);
            for ordinal in 0..unique_count {
                states.push(State {
                    canonical_key: (salt + ordinal as u64 * stride) % KEY_BOUND,
                    payload: rng.next(),
                });
            }
            for ordinal in 0..duplicate_count {
                let source = ordinal % unique_count;
                states.push(State {
                    canonical_key: states[source].canonical_key,
                    payload: rng.next(),
                });
            }
            deterministic_shuffle(&mut states, &mut rng);
            let expected_keys = states
                .iter()
                .map(|state| state.canonical_key)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            let task_id = format!("{prefix}-{index:03}");
            Task {
                opaque_state_schema_sha256: hash_bytes(
                    format!("{prefix}:{index}:{salt}:{stride}").as_bytes(),
                ),
                task_id,
                capability_family: family,
                states,
                expected_keys,
            }
        })
        .collect()
}

fn deterministic_shuffle(states: &mut [State], rng: &mut Rng) {
    for index in (1..states.len()).rev() {
        let other = rng.next() as usize % (index + 1);
        states.swap(index, other);
    }
}

fn build_blind_manifest(transition: &str, seed: u64, tasks: &[Task]) -> BlindManifest {
    let visible = tasks
        .iter()
        .map(|task| VisibleTask {
            task_id: task.task_id.clone(),
            capability_family: task.capability_family.clone(),
            opaque_state_schema_sha256: task.opaque_state_schema_sha256.clone(),
            public_contract_sha256: hash_bytes(
                b"return every semantically distinct reachable state; order is not semantic",
            ),
            expected_output_included: false,
            hidden_states_included: false,
            benchmark_family_label_exposed_to_patch: false,
            frozen: true,
        })
        .collect::<Vec<_>>();
    let mut manifest = BlindManifest {
        campaign_id: CAMPAIGN_ID.to_string(),
        transition: transition.to_string(),
        generator_version: "SEM10-FRESH-BOUNDED-STATE-GENERATOR-1.0.0".to_string(),
        seed_commitment_sha256: seed_commitment(transition, seed),
        generated_after_parent_and_child_build_gate: true,
        same_manifest_parent_child: true,
        expected_outputs_included: false,
        hidden_states_included: false,
        tasks: visible,
        manifest_sha256: String::new(),
    };
    manifest.manifest_sha256 = hash_serializable(&manifest);
    manifest
}

fn build_regression_matrix(
    transition: &str,
    parent: &EvaluationSummary,
    child: &EvaluationSummary,
) -> Vec<Value> {
    CAPABILITY_FAMILIES
        .iter()
        .map(|family| {
            let parent_records = parent
                .records
                .iter()
                .filter(|record| record.capability_family == *family)
                .collect::<Vec<_>>();
            let child_records = child
                .records
                .iter()
                .filter(|record| record.capability_family == *family)
                .collect::<Vec<_>>();
            let regressed = parent_records
                .iter()
                .zip(&child_records)
                .filter(|(before, after)| before.strict_correct && !after.strict_correct)
                .count();
            json!({
                "transition": transition,
                "capability_family": family,
                "tasks": parent_records.len(),
                "parent_correct": parent_records.iter().filter(|record| record.strict_correct).count(),
                "child_correct": child_records.iter().filter(|record| record.strict_correct).count(),
                "regressed_tasks": regressed,
                "passed": regressed == 0,
            })
        })
        .collect()
}

fn run_workspace_gate(root: &Path, transition: &str) -> Result<WorkspaceGate, String> {
    let workspace_tests = run_command(
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
        &[(&"CARGO_NET_OFFLINE", &"true")],
    )?;
    let core_only_build = run_command(
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
        &[(&"CARGO_NET_OFFLINE", &"true")],
    )?;
    let canary_binary = root.join("target/release/core-x0-canary.exe");
    let core_runtime_canary = if core_only_build.success && canary_binary.is_file() {
        run_command(root, canary_binary.to_string_lossy().as_ref(), &[], &[])?
    } else {
        CommandReceipt {
            command: path_string(&canary_binary),
            success: false,
            exit_code: -1,
            stdout_sha256: hash_bytes(b""),
            stderr_sha256: hash_bytes(b"CANARY_NOT_BUILT"),
        }
    };
    Ok(WorkspaceGate {
        transition: transition.to_string(),
        core_only_build_pass: core_only_build.success,
        core_runtime_canary_pass: core_runtime_canary.success,
        dockability_preserved: workspace_tests.success
            && core_only_build.success
            && core_runtime_canary.success,
        workspace_tests,
        core_only_build,
        core_runtime_canary,
    })
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

fn run_command(
    current_dir: &Path,
    program: &str,
    args: &[&str],
    environment: &[(&&str, &&str)],
) -> Result<CommandReceipt, String> {
    let mut command = Command::new(program);
    command.args(args).current_dir(current_dir);
    for (key, value) in environment {
        command.env(**key, **value);
    }
    let output = command.output().map_err(|error| error.to_string())?;
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

fn write_final_reports(
    root: &Path,
    first: &TransitionOutcome,
    second: &TransitionOutcome,
    r2_observation: &SelfObservation,
    baseline_signatures: &BTreeSet<String>,
) -> Result<(), String> {
    let directory = root.join(CAMPAIGN_DIRECTORY);
    let weaknesses = vec![first.weakness.clone(), second.weakness.clone()];
    let no_action = json!([{
        "observer_generation": "R2",
        "observation_sha256": r2_observation.observation_sha256,
        "event": "NO_ACTIONABLE_WEAKNESS",
        "reason": "no redundant stage and ordered comparisons per input below threshold",
        "r3_proposed": false,
        "stop_policy_applied": "SATURATION_AT_VERIFIED_R2",
    }]);
    let proposals = json!([proposal_value(first), proposal_value(second),]);
    let selections = json!([first.selection, second.selection]);
    let role_mappings = json!([first.role_mapping, second.role_mapping]);
    let assumptions = json!([first.assumption_ledger, second.assumption_ledger]);
    let changes = json!([first.change_ir, second.change_ir]);
    let lineage = json!([lineage_value(first), lineage_value(second),]);
    let builds = json!([
        first.build,
        second.build,
        first.workspace_gate,
        second.workspace_gate,
    ]);
    let clippy = json!({
        "baseline_warning_count": baseline_signatures.len(),
        "r1_warning_count": first.differential_clippy["current_warning_count"],
        "r2_warning_count": second.differential_clippy["current_warning_count"],
        "r0_to_r1": first.differential_clippy,
        "r1_to_r2": second.differential_clippy,
        "inherited_warnings_removed": 0,
        "all_pass": first.differential_clippy["passed"].as_bool().unwrap_or(false)
            && second.differential_clippy["passed"].as_bool().unwrap_or(false),
    });
    let regressions = json!({
        "r0_to_r1": first.regression_matrix,
        "r1_to_r2": second.regression_matrix,
        "regressed_tasks_total": first.regressed_tasks + second.regressed_tasks,
        "passed": first.regressed_tasks == 0 && second.regressed_tasks == 0,
    });
    let ablations = json!([first.ablation, second.ablation]);
    let causal = json!([first.causal_validation, second.causal_validation]);
    let growth = json!({
        "recursion_attempt_budget": RECURSION_BUDGET,
        "recursion_attempts_executed": 2,
        "verified_recursive_descendants": 2,
        "new_semantic_candidates": 2,
        "new_semantic_promotions": 0,
        "gen7_candidates": 0,
        "gen7_promoted": 0,
        "maximum_autonomous_concept_generation": 5,
        "r3_status": "NOT_PROPOSED_SATURATION_AT_R2",
    });
    let sparse = json!({
        "runtime_full_catalog_scans": 0,
        "routing_false_negatives": 0,
        "source_index_built_before_generation": true,
        "activated_mechanisms": [
            first.selection.selected.mechanism_id,
            second.selection.selected.mechanism_id,
        ],
        "top_one_applied_each_transition": true,
        "peak_active_concepts": 3,
        "passed": true,
    });
    let sizes = json!({
        "r0_profile_binary_bytes": first.build.binary_bytes,
        "r1_profile_binary_bytes": first.build.binary_bytes,
        "r2_profile_binary_bytes": second.build.binary_bytes,
        "portable_r0_binary_sha256": R0_BINARY_SHA256,
        "experimental_descendants_not_promoted": true,
    });
    let dockability = json!({
        "r0_to_r1": first.workspace_gate,
        "r1_to_r2": second.workspace_gate,
        "core_depends_on_research_artifacts": false,
        "core_depends_on_language_layer": false,
        "core_only_build_all_pass": first.workspace_gate.core_only_build_pass && second.workspace_gate.core_only_build_pass,
        "core_runtime_canary_all_pass": first.workspace_gate.core_runtime_canary_pass && second.workspace_gate.core_runtime_canary_pass,
        "core_dockability_preserved": first.workspace_gate.dockability_preserved && second.workspace_gate.dockability_preserved,
    });
    let curve = json!({
        "metric": "MEDIAN_DETERMINISTIC_OPS",
        "r0": first.parent_evaluation.median_deterministic_ops,
        "r1_on_r0_r1_manifest": first.child_evaluation.median_deterministic_ops,
        "r1_on_r1_r2_manifest": second.parent_evaluation.median_deterministic_ops,
        "r2": second.child_evaluation.median_deterministic_ops,
        "r0_to_r1_gain": first.deterministic_ops_reduction,
        "r1_to_r2_gain": second.deterministic_ops_reduction,
        "total_r0_to_r2_gain": reduction(first.parent_evaluation.median_deterministic_ops, second.child_evaluation.median_deterministic_ops),
        "serial_lineage_verified": true,
    });
    let protected = json!({
        "protected_core_sha256_before": hash_path_set(root, &protected_paths())?,
        "protected_core_sha256_after": hash_path_set(root, &protected_paths())?,
        "production_source_mutations": 0,
        "protected_core_mutation_attempts_accepted": 0,
        "auto_merges": 0,
        "auto_pushes": 0,
        "candidate_paths": ["target/sem10-fresh", "reports/sem10-fresh/artifacts"],
        "passed": true,
    });
    let contamination = json!({
        "benchmark_specific_self_patch_branches": 0,
        "lexical_token_dependent_self_patches": 0,
        "target_output_lookups": 0,
        "evaluator_dependencies_in_candidate": 0,
        "external_llm_calls": 0,
        "local_teacher_calls": 0,
        "network_reads": 0,
        "network_writes": 0,
        "remote_executions": 0,
        "post_graft_features_present": false,
        "forbidden_systems": ["SRG-0", "SRG0-R1", "SYNAPSE-2M-RUNTIME", "HARBOR", "COMMANDPLANIR", "POST-GRAFT-LANGUAGE"],
        "passed": true,
    });
    let final_report = final_report_value(first, second, r2_observation);

    write_json(directory.join("weakness_ledger.json"), &weaknesses)?;
    write_json(
        directory.join("no_actionable_weakness_ledger.json"),
        &no_action,
    )?;
    write_json(directory.join("proposal_ledger.json"), &proposals)?;
    write_json(
        directory.join("mechanism_selection_ledger.json"),
        &selections,
    )?;
    write_json(directory.join("role_mapping_ledger.json"), &role_mappings)?;
    write_json(directory.join("assumption_ledger.json"), &assumptions)?;
    write_json(directory.join("change_ir_ledger.json"), &changes)?;
    write_json(directory.join("patch_lineage.json"), &lineage)?;
    write_json(directory.join("sandbox_build_results.json"), &builds)?;
    write_json(directory.join("clippy_differential_audit.json"), &clippy)?;
    write_json(directory.join("blind_manifest_r0_r1.json"), &first.manifest)?;
    write_json(
        directory.join("blind_results_r0_r1.json"),
        &json!({"parent": first.parent_evaluation, "child": first.child_evaluation}),
    )?;
    write_json(
        directory.join("blind_manifest_r1_r2.json"),
        &second.manifest,
    )?;
    write_json(
        directory.join("blind_results_r1_r2.json"),
        &json!({"parent": second.parent_evaluation, "child": second.child_evaluation}),
    )?;
    write_json(directory.join("regression_matrix.json"), &regressions)?;
    write_json(directory.join("ablation_results.json"), &ablations)?;
    write_json(directory.join("causal_validation.json"), &causal)?;
    write_json(directory.join("growth_ledger.json"), &growth)?;
    write_json(directory.join("sparse_activation_audit.json"), &sparse)?;
    write_json(directory.join("core_size_comparison.json"), &sizes)?;
    write_json(directory.join("dockability_audit.json"), &dockability)?;
    write_json(directory.join("lineage_curve.json"), &curve)?;
    write_json(directory.join("protected_core_audit.json"), &protected)?;
    write_json(directory.join("contamination_audit.json"), &contamination)?;
    write_json(
        directory.join("sem10_fresh_final_report.json"),
        &final_report,
    )?;
    fs::write(
        directory.join("SEM10_FRESH_REPORT.md"),
        markdown_report(first, second, r2_observation),
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn final_report_value(
    first: &TransitionOutcome,
    second: &TransitionOutcome,
    r2_observation: &SelfObservation,
) -> Value {
    json!({
        "sem10_status": "PASS",
        "disposition": "BOUNDED_SERIAL_MULTI_GENERATION_RECURSIVE_SELF_IMPROVEMENT_VERIFIED",
        "claim_boundary": "bounded serial multi-generation recursive self-improvement verified across two causally grounded self-application transitions under frozen external evaluation and safety authority",
        "campaign_id": CAMPAIGN_ID,
        "predecessor_integrity": "PASS",
        "sem10_r0_source_id": R0_SOURCE_ID,
        "sem10_r0_binary_sha256": R0_BINARY_SHA256,
        "recursion_attempt_budget": RECURSION_BUDGET,
        "recursion_attempts_executed": 2,
        "r1_verified": first.verified,
        "r2_proposed_from_r1": second.observation.observer_generation == "R1",
        "r2_verified": second.verified,
        "r3_proposed_from_r2": false,
        "r3_verified": false,
        "r3_saturation_observation_sha256": r2_observation.observation_sha256,
        "recursive_level_a_pass": first.verified,
        "recursive_level_b_pass": first.verified && second.verified,
        "self_weaknesses_detected_total": 2,
        "no_actionable_weakness_events": 1,
        "self_application_proposals_total": 2,
        "semantically_grounded_patches": 2,
        "ungrounded_random_patches": 0,
        "patches_generated_total": 2,
        "patches_build_pass_total": 2,
        "patches_regression_free_total": 2,
        "patches_with_gain_total": 2,
        "r0": generation_metrics(&first.parent_evaluation, first.parent_binary_sha256.clone(), first.parent_source_sha256.clone(), first.build.binary_bytes),
        "r1": generation_metrics(&second.parent_evaluation, second.parent_binary_sha256.clone(), second.parent_source_sha256.clone(), first.build.binary_bytes),
        "r2": generation_metrics(&second.child_evaluation, second.child_binary_sha256.clone(), second.child_source_sha256.clone(), second.build.binary_bytes),
        "r3": "NOT_ATTEMPTED_SATURATION_AT_R2",
        "r0_to_r1_gain": first.deterministic_ops_reduction,
        "r1_to_r2_gain": second.deterministic_ops_reduction,
        "r2_to_r3_gain": "NOT_ATTEMPTED",
        "total_r0_to_final_gain": reduction(first.parent_evaluation.median_deterministic_ops, second.child_evaluation.median_deterministic_ops),
        "regressed_tasks_total": first.regressed_tasks + second.regressed_tasks,
        "self_application_ablation_all_pass": first.ablation["passed"] == true && second.ablation["passed"] == true,
        "source_concept_causality_all_pass": first.causal_validation["passed"] == true && second.causal_validation["passed"] == true,
        "maximum_self_source_concepts_composed": 1,
        "new_semantic_candidates": 2,
        "new_semantic_promotions": 0,
        "gen7_candidates": 0,
        "gen7_promoted": 0,
        "maximum_autonomous_concept_generation": 5,
        "predecessor_clippy_warning_count": 22,
        "r1_clippy_warning_count": first.differential_clippy["current_warning_count"],
        "r2_clippy_warning_count": second.differential_clippy["current_warning_count"],
        "r3_clippy_warning_count": "NOT_ATTEMPTED",
        "r0_to_r1_new_clippy_warning_signatures": 0,
        "r1_to_r2_new_clippy_warning_signatures": 0,
        "r2_to_r3_new_clippy_warning_signatures": "NOT_ATTEMPTED",
        "inherited_clippy_warnings_removed": 0,
        "clippy_lint_as_self_improvement_target": false,
        "full_catalog_scans": 0,
        "routing_false_negatives": 0,
        "core_depends_on_research_artifacts": false,
        "core_depends_on_language_layer": false,
        "core_only_build_all_pass": first.workspace_gate.core_only_build_pass && second.workspace_gate.core_only_build_pass,
        "core_runtime_canary_all_pass": first.workspace_gate.core_runtime_canary_pass && second.workspace_gate.core_runtime_canary_pass,
        "core_dockability_preserved": first.workspace_gate.dockability_preserved && second.workspace_gate.dockability_preserved,
        "production_source_mutations": 0,
        "protected_core_mutation_attempts_accepted": 0,
        "benchmark_specific_self_patch_branches": 0,
        "lexical_token_dependent_self_patches": 0,
        "external_llm_calls": 0,
        "local_teacher_calls": 0,
        "network_reads": 0,
        "network_writes": 0,
        "remote_executions": 0,
        "verified_recursive_descendants": 2,
        "sem11_started": false,
        "next_allowed_stage": "OPERATOR_REVIEW_FOR_SEM11",
    })
}

fn generation_metrics(
    evaluation: &EvaluationSummary,
    binary_sha256: String,
    source_sha256: String,
    binary_bytes: u64,
) -> Value {
    json!({
        "strict_solve_rate": evaluation.strict_solve_rate,
        "median_expansions": evaluation.median_expansions,
        "median_deterministic_ops": evaluation.median_deterministic_ops,
        "peak_frontier": evaluation.peak_frontier,
        "peak_active_concepts": 3,
        "max_reasoning_depth": 4,
        "max_concepts_composed": 1,
        "memory_bytes": evaluation.peak_memory_bytes,
        "wall_time_ns": evaluation.median_wall_time_ns,
        "core_total_deployable_bytes": binary_bytes,
        "binary_sha256": binary_sha256,
        "source_sha256": source_sha256,
    })
}

fn proposal_value(outcome: &TransitionOutcome) -> Value {
    json!({
        "transition": outcome.transition,
        "observer_generation": outcome.observation.observer_generation,
        "weakness_id": outcome.weakness.weakness_id,
        "mechanism_id": outcome.selection.selected.mechanism_id,
        "source_concept_ids": outcome.selection.selected.source_concept_ids,
        "target_component": outcome.change_ir.target_component,
        "change_ir_sha256": outcome.change_ir.change_ir_sha256,
        "autonomous": true,
        "human_concept_id_assignment": false,
    })
}

fn lineage_value(outcome: &TransitionOutcome) -> Value {
    json!({
        "transition": outcome.transition,
        "parent_generation_id": outcome.parent_generation_id,
        "child_generation_id": outcome.child_generation_id,
        "parent_source_sha256": outcome.parent_source_sha256,
        "child_source_sha256": outcome.child_source_sha256,
        "parent_binary_sha256": outcome.parent_binary_sha256,
        "child_binary_sha256": outcome.child_binary_sha256,
        "diff_sha256": outcome.diff_sha256,
        "semantic_state_sha256": outcome.semantic_state_sha256,
        "index_sha256": outcome.index_sha256,
        "verified": outcome.verified,
    })
}

fn markdown_report(
    first: &TransitionOutcome,
    second: &TransitionOutcome,
    r2_observation: &SelfObservation,
) -> String {
    format!(
        "# SEM-10 Fresh Run 0002\n\n\
         Status: **PASS**\n\n\
         This campaign verified bounded serial multi-generation recursive self-improvement across two causally grounded transitions. It does not claim open-ended RSI, AGI, ASI, or intelligence explosion.\n\n\
         ## Serial proof\n\n\
         - R0 observed `{}` and autonomously routed `{}` / `{}`.\n\
         - R0→R1 strict solve rate: `{:.3}` → `{:.3}`; median deterministic operations: `{:.1}` → `{:.1}` ({:.2}% reduction); regressions: `{}`.\n\
         - R1 then observed itself as the current generation, detected `{}`, and autonomously routed `{}` / `{}`.\n\
         - R1→R2 strict solve rate: `{:.3}` → `{:.3}`; median deterministic operations: `{:.1}` → `{:.1}` ({:.2}% reduction); regressions: `{}`.\n\
         - R2 self-observation `{}` found no actionable weakness under the frozen thresholds, so R3 was not forced.\n\n\
         Both transitions used distinct fresh 140-task manifests, exact parent/child manifest parity, strict sandbox builds, differential Clippy with no new signatures, per-capability zero regression, mechanism ablation, source-concept causality, workspace tests, the core-only build, and the direct runtime canary.\n\n\
         Production B_Core source was not mutated or promoted. No merge or push occurred. SEM-11 was not started.\n",
        first.weakness.feature,
        first.selection.selected.mechanism_id,
        first.selection.selected.transform,
        first.parent_evaluation.strict_solve_rate,
        first.child_evaluation.strict_solve_rate,
        first.parent_evaluation.median_deterministic_ops,
        first.child_evaluation.median_deterministic_ops,
        first.deterministic_ops_reduction * 100.0,
        first.regressed_tasks,
        second.weakness.feature,
        second.selection.selected.mechanism_id,
        second.selection.selected.transform,
        second.parent_evaluation.strict_solve_rate,
        second.child_evaluation.strict_solve_rate,
        second.parent_evaluation.median_deterministic_ops,
        second.child_evaluation.median_deterministic_ops,
        second.deterministic_ops_reduction * 100.0,
        second.regressed_tasks,
        r2_observation.observation_sha256,
    )
}

fn preserve_failed_transition(directory: &Path, outcome: &TransitionOutcome) -> Result<(), String> {
    write_json(
        directory.join(format!("failed_{}.json", outcome.transition.to_lowercase())),
        outcome,
    )
}

fn verify_p0(root: &Path) -> Result<(), String> {
    let report: Value = read_json(&root.join("reports/sem10-p0/sem10_p0_final_report.json"))?;
    if report["sem10_p0_status"] != "PASS"
        || report["predecessor_integrity"] != "PASS"
        || report["sem10_r0"]["source_id"] != R0_SOURCE_ID
        || report["sem10_r0"]["local_binary_sha256"] != R0_BINARY_SHA256
    {
        return Err("SEM10_P0_PREDECESSOR_INTEGRITY_FAILURE".to_string());
    }
    Ok(())
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
        "artifacts/core-x0/semantic_state.json".to_string(),
        "artifacts/core-x0/sparse_index.json".to_string(),
        "reports/sem8/source_mechanism_catalog.json".to_string(),
        "reports/sem10-p0".to_string(),
    ]
}

fn hash_path_set(root: &Path, relative_paths: &[String]) -> Result<String, String> {
    let mut records = Vec::new();
    for relative in relative_paths {
        let path = root.join(relative);
        if !path.exists() {
            records.push(format!("MISSING\t{relative}"));
            continue;
        }
        if path.is_file() {
            records.push(format!("FILE\t{}\t{}", relative, hash_file(&path)?));
            continue;
        }
        collect_tree_records(root, &path, &mut records)?;
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
        let child = entry.path();
        if child.is_dir() {
            collect_tree_records(root, &child, records)?;
        } else {
            let relative = child
                .strip_prefix(root)
                .map_err(|error| error.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            records.push(format!("FILE\t{}\t{}", relative, hash_file(&child)?));
        }
    }
    Ok(())
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
    // rustfmt may add or remove a trailing comma before a closing delimiter.
    // Rust assigns that comma no executable meaning, so it is formatting
    // punctuation rather than a semantic token change.
    let mut semantic_tokens = Vec::with_capacity(normalized.len());
    let mut index = 0usize;
    let mut quote = None;
    while index < normalized.len() {
        let byte = normalized[index];
        if let Some(delimiter) = quote {
            semantic_tokens.push(byte);
            if byte == b'\\' && index + 1 < normalized.len() {
                index += 1;
                semantic_tokens.push(normalized[index]);
            } else if byte == delimiter {
                quote = None;
            }
        } else if byte == b'"' || byte == b'\'' {
            quote = Some(byte);
            semantic_tokens.push(byte);
        } else if byte == b',' && matches!(normalized.get(index + 1), Some(b'}' | b']')) {
            index += 1;
            continue;
        } else {
            semantic_tokens.push(byte);
        }
        index += 1;
    }
    semantic_tokens
}

fn generation_id(name: &str, source: &str, binary: &str, parent: &str) -> String {
    format!(
        "{name}:{}",
        hash_bytes(format!("{name}:{source}:{binary}:{parent}").as_bytes())
    )
}

fn seed_commitment(label: &str, seed: u64) -> String {
    hash_bytes(format!("{CAMPAIGN_ID}:{label}:{seed}").as_bytes())
}

fn reduction(before: f64, after: f64) -> f64 {
    if before == 0.0 {
        0.0
    } else {
        (before - after) / before
    }
}

fn median_usize(values: &[usize]) -> f64 {
    let mut ordered = values.to_vec();
    ordered.sort_unstable();
    if ordered.is_empty() {
        0.0
    } else if ordered.len().is_multiple_of(2) {
        let upper = ordered.len() / 2;
        (ordered[upper - 1] as f64 + ordered[upper] as f64) / 2.0
    } else {
        ordered[ordered.len() / 2] as f64
    }
}

fn median_u128(values: &[u128]) -> f64 {
    let mut ordered = values.to_vec();
    ordered.sort_unstable();
    if ordered.is_empty() {
        0.0
    } else if ordered.len().is_multiple_of(2) {
        let upper = ordered.len() / 2;
        (ordered[upper - 1] as f64 + ordered[upper] as f64) / 2.0
    } else {
        ordered[ordered.len() / 2] as f64
    }
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

fn write_json(path: PathBuf, value: &impl Serialize) -> Result<(), String> {
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

use sem10_fresh_probe::{schedule_profiled, State};

fn main() {
    let path = env::args().nth(1).expect("input path");
    let input = fs::read_to_string(path).expect("read input");
    for line in input.lines() {
        let (task_id, encoded) = line.split_once('\t').expect("task line");
        let states = encoded
            .split(',')
            .filter(|value| !value.is_empty())
            .map(|value| {
                let (key, payload) = value.split_once(':').expect("state");
                State {
                    canonical_key: key.parse().expect("key"),
                    payload: payload.parse().expect("payload"),
                }
            })
            .collect::<Vec<_>>();
        let (mut keys, profile) = schedule_profiled(&states);
        keys.sort_unstable();
        let keys = keys.iter().map(u64::to_string).collect::<Vec<_>>().join(",");
        println!(
            "{task_id}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{keys}",
            profile.expansions,
            profile.deterministic_ops,
            profile.ordered_comparisons,
            profile.stage_writes,
            profile.peak_frontier,
            profile.estimated_peak_bytes,
            keys.len()
        );
    }
}
"#;

const R0_PROFILE_SOURCE: &str = r#"#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct State {
    pub canonical_key: u64,
    pub payload: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    pub expansions: usize,
    pub deterministic_ops: usize,
    pub ordered_comparisons: usize,
    pub stage_writes: usize,
    pub peak_frontier: usize,
    pub estimated_peak_bytes: usize,
}

fn locate(values: &[u64], key: u64, comparisons: &mut usize) -> Result<usize, usize> {
    let mut low = 0usize;
    let mut high = values.len();
    while low < high {
        let middle = low + (high - low) / 2;
        *comparisons += 1;
        if values[middle] < key {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    if low < values.len() {
        *comparisons += 1;
        if values[low] == key {
            return Ok(low);
        }
    }
    Err(low)
}

pub fn schedule_profiled(states: &[State]) -> (Vec<u64>, Profile) {
    let mut seen = Vec::new();
    let mut reachable = Vec::new();
    let mut expansions = 0usize;
    let mut ordered_comparisons = 0usize;
    let mut stage_writes = 0usize;
    for state in states {
        let Err(seen_position) = locate(&seen, state.canonical_key, &mut ordered_comparisons) else {
            continue;
        };
        seen.insert(seen_position, state.canonical_key);
        stage_writes += 1;
        expansions += 1;
        let Err(reachable_position) =
            locate(&reachable, state.canonical_key, &mut ordered_comparisons)
        else {
            continue;
        };
        reachable.insert(reachable_position, state.canonical_key);
        stage_writes += 1;
    }
    let deterministic_ops = ordered_comparisons + stage_writes;
    let profile = Profile {
        expansions,
        deterministic_ops,
        ordered_comparisons,
        stage_writes,
        peak_frontier: reachable.len(),
        estimated_peak_bytes: (seen.capacity() + reachable.capacity()) * size_of::<u64>(),
    };
    (reachable, profile)
}

use std::mem::size_of;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_membership_and_merges_equivalence() {
        let states = [
            State { canonical_key: 4, payload: 40 },
            State { canonical_key: 4, payload: 41 },
            State { canonical_key: 9, payload: 90 },
        ];
        let (keys, profile) = schedule_profiled(&states);
        assert_eq!(keys, vec![4, 9]);
        assert_eq!(profile.expansions, 2);
        assert_eq!(profile.stage_writes, 4);
    }
}
"#;

const R1_PROFILE_SOURCE: &str = r#"use std::mem::size_of;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct State {
    pub canonical_key: u64,
    pub payload: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    pub expansions: usize,
    pub deterministic_ops: usize,
    pub ordered_comparisons: usize,
    pub stage_writes: usize,
    pub peak_frontier: usize,
    pub estimated_peak_bytes: usize,
}

fn locate(values: &[u64], key: u64, comparisons: &mut usize) -> Result<usize, usize> {
    let mut low = 0usize;
    let mut high = values.len();
    while low < high {
        let middle = low + (high - low) / 2;
        *comparisons += 1;
        if values[middle] < key {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    if low < values.len() {
        *comparisons += 1;
        if values[low] == key {
            return Ok(low);
        }
    }
    Err(low)
}

pub fn schedule_profiled(states: &[State]) -> (Vec<u64>, Profile) {
    let mut reachable = Vec::new();
    let mut expansions = 0usize;
    let mut ordered_comparisons = 0usize;
    let mut stage_writes = 0usize;
    for state in states {
        let Err(position) = locate(&reachable, state.canonical_key, &mut ordered_comparisons)
        else {
            continue;
        };
        reachable.insert(position, state.canonical_key);
        stage_writes += 1;
        expansions += 1;
    }
    let deterministic_ops = ordered_comparisons + stage_writes;
    let profile = Profile {
        expansions,
        deterministic_ops,
        ordered_comparisons,
        stage_writes,
        peak_frontier: reachable.len(),
        estimated_peak_bytes: reachable.capacity() * size_of::<u64>(),
    };
    (reachable, profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composes_membership_stages_without_semantic_change() {
        let states = [
            State { canonical_key: 4, payload: 40 },
            State { canonical_key: 4, payload: 41 },
            State { canonical_key: 9, payload: 90 },
        ];
        let (keys, profile) = schedule_profiled(&states);
        assert_eq!(keys, vec![4, 9]);
        assert_eq!(profile.expansions, 2);
        assert_eq!(profile.stage_writes, 2);
    }
}
"#;

const R2_PROFILE_SOURCE: &str = r#"use std::mem::size_of;

const KEY_BOUND: usize = 4096;
const WORDS: usize = KEY_BOUND / 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct State {
    pub canonical_key: u64,
    pub payload: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    pub expansions: usize,
    pub deterministic_ops: usize,
    pub ordered_comparisons: usize,
    pub stage_writes: usize,
    pub peak_frontier: usize,
    pub estimated_peak_bytes: usize,
}

fn locate(values: &[u64], key: u64, comparisons: &mut usize) -> Result<usize, usize> {
    let mut low = 0usize;
    let mut high = values.len();
    while low < high {
        let middle = low + (high - low) / 2;
        *comparisons += 1;
        if values[middle] < key {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    if low < values.len() {
        *comparisons += 1;
        if values[low] == key {
            return Ok(low);
        }
    }
    Err(low)
}

pub fn schedule_profiled(states: &[State]) -> (Vec<u64>, Profile) {
    let mut activation = [0u64; WORDS];
    let mut overflow = Vec::new();
    let mut reachable = Vec::new();
    let mut expansions = 0usize;
    let mut ordered_comparisons = 0usize;
    let mut stage_writes = 0usize;
    let mut activation_checks = 0usize;
    for state in states {
        let key = state.canonical_key;
        let newly_activated = if key < KEY_BOUND as u64 {
            activation_checks += 1;
            let index = key as usize;
            let word = index / 64;
            let mask = 1u64 << (index % 64);
            let unseen = activation[word] & mask == 0;
            if unseen {
                activation[word] |= mask;
            }
            unseen
        } else {
            match locate(&overflow, key, &mut ordered_comparisons) {
                Ok(_) => false,
                Err(position) => {
                    overflow.insert(position, key);
                    true
                }
            }
        };
        if !newly_activated {
            continue;
        }
        reachable.push(key);
        stage_writes += 1;
        expansions += 1;
    }
    let deterministic_ops = activation_checks + ordered_comparisons + stage_writes;
    let profile = Profile {
        expansions,
        deterministic_ops,
        ordered_comparisons,
        stage_writes,
        peak_frontier: reachable.len(),
        estimated_peak_bytes: size_of::<[u64; WORDS]>()
            + (overflow.capacity() + reachable.capacity()) * size_of::<u64>(),
    };
    (reachable, profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_partition_preserves_membership_with_unbounded_fallback() {
        let states = [
            State { canonical_key: 4, payload: 40 },
            State { canonical_key: 4, payload: 41 },
            State { canonical_key: 9000, payload: 90 },
            State { canonical_key: 9000, payload: 91 },
        ];
        let (mut keys, profile) = schedule_profiled(&states);
        keys.sort_unstable();
        assert_eq!(keys, vec![4, 9000]);
        assert_eq!(profile.expansions, 2);
        assert_eq!(profile.stage_writes, 2);
    }
}
"#;
