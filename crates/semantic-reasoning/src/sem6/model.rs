use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::sem5::model::{ProgramType, ScalarExpression, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QueryCategory {
    DefineSymbol,
    DefineTerm,
    GetTypeSignature,
    GetApiContract,
    GetPreconditions,
    GetPostconditions,
    GetStandardSemantics,
    GetFormalRule,
    GetDataFormatSpec,
    GetProtocolFieldMeaning,
    GetSolution,
    GetWorkedExampleForActiveTask,
    GetReferenceImplementation,
    GetTargetFormula,
    GetAnswer,
    GetBenchmarkPatch,
    SearchExactActiveProblem,
    SearchErrorPlusTargetTaskForSolution,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GapClass {
    UnknownSymbol,
    UnknownType,
    UnknownRelation,
    UnknownOperator,
    UnknownApi,
    UnknownProtocol,
    UnknownDataFormat,
    UnknownDomainConstraint,
    AmbiguousDefinition,
    ConflictingDefinitions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SourceAuthority {
    OfficialStandard,
    OfficialDocumentation,
    OriginalPaper,
    InstitutionalReference,
    SecondarySource,
    Untrusted,
}

impl SourceAuthority {
    pub const fn rank(self) -> usize {
        match self {
            Self::OfficialStandard => 6,
            Self::OfficialDocumentation => 5,
            Self::OriginalPaper => 4,
            Self::InstitutionalReference => 3,
            Self::SecondarySource => 2,
            Self::Untrusted => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SpanClass {
    Definition,
    Signature,
    Precondition,
    Postcondition,
    NormativeRule,
    Example,
    Implementation,
    SolutionLike,
    Commentary,
    Unknown,
}

impl SpanClass {
    pub const fn importable(self) -> bool {
        matches!(
            self,
            Self::Definition
                | Self::Signature
                | Self::Precondition
                | Self::Postcondition
                | Self::NormativeRule
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KnowledgeDomain {
    ProgrammingApi,
    MathematicalFormal,
    ProtocolSpecification,
    AmbiguousConflict,
    AdversarialContamination,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ForagingEnvironment {
    SealedCorpusA,
    ControlledLiveB,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RetrievalCondition {
    NoForagingA,
    KeywordRetrievalB,
    SemanticGapRetrievalC,
    FullDefinitionForagingD,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticFactPayload {
    pub symbol: String,
    pub signature_inputs: Vec<ProgramType>,
    pub signature_output: ProgramType,
    pub formal_body: ScalarExpression,
    pub preconditions: Vec<String>,
    pub postconditions: Vec<String>,
    pub invariants: Vec<String>,
    pub effects: Vec<String>,
    pub scope: String,
    pub source_version: String,
    pub applicability_version_range: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan {
    pub span_id: String,
    pub class: SpanClass,
    pub text: String,
    pub fact: Option<SemanticFactPayload>,
    pub injection_like: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceDocument {
    pub source_id: String,
    pub title: String,
    pub source_identifier: String,
    pub url: Option<String>,
    pub authority: SourceAuthority,
    pub source_version: String,
    pub scope: String,
    pub retrieval_time_utc: String,
    pub retrieved_bytes: usize,
    pub content_sha256: String,
    pub live_retrieval: bool,
    pub search_snippet_only: bool,
    pub spans: Vec<SourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibleKnowledgeTask {
    pub task_id: String,
    pub environment: ForagingEnvironment,
    pub domain: KnowledgeDomain,
    pub active_problem: String,
    pub active_problem_sha256: String,
    pub unknown_symbol: String,
    pub required_version: String,
    pub required_scope: String,
    pub input_types: Vec<ProgramType>,
    pub output_type: ProgramType,
    pub demonstrations: Vec<Vec<Value>>,
    pub target_solution_included: bool,
    pub intent_frozen: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeEvaluatorTask {
    pub visible: VisibleKnowledgeTask,
    pub expected_fact_id: String,
    pub relevant_source_ids: Vec<String>,
    pub hidden_cases: Vec<Vec<Value>>,
    pub ambiguity_requires_multiple_sources: bool,
    pub contamination_canary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeTaskManifest {
    pub run_id: String,
    pub generator_version: String,
    pub environment: ForagingEnvironment,
    pub seed_commitment_sha256: String,
    pub tasks: Vec<VisibleKnowledgeTask>,
    pub expected_facts_included: bool,
    pub relevant_source_ids_included: bool,
    pub hidden_cases_included: bool,
    pub target_solutions_included: bool,
    pub manifest_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeGapEvent {
    pub event_id: String,
    pub task_id: String,
    pub gap_class: GapClass,
    pub unknown: String,
    pub existing_concepts_insufficient_because: String,
    pub minimum_information_needed: String,
    pub external_retrieval_necessary: bool,
    pub confidence: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForagingRequest {
    pub request_id: String,
    pub task_id: String,
    pub category: QueryCategory,
    pub query: String,
    pub requested_symbol: String,
    pub requested_scope: String,
    pub requested_version: String,
    pub classification_allowed: bool,
    pub exact_task_leak: bool,
    pub near_task_similarity: f64,
    pub sanitized: bool,
    pub executed: bool,
    pub rejection_reason: Option<String>,
    pub request_budget: usize,
    pub content_budget_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractionRecord {
    pub task_id: String,
    pub source_id: String,
    pub spans_seen: usize,
    pub definition_spans_accepted: usize,
    pub facts_extracted: usize,
    pub facts_accepted: usize,
    pub facts_rejected: usize,
    pub example_spans_quarantined: usize,
    pub implementation_spans_quarantined: usize,
    pub solution_like_spans_quarantined: usize,
    pub injection_like_spans_detected: usize,
    pub control_instructions_executed: usize,
    pub accepted_fact_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledSemanticFact {
    pub fact_id: String,
    pub opaque_concept_id: String,
    pub lexical_alias: String,
    pub signature_inputs: Vec<ProgramType>,
    pub signature_output: ProgramType,
    pub formal_body: ScalarExpression,
    pub preconditions: Vec<String>,
    pub postconditions: Vec<String>,
    pub invariants: Vec<String>,
    pub effects: Vec<String>,
    pub source_ids: Vec<String>,
    pub source_versions: Vec<String>,
    pub scope: String,
    pub applicability_version_range: String,
    pub confidence: f64,
    pub agreement_count: usize,
    pub conflict: bool,
    pub type_check_passed: bool,
    pub generated_probe_count: usize,
    pub generated_probes_passed: usize,
    pub normative_consistency_passed: bool,
    pub validation_passed: bool,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceConflict {
    pub conflict_id: String,
    pub symbol: String,
    pub source_ids: Vec<String>,
    pub disagreement: String,
    pub authority_compared: bool,
    pub versions_compared: bool,
    pub scopes_compared: bool,
    pub resolution: String,
    pub resolved: bool,
    pub unresolved_hypotheses_preserved: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskForagingResult {
    pub task_id: String,
    pub environment: ForagingEnvironment,
    pub domain: KnowledgeDomain,
    pub condition: RetrievalCondition,
    pub gap_detected: bool,
    pub request_ids: Vec<String>,
    pub source_ids_retrieved: Vec<String>,
    pub compiled_fact_ids: Vec<String>,
    pub solved: bool,
    pub zero_demonstrations: bool,
    pub semantic_extraction_correct: bool,
    pub solution_dependency: bool,
    pub false_semantic_imports: usize,
    pub semantic_facts_accepted: usize,
    pub semantic_facts_rejected: usize,
    pub retrieved_bytes: usize,
    pub queries_issued: usize,
    pub rust_program_ir_valid: bool,
    pub rust_compiled: bool,
    pub rust_runtime_valid: bool,
    pub hidden_property_cases_passed: usize,
    pub hidden_property_cases_total: usize,
    pub proof_kernel_verified: bool,
    pub stop_condition: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalConditionReport {
    pub environment: ForagingEnvironment,
    pub condition: RetrievalCondition,
    pub task_results: Vec<TaskForagingResult>,
    pub solve_rate: f64,
    pub semantic_extraction_accuracy: f64,
    pub false_semantic_import_rate: f64,
    pub queries_issued: usize,
    pub documents_retrieved: usize,
    pub retrieved_bytes: usize,
    pub equal_request_budget_per_task: usize,
    pub equal_content_budget_bytes_per_task: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalConceptCandidate {
    pub concept_id: String,
    pub generation: usize,
    pub parent_ids: Vec<String>,
    pub opaque_external_fact_ids: Vec<String>,
    pub semantic_signature: String,
    pub reusable_behavior: String,
    pub discovery_domains: Vec<KnowledgeDomain>,
    pub provenance: Vec<String>,
    pub identity_wrapper: bool,
    pub external_prose_is_authority: bool,
    pub compression_ratio: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalConceptPromotion {
    pub candidate: ExternalConceptCandidate,
    pub source_provenance_pass: bool,
    pub semantic_compilation_pass: bool,
    pub internal_consistency_pass: bool,
    pub counterfactual_validation_pass: bool,
    pub fresh_reuse_pass: bool,
    pub scope_version_validity_pass: bool,
    pub causal_utility_pass: bool,
    pub full_lineage_pass: bool,
    pub promoted: bool,
    pub postseal_human_interpretation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsolidationRecord {
    pub event_id: String,
    pub concept_id: String,
    pub prior_state: String,
    pub new_state: String,
    pub linked_existing_concept_ids: Vec<String>,
    pub lexical_aliases: Vec<String>,
    pub version_scope: String,
    pub source_ids: Vec<String>,
    pub existing_concepts_overwritten: usize,
    pub versioned_change: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalEfficiency {
    pub queries_issued: usize,
    pub documents_retrieved: usize,
    pub bytes_retrieved: usize,
    pub authoritative_documents_used: usize,
    pub semantic_facts_extracted: usize,
    pub semantic_facts_accepted: usize,
    pub semantic_facts_rejected: usize,
    pub tasks_solved: usize,
    pub tasks_solved_per_query: f64,
    pub useful_concepts_per_retrieved_kb: f64,
    pub knowledge_gain_per_retrieved_kb: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalAblation {
    pub ablation: String,
    pub sealed_corpus_only: bool,
    pub solve_rate: f64,
    pub contamination_events: usize,
    pub false_import_rate: f64,
    pub retained_concepts: usize,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossDomainTransfer {
    pub concept_id: String,
    pub source_domain: KnowledgeDomain,
    pub target_domain: KnowledgeDomain,
    pub task_ids: Vec<String>,
    pub selected_by_semantic_compatibility: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanaryAudit {
    pub planted_solution_documents: usize,
    pub planted_solution_spans: usize,
    pub solution_like_spans_quarantined: usize,
    pub planted_solutions_reaching_solver: usize,
    pub solution_contamination_events: usize,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalInstructionAudit {
    pub injection_like_spans_detected: usize,
    pub control_instructions_executed: usize,
    pub remote_commands_executed: usize,
    pub uploads_performed: usize,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkSecurityAudit {
    pub network_access: bool,
    pub unrestricted_web_access: bool,
    pub solution_foraging: bool,
    pub definition_foraging: bool,
    pub http_read_only: bool,
    pub network_read_requests: usize,
    pub network_write_requests: usize,
    pub remote_executions: usize,
    pub authenticated_account_mutations: usize,
    pub download_executions: usize,
    pub search_snippets_used_as_authority: usize,
    pub retrieved_code_executed: usize,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SparseActivationAudit {
    pub total_concepts: usize,
    pub peak_active_concepts: usize,
    pub full_catalog_scans: usize,
    pub routing_false_negatives: usize,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreezeRecord {
    pub run_id: String,
    pub system_version: String,
    pub corpus_generator_version: String,
    pub sem6a_manifest_sha256: String,
    pub sem6b_intent_manifest_sha256: String,
    pub live_source_snapshot_sha256: String,
    pub task_intent_frozen_before_live_retrieval: bool,
    pub frozen_before_final_tuning: bool,
    pub post_blind_tuning: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sem6FinalReport {
    pub sem6_status: String,
    pub disposition: String,
    pub run_id: String,
    pub canonical_integrity: String,
    pub predecessor_integrity: String,
    pub sem6a_status: String,
    pub sem6b_status: String,
    pub network_read_requests: usize,
    pub network_write_requests: usize,
    pub remote_executions: usize,
    pub sealed_corpus_blind_tasks: usize,
    pub live_foraging_blind_tasks: usize,
    pub baseline_a_solve_rate: f64,
    pub baseline_b_solve_rate: f64,
    pub baseline_c_solve_rate: f64,
    pub full_d_solve_rate: f64,
    pub sealed_corpus_definition_zero_shot_solve_rate: f64,
    pub live_foraging_definition_zero_shot_solve_rate: f64,
    pub knowledge_gaps_detected: usize,
    pub foraging_requests_proposed: usize,
    pub foraging_requests_executed: usize,
    pub unnecessary_foraging_rate: f64,
    pub missed_necessary_foraging_rate: f64,
    pub documents_retrieved: usize,
    pub authoritative_documents_used: usize,
    pub semantic_facts_extracted: usize,
    pub semantic_facts_accepted: usize,
    pub semantic_facts_rejected: usize,
    pub external_concept_candidates: usize,
    pub external_concepts_promoted: usize,
    pub cross_domain_foraged_concept_transfer_count: usize,
    pub source_conflicts_detected: usize,
    pub source_conflicts_resolved: usize,
    pub unresolved_source_conflicts: usize,
    pub solution_like_spans_quarantined: usize,
    pub solution_contamination_events: usize,
    pub false_semantic_import_rate: f64,
    pub external_solution_dependencies: usize,
    pub external_document_control_instructions_detected: usize,
    pub external_document_control_instructions_executed: usize,
    pub gen5_candidates: usize,
    pub gen5_promoted: usize,
    pub max_autonomous_concept_generation: usize,
    pub retrieval_bytes_or_tokens: usize,
    pub tasks_solved_per_query: f64,
    pub full_catalog_scans: usize,
    pub routing_false_negatives: usize,
    pub gates: BTreeMap<String, bool>,
    pub recursive_source_mutations: usize,
    pub self_observe: bool,
    pub self_measure: bool,
    pub self_propose: bool,
    pub self_apply: bool,
    pub source_mutation: bool,
    pub auto_patch: bool,
    pub auto_commit: bool,
    pub auto_push: bool,
    pub sem7_started: bool,
    pub next_allowed_stage: String,
}
