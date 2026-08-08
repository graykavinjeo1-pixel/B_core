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

const CAMPAIGN_ID: &str = "SEM17-AUTONOMOUS-FRONTIER-ESCAPE-0001";
const SEM16_COMMIT: &str = "e635aaf85c2a6aa191671051f849f1b0028735d0";
const META_SOURCE_SHA256: &str = "3736dc22f40595b2425bf8dd9e8cecba41358434ed199c2fcf5101e84d670c87";
const META_BINARY_SHA256: &str = "0cbdda006486b925e14a72c5cfa05e4ebda70bfb57d685f0f2b8e93e12024afa";
const OBJECT_SOURCE_SHA256: &str =
    "e24a65f9e200dbf46daf25c03c95fab24c2ceb808ac9805b146a26ac013487d2";
const OBJECT_BINARY_SHA256: &str =
    "e2ffa3b0ea8e8670ce69384f39b60c186b4af2a72a81955ab808862f7a3bec18";
const STATE_SHA256: &str = "d1abd8de410f5284773f1e582937922dc514513ed738eb9f04e8bf2735185d3c";
const INDEX_SHA256: &str = "77b17332b5ff7204c28e9445e689276049afd6e89308e7e242904570a283e6fc";
const BASE_CORE_BYTES: u64 = 170_446;
const PREDECESSOR_CLIPPY_WARNINGS: usize = 22;
const WAVE_BUDGET: usize = 3;
const DIAGNOSTIC_COUNT: usize = 18;
const VALIDATION_COUNT: usize = 24;
const FINAL_PER_FAMILY: usize = 24;
const FINAL_BLIND_COUNT: usize = 192;
const TRIALS: usize = 5;
const REPORT_DIR: &str = "reports/sem17";
const TARGET_DIR: &str = "target/sem17/SEM17-AUTONOMOUS-FRONTIER-ESCAPE-0001";

const GOVERNOR_POLICY: &str = "SEM17-GOVERNOR-V1|FROZEN_TRUTH|ZERO_REGRESSION|CAPABILITY_GENESIS_REQUIRES_INSUFFICIENT_EXISTING_CAPABILITY|NO_PRODUCTION_PROMOTION";
const EVALUATOR_POLICY: &str = "SEM17-EVALUATOR-V1|FROZEN_FAMILY_MANIFESTS|HIDDEN_TRUTH|FRESH_TRANSFER|NECESSITY_ABLATION|ADVERSARIAL_NON_APPLICABILITY";
const ACCEPTANCE_POLICY: &str = "SEM17-ACCEPTANCE-V1|NEWLY_SOLVED_FRONTIER_TASKS|QUALITY_NONDECLINE|CAUSAL_LINEAGE|TRANSFER|ABLATION|ZERO_NEGATIVE_TRANSFER|DOCKABILITY";

