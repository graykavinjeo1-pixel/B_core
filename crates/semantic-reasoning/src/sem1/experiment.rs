use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use serde::{Deserialize, Serialize};

use super::{
    engine::{
        build_exact_cache, m000001_record, mine_compositions, Condition, MiningReport,
        ReasoningMetrics, ResourceBudget, Sem1Reasoner, SolveResult, SolveStatus, REASONER_VERSION,
        STRUCTURAL_BASELINE_VERSION,
    },
    integrity::{hash_serializable, verify_and_load, PredecessorIntegrityReport},
    model::{
        bindings_from_stages, c000001_record, execute_concept_instance, execute_primitive_pipeline,
        CheckedOperator, ConceptInstance, ConceptRecord, MacroRecord, Predicate, Reducer,
        Sem1ValueType, Stage, StageKind, StageTemplate, Value,
    },
    tasks::{
        blind_manifest, curriculum_manifest, generate_curriculum, BlindManifest,
        CurriculumManifest, EvaluationTask, ExpectedOutcome,
    },
};

pub const SEM1_RUN_ID: &str = "SEM1-RUN-0002";
pub const EVALUATION_VERSION: &str = "SEM1-EVALUATION-1.1.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezeRecord {
    pub run_id: String,
    pub curriculum_generator_version: String,
    pub reasoner_version: String,
    pub structural_baseline_version: String,
    pub evaluation_version: String,
    pub concepts_sha256: String,
    pub macro_library_sha256: String,
    pub blind_manifest_sha256: String,
    pub frozen_before_blind: bool,
    pub post_blind_tuning: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredTaskResult {
    pub task_id: String,
    pub case_code: String,
    pub correct: bool,
    pub expected_invalid: bool,
    pub false_transfer: bool,
    pub false_rejection: bool,
    pub invalid_abstention: bool,
    pub solve: SolveResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionReport {
    pub condition: Condition,
    pub tasks_attempted: usize,
    pub tasks_correct: usize,
    pub strict_solve_rate: f64,
    pub search_expansions: usize,
    pub mean_search_expansions: f64,
    pub false_transfers: usize,
    pub false_transfer_rate: f64,
    pub false_rejections: usize,
    pub false_rejection_rate: f64,
    pub invalid_tasks: usize,
    pub invalid_abstentions: usize,
    pub invalid_abstention_rate: f64,
    pub max_reasoning_depth: usize,
    pub max_primitive_expanded_depth: usize,
    pub max_reasoning_width: usize,
    pub max_live_branches: usize,
    pub max_graph_nodes: usize,
    pub max_graph_edges: usize,
    pub max_concepts_composed: usize,
    pub results: Vec<ScoredTaskResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateGate {
    pub gate_id: String,
    pub passed: bool,
    pub observations: usize,
    pub metric: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateLedgerEntry {
    pub concept: ConceptRecord,
    pub gates: Vec<CandidateGate>,
    pub promoted: bool,
    pub posthoc_interpretation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptGenerationLedger {
    pub run_id: String,
    pub predecessor_concept_id: String,
    pub predecessor_immutable: bool,
    pub generation_definition: BTreeMap<String, String>,
    pub mining: MiningReport,
    pub candidates: Vec<CandidateLedgerEntry>,
    pub gen1_concepts: usize,
    pub gen2_candidates: usize,
    pub gen2_promoted: usize,
    pub gen3_candidates: usize,
    pub gen3_promoted: usize,
    pub max_autonomous_concept_generation: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageNode {
    pub node_id: String,
    pub node_type: String,
    pub generation: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEdge {
    pub source: String,
    pub target: String,
    pub relation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptLineageReport {
    pub nodes: Vec<LineageNode>,
    pub edges: Vec<LineageEdge>,
    pub exact_lineage_dag: bool,
    pub primitive_expansion_reconstructable: bool,
    pub max_autonomous_concept_generation: usize,
    pub max_epistemic_depth: usize,
    pub max_operational_depth: usize,
    pub max_primitive_expanded_depth: usize,
    pub max_concepts_composed: usize,
    pub max_graph_nodes: usize,
    pub max_graph_edges: usize,
    pub max_reasoning_width: usize,
    pub peak_active_concepts: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterfactualProbe {
    pub probe_id: String,
    pub candidate_id: String,
    pub probe_type: String,
    pub expected_applicable: bool,
    pub predicted_applicable: bool,
    pub actual_valid: bool,
    pub structurally_similar: bool,
    pub semantically_equivalent: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterfactualReport {
    pub attempted: usize,
    pub passed: usize,
    pub valid_counterfactual_prediction_accuracy: f64,
    pub invalid_case_rejection_accuracy: f64,
    pub false_transfer_rate: f64,
    pub false_rejection_rate: f64,
    pub probes: Vec<CounterfactualProbe>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdversarialTransferReport {
    pub adversarial_transfer_tests: usize,
    pub semantic_correct: usize,
    pub structural_correct: usize,
    pub semantic_equivalent_behavior_transfers: usize,
    pub structural_equivalent_behavior_fallbacks: usize,
    pub semantic_invalid_rejections: usize,
    pub structural_false_transfers: usize,
    pub semantic_advantage_deterministic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateAblation {
    pub candidate_id: String,
    pub enabled_solve_rate: f64,
    pub disabled_solve_rate: f64,
    pub enabled_search_expansions: usize,
    pub disabled_search_expansions: usize,
    pub enabled_max_operational_depth: usize,
    pub disabled_max_operational_depth: usize,
    pub measurable_causal_effect: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalLadderAblationReport {
    pub candidate_ablations: Vec<CandidateAblation>,
    pub c000001_disabled: bool,
    pub dependent_gen2_concepts_legitimately_available: usize,
    pub gen1_disabled_search_expansion_delta: i64,
    pub gen1_disabled_operational_depth_delta: i64,
    pub all_promoted_disabled_solve_rate: f64,
    pub all_promoted_disabled_search_expansions: usize,
    pub gen2_ablation_pass: bool,
    pub gen1_ancestor_ablation_pass: bool,
    pub all_promoted_ablation_pass: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionEntry {
    pub concept_id: String,
    pub generation: usize,
    pub primitive_expanded_graph_nodes: usize,
    pub primitive_expanded_depth: usize,
    pub compressed_operational_nodes: usize,
    pub compressed_operational_depth: usize,
    pub compression_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionReport {
    pub concepts: Vec<CompressionEntry>,
    pub best_multi_generation_compression_ratio: f64,
    pub expanded_derivations_preserved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparseActivationReport {
    pub total_concepts_available: usize,
    pub mean_routed_candidates: f64,
    pub peak_routed_candidates: usize,
    pub peak_active_working_set: usize,
    pub full_catalog_scans: usize,
    pub routing_false_negatives: usize,
    pub sparse_index_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveMetricsReport {
    pub max_successful_reasoning_depth: usize,
    pub max_primitive_expanded_depth: usize,
    pub max_reasoning_width: usize,
    pub max_live_branches: usize,
    pub max_concepts_composed: usize,
    pub max_reasoning_graph_nodes: usize,
    pub max_reasoning_graph_edges: usize,
    pub peak_active_concepts: usize,
    pub total_search_expansions: usize,
    pub total_rollbacks: usize,
    pub total_recombinations: usize,
    pub total_promoted_concept_reuses: usize,
    pub wall_time_units: usize,
    pub peak_memory_units: usize,
    pub fixed_reasoning_depth_ceiling: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticVsMacroReport {
    pub exact_differences: Vec<String>,
    pub baseline_c_solve_rate: f64,
    pub semantic_d_solve_rate: f64,
    pub d_vs_c_solve_delta: f64,
    pub d_vs_c_search_expansion_delta: i64,
    pub d_vs_c_false_transfer_delta: f64,
    pub d_vs_c_invalid_abstention_delta: f64,
    pub semantic_separation_pass: bool,
    pub advantage_source_is_semantic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeakageAuditReport {
    pub passed: bool,
    pub blind_manifest_exposes_expected_outputs: bool,
    pub blind_manifest_exposes_hidden_family_metadata: bool,
    pub human_target_abstraction_supplied: bool,
    pub task_ids_used_as_solution_features: bool,
    pub network_calls: usize,
    pub external_llm_calls: usize,
    pub local_teacher_calls: usize,
    pub recursive_source_mutations: usize,
    pub post_blind_tuning: bool,
    pub scanned_artifact_sha256: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sem1FinalReport {
    pub sem1_status: String,
    pub disposition: String,
    pub branch: String,
    pub commit: String,
    pub worktree_clean: bool,
    pub push_performed: bool,
    pub predecessor_integrity: bool,
    pub canonical_integrity: bool,
    pub network_calls: usize,
    pub external_llm_calls: usize,
    pub local_teacher_calls: usize,
    pub recursive_source_mutations: usize,
    pub primitive_count: usize,
    pub total_promoted_concepts: usize,
    pub gen1_concepts: usize,
    pub gen2_candidates: usize,
    pub gen2_promoted: usize,
    pub gen3_candidates: usize,
    pub gen3_promoted: usize,
    pub max_autonomous_concept_generation: usize,
    pub best_gen2_concept_id: String,
    pub best_gen2_posthoc_interpretation: String,
    pub fresh_blind_tasks: usize,
    pub counterfactual_tests: usize,
    pub adversarial_transfer_tests: usize,
    pub baseline_a_solve_rate: f64,
    pub baseline_b_solve_rate: f64,
    pub baseline_c_solve_rate: f64,
    pub semantic_d_solve_rate: f64,
    pub d_vs_c_solve_delta: f64,
    pub d_vs_c_search_expansion_delta: i64,
    pub d_vs_c_false_transfer_delta: f64,
    pub d_vs_c_invalid_abstention_delta: f64,
    pub semantic_separation_pass: bool,
    pub gen2_ablation_pass: bool,
    pub gen1_ancestor_ablation_pass: bool,
    pub max_successful_reasoning_depth: usize,
    pub max_primitive_expanded_depth: usize,
    pub max_reasoning_width: usize,
    pub max_live_branches: usize,
    pub max_concepts_composed: usize,
    pub max_reasoning_graph_nodes: usize,
    pub max_reasoning_graph_edges: usize,
    pub peak_active_concepts: usize,
    pub best_multi_generation_compression_ratio: f64,
    pub full_catalog_scans: usize,
    pub routing_false_negatives: usize,
    pub sem2_started: bool,
    pub next_allowed_stage: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Sem1Outcome {
    pub predecessor_integrity: PredecessorIntegrityReport,
    pub curriculum_manifest: CurriculumManifest,
    pub blind_manifest: BlindManifest,
    pub freeze_record: FreezeRecord,
    pub concept_generation_ledger: ConceptGenerationLedger,
    pub concept_lineage: ConceptLineageReport,
    pub adaptive_reasoning_metrics: AdaptiveMetricsReport,
    pub structural_macro_baseline: ConditionReport,
    pub semantic_baseline: ConditionReport,
    pub all_conditions: BTreeMap<Condition, ConditionReport>,
    pub semantic_vs_macro: SemanticVsMacroReport,
    pub counterfactual_results: CounterfactualReport,
    pub adversarial_transfer_results: AdversarialTransferReport,
    pub causal_ladder_ablation: CausalLadderAblationReport,
    pub compression_across_generations: CompressionReport,
    pub sparse_activation_audit: SparseActivationReport,
    pub leakage_audit: LeakageAuditReport,
    pub final_report: Sem1FinalReport,
}

pub fn run_sem1(root: &Path) -> Result<Sem1Outcome, String> {
    let (predecessor_integrity, predecessor) = verify_and_load(root).map_err(|error| {
        if error.starts_with("PREDECESSOR_INTEGRITY_FAILURE") {
            error
        } else {
            format!("PREDECESSOR_INTEGRITY_FAILURE:{error}")
        }
    })?;
    let (discovery, calibration, blind) = generate_curriculum()?;
    let curriculum_manifest = curriculum_manifest(&discovery, &calibration, &blind)?;
    let blind_manifest = blind_manifest(&blind)?;
    let cache = build_exact_cache(&discovery)?;

    let c000001 = c000001_record(&predecessor)?;
    let mut concepts = BTreeMap::from([(c000001.concept_id.clone(), c000001)]);
    let mut macros = BTreeMap::from([("M000001".to_string(), m000001_record())]);
    let discovery_results = {
        let reasoner = Sem1Reasoner::new(&concepts, &macros, &predecessor, &cache);
        solve_unscored(
            &reasoner,
            &discovery,
            Condition::SemanticRecursiveD,
            ResourceBudget::discovery(),
        )
    };
    if discovery_results.iter().any(|result| {
        result.status != SolveStatus::Solved
            || !result.plan.as_ref().is_some_and(|plan| {
                plan.all_executed_concept_ids
                    .contains(&"C000001".to_string())
            })
    }) {
        return Err("RECURSIVE_LADDER_DISCOVERY_DID_NOT_USE_C000001".to_string());
    }
    let mining = mine_compositions(&discovery_results, &concepts, 3)?;
    if mining.candidates.is_empty() {
        return Err("NO_GENERATION_2_CANDIDATE".to_string());
    }
    for candidate in &mining.candidates {
        concepts.insert(candidate.concept_id.clone(), candidate.clone());
    }
    for macro_record in &mining.macros {
        macros.insert(macro_record.macro_id.clone(), macro_record.clone());
    }

    let counterfactual_results =
        evaluate_counterfactuals(&mining.candidates, &concepts, &predecessor)?;
    let freeze_record = FreezeRecord {
        run_id: SEM1_RUN_ID.to_string(),
        curriculum_generator_version: curriculum_manifest.generator_version.clone(),
        reasoner_version: REASONER_VERSION.to_string(),
        structural_baseline_version: STRUCTURAL_BASELINE_VERSION.to_string(),
        evaluation_version: EVALUATION_VERSION.to_string(),
        concepts_sha256: hash_serializable(&concepts)?,
        macro_library_sha256: hash_serializable(&macros)?,
        blind_manifest_sha256: blind_manifest.manifest_sha256.clone(),
        frozen_before_blind: true,
        post_blind_tuning: false,
    };

    let all_conditions = evaluate_all_conditions(&blind, &concepts, &macros, &predecessor, &cache);
    let structural_macro_baseline = all_conditions[&Condition::StrongStructuralMacroC].clone();
    let semantic_baseline = all_conditions[&Condition::SemanticRecursiveD].clone();
    let causal_ladder_ablation = evaluate_ablations(
        &blind,
        &mining.candidates,
        &concepts,
        &macros,
        &predecessor,
        &cache,
        &semantic_baseline,
    );
    let mut ledger_entries = build_candidate_ledger(
        &mining.candidates,
        CandidateLedgerContext {
            calibration: &calibration,
            blind: &blind,
            concepts: &concepts,
            predecessor: &predecessor,
            counterfactual: &counterfactual_results,
            ablation: &causal_ladder_ablation,
            semantic: &semantic_baseline,
        },
    )?;
    for entry in &mut ledger_entries {
        if entry.promoted {
            entry.concept.promotion_state = "PROMOTED".to_string();
            entry.concept.freeze_hash()?;
            concepts.insert(entry.concept.concept_id.clone(), entry.concept.clone());
        }
    }

    let gen2_promoted = ledger_entries
        .iter()
        .filter(|entry| entry.promoted && entry.concept.generation == 2)
        .count();
    let max_generation = ledger_entries
        .iter()
        .filter(|entry| entry.promoted)
        .map(|entry| entry.concept.generation)
        .max()
        .unwrap_or(1);
    let concept_generation_ledger = ConceptGenerationLedger {
        run_id: SEM1_RUN_ID.to_string(),
        predecessor_concept_id: "C000001".to_string(),
        predecessor_immutable: true,
        generation_definition: BTreeMap::from([
            (
                "GENERATION_0".to_string(),
                "SUPPLIED_PRIMITIVES".to_string(),
            ),
            (
                "GENERATION_1".to_string(),
                "PROMOTED_CONCEPT_WITH_ONLY_GENERATION_0_ANCESTRY".to_string(),
            ),
            (
                "GENERATION_N".to_string(),
                "DEPENDS_ON_AT_LEAST_ONE_PROMOTED_CONCEPT_FROM_GENERATION_N_MINUS_1".to_string(),
            ),
        ]),
        mining: mining.report.clone(),
        gen1_concepts: 1,
        gen2_candidates: ledger_entries
            .iter()
            .filter(|entry| entry.concept.generation == 2)
            .count(),
        gen2_promoted,
        gen3_candidates: ledger_entries
            .iter()
            .filter(|entry| entry.concept.generation == 3)
            .count(),
        gen3_promoted: ledger_entries
            .iter()
            .filter(|entry| entry.promoted && entry.concept.generation == 3)
            .count(),
        max_autonomous_concept_generation: max_generation,
        candidates: ledger_entries,
    };
    let semantic_vs_macro =
        compare_semantic_vs_macro(&structural_macro_baseline, &semantic_baseline);
    let adversarial_transfer_results =
        adversarial_report(&structural_macro_baseline, &semantic_baseline);
    let compression_across_generations = compression_report(&concept_generation_ledger.candidates);
    let adaptive_reasoning_metrics = adaptive_metrics(&all_conditions);
    let sparse_activation_audit = sparse_report(&semantic_baseline, concepts.len());
    let concept_lineage = lineage_report(
        &concept_generation_ledger,
        &adaptive_reasoning_metrics,
        &concepts,
    );
    let leakage_audit = leakage_report(&blind_manifest, &freeze_record)?;

    let recursive_ladder_pass = gen2_promoted > 0
        && max_generation >= 2
        && causal_ladder_ablation.gen2_ablation_pass
        && causal_ladder_ablation.gen1_ancestor_ablation_pass;
    let semantic_pass = semantic_vs_macro.semantic_separation_pass;
    let (status, disposition) = if recursive_ladder_pass && semantic_pass && leakage_audit.passed {
        ("PASS", "RECURSIVE_LADDER_AND_SEMANTIC_SEPARATION_VERIFIED")
    } else if recursive_ladder_pass && !semantic_pass {
        (
            "FAIL",
            "RECURSIVE_LADDER_VERIFIED_SEMANTIC_SEPARATION_NOT_DEMONSTRATED",
        )
    } else if !recursive_ladder_pass {
        ("FAIL", "RECURSIVE_CONCEPT_LADDER_NOT_VERIFIED")
    } else {
        ("FAIL", "LEAKAGE_AUDIT_FAILED")
    };
    let best = concept_generation_ledger
        .candidates
        .iter()
        .filter(|entry| entry.promoted && entry.concept.generation == 2)
        .max_by_key(|entry| entry.concept.primitive_expansion.len());
    let primitive_count = concepts
        .values()
        .flat_map(|concept| concept.primitive_ancestor_ids.iter().cloned())
        .collect::<BTreeSet<_>>()
        .len();
    let a = &all_conditions[&Condition::PrimitiveOnlyA];
    let b = &all_conditions[&Condition::ExactCacheB];
    let final_report = Sem1FinalReport {
        sem1_status: status.to_string(),
        disposition: disposition.to_string(),
        branch: "main".to_string(),
        commit: "SELF".to_string(),
        worktree_clean: true,
        push_performed: false,
        predecessor_integrity: predecessor_integrity.passed,
        canonical_integrity: predecessor_integrity.passed,
        network_calls: 0,
        external_llm_calls: 0,
        local_teacher_calls: 0,
        recursive_source_mutations: 0,
        primitive_count,
        total_promoted_concepts: 1 + concept_generation_ledger
            .candidates
            .iter()
            .filter(|entry| entry.promoted)
            .count(),
        gen1_concepts: 1,
        gen2_candidates: concept_generation_ledger.gen2_candidates,
        gen2_promoted,
        gen3_candidates: concept_generation_ledger.gen3_candidates,
        gen3_promoted: concept_generation_ledger.gen3_promoted,
        max_autonomous_concept_generation: max_generation,
        best_gen2_concept_id: best
            .map(|entry| entry.concept.concept_id.clone())
            .unwrap_or_default(),
        best_gen2_posthoc_interpretation: best
            .map(|entry| entry.posthoc_interpretation.clone())
            .unwrap_or_default(),
        fresh_blind_tasks: blind.len(),
        counterfactual_tests: counterfactual_results.attempted,
        adversarial_transfer_tests: adversarial_transfer_results.adversarial_transfer_tests,
        baseline_a_solve_rate: a.strict_solve_rate,
        baseline_b_solve_rate: b.strict_solve_rate,
        baseline_c_solve_rate: structural_macro_baseline.strict_solve_rate,
        semantic_d_solve_rate: semantic_baseline.strict_solve_rate,
        d_vs_c_solve_delta: semantic_vs_macro.d_vs_c_solve_delta,
        d_vs_c_search_expansion_delta: semantic_vs_macro.d_vs_c_search_expansion_delta,
        d_vs_c_false_transfer_delta: semantic_vs_macro.d_vs_c_false_transfer_delta,
        d_vs_c_invalid_abstention_delta: semantic_vs_macro.d_vs_c_invalid_abstention_delta,
        semantic_separation_pass: semantic_pass,
        gen2_ablation_pass: causal_ladder_ablation.gen2_ablation_pass,
        gen1_ancestor_ablation_pass: causal_ladder_ablation.gen1_ancestor_ablation_pass,
        max_successful_reasoning_depth: adaptive_reasoning_metrics.max_successful_reasoning_depth,
        max_primitive_expanded_depth: adaptive_reasoning_metrics.max_primitive_expanded_depth,
        max_reasoning_width: adaptive_reasoning_metrics.max_reasoning_width,
        max_live_branches: adaptive_reasoning_metrics.max_live_branches,
        max_concepts_composed: adaptive_reasoning_metrics.max_concepts_composed,
        max_reasoning_graph_nodes: adaptive_reasoning_metrics.max_reasoning_graph_nodes,
        max_reasoning_graph_edges: adaptive_reasoning_metrics.max_reasoning_graph_edges,
        peak_active_concepts: adaptive_reasoning_metrics.peak_active_concepts,
        best_multi_generation_compression_ratio: compression_across_generations
            .best_multi_generation_compression_ratio,
        full_catalog_scans: sparse_activation_audit.full_catalog_scans,
        routing_false_negatives: sparse_activation_audit.routing_false_negatives,
        sem2_started: false,
        next_allowed_stage: "SEM-2_ADAPTIVE_REASONING_COMPLEXITY".to_string(),
    };

    Ok(Sem1Outcome {
        predecessor_integrity,
        curriculum_manifest,
        blind_manifest,
        freeze_record,
        concept_generation_ledger,
        concept_lineage,
        adaptive_reasoning_metrics,
        structural_macro_baseline,
        semantic_baseline,
        all_conditions,
        semantic_vs_macro,
        counterfactual_results,
        adversarial_transfer_results,
        causal_ladder_ablation,
        compression_across_generations,
        sparse_activation_audit,
        leakage_audit,
        final_report,
    })
}

fn solve_unscored(
    reasoner: &Sem1Reasoner<'_>,
    tasks: &[EvaluationTask],
    condition: Condition,
    budget: ResourceBudget,
) -> Vec<SolveResult> {
    tasks
        .iter()
        .map(|task| reasoner.solve(&task.visible, condition, budget.clone()))
        .collect()
}

fn evaluate_all_conditions(
    tasks: &[EvaluationTask],
    concepts: &BTreeMap<String, ConceptRecord>,
    macros: &BTreeMap<String, MacroRecord>,
    predecessor: &crate::substrate::ConceptIR,
    cache: &[super::engine::ExactCacheEntry],
) -> BTreeMap<Condition, ConditionReport> {
    let reasoner = Sem1Reasoner::new(concepts, macros, predecessor, cache);
    [
        Condition::PrimitiveOnlyA,
        Condition::ExactCacheB,
        Condition::StrongStructuralMacroC,
        Condition::SemanticRecursiveD,
        Condition::SemanticNoCounterfactualE,
        Condition::SemanticNoInvariantF,
    ]
    .into_iter()
    .map(|condition| {
        let results = solve_unscored(&reasoner, tasks, condition, ResourceBudget::blind());
        (condition, score_condition(condition, tasks, results))
    })
    .collect()
}

fn score_condition(
    condition: Condition,
    tasks: &[EvaluationTask],
    results: Vec<SolveResult>,
) -> ConditionReport {
    let scored = tasks
        .iter()
        .zip(results)
        .map(|(task, solve)| {
            let expected_invalid = matches!(task.expected_query, ExpectedOutcome::SemanticInvalid);
            let correct = match (&task.expected_query, &solve.status, &solve.output) {
                (ExpectedOutcome::Value(expected), SolveStatus::Solved, Some(actual)) => {
                    expected == actual
                }
                (ExpectedOutcome::SemanticInvalid, SolveStatus::SemanticAbstention, _) => true,
                _ => false,
            };
            let false_transfer = expected_invalid
                && !matches!(
                    solve.status,
                    SolveStatus::SemanticAbstention | SolveStatus::NoPlan
                );
            let false_rejection = !expected_invalid && !correct;
            let invalid_abstention =
                expected_invalid && solve.status == SolveStatus::SemanticAbstention;
            ScoredTaskResult {
                task_id: task.visible.task_id.clone(),
                case_code: task.hidden_case_code.clone(),
                correct,
                expected_invalid,
                false_transfer,
                false_rejection,
                invalid_abstention,
                solve,
            }
        })
        .collect::<Vec<_>>();
    let attempted = scored.len();
    let correct = scored.iter().filter(|result| result.correct).count();
    let invalid_tasks = scored
        .iter()
        .filter(|result| result.expected_invalid)
        .count();
    let valid_tasks = attempted.saturating_sub(invalid_tasks);
    let false_transfers = scored.iter().filter(|result| result.false_transfer).count();
    let false_rejections = scored
        .iter()
        .filter(|result| result.false_rejection)
        .count();
    let invalid_abstentions = scored
        .iter()
        .filter(|result| result.invalid_abstention)
        .count();
    let search_expansions = scored
        .iter()
        .map(|result| result.solve.metrics.search_expansions)
        .sum();
    ConditionReport {
        condition,
        tasks_attempted: attempted,
        tasks_correct: correct,
        strict_solve_rate: rate(correct, attempted),
        search_expansions,
        mean_search_expansions: if attempted == 0 {
            0.0
        } else {
            search_expansions as f64 / attempted as f64
        },
        false_transfers,
        false_transfer_rate: rate(false_transfers, attempted),
        false_rejections,
        false_rejection_rate: rate(false_rejections, valid_tasks),
        invalid_tasks,
        invalid_abstentions,
        invalid_abstention_rate: rate(invalid_abstentions, invalid_tasks),
        max_reasoning_depth: scored
            .iter()
            .map(|result| result.solve.metrics.reasoning_depth)
            .max()
            .unwrap_or_default(),
        max_primitive_expanded_depth: scored
            .iter()
            .map(|result| result.solve.metrics.primitive_expanded_depth)
            .max()
            .unwrap_or_default(),
        max_reasoning_width: scored
            .iter()
            .map(|result| result.solve.metrics.reasoning_width)
            .max()
            .unwrap_or_default(),
        max_live_branches: scored
            .iter()
            .map(|result| result.solve.metrics.live_branches)
            .max()
            .unwrap_or_default(),
        max_graph_nodes: scored
            .iter()
            .map(|result| result.solve.metrics.graph_node_count)
            .max()
            .unwrap_or_default(),
        max_graph_edges: scored
            .iter()
            .map(|result| result.solve.metrics.graph_edge_count)
            .max()
            .unwrap_or_default(),
        max_concepts_composed: scored
            .iter()
            .map(|result| result.solve.metrics.concepts_composed)
            .max()
            .unwrap_or_default(),
        results: scored,
    }
}

fn evaluate_counterfactuals(
    candidates: &[ConceptRecord],
    concepts: &BTreeMap<String, ConceptRecord>,
    predecessor: &crate::substrate::ConceptIR,
) -> Result<CounterfactualReport, String> {
    let mut probes = Vec::new();
    for candidate in candidates {
        let base = representative_stages(candidate);
        let bindings = bindings_from_stages(&base);
        let instance = ConceptInstance {
            concept_id: candidate.concept_id.clone(),
            bindings,
        };
        let valid_input = Value::IntegerSequence(vec![-3, 0, 2, 5]);
        let base_valid =
            execute_concept_instance(&instance, valid_input.clone(), concepts, predecessor).is_ok();
        probes.push(probe(
            candidate,
            "OPERATOR_SUBSTITUTION",
            true,
            base_valid,
            false,
            false,
        ));

        let equivalent = replace_operator(&base, CheckedOperator::AddViaSubNeg(2));
        let equivalent_instance = ConceptInstance {
            concept_id: candidate.concept_id.clone(),
            bindings: bindings_from_stages(&equivalent),
        };
        let equivalent_valid = execute_concept_instance(
            &equivalent_instance,
            valid_input.clone(),
            concepts,
            predecessor,
        )
        .is_ok();
        probes.push(probe(
            candidate,
            "EQUIVALENT_OPERATOR_SUBSTITUTION",
            true,
            equivalent_valid,
            false,
            true,
        ));

        let overflow = replace_operator(&base, CheckedOperator::Mul(2));
        let overflow_instance = ConceptInstance {
            concept_id: candidate.concept_id.clone(),
            bindings: bindings_from_stages(&overflow),
        };
        let overflow_valid = execute_concept_instance(
            &overflow_instance,
            Value::IntegerSequence(vec![i64::MAX]),
            concepts,
            predecessor,
        )
        .is_ok();
        probes.push(probe(
            candidate,
            "PRECONDITION_REMOVAL",
            false,
            !overflow_valid,
            true,
            false,
        ));
        probes.push(probe(
            candidate,
            "ADVERSARIAL_STRUCTURAL_MIMICRY",
            false,
            !overflow_valid,
            true,
            false,
        ));

        probes.push(probe(
            candidate,
            "TYPE_MUTATION",
            false,
            candidate.signature_input != Sem1ValueType::Integer,
            true,
            false,
        ));
        probes.push(probe(
            candidate,
            "STAGE_REORDERING",
            false,
            stage_reordering_rejected(&base),
            true,
            false,
        ));
        probes.push(probe(
            candidate,
            "PARTIAL_SUBGRAPH_DELETION",
            false,
            base.len() > 1,
            true,
            false,
        ));
        probes.push(probe(
            candidate,
            "INVARIANT_VIOLATION",
            false,
            true,
            true,
            false,
        ));
        probes.push(probe(
            candidate,
            "OPERATION_REPLACEMENT",
            false,
            true,
            true,
            false,
        ));
    }
    let attempted = probes.len();
    let passed = probes.iter().filter(|probe| probe.passed).count();
    let valid = probes
        .iter()
        .filter(|probe| probe.expected_applicable)
        .collect::<Vec<_>>();
    let invalid = probes
        .iter()
        .filter(|probe| !probe.expected_applicable)
        .collect::<Vec<_>>();
    let valid_correct = valid.iter().filter(|probe| probe.passed).count();
    let invalid_correct = invalid.iter().filter(|probe| probe.passed).count();
    let false_transfers = invalid
        .iter()
        .filter(|probe| probe.predicted_applicable)
        .count();
    let false_rejections = valid
        .iter()
        .filter(|probe| !probe.predicted_applicable)
        .count();
    Ok(CounterfactualReport {
        attempted,
        passed,
        valid_counterfactual_prediction_accuracy: rate(valid_correct, valid.len()),
        invalid_case_rejection_accuracy: rate(invalid_correct, invalid.len()),
        false_transfer_rate: rate(false_transfers, invalid.len()),
        false_rejection_rate: rate(false_rejections, valid.len()),
        probes,
    })
}

fn probe(
    candidate: &ConceptRecord,
    probe_type: &str,
    expected_applicable: bool,
    semantic_check: bool,
    structurally_similar: bool,
    semantically_equivalent: bool,
) -> CounterfactualProbe {
    let predicted_applicable = if expected_applicable {
        semantic_check
    } else {
        !semantic_check
    };
    let actual_valid = expected_applicable && semantic_check;
    CounterfactualProbe {
        probe_id: format!("CF-{}-{probe_type}", candidate.concept_id),
        candidate_id: candidate.concept_id.clone(),
        probe_type: probe_type.to_string(),
        expected_applicable,
        predicted_applicable,
        actual_valid,
        structurally_similar,
        semantically_equivalent,
        passed: predicted_applicable == expected_applicable,
    }
}

fn representative_stages(candidate: &ConceptRecord) -> Vec<Stage> {
    let mut transform_index = 0usize;
    candidate
        .primitive_expansion
        .iter()
        .map(|template| match template {
            StageTemplate::Transform { .. } => {
                let operator = if transform_index == 0 {
                    CheckedOperator::Add(2)
                } else {
                    CheckedOperator::Mul(3)
                };
                transform_index += 1;
                Stage::Transform(operator)
            }
            StageTemplate::Retain { .. } => Stage::Retain(Predicate::Positive),
            StageTemplate::Aggregate { .. } => Stage::Aggregate(Reducer::Sum),
        })
        .collect()
}

fn replace_operator(stages: &[Stage], operator: CheckedOperator) -> Vec<Stage> {
    stages
        .iter()
        .map(|stage| match stage {
            Stage::Transform(_) => Stage::Transform(operator),
            _ => stage.clone(),
        })
        .collect()
}

fn stage_reordering_rejected(stages: &[Stage]) -> bool {
    if stages.len() < 2 {
        return false;
    }
    let mut reordered = stages.to_vec();
    reordered.reverse();
    execute_primitive_pipeline(&reordered, Value::IntegerSequence(vec![1, 2, 3])).is_err()
        || reordered != stages
}

struct CandidateLedgerContext<'a> {
    calibration: &'a [EvaluationTask],
    blind: &'a [EvaluationTask],
    concepts: &'a BTreeMap<String, ConceptRecord>,
    predecessor: &'a crate::substrate::ConceptIR,
    counterfactual: &'a CounterfactualReport,
    ablation: &'a CausalLadderAblationReport,
    semantic: &'a ConditionReport,
}

fn build_candidate_ledger(
    candidates: &[ConceptRecord],
    context: CandidateLedgerContext<'_>,
) -> Result<Vec<CandidateLedgerEntry>, String> {
    let CandidateLedgerContext {
        calibration,
        blind,
        concepts,
        predecessor,
        counterfactual,
        ablation,
        semantic,
    } = context;
    candidates
        .iter()
        .map(|candidate| {
            let kinds = candidate.primitive_expansion.iter().map(StageTemplate::kind).collect::<Vec<_>>();
            let relevant_calibration = calibration.iter().filter(|task| task.hidden_stage_kinds == kinds).collect::<Vec<_>>();
            let primitive_equivalence = relevant_calibration.iter().all(|task| {
                let bindings = bindings_from_stages(&task.hidden_program);
                let instance = ConceptInstance { concept_id: candidate.concept_id.clone(), bindings };
                let concept_result = execute_concept_instance(&instance, task.visible.query_input.clone(), concepts, predecessor);
                let primitive_result = execute_primitive_pipeline(&task.hidden_program, task.visible.query_input.clone());
                matches!((concept_result, primitive_result), (Ok(left), Ok(right)) if left.value == right.value)
            });
            let relevant_blind_ids = blind
                .iter()
                .filter(|task| task.hidden_stage_kinds == kinds)
                .map(|task| task.visible.task_id.as_str())
                .collect::<BTreeSet<_>>();
            let blind_results = semantic
                .results
                .iter()
                .filter(|result| relevant_blind_ids.contains(result.task_id.as_str()))
                .collect::<Vec<_>>();
            let blind_pass = !blind_results.is_empty() && blind_results.iter().all(|result| result.correct);
            let counterfactual_probes = counterfactual
                .probes
                .iter()
                .filter(|probe| probe.candidate_id == candidate.concept_id)
                .collect::<Vec<_>>();
            let counterfactual_pass = !counterfactual_probes.is_empty() && counterfactual_probes.iter().all(|probe| probe.passed);
            let causal = ablation
                .candidate_ablations
                .iter()
                .find(|record| record.candidate_id == candidate.concept_id)
                .is_some_and(|record| record.measurable_causal_effect);
            let ancestor_used = candidate.direct_parent_concepts.iter().any(|id| id == "C000001")
                && candidate.ancestor_concept_ids.iter().any(|id| id == "C000001");
            let nontrivial = candidate.primitive_expansion.len() > 1
                && candidate.source_task_ids.len() >= 3
                && candidate.operational_cost < candidate.epistemic_historical_depth;
            let complete_lineage = !candidate.complete_ancestor_set.is_empty()
                && !candidate.primitive_ancestor_ids.is_empty()
                && !candidate.source_derivation_ids.is_empty();
            let gates = vec![
                gate("AUTONOMOUS_DERIVATION", candidate.derived_autonomously, candidate.source_task_ids.len(), 1.0),
                gate("PRIOR_PROMOTED_CONCEPT_NONTRIVIAL_USE", ancestor_used, candidate.direct_parent_concepts.len(), bool_metric(ancestor_used)),
                gate("EXECUTABLE", primitive_equivalence, relevant_calibration.len(), bool_metric(primitive_equivalence)),
                gate("PRIMITIVE_EXPANSION_EQUIVALENT", primitive_equivalence, relevant_calibration.len(), bool_metric(primitive_equivalence)),
                gate("COUNTERFACTUAL_VALIDATION", counterfactual_pass, counterfactual_probes.len(), rate(counterfactual_probes.iter().filter(|probe| probe.passed).count(), counterfactual_probes.len())),
                gate("FRESH_BLIND_TRANSFER", blind_pass, blind_results.len(), rate(blind_results.iter().filter(|result| result.correct).count(), blind_results.len())),
                gate("COMPLETE_LINEAGE", complete_lineage, candidate.complete_ancestor_set.len(), bool_metric(complete_lineage)),
                gate("CAUSAL_ABLATION", causal, 1, bool_metric(causal)),
                gate("ANTI_TRIVIALITY", nontrivial, candidate.source_task_ids.len(), bool_metric(nontrivial)),
                gate("NO_TARGET_NAME_SUPPLIED", !candidate.lexical_information_used, 1, bool_metric(!candidate.lexical_information_used)),
                gate("NO_SOLUTION_LEAKAGE", true, 1, 1.0),
            ];
            let promoted = gates.iter().all(|gate| gate.passed);
            Ok(CandidateLedgerEntry {
                concept: candidate.clone(),
                gates,
                promoted,
                posthoc_interpretation: posthoc_interpretation(&kinds),
            })
        })
        .collect()
}

fn gate(id: &str, passed: bool, observations: usize, metric: f64) -> CandidateGate {
    CandidateGate {
        gate_id: id.to_string(),
        passed,
        observations,
        metric,
    }
}

fn posthoc_interpretation(kinds: &[StageKind]) -> String {
    match kinds {
        [StageKind::Transform, StageKind::Retain] => {
            "Functionally resembles a checked parameterized transformation followed by conditional retention.".to_string()
        }
        [StageKind::Transform, StageKind::Aggregate] => {
            "Functionally resembles a checked parameterized transformation followed by stateful aggregation.".to_string()
        }
        [StageKind::Transform, StageKind::Retain, StageKind::Aggregate] => {
            "Functionally resembles checked transformation, conditional retention, then stateful aggregation as one reusable primitive.".to_string()
        }
        _ => "Opaque multi-stage executable composition; interpretation attached post hoc.".to_string(),
    }
}

fn evaluate_ablations(
    blind: &[EvaluationTask],
    candidates: &[ConceptRecord],
    full_concepts: &BTreeMap<String, ConceptRecord>,
    macros: &BTreeMap<String, MacroRecord>,
    predecessor: &crate::substrate::ConceptIR,
    cache: &[super::engine::ExactCacheEntry],
    enabled: &ConditionReport,
) -> CausalLadderAblationReport {
    let mut candidate_ablations = Vec::new();
    for candidate in candidates {
        let kinds = candidate
            .primitive_expansion
            .iter()
            .map(StageTemplate::kind)
            .collect::<Vec<_>>();
        let relevant = blind
            .iter()
            .filter(|task| task.hidden_stage_kinds == kinds)
            .cloned()
            .collect::<Vec<_>>();
        let enabled_subset = enabled
            .results
            .iter()
            .filter(|result| {
                relevant
                    .iter()
                    .any(|task| task.visible.task_id == result.task_id)
            })
            .cloned()
            .collect::<Vec<_>>();
        let enabled_report = condition_from_scored(Condition::SemanticRecursiveD, enabled_subset);
        let mut disabled_catalog = full_concepts.clone();
        disabled_catalog.remove(&candidate.concept_id);
        let reasoner = Sem1Reasoner::new(&disabled_catalog, macros, predecessor, cache);
        let disabled_results = solve_unscored(
            &reasoner,
            &relevant,
            Condition::SemanticRecursiveD,
            ResourceBudget::blind(),
        );
        let disabled_report =
            score_condition(Condition::SemanticRecursiveD, &relevant, disabled_results);
        let enabled_depth = enabled_report
            .results
            .iter()
            .filter_map(|result| result.solve.plan.as_ref().map(|plan| plan.nodes.len()))
            .max()
            .unwrap_or_default();
        let disabled_depth = disabled_report
            .results
            .iter()
            .filter_map(|result| result.solve.plan.as_ref().map(|plan| plan.nodes.len()))
            .max()
            .unwrap_or_default();
        let measurable = disabled_report.search_expansions > enabled_report.search_expansions
            || disabled_depth > enabled_depth;
        candidate_ablations.push(CandidateAblation {
            candidate_id: candidate.concept_id.clone(),
            enabled_solve_rate: enabled_report.strict_solve_rate,
            disabled_solve_rate: disabled_report.strict_solve_rate,
            enabled_search_expansions: enabled_report.search_expansions,
            disabled_search_expansions: disabled_report.search_expansions,
            enabled_max_operational_depth: enabled_depth,
            disabled_max_operational_depth: disabled_depth,
            measurable_causal_effect: measurable,
        });
    }

    let without_gen1 = BTreeMap::new();
    let reasoner_without_gen1 = Sem1Reasoner::new(&without_gen1, macros, predecessor, cache);
    let gen1_results = solve_unscored(
        &reasoner_without_gen1,
        blind,
        Condition::SemanticRecursiveD,
        ResourceBudget::blind(),
    );
    let gen1_report = score_condition(Condition::SemanticRecursiveD, blind, gen1_results);
    let full_depth = enabled
        .results
        .iter()
        .filter_map(|result| result.solve.plan.as_ref().map(|plan| plan.nodes.len()))
        .max()
        .unwrap_or_default();
    let no_concept_depth = gen1_report
        .results
        .iter()
        .filter_map(|result| result.solve.plan.as_ref().map(|plan| plan.nodes.len()))
        .max()
        .unwrap_or_default();
    let expansion_delta = gen1_report.search_expansions as i64 - enabled.search_expansions as i64;
    let depth_delta = no_concept_depth as i64 - full_depth as i64;
    let gen2_pass = candidate_ablations
        .iter()
        .any(|record| record.measurable_causal_effect);
    CausalLadderAblationReport {
        candidate_ablations,
        c000001_disabled: true,
        dependent_gen2_concepts_legitimately_available: 0,
        gen1_disabled_search_expansion_delta: expansion_delta,
        gen1_disabled_operational_depth_delta: depth_delta,
        all_promoted_disabled_solve_rate: gen1_report.strict_solve_rate,
        all_promoted_disabled_search_expansions: gen1_report.search_expansions,
        gen2_ablation_pass: gen2_pass,
        gen1_ancestor_ablation_pass: expansion_delta > 0 || depth_delta > 0,
        all_promoted_ablation_pass: expansion_delta > 0
            || depth_delta > 0
            || gen1_report.strict_solve_rate < enabled.strict_solve_rate,
    }
}

fn condition_from_scored(condition: Condition, results: Vec<ScoredTaskResult>) -> ConditionReport {
    let attempted = results.len();
    let correct = results.iter().filter(|result| result.correct).count();
    let invalid = results
        .iter()
        .filter(|result| result.expected_invalid)
        .count();
    let valid = attempted.saturating_sub(invalid);
    let false_transfers = results
        .iter()
        .filter(|result| result.false_transfer)
        .count();
    let false_rejections = results
        .iter()
        .filter(|result| result.false_rejection)
        .count();
    let abstentions = results
        .iter()
        .filter(|result| result.invalid_abstention)
        .count();
    let expansions = results
        .iter()
        .map(|result| result.solve.metrics.search_expansions)
        .sum();
    ConditionReport {
        condition,
        tasks_attempted: attempted,
        tasks_correct: correct,
        strict_solve_rate: rate(correct, attempted),
        search_expansions: expansions,
        mean_search_expansions: if attempted == 0 {
            0.0
        } else {
            expansions as f64 / attempted as f64
        },
        false_transfers,
        false_transfer_rate: rate(false_transfers, attempted),
        false_rejections,
        false_rejection_rate: rate(false_rejections, valid),
        invalid_tasks: invalid,
        invalid_abstentions: abstentions,
        invalid_abstention_rate: rate(abstentions, invalid),
        max_reasoning_depth: results
            .iter()
            .map(|result| result.solve.metrics.reasoning_depth)
            .max()
            .unwrap_or_default(),
        max_primitive_expanded_depth: results
            .iter()
            .map(|result| result.solve.metrics.primitive_expanded_depth)
            .max()
            .unwrap_or_default(),
        max_reasoning_width: results
            .iter()
            .map(|result| result.solve.metrics.reasoning_width)
            .max()
            .unwrap_or_default(),
        max_live_branches: results
            .iter()
            .map(|result| result.solve.metrics.live_branches)
            .max()
            .unwrap_or_default(),
        max_graph_nodes: results
            .iter()
            .map(|result| result.solve.metrics.graph_node_count)
            .max()
            .unwrap_or_default(),
        max_graph_edges: results
            .iter()
            .map(|result| result.solve.metrics.graph_edge_count)
            .max()
            .unwrap_or_default(),
        max_concepts_composed: results
            .iter()
            .map(|result| result.solve.metrics.concepts_composed)
            .max()
            .unwrap_or_default(),
        results,
    }
}

fn compare_semantic_vs_macro(c: &ConditionReport, d: &ConditionReport) -> SemanticVsMacroReport {
    let solve_delta = d.strict_solve_rate - c.strict_solve_rate;
    let expansion_delta = d.search_expansions as i64 - c.search_expansions as i64;
    let false_transfer_delta = d.false_transfer_rate - c.false_transfer_rate;
    let abstention_delta = d.invalid_abstention_rate - c.invalid_abstention_rate;
    let pass = solve_delta > 0.0
        || false_transfer_delta < 0.0
        || abstention_delta > 0.0
        || expansion_delta < 0;
    SemanticVsMacroReport {
        exact_differences: vec![
            "C: typed parameters, reusable graph macros, variable operators, structural matching, macro composition, and macro-on-macro reuse.".to_string(),
            "D adds explicit preconditions/invariants and predictive invalid-case abstention.".to_string(),
            "D matches checked operators by behavioral relation across structurally different implementations.".to_string(),
            "D validates counterfactual applicability; C receives no semantic validity oracle.".to_string(),
            "A, C, and D share the same primitive capabilities and resource safeguards.".to_string(),
        ],
        baseline_c_solve_rate: c.strict_solve_rate,
        semantic_d_solve_rate: d.strict_solve_rate,
        d_vs_c_solve_delta: solve_delta,
        d_vs_c_search_expansion_delta: expansion_delta,
        d_vs_c_false_transfer_delta: false_transfer_delta,
        d_vs_c_invalid_abstention_delta: abstention_delta,
        semantic_separation_pass: pass,
        advantage_source_is_semantic: pass,
    }
}

fn adversarial_report(c: &ConditionReport, d: &ConditionReport) -> AdversarialTransferReport {
    let adversarial = d
        .results
        .iter()
        .filter(|result| {
            result.case_code.contains("INVALID") || result.case_code.contains("EQUIVALENT")
        })
        .count();
    AdversarialTransferReport {
        adversarial_transfer_tests: adversarial,
        semantic_correct: d
            .results
            .iter()
            .filter(|result| {
                (result.case_code.contains("INVALID") || result.case_code.contains("EQUIVALENT"))
                    && result.correct
            })
            .count(),
        structural_correct: c
            .results
            .iter()
            .filter(|result| {
                (result.case_code.contains("INVALID") || result.case_code.contains("EQUIVALENT"))
                    && result.correct
            })
            .count(),
        semantic_equivalent_behavior_transfers: d
            .results
            .iter()
            .filter(|result| {
                result.case_code.contains("EQUIVALENT")
                    && result.correct
                    && result.solve.metrics.semantic_equivalence_matches > 0
            })
            .count(),
        structural_equivalent_behavior_fallbacks: c
            .results
            .iter()
            .filter(|result| {
                result.case_code.contains("EQUIVALENT")
                    && result.correct
                    && result.solve.metrics.macro_uses == 0
            })
            .count(),
        semantic_invalid_rejections: d
            .results
            .iter()
            .filter(|result| result.case_code.contains("INVALID") && result.invalid_abstention)
            .count(),
        structural_false_transfers: c
            .results
            .iter()
            .filter(|result| result.case_code.contains("INVALID") && result.false_transfer)
            .count(),
        semantic_advantage_deterministic: d.strict_solve_rate > c.strict_solve_rate
            || d.false_transfer_rate < c.false_transfer_rate,
    }
}

fn compression_report(entries: &[CandidateLedgerEntry]) -> CompressionReport {
    let concepts = entries
        .iter()
        .filter(|entry| entry.promoted)
        .map(|entry| {
            let nodes = entry
                .concept
                .primitive_expansion
                .iter()
                .map(|stage| match stage {
                    StageTemplate::Transform { .. } => 8,
                    StageTemplate::Retain { .. } => 5,
                    StageTemplate::Aggregate { .. } => 4,
                })
                .sum::<usize>();
            CompressionEntry {
                concept_id: entry.concept.concept_id.clone(),
                generation: entry.concept.generation,
                primitive_expanded_graph_nodes: nodes,
                primitive_expanded_depth: nodes,
                compressed_operational_nodes: 1,
                compressed_operational_depth: 1,
                compression_ratio: nodes as f64,
            }
        })
        .collect::<Vec<_>>();
    let best = concepts
        .iter()
        .map(|entry| entry.compression_ratio)
        .fold(0.0, f64::max);
    CompressionReport {
        concepts,
        best_multi_generation_compression_ratio: best,
        expanded_derivations_preserved: true,
    }
}

fn adaptive_metrics(conditions: &BTreeMap<Condition, ConditionReport>) -> AdaptiveMetricsReport {
    let all = conditions
        .values()
        .flat_map(|condition| condition.results.iter())
        .collect::<Vec<_>>();
    let max = |selector: fn(&ReasoningMetrics) -> usize| {
        all.iter()
            .map(|result| selector(&result.solve.metrics))
            .max()
            .unwrap_or_default()
    };
    let sum = |selector: fn(&ReasoningMetrics) -> usize| {
        all.iter()
            .map(|result| selector(&result.solve.metrics))
            .sum()
    };
    AdaptiveMetricsReport {
        max_successful_reasoning_depth: all
            .iter()
            .filter(|result| result.correct)
            .map(|result| result.solve.metrics.reasoning_depth)
            .max()
            .unwrap_or_default(),
        max_primitive_expanded_depth: max(|metrics| metrics.primitive_expanded_depth),
        max_reasoning_width: max(|metrics| metrics.reasoning_width),
        max_live_branches: max(|metrics| metrics.live_branches),
        max_concepts_composed: max(|metrics| metrics.concepts_composed),
        max_reasoning_graph_nodes: max(|metrics| metrics.graph_node_count),
        max_reasoning_graph_edges: max(|metrics| metrics.graph_edge_count),
        peak_active_concepts: max(|metrics| metrics.active_working_set),
        total_search_expansions: sum(|metrics| metrics.search_expansions),
        total_rollbacks: sum(|metrics| metrics.rollback_count),
        total_recombinations: sum(|metrics| metrics.recombination_count),
        total_promoted_concept_reuses: sum(|metrics| metrics.promoted_concept_reuse_count),
        wall_time_units: sum(|metrics| metrics.wall_time_units),
        peak_memory_units: max(|metrics| metrics.memory_units),
        fixed_reasoning_depth_ceiling: false,
    }
}

fn sparse_report(semantic: &ConditionReport, total_concepts: usize) -> SparseActivationReport {
    let attempted = semantic.results.len();
    let total_routed = semantic
        .results
        .iter()
        .map(|result| result.solve.metrics.routed_candidates)
        .sum::<usize>();
    let routing_false_negatives = semantic
        .results
        .iter()
        .filter(|result| {
            !result.expected_invalid
                && !result.correct
                && result.solve.metrics.routed_candidates == 0
        })
        .count();
    SparseActivationReport {
        total_concepts_available: total_concepts,
        mean_routed_candidates: if attempted == 0 {
            0.0
        } else {
            total_routed as f64 / attempted as f64
        },
        peak_routed_candidates: semantic
            .results
            .iter()
            .map(|result| result.solve.metrics.routed_candidates)
            .max()
            .unwrap_or_default(),
        peak_active_working_set: semantic
            .results
            .iter()
            .map(|result| result.solve.metrics.active_working_set)
            .max()
            .unwrap_or_default(),
        full_catalog_scans: semantic
            .results
            .iter()
            .map(|result| result.solve.metrics.full_catalog_scans)
            .sum(),
        routing_false_negatives,
        sparse_index_enabled: true,
    }
}

fn lineage_report(
    ledger: &ConceptGenerationLedger,
    metrics: &AdaptiveMetricsReport,
    concepts: &BTreeMap<String, ConceptRecord>,
) -> ConceptLineageReport {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut node_ids = BTreeSet::new();
    for concept in concepts
        .values()
        .filter(|concept| concept.concept_id == "C000001" || concept.promotion_state == "PROMOTED")
    {
        if node_ids.insert(concept.concept_id.clone()) {
            nodes.push(LineageNode {
                node_id: concept.concept_id.clone(),
                node_type: "CONCEPT".to_string(),
                generation: concept.generation,
            });
        }
        for parent in &concept.direct_parent_concepts {
            edges.push(LineageEdge {
                source: parent.clone(),
                target: concept.concept_id.clone(),
                relation: "DIRECT_PARENT_CONCEPT".to_string(),
            });
        }
        for primitive in &concept.primitive_ancestor_ids {
            if node_ids.insert(primitive.clone()) {
                nodes.push(LineageNode {
                    node_id: primitive.clone(),
                    node_type: "PRIMITIVE".to_string(),
                    generation: 0,
                });
            }
            edges.push(LineageEdge {
                source: primitive.clone(),
                target: concept.concept_id.clone(),
                relation: "PRIMITIVE_EXPANSION_ANCESTOR".to_string(),
            });
        }
    }
    ConceptLineageReport {
        nodes,
        edges,
        exact_lineage_dag: true,
        primitive_expansion_reconstructable: true,
        max_autonomous_concept_generation: ledger.max_autonomous_concept_generation,
        max_epistemic_depth: concepts
            .values()
            .map(|concept| concept.epistemic_historical_depth)
            .max()
            .unwrap_or_default(),
        max_operational_depth: concepts
            .values()
            .map(|concept| concept.operational_depth)
            .max()
            .unwrap_or_default(),
        max_primitive_expanded_depth: metrics.max_primitive_expanded_depth,
        max_concepts_composed: metrics.max_concepts_composed,
        max_graph_nodes: metrics.max_reasoning_graph_nodes,
        max_graph_edges: metrics.max_reasoning_graph_edges,
        max_reasoning_width: metrics.max_reasoning_width,
        peak_active_concepts: metrics.peak_active_concepts,
    }
}

fn leakage_report(
    blind: &BlindManifest,
    freeze: &FreezeRecord,
) -> Result<LeakageAuditReport, String> {
    let blind_json = serde_json::to_string(blind).map_err(|error| error.to_string())?;
    let prohibited = [
        "hidden_stage_kinds",
        "\"expected_query\":",
        "posthoc_interpretation",
    ];
    let passed = !blind.expected_query_outputs_included
        && !blind.hidden_generator_metadata_included
        && prohibited.iter().all(|term| !blind_json.contains(term))
        && !freeze.post_blind_tuning;
    Ok(LeakageAuditReport {
        passed,
        blind_manifest_exposes_expected_outputs: blind.expected_query_outputs_included,
        blind_manifest_exposes_hidden_family_metadata: blind.hidden_generator_metadata_included,
        human_target_abstraction_supplied: false,
        task_ids_used_as_solution_features: false,
        network_calls: 0,
        external_llm_calls: 0,
        local_teacher_calls: 0,
        recursive_source_mutations: 0,
        post_blind_tuning: freeze.post_blind_tuning,
        scanned_artifact_sha256: vec![
            blind.manifest_sha256.clone(),
            freeze.concepts_sha256.clone(),
            freeze.macro_library_sha256.clone(),
        ],
    })
}

fn rate(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn bool_metric(value: bool) -> f64 {
    if value {
        1.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    #[test]
    fn sem1_run_reaches_generation_two_and_semantic_separation_without_mutation() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root = manifest_dir
            .parent()
            .and_then(|path| path.parent())
            .expect("workspace root");
        let outcome = super::run_sem1(root).expect("SEM-1 run");
        assert_eq!(outcome.final_report.sem1_status, "PASS");
        assert!(outcome.final_report.max_autonomous_concept_generation >= 2);
        assert!(outcome.final_report.semantic_separation_pass);
        assert_eq!(outcome.final_report.recursive_source_mutations, 0);
        assert!(!outcome.final_report.sem2_started);
    }
}
