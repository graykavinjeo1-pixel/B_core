use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Language {
    Korean,
    English,
    Opaque,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LanguageTaskCategory {
    KoreanGrounding,
    EnglishGrounding,
    ParaphraseSynonym,
    AmbiguityReference,
    OpaqueRelexicalization,
    LanguageToForaging,
    LanguageToProgram,
    LanguageToMath,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GroundingDomain {
    PriorSemantic,
    Programming,
    Mathematics,
    ExternalForaged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SemanticOperation {
    Identify,
    AddEach,
    MultiplyEach,
    FilterGreater,
    FilterAtLeast,
    FilterNotGreater,
    Sum,
    CountGreater,
    RecurrenceStep,
    StatusClass,
    ScopedLookup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Quantifier {
    All,
    Any,
    None,
    ExactlyOne,
    AtLeast,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GroundingCondition {
    LexicalLookupA,
    StructuralParserB,
    SemanticNoConsolidationC,
    FullBidirectionalD,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RealizationStyle {
    Concise,
    Detailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticConcept {
    pub concept_id: String,
    pub generation: usize,
    pub parent_ids: Vec<String>,
    pub semantic_kind: String,
    pub executable_signature: String,
    pub invariants: Vec<String>,
    pub upstream_payload_sha256: String,
    pub concept_about_language: bool,
    pub required_lexical_tokens: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LexicalAlias {
    pub alias_id: String,
    pub surface_form: String,
    pub language: Language,
    pub concept_id: Option<String>,
    pub sense_id: String,
    pub morphological_features: BTreeMap<String, String>,
    pub syntactic_role: String,
    pub scope: String,
    pub confidence: f64,
    pub provenance: Vec<String>,
    pub version: String,
    pub semantic_grounding_complete: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeaningRequestIR {
    pub target_concept_id: String,
    pub target_state: String,
    pub inputs: Vec<String>,
    pub output: String,
    pub constraints: Vec<String>,
    pub requested_relations: Vec<String>,
    pub operation: SemanticOperation,
    pub parameter: Option<i64>,
    pub modifiers: Vec<String>,
    pub quantifier: Option<Quantifier>,
    pub quantifier_threshold: Option<usize>,
    pub ordering: Vec<String>,
    pub scope: String,
    pub reference_bindings: BTreeMap<String, String>,
    pub ambiguity_set: Vec<String>,
    pub lexical_mapping_confidence: f64,
    pub semantic_concept_confidence: f64,
    pub parse_confidence: f64,
    pub reference_resolution_confidence: f64,
    pub raw_text_in_reasoning_hot_path: bool,
}

impl MeaningRequestIR {
    pub fn semantic_projection(&self) -> SemanticProjection {
        SemanticProjection {
            target_concept_id: self.target_concept_id.clone(),
            target_state: self.target_state.clone(),
            inputs: self.inputs.clone(),
            output: self.output.clone(),
            constraints: self.constraints.clone(),
            requested_relations: self.requested_relations.clone(),
            operation: self.operation,
            parameter: self.parameter,
            modifiers: self.modifiers.clone(),
            quantifier: self.quantifier,
            quantifier_threshold: self.quantifier_threshold,
            ordering: self.ordering.clone(),
            scope: self.scope.clone(),
            reference_bindings: self.reference_bindings.clone(),
            ambiguity_set: self.ambiguity_set.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticProjection {
    pub target_concept_id: String,
    pub target_state: String,
    pub inputs: Vec<String>,
    pub output: String,
    pub constraints: Vec<String>,
    pub requested_relations: Vec<String>,
    pub operation: SemanticOperation,
    pub parameter: Option<i64>,
    pub modifiers: Vec<String>,
    pub quantifier: Option<Quantifier>,
    pub quantifier_threshold: Option<usize>,
    pub ordering: Vec<String>,
    pub scope: String,
    pub reference_bindings: BTreeMap<String, String>,
    pub ambiguity_set: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibleLanguageTask {
    pub task_id: String,
    pub category: LanguageTaskCategory,
    pub language: Language,
    pub domain: GroundingDomain,
    pub text: String,
    pub context: String,
    pub paraphrases: Vec<String>,
    pub near_contrast: Option<String>,
    pub introduced_alias: Option<String>,
    pub definition: Option<String>,
    pub definition_language: Option<Language>,
    pub target_language: Language,
    pub lookup_only: bool,
    pub active_text_sha256: String,
    pub answers_included: bool,
    pub expected_goal_ir_included: bool,
    pub target_program_included: bool,
    pub frozen: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LanguageEvaluatorTask {
    pub visible: VisibleLanguageTask,
    pub expected: MeaningRequestIR,
    pub near_contrast_expected: Option<MeaningRequestIR>,
    pub hidden_inputs: Vec<Vec<i64>>,
    pub requires_composition: bool,
    pub requires_semantic_disambiguation: bool,
    pub requires_alias_consolidation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageTaskManifest {
    pub run_id: String,
    pub generator_version: String,
    pub seed_commitment_sha256: String,
    pub tasks: Vec<VisibleLanguageTask>,
    pub expected_goal_ir_included: bool,
    pub hidden_inputs_included: bool,
    pub target_answers_included: bool,
    pub target_programs_included: bool,
    pub frozen_before_evaluation: bool,
    pub manifest_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroundingRecord {
    pub task_id: String,
    pub category: LanguageTaskCategory,
    pub language: Language,
    pub domain: GroundingDomain,
    pub condition: GroundingCondition,
    pub candidate_concept_ids: Vec<String>,
    pub selected_concept_id: Option<String>,
    pub meaning_request_ir: Option<MeaningRequestIR>,
    pub semantic_projection_sha256: Option<String>,
    pub grounded_correctly: bool,
    pub paraphrases_equivalent: bool,
    pub near_contrast_preserved: bool,
    pub ambiguity_resolved_by_context: bool,
    pub alias_attached: bool,
    pub semantic_duplicate_avoided: bool,
    pub semantic_duplicate_created: bool,
    pub homonym_false_merge: bool,
    pub raw_text_entered_reasoner: bool,
    pub semantic_execution_passed: bool,
    pub program_ir_created: bool,
    pub rust_compiled: bool,
    pub rust_executed: bool,
    pub proof_kernel_verified: bool,
    pub solution_dependency: bool,
    pub solved: bool,
    pub abstention_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroundingConditionReport {
    pub condition: GroundingCondition,
    pub records: Vec<GroundingRecord>,
    pub solve_rate: f64,
    pub language_to_concept_accuracy: f64,
    pub semantic_execution_rate: f64,
    pub equal_parse_budget: usize,
    pub equal_active_concept_budget: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticHashStep {
    pub concept_id: String,
    pub semantic_hash_before_language: String,
    pub semantic_hash_after_alias_attach: String,
    pub semantic_hash_after_rename: String,
    pub semantic_hash_after_second_language: String,
    pub semantic_hash_after_alias_removal: String,
    pub lexical_store_hashes_distinct: usize,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AliasOperationResult {
    pub concept_id: String,
    pub unnamed_execution_passed: bool,
    pub alias_attached: bool,
    pub renamed: bool,
    pub second_language_attached: bool,
    pub aliases_removed: bool,
    pub execution_after_each_step_passed: bool,
    pub semantic_hash_invariant: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealizationRecord {
    pub task_id: String,
    pub concept_id: String,
    pub language: Language,
    pub style: RealizationStyle,
    pub text: String,
    pub derivation_sha256: String,
    pub realized_claims: Vec<String>,
    pub unsupported_claims: usize,
    pub reparsed_semantics_match: bool,
    pub faithful: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LanguageAblation {
    pub name: String,
    pub lexical_layer_enabled: bool,
    pub semantic_substrate_enabled: bool,
    pub tasks: usize,
    pub solved: usize,
    pub solve_rate: f64,
    pub expected_direction_observed: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DuplicateConceptAudit {
    pub semantic_duplicates_avoided: usize,
    pub false_semantic_duplicate_merges: usize,
    pub duplicate_semantic_concepts_created: usize,
    pub homonym_false_merges: usize,
    pub same_surface_distinct_senses_preserved: usize,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LexicalContaminationAudit {
    pub concepts_scanned: usize,
    pub korean_token_dependencies: usize,
    pub english_token_dependencies: usize,
    pub prompt_fragment_dependencies: usize,
    pub lexical_id_semantic_conditions: usize,
    pub benchmark_sentence_dependencies: usize,
    pub lexical_token_dependent_promoted_concepts: usize,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sem7ContaminationAudit {
    pub external_llm_calls: usize,
    pub local_teacher_calls: usize,
    pub network_calls: usize,
    pub recursive_source_mutations: usize,
    pub target_answers_visible: usize,
    pub direct_text_to_program_shortcuts: usize,
    pub raw_text_reasoner_inputs: usize,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SparseLanguageAudit {
    pub total_semantic_concepts: usize,
    pub total_aliases: usize,
    pub peak_candidate_concepts: usize,
    pub peak_active_concepts: usize,
    pub full_catalog_scans: usize,
    pub routing_false_negatives: usize,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sem7FreezeRecord {
    pub run_id: String,
    pub adapter_version: String,
    pub generator_version: String,
    pub blind_manifest_sha256: String,
    pub frozen_before_final_tuning: bool,
    pub evaluator_expectations_visible_before_freeze: bool,
    pub post_blind_tuning: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sem7FinalReport {
    pub sem7_status: String,
    pub disposition: String,
    pub run_id: String,
    pub canonical_integrity: String,
    pub predecessor_integrity: String,
    pub fresh_blind_tasks: usize,
    pub korean_grounding_tasks: usize,
    pub english_grounding_tasks: usize,
    pub language_to_program_tasks: usize,
    pub language_to_math_tasks: usize,
    pub language_to_foraging_tasks: usize,
    pub language_to_goal_ir_accuracy: f64,
    pub goal_ir_reasoning_equivalence_rate: f64,
    pub concept_to_korean_faithfulness: f64,
    pub concept_to_english_faithfulness: f64,
    pub multilingual_shared_concepts: usize,
    pub opaque_relexicalization_pass: bool,
    pub unnamed_concept_operation_pass: bool,
    pub semantic_hash_invariance_pass: bool,
    pub language_ablation_pass: bool,
    pub semantic_ablation_pass: bool,
    pub unsupported_explanation_facts: usize,
    pub lexical_token_dependent_promoted_concepts: usize,
    pub external_llm_calls: usize,
    pub local_teacher_calls: usize,
    pub recursive_source_mutations: usize,
    pub full_catalog_scans: usize,
    pub routing_false_negatives: usize,
    pub language_cortex_boundary_pass: bool,
    pub semantic_language_separation_pass: bool,
    pub gates: BTreeMap<String, bool>,
    pub sem8_started: bool,
    pub next_allowed_stage: String,
}