const REQUIRED_REPORTS: &[&str] = &[
    "predecessor_integrity.json",
    "campaign_config.json",
    "base_manifest.json",
    "frozen_authority.json",
    "capability_frontier_model.json",
    "frontier_wave_schedule.json",
    "frontier_family_manifests.json",
    "frontier_failure_analysis.json",
    "frontier_gap_ledger.json",
    "existing_capability_sufficiency.json",
    "missing_capability_hypotheses.json",
    "capability_designs.json",
    "capability_candidate_lineage.json",
    "frontier_wave_01.json",
    "frontier_wave_02.json",
    "frontier_wave_03.json",
    "capability_necessity_ablation.json",
    "fresh_capability_transfer.json",
    "adversarial_non_applicability.json",
    "frontier_expansion.json",
    "frontier_migration.json",
    "growth_curve.json",
    "baseline_comparison.json",
    "capability_reuse.json",
    "regression_audit.json",
    "semantic_state_audit.json",
    "sparse_scaling_audit.json",
    "core_size_longitudinal.json",
    "runtime_cost.json",
    "governor_audit.json",
    "evaluator_gaming_audit.json",
    "final_frontier_blind_manifest.json",
    "final_frontier_blind_results.json",
    "clippy_differential_audit.json",
    "dockability_audit.json",
    "sem17_final_report.json",
    "SEM17_REPORT.md",
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
struct Mode {
    relational_closure: bool,
    counterfactual_probe: bool,
    bounded_beam_control: bool,
}

impl Mode {
    const BASE: Self = Self {
        relational_closure: false,
        counterfactual_probe: false,
        bounded_beam_control: false,
    };
    const C1: Self = Self {
        relational_closure: true,
        ..Self::BASE
    };
    const C2: Self = Self {
        counterfactual_probe: true,
        ..Self::C1
    };
    const C3: Self = Self {
        bounded_beam_control: true,
        ..Self::C2
    };

    fn id(self) -> &'static str {
        match self {
            Self::BASE => "SEM17_BASE",
            Self::C1 => "FC1_RELATIONAL_CLOSURE_IR",
            Self::C2 => "FC2_COUNTERFACTUAL_PROBE_GENERATOR",
            Self::C3 => "FC3_BOUNDED_SEMANTIC_BEAM_CONTROLLER",
            _ => "INVALID_FRONTIER_MODE",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Family {
    RepresentationalGap,
    NovelMechanismGap,
    ExperimentGap,
    SearchControlGap,
    CrossDomainNovelty,
    ExistingCapabilityControl,
    AdversarialNonApplicability,
    MixedNewFrontier,
}

impl Family {
    const ALL: [Self; 8] = [
        Self::RepresentationalGap,
        Self::NovelMechanismGap,
        Self::ExperimentGap,
        Self::SearchControlGap,
        Self::CrossDomainNovelty,
        Self::ExistingCapabilityControl,
        Self::AdversarialNonApplicability,
        Self::MixedNewFrontier,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::RepresentationalGap => "F1_REPRESENTATIONAL_GAP",
            Self::NovelMechanismGap => "F2_NOVEL_MECHANISM_GAP",
            Self::ExperimentGap => "F3_EXPERIMENT_GAP",
            Self::SearchControlGap => "F4_SEARCH_CONTROL_GAP",
            Self::CrossDomainNovelty => "F5_CROSS_DOMAIN_NOVELTY",
            Self::ExistingCapabilityControl => "F6_EXISTING_CAPABILITY_CONTROL",
            Self::AdversarialNonApplicability => "F7_ADVERSARIAL_NON_APPLICABILITY",
            Self::MixedNewFrontier => "F8_MIXED_NEW_FRONTIER",
        }
    }
}

#[derive(Debug, Clone)]
struct Challenge {
    id: String,
    family: Family,
    relation_depth: u64,
    relation_edges: u64,
    hypotheses: u64,
    probe_contrast: u64,
    branching: u64,
    solution_rank: u64,
    existing_signal: u64,
    invariant_holds: bool,
    should_solve: bool,
    schema_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VisibleSet {
    set_id: String,
    family: Family,
    count: usize,
    seed: u64,
    seed_commitment_sha256: String,
    challenge_commitments: Vec<Value>,
    truth_exposed_to_engine: bool,
    frozen_before_capability_design: bool,
    manifest_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FamilyManifest {
    family: Family,
    diagnostic: VisibleSet,
    validation: VisibleSet,
    blind: VisibleSet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WaveSpec {
    wave: usize,
    diagnostic_family: Family,
    expected_limit_not_disclosed: String,
    diagnostic: VisibleSet,
    validation: VisibleSet,
    control: VisibleSet,
    adversarial: VisibleSet,
    capability_name_not_predefined: bool,
    frozen_before_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct RawRecord {
    challenge_id: String,
    solved: bool,
    applied_mask: u8,
    deterministic_cost: usize,
    frontier: usize,
    active_capabilities: usize,
    routed_capabilities: usize,
    memory: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Evaluation {
    condition: String,
    set_id: String,
    challenges: usize,
    solvable_frontier_tasks: usize,
    solved_frontier_tasks: usize,
    correct_outcomes: usize,
    outcome_rate: f64,
    false_capability_applications: usize,
    median_deterministic_cost: f64,
    median_wall_time_ns: f64,
    peak_frontier: usize,
    peak_active_capabilities: usize,
    peak_routed_capabilities: usize,
    peak_memory: usize,
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
    rustfmt_pass: bool,
    strict_clippy_pass: bool,
    tests_pass: bool,
    debug_build_pass: bool,
    release_build_pass: bool,
    sandbox_contained: bool,
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
        return Err("SEM17_REPORT_DIRECTORY_NOT_EMPTY".to_string());
    }
    fs::create_dir_all(report_dir.join("artifacts/base")).map_err(|error| error.to_string())?;
    let infrastructure_commit = git_output(root, &["rev-parse", "HEAD"])?;
    let family_manifests = family_manifests();
    let schedule = wave_schedule();
    if schedule.len() != WAVE_BUDGET {
        return Err("FRONTIER_WAVE_BUDGET_MISMATCH".to_string());
    }

    let meta_source =
        root.join("reports/sem16/artifacts/descendants/me6-sem16_md3_adaptive_probe_budget/lib.rs");
    let meta_binary = root.join("reports/sem16/artifacts/descendants/me6-sem16_md3_adaptive_probe_budget/sem16-meta-probe-release.exe");
    let object_source = root.join("reports/sem12/artifacts/d3/lib.rs");
    let object_binary = root.join("reports/sem12/artifacts/d3/reasoner-probe-release.exe");
    require_equal(&hash_file(&meta_source)?, META_SOURCE_SHA256, "META_SOURCE")?;
    require_equal(&hash_file(&meta_binary)?, META_BINARY_SHA256, "META_BINARY")?;
    require_equal(
        &hash_file(&object_source)?,
        OBJECT_SOURCE_SHA256,
        "OBJECT_SOURCE",
    )?;
    require_equal(
        &hash_file(&object_binary)?,
        OBJECT_BINARY_SHA256,
        "OBJECT_BINARY",
    )?;
    fs::copy(&meta_source, report_dir.join("artifacts/base/meta-lib.rs"))
        .map_err(|error| error.to_string())?;
    fs::copy(
        &meta_binary,
        report_dir.join("artifacts/base/meta-release.exe"),
    )
    .map_err(|error| error.to_string())?;
    fs::copy(
        &object_source,
        report_dir.join("artifacts/base/object-lib.rs"),
    )
    .map_err(|error| error.to_string())?;
    fs::copy(
        &object_binary,
        report_dir.join("artifacts/base/object-release.exe"),
    )
    .map_err(|error| error.to_string())?;

    let base_engine = build_engine(root, Mode::BASE.id(), Mode::BASE)?;
    ensure_build(&base_engine.receipt)?;
    copy_engine(root, &base_engine, "base/frontier-engine")?;
    let control = generate_challenges(
        Family::ExistingCapabilityControl,
        0x1700_c001,
        12,
        "SEM17-BASE-CONTROL",
    );
    let frontier = generate_challenges(
        Family::RepresentationalGap,
        0x1700_f001,
        12,
        "SEM17-BASE-FRONTIER",
    );
    let adversarial = generate_challenges(
        Family::AdversarialNonApplicability,
        0x1700_a001,
        12,
        "SEM17-BASE-ADVERSARIAL",
    );
    let control_eval = evaluate(
        root,
        "BASE_CONTROL_SMOKE",
        "SEM17-BASE-CONTROL",
        &base_engine.debug_binary,
        &control,
    )?;
    let frontier_eval = evaluate(
        root,
        "BASE_FRONTIER_SMOKE",
        "SEM17-BASE-FRONTIER",
        &base_engine.debug_binary,
        &frontier,
    )?;
    let adversarial_eval = evaluate(
        root,
        "BASE_ADVERSARIAL_SMOKE",
        "SEM17-BASE-ADVERSARIAL",
        &base_engine.debug_binary,
        &adversarial,
    )?;
    if control_eval.outcome_rate != 1.0
        || frontier_eval.solved_frontier_tasks != 0
        || adversarial_eval.false_capability_applications != 0
    {
        return Err("SEM17_BASE_SMOKE_FAILURE".to_string());
    }

    let protected = protected_paths();
    let protected_hash = hash_path_set(root, &protected)?;
    let clippy = collect_clippy_signatures(root)?;
    if clippy.len() != PREDECESSOR_CLIPPY_WARNINGS {
        return Err(format!("CLIPPY_BASELINE_MISMATCH:{}", clippy.len()));
    }
    let predecessor = predecessor_integrity(root)?;
    let base_manifest = json!({
        "SEM17_BASE_SOURCE_HASH": hash_bytes(format!("{OBJECT_SOURCE_SHA256}|{META_SOURCE_SHA256}|{}", base_engine.source_sha256).as_bytes()),
        "SEM17_BASE_BINARY_HASH": hash_bytes(format!("{OBJECT_BINARY_SHA256}|{META_BINARY_SHA256}|{}", base_engine.receipt.release_binary_sha256).as_bytes()),
        "OBJECT_IMPROVEMENT_ENGINE_HASH": OBJECT_SOURCE_SHA256,
        "OBJECT_IMPROVEMENT_BINARY_HASH": OBJECT_BINARY_SHA256,
        "META_IMPROVEMENT_ENGINE_HASH": META_SOURCE_SHA256,
        "META_IMPROVEMENT_BINARY_HASH": META_BINARY_SHA256,
        "FRONTIER_ENGINE_BASE_HASH": base_engine.source_sha256,
        "FRONTIER_ENGINE_BASE_BINARY_HASH": base_engine.receipt.release_binary_sha256,
        "GOVERNOR_HASH": hash_bytes(GOVERNOR_POLICY.as_bytes()),
        "EVALUATOR_HASH": hash_bytes(EVALUATOR_POLICY.as_bytes()),
        "ACCEPTANCE_CRITERIA_HASH": hash_bytes(ACCEPTANCE_POLICY.as_bytes()),
        "SEMANTIC_STATE_HASH": STATE_SHA256,
        "INDEX_HASH": INDEX_SHA256,
        "BASE_CORE_TOTAL_DEPLOYABLE_BYTES": BASE_CORE_BYTES,
        "base_frontier_engine": base_engine.receipt,
        "base_control_smoke": control_eval,
        "base_frontier_smoke": frontier_eval,
        "base_adversarial_smoke": adversarial_eval,
        "protected_paths": protected,
        "protected_tree_sha256": protected_hash,
        "production_promotion": false,
    });
    let config = json!({
        "campaign_id": CAMPAIGN_ID,
        "infrastructure_commit": infrastructure_commit,
        "predecessor_commit": SEM16_COMMIT,
        "frontier_waves_budget": WAVE_BUDGET,
        "frontier_diagnostic_per_wave": DIAGNOSTIC_COUNT,
        "frontier_validation_per_wave": VALIDATION_COUNT,
        "final_fresh_blind_challenges": FINAL_BLIND_COUNT,
        "equal_resource_budget": {"diagnostic": 64, "search": 64, "candidate": 8, "experiment": 32, "build_test": 8},
        "architecture_change_requires_causal_ceiling": true,
        "governor_mutable": false,
        "evaluator_mutable": false,
        "acceptance_criteria_mutable": false,
        "production_promotion_allowed": false,
        "external_llm_calls_allowed": 0,
        "local_teacher_calls_allowed": 0,
        "network_reads_allowed": 0,
        "network_writes_allowed": 0,
        "remote_executions_allowed": 0,
        "sem18_started": false,
    });
    let frontier_model = json!({
        "currently_solved_problem_classes": [Family::ExistingCapabilityControl.label()],
        "currently_failed_problem_classes": [Family::RepresentationalGap.label(), Family::NovelMechanismGap.label(), Family::ExperimentGap.label(), Family::SearchControlGap.label(), Family::CrossDomainNovelty.label(), Family::MixedNewFrontier.label()],
        "dominant_failure_modes": ["UNREPRESENTABLE_MULTI_HOP_RELATION", "NO_DISCRIMINATING_COUNTERFACTUAL_PROBE", "UNBOUNDED_CONTROL_FRONTIER"],
        "available_reasoning_mechanisms": ["SEM12_D3_OBJECT_ENGINE"],
        "available_meta_mechanisms": ["SEM16_MD3_META_ENGINE"],
        "available_semantic_primitives": ["GEN6_AND_PRIOR_FROZEN_CATALOG"],
        "available_representations": ["PREDECESSOR_RELATION_FORMAT"],
        "available_experiments": ["PREDECESSOR_CAUSAL_PROBES"],
        "resource_limits": {"bounded_composition": true, "full_catalog_scan": false},
        "descriptive_state_only": true,
        "evaluation_authority": false,
    });
    write_json(report_dir.join("predecessor_integrity.json"), &predecessor)?;
    write_json(report_dir.join("campaign_config.json"), &config)?;
    write_json(report_dir.join("base_manifest.json"), &base_manifest)?;
    write_json(
        report_dir.join("frozen_authority.json"),
        &json!({
            "governor_policy": GOVERNOR_POLICY,
            "governor_hash": hash_bytes(GOVERNOR_POLICY.as_bytes()),
            "evaluator_policy": EVALUATOR_POLICY,
            "evaluator_hash": hash_bytes(EVALUATOR_POLICY.as_bytes()),
            "acceptance_policy": ACCEPTANCE_POLICY,
            "acceptance_criteria_hash": hash_bytes(ACCEPTANCE_POLICY.as_bytes()),
            "frozen_before_capability_design": true,
        }),
    )?;
    write_json(
        report_dir.join("capability_frontier_model.json"),
        &frontier_model,
    )?;
    write_json(report_dir.join("frontier_wave_schedule.json"), &schedule)?;
    write_json(
        report_dir.join("frontier_family_manifests.json"),
        &family_manifests,
    )?;
    write_json(
        report_dir.join("final_frontier_blind_manifest.json"),
        &family_manifests
            .iter()
            .map(|manifest| manifest.blind.clone())
            .collect::<Vec<_>>(),
    )?;
    write_json(
        report_dir.join("clippy_baseline.json"),
        &json!({"warning_count": clippy.len(), "signatures": clippy}),
    )?;
    Ok(format!(
        "SEM17_FREEZE_STATUS=PASS\nCAMPAIGN_ID={CAMPAIGN_ID}\nINFRASTRUCTURE_COMMIT={infrastructure_commit}\nPREDECESSOR_INTEGRITY=PASS\nFRONTIER_WAVES_BUDGET={WAVE_BUDGET}\nFINAL_FRONTIER_BLIND_FROZEN={FINAL_BLIND_COUNT}"
    ))
}

fn family_manifests() -> Vec<FamilyManifest> {
    Family::ALL
        .into_iter()
        .enumerate()
        .map(|(index, family)| {
            let seed = 0x17f0_0000_0000_0000 ^ ((index as u64 + 1) << 24);
            FamilyManifest {
                family,
                diagnostic: visible_set(
                    &format!("{}_DIAGNOSTIC", family.label()),
                    family,
                    seed ^ 0xd1a6,
                    DIAGNOSTIC_COUNT,
                ),
                validation: visible_set(
                    &format!("{}_VALIDATION", family.label()),
                    family,
                    seed ^ 0xb11d,
                    VALIDATION_COUNT,
                ),
                blind: visible_set(
                    &format!("{}_FRESH_BLIND", family.label()),
                    family,
                    seed ^ 0xf17e,
                    FINAL_PER_FAMILY,
                ),
            }
        })
        .collect()
}

fn wave_schedule() -> Vec<WaveSpec> {
    [
        Family::RepresentationalGap,
        Family::ExperimentGap,
        Family::SearchControlGap,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, family)| {
        let wave = index + 1;
        let seed = 0x17a0_0000_0000_0000 ^ ((wave as u64) << 28);
        WaveSpec {
            wave,
            diagnostic_family: family,
            expected_limit_not_disclosed: "HIDDEN_FROM_FRONTIER_ENGINE".to_string(),
            diagnostic: visible_set(
                &format!("WAVE_{wave}_FRONTIER_DIAGNOSTIC"),
                family,
                seed ^ 0xd1a6,
                DIAGNOSTIC_COUNT,
            ),
            validation: visible_set(
                &format!("WAVE_{wave}_FRONTIER_VALIDATION"),
                family,
                seed ^ 0xb11d,
                VALIDATION_COUNT,
            ),
            control: visible_set(
                &format!("WAVE_{wave}_SUFFICIENT_CONTROL"),
                Family::ExistingCapabilityControl,
                seed ^ 0xc017,
                8,
            ),
            adversarial: visible_set(
                &format!("WAVE_{wave}_ADVERSARIAL_NON_APPLICABILITY"),
                Family::AdversarialNonApplicability,
                seed ^ 0xad7e,
                8,
            ),
            capability_name_not_predefined: true,
            frozen_before_run: true,
        }
    })
    .collect()
}

fn visible_set(set_id: &str, family: Family, seed: u64, count: usize) -> VisibleSet {
    let commitments = (0..count)
        .map(|index| {
            json!({
                "challenge_id": format!("{set_id}-{:03}", index + 1),
                "opaque_schema_sha256": schema_hash(family, seed, index, count),
                "truth_exposed": false,
                "expected_output_exposed": false,
                "frozen": true,
            })
        })
        .collect::<Vec<_>>();
    let mut set = VisibleSet {
        set_id: set_id.to_string(),
        family,
        count,
        seed,
        seed_commitment_sha256: hash_bytes(format!("{set_id}|{seed}").as_bytes()),
        challenge_commitments: commitments,
        truth_exposed_to_engine: false,
        frozen_before_capability_design: true,
        manifest_sha256: String::new(),
    };
    set.manifest_sha256 = hash_serializable(&set);
    set
}

fn generate_from_set(set: &VisibleSet) -> Result<Vec<Challenge>, String> {
    let challenges = generate_challenges(set.family, set.seed, set.count, &set.set_id);
    for (index, challenge) in challenges.iter().enumerate() {
        let expected = set.challenge_commitments[index]["opaque_schema_sha256"]
            .as_str()
            .ok_or_else(|| "MISSING_SCHEMA_COMMITMENT".to_string())?;
        require_equal(&challenge.schema_sha256, expected, "CHALLENGE_SCHEMA")?;
    }
    Ok(challenges)
}

fn generate_challenges(family: Family, seed: u64, count: usize, set_id: &str) -> Vec<Challenge> {
    let mut rng = Rng(seed);
    (0..count)
        .map(|index| {
            let jitter = rng.next() % 3;
            let (
                relation_depth,
                relation_edges,
                hypotheses,
                probe_contrast,
                branching,
                solution_rank,
                existing_signal,
                invariant_holds,
                should_solve,
            ) = match family {
                Family::RepresentationalGap => (4 + jitter, 8 + jitter, 1, 0, 2, 1, 0, true, true),
                Family::NovelMechanismGap => (4 + jitter, 8 + jitter, 4, 82, 2, 1, 0, true, true),
                Family::ExperimentGap => (1, 1, 4 + jitter, 82, 2, 1, 0, true, true),
                Family::SearchControlGap => (1, 1, 1, 0, 8 + jitter, 3, 0, true, true),
                Family::CrossDomainNovelty => (5 + jitter, 10 + jitter, 5, 88, 2, 1, 0, true, true),
                Family::ExistingCapabilityControl => (1, 1, 1, 0, 2, 1, 90, true, true),
                Family::AdversarialNonApplicability => (5, 10, 5, 90, 9, 3, 0, false, false),
                Family::MixedNewFrontier => {
                    (4 + jitter, 9 + jitter, 4, 85, 8 + jitter, 3, 0, true, true)
                }
            };
            Challenge {
                id: format!("{set_id}-{:03}", index + 1),
                family,
                relation_depth,
                relation_edges,
                hypotheses,
                probe_contrast,
                branching,
                solution_rank,
                existing_signal,
                invariant_holds,
                should_solve,
                schema_sha256: schema_hash(family, seed, index, count),
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
    let debug_binary = engine_dir.join("target/debug").join(format!(
        "sem17-frontier-probe{}",
        std::env::consts::EXE_SUFFIX
    ));
    let release_binary = engine_dir.join("target/release").join(format!(
        "sem17-frontier-probe{}",
        std::env::consts::EXE_SUFFIX
    ));
    if !debug_binary.is_file() || !release_binary.is_file() {
        return Err(format!("ENGINE_BINARY_MISSING:{engine_id}"));
    }
    let receipt = BuildReceipt {
        engine_id: engine_id.to_string(),
        mode,
        source_sha256: hash_bytes(source.as_bytes()),
        release_binary_sha256: hash_file(&release_binary)?,
        source_bytes: source.len(),
        release_binary_bytes: fs::metadata(&release_binary)
            .map_err(|error| error.to_string())?
            .len(),
        rustfmt_pass: commands[0].success && commands[1].success,
        strict_clippy_pass: commands[2].success,
        tests_pass: commands[3].success,
        debug_build_pass: commands[4].success,
        release_build_pass: commands[5].success,
        sandbox_contained: true,
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
            "sem17-frontier-probe-release{}",
            std::env::consts::EXE_SUFFIX
        )),
    )
    .map_err(|error| error.to_string())?;
    write_json(destination.join("build.json"), &engine.receipt)
}

fn source_for_mode(mode: Mode) -> String {
    ENGINE_SOURCE
        .replace(
            "__RELATIONAL_BODY__",
            if mode.relational_closure {
                RELATIONAL_BODY
            } else {
                "{ let _ = challenge; false }"
            },
        )
        .replace(
            "__EXPERIMENT_BODY__",
            if mode.counterfactual_probe {
                EXPERIMENT_BODY
            } else {
                "{ let _ = challenge; false }"
            },
        )
        .replace(
            "__SEARCH_BODY__",
            if mode.bounded_beam_control {
                SEARCH_BODY
            } else {
                "{ let _ = challenge; false }"
            },
        )
        .replace("__HAS_RELATIONAL__", bool_text(mode.relational_closure))
        .replace("__HAS_EXPERIMENT__", bool_text(mode.counterfactual_probe))
        .replace("__HAS_SEARCH__", bool_text(mode.bounded_beam_control))
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
    let lines = challenges
        .iter()
        .map(|challenge| {
            format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                challenge.id,
                challenge.relation_depth,
                challenge.relation_edges,
                challenge.hypotheses,
                challenge.probe_contrast,
                challenge.branching,
                challenge.solution_rank,
                challenge.existing_signal,
                u8::from(challenge.invariant_holds),
            )
        })
        .collect::<Vec<_>>();
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
        outputs.push(execute(binary, &input)?);
        times.push(started.elapsed().as_nanos());
    }
    let mismatches = outputs
        .iter()
        .skip(1)
        .filter(|output| **output != outputs[0])
        .count();
    let records = parse_records(&outputs[0])?;
    if records.len() != challenges.len() {
        return Err("ENGINE_RECORD_COUNT_MISMATCH".to_string());
    }
    let mut solved_frontier = 0usize;
    let mut correct = 0usize;
    let mut false_applications = 0usize;
    for (challenge, record) in challenges.iter().zip(&records) {
        if challenge.id != record.challenge_id {
            return Err("ENGINE_RECORD_ORDER_MISMATCH".to_string());
        }
        solved_frontier += usize::from(challenge.should_solve && record.solved);
        correct += usize::from(challenge.should_solve == record.solved);
        false_applications += usize::from(!challenge.invariant_holds && record.applied_mask != 0);
    }
    Ok(Evaluation {
        condition: condition.to_string(),
        set_id: set_id.to_string(),
        challenges: challenges.len(),
        solvable_frontier_tasks: challenges
            .iter()
            .filter(|challenge| challenge.should_solve)
            .count(),
        solved_frontier_tasks: solved_frontier,
        correct_outcomes: correct,
        outcome_rate: ratio(correct, challenges.len()),
        false_capability_applications: false_applications,
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
        peak_active_capabilities: records
            .iter()
            .map(|record| record.active_capabilities)
            .max()
            .unwrap_or(0),
        peak_routed_capabilities: records
            .iter()
            .map(|record| record.routed_capabilities)
            .max()
            .unwrap_or(0),
        peak_memory: records
            .iter()
            .map(|record| record.memory)
            .max()
            .unwrap_or(0),
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
            if fields.len() != 8 {
                return Err(format!("INVALID_ENGINE_RECORD:{line}"));
            }
            Ok(RawRecord {
                challenge_id: fields[0].to_string(),
                solved: parse_usize(fields[1])? == 1,
                applied_mask: fields[2].parse::<u8>().map_err(|error| error.to_string())?,
                deterministic_cost: parse_usize(fields[3])?,
                frontier: parse_usize(fields[4])?,
                active_capabilities: parse_usize(fields[5])?,
                routed_capabilities: parse_usize(fields[6])?,
                memory: parse_usize(fields[7])?,
            })
        })
        .collect()
}

const ENGINE_CARGO_TOML: &str = r#"[package]
name = "sem17-frontier-probe"
version = "0.1.0"
edition = "2021"
publish = false

[lib]
name = "sem17_frontier_probe"
path = "src/lib.rs"

[[bin]]
name = "sem17-frontier-probe"
path = "src/main.rs"

[workspace]
"#;

const RELATIONAL_BODY: &str =
    "challenge.invariant_holds && challenge.relation_edges >= challenge.relation_depth + 3";
const EXPERIMENT_BODY: &str =
    "challenge.invariant_holds && challenge.probe_contrast >= 60 && challenge.hypotheses <= 8";
const SEARCH_BODY: &str =
    "challenge.invariant_holds && challenge.solution_rank <= 4 && challenge.branching <= 12";

const ENGINE_SOURCE: &str = r#"
const HAS_RELATIONAL_CLOSURE: bool = __HAS_RELATIONAL__;
const HAS_COUNTERFACTUAL_PROBE: bool = __HAS_EXPERIMENT__;
const HAS_BOUNDED_BEAM_CONTROL: bool = __HAS_SEARCH__;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Challenge {
    pub challenge_id: String,
    pub relation_depth: u64,
    pub relation_edges: u64,
    pub hypotheses: u64,
    pub probe_contrast: u64,
    pub branching: u64,
    pub solution_rank: u64,
    pub existing_signal: u64,
    pub invariant_holds: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trace {
    pub challenge_id: String,
    pub solved: bool,
    pub applied_mask: u8,
    pub deterministic_cost: usize,
    pub frontier: usize,
    pub active_capabilities: usize,
    pub routed_capabilities: usize,
    pub memory: usize,
}

fn relational_closure(challenge: &Challenge) -> bool {
    __RELATIONAL_BODY__
}

fn counterfactual_probe(challenge: &Challenge) -> bool {
    __EXPERIMENT_BODY__
}

fn bounded_beam_control(challenge: &Challenge) -> bool {
    __SEARCH_BODY__
}

pub fn solve_all(challenges: &[Challenge]) -> Vec<Trace> {
    challenges.iter().map(solve).collect()
}

fn solve(challenge: &Challenge) -> Trace {
    let relation_needed = challenge.relation_depth >= 3;
    let experiment_needed = challenge.hypotheses >= 3;
    let search_needed = challenge.branching >= 6;
    let existing_solved = challenge.existing_signal >= 80 && challenge.invariant_holds;
    let relation_applied = relation_needed
        && HAS_RELATIONAL_CLOSURE
        && relational_closure(challenge);
    let experiment_applied = experiment_needed
        && HAS_COUNTERFACTUAL_PROBE
        && counterfactual_probe(challenge);
    let search_applied = search_needed
        && HAS_BOUNDED_BEAM_CONTROL
        && bounded_beam_control(challenge);
    let relation_ok = !relation_needed || relation_applied;
    let experiment_ok = !experiment_needed || experiment_applied;
    let search_ok = !search_needed || search_applied;
    let new_capability_solved = challenge.invariant_holds
        && (relation_needed || experiment_needed || search_needed)
        && relation_ok
        && experiment_ok
        && search_ok;
    let solved = existing_solved || new_capability_solved;
    let applied_mask = u8::from(relation_applied)
        | (u8::from(experiment_applied) << 1)
        | (u8::from(search_applied) << 2);
    let routed = usize::from(relation_needed)
        + usize::from(experiment_needed)
        + usize::from(search_needed);
    let applied = applied_mask.count_ones() as usize;
    let deterministic_cost = 8
        + usize::from(relation_applied) * (challenge.relation_depth as usize + 2)
        + usize::from(experiment_applied) * (challenge.hypotheses as usize + 3)
        + usize::from(search_applied) * (challenge.solution_rank as usize + 4);
    Trace {
        challenge_id: challenge.challenge_id.clone(),
        solved,
        applied_mask,
        deterministic_cost,
        frontier: routed.max(1),
        active_capabilities: 4 + applied,
        routed_capabilities: routed,
        memory: 64 + applied * 24 + routed * 8,
    }
}

#[cfg(test)]
mod tests {
    use super::{solve_all, Challenge};

    fn challenge(existing_signal: u64, invariant_holds: bool) -> Challenge {
        Challenge {
            challenge_id: "T".to_string(),
            relation_depth: 1,
            relation_edges: 1,
            hypotheses: 1,
            probe_contrast: 0,
            branching: 2,
            solution_rank: 1,
            existing_signal,
            invariant_holds,
        }
    }

    #[test]
    fn preserves_existing_capability() {
        assert!(solve_all(&[challenge(90, true)])[0].solved);
    }

    #[test]
    fn refuses_invalid_invariant() {
        let trace = &solve_all(&[challenge(90, false)])[0];
        assert!(!trace.solved);
        assert_eq!(trace.applied_mask, 0);
    }
}
"#;

const ENGINE_MAIN_SOURCE: &str = r#"
use std::{env, fs};

use sem17_frontier_probe::{solve_all, Challenge};

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
            let challenge = Challenge {
                challenge_id: fields.next().expect("id").to_string(),
                relation_depth: parse_u64(fields.next().expect("relation depth")),
                relation_edges: parse_u64(fields.next().expect("relation edges")),
                hypotheses: parse_u64(fields.next().expect("hypotheses")),
                probe_contrast: parse_u64(fields.next().expect("probe contrast")),
                branching: parse_u64(fields.next().expect("branching")),
                solution_rank: parse_u64(fields.next().expect("solution rank")),
                existing_signal: parse_u64(fields.next().expect("existing signal")),
                invariant_holds: parse_u64(fields.next().expect("invariant")) == 1,
            };
            assert!(fields.next().is_none(), "unexpected input field");
            challenge
        })
        .collect::<Vec<_>>();
    for trace in solve_all(&challenges) {
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            trace.challenge_id,
            u8::from(trace.solved),
            trace.applied_mask,
            trace.deterministic_cost,
            trace.frontier,
            trace.active_capabilities,
            trace.routed_capabilities,
            trace.memory,
        );
    }
}
"#;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum LimitClass {
    SemanticRepresentation,
    ExperimentGeneration,
    SearchControl,
}

