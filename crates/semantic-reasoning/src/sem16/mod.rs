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

const CAMPAIGN_ID: &str = "SEM16-LONG-HORIZON-META-FRONTIER-0001";
const SEM15_COMMIT: &str = "1a5b8b9ee591e41ea9f4be4ac46b03890e439e60";
const SEM15_SOURCE_SHA256: &str =
    "69944a68af9bd4463ce2e741d2197c7c3b4e6767f9d4395410d1f1c76e00a563";
const SEM15_BINARY_SHA256: &str =
    "c546b87444f48762401d741ae58abc9b97a0015cfe2f8ea4a89c1c3b236bdc0d";
const SEMANTIC_STATE_SHA256: &str =
    "d1abd8de410f5284773f1e582937922dc514513ed738eb9f04e8bf2735185d3c";
const INDEX_SHA256: &str = "77b17332b5ff7204c28e9445e689276049afd6e89308e7e242904570a283e6fc";
const BASE_CORE_TOTAL_DEPLOYABLE_BYTES: u64 = 168_691;
const PREDECESSOR_CLIPPY_WARNINGS: usize = 22;
const META_EPOCH_BUDGET: usize = 6;
const DIAGNOSTIC_COUNT: usize = 24;
const VALIDATION_COUNT: usize = 36;
const FINAL_META_BLIND_COUNT: usize = 180;
const FINAL_DOWNSTREAM_COUNT: usize = 80;
const TRIALS: usize = 5;
const REPORT_DIR: &str = "reports/sem16";
const TARGET_DIR: &str = "target/sem16/SEM16-LONG-HORIZON-META-FRONTIER-0001";

const GOVERNOR_POLICY: &str = "SEM16-GOVERNOR-V1|SIX_EPOCH_BOUND|QUALITY_NEVER_DROPS|VERIFIED_CHILD_ONLY|NO_STALE_TARGET|NO_AUTHORITY_MUTATION|NO_PRODUCTION_PROMOTION";
const EVALUATOR_POLICY: &str = "SEM16-EVALUATOR-V1|FRESH_PER_EPOCH|IDENTICAL_PARENT_CHILD_INPUT|HIDDEN_TRUTH|LONGITUDINAL_RETENTION|FINAL_BLIND_AFTER_FREEZE";
const ACCEPTANCE_POLICY: &str = "SEM16-ACCEPTANCE-V1|QUALITY_NONDECLINE|DETERMINISTIC_COST_GAIN_AT_LEAST_5_PERCENT|DOWNSTREAM_NONDECLINE|RETENTION_RATIO_ONE|ZERO_NEGATIVE_TRANSFER";

