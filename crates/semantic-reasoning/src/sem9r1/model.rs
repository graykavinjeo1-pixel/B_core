use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::sem9::model::{CapabilityFamily, RegressionFamilyResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactHash {
    pub relative_path: String,
    pub byte_length: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Run0001FailureReceipt {
    pub run_id: String,
    pub status: String,
    pub disposition: String,
    pub predecessor_commit: String,
    pub critical_artifacts: Vec<ArtifactHash>,
    pub artifacts_verified: usize,
    pub run0001_overwritten: bool,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Run0001ExecutionPathAudit {
    pub source_generated: bool,
    pub format_checked: bool,
    pub format_check_passed: bool,
    pub raw_compile_attempted: bool,
    pub raw_compile_succeeded: bool,
    pub clippy_run: bool,
    pub clippy_passed: bool,
    pub tests_run: bool,
    pub tests_passed: bool,
    pub behavioral_eval_run: bool,
    pub canonical_build_gate_passed: bool,
    pub behavioral_path_operation: String,
    pub candidate_source_operation: String,
    pub diagnostic_equivalence_cases: usize,
    pub diagnostic_equivalence_failures: usize,
    pub candidate_evaluation_path_equivalent: bool,
    pub built_zero_explanation: String,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailedCandidateFreeze {
    pub failed_candidate_semantic_id: String,
    pub failed_candidate_source_sha256: String,
    pub failed_candidate_patch_sha256: String,
    pub mapping_sha256: String,
    pub assumptions_sha256: String,
    pub target_component: String,
    pub source_concept_id: String,
    pub source_mechanism_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormatEquivalenceAudit {
    pub failed_candidate_source_sha256: String,
    pub formatted_candidate_source_sha256: String,
    pub failed_token_stream_sha256: String,
    pub formatted_token_stream_sha256: String,
    pub non_format_token_changes: usize,
    pub comments_ignored: usize,
    pub candidate_mapping_changed: bool,
    pub candidate_assumptions_changed: bool,
    pub candidate_target_changed: bool,
    pub candidate_logic_changed: bool,
    pub rustfmt_only: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct R1CommandResult {
    pub ordinal: usize,
    pub command: String,
    pub success: bool,
    pub exit_code: i32,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct R1BuildResults {
    pub candidate_id: String,
    pub strict_gate_order: Vec<String>,
    pub commands: Vec<R1CommandResult>,
    pub semantic_token_equivalence_pass: bool,
    pub cargo_fmt_check_pass: bool,
    pub clippy_d_warnings_pass: bool,
    pub workspace_tests_pass: bool,
    pub sandbox_containment_pass: bool,
    pub predecessor_binary_sha256: String,
    pub candidate_binary_sha256: String,
    pub production_source_sha256_before: String,
    pub production_source_sha256_after: String,
    pub production_source_mutations: usize,
    pub canonical_build_gate_pass: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct R1EvaluationRecord {
    pub task_id: String,
    pub capability_family: CapabilityFamily,
    pub condition: String,
    pub strict_correct: bool,
    pub search_expansions: usize,
    pub peak_frontier: usize,
    pub output_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct R1ConditionReport {
    pub condition: String,
    pub tasks: usize,
    pub strict_solved: usize,
    pub strict_solve_rate: f64,
    pub median_expansions: f64,
    pub p95_expansions: usize,
    pub peak_frontier: usize,
    pub p95_frontier: usize,
    pub median_wall_time_ns: f64,
    pub p95_wall_time_ns: u128,
    pub wall_time_spread_ns: u128,
    pub estimated_peak_memory_bytes: usize,
    pub deterministic_repetitions: usize,
    pub expansion_spread: usize,
    pub records: Vec<R1EvaluationRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct R1PerformanceResults {
    pub predecessor_median_expansions: f64,
    pub candidate_median_expansions: f64,
    pub predecessor_peak_frontier: usize,
    pub candidate_peak_frontier: usize,
    pub expansion_reduction: f64,
    pub frontier_reduction: f64,
    pub wall_time_reduction: f64,
    pub memory_reduction: f64,
    pub newly_solved_tasks: usize,
    pub regressed_tasks: usize,
    pub repeated_trials: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct R1AblationResult {
    pub candidate_on_median_expansions: f64,
    pub mechanism_disabled_median_expansions: f64,
    pub candidate_on_solve_rate: f64,
    pub mechanism_disabled_solve_rate: f64,
    pub gain_removed: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct R1SourceLineage {
    pub source_concept_id: String,
    pub source_mechanism_id: String,
    pub source_origin: String,
    pub target_component: String,
    pub run0001_mapping_sha256: String,
    pub run0002_mapping_sha256: String,
    pub human_reselection_performed: bool,
    pub source_concept_lineage_intact: bool,
    pub source_concept_causality_pass: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct R1ProtectedCoreAudit {
    pub production_source_sha256_before: String,
    pub production_source_sha256_after: String,
    pub production_source_mutations: usize,
    pub protected_core_mutation_attempts_accepted: usize,
    pub auto_merges: usize,
    pub auto_pushes: usize,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct R1LeakageAudit {
    pub benchmark_specific_self_patch_branches: usize,
    pub run0001_task_specific_patch_branches: usize,
    pub target_output_lookups: usize,
    pub evaluator_dependencies: usize,
    pub external_llm_calls: usize,
    pub local_teacher_calls: usize,
    pub network_writes: usize,
    pub remote_executions: usize,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct R1SparseAudit {
    pub full_catalog_scans: usize,
    pub routing_false_negatives: usize,
    pub source_reselection_performed: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sem9R1FinalReport {
    pub sem9_r1_status: String,
    pub disposition: String,
    pub run0001_preserved: bool,
    pub run0001_failure_receipt_sha256: String,
    pub run0001_evaluation_path_audit_pass: bool,
    pub run0002_id: String,
    pub run0002_fresh_blind_tasks: usize,
    pub run0002_blind_manifest_sha256: String,
    pub canonical_integrity: String,
    pub predecessor_integrity: String,
    pub failed_candidate_source_sha256: String,
    pub formatted_candidate_source_sha256: String,
    pub non_format_token_changes: usize,
    pub candidate_mapping_changed: bool,
    pub candidate_assumptions_changed: bool,
    pub candidate_target_changed: bool,
    pub candidate_logic_changed: bool,
    pub cargo_fmt_check_pass: bool,
    pub clippy_d_warnings_pass: bool,
    pub workspace_tests_pass: bool,
    pub production_source_mutations: usize,
    pub protected_core_mutation_attempts_accepted: usize,
    pub predecessor_strict_solve_rate_run0002: f64,
    pub candidate_strict_solve_rate_run0002: f64,
    pub performance: R1PerformanceResults,
    pub self_application_ablation_pass: bool,
    pub source_concept_lineage_intact: bool,
    pub source_concept_causality_pass: bool,
    pub benchmark_specific_self_patch_branches: usize,
    pub run0001_task_specific_patch_branches: usize,
    pub full_catalog_scans: usize,
    pub routing_false_negatives: usize,
    pub external_llm_calls: usize,
    pub local_teacher_calls: usize,
    pub network_writes: usize,
    pub remote_executions: usize,
    pub verified_self_application_candidates: usize,
    pub regression_matrix: Vec<RegressionFamilyResult>,
    pub gates: BTreeMap<String, bool>,
    pub sem10_started: bool,
    pub next_allowed_stage: String,
}
