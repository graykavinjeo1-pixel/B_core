use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskClass {
    Depth,
    Width,
    Recombination,
    Composition,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Split {
    Development,
    Calibration,
    FreshBlind,
    AdversarialBlind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Condition {
    PrimitiveFixedA,
    SemanticNonAdaptiveB,
    FixedHeuristicC,
    AdaptiveD,
    DMinusInformationGain,
    DMinusSemanticPruning,
    DMinusDecomposition,
    DMinusStateMerging,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ControllerAction {
    ExpandCurrent,
    BranchAlternative,
    PruneBranch,
    DecomposeGoal,
    SwitchSubproblem,
    RecombineResults,
    ExecuteProbe,
    GenerateCounterfactual,
    ReuseConcept,
    Backtrack,
    CompressIntermediate,
    StopSolved,
    StopResourceExhausted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateAction {
    pub action_id: String,
    pub structural_shape: String,
    pub input_type: String,
    pub output_type: String,
    pub required_facts: BTreeSet<String>,
    pub export_contract: String,
    pub resulting_semantic_state: String,
    pub invariant_consistent: bool,
    pub concept_id: Option<String>,
    pub concept_generation: usize,
    pub primitive_expansion_cost: usize,
    pub execution_cost: usize,
    pub observed_failure_signature: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Goal {
    pub goal_id: String,
    pub dependencies: Vec<String>,
    pub input_type: String,
    pub output_type: String,
    pub required_export_contract: String,
    pub candidates: Vec<CandidateAction>,
    pub recombination: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProbeContract {
    pub probe_id: String,
    pub cost: usize,
    pub candidate_predictions: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibleTask {
    pub task_id: String,
    pub initial_facts: BTreeSet<String>,
    pub goals: Vec<Goal>,
    pub probes: Vec<ProbeContract>,
    pub resource_class: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluatorMetadata {
    pub task_class: TaskClass,
    pub required_depth: usize,
    pub required_concepts: usize,
    pub correct_branches: BTreeMap<String, String>,
    pub difficulty_band: String,
    pub intended_decomposition: usize,
    pub expected_recombinations: usize,
    pub adversarial_features: Vec<String>,
    pub probe_observations: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationTask {
    pub visible: VisibleTask,
    pub split: Split,
    #[serde(skip_serializing)]
    pub evaluator: EvaluatorMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TerminationReason {
    VerifiedSuccess,
    ResourceExhausted,
    VerifierFailure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceBudget {
    pub max_expansions: usize,
    pub max_wall_time_units: usize,
    pub max_memory_units: usize,
    pub max_live_frontier: usize,
    pub max_stagnation: usize,
}

impl ResourceBudget {
    pub fn equal_resource() -> Self {
        Self {
            max_expansions: 20_000,
            max_wall_time_units: 20_000,
            max_memory_units: 8_192,
            max_live_frontier: 512,
            max_stagnation: 8,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CanonicalMetrics {
    pub solution_graph_depth: usize,
    pub primitive_expanded_solution_depth: usize,
    pub search_trajectory_max_depth: usize,
    pub instantaneous_frontier_width: usize,
    pub peak_simultaneously_live_branches: usize,
    pub cumulative_branches_generated: usize,
    pub cumulative_search_expansions: usize,
    pub peak_active_concepts: usize,
    pub mean_active_concepts: f64,
    pub concepts_composed: usize,
    pub decomposition_count: usize,
    pub subproblems_created: usize,
    pub subproblems_solved: usize,
    pub recombination_count: usize,
    pub maximum_decomposition_tree_depth: usize,
    pub maximum_simultaneous_subproblems: usize,
    pub useful_branch_ratio: f64,
    pub pruned_branch_count: usize,
    pub false_prune_count: usize,
    pub semantic_prune_count: usize,
    pub dominance_merge_count: usize,
    pub false_merge_count: usize,
    pub information_probes_proposed: usize,
    pub information_probes_executed: usize,
    pub hypotheses_eliminated: usize,
    pub expansions_saved_by_probes: usize,
    pub stagnation_prunes: usize,
    pub backtracks: usize,
    pub rollbacks: usize,
    pub promoted_concept_reuse_count: usize,
    pub cross_generation_concept_composition_count: usize,
    pub wall_time_units: usize,
    pub peak_memory_units: usize,
    pub branch_expansion_gini: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchAllocation {
    pub branch_id: String,
    pub allocated_expansions: usize,
    pub actual_expansions: usize,
    pub estimated_value: f64,
    pub termination_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    pub sequence: usize,
    pub task_id: String,
    pub goal_id: Option<String>,
    pub action: ControllerAction,
    pub action_value: f64,
    pub reason: String,
    pub cumulative_expansions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolveResult {
    pub task_id: String,
    pub condition: Condition,
    pub solved: bool,
    pub strictly_correct: bool,
    pub termination_reason: TerminationReason,
    pub selected_actions: BTreeMap<String, String>,
    pub metrics: CanonicalMetrics,
    pub allocations: Vec<BranchAllocation>,
    pub trace: Vec<TraceEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionSummary {
    pub condition: Condition,
    pub tasks: usize,
    pub solved: usize,
    pub strict_solve_rate: f64,
    pub total_search_expansions: usize,
    pub median_search_expansions: f64,
    pub peak_live_branches: usize,
    pub peak_frontier_width: usize,
    pub cumulative_branches_generated: usize,
    pub false_prunes: usize,
    pub false_merges: usize,
    pub results: Vec<SolveResult>,
}
