use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProgramType {
    Int,
    Bool,
    String,
    SequenceInt,
    NestedSequenceInt,
    Bytes,
    Image,
    Unit,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Effect {
    Pure,
    LocalMutation,
    BufferMutation,
    SandboxFileRead,
    SandboxFileWrite,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "value_kind",
    content = "value",
    rename_all = "SCREAMING_SNAKE_CASE"
)]
pub enum Value {
    Int(i64),
    Bool(bool),
    String(String),
    Sequence(Vec<i64>),
    NestedSequence(Vec<Vec<i64>>),
    Bytes(Vec<u8>),
    Image(ImageValue),
    Unit,
}

impl Value {
    pub fn program_type(&self) -> ProgramType {
        match self {
            Self::Int(_) => ProgramType::Int,
            Self::Bool(_) => ProgramType::Bool,
            Self::String(_) => ProgramType::String,
            Self::Sequence(_) => ProgramType::SequenceInt,
            Self::NestedSequence(_) => ProgramType::NestedSequenceInt,
            Self::Bytes(_) => ProgramType::Bytes,
            Self::Image(_) => ProgramType::Image,
            Self::Unit => ProgramType::Unit,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageValue {
    pub width: usize,
    pub height: usize,
    pub channels: usize,
    pub pixels: Vec<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UnaryOperator {
    Negate,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StringTransformOperator {
    Trim,
    Lowercase,
    Uppercase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeMeta {
    pub node_id: String,
    pub input_types: Vec<ProgramType>,
    pub output_type: ProgramType,
    pub preconditions: Vec<String>,
    pub effects: Vec<Effect>,
    pub data_dependencies: Vec<String>,
    pub control_dependencies: Vec<String>,
    pub provenance: Vec<String>,
    pub primitive_cost: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramNode {
    pub meta: NodeMeta,
    pub kind: NodeKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "node_kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NodeKind {
    Literal {
        value: Value,
    },
    Variable {
        name: String,
    },
    Load {
        name: String,
    },
    Store {
        name: String,
        value: Box<ProgramNode>,
    },
    UnaryOp {
        operator: UnaryOperator,
        input: Box<ProgramNode>,
    },
    StringTransform {
        operator: StringTransformOperator,
        input: Box<ProgramNode>,
    },
    BinaryOp {
        operator: BinaryOperator,
        left: Box<ProgramNode>,
        right: Box<ProgramNode>,
    },
    SequenceCreate {
        elements: Vec<ProgramNode>,
    },
    SequenceRead {
        sequence: Box<ProgramNode>,
        index: Box<ProgramNode>,
    },
    SequenceLength {
        sequence: Box<ProgramNode>,
    },
    SequenceWrite {
        binding: String,
        index: Box<ProgramNode>,
        value: Box<ProgramNode>,
    },
    SequenceAppend {
        binding: String,
        value: Box<ProgramNode>,
    },
    If {
        condition: Box<ProgramNode>,
        then_node: Box<ProgramNode>,
        else_node: Box<ProgramNode>,
    },
    Loop {
        source: Box<ProgramNode>,
        item_binding: String,
        index_binding: String,
        body: Box<ProgramNode>,
    },
    Call {
        api_token: String,
        args: Vec<ProgramNode>,
    },
    Return {
        value: Box<ProgramNode>,
    },
    Block {
        nodes: Vec<ProgramNode>,
    },
    Break,
    Continue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindingSpec {
    pub name: String,
    pub value_type: ProgramType,
    pub mutable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramIR {
    pub program_id: String,
    pub inputs: Vec<BindingSpec>,
    pub output_type: ProgramType,
    pub allowed_effects: Vec<Effect>,
    pub root: ProgramNode,
    pub concept_ids: Vec<String>,
    pub graph_edges: Vec<[String; 2]>,
    pub provenance: Vec<String>,
    pub primitive_expanded_nodes: usize,
    pub operational_nodes: usize,
    pub solution_graph_depth: usize,
    pub primitive_expanded_depth: usize,
    pub search_trajectory_depth: usize,
    pub simultaneous_subproblems: usize,
    pub recombinations: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "expression_kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScalarExpression {
    Argument {
        index: usize,
    },
    Constant {
        value: i64,
    },
    BoolConstant {
        value: bool,
    },
    StringConstant {
        value: String,
    },
    Unary {
        operator: UnaryOperator,
        input: Box<ScalarExpression>,
    },
    StringTransform {
        operator: StringTransformOperator,
        input: Box<ScalarExpression>,
    },
    Binary {
        operator: BinaryOperator,
        left: Box<ScalarExpression>,
        right: Box<ScalarExpression>,
    },
    Length {
        input: Box<ScalarExpression>,
    },
    Index {
        collection: Box<ScalarExpression>,
        index: Box<ScalarExpression>,
    },
    OpaqueCall {
        api_token: String,
        args: Vec<ScalarExpression>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiDefinition {
    pub api_token: String,
    pub inputs: Vec<ProgramType>,
    pub output: ProgramType,
    pub effect: Effect,
    pub preconditions: Vec<String>,
    pub postconditions: Vec<String>,
    pub formal_body: ScalarExpression,
    pub examples: Vec<Vec<Value>>,
    pub randomized_symbol: bool,
    pub provenance: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "relation_kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RelationSpec {
    Scalar {
        expression: ScalarExpression,
    },
    /// A name-independent typed mechanism lowered from concrete source
    /// operands.  The condition and both postimages remain explicit so the
    /// generated syntax can be falsified without selecting a task-specific
    /// answer template.
    Mechanism {
        condition: Option<ScalarExpression>,
        postimage: ScalarExpression,
        otherwise: Option<ScalarExpression>,
    },
    Collection {
        expression: ScalarExpression,
        include_when: Option<ScalarExpression>,
    },
    Stateful {
        initial: i64,
        update: ScalarExpression,
        reset_when: Option<ScalarExpression>,
        emit_each: bool,
    },
    Nested {
        expression: ScalarExpression,
        include_when: Option<ScalarExpression>,
    },
    Buffer {
        expression: ScalarExpression,
        write_output: bool,
    },
    Image {
        expression: ScalarExpression,
        apply_when: Option<ScalarExpression>,
    },
    Composition {
        stages: Vec<RelationSpec>,
    },
    OpaqueUse {
        api_token: String,
        arguments: Vec<ScalarExpression>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DataSplit {
    Discovery,
    Calibration,
    FreshBlind,
    OpaqueApiBlind,
    AdversarialBlind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramTask {
    pub task_id: String,
    pub split: DataSplit,
    pub inputs: Vec<BindingSpec>,
    pub output_type: ProgramType,
    pub relation: RelationSpec,
    pub definitions: Vec<ApiDefinition>,
    pub allowed_effects: Vec<Effect>,
    pub preconditions: Vec<String>,
    pub postconditions: Vec<String>,
    pub invariants: Vec<String>,
    pub demonstrations: Vec<Vec<Value>>,
    pub provenance: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProgramTaskFamily {
    ScalarBasic,
    Sequence,
    NestedSequence,
    Stateful,
    FileTransform,
    ImageTransform,
    MultiStage,
    OpaqueApi,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluatorMetadata {
    pub family: ProgramTaskFamily,
    pub adversarial: bool,
    pub hidden_cases: Vec<BTreeMap<String, Value>>,
    pub invalid_cases: Vec<BTreeMap<String, Value>>,
    pub expected_effects: Vec<Effect>,
    pub solution_graph_depth: usize,
    pub primitive_expanded_depth: usize,
    pub concepts_composed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluatorTask {
    pub visible: ProgramTask,
    pub evaluator: EvaluatorMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskManifest {
    pub run_id: String,
    pub generator_version: String,
    pub seed_commitment_sha256: String,
    pub split: DataSplit,
    pub tasks: Vec<ProgramTask>,
    pub expected_outputs_included: bool,
    pub family_metadata_included: bool,
    pub reference_source_included: bool,
    pub manifest_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SynthesisCondition {
    PrimitiveA,
    StructuralB,
    SemanticNoPromotionC,
    FirstPrinciplesD,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynthesisRecord {
    pub task_id: String,
    pub condition: SynthesisCondition,
    pub program_id: Option<String>,
    pub program_ir_valid: bool,
    pub rust_compiled: bool,
    pub runtime_valid: bool,
    pub visible_outputs_match: bool,
    pub property_tests_passed: usize,
    pub property_tests_total: usize,
    pub invalid_inputs_handled: bool,
    pub forbidden_effect_accepted: bool,
    pub solved: bool,
    pub search_nodes_expanded: usize,
    pub search_frontier_peak: usize,
    pub used_concept_ids: Vec<String>,
    pub primitive_expanded_ir_nodes: usize,
    pub operational_nodes: usize,
    pub first_attempt_correct: bool,
    pub repair_attempts: usize,
    pub successful_repairs: usize,
    pub emitted_source_sha256: Option<String>,
    pub compiler_stdout: String,
    pub compiler_stderr: String,
    pub runtime_stdout: String,
    pub runtime_stderr: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConditionReport {
    pub condition: SynthesisCondition,
    pub records: Vec<SynthesisRecord>,
    pub solve_rate: f64,
    pub program_ir_valid_rate: f64,
    pub rust_compile_rate: f64,
    pub runtime_valid_rate: f64,
    pub property_generalization_pass_rate: f64,
    pub mean_search_nodes: f64,
    pub equal_search_budget: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgrammingPrimitiveRecord {
    pub primitive_id: String,
    pub node_kind: String,
    pub input_types: Vec<ProgramType>,
    pub output_type: ProgramType,
    pub effects: Vec<Effect>,
    pub executable_semantics: String,
    pub provenance: Vec<String>,
    pub high_level_algorithm_seeded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramIrSpec {
    pub version: String,
    pub authoritative_representation: String,
    pub node_types: Vec<String>,
    pub explicit_types: Vec<ProgramType>,
    pub explicit_effects: Vec<Effect>,
    pub carries_dependencies: bool,
    pub carries_provenance: bool,
    pub rust_is_adapter_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustMinAllowlist {
    pub version: String,
    pub allowed: Vec<String>,
    pub forbidden: Vec<String>,
    pub offline_compilation: bool,
    pub external_crates: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxAudit {
    pub isolated_temporary_workspace: bool,
    pub network_disabled_by_construction: bool,
    pub host_mutation_prohibited: bool,
    pub execution_timeout_ms: u64,
    pub output_limit_bytes: usize,
    pub memory_limit_practical: bool,
    pub arbitrary_paths_rejected: bool,
    pub unsafe_rejected: bool,
    pub external_dependencies: usize,
    pub programs_compiled: usize,
    pub programs_executed: usize,
    pub containment_violations: usize,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgramConcept {
    pub concept_id: String,
    pub generation: usize,
    pub parent_ids: Vec<String>,
    pub semantic_signature: String,
    pub input_types: Vec<ProgramType>,
    pub output_type: ProgramType,
    pub effects: Vec<Effect>,
    pub reusable_ir_fragment: String,
    pub provenance: Vec<String>,
    pub discovery_evidence_ids: Vec<String>,
    pub human_name_revealed_post_seal: Option<String>,
    pub rust_tokens_in_definition: usize,
    pub identity_wrapper: bool,
    pub primitive_expanded_nodes: usize,
    pub operational_nodes: usize,
    pub compression_ratio: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgrammingPromotion {
    pub concept: ProgramConcept,
    pub proposed_autonomously: bool,
    pub semantic_consistency_pass: bool,
    pub compression_pass: bool,
    pub discovery_reuse_pass: bool,
    pub calibration_pass: bool,
    pub fresh_blind_reuse_pass: bool,
    pub causal_ablation_pass: bool,
    pub cross_instance_pass: bool,
    pub language_separation_pass: bool,
    pub generation_parent_pass: bool,
    pub promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AblationRecord {
    pub concept_id: String,
    pub ancestor_ablation: bool,
    pub full_solve_rate: f64,
    pub ablated_solve_rate: f64,
    pub full_mean_search_nodes: f64,
    pub ablated_mean_search_nodes: f64,
    pub lost_solutions: usize,
    pub search_cost_increase: f64,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferRecord {
    pub concept_id: String,
    pub discovery_domain: String,
    pub transfer_domain: String,
    pub task_ids: Vec<String>,
    pub semantic_compatibility_key: String,
    pub selected_by_family_label: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CounterfactualRecord {
    pub concept_id: String,
    pub perturbation: String,
    pub predicted_change: String,
    pub observed_change: String,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetLeakageAudit {
    pub reference_implementations_in_solver: usize,
    pub target_algorithm_names_in_solver: usize,
    pub expected_source_programs_in_solver: usize,
    pub fixture_specific_branches: usize,
    pub task_id_dispatch_branches: usize,
    pub stable_opaque_api_meanings: usize,
    pub hidden_answer_lookups: usize,
    pub target_program_solver_leaks: usize,
    pub audited_files: Vec<String>,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageSeparationAudit {
    pub promoted_concepts: usize,
    pub rust_token_dependent_promoted_concepts: usize,
    pub second_textual_representation_checks: usize,
    pub second_representation_failures: usize,
    pub rust_specific_api_concepts: usize,
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
pub struct ContaminationAudit {
    pub network_calls: usize,
    pub external_llm_calls: usize,
    pub local_teacher_calls: usize,
    pub recursive_source_mutations: usize,
    pub blind_answer_reads_by_solver: usize,
    pub self_observe: bool,
    pub self_measure: bool,
    pub self_propose: bool,
    pub self_apply: bool,
    pub auto_commit: bool,
    pub auto_push: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreezeRecord {
    pub run_id: String,
    pub synthesizer_version: String,
    pub ir_version: String,
    pub emitter_version: String,
    pub sandbox_version: String,
    pub blind_generator_version: String,
    pub blind_manifest_sha256: String,
    pub opaque_api_manifest_sha256: String,
    pub adversarial_manifest_sha256: String,
    pub frozen_before_final_tuning: bool,
    pub solver_blind_access_before_freeze: bool,
    pub post_blind_tuning: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityFrontier {
    pub all_conditions_at_ceiling: bool,
    pub primary_comparison: String,
    pub solve_rate_delta_d_minus_c: f64,
    pub search_cost_reduction_d_vs_c: f64,
    pub frontier_evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sem5FinalReport {
    pub sem5_status: String,
    pub disposition: String,
    pub run_id: String,
    pub canonical_integrity: String,
    pub predecessor_integrity: String,
    pub programming_primitive_count: usize,
    pub program_ir_node_types: usize,
    pub fresh_blind_tasks: usize,
    pub opaque_api_blind_tasks: usize,
    pub adversarial_programming_tasks: usize,
    pub baseline_a_solve_rate: f64,
    pub baseline_b_solve_rate: f64,
    pub baseline_c_solve_rate: f64,
    pub full_d_solve_rate: f64,
    pub program_ir_valid_rate: f64,
    pub rust_compile_rate: f64,
    pub runtime_valid_rate: f64,
    pub property_generalization_pass_rate: f64,
    pub definition_only_api_zero_shot_solve_rate: f64,
    pub autonomous_program_candidates: usize,
    pub promoted_program_concepts: usize,
    pub best_program_concept_id: String,
    pub best_program_concept_posthoc_interpretation: String,
    pub gen3_candidates: usize,
    pub gen3_promoted: usize,
    pub gen4_candidates: usize,
    pub gen4_promoted: usize,
    pub max_autonomous_concept_generation: usize,
    pub cross_domain_concept_transfer_count: usize,
    pub predecessor_concept_reuse_count: usize,
    pub programming_ablation_pass: bool,
    pub ancestor_ablation_pass: bool,
    pub best_primitive_expanded_ir_nodes: usize,
    pub best_compressed_operational_nodes: usize,
    pub best_program_compression_ratio: f64,
    pub max_solution_graph_depth: usize,
    pub max_primitive_expanded_depth: usize,
    pub max_search_trajectory_depth: usize,
    pub max_concepts_composed: usize,
    pub max_simultaneous_subproblems: usize,
    pub max_recombinations: usize,
    pub peak_active_concepts: usize,
    pub first_attempt_correct_programs: usize,
    pub repair_attempts: usize,
    pub successful_repairs: usize,
    pub invalid_effect_accepted: usize,
    pub target_program_solver_leaks: usize,
    pub rust_token_dependent_promoted_concepts: usize,
    pub full_catalog_scans: usize,
    pub routing_false_negatives: usize,
    pub gates: BTreeMap<String, bool>,
    pub sem6_started: bool,
    pub next_allowed_stage: String,
}