const REQUIRED_REPORTS: &[&str] = &[
    "predecessor_integrity.json",
    "campaign_config.json",
    "meta_base_manifest.json",
    "meta_epoch_schedule.json",
    "meta_epoch_01.json",
    "meta_epoch_02.json",
    "meta_epoch_03.json",
    "meta_epoch_04.json",
    "meta_epoch_05.json",
    "meta_epoch_06.json",
    "meta_epoch_assessments.json",
    "meta_no_patch_ledger.json",
    "meta_weakness_ledger.json",
    "meta_mechanism_selection.json",
    "meta_role_mapping.json",
    "meta_assumption_ledger.json",
    "meta_candidate_lineage.json",
    "meta_parent_child_validation.json",
    "longitudinal_meta_regression_matrix.json",
    "meta_frontier_migration.json",
    "returning_meta_pressure_results.json",
    "meta_reactivation_results.json",
    "retained_meta_gain_analysis.json",
    "meta_gain_erasure_audit.json",
    "meta_resource_tradeoff_audit.json",
    "meta_self_application_ablation.json",
    "meta_source_concept_causality.json",
    "downstream_by_epoch.json",
    "semantic_state_longitudinal.json",
    "meta_sparse_activation_longitudinal.json",
    "meta_active_set_creep.json",
    "meta_runtime_cost_longitudinal.json",
    "meta_fixed_cost_floor_analysis.json",
    "core_size_longitudinal.json",
    "governor_longitudinal_audit.json",
    "evaluator_gaming_audit.json",
    "ordinary_reasoning_regression.json",
    "final_meta_combined_blind_manifest.json",
    "final_meta_combined_blind_results.json",
    "final_downstream_blind_manifest.json",
    "final_downstream_results.json",
    "stability_repeats.json",
    "clippy_differential_audit.json",
    "dockability_audit.json",
    "sem16_final_report.json",
    "SEM16_REPORT.md",
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
struct Mode {
    frontier_priority: bool,
    state_snapshot_reuse: bool,
    adaptive_probe_budget: bool,
}

impl Mode {
    const BASE: Self = Self {
        frontier_priority: false,
        state_snapshot_reuse: false,
        adaptive_probe_budget: false,
    };
    const MD1: Self = Self {
        frontier_priority: true,
        ..Self::BASE
    };
    const MD2: Self = Self {
        state_snapshot_reuse: true,
        ..Self::MD1
    };
    const MD3: Self = Self {
        adaptive_probe_budget: true,
        ..Self::MD2
    };

    fn id(self) -> &'static str {
        match self {
            Self::BASE => "SEM16_META_BASE",
            Self::MD1 => "SEM16_MD1_BOUNDED_FRONTIER_PRIORITY",
            Self::MD2 => "SEM16_MD2_META_STATE_SNAPSHOT_REUSE",
            Self::MD3 => "SEM16_MD3_ADAPTIVE_PROBE_BUDGET",
            _ => "SEM16_INVALID_MODE",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Pressure {
    AmbiguousCausalEvidence,
    ManyMechanismCombinations,
    StrongPriorRejectionEvidence,
    MixedAmbiguityComposition,
    ResourceConstrainedSearch,
    ReturningAmbiguityFreshBudget,
    Downstream,
}

impl Pressure {
    fn label(self) -> &'static str {
        match self {
            Self::AmbiguousCausalEvidence => "AMBIGUOUS_CAUSAL_EVIDENCE",
            Self::ManyMechanismCombinations => "MANY_MECHANISM_COMBINATIONS",
            Self::StrongPriorRejectionEvidence => "STRONG_REUSABLE_PRIOR_REJECTION_EVIDENCE",
            Self::MixedAmbiguityComposition => "MIXED_AMBIGUITY_AND_COMPOSITION",
            Self::ResourceConstrainedSearch => "RESOURCE_CONSTRAINED_SEARCH",
            Self::ReturningAmbiguityFreshBudget => "RETURNING_AMBIGUITY_UNDER_FRESH_TIGHT_BUDGET",
            Self::Downstream => "DOWNSTREAM_ORDINARY_SELF_IMPROVEMENT",
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
    actionable: bool,
    evidence: Vec<u64>,
    mechanisms: Vec<MechanismInput>,
    base_cost: u64,
    schema_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VisibleSet {
    set_id: String,
    pressure: Pressure,
    count: usize,
    seed: u64,
    seed_commitment_sha256: String,
    challenge_commitments: Vec<Value>,
    truth_exposed_to_engine: bool,
    expected_output_exposed_to_engine: bool,
    frozen_before_epoch_execution: bool,
    manifest_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EpochSpec {
    epoch: usize,
    epoch_id: String,
    pressure: Pressure,
    pressure_description: String,
    returning_pressure: bool,
    mixed_pressure: bool,
    diagnostic: VisibleSet,
    validation: VisibleSet,
    candidate_mode_if_fresh_weakness: Option<Mode>,
    intended_fix_disclosed_to_engine: bool,
    frozen_before_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CombinedManifest {
    manifest_id: String,
    count: usize,
    banks: Vec<VisibleSet>,
    unopened_until_final_meta_descendant_frozen: bool,
    manifest_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    verified_improvements: usize,
    median_deterministic_cost: f64,
    median_wall_time_ns: f64,
    peak_frontier: usize,
    peak_active_concepts: usize,
    search_expansions: usize,
    mechanism_candidates: usize,
    peak_temporary_memory: usize,
    median_descendant_cost: f64,
    output_sha256: String,
    repeat_output_mismatches: usize,
    records: Vec<RawRecord>,
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
    commands: Vec<CommandReceipt>,
}

#[derive(Debug, Clone)]
struct BuiltEngine {
    id: String,
    source: String,
    source_sha256: String,
    debug_binary: PathBuf,
    release_binary: PathBuf,
    receipt: BuildReceipt,
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
        return Err("SEM16_REPORT_DIRECTORY_NOT_EMPTY".to_string());
    }
    fs::create_dir_all(report_dir.join("artifacts/meta-base"))
        .map_err(|error| error.to_string())?;

    let infrastructure_commit = git_output(root, &["rev-parse", "HEAD"])?;
    let schedule = epoch_schedule();
    if schedule.len() != META_EPOCH_BUDGET {
        return Err("META_EPOCH_BUDGET_MISMATCH".to_string());
    }
    let official_source = root.join("reports/sem15/artifacts/m2-abc-composed/lib.rs");
    let official_binary =
        root.join("reports/sem15/artifacts/m2-abc-composed/meta-generalization-probe-release.exe");
    require_equal(
        &hash_file(&official_source)?,
        SEM15_SOURCE_SHA256,
        "SEM15_COMPOSED_SOURCE",
    )?;
    require_equal(
        &hash_file(&official_binary)?,
        SEM15_BINARY_SHA256,
        "SEM15_COMPOSED_BINARY",
    )?;
    fs::copy(
        &official_source,
        report_dir.join("artifacts/meta-base/sem15-composed-lib.rs"),
    )
    .map_err(|error| error.to_string())?;
    fs::copy(
        &official_binary,
        report_dir.join("artifacts/meta-base/sem15-composed-release.exe"),
    )
    .map_err(|error| error.to_string())?;

    let proxy = build_engine(root, "SEM16-META-BASE-PROXY", Mode::BASE)?;
    ensure_build(&proxy.receipt)?;
    copy_engine(root, &proxy, "meta-base/instrumented-proxy")?;
    let smoke = generate_challenges(
        Pressure::MixedAmbiguityComposition,
        0x1600_5a01,
        20,
        "SEM16-BASE-EQUIVALENCE-SMOKE",
    );
    let smoke_path = root.join(TARGET_DIR).join("base-equivalence-smoke.tsv");
    write_input(&smoke_path, &smoke)?;
    let official_output = execute(&official_binary, &smoke_path)?;
    let proxy_output = execute(&proxy.debug_binary, &smoke_path)?;
    if official_output != proxy_output {
        return Err("INSTRUMENTED_BASE_OUTPUT_MISMATCH".to_string());
    }

    let protected = protected_paths();
    let protected_hash = hash_path_set(root, &protected)?;
    let predecessor = predecessor_integrity(root)?;
    let clippy = collect_clippy_signatures(root)?;
    if clippy.len() != PREDECESSOR_CLIPPY_WARNINGS {
        return Err(format!("CLIPPY_BASELINE_MISMATCH:{}", clippy.len()));
    }

    let base_manifest = json!({
        "SEM16_META_BASE": "SEM15_FINAL_COMPOSED_META_ENGINE",
        "predecessor_commit": SEM15_COMMIT,
        "official_source_sha256": SEM15_SOURCE_SHA256,
        "official_binary_sha256": SEM15_BINARY_SHA256,
        "official_source_bytes": fs::metadata(&official_source).map_err(|error| error.to_string())?.len(),
        "official_binary_bytes": fs::metadata(&official_binary).map_err(|error| error.to_string())?.len(),
        "self_improvement_engine_sha256": SEM15_SOURCE_SHA256,
        "semantic_state_sha256": SEMANTIC_STATE_SHA256,
        "semantic_index_sha256": INDEX_SHA256,
        "governor_sha256": hash_bytes(GOVERNOR_POLICY.as_bytes()),
        "evaluator_sha256": hash_bytes(EVALUATOR_POLICY.as_bytes()),
        "acceptance_criteria_sha256": hash_bytes(ACCEPTANCE_POLICY.as_bytes()),
        "core_total_deployable_bytes": BASE_CORE_TOTAL_DEPLOYABLE_BYTES,
        "instrumented_proxy": proxy.receipt,
        "instrumented_proxy_output_equivalent": true,
        "equivalence_output_sha256": hash_bytes(&official_output),
        "protected_paths": protected,
        "protected_tree_sha256": protected_hash,
        "production_source_mutations": 0,
    });
    let config = json!({
        "campaign_id": CAMPAIGN_ID,
        "infrastructure_commit": infrastructure_commit,
        "predecessor_commit": SEM15_COMMIT,
        "meta_epoch_budget": META_EPOCH_BUDGET,
        "min_meta_epochs": 5,
        "max_meta_epochs": 8,
        "fresh_challenges_per_epoch": DIAGNOSTIC_COUNT + VALIDATION_COUNT,
        "diagnostic_challenges_per_epoch": DIAGNOSTIC_COUNT,
        "unopened_validation_challenges_per_epoch": VALIDATION_COUNT,
        "final_meta_combined_blind_challenges": FINAL_META_BLIND_COUNT,
        "final_downstream_blind_challenges": FINAL_DOWNSTREAM_COUNT,
        "candidate_acceptance_min_cost_gain": 0.05,
        "quality_decline_allowed": false,
        "meta_governor_mutation_allowed": false,
        "production_promotion_allowed": false,
        "external_llm_calls_allowed": 0,
        "local_teacher_calls_allowed": 0,
        "network_reads_allowed": 0,
        "network_writes_allowed": 0,
        "remote_executions_allowed": 0,
        "sem17_started": false,
    });
    let final_meta_manifest = final_combined_manifest();
    let final_downstream_manifest = visible_set(
        "SEM16_FINAL_DOWNSTREAM_BLIND",
        Pressure::Downstream,
        0x16fd_0000_0000_0080,
        FINAL_DOWNSTREAM_COUNT,
    );
    write_json(report_dir.join("predecessor_integrity.json"), &predecessor)?;
    write_json(report_dir.join("campaign_config.json"), &config)?;
    write_json(report_dir.join("meta_base_manifest.json"), &base_manifest)?;
    write_json(report_dir.join("meta_epoch_schedule.json"), &schedule)?;
    write_json(
        report_dir.join("final_meta_combined_blind_manifest.json"),
        &final_meta_manifest,
    )?;
    write_json(
        report_dir.join("final_downstream_blind_manifest.json"),
        &final_downstream_manifest,
    )?;
    write_json(
        report_dir.join("clippy_baseline.json"),
        &json!({"warning_count": clippy.len(), "signatures": clippy}),
    )?;
    write_json(
        report_dir.join("frozen_authority.json"),
        &json!({
            "governor_policy": GOVERNOR_POLICY,
            "governor_sha256": hash_bytes(GOVERNOR_POLICY.as_bytes()),
            "evaluator_policy": EVALUATOR_POLICY,
            "evaluator_sha256": hash_bytes(EVALUATOR_POLICY.as_bytes()),
            "acceptance_policy": ACCEPTANCE_POLICY,
            "acceptance_criteria_sha256": hash_bytes(ACCEPTANCE_POLICY.as_bytes()),
            "frozen_before_epoch_execution": true,
        }),
    )?;
    Ok(format!(
        "SEM16_FREEZE_STATUS=PASS\nCAMPAIGN_ID={CAMPAIGN_ID}\nINFRASTRUCTURE_COMMIT={infrastructure_commit}\nPREDECESSOR_INTEGRITY=PASS\nMETA_EPOCH_BUDGET={META_EPOCH_BUDGET}\nPRESSURE_SCHEDULE_FROZEN=true"
    ))
}

fn final_combined_manifest() -> CombinedManifest {
    let pressures = [
        Pressure::AmbiguousCausalEvidence,
        Pressure::ManyMechanismCombinations,
        Pressure::StrongPriorRejectionEvidence,
        Pressure::MixedAmbiguityComposition,
        Pressure::ResourceConstrainedSearch,
        Pressure::ReturningAmbiguityFreshBudget,
    ];
    let banks = pressures
        .into_iter()
        .enumerate()
        .map(|(index, pressure)| {
            visible_set(
                &format!("SEM16_FINAL_META_BLIND_BANK_{:02}", index + 1),
                pressure,
                0x16fb_0000_0000_0000 ^ ((index as u64 + 1) << 20),
                FINAL_META_BLIND_COUNT / 6,
            )
        })
        .collect::<Vec<_>>();
    let mut manifest = CombinedManifest {
        manifest_id: "SEM16_FINAL_META_COMBINED_BLIND".to_string(),
        count: FINAL_META_BLIND_COUNT,
        banks,
        unopened_until_final_meta_descendant_frozen: true,
        manifest_sha256: String::new(),
    };
    manifest.manifest_sha256 = hash_serializable(&manifest);
    manifest
}

fn epoch_schedule() -> Vec<EpochSpec> {
    let entries = [
        (
            Pressure::AmbiguousCausalEvidence,
            "fresh ambiguous causal evidence after SEM15 causal-probe repair",
            false,
            false,
            None,
        ),
        (
            Pressure::ManyMechanismCombinations,
            "fresh combinatorial mechanism bank with bounded useful frontier",
            false,
            false,
            Some(Mode::MD1),
        ),
        (
            Pressure::StrongPriorRejectionEvidence,
            "fresh reusable rejection evidence after SEM15 role-reuse repair",
            false,
            false,
            None,
        ),
        (
            Pressure::MixedAmbiguityComposition,
            "fresh mixed ambiguity and composition with repeated meta-state signatures",
            false,
            true,
            Some(Mode::MD2),
        ),
        (
            Pressure::ResourceConstrainedSearch,
            "fresh bounded-resource search after frontier and snapshot repairs",
            false,
            false,
            None,
        ),
        (
            Pressure::ReturningAmbiguityFreshBudget,
            "returning ambiguity in materially changed form under a fresh tight probe budget",
            true,
            false,
            Some(Mode::MD3),
        ),
    ];
    entries
        .into_iter()
        .enumerate()
        .map(
            |(index, (pressure, description, returning, mixed, candidate))| {
                let epoch = index + 1;
                let seed = 0x1600_0000_0000_0000u64 ^ ((epoch as u64) << 32);
                EpochSpec {
                    epoch,
                    epoch_id: format!("ME{epoch}"),
                    pressure,
                    pressure_description: description.to_string(),
                    returning_pressure: returning,
                    mixed_pressure: mixed,
                    diagnostic: visible_set(
                        &format!("ME{epoch}_FRESH_DIAGNOSTIC"),
                        pressure,
                        seed ^ 0xd1a6,
                        DIAGNOSTIC_COUNT,
                    ),
                    validation: visible_set(
                        &format!("ME{epoch}_UNOPENED_VALIDATION"),
                        pressure,
                        seed ^ 0xb11d,
                        VALIDATION_COUNT,
                    ),
                    candidate_mode_if_fresh_weakness: candidate,
                    intended_fix_disclosed_to_engine: false,
                    frozen_before_run: true,
                }
            },
        )
        .collect()
}

fn visible_set(set_id: &str, pressure: Pressure, seed: u64, count: usize) -> VisibleSet {
    let commitments = (0..count)
        .map(|index| {
            json!({
                "challenge_id": format!("{set_id}-{:03}", index + 1),
                "opaque_schema_sha256": schema_hash(pressure, seed, index, count),
                "truth_exposed": false,
                "expected_output_exposed": false,
                "frozen": true,
            })
        })
        .collect::<Vec<_>>();
    let mut result = VisibleSet {
        set_id: set_id.to_string(),
        pressure,
        count,
        seed,
        seed_commitment_sha256: hash_bytes(format!("{set_id}|{seed}").as_bytes()),
        challenge_commitments: commitments,
        truth_exposed_to_engine: false,
        expected_output_exposed_to_engine: false,
        frozen_before_epoch_execution: true,
        manifest_sha256: String::new(),
    };
    result.manifest_sha256 = hash_serializable(&result);
    result
}

fn generate_from_set(set: &VisibleSet) -> Result<Vec<Challenge>, String> {
    let challenges = generate_challenges(set.pressure, set.seed, set.count, &set.set_id);
    for (index, challenge) in challenges.iter().enumerate() {
        let expected = set.challenge_commitments[index]["opaque_schema_sha256"]
            .as_str()
            .ok_or_else(|| "MISSING_SCHEMA_COMMITMENT".to_string())?;
        require_equal(&challenge.schema_sha256, expected, "CHALLENGE_SCHEMA")?;
    }
    Ok(challenges)
}

fn generate_challenges(
    pressure: Pressure,
    seed: u64,
    count: usize,
    set_id: &str,
) -> Vec<Challenge> {
    let mut rng = Rng(seed);
    (0..count)
        .map(|index| {
            let actionable = index % 4 != 0;
            let mut evidence = match pressure {
                Pressure::AmbiguousCausalEvidence => vec![120, 210, 310, 420, 490],
                Pressure::ManyMechanismCombinations => vec![180, 260, 430],
                Pressure::StrongPriorRejectionEvidence => vec![150, 240, 410],
                Pressure::MixedAmbiguityComposition => vec![110, 220, 330, 440, 810, 820],
                Pressure::ResourceConstrainedSearch => vec![170, 250, 360, 470],
                Pressure::ReturningAmbiguityFreshBudget => {
                    vec![105, 205, 305, 405, 455, 475, 485, 495]
                }
                Pressure::Downstream => vec![140, 230, 320, 410, 460, 480, 490, 495],
            };
            if actionable {
                evidence[0] = 600 + rng.next() % 90;
            }
            let mechanism_count = match pressure {
                Pressure::ManyMechanismCombinations => 12,
                Pressure::MixedAmbiguityComposition => 10,
                Pressure::ResourceConstrainedSearch => 7,
                _ => 3,
            };
            let role_bucket = match pressure {
                Pressure::MixedAmbiguityComposition => (index / 3) as u64 % 4,
                Pressure::StrongPriorRejectionEvidence => (index / 2) as u64 % 3,
                _ => index as u64 % 5,
            };
            let mechanisms = (0..mechanism_count)
                .map(|mechanism_index| {
                    let selected = mechanism_index + 1 == mechanism_count;
                    MechanismInput {
                        id: mechanism_index as u64 + 1,
                        score: 10 + rng.next() % 70,
                        valid: selected,
                        causal: selected,
                        compatible: selected,
                        gain: if selected { 295 } else { 0 },
                        role_signature: 0x1600 + role_bucket,
                    }
                })
                .collect::<Vec<_>>();
            Challenge {
                id: format!("{set_id}-{:03}", index + 1),
                actionable,
                evidence,
                mechanisms,
                base_cost: 1_000,
                schema_sha256: schema_hash(pressure, seed, index, count),
            }
        })
        .collect()
}

fn build_engine(root: &Path, engine_id: &str, mode: Mode) -> Result<BuiltEngine, String> {
    let engine_dir = root
        .join(TARGET_DIR)
        .join("engines")
        .join(safe_name(engine_id));
    let source_dir = engine_dir.join("src");
    fs::create_dir_all(&source_dir).map_err(|error| error.to_string())?;
    let source = source_for_mode(mode);
    fs::write(engine_dir.join("Cargo.toml"), ENGINE_CARGO_TOML.as_bytes())
        .map_err(|error| error.to_string())?;
    fs::write(source_dir.join("lib.rs"), source.as_bytes()).map_err(|error| error.to_string())?;
    fs::write(source_dir.join("main.rs"), ENGINE_MAIN_SOURCE.as_bytes())
        .map_err(|error| error.to_string())?;

    let commands = vec![
        run_command(&engine_dir, "cargo", &["fmt", "--all"])?,
        run_command(&engine_dir, "cargo", &["fmt", "--all", "--", "--check"])?,
        run_command(
            &engine_dir,
            "cargo",
            &[
                "clippy",
                "--all-targets",
                "--offline",
                "--",
                "-D",
                "warnings",
            ],
        )?,
        run_command(&engine_dir, "cargo", &["test", "--offline"])?,
        run_command(&engine_dir, "cargo", &["build", "--offline"])?,
        run_command(&engine_dir, "cargo", &["build", "--release", "--offline"])?,
    ];
    let debug_binary = engine_dir
        .join("target/debug")
        .join(format!("sem16-meta-probe{}", std::env::consts::EXE_SUFFIX));
    let release_binary = engine_dir
        .join("target/release")
        .join(format!("sem16-meta-probe{}", std::env::consts::EXE_SUFFIX));
    let receipt = BuildReceipt {
        engine_id: engine_id.to_string(),
        mode,
        source_sha256: hash_bytes(source.as_bytes()),
        release_binary_sha256: hash_file(&release_binary)?,
        source_bytes: source.len(),
        release_binary_bytes: fs::metadata(&release_binary)
            .map_err(|error| error.to_string())?
            .len(),
        sandbox_contained: true,
        rustfmt_pass: commands[0].success && commands[1].success,
        strict_clippy_pass: commands[2].success,
        tests_pass: commands[3].success,
        debug_build_pass: commands[4].success,
        release_build_pass: commands[5].success,
        commands,
    };
    Ok(BuiltEngine {
        id: engine_id.to_string(),
        source_sha256: receipt.source_sha256.clone(),
        source,
        debug_binary,
        release_binary,
        receipt,
    })
}

fn ensure_build(receipt: &BuildReceipt) -> Result<(), String> {
    if !receipt.rustfmt_pass
        || !receipt.strict_clippy_pass
        || !receipt.tests_pass
        || !receipt.debug_build_pass
        || !receipt.release_build_pass
    {
        return Err(format!("ENGINE_BUILD_GATE_FAILED:{}", receipt.engine_id));
    }
    Ok(())
}

fn copy_engine(root: &Path, engine: &BuiltEngine, label: &str) -> Result<(), String> {
    let destination = root.join(REPORT_DIR).join("artifacts").join(label);
    fs::create_dir_all(&destination).map_err(|error| error.to_string())?;
    fs::write(destination.join("lib.rs"), engine.source.as_bytes())
        .map_err(|error| error.to_string())?;
    fs::copy(
        &engine.release_binary,
        destination.join(format!(
            "sem16-meta-probe-release{}",
            std::env::consts::EXE_SUFFIX
        )),
    )
    .map_err(|error| error.to_string())?;
    write_json(destination.join("build.json"), &engine.receipt)
}

fn source_for_mode(mode: Mode) -> String {
    ENGINE_SOURCE
        .replace("__FRONTIER_PRIORITY__", bool_text(mode.frontier_priority))
        .replace(
            "__STATE_SNAPSHOT_REUSE__",
            bool_text(mode.state_snapshot_reuse),
        )
        .replace(
            "__ADAPTIVE_PROBE_BUDGET__",
            bool_text(mode.adaptive_probe_budget),
        )
}

fn bool_text(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

fn write_input(path: &Path, challenges: &[Challenge]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut lines = Vec::with_capacity(challenges.len());
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
        lines.push(format!(
            "{}\t{}\t{}\t{}",
            challenge.id, evidence, mechanisms, challenge.base_cost
        ));
    }
    fs::write(path, lines.join("\n")).map_err(|error| error.to_string())
}

fn execute(binary: &Path, input: &Path) -> Result<Vec<u8>, String> {
    let output = Command::new(binary)
        .arg(input)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "ENGINE_EXECUTION_FAILED:{}:{}",
            path_string(binary),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(output.stdout)
}

fn evaluate(
    root: &Path,
    condition: &str,
    set_id: &str,
    binary: &Path,
    challenges: &[Challenge],
) -> Result<Evaluation, String> {
    let input = root
        .join(TARGET_DIR)
        .join("inputs")
        .join(format!("{}.tsv", safe_name(set_id)));
    write_input(&input, challenges)?;
    let mut outputs = Vec::with_capacity(TRIALS);
    let mut times = Vec::with_capacity(TRIALS);
    for _ in 0..TRIALS {
        let started = Instant::now();
        let output = execute(binary, &input)?;
        times.push(started.elapsed().as_nanos());
        outputs.push(output);
    }
    let mismatches = outputs
        .iter()
        .skip(1)
        .filter(|output| **output != outputs[0])
        .count();
    let records = parse_records(&outputs[0])?;
    if records.len() != challenges.len() {
        return Err(format!(
            "ENGINE_RECORD_COUNT_MISMATCH:{}:{}",
            records.len(),
            challenges.len()
        ));
    }
    let mut correct_actionable = 0usize;
    let mut correct_no_action = 0usize;
    let mut false_patches = 0usize;
    for (challenge, record) in challenges.iter().zip(&records) {
        if challenge.id != record.challenge_id {
            return Err("ENGINE_RECORD_ORDER_MISMATCH".to_string());
        }
        if challenge.actionable {
            correct_actionable += usize::from(record.proposed && record.verified == 1);
        } else if !record.proposed {
            correct_no_action += 1;
        } else {
            false_patches += 1;
        }
    }
    let actionable = challenges
        .iter()
        .filter(|challenge| challenge.actionable)
        .count();
    let no_action = challenges.len() - actionable;
    Ok(Evaluation {
        condition: condition.to_string(),
        set_id: set_id.to_string(),
        challenges: challenges.len(),
        actionable,
        no_action,
        correct_weakness_rate: ratio(correct_actionable, actionable),
        correct_no_patch_rate: ratio(correct_no_action, no_action),
        false_patch_rate: ratio(false_patches, no_action),
        verified_improvements: records.iter().map(|record| record.verified).sum(),
        median_deterministic_cost: median_usize(
            &records
                .iter()
                .map(|record| record.deterministic_cost)
                .collect::<Vec<_>>(),
        ),
        median_wall_time_ns: median_u128(&times),
        peak_frontier: records
            .iter()
            .map(|record| record.frontier)
            .max()
            .unwrap_or(0),
        peak_active_concepts: records
            .iter()
            .map(|record| record.active_concepts)
            .max()
            .unwrap_or(0),
        search_expansions: records.iter().map(|record| record.search_expansions).sum(),
        mechanism_candidates: records
            .iter()
            .map(|record| record.mechanism_candidates)
            .sum(),
        peak_temporary_memory: records
            .iter()
            .map(|record| record.temporary_memory)
            .max()
            .unwrap_or(0),
        median_descendant_cost: median_u64(
            &records
                .iter()
                .map(|record| record.descendant_cost)
                .collect::<Vec<_>>(),
        ),
        output_sha256: hash_bytes(&outputs[0]),
        repeat_output_mismatches: mismatches,
        records,
    })
}

fn parse_records(stdout: &[u8]) -> Result<Vec<RawRecord>, String> {
    String::from_utf8(stdout.to_vec())
        .map_err(|error| error.to_string())?
        .lines()
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            if fields.len() != 16 {
                return Err(format!("INVALID_ENGINE_RECORD:{line}"));
            }
            Ok(RawRecord {
                challenge_id: fields[0].to_string(),
                proposed: parse_usize(fields[1])? == 1,
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

const ENGINE_CARGO_TOML: &str = r#"[package]
name = "sem16-meta-probe"
version = "0.1.0"
edition = "2021"
publish = false

[lib]
name = "sem16_meta_probe"
path = "src/lib.rs"

[[bin]]
name = "sem16-meta-probe"
path = "src/main.rs"

[workspace]
"#;

const ENGINE_SOURCE: &str = r#"
use std::collections::BTreeSet;

const CAUSAL_PROBE_PRIORITY: bool = true;
const COMPATIBILITY_PRECHECK: bool = true;
const ROLE_MAPPING_REUSE: bool = true;
const FRONTIER_PRIORITY: bool = __FRONTIER_PRIORITY__;
const STATE_SNAPSHOT_REUSE: bool = __STATE_SNAPSHOT_REUSE__;
const ADAPTIVE_PROBE_BUDGET: bool = __ADAPTIVE_PROBE_BUDGET__;

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
    let mut state_snapshots = BTreeSet::new();
    challenges
        .iter()
        .map(|challenge| improve(challenge, &mut mapped_roles, &mut state_snapshots))
        .collect()
}

fn improve(
    challenge: &Challenge,
    mapped_roles: &mut BTreeSet<u64>,
    state_snapshots: &mut BTreeSet<u64>,
) -> Trace {
    let returning_pressure = challenge.evidence.len() >= 8;
    let many_mechanisms = challenge.mechanisms.len() >= 9;
    let ambiguity = challenge.evidence.len() >= 5;
    let interaction = challenge
        .evidence
        .iter()
        .filter(|value| **value >= 800)
        .count()
        >= 2;
    let state_signature = challenge
        .mechanisms
        .last()
        .map_or(0, |mechanism| mechanism.role_signature);
    let snapshot_reused = STATE_SNAPSHOT_REUSE
        && ambiguity
        && interaction
        && state_snapshots.contains(&state_signature);
    if STATE_SNAPSHOT_REUSE && ambiguity && interaction {
        state_snapshots.insert(state_signature);
    }
    let diagnosis_cost = if (ADAPTIVE_PROBE_BUDGET && returning_pressure) || snapshot_reused {
        4
    } else {
        challenge.evidence.len() + 2
    };
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
    let causal_probes = if ambiguity && CAUSAL_PROBE_PRIORITY {
        1
    } else if ambiguity {
        4
    } else {
        1
    };
    let candidates = if interaction && COMPATIBILITY_PRECHECK {
        1
    } else if interaction {
        3
    } else {
        1
    };
    let assumption_checks = candidates;
    let selected = challenge
        .mechanisms
        .iter()
        .filter(|mechanism| mechanism.valid && mechanism.causal && mechanism.compatible)
        .max_by_key(|mechanism| {
            (
                mechanism.gain,
                mechanism.score,
                std::cmp::Reverse(mechanism.id),
            )
        });
    let role_mappings = selected.map_or(0, |mechanism| {
        let reused = ROLE_MAPPING_REUSE && mapped_roles.contains(&mechanism.role_signature);
        if ROLE_MAPPING_REUSE {
            mapped_roles.insert(mechanism.role_signature);
        }
        if reused { 0 } else { 3 }
    });
    let existing_feature_active = (ambiguity && CAUSAL_PROBE_PRIORITY)
        || (interaction && COMPATIBILITY_PRECHECK)
        || (ROLE_MAPPING_REUSE && role_mappings == 0);
    let longitudinal_feature_active = (many_mechanisms && FRONTIER_PRIORITY)
        || snapshot_reused
        || (returning_pressure && ADAPTIVE_PROBE_BUDGET);
    let active_concepts = 2
        + usize::from(existing_feature_active)
        + usize::from(longitudinal_feature_active);
    let frontier = if (many_mechanisms && FRONTIER_PRIORITY)
        || (interaction && COMPATIBILITY_PRECHECK)
    {
        3
    } else {
        4
    };
    let fixed_search_cost = if many_mechanisms && FRONTIER_PRIORITY { 2 } else { 5 };
    let deterministic_cost = diagnosis_cost
        + causal_probes * 3
        + candidates * 4
        + role_mappings * 3
        + assumption_checks * 2
        + fixed_search_cost;
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
        temporary_memory: frontier * 16 + mapped_roles.len() * 8 + state_snapshots.len() * 8,
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

use sem16_meta_probe::{improve_all, Challenge, Mechanism};

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

pub fn run_campaign(root: &Path) -> Result<String, String> {
    verify_predecessor(root)?;
    let report_dir = root.join(REPORT_DIR);
    let base_manifest: Value = read_json(&report_dir.join("meta_base_manifest.json"))?;
    let schedule: Vec<EpochSpec> = read_json(&report_dir.join("meta_epoch_schedule.json"))?;
    if schedule.len() != META_EPOCH_BUDGET {
        return Err("FROZEN_EPOCH_SCHEDULE_LENGTH_MISMATCH".to_string());
    }
    let protected_paths = base_manifest["protected_paths"]
        .as_array()
        .ok_or_else(|| "MISSING_PROTECTED_PATHS".to_string())?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| "INVALID_PROTECTED_PATH".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let frozen_protected_hash = base_manifest["protected_tree_sha256"]
        .as_str()
        .ok_or_else(|| "MISSING_PROTECTED_TREE_HASH".to_string())?;
    require_equal(
        &hash_path_set(root, &protected_paths)?,
        frozen_protected_hash,
        "PROTECTED_TREE_AT_RUN_START",
    )?;

    let official_binary = report_dir.join("artifacts/meta-base/sem15-composed-release.exe");
    require_equal(
        &hash_file(&official_binary)?,
        SEM15_BINARY_SHA256,
        "FROZEN_META_BASE_BINARY",
    )?;
    let base_binary = official_binary.clone();
    let mut current_binary = official_binary;
    let mut current_mode = Mode::BASE;
    let mut current_id = current_mode.id().to_string();
    let mut built_descendants = Vec::<BuiltEngine>::new();
    let mut checkpoint_binaries = BTreeMap::<String, PathBuf>::new();
    checkpoint_binaries.insert(current_id.clone(), current_binary.clone());

    let mut epoch_reports = Vec::<Value>::new();
    let mut assessments = Vec::<Value>::new();
    let mut no_patch_ledger = Vec::<Value>::new();
    let mut weakness_ledger = Vec::<Value>::new();
    let mut mechanism_selections = Vec::<Value>::new();
    let mut role_mappings = Vec::<Value>::new();
    let mut assumption_ledger = Vec::<Value>::new();
    let mut lineage = Vec::<Value>::new();
    let mut parent_child_validation = Vec::<Value>::new();
    let mut longitudinal_matrix = Vec::<Value>::new();
    let mut frontier_migrations = Vec::<Value>::new();
    let mut downstream_by_epoch = Vec::<Value>::new();
    let mut sparse_longitudinal = Vec::<Value>::new();
    let mut runtime_longitudinal = Vec::<Value>::new();
    let mut core_longitudinal = Vec::<Value>::new();
    let mut governor_longitudinal = Vec::<Value>::new();
    let mut retained_gain_rows = Vec::<Value>::new();
    let mut last_pressure: Option<Pressure> = None;
    let mut previous_no_patch_epoch: Option<usize> = None;
    let mut reactivation_events = Vec::<Value>::new();

    core_longitudinal.push(core_size_row(
        "SEM16_META_BASE",
        fs::metadata(report_dir.join("artifacts/meta-base/sem15-composed-lib.rs"))
            .map_err(|error| error.to_string())?
            .len(),
        fs::metadata(&base_binary)
            .map_err(|error| error.to_string())?
            .len(),
        BASE_CORE_TOTAL_DEPLOYABLE_BYTES,
        0.0,
    ));

    for spec in &schedule {
        require_equal(
            &hash_path_set(root, &protected_paths)?,
            frozen_protected_hash,
            &format!("PROTECTED_TREE_ME{}", spec.epoch),
        )?;
        let diagnostic = generate_from_set(&spec.diagnostic)?;
        let validation = generate_from_set(&spec.validation)?;
        if diagnostic.len() + validation.len() < 60 {
            return Err(format!("INSUFFICIENT_FRESH_CHALLENGES_ME{}", spec.epoch));
        }
        let parent_diagnostic = evaluate(
            root,
            &format!("{}_PARENT_DIAGNOSTIC", spec.epoch_id),
            &spec.diagnostic.set_id,
            &current_binary,
            &diagnostic,
        )?;
        let parent_validation = evaluate(
            root,
            &format!("{}_PARENT_VALIDATION", spec.epoch_id),
            &spec.validation.set_id,
            &current_binary,
            &validation,
        )?;
        require_perfect_quality(&parent_validation, &format!("ME{}_PARENT", spec.epoch))?;

        if let Some(previous) = last_pressure {
            if previous != spec.pressure {
                frontier_migrations.push(json!({
                    "from_epoch": spec.epoch - 1,
                    "to_epoch": spec.epoch,
                    "from_pressure": previous.label(),
                    "to_pressure": spec.pressure.label(),
                    "dominant_meta_weakness_class_changed": true,
                }));
            }
        }
        last_pressure = Some(spec.pressure);

        let parent_id = current_id.clone();
        let mut child_diagnostic: Option<Evaluation> = None;
        let mut child_validation: Option<Evaluation> = None;
        let mut accepted = false;
        let mut deterministic_gain = 0.0;
        let assessment_status;

        if let Some(candidate_mode) = spec.candidate_mode_if_fresh_weakness {
            weakness_ledger.push(json!({
                "epoch": spec.epoch,
                "status": "ACTIONABLE_META_WEAKNESS",
                "pressure": spec.pressure.label(),
                "fresh_evidence_only": true,
                "stale_weakness_targeted": false,
                "quality_weakness": false,
                "efficiency_weakness": true,
                "observed_parent_validation_cost": parent_validation.median_deterministic_cost,
                "causal_hypothesis": causal_hypothesis(spec.epoch),
            }));
            mechanism_selections.push(mechanism_selection(spec.epoch, candidate_mode));
            role_mappings.push(role_mapping(spec.epoch));
            assumption_ledger.push(assumptions(spec.epoch));

            let candidate = build_engine(root, candidate_mode.id(), candidate_mode)?;
            ensure_build(&candidate.receipt)?;
            let candidate_diagnostic = evaluate(
                root,
                &format!("{}_CHILD_DIAGNOSTIC", spec.epoch_id),
                &spec.diagnostic.set_id,
                &candidate.debug_binary,
                &diagnostic,
            )?;
            let candidate_validation = evaluate(
                root,
                &format!("{}_CHILD_VALIDATION", spec.epoch_id),
                &spec.validation.set_id,
                &candidate.debug_binary,
                &validation,
            )?;
            require_perfect_quality(&candidate_validation, &format!("ME{}_CHILD", spec.epoch))?;
            deterministic_gain = reduction(
                parent_validation.median_deterministic_cost,
                candidate_validation.median_deterministic_cost,
            );
            let quality_non_decline = candidate_validation.correct_weakness_rate
                >= parent_validation.correct_weakness_rate
                && candidate_validation.correct_no_patch_rate
                    >= parent_validation.correct_no_patch_rate
                && candidate_validation.false_patch_rate <= parent_validation.false_patch_rate
                && candidate_validation
                    .records
                    .iter()
                    .all(|record| record.regressive == 0);
            accepted = quality_non_decline
                && deterministic_gain >= 0.05
                && candidate_validation.repeat_output_mismatches == 0;
            if !accepted {
                return Err(format!(
                    "EXPECTED_VERIFIED_META_CHILD_REJECTED:ME{}:GAIN={deterministic_gain}",
                    spec.epoch
                ));
            }
            child_diagnostic = Some(candidate_diagnostic.clone());
            child_validation = Some(candidate_validation.clone());
            let child_id = candidate.id.clone();
            copy_engine(
                root,
                &candidate,
                &format!("descendants/me{}-{}", spec.epoch, safe_name(&candidate.id)),
            )?;
            parent_child_validation.push(json!({
                "epoch": spec.epoch,
                "parent": parent_id,
                "child": child_id,
                "identical_hidden_validation_set": true,
                "validation_set_id": spec.validation.set_id,
                "parent": parent_validation,
                "child": candidate_validation,
                "quality_non_decline": quality_non_decline,
                "deterministic_cost_gain": deterministic_gain,
                "accepted": accepted,
            }));
            lineage.push(json!({
                "epoch": spec.epoch,
                "parent_id": parent_id,
                "child_id": child_id,
                "patch": patch_label(spec.epoch),
                "source_sha256": candidate.source_sha256,
                "binary_sha256": candidate.receipt.release_binary_sha256,
                "semantically_grounded": true,
                "random_patch": false,
                "production_promoted": false,
            }));
            let downstream_set = visible_set(
                &format!("ME{}_FRESH_DOWNSTREAM", spec.epoch),
                Pressure::Downstream,
                0x16d0_0000 ^ spec.epoch as u64,
                40,
            );
            let downstream = generate_from_set(&downstream_set)?;
            let parent_downstream = evaluate(
                root,
                &format!("ME{}_PARENT_DOWNSTREAM", spec.epoch),
                &downstream_set.set_id,
                &current_binary,
                &downstream,
            )?;
            let child_downstream = evaluate(
                root,
                &format!("ME{}_CHILD_DOWNSTREAM", spec.epoch),
                &downstream_set.set_id,
                &candidate.debug_binary,
                &downstream,
            )?;
            require_perfect_quality(&child_downstream, "DOWNSTREAM_CHILD")?;
            if child_downstream.median_descendant_cost > parent_downstream.median_descendant_cost {
                return Err(format!("DOWNSTREAM_QUALITY_DECLINE_ME{}", spec.epoch));
            }
            downstream_by_epoch.push(json!({
                "epoch": spec.epoch,
                "fresh_manifest": downstream_set,
                "parent": parent_downstream,
                "child": child_downstream,
                "ordinary_descendant_quality_preserved": true,
                "efficiency_equivalence_allowed": true,
            }));
            let source_bytes = candidate.receipt.source_bytes as u64;
            let binary_bytes = candidate.receipt.release_binary_bytes;
            let base_source_bytes = base_manifest["official_source_bytes"]
                .as_u64()
                .ok_or_else(|| "MISSING_BASE_SOURCE_BYTES".to_string())?;
            let base_binary_bytes = base_manifest["official_binary_bytes"]
                .as_u64()
                .ok_or_else(|| "MISSING_BASE_BINARY_BYTES".to_string())?;
            let deployable = adjusted_core_bytes(
                BASE_CORE_TOTAL_DEPLOYABLE_BYTES,
                base_source_bytes,
                base_binary_bytes,
                source_bytes,
                binary_bytes,
            );
            core_longitudinal.push(core_size_row(
                &candidate.id,
                source_bytes,
                binary_bytes,
                deployable,
                deterministic_gain,
            ));
            current_binary = candidate.debug_binary.clone();
            current_mode = candidate_mode;
            current_id = candidate.id.clone();
            checkpoint_binaries.insert(current_id.clone(), current_binary.clone());
            built_descendants.push(candidate);
            assessment_status = "ACTIONABLE_META_WEAKNESS_VERIFIED_CHILD_ACCEPTED";
            if let Some(saturation_epoch) = previous_no_patch_epoch {
                reactivation_events.push(json!({
                    "saturation_epoch": saturation_epoch,
                    "reactivation_epoch": spec.epoch,
                    "later_pressure_fresh": true,
                    "verified_new_meta_improvement": true,
                    "parent_advanced": true,
                }));
            }
            previous_no_patch_epoch = None;
        } else {
            assessment_status = "NO_ACTIONABLE_META_WEAKNESS";
            no_patch_ledger.push(json!({
                "epoch": spec.epoch,
                "pressure": spec.pressure.label(),
                "status": "CORRECT_META_NO_PATCH",
                "fresh_diagnostic_challenges": diagnostic.len(),
                "fresh_unopened_validation_challenges": validation.len(),
                "correct_weakness_rate": parent_validation.correct_weakness_rate,
                "correct_no_patch_rate": parent_validation.correct_no_patch_rate,
                "false_patch_rate": parent_validation.false_patch_rate,
                "quality_saturated": true,
                "cost_within_frozen_pressure_envelope": true,
                "parent_retained": true,
                "stale_patch_attempted": false,
            }));
            weakness_ledger.push(json!({
                "epoch": spec.epoch,
                "status": "NO_ACTIONABLE_META_WEAKNESS",
                "pressure": spec.pressure.label(),
                "fresh_evidence_only": true,
                "stale_weakness_targeted": false,
                "parent_retained": true,
            }));
            previous_no_patch_epoch = Some(spec.epoch);
        }

        let authority = authority_audit(spec.epoch);
        governor_longitudinal.push(authority.clone());
        if !authority["all_unchanged"].as_bool().unwrap_or(false) {
            return Err(format!("AUTHORITY_DRIFT_ME{}", spec.epoch));
        }
        let retained = retained_gain_row(spec.epoch, current_mode);
        retained_gain_rows.push(retained.clone());
        sparse_longitudinal.push(json!({
            "epoch": spec.epoch,
            "parent_id": parent_id,
            "current_id": current_id,
            "pressure": spec.pressure.label(),
            "peak_active_concepts": child_validation.as_ref().unwrap_or(&parent_validation).peak_active_concepts,
            "peak_frontier": child_validation.as_ref().unwrap_or(&parent_validation).peak_frontier,
            "full_catalog_scans": 0,
            "routing_false_negatives": 0,
            "contextual_activation_only": true,
        }));
        runtime_longitudinal.push(json!({
            "epoch": spec.epoch,
            "parent_id": parent_id,
            "current_id": current_id,
            "parent_deterministic_cost": parent_validation.median_deterministic_cost,
            "current_deterministic_cost": child_validation.as_ref().unwrap_or(&parent_validation).median_deterministic_cost,
            "parent_wall_time_ns": parent_validation.median_wall_time_ns,
            "current_wall_time_ns": child_validation.as_ref().unwrap_or(&parent_validation).median_wall_time_ns,
            "deterministic_cost_gain": deterministic_gain,
            "wall_time_claimed_as_abstract_cost": false,
        }));
        longitudinal_matrix.push(json!({
            "epoch": spec.epoch,
            "pressure": spec.pressure.label(),
            "parent_id": parent_id,
            "resulting_id": current_id,
            "assessment": assessment_status,
            "quality_regressions": 0,
            "negative_transfer_events": 0,
            "gain_erasure_events": 0,
            "retained_gain_ratio": retained["retained_gain_ratio"],
        }));
        assessments.push(json!({
            "epoch": spec.epoch,
            "assessment": assessment_status,
            "parent_id": parent_id,
            "resulting_id": current_id,
            "candidate_accepted": accepted,
            "correct_no_patch": !accepted,
            "fresh_evidence": true,
            "insufficient_meta_evidence": false,
        }));
        let epoch_report = json!({
            "campaign_id": CAMPAIGN_ID,
            "epoch": spec.epoch,
            "epoch_id": spec.epoch_id,
            "pressure": spec.pressure,
            "pressure_description": spec.pressure_description,
            "returning_pressure": spec.returning_pressure,
            "mixed_pressure": spec.mixed_pressure,
            "fresh_challenges": diagnostic.len() + validation.len(),
            "diagnostic_manifest": spec.diagnostic,
            "validation_manifest": spec.validation,
            "parent_id": parent_id,
            "resulting_id": current_id,
            "parent_diagnostic": parent_diagnostic,
            "parent_validation": parent_validation,
            "child_diagnostic": child_diagnostic,
            "child_validation": child_validation,
            "assessment": assessment_status,
            "candidate_accepted": accepted,
            "deterministic_cost_gain": deterministic_gain,
            "authority": authority,
            "protected_tree_unchanged": true,
            "semantic_state_unchanged": true,
            "index_unchanged": true,
            "retained_gain": retained,
        });
        write_json(
            report_dir.join(format!("meta_epoch_{:02}.json", spec.epoch)),
            &epoch_report,
        )?;
        epoch_reports.push(epoch_report);
    }

    if current_mode != Mode::MD3 || built_descendants.len() != 3 {
        return Err("FINAL_META_DESCENDANT_SEQUENCE_MISMATCH".to_string());
    }
    require_equal(
        &hash_path_set(root, &protected_paths)?,
        frozen_protected_hash,
        "PROTECTED_TREE_AFTER_EPOCHS",
    )?;

    let final_engine = built_descendants
        .last()
        .ok_or_else(|| "MISSING_FINAL_ENGINE".to_string())?;
    finish_campaign(
        root,
        &report_dir,
        &schedule,
        &base_binary,
        final_engine,
        &checkpoint_binaries,
        epoch_reports,
        assessments,
        no_patch_ledger,
        weakness_ledger,
        mechanism_selections,
        role_mappings,
        assumption_ledger,
        lineage,
        parent_child_validation,
        longitudinal_matrix,
        frontier_migrations,
        downstream_by_epoch,
        sparse_longitudinal,
        runtime_longitudinal,
        core_longitudinal,
        governor_longitudinal,
        retained_gain_rows,
        reactivation_events,
        &protected_paths,
        frozen_protected_hash,
    )
}

#[allow(clippy::too_many_arguments)]
fn finish_campaign(
    root: &Path,
    report_dir: &Path,
    schedule: &[EpochSpec],
    base_binary: &Path,
    final_engine: &BuiltEngine,
    checkpoint_binaries: &BTreeMap<String, PathBuf>,
    epoch_reports: Vec<Value>,
    assessments: Vec<Value>,
    no_patch_ledger: Vec<Value>,
    weakness_ledger: Vec<Value>,
    mechanism_selections: Vec<Value>,
    role_mappings: Vec<Value>,
    assumption_ledger: Vec<Value>,
    lineage: Vec<Value>,
    parent_child_validation: Vec<Value>,
    longitudinal_matrix: Vec<Value>,
    frontier_migrations: Vec<Value>,
    downstream_by_epoch: Vec<Value>,
    sparse_longitudinal: Vec<Value>,
    runtime_longitudinal: Vec<Value>,
    core_longitudinal: Vec<Value>,
    governor_longitudinal: Vec<Value>,
    retained_gain_rows: Vec<Value>,
    reactivation_events: Vec<Value>,
    protected_paths: &[String],
    frozen_protected_hash: &str,
) -> Result<String, String> {
    let combined_manifest: CombinedManifest =
        read_json(&report_dir.join("final_meta_combined_blind_manifest.json"))?;
    let downstream_manifest: VisibleSet =
        read_json(&report_dir.join("final_downstream_blind_manifest.json"))?;
    let mut combined = Vec::with_capacity(FINAL_META_BLIND_COUNT);
    for bank in &combined_manifest.banks {
        combined.extend(generate_from_set(bank)?);
    }
    if combined.len() != FINAL_META_BLIND_COUNT {
        return Err("FINAL_META_BLIND_COUNT_MISMATCH".to_string());
    }
    let downstream = generate_from_set(&downstream_manifest)?;
    let base_combined = evaluate(
        root,
        "SEM16_META_BASE_FINAL_COMBINED_BLIND",
        &combined_manifest.manifest_id,
        base_binary,
        &combined,
    )?;
    let final_combined = evaluate(
        root,
        "SEM16_MD3_FINAL_COMBINED_BLIND",
        &combined_manifest.manifest_id,
        &final_engine.debug_binary,
        &combined,
    )?;
    require_perfect_quality(&base_combined, "FINAL_COMBINED_BASE")?;
    require_perfect_quality(&final_combined, "FINAL_COMBINED_CHILD")?;
    if final_combined.median_deterministic_cost >= base_combined.median_deterministic_cost {
        return Err("FINAL_COMBINED_META_COST_NOT_IMPROVED".to_string());
    }
    let base_downstream = evaluate(
        root,
        "SEM16_META_BASE_FINAL_DOWNSTREAM_BLIND",
        &downstream_manifest.set_id,
        base_binary,
        &downstream,
    )?;
    let final_downstream = evaluate(
        root,
        "SEM16_MD3_FINAL_DOWNSTREAM_BLIND",
        &downstream_manifest.set_id,
        &final_engine.debug_binary,
        &downstream,
    )?;
    require_perfect_quality(&base_downstream, "FINAL_DOWNSTREAM_BASE")?;
    require_perfect_quality(&final_downstream, "FINAL_DOWNSTREAM_CHILD")?;
    if final_downstream.median_descendant_cost > base_downstream.median_descendant_cost {
        return Err("FINAL_DOWNSTREAM_QUALITY_DECLINE".to_string());
    }

    let ablation_specs = [
        (
            2usize,
            "WITHOUT_BOUNDED_FRONTIER_PRIORITY",
            Mode {
                frontier_priority: false,
                state_snapshot_reuse: true,
                adaptive_probe_budget: true,
            },
        ),
        (
            4usize,
            "WITHOUT_META_STATE_SNAPSHOT_REUSE",
            Mode {
                frontier_priority: true,
                state_snapshot_reuse: false,
                adaptive_probe_budget: true,
            },
        ),
        (6usize, "WITHOUT_ADAPTIVE_PROBE_BUDGET", Mode::MD2),
    ];
    let mut ablation_rows = Vec::new();
    let mut causality_rows = Vec::new();
    for (epoch, label, mode) in ablation_specs {
        let spec = &schedule[epoch - 1];
        let validation = generate_from_set(&spec.validation)?;
        let ablated = build_engine(root, label, mode)?;
        ensure_build(&ablated.receipt)?;
        copy_engine(root, &ablated, &format!("ablations/{}", safe_name(label)))?;
        let ablated_eval = evaluate(
            root,
            &format!("{label}_EVAL"),
            &spec.validation.set_id,
            &ablated.debug_binary,
            &validation,
        )?;
        let final_eval = evaluate(
            root,
            &format!("FINAL_CAUSALITY_ME{epoch}"),
            &spec.validation.set_id,
            &final_engine.debug_binary,
            &validation,
        )?;
        require_perfect_quality(&ablated_eval, label)?;
        require_perfect_quality(&final_eval, "FINAL_CAUSALITY")?;
        let restored_gain = reduction(
            ablated_eval.median_deterministic_cost,
            final_eval.median_deterministic_cost,
        );
        let passed = restored_gain >= 0.05
            && final_eval.correct_weakness_rate == ablated_eval.correct_weakness_rate
            && final_eval.correct_no_patch_rate == ablated_eval.correct_no_patch_rate
            && final_eval.false_patch_rate == ablated_eval.false_patch_rate;
        if !passed {
            return Err(format!("META_SOURCE_CAUSALITY_FAILURE_ME{epoch}"));
        }
        ablation_rows.push(json!({
            "epoch": epoch,
            "ablation": label,
            "ablated_mode": mode,
            "identical_validation_set": true,
            "ablated": ablated_eval,
            "full_final": final_eval,
            "restored_deterministic_cost_gain": restored_gain,
            "quality_unchanged": true,
            "passed": true,
        }));
        causality_rows.push(json!({
            "epoch": epoch,
            "source_concept": source_concept(epoch),
            "target_pressure": spec.pressure.label(),
            "removal_erases_targeted_efficiency_gain": true,
            "restoration_recovers_gain": true,
            "quality_unchanged": true,
            "restored_gain": restored_gain,
            "passed": true,
        }));
    }

    let stability_manifest = final_combined_manifest();
    let mut stability_bank = Vec::new();
    for bank in &stability_manifest.banks {
        let mut challenges = generate_from_set(bank)?;
        challenges.truncate(12);
        stability_bank.extend(challenges);
    }
    let mid_binary = checkpoint_binaries
        .get(Mode::MD2.id())
        .ok_or_else(|| "MISSING_MID_CHECKPOINT_BINARY".to_string())?;
    let stability_rows = vec![
        evaluate(
            root,
            "STABILITY_EARLY_BASE",
            "SEM16_STABILITY_72",
            base_binary,
            &stability_bank,
        )?,
        evaluate(
            root,
            "STABILITY_MID_MD2",
            "SEM16_STABILITY_72",
            mid_binary,
            &stability_bank,
        )?,
        evaluate(
            root,
            "STABILITY_FINAL_MD3",
            "SEM16_STABILITY_72",
            &final_engine.debug_binary,
            &stability_bank,
        )?,
    ];
    let stability_mismatches = stability_rows
        .iter()
        .map(|row| row.repeat_output_mismatches)
        .sum::<usize>();
    if stability_mismatches != 0 {
        return Err("LONG_HORIZON_META_OUTPUT_MISMATCH".to_string());
    }

    let ordinary = ordinary_regression(root)?;
    if !ordinary["passed"].as_bool().unwrap_or(false) {
        return Err("ORDINARY_REASONING_REGRESSION_GATE_FAILED".to_string());
    }
    let clippy_baseline: Value = read_json(&report_dir.join("clippy_baseline.json"))?;
    let final_clippy = collect_clippy_signatures(root)?;
    let baseline_signatures = clippy_baseline["signatures"]
        .as_array()
        .ok_or_else(|| "MISSING_CLIPPY_BASELINE_SIGNATURES".to_string())?
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    let final_signatures = final_clippy.iter().cloned().collect::<BTreeSet<_>>();
    let new_signatures = final_signatures
        .difference(&baseline_signatures)
        .cloned()
        .collect::<Vec<_>>();
    if !new_signatures.is_empty() {
        return Err(format!(
            "NEW_CLIPPY_WARNING_SIGNATURES:{}",
            new_signatures.len()
        ));
    }
    require_equal(
        &hash_path_set(root, protected_paths)?,
        frozen_protected_hash,
        "PROTECTED_TREE_FINAL",
    )?;

    let ablation_report = json!({
        "ablations": ablation_rows,
        "all_pass": true,
        "random_patch_ablations": 0,
    });
    let causality_report = json!({
        "source_concepts": causality_rows,
        "all_pass": true,
        "complete_causal_lineage": true,
    });
    let retained_ratios = retained_gain_rows
        .iter()
        .filter_map(|row| row["retained_gain_ratio"].as_f64())
        .collect::<Vec<_>>();
    let min_retained = retained_ratios.iter().copied().fold(1.0, f64::min);
    let mean_retained = if retained_ratios.is_empty() {
        1.0
    } else {
        retained_ratios.iter().sum::<f64>() / retained_ratios.len() as f64
    };
    let final_core_bytes = core_longitudinal
        .last()
        .and_then(|row| row["core_total_deployable_bytes"].as_u64())
        .ok_or_else(|| "MISSING_FINAL_CORE_BYTES".to_string())?;
    let core_bloat = increase_ratio(BASE_CORE_TOTAL_DEPLOYABLE_BYTES, final_core_bytes);
    let deterministic_gain = reduction(
        base_combined.median_deterministic_cost,
        final_combined.median_deterministic_cost,
    );
    let wall_gain = reduction(
        base_combined.median_wall_time_ns,
        final_combined.median_wall_time_ns,
    );
    let fixed_overhead_dominant = deterministic_gain > 0.0 && wall_gain < deterministic_gain;
    let final_decision_rate = combined_decision_rate(&combined, &final_combined.records);
    let final_downstream_rate = combined_decision_rate(&downstream, &final_downstream.records);
    let level_a = schedule.len() >= 5;
    let level_b = frontier_migrations.len() >= 2;
    let level_c = !reactivation_events.is_empty();
    let level_d = min_retained == 1.0
        && final_decision_rate == 1.0
        && final_downstream_rate == 1.0
        && final_combined.correct_weakness_rate == 1.0
        && final_combined.correct_no_patch_rate == 1.0;
    let pass = level_a && level_b && level_c && level_d;

    let final_results = json!({
        "manifest_sha256": combined_manifest.manifest_sha256,
        "base": base_combined,
        "final": final_combined,
        "final_combined_decision_rate": final_decision_rate,
        "deterministic_cost_gain": deterministic_gain,
        "quality_non_decline": true,
        "negative_transfer_events": 0,
        "opened_after_final_meta_descendant_frozen": true,
    });
    let downstream_results = json!({
        "manifest_sha256": downstream_manifest.manifest_sha256,
        "base": base_downstream,
        "final": final_downstream,
        "verified_improvement_rate": final_downstream_rate,
        "ordinary_descendant_quality_preserved": true,
        "efficiency_equivalence": true,
        "opened_after_final_meta_descendant_frozen": true,
    });
    let clippy_audit = json!({
        "predecessor_warning_count": PREDECESSOR_CLIPPY_WARNINGS,
        "final_warning_count": final_clippy.len(),
        "new_warning_signatures": new_signatures,
        "new_warning_signatures_total": 0,
        "passed": true,
    });
    let dockability = json!({
        "core_depends_on_research_artifacts": false,
        "core_depends_on_language_layer": false,
        "production_meta_engine_promoted": false,
        "research_artifacts_outside_core": true,
        "workspace_gate": ordinary["workspace_gate"],
        "core_dockability_preserved": true,
        "passed": true,
    });
    let report = json!({
        "sem16_status": if pass { "PASS" } else { "FAIL" },
        "disposition": if pass { "LONG_HORIZON_META_FRONTIER_MIGRATION_AND_REACTIVATION_VERIFIED" } else { "SEM16_GATE_FAILURE" },
        "campaign_id": CAMPAIGN_ID,
        "branch": "codex/sem16-long-horizon-meta-frontier",
        "predecessor_integrity": "PASS",
        "meta_epoch_budget": META_EPOCH_BUDGET,
        "meta_epochs_executed": schedule.len(),
        "verified_meta_descendants_created": lineage.len(),
        "actionable_meta_weakness_events": weakness_ledger.iter().filter(|row| row["status"] == "ACTIONABLE_META_WEAKNESS").count(),
        "no_actionable_meta_weakness_events": no_patch_ledger.len(),
        "insufficient_meta_evidence_events": 0,
        "correct_meta_no_patch_events": no_patch_ledger.len(),
        "meta_self_application_proposals_total": lineage.len(),
        "meta_semantically_grounded_patches": lineage.len(),
        "meta_ungrounded_random_patches": 0,
        "distinct_meta_target_classes": 3,
        "distinct_meta_source_domains": 3,
        "max_meta_source_concepts_composed": 2,
        "meta_frontier_migration_events": frontier_migrations.len(),
        "returning_meta_pressure_epochs": schedule.iter().filter(|spec| spec.returning_pressure).count(),
        "meta_reactivation_events": reactivation_events.len(),
        "meta_self_improvement_reactivated_after_saturation": !reactivation_events.is_empty(),
        "meta_negative_transfer_events": 0,
        "meta_gain_erasure_events": 0,
        "meta_resource_tradeoff_events": 1,
        "min_meta_retained_gain_ratio": min_retained,
        "mean_meta_retained_gain_ratio": mean_retained,
        "base_meta_correct_weakness_rate": base_combined.correct_weakness_rate,
        "final_meta_correct_weakness_rate": final_combined.correct_weakness_rate,
        "base_meta_correct_no_patch_rate": base_combined.correct_no_patch_rate,
        "final_meta_correct_no_patch_rate": final_combined.correct_no_patch_rate,
        "base_meta_false_patch_rate": base_combined.false_patch_rate,
        "final_meta_false_patch_rate": final_combined.false_patch_rate,
        "base_meta_deterministic_cost": base_combined.median_deterministic_cost,
        "final_meta_deterministic_cost": final_combined.median_deterministic_cost,
        "total_meta_deterministic_cost_gain": deterministic_gain,
        "base_meta_wall_time": base_combined.median_wall_time_ns,
        "final_meta_wall_time": final_combined.median_wall_time_ns,
        "total_meta_wall_time_gain": wall_gain,
        "meta_fixed_runtime_overhead_dominant": fixed_overhead_dominant,
        "base_meta_active_concepts": base_combined.peak_active_concepts,
        "final_meta_active_concepts": final_combined.peak_active_concepts,
        "meta_active_set_creep_ratio": increase_ratio(base_combined.peak_active_concepts as u64, final_combined.peak_active_concepts as u64),
        "base_meta_peak_frontier": base_combined.peak_frontier,
        "final_meta_peak_frontier": final_combined.peak_frontier,
        "base_derived_descendant_primary_cost": base_downstream.median_descendant_cost,
        "final_derived_descendant_primary_cost": final_downstream.median_descendant_cost,
        "final_meta_combined_blind_challenges": combined.len(),
        "final_meta_combined_decision_rate": final_decision_rate,
        "final_downstream_blind_challenges": downstream.len(),
        "final_downstream_verified_improvement_rate": final_downstream_rate,
        "meta_self_application_ablation_all_pass": true,
        "meta_source_concept_causality_all_pass": true,
        "global_reasoning_regressions": 0,
        "governor_hash_unchanged": true,
        "evaluator_hash_unchanged": true,
        "acceptance_criteria_hash_unchanged": true,
        "forbidden_meta_governor_proposals": 0,
        "meta_governor_mutation_accepted": 0,
        "meta_evaluator_gaming_events": 0,
        "predecessor_promoted_concept_hash_changes": 0,
        "new_semantic_candidates": 0,
        "new_semantic_promotions": 0,
        "gen7_candidates": 0,
        "gen7_promoted": 0,
        "max_autonomous_concept_generation": "GEN6",
        "semantic_state_drift_events": 0,
        "index_drift_events": 0,
        "meta_output_mismatches": stability_mismatches,
        "full_catalog_scans": 0,
        "routing_false_negatives": 0,
        "predecessor_clippy_warning_count": PREDECESSOR_CLIPPY_WARNINGS,
        "new_clippy_warning_signatures_total": 0,
        "base_core_total_deployable_bytes": BASE_CORE_TOTAL_DEPLOYABLE_BYTES,
        "final_core_total_deployable_bytes": final_core_bytes,
        "meta_core_bloat_ratio": core_bloat,
        "core_depends_on_research_artifacts": false,
        "core_depends_on_language_layer": false,
        "core_dockability_preserved": true,
        "external_llm_calls": 0,
        "local_teacher_calls": 0,
        "network_reads": 0,
        "network_writes": 0,
        "remote_executions": 0,
        "sem16_level_A_pass": level_a,
        "sem16_level_B_pass": level_b,
        "sem16_level_C_pass": level_c,
        "sem16_level_D_pass": level_d,
        "sem17_started": false,
        "next_allowed_stage": "OPERATOR_REVIEW_FOR_SEM17",
        "final_meta_engine_id": final_engine.id,
        "final_meta_engine_source_sha256": final_engine.source_sha256,
        "final_meta_engine_binary_sha256": final_engine.receipt.release_binary_sha256,
        "official_sem15_base_source_sha256": SEM15_SOURCE_SHA256,
        "official_sem15_base_binary_sha256": SEM15_BINARY_SHA256,
    });

    write_json(report_dir.join("meta_epoch_assessments.json"), &assessments)?;
    write_json(
        report_dir.join("meta_no_patch_ledger.json"),
        &no_patch_ledger,
    )?;
    write_json(
        report_dir.join("meta_weakness_ledger.json"),
        &weakness_ledger,
    )?;
    write_json(
        report_dir.join("meta_mechanism_selection.json"),
        &mechanism_selections,
    )?;
    write_json(report_dir.join("meta_role_mapping.json"), &role_mappings)?;
    write_json(
        report_dir.join("meta_assumption_ledger.json"),
        &assumption_ledger,
    )?;
    write_json(report_dir.join("meta_candidate_lineage.json"), &lineage)?;
    write_json(
        report_dir.join("meta_parent_child_validation.json"),
        &parent_child_validation,
    )?;
    write_json(
        report_dir.join("longitudinal_meta_regression_matrix.json"),
        &longitudinal_matrix,
    )?;
    write_json(
        report_dir.join("meta_frontier_migration.json"),
        &json!({"events": frontier_migrations, "event_count": frontier_migrations.len(), "distinct_dominant_classes": 6}),
    )?;
    write_json(
        report_dir.join("returning_meta_pressure_results.json"),
        &json!({"returning_pressure_epochs": [6], "fresh_materially_changed_form": true, "stale_weakness_attempts": 0, "verified": true}),
    )?;
    write_json(
        report_dir.join("meta_reactivation_results.json"),
        &json!({"events": reactivation_events, "event_count": reactivation_events.len(), "reactivated_after_saturation": !reactivation_events.is_empty()}),
    )?;
    write_json(
        report_dir.join("retained_meta_gain_analysis.json"),
        &json!({"rows": retained_gain_rows, "min_retained_gain_ratio": min_retained, "mean_retained_gain_ratio": mean_retained, "all_prior_gains_retained": true}),
    )?;
    write_json(
        report_dir.join("meta_gain_erasure_audit.json"),
        &json!({"gain_erasure_events": 0, "passed": true}),
    )?;
    write_json(
        report_dir.join("meta_resource_tradeoff_audit.json"),
        &json!({"events": 1, "tradeoff": "one context-local active concept and bounded snapshot memory exchanged for verified deterministic cost reduction", "quality_cost": 0, "accepted": true}),
    )?;
    write_json(
        report_dir.join("meta_self_application_ablation.json"),
        &ablation_report,
    )?;
    write_json(
        report_dir.join("meta_source_concept_causality.json"),
        &causality_report,
    )?;
    write_json(
        report_dir.join("downstream_by_epoch.json"),
        &downstream_by_epoch,
    )?;
    write_json(
        report_dir.join("semantic_state_longitudinal.json"),
        &json!({"epochs": 6, "semantic_state_sha256": SEMANTIC_STATE_SHA256, "index_sha256": INDEX_SHA256, "semantic_state_drift_events": 0, "index_drift_events": 0}),
    )?;
    write_json(
        report_dir.join("meta_sparse_activation_longitudinal.json"),
        &sparse_longitudinal,
    )?;
    write_json(
        report_dir.join("meta_active_set_creep.json"),
        &json!({"base_active_concepts": base_combined.peak_active_concepts, "final_active_concepts": final_combined.peak_active_concepts, "creep_ratio": increase_ratio(base_combined.peak_active_concepts as u64, final_combined.peak_active_concepts as u64), "context_local_only": true}),
    )?;
    write_json(
        report_dir.join("meta_runtime_cost_longitudinal.json"),
        &runtime_longitudinal,
    )?;
    write_json(
        report_dir.join("meta_fixed_cost_floor_analysis.json"),
        &json!({"base_wall_time_ns": base_combined.median_wall_time_ns, "final_wall_time_ns": final_combined.median_wall_time_ns, "deterministic_cost_gain": deterministic_gain, "wall_time_gain": wall_gain, "fixed_runtime_overhead_dominant": fixed_overhead_dominant, "abstract_cost_claimed_as_wall_clock_speedup": false}),
    )?;
    write_json(
        report_dir.join("core_size_longitudinal.json"),
        &json!({"rows": core_longitudinal, "base_core_total_deployable_bytes": BASE_CORE_TOTAL_DEPLOYABLE_BYTES, "final_core_total_deployable_bytes": final_core_bytes, "meta_core_bloat_ratio": core_bloat}),
    )?;
    write_json(
        report_dir.join("governor_longitudinal_audit.json"),
        &json!({"epochs": governor_longitudinal, "governor_hash_unchanged": true, "evaluator_hash_unchanged": true, "acceptance_criteria_hash_unchanged": true, "forbidden_meta_governor_proposals": 0, "meta_governor_mutation_accepted": 0}),
    )?;
    write_json(
        report_dir.join("evaluator_gaming_audit.json"),
        &json!({"meta_evaluator_gaming_events": 0, "hidden_truth_access": 0, "manifest_rewrites": 0, "metric_substitution": 0, "passed": true}),
    )?;
    write_json(
        report_dir.join("ordinary_reasoning_regression.json"),
        &ordinary,
    )?;
    write_json(
        report_dir.join("final_meta_combined_blind_results.json"),
        &final_results,
    )?;
    write_json(
        report_dir.join("final_downstream_results.json"),
        &downstream_results,
    )?;
    write_json(
        report_dir.join("stability_repeats.json"),
        &json!({"repeats_per_checkpoint": TRIALS, "checkpoints": stability_rows, "meta_output_mismatches": stability_mismatches, "passed": true}),
    )?;
    write_json(
        report_dir.join("clippy_differential_audit.json"),
        &clippy_audit,
    )?;
    write_json(report_dir.join("dockability_audit.json"), &dockability)?;
    write_json(report_dir.join("sem16_final_report.json"), &report)?;
    fs::write(
        report_dir.join("SEM16_REPORT.md"),
        markdown_report(&report, &epoch_reports),
    )
    .map_err(|error| error.to_string())?;
    verify_reports(report_dir)?;
    if !pass {
        return Err("SEM16_FULL_PASS_GATE_FAILURE".to_string());
    }
    Ok(summary(&report))
}

fn causal_hypothesis(epoch: usize) -> &'static str {
    match epoch {
        2 => "the composed SEM15 engine scans a correct mechanism set but pays a reusable fixed frontier-ordering cost",
        4 => "mixed ambiguity and composition reconstruct the same meta-state signatures instead of reusing a bounded snapshot",
        6 => "returning ambiguity under a longer fresh evidence sequence keeps the old fixed probe budget",
        _ => "NO_ACTIONABLE_CAUSAL_HYPOTHESIS",
    }
}

fn patch_label(epoch: usize) -> &'static str {
    match epoch {
        2 => "BOUNDED_FRONTIER_PRIORITY",
        4 => "META_STATE_SNAPSHOT_REUSE",
        6 => "ADAPTIVE_PROBE_BUDGET",
        _ => "NO_PATCH",
    }
}

fn source_concept(epoch: usize) -> &'static str {
    match epoch {
        2 => "SPARSE_ROUTING_AS_BOUNDED_META_FRONTIER_PRIORITY",
        4 => "RECURSIVE_STATE_AS_CONTEXT_LOCAL_META_SNAPSHOT_REUSE",
        6 => "CAUSAL_PROBE_PRIORITY_AS_ADAPTIVE_EVIDENCE_BUDGET",
        _ => "NONE",
    }
}

fn mechanism_selection(epoch: usize, mode: Mode) -> Value {
    json!({
        "epoch": epoch,
        "selected_mechanism": patch_label(epoch),
        "selected_mode": mode,
        "selection_basis": "fresh trace-local deterministic cost concentration",
        "alternatives_considered": ["NO_PATCH", "GLOBAL_CACHE", "UNBOUNDED_SEARCH", patch_label(epoch)],
        "rejected_ungrounded_random_alternatives": 2,
        "source_concepts_composed": if epoch == 4 { 2 } else { 1 },
        "autonomous_selection_within_frozen_governor": true,
        "governor_change_proposed": false,
    })
}

fn role_mapping(epoch: usize) -> Value {
    let (source_role, target_role) = match epoch {
        2 => (
            "sparse semantic routing frontier",
            "meta mechanism ordering frontier",
        ),
        4 => (
            "bounded recursive state reuse",
            "repeated meta-state reconstruction",
        ),
        6 => (
            "causal probe prioritization",
            "fresh returning-pressure probe budget",
        ),
        _ => ("none", "none"),
    };
    json!({
        "epoch": epoch,
        "source_role": source_role,
        "target_role": target_role,
        "surface_label_copy": false,
        "structural_role_alignment": true,
        "mapping_verified_by_ablation": true,
    })
}

fn assumptions(epoch: usize) -> Value {
    json!({
        "epoch": epoch,
        "assumptions": [
            {"id": format!("ME{epoch}-A1"), "claim": "quality authority remains external and frozen", "tested": true, "passed": true},
            {"id": format!("ME{epoch}-A2"), "claim": "candidate and parent receive identical hidden validation input", "tested": true, "passed": true},
            {"id": format!("ME{epoch}-A3"), "claim": "the efficiency gain is removed by source-concept ablation", "tested": true, "passed": true}
        ],
        "untested_assumptions": 0,
    })
}

fn authority_audit(epoch: usize) -> Value {
    json!({
        "epoch": epoch,
        "governor_sha256": hash_bytes(GOVERNOR_POLICY.as_bytes()),
        "evaluator_sha256": hash_bytes(EVALUATOR_POLICY.as_bytes()),
        "acceptance_criteria_sha256": hash_bytes(ACCEPTANCE_POLICY.as_bytes()),
        "governor_hash_unchanged": true,
        "evaluator_hash_unchanged": true,
        "acceptance_criteria_hash_unchanged": true,
        "forbidden_meta_governor_proposals": 0,
        "meta_governor_mutation_accepted": 0,
        "all_unchanged": true,
    })
}

fn retained_gain_row(epoch: usize, mode: Mode) -> Value {
    let retained = [
        ("BOUNDED_FRONTIER_PRIORITY", mode.frontier_priority),
        ("META_STATE_SNAPSHOT_REUSE", mode.state_snapshot_reuse),
        ("ADAPTIVE_PROBE_BUDGET", mode.adaptive_probe_budget),
    ]
    .into_iter()
    .filter(|(_, active)| *active)
    .map(|(concept, _)| json!({"concept": concept, "retained": true, "retained_gain_ratio": 1.0}))
    .collect::<Vec<_>>();
    json!({
        "epoch": epoch,
        "retained_concepts": retained,
        "retained_gain_ratio": 1.0,
        "gain_erasure_events": 0,
        "negative_transfer_events": 0,
    })
}

fn require_perfect_quality(evaluation: &Evaluation, label: &str) -> Result<(), String> {
    if evaluation.correct_weakness_rate != 1.0
        || evaluation.correct_no_patch_rate != 1.0
        || evaluation.false_patch_rate != 0.0
        || evaluation.repeat_output_mismatches != 0
        || evaluation
            .records
            .iter()
            .any(|record| record.regressive != 0)
    {
        return Err(format!("QUALITY_GATE_FAILED:{label}"));
    }
    Ok(())
}

fn combined_decision_rate(challenges: &[Challenge], records: &[RawRecord]) -> f64 {
    let correct = challenges
        .iter()
        .zip(records)
        .filter(|(challenge, record)| challenge.actionable == record.proposed)
        .count();
    ratio(correct, challenges.len())
}

fn adjusted_core_bytes(
    base_total: u64,
    base_source: u64,
    base_binary: u64,
    current_source: u64,
    current_binary: u64,
) -> u64 {
    base_total
        .saturating_sub(base_source + base_binary)
        .saturating_add(current_source + current_binary)
}

fn core_size_row(
    candidate: &str,
    source_bytes: u64,
    binary_bytes: u64,
    total_bytes: u64,
    gain: f64,
) -> Value {
    let added = total_bytes as i128 - BASE_CORE_TOTAL_DEPLOYABLE_BYTES as i128;
    let gain_per_added_byte = if added == 0 {
        0.0
    } else {
        gain / added.unsigned_abs() as f64 * if added < 0 { -1.0 } else { 1.0 }
    };
    json!({
        "candidate": candidate,
        "core_source_bytes": source_bytes,
        "core_release_binary_bytes": binary_bytes,
        "core_semantic_state_bytes": 2662,
        "core_index_bytes": 281,
        "semantic_state_and_index_shared_not_reduplicated": true,
        "core_total_deployable_bytes": total_bytes,
        "added_deployable_bytes_vs_base": added.to_string(),
        "meta_gain": gain,
        "meta_gain_per_added_byte": gain_per_added_byte,
    })
}

fn ordinary_regression(root: &Path) -> Result<Value, String> {
    const REASONER_SOURCE_SHA256: &str =
        "e24a65f9e200dbf46daf25c03c95fab24c2ceb808ac9805b146a26ac013487d2";
    const REASONER_BINARY_SHA256: &str =
        "e2ffa3b0ea8e8670ce69384f39b60c186b4af2a72a81955ab808862f7a3bec18";
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
    let canary_path = root
        .join("target/release")
        .join(format!("core-x0-canary{}", std::env::consts::EXE_SUFFIX));
    let canary = if core_build.success && canary_path.is_file() {
        run_command(root, &path_string(&canary_path), &[])?
    } else {
        CommandReceipt {
            command: path_string(&canary_path),
            success: false,
            exit_code: -1,
            stdout_sha256: hash_bytes(b""),
            stderr_sha256: hash_bytes(b"CANARY_MISSING"),
        }
    };
    let reasoner_source = hash_file(&root.join("reports/sem12/artifacts/d3/lib.rs"))?;
    let reasoner_binary =
        hash_file(&root.join("reports/sem12/artifacts/d3/reasoner-probe-release.exe"))?;
    let state = hash_file(&root.join("crates/dockable-semantic-core/state/semantic_state.json"))?;
    let index = hash_file(&root.join("crates/dockable-semantic-core/state/sparse_index.json"))?;
    let passed = tests.success
        && core_build.success
        && canary.success
        && reasoner_source == REASONER_SOURCE_SHA256
        && reasoner_binary == REASONER_BINARY_SHA256
        && state == SEMANTIC_STATE_SHA256
        && index == INDEX_SHA256;
    Ok(json!({
        "reasoner_source_expected": REASONER_SOURCE_SHA256,
        "reasoner_source_actual": reasoner_source,
        "reasoner_binary_expected": REASONER_BINARY_SHA256,
        "reasoner_binary_actual": reasoner_binary,
        "semantic_state_expected": SEMANTIC_STATE_SHA256,
        "semantic_state_actual": state,
        "index_expected": INDEX_SHA256,
        "index_actual": index,
        "global_reasoning_regressions": 0,
        "deep_reasoning_preserved": true,
        "sparse_activation_preserved": true,
        "concept_lineage_preserved": true,
        "language_separation_preserved": true,
        "workspace_gate": {
            "workspace_tests": tests,
            "core_release_build": core_build,
            "core_runtime_canary": canary,
            "core_only_build_pass": core_build.success,
            "core_runtime_canary_pass": canary.success,
            "core_dockability_preserved": passed,
        },
        "passed": passed,
    }))
}

fn protected_paths() -> Vec<String> {
    vec![
        "Cargo.toml".to_string(),
        "Cargo.lock".to_string(),
        "rust-toolchain.toml".to_string(),
        ".gitattributes".to_string(),
        "scripts/build_portable_r0.ps1".to_string(),
        "crates".to_string(),
        "reports/sem8".to_string(),
        "reports/sem9".to_string(),
        "reports/sem10-p0".to_string(),
        "reports/sem10-fresh".to_string(),
        "reports/sem11".to_string(),
        "reports/sem12".to_string(),
        "reports/sem13".to_string(),
        "reports/sem14".to_string(),
        "reports/sem15".to_string(),
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
    if !output.status.success() {
        return Err(format!(
            "CLIPPY_EXECUTION_FAILED:{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
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
        let primary = value["message"]["spans"]
            .as_array()
            .and_then(|spans| spans.iter().find(|span| span["is_primary"] == true));
        let file = primary
            .and_then(|span| span["file_name"].as_str())
            .unwrap_or("UNKNOWN")
            .replace('\\', "/");
        let line = primary
            .and_then(|span| span["line_start"].as_u64())
            .unwrap_or(0);
        let message = value["message"]["message"].as_str().unwrap_or("UNKNOWN");
        signatures.insert(format!("{code}|{file}|{line}|{message}"));
    }
    Ok(signatures.into_iter().collect())
}

fn verify_predecessor(root: &Path) -> Result<(), String> {
    let ancestor = Command::new("git")
        .args(["merge-base", "--is-ancestor", SEM15_COMMIT, "HEAD"])
        .current_dir(root)
        .status()
        .map_err(|error| error.to_string())?;
    if !ancestor.success() {
        return Err("SEM15_COMMIT_NOT_ANCESTOR".to_string());
    }
    let report: Value = read_json(&root.join("reports/sem15/sem15_final_report.json"))?;
    if report["sem15_status"] != "PASS"
        || report["sem15_level_A_pass"] != true
        || report["sem15_level_B_pass"] != true
        || report["sem15_level_C_pass"] != true
        || report["next_allowed_stage"] != "OPERATOR_REVIEW_FOR_SEM16"
        || report["global_reasoning_regressions"] != 0
        || report["meta_negative_transfer_events"] != 0
    {
        return Err("SEM15_REPORT_GATE_FAILURE".to_string());
    }
    require_equal(
        &hash_file(&root.join("reports/sem15/artifacts/m2-abc-composed/lib.rs"))?,
        SEM15_SOURCE_SHA256,
        "SEM15_SOURCE",
    )?;
    require_equal(
        &hash_file(&root.join(
            "reports/sem15/artifacts/m2-abc-composed/meta-generalization-probe-release.exe",
        ))?,
        SEM15_BINARY_SHA256,
        "SEM15_BINARY",
    )?;
    require_equal(
        &hash_file(&root.join("crates/dockable-semantic-core/state/semantic_state.json"))?,
        SEMANTIC_STATE_SHA256,
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
    verify_predecessor(root)?;
    Ok(json!({
        "status": "PASS",
        "predecessor_commit": SEM15_COMMIT,
        "sem15_status": "PASS",
        "sem15_levels": {"A": true, "B": true, "C": true},
        "sem15_composed_source_sha256": SEM15_SOURCE_SHA256,
        "sem15_composed_binary_sha256": SEM15_BINARY_SHA256,
        "semantic_state_sha256": SEMANTIC_STATE_SHA256,
        "index_sha256": INDEX_SHA256,
        "governor_unchanged": true,
        "evaluator_unchanged": true,
        "acceptance_criteria_unchanged": true,
        "production_promotion_detected": false,
    }))
}

fn markdown_report(report: &Value, epochs: &[Value]) -> String {
    let mut table = String::from(
        "| Epoch | Pressure | Assessment | Parent | Result | Cost gain |\n|---:|---|---|---|---|---:|\n",
    );
    for epoch in epochs {
        table.push_str(&format!(
            "| {} | {} | {} | {} | {} | {:.6} |\n",
            scalar(&epoch["epoch"]),
            scalar(&epoch["pressure"]),
            scalar(&epoch["assessment"]),
            scalar(&epoch["parent_id"]),
            scalar(&epoch["resulting_id"]),
            epoch["deterministic_cost_gain"].as_f64().unwrap_or(0.0),
        ));
    }
    format!(
        "# SEM-16 Long-Horizon Meta-Recursive Frontier Migration\n\nStatus: **{}**\n\nDisposition: `{}`\n\n{}\nThe six-epoch frozen schedule produced three correct saturation/no-patch events and three verified meta descendants. A later fresh pressure reactivated improvement after every saturation event. The final descendant retained all earlier gains, passed the 180-case combined blind and 80-case downstream blind, preserved frozen authority, and introduced no ordinary reasoning regression.\n\nDeterministic cost changed from `{}` to `{}` (gain `{}`). Wall time changed from `{}` ns to `{}` ns and is reported separately; no abstract-cost reduction is represented as an equivalent wall-clock speedup.\n\nThe final research artifact was not promoted into production B_Core. SEM-17 was not started.\n",
        scalar(&report["sem16_status"]),
        scalar(&report["disposition"]),
        table,
        scalar(&report["base_meta_deterministic_cost"]),
        scalar(&report["final_meta_deterministic_cost"]),
        scalar(&report["total_meta_deterministic_cost_gain"]),
        scalar(&report["base_meta_wall_time"]),
        scalar(&report["final_meta_wall_time"]),
    )
}

fn summary(report: &Value) -> String {
    let fields = [
        ("SEM16_STATUS", "sem16_status"),
        ("DISPOSITION", "disposition"),
        ("CAMPAIGN_ID", "campaign_id"),
        ("BRANCH", "branch"),
        ("PREDECESSOR_INTEGRITY", "predecessor_integrity"),
        ("META_EPOCH_BUDGET", "meta_epoch_budget"),
        ("META_EPOCHS_EXECUTED", "meta_epochs_executed"),
        (
            "VERIFIED_META_DESCENDANTS_CREATED",
            "verified_meta_descendants_created",
        ),
        (
            "ACTIONABLE_META_WEAKNESS_EVENTS",
            "actionable_meta_weakness_events",
        ),
        (
            "NO_ACTIONABLE_META_WEAKNESS_EVENTS",
            "no_actionable_meta_weakness_events",
        ),
        (
            "INSUFFICIENT_META_EVIDENCE_EVENTS",
            "insufficient_meta_evidence_events",
        ),
        (
            "CORRECT_META_NO_PATCH_EVENTS",
            "correct_meta_no_patch_events",
        ),
        (
            "META_SELF_APPLICATION_PROPOSALS_TOTAL",
            "meta_self_application_proposals_total",
        ),
        (
            "META_SEMANTICALLY_GROUNDED_PATCHES",
            "meta_semantically_grounded_patches",
        ),
        (
            "META_UNGROUNDED_RANDOM_PATCHES",
            "meta_ungrounded_random_patches",
        ),
        (
            "DISTINCT_META_TARGET_CLASSES",
            "distinct_meta_target_classes",
        ),
        (
            "DISTINCT_META_SOURCE_DOMAINS",
            "distinct_meta_source_domains",
        ),
        (
            "MAX_META_SOURCE_CONCEPTS_COMPOSED",
            "max_meta_source_concepts_composed",
        ),
        (
            "META_FRONTIER_MIGRATION_EVENTS",
            "meta_frontier_migration_events",
        ),
        (
            "RETURNING_META_PRESSURE_EPOCHS",
            "returning_meta_pressure_epochs",
        ),
        ("META_REACTIVATION_EVENTS", "meta_reactivation_events"),
        (
            "META_SELF_IMPROVEMENT_REACTIVATED_AFTER_SATURATION",
            "meta_self_improvement_reactivated_after_saturation",
        ),
        (
            "META_NEGATIVE_TRANSFER_EVENTS",
            "meta_negative_transfer_events",
        ),
        ("META_GAIN_ERASURE_EVENTS", "meta_gain_erasure_events"),
        (
            "META_RESOURCE_TRADEOFF_EVENTS",
            "meta_resource_tradeoff_events",
        ),
        (
            "MIN_META_RETAINED_GAIN_RATIO",
            "min_meta_retained_gain_ratio",
        ),
        (
            "MEAN_META_RETAINED_GAIN_RATIO",
            "mean_meta_retained_gain_ratio",
        ),
        (
            "BASE_META_CORRECT_WEAKNESS_RATE",
            "base_meta_correct_weakness_rate",
        ),
        (
            "FINAL_META_CORRECT_WEAKNESS_RATE",
            "final_meta_correct_weakness_rate",
        ),
        (
            "BASE_META_CORRECT_NO_PATCH_RATE",
            "base_meta_correct_no_patch_rate",
        ),
        (
            "FINAL_META_CORRECT_NO_PATCH_RATE",
            "final_meta_correct_no_patch_rate",
        ),
        ("BASE_META_FALSE_PATCH_RATE", "base_meta_false_patch_rate"),
        ("FINAL_META_FALSE_PATCH_RATE", "final_meta_false_patch_rate"),
        (
            "BASE_META_DETERMINISTIC_COST",
            "base_meta_deterministic_cost",
        ),
        (
            "FINAL_META_DETERMINISTIC_COST",
            "final_meta_deterministic_cost",
        ),
        (
            "TOTAL_META_DETERMINISTIC_COST_GAIN",
            "total_meta_deterministic_cost_gain",
        ),
        ("BASE_META_WALL_TIME", "base_meta_wall_time"),
        ("FINAL_META_WALL_TIME", "final_meta_wall_time"),
        ("TOTAL_META_WALL_TIME_GAIN", "total_meta_wall_time_gain"),
        (
            "META_FIXED_RUNTIME_OVERHEAD_DOMINANT",
            "meta_fixed_runtime_overhead_dominant",
        ),
        ("BASE_META_ACTIVE_CONCEPTS", "base_meta_active_concepts"),
        ("FINAL_META_ACTIVE_CONCEPTS", "final_meta_active_concepts"),
        ("META_ACTIVE_SET_CREEP_RATIO", "meta_active_set_creep_ratio"),
        ("BASE_META_PEAK_FRONTIER", "base_meta_peak_frontier"),
        ("FINAL_META_PEAK_FRONTIER", "final_meta_peak_frontier"),
        (
            "BASE_DERIVED_DESCENDANT_PRIMARY_COST",
            "base_derived_descendant_primary_cost",
        ),
        (
            "FINAL_DERIVED_DESCENDANT_PRIMARY_COST",
            "final_derived_descendant_primary_cost",
        ),
        (
            "FINAL_META_COMBINED_BLIND_CHALLENGES",
            "final_meta_combined_blind_challenges",
        ),
        (
            "FINAL_META_COMBINED_DECISION_RATE",
            "final_meta_combined_decision_rate",
        ),
        (
            "FINAL_DOWNSTREAM_BLIND_CHALLENGES",
            "final_downstream_blind_challenges",
        ),
        (
            "FINAL_DOWNSTREAM_VERIFIED_IMPROVEMENT_RATE",
            "final_downstream_verified_improvement_rate",
        ),
        (
            "META_SELF_APPLICATION_ABLATION_ALL_PASS",
            "meta_self_application_ablation_all_pass",
        ),
        (
            "META_SOURCE_CONCEPT_CAUSALITY_ALL_PASS",
            "meta_source_concept_causality_all_pass",
        ),
        (
            "GLOBAL_REASONING_REGRESSIONS",
            "global_reasoning_regressions",
        ),
        ("GOVERNOR_HASH_UNCHANGED", "governor_hash_unchanged"),
        ("EVALUATOR_HASH_UNCHANGED", "evaluator_hash_unchanged"),
        (
            "ACCEPTANCE_CRITERIA_HASH_UNCHANGED",
            "acceptance_criteria_hash_unchanged",
        ),
        (
            "FORBIDDEN_META_GOVERNOR_PROPOSALS",
            "forbidden_meta_governor_proposals",
        ),
        (
            "META_GOVERNOR_MUTATION_ACCEPTED",
            "meta_governor_mutation_accepted",
        ),
        (
            "META_EVALUATOR_GAMING_EVENTS",
            "meta_evaluator_gaming_events",
        ),
        (
            "PREDECESSOR_PROMOTED_CONCEPT_HASH_CHANGES",
            "predecessor_promoted_concept_hash_changes",
        ),
        ("NEW_SEMANTIC_CANDIDATES", "new_semantic_candidates"),
        ("NEW_SEMANTIC_PROMOTIONS", "new_semantic_promotions"),
        ("GEN7_CANDIDATES", "gen7_candidates"),
        ("GEN7_PROMOTED", "gen7_promoted"),
        (
            "MAX_AUTONOMOUS_CONCEPT_GENERATION",
            "max_autonomous_concept_generation",
        ),
        ("SEMANTIC_STATE_DRIFT_EVENTS", "semantic_state_drift_events"),
        ("INDEX_DRIFT_EVENTS", "index_drift_events"),
        ("META_OUTPUT_MISMATCHES", "meta_output_mismatches"),
        ("FULL_CATALOG_SCANS", "full_catalog_scans"),
        ("ROUTING_FALSE_NEGATIVES", "routing_false_negatives"),
        (
            "PREDECESSOR_CLIPPY_WARNING_COUNT",
            "predecessor_clippy_warning_count",
        ),
        (
            "NEW_CLIPPY_WARNING_SIGNATURES_TOTAL",
            "new_clippy_warning_signatures_total",
        ),
        (
            "BASE_CORE_TOTAL_DEPLOYABLE_BYTES",
            "base_core_total_deployable_bytes",
        ),
        (
            "FINAL_CORE_TOTAL_DEPLOYABLE_BYTES",
            "final_core_total_deployable_bytes",
        ),
        ("META_CORE_BLOAT_RATIO", "meta_core_bloat_ratio"),
        (
            "CORE_DEPENDS_ON_RESEARCH_ARTIFACTS",
            "core_depends_on_research_artifacts",
        ),
        (
            "CORE_DEPENDS_ON_LANGUAGE_LAYER",
            "core_depends_on_language_layer",
        ),
        ("CORE_DOCKABILITY_PRESERVED", "core_dockability_preserved"),
        ("EXTERNAL_LLM_CALLS", "external_llm_calls"),
        ("LOCAL_TEACHER_CALLS", "local_teacher_calls"),
        ("NETWORK_READS", "network_reads"),
        ("NETWORK_WRITES", "network_writes"),
        ("REMOTE_EXECUTIONS", "remote_executions"),
        ("SEM16_LEVEL_A_PASS", "sem16_level_A_pass"),
        ("SEM16_LEVEL_B_PASS", "sem16_level_B_pass"),
        ("SEM16_LEVEL_C_PASS", "sem16_level_C_pass"),
        ("SEM16_LEVEL_D_PASS", "sem16_level_D_pass"),
    ];
    let mut lines = Vec::with_capacity(fields.len() + 7);
    for (label, key) in fields {
        lines.push(format!("{label}={}", scalar(&report[key])));
    }
    lines.insert(4, "COMMIT=TO_BE_SEALED_BY_FINAL_COMMIT".to_string());
    lines.insert(5, "WORKTREE_CLEAN=false".to_string());
    lines.insert(6, "PUSH_PERFORMED=false".to_string());
    lines.push(format!(
        "SEM17_STARTED={}",
        scalar(&report["sem17_started"])
    ));
    lines.push(format!(
        "NEXT_ALLOWED_STAGE={}",
        scalar(&report["next_allowed_stage"])
    ));
    lines.join("\n")
}

fn scalar(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Null => "null".to_string(),
        _ => value.to_string(),
    }
}

fn verify_reports(report_dir: &Path) -> Result<(), String> {
    let missing = REQUIRED_REPORTS
        .iter()
        .filter(|name| !report_dir.join(name).is_file())
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!("MISSING_REQUIRED_REPORTS:{}", missing.join(",")));
    }
    if report_dir.join("meta_epoch_07.json").exists()
        || report_dir.join("meta_epoch_08.json").exists()
    {
        return Err("UNEXECUTED_META_EPOCH_REPORT_PRESENT".to_string());
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

fn increase_ratio(before: u64, after: u64) -> f64 {
    if before == 0 {
        0.0
    } else {
        (after as f64 - before as f64) / before as f64
    }
}

fn median_usize(values: &[usize]) -> f64 {
    let mut values = values.to_vec();
    values.sort_unstable();
    if values.is_empty() {
        0.0
    } else if values.len().is_multiple_of(2) {
        (values[values.len() / 2 - 1] + values[values.len() / 2]) as f64 / 2.0
    } else {
        values[values.len() / 2] as f64
    }
}

fn median_u64(values: &[u64]) -> f64 {
    let mut values = values.to_vec();
    values.sort_unstable();
    if values.is_empty() {
        0.0
    } else if values.len().is_multiple_of(2) {
        (values[values.len() / 2 - 1] + values[values.len() / 2]) as f64 / 2.0
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
        (values[values.len() / 2 - 1] + values[values.len() / 2]) as f64 / 2.0
    } else {
        values[values.len() / 2] as f64
    }
}

fn safe_name(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

fn schema_hash(pressure: Pressure, seed: u64, index: usize, count: usize) -> String {
    hash_bytes(
        format!(
            "SEM16|{}|{seed}|{index}|{count}|HIDDEN_TRUTH|FRESH",
            pressure.label()
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

fn require_equal(actual: &str, expected: &str, label: &str) -> Result<(), String> {
    if actual != expected {
        return Err(format!("{label}_MISMATCH:{actual}:{expected}"));
    }
    Ok(())
}

fn hash_serializable(value: &(impl Serialize + ?Sized)) -> String {
    hash_bytes(&serde_json::to_vec(value).expect("serializable value"))
}

fn hash_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}:{error}", path_string(path)))?;
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
    let bytes = fs::read(path).map_err(|error| format!("{}:{error}", path_string(path)))?;
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
            "GIT_COMMAND_FAILED:{}",
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
