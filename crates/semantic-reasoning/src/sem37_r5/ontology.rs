use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IdentifiabilityState {
    FullyIdentifiable,
    PartiallyIdentifiable,
    NotIdentifiableUnderAvailableEvidence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterventionEvidence {
    pub contract_id: String,
    pub targets: Vec<usize>,
    pub times: Vec<usize>,
    pub predicted_outcomes_before_intervention: Value,
    pub observed_outcomes_after_prediction: Value,
    pub mediator_intervention_available: bool,
    pub prediction_commitment: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PathSpecificCausalIr {
    pub case_id: String,
    pub source: usize,
    pub target: usize,
    pub candidate_paths: Vec<Vec<usize>>,
    pub direct_path_candidate: Option<Vec<usize>>,
    pub mediator_paths: Vec<Vec<usize>>,
    pub common_cause_hypotheses: Vec<usize>,
    pub available_interventions: Vec<String>,
    pub intervention_constraints: Vec<String>,
    pub path_identifiability: IdentifiabilityState,
    pub direct_effect_evidence: Vec<String>,
    pub mediated_effect_evidence: Vec<String>,
    pub mixed_effect_evidence: Vec<String>,
    pub unresolved_components: Vec<String>,
    pub uncertainty_millionths: u64,
    pub provenance: Vec<String>,
    pub verification: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DirectPathCertificate {
    pub case_id: String,
    pub source: usize,
    pub target: usize,
    pub candidate_mediators: Vec<usize>,
    pub candidate_confounders: Vec<usize>,
    pub interventions_available: Vec<String>,
    pub interventions_performed: Vec<String>,
    pub predicted_outcomes_before_intervention: Value,
    pub observed_outcomes: Value,
    pub path_specific_evidence: Vec<String>,
    pub identifiability: IdentifiabilityState,
    pub remaining_uncertainty: Vec<String>,
    pub promotion_rationale: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediatedPathCertificate {
    pub case_id: String,
    pub source: usize,
    pub mediator_path: Vec<usize>,
    pub target: usize,
    pub path_intervention_evidence: Vec<String>,
    pub temporal_evidence: Vec<String>,
    pub transfer_evidence: Vec<String>,
    pub identifiability: IdentifiabilityState,
    pub uncertainty_millionths: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnresolvedCertificate {
    pub case_id: String,
    pub remaining_hypotheses: Vec<String>,
    pub discrimination_limit: String,
    pub resolving_intervention_if_available: String,
}

pub const PREDICTIVE_RESIDUAL_IS_DIRECT_EFFECT_PROOF: bool = false;
pub const TEMPORAL_PRECEDENCE_IS_DIRECT_EFFECT_AUTHORITY: bool = false;
pub const WHOLE_CAUSAL_MEDIATION_ALGORITHM_TRANSPLANTS: u64 = 0;
pub const HAND_TUNED_DIRECTNESS_THRESHOLD_EVENTS: u64 = 0;
pub const UNAVAILABLE_COUNTERFACTUAL_USED_AS_OBSERVED_EVIDENCE: u64 = 0;
