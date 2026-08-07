use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::sem8::model::{
    AssumptionKind, AssumptionStatus, Domain, MechanismTransform, RelationKind, RoleKind,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelfRole {
    pub role_id: String,
    pub kind: RoleKind,
    pub type_class: String,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelfRelation {
    pub from_role_id: String,
    pub kind: RelationKind,
    pub to_role_id: String,
    pub essential: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelfMechanismIR {
    pub component_id: String,
    pub role: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub state: Vec<String>,
    pub transformations: Vec<String>,
    pub roles: Vec<SelfRole>,
    pub relations: Vec<SelfRelation>,
    pub preconditions: Vec<String>,
    pub invariants: Vec<String>,
    pub dependencies: Vec<String>,
    pub resource_cost: Vec<String>,
    pub failure_modes: Vec<String>,
    pub externally_visible_behavior: Vec<String>,
    pub protected_status: bool,
    pub eligible_for_self_application: bool,
    pub provenance: Vec<String>,
    pub semantic_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtectedCoreEntry {
    pub component_id: String,
    pub relative_path: String,
    pub sha256: String,
    pub mutation_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtectedCoreManifest {
    pub run_id: String,
    pub entries: Vec<ProtectedCoreEntry>,
    pub source_tree_sha256: String,
    pub evaluator_tree_sha256: String,
    pub mutation_authority_enabled: bool,
    pub frozen_before_proposals: bool,
    pub manifest_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelfWeaknessRecord {
    pub weakness_id: String,
    pub component_id: String,
    pub observed_mechanism: String,
    pub measured_cost: usize,
    pub baseline_operations: usize,
    pub redundant_operations: usize,
    pub redundancy_rate: f64,
    pub affected_task_classes: Vec<String>,
    pub supporting_traces: Vec<String>,
    pub candidate_causal_explanation: String,
    pub required_role_signature: Vec<RoleKind>,
    pub assumption_evidence: BTreeMap<AssumptionKind, AssumptionStatus>,
    pub confidence: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelfRoleMapping {
    pub proposal_id: String,
    pub source_mechanism_id: String,
    pub source_concept_ids: Vec<String>,
    pub self_target_component: String,
    pub bindings: BTreeMap<String, String>,
    pub required_roles_mapped: usize,
    pub required_roles_total: usize,
    pub essential_relations_preserved: usize,
    pub essential_relations_total: usize,
    pub pass: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelfAssumptionLedgerEntry {
    pub proposal_id: String,
    pub source_mechanism_id: String,
    pub assumption_id: String,
    pub kind: AssumptionKind,
    pub required: bool,
    pub status: AssumptionStatus,
    pub self_target_evidence: String,
    pub expected_risk: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SelfApplicationDisposition {
    RejectedMapping,
    ValidNoPatch,
    PatchInvalid,
    PatchRegression,
    PatchNoGain,
    PatchGain,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelfApplicationProposal {
    pub proposal_id: String,
    pub weakness_id: String,
    pub target_component_id: String,
    pub source_mechanism_id: String,
    pub source_concept_ids: Vec<String>,
    pub source_origin_domain: Domain,
    pub source_transform: MechanismTransform,
    pub retrieval_score: f64,
    pub candidates_considered: usize,
    pub human_source_target_mapping: bool,
    pub valid_self_analogy: bool,
    pub executable_self_modification: bool,
    pub beneficial_self_modification: bool,
    pub disposition: SelfApplicationDisposition,
    pub rejection_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChangeOperation {
    MergeEquivalentStates,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeIR {
    pub change_id: String,
    pub proposal_id: String,
    pub target_component_id: String,
    pub source_mechanism_id: String,
    pub source_concept_ids: Vec<String>,
    pub operation: ChangeOperation,
    pub equivalence_key: String,
    pub preserved_invariants: Vec<String>,
    pub forbidden_components: Vec<String>,
    pub one_generation_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidatePatchPlan {
    pub candidate_id: String,
    pub change_ir: ChangeIR,
    pub files_changed: usize,
    pub lines_changed: usize,
    pub functions_changed: usize,
    pub components_touched: usize,
    pub sandbox_relative_path: String,
    pub generated_before_blind_open: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidatePatch {
    pub candidate_id: String,
    pub baseline_source_sha256: String,
    pub candidate_source_sha256: String,
    pub unified_diff: String,
    pub changed_paths: Vec<String>,
    pub protected_paths_touched: Vec<String>,
    pub benchmark_specific_branches: usize,
    pub provenance_chain: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandResult {
    pub command: String,
    pub success: bool,
    pub exit_code: i32,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxBuildResult {
    pub candidate_id: String,
    pub sandbox_only: bool,
    pub workspace_path: String,
    pub production_source_sha256_before: String,
    pub production_source_sha256_after: String,
    pub predecessor_binary_sha256: String,
    pub candidate_binary_sha256: String,
    pub commands: Vec<CommandResult>,
    pub fmt_pass: bool,
    pub clippy_pass: bool,
    pub build_pass: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxTestResult {
    pub candidate_id: String,
    pub commands: Vec<CommandResult>,
    pub tests_passed: usize,
    pub tests_failed: usize,
    pub regression_contracts_present: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CapabilityFamily {
    SemanticConcept,
    AdaptiveReasoning,
    MathematicalDerivation,
    Programming,
    DefinitionForaging,
    LanguageAdapter,
    CrossDomainTransfer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibleSelfTask {
    pub task_id: String,
    pub capability_family: CapabilityFamily,
    pub opaque_state_schema_sha256: String,
    pub public_contract_sha256: String,
    pub expected_output_included: bool,
    pub hidden_states_included: bool,
    pub benchmark_family_label_exposed_to_patch: bool,
    pub frozen: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreshBlindManifest {
    pub run_id: String,
    pub generator_version: String,
    pub seed_commitment_sha256: String,
    pub fresh_tasks: Vec<VisibleSelfTask>,
    pub adversarial_tasks: Vec<VisibleSelfTask>,
    pub self_diagnostic_tasks_included: bool,
    pub expected_outputs_included: bool,
    pub hidden_states_included: bool,
    pub frozen_before_candidate_evaluation: bool,
    pub manifest_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasoningState {
    pub canonical_key: u64,
    pub payload: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfEvaluatorTask {
    pub visible: VisibleSelfTask,
    pub states: Vec<ReasoningState>,
    pub expected_unique_keys: Vec<u64>,
    pub adversarial: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SelfBaseline {
    FrozenPredecessorA,
    RandomSafeMutationB,
    GenericHeuristicC,
    AutonomousSelfApplicationD,
    MechanismDisabledAblation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelfEvaluationRecord {
    pub task_id: String,
    pub capability_family: CapabilityFamily,
    pub condition: SelfBaseline,
    pub strict_correct: bool,
    pub search_expansions: usize,
    pub peak_frontier: usize,
    pub duplicate_states: usize,
    pub deterministic_resource_cost: usize,
    pub output_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelfBaselineReport {
    pub condition: SelfBaseline,
    pub tasks: usize,
    pub strict_solved: usize,
    pub strict_solve_rate: f64,
    pub median_expansions: f64,
    pub p95_expansions: usize,
    pub median_peak_frontier: f64,
    pub p95_peak_frontier: usize,
    pub median_resource_cost: f64,
    pub repetitions: usize,
    pub expansion_spread: usize,
    pub records: Vec<SelfEvaluationRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegressionFamilyResult {
    pub stage: String,
    pub protected_capability: String,
    pub predecessor_correct: usize,
    pub candidate_correct: usize,
    pub tasks: usize,
    pub regressed_tasks: usize,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AblationResult {
    pub ablation_id: String,
    pub enabled_median_expansions: f64,
    pub disabled_median_expansions: f64,
    pub enabled_solve_rate: f64,
    pub disabled_solve_rate: f64,
    pub gain_removed_or_materially_reduced: bool,
    pub passed: bool,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceConceptAblation {
    pub removed_source_concept_id: String,
    pub original_selected_mechanism: String,
    pub replacement_selected_mechanism: Option<String>,
    pub original_patch_operation: Option<ChangeOperation>,
    pub replacement_patch_operation: Option<ChangeOperation>,
    pub same_candidate_design_recovered: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafetyGateResults {
    pub protected_core_mutation_attempts: usize,
    pub protected_core_mutation_attempts_accepted: usize,
    pub safety_gate_rejections: Vec<String>,
    pub production_source_mutations: usize,
    pub auto_merges: usize,
    pub auto_pushes: usize,
    pub one_self_application_generation_enforced: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelfLeakageAudit {
    pub target_solution_leaks: usize,
    pub evaluator_expected_answers_read_by_generator: usize,
    pub benchmark_specific_self_patch_branches: usize,
    pub external_llm_calls: usize,
    pub local_teacher_calls: usize,
    pub network_writes: usize,
    pub remote_executions: usize,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelfSparseAudit {
    pub source_mechanisms_available: usize,
    pub peak_source_candidates_retrieved: usize,
    pub indexed_route_keys: usize,
    pub full_catalog_scans: usize,
    pub routing_false_negatives: usize,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerformanceResults {
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
    pub target_subset_expansion_reduction: f64,
    pub deterministic_repetitions: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sem9FinalReport {
    pub sem9_status: String,
    pub disposition: String,
    pub run_id: String,
    pub canonical_integrity: String,
    pub predecessor_integrity: String,
    pub production_source_mutations: usize,
    pub protected_core_mutation_attempts: usize,
    pub protected_core_mutation_attempts_accepted: usize,
    pub self_weaknesses_detected: usize,
    pub self_application_proposals: usize,
    pub self_applications_rejected_before_patch: usize,
    pub candidate_patches_generated: usize,
    pub candidate_patches_built: usize,
    pub candidate_patches_regression_free: usize,
    pub candidate_patches_with_gain: usize,
    pub best_self_source_concept_id: String,
    pub best_self_source_concept_origin_domain: Domain,
    pub best_self_target_component: String,
    pub best_self_role_mapping_pass: bool,
    pub best_self_assumption_pass: bool,
    pub fresh_blind_tasks: usize,
    pub predecessor_strict_solve_rate: f64,
    pub best_candidate_strict_solve_rate: f64,
    pub performance: PerformanceResults,
    pub self_application_ablation_pass: bool,
    pub source_concept_causality_pass: bool,
    pub benchmark_specific_self_patch_branches: usize,
    pub full_catalog_scans: usize,
    pub routing_false_negatives: usize,
    pub external_llm_calls: usize,
    pub local_teacher_calls: usize,
    pub network_writes: usize,
    pub remote_executions: usize,
    pub verified_self_application_candidates: usize,
    pub autonomous_self_target_selection_pass: bool,
    pub autonomous_source_concept_selection_pass: bool,
    pub self_role_mapping_pass: bool,
    pub self_patch_execution_pass: bool,
    pub fresh_blind_improvement_pass: bool,
    pub zero_regression_pass: bool,
    pub protected_core_pass: bool,
    pub production_immutability_pass: bool,
    pub gen7_candidates: usize,
    pub gen7_promoted: usize,
    pub max_autonomous_concept_generation: usize,
    pub gates: BTreeMap<String, bool>,
    pub sem10_started: bool,
    pub next_allowed_stage: String,
}