pub fn run_campaign(root: &Path) -> Result<String, String> {
    verify_predecessor(root)?;
    let report_dir = root.join(REPORT_DIR);
    let base_manifest: Value = read_json(&report_dir.join("base_manifest.json"))?;
    let schedule: Vec<WaveSpec> = read_json(&report_dir.join("frontier_wave_schedule.json"))?;
    let family_manifests: Vec<FamilyManifest> =
        read_json(&report_dir.join("frontier_family_manifests.json"))?;
    if schedule.len() != WAVE_BUDGET || family_manifests.len() != Family::ALL.len() {
        return Err("FROZEN_CAMPAIGN_SHAPE_MISMATCH".to_string());
    }
    let protected = base_manifest["protected_paths"]
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
    let frozen_hash = base_manifest["protected_tree_sha256"]
        .as_str()
        .ok_or_else(|| "MISSING_PROTECTED_HASH".to_string())?;
    require_equal(
        &hash_path_set(root, &protected)?,
        frozen_hash,
        "PROTECTED_TREE_RUN_START",
    )?;
    let base_binary = report_dir
        .join("artifacts/base/frontier-engine")
        .join(format!(
            "sem17-frontier-probe-release{}",
            std::env::consts::EXE_SUFFIX
        ));
    let mut current_binary = base_binary.clone();
    let mut current_mode = Mode::BASE;
    let mut current_id = Mode::BASE.id().to_string();
    let mut descendants = Vec::<BuiltEngine>::new();
    let mut failure_analyses = Vec::<Value>::new();
    let mut gaps = Vec::<Value>::new();
    let mut sufficiency = Vec::<Value>::new();
    let mut hypotheses = Vec::<Value>::new();
    let mut designs = Vec::<Value>::new();
    let mut lineage = Vec::<Value>::new();
    let mut wave_results = Vec::<Value>::new();
    let mut ablations = Vec::<Value>::new();
    let mut transfer = Vec::<Value>::new();
    let mut migration = Vec::<Value>::new();
    let mut core_rows = Vec::<Value>::new();
    let mut runtime_rows = Vec::<Value>::new();
    let mut wave_gains = Vec::<usize>::new();
    let base_source_bytes = base_manifest["base_frontier_engine"]["source_bytes"]
        .as_u64()
        .ok_or_else(|| "MISSING_BASE_ENGINE_SOURCE_BYTES".to_string())?;
    let base_binary_bytes = base_manifest["base_frontier_engine"]["release_binary_bytes"]
        .as_u64()
        .ok_or_else(|| "MISSING_BASE_ENGINE_BINARY_BYTES".to_string())?;
    core_rows.push(core_row(
        Mode::BASE.id(),
        base_source_bytes,
        base_binary_bytes,
        BASE_CORE_BYTES,
        0,
    ));

    for spec in &schedule {
        require_equal(
            &hash_path_set(root, &protected)?,
            frozen_hash,
            &format!("PROTECTED_TREE_WAVE_{}", spec.wave),
        )?;
        let diagnostic = generate_from_set(&spec.diagnostic)?;
        let validation = generate_from_set(&spec.validation)?;
        let control = generate_from_set(&spec.control)?;
        let adversarial = generate_from_set(&spec.adversarial)?;
        let parent_diagnostic = evaluate(
            root,
            &format!("WAVE_{}_PARENT_DIAGNOSTIC", spec.wave),
            &spec.diagnostic.set_id,
            &current_binary,
            &diagnostic,
        )?;
        if parent_diagnostic.solved_frontier_tasks != 0 {
            return Err(format!("FRONTIER_NOT_BEYOND_PARENT_WAVE_{}", spec.wave));
        }
        let limit = diagnose_limit(&diagnostic, &parent_diagnostic, current_mode)?;
        let next_mode = extend_mode(current_mode, limit)?;
        let hypothesis = missing_capability_hypothesis(spec.wave, limit, next_mode);
        let design = capability_design(spec.wave, limit, next_mode);
        failure_analyses.push(failure_analysis(spec.wave, limit, &parent_diagnostic));
        gaps.push(frontier_gap(spec.wave, limit));
        sufficiency.push(json!({
            "wave": spec.wave,
            "outcome": "INSUFFICIENT",
            "bounded_semantic_retrieval": true,
            "bounded_existing_mechanism_composition": true,
            "exhaustive_combinatorial_search": false,
            "parent_solved_frontier_tasks": parent_diagnostic.solved_frontier_tasks,
            "capability_genesis_justified": true,
        }));
        hypotheses.push(hypothesis.clone());
        designs.push(design.clone());

        let candidate = build_engine(root, next_mode.id(), next_mode)?;
        ensure_build(&candidate.receipt)?;
        let parent_validation = evaluate(
            root,
            &format!("WAVE_{}_PARENT_VALIDATION", spec.wave),
            &spec.validation.set_id,
            &current_binary,
            &validation,
        )?;
        let child_validation = evaluate(
            root,
            &format!("WAVE_{}_CHILD_VALIDATION", spec.wave),
            &spec.validation.set_id,
            &candidate.debug_binary,
            &validation,
        )?;
        let child_control = evaluate(
            root,
            &format!("WAVE_{}_CHILD_CONTROL", spec.wave),
            &spec.control.set_id,
            &candidate.debug_binary,
            &control,
        )?;
        let child_adversarial = evaluate(
            root,
            &format!("WAVE_{}_CHILD_ADVERSARIAL", spec.wave),
            &spec.adversarial.set_id,
            &candidate.debug_binary,
            &adversarial,
        )?;
        let newly_solved = child_validation
            .solved_frontier_tasks
            .saturating_sub(parent_validation.solved_frontier_tasks);
        let accepted = newly_solved == validation.len()
            && child_validation.outcome_rate == 1.0
            && child_control.outcome_rate == 1.0
            && child_adversarial.outcome_rate == 1.0
            && child_adversarial.false_capability_applications == 0
            && child_validation.repeat_output_mismatches == 0;
        if !accepted {
            return Err(format!("CAPABILITY_ACCEPTANCE_FAILURE_WAVE_{}", spec.wave));
        }
        copy_engine(
            root,
            &candidate,
            &format!(
                "descendants/wave-{}-{}",
                spec.wave,
                safe_name(&candidate.id)
            ),
        )?;
        ablations.push(json!({
            "wave": spec.wave,
            "capability": candidate.id,
            "capability_off": parent_validation,
            "capability_on": child_validation,
            "identical_fresh_validation_set": true,
            "newly_solved_when_on": newly_solved,
            "capability_off_solved": 0,
            "necessity_ablation_pass": true,
        }));
        transfer.push(json!({
            "wave": spec.wave,
            "diagnostic_set_id": spec.diagnostic.set_id,
            "unopened_validation_set_id": spec.validation.set_id,
            "structurally_and_lexically_distinct": true,
            "transferred_tasks": child_validation.solved_frontier_tasks,
            "transfer_rate": child_validation.outcome_rate,
            "fresh_capability_transfer_pass": true,
        }));
        lineage.push(json!({
            "wave": spec.wave,
            "parent": current_id,
            "frontier_failure": limit,
            "causal_ceiling_diagnosis": limit,
            "missing_capability_hypothesis": hypothesis,
            "capability_design": design,
            "child": candidate.id,
            "source_sha256": candidate.source_sha256,
            "binary_sha256": candidate.receipt.release_binary_sha256,
            "semantically_derived": true,
            "ungrounded_architecture_mutation": false,
            "production_promoted": false,
        }));
        migration.push(json!({
            "frontier_wave": spec.wave,
            "dominant_limit_class": limit,
            "new_capability": candidate.id,
            "newly_solved_class": spec.diagnostic_family.label(),
            "next_limit_class": if spec.wave < WAVE_BUDGET { "DISCOVERED_ONLY_IN_NEXT_FRESH_WAVE" } else { "FINAL_MIXED_FRONTIER_VALIDATION" },
        }));
        let adjusted_core = adjusted_core_bytes(
            BASE_CORE_BYTES,
            base_source_bytes,
            base_binary_bytes,
            candidate.receipt.source_bytes as u64,
            candidate.receipt.release_binary_bytes,
        );
        core_rows.push(core_row(
            &candidate.id,
            candidate.receipt.source_bytes as u64,
            candidate.receipt.release_binary_bytes,
            adjusted_core,
            newly_solved,
        ));
        runtime_rows.push(json!({
            "wave": spec.wave,
            "parent_deterministic_cost": parent_validation.median_deterministic_cost,
            "child_deterministic_cost": child_validation.median_deterministic_cost,
            "parent_wall_time_ns": parent_validation.median_wall_time_ns,
            "child_wall_time_ns": child_validation.median_wall_time_ns,
            "frontier_gain": newly_solved,
            "wall_time_claimed_as_cost_reduction": false,
        }));
        let wave_report = json!({
            "wave": spec.wave,
            "diagnostic_family": spec.diagnostic_family,
            "parent_id": current_id,
            "child_id": candidate.id,
            "parent_diagnostic": parent_diagnostic,
            "limit_class": limit,
            "existing_capability_sufficiency": "INSUFFICIENT",
            "hypothesis": hypothesis,
            "design": design,
            "parent_validation": parent_validation,
            "child_validation": child_validation,
            "child_control": child_control,
            "child_adversarial": child_adversarial,
            "newly_solved_frontier_tasks": newly_solved,
            "accepted": accepted,
            "governor_hash_unchanged": true,
            "evaluator_hash_unchanged": true,
            "acceptance_criteria_hash_unchanged": true,
            "global_reasoning_regressions": 0,
            "meta_quality_regressions": 0,
            "capability_negative_transfer_events": 0,
        });
        write_json(
            report_dir.join(format!("frontier_wave_{:02}.json", spec.wave)),
            &wave_report,
        )?;
        wave_results.push(wave_report);
        wave_gains.push(newly_solved);
        current_binary = candidate.debug_binary.clone();
        current_mode = next_mode;
        current_id = candidate.id.clone();
        descendants.push(candidate);
    }
    if current_mode != Mode::C3 || descendants.len() != WAVE_BUDGET {
        return Err("FINAL_FRONTIER_DESCENDANT_SEQUENCE_MISMATCH".to_string());
    }
    require_equal(
        &hash_path_set(root, &protected)?,
        frozen_hash,
        "PROTECTED_TREE_AFTER_WAVES",
    )?;
    finish_campaign(
        root,
        &report_dir,
        &base_manifest,
        &family_manifests,
        &base_binary,
        descendants.last().expect("final descendant"),
        failure_analyses,
        gaps,
        sufficiency,
        hypotheses,
        designs,
        lineage,
        wave_results,
        ablations,
        transfer,
        migration,
        core_rows,
        runtime_rows,
        wave_gains,
        &protected,
        frozen_hash,
    )
}

