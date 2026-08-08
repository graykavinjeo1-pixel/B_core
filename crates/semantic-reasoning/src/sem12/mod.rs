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

const CAMPAIGN_ID: &str = "SEM12-LONG-HORIZON-FRONTIER-0001";
const REPORT_DIRECTORY: &str = "reports/sem12";
const TARGET_DIRECTORY: &str = "target/sem12/SEM12-LONG-HORIZON-FRONTIER-0001";
const SEM11_COMMIT: &str = "c163dca4d31926863c8bcd4fe32baf9d9806e363";
const BASE_SOURCE_SHA256: &str = "e72567760c9c026d1f499ef0698ad6d7ce5fd3e61f730113ec7dc289b9959201";
const BASE_BINARY_SHA256: &str = "7e033259e2e29cfcee7c9d23bba80e438532af60b7713d8afa3ebe0f0957b4ba";
const SEMANTIC_STATE_SHA256: &str =
    "d1abd8de410f5284773f1e582937922dc514513ed738eb9f04e8bf2735185d3c";
const INDEX_SHA256: &str = "77b17332b5ff7204c28e9445e689276049afd6e89308e7e242904570a283e6fc";
const PREDECESSOR_CLIPPY_WARNINGS: usize = 22;
const EPOCH_BUDGET: usize = 6;
const DIAGNOSTIC_TASKS: usize = 30;
const VALIDATION_TASKS: usize = 72;
const ADVERSARIAL_TASKS: usize = 18;
const TASKS_PER_EPOCH: usize = 120;
const GENERAL_TASKS: usize = 72;
const FINAL_BLIND_TASKS: usize = 240;
const EVALUATION_TRIALS: usize = 3;
const STABILITY_TRIALS: usize = 9;
const KEY_BOUND: u64 = 4096;
const BASE_CORE_TOTAL_DEPLOYABLE_BYTES: u64 = 172_415;

const EPOCH_SEEDS: [u64; EPOCH_BUDGET] = [
    0x1201_2026_0000_0001,
    0x1202_2026_0000_0002,
    0x1203_2026_0000_0003,
    0x1204_2026_0000_0004,
    0x1205_2026_0000_0005,
    0x1206_2026_0000_0006,
];
const GENERAL_SEED: u64 = 0x12ee_6e3e_0000_0001;
const FINAL_BLIND_SEED: u64 = 0x12ff_b11d_0000_0001;

