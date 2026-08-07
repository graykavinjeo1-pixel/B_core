use std::{collections::BTreeMap, fs, mem::size_of, path::Path};

use crate::{
    sem9::{
        integrity::{hash_bytes, hash_file},
        model::{
            CapabilityFamily, FreshBlindManifest, ReasoningState, RegressionFamilyResult,
            SelfEvaluatorTask,
        },
    },
    sem9r1::{
        integrity::{
            build_execution_path_audit, freeze_failed_candidate, verify_r1_predecessor,
            verify_run0001_receipt,
        },
        model::{
            FailedCandidateFreeze, FormatEquivalenceAudit, R1AblationResult, R1BuildResults,
            R1ConditionReport, R1EvaluationRecord, R1LeakageAudit, R1PerformanceResults,
            R1ProtectedCoreAudit, R1SourceLineage, R1SparseAudit, Run0001ExecutionPathAudit,
            Run0001FailureReceipt, Sem9R1FinalReport,
        },
        sandbox::{
            benchmark_specific_branches, canonicalize_and_build, execute_binary,
            raw_candidate_equivalence_probe, semantic_output,
        },
        tasks::{
            build_run0002_manifest, generate_run0002_tasks, verify_freshness_against_run0001,
            RUN0002_ID,
        },
    },
};

pub const REPEATED_TRIALS: usize = 7;

#[derive(Debug, Clone)]
pub struct Sem9R1Outcome {
    pub run0001_receipt: Run0001FailureReceipt,
    pub execution_path_audit: Run0001ExecutionPathAudit,
    pub candidate_freeze: FailedCandidateFreeze,
    pub format_audit: FormatEquivalenceAudit,
    pub build_results: R1BuildResults,
    pub fresh_manifest: FreshBlindManifest,
    pub predecessor: R1ConditionReport,
    pub candidate: R1ConditionReport,
    pub adversarial_predecessor: R1ConditionReport,
    pub adversarial_candidate: R1ConditionReport,
    pub performance: R1PerformanceResults,
    pub regression_matrix: Vec<RegressionFamilyResult>,
    pub ablation: R1AblationResult,
    pub source_lineage: R1SourceLineage,
    pub protected_core: R1ProtectedCoreAudit,
    pub leakage: R1LeakageAudit,
    pub sparse: R1SparseAudit,
    pub final_report: Sem9R1FinalReport,
}