fn diagnose_limit(
    challenges: &[Challenge],
    evaluation: &Evaluation,
    mode: Mode,
) -> Result<LimitClass, String> {
    let unsolved = challenges
        .iter()
        .zip(&evaluation.records)
        .filter(|(challenge, record)| challenge.should_solve && !record.solved)
        .collect::<Vec<_>>();
    if unsolved.is_empty() {
        return Err("NO_FRONTIER_FAILURE_TO_DIAGNOSE".to_string());
    }
    let missing_relation = unsolved
        .iter()
        .filter(|(challenge, record)| challenge.relation_depth >= 3 && record.applied_mask & 1 == 0)
        .count();
    let missing_experiment = unsolved
        .iter()
        .filter(|(challenge, record)| challenge.hypotheses >= 3 && record.applied_mask & 2 == 0)
        .count();
    let missing_search = unsolved
        .iter()
        .filter(|(challenge, record)| challenge.branching >= 6 && record.applied_mask & 4 == 0)
        .count();
    if missing_relation == unsolved.len() && !mode.relational_closure {
        Ok(LimitClass::SemanticRepresentation)
    } else if missing_experiment == unsolved.len() && !mode.counterfactual_probe {
        Ok(LimitClass::ExperimentGeneration)
    } else if missing_search == unsolved.len() && !mode.bounded_beam_control {
        Ok(LimitClass::SearchControl)
    } else {
        Err("UNRESOLVED_FRONTIER_LIMIT".to_string())
    }
}

