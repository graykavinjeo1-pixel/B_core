use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Rational {
    pub numerator: i64,
    pub denominator: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Expr {
    Rational(Rational),
    Variable(String),
    Add(Box<Expr>, Box<Expr>),
    Subtract(Box<Expr>, Box<Expr>),
    Multiply(Box<Expr>, Box<Expr>),
    Divide(Box<Expr>, Box<Expr>),
    Negate(Box<Expr>),
    Power(Box<Expr>, u32),
    Apply {
        operator_token: String,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MathObjectKind {
    Integer,
    Rational,
    Symbol,
    Variable,
    Expression,
    Equality,
    Inequality,
    Function,
    Sequence,
    Condition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MathPrimitive {
    Add,
    Subtract,
    Multiply,
    Divide,
    Negate,
    Compare,
    Substitute,
    PowerNonNegativeInteger,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MathematicalPrimitiveRecord {
    pub primitive_id: String,
    pub operation: MathPrimitive,
    pub input_domain: Vec<MathObjectKind>,
    pub output_domain: MathObjectKind,
    pub preconditions: Vec<String>,
    pub executable_semantics: String,
    pub provenance: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Equality {
    pub left: Expr,
    pub right: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "condition", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FormalCondition {
    NonZero {
        expression: Expr,
    },
    NonNegativeInteger {
        variable: String,
    },
    VariablesRational {
        variables: Vec<String>,
    },
    OperatorDomain {
        operator_token: String,
        predicate: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProofKind {
    DirectDerivation,
    Equational,
    Substitution,
    CaseAnalysis,
    MathematicalInduction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RuleCode {
    EqualityReflexivity,
    EqualitySymmetry,
    EqualityTransitivity,
    SubstituteEquals,
    AddBothSides,
    SubtractBothSides,
    MultiplyBothSides,
    DivideBothSidesNonZero,
    AdditionAssociativity,
    AdditionCommutativity,
    MultiplicationAssociativity,
    MultiplicationCommutativity,
    Distributivity,
    IdentityElements,
    AdditiveInverse,
    DefinitionExpansion,
    RationalPolynomialNormalization,
    CaseSplit,
    InductionBase,
    InductionStep,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransformationRuleRecord {
    pub rule_id: String,
    pub rule: RuleCode,
    pub formal_preconditions: Vec<String>,
    pub checkable_semantics: String,
    pub domain_restrictions: Vec<String>,
    pub provenance: Vec<String>,
    pub target_formula_encoded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorDefinition {
    pub operator_token: String,
    pub parameters: Vec<String>,
    pub body: Expr,
    pub domain_conditions: Vec<FormalCondition>,
    pub examples: Vec<Equality>,
    pub randomized_symbol: bool,
    pub provenance: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofStep {
    pub sequence: usize,
    pub rule_applied: RuleCode,
    pub source_state: Equality,
    pub result_state: Equality,
    pub witness: Option<Expr>,
    pub preconditions_checked: Vec<FormalCondition>,
    pub supporting_concepts: Vec<String>,
    pub proof_dependencies: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InductionObligation {
    pub index_variable: String,
    pub base_index: i64,
    pub recurrence_base: Expr,
    pub recurrence_delta: Expr,
    pub candidate: Expr,
    pub base_equality: Equality,
    pub successor_difference_equality: Equality,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofCertificate {
    pub certificate_id: String,
    pub proof_kind: ProofKind,
    pub assumptions: Vec<FormalCondition>,
    pub initial_statement: Equality,
    pub conclusion: Equality,
    pub steps: Vec<ProofStep>,
    pub induction: Option<InductionObligation>,
    pub kernel_verified: bool,
    pub primitive_expanded_proof_steps: usize,
    pub proof_dependencies: Vec<String>,
    pub experimental_evidence_ids: Vec<String>,
    pub formal_proof_evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MathTaskFamily {
    SymbolicEquation,
    Recurrence,
    GeneratedIdentity,
    DefinitionOnlyOperator,
    MultiConceptAdversarial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DataSplit {
    Discovery,
    Calibration,
    FreshBlind,
    DefinitionOnlyBlind,
    AdversarialBlind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "statement_kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MathStatement {
    SolveEquation {
        equation: Equality,
        solve_for: String,
    },
    DeriveRecurrenceRelation {
        index_variable: String,
        base_index: i64,
        base_value: Expr,
        delta: Expr,
    },
    DeriveEquivalentIdentity {
        expression: Expr,
    },
    ApplyDefinition {
        operator_token: String,
        arguments: Vec<Expr>,
    },
    ReuseDerivedRelation {
        index_variable: String,
        base_index: i64,
        base_value: Expr,
        delta: Expr,
        required_reasoning_layers: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MathProblem {
    pub task_id: String,
    pub split: DataSplit,
    pub statement: MathStatement,
    pub definitions: Vec<OperatorDefinition>,
    pub assumptions: Vec<FormalCondition>,
    pub zero_demonstrations: bool,
    pub provenance: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluatorMetadata {
    pub family: MathTaskFamily,
    pub adversarial: bool,
    pub invalid_case: bool,
    pub expected_applicable: bool,
    pub target_formula_stored: bool,
    pub solution_graph_depth: usize,
    pub primitive_expanded_depth: usize,
    pub concepts_composed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluatorTask {
    pub visible: MathProblem,
    #[serde(skip_serializing)]
    pub evaluator: EvaluatorMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlindManifest {
    pub run_id: String,
    pub generator_version: String,
    pub seed: u64,
    pub split: DataSplit,
    pub tasks: Vec<MathProblem>,
    pub expected_answers_included: bool,
    pub target_formulas_included: bool,
    pub proof_scripts_included: bool,
    pub human_formula_names_included: bool,
    pub reasoner_access_before_freeze: bool,
    pub manifest_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReasonerCondition {
    PrimitiveA,
    StructuralMacroB,
    SemanticNoPromotionC,
    FirstPrinciplesD,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolveRecord {
    pub task_id: String,
    pub condition: ReasonerCondition,
    pub accepted: bool,
    pub applicable: bool,
    pub invalid_transfer: bool,
    pub candidate_relation: Option<Equality>,
    pub computed_value: Option<Expr>,
    pub proof_certificate_id: Option<String>,
    pub proof_steps: usize,
    pub primitive_expanded_steps: usize,
    pub search_expansions: usize,
    pub used_concept_ids: Vec<String>,
    pub definition_examples_seen: usize,
    pub evaluator_target_formula_access: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MathMetrics {
    pub tasks: usize,
    pub solved: usize,
    pub solve_rate: f64,
    pub invalid_cases: usize,
    pub invalid_transfers: usize,
    pub invalid_transfer_rate: f64,
    pub invalid_transformations_accepted: usize,
    pub total_search_expansions: usize,
    pub median_search_expansions: f64,
    pub median_proof_steps: f64,
    pub definition_only_tasks: usize,
    pub definition_only_solved: usize,
    pub definition_only_zero_shot_solve_rate: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MathematicalCandidate {
    pub concept_id: String,
    pub domain: String,
    pub input_signature: Vec<MathObjectKind>,
    pub output_signature: MathObjectKind,
    pub preconditions: Vec<FormalCondition>,
    pub invariants: Vec<String>,
    pub derived_relation: Equality,
    pub transformation_semantics: String,
    pub proof_certificate_id: String,
    pub applicability_signature_sha256: String,
    pub derivation_lineage: Vec<String>,
    pub counterexamples: Vec<String>,
    pub operational_cost: usize,
    pub primitive_expanded_cost: usize,
    pub epistemic_depth: usize,
    pub operational_depth: usize,
    pub derived_autonomously: bool,
    pub supplied_by_teacher: bool,
    pub formula_lookup_used: bool,
    pub content_hash_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MathematicalPromotion {
    pub concept: MathematicalCandidate,
    pub formal_proof_pass: bool,
    pub executable_applicability_pass: bool,
    pub explicit_preconditions_pass: bool,
    pub fresh_blind_reuse_pass: bool,
    pub causal_ablation_pass: bool,
    pub compression_benefit_pass: bool,
    pub full_lineage_pass: bool,
    pub promoted: bool,
    pub compression_ratio: f64,
    pub postseal_human_interpretation: String,
    pub human_interpretation_attached_after_seal: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConditionReport {
    pub condition: ReasonerCondition,
    pub metrics: MathMetrics,
    pub records: Vec<SolveRecord>,
    pub new_math_promotion_enabled: bool,
    pub formula_catalog_available: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MathematicalAblation {
    pub concept_id: String,
    pub tasks: usize,
    pub with_concept_solve_rate: f64,
    pub without_concept_solve_rate: f64,
    pub solve_rate_delta: f64,
    pub with_concept_search_expansions: usize,
    pub without_concept_search_expansions: usize,
    pub search_expansion_delta: isize,
    pub with_concept_reasoning_depth: usize,
    pub without_concept_reasoning_depth: usize,
    pub reasoning_depth_delta: isize,
    pub with_concept_proof_length: usize,
    pub without_concept_proof_length: usize,
    pub proof_length_delta: isize,
    pub wall_time_proxy_delta: isize,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CounterfactualMathRecord {
    pub concept_id: String,
    pub counterfactual: String,
    pub applicability_revised: bool,
    pub prediction_revised: bool,
    pub kernel_result: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofKernelAudit {
    pub independent_from_reasoner_search: bool,
    pub solution_search_operations: usize,
    pub certificates_checked: usize,
    pub transformation_steps_checked: usize,
    pub induction_proofs_verified: usize,
    pub invalid_cancellation_rejected: bool,
    pub divide_by_zero_rejected: bool,
    pub domain_invalid_transformations_rejected: bool,
    pub type_violations_rejected: bool,
    pub hidden_assumptions_rejected: bool,
    pub invalid_transformations_accepted: usize,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormulaLeakageAudit {
    pub solver_visible_files_scanned: usize,
    pub solver_visible_literals_scanned: usize,
    pub blind_tasks_scanned: usize,
    pub target_formula_solver_leaks: usize,
    pub target_proof_scripts_exposed: usize,
    pub named_solution_templates_exposed: usize,
    pub benchmark_specific_branches: usize,
    pub hidden_formula_aliases: usize,
    pub evaluator_target_formulas_stored: usize,
    pub manual_audit_completed: bool,
    pub evaluator_isolated: bool,
    pub passed: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SparseActivationAudit {
    pub total_concepts: usize,
    pub routed_candidates: usize,
    pub peak_active_concepts: usize,
    pub full_catalog_scans: usize,
    pub routing_false_negatives: usize,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContaminationAudit {
    pub passed: bool,
    pub network_calls: usize,
    pub web_calls: usize,
    pub external_llm_calls: usize,
    pub local_teacher_calls: usize,
    pub external_cas_calls: usize,
    pub smt_solver_calls: usize,
    pub recursive_source_mutations: usize,
    pub blind_answer_reads_by_reasoner: usize,
    pub self_observe: bool,
    pub self_measure: bool,
    pub self_propose: bool,
    pub self_apply: bool,
    pub source_mutation: bool,
    pub auto_patch: bool,
    pub auto_commit: bool,
    pub auto_push: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateResult {
    pub gate: String,
    pub passed: bool,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sem4FinalReport {
    pub sem4_status: String,
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
    pub math_primitive_count: usize,
    pub transformation_rule_count: usize,
    pub fresh_blind_tasks: usize,
    pub definition_only_blind_tasks: usize,
    pub adversarial_math_tasks: usize,
    pub baseline_a_solve_rate: f64,
    pub baseline_b_solve_rate: f64,
    pub baseline_c_solve_rate: f64,
    pub first_principles_d_solve_rate: f64,
    pub definition_only_zero_shot_solve_rate: f64,
    pub autonomous_math_candidates: usize,
    pub promoted_math_concepts: usize,
    pub formally_proved_new_relations: usize,
    pub best_math_concept_id: String,
    pub best_math_concept_posthoc_interpretation: String,
    pub best_primitive_expanded_proof_steps: usize,
    pub best_compressed_operational_steps: usize,
    pub best_math_compression_ratio: f64,
    pub mathematical_ablation_pass: bool,
    pub invalid_transfer_rate: f64,
    pub invalid_transformation_accepted: usize,
    pub induction_proofs_verified: usize,
    pub recurrence_closed_forms_discovered: usize,
    pub max_solution_graph_depth: usize,
    pub max_primitive_expanded_depth: usize,
    pub max_concepts_composed: usize,
    pub peak_active_concepts: usize,
    pub target_formula_solver_leaks: usize,
    pub full_catalog_scans: usize,
    pub routing_false_negatives: usize,
    pub first_principles_derivation_pass: bool,
    pub formal_proof_pass: bool,
    pub definition_only_generalization_pass: bool,
    pub formula_leakage_audit_pass: bool,
    pub gates: Vec<GateResult>,
    pub sem5_started: bool,
    pub next_allowed_stage: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreezeRecord {
    pub run_id: String,
    pub reasoner_version: String,
    pub proof_kernel_version: String,
    pub blind_generator_version: String,
    pub blind_manifest_sha256: String,
    pub definition_only_manifest_sha256: String,
    pub adversarial_manifest_sha256: String,
    pub frozen_before_final_tuning: bool,
    pub reasoner_blind_access_before_freeze: bool,
    pub post_blind_tuning: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DerivationFamilyReport {
    pub family: MathTaskFamily,
    pub tasks: usize,
    pub solved: usize,
    pub solve_rate: f64,
    pub records: Vec<SolveRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryManifest {
    pub generator_version: String,
    pub tasks: Vec<MathProblem>,
    pub target_formulas_supplied: bool,
    pub worked_examples_supplied: bool,
    pub active_experiment_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MathematicalCandidateCatalog {
    pub candidates: Vec<MathematicalCandidate>,
    pub generated_by_formula_lookup: usize,
    pub generated_by_symbolic_derivation: usize,
    pub generated_by_numerical_fit_only: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofCertificateCatalog {
    pub certificates: Vec<ProofCertificate>,
    pub experimental_evidence_count: usize,
    pub formal_proof_evidence_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromotionCatalog {
    pub promotions: Vec<MathematicalPromotion>,
    pub promotion_gates_lowered: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveMathExperiment {
    pub experiment_id: String,
    pub candidate_id: String,
    pub selected_input: i64,
    pub competing_hypotheses: usize,
    pub hypotheses_eliminated: usize,
    pub experimental_only: bool,
    pub used_as_formal_proof: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BaselineSummary {
    pub reports: BTreeMap<String, ConditionReport>,
    pub equal_task_set: bool,
    pub equal_search_budget: bool,
}
