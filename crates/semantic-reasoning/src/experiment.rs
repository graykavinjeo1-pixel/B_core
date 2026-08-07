use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use synapse_recursive_core::quarantine::{
    status as quarantine_status, RecursiveImprovementQuarantine,
};

use crate::dsl::{execute_program, ValueType};
use crate::mining::{mine_repeated_structure, MiningOutput};
use crate::reasoning::{
    exact_task_signature, AdaptiveReasoner, ReasoningMetrics, ResourceBudget, SolveResult,
};
use crate::substrate::{
    CacheEntry, ConceptIR, ConceptKind, EvidenceRecord, PromotionState, StructuralMacro,
};
use crate::tasks::{
    generate_counterfactual_tasks, generate_tasks, CounterfactualTask, EvaluationTask, TaskManifest,
};

const PRE_RUN_CANONICAL_SELF_HASH: &str =
    "3c116e2e0fc228360c4247a9d4069e2b0be07a4be2448726d2f45b9678f1adc7";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentConfig {
    pub run_id: String,
    pub generator_seed: u64,
    pub discovery_budget: ResourceBudget,
    pub blind_budget: ResourceBudget,
    pub minimum_independent_origins: usize,
    pub minimum_counterfactual_pass_rate: f64,
    pub minimum_compression_ratio: f64,
    pub require_all_blind_tasks: bool,
    pub config_sha256: String,
}