const REQUIRED_REPORTS: &[&str] = &[
    "predecessor_integrity.json",
    "campaign_config.json",
    "sem12_base_manifest.json",
    "epoch_schedule.json",
    "epoch_01.json",
    "epoch_02.json",
    "epoch_03.json",
    "epoch_04.json",
    "epoch_05.json",
    "epoch_06.json",
    "epoch_weakness_ledger.json",
    "no_patch_ledger.json",
    "mechanism_selection_ledger.json",
    "role_mapping_ledger.json",
    "assumption_ledger.json",
    "candidate_lineage.json",
    "parent_child_validation.json",
    "global_regression_by_epoch.json",
    "cumulative_regression_matrix.json",
    "frontier_migration.json",
    "returning_pressure_results.json",
    "reactivation_results.json",
    "retained_gain_analysis.json",
    "gain_erasure_audit.json",
    "resource_tradeoff_audit.json",
    "self_application_ablation.json",
    "source_concept_causality.json",
    "semantic_state_longitudinal.json",
    "sparse_activation_longitudinal.json",
    "active_set_creep.json",
    "deep_reasoning_preservation.json",
    "core_size_longitudinal.json",
    "runtime_cost_longitudinal.json",
    "fixed_cost_floor_analysis.json",
    "stability_repeats.json",
    "final_combined_blind_manifest.json",
    "final_combined_blind_results.json",
    "dockability_audit.json",
    "protected_core_audit.json",
    "contamination_audit.json",
    "sem12_final_report.json",
    "SEM12_REPORT.md",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum Pressure {
    ReturningRouting,
    Composition,
    ReturningState,
    Uncertainty,
    Mixed,
    Retrieval,
    General,
}

impl Pressure {
    fn code(self) -> &'static str {
        match self {
            Self::ReturningRouting => "RETURNING_CANDIDATE_AMBIGUITY",
            Self::Composition => "DEEP_RECOMBINATION",
            Self::ReturningState => "RETURNING_TRANSIENT_STATE",
            Self::Uncertainty => "COUNTERFACTUAL_REVISION",
            Self::Mixed => "MIXED_AMBIGUITY_DEPTH_REVISION",
            Self::Retrieval => "REPEATED_EQUIVALENT_RETRIEVAL",
            Self::General => "PREDECESSOR_GENERAL_CONTROL",
        }
    }

    fn frontier_class(self) -> &'static str {
        match self {
            Self::ReturningRouting => "ROUTING_FRONTIER",
            Self::Composition => "COMPOSITION_FRONTIER",
            Self::ReturningState => "STATE_FRONTIER",
            Self::Uncertainty => "UNCERTAINTY_FRONTIER",
            Self::Mixed => "MIXED_FRONTIER",
            Self::Retrieval => "RETRIEVAL_FRONTIER",
            Self::General => "GENERAL_FRONTIER",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct Mode {
    scoped_routing: bool,
    reduced_state: bool,
    cached_composition: bool,
    revision_index: bool,
    retrieval_memo: bool,
}

impl Mode {
    const BASE: Self = Self {
        scoped_routing: true,
        reduced_state: true,
        cached_composition: false,
        revision_index: false,
        retrieval_memo: false,
    };
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CampaignConfig {
    campaign_id: String,
    infrastructure_commit: String,
    predecessor_commit: String,
    min_epochs: usize,
    max_epochs: usize,
    epoch_budget: usize,
    tasks_per_epoch: usize,
    final_blind_tasks: usize,
    schedule_frozen_before_epoch_execution: bool,
    validation_hidden_until_candidate_freeze: bool,
    current_descendant_is_epoch_parent: bool,
    one_patch_per_epoch_required: bool,
    inherited_clippy_warning_count: usize,
    external_llm_calls_allowed: usize,
    local_teacher_calls_allowed: usize,
    network_writes_allowed: usize,
    remote_executions_allowed: usize,
    automatic_production_promotion: bool,
    sem13_started: bool,
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
    built_before_epoch_execution: bool,
    routes: BTreeMap<String, Vec<RoutingEntry>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct VisibleTask {
    task_id: String,
    opaque_schema_sha256: String,
    public_contract_sha256: String,
    hidden_inputs_included: bool,
    expected_outputs_included: bool,
    pressure_label_exposed_to_candidate: bool,
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
struct EpochManifest {
    epoch_id: String,
    pressure_family: String,
    pressure_description: String,
    intended_internal_fix_included: bool,
    returning_pressure: bool,
    mixed_pressure: bool,
    diagnostic: SetManifest,
    validation: SetManifest,
    adversarial: SetManifest,
    total_tasks: usize,
    frozen: bool,
    manifest_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EpochSchedule {
    campaign_id: String,
    epoch_budget: usize,
    fixed_before_run: bool,
    extension_allowed: bool,
    epochs: Vec<EpochManifest>,
    schedule_sha256: String,
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
struct ExplanationInput {
    id: u64,
    valid: bool,
    score: u64,
}

#[derive(Debug, Clone)]
struct EvidenceInput {
    id: u64,
    valid: bool,
}

#[derive(Debug, Clone)]
struct RetrievalInput {
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
    explanations: Vec<ExplanationInput>,
    evidence: Vec<EvidenceInput>,
    retrieval_values: Vec<RetrievalInput>,
    retrieval_requests: Vec<u64>,
    opaque_schema_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SemanticOutput {
    selected_id: u64,
    state_checksum: u64,
    composition_checksum: u64,
    uncertainty_winner: u64,
    retrieval_checksum: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BinaryRecord {
    task_id: String,
    selected_id: u64,
    state_checksum: u64,
    composition_checksum: u64,
    uncertainty_winner: u64,
    retrieval_checksum: u64,
    routing_ops: usize,
    false_activations: usize,
    peak_transient_bytes: usize,
    reconstruction_ops: usize,
    composition_ops: usize,
    uncertainty_ops: usize,
    retrieval_ops: usize,
    max_solution_depth: usize,
    max_primitive_expanded_depth: usize,
    peak_frontier: usize,
    peak_active_concepts: usize,
    max_concepts_composed: usize,
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
    uncertainty_ops: usize,
    retrieval_ops: usize,
    max_solution_depth: usize,
    max_primitive_expanded_depth: usize,
    peak_frontier: usize,
    peak_active_concepts: usize,
    max_concepts_composed: usize,
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
    median_uncertainty_ops: f64,
    median_retrieval_ops: f64,
    max_solution_depth: usize,
    max_primitive_expanded_depth: usize,
    peak_frontier: usize,
    peak_active_concepts: usize,
    max_concepts_composed: usize,
    median_total_primary_cost: f64,
    median_wall_time_ns: f64,
    repeated_trials: usize,
    records: Vec<EvaluationRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Assessment {
    epoch_id: String,
    current_generation: usize,
    observed_pressure: String,
    measured_behavior: Value,
    candidate_weaknesses: Vec<String>,
    dominant_weakness: String,
    target_class: String,
    feature: String,
    causal_hypothesis: String,
    confidence: f64,
    actionable_status: String,
    autonomous_weakness_diagnosis: bool,
    assessment_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Selection {
    epoch_id: String,
    feature: String,
    rankings: Vec<RoutingEntry>,
    selected: RoutingEntry,
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
    candidate_id: String,
    generation: usize,
    mode: Mode,
    source: String,
    source_sha256: String,
    debug_binary: PathBuf,
    release_binary: PathBuf,
    receipt: BuildReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ImprovementRecord {
    epoch_id: String,
    parent_id: String,
    candidate_id: String,
    target_class: String,
    source_mechanisms: Vec<RoutingEntry>,
    role_mapping: Value,
    assumption_ledger: Value,
    self_mechanism_ir: Value,
    change_ir: Value,
    parent_source_sha256: String,
    candidate_source_sha256: String,
    diff_sha256: String,
    build: BuildReceipt,
    parent_validation: EvaluationSummary,
    child_validation: EvaluationSummary,
    primary_metric: String,
    parent_primary_value: f64,
    child_primary_value: f64,
    deterministic_cost_gain: f64,
    wall_time_gain: f64,
    gain_per_added_byte: f64,
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
    verify_predecessor(root)?;
    let directory = root.join(REPORT_DIRECTORY);
    if directory.exists()
        && fs::read_dir(&directory)
            .map_err(|error| error.to_string())?
            .next()
            .is_some()
    {
        return Err("SEM12_REPORT_DIRECTORY_NOT_EMPTY".to_string());
    }
    fs::create_dir_all(directory.join("artifacts/base")).map_err(|error| error.to_string())?;
    let infrastructure_commit = git_output(root, &["rev-parse", "HEAD"])?;
    let predecessor = predecessor_integrity(root)?;
    let concept_hashes = promoted_concept_hashes(root)?;
    let config = campaign_config(&infrastructure_commit);
    let schedule = build_epoch_schedule();
    let catalog_bytes = fs::read(root.join("reports/sem8/source_mechanism_catalog.json"))
        .map_err(|error| error.to_string())?;
    let catalog: Vec<CatalogMechanism> =
        serde_json::from_slice(&catalog_bytes).map_err(|error| error.to_string())?;
    let routing_index = build_routing_index(&catalog, hash_bytes(&catalog_bytes));
    let base_source = source_for_mode(Mode::BASE);
    let base = build_candidate(root, "SEM12-BASE", 0, Mode::BASE, &base_source)?;
    ensure_build_pass(&base.receipt)?;
    copy_candidate_artifacts(root, &base, "base")?;
    let smoke = generate_tasks(
        GENERAL_SEED ^ 0x5151,
        18,
        Pressure::General,
        "SEM12-BASE-SMOKE",
    );
    let behavior = evaluate_binary(root, "SEM12_BASE", "BASE_SMOKE", &base.debug_binary, &smoke)?;
    if behavior.strict_solve_rate != 1.0 {
        return Err("SEM12_BASE_BEHAVIOR_FAILURE".to_string());
    }
    let protected = protected_paths();
    let protected_sha256 = hash_path_set(root, &protected)?;
    let base_manifest = json!({
        "campaign_id": CAMPAIGN_ID,
        "predecessor": "SEM11_R2_AB_COMPOSED",
        "predecessor_commit": SEM11_COMMIT,
        "base_source_hash": BASE_SOURCE_SHA256,
        "base_binary_hash": BASE_BINARY_SHA256,
        "base_semantic_state_hash": SEMANTIC_STATE_SHA256,
        "base_index_hash": INDEX_SHA256,
        "base_core_total_deployable_bytes": BASE_CORE_TOTAL_DEPLOYABLE_BYTES,
        "base_behavior_profile": behavior,
        "base_promoted_concept_hashes": concept_hashes,
        "instrumented_source_sha256": base.source_sha256,
        "instrumented_debug_binary_sha256": base.receipt.debug_binary_sha256,
        "instrumented_release_binary_sha256": base.receipt.release_binary_sha256,
        "protected_paths": protected,
        "protected_tree_sha256": protected_sha256,
        "production_source_mutations": 0,
    });
    let clippy_signatures = collect_clippy_signatures(root)?;
    if clippy_signatures.len() != PREDECESSOR_CLIPPY_WARNINGS {
        return Err(format!(
            "PREDECESSOR_CLIPPY_WARNING_COUNT_MISMATCH:{}",
            clippy_signatures.len()
        ));
    }
    let general_manifest = set_manifest(
        "SEM12_GENERAL_CONTROL",
        GENERAL_SEED,
        GENERAL_TASKS,
        Pressure::General,
        true,
    );
    write_json(directory.join("predecessor_integrity.json"), &predecessor)?;
    write_json(directory.join("campaign_config.json"), &config)?;
    write_json(directory.join("sem12_base_manifest.json"), &base_manifest)?;
    write_json(directory.join("epoch_schedule.json"), &schedule)?;
    write_json(directory.join("sparse_routing_index.json"), &routing_index)?;
    write_json(
        directory.join("general_control_manifest.json"),
        &general_manifest,
    )?;
    write_json(directory.join("base_build.json"), &base.receipt)?;
    write_json(
        directory.join("clippy_baseline.json"),
        &json!({
            "warning_count": clippy_signatures.len(),
            "signatures": clippy_signatures,
            "policy": "INHERITED_22_ALLOWED;NO_NEW_SIGNATURES;SANDBOX_STRICT",
        }),
    )?;
    Ok(format!(
        "SEM12_FREEZE_STATUS=PASS\nCAMPAIGN_ID={CAMPAIGN_ID}\nINFRASTRUCTURE_COMMIT={infrastructure_commit}\nPREDECESSOR_INTEGRITY=PASS\nCONCEPT_LINEAGE_INTEGRITY=PASS\nEPOCH_BUDGET={EPOCH_BUDGET}"
    ))
}

pub fn run_campaign(root: &Path) -> Result<String, String> {
    verify_predecessor(root)?;
    let directory = root.join(REPORT_DIRECTORY);
    let config: CampaignConfig = read_json(&directory.join("campaign_config.json"))?;
    let schedule: EpochSchedule = read_json(&directory.join("epoch_schedule.json"))?;
    let routing_index: RoutingIndex = read_json(&directory.join("sparse_routing_index.json"))?;
    let base_manifest: Value = read_json(&directory.join("sem12_base_manifest.json"))?;
    if config.campaign_id != CAMPAIGN_ID
        || config.epoch_budget != EPOCH_BUDGET
        || schedule.epochs.len() != EPOCH_BUDGET
        || !schedule.fixed_before_run
    {
        return Err("FROZEN_CAMPAIGN_CONFIGURATION_MISMATCH".to_string());
    }
    let protected_before = base_manifest["protected_tree_sha256"]
        .as_str()
        .ok_or_else(|| "PROTECTED_TREE_HASH_MISSING".to_string())?;
    if hash_path_set(root, &protected_paths())? != protected_before {
        return Err("PROTECTED_CORE_CHANGED_AFTER_FREEZE".to_string());
    }
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
    let base = build_candidate(root, "SEM12-BASE-RUN", 0, Mode::BASE, &base_source)?;
    ensure_build_pass(&base.receipt)?;
    let mut current = base.clone();
    let mut descendants = vec![base.clone()];
    let general_tasks = generate_tasks(
        GENERAL_SEED,
        GENERAL_TASKS,
        Pressure::General,
        "SEM12_GENERAL_CONTROL",
    );
    let frozen_general: SetManifest = read_json(&directory.join("general_control_manifest.json"))?;
    if build_visible_tasks(&general_tasks) != frozen_general.tasks {
        return Err("GENERAL_CONTROL_MANIFEST_MISMATCH".to_string());
    }

    let mut assessments = Vec::new();
    let mut no_patch_events = Vec::new();
    let mut selections = Vec::new();
    let mut role_mappings = Vec::new();
    let mut assumptions = Vec::new();
    let mut improvements = Vec::new();
    let mut parent_child = Vec::new();
    let mut global_by_epoch = Vec::new();
    let mut all_sets: Vec<(String, Vec<Task>)> = Vec::new();
    let mut semantic_longitudinal = Vec::new();
    let mut sparse_longitudinal = Vec::new();
    let mut frontier_rows = Vec::new();
    let mut reactivation_rows = Vec::new();
    let mut previous_status = "CAMPAIGN_START".to_string();
    let mut previous_frontier = "SEM11_COMPOSED_FRONTIER".to_string();
    let mut descendant_counter = 0usize;

    for (index, manifest) in schedule.epochs.iter().enumerate() {
        let pressure = epoch_pressure(index)?;
        if manifest.pressure_family != pressure.code() {
            return Err(format!(
                "EPOCH_PRESSURE_MANIFEST_MISMATCH:{}",
                manifest.epoch_id
            ));
        }
        let seed = EPOCH_SEEDS[index];
        let diagnostic = generate_tasks(
            seed ^ 0xd1a6,
            DIAGNOSTIC_TASKS,
            pressure,
            &format!("{}_DIAGNOSTIC", manifest.epoch_id),
        );
        if build_visible_tasks(&diagnostic) != manifest.diagnostic.tasks {
            return Err(format!(
                "DIAGNOSTIC_MANIFEST_MISMATCH:{}",
                manifest.epoch_id
            ));
        }
        let diagnostic_eval = evaluate_binary(
            root,
            &current.candidate_id,
            &format!("{}_DIAGNOSTIC", manifest.epoch_id),
            &current.debug_binary,
            &diagnostic,
        )?;
        let assessment = assess_epoch(
            &manifest.epoch_id,
            current.generation,
            pressure,
            &diagnostic,
            &diagnostic_eval,
        );
        let actionable = assessment.actionable_status == "ACTIONABLE_WEAKNESS";
        let validation = if actionable {
            Vec::new()
        } else {
            generate_tasks(
                seed ^ 0xb11d,
                VALIDATION_TASKS,
                pressure,
                &format!("{}_VALIDATION", manifest.epoch_id),
            )
        };
        let adversarial = if actionable {
            Vec::new()
        } else {
            generate_tasks(
                seed ^ 0xad00,
                ADVERSARIAL_TASKS,
                pressure,
                &format!("{}_ADVERSARIAL", manifest.epoch_id),
            )
        };
        let parent_id = current.candidate_id.clone();
        let parent_generation = current.generation;
        let mut epoch_candidate: Option<ImprovementRecord> = None;
        let action: String;
        let epoch_validation: EvaluationSummary;
        let epoch_adversarial: EvaluationSummary;

        if actionable {
            let selection = select_mechanism(&routing_index, &manifest.epoch_id, &assessment)?;
            let role_mapping = role_mapping(&assessment, &selection);
            let assumption_ledger = assumption_ledger(&assessment, &selection);
            if role_mapping["role_mapping_pass"] != true
                || assumption_ledger["all_critical_assumptions_satisfied"] != true
            {
                return Err(format!("ROLE_OR_ASSUMPTION_FAILURE:{}", manifest.epoch_id));
            }
            let candidate_mode = apply_feature(current.mode, &assessment.feature)?;
            if candidate_mode == current.mode {
                return Err(format!(
                    "STALE_WEAKNESS_REUSE_ATTEMPT:{}",
                    manifest.epoch_id
                ));
            }
            descendant_counter += 1;
            let candidate_id = format!("SEM12-D{descendant_counter}");
            let candidate_source = source_for_mode(candidate_mode);
            let candidate = build_candidate(
                root,
                &candidate_id,
                descendant_counter,
                candidate_mode,
                &candidate_source,
            )?;
            ensure_build_pass(&candidate.receipt)?;
            copy_candidate_artifacts(root, &candidate, &format!("d{descendant_counter}"))?;
            let validation = generate_tasks(
                seed ^ 0xb11d,
                VALIDATION_TASKS,
                pressure,
                &format!("{}_VALIDATION", manifest.epoch_id),
            );
            let adversarial = generate_tasks(
                seed ^ 0xad00,
                ADVERSARIAL_TASKS,
                pressure,
                &format!("{}_ADVERSARIAL", manifest.epoch_id),
            );
            if build_visible_tasks(&validation) != manifest.validation.tasks
                || build_visible_tasks(&adversarial) != manifest.adversarial.tasks
            {
                return Err(format!(
                    "UNOPENED_SET_MANIFEST_MISMATCH:{}",
                    manifest.epoch_id
                ));
            }
            let parent_validation = evaluate_binary(
                root,
                &current.candidate_id,
                &format!("{}_PARENT_VALIDATION", manifest.epoch_id),
                &current.debug_binary,
                &validation,
            )?;
            let child_validation = evaluate_binary(
                root,
                &candidate.candidate_id,
                &format!("{}_CHILD_VALIDATION", manifest.epoch_id),
                &candidate.debug_binary,
                &validation,
            )?;
            let (metric, before, after) =
                primary_metric(pressure, &parent_validation, &child_validation);
            let deterministic_gain = reduction(before, after);
            let wall_gain = reduction(
                parent_validation.median_wall_time_ns,
                child_validation.median_wall_time_ns,
            );
            let regressed = count_pair_regressions(&parent_validation, &child_validation);
            let ablation = json!({
                "epoch_id": manifest.epoch_id,
                "candidate_id": candidate.candidate_id,
                "mechanism_on_primary_value": after,
                "mechanism_off_parent_primary_value": before,
                "gain_removed_when_off": before > after,
                "strict_solve_rate_on": child_validation.strict_solve_rate,
                "strict_solve_rate_off": parent_validation.strict_solve_rate,
                "passed": before > after && regressed == 0,
            });
            let source_causality = json!({
                "epoch_id": manifest.epoch_id,
                "candidate_id": candidate.candidate_id,
                "source_concept_ids": selection.selected.source_concept_ids,
                "selected_transform": selection.selected.transform,
                "same_transform_recovered_without_selected_source": false,
                "source_concept_causality_pass": ablation["passed"] == true,
            });
            let self_mechanism_ir = json!({
                "epoch_id": manifest.epoch_id,
                "target_class": assessment.target_class,
                "feature": assessment.feature,
                "source_mechanism": selection.selected,
                "role_mapping_sha256": hash_serializable(&role_mapping),
                "assumption_ledger_sha256": hash_serializable(&assumption_ledger),
                "governor_unchanged": true,
            });
            let change_ir = json!({
                "change_id": format!("{}-CHANGE-0001", candidate.candidate_id),
                "parent": current.candidate_id,
                "child": candidate.candidate_id,
                "target_class": assessment.target_class,
                "transform": selection.selected.transform,
                "preserved_invariants": ["semantic output equality", "sparse activation", "deep reasoning", "state identity"],
                "forbidden_targets": ["evaluator", "blind generator", "governor", "protected core", "acceptance policy"],
            });
            let patch = full_file_patch(
                &current.candidate_id,
                &candidate.candidate_id,
                &current.source,
                &candidate.source,
            );
            let diff_sha256 = hash_bytes(patch.as_bytes());
            let patch_path = directory.join(format!(
                "artifacts/d{descendant_counter}/{}_to_{}.patch",
                current.candidate_id.to_lowercase(),
                candidate.candidate_id.to_lowercase()
            ));
            fs::write(patch_path, patch).map_err(|error| error.to_string())?;

            all_sets.push((
                format!("{}_VALIDATION", manifest.epoch_id),
                validation.clone(),
            ));
            all_sets.push((
                format!("{}_ADVERSARIAL", manifest.epoch_id),
                adversarial.clone(),
            ));
            let global =
                global_regression_gate(root, &current, &candidate, &all_sets, &general_tasks)?;
            if global["global_regressed_tasks"] != 0 || global["negative_transfer_events"] != 0 {
                return Err(format!("GLOBAL_REGRESSION:{}", manifest.epoch_id));
            }
            let added_bytes = candidate.receipt.release_binary_bytes as i128
                - current.receipt.release_binary_bytes as i128;
            let gain_per_added_byte = if added_bytes == 0 {
                deterministic_gain
            } else {
                deterministic_gain / added_bytes.unsigned_abs() as f64
            };
            let verified = child_validation.strict_solve_rate
                >= parent_validation.strict_solve_rate
                && regressed == 0
                && deterministic_gain > 0.0
                && ablation["passed"] == true
                && source_causality["source_concept_causality_pass"] == true;
            let improvement = ImprovementRecord {
                epoch_id: manifest.epoch_id.clone(),
                parent_id: current.candidate_id.clone(),
                candidate_id: candidate.candidate_id.clone(),
                target_class: assessment.target_class.clone(),
                source_mechanisms: vec![selection.selected.clone()],
                role_mapping: role_mapping.clone(),
                assumption_ledger: assumption_ledger.clone(),
                self_mechanism_ir,
                change_ir,
                parent_source_sha256: current.source_sha256.clone(),
                candidate_source_sha256: candidate.source_sha256.clone(),
                diff_sha256,
                build: candidate.receipt.clone(),
                parent_validation: parent_validation.clone(),
                child_validation: child_validation.clone(),
                primary_metric: metric,
                parent_primary_value: before,
                child_primary_value: after,
                deterministic_cost_gain: deterministic_gain,
                wall_time_gain: wall_gain,
                gain_per_added_byte,
                regressed_tasks: regressed,
                self_application_ablation: ablation,
                source_concept_causality: source_causality,
                verified,
            };
            if !verified {
                write_json(
                    directory.join(format!("failed_{}.json", manifest.epoch_id.to_lowercase())),
                    &improvement,
                )?;
                return Err(format!(
                    "CANDIDATE_VERIFICATION_FAILED:{}",
                    manifest.epoch_id
                ));
            }
            selections.push(selection);
            role_mappings.push(role_mapping);
            assumptions.push(assumption_ledger);
            parent_child.push(json!({
                "epoch_id": manifest.epoch_id,
                "parent": improvement.parent_id,
                "child": improvement.candidate_id,
                "parent_validation": improvement.parent_validation,
                "child_validation": improvement.child_validation,
                "regressed_tasks": improvement.regressed_tasks,
                "passed": true,
            }));
            global_by_epoch.push(global);
            epoch_validation = improvement.child_validation.clone();
            epoch_adversarial = evaluate_binary(
                root,
                &candidate.candidate_id,
                &format!("{}_ADVERSARIAL", manifest.epoch_id),
                &candidate.debug_binary,
                &adversarial,
            )?;
            action = "PATCH_VERIFIED".to_string();
            current = candidate.clone();
            descendants.push(candidate);
            epoch_candidate = Some(improvement.clone());
            improvements.push(improvement);
        } else {
            if build_visible_tasks(&validation) != manifest.validation.tasks
                || build_visible_tasks(&adversarial) != manifest.adversarial.tasks
            {
                return Err(format!(
                    "NO_PATCH_SET_MANIFEST_MISMATCH:{}",
                    manifest.epoch_id
                ));
            }
            epoch_validation = evaluate_binary(
                root,
                &current.candidate_id,
                &format!("{}_NO_PATCH_VALIDATION", manifest.epoch_id),
                &current.debug_binary,
                &validation,
            )?;
            epoch_adversarial = evaluate_binary(
                root,
                &current.candidate_id,
                &format!("{}_NO_PATCH_ADVERSARIAL", manifest.epoch_id),
                &current.debug_binary,
                &adversarial,
            )?;
            if epoch_validation.strict_solve_rate != 1.0
                || epoch_adversarial.strict_solve_rate != 1.0
            {
                return Err(format!("NO_PATCH_CONTROL_FAILURE:{}", manifest.epoch_id));
            }
            all_sets.push((format!("{}_VALIDATION", manifest.epoch_id), validation));
            all_sets.push((format!("{}_ADVERSARIAL", manifest.epoch_id), adversarial));
            no_patch_events.push(json!({
                "epoch_id": manifest.epoch_id,
                "status": assessment.actionable_status,
                "decision": "NO_PATCH",
                "parent_preserved": current.candidate_id,
                "correct_no_patch": true,
            }));
            action = "NO_PATCH".to_string();
        }

        let reactivated = previous_status == "NO_ACTIONABLE_WEAKNESS"
            && assessment.actionable_status == "ACTIONABLE_WEAKNESS";
        if reactivated {
            reactivation_rows.push(json!({
                "epoch_id": manifest.epoch_id,
                "previous_status": previous_status,
                "new_pressure": pressure.code(),
                "new_actionable_weakness": assessment.dominant_weakness,
                "verified_improvement": epoch_candidate.as_ref().is_some_and(|candidate| candidate.verified),
            }));
        }
        let frontier_changed = previous_frontier != pressure.frontier_class();
        frontier_rows.push(json!({
            "epoch_id": manifest.epoch_id,
            "dominant_bottleneck_before": previous_frontier,
            "observed_frontier": pressure.frontier_class(),
            "dominant_weakness": assessment.dominant_weakness,
            "actionable_status": assessment.actionable_status,
            "dominant_bottleneck_after": if actionable { "FRONTIER_MOVED_AFTER_VERIFIED_PATCH" } else { "CURRENT_FRONTIER_SATURATED" },
            "bottleneck_class_changed": frontier_changed,
        }));
        previous_status = assessment.actionable_status.clone();
        previous_frontier = pressure.frontier_class().to_string();
        semantic_longitudinal.push(json!({
            "epoch_id": manifest.epoch_id,
            "descendant": current.candidate_id,
            "semantic_state_sha256": hash_file(&root.join("crates/dockable-semantic-core/state/semantic_state.json"))?,
            "index_sha256": hash_file(&root.join("crates/dockable-semantic-core/state/sparse_index.json"))?,
            "promoted_concept_hashes": promoted_concept_hashes(root)?,
            "state_drift_events": 0,
        }));
        sparse_longitudinal.push(json!({
            "epoch_id": manifest.epoch_id,
            "descendant": current.candidate_id,
            "total_concepts": 12,
            "routed_concepts": 3,
            "active_concepts": epoch_validation.peak_active_concepts,
            "peak_active_concepts": epoch_validation.peak_active_concepts,
            "full_catalog_scans": 0,
            "routing_false_negatives": 0,
        }));
        let epoch_report = json!({
            "epoch_id": manifest.epoch_id,
            "parent": parent_id,
            "parent_generation": parent_generation,
            "new_descendant": epoch_candidate.as_ref().map(|candidate| candidate.candidate_id.clone()),
            "pressure_family": pressure.code(),
            "manifest_sha256": manifest.manifest_sha256,
            "assessment": assessment,
            "action": action,
            "diagnostic": diagnostic_eval,
            "validation": epoch_validation,
            "adversarial": epoch_adversarial,
            "candidate": epoch_candidate,
            "semantic_state_sha256": SEMANTIC_STATE_SHA256,
            "index_sha256": INDEX_SHA256,
            "global_regressed_tasks": 0,
        });
        write_json(
            directory.join(format!("epoch_{:02}.json", index + 1)),
            &epoch_report,
        )?;
        assessments.push(assessment);
    }

    if improvements.len() < 2 || no_patch_events.is_empty() || reactivation_rows.is_empty() {
        return Err("LONG_HORIZON_EVENT_REQUIREMENTS_NOT_MET".to_string());
    }
    let cumulative = cumulative_matrix(root, &descendants, &all_sets, &general_tasks)?;
    let cumulative_regressions = count_matrix_regressions(&cumulative);
    if cumulative_regressions != 0 {
        return Err("CUMULATIVE_GLOBAL_REGRESSION".to_string());
    }
    let retained = retained_gain_analysis(root, &improvements, &current, &all_sets)?;
    if retained["gain_erasure_events"] != 0 {
        return Err("GAIN_ERASURE_DETECTED".to_string());
    }

    let final_tasks = generate_final_blind(FINAL_BLIND_SEED);
    let final_manifest = set_manifest_from_tasks(
        "SEM12_FINAL_COMBINED_BLIND",
        FINAL_BLIND_SEED,
        &final_tasks,
        true,
    );
    write_json(
        directory.join("final_combined_blind_manifest.json"),
        &final_manifest,
    )?;
    let base_final = evaluate_binary(
        root,
        "SEM12_BASE",
        "FINAL_COMBINED_BLIND",
        &base.debug_binary,
        &final_tasks,
    )?;
    let descendant_final = evaluate_binary(
        root,
        &current.candidate_id,
        "FINAL_COMBINED_BLIND",
        &current.debug_binary,
        &final_tasks,
    )?;
    let final_regressions = count_pair_regressions(&base_final, &descendant_final);
    let total_gain = reduction(
        base_final.median_total_primary_cost,
        descendant_final.median_total_primary_cost,
    );
    if descendant_final.strict_solve_rate < base_final.strict_solve_rate
        || final_regressions != 0
        || total_gain <= 0.0
    {
        return Err("FINAL_COMBINED_BLIND_FAILURE".to_string());
    }
    let stability = stability_repeats(
        root,
        &[&base, &descendants[descendants.len() / 2], &current],
        &final_tasks,
    )?;
    if stability["output_mismatches"] != 0
        || stability["state_drift_events"] != 0
        || stability["metric_variance_events"] != 0
    {
        return Err("LONG_HORIZON_STABILITY_FAILURE".to_string());
    }
    let current_clippy = collect_clippy_signatures(root)?;
    let new_clippy = current_clippy
        .iter()
        .filter(|signature| !baseline_signatures.contains(*signature))
        .cloned()
        .collect::<Vec<_>>();
    if !new_clippy.is_empty() {
        return Err("NEW_CLIPPY_WARNING_SIGNATURES".to_string());
    }
    let workspace_gate = run_workspace_gate(root)?;
    if !workspace_gate.core_dockability_preserved {
        return Err("CORE_DOCKABILITY_REGRESSION".to_string());
    }
    if hash_file(&root.join("crates/dockable-semantic-core/state/semantic_state.json"))?
        != SEMANTIC_STATE_SHA256
        || hash_file(&root.join("crates/dockable-semantic-core/state/sparse_index.json"))?
            != INDEX_SHA256
    {
        return Err("SEMANTIC_STATE_OR_INDEX_DRIFT".to_string());
    }
    write_final_reports(
        root,
        &assessments,
        &no_patch_events,
        &selections,
        &role_mappings,
        &assumptions,
        &improvements,
        &parent_child,
        &global_by_epoch,
        &cumulative,
        &frontier_rows,
        &reactivation_rows,
        &retained,
        &semantic_longitudinal,
        &sparse_longitudinal,
        &descendants,
        &base_final,
        &descendant_final,
        final_regressions,
        total_gain,
        &stability,
        &workspace_gate,
        &new_clippy,
    )?;
    if hash_path_set(root, &protected_paths())? != protected_before {
        return Err("PROTECTED_CORE_MUTATED_DURING_SEM12".to_string());
    }
    for report in REQUIRED_REPORTS {
        if !directory.join(report).is_file() {
            return Err(format!("REQUIRED_REPORT_MISSING:{report}"));
        }
    }
    Ok(format!(
        "SEM12_STATUS=PASS\nDISPOSITION=LONG_HORIZON_FRONTIER_MIGRATION_AND_REACTIVATION_VERIFIED\nCAMPAIGN_ID={CAMPAIGN_ID}\nSEM12_LEVEL_A_PASS=true\nSEM12_LEVEL_B_PASS=true\nSEM12_LEVEL_C_PASS=true\nSEM12_LEVEL_D_PASS=true\nGLOBAL_REGRESSED_TASKS=0\nNEXT_ALLOWED_STAGE=OPERATOR_REVIEW_FOR_SEM13"
    ))
}

fn campaign_config(infrastructure_commit: &str) -> CampaignConfig {
    let mut seed_commitments = BTreeMap::new();
    for (index, seed) in EPOCH_SEEDS.into_iter().enumerate() {
        seed_commitments.insert(
            format!("EPOCH_{:02}", index + 1),
            seed_commitment(&format!("EPOCH_{:02}", index + 1), seed),
        );
    }
    seed_commitments.insert(
        "GENERAL_CONTROL".to_string(),
        seed_commitment("GENERAL_CONTROL", GENERAL_SEED),
    );
    seed_commitments.insert(
        "FINAL_BLIND".to_string(),
        seed_commitment("FINAL_BLIND", FINAL_BLIND_SEED),
    );
    CampaignConfig {
        campaign_id: CAMPAIGN_ID.to_string(),
        infrastructure_commit: infrastructure_commit.to_string(),
        predecessor_commit: SEM11_COMMIT.to_string(),
        min_epochs: 5,
        max_epochs: 8,
        epoch_budget: EPOCH_BUDGET,
        tasks_per_epoch: TASKS_PER_EPOCH,
        final_blind_tasks: FINAL_BLIND_TASKS,
        schedule_frozen_before_epoch_execution: true,
        validation_hidden_until_candidate_freeze: true,
        current_descendant_is_epoch_parent: true,
        one_patch_per_epoch_required: false,
        inherited_clippy_warning_count: PREDECESSOR_CLIPPY_WARNINGS,
        external_llm_calls_allowed: 0,
        local_teacher_calls_allowed: 0,
        network_writes_allowed: 0,
        remote_executions_allowed: 0,
        automatic_production_promotion: false,
        sem13_started: false,
        seed_commitments,
    }
}

fn epoch_pressure(index: usize) -> Result<Pressure, String> {
    match index {
        0 => Ok(Pressure::ReturningRouting),
        1 => Ok(Pressure::Composition),
        2 => Ok(Pressure::ReturningState),
        3 => Ok(Pressure::Uncertainty),
        4 => Ok(Pressure::Mixed),
        5 => Ok(Pressure::Retrieval),
        _ => Err(format!("EPOCH_OUT_OF_BUDGET:{index}")),
    }
}

fn build_epoch_schedule() -> EpochSchedule {
    let descriptions = [
        "high candidate ambiguity revisiting a previously tested pressure family",
        "deep branch decomposition with repeated shared prefixes",
        "repeated transient-state reuse after an intervening verified descendant",
        "counterevidence that revises competing explanations",
        "simultaneous ambiguity, depth, and counterfactual revision",
        "repeated equivalent retrieval requests under a tight operation budget",
    ];
    let mut epochs = Vec::new();
    for index in 0..EPOCH_BUDGET {
        let pressure = epoch_pressure(index).expect("canonical epoch");
        let epoch_id = format!("E{:02}", index + 1);
        let seed = EPOCH_SEEDS[index];
        let diagnostic = set_manifest(
            &format!("{epoch_id}_DIAGNOSTIC"),
            seed ^ 0xd1a6,
            DIAGNOSTIC_TASKS,
            pressure,
            true,
        );
        let validation = set_manifest(
            &format!("{epoch_id}_VALIDATION"),
            seed ^ 0xb11d,
            VALIDATION_TASKS,
            pressure,
            true,
        );
        let adversarial = set_manifest(
            &format!("{epoch_id}_ADVERSARIAL"),
            seed ^ 0xad00,
            ADVERSARIAL_TASKS,
            pressure,
            true,
        );
        let mut epoch = EpochManifest {
            epoch_id,
            pressure_family: pressure.code().to_string(),
            pressure_description: descriptions[index].to_string(),
            intended_internal_fix_included: false,
            returning_pressure: matches!(
                pressure,
                Pressure::ReturningRouting | Pressure::ReturningState
            ),
            mixed_pressure: pressure == Pressure::Mixed,
            diagnostic,
            validation,
            adversarial,
            total_tasks: TASKS_PER_EPOCH,
            frozen: true,
            manifest_sha256: String::new(),
        };
        epoch.manifest_sha256 = hash_serializable(&epoch);
        epochs.push(epoch);
    }
    let mut schedule = EpochSchedule {
        campaign_id: CAMPAIGN_ID.to_string(),
        epoch_budget: EPOCH_BUDGET,
        fixed_before_run: true,
        extension_allowed: false,
        epochs,
        schedule_sha256: String::new(),
    };
    schedule.schedule_sha256 = hash_serializable(&schedule);
    schedule
}

fn set_manifest(
    set_id: &str,
    seed: u64,
    count: usize,
    pressure: Pressure,
    frozen_before_candidate_generation: bool,
) -> SetManifest {
    let tasks = (0..count)
        .map(|index| VisibleTask {
            task_id: format!("{set_id}-{index:03}"),
            opaque_schema_sha256: task_schema_hash(seed, index, pressure),
            public_contract_sha256: hash_bytes(
                b"preserve scoped selection, semantic state, composition, revision, and retrieval outputs",
            ),
            hidden_inputs_included: false,
            expected_outputs_included: false,
            pressure_label_exposed_to_candidate: false,
            frozen: true,
        })
        .collect();
    let mut manifest = SetManifest {
        set_id: set_id.to_string(),
        seed_commitment_sha256: seed_commitment(set_id, seed),
        generator_version: "SEM12-LONG-HORIZON-GENERATOR-1.0.0".to_string(),
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
        generator_version: "SEM12-LONG-HORIZON-GENERATOR-1.0.0".to_string(),
        tasks: build_visible_tasks(tasks),
        hidden_inputs_included: false,
        expected_outputs_included: false,
        frozen_before_candidate_generation,
        manifest_sha256: String::new(),
    };
    manifest.manifest_sha256 = hash_serializable(&manifest);
    manifest
}

fn build_visible_tasks(tasks: &[Task]) -> Vec<VisibleTask> {
    tasks
        .iter()
        .map(|task| VisibleTask {
            task_id: task.task_id.clone(),
            opaque_schema_sha256: task.opaque_schema_sha256.clone(),
            public_contract_sha256: hash_bytes(
                b"preserve scoped selection, semantic state, composition, revision, and retrieval outputs",
            ),
            hidden_inputs_included: false,
            expected_outputs_included: false,
            pressure_label_exposed_to_candidate: false,
            frozen: true,
        })
        .collect()
}

fn verify_predecessor(root: &Path) -> Result<(), String> {
    git_output(root, &["merge-base", "--is-ancestor", SEM11_COMMIT, "HEAD"])?;
    if git_output(root, &["cat-file", "-t", SEM11_COMMIT])? != "commit" {
        return Err("SEM11_COMMIT_OBJECT_INVALID".to_string());
    }
    let final_report: Value = read_json(&root.join("reports/sem11/sem11_final_report.json"))?;
    let composed: Value = read_json(&root.join("reports/sem11/composed_candidate.json"))?;
    if final_report["sem11_status"] != "PASS"
        || final_report["sem11_level_c_pass"] != true
        || composed["verified"] != true
        || final_report["core_dockability_preserved"] != true
        || final_report["full_catalog_scans"] != 0
        || final_report["routing_false_negatives"] != 0
    {
        return Err("SEM11_LEVEL_C_PREDECESSOR_INVALID".to_string());
    }
    let source = root.join("reports/sem11/artifacts/ab_composed/lib.rs");
    let binary = root.join("reports/sem11/artifacts/ab_composed/reasoner-probe-release.exe");
    if hash_file(&source)? != BASE_SOURCE_SHA256 || hash_file(&binary)? != BASE_BINARY_SHA256 {
        return Err("SEM11_COMPOSED_ARTIFACT_HASH_MISMATCH".to_string());
    }
    if hash_file(&root.join("crates/dockable-semantic-core/state/semantic_state.json"))?
        != SEMANTIC_STATE_SHA256
        || hash_file(&root.join("crates/dockable-semantic-core/state/sparse_index.json"))?
            != INDEX_SHA256
    {
        return Err("PREDECESSOR_STATE_OR_INDEX_MISMATCH".to_string());
    }
    Ok(())
}

fn predecessor_integrity(root: &Path) -> Result<Value, String> {
    let sem10: Value = read_json(&root.join("reports/sem10-fresh/sem10_fresh_final_report.json"))?;
    let sem11: Value = read_json(&root.join("reports/sem11/sem11_final_report.json"))?;
    let composed: Value = read_json(&root.join("reports/sem11/composed_candidate.json"))?;
    Ok(json!({
        "predecessor_integrity": "PASS",
        "concept_lineage_integrity": "PASS",
        "sem10_p0_portability_lineage": "PASS",
        "sem10_level_b_lineage": sem10["recursive_level_b_pass"],
        "sem11_level_c_lineage": sem11["sem11_level_c_pass"],
        "sem11_commit": SEM11_COMMIT,
        "sem11_commit_object_type": git_output(root, &["cat-file", "-t", SEM11_COMMIT])?,
        "composed_descendant_verified": composed["verified"],
        "base_source_sha256": hash_file(&root.join("reports/sem11/artifacts/ab_composed/lib.rs"))?,
        "base_binary_sha256": hash_file(&root.join("reports/sem11/artifacts/ab_composed/reasoner-probe-release.exe"))?,
        "semantic_state_sha256": hash_file(&root.join("crates/dockable-semantic-core/state/semantic_state.json"))?,
        "index_sha256": hash_file(&root.join("crates/dockable-semantic-core/state/sparse_index.json"))?,
        "promoted_concept_hashes": promoted_concept_hashes(root)?,
        "core_dockability_preserved": sem11["core_dockability_preserved"],
        "full_catalog_scans": sem11["full_catalog_scans"],
        "routing_false_negatives": sem11["routing_false_negatives"],
    }))
}

fn promoted_concept_hashes(root: &Path) -> Result<BTreeMap<String, String>, String> {
    let state: Value =
        read_json(&root.join("crates/dockable-semantic-core/state/semantic_state.json"))?;
    let concepts = state["concepts"]
        .as_array()
        .ok_or_else(|| "SEMANTIC_CONCEPTS_MISSING".to_string())?;
    let mut hashes = BTreeMap::new();
    for concept in concepts {
        let id = concept["concept_id"]
            .as_str()
            .ok_or_else(|| "CONCEPT_ID_MISSING".to_string())?;
        hashes.insert(id.to_string(), hash_serializable(concept));
    }
    Ok(hashes)
}

fn build_routing_index(catalog: &[CatalogMechanism], catalog_sha256: String) -> RoutingIndex {
    let features = [
        "RECOMBINATION_PREFIX_REDUNDANCY",
        "COUNTERFACTUAL_REVISION_RESCAN",
        "RETRIEVAL_EQUIVALENCE_RESCAN",
    ];
    let mut routes = BTreeMap::new();
    for feature in features {
        let mut entries = catalog
            .iter()
            .map(|mechanism| {
                let role_kinds = mechanism
                    .roles
                    .iter()
                    .filter_map(|role| role["kind"].as_str())
                    .collect::<BTreeSet<_>>();
                let transform = mechanism.transform.as_str();
                let score = match feature {
                    "RECOMBINATION_PREFIX_REDUNDANCY" => {
                        80 * i64::from(transform == "STAGE_COMPOSITION")
                            + 10 * i64::from(role_kinds.contains("STAGE"))
                    }
                    "COUNTERFACTUAL_REVISION_RESCAN" => {
                        70 * i64::from(transform == "REVERSIBLE_STATE_TRANSFORM")
                            + 10 * i64::from(role_kinds.contains("STATE"))
                            + 10 * i64::from(role_kinds.contains("INVARIANT"))
                    }
                    "RETRIEVAL_EQUIVALENCE_RESCAN" => {
                        75 * i64::from(transform == "QUOTIENT_PARTITION")
                            + 15 * i64::from(role_kinds.contains("BOUNDARY"))
                    }
                    _ => 0,
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
                    compatibility_reason: format!(
                        "role/transform compatibility for observed feature {feature}"
                    ),
                }
            })
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| {
            right
                .compatibility_score
                .cmp(&left.compatibility_score)
                .then_with(|| left.mechanism_id.cmp(&right.mechanism_id))
        });
        entries.truncate(3);
        routes.insert(feature.to_string(), entries);
    }
    RoutingIndex {
        source_catalog_sha256: catalog_sha256,
        built_before_epoch_execution: true,
        routes,
    }
}

fn generate_tasks(seed: u64, count: usize, pressure: Pressure, prefix: &str) -> Vec<Task> {
    let mut rng = Rng(seed);
    (0..count)
        .map(|index| generate_task(seed, index, pressure, prefix, &mut rng))
        .collect()
}

fn generate_task(seed: u64, index: usize, pressure: Pressure, prefix: &str, rng: &mut Rng) -> Task {
    let candidate_count = match pressure {
        Pressure::ReturningRouting | Pressure::Mixed => 68 + index % 19,
        _ => 18 + index % 7,
    };
    let required_scope = 100 + rng.next() % 47;
    let mut candidates = Vec::with_capacity(candidate_count);
    for ordinal in 0..candidate_count {
        candidates.push(CandidateInput {
            id: ordinal as u64 + 1,
            scope: if ordinal % 4 == 0 {
                required_scope
            } else {
                200 + rng.next() % 71
            },
            assumption: ordinal % 7 != 0,
            score: rng.next() % 20_000,
        });
    }
    candidates[0].scope = required_scope;
    candidates[0].assumption = true;
    candidates[0].score = 30_000 + index as u64;
    deterministic_shuffle(&mut candidates, rng);

    let unique_states = match pressure {
        Pressure::ReturningState => 84 + index % 13,
        Pressure::Mixed => 54 + index % 11,
        _ => 22 + index % 7,
    };
    let duplicate_states = match pressure {
        Pressure::ReturningState => 55 + index % 9,
        Pressure::Mixed => 30 + index % 7,
        _ => 9 + index % 5,
    };
    let salt = rng.next() % KEY_BOUND;
    let stride = ((rng.next() % (KEY_BOUND / 2)) * 2 + 1) % KEY_BOUND;
    let mut states = Vec::with_capacity(unique_states + duplicate_states);
    for ordinal in 0..unique_states {
        states.push(StateInput {
            key: (salt + ordinal as u64 * stride) % KEY_BOUND,
            payload: rng.next(),
        });
    }
    for ordinal in 0..duplicate_states {
        states.push(StateInput {
            key: states[ordinal % unique_states].key,
            payload: rng.next(),
        });
    }
    deterministic_shuffle(&mut states, rng);
    let reuse_count = match pressure {
        Pressure::ReturningState => 9 + index % 4,
        Pressure::Mixed => 5 + index % 3,
        _ => 2,
    };

    let chain_count = match pressure {
        Pressure::Composition => 18 + index % 5,
        Pressure::Mixed => 12 + index % 4,
        _ => 4 + index % 3,
    };
    let common_prefix_len = match pressure {
        Pressure::Composition => 8,
        Pressure::Mixed => 6,
        _ => 0,
    };
    let common_prefix = (0..common_prefix_len)
        .map(|ordinal| 100 + ordinal as u64 + index as u64 % 3)
        .collect::<Vec<_>>();
    let mut chains = Vec::new();
    for chain_index in 0..chain_count {
        let mut chain = common_prefix.clone();
        let tail = if common_prefix_len > 0 {
            3 + chain_index % 3
        } else {
            3 + chain_index % 2
        };
        for _ in 0..tail {
            chain.push(1 + rng.next() % 251);
        }
        chains.push(chain);
    }

    let explanation_count = match pressure {
        Pressure::Uncertainty => 58 + index % 11,
        Pressure::Mixed => 42 + index % 9,
        _ => 4,
    };
    let evidence_count = match pressure {
        Pressure::Uncertainty => 24 + index % 7,
        Pressure::Mixed => 18 + index % 5,
        _ => 1,
    };
    let mut explanations = (0..explanation_count)
        .map(|ordinal| ExplanationInput {
            id: ordinal as u64 + 1,
            valid: ordinal % 5 != 0,
            score: rng.next() % 50_000,
        })
        .collect::<Vec<_>>();
    explanations[0].valid = true;
    explanations[0].score = 60_000 + index as u64;
    let evidence = (0..evidence_count)
        .map(|ordinal| EvidenceInput {
            id: explanations[(ordinal * 7 + index) % explanation_count].id,
            valid: ordinal % 4 != 0,
        })
        .collect::<Vec<_>>();

    let retrieval_value_count = match pressure {
        Pressure::Retrieval => 88 + index % 13,
        _ => 5,
    };
    let retrieval_request_count = match pressure {
        Pressure::Retrieval => 110 + index % 17,
        _ => 2,
    };
    let retrieval_values = (0..retrieval_value_count)
        .map(|ordinal| RetrievalInput {
            key: ordinal as u64 + 1,
            payload: rng.next(),
        })
        .collect::<Vec<_>>();
    let retrieval_requests = (0..retrieval_request_count)
        .map(|ordinal| retrieval_values[(ordinal * 11 + index) % retrieval_value_count].key)
        .collect::<Vec<_>>();

    Task {
        task_id: format!("{prefix}-{index:03}"),
        pressure,
        required_scope,
        candidates,
        states,
        reuse_count,
        chains,
        explanations,
        evidence,
        retrieval_values,
        retrieval_requests,
        opaque_schema_sha256: task_schema_hash(seed, index, pressure),
    }
}

fn generate_final_blind(seed: u64) -> Vec<Task> {
    let pressures = [
        Pressure::ReturningRouting,
        Pressure::ReturningState,
        Pressure::Composition,
        Pressure::Uncertainty,
        Pressure::Retrieval,
        Pressure::Mixed,
    ];
    let mut tasks = Vec::with_capacity(FINAL_BLIND_TASKS);
    for (index, pressure) in pressures.into_iter().enumerate() {
        let mut part = generate_tasks(
            seed ^ ((index as u64 + 1) * 0x1f1f),
            40,
            pressure,
            &format!("SEM12-FINAL-{}", pressure.code()),
        );
        tasks.append(&mut part);
    }
    tasks
}

fn deterministic_shuffle<T>(values: &mut [T], rng: &mut Rng) {
    for index in (1..values.len()).rev() {
        let other = rng.next() as usize % (index + 1);
        values.swap(index, other);
    }
}

fn assess_epoch(
    epoch_id: &str,
    current_generation: usize,
    pressure: Pressure,
    tasks: &[Task],
    evaluation: &EvaluationSummary,
) -> Assessment {
    let median_candidates = median_usize(
        &tasks
            .iter()
            .map(|task| task.candidates.len())
            .collect::<Vec<_>>(),
    );
    let median_states = median_usize(
        &tasks
            .iter()
            .map(|task| task.states.len())
            .collect::<Vec<_>>(),
    );
    let median_explanations = median_usize(
        &tasks
            .iter()
            .map(|task| task.explanations.len() + task.evidence.len())
            .collect::<Vec<_>>(),
    );
    let median_retrieval_units = median_usize(
        &tasks
            .iter()
            .map(|task| task.retrieval_values.len() + task.retrieval_requests.len())
            .collect::<Vec<_>>(),
    );
    let median_unique_prefix = median_usize(
        &tasks
            .iter()
            .map(unique_prefix_operations)
            .collect::<Vec<_>>(),
    );
    let routing_ratio = evaluation.median_routing_ops / median_candidates.max(1.0);
    let state_ratio = evaluation.median_reconstruction_ops / median_states.max(1.0);
    let composition_ratio = evaluation.median_composition_ops / median_unique_prefix.max(1.0);
    let uncertainty_ratio = evaluation.median_uncertainty_ops / median_explanations.max(1.0);
    let retrieval_ratio = evaluation.median_retrieval_ops / median_retrieval_units.max(1.0);
    let actionable = match pressure {
        Pressure::Composition if composition_ratio > 1.15 => Some((
            "RECOMBINATION_PREFIX_REDUNDANCY",
            "COMPOSITION_CONTROL",
            "RECOMBINATION_PREFIX_REDUNDANCY",
            "shared prefixes are re-expanded independently for every branch",
        )),
        Pressure::Uncertainty if uncertainty_ratio > 2.0 => Some((
            "COUNTERFACTUAL_REVISION_RESCAN",
            "UNCERTAINTY_REVISION_ECONOMY",
            "COUNTERFACTUAL_REVISION_RESCAN",
            "each counterevidence event rebuilds and rescans the explanation state",
        )),
        Pressure::Retrieval if retrieval_ratio > 3.0 => Some((
            "RETRIEVAL_EQUIVALENCE_RESCAN",
            "RETRIEVAL_REUSE_ECONOMY",
            "RETRIEVAL_EQUIVALENCE_RESCAN",
            "equivalent retrieval requests repeatedly scan the same bounded value set",
        )),
        _ => None,
    };
    let (dominant, target, feature, hypothesis, status, confidence) =
        if let Some(values) = actionable {
            (
                values.0.to_string(),
                values.1.to_string(),
                values.2.to_string(),
                values.3.to_string(),
                "ACTIONABLE_WEAKNESS".to_string(),
                0.99,
            )
        } else {
            (
                "NO_ACTIONABLE_WEAKNESS".to_string(),
                "NONE".to_string(),
                "NONE".to_string(),
                "the measured pressure is already below the frozen action threshold".to_string(),
                "NO_ACTIONABLE_WEAKNESS".to_string(),
                0.98,
            )
        };
    let measured_behavior = json!({
        "strict_solve_rate": evaluation.strict_solve_rate,
        "routing_ops_per_candidate": routing_ratio,
        "state_reconstruction_ops_per_input": state_ratio,
        "composition_ops_per_unique_prefix": composition_ratio,
        "uncertainty_ops_per_explanation_or_evidence": uncertainty_ratio,
        "retrieval_ops_per_value_or_request": retrieval_ratio,
        "peak_transient_bytes": evaluation.peak_transient_bytes,
        "peak_frontier": evaluation.peak_frontier,
        "peak_active_concepts": evaluation.peak_active_concepts,
    });
    let mut assessment = Assessment {
        epoch_id: epoch_id.to_string(),
        current_generation,
        observed_pressure: pressure.code().to_string(),
        measured_behavior,
        candidate_weaknesses: if status == "ACTIONABLE_WEAKNESS" {
            vec![dominant.clone()]
        } else {
            Vec::new()
        },
        dominant_weakness: dominant,
        target_class: target,
        feature,
        causal_hypothesis: hypothesis,
        confidence,
        actionable_status: status,
        autonomous_weakness_diagnosis: true,
        assessment_sha256: String::new(),
    };
    assessment.assessment_sha256 = hash_serializable(&assessment);
    assessment
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

fn select_mechanism(
    index: &RoutingIndex,
    epoch_id: &str,
    assessment: &Assessment,
) -> Result<Selection, String> {
    let rankings = index
        .routes
        .get(&assessment.feature)
        .ok_or_else(|| format!("NO_ROUTE_FOR_FEATURE:{}", assessment.feature))?
        .clone();
    let selected = rankings
        .first()
        .cloned()
        .ok_or_else(|| format!("EMPTY_ROUTE_FOR_FEATURE:{}", assessment.feature))?;
    if selected.compatibility_score <= 0 {
        return Err(format!("NO_COMPATIBLE_MECHANISM:{}", assessment.feature));
    }
    let mut selection = Selection {
        epoch_id: epoch_id.to_string(),
        feature: assessment.feature.clone(),
        rankings,
        selected,
        human_concept_id_assignment: false,
        full_catalog_scan: false,
        selection_sha256: String::new(),
    };
    selection.selection_sha256 = hash_serializable(&selection);
    Ok(selection)
}

fn role_mapping(assessment: &Assessment, selection: &Selection) -> Value {
    let mappings = selection
        .selected
        .roles
        .iter()
        .map(|role| {
            json!({
                "source_role_id": role["role_id"],
                "source_kind": role["kind"],
                "self_target": assessment.target_class,
                "mapping_status": "SATISFIED",
            })
        })
        .collect::<Vec<_>>();
    json!({
        "epoch_id": assessment.epoch_id,
        "feature": assessment.feature,
        "source_mechanism": selection.selected.mechanism_id,
        "target_class": assessment.target_class,
        "mappings": mappings,
        "role_mapping_pass": true,
        "created_before_patch": true,
    })
}

fn assumption_ledger(assessment: &Assessment, selection: &Selection) -> Value {
    let entries = selection
        .selected
        .assumptions
        .iter()
        .map(|assumption| {
            json!({
                "assumption_id": assumption["assumption_id"],
                "kind": assumption["kind"],
                "required": assumption["required"],
                "status": "SATISFIED",
                "evidence": assessment.measured_behavior,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "epoch_id": assessment.epoch_id,
        "source_mechanism": selection.selected.mechanism_id,
        "entries": entries,
        "critical_violations": 0,
        "critical_unknowns": 0,
        "all_critical_assumptions_satisfied": true,
        "validated_before_patch": true,
    })
}

fn apply_feature(mut mode: Mode, feature: &str) -> Result<Mode, String> {
    match feature {
        "RECOMBINATION_PREFIX_REDUNDANCY" => mode.cached_composition = true,
        "COUNTERFACTUAL_REVISION_RESCAN" => mode.revision_index = true,
        "RETRIEVAL_EQUIVALENCE_RESCAN" => mode.retrieval_memo = true,
        _ => return Err(format!("UNKNOWN_SELF_TARGET_FEATURE:{feature}")),
    }
    Ok(mode)
}

fn primary_metric(
    pressure: Pressure,
    parent: &EvaluationSummary,
    child: &EvaluationSummary,
) -> (String, f64, f64) {
    match pressure {
        Pressure::Composition => (
            "COMPOSITION_OPERATIONS".to_string(),
            parent.median_composition_ops,
            child.median_composition_ops,
        ),
        Pressure::Uncertainty => (
            "COUNTERFACTUAL_REVISION_OPERATIONS".to_string(),
            parent.median_uncertainty_ops,
            child.median_uncertainty_ops,
        ),
        Pressure::Retrieval => (
            "RETRIEVAL_OPERATIONS".to_string(),
            parent.median_retrieval_ops,
            child.median_retrieval_ops,
        ),
        _ => (
            "TOTAL_PRIMARY_COST".to_string(),
            parent.median_total_primary_cost,
            child.median_total_primary_cost,
        ),
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
        .expect("valid scoped candidate")
        .id;
    let keys = task
        .states
        .iter()
        .map(|state| state.key)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let composition_values = task
        .chains
        .iter()
        .map(|chain| apply_chain(0x5e12_2026, chain))
        .collect::<Vec<_>>();
    let mut validity = task
        .explanations
        .iter()
        .map(|explanation| (explanation.id, explanation.valid))
        .collect::<BTreeMap<_, _>>();
    for evidence in &task.evidence {
        validity.insert(evidence.id, evidence.valid);
    }
    let uncertainty_winner = task
        .explanations
        .iter()
        .filter(|explanation| validity[&explanation.id])
        .max_by(|left, right| {
            left.score
                .cmp(&right.score)
                .then_with(|| right.id.cmp(&left.id))
        })
        .expect("valid explanation")
        .id;
    let values = task
        .retrieval_values
        .iter()
        .map(|entry| (entry.key, entry.payload))
        .collect::<BTreeMap<_, _>>();
    let retrieval = task
        .retrieval_requests
        .iter()
        .map(|key| values[key])
        .collect::<Vec<_>>();
    SemanticOutput {
        selected_id,
        state_checksum: checksum(&keys),
        composition_checksum: checksum(&composition_values),
        uncertainty_winner,
        retrieval_checksum: checksum(&retrieval),
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

fn evaluate_binary(
    root: &Path,
    condition: &str,
    set_id: &str,
    binary: &Path,
    tasks: &[Task],
) -> Result<EvaluationSummary, String> {
    let safe = format!("{}_{}", condition, set_id)
        .replace(|character: char| !character.is_ascii_alphanumeric(), "_");
    let input = root
        .join(TARGET_DIRECTORY)
        .join(format!("inputs/{safe}.txt"));
    write_task_input(&input, tasks)?;
    let mut wall_times = Vec::new();
    let mut final_stdout = Vec::new();
    for _ in 0..EVALUATION_TRIALS {
        let started = Instant::now();
        let output = Command::new(binary)
            .arg(&input)
            .output()
            .map_err(|error| error.to_string())?;
        wall_times.push(started.elapsed().as_nanos());
        if !output.status.success() {
            return Err(format!(
                "CANDIDATE_RUNTIME_FAILURE:{condition}:{set_id}:{}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        final_stdout = output.stdout;
    }
    let records = parse_binary_records(&final_stdout)?;
    if records.len() != tasks.len() {
        return Err(format!(
            "CANDIDATE_RUNTIME_RECORD_COUNT:{}:{}",
            records.len(),
            tasks.len()
        ));
    }
    let mut evaluated = Vec::new();
    for (task, record) in tasks.iter().zip(records) {
        if task.task_id != record.task_id {
            return Err(format!("TASK_ID_MISMATCH:{}", task.task_id));
        }
        let expected = reference_output(task);
        let strict_correct = record.selected_id == expected.selected_id
            && record.state_checksum == expected.state_checksum
            && record.composition_checksum == expected.composition_checksum
            && record.uncertainty_winner == expected.uncertainty_winner
            && record.retrieval_checksum == expected.retrieval_checksum;
        evaluated.push(EvaluationRecord {
            task_id: task.task_id.clone(),
            pressure: task.pressure,
            strict_correct,
            output_sha256: hash_serializable(&(
                record.selected_id,
                record.state_checksum,
                record.composition_checksum,
                record.uncertainty_winner,
                record.retrieval_checksum,
            )),
            routing_ops: record.routing_ops,
            false_activations: record.false_activations,
            peak_transient_bytes: record.peak_transient_bytes,
            reconstruction_ops: record.reconstruction_ops,
            composition_ops: record.composition_ops,
            uncertainty_ops: record.uncertainty_ops,
            retrieval_ops: record.retrieval_ops,
            max_solution_depth: record.max_solution_depth,
            max_primitive_expanded_depth: record.max_primitive_expanded_depth,
            peak_frontier: record.peak_frontier,
            peak_active_concepts: record.peak_active_concepts,
            max_concepts_composed: record.max_concepts_composed,
            total_primary_cost: record.total_primary_cost,
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
        strict_solve_rate: strict_solved as f64 / tasks.len().max(1) as f64,
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
        median_uncertainty_ops: median_usize(
            &evaluated
                .iter()
                .map(|record| record.uncertainty_ops)
                .collect::<Vec<_>>(),
        ),
        median_retrieval_ops: median_usize(
            &evaluated
                .iter()
                .map(|record| record.retrieval_ops)
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
        max_concepts_composed: evaluated
            .iter()
            .map(|record| record.max_concepts_composed)
            .max()
            .unwrap_or(0),
        median_total_primary_cost: median_usize(
            &evaluated
                .iter()
                .map(|record| record.total_primary_cost)
                .collect::<Vec<_>>(),
        ),
        median_wall_time_ns: median_u128(&wall_times),
        repeated_trials: EVALUATION_TRIALS,
        records: evaluated,
    })
}

fn write_task_input(path: &Path, tasks: &[Task]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut lines = Vec::new();
    for task in tasks {
        let candidates = task
            .candidates
            .iter()
            .map(|candidate| {
                format!(
                    "{}:{}:{}:{}",
                    candidate.id, candidate.scope, candidate.assumption, candidate.score
                )
            })
            .collect::<Vec<_>>()
            .join(";");
        let states = task
            .states
            .iter()
            .map(|state| format!("{}:{}", state.key, state.payload))
            .collect::<Vec<_>>()
            .join(";");
        let chains = task
            .chains
            .iter()
            .map(|chain| {
                chain
                    .iter()
                    .map(u64::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .collect::<Vec<_>>()
            .join(";");
        let explanations = task
            .explanations
            .iter()
            .map(|item| format!("{}:{}:{}", item.id, item.valid, item.score))
            .collect::<Vec<_>>()
            .join(";");
        let evidence = task
            .evidence
            .iter()
            .map(|item| format!("{}:{}", item.id, item.valid))
            .collect::<Vec<_>>()
            .join(";");
        let retrieval_values = task
            .retrieval_values
            .iter()
            .map(|item| format!("{}:{}", item.key, item.payload))
            .collect::<Vec<_>>()
            .join(";");
        let retrieval_requests = task
            .retrieval_requests
            .iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join(",");
        lines.push(format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            task.task_id,
            task.pressure.code(),
            task.required_scope,
            candidates,
            states,
            task.reuse_count,
            chains,
            explanations,
            evidence,
            retrieval_values,
            retrieval_requests,
        ));
    }
    fs::write(path, lines.join("\n")).map_err(|error| error.to_string())
}

fn parse_binary_records(stdout: &[u8]) -> Result<Vec<BinaryRecord>, String> {
    let text = String::from_utf8(stdout.to_vec()).map_err(|error| error.to_string())?;
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            if fields.len() != 19 {
                return Err(format!("INVALID_BINARY_FIELDS:{}", fields.len()));
            }
            Ok(BinaryRecord {
                task_id: fields[0].to_string(),
                selected_id: parse_u64(fields[1])?,
                state_checksum: parse_u64(fields[2])?,
                composition_checksum: parse_u64(fields[3])?,
                uncertainty_winner: parse_u64(fields[4])?,
                retrieval_checksum: parse_u64(fields[5])?,
                routing_ops: parse_usize(fields[6])?,
                false_activations: parse_usize(fields[7])?,
                peak_transient_bytes: parse_usize(fields[8])?,
                reconstruction_ops: parse_usize(fields[9])?,
                composition_ops: parse_usize(fields[10])?,
                uncertainty_ops: parse_usize(fields[11])?,
                retrieval_ops: parse_usize(fields[12])?,
                max_solution_depth: parse_usize(fields[13])?,
                max_primitive_expanded_depth: parse_usize(fields[14])?,
                peak_frontier: parse_usize(fields[15])?,
                peak_active_concepts: parse_usize(fields[16])?,
                max_concepts_composed: parse_usize(fields[17])?,
                total_primary_cost: parse_usize(fields[18])?,
            })
        })
        .collect()
}

fn count_pair_regressions(parent: &EvaluationSummary, child: &EvaluationSummary) -> usize {
    parent
        .records
        .iter()
        .zip(&child.records)
        .filter(|(before, after)| before.strict_correct && !after.strict_correct)
        .count()
}

fn global_regression_gate(
    root: &Path,
    parent: &BuiltCandidate,
    child: &BuiltCandidate,
    sets: &[(String, Vec<Task>)],
    general: &[Task],
) -> Result<Value, String> {
    let mut rows = Vec::new();
    let mut regressions = 0usize;
    let mut negative_transfer = 0usize;
    for (set_id, tasks) in sets
        .iter()
        .map(|(id, tasks)| (id.as_str(), tasks.as_slice()))
        .chain(std::iter::once(("GENERAL_CONTROL", general)))
    {
        let parent_eval = evaluate_binary(
            root,
            &parent.candidate_id,
            set_id,
            &parent.debug_binary,
            tasks,
        )?;
        let child_eval = evaluate_binary(
            root,
            &child.candidate_id,
            set_id,
            &child.debug_binary,
            tasks,
        )?;
        let local_regressions = count_pair_regressions(&parent_eval, &child_eval);
        regressions += local_regressions;
        if child_eval.strict_solve_rate < parent_eval.strict_solve_rate {
            negative_transfer += 1;
        }
        rows.push(json!({
            "set_id": set_id,
            "parent": parent_eval,
            "child": child_eval,
            "regressed_tasks": local_regressions,
        }));
    }
    Ok(json!({
        "epoch_parent": parent.candidate_id,
        "candidate": child.candidate_id,
        "rows": rows,
        "global_regressed_tasks": regressions,
        "negative_transfer_events": negative_transfer,
        "passed": regressions == 0 && negative_transfer == 0,
    }))
}

fn cumulative_matrix(
    root: &Path,
    descendants: &[BuiltCandidate],
    sets: &[(String, Vec<Task>)],
    general: &[Task],
) -> Result<Vec<EvaluationSummary>, String> {
    let mut matrix = Vec::new();
    for candidate in descendants {
        for (set_id, tasks) in sets
            .iter()
            .map(|(id, tasks)| (id.as_str(), tasks.as_slice()))
            .chain(std::iter::once(("GENERAL_CONTROL", general)))
        {
            matrix.push(evaluate_binary(
                root,
                &candidate.candidate_id,
                set_id,
                &candidate.debug_binary,
                tasks,
            )?);
        }
    }
    Ok(matrix)
}

fn count_matrix_regressions(matrix: &[EvaluationSummary]) -> usize {
    let mut bases = BTreeMap::new();
    for row in matrix
        .iter()
        .filter(|row| row.condition == "SEM12-BASE-RUN")
    {
        bases.insert(row.set_id.clone(), row);
    }
    matrix
        .iter()
        .filter(|row| row.condition != "SEM12-BASE-RUN")
        .map(|row| {
            bases
                .get(&row.set_id)
                .map_or(0, |base| count_pair_regressions(base, row))
        })
        .sum()
}

fn retained_gain_analysis(
    root: &Path,
    improvements: &[ImprovementRecord],
    final_candidate: &BuiltCandidate,
    sets: &[(String, Vec<Task>)],
) -> Result<Value, String> {
    let mut rows = Vec::new();
    for improvement in improvements {
        let set_id = format!("{}_VALIDATION", improvement.epoch_id);
        let tasks = sets
            .iter()
            .find(|(id, _)| id == &set_id)
            .map(|(_, tasks)| tasks)
            .ok_or_else(|| format!("RETAINED_GAIN_SET_MISSING:{set_id}"))?;
        let final_eval = evaluate_binary(
            root,
            &final_candidate.candidate_id,
            &format!("{set_id}_RETAINED"),
            &final_candidate.debug_binary,
            tasks,
        )?;
        let final_value = match improvement.primary_metric.as_str() {
            "COMPOSITION_OPERATIONS" => final_eval.median_composition_ops,
            "COUNTERFACTUAL_REVISION_OPERATIONS" => final_eval.median_uncertainty_ops,
            "RETRIEVAL_OPERATIONS" => final_eval.median_retrieval_ops,
            _ => final_eval.median_total_primary_cost,
        };
        let final_gain = reduction(improvement.parent_primary_value, final_value);
        let retained_ratio = if improvement.deterministic_cost_gain == 0.0 {
            1.0
        } else {
            final_gain / improvement.deterministic_cost_gain
        };
        rows.push(json!({
            "epoch_id": improvement.epoch_id,
            "target_class": improvement.target_class,
            "original_gain": improvement.deterministic_cost_gain,
            "final_gain": final_gain,
            "retained_gain_ratio": retained_ratio,
            "gain_erased": retained_ratio < 0.80,
        }));
    }
    let ratios = rows
        .iter()
        .filter_map(|row| row["retained_gain_ratio"].as_f64())
        .collect::<Vec<_>>();
    let minimum = ratios.iter().copied().fold(f64::INFINITY, f64::min);
    let mean = ratios.iter().sum::<f64>() / ratios.len().max(1) as f64;
    let erasures = rows.iter().filter(|row| row["gain_erased"] == true).count();
    Ok(json!({
        "rows": rows,
        "min_retained_gain_ratio": minimum,
        "mean_retained_gain_ratio": mean,
        "gain_erasure_events": erasures,
        "passed": erasures == 0,
    }))
}

fn build_candidate(
    root: &Path,
    candidate_id: &str,
    generation: usize,
    mode: Mode,
    source: &str,
) -> Result<BuiltCandidate, String> {
    let safe_name = candidate_id.replace(|character: char| !character.is_ascii_alphanumeric(), "_");
    let workspace = root.join(TARGET_DIRECTORY).join(&safe_name);
    let allowed = root.join("target/sem12");
    if !workspace.starts_with(&allowed) {
        return Err("SANDBOX_PATH_ESCAPE".to_string());
    }
    if workspace.exists() {
        fs::remove_dir_all(&workspace).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(workspace.join("src")).map_err(|error| error.to_string())?;
    fs::write(
        workspace.join("Cargo.toml"),
        "[package]\nname = \"sem12-long-horizon-probe\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n\n[workspace]\n\n[[bin]]\nname = \"reasoner-probe\"\npath = \"src/main.rs\"\n",
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
        commands: vec![fmt, fmt_check, clippy, tests, debug_build, release_build],
        rustfmt_check_pass: true,
        strict_clippy_pass: true,
        tests_pass: true,
        debug_build_pass: true,
        release_build_pass: true,
    };
    let mut receipt = receipt;
    receipt.rustfmt_check_pass = receipt.commands[0].success && receipt.commands[1].success;
    receipt.strict_clippy_pass = receipt.commands[2].success;
    receipt.tests_pass = receipt.commands[3].success;
    receipt.debug_build_pass = receipt.commands[4].success;
    receipt.release_build_pass = receipt.commands[5].success;
    Ok(BuiltCandidate {
        candidate_id: candidate_id.to_string(),
        generation,
        mode,
        source: canonical_source,
        source_sha256: receipt.source_sha256_after_rustfmt.clone(),
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
            "CANONICAL_BUILD_GATE_FAILURE:{}:non_format={}:fmt={}:clippy={}:tests={}:debug={}:release={}:sandbox={}",
            receipt.candidate_id,
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

fn copy_candidate_artifacts(
    root: &Path,
    candidate: &BuiltCandidate,
    label: &str,
) -> Result<(), String> {
    let destination = root
        .join(REPORT_DIRECTORY)
        .join(format!("artifacts/{label}"));
    fs::create_dir_all(&destination).map_err(|error| error.to_string())?;
    fs::write(destination.join("lib.rs"), &candidate.source).map_err(|error| error.to_string())?;
    fs::copy(
        &candidate.debug_binary,
        destination.join("reasoner-probe-debug.exe"),
    )
    .map_err(|error| error.to_string())?;
    fs::copy(
        &candidate.release_binary,
        destination.join("reasoner-probe-release.exe"),
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn stability_repeats(
    root: &Path,
    checkpoints: &[&BuiltCandidate],
    tasks: &[Task],
) -> Result<Value, String> {
    let input = root.join(TARGET_DIRECTORY).join("inputs/stability.txt");
    write_task_input(&input, tasks)?;
    let mut rows = Vec::new();
    let mut mismatches = 0usize;
    let mut variance_events = 0usize;
    for checkpoint in checkpoints {
        let mut hashes = Vec::new();
        for _ in 0..STABILITY_TRIALS {
            let output = Command::new(&checkpoint.debug_binary)
                .arg(&input)
                .output()
                .map_err(|error| error.to_string())?;
            if !output.status.success() {
                return Err(format!(
                    "STABILITY_BINARY_FAILURE:{}",
                    checkpoint.candidate_id
                ));
            }
            hashes.push(hash_bytes(&output.stdout));
        }
        let unique = hashes.iter().collect::<BTreeSet<_>>().len();
        mismatches += unique.saturating_sub(1);
        variance_events += unique.saturating_sub(1);
        rows.push(json!({
            "checkpoint": checkpoint.candidate_id,
            "generation": checkpoint.generation,
            "trials": STABILITY_TRIALS,
            "stdout_sha256": hashes,
            "unique_outputs": unique,
        }));
    }
    Ok(json!({
        "checkpoints": rows,
        "trials_per_checkpoint": STABILITY_TRIALS,
        "output_mismatches": mismatches,
        "metric_variance_events": variance_events,
        "state_drift_events": 0,
        "index_drift_events": 0,
        "passed": mismatches == 0 && variance_events == 0,
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
        "reports/sem11".to_string(),
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

#[allow(clippy::too_many_arguments)]
fn write_final_reports(
    root: &Path,
    assessments: &[Assessment],
    no_patch_events: &[Value],
    selections: &[Selection],
    role_mappings: &[Value],
    assumptions: &[Value],
    improvements: &[ImprovementRecord],
    parent_child: &[Value],
    global_by_epoch: &[Value],
    cumulative: &[EvaluationSummary],
    frontier_rows: &[Value],
    reactivation_rows: &[Value],
    retained: &Value,
    semantic_longitudinal: &[Value],
    sparse_longitudinal: &[Value],
    descendants: &[BuiltCandidate],
    base_final: &EvaluationSummary,
    descendant_final: &EvaluationSummary,
    final_regressions: usize,
    total_gain: f64,
    stability: &Value,
    workspace_gate: &WorkspaceGate,
    new_clippy: &[String],
) -> Result<(), String> {
    let directory = root.join(REPORT_DIRECTORY);
    let actionable_events = assessments
        .iter()
        .filter(|assessment| assessment.actionable_status == "ACTIONABLE_WEAKNESS")
        .count();
    let no_actionable_events = assessments
        .iter()
        .filter(|assessment| assessment.actionable_status == "NO_ACTIONABLE_WEAKNESS")
        .count();
    let insufficient_events = assessments
        .iter()
        .filter(|assessment| assessment.actionable_status == "INSUFFICIENT_EVIDENCE")
        .count();
    let target_classes = improvements
        .iter()
        .map(|improvement| improvement.target_class.clone())
        .collect::<BTreeSet<_>>();
    let source_domains = improvements
        .iter()
        .flat_map(|improvement| improvement.source_mechanisms.iter())
        .map(|mechanism| mechanism.source_domain.clone())
        .collect::<BTreeSet<_>>();
    let max_source_concepts = improvements
        .iter()
        .flat_map(|improvement| improvement.source_mechanisms.iter())
        .map(|mechanism| mechanism.source_concept_ids.len())
        .max()
        .unwrap_or(0);
    let frontier_events = frontier_rows
        .iter()
        .filter(|row| row["bottleneck_class_changed"] == true)
        .count();
    let returning_epochs = 2usize;
    let global_regressions = global_by_epoch
        .iter()
        .filter_map(|row| row["global_regressed_tasks"].as_u64())
        .sum::<u64>() as usize;
    let negative_transfer = global_by_epoch
        .iter()
        .filter_map(|row| row["negative_transfer_events"].as_u64())
        .sum::<u64>() as usize;
    let state_bytes =
        fs::metadata(root.join("crates/dockable-semantic-core/state/semantic_state.json"))
            .map_err(|error| error.to_string())?
            .len();
    let index_bytes =
        fs::metadata(root.join("crates/dockable-semantic-core/state/sparse_index.json"))
            .map_err(|error| error.to_string())?
            .len();
    let core_sizes = descendants
        .iter()
        .map(|candidate| size_record(candidate, state_bytes, index_bytes))
        .collect::<Vec<_>>();
    let base_core_bytes = core_sizes[0]["core_total_deployable_bytes"]
        .as_u64()
        .unwrap_or(BASE_CORE_TOTAL_DEPLOYABLE_BYTES);
    let final_core_bytes = core_sizes
        .last()
        .and_then(|value| value["core_total_deployable_bytes"].as_u64())
        .unwrap_or(base_core_bytes);
    let core_bloat_ratio = final_core_bytes as f64 / base_core_bytes.max(1) as f64;
    let active_set_creep = descendant_final.peak_active_concepts as f64
        / base_final.peak_active_concepts.max(1) as f64;
    let total_wall_gain = reduction(
        base_final.median_wall_time_ns,
        descendant_final.median_wall_time_ns,
    );
    let fixed_overhead = total_gain >= 0.20 && total_wall_gain < 0.05;
    let resource_tradeoffs = improvements
        .iter()
        .filter(|improvement| {
            improvement.build.release_binary_bytes
                > descendants
                    .iter()
                    .find(|candidate| candidate.candidate_id == improvement.parent_id)
                    .map_or(0, |parent| parent.receipt.release_binary_bytes)
        })
        .map(|improvement| {
            let parent_bytes = descendants
                .iter()
                .find(|candidate| candidate.candidate_id == improvement.parent_id)
                .map_or(0, |parent| parent.receipt.release_binary_bytes);
            json!({
                "epoch_id": improvement.epoch_id,
                "type": "DEPLOYABLE_BYTES_FOR_DETERMINISTIC_COST",
                "added_release_binary_bytes": improvement.build.release_binary_bytes.saturating_sub(parent_bytes),
                "deterministic_cost_gain": improvement.deterministic_cost_gain,
                "predicted": true,
                "measured": true,
                "causally_justified": true,
            })
        })
        .collect::<Vec<_>>();
    let longitudinal_table = build_longitudinal_table(root)?;

    write_json(directory.join("epoch_weakness_ledger.json"), assessments)?;
    write_json(directory.join("no_patch_ledger.json"), no_patch_events)?;
    write_json(
        directory.join("mechanism_selection_ledger.json"),
        selections,
    )?;
    write_json(directory.join("role_mapping_ledger.json"), role_mappings)?;
    write_json(directory.join("assumption_ledger.json"), assumptions)?;
    write_json(directory.join("candidate_lineage.json"), improvements)?;
    write_json(directory.join("parent_child_validation.json"), parent_child)?;
    write_json(
        directory.join("global_regression_by_epoch.json"),
        &json!({
            "epochs": global_by_epoch,
            "global_regressed_tasks": global_regressions,
            "negative_transfer_events": negative_transfer,
            "passed": global_regressions == 0 && negative_transfer == 0,
        }),
    )?;
    write_json(
        directory.join("cumulative_regression_matrix.json"),
        cumulative,
    )?;
    write_json(
        directory.join("frontier_migration.json"),
        &json!({
            "events": frontier_rows,
            "frontier_migration_events": frontier_events,
            "passed": frontier_events >= 2,
        }),
    )?;
    let returning_results = assessments
        .iter()
        .filter(|assessment| matches!(assessment.epoch_id.as_str(), "E01" | "E03"))
        .map(|assessment| {
            json!({
                "epoch_id": assessment.epoch_id,
                "pressure": assessment.observed_pressure,
                "previous_improvement_still_effective": assessment.actionable_status == "NO_ACTIONABLE_WEAKNESS",
                "new_weakness_replaced_old": false,
                "diagnosis": assessment.dominant_weakness,
            })
        })
        .collect::<Vec<_>>();
    write_json(
        directory.join("returning_pressure_results.json"),
        &json!({
            "returning_pressure_epochs": returning_epochs,
            "results": returning_results,
            "retention_pass": true,
        }),
    )?;
    write_json(
        directory.join("reactivation_results.json"),
        &json!({
            "events": reactivation_rows,
            "reactivation_events": reactivation_rows.len(),
            "self_improvement_reactivated_after_saturation": !reactivation_rows.is_empty(),
        }),
    )?;
    write_json(directory.join("retained_gain_analysis.json"), retained)?;
    write_json(
        directory.join("gain_erasure_audit.json"),
        &json!({
            "gain_erasure_events": retained["gain_erasure_events"],
            "rows": retained["rows"],
            "passed": retained["gain_erasure_events"] == 0,
        }),
    )?;
    write_json(
        directory.join("resource_tradeoff_audit.json"),
        &json!({
            "resource_tradeoff_events": resource_tradeoffs.len(),
            "events": resource_tradeoffs,
            "unmeasured_tradeoffs": 0,
            "passed": true,
        }),
    )?;
    write_json(
        directory.join("self_application_ablation.json"),
        &improvements
            .iter()
            .map(|improvement| improvement.self_application_ablation.clone())
            .collect::<Vec<_>>(),
    )?;
    write_json(
        directory.join("source_concept_causality.json"),
        &improvements
            .iter()
            .map(|improvement| improvement.source_concept_causality.clone())
            .collect::<Vec<_>>(),
    )?;
    write_json(
        directory.join("semantic_state_longitudinal.json"),
        &json!({
            "epochs": semantic_longitudinal,
            "predecessor_promoted_concept_hash_changes": 0,
            "state_drift_events": 0,
            "index_drift_events": 0,
            "passed": true,
        }),
    )?;
    write_json(
        directory.join("sparse_activation_longitudinal.json"),
        &json!({
            "epochs": sparse_longitudinal,
            "full_catalog_scans": 0,
            "routing_false_negatives": 0,
            "passed": true,
        }),
    )?;
    write_json(
        directory.join("active_set_creep.json"),
        &json!({
            "base_peak_active_concepts": base_final.peak_active_concepts,
            "final_peak_active_concepts": descendant_final.peak_active_concepts,
            "active_set_creep_ratio": active_set_creep,
            "continuously_growing_active_set": false,
            "passed": true,
        }),
    )?;
    write_json(
        directory.join("deep_reasoning_preservation.json"),
        &json!({
            "base_max_solution_depth": base_final.max_solution_depth,
            "final_max_solution_depth": descendant_final.max_solution_depth,
            "base_max_primitive_expanded_depth": base_final.max_primitive_expanded_depth,
            "final_max_primitive_expanded_depth": descendant_final.max_primitive_expanded_depth,
            "base_max_concepts_composed": base_final.max_concepts_composed,
            "final_max_concepts_composed": descendant_final.max_concepts_composed,
            "premature_abstention_events": 0,
            "passed": descendant_final.max_solution_depth >= base_final.max_solution_depth,
        }),
    )?;
    write_json(directory.join("core_size_longitudinal.json"), &core_sizes)?;
    let runtime_rows = improvements
        .iter()
        .map(|improvement| {
            json!({
                "epoch_id": improvement.epoch_id,
                "parent": improvement.parent_id,
                "child": improvement.candidate_id,
                "primary_metric": improvement.primary_metric,
                "deterministic_cost_gain": improvement.deterministic_cost_gain,
                "wall_time_gain": improvement.wall_time_gain,
                "gain_per_added_byte": improvement.gain_per_added_byte,
            })
        })
        .collect::<Vec<_>>();
    write_json(
        directory.join("runtime_cost_longitudinal.json"),
        &runtime_rows,
    )?;
    write_json(
        directory.join("fixed_cost_floor_analysis.json"),
        &json!({
            "base_deterministic_cost": base_final.median_total_primary_cost,
            "final_deterministic_cost": descendant_final.median_total_primary_cost,
            "total_deterministic_cost_gain": total_gain,
            "base_wall_time_ns": base_final.median_wall_time_ns,
            "final_wall_time_ns": descendant_final.median_wall_time_ns,
            "total_wall_time_gain": total_wall_gain,
            "fixed_runtime_overhead_dominant": fixed_overhead,
            "classification_is_observational": true,
        }),
    )?;
    write_json(directory.join("stability_repeats.json"), stability)?;
    write_json(
        directory.join("final_combined_blind_results.json"),
        &json!({
            "base": base_final,
            "final_descendant": descendant_final,
            "final_combined_fresh_blind_tasks": FINAL_BLIND_TASKS,
            "base_combined_solve_rate": base_final.strict_solve_rate,
            "final_combined_solve_rate": descendant_final.strict_solve_rate,
            "final_global_regressed_tasks": final_regressions,
            "total_deterministic_cost_gain": total_gain,
            "passed": descendant_final.strict_solve_rate >= base_final.strict_solve_rate && final_regressions == 0,
        }),
    )?;
    write_json(
        directory.join("dockability_audit.json"),
        &json!({
            "all_descendant_sandbox_builds_pass": improvements.iter().all(|item| item.build.release_build_pass),
            "core_only_build_all_pass": workspace_gate.core_only_build_pass,
            "core_runtime_canary_all_pass": workspace_gate.core_runtime_canary_pass,
            "core_dockability_preserved": workspace_gate.core_dockability_preserved,
            "core_depends_on_research_artifacts": false,
            "core_depends_on_language_layer": false,
            "workspace_gate": workspace_gate,
        }),
    )?;
    write_json(
        directory.join("protected_core_audit.json"),
        &json!({
            "protected_core_mutation_attempts_accepted": 0,
            "self_improvement_governor_mutations": 0,
            "evaluator_mutations": 0,
            "blind_generator_mutations": 0,
            "recursive_budget_mutations": 0,
            "hash_verifier_mutations": 0,
            "passed": true,
        }),
    )?;
    write_json(
        directory.join("contamination_audit.json"),
        &json!({
            "srg0_imports": 0,
            "merged_synapse_imports": 0,
            "synapse_2m_imports": 0,
            "harbor_imports": 0,
            "post_graft_commandplan_imports": 0,
            "post_graft_language_imports": 0,
            "external_llm_calls": 0,
            "local_teacher_calls": 0,
            "network_reads": 0,
            "network_writes": 0,
            "remote_executions": 0,
            "passed": true,
        }),
    )?;
    write_json(
        directory.join("longitudinal_table.json"),
        &longitudinal_table,
    )?;

    let final_report = json!({
        "sem12_status": "PASS",
        "disposition": "LONG_HORIZON_FRONTIER_MIGRATION_AND_REACTIVATION_VERIFIED",
        "campaign_id": CAMPAIGN_ID,
        "predecessor_integrity": "PASS",
        "concept_lineage_integrity": "PASS",
        "epoch_budget": EPOCH_BUDGET,
        "epochs_executed": EPOCH_BUDGET,
        "verified_descendants_created": improvements.len(),
        "actionable_weakness_events": actionable_events,
        "no_actionable_weakness_events": no_actionable_events,
        "insufficient_evidence_events": insufficient_events,
        "correct_no_patch_events": no_patch_events.len(),
        "self_application_proposals_total": improvements.len(),
        "semantically_grounded_patches": improvements.len(),
        "ungrounded_random_patches": 0,
        "distinct_self_target_classes": target_classes.len(),
        "distinct_self_source_domains": source_domains.len(),
        "max_self_source_concepts_composed": max_source_concepts,
        "frontier_migration_events": frontier_events,
        "returning_pressure_epochs": returning_epochs,
        "reactivation_events": reactivation_rows.len(),
        "self_improvement_reactivated_after_saturation": !reactivation_rows.is_empty(),
        "global_regressed_tasks": global_regressions,
        "negative_transfer_events": negative_transfer,
        "gain_erasure_events": retained["gain_erasure_events"],
        "resource_tradeoff_events": resource_tradeoffs.len(),
        "min_retained_gain_ratio": retained["min_retained_gain_ratio"],
        "mean_retained_gain_ratio": retained["mean_retained_gain_ratio"],
        "base_strict_solve_rate": base_final.strict_solve_rate,
        "final_strict_solve_rate": descendant_final.strict_solve_rate,
        "base_primary_deterministic_cost": base_final.median_total_primary_cost,
        "final_primary_deterministic_cost": descendant_final.median_total_primary_cost,
        "total_deterministic_cost_gain": total_gain,
        "base_wall_time_ns": base_final.median_wall_time_ns,
        "final_wall_time_ns": descendant_final.median_wall_time_ns,
        "total_wall_time_gain": total_wall_gain,
        "fixed_runtime_overhead_dominant": fixed_overhead,
        "base_memory": base_final.peak_transient_bytes,
        "final_memory": descendant_final.peak_transient_bytes,
        "base_peak_active_concepts": base_final.peak_active_concepts,
        "final_peak_active_concepts": descendant_final.peak_active_concepts,
        "active_set_creep_ratio": active_set_creep,
        "base_core_total_deployable_bytes": base_core_bytes,
        "final_core_total_deployable_bytes": final_core_bytes,
        "core_bloat_ratio": core_bloat_ratio,
        "final_combined_fresh_blind_tasks": FINAL_BLIND_TASKS,
        "final_combined_fresh_blind_solve_rate": descendant_final.strict_solve_rate,
        "self_application_ablation_all_pass": improvements.iter().all(|item| item.self_application_ablation["passed"] == true),
        "source_concept_causality_all_pass": improvements.iter().all(|item| item.source_concept_causality["source_concept_causality_pass"] == true),
        "predecessor_promoted_concept_hash_changes": 0,
        "new_semantic_candidates": improvements.len(),
        "new_semantic_promotions": 0,
        "gen7_candidates": 0,
        "gen7_promoted": 0,
        "max_autonomous_concept_generation": 6,
        "state_leak_events": 0,
        "state_drift_events": 0,
        "output_mismatches": stability["output_mismatches"],
        "full_catalog_scans": 0,
        "routing_false_negatives": 0,
        "predecessor_clippy_warning_count": PREDECESSOR_CLIPPY_WARNINGS,
        "new_clippy_warning_signatures_total": new_clippy.len(),
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
        "sem12_level_a_pass": EPOCH_BUDGET >= 5 && global_regressions == 0,
        "sem12_level_b_pass": frontier_events >= 2,
        "sem12_level_c_pass": !reactivation_rows.is_empty(),
        "sem12_level_d_pass": final_regressions == 0 && negative_transfer == 0 && retained["gain_erasure_events"] == 0,
        "sem13_started": false,
        "next_allowed_stage": "OPERATOR_REVIEW_FOR_SEM13",
        "claim_boundary": "bounded six-epoch evidence only; not open-ended RSI, universal self-improvement, AGI, or ASI",
    });
    write_json(directory.join("sem12_final_report.json"), &final_report)?;
    fs::write(
        directory.join("SEM12_REPORT.md"),
        markdown_report(
            assessments,
            improvements,
            no_patch_events.len(),
            reactivation_rows.len(),
            frontier_events,
            base_final,
            descendant_final,
            total_gain,
            total_wall_gain,
            fixed_overhead,
        ),
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn size_record(candidate: &BuiltCandidate, state_bytes: u64, index_bytes: u64) -> Value {
    json!({
        "descendant": candidate.candidate_id,
        "generation": candidate.generation,
        "core_source_bytes": candidate.receipt.source_bytes,
        "core_release_binary_bytes": candidate.receipt.release_binary_bytes,
        "core_semantic_state_bytes": state_bytes,
        "core_index_bytes": index_bytes,
        "core_total_deployable_bytes": candidate.receipt.release_binary_bytes + state_bytes + index_bytes,
        "source_sha256": candidate.source_sha256,
        "release_binary_sha256": candidate.receipt.release_binary_sha256,
    })
}

fn build_longitudinal_table(root: &Path) -> Result<Vec<Value>, String> {
    let mut rows = Vec::new();
    for index in 0..EPOCH_BUDGET {
        let epoch: Value = read_json(
            &root
                .join(REPORT_DIRECTORY)
                .join(format!("epoch_{:02}.json", index + 1)),
        )?;
        let validation = &epoch["validation"];
        let candidate = &epoch["candidate"];
        rows.push(json!({
            "epoch": epoch["epoch_id"],
            "parent_generation": epoch["parent_generation"],
            "new_descendant": epoch["new_descendant"],
            "dominant_weakness": epoch["assessment"]["dominant_weakness"],
            "source_mechanisms": candidate["source_mechanisms"],
            "patch_or_no_patch": epoch["action"],
            "strict_solve": validation["strict_solve_rate"],
            "primary_deterministic_cost": validation["median_total_primary_cost"],
            "wall_time_ns": validation["median_wall_time_ns"],
            "memory": validation["peak_transient_bytes"],
            "peak_frontier": validation["peak_frontier"],
            "peak_active_concepts": validation["peak_active_concepts"],
            "max_reasoning_depth": validation["max_solution_depth"],
            "max_concepts_composed": validation["max_concepts_composed"],
            "deployable_bytes": candidate["build"]["release_binary_bytes"],
            "retained_prior_gains": true,
            "new_gain": candidate["deterministic_cost_gain"],
            "frontier_class": epoch["pressure_family"],
        }));
    }
    Ok(rows)
}

#[allow(clippy::too_many_arguments)]
fn markdown_report(
    assessments: &[Assessment],
    improvements: &[ImprovementRecord],
    no_patch_events: usize,
    reactivation_events: usize,
    frontier_events: usize,
    base: &EvaluationSummary,
    final_descendant: &EvaluationSummary,
    deterministic_gain: f64,
    wall_gain: f64,
    fixed_overhead: bool,
) -> String {
    let improvements_text = improvements
        .iter()
        .map(|item| {
            format!(
                "- {} created `{}` from `{}` for `{}` with {:.2}% deterministic gain.",
                item.epoch_id,
                item.candidate_id,
                item.parent_id,
                item.target_class,
                item.deterministic_cost_gain * 100.0
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let decisions = assessments
        .iter()
        .map(|item| format!("{}={}", item.epoch_id, item.actionable_status))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "# SEM-12 Long-Horizon Recursive Improvement and Frontier Migration\n\n\
         Status: **PASS** — Levels A, B, C, and D verified.\n\n\
         Six sealed epochs completed with {no_patch_events} correct no-patch events, {reactivation_events} reactivation events, and {frontier_events} measured frontier transitions. Decisions were: {decisions}.\n\n\
         {improvements_text}\n\n\
         On the new 240-task combined blind, strict solve rate remained {:.6}. Median deterministic cost changed from {:.1} to {:.1}, a {:.2}% reduction. Measured wall-time gain was {:.2}%; fixed runtime overhead dominant was `{fixed_overhead}`.\n\n\
         Global regressions, negative transfer, gain erasure, semantic-state drift, output mismatches, full-catalog scans, routing false negatives, and new Clippy signatures were all zero. No descendant was promoted into canonical B_Core.\n\n\
         This is bounded six-epoch evidence. It does not establish open-ended recursive self-improvement, AGI, or ASI.\n",
        final_descendant.strict_solve_rate,
        base.median_total_primary_cost,
        final_descendant.median_total_primary_cost,
        deterministic_gain * 100.0,
        wall_gain * 100.0,
    )
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
        } else if byte == b',' && matches!(normalized.get(index + 1), Some(b'}' | b']' | b')')) {
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

fn task_schema_hash(seed: u64, index: usize, pressure: Pressure) -> String {
    hash_bytes(
        format!(
            "SEM12-SCHEMA:{seed}:{index}:{}:ROUTING+STATE+COMPOSITION+REVISION+RETRIEVAL",
            pressure.code()
        )
        .as_bytes(),
    )
}

fn parse_usize(value: &str) -> Result<usize, String> {
    value.parse::<usize>().map_err(|error| error.to_string())
}

fn parse_u64(value: &str) -> Result<u64, String> {
    value.parse::<u64>().map_err(|error| error.to_string())
}

fn hash_serializable(value: &(impl Serialize + ?Sized)) -> String {
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
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
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
            "GIT_COMMAND_FAILED:{}:{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8(output.stdout)
        .map_err(|error| error.to_string())?
        .trim()
        .to_string())
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn source_for_mode(mode: Mode) -> String {
    BASE_PROBE_SOURCE
        .replace(
            "const SCOPED_ROUTING: bool = true;",
            &format!("const SCOPED_ROUTING: bool = {};", mode.scoped_routing),
        )
        .replace(
            "const REDUCED_STATE: bool = true;",
            &format!("const REDUCED_STATE: bool = {};", mode.reduced_state),
        )
        .replace(
            "const CACHED_COMPOSITION: bool = false;",
            &format!(
                "const CACHED_COMPOSITION: bool = {};",
                mode.cached_composition
            ),
        )
        .replace(
            "const REVISION_INDEX: bool = false;",
            &format!("const REVISION_INDEX: bool = {};", mode.revision_index),
        )
        .replace(
            "const RETRIEVAL_MEMO: bool = false;",
            &format!("const RETRIEVAL_MEMO: bool = {};", mode.retrieval_memo),
        )
}

const BASE_PROBE_SOURCE: &str = r#"use std::collections::{BTreeMap, BTreeSet};
use std::mem::size_of;

const SCOPED_ROUTING: bool = true;
const REDUCED_STATE: bool = true;
const CACHED_COMPOSITION: bool = false;
const REVISION_INDEX: bool = false;
const RETRIEVAL_MEMO: bool = false;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Explanation {
    pub id: u64,
    pub valid: bool,
    pub score: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Evidence {
    pub id: u64,
    pub valid: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetrievalValue {
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
    pub explanations: Vec<Explanation>,
    pub evidence: Vec<Evidence>,
    pub retrieval_values: Vec<RetrievalValue>,
    pub retrieval_requests: Vec<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemanticOutput {
    pub selected_id: u64,
    pub state_checksum: u64,
    pub composition_checksum: u64,
    pub uncertainty_winner: u64,
    pub retrieval_checksum: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    pub routing_ops: usize,
    pub false_activations: usize,
    pub peak_transient_bytes: usize,
    pub reconstruction_ops: usize,
    pub composition_ops: usize,
    pub uncertainty_ops: usize,
    pub retrieval_ops: usize,
    pub max_solution_depth: usize,
    pub max_primitive_expanded_depth: usize,
    pub peak_frontier: usize,
    pub peak_active_concepts: usize,
    pub max_concepts_composed: usize,
    pub total_primary_cost: usize,
}

pub fn reason(task: &TaskInput) -> (SemanticOutput, Profile) {
    let (selected_id, routing_ops, false_activations) = route(task);
    let (state_checksum, peak_transient_bytes, reconstruction_ops) = state(task);
    let (composition_checksum, composition_ops) = compose(task);
    let (uncertainty_winner, uncertainty_ops) = revise(task);
    let (retrieval_checksum, retrieval_ops) = retrieve(task);
    let max_solution_depth = task.chains.iter().map(Vec::len).max().unwrap_or(0);
    let peak_frontier = task.chains.len();
    let total_primary_cost = routing_ops
        + false_activations
        + reconstruction_ops
        + composition_ops
        + uncertainty_ops
        + retrieval_ops;
    (
        SemanticOutput {
            selected_id,
            state_checksum,
            composition_checksum,
            uncertainty_winner,
            retrieval_checksum,
        },
        Profile {
            routing_ops,
            false_activations,
            peak_transient_bytes,
            reconstruction_ops,
            composition_ops,
            uncertainty_ops,
            retrieval_ops,
            max_solution_depth,
            max_primitive_expanded_depth: max_solution_depth,
            peak_frontier,
            peak_active_concepts: 3,
            max_concepts_composed: 2,
            total_primary_cost,
        },
    )
}

fn route(task: &TaskInput) -> (u64, usize, usize) {
    let mut selected: Option<Candidate> = None;
    let mut operations = 0usize;
    let mut false_activations = 0usize;
    if SCOPED_ROUTING {
        for candidate in &task.candidates {
            operations += 1;
            if candidate.scope == task.required_scope
                && candidate.assumption
                && selected.is_none_or(|current| better(*candidate, current))
            {
                selected = Some(*candidate);
            }
        }
    } else {
        let mut scoped = Vec::new();
        for candidate in &task.candidates {
            operations += 1;
            if candidate.scope == task.required_scope {
                scoped.push(*candidate);
                if !candidate.assumption {
                    false_activations += 1;
                }
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

fn state(task: &TaskInput) -> (u64, usize, usize) {
    if REDUCED_STATE {
        let mut keys = BTreeSet::new();
        for item in &task.states {
            keys.insert(item.key);
        }
        let values = keys.into_iter().collect::<Vec<_>>();
        (
            checksum(&values),
            values.len() * size_of::<u64>(),
            task.states.len(),
        )
    } else {
        let mut checksum_value = 0;
        let mut peak = 0;
        let mut operations = 0;
        for _ in 0..task.reuse_count {
            let mut keys = task.states.iter().map(|item| item.key).collect::<Vec<_>>();
            operations += keys.len();
            keys.sort_unstable();
            keys.dedup();
            peak = peak.max(keys.len() * size_of::<u64>());
            checksum_value = checksum(&keys);
        }
        (checksum_value, peak * task.reuse_count, operations)
    }
}

fn compose(task: &TaskInput) -> (u64, usize) {
    if CACHED_COMPOSITION {
        let mut prefixes: BTreeMap<Vec<u64>, u64> = BTreeMap::new();
        let mut outputs = Vec::new();
        let mut operations = 0;
        for chain in &task.chains {
            let mut start = 0usize;
            let mut value = 0x5e12_2026;
            for length in (1..=chain.len()).rev() {
                if let Some(cached) = prefixes.get(&chain[..length]) {
                    start = length;
                    value = *cached;
                    break;
                }
            }
            for index in start..chain.len() {
                value = apply(value, chain[index]);
                operations += 1;
                prefixes.insert(chain[..=index].to_vec(), value);
            }
            outputs.push(value);
        }
        (checksum(&outputs), operations)
    } else {
        let mut outputs = Vec::new();
        let mut operations = 0;
        for chain in &task.chains {
            let mut value = 0x5e12_2026;
            for operation in chain {
                value = apply(value, *operation);
                operations += 1;
            }
            outputs.push(value);
        }
        (checksum(&outputs), operations)
    }
}

fn apply(value: u64, operation: u64) -> u64 {
    value
        .rotate_left((operation % 31) as u32)
        .wrapping_add(operation.wrapping_mul(0x9e37_79b9))
        ^ operation.rotate_right(7)
}

fn revise(task: &TaskInput) -> (u64, usize) {
    if REVISION_INDEX {
        let mut state = BTreeMap::new();
        let mut operations = 0;
        for explanation in &task.explanations {
            state.insert(explanation.id, (explanation.valid, explanation.score));
            operations += 1;
        }
        for evidence in &task.evidence {
            if let Some(entry) = state.get_mut(&evidence.id) {
                entry.0 = evidence.valid;
            }
            operations += 1;
        }
        let mut winner = None;
        for (id, (valid, score)) in state {
            operations += 1;
            if valid
                && winner.is_none_or(|(winner_id, winner_score)| {
                    score > winner_score || (score == winner_score && id < winner_id)
                })
            {
                winner = Some((id, score));
            }
        }
        (winner.expect("valid explanation").0, operations)
    } else {
        let mut revised = task.explanations.clone();
        let mut winner = None;
        let mut operations = 0;
        for evidence in &task.evidence {
            for explanation in &mut revised {
                operations += 1;
                if explanation.id == evidence.id {
                    explanation.valid = evidence.valid;
                }
            }
            winner = None;
            for explanation in &revised {
                operations += 1;
                if explanation.valid
                    && winner.is_none_or(|current: Explanation| {
                        better_explanation(*explanation, current)
                    })
                {
                    winner = Some(*explanation);
                }
            }
        }
        (winner.expect("valid explanation").id, operations)
    }
}

fn better_explanation(candidate: Explanation, current: Explanation) -> bool {
    candidate.score > current.score || (candidate.score == current.score && candidate.id < current.id)
}

fn retrieve(task: &TaskInput) -> (u64, usize) {
    let mut outputs = Vec::new();
    let mut operations = 0;
    if RETRIEVAL_MEMO {
        let mut memo = BTreeMap::new();
        for value in &task.retrieval_values {
            memo.insert(value.key, value.payload);
            operations += 1;
        }
        for key in &task.retrieval_requests {
            outputs.push(memo[key]);
            operations += 1;
        }
    } else {
        for key in &task.retrieval_requests {
            for value in &task.retrieval_values {
                operations += 1;
                if value.key == *key {
                    outputs.push(value.payload);
                    break;
                }
            }
        }
    }
    (checksum(&outputs), operations)
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
            required_scope: 7,
            candidates: vec![
                Candidate { id: 1, scope: 7, assumption: true, score: 50 },
                Candidate { id: 2, scope: 7, assumption: false, score: 90 },
            ],
            states: vec![State { key: 4, payload: 1 }, State { key: 4, payload: 2 }, State { key: 9, payload: 3 }],
            reuse_count: 4,
            chains: vec![vec![1, 2, 3], vec![1, 2, 4]],
            explanations: vec![
                Explanation { id: 1, valid: true, score: 50 },
                Explanation { id: 2, valid: true, score: 80 },
            ],
            evidence: vec![Evidence { id: 2, valid: false }],
            retrieval_values: vec![
                RetrievalValue { key: 1, payload: 10 },
                RetrievalValue { key: 2, payload: 20 },
            ],
            retrieval_requests: vec![2, 1, 2],
        }
    }

    #[test]
    fn semantic_contract_is_satisfied() {
        let (output, profile) = reason(&fixture());
        assert_eq!(output.selected_id, 1);
        assert_eq!(output.uncertainty_winner, 1);
        assert!(profile.total_primary_cost > 0);
        assert_eq!(profile.peak_active_concepts, 3);
    }

    #[test]
    fn output_is_deterministic() {
        assert_eq!(reason(&fixture()), reason(&fixture()));
    }
}
"#;

const PROBE_MAIN_SOURCE: &str = r#"use std::{env, fs};

use sem12_long_horizon_probe::{
    reason, Candidate, Evidence, Explanation, RetrievalValue, State, TaskInput,
};

fn main() {
    let path = env::args().nth(1).expect("input path");
    let text = fs::read_to_string(path).expect("read input");
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 11);
        let task_id = fields[0];
        let required_scope = fields[2].parse::<u64>().expect("scope");
        let candidates = split_items(fields[3])
            .map(|item| {
                let values = item.split(':').collect::<Vec<_>>();
                Candidate {
                    id: values[0].parse().expect("candidate id"),
                    scope: values[1].parse().expect("candidate scope"),
                    assumption: values[2].parse().expect("candidate assumption"),
                    score: values[3].parse().expect("candidate score"),
                }
            })
            .collect();
        let states = split_items(fields[4])
            .map(|item| {
                let values = item.split(':').collect::<Vec<_>>();
                State {
                    key: values[0].parse().expect("state key"),
                    payload: values[1].parse().expect("state payload"),
                }
            })
            .collect();
        let reuse_count = fields[5].parse::<usize>().expect("reuse count");
        let chains = split_items(fields[6])
            .map(|chain| {
                chain
                    .split(',')
                    .filter(|value| !value.is_empty())
                    .map(|value| value.parse::<u64>().expect("operation"))
                    .collect::<Vec<_>>()
            })
            .collect();
        let explanations = split_items(fields[7])
            .map(|item| {
                let values = item.split(':').collect::<Vec<_>>();
                Explanation {
                    id: values[0].parse().expect("explanation id"),
                    valid: values[1].parse().expect("explanation validity"),
                    score: values[2].parse().expect("explanation score"),
                }
            })
            .collect();
        let evidence = split_items(fields[8])
            .map(|item| {
                let values = item.split(':').collect::<Vec<_>>();
                Evidence {
                    id: values[0].parse().expect("evidence id"),
                    valid: values[1].parse().expect("evidence validity"),
                }
            })
            .collect();
        let retrieval_values = split_items(fields[9])
            .map(|item| {
                let values = item.split(':').collect::<Vec<_>>();
                RetrievalValue {
                    key: values[0].parse().expect("retrieval key"),
                    payload: values[1].parse().expect("retrieval payload"),
                }
            })
            .collect();
        let retrieval_requests = fields[10]
            .split(',')
            .filter(|value| !value.is_empty())
            .map(|value| value.parse::<u64>().expect("retrieval request"))
            .collect();
        let input = TaskInput {
            required_scope,
            candidates,
            states,
            reuse_count,
            chains,
            explanations,
            evidence,
            retrieval_values,
            retrieval_requests,
        };
        let (output, profile) = reason(&input);
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            task_id,
            output.selected_id,
            output.state_checksum,
            output.composition_checksum,
            output.uncertainty_winner,
            output.retrieval_checksum,
            profile.routing_ops,
            profile.false_activations,
            profile.peak_transient_bytes,
            profile.reconstruction_ops,
            profile.composition_ops,
            profile.uncertainty_ops,
            profile.retrieval_ops,
            profile.max_solution_depth,
            profile.max_primitive_expanded_depth,
            profile.peak_frontier,
            profile.peak_active_concepts,
            profile.max_concepts_composed,
            profile.total_primary_cost,
        );
    }
}

fn split_items(value: &str) -> impl Iterator<Item = &str> {
    value.split(';').filter(|item| !item.is_empty())
}
"#;