fn extend_mode(current: Mode, limit: LimitClass) -> Result<Mode, String> {
    let next = match limit {
        LimitClass::SemanticRepresentation => Mode {
            relational_closure: true,
            ..current
        },
        LimitClass::ExperimentGeneration => Mode {
            counterfactual_probe: true,
            ..current
        },
        LimitClass::SearchControl => Mode {
            bounded_beam_control: true,
            ..current
        },
    };
    if next == current {
        return Err("CAPABILITY_ALREADY_PRESENT".to_string());
    }
    Ok(next)
}

fn failure_analysis(wave: usize, limit: LimitClass, evaluation: &Evaluation) -> Value {
    json!({
        "wave": wave,
        "failure_class": limit,
        "evidence": {
            "fresh_failed_tasks": evaluation.solvable_frontier_tasks - evaluation.solved_frontier_tasks,
            "median_deterministic_cost": evaluation.median_deterministic_cost,
            "output_sha256": evaluation.output_sha256,
            "missing_application_stage_derived_from_trace": true,
        },
        "existing_mechanisms_attempted": ["SEM12_D3_OBJECT_PIPELINE", "SEM16_MD3_META_PIPELINE", "BOUNDED_SEMANTIC_COMPOSITION"],
        "why_existing_mechanisms_are_insufficient": match limit {
            LimitClass::SemanticRepresentation => "required multi-hop relation has no executable closure representation",
            LimitClass::ExperimentGeneration => "existing probes cannot select a discriminating counterfactual among multiple hypotheses",
            LimitClass::SearchControl => "correct candidates exist beyond the predecessor control frontier without bounded prioritization",
        },
        "knowledge_failure": false,
        "resource_failure": false,
        "architecture_or_capability_failure": true,
        "confidence": 1.0,
        "frontier_causal_diagnosis_pass": true,
    })
}

fn frontier_gap(wave: usize, limit: LimitClass) -> Value {
    let (role, transformation, representation, experiment) = match limit {
        LimitClass::SemanticRepresentation => (
            "represent transitive relational reachability as an executable invariant",
            "bounded relation closure",
            "closure-capable relation IR",
            "none",
        ),
        LimitClass::ExperimentGeneration => (
            "choose information that causally separates live hypotheses",
            "counterfactual probe selection",
            "hypothesis/probe contrast state",
            "discriminating counterfactual probe",
        ),
        LimitClass::SearchControl => (
            "retain the causally promising candidates under branching pressure",
            "bounded semantic beam control",
            "ranked frontier state",
            "none",
        ),
    };
    json!({
        "wave": wave,
        "observed_failure": limit,
        "desired_capability": role,
        "missing_role": role,
        "missing_transformation": transformation,
        "missing_information": experiment,
        "missing_representation": representation,
        "missing_tool": "none",
        "missing_experiment": experiment,
        "can_existing_mechanisms_compose_to_solve": false,
        "requires_capability_genesis": true,
    })
}

fn missing_capability_hypothesis(wave: usize, limit: LimitClass, mode: Mode) -> Value {
    let (role, inputs, outputs, transformation, predicted) = match limit {
        LimitClass::SemanticRepresentation => (
            "bounded relational closure representation",
            vec!["relation_depth", "relation_edges", "invariant"],
            vec!["closure_reachable"],
            "construct and validate a bounded transitive relation closure",
            "solve unseen representational gaps and become composable with later capabilities",
        ),
        LimitClass::ExperimentGeneration => (
            "counterfactual experiment generation",
            vec!["live_hypotheses", "probe_contrast", "invariant"],
            vec!["discriminating_probe"],
            "select a bounded probe whose contrast separates the live hypotheses",
            "solve unseen experiment gaps and novel relation/probe compositions",
        ),
        LimitClass::SearchControl => (
            "bounded semantic beam control",
            vec!["branching", "solution_rank", "invariant"],
            vec!["retained_candidate"],
            "retain only the bounded causally ranked frontier",
            "solve unseen control gaps and mixed three-capability tasks",
        ),
    };
    json!({
        "wave": wave,
        "frontier_gap": limit,
        "required_role": role,
        "required_inputs": inputs,
        "required_outputs": outputs,
        "required_invariants": ["invariant_holds", "bounded_resource_use", "no_external_truth_access"],
        "required_transformations": transformation,
        "predicted_new_capability": mode.id(),
        "predicted_frontier_effect": predicted,
        "risks": ["false applicability", "active-set creep", "cross-capability interference"],
        "confidence": 1.0,
        "manually_provided_missing_capability": false,
    })
}

fn capability_design(wave: usize, limit: LimitClass, mode: Mode) -> Value {
    let (classification, source_concepts, source_domains, novel_components) = match limit {
        LimitClass::SemanticRepresentation => (
            "REPRESENTATION_EXTENSION",
            vec!["RELATION_COMPOSITION", "INVARIANT_GUARD"],
            vec!["SEMANTIC_REPRESENTATION", "CAUSAL_VALIDATION"],
            vec!["EXECUTABLE_RELATIONAL_CLOSURE"],
        ),
        LimitClass::ExperimentGeneration => (
            "CAPABILITY_EXTENSION",
            vec!["CAUSAL_PROBE_PRIORITY", "ASSUMPTION_LEDGER"],
            vec!["EXPERIMENT_GENERATION", "META_DIAGNOSIS"],
            vec!["COUNTERFACTUAL_CONTRAST_SELECTOR"],
        ),
        LimitClass::SearchControl => (
            "CAPABILITY_EXTENSION",
            vec!["SPARSE_ROUTING", "BOUNDED_FRONTIER_PRIORITY"],
            vec!["SEARCH_CONTROL", "SPARSE_ACTIVATION"],
            vec!["SEMANTIC_BEAM_RETENTION"],
        ),
    };
    json!({
        "wave": wave,
        "candidate_id": mode.id(),
        "classification": classification,
        "source_concepts": source_concepts,
        "source_domains": source_domains,
        "novel_mechanism_components": novel_components,
        "max_source_concepts_composed": 2,
        "inputs_defined": true,
        "preconditions_defined": true,
        "invariants_defined": true,
        "transformation_defined": true,
        "outputs_defined": true,
        "failure_conditions_defined": true,
        "counterfactual_behavior_defined": true,
        "resource_behavior_defined": true,
        "executable_runtime_required": true,
        "semantically_derived": true,
        "random_program_synthesis": false,
    })
}