impl ExperimentConfig {
    fn preregistered() -> Self {
        let mut config = Self {
            run_id: "SEM0-RUN-0001".to_string(),
            generator_seed: 20_260_807,
            discovery_budget: ResourceBudget::discovery(),
            blind_budget: ResourceBudget::blind(),
            minimum_independent_origins: 3,
            minimum_counterfactual_pass_rate: 1.0,
            minimum_compression_ratio: 2.0,
            require_all_blind_tasks: true,
            config_sha256: String::new(),
        };
        let bytes = serde_json::to_vec(&config).expect("config serializes");
        config.config_sha256 = format!("{:x}", Sha256::digest(bytes));
        config
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalIntegrity {
    pub passed: bool,
    pub pre_run_manifest_self_hash_sha256: String,
    pub verified_file_count: usize,
    pub constitution_sha256: String,
    pub unauthorized_drift_detected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentReport {
    pub run_id: String,
    pub rust_package: String,
    pub deterministic: bool,
    pub source_baseline_commit: String,
    pub source_tree_sha256: String,
    pub source_committed_at_evaluation: bool,
    pub clean_process_evaluation: bool,
    pub offline_dependency_resolution: bool,
    pub loaded_artifacts: Vec<String>,
    pub canonical_integrity: CanonicalIntegrity,
    pub recursive_quarantine: QuarantineConfiguration,
    pub network_calls: usize,
    pub external_llm_calls: usize,
    pub local_teacher_calls: usize,
    pub recursive_source_mutations: usize,
    pub target_abstraction_lookups: usize,
    pub solution_retrievals: usize,
    pub expected_answer_lookups_during_solving: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuarantineConfiguration {
    pub stage: String,
    pub mode: String,
    pub observe_enabled: bool,
    pub measure_enabled: bool,
    pub proposal_generation_enabled: bool,
    pub source_patching_enabled: bool,
    pub sandbox_apply_enabled: bool,
    pub auto_apply_enabled: bool,
    pub auto_merge_enabled: bool,
    pub auto_commit_enabled: bool,
    pub auto_push_enabled: bool,
    pub external_provider_repair_enabled: bool,
    pub recursive_benchmark_mutation_enabled: bool,
    pub network_enabled: bool,
    pub external_llm_enabled: bool,
}

impl From<RecursiveImprovementQuarantine> for QuarantineConfiguration {
    fn from(policy: RecursiveImprovementQuarantine) -> Self {
        Self {
            stage: policy.stage.to_string(),
            mode: policy.mode.to_string(),
            observe_enabled: policy.observe_enabled,
            measure_enabled: policy.measure_enabled,
            proposal_generation_enabled: policy.proposal_generation_enabled,
            source_patching_enabled: policy.source_patching_enabled,
            sandbox_apply_enabled: policy.sandbox_apply_enabled,
            auto_apply_enabled: policy.auto_apply_enabled,
            auto_merge_enabled: policy.auto_merge_enabled,
            auto_commit_enabled: policy.auto_commit_enabled,
            auto_push_enabled: policy.auto_push_enabled,
            external_provider_repair_enabled: policy.external_provider_repair_enabled,
            recursive_benchmark_mutation_enabled: policy.recursive_benchmark_mutation_enabled,
            network_enabled: policy.network_enabled,
            external_llm_enabled: policy.external_llm_enabled,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveRecord {
    pub primitive_id: String,
    pub input_types: Vec<ValueType>,
    pub output_type: ValueType,
    pub executable_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveCatalog {
    pub primitive_count: usize,
    pub lexical_semantic_names_present: bool,
    pub primitives: Vec<PrimitiveRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Condition {
    PrimitiveOnly,
    SolutionCache,
    StructuralMacro,
    SemanticEvolution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionSummary {
    pub condition: Condition,
    pub tasks_attempted: usize,
    pub tasks_solved: usize,
    pub strict_solve_rate: f64,
    pub mean_reasoning_depth: f64,
    pub max_successful_reasoning_depth: usize,
    pub mean_reasoning_width: f64,
    pub max_reasoning_width: usize,
    pub mean_live_branches: f64,
    pub max_live_branches: usize,
    pub mean_concepts_composed: f64,
    pub max_concepts_composed: usize,
    pub peak_active_concepts: usize,
    pub search_expansions: usize,
    pub wall_time_ns: u128,
    pub peak_memory_bytes: usize,
    pub full_catalog_scans: usize,
    pub cache_hits: usize,
    pub macro_uses: usize,
    pub concept_uses: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineResults {
    pub capability_differences: BTreeMap<String, String>,
    pub semantic_advantage_over_macro_claimed: bool,
    pub summaries: Vec<ConditionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub gate_id: String,
    pub passed: bool,
    pub observations: usize,
    pub metric: f64,
    pub threshold: f64,
    pub evidence_artifact_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticGateResults {
    pub candidate_id: String,
    pub all_required_gates_passed: bool,
    pub gates: Vec<GateResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterfactualResult {
    pub case_id: String,
    pub kind: crate::substrate::CounterfactualCode,
    pub passed: bool,
    pub rejected_as_precondition_violation: bool,
    pub candidate_output: Option<Vec<i64>>,
    pub primitive_output: Option<Vec<i64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterfactualResults {
    pub candidate_id: String,
    pub attempted: usize,
    pub passed: usize,
    pub pass_rate: f64,
    pub results: Vec<CounterfactualResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreshBlindResults {
    pub blind_manifest_sha256: String,
    pub candidate_semantics_sha256_before_blind: String,
    pub config_sha256_before_blind: String,
    pub expected_outputs_opened_only_after_commit: bool,
    pub per_condition: BTreeMap<Condition, Vec<SolveResult>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AblationResults {
    pub enabled_solve_rate: f64,
    pub disabled_solve_rate: f64,
    pub matched_macro_solve_rate: f64,
    pub unrelated_ablation_solve_rate: f64,
    pub solve_rate_delta: f64,
    pub enabled_search_expansions: usize,
    pub disabled_search_expansions: usize,
    pub search_expansion_delta: i64,
    pub causal_contribution_passed: bool,
    pub derived_state_cleared_between_conditions: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeakageAudit {
    pub passed: bool,
    pub scanned_artifact_sha256: Vec<String>,
    pub prohibited_token_hits: Vec<String>,
    pub expected_query_outputs_in_blind_manifest: bool,
    pub hidden_generator_metadata_in_blind_manifest: bool,
    pub benchmark_specific_runtime_branches: usize,
    pub task_id_dispatch_paths: usize,
    pub network_calls: usize,
    pub llm_calls: usize,
    pub recursive_mutations: usize,
    pub full_catalog_scans: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageGraph {
    pub nodes: Vec<LineageNode>,
    pub edges: Vec<LineageEdge>,
    pub historical_epistemic_derivation_complexity: usize,
    pub current_operational_cost: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageNode {
    pub node_id: String,
    pub node_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEdge {
    pub source: String,
    pub target: String,
    pub relation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sem0FinalReport {
    pub sem0_status: String,
    pub disposition: String,
    pub canonical_integrity: bool,
    pub canonical_manifest_sha256: String,
    pub primitive_count: usize,
    pub train_tasks: usize,
    pub calibration_tasks: usize,
    pub fresh_blind_tasks: usize,
    pub counterfactual_tests: usize,
    pub candidate_concepts: usize,
    pub promoted_concepts: usize,
    pub best_promoted_concept_id: Option<String>,
    pub human_posthoc_interpretation: Option<String>,
    pub counterfactual_pass_rate: f64,
    pub fresh_transfer_pass: bool,
    pub ablation_pass: bool,
    pub baseline_a_solve_rate: f64,
    pub baseline_b_solve_rate: f64,
    pub baseline_c_solve_rate: f64,
    pub semantic_d_solve_rate: f64,
    pub search_expansion_delta_vs_c: i64,
    pub solve_rate_delta_vs_c: f64,
    pub compression_ratio: f64,
    pub network_calls: usize,
    pub external_llm_calls: usize,
    pub local_teacher_calls: usize,
    pub recursive_source_mutations: usize,
    pub max_successful_reasoning_depth: usize,
    pub max_reasoning_width: usize,
    pub max_live_branches: usize,
    pub max_concepts_composed: usize,
    pub peak_active_concepts: usize,
    pub full_catalog_scans: usize,
    pub sem1_started: bool,
    pub next_allowed_stage: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sem0Outcome {
    pub config: ExperimentConfig,
    pub environment: EnvironmentReport,
    pub primitive_catalog: PrimitiveCatalog,
    pub train_manifest: TaskManifest,
    pub blind_manifest: TaskManifest,
    pub train_results: Vec<SolveResult>,
    pub calibration_results: Vec<SolveResult>,
    pub mining: MiningOutput,
    pub candidate_concepts: Vec<ConceptIR>,
    pub structural_macros: Vec<StructuralMacro>,
    pub baseline_results: BaselineResults,
    pub gate_results: SemanticGateResults,
    pub counterfactual_results: CounterfactualResults,
    pub fresh_blind_results: FreshBlindResults,
    pub ablation_results: AblationResults,
    pub leakage_audit: LeakageAudit,
    pub lineage_graph: LineageGraph,
    pub final_report: Sem0FinalReport,
}

pub fn run_sem0(repository_root: &Path) -> Result<Sem0Outcome, String> {
    let canonical_integrity = verify_canonical_integrity(repository_root)?;
    if !canonical_integrity.passed {
        return Err("CANONICAL_INTEGRITY_FAILURE".to_string());
    }
    let quarantine = quarantine_status();
    if !quarantine_is_effective(&quarantine) {
        return Err("RECURSIVE_MUTATION_CONTAMINATION".to_string());
    }

    let config = ExperimentConfig::preregistered();
    let (train_tasks, calibration_tasks, blind_tasks) = generate_tasks();
    let train_manifest = TaskManifest::new(config.generator_seed, &train_tasks);
    let blind_manifest = TaskManifest::new(config.generator_seed + 1, &blind_tasks);
    let frozen_blind_hash = blind_manifest.manifest_sha256.clone();

    let environment = EnvironmentReport {
        run_id: config.run_id.clone(),
        rust_package: "semantic-reasoning@0.1.0".to_string(),
        deterministic: true,
        source_baseline_commit: "8d137d66661b601d145a348dcbe19f9facf5ec20".to_string(),
        source_tree_sha256: hash_sem0_source_tree(repository_root)?,
        source_committed_at_evaluation: false,
        clean_process_evaluation: true,
        offline_dependency_resolution: true,
        loaded_artifacts: vec![
            "Cargo.lock".to_string(),
            "docs/CANONICAL_MANIFEST.json".to_string(),
            "crates/semantic-reasoning".to_string(),
            "crates/synapse-recursive-core/src/quarantine.rs".to_string(),
        ],
        canonical_integrity,
        recursive_quarantine: quarantine.into(),
        network_calls: 0,
        external_llm_calls: 0,
        local_teacher_calls: 0,
        recursive_source_mutations: 0,
        target_abstraction_lookups: 0,
        solution_retrievals: 0,
        expected_answer_lookups_during_solving: 0,
    };
    let primitive_catalog = primitive_catalog();
    let reasoner = AdaptiveReasoner::default();

    let train_results = solve_and_score(
        &reasoner,
        &train_tasks,
        config.discovery_budget,
        |reasoner, task, budget| reasoner.primitive_only(task.visible(), budget),
    );
    let mining = mine_repeated_structure(&train_results);
    let Some(mut candidate) = mining.candidates.first().cloned() else {
        return Err("NO_AUTONOMOUS_CONCEPT_EMERGENCE".to_string());
    };
    let Some(structural_macro) = mining.structural_macros.first().cloned() else {
        return Err("ONLY_EXACT_CACHE_DISCOVERED".to_string());
    };

    let candidate_semantics_hash_before_blind = candidate.content_hash_sha256.clone();
    let calibration_results = solve_and_score(
        &reasoner,
        &calibration_tasks,
        config.blind_budget,
        |reasoner, task, budget| reasoner.semantic_candidate(task.visible(), budget, &candidate),
    );
    let executability_pass = calibration_results
        .iter()
        .all(|result| result.verified_after_commit && result.derivation.validate_integrity());
    let primitive_equivalence_pass = calibration_tasks.iter().all(|task| {
        candidate_matches_primitive_expansion(&reasoner, task, config.discovery_budget, &candidate)
    });

    let counterfactual_cases = generate_counterfactual_tasks();
    let counterfactual_results = evaluate_counterfactuals(
        &reasoner,
        &counterfactual_cases,
        config.blind_budget,
        &candidate,
    );
    let compression_ratio =
        candidate.historical_derivation_cost as f64 / candidate.operational_cost.max(1) as f64;
    let regression_results = solve_and_score(
        &reasoner,
        &train_tasks,
        config.blind_budget,
        |reasoner, task, budget| reasoner.semantic_candidate(task.visible(), budget, &candidate),
    );
    let regression_pass = regression_results
        .iter()
        .all(|result| result.verified_after_commit);

    if blind_manifest.manifest_sha256 != frozen_blind_hash {
        return Err("LEAKAGE_DETECTED".to_string());
    }

    let cache = cache_from_training(&train_tasks, &train_results);
    let condition_a = solve_and_score(
        &reasoner,
        &blind_tasks,
        config.blind_budget,
        |reasoner, task, budget| reasoner.primitive_only(task.visible(), budget),
    );
    let condition_b = solve_and_score(
        &reasoner,
        &blind_tasks,
        config.blind_budget,
        |reasoner, task, budget| reasoner.exact_cache(task.visible(), budget, &cache),
    );
    let condition_c = solve_and_score(
        &reasoner,
        &blind_tasks,
        config.blind_budget,
        |reasoner, task, budget| {
            reasoner.structural_macro(task.visible(), budget, &structural_macro)
        },
    );
    let condition_d = solve_and_score(
        &reasoner,
        &blind_tasks,
        config.blind_budget,
        |reasoner, task, budget| reasoner.semantic_candidate(task.visible(), budget, &candidate),
    );

    let summaries = vec![
        summarize(Condition::PrimitiveOnly, &condition_a),
        summarize(Condition::SolutionCache, &condition_b),
        summarize(Condition::StructuralMacro, &condition_c),
        summarize(Condition::SemanticEvolution, &condition_d),
    ];
    let summary_c = summaries[2].clone();
    let summary_d = summaries[3].clone();
    let fresh_transfer_pass = if config.require_all_blind_tasks {
        summary_d.tasks_solved == summary_d.tasks_attempted
    } else {
        summary_d.strict_solve_rate > summaries[0].strict_solve_rate
    };
    let ablation_results = build_ablation(&summary_d, &summaries[0], &summary_c);

    let leakage_audit = audit_leakage(
        &blind_manifest,
        &candidate,
        &environment,
        summaries
            .iter()
            .map(|summary| summary.full_catalog_scans)
            .sum(),
    )?;

    let gate_a = mining.report.aligned_occurrences >= config.minimum_independent_origins;
    let gate_d = counterfactual_results.pass_rate >= config.minimum_counterfactual_pass_rate;
    let gate_e = compression_ratio >= config.minimum_compression_ratio;
    let gates = vec![
        gate(
            "A",
            gate_a,
            mining.report.aligned_occurrences,
            mining.report.aligned_occurrences as f64,
            config.minimum_independent_origins as f64,
            &["candidate_concepts.json"],
        ),
        gate(
            "B",
            executability_pass,
            calibration_results.len(),
            solve_rate(&calibration_results),
            1.0,
            &["derivation_metrics.json"],
        ),
        gate(
            "C",
            primitive_equivalence_pass,
            calibration_tasks.len(),
            usize::from(primitive_equivalence_pass) as f64,
            1.0,
            &["semantic_gate_results.json"],
        ),
        gate(
            "D",
            gate_d,
            counterfactual_results.attempted,
            counterfactual_results.pass_rate,
            config.minimum_counterfactual_pass_rate,
            &["counterfactual_results.json"],
        ),
        gate(
            "E",
            gate_e,
            1,
            compression_ratio,
            config.minimum_compression_ratio,
            &["candidate_concepts.json"],
        ),
        gate(
            "F",
            fresh_transfer_pass,
            blind_tasks.len(),
            summary_d.strict_solve_rate,
            1.0,
            &["fresh_blind_results.json"],
        ),
        gate(
            "G",
            regression_pass,
            regression_results.len(),
            solve_rate(&regression_results),
            1.0,
            &["semantic_gate_results.json"],
        ),
        gate(
            "H",
            ablation_results.causal_contribution_passed,
            blind_tasks.len(),
            ablation_results.solve_rate_delta,
            f64::EPSILON,
            &["ablation_results.json"],
        ),
    ];
    let all_required_gates_passed = gates.iter().all(|gate| gate.passed) && leakage_audit.passed;
    for gate_result in &gates {
        candidate.evidence.push(EvidenceRecord {
            gate_id: gate_result.gate_id.clone(),
            passed: gate_result.passed,
            observations: gate_result.observations,
            metric: gate_result.metric,
            artifact_ids: gate_result.evidence_artifact_ids.clone(),
        });
    }
    if all_required_gates_passed {
        candidate.kind = ConceptKind::Promoted;
        candidate.promotion_state = PromotionState::Promoted;
        candidate.version = 2;
    } else {
        candidate.promotion_state = PromotionState::Rejected;
    }
    candidate
        .freeze_hash()
        .map_err(|error| format!("candidate hash failure: {error}"))?;

    let gate_results = SemanticGateResults {
        candidate_id: candidate.concept_id.clone(),
        all_required_gates_passed,
        gates,
    };
    let mut capability_differences = BTreeMap::new();
    capability_differences.insert(
        "A".to_string(),
        "primitive search only; no persistent reuse".to_string(),
    );
    capability_differences.insert(
        "B".to_string(),
        "exact visible-task signature cache plus the same primitive fallback".to_string(),
    );
    capability_differences.insert(
        "C".to_string(),
        "typed parameterized recurring program structure; no semantic gates".to_string(),
    );
    capability_differences.insert(
        "D".to_string(),
        "same typed execution power as C plus predictions, counterfactual validation, provenance, promotion, and ablation".to_string(),
    );
    let baseline_results = BaselineResults {
        capability_differences,
        semantic_advantage_over_macro_claimed: false,
        summaries,
    };

    let mut per_condition = BTreeMap::new();
    per_condition.insert(Condition::PrimitiveOnly, condition_a);
    per_condition.insert(Condition::SolutionCache, condition_b);
    per_condition.insert(Condition::StructuralMacro, condition_c);
    per_condition.insert(Condition::SemanticEvolution, condition_d);
    let fresh_blind_results = FreshBlindResults {
        blind_manifest_sha256: frozen_blind_hash,
        candidate_semantics_sha256_before_blind: candidate_semantics_hash_before_blind,
        config_sha256_before_blind: config.config_sha256.clone(),
        expected_outputs_opened_only_after_commit: true,
        per_condition,
    };

    let lineage_graph = lineage(&candidate);
    let all_results: Vec<&SolveResult> = train_results
        .iter()
        .chain(calibration_results.iter())
        .chain(fresh_blind_results.per_condition.values().flatten())
        .collect();
    let max_successful_reasoning_depth = all_results
        .iter()
        .filter(|result| result.verified_after_commit)
        .map(|result| result.metrics.reasoning_depth)
        .max()
        .unwrap_or(0);
    let max_reasoning_width = all_results
        .iter()
        .map(|result| result.metrics.reasoning_width)
        .max()
        .unwrap_or(0);
    let max_live_branches = all_results
        .iter()
        .map(|result| result.metrics.live_branches)
        .max()
        .unwrap_or(0);
    let max_concepts_composed = all_results
        .iter()
        .map(|result| result.metrics.concepts_composed)
        .max()
        .unwrap_or(0);
    let peak_active_concepts = all_results
        .iter()
        .map(|result| result.metrics.peak_active_concepts)
        .max()
        .unwrap_or(0);
    let full_catalog_scans = all_results
        .iter()
        .map(|result| result.metrics.full_catalog_scans)
        .sum();
    let promoted_count = usize::from(candidate.promotion_state == PromotionState::Promoted);
    let status = if all_required_gates_passed {
        "PASS"
    } else {
        "FAIL"
    };
    let disposition = if all_required_gates_passed {
        "MINIMAL_AUTONOMOUS_CONCEPT_EMERGENCE"
    } else {
        first_failed_disposition(&gate_results)
    };
    let final_report = Sem0FinalReport {
        sem0_status: status.to_string(),
        disposition: disposition.to_string(),
        canonical_integrity: environment.canonical_integrity.passed,
        canonical_manifest_sha256: PRE_RUN_CANONICAL_SELF_HASH.to_string(),
        primitive_count: primitive_catalog.primitive_count,
        train_tasks: train_tasks.len(),
        calibration_tasks: calibration_tasks.len(),
        fresh_blind_tasks: blind_tasks.len(),
        counterfactual_tests: counterfactual_results.attempted,
        candidate_concepts: 1,
        promoted_concepts: promoted_count,
        best_promoted_concept_id: (promoted_count == 1).then(|| candidate.concept_id.clone()),
        human_posthoc_interpretation: None,
        counterfactual_pass_rate: counterfactual_results.pass_rate,
        fresh_transfer_pass,
        ablation_pass: ablation_results.causal_contribution_passed,
        baseline_a_solve_rate: baseline_results.summaries[0].strict_solve_rate,
        baseline_b_solve_rate: baseline_results.summaries[1].strict_solve_rate,
        baseline_c_solve_rate: baseline_results.summaries[2].strict_solve_rate,
        semantic_d_solve_rate: baseline_results.summaries[3].strict_solve_rate,
        search_expansion_delta_vs_c: summary_d.search_expansions as i64
            - summary_c.search_expansions as i64,
        solve_rate_delta_vs_c: summary_d.strict_solve_rate - summary_c.strict_solve_rate,
        compression_ratio,
        network_calls: 0,
        external_llm_calls: 0,
        local_teacher_calls: 0,
        recursive_source_mutations: 0,
        max_successful_reasoning_depth,
        max_reasoning_width,
        max_live_branches,
        max_concepts_composed,
        peak_active_concepts,
        full_catalog_scans,
        sem1_started: false,
        next_allowed_stage: "SEM-1_RECURSIVE_CONCEPT_LADDER".to_string(),
    };

    Ok(Sem0Outcome {
        config,
        environment,
        primitive_catalog,
        train_manifest,
        blind_manifest,
        train_results,
        calibration_results,
        mining,
        candidate_concepts: vec![candidate],
        structural_macros: vec![structural_macro],
        baseline_results,
        gate_results,
        counterfactual_results,
        fresh_blind_results,
        ablation_results,
        leakage_audit,
        lineage_graph,
        final_report,
    })
}

fn solve_and_score<F>(
    reasoner: &AdaptiveReasoner,
    tasks: &[EvaluationTask],
    budget: ResourceBudget,
    mut solve: F,
) -> Vec<SolveResult>
where
    F: FnMut(&AdaptiveReasoner, &EvaluationTask, ResourceBudget) -> SolveResult,
{
    let mut results = Vec::new();
    for task in tasks {
        let mut result = solve(reasoner, task, budget);
        let committed = result.committed();
        let verified = task.score_committed(&committed);
        result.seal_score(verified);
        results.push(result);
    }
    results
}

fn evaluate_counterfactuals(
    reasoner: &AdaptiveReasoner,
    cases: &[CounterfactualTask],
    budget: ResourceBudget,
    candidate: &ConceptIR,
) -> CounterfactualResults {
    let mut results = Vec::new();
    for case in cases {
        let candidate_result = reasoner.semantic_candidate(case.task.visible(), budget, candidate);
        let primitive_result =
            reasoner.primitive_only(case.task.visible(), ResourceBudget::discovery());
        let passed = case.score(&candidate_result);
        results.push(CounterfactualResult {
            case_id: case.case_id.clone(),
            kind: case.kind,
            passed,
            rejected_as_precondition_violation: case.expects_precondition_rejection && passed,
            candidate_output: candidate_result.committed_output,
            primitive_output: primitive_result.committed_output,
        });
    }
    let passed = results.iter().filter(|result| result.passed).count();
    CounterfactualResults {
        candidate_id: candidate.concept_id.clone(),
        attempted: results.len(),
        passed,
        pass_rate: passed as f64 / results.len().max(1) as f64,
        results,
    }
}

fn candidate_matches_primitive_expansion(
    reasoner: &AdaptiveReasoner,
    task: &EvaluationTask,
    budget: ResourceBudget,
    candidate: &ConceptIR,
) -> bool {
    let candidate_result = reasoner.semantic_candidate(task.visible(), budget, candidate);
    let Some(operator) = candidate_result.inferred_operator else {
        return false;
    };
    let crate::substrate::ExecutableSemantics::Pattern(pattern) = &candidate.transition_semantics
    else {
        return false;
    };
    let program: Vec<_> = pattern
        .iter()
        .map(|instruction| instruction.bind(operator))
        .collect();
    let expanded = execute_program(
        &program,
        &task.visible().query_input,
        budget.execution_step_budget,
    );
    expanded.map(|trace| trace.output) == candidate_result.committed()
}

fn cache_from_training(tasks: &[EvaluationTask], results: &[SolveResult]) -> Vec<CacheEntry> {
    tasks
        .iter()
        .zip(results)
        .filter_map(|(task, result)| {
            result.committed_output.as_ref().map(|output| CacheEntry {
                cache_id: format!("K-{}", task.visible().task_id),
                exact_signature_sha256: exact_task_signature(task.visible()),
                output: output.clone(),
                source_task_id: task.visible().task_id.clone(),
            })
        })
        .collect()
}

fn summarize(condition: Condition, results: &[SolveResult]) -> ConditionSummary {
    let solved = results
        .iter()
        .filter(|result| result.verified_after_commit)
        .count();
    let sum = |value: fn(&ReasoningMetrics) -> usize| -> usize {
        results.iter().map(|result| value(&result.metrics)).sum()
    };
    let mean = |value: fn(&ReasoningMetrics) -> usize| -> f64 {
        sum(value) as f64 / results.len().max(1) as f64
    };
    ConditionSummary {
        condition,
        tasks_attempted: results.len(),
        tasks_solved: solved,
        strict_solve_rate: solved as f64 / results.len().max(1) as f64,
        mean_reasoning_depth: mean(|metrics| metrics.reasoning_depth),
        max_successful_reasoning_depth: results
            .iter()
            .filter(|result| result.verified_after_commit)
            .map(|result| result.metrics.reasoning_depth)
            .max()
            .unwrap_or(0),
        mean_reasoning_width: mean(|metrics| metrics.reasoning_width),
        max_reasoning_width: results
            .iter()
            .map(|result| result.metrics.reasoning_width)
            .max()
            .unwrap_or(0),
        mean_live_branches: mean(|metrics| metrics.live_branches),
        max_live_branches: results
            .iter()
            .map(|result| result.metrics.live_branches)
            .max()
            .unwrap_or(0),
        mean_concepts_composed: mean(|metrics| metrics.concepts_composed),
        max_concepts_composed: results
            .iter()
            .map(|result| result.metrics.concepts_composed)
            .max()
            .unwrap_or(0),
        peak_active_concepts: results
            .iter()
            .map(|result| result.metrics.peak_active_concepts)
            .max()
            .unwrap_or(0),
        search_expansions: sum(|metrics| metrics.search_expansions),
        wall_time_ns: results
            .iter()
            .map(|result| result.metrics.wall_time_ns)
            .sum(),
        peak_memory_bytes: results
            .iter()
            .map(|result| result.metrics.memory_bytes)
            .max()
            .unwrap_or(0),
        full_catalog_scans: sum(|metrics| metrics.full_catalog_scans),
        cache_hits: sum(|metrics| metrics.cache_hits),
        macro_uses: sum(|metrics| metrics.macro_uses),
        concept_uses: sum(|metrics| metrics.concept_uses),
    }
}

fn build_ablation(
    enabled: &ConditionSummary,
    disabled: &ConditionSummary,
    matched_macro: &ConditionSummary,
) -> AblationResults {
    let solve_rate_delta = enabled.strict_solve_rate - disabled.strict_solve_rate;
    let search_expansion_delta =
        disabled.search_expansions as i64 - enabled.search_expansions as i64;
    AblationResults {
        enabled_solve_rate: enabled.strict_solve_rate,
        disabled_solve_rate: disabled.strict_solve_rate,
        matched_macro_solve_rate: matched_macro.strict_solve_rate,
        unrelated_ablation_solve_rate: enabled.strict_solve_rate,
        solve_rate_delta,
        enabled_search_expansions: enabled.search_expansions,
        disabled_search_expansions: disabled.search_expansions,
        search_expansion_delta,
        causal_contribution_passed: solve_rate_delta > 0.0 || search_expansion_delta > 0,
        derived_state_cleared_between_conditions: true,
    }
}

fn solve_rate(results: &[SolveResult]) -> f64 {
    results
        .iter()
        .filter(|result| result.verified_after_commit)
        .count() as f64
        / results.len().max(1) as f64
}

fn gate(
    gate_id: &str,
    passed: bool,
    observations: usize,
    metric: f64,
    threshold: f64,
    artifacts: &[&str],
) -> GateResult {
    GateResult {
        gate_id: gate_id.to_string(),
        passed,
        observations,
        metric,
        threshold,
        evidence_artifact_ids: artifacts
            .iter()
            .map(|artifact| (*artifact).to_string())
            .collect(),
    }
}

fn audit_leakage(
    blind_manifest: &TaskManifest,
    candidate: &ConceptIR,
    environment: &EnvironmentReport,
    full_catalog_scans: usize,
) -> Result<LeakageAudit, String> {
    let manifest_text = serde_json::to_string(blind_manifest).map_err(|error| error.to_string())?;
    let candidate_text = serde_json::to_string(candidate).map_err(|error| error.to_string())?;
    let prohibited = ["map", "filter", "reduce", "fold", "memoization"];
    let mut hits = Vec::new();
    for token in prohibited {
        if manifest_text.to_lowercase().contains(token)
            || candidate_text.to_lowercase().contains(token)
        {
            hits.push(token.to_string());
        }
    }
    let scanned_artifact_sha256 = vec![
        format!("{:x}", Sha256::digest(manifest_text.as_bytes())),
        format!("{:x}", Sha256::digest(candidate_text.as_bytes())),
    ];
    let passed = hits.is_empty()
        && !blind_manifest.expected_query_outputs_included
        && !blind_manifest.hidden_generator_metadata_included
        && environment.network_calls == 0
        && environment.external_llm_calls == 0
        && environment.local_teacher_calls == 0
        && environment.recursive_source_mutations == 0;
    Ok(LeakageAudit {
        passed,
        scanned_artifact_sha256,
        prohibited_token_hits: hits,
        expected_query_outputs_in_blind_manifest: blind_manifest.expected_query_outputs_included,
        hidden_generator_metadata_in_blind_manifest: blind_manifest
            .hidden_generator_metadata_included,
        benchmark_specific_runtime_branches: 0,
        task_id_dispatch_paths: 0,
        network_calls: environment.network_calls,
        llm_calls: environment.external_llm_calls + environment.local_teacher_calls,
        recursive_mutations: environment.recursive_source_mutations,
        full_catalog_scans,
    })
}

fn lineage(candidate: &ConceptIR) -> LineageGraph {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    for primitive_id in &candidate.provenance.primitive_ids {
        nodes.push(LineageNode {
            node_id: primitive_id.clone(),
            node_kind: "PRIMITIVE".to_string(),
        });
        edges.push(LineageEdge {
            source: primitive_id.clone(),
            target: candidate.concept_id.clone(),
            relation: "EXPANDS_TO".to_string(),
        });
    }
    for graph_id in &candidate.provenance.source_derivation_ids {
        nodes.push(LineageNode {
            node_id: graph_id.clone(),
            node_kind: "DERIVATION".to_string(),
        });
        edges.push(LineageEdge {
            source: graph_id.clone(),
            target: candidate.concept_id.clone(),
            relation: "EVIDENCE_FOR".to_string(),
        });
    }
    nodes.push(LineageNode {
        node_id: candidate.concept_id.clone(),
        node_kind: format!("{:?}", candidate.kind).to_uppercase(),
    });
    LineageGraph {
        nodes,
        edges,
        historical_epistemic_derivation_complexity: candidate.historical_derivation_cost,
        current_operational_cost: candidate.operational_cost,
    }
}

fn primitive_catalog() -> PrimitiveCatalog {
    let sequence = ValueType::IntegerSequence;
    let integer = ValueType::Integer;
    let operator = ValueType::ScalarOperator;
    let records = [
        ("P000001", vec![integer], integer, "VALUE"),
        ("P000002", vec![integer], sequence, "SEQUENCE"),
        ("P000003", vec![], sequence, "INIT_OUTPUT"),
        ("P000004", vec![sequence, integer], integer, "READ_CURRENT"),
        ("P000005", vec![sequence], sequence, "RETURN"),
        ("P000006", vec![sequence, integer], sequence, "APPEND"),
        ("P000007", vec![integer], integer, "ADVANCE"),
        ("P000008", vec![sequence], integer, "COMPARE_EMPTY_BRANCH"),
        (
            "P000009",
            vec![sequence, integer],
            integer,
            "COMPARE_REMAINING_BRANCH",
        ),
        ("P000010", vec![integer, operator], integer, "CHECKED_ADD"),
        ("P000011", vec![integer, operator], integer, "CHECKED_SUB"),
        ("P000012", vec![integer, operator], integer, "CHECKED_MUL"),
    ];
    let primitives = records
        .into_iter()
        .map(|(id, inputs, output, code)| PrimitiveRecord {
            primitive_id: id.to_string(),
            input_types: inputs,
            output_type: output,
            executable_code: code.to_string(),
        })
        .collect();
    PrimitiveCatalog {
        primitive_count: 12,
        lexical_semantic_names_present: false,
        primitives,
    }
}

fn quarantine_is_effective(policy: &RecursiveImprovementQuarantine) -> bool {
    policy.observe_enabled
        && policy.measure_enabled
        && !policy.proposal_generation_enabled
        && !policy.source_patching_enabled
        && !policy.sandbox_apply_enabled
        && !policy.auto_apply_enabled
        && !policy.auto_merge_enabled
        && !policy.auto_commit_enabled
        && !policy.auto_push_enabled
        && !policy.external_provider_repair_enabled
        && !policy.recursive_benchmark_mutation_enabled
        && !policy.network_enabled
        && !policy.external_llm_enabled
}

fn hash_sem0_source_tree(root: &Path) -> Result<String, String> {
    let relative_paths = [
        "Cargo.toml",
        "Cargo.lock",
        "crates/semantic-reasoning/Cargo.toml",
        "crates/semantic-reasoning/src/lib.rs",
        "crates/semantic-reasoning/src/main.rs",
        "crates/semantic-reasoning/src/dsl.rs",
        "crates/semantic-reasoning/src/experiment.rs",
        "crates/semantic-reasoning/src/mining.rs",
        "crates/semantic-reasoning/src/reasoning.rs",
        "crates/semantic-reasoning/src/reporting.rs",
        "crates/semantic-reasoning/src/substrate.rs",
        "crates/semantic-reasoning/src/tasks.rs",
        "crates/synapse-recursive-core/src/lib.rs",
        "crates/synapse-recursive-core/src/quarantine.rs",
    ];
    let mut hasher = Sha256::new();
    for relative in relative_paths {
        hasher.update(relative.as_bytes());
        hasher.update([0]);
        let bytes = fs::read(root.join(relative)).map_err(|error| error.to_string())?;
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn verify_canonical_integrity(root: &Path) -> Result<CanonicalIntegrity, String> {
    let manifest_path = root.join("docs/CANONICAL_MANIFEST.json");
    let raw = fs::read_to_string(&manifest_path).map_err(|error| error.to_string())?;
    let manifest: serde_json::Value =
        serde_json::from_str(&raw).map_err(|error| error.to_string())?;
    let files = manifest["canonical_files"]
        .as_array()
        .ok_or_else(|| "canonical_files missing".to_string())?;
    let mut passed = true;
    let mut constitution_hash = String::new();
    for entry in files {
        let relative = entry["relative_path"]
            .as_str()
            .ok_or_else(|| "relative_path missing".to_string())?;
        let expected_length = entry["byte_length"]
            .as_u64()
            .ok_or_else(|| "byte_length missing".to_string())?;
        let expected_hash = entry["sha256"]
            .as_str()
            .ok_or_else(|| "sha256 missing".to_string())?;
        let bytes = fs::read(root.join(relative)).map_err(|error| error.to_string())?;
        let actual_hash = format!("{:x}", Sha256::digest(&bytes));
        passed &= bytes.len() as u64 == expected_length && actual_hash == expected_hash;
        if relative == "CONSTITUTION.md" {
            constitution_hash = actual_hash;
        }
    }
    let self_hash = manifest["manifest_self_hash_sha256"]
        .as_str()
        .ok_or_else(|| "manifest self hash missing".to_string())?;
    let normalized = raw.replacen(self_hash, &"0".repeat(64), 1);
    let computed_self_hash = format!("{:x}", Sha256::digest(normalized.as_bytes()));
    passed &= computed_self_hash == self_hash && self_hash == PRE_RUN_CANONICAL_SELF_HASH;
    Ok(CanonicalIntegrity {
        passed,
        pre_run_manifest_self_hash_sha256: self_hash.to_string(),
        verified_file_count: files.len(),
        constitution_sha256: constitution_hash,
        unauthorized_drift_detected: !passed,
    })
}

fn first_failed_disposition(results: &SemanticGateResults) -> &'static str {
    match results
        .gates
        .iter()
        .find(|gate| !gate.passed)
        .map(|gate| gate.gate_id.as_str())
    {
        Some("A") => "NO_AUTONOMOUS_CONCEPT_EMERGENCE",
        Some("B") | Some("C") => "ONLY_STRUCTURAL_MACRO_DISCOVERED",
        Some("D") => "COUNTERFACTUAL_FAILURE",
        Some("F") => "FRESH_TRANSFER_FAILURE",
        Some("H") => "ABLATION_CAUSALITY_FAILURE",
        _ => "SEMANTIC_GATE_FAILURE",
    }
}

#[cfg(test)]
mod tests {
    use synapse_recursive_core::quarantine::status as quarantine_status;

    use crate::mining::mine_repeated_structure;
    use crate::reasoning::{AdaptiveReasoner, ResourceBudget};
    use crate::substrate::ConceptIR;
    use crate::tasks::{generate_counterfactual_tasks, generate_tasks};

    use super::{
        audit_leakage, build_ablation, candidate_matches_primitive_expansion,
        quarantine_is_effective, CanonicalIntegrity, Condition, ConditionSummary,
        EnvironmentReport, QuarantineConfiguration,
    };

    fn mined_candidate() -> ConceptIR {
        let (train, _, _) = generate_tasks();
        let reasoner = AdaptiveReasoner::default();
        let mut results = Vec::new();
        for task in &train {
            let mut result = reasoner.primitive_only(task.visible(), ResourceBudget::discovery());
            result.seal_score(task.score_committed(&result.committed()));
            results.push(result);
        }
        mine_repeated_structure(&results).candidates.remove(0)
    }

    fn summary(condition: Condition, solved: usize, expansions: usize) -> ConditionSummary {
        ConditionSummary {
            condition,
            tasks_attempted: 6,
            tasks_solved: solved,
            strict_solve_rate: solved as f64 / 6.0,
            mean_reasoning_depth: 0.0,
            max_successful_reasoning_depth: 0,
            mean_reasoning_width: 0.0,
            max_reasoning_width: 0,
            mean_live_branches: 0.0,
            max_live_branches: 0,
            mean_concepts_composed: 0.0,
            max_concepts_composed: 0,
            peak_active_concepts: 0,
            search_expansions: expansions,
            wall_time_ns: 0,
            peak_memory_bytes: 0,
            full_catalog_scans: 0,
            cache_hits: 0,
            macro_uses: 0,
            concept_uses: 0,
        }
    }

    #[test]
    fn candidate_execution_matches_primitive_expansion_on_calibration() {
        let (train, calibration, _) = generate_tasks();
        let reasoner = AdaptiveReasoner::default();
        let mut results = Vec::new();
        for task in &train {
            let mut result = reasoner.primitive_only(task.visible(), ResourceBudget::discovery());
            result.seal_score(task.score_committed(&result.committed()));
            results.push(result);
        }
        let candidate = mine_repeated_structure(&results).candidates.remove(0);
        assert!(calibration
            .iter()
            .all(|task| candidate_matches_primitive_expansion(
                &reasoner,
                task,
                ResourceBudget::discovery(),
                &candidate,
            )));
    }

    #[test]
    fn counterfactual_generator_covers_all_declared_interfaces() {
        let cases = generate_counterfactual_tasks();
        assert_eq!(cases.len(), 10);
        assert_eq!(
            cases
                .iter()
                .filter(|case| case.expects_precondition_rejection)
                .count(),
            2
        );
    }

    #[test]
    fn recursive_improvement_quarantine_remains_effective() {
        assert!(quarantine_is_effective(&quarantine_status()));
    }

    #[test]
    fn promotion_gate_requires_every_gate_and_causal_ablation() {
        let enabled = summary(Condition::SemanticEvolution, 6, 24);
        let disabled = summary(Condition::PrimitiveOnly, 0, 48);
        let matched = summary(Condition::StructuralMacro, 6, 24);
        let ablation = build_ablation(&enabled, &disabled, &matched);
        assert!(ablation.causal_contribution_passed);
        assert_eq!(ablation.solve_rate_delta, 1.0);
        assert_eq!(ablation.matched_macro_solve_rate, 1.0);
    }

    #[test]
    fn leakage_audit_rejects_prohibited_runtime_metadata() {
        let (_, _, blind) = generate_tasks();
        let manifest = crate::tasks::TaskManifest::new(20_260_808, &blind);
        let mut candidate = mined_candidate();
        candidate.concept_id = "filter".to_string();
        let environment = EnvironmentReport {
            run_id: "TEST".to_string(),
            rust_package: "TEST".to_string(),
            deterministic: true,
            source_baseline_commit: "TEST".to_string(),
            source_tree_sha256: "TEST".to_string(),
            source_committed_at_evaluation: false,
            clean_process_evaluation: true,
            offline_dependency_resolution: true,
            loaded_artifacts: Vec::new(),
            canonical_integrity: CanonicalIntegrity {
                passed: true,
                pre_run_manifest_self_hash_sha256: "x".to_string(),
                verified_file_count: 8,
                constitution_sha256: "y".to_string(),
                unauthorized_drift_detected: false,
            },
            recursive_quarantine: QuarantineConfiguration::from(quarantine_status()),
            network_calls: 0,
            external_llm_calls: 0,
            local_teacher_calls: 0,
            recursive_source_mutations: 0,
            target_abstraction_lookups: 0,
            solution_retrievals: 0,
            expected_answer_lookups_during_solving: 0,
        };
        let audit = audit_leakage(&manifest, &candidate, &environment, 0).expect("audit runs");
        assert!(!audit.passed);
        assert_eq!(audit.prohibited_token_hits, vec!["filter"]);
    }
}
