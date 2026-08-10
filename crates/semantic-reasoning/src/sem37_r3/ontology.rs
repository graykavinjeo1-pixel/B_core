use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CausalRelationClass {
    Direct,
    Mediated,
    Confounded,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CausalRelation {
    pub source: usize,
    pub target: usize,
    pub class: CausalRelationClass,
    pub lag: u64,
    pub evidence_score: f64,
    pub uncertainty: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DirectPathCertificate {
    pub case_id: String,
    pub source: usize,
    pub target: usize,
    pub supporting_evidence: String,
    pub competing_mediator_paths: Vec<Vec<usize>>,
    pub competing_common_cause_hypotheses: Vec<usize>,
    pub intervention_observation_evidence: String,
    pub uncertainty: f64,
    pub promotion_rationale: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediatedPathCertificate {
    pub case_id: String,
    pub source: usize,
    pub mediator_path: Vec<usize>,
    pub target: usize,
    pub semantic_temporal_ordering: String,
    pub evidence: String,
    pub uncertainty: f64,
    pub counterfactual_implication: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransferDecision {
    Promote,
    Abstain,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromotionCertificate {
    pub case_id: String,
    pub expected_benefit: f64,
    pub uncertainty: f64,
    pub known_negative_evidence: String,
    pub applicability_conditions: String,
    pub counterfactual_no_change_expectation: String,
    pub possible_downside: String,
    pub reason_abstention_was_rejected: String,
}

pub const TOTAL_EFFECT_USED_AS_DIRECT_EDGE_AUTHORITY: bool = false;
pub const TOPOLOGY_TEMPLATE_TO_CAUSAL_CLASS_AUTHORITY: bool = false;
pub const BENCHMARK_ID_TO_CAUSAL_CLASS_AUTHORITY: bool = false;
pub const R2_METHOD_IS_PROMOTION_AUTHORITY: bool = false;
pub const LAG_USED_AS_MEDIATOR_AUTHORITY: bool = false;
pub const SIMILARITY_ONLY_TRANSFER_PROMOTION_EVENTS: u64 = 0;
pub const TASK_ID_NEGATIVE_TRANSFER_BLOCKLIST_AUTHORITY: bool = false;