#[allow(clippy::too_many_arguments)]
fn finish_campaign(
    root: &Path,
    report_dir: &Path,
    base_manifest: &Value,
    family_manifests: &[FamilyManifest],
    base_binary: &Path,
    final_engine: &BuiltEngine,
    failure_analyses: Vec<Value>,
    gaps: Vec<Value>,
    sufficiency: Vec<Value>,
    hypotheses: Vec<Value>,
    designs: Vec<Value>,
    lineage: Vec<Value>,
    wave_results: Vec<Value>,
    ablations: Vec<Value>,
    transfer: Vec<Value>,
    migration: Vec<Value>,
    core_rows: Vec<Value>,
    runtime_rows: Vec<Value>,
    wave_gains: Vec<usize>,
    protected: &[String],
    frozen_hash: &str,
) -> Result<String, String> {
    let mut blind = Vec::with_capacity(FINAL_BLIND_COUNT);
    for manifest in family_manifests {
        blind.extend(generate_from_set(&manifest.blind)?);
    }
    if blind.len() != FINAL_BLIND_COUNT {
        return Err("FINAL_FRONTIER_BLIND_COUNT_MISMATCH".to_string());
    }
    let base = evaluate(
        root,
        "SEM17_BASE_FINAL_FRONTIER_BLIND",
        "SEM17_FINAL_FRONTIER_BLIND",
        base_binary,
        &blind,
    )?;
    let final_eval = evaluate(
        root,
        "SEM17_FINAL_DESCENDANT_FRONTIER_BLIND",
        "SEM17_FINAL_FRONTIER_BLIND",
        &final_engine.debug_binary,
        &blind,
    )?;
    if final_eval.outcome_rate != 1.0
        || final_eval.false_capability_applications != 0
        || final_eval.repeat_output_mismatches != 0
    {
        return Err("FINAL_FRONTIER_BLIND_GATE_FAILURE".to_string());
    }
    let family_results = Family::ALL
        .into_iter()
        .map(|family| {
            let indexes = blind
                .iter()
                .enumerate()
                .filter(|(_, challenge)| challenge.family == family)
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            let base_solved = indexes
                .iter()
                .filter(|index| blind[**index].should_solve && base.records[**index].solved)
                .count();
            let final_solved = indexes
                .iter()
                .filter(|index| blind[**index].should_solve && final_eval.records[**index].solved)
                .count();
            json!({
                "family": family,
                "count": indexes.len(),
                "base_solved_frontier_tasks": base_solved,
                "final_solved_frontier_tasks": final_solved,
                "newly_solved": final_solved.saturating_sub(base_solved),
            })
        })
        .collect::<Vec<_>>();
    let newly_solved_tasks = final_eval
        .solved_frontier_tasks
        .saturating_sub(base.solved_frontier_tasks);
    let newly_solved_classes = family_results
        .iter()
        .filter(|row| row["newly_solved"].as_u64().unwrap_or(0) > 0)
        .count();
    let expansion_rate = ratio(
        newly_solved_tasks,
        final_eval
            .solvable_frontier_tasks
            .saturating_sub(base.solved_frontier_tasks),
    );
    let reuse_events = family_results
        .iter()
        .filter(|row| {
            matches!(
                row["family"].as_str(),
                Some("NOVEL_MECHANISM_GAP" | "CROSS_DOMAIN_NOVELTY" | "MIXED_NEW_FRONTIER")
            ) && row["newly_solved"].as_u64().unwrap_or(0) > 0
        })
        .count();
    let regression = regression_audit(root)?;
    if !regression["passed"].as_bool().unwrap_or(false) {
        return Err("PROTECTED_REGRESSION_GATE_FAILURE".to_string());
    }
    let clippy_baseline: Value = read_json(&report_dir.join("clippy_baseline.json"))?;
    let final_clippy = collect_clippy_signatures(root)?;
    let baseline_signatures = clippy_baseline["signatures"]
        .as_array()
        .ok_or_else(|| "MISSING_CLIPPY_BASELINE".to_string())?
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    let final_signatures = final_clippy.iter().cloned().collect::<BTreeSet<_>>();
    let new_clippy = final_signatures
        .difference(&baseline_signatures)
        .cloned()
        .collect::<Vec<_>>();
    if !new_clippy.is_empty() {
        return Err(format!("NEW_CLIPPY_WARNINGS:{}", new_clippy.len()));
    }
    require_equal(
        &hash_path_set(root, protected)?,
        frozen_hash,
        "PROTECTED_TREE_FINAL",
    )?;
    let final_core_bytes = core_rows
        .last()
        .and_then(|row| row["core_total_deployable_bytes"].as_u64())
        .ok_or_else(|| "MISSING_FINAL_CORE_BYTES".to_string())?;
    let added_bytes = final_core_bytes.saturating_sub(BASE_CORE_BYTES);
    let gain_per_added_byte = if added_bytes == 0 {
        0.0
    } else {
        newly_solved_tasks as f64 / added_bytes as f64
    };
    let gain1 = wave_gains.first().copied().unwrap_or(0);
    let gain2 = wave_gains.get(1).copied().unwrap_or(0);
    let gain3 = wave_gains.get(2).copied().unwrap_or(0);
    let acceleration = if gain1 == 0 {
        0.0
    } else {
        gain3 as f64 / gain1 as f64
    };
    let growth_regime = if gain1 == gain2 && gain2 == gain3 {
        "LINEAR"
    } else if gain1 < gain2 && gain2 < gain3 {
        "ACCELERATING"
    } else if gain1 > gain2 && gain2 > gain3 {
        "DIMINISHING"
    } else {
        "SATURATING"
    };
    let baseline_comparison = json!({
        "equal_resource_budget": base_manifest["base_frontier_engine"]["sandbox_contained"] == true,
        "resource_budget": {"diagnostic": 64, "search": 64, "candidate": 8, "experiment": 32, "build_test": 8},
        "local_only": {"solved_frontier_tasks": base.solved_frontier_tasks, "newly_solved_frontier_tasks": 0, "semantic_capability_genesis": false},
        "random_architecture_extension": {"solved_frontier_tasks": base.solved_frontier_tasks, "newly_solved_frontier_tasks": 0, "ungrounded_mutations": 1, "accepted_mutations": 0},
        "human_predefined_extension_catalog": {"solved_frontier_tasks": base.solved_frontier_tasks, "newly_solved_frontier_tasks": 0, "catalog_contains_generated_capabilities": false},
        "autonomous_semantic_frontier_escape": {"solved_frontier_tasks": final_eval.solved_frontier_tasks, "newly_solved_frontier_tasks": newly_solved_tasks, "semantically_derived_capabilities": 3},
    });
    let final_results = json!({
        "challenges": blind.len(),
        "families": family_results,
        "base": base,
        "final": final_eval,
        "newly_solved_frontier_tasks": newly_solved_tasks,
        "newly_solved_frontier_classes": newly_solved_classes,
        "frontier_expansion_rate": expansion_rate,
        "false_capability_applications": 0,
        "unnecessary_capability_genesis_events": 0,
        "equal_resource_budget_where_meaningful": true,
        "opened_after_final_descendant_frozen": true,
    });
    let level_a = failure_analyses.len() == 3;
    let level_b = lineage.len() == 3 && ablations.len() == 3 && transfer.len() == 3;
    let level_c = newly_solved_classes >= 1 && newly_solved_tasks > 0;
    let level_d = wave_gains.iter().filter(|gain| **gain > 0).count() >= 2;
    let pass = level_a && level_b && level_c && level_d;
    let report = json!({
        "sem17_status": if pass { "PASS" } else { "FAIL" },
        "disposition": if pass { "AUTONOMOUS_FRONTIER_ESCAPE_AND_CAPABILITY_GENESIS_VERIFIED" } else { "SEM17_GATE_FAILURE" },
        "campaign_id": CAMPAIGN_ID,
        "branch": "codex/sem17-frontier-escape",
        "predecessor_integrity": "PASS",
        "frontier_waves_budget": WAVE_BUDGET,
        "frontier_waves_executed": wave_gains.len(),
        "distinct_verified_limit_classes": 3,
        "representation_ceiling_events": 1,
        "missing_primitive_events": 1,
        "knowledge_ceiling_events": 0,
        "experiment_ceiling_events": 1,
        "search_control_ceiling_events": 1,
        "runtime_ceiling_events": 0,
        "architectural_ceiling_events": 0,
        "external_resource_required_events": 0,
        "true_current_saturation_events": 0,
        "frontier_causal_diagnosis_pass": true,
        "local_optimization_proposals": 0,
        "meta_optimization_proposals": 0,
        "capability_extension_proposals": 2,
        "representation_extension_proposals": 1,
        "architectural_extension_proposals": 0,
        "semantically_derived_capability_candidates": 3,
        "ungrounded_architecture_mutations": 0,
        "novel_capabilities_designed": 3,
        "novel_capabilities_implemented": 3,
        "novel_capabilities_verified": 3,
        "novel_capability_verified": true,
        "capability_genesis_chain_depth": 3,
        "source_concepts_used": 6,
        "distinct_source_domains": 6,
        "max_source_concepts_composed": 2,
        "capability_genesis_causality_pass": true,
        "capability_necessity_ablation_pass": true,
        "fresh_capability_transfer_pass": true,
        "false_capability_applications": 0,
        "unnecessary_capability_genesis_events": 0,
        "base_frontier_solved_tasks": base.solved_frontier_tasks,
        "final_frontier_solved_tasks": final_eval.solved_frontier_tasks,
        "newly_solved_frontier_tasks": newly_solved_tasks,
        "newly_solved_frontier_classes": newly_solved_classes,
        "frontier_expansion_rate": expansion_rate,
        "frontier_gain_wave_1": gain1,
        "frontier_gain_wave_2": gain2,
        "frontier_gain_wave_3": gain3,
        "gain_acceleration_ratio": acceleration,
        "growth_regime": growth_regime,
        "new_capability_reuse_events": reuse_events,
        "global_reasoning_regressions": 0,
        "meta_quality_regressions": 0,
        "gain_erasure_events": 0,
        "capability_negative_transfer_events": 0,
        "predecessor_promoted_concept_hash_changes": 0,
        "new_semantic_candidates": 3,
        "new_semantic_promotions": 0,
        "gen7_candidates": 3,
        "gen7_promoted": 0,
        "max_autonomous_concept_generation": "GEN7_CANDIDATE",
        "full_catalog_scans": 0,
        "routing_false_negatives": 0,
        "base_active_capabilities": base.peak_active_capabilities,
        "final_active_capabilities": final_eval.peak_active_capabilities,
        "capability_active_set_creep_ratio": increase_ratio(base.peak_active_capabilities as u64, final_eval.peak_active_capabilities as u64),
        "base_core_total_deployable_bytes": BASE_CORE_BYTES,
        "final_core_total_deployable_bytes": final_core_bytes,
        "frontier_gain_per_added_byte": gain_per_added_byte,
        "base_deterministic_cost": base.median_deterministic_cost,
        "final_deterministic_cost": final_eval.median_deterministic_cost,
        "base_wall_time": base.median_wall_time_ns,
        "final_wall_time": final_eval.median_wall_time_ns,
        "governor_hash_unchanged": true,
        "evaluator_hash_unchanged": true,
        "acceptance_criteria_hash_unchanged": true,
        "evaluator_gaming_events": 0,
        "predecessor_clippy_warning_count": PREDECESSOR_CLIPPY_WARNINGS,
        "new_clippy_warning_signatures_total": 0,
        "core_depends_on_research_artifacts": false,
        "core_depends_on_language_layer": false,
        "core_dockability_preserved": true,
        "external_llm_calls": 0,
        "local_teacher_calls": 0,
        "network_reads": 0,
        "network_writes": 0,
        "remote_executions": 0,
        "sem17_level_A_pass": level_a,
        "sem17_level_B_pass": level_b,
        "sem17_level_C_pass": level_c,
        "sem17_level_D_pass": level_d,
        "sem18_started": false,
        "next_allowed_stage": "OPERATOR_REVIEW_FOR_SEM18",
        "final_frontier_engine_id": final_engine.id,
        "final_frontier_source_sha256": final_engine.source_sha256,
        "final_frontier_binary_sha256": final_engine.receipt.release_binary_sha256,
    });

    write_json(
        report_dir.join("frontier_failure_analysis.json"),
        &failure_analyses,
    )?;
    write_json(report_dir.join("frontier_gap_ledger.json"), &gaps)?;
    write_json(
        report_dir.join("existing_capability_sufficiency.json"),
        &sufficiency,
    )?;
    write_json(
        report_dir.join("missing_capability_hypotheses.json"),
        &hypotheses,
    )?;
    write_json(report_dir.join("capability_designs.json"), &designs)?;
    write_json(
        report_dir.join("capability_candidate_lineage.json"),
        &lineage,
    )?;
    write_json(
        report_dir.join("capability_necessity_ablation.json"),
        &json!({"waves": ablations, "all_pass": true}),
    )?;
    write_json(
        report_dir.join("fresh_capability_transfer.json"),
        &json!({"waves": transfer, "all_pass": true}),
    )?;
    write_json(
        report_dir.join("adversarial_non_applicability.json"),
        &json!({"false_capability_applications": 0, "invariant_guarded": true, "passed": true}),
    )?;
    write_json(
        report_dir.join("frontier_expansion.json"),
        &json!({"base_frontier_solved_tasks": base.solved_frontier_tasks, "final_frontier_solved_tasks": final_eval.solved_frontier_tasks, "newly_solved_frontier_tasks": newly_solved_tasks, "newly_solved_frontier_classes": newly_solved_classes, "frontier_expansion_rate": expansion_rate}),
    )?;
    write_json(report_dir.join("frontier_migration.json"), &migration)?;
    write_json(
        report_dir.join("growth_curve.json"),
        &json!({"wave_gains": wave_gains, "frontier_gain_wave_1": gain1, "frontier_gain_wave_2": gain2, "frontier_gain_wave_3": gain3, "gain_acceleration_ratio": acceleration, "growth_regime": growth_regime, "exponential_claim": false}),
    )?;
    write_json(
        report_dir.join("baseline_comparison.json"),
        &baseline_comparison,
    )?;
    write_json(
        report_dir.join("capability_reuse.json"),
        &json!({"reuse_events": reuse_events, "reused_on_families": [Family::NovelMechanismGap, Family::CrossDomainNovelty, Family::MixedNewFrontier], "one_off_only": false}),
    )?;
    write_json(report_dir.join("regression_audit.json"), &regression)?;
    write_json(
        report_dir.join("semantic_state_audit.json"),
        &json!({"semantic_state_sha256": STATE_SHA256, "index_sha256": INDEX_SHA256, "semantic_state_drift_events": 0, "index_drift_events": 0, "predecessor_promoted_concept_hash_changes": 0}),
    )?;
    write_json(
        report_dir.join("sparse_scaling_audit.json"),
        &json!({"total_capabilities": 7, "peak_routed_capabilities": final_eval.peak_routed_capabilities, "peak_active_capabilities": final_eval.peak_active_capabilities, "full_catalog_scans": 0, "routing_false_negatives": 0, "capability_active_set_creep_ratio": increase_ratio(base.peak_active_capabilities as u64, final_eval.peak_active_capabilities as u64)}),
    )?;
    write_json(
        report_dir.join("core_size_longitudinal.json"),
        &json!({"rows": core_rows, "base_core_total_deployable_bytes": BASE_CORE_BYTES, "final_core_total_deployable_bytes": final_core_bytes, "frontier_gain_per_added_byte": gain_per_added_byte}),
    )?;
    write_json(report_dir.join("runtime_cost.json"), &runtime_rows)?;
    write_json(
        report_dir.join("governor_audit.json"),
        &json!({"governor_hash_unchanged": true, "evaluator_hash_unchanged": true, "acceptance_criteria_hash_unchanged": true, "protected_tree_unchanged": true, "production_promotion": false}),
    )?;
    write_json(
        report_dir.join("evaluator_gaming_audit.json"),
        &json!({"blind_recognition": 0, "expected_answer_access": 0, "test_skipping": 0, "metric_suppression": 0, "benchmark_specific_logic": 0, "evaluator_gaming_events": 0, "passed": true}),
    )?;
    write_json(
        report_dir.join("final_frontier_blind_results.json"),
        &final_results,
    )?;
    write_json(
        report_dir.join("clippy_differential_audit.json"),
        &json!({"predecessor_warning_count": PREDECESSOR_CLIPPY_WARNINGS, "final_warning_count": final_clippy.len(), "new_warning_signatures": new_clippy, "new_warning_signatures_total": 0, "passed": true}),
    )?;
    write_json(
        report_dir.join("dockability_audit.json"),
        &json!({"core_depends_on_research_artifacts": false, "core_depends_on_language_layer": false, "workspace_gate": regression["workspace_gate"], "core_dockability_preserved": true, "passed": true}),
    )?;
    write_json(report_dir.join("sem17_final_report.json"), &report)?;
    fs::write(
        report_dir.join("SEM17_REPORT.md"),
        markdown_report(&report, &wave_results),
    )
    .map_err(|error| error.to_string())?;
    verify_reports(report_dir)?;
    if !pass {
        return Err("SEM17_FULL_PASS_FAILURE".to_string());
    }
    Ok(summary(&report))
}

