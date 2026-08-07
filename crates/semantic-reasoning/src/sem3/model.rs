use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UncertaintyKind {
    UncertainPrecondition,
    UncertainInvariant,
    UncertainRelation,
    UncertainOperatorDomain,
    UncertainCounterfactual,
    CompetingAbstractions,
    AmbiguousConceptBoundary,
    FailedTransfer,
    LowConfidencePrediction,
    UnexplainedEpisode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HypothesisRule {
    NonNegative,
    NonZero,
    Universal,
}

impl HypothesisRule {
    pub fn predict(self, value: i64, counterfactual: bool) -> bool {
        let base = match self {
            Self::NonNegative => value >= 0,
            Self::NonZero => value != 0,
            Self::Universal => true,
        };
        if counterfactual {
            !base
        } else {
            base
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetingHypothesis {
    pub hypothesis_id: String,
    pub rule: HypothesisRule,
    pub confidence: f64,
    pub supporting_evidence: Vec<String>,
    pub contradicting_evidence: Vec<String>,
    pub retained: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UncertaintyItem {
    pub uncertainty_id: String,
    pub kind: UncertaintyKind,
    pub affected_concepts: Vec<String>,
    pub relation_code: String,
    pub competing_hypotheses: Vec<CompetingHypothesis>,
    pub supporting_evidence: Vec<String>,
    pub contradicting_evidence: Vec<String>,
    pub confidence: f64,
    pub expected_consequences_if_resolved: Vec<String>,
    pub provenance: Vec<String>,
    pub resolved: bool,
    pub resolved_hypothesis_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UncertaintyLedger {
    pub ledger_version: String,
    pub items: Vec<UncertaintyItem>,
    pub append_only_revision_ids: Vec<String>,
    pub fabricated_uncertainty_count: usize,
}

impl UncertaintyLedger {
    pub fn unresolved_count(&self) -> usize {
        self.items.iter().filter(|item| !item.resolved).count()
    }

    pub fn resolved_count(&self) -> usize {
        self.items.iter().filter(|item| item.resolved).count()
    }

    pub fn retained_hypothesis_count(&self) -> usize {
        self.items
            .iter()
            .flat_map(|item| &item.competing_hypotheses)
            .filter(|hypothesis| hypothesis.retained)
            .count()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SelectorCondition {
    RandomA,
    NoveltyB,
    FixedCurriculumC,
    UncertaintyOnlyD,
    ActiveSemanticE,
    EMinusInformationGain,
    EMinusFrontier,
    EMinusAbstractionValue,
    EMinusCounterfactuals,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CompetenceClass {
    Mastered,
    Frontier,
    CurrentlyUnsolved,
    OutOfDomain,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperimentQuery {
    pub experiment_id: String,
    pub parent_uncertainty_id: String,
    pub generating_concept_ids: Vec<String>,
    pub value: i64,
    pub counterfactual: bool,
    pub sequence_shape: usize,
    pub composition_arity: usize,
    pub operator_substitution_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentScore {
    pub expected_information_gain: f64,
    pub expected_uncertainty_reduction: f64,
    pub discriminative_hypothesis_value: f64,
    pub concept_boundary_clarification: f64,
    pub expected_transfer_value: f64,
    pub expected_reusable_abstraction_value: f64,
    pub competence_frontier_value: f64,
    pub execution_cost: f64,
    pub redundancy: f64,
    pub triviality: f64,
    pub invalid_experiment_probability: f64,
    pub total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateExperiment {
    pub query: ExperimentQuery,
    pub candidate_hypothesis_ids: Vec<String>,
    pub predicted_outcomes: BTreeMap<String, bool>,
    pub competence_class: CompetenceClass,
    pub valid_in_closed_world: bool,
    pub surface_signature: String,
    pub semantic_signature: String,
    pub duplicate_of: Option<String>,
    pub near_duplicate_of: Option<String>,
    pub score: ExperimentScore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentObservation {
    pub experiment_id: String,
    pub applicable: bool,
    pub output_class: String,
    pub execution_cost: usize,
    pub environment_rule_exposed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentSelectionRecord {
    pub sequence: usize,
    pub condition: SelectorCondition,
    pub selected_experiment_id: String,
    pub parent_uncertainty_id: String,
    pub candidate_count: usize,
    pub predicted_outcomes: BTreeMap<String, bool>,
    pub score: ExperimentScore,
    pub structured_explanation: BTreeMap<String, String>,
    pub observation: EnvironmentObservation,
    pub hypotheses_eliminated: usize,
    pub realized_information_gain: f64,
    pub uncertainty_resolved: bool,
    pub influenced_model_revision: bool,
    pub influenced_concept_promotion: bool,
    pub provenance: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSurpriseEvent {
    pub surprise_id: String,
    pub experiment_id: String,
    pub uncertainty_id: String,
    pub predicted_majority_outcome: bool,
    pub actual_outcome: bool,
    pub diagnosis: String,
    pub prior_valid_concepts_mutated: bool,
    pub created_revision_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRevision {
    pub revision_id: String,
    pub uncertainty_id: String,
    pub eliminated_hypotheses: Vec<String>,
    pub retained_hypotheses: Vec<String>,
    pub resolved_hypothesis_id: Option<String>,
    pub evidence_experiment_id: String,
    pub existing_promoted_concepts_mutated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibleBlindTask {
    pub task_id: String,
    pub concept_id: String,
    pub relation_code: String,
    pub value: i64,
    pub counterfactual: bool,
    pub sequence_shape: usize,
    pub composition_arity: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindEvaluatorMetadata {
    pub uncertainty_id: String,
    pub expected_applicable: bool,
    pub family_label: String,
    pub boundary_case: bool,
    pub transfer_case: bool,
    pub solution_graph_depth: usize,
    pub primitive_expanded_depth: usize,
    pub simultaneous_subproblems: usize,
    pub recombinations: usize,
    pub semantic_traps: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalBlindTask {
    pub visible: VisibleBlindTask,
    #[serde(skip_serializing)]
    pub evaluator: BlindEvaluatorMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrozenBlindManifest {
    pub generator_version: String,
    pub seed: u64,
    pub tasks: Vec<VisibleBlindTask>,
    pub expected_answers_included: bool,
    pub hidden_family_labels_included: bool,
    pub intended_concepts_included: bool,
    pub difficulty_classification_included: bool,
    pub selector_access_before_or_during_curriculum: bool,
    pub manifest_sha256: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BlindMetrics {
    pub tasks: usize,
    pub strictly_solved: usize,
    pub solve_rate: f64,
    pub counterfactual_accuracy: f64,
    pub false_transfers: usize,
    pub false_transfer_rate: f64,
    pub false_rejections: usize,
    pub false_rejection_rate: f64,
    pub invalid_cases: usize,
    pub invalid_abstentions: usize,
    pub invalid_abstention_rate: f64,
    pub total_search_expansions: usize,
    pub median_search_expansions: f64,
    pub max_solution_graph_depth: usize,
    pub max_primitive_expanded_depth: usize,
    pub max_concepts_composed: usize,
    pub max_simultaneous_subproblems: usize,
    pub max_recombinations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningCurvePoint {
    pub experiments_executed: usize,
    pub external_blind: BlindMetrics,
    pub promoted_concepts: usize,
    pub validated_relations: usize,
    pub resolved_uncertainties: usize,
    pub remaining_uncertainties: usize,
    pub hypotheses_remaining: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CurriculumQualityMetrics {
    pub candidate_experiments_generated: usize,
    pub experiments_selected: usize,
    pub experiments_executed: usize,
    pub mean_expected_information_gain: f64,
    pub mean_realized_information_gain: f64,
    pub hypotheses_eliminated: usize,
    pub uncertainties_resolved: usize,
    pub duplicate_rate: f64,
    pub near_duplicate_rate: f64,
    pub mastered_replay_rate: f64,
    pub frontier_task_fraction: f64,
    pub too_easy_fraction: f64,
    pub currently_unsolved_fraction: f64,
    pub invalid_experiment_fraction: f64,
    pub self_generated_solve_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArmReport {
    pub condition: SelectorCondition,
    pub experiment_budget: usize,
    pub equal_budget_enforced: bool,
    pub initial_ledger: UncertaintyLedger,
    pub final_ledger: UncertaintyLedger,
    pub selected_experiment_ids: Vec<String>,
    pub learning_curve: Vec<LearningCurvePoint>,
    pub final_external_blind: BlindMetrics,
    pub curriculum_quality: CurriculumQualityMetrics,
    pub surprises: Vec<SemanticSurpriseEvent>,
    pub revisions: Vec<ModelRevision>,
    pub local_active_inference_probes: usize,
    pub epistemic_experiments: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedExperimentCatalog {
    pub generator_version: String,
    pub closed_world: bool,
    pub environment_hidden: bool,
    pub experiments: Vec<CandidateExperiment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptDiscoveryReport {
    pub new_candidate_concepts: usize,
    pub new_promoted_concepts: usize,
    pub generation_3_candidates: usize,
    pub generation_3_promoted: usize,
    pub maximum_autonomous_concept_generation: usize,
    pub promotion_gates_lowered: bool,
    pub discovery_origins: BTreeMap<String, usize>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityFrontierEntry {
    pub task_id: String,
    pub solution_graph_depth: usize,
    pub primitive_expanded_depth: usize,
    pub concepts_composed: usize,
    pub simultaneous_subproblems: usize,
    pub recombinations: usize,
    pub semantic_traps: usize,
    pub search_expansions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityFrontierReport {
    pub phase: String,
    pub solved_tasks: usize,
    pub entries: Vec<CapabilityFrontierEntry>,
    pub maximum_solution_graph_depth: usize,
    pub maximum_primitive_expanded_depth: usize,
    pub maximum_concepts_composed: usize,
    pub maximum_simultaneous_subproblems: usize,
    pub maximum_recombinations: usize,
}

pub fn retained_rules(item: &UncertaintyItem) -> BTreeSet<HypothesisRule> {
    item.competing_hypotheses
        .iter()
        .filter(|hypothesis| hypothesis.retained)
        .map(|hypothesis| hypothesis.rule)
        .collect()
}
