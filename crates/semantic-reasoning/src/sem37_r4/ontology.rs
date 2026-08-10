use serde::{Deserialize, Serialize};

use crate::sem37_r3::ontology::CausalRelationClass;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IdentifiabilityState {
    Identifiable,
    PartiallyIdentifiable,
    NonIdentifiableUnderAvailableEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediatedEffectComponent {
    pub mediator_path: Vec<usize>,
    pub effect_units: u64,
    pub path_ordering_verified: bool,
    pub applicability: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalEffectDecomposition {
    pub case_id: String,
    pub source: usize,
    pub target: usize,
    pub total_effect_units: u64,
    pub direct_component_units: u64,
    pub mediated_components: Vec<MediatedEffectComponent>,
    pub confounding_component_units: u64,
    pub unresolved_component_units: u64,
    pub intervention_evidence: String,
    pub observational_evidence: String,
    pub temporal_evidence: String,
    pub uncertainty_millionths: u64,
    pub identifiability: IdentifiabilityState,
    pub provenance: String,
    pub verification: String,
    pub promoted_class: CausalRelationClass,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectEffectCertificate {
    pub case_id: String,
    pub source: usize,
    pub target: usize,
    pub total_influence_evidence: String,
    pub candidate_mediator_paths: Vec<Vec<usize>>,
    pub candidate_common_causes: Vec<usize>,
    pub residual_direct_component_evidence: String,
    pub identifiability: IdentifiabilityState,
    pub uncertainty_millionths: u64,
    pub temporal_ordering: String,
    pub promotion_rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediatedEffectCertificate {
    pub case_id: String,
    pub source: usize,
    pub mediator_path: Vec<usize>,
    pub target: usize,
    pub path_ordering: String,
    pub path_applicability: String,
    pub observational_interventional_evidence: String,
    pub uncertainty_millionths: u64,
    pub total_vs_mediated_relationship: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransferDecision {
    Apply,
    NoChange,
    Abstain,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CounterfactualPromotionCertificate {
    pub case_id: String,
    pub candidate_mechanism_context: String,
    pub applicability: String,
    pub apply_prediction: Vec<u64>,
    pub no_change_prediction: Vec<u64>,
    pub predicted_net_benefit: f64,
    pub uncertainty: f64,
    pub known_negative_evidence: String,
    pub possible_downside: String,
    pub promotion_rationale: String,
}

pub const CAUSAL_TAXONOMY_WITHOUT_EFFECT_DECOMPOSITION_IS_SUFFICIENT: bool = false;
pub const TOTAL_EFFECT_USED_AS_DIRECT_EDGE_AUTHORITY: bool = false;
pub const MDL_OR_COMPRESSION_IS_DIRECTNESS_AUTHORITY: bool = false;
pub const TEMPORAL_LAG_USED_AS_MEDIATOR_AUTHORITY: bool = false;
pub const BENCHMARK_ID_TO_CAUSAL_DECOMPOSITION_AUTHORITY: bool = false;
pub const TOPOLOGY_TEMPLATE_TO_CAUSAL_DECOMPOSITION_AUTHORITY: bool = false;
pub const DATASET_ID_TO_DIRECTNESS_AUTHORITY: bool = false;
pub const TASK_ID_NEGATIVE_TRANSFER_BLOCKLIST_AUTHORITY: bool = false;
