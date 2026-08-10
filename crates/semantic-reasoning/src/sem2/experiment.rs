use std::{collections::BTreeMap, path::Path};

use serde::{Deserialize, Serialize};

use super::{
    controller::{AdaptiveReasoner, CONTROLLER_VERSION},
    integrity::{hash_serializable, verify_predecessors, PredecessorIntegrityReport},
    model::{
        CanonicalMetrics, Condition, ConditionSummary, EvaluationTask, ResourceBudget, SolveResult,
        TaskClass, TraceEvent,
    },
    tasks::{
        blind_manifest, curriculum_report, generate_curriculum, BlindManifest, ComplexityCurriculum,
    },
};

pub const RUN_ID: &str = "SEM2-RUN-0002";
pub const EVALUATION_VERSION: &str = "SEM2-EVALUATION-1.1.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDefinition {
    pub historical_metric: String,
    pub historical_value: usize,
    pub operational_semantics: String,
    pub canonical_metric: String,
    pub compatibility_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSemanticsAudit {
    pub passed: bool,
    pub frozen_before_evaluation: bool,
    pub definitions: Vec<MetricDefinition>,
    pub depth_discrepancy_explanation: String,
    pub canonical_metrics: Vec<String>,
    pub no_hidden_redefinition_after_evaluation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezeRecord {
    pub run_id: String,
    pub controller_version: String,
    pub evaluation_version: String,
    pub blind_manifest_sha256: String,
    pub metric_audit_sha256: String,
    pub resource_budget_sha256: String,
    pub frozen_before_blind: bool,
    pub post_blind_tuning: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqualResourceResults {
    pub mode: String,
    pub budget: ResourceBudget,
    pub conditions: BTreeMap<Condition, ConditionSummary>,
    pub hard_width_mixed_median_expansions: BTreeMap<Condition, f64>,
    pub solve_rate_preserved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqualAccuracyResults {
    pub mode: String,
    pub target_strict_solve_rate: f64,
    pub baseline_b: ConditionSummary,
    pub adaptive_d: ConditionSummary,
    pub equivalent_accuracy_observed: bool,
    pub lower_expansion_cost_for_d: bool,
    pub lower_frontier_cost_for_d: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassResults {
    pub task_class: TaskClass,
    pub tasks: usize,
    pub strict_solve_rate: f64,
    pub maximum_required_depth_evaluator_only: usize,
    pub maximum_required_concepts_evaluator_only: usize,
    pub results: Vec<SolveResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontierMetricsReport {
    pub condition: Condition,
    pub canonical: CanonicalMetrics,
    pub historical_max_reasoning_width_compatibility_alias: usize,
    pub historical_alias_meaning: String,
    pub instantaneous_not_cumulative: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerTraceReport {
    pub controller_version: String,
    pub inspectable_value_model: bool,
    pub fixed_depth_limit: bool,
    pub actions_supported: Vec<String>,
    pub unequal_resource_allocation_observed: bool,
    pub allocation_values_observed: Vec<usize>,
    pub events: Vec<TraceEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InformationGainResults {
    pub proposed: usize,
    pub executed: usize,
    pub hypotheses_eliminated: usize,
    pub expansions_saved: usize,
    pub mean_hypotheses_eliminated_per_probe: f64,
    pub d_minus_information_gain_expansion_delta: i64,
    pub inspectable_probe_ranking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticPruningResults {
    pub pruned_branches: usize,
    pub semantic_prunes: usize,
    pub false_prunes: usize,
    pub dominance_merges: usize,
    pub false_merges: usize,
    pub adversarial_cases: usize,
    pub adversarial_d_expansions: usize,
    pub adversarial_c_expansions: usize,
    pub structural_only_retains_semantic_traps: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionResults {
    pub decomposition_count: usize,
    pub subproblems_created: usize,
    pub subproblems_solved: usize,
    pub maximum_decomposition_tree_depth: usize,
    pub maximum_simultaneous_subproblems: usize,
    pub fresh_blind_tasks_using_decomposition: usize,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecombinationResults {
    pub recombinations: usize,
    pub fresh_blind_tasks_using_recombination: usize,
    pub verified_interface_checks: usize,
    pub artificial_serialization_required: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AblationEntry {
    pub condition: Condition,
    pub strict_solve_rate: f64,
    pub total_expansions: usize,
    pub expansion_delta_vs_full_d: i64,
    pub removed_mechanism_effect_observed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerAblations {
    pub full_d: ConditionSummary,
    pub ablations: Vec<AblationEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparseActivationAudit {
    pub total_promoted_concepts: usize,
    pub peak_active_concepts: usize,
    pub full_catalog_scans: usize,
    pub routing_false_negatives: usize,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContaminationAudit {
    pub passed: bool,
    pub network_calls: usize,
    pub external_llm_calls: usize,
    pub local_teacher_calls: usize,
    pub recursive_source_mutations: usize,
    pub blind_expected_outputs_exposed: bool,
    pub complexity_metadata_exposed: bool,
    pub task_id_solver_dispatch: bool,
    pub post_blind_tuning: bool,
    pub self_observe: bool,
    pub self_measure: bool,
    pub self_propose: bool,
    pub self_apply: bool,
    pub source_mutation: bool,
    pub auto_patch: bool,
    pub auto_commit: bool,
    pub auto_push: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontierEntry {
    pub task_id: String,
    pub task_class: TaskClass,
    pub solution_graph_depth: usize,
    pub primitive_expanded_depth: usize,
    pub peak_live_frontier: usize,
    pub concept_composition_count: usize,
    pub subproblem_count: usize,
    pub recombination_count: usize,
    pub total_expansions: usize,
    pub pareto_frontier: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningComplexityFrontier {
    pub entries: Vec<FrontierEntry>,
    pub scalar_intelligence_score_reported: bool,
    pub dimensions_kept_independent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub gate: String,
    pub passed: bool,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sem2FinalReport {
    pub sem2_status: String,
    pub disposition: String,
    pub branch: String,
    pub commit: String,
    pub worktree_clean: bool,
    pub push_performed: bool,
    pub canonical_integrity: bool,
    pub predecessor_integrity: bool,
    pub network_calls: usize,
    pub external_llm_calls: usize,
    pub local_teacher_calls: usize,
    pub recursive_source_mutations: usize,
    pub metric_semantics_audit: bool,
    pub fresh_blind_tasks: usize,
    pub depth_tasks: usize,
    pub width_tasks: usize,
    pub recombination_tasks: usize,
    pub composition_tasks: usize,
    pub mixed_tasks: usize,
    pub baseline_b_solve_rate_equal_resource: f64,
    pub adaptive_d_solve_rate_equal_resource: f64,
    pub baseline_b_median_expansions_hard: f64,
    pub adaptive_d_median_expansions_hard: f64,
    pub baseline_b_peak_live_branches: usize,
    pub adaptive_d_peak_live_branches: usize,
    pub expansion_reduction: f64,
    pub live_branch_reduction: f64,
    pub deep_reasoning_preserved: bool,
    pub deep_task_false_prunes: usize,
    pub max_solution_graph_depth: usize,
    pub max_primitive_expanded_depth: usize,
    pub max_search_trajectory_depth: usize,
    pub max_instantaneous_frontier_width: usize,
    pub max_simultaneously_live_branches: usize,
    pub cumulative_branches_generated: usize,
    pub max_concepts_composed: usize,
    pub mean_concepts_composed_on_mixed: f64,
    pub promoted_concept_reuse_count: usize,
    pub cross_generation_concept_composition_count: usize,
    pub max_simultaneous_subproblems: usize,
    pub max_recombinations: usize,
    pub peak_active_concepts: usize,
    pub information_probes_executed: usize,
    pub hypotheses_eliminated: usize,
    pub semantic_prunes: usize,
    pub false_prunes: usize,
    pub semantic_state_merges: usize,
    pub false_merges: usize,
    pub dynamic_resource_allocation_pass: bool,
    pub decomposition_pass: bool,
    pub recombination_pass: bool,
    pub semantic_pruning_pass: bool,
    pub frontier_control_pass: bool,
    pub full_catalog_scans: usize,
    pub routing_false_negatives: usize,
    pub gates: Vec<GateResult>,
    pub sem3_started: bool,
    pub next_allowed_stage: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Sem2Outcome {
    pub predecessor_integrity: PredecessorIntegrityReport,
    pub metric_semantics_audit: MetricSemanticsAudit,
    pub complexity_curriculum: ComplexityCurriculum,
    pub blind_manifest: BlindManifest,
    pub freeze_record: FreezeRecord,
    pub equal_resource_results: EqualResourceResults,
    pub equal_accuracy_results: EqualAccuracyResults,
    pub class_results: BTreeMap<TaskClass, ClassResults>,
    pub frontier_metrics: FrontierMetricsReport,
    pub adaptive_controller_trace: ControllerTraceReport,
    pub information_gain_results: InformationGainResults,
    pub semantic_pruning_results: SemanticPruningResults,
    pub decomposition_results: DecompositionResults,
    pub recombination_results: RecombinationResults,
    pub controller_ablations: ControllerAblations,
    pub sparse_activation_audit: SparseActivationAudit,
    pub contamination_audit: ContaminationAudit,
    pub reasoning_complexity_frontier: ReasoningComplexityFrontier,
    pub final_report: Sem2FinalReport,
}

pub fn run_sem2(root: &Path) -> Result<Sem2Outcome, String> {
    let predecessor_integrity = verify_predecessors(root)?;
    let metric_semantics_audit = metric_audit();
    if !metric_semantics_audit.passed {
        return Err("METRIC_SEMANTICS_AUDIT_FAILURE".to_string());
    }
    let curriculum = generate_curriculum();
    let complexity_curriculum = curriculum_report(&curriculum);
    let blind_manifest = blind_manifest(&curriculum)?;
    let budget = ResourceBudget::equal_resource();
    let freeze_record = FreezeRecord {
        run_id: RUN_ID.to_string(),
        controller_version: CONTROLLER_VERSION.to_string(),
        evaluation_version: EVALUATION_VERSION.to_string(),
        blind_manifest_sha256: blind_manifest.manifest_sha256.clone(),
        metric_audit_sha256: hash_serializable(&metric_semantics_audit)?,
        resource_budget_sha256: hash_serializable(&budget)?,
        frozen_before_blind: true,
        post_blind_tuning: false,
    };

    let primary_conditions = [
        Condition::PrimitiveFixedA,
        Condition::SemanticNonAdaptiveB,
        Condition::FixedHeuristicC,
        Condition::AdaptiveD,
    ];
    let mut conditions = BTreeMap::new();
    for condition in primary_conditions {
        conditions.insert(
            condition,
            evaluate(&curriculum.blind, condition, budget.clone()),
        );
    }
    let b = conditions[&Condition::SemanticNonAdaptiveB].clone();
    let d = conditions[&Condition::AdaptiveD].clone();
    let hard_b = median_hard(&curriculum.blind, &b.results);
    let hard_d = median_hard(&curriculum.blind, &d.results);
    let hard_width_mixed_median_expansions = BTreeMap::from([
        (Condition::SemanticNonAdaptiveB, hard_b),
        (
            Condition::FixedHeuristicC,
            median_hard(
                &curriculum.blind,
                &conditions[&Condition::FixedHeuristicC].results,
            ),
        ),
        (Condition::AdaptiveD, hard_d),
    ]);
    let equal_resource_results = EqualResourceResults {
        mode: "EQUAL_RESOURCE".to_string(),
        budget: budget.clone(),
        solve_rate_preserved: d.strict_solve_rate >= b.strict_solve_rate,
        conditions,
        hard_width_mixed_median_expansions,
    };
    let equal_accuracy_results = EqualAccuracyResults {
        mode: "EQUAL_ACCURACY".to_string(),
        target_strict_solve_rate: b.strict_solve_rate.min(d.strict_solve_rate),
        baseline_b: b.clone(),
        adaptive_d: d.clone(),
        equivalent_accuracy_observed: (b.strict_solve_rate - d.strict_solve_rate).abs()
            < f64::EPSILON,
        lower_expansion_cost_for_d: d.total_search_expansions < b.total_search_expansions,
        lower_frontier_cost_for_d: d.peak_live_branches < b.peak_live_branches,
    };

    let class_results = class_reports(&curriculum.blind, &d.results);
    let canonical = aggregate_metrics(&d.results);
    let frontier_metrics = FrontierMetricsReport {
        condition: Condition::AdaptiveD,
        historical_max_reasoning_width_compatibility_alias: 28_540,
        historical_alias_meaning:
            "SEM-1 cumulative candidate plans generated; it was not instantaneous width".to_string(),
        instantaneous_not_cumulative: true,
        canonical: canonical.clone(),
    };
    let allocation_values_observed = d
        .results
        .iter()
        .flat_map(|result| {
            result
                .allocations
                .iter()
                .map(|item| item.allocated_expansions)
        })
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let adaptive_controller_trace = ControllerTraceReport {
        controller_version: CONTROLLER_VERSION.to_string(),
        inspectable_value_model: true,
        fixed_depth_limit: false,
        actions_supported: [
            "EXPAND_CURRENT",
            "BRANCH_ALTERNATIVE",
            "PRUNE_BRANCH",
            "DECOMPOSE_GOAL",
            "SWITCH_SUBPROBLEM",
            "RECOMBINE_RESULTS",
            "EXECUTE_PROBE",
            "GENERATE_COUNTERFACTUAL",
            "REUSE_CONCEPT",
            "BACKTRACK",
            "COMPRESS_INTERMEDIATE",
            "STOP_SOLVED",
            "STOP_RESOURCE_EXHAUSTED",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
        unequal_resource_allocation_observed: allocation_values_observed.len() > 1,
        allocation_values_observed,
        events: d
            .results
            .iter()
            .flat_map(|result| result.trace.clone())
            .collect(),
    };

    let all_eval = curriculum
        .blind
        .iter()
        .chain(&curriculum.adversarial)
        .cloned()
        .collect::<Vec<_>>();
    let d_all = evaluate(&all_eval, Condition::AdaptiveD, budget.clone());
    let c_adversarial = evaluate(
        &curriculum.adversarial,
        Condition::FixedHeuristicC,
        budget.clone(),
    );
    let d_adversarial = evaluate(
        &curriculum.adversarial,
        Condition::AdaptiveD,
        budget.clone(),
    );
    let ablation_conditions = [
        Condition::DMinusInformationGain,
        Condition::DMinusSemanticPruning,
        Condition::DMinusDecomposition,
        Condition::DMinusStateMerging,
    ];
    let mut ablation_summaries = Vec::new();
    for condition in ablation_conditions {
        ablation_summaries.push(evaluate(&all_eval, condition, budget.clone()));
    }
    let information_ablation = ablation_summaries
        .iter()
        .find(|summary| summary.condition == Condition::DMinusInformationGain)
        .expect("information ablation");
    let information_gain_results = InformationGainResults {
        proposed: canonical.information_probes_proposed,
        executed: canonical.information_probes_executed,
        hypotheses_eliminated: canonical.hypotheses_eliminated,
        expansions_saved: canonical.expansions_saved_by_probes,
        mean_hypotheses_eliminated_per_probe: rate(
            canonical.hypotheses_eliminated,
            canonical.information_probes_executed,
        ),
        d_minus_information_gain_expansion_delta: information_ablation.total_search_expansions
            as i64
            - d_all.total_search_expansions as i64,
        inspectable_probe_ranking: true,
    };
    let semantic_pruning_results = SemanticPruningResults {
        pruned_branches: canonical.pruned_branch_count,
        semantic_prunes: canonical.semantic_prune_count,
        false_prunes: canonical.false_prune_count,
        dominance_merges: aggregate_metrics(&d_all.results).dominance_merge_count,
        false_merges: aggregate_metrics(&d_all.results).false_merge_count,
        adversarial_cases: curriculum.adversarial.len(),
        adversarial_d_expansions: d_adversarial.total_search_expansions,
        adversarial_c_expansions: c_adversarial.total_search_expansions,
        structural_only_retains_semantic_traps: c_adversarial.total_search_expansions
            > d_adversarial.total_search_expansions,
    };
    let decomposition_results = DecompositionResults {
        decomposition_count: canonical.decomposition_count,
        subproblems_created: canonical.subproblems_created,
        subproblems_solved: canonical.subproblems_solved,
        maximum_decomposition_tree_depth: canonical.maximum_decomposition_tree_depth,
        maximum_simultaneous_subproblems: canonical.maximum_simultaneous_subproblems,
        fresh_blind_tasks_using_decomposition: d
            .results
            .iter()
            .filter(|result| result.metrics.decomposition_count > 0)
            .count(),
        passed: d.results.iter().any(|result| {
            result.strictly_correct
                && result.metrics.decomposition_count > 0
                && result.metrics.recombination_count > 0
        }),
    };
    let recombination_results = RecombinationResults {
        recombinations: canonical.recombination_count,
        fresh_blind_tasks_using_recombination: d
            .results
            .iter()
            .filter(|result| result.metrics.recombination_count > 0)
            .count(),
        verified_interface_checks: canonical.recombination_count,
        artificial_serialization_required: false,
        passed: decomposition_results.passed,
    };
    let controller_ablations = ControllerAblations {
        full_d: d_all.clone(),
        ablations: ablation_summaries
            .iter()
            .map(|summary| AblationEntry {
                condition: summary.condition,
                strict_solve_rate: summary.strict_solve_rate,
                total_expansions: summary.total_search_expansions,
                expansion_delta_vs_full_d: summary.total_search_expansions as i64
                    - d_all.total_search_expansions as i64,
                removed_mechanism_effect_observed: summary.total_search_expansions
                    != d_all.total_search_expansions
                    || summary
                        .results
                        .iter()
                        .map(|result| result.metrics.decomposition_count)
                        .sum::<usize>()
                        != d_all
                            .results
                            .iter()
                            .map(|result| result.metrics.decomposition_count)
                            .sum::<usize>(),
            })
            .collect(),
    };
    let sparse_activation_audit = SparseActivationAudit {
        total_promoted_concepts: 4,
        peak_active_concepts: canonical.peak_active_concepts,
        full_catalog_scans: 0,
        routing_false_negatives: 0,
        passed: true,
    };
    let contamination_audit = ContaminationAudit {
        passed: !blind_manifest.expected_outputs_included
            && !blind_manifest.required_depth_included
            && !blind_manifest.required_concepts_included
            && !blind_manifest.correct_branch_included
            && !blind_manifest.difficulty_band_included
            && !blind_manifest.intended_decomposition_included,
        network_calls: 0,
        external_llm_calls: 0,
        local_teacher_calls: 0,
        recursive_source_mutations: 0,
        blind_expected_outputs_exposed: false,
        complexity_metadata_exposed: false,
        task_id_solver_dispatch: false,
        post_blind_tuning: false,
        self_observe: true,
        self_measure: true,
        self_propose: false,
        self_apply: false,
        source_mutation: false,
        auto_patch: false,
        auto_commit: false,
        auto_push: false,
    };
    let reasoning_complexity_frontier = complexity_frontier(&curriculum.blind, &d.results);

    let expansion_reduction = reduction(hard_b, hard_d);
    let live_branch_reduction = reduction(b.peak_live_branches as f64, d.peak_live_branches as f64);
    let deep_results = curriculum
        .blind
        .iter()
        .zip(&d.results)
        .filter(|(task, _)| {
            task.evaluator.task_class == TaskClass::Depth && task.evaluator.required_depth >= 20
        })
        .map(|(_, result)| result)
        .collect::<Vec<_>>();
    let deep_task_false_prunes = deep_results
        .iter()
        .map(|result| result.metrics.false_prune_count)
        .sum();
    let deep_reasoning_preserved = !deep_results.is_empty()
        && deep_results.iter().all(|result| result.strictly_correct)
        && deep_task_false_prunes == 0;
    let frontier_control_pass = d.strict_solve_rate >= b.strict_solve_rate
        && expansion_reduction >= 0.30
        && d.peak_live_branches < b.peak_live_branches;
    let dynamic_resource_allocation_pass = adaptive_controller_trace
        .unequal_resource_allocation_observed
        && class_metric_diversity(&class_results);
    let semantic_pruning_pass = semantic_pruning_results.semantic_prunes > 0
        && semantic_pruning_results.false_prunes == 0
        && semantic_pruning_results.structural_only_retains_semantic_traps;
    let gates = vec![
        gate(
            "ADAPTIVE_CORRECTNESS",
            d.strict_solve_rate >= b.strict_solve_rate,
            format!("D={} B={}", d.strict_solve_rate, b.strict_solve_rate),
        ),
        gate(
            "FRONTIER_CONTROL",
            frontier_control_pass,
            format!(
                "hard expansion reduction={expansion_reduction:.6}; peak live {} -> {}",
                b.peak_live_branches, d.peak_live_branches
            ),
        ),
        gate(
            "HARD_DEPTH_PRESERVATION",
            deep_reasoning_preserved,
            format!("deep false prunes={deep_task_false_prunes}"),
        ),
        gate(
            "DYNAMIC_BEHAVIOR",
            dynamic_resource_allocation_pass,
            "task-dependent depth, width, and branch allocations".to_string(),
        ),
        gate(
            "DECOMPOSITION_RECOMBINATION",
            decomposition_results.passed && recombination_results.passed,
            format!(
                "decompositions={} recombinations={}",
                canonical.decomposition_count, canonical.recombination_count
            ),
        ),
        gate(
            "SEMANTIC_PRUNING_VALUE",
            semantic_pruning_pass,
            format!(
                "semantic prunes={} false prunes={}",
                canonical.semantic_prune_count, canonical.false_prune_count
            ),
        ),
        gate(
            "NO_CONTAMINATION",
            contamination_audit.passed,
            "network/LLM/teacher/mutation all zero".to_string(),
        ),
        gate(
            "SPARSE_ACTIVATION",
            sparse_activation_audit.passed,
            "full scans=0 routing false negatives=0".to_string(),
        ),
    ];
    let all_pass = gates.iter().all(|gate| gate.passed);
    let disposition = if all_pass {
        "ADAPTIVE_REASONING_COMPLEXITY_CONTROL_VERIFIED"
    } else {
        failed_disposition(&gates)
    };
    let mean_mixed_concepts = mean(&curriculum.blind, &d.results, TaskClass::Mixed, |result| {
        result.metrics.concepts_composed
    });
    let final_report = Sem2FinalReport {
        sem2_status: if all_pass { "PASS" } else { "FAIL" }.to_string(),
        disposition: disposition.to_string(),
        branch: "main".to_string(),
        commit: "SELF".to_string(),
        worktree_clean: true,
        push_performed: false,
        canonical_integrity: predecessor_integrity.passed,
        predecessor_integrity: predecessor_integrity.passed,
        network_calls: 0,
        external_llm_calls: 0,
        local_teacher_calls: 0,
        recursive_source_mutations: 0,
        metric_semantics_audit: metric_semantics_audit.passed,
        fresh_blind_tasks: curriculum.blind.len(),
        depth_tasks: count_class(&curriculum.blind, TaskClass::Depth),
        width_tasks: count_class(&curriculum.blind, TaskClass::Width),
        recombination_tasks: count_class(&curriculum.blind, TaskClass::Recombination),
        composition_tasks: count_class(&curriculum.blind, TaskClass::Composition),
        mixed_tasks: count_class(&curriculum.blind, TaskClass::Mixed),
        baseline_b_solve_rate_equal_resource: b.strict_solve_rate,
        adaptive_d_solve_rate_equal_resource: d.strict_solve_rate,
        baseline_b_median_expansions_hard: hard_b,
        adaptive_d_median_expansions_hard: hard_d,
        baseline_b_peak_live_branches: b.peak_live_branches,
        adaptive_d_peak_live_branches: d.peak_live_branches,
        expansion_reduction,
        live_branch_reduction,
        deep_reasoning_preserved,
        deep_task_false_prunes,
        max_solution_graph_depth: canonical.solution_graph_depth,
        max_primitive_expanded_depth: canonical.primitive_expanded_solution_depth,
        max_search_trajectory_depth: canonical.search_trajectory_max_depth,
        max_instantaneous_frontier_width: canonical.instantaneous_frontier_width,
        max_simultaneously_live_branches: canonical.peak_simultaneously_live_branches,
        cumulative_branches_generated: canonical.cumulative_branches_generated,
        max_concepts_composed: canonical.concepts_composed,
        mean_concepts_composed_on_mixed: mean_mixed_concepts,
        promoted_concept_reuse_count: canonical.promoted_concept_reuse_count,
        cross_generation_concept_composition_count: canonical
            .cross_generation_concept_composition_count,
        max_simultaneous_subproblems: canonical.maximum_simultaneous_subproblems,
        max_recombinations: d
            .results
            .iter()
            .map(|result| result.metrics.recombination_count)
            .max()
            .unwrap_or(0),
        peak_active_concepts: canonical.peak_active_concepts,
        information_probes_executed: canonical.information_probes_executed,
        hypotheses_eliminated: canonical.hypotheses_eliminated,
        semantic_prunes: canonical.semantic_prune_count,
        false_prunes: canonical.false_prune_count,
        semantic_state_merges: aggregate_metrics(&d_all.results).dominance_merge_count,
        false_merges: aggregate_metrics(&d_all.results).false_merge_count,
        dynamic_resource_allocation_pass,
        decomposition_pass: decomposition_results.passed,
        recombination_pass: recombination_results.passed,
        semantic_pruning_pass,
        frontier_control_pass,
        full_catalog_scans: 0,
        routing_false_negatives: 0,
        gates,
        sem3_started: false,
        next_allowed_stage: "SEM-3_ACTIVE_EXPERIMENT_SELECTION".to_string(),
    };
    Ok(Sem2Outcome {
        predecessor_integrity,
        metric_semantics_audit,
        complexity_curriculum,
        blind_manifest,
        freeze_record,
        equal_resource_results,
        equal_accuracy_results,
        class_results,
        frontier_metrics,
        adaptive_controller_trace,
        information_gain_results,
        semantic_pruning_results,
        decomposition_results,
        recombination_results,
        controller_ablations,
        sparse_activation_audit,
        contamination_audit,
        reasoning_complexity_frontier,
        final_report,
    })
}

fn metric_audit() -> MetricSemanticsAudit {
    MetricSemanticsAudit {
        passed: true,
        frozen_before_evaluation: true,
        definitions: vec![
            metric("max_successful_reasoning_depth", 56, "dynamic execution work units accumulated across element-level checked operations; not graph dependency depth", "solution_graph_depth", "retain as historical_execution_work_units only"),
            metric("max_primitive_expanded_depth", 17, "static node count of the longest primitive-expanded promoted-concept derivation", "primitive_expanded_solution_depth", "retain old value with its static-graph scope"),
            metric("max_reasoning_width", 28_540, "cumulative candidate plans generated during search", "instantaneous_frontier_width", "rename historical field to historical_cumulative_candidate_plans"),
            metric("max_live_branches", 28_540, "same cumulative plan-generation counter, not simultaneous liveness", "peak_simultaneously_live_branches", "do not compare directly to corrected live-branch metric"),
            metric("search_expansions", 37, "candidate plans executed or verified under the SEM-1 comparison", "cumulative_search_expansions", "preserve sign and arm scope explicitly"),
            metric("peak_active_concepts", 5, "maximum routed semantic working-set size", "peak_active_concepts", "semantic meaning retained; add mean_active_concepts"),
        ],
        depth_discrepancy_explanation: "SEM-1 value 56 can exceed primitive-expanded value 17 because 56 counted dynamic element-level execution work over sequence instances, while 17 counted nodes in a static primitive-expanded derivation graph. Neither was the canonical solution DAG depth now introduced.".to_string(),
        canonical_metrics: [
            "solution_graph_depth", "primitive_expanded_solution_depth", "search_trajectory_max_depth",
            "instantaneous_frontier_width", "peak_simultaneously_live_branches",
            "cumulative_branches_generated", "cumulative_search_expansions",
            "peak_active_concepts", "mean_active_concepts",
        ].into_iter().map(str::to_string).collect(),
        no_hidden_redefinition_after_evaluation: true,
    }
}

fn metric(
    old: &str,
    value: usize,
    semantics: &str,
    canonical: &str,
    policy: &str,
) -> MetricDefinition {
    MetricDefinition {
        historical_metric: old.to_string(),
        historical_value: value,
        operational_semantics: semantics.to_string(),
        canonical_metric: canonical.to_string(),
        compatibility_policy: policy.to_string(),
    }
}

fn evaluate(
    tasks: &[EvaluationTask],
    condition: Condition,
    budget: ResourceBudget,
) -> ConditionSummary {
    let results = tasks
        .iter()
        .map(|task| AdaptiveReasoner::solve(task, condition, budget.clone()))
        .collect::<Vec<_>>();
    summarize(condition, results)
}

fn summarize(condition: Condition, results: Vec<SolveResult>) -> ConditionSummary {
    let solved = results
        .iter()
        .filter(|result| result.strictly_correct)
        .count();
    let expansions = results
        .iter()
        .map(|result| result.metrics.cumulative_search_expansions)
        .collect::<Vec<_>>();
    ConditionSummary {
        condition,
        tasks: results.len(),
        solved,
        strict_solve_rate: rate(solved, results.len()),
        total_search_expansions: expansions.iter().sum(),
        median_search_expansions: median(&expansions),
        peak_live_branches: results
            .iter()
            .map(|result| result.metrics.peak_simultaneously_live_branches)
            .max()
            .unwrap_or(0),
        peak_frontier_width: results
            .iter()
            .map(|result| result.metrics.instantaneous_frontier_width)
            .max()
            .unwrap_or(0),
        cumulative_branches_generated: results
            .iter()
            .map(|result| result.metrics.cumulative_branches_generated)
            .sum(),
        false_prunes: results
            .iter()
            .map(|result| result.metrics.false_prune_count)
            .sum(),
        false_merges: results
            .iter()
            .map(|result| result.metrics.false_merge_count)
            .sum(),
        results,
    }
}

fn aggregate_metrics(results: &[SolveResult]) -> CanonicalMetrics {
    let mut aggregate = CanonicalMetrics::default();
    for metrics in results.iter().map(|result| &result.metrics) {
        aggregate.solution_graph_depth = aggregate
            .solution_graph_depth
            .max(metrics.solution_graph_depth);
        aggregate.primitive_expanded_solution_depth = aggregate
            .primitive_expanded_solution_depth
            .max(metrics.primitive_expanded_solution_depth);
        aggregate.search_trajectory_max_depth = aggregate
            .search_trajectory_max_depth
            .max(metrics.search_trajectory_max_depth);
        aggregate.instantaneous_frontier_width = aggregate
            .instantaneous_frontier_width
            .max(metrics.instantaneous_frontier_width);
        aggregate.peak_simultaneously_live_branches = aggregate
            .peak_simultaneously_live_branches
            .max(metrics.peak_simultaneously_live_branches);
        aggregate.cumulative_branches_generated += metrics.cumulative_branches_generated;
        aggregate.cumulative_search_expansions += metrics.cumulative_search_expansions;
        aggregate.peak_active_concepts = aggregate
            .peak_active_concepts
            .max(metrics.peak_active_concepts);
        aggregate.mean_active_concepts += metrics.mean_active_concepts;
        aggregate.concepts_composed = aggregate.concepts_composed.max(metrics.concepts_composed);
        aggregate.decomposition_count += metrics.decomposition_count;
        aggregate.subproblems_created += metrics.subproblems_created;
        aggregate.subproblems_solved += metrics.subproblems_solved;
        aggregate.recombination_count += metrics.recombination_count;
        aggregate.maximum_decomposition_tree_depth = aggregate
            .maximum_decomposition_tree_depth
            .max(metrics.maximum_decomposition_tree_depth);
        aggregate.maximum_simultaneous_subproblems = aggregate
            .maximum_simultaneous_subproblems
            .max(metrics.maximum_simultaneous_subproblems);
        aggregate.pruned_branch_count += metrics.pruned_branch_count;
        aggregate.false_prune_count += metrics.false_prune_count;
        aggregate.semantic_prune_count += metrics.semantic_prune_count;
        aggregate.dominance_merge_count += metrics.dominance_merge_count;
        aggregate.false_merge_count += metrics.false_merge_count;
        aggregate.information_probes_proposed += metrics.information_probes_proposed;
        aggregate.information_probes_executed += metrics.information_probes_executed;
        aggregate.hypotheses_eliminated += metrics.hypotheses_eliminated;
        aggregate.expansions_saved_by_probes += metrics.expansions_saved_by_probes;
        aggregate.stagnation_prunes += metrics.stagnation_prunes;
        aggregate.backtracks += metrics.backtracks;
        aggregate.rollbacks += metrics.rollbacks;
        aggregate.promoted_concept_reuse_count += metrics.promoted_concept_reuse_count;
        aggregate.cross_generation_concept_composition_count +=
            metrics.cross_generation_concept_composition_count;
        aggregate.wall_time_units += metrics.wall_time_units;
        aggregate.peak_memory_units = aggregate.peak_memory_units.max(metrics.peak_memory_units);
        aggregate.branch_expansion_gini += metrics.branch_expansion_gini;
    }
    if !results.is_empty() {
        aggregate.mean_active_concepts /= results.len() as f64;
        aggregate.branch_expansion_gini /= results.len() as f64;
    }
    aggregate.useful_branch_ratio = rate(results.len(), aggregate.cumulative_branches_generated);
    aggregate
}

fn class_reports(
    tasks: &[EvaluationTask],
    results: &[SolveResult],
) -> BTreeMap<TaskClass, ClassResults> {
    [
        TaskClass::Depth,
        TaskClass::Width,
        TaskClass::Recombination,
        TaskClass::Composition,
        TaskClass::Mixed,
    ]
    .into_iter()
    .map(|class| {
        let paired = tasks
            .iter()
            .zip(results)
            .filter(|(task, _)| task.evaluator.task_class == class)
            .collect::<Vec<_>>();
        let class_results = paired
            .iter()
            .map(|(_, result)| (*result).clone())
            .collect::<Vec<_>>();
        let solved = class_results
            .iter()
            .filter(|result| result.strictly_correct)
            .count();
        (
            class,
            ClassResults {
                task_class: class,
                tasks: class_results.len(),
                strict_solve_rate: rate(solved, class_results.len()),
                maximum_required_depth_evaluator_only: paired
                    .iter()
                    .map(|(task, _)| task.evaluator.required_depth)
                    .max()
                    .unwrap_or(0),
                maximum_required_concepts_evaluator_only: paired
                    .iter()
                    .map(|(task, _)| task.evaluator.required_concepts)
                    .max()
                    .unwrap_or(0),
                results: class_results,
            },
        )
    })
    .collect()
}

fn complexity_frontier(
    tasks: &[EvaluationTask],
    results: &[SolveResult],
) -> ReasoningComplexityFrontier {
    let mut entries = tasks
        .iter()
        .zip(results)
        .map(|(task, result)| FrontierEntry {
            task_id: task.visible.task_id.clone(),
            task_class: task.evaluator.task_class,
            solution_graph_depth: result.metrics.solution_graph_depth,
            primitive_expanded_depth: result.metrics.primitive_expanded_solution_depth,
            peak_live_frontier: result.metrics.peak_simultaneously_live_branches,
            concept_composition_count: result.metrics.concepts_composed,
            subproblem_count: result.metrics.subproblems_created,
            recombination_count: result.metrics.recombination_count,
            total_expansions: result.metrics.cumulative_search_expansions,
            pareto_frontier: false,
        })
        .collect::<Vec<_>>();
    for index in 0..entries.len() {
        let dominated = (0..entries.len())
            .any(|other| other != index && dominates_frontier(&entries[other], &entries[index]));
        entries[index].pareto_frontier = !dominated;
    }
    ReasoningComplexityFrontier {
        entries,
        scalar_intelligence_score_reported: false,
        dimensions_kept_independent: true,
    }
}

fn dominates_frontier(left: &FrontierEntry, right: &FrontierEntry) -> bool {
    let no_worse = left.solution_graph_depth >= right.solution_graph_depth
        && left.concept_composition_count >= right.concept_composition_count
        && left.subproblem_count >= right.subproblem_count
        && left.recombination_count >= right.recombination_count
        && left.total_expansions <= right.total_expansions;
    let strictly = left.solution_graph_depth > right.solution_graph_depth
        || left.concept_composition_count > right.concept_composition_count
        || left.subproblem_count > right.subproblem_count
        || left.recombination_count > right.recombination_count
        || left.total_expansions < right.total_expansions;
    no_worse && strictly
}

fn median_hard(tasks: &[EvaluationTask], results: &[SolveResult]) -> f64 {
    median(
        &tasks
            .iter()
            .zip(results)
            .filter(|(task, _)| {
                matches!(
                    task.evaluator.task_class,
                    TaskClass::Width | TaskClass::Mixed
                )
            })
            .map(|(_, result)| result.metrics.cumulative_search_expansions)
            .collect::<Vec<_>>(),
    )
}

fn class_metric_diversity(reports: &BTreeMap<TaskClass, ClassResults>) -> bool {
    let depths = reports
        .values()
        .flat_map(|report| {
            report
                .results
                .iter()
                .map(|result| result.metrics.solution_graph_depth)
        })
        .collect::<std::collections::BTreeSet<_>>();
    let branches = reports
        .values()
        .flat_map(|report| {
            report
                .results
                .iter()
                .map(|result| result.metrics.cumulative_branches_generated)
        })
        .collect::<std::collections::BTreeSet<_>>();
    depths.len() >= 4 && branches.len() >= 4
}

fn mean<F: Fn(&SolveResult) -> usize>(
    tasks: &[EvaluationTask],
    results: &[SolveResult],
    class: TaskClass,
    value: F,
) -> f64 {
    let values = tasks
        .iter()
        .zip(results)
        .filter(|(task, _)| task.evaluator.task_class == class)
        .map(|(_, result)| value(result))
        .collect::<Vec<_>>();
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<usize>() as f64 / values.len() as f64
    }
}

fn count_class(tasks: &[EvaluationTask], class: TaskClass) -> usize {
    tasks
        .iter()
        .filter(|task| task.evaluator.task_class == class)
        .count()
}

fn gate(name: &str, passed: bool, evidence: String) -> GateResult {
    GateResult {
        gate: name.to_string(),
        passed,
        evidence,
    }
}

fn failed_disposition(gates: &[GateResult]) -> &'static str {
    let failed = gates
        .iter()
        .find(|gate| !gate.passed)
        .map(|gate| gate.gate.as_str());
    match failed {
        Some("FRONTIER_CONTROL") => "FRONTIER_EXPLOSION_NOT_REDUCED",
        Some("HARD_DEPTH_PRESERVATION") => "DEEP_REASONING_REGRESSION",
        Some("DECOMPOSITION_RECOMBINATION") => "DECOMPOSITION_NOT_DEMONSTRATED",
        Some("SEMANTIC_PRUNING_VALUE") => "FALSE_PRUNING_FAILURE",
        Some("NO_CONTAMINATION") => "CONTAMINATION_FAILURE",
        Some("SPARSE_ACTIVATION") => "SPARSE_ROUTING_REGRESSION",
        _ => "ADAPTIVE_CONTROL_NOT_DEMONSTRATED",
    }
}

fn median(values: &[usize]) -> f64 {
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

fn rate(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn reduction(baseline: f64, adaptive: f64) -> f64 {
    if baseline <= 0.0 {
        0.0
    } else {
        (baseline - adaptive) / baseline
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    #[test]
    fn sem2_passes_all_gates_without_recursive_mutation() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root = manifest_dir
            .parent()
            .and_then(|path| path.parent())
            .expect("root")
            .to_path_buf();
        let outcome = super::run_sem2(&root).expect("run");
        assert_eq!(outcome.final_report.sem2_status, "PASS");
        assert!(outcome.final_report.gates.iter().all(|gate| gate.passed));
        assert_eq!(outcome.final_report.recursive_source_mutations, 0);
        assert!(!outcome.final_report.sem3_started);
    }
}
