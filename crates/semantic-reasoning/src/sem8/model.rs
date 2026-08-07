use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Domain {
    Mathematics,
    Programming,
    StatefulMachine,
    DataTransform,
    ExternalDefinition,
    DomainLight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RoleKind {
    State,
    Input,
    Transform,
    Condition,
    Accumulator,
    Boundary,
    Termination,
    Invariant,
    Resource,
    Observation,
    Output,
    Stage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RelationKind {
    Requires,
    Transforms,
    Preserves,
    Consumes,
    Produces,
    Precedes,
    Guards,
    Terminates,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MechanismTransform {
    StateEvolution,
    ElementwiseTransform,
    GuardedTraversal,
    StatefulReduction,
    StageComposition,
    QuotientPartition,
    ScopedRelation,
    ReversibleStateTransform,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AssumptionKind {
    Deterministic,
    Terminates,
    InvariantGlobal,
    OrderPreserving,
    Reversible,
    Lossless,
    Pure,
    Associative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AssumptionStatus {
    Satisfied,
    Violated,
    Unknown,
    Irrelevant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MechanismRole {
    pub role_id: String,
    pub kind: RoleKind,
    pub type_class: String,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MechanismRelation {
    pub from_role_id: String,
    pub kind: RelationKind,
    pub to_role_id: String,
    pub essential: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MechanismAssumption {
    pub assumption_id: String,
    pub kind: AssumptionKind,
    pub required: bool,
    pub evidence_origin: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MechanismIR {
    pub mechanism_id: String,
    pub source_concept_ids: Vec<String>,
    pub source_domain: Domain,
    pub generation: usize,
    pub roles: Vec<MechanismRole>,
    pub states: Vec<String>,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub preconditions: Vec<String>,
    pub invariants: Vec<String>,
    pub transform: MechanismTransform,
    pub transformations: Vec<String>,
    pub dependency_edges: Vec<MechanismRelation>,
    pub causal_edges: Vec<MechanismRelation>,
    pub branch_conditions: Vec<String>,
    pub termination_conditions: Vec<String>,
    pub preserved_properties: Vec<String>,
    pub consumed_properties: Vec<String>,
    pub produced_properties: Vec<String>,
    pub failure_conditions: Vec<String>,
    pub assumptions: Vec<MechanismAssumption>,
    pub executable: bool,
    pub provenance: Vec<String>,
    pub semantic_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SourceSplit {
    Development,
    Blind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceManifestEntry {
    pub mechanism_id: String,
    pub semantic_sha256: String,
    pub source_domain: Domain,
    pub split: SourceSplit,
    pub target_pair_metadata_included: bool,
    pub human_analogy_label_included: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceManifest {
    pub run_id: String,
    pub split: SourceSplit,
    pub entries: Vec<SourceManifestEntry>,
    pub frozen_before_evaluation: bool,
    pub manifest_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransferTaskCategory {
    MathToProgramState,
    ProgramToMathState,
    CrossDataDomain,
    OpaqueStateMachine,
    StructuralMimicAdversarial,
    SemanticEquivalentDifferentStructure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TargetBehavior {
    AddEach,
    MultiplyEach,
    FilterGreater,
    Sum,
    StateDelta,
    ReverseDelta,
    QuotientClass,
    ComposeDeltas,
    ScopedIdentity,
    MapThenSum,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetRoleDefinition {
    pub opaque_role_id: String,
    pub kind: RoleKind,
    pub type_class: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetRelationDefinition {
    pub from_opaque_role_id: String,
    pub kind: RelationKind,
    pub to_opaque_role_id: String,
    pub essential: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetAssumptionEvidence {
    pub kind: AssumptionKind,
    pub status: AssumptionStatus,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibleTransferTask {
    pub task_id: String,
    pub target_domain: Domain,
    pub opaque_entities: Vec<String>,
    pub roles: Vec<TargetRoleDefinition>,
    pub relations: Vec<TargetRelationDefinition>,
    pub behavior: TargetBehavior,
    pub parameter: i64,
    pub assumption_evidence: Vec<TargetAssumptionEvidence>,
    pub graph_nodes: usize,
    pub graph_edges: usize,
    pub primitive_set_sha256: String,
    pub executable_definition_sha256: String,
    pub zero_target_examples: bool,
    pub target_solution_included: bool,
    pub source_mechanism_id_included: bool,
    pub intended_analogy_included: bool,
    pub correct_role_mapping_included: bool,
    pub transfer_family_included: bool,
    pub frozen: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferEvaluatorTask {
    pub visible: VisibleTransferTask,
    pub category: TransferTaskCategory,
    pub compatible_transforms: Vec<MechanismTransform>,
    pub expected_source_count: usize,
    pub invalid_analogy: bool,
    pub semantic_equivalence_different_structure: bool,
    pub hidden_inputs: Vec<Vec<i64>>,
    pub target_only_expansions_required: usize,
    pub transfer_expansions_required: usize,
    pub source_split_required: SourceSplit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetManifest {
    pub run_id: String,
    pub generator_version: String,
    pub seed_commitment_sha256: String,
    pub tasks: Vec<VisibleTransferTask>,
    pub target_answers_included: bool,
    pub source_target_pairs_included: bool,
    pub evaluator_categories_included: bool,
    pub hidden_cases_included: bool,
    pub frozen_before_evaluation: bool,
    pub manifest_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransferCondition {
    TargetOnlyA,
    StructuralSimilarityB,
    SemanticRoleMappingC,
    FullMechanismTransferD,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleMapping {
    pub source_mechanism_id: String,
    pub target_task_id: String,
    pub role_bindings: BTreeMap<String, String>,
    pub required_roles_mapped: usize,
    pub required_roles_total: usize,
    pub essential_relations_preserved: usize,
    pub essential_relations_total: usize,
    pub semantic_role_pass: bool,
    pub relation_preservation_pass: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssumptionLedgerEntry {
    pub task_id: String,
    pub source_mechanism_id: String,
    pub assumption_id: String,
    pub kind: AssumptionKind,
    pub required: bool,
    pub status: AssumptionStatus,
    pub target_evidence: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransferDisposition {
    Instantiated,
    RejectedAssumption,
    TargetOnlySolved,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransferRecord {
    pub task_id: String,
    pub category: TransferTaskCategory,
    pub condition: TransferCondition,
    pub target_domain: Domain,
    pub zero_shot: bool,
    pub transfer_attempted: bool,
    pub selected_source_mechanism_ids: Vec<String>,
    pub candidate_mechanisms_considered: usize,
    pub role_mappings: Vec<RoleMapping>,
    pub assumption_ledger: Vec<AssumptionLedgerEntry>,
    pub required_assumptions_satisfied: bool,
    pub relation_preservation_passed: bool,
    pub target_candidate_instantiated: bool,
    pub target_verifier_passed: bool,
    pub invalid_analogy: bool,
    pub invalid_transfer_accepted: bool,
    pub invalid_transfer_rejected: bool,
    pub structural_mimic: bool,
    pub semantic_equivalence_different_structure: bool,
    pub source_used: bool,
    pub causal_utility: bool,
    pub search_expansions: usize,
    pub reasoning_depth: usize,
    pub active_branches: usize,
    pub wall_time_class: String,
    pub active_concept_budget: usize,
    pub disposition: TransferDisposition,
    pub solved: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BaselineReport {
    pub condition: TransferCondition,
    pub records: Vec<TransferRecord>,
    pub tasks: usize,
    pub solved: usize,
    pub solve_rate: f64,
    pub median_expansions: f64,
    pub median_reasoning_depth: f64,
    pub peak_active_branches: usize,
    pub equal_expansion_budget: usize,
    pub equal_wall_time_class: String,
    pub equal_active_concept_budget: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferAblationRecord {
    pub task_id: String,
    pub source_mechanisms_enabled: Vec<String>,
    pub solved_with_transfer: bool,
    pub solved_without_transfer: bool,
    pub expansions_with_transfer: usize,
    pub expansions_without_transfer: usize,
    pub reasoning_depth_with_transfer: usize,
    pub reasoning_depth_without_transfer: usize,
    pub causal_value: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransferDistanceRecord {
    pub task_id: String,
    pub source_domain: Domain,
    pub target_domain: Domain,
    pub type_difference: f64,
    pub vocabulary_difference: f64,
    pub graph_shape_difference: f64,
    pub primitive_set_overlap: f64,
    pub semantic_role_overlap: f64,
    pub aggregate_distance: f64,
    pub solved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossDomainCandidate {
    pub candidate_id: String,
    pub generation: usize,
    pub parent_concept_ids: Vec<String>,
    pub source_domains: Vec<Domain>,
    pub role_kinds: Vec<RoleKind>,
    pub relation_kinds: Vec<RelationKind>,
    pub required_domain_tokens: Vec<String>,
    pub executable_instances: usize,
    pub fresh_domains_validated: usize,
    pub broken_assumption_aware: bool,
    pub causal_ablation_passed: bool,
    pub compression_ratio_milli: usize,
    pub provenance: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossDomainPromotion {
    pub candidate_id: String,
    pub promoted_concept_id: Option<String>,
    pub promoted: bool,
    pub multi_domain_pass: bool,
    pub executable_pass: bool,
    pub relation_preservation_pass: bool,
    pub fresh_domain_pass: bool,
    pub broken_assumption_pass: bool,
    pub causal_ablation_pass: bool,
    pub compression_reuse_pass: bool,
    pub provenance_pass: bool,
    pub predecessor_concepts_overwritten: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SparseTransferAudit {
    pub total_source_mechanisms: usize,
    pub indexed_route_keys: usize,
    pub peak_candidates_retrieved: usize,
    pub peak_active_concepts: usize,
    pub full_catalog_scans: usize,
    pub routing_false_negatives: usize,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferContaminationAudit {
    pub source_target_pair_labels_visible: usize,
    pub target_solutions_visible: usize,
    pub human_analogy_labels_visible: usize,
    pub lexical_similarity_used_as_transfer_authority: usize,
    pub external_transfer_solution_dependencies: usize,
    pub network_calls: usize,
    pub external_llm_calls: usize,
    pub local_teacher_calls: usize,
    pub recursive_source_mutations: usize,
    pub self_observe: bool,
    pub self_measure: bool,
    pub self_propose: bool,
    pub self_apply: bool,
    pub source_mutation: bool,
    pub auto_patch: bool,
    pub auto_commit: bool,
    pub auto_push: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sem8FinalReport {
    pub sem8_status: String,
    pub disposition: String,
    pub run_id: String,
    pub canonical_integrity: String,
    pub predecessor_integrity: String,
    pub predecessor_semantic_hash_changes: usize,
    pub fresh_blind_transfer_tasks: usize,
    pub zero_shot_transfer_tasks: usize,
    pub adversarial_transfer_tasks: usize,
    pub source_mechanisms_available: usize,
    pub source_mechanisms_selected: usize,
    pub transfer_candidates: usize,
    pub valid_transfers: usize,
    pub causally_useful_transfers: usize,
    pub baseline_a_solve_rate: f64,
    pub baseline_b_solve_rate: f64,
    pub baseline_c_solve_rate: f64,
    pub full_d_solve_rate: f64,
    pub baseline_a_median_expansions: f64,
    pub full_d_median_expansions: f64,
    pub zero_shot_cross_domain_transfer_rate: f64,
    pub role_mapping_pass_rate: f64,
    pub relation_preservation_pass_rate: f64,
    pub broken_assumption_cases: usize,
    pub broken_assumption_detection_rate: f64,
    pub structural_mimic_cases: usize,
    pub structural_mimic_false_transfer_rate: f64,
    pub semantic_equivalence_transfer_cases: usize,
    pub semantic_equivalence_transfer_rate: f64,
    pub invalid_transfer_attempts: usize,
    pub invalid_transfers_accepted: usize,
    pub invalid_transfers_rejected: usize,
    pub transfer_ablation_pass: bool,
    pub direct_source_transfers: usize,
    pub adapted_source_transfers: usize,
    pub new_cross_domain_candidates: usize,
    pub new_cross_domain_abstractions_promoted: usize,
    pub max_source_mechanisms_composed: usize,
    pub gen6_candidates: usize,
    pub gen6_promoted: usize,
    pub max_autonomous_concept_generation: usize,
    pub lexical_similarity_used_as_transfer_authority: usize,
    pub external_transfer_solution_dependencies: usize,
    pub full_catalog_scans: usize,
    pub routing_false_negatives: usize,
    pub autonomous_source_selection_pass: bool,
    pub cross_domain_role_mapping_pass: bool,
    pub executable_transfer_pass: bool,
    pub broken_assumption_discipline_pass: bool,
    pub structural_mimic_resistance_pass: bool,
    pub semantic_equivalence_transfer_pass: bool,
    pub causal_transfer_pass: bool,
    pub transfer_leakage_audit_pass: bool,
    pub gates: BTreeMap<String, bool>,
    pub recursive_source_mutations: usize,
    pub sem9_started: bool,
    pub next_allowed_stage: String,
}