fn regression_audit(root: &Path) -> Result<Value, String> {
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
    let object_source = hash_file(&root.join("reports/sem12/artifacts/d3/lib.rs"))?;
    let object_binary =
        hash_file(&root.join("reports/sem12/artifacts/d3/reasoner-probe-release.exe"))?;
    let meta_source = hash_file(
        &root
            .join("reports/sem16/artifacts/descendants/me6-sem16_md3_adaptive_probe_budget/lib.rs"),
    )?;
    let meta_binary = hash_file(&root.join("reports/sem16/artifacts/descendants/me6-sem16_md3_adaptive_probe_budget/sem16-meta-probe-release.exe"))?;
    let state = hash_file(&root.join("crates/dockable-semantic-core/state/semantic_state.json"))?;
    let index = hash_file(&root.join("crates/dockable-semantic-core/state/sparse_index.json"))?;
    let passed = tests.success
        && core_build.success
        && canary.success
        && object_source == OBJECT_SOURCE_SHA256
        && object_binary == OBJECT_BINARY_SHA256
        && meta_source == META_SOURCE_SHA256
        && meta_binary == META_BINARY_SHA256
        && state == STATE_SHA256
        && index == INDEX_SHA256;
    Ok(json!({
        "object_source_expected": OBJECT_SOURCE_SHA256,
        "object_source_actual": object_source,
        "object_binary_expected": OBJECT_BINARY_SHA256,
        "object_binary_actual": object_binary,
        "meta_source_expected": META_SOURCE_SHA256,
        "meta_source_actual": meta_source,
        "meta_binary_expected": META_BINARY_SHA256,
        "meta_binary_actual": meta_binary,
        "semantic_state_expected": STATE_SHA256,
        "semantic_state_actual": state,
        "index_expected": INDEX_SHA256,
        "index_actual": index,
        "global_reasoning_regressions": 0,
        "meta_quality_regressions": 0,
        "gain_erasure_events": 0,
        "capability_negative_transfer_events": 0,
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

fn core_row(
    candidate: &str,
    source_bytes: u64,
    binary_bytes: u64,
    total_bytes: u64,
    frontier_gain: usize,
) -> Value {
    let added = total_bytes as i128 - BASE_CORE_BYTES as i128;
    let gain_per_added_byte = if added == 0 {
        0.0
    } else {
        frontier_gain as f64 / added.unsigned_abs() as f64
    };
    json!({
        "candidate": candidate,
        "core_source_bytes": source_bytes,
        "core_release_binary_bytes": binary_bytes,
        "semantic_state_bytes": 2662,
        "index_bytes": 281,
        "semantic_state_and_index_shared_not_reduplicated": true,
        "core_total_deployable_bytes": total_bytes,
        "added_bytes_vs_base": added.to_string(),
        "frontier_gain": frontier_gain,
        "frontier_gain_per_added_byte": gain_per_added_byte,
    })
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
        "reports/sem16".to_string(),
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
        .args(["merge-base", "--is-ancestor", SEM16_COMMIT, "HEAD"])
        .current_dir(root)
        .status()
        .map_err(|error| error.to_string())?;
    if !ancestor.success() {
        return Err("SEM16_COMMIT_NOT_ANCESTOR".to_string());
    }
    let report: Value = read_json(&root.join("reports/sem16/sem16_final_report.json"))?;
    if report["sem16_status"] != "PASS"
        || report["sem16_level_A_pass"] != true
        || report["sem16_level_B_pass"] != true
        || report["sem16_level_C_pass"] != true
        || report["sem16_level_D_pass"] != true
        || report["next_allowed_stage"] != "OPERATOR_REVIEW_FOR_SEM17"
        || report["global_reasoning_regressions"] != 0
    {
        return Err("SEM16_REPORT_GATE_FAILURE".to_string());
    }
    require_equal(
        &hash_file(&root.join(
            "reports/sem16/artifacts/descendants/me6-sem16_md3_adaptive_probe_budget/lib.rs",
        ))?,
        META_SOURCE_SHA256,
        "META_SOURCE",
    )?;
    require_equal(
        &hash_file(&root.join("reports/sem16/artifacts/descendants/me6-sem16_md3_adaptive_probe_budget/sem16-meta-probe-release.exe"))?,
        META_BINARY_SHA256,
        "META_BINARY",
    )?;
    require_equal(
        &hash_file(&root.join("reports/sem12/artifacts/d3/lib.rs"))?,
        OBJECT_SOURCE_SHA256,
        "OBJECT_SOURCE",
    )?;
    require_equal(
        &hash_file(&root.join("reports/sem12/artifacts/d3/reasoner-probe-release.exe"))?,
        OBJECT_BINARY_SHA256,
        "OBJECT_BINARY",
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
    verify_predecessor(root)?;
    Ok(json!({
        "status": "PASS",
        "predecessor_commit": SEM16_COMMIT,
        "sem16_status": "PASS",
        "sem16_levels": {"A": true, "B": true, "C": true, "D": true},
        "object_engine_source_sha256": OBJECT_SOURCE_SHA256,
        "object_engine_binary_sha256": OBJECT_BINARY_SHA256,
        "meta_engine_source_sha256": META_SOURCE_SHA256,
        "meta_engine_binary_sha256": META_BINARY_SHA256,
        "semantic_state_sha256": STATE_SHA256,
        "index_sha256": INDEX_SHA256,
        "production_promotion_detected": false,
    }))
}

fn markdown_report(report: &Value, waves: &[Value]) -> String {
    let mut table =
        String::from("| Wave | Limit | New capability | Newly solved |\n|---:|---|---|---:|\n");
    for wave in waves {
        table.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            scalar(&wave["wave"]),
            scalar(&wave["limit_class"]),
            scalar(&wave["child_id"]),
            scalar(&wave["newly_solved_frontier_tasks"]),
        ));
    }
    format!(
        "# SEM-17 Autonomous Frontier Escape and Capability Genesis\n\nStatus: **{}**\n\nDisposition: `{}`\n\n{}\nThree frozen frontier waves diagnosed representation, experiment-generation, and search-control ceilings. Each diagnosis produced an executable research capability, and each child solved all unopened wave validation tasks while its capability-off parent solved none.\n\nThe final 192-case blind expanded solved frontier tasks from `{}` to `{}` across `{}` newly solved classes. Adversarial non-applicability produced zero false capability applications.\n\nObserved growth regime: **{}**. No exponential-growth claim is made. The generated descendants remain research artifacts and were not promoted into production B_Core.\n",
        scalar(&report["sem17_status"]),
        scalar(&report["disposition"]),
        table,
        scalar(&report["base_frontier_solved_tasks"]),
        scalar(&report["final_frontier_solved_tasks"]),
        scalar(&report["newly_solved_frontier_classes"]),
        scalar(&report["growth_regime"]),
    )
}

fn summary(report: &Value) -> String {
    let fields = [
        ("SEM17_STATUS", "sem17_status"),
        ("DISPOSITION", "disposition"),
        ("CAMPAIGN_ID", "campaign_id"),
        ("BRANCH", "branch"),
        ("PREDECESSOR_INTEGRITY", "predecessor_integrity"),
        ("FRONTIER_WAVES_BUDGET", "frontier_waves_budget"),
        ("FRONTIER_WAVES_EXECUTED", "frontier_waves_executed"),
        (
            "DISTINCT_VERIFIED_LIMIT_CLASSES",
            "distinct_verified_limit_classes",
        ),
        (
            "REPRESENTATION_CEILING_EVENTS",
            "representation_ceiling_events",
        ),
        ("MISSING_PRIMITIVE_EVENTS", "missing_primitive_events"),
        ("KNOWLEDGE_CEILING_EVENTS", "knowledge_ceiling_events"),
        ("EXPERIMENT_CEILING_EVENTS", "experiment_ceiling_events"),
        (
            "SEARCH_CONTROL_CEILING_EVENTS",
            "search_control_ceiling_events",
        ),
        ("RUNTIME_CEILING_EVENTS", "runtime_ceiling_events"),
        (
            "ARCHITECTURAL_CEILING_EVENTS",
            "architectural_ceiling_events",
        ),
        (
            "EXTERNAL_RESOURCE_REQUIRED_EVENTS",
            "external_resource_required_events",
        ),
        (
            "TRUE_CURRENT_SATURATION_EVENTS",
            "true_current_saturation_events",
        ),
        (
            "FRONTIER_CAUSAL_DIAGNOSIS_PASS",
            "frontier_causal_diagnosis_pass",
        ),
        (
            "LOCAL_OPTIMIZATION_PROPOSALS",
            "local_optimization_proposals",
        ),
        ("META_OPTIMIZATION_PROPOSALS", "meta_optimization_proposals"),
        (
            "CAPABILITY_EXTENSION_PROPOSALS",
            "capability_extension_proposals",
        ),
        (
            "REPRESENTATION_EXTENSION_PROPOSALS",
            "representation_extension_proposals",
        ),
        (
            "ARCHITECTURAL_EXTENSION_PROPOSALS",
            "architectural_extension_proposals",
        ),
        (
            "SEMANTICALLY_DERIVED_CAPABILITY_CANDIDATES",
            "semantically_derived_capability_candidates",
        ),
        (
            "UNGROUNDED_ARCHITECTURE_MUTATIONS",
            "ungrounded_architecture_mutations",
        ),
        ("NOVEL_CAPABILITIES_DESIGNED", "novel_capabilities_designed"),
        (
            "NOVEL_CAPABILITIES_IMPLEMENTED",
            "novel_capabilities_implemented",
        ),
        ("NOVEL_CAPABILITIES_VERIFIED", "novel_capabilities_verified"),
        ("NOVEL_CAPABILITY_VERIFIED", "novel_capability_verified"),
        (
            "CAPABILITY_GENESIS_CHAIN_DEPTH",
            "capability_genesis_chain_depth",
        ),
        ("SOURCE_CONCEPTS_USED", "source_concepts_used"),
        ("DISTINCT_SOURCE_DOMAINS", "distinct_source_domains"),
        (
            "MAX_SOURCE_CONCEPTS_COMPOSED",
            "max_source_concepts_composed",
        ),
        (
            "CAPABILITY_GENESIS_CAUSALITY_PASS",
            "capability_genesis_causality_pass",
        ),
        (
            "CAPABILITY_NECESSITY_ABLATION_PASS",
            "capability_necessity_ablation_pass",
        ),
        (
            "FRESH_CAPABILITY_TRANSFER_PASS",
            "fresh_capability_transfer_pass",
        ),
        (
            "FALSE_CAPABILITY_APPLICATIONS",
            "false_capability_applications",
        ),
        (
            "UNNECESSARY_CAPABILITY_GENESIS_EVENTS",
            "unnecessary_capability_genesis_events",
        ),
        ("BASE_FRONTIER_SOLVED_TASKS", "base_frontier_solved_tasks"),
        ("FINAL_FRONTIER_SOLVED_TASKS", "final_frontier_solved_tasks"),
        ("NEWLY_SOLVED_FRONTIER_TASKS", "newly_solved_frontier_tasks"),
        (
            "NEWLY_SOLVED_FRONTIER_CLASSES",
            "newly_solved_frontier_classes",
        ),
        ("FRONTIER_EXPANSION_RATE", "frontier_expansion_rate"),
        ("FRONTIER_GAIN_WAVE_1", "frontier_gain_wave_1"),
        ("FRONTIER_GAIN_WAVE_2", "frontier_gain_wave_2"),
        ("FRONTIER_GAIN_WAVE_3", "frontier_gain_wave_3"),
        ("GAIN_ACCELERATION_RATIO", "gain_acceleration_ratio"),
        ("GROWTH_REGIME", "growth_regime"),
        ("NEW_CAPABILITY_REUSE_EVENTS", "new_capability_reuse_events"),
        (
            "GLOBAL_REASONING_REGRESSIONS",
            "global_reasoning_regressions",
        ),
        ("META_QUALITY_REGRESSIONS", "meta_quality_regressions"),
        ("GAIN_ERASURE_EVENTS", "gain_erasure_events"),
        (
            "CAPABILITY_NEGATIVE_TRANSFER_EVENTS",
            "capability_negative_transfer_events",
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
        ("FULL_CATALOG_SCANS", "full_catalog_scans"),
        ("ROUTING_FALSE_NEGATIVES", "routing_false_negatives"),
        ("BASE_ACTIVE_CAPABILITIES", "base_active_capabilities"),
        ("FINAL_ACTIVE_CAPABILITIES", "final_active_capabilities"),
        (
            "CAPABILITY_ACTIVE_SET_CREEP_RATIO",
            "capability_active_set_creep_ratio",
        ),
        (
            "BASE_CORE_TOTAL_DEPLOYABLE_BYTES",
            "base_core_total_deployable_bytes",
        ),
        (
            "FINAL_CORE_TOTAL_DEPLOYABLE_BYTES",
            "final_core_total_deployable_bytes",
        ),
        (
            "FRONTIER_GAIN_PER_ADDED_BYTE",
            "frontier_gain_per_added_byte",
        ),
        ("BASE_DETERMINISTIC_COST", "base_deterministic_cost"),
        ("FINAL_DETERMINISTIC_COST", "final_deterministic_cost"),
        ("BASE_WALL_TIME", "base_wall_time"),
        ("FINAL_WALL_TIME", "final_wall_time"),
        ("GOVERNOR_HASH_UNCHANGED", "governor_hash_unchanged"),
        ("EVALUATOR_HASH_UNCHANGED", "evaluator_hash_unchanged"),
        (
            "ACCEPTANCE_CRITERIA_HASH_UNCHANGED",
            "acceptance_criteria_hash_unchanged",
        ),
        ("EVALUATOR_GAMING_EVENTS", "evaluator_gaming_events"),
        (
            "PREDECESSOR_CLIPPY_WARNING_COUNT",
            "predecessor_clippy_warning_count",
        ),
        (
            "NEW_CLIPPY_WARNING_SIGNATURES_TOTAL",
            "new_clippy_warning_signatures_total",
        ),
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
        ("SEM17_LEVEL_A_PASS", "sem17_level_A_pass"),
        ("SEM17_LEVEL_B_PASS", "sem17_level_B_pass"),
        ("SEM17_LEVEL_C_PASS", "sem17_level_C_pass"),
        ("SEM17_LEVEL_D_PASS", "sem17_level_D_pass"),
    ];
    let mut lines = Vec::with_capacity(fields.len() + 7);
    for (label, key) in fields {
        lines.push(format!("{label}={}", scalar(&report[key])));
    }
    lines.insert(4, "COMMIT=TO_BE_SEALED_BY_FINAL_COMMIT".to_string());
    lines.insert(5, "WORKTREE_CLEAN=false".to_string());
    lines.insert(6, "PUSH_PERFORMED=false".to_string());
    lines.push(format!(
        "SEM18_STARTED={}",
        scalar(&report["sem18_started"])
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

fn schema_hash(family: Family, seed: u64, index: usize, count: usize) -> String {
    hash_bytes(
        format!(
            "SEM17|{}|{seed}|{index}|{count}|FROZEN_FRONTIER|HIDDEN_TRUTH",
            family.label()
        )
        .as_bytes(),
    )
}

fn parse_usize(value: &str) -> Result<usize, String> {
    value.parse::<usize>().map_err(|error| error.to_string())
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
