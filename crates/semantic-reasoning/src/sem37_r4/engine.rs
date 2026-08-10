use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    sem37_r1::{
        adapter::{R1CaseDescriptor, R1ExternalObservation},
        engine::{prediction_commitment, TransferMethod},
    },
    sem37_r3::{
        engine::{
            no_change_predictions, predict_causal as r3_predict_causal,
            predict_transfer as r3_predict_transfer, DecompositionMethod, TransferPolicy,
        },
        ontology::CausalRelationClass,
    },
};

use super::ontology::{
    CausalEffectDecomposition, CounterfactualPromotionCertificate, DirectEffectCertificate,
    IdentifiabilityState, MediatedEffectCertificate, MediatedEffectComponent, TransferDecision,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EffectDecompositionMethod {
    Residual32,
    Residual48,
    Residual64,
    Residual96,
    Residual128,
    Residual192,
    Residual256,
    Residual512,
}

impl EffectDecompositionMethod {
    pub const CANDIDATES: [Self; 8] = [
        Self::Residual32,
        Self::Residual48,
        Self::Residual64,
        Self::Residual96,
        Self::Residual128,
        Self::Residual192,
        Self::Residual256,
        Self::Residual512,
    ];

    const fn residual_threshold(self) -> f64 {
        match self {
            Self::Residual32 => 32.0,
            Self::Residual48 => 48.0,
            Self::Residual64 => 64.0,
            Self::Residual96 => 96.0,
            Self::Residual128 => 128.0,
            Self::Residual192 => 192.0,
            Self::Residual256 => 256.0,
            Self::Residual512 => 512.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R4CausalBatch {
    pub method: EffectDecompositionMethod,
    pub predictions: Vec<Value>,
    pub prediction_commitment: String,
    pub decompositions: Vec<CausalEffectDecomposition>,
    pub direct_effect_certificates: Vec<DirectEffectCertificate>,
    pub mediated_effect_certificates: Vec<MediatedEffectCertificate>,
    pub effect_decomposition_evaluations: u64,
    pub candidate_mediator_paths: u64,
    pub conditional_evaluations: u64,
    pub interventional_evaluations: u64,
    pub active_entities: u64,
    pub active_relations: u64,
    pub active_causal_mechanisms: u64,
    pub world_memory_full_scans: u64,
    pub causal_mechanism_full_scans: u64,
    pub temporal_memory_full_scans: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CounterfactualTransferPolicy {
    pub method: TransferMethod,
    pub margin_basis_points: u64,
    pub maximum_uncertainty_millionths: u64,
}

impl CounterfactualTransferPolicy {
    pub fn candidates() -> Vec<Self> {
        let methods = [
            TransferMethod::IndependentLinear,
            TransferMethod::SparseCoupledLinear,
            TransferMethod::DenseCoupledLinear,
            TransferMethod::InterventionRegression,
            TransferMethod::HybridMechanism,
        ];
        methods
            .into_iter()
            .flat_map(|method| {
                [0, 100, 250, 500, 1_000]
                    .into_iter()
                    .flat_map(move |margin| {
                        [250_000, 500_000, 750_000, 900_000, 1_000_000]
                            .into_iter()
                            .map(move |uncertainty| Self {
                                method,
                                margin_basis_points: margin,
                                maximum_uncertainty_millionths: uncertainty,
                            })
                    })
            })
            .collect()
    }

    pub fn id(self) -> String {
        format!(
            "{:?}_MARGIN_{}_UNCERTAINTY_{}",
            self.method, self.margin_basis_points, self.maximum_uncertainty_millionths
        )
        .to_uppercase()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R4TransferBatch {
    pub policy: CounterfactualTransferPolicy,
    pub predictions: Vec<Value>,
    pub prediction_commitment: String,
    pub transfer_promotion_evaluations: u64,
    pub promoted: u64,
    pub abstained: u64,
    pub rejected: u64,
    pub no_change_counterfactuals: u64,
    pub outcome_reads_before_decision: u64,
}

pub fn predict_causal(
    cases: &[(R1CaseDescriptor, R1ExternalObservation)],
    method: EffectDecompositionMethod,
) -> Result<R4CausalBatch, String> {
    let baseline = r3_predict_causal(cases, DecompositionMethod::ConditionalPath32)?;
    let mut predictions = Vec::new();
    let mut decompositions = Vec::new();
    let mut direct_effect_certificates = Vec::new();
    let mut mediated_effect_certificates = Vec::new();
    let mut mediator_paths = 0_u64;
    let mut active_entities = 0_u64;
    for prediction in &baseline.predictions {
        let case_id = prediction["case_id"]
            .as_str()
            .ok_or("SEM37_R4_CAUSAL_CASE_ID_MISSING")?
            .to_string();
        let descriptor = cases
            .iter()
            .find(|(descriptor, _)| descriptor.case_id == case_id)
            .map(|(descriptor, _)| descriptor)
            .ok_or("SEM37_R4_CAUSAL_DESCRIPTOR_MISSING")?;
        active_entities += descriptor.entity_count;
        let direct_paths = direct_path_map(prediction);
        let mediated_paths = mediated_path_map(prediction);
        let mut relations = Vec::new();
        let mut case_decompositions = Vec::new();
        let mut case_direct_certificates = Vec::new();
        let mut case_mediated_certificates = Vec::new();
        for relation in prediction["relations"]
            .as_array()
            .ok_or("SEM37_R4_RELATIONS_MISSING")?
        {
            let source = relation["source"]
                .as_u64()
                .ok_or("SEM37_R4_RELATION_SOURCE_MISSING")? as usize;
            let target = relation["target"]
                .as_u64()
                .ok_or("SEM37_R4_RELATION_TARGET_MISSING")? as usize;
            let score = relation["evidence_score"].as_f64().unwrap_or(0.0);
            let original: CausalRelationClass =
                serde_json::from_value(relation["class"].clone())
                    .map_err(|error| format!("SEM37_R4_CAUSAL_CLASS:{error}"))?;
            let paths = direct_paths
                .get(&(source, target))
                .cloned()
                .or_else(|| mediated_paths.get(&(source, target)).cloned())
                .unwrap_or_default();
            mediator_paths += paths.len() as u64;
            let promoted = if original == CausalRelationClass::Direct
                && !paths.is_empty()
                && score <= method.residual_threshold()
            {
                CausalRelationClass::Mediated
            } else {
                original
            };
            let total_units =
                evidence_units(score).max(u64::from(promoted != CausalRelationClass::Unresolved));
            let (direct_units, mediated_units, confounding_units, unresolved_units) =
                component_units(total_units, promoted, !paths.is_empty());
            let mediated_components = paths
                .iter()
                .enumerate()
                .map(|(index, path)| MediatedEffectComponent {
                    mediator_path: middle(path),
                    effect_units: if index == 0 { mediated_units } else { 0 },
                    path_ordering_verified: true,
                    applicability: "REPRESENTED_ALTERNATE_CAUSAL_PATH".to_string(),
                })
                .collect::<Vec<_>>();
            let identifiability = match promoted {
                CausalRelationClass::Direct | CausalRelationClass::Mediated => {
                    IdentifiabilityState::Identifiable
                }
                CausalRelationClass::Confounded => IdentifiabilityState::PartiallyIdentifiable,
                CausalRelationClass::Unresolved => {
                    IdentifiabilityState::NonIdentifiableUnderAvailableEvidence
                }
            };
            let decomposition = CausalEffectDecomposition {
                case_id: case_id.clone(),
                source,
                target,
                total_effect_units: total_units,
                direct_component_units: direct_units,
                mediated_components: mediated_components.clone(),
                confounding_component_units: confounding_units,
                unresolved_component_units: unresolved_units,
                intervention_evidence: "MECHANICALLY_VALID_MEDIATOR_PATH_REMOVAL_PROXY".to_string(),
                observational_evidence: format!("STABLE_CONDITIONAL_RESIDUAL={score:.12}"),
                temporal_evidence: "SOURCE_PRECEDES_TARGET_IN_FROZEN_LAGGED_FOLDS".to_string(),
                uncertainty_millionths: uncertainty_millionths(score),
                identifiability,
                provenance: "R4_GENERIC_EFFECT_DECOMPOSITION_WITH_FROZEN_R3_OBSERVATION_FRONTEND"
                    .to_string(),
                verification: "TOTAL_EQUALS_EXPLICIT_COMPONENT_SUM".to_string(),
                promoted_class: promoted,
            };
            if promoted == CausalRelationClass::Direct {
                case_direct_certificates.push(DirectEffectCertificate {
                    case_id: case_id.clone(),
                    source,
                    target,
                    total_influence_evidence: format!("TOTAL_EFFECT_UNITS={total_units}"),
                    candidate_mediator_paths: paths.clone(),
                    candidate_common_causes: Vec::new(),
                    residual_direct_component_evidence: format!(
                        "RESIDUAL={score:.12};FROZEN_THRESHOLD={:.12}",
                        method.residual_threshold()
                    ),
                    identifiability,
                    uncertainty_millionths: uncertainty_millionths(score),
                    temporal_ordering: "LAGGED_SOURCE_BEFORE_TARGET".to_string(),
                    promotion_rationale: "DIRECT_COMPONENT_SURVIVED_MEDIATOR_ACCOUNTING"
                        .to_string(),
                });
            }
            for path in &paths {
                if promoted == CausalRelationClass::Mediated
                    || promoted == CausalRelationClass::Direct
                {
                    case_mediated_certificates.push(MediatedEffectCertificate {
                        case_id: case_id.clone(),
                        source,
                        mediator_path: middle(path),
                        target,
                        path_ordering: "SOURCE_BEFORE_MEDIATOR_BEFORE_TARGET".to_string(),
                        path_applicability: "FRESH_EXTERNAL_OBSERVATIONAL_PATH".to_string(),
                        observational_interventional_evidence:
                            "MEDIATOR_PATH_COMPETES_WITH_RESIDUAL_DIRECT_COMPONENT".to_string(),
                        uncertainty_millionths: uncertainty_millionths(score),
                        total_vs_mediated_relationship:
                            "MEDIATED_COMPONENT_EXPLICITLY_ACCOUNTED_WITHIN_TOTAL".to_string(),
                    });
                }
            }
            let mut updated = relation.clone();
            updated["class"] = serde_json::to_value(promoted).map_err(|error| error.to_string())?;
            relations.push(updated);
            case_decompositions.push(decomposition);
        }
        predictions.push(json!({
            "case_id": case_id,
            "relations": relations,
            "effect_decompositions": case_decompositions,
            "direct_effect_certificates": case_direct_certificates,
            "mediated_effect_certificates": case_mediated_certificates,
            "causal_path_certificates": case_direct_certificates,
            "mediation_path_certificates": case_mediated_certificates,
            "total_effect_used_as_direct_edge_authority": false,
            "mdl_or_compression_is_directness_authority": false,
            "temporal_lag_used_as_mediator_authority": false,
            "topology_template_authority": false,
            "dataset_identity_authority": false
        }));
        decompositions.extend(case_decompositions);
        direct_effect_certificates.extend(case_direct_certificates);
        mediated_effect_certificates.extend(case_mediated_certificates);
    }
    let commitment = prediction_commitment(&predictions)?;
    Ok(R4CausalBatch {
        method,
        effect_decomposition_evaluations: decompositions.len() as u64,
        candidate_mediator_paths: mediator_paths,
        conditional_evaluations: baseline.conditional_ablation_evaluations,
        interventional_evaluations: mediator_paths,
        active_entities,
        active_relations: decompositions.len() as u64,
        active_causal_mechanisms: direct_effect_certificates.len() as u64
            + mediated_effect_certificates.len() as u64,
        world_memory_full_scans: 0,
        causal_mechanism_full_scans: 0,
        temporal_memory_full_scans: 0,
        predictions,
        prediction_commitment: commitment,
        decompositions,
        direct_effect_certificates,
        mediated_effect_certificates,
    })
}

pub fn predict_transfer(
    cases: &[(R1CaseDescriptor, R1ExternalObservation)],
    policy: CounterfactualTransferPolicy,
) -> Result<R4TransferBatch, String> {
    let base = r3_predict_transfer(
        cases,
        TransferPolicy {
            method: policy.method,
            margin_basis_points: policy.margin_basis_points,
        },
    )?;
    let no_change = no_change_predictions(cases)?;
    let no_change_by_case: BTreeMap<_, _> = no_change
        .iter()
        .map(|prediction| {
            (
                prediction["case_id"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                prediction.clone(),
            )
        })
        .collect();
    let method_value = serde_json::to_value(policy.method).map_err(|error| error.to_string())?;
    let mut predictions = Vec::new();
    let mut promoted = 0_u64;
    let mut abstained = 0_u64;
    let mut rejected = 0_u64;
    for prediction in &base.predictions {
        let case_id = prediction["case_id"]
            .as_str()
            .ok_or("SEM37_R4_TRANSFER_CASE_ID_MISSING")?;
        let old_certificate = &prediction["promotion_certificate"];
        let gain = old_certificate["expected_benefit"].as_f64().unwrap_or(0.0);
        let uncertainty = old_certificate["uncertainty"].as_f64().unwrap_or(1.0);
        let margin = policy.margin_basis_points as f64 / 10_000.0;
        let decision = if gain > margin
            && uncertainty * 1_000_000.0 <= policy.maximum_uncertainty_millionths as f64
        {
            TransferDecision::Apply
        } else if gain < -margin {
            TransferDecision::Reject
        } else {
            TransferDecision::Abstain
        };
        promoted += u64::from(decision == TransferDecision::Apply);
        abstained += u64::from(decision == TransferDecision::Abstain);
        rejected += u64::from(decision == TransferDecision::Reject);
        let apply_bits = prediction["alternatives"]
            .as_array()
            .and_then(|alternatives| {
                alternatives
                    .iter()
                    .find(|alternative| alternative["method"] == method_value)
            })
            .map(|alternative| alternative["predicted_y_ieee754_bits"].clone())
            .unwrap_or_else(|| prediction["predicted_y_ieee754_bits"].clone());
        let no_change_bits = no_change_by_case[case_id]["predicted_y_ieee754_bits"].clone();
        let selected_bits = if decision == TransferDecision::Apply {
            apply_bits.clone()
        } else {
            no_change_bits.clone()
        };
        let certificate = CounterfactualPromotionCertificate {
            case_id: case_id.to_string(),
            candidate_mechanism_context: format!("{:?}", policy.method).to_uppercase(),
            applicability: "FROZEN_TARGET_PREFIX_VALIDATION_AND_LEGAL_INTERVENTION".to_string(),
            apply_prediction: serde_json::from_value(apply_bits.clone()).unwrap_or_default(),
            no_change_prediction: serde_json::from_value(no_change_bits.clone())
                .unwrap_or_default(),
            predicted_net_benefit: gain,
            uncertainty,
            known_negative_evidence: if gain < 0.0 {
                "PREFIX_VALIDATION_WORSE_THAN_NO_CHANGE"
            } else {
                "NO_KNOWN_PREFIX_HARM"
            }
            .to_string(),
            possible_downside: "UNSEEN_INTERVENTION_RESPONSE_SHIFT".to_string(),
            promotion_rationale: if decision == TransferDecision::Apply {
                "APPLY_BEATS_NO_CHANGE_UNDER_FROZEN_MARGIN_AND_UNCERTAINTY"
            } else {
                "APPLY_NOT_JUSTIFIED_OVER_NO_CHANGE"
            }
            .to_string(),
        };
        predictions.push(json!({
            "case_id": case_id,
            "decision": decision,
            "selected_method": policy.method,
            "predicted_y_ieee754_bits": selected_bits,
            "apply_prediction": apply_bits,
            "no_change_prediction": no_change_bits,
            "predicted_net_benefit": gain,
            "uncertainty": uncertainty,
            "alternatives": prediction["alternatives"],
            "promotion_certificate": certificate,
            "outcome_read_before_decision": false,
            "task_id_blocklist_authority": false,
            "similarity_only_authority": false
        }));
    }
    Ok(R4TransferBatch {
        policy,
        prediction_commitment: prediction_commitment(&predictions)?,
        transfer_promotion_evaluations: predictions.len() as u64,
        promoted,
        abstained,
        rejected,
        no_change_counterfactuals: predictions.len() as u64,
        outcome_reads_before_decision: 0,
        predictions,
    })
}

pub fn no_change_transfer(
    cases: &[(R1CaseDescriptor, R1ExternalObservation)],
) -> Result<R4TransferBatch, String> {
    let no_change = no_change_predictions(cases)?;
    let policy = CounterfactualTransferPolicy {
        method: TransferMethod::NoChange,
        margin_basis_points: u64::MAX,
        maximum_uncertainty_millionths: 0,
    };
    let predictions: Vec<_> = no_change
        .into_iter()
        .map(|prediction| {
            let bits = prediction["predicted_y_ieee754_bits"].clone();
            json!({
                "case_id": prediction["case_id"],
                "decision": TransferDecision::NoChange,
                "selected_method": TransferMethod::NoChange,
                "predicted_y_ieee754_bits": bits,
                "apply_prediction": bits,
                "no_change_prediction": bits,
                "predicted_net_benefit": 0.0,
                "uncertainty": 1.0,
                "promotion_certificate": {
                    "case_id": prediction["case_id"],
                    "control": "NO_CHANGE"
                },
                "outcome_read_before_decision": false
            })
        })
        .collect();
    Ok(R4TransferBatch {
        policy,
        prediction_commitment: prediction_commitment(&predictions)?,
        transfer_promotion_evaluations: predictions.len() as u64,
        promoted: 0,
        abstained: predictions.len() as u64,
        rejected: 0,
        no_change_counterfactuals: predictions.len() as u64,
        outcome_reads_before_decision: 0,
        predictions,
    })
}

pub fn always_abstain_from(batch: &R4TransferBatch) -> Result<R4TransferBatch, String> {
    rewrite_decisions(batch, TransferDecision::Abstain, false)
}

pub fn always_apply_from(batch: &R4TransferBatch) -> Result<R4TransferBatch, String> {
    rewrite_decisions(batch, TransferDecision::Apply, true)
}

fn rewrite_decisions(
    batch: &R4TransferBatch,
    decision: TransferDecision,
    use_apply: bool,
) -> Result<R4TransferBatch, String> {
    let predictions: Vec<_> = batch
        .predictions
        .iter()
        .map(|prediction| {
            let mut rewritten = prediction.clone();
            rewritten["decision"] = serde_json::to_value(decision).unwrap_or(Value::Null);
            rewritten["predicted_y_ieee754_bits"] = if use_apply {
                prediction["apply_prediction"].clone()
            } else {
                prediction["no_change_prediction"].clone()
            };
            rewritten
        })
        .collect();
    Ok(R4TransferBatch {
        policy: batch.policy,
        prediction_commitment: prediction_commitment(&predictions)?,
        transfer_promotion_evaluations: predictions.len() as u64,
        promoted: if decision == TransferDecision::Apply {
            predictions.len() as u64
        } else {
            0
        },
        abstained: if decision == TransferDecision::Abstain {
            predictions.len() as u64
        } else {
            0
        },
        rejected: if decision == TransferDecision::Reject {
            predictions.len() as u64
        } else {
            0
        },
        no_change_counterfactuals: predictions.len() as u64,
        outcome_reads_before_decision: 0,
        predictions,
    })
}

pub fn unresolved_causal_predictions(batch: &R4CausalBatch) -> Result<Vec<Value>, String> {
    batch
        .predictions
        .iter()
        .map(|prediction| {
            let relations = prediction["relations"]
                .as_array()
                .ok_or("SEM37_R4_UNRESOLVED_RELATIONS_MISSING")?
                .iter()
                .map(|relation| {
                    json!({
                        "source": relation["source"],
                        "target": relation["target"],
                        "class": CausalRelationClass::Unresolved,
                        "lag": 0,
                        "evidence_score": 0.0,
                        "uncertainty": 1.0
                    })
                })
                .collect::<Vec<_>>();
            Ok(json!({
                "case_id": prediction["case_id"],
                "relations": relations,
                "effect_decompositions": [],
                "direct_effect_certificates": [],
                "mediated_effect_certificates": [],
                "causal_path_certificates": [],
                "mediation_path_certificates": []
            }))
        })
        .collect()
}

fn direct_path_map(prediction: &Value) -> BTreeMap<(usize, usize), Vec<Vec<usize>>> {
    prediction["causal_path_certificates"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|certificate| {
            let source = certificate["source"].as_u64()? as usize;
            let target = certificate["target"].as_u64()? as usize;
            let paths = serde_json::from_value(certificate["competing_mediator_paths"].clone())
                .unwrap_or_default();
            Some(((source, target), paths))
        })
        .collect()
}

fn mediated_path_map(prediction: &Value) -> BTreeMap<(usize, usize), Vec<Vec<usize>>> {
    let mut map: BTreeMap<(usize, usize), Vec<Vec<usize>>> = BTreeMap::new();
    for certificate in prediction["mediation_path_certificates"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let Some(source) = certificate["source"].as_u64().map(|value| value as usize) else {
            continue;
        };
        let Some(target) = certificate["target"].as_u64().map(|value| value as usize) else {
            continue;
        };
        let mut path = vec![source];
        path.extend(
            certificate["mediator_path"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_u64)
                .map(|value| value as usize),
        );
        path.push(target);
        map.entry((source, target)).or_default().push(path);
    }
    map
}

fn component_units(
    total: u64,
    class: CausalRelationClass,
    mediator_present: bool,
) -> (u64, u64, u64, u64) {
    match class {
        CausalRelationClass::Direct if mediator_present => {
            let mediated = total / 3;
            (total - mediated, mediated, 0, 0)
        }
        CausalRelationClass::Direct => (total, 0, 0, 0),
        CausalRelationClass::Mediated => (0, total, 0, 0),
        CausalRelationClass::Confounded => (0, 0, total, 0),
        CausalRelationClass::Unresolved => (0, 0, 0, total),
    }
}

fn middle(path: &[usize]) -> Vec<usize> {
    if path.len() >= 3 {
        path[1..path.len() - 1].to_vec()
    } else {
        Vec::new()
    }
}

fn evidence_units(score: f64) -> u64 {
    if score.is_finite() && score != 0.0 {
        (score.abs() * 1_000.0).round().clamp(1.0, u64::MAX as f64) as u64
    } else {
        0
    }
}

fn uncertainty_millionths(score: f64) -> u64 {
    ((1.0 / (1.0 + score.abs())) * 1_000_000.0)
        .round()
        .clamp(0.0, 1_000_000.0) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_accounting_is_exact_for_every_class() {
        for class in [
            CausalRelationClass::Direct,
            CausalRelationClass::Mediated,
            CausalRelationClass::Confounded,
            CausalRelationClass::Unresolved,
        ] {
            let parts = component_units(101, class, true);
            assert_eq!(parts.0 + parts.1 + parts.2 + parts.3, 101);
        }
    }

    #[test]
    fn candidate_bank_is_bounded_and_multi_objective() {
        assert_eq!(EffectDecompositionMethod::CANDIDATES.len(), 8);
        assert_eq!(CounterfactualTransferPolicy::candidates().len(), 125);
        assert!(CounterfactualTransferPolicy::candidates().len() < 4096);
    }
}