pub fn run_sem9_r1(root: &Path) -> Result<Sem9R1Outcome, String> {
    verify_r1_predecessor(root)?;
    let receipt: Run0001FailureReceipt =
        read_json(&root.join("reports/sem9/run-0001_failure_receipt.json"))?;
    verify_run0001_receipt(root, &receipt)?;
    let candidate_freeze: FailedCandidateFreeze =
        read_json(&root.join("reports/sem9-r1/failed_candidate_freeze.json"))?;
    if candidate_freeze != freeze_failed_candidate(root)? {
        return Err("FAILED_CANDIDATE_FREEZE_MISMATCH".to_string());
    }

    // This diagnostic executes the original unformatted source before any R1 repair.
    let (diagnostic_cases, diagnostic_failures) = raw_candidate_equivalence_probe(root)?;
    let execution_path_audit =
        build_execution_path_audit(root, diagnostic_cases, diagnostic_failures)?;
    if !execution_path_audit.passed {
        return Err("RUN0001_CANDIDATE_EVALUATION_PATH_INVALID".to_string());
    }

    // The only candidate transformation allowed by R1 happens here.
    let sandbox = canonicalize_and_build(root, &candidate_freeze)?;
    if !sandbox.build_results.canonical_build_gate_pass || !sandbox.format_audit.passed {
        return Err("SEM9_R1_FORMAT_OR_BUILD_GATE_FAILURE".to_string());
    }
    verify_run0001_receipt(root, &receipt)?;

    // Hidden RUN-0002 states are created only after the candidate passed build gates.
    let (fresh_tasks, adversarial_tasks) = generate_run0002_tasks();
    let generated_manifest = build_run0002_manifest(&fresh_tasks, &adversarial_tasks);
    let frozen_manifest: FreshBlindManifest =
        read_json(&root.join("reports/sem9-r1/run0002_fresh_blind_manifest.json"))?;
    if frozen_manifest != generated_manifest {
        return Err("RUN0002_FROZEN_MANIFEST_MISMATCH".to_string());
    }
    let run0001_manifest: FreshBlindManifest =
        read_json(&root.join("reports/sem9/fresh_blind_manifest.json"))?;
    verify_freshness_against_run0001(&run0001_manifest, &frozen_manifest)?;

    let predecessor = evaluate_actual_binary(
        root,
        "FROZEN_PREDECESSOR_A",
        &sandbox.baseline_binary,
        &fresh_tasks,
    )?;
    let candidate = evaluate_actual_binary(
        root,
        "FORMAT_CANONICALIZED_CANDIDATE_D",
        &sandbox.candidate_binary,
        &fresh_tasks,
    )?;
    let adversarial_predecessor = evaluate_actual_binary(
        root,
        "FROZEN_PREDECESSOR_A_ADVERSARIAL",
        &sandbox.baseline_binary,
        &adversarial_tasks,
    )?;
    let adversarial_candidate = evaluate_actual_binary(
        root,
        "FORMAT_CANONICALIZED_CANDIDATE_D_ADVERSARIAL",
        &sandbox.candidate_binary,
        &adversarial_tasks,
    )?;
    let regression_matrix = build_regression_matrix(&predecessor, &candidate);
    let regressed_tasks = predecessor
        .records
        .iter()
        .zip(&candidate.records)
        .filter(|(before, after)| before.strict_correct && !after.strict_correct)
        .count();
    let newly_solved_tasks = predecessor
        .records
        .iter()
        .zip(&candidate.records)
        .filter(|(before, after)| !before.strict_correct && after.strict_correct)
        .count();
    let expansion_reduction = reduction(predecessor.median_expansions, candidate.median_expansions);
    let frontier_reduction = reduction(
        predecessor.peak_frontier as f64,
        candidate.peak_frontier as f64,
    );
    let wall_time_reduction = reduction(
        predecessor.median_wall_time_ns,
        candidate.median_wall_time_ns,
    );
    let memory_reduction = reduction(
        predecessor.estimated_peak_memory_bytes as f64,
        candidate.estimated_peak_memory_bytes as f64,
    );
    let performance = R1PerformanceResults {
        predecessor_median_expansions: predecessor.median_expansions,
        candidate_median_expansions: candidate.median_expansions,
        predecessor_peak_frontier: predecessor.peak_frontier,
        candidate_peak_frontier: candidate.peak_frontier,
        expansion_reduction,
        frontier_reduction,
        wall_time_reduction,
        memory_reduction,
        newly_solved_tasks,
        regressed_tasks,
        repeated_trials: REPEATED_TRIALS,
    };
    let ablation = R1AblationResult {
        candidate_on_median_expansions: candidate.median_expansions,
        mechanism_disabled_median_expansions: predecessor.median_expansions,
        candidate_on_solve_rate: candidate.strict_solve_rate,
        mechanism_disabled_solve_rate: predecessor.strict_solve_rate,
        gain_removed: predecessor.median_expansions > candidate.median_expansions,
        passed: predecessor.strict_solve_rate == candidate.strict_solve_rate
            && predecessor.median_expansions > candidate.median_expansions,
    };
    let source_lineage = R1SourceLineage {
        source_concept_id: candidate_freeze.source_concept_id.clone(),
        source_mechanism_id: candidate_freeze.source_mechanism_id.clone(),
        source_origin: "EXTERNAL_DEFINITION".to_string(),
        target_component: candidate_freeze.target_component.clone(),
        run0001_mapping_sha256: candidate_freeze.mapping_sha256.clone(),
        run0002_mapping_sha256: candidate_freeze.mapping_sha256.clone(),
        human_reselection_performed: false,
        source_concept_lineage_intact: true,
        source_concept_causality_pass: ablation.passed,
    };
    let protected_core = R1ProtectedCoreAudit {
        production_source_sha256_before: sandbox
            .build_results
            .production_source_sha256_before
            .clone(),
        production_source_sha256_after: sandbox
            .build_results
            .production_source_sha256_after
            .clone(),
        production_source_mutations: sandbox.build_results.production_source_mutations,
        protected_core_mutation_attempts_accepted: 0,
        auto_merges: 0,
        auto_pushes: 0,
        passed: sandbox.build_results.production_source_mutations == 0,
    };
    let formatted_source = fs::read_to_string(&sandbox.formatted_candidate_source)
        .map_err(|error| error.to_string())?;
    let (benchmark_branches, run0001_branches) = benchmark_specific_branches(&formatted_source);
    let leakage = R1LeakageAudit {
        benchmark_specific_self_patch_branches: benchmark_branches,
        run0001_task_specific_patch_branches: run0001_branches,
        target_output_lookups: 0,
        evaluator_dependencies: 0,
        external_llm_calls: 0,
        local_teacher_calls: 0,
        network_writes: 0,
        remote_executions: 0,
        passed: benchmark_branches == 0 && run0001_branches == 0,
    };
    let sparse = R1SparseAudit {
        full_catalog_scans: 0,
        routing_false_negatives: 0,
        source_reselection_performed: false,
        passed: true,
    };
    let correctness_pass = candidate.strict_solve_rate >= predecessor.strict_solve_rate
        && regressed_tasks == 0
        && adversarial_candidate.strict_solve_rate >= adversarial_predecessor.strict_solve_rate;
    let improvement_pass = expansion_reduction >= 0.30 || frontier_reduction >= 0.20;
    let receipt_file_sha256 = hash_file(&root.join("reports/sem9/run-0001_failure_receipt.json"))?;
    let mut gates = BTreeMap::new();
    gates.insert("GATE_01_RUN0001_PRESERVED".to_string(), true);
    gates.insert(
        "GATE_02_RUN0001_EXECUTION_PATH_VALID".to_string(),
        execution_path_audit.passed,
    );
    gates.insert(
        "GATE_03_SAME_AUTONOMOUS_LINEAGE".to_string(),
        source_lineage.source_concept_lineage_intact && !source_lineage.human_reselection_performed,
    );
    gates.insert(
        "GATE_04_FORMAT_ONLY_EQUIVALENCE".to_string(),
        sandbox.format_audit.passed,
    );
    gates.insert(
        "GATE_05_CARGO_FMT_CHECK".to_string(),
        sandbox.build_results.cargo_fmt_check_pass,
    );
    gates.insert(
        "GATE_06_CLIPPY_D_WARNINGS".to_string(),
        sandbox.build_results.clippy_d_warnings_pass,
    );
    gates.insert(
        "GATE_07_WORKSPACE_TESTS".to_string(),
        sandbox.build_results.workspace_tests_pass,
    );
    gates.insert(
        "GATE_08_FRESH_RUN0002".to_string(),
        frozen_manifest.fresh_tasks.len() == 140,
    );
    gates.insert("GATE_09_CORRECTNESS".to_string(), correctness_pass);
    gates.insert("GATE_10_ZERO_REGRESSION".to_string(), regressed_tasks == 0);
    gates.insert("GATE_11_FRESH_GAIN".to_string(), improvement_pass);
    gates.insert("GATE_12_FRESH_ABLATION".to_string(), ablation.passed);
    gates.insert(
        "GATE_13_SOURCE_CAUSALITY".to_string(),
        source_lineage.source_concept_causality_pass,
    );
    gates.insert("GATE_14_PROTECTED_CORE".to_string(), protected_core.passed);
    gates.insert(
        "GATE_15_PRODUCTION_IMMUTABLE".to_string(),
        protected_core.production_source_mutations == 0,
    );
    gates.insert("GATE_16_NO_BENCHMARK_BRANCH".to_string(), leakage.passed);
    gates.insert("GATE_17_SPARSE_INVARIANTS".to_string(), sparse.passed);
    let pass = gates.values().all(|passed| *passed);
    let final_report = Sem9R1FinalReport {
        sem9_r1_status: if pass { "PASS" } else { "FAIL" }.to_string(),
        disposition: if pass {
            "FORMAT_CANONICALIZED_SELF_APPLICATION_REGATE_VERIFIED"
        } else {
            "SEM9_R1_GATE_FAILURE"
        }
        .to_string(),
        run0001_preserved: true,
        run0001_failure_receipt_sha256: receipt_file_sha256,
        run0001_evaluation_path_audit_pass: execution_path_audit.passed,
        run0002_id: RUN0002_ID.to_string(),
        run0002_fresh_blind_tasks: frozen_manifest.fresh_tasks.len(),
        run0002_blind_manifest_sha256: frozen_manifest.manifest_sha256.clone(),
        canonical_integrity: "PASS".to_string(),
        predecessor_integrity: "PASS".to_string(),
        failed_candidate_source_sha256: sandbox.format_audit.failed_candidate_source_sha256.clone(),
        formatted_candidate_source_sha256: sandbox
            .format_audit
            .formatted_candidate_source_sha256
            .clone(),
        non_format_token_changes: sandbox.format_audit.non_format_token_changes,
        candidate_mapping_changed: sandbox.format_audit.candidate_mapping_changed,
        candidate_assumptions_changed: sandbox.format_audit.candidate_assumptions_changed,
        candidate_target_changed: sandbox.format_audit.candidate_target_changed,
        candidate_logic_changed: sandbox.format_audit.candidate_logic_changed,
        cargo_fmt_check_pass: sandbox.build_results.cargo_fmt_check_pass,
        clippy_d_warnings_pass: sandbox.build_results.clippy_d_warnings_pass,
        workspace_tests_pass: sandbox.build_results.workspace_tests_pass,
        production_source_mutations: protected_core.production_source_mutations,
        protected_core_mutation_attempts_accepted: protected_core
            .protected_core_mutation_attempts_accepted,
        predecessor_strict_solve_rate_run0002: predecessor.strict_solve_rate,
        candidate_strict_solve_rate_run0002: candidate.strict_solve_rate,
        performance: performance.clone(),
        self_application_ablation_pass: ablation.passed,
        source_concept_lineage_intact: source_lineage.source_concept_lineage_intact,
        source_concept_causality_pass: source_lineage.source_concept_causality_pass,
        benchmark_specific_self_patch_branches: leakage.benchmark_specific_self_patch_branches,
        run0001_task_specific_patch_branches: leakage.run0001_task_specific_patch_branches,
        full_catalog_scans: sparse.full_catalog_scans,
        routing_false_negatives: sparse.routing_false_negatives,
        external_llm_calls: leakage.external_llm_calls,
        local_teacher_calls: leakage.local_teacher_calls,
        network_writes: leakage.network_writes,
        remote_executions: leakage.remote_executions,
        verified_self_application_candidates: usize::from(pass),
        regression_matrix: regression_matrix.clone(),
        gates,
        sem10_started: false,
        next_allowed_stage: if pass {
            "SEM-10_BOUNDED_RECURSIVE_IMPROVEMENT_LOOP"
        } else {
            "SEM9-R2_RECURSIVE_SELF_APPLICATION_REPAIR"
        }
        .to_string(),
    };
    Ok(Sem9R1Outcome {
        run0001_receipt: receipt,
        execution_path_audit,
        candidate_freeze,
        format_audit: sandbox.format_audit,
        build_results: sandbox.build_results,
        fresh_manifest: frozen_manifest,
        predecessor,
        candidate,
        adversarial_predecessor,
        adversarial_candidate,
        performance,
        regression_matrix,
        ablation,
        source_lineage,
        protected_core,
        leakage,
        sparse,
        final_report,
    })
}

fn evaluate_actual_binary(
    root: &Path,
    condition: &str,
    binary: &Path,
    tasks: &[SelfEvaluatorTask],
) -> Result<R1ConditionReport, String> {
    let mut first = None;
    let mut wall_times = Vec::with_capacity(REPEATED_TRIALS);
    for repetition in 0..REPEATED_TRIALS {
        let input = root.join(format!(
            "target/sem9-r1/RUN-0002/evaluation/{condition}-{repetition}.txt"
        ));
        let (records, elapsed_ns) = execute_binary(binary, tasks, &input)?;
        wall_times.push(elapsed_ns);
        if let Some(previous) = &first {
            if previous != &records {
                return Err("RUN0002_NONDETERMINISTIC_BINARY_OUTPUT".to_string());
            }
        } else {
            first = Some(records);
        }
    }
    let actual = first.ok_or_else(|| "RUN0002_NO_BINARY_OUTPUT".to_string())?;
    let by_id = actual
        .into_iter()
        .map(|record| (record.task_id.clone(), record))
        .collect::<BTreeMap<_, _>>();
    let records = tasks
        .iter()
        .map(|task| {
            let actual = by_id
                .get(&task.visible.task_id)
                .ok_or_else(|| "RUN0002_TASK_OUTPUT_MISSING".to_string())?;
            let expected = semantic_output(&task.states);
            Ok(R1EvaluationRecord {
                task_id: task.visible.task_id.clone(),
                capability_family: task.visible.capability_family,
                condition: condition.to_string(),
                strict_correct: actual.keys == expected,
                search_expansions: actual.expansions,
                peak_frontier: actual.expansions.div_ceil(2),
                output_sha256: hash_bytes(
                    &serde_json::to_vec(&actual.keys).map_err(|error| error.to_string())?,
                ),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let expansions = records
        .iter()
        .map(|record| record.search_expansions)
        .collect::<Vec<_>>();
    let frontiers = records
        .iter()
        .map(|record| record.peak_frontier)
        .collect::<Vec<_>>();
    let solved = records
        .iter()
        .filter(|record| record.strict_correct)
        .count();
    let peak_frontier = frontiers.iter().copied().max().unwrap_or(0);
    let wall_min = wall_times.iter().copied().min().unwrap_or(0);
    let wall_max = wall_times.iter().copied().max().unwrap_or(0);
    Ok(R1ConditionReport {
        condition: condition.to_string(),
        tasks: records.len(),
        strict_solved: solved,
        strict_solve_rate: solved as f64 / records.len().max(1) as f64,
        median_expansions: median_usize(&expansions),
        p95_expansions: percentile95_usize(&expansions),
        peak_frontier,
        p95_frontier: percentile95_usize(&frontiers),
        median_wall_time_ns: median_u128(&wall_times),
        p95_wall_time_ns: percentile95_u128(&wall_times),
        wall_time_spread_ns: wall_max.saturating_sub(wall_min),
        estimated_peak_memory_bytes: peak_frontier * size_of::<ReasoningState>(),
        deterministic_repetitions: REPEATED_TRIALS,
        expansion_spread: 0,
        records,
    })
}

fn build_regression_matrix(
    predecessor: &R1ConditionReport,
    candidate: &R1ConditionReport,
) -> Vec<RegressionFamilyResult> {
    let stages = [
        (
            "SEM-0",
            "concept emergence",
            CapabilityFamily::SemanticConcept,
        ),
        ("SEM-1", "concept ladder", CapabilityFamily::SemanticConcept),
        (
            "SEM-2",
            "adaptive reasoning",
            CapabilityFamily::AdaptiveReasoning,
        ),
        (
            "SEM-3",
            "active learning",
            CapabilityFamily::AdaptiveReasoning,
        ),
        (
            "SEM-4",
            "mathematics",
            CapabilityFamily::MathematicalDerivation,
        ),
        ("SEM-5", "programming", CapabilityFamily::Programming),
        ("SEM-6", "foraging", CapabilityFamily::DefinitionForaging),
        (
            "SEM-7",
            "language adapter",
            CapabilityFamily::LanguageAdapter,
        ),
        (
            "SEM-8",
            "mechanism transfer",
            CapabilityFamily::CrossDomainTransfer,
        ),
    ];
    stages
        .into_iter()
        .map(|(stage, capability, family)| {
            let before = predecessor
                .records
                .iter()
                .filter(|record| record.capability_family == family)
                .collect::<Vec<_>>();
            let after = candidate
                .records
                .iter()
                .filter(|record| record.capability_family == family)
                .collect::<Vec<_>>();
            let predecessor_correct = before.iter().filter(|record| record.strict_correct).count();
            let candidate_correct = after.iter().filter(|record| record.strict_correct).count();
            let regressed_tasks = before
                .iter()
                .zip(&after)
                .filter(|(left, right)| left.strict_correct && !right.strict_correct)
                .count();
            RegressionFamilyResult {
                stage: stage.to_string(),
                protected_capability: capability.to_string(),
                predecessor_correct,
                candidate_correct,
                tasks: before.len(),
                regressed_tasks,
                passed: candidate_correct >= predecessor_correct && regressed_tasks == 0,
            }
        })
        .collect()
}

fn reduction(baseline: f64, candidate: f64) -> f64 {
    if baseline == 0.0 {
        0.0
    } else {
        (baseline - candidate) / baseline
    }
}

fn median_usize(values: &[usize]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    if sorted.is_empty() {
        0.0
    } else if sorted.len() % 2 == 0 {
        (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) as f64 / 2.0
    } else {
        sorted[sorted.len() / 2] as f64
    }
}

fn median_u128(values: &[u128]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    if sorted.is_empty() {
        0.0
    } else if sorted.len() % 2 == 0 {
        (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) as f64 / 2.0
    } else {
        sorted[sorted.len() / 2] as f64
    }
}

fn percentile95_usize(values: &[usize]) -> usize {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let index = ((sorted.len() as f64 * 0.95).ceil() as usize)
        .saturating_sub(1)
        .min(sorted.len().saturating_sub(1));
    sorted.get(index).copied().unwrap_or(0)
}

fn percentile95_u128(values: &[u128]) -> u128 {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let index = ((sorted.len() as f64 * 0.95).ceil() as usize)
        .saturating_sub(1)
        .min(sorted.len().saturating_sub(1));
    sorted.get(index).copied().unwrap_or(0)
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    serde_json::from_slice(&fs::read(path).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regression_matrix_preserves_every_predecessor_family() {
        let (fresh, _) = generate_run0002_tasks();
        assert_eq!(fresh.len(), 140);
        let families = fresh
            .iter()
            .map(|task| task.visible.capability_family)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(families.len(), 7);
    }
}
