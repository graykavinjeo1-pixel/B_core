use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    sem37_r1::{
        adapter::{R1CaseDescriptor, R1ExternalLane, R1ExternalObservation},
        engine::{
            predict_batch as r1_predict_batch, prediction_commitment, TransferMethod,
            TransferResearchMode,
        },
    },
    sem37_r2::engine::{predict_lane_a as r2_predict_lane_a, CausalPrecisionMethod},
};

use super::ontology::{
    CausalRelation, CausalRelationClass, DirectPathCertificate, MediatedPathCertificate,
    PromotionCertificate, TransferDecision,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DecompositionMethod {
    ConditionalPath0,
    ConditionalPath2,
    ConditionalPath4,
    ConditionalPath8,
    ConditionalPath16,
    ConditionalPath32,
    ConditionalPath64,
}

impl DecompositionMethod {
    pub const CANDIDATES: [Self; 7] = [
        Self::ConditionalPath0,
        Self::ConditionalPath2,
        Self::ConditionalPath4,
        Self::ConditionalPath8,
        Self::ConditionalPath16,
        Self::ConditionalPath32,
        Self::ConditionalPath64,
    ];

    const fn threshold(self) -> f64 {
        match self {
            Self::ConditionalPath0 => 0.0,
            Self::ConditionalPath2 => 2.0,
            Self::ConditionalPath4 => 4.0,
            Self::ConditionalPath8 => 8.0,
            Self::ConditionalPath16 => 16.0,
            Self::ConditionalPath32 => 32.0,
            Self::ConditionalPath64 => 64.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalBatch {
    pub method: DecompositionMethod,
    pub predictions: Vec<Value>,
    pub prediction_commitment: String,
    pub relations: Vec<CausalRelation>,
    pub direct_certificates: Vec<DirectPathCertificate>,
    pub mediated_certificates: Vec<MediatedPathCertificate>,
    pub causal_tests_performed: u64,
    pub conditional_ablation_evaluations: u64,
    pub mediator_hypotheses_considered: u64,
    pub direct_hypotheses_considered: u64,
    pub active_semantic_nodes: u64,
    pub world_memory_full_scans: u64,
    pub causal_mechanism_full_scans: u64,
    pub temporal_memory_full_scans: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TransferPolicy {
    pub method: TransferMethod,
    pub margin_basis_points: u64,
}

impl TransferPolicy {
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
                    .map(move |margin_basis_points| Self {
                        method,
                        margin_basis_points,
                    })
            })
            .collect()
    }

    pub fn id(self) -> String {
        format!("{:?}_MARGIN_{}", self.method, self.margin_basis_points).to_uppercase()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferBatch {
    pub policy: TransferPolicy,
    pub predictions: Vec<Value>,
    pub prediction_commitment: String,
    pub transfer_promotion_evaluations: u64,
    pub promoted: u64,
    pub abstained: u64,
    pub rejected: u64,
}

pub fn predict_causal(
    cases: &[(R1CaseDescriptor, R1ExternalObservation)],
    method: DecompositionMethod,
) -> Result<CausalBatch, String> {
    let lane_a: Vec<_> = cases
        .iter()
        .filter(|(descriptor, _)| descriptor.lane == R1ExternalLane::A)
        .cloned()
        .collect();
    let r2 = r2_predict_lane_a(
        &lane_a,
        CausalPrecisionMethod::PairwiseTriadStableAblationMdl,
    )?;
    let r2_by_case: BTreeMap<String, BTreeSet<(usize, usize)>> = r2
        .predictions
        .iter()
        .map(|prediction| {
            let case_id = prediction["case_id"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let edges = prediction["edges"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|edge| {
                    Some((
                        edge.get(0)?.as_u64()? as usize,
                        edge.get(1)?.as_u64()? as usize,
                    ))
                })
                .collect();
            (case_id, edges)
        })
        .collect();
    let mut predictions = Vec::new();
    let mut all_relations = Vec::new();
    let mut all_direct_certificates = Vec::new();
    let mut all_mediated_certificates = Vec::new();
    let mut causal_tests = 0_u64;
    let mut conditional_tests = 0_u64;
    let mut mediator_hypotheses = 0_u64;
    let mut direct_hypotheses = 0_u64;
    let mut active_nodes = 0_u64;
    for (descriptor, observation) in &lane_a {
        let values = entity_series(observation, descriptor.entity_count as usize)?;
        let variables = descriptor.entity_count as usize;
        active_nodes += variables as u64;
        let direct_candidates = r2_by_case
            .get(&descriptor.case_id)
            .cloned()
            .unwrap_or_default();
        let mut support_edges = BTreeSet::new();
        let mut conditional_scores = BTreeMap::new();
        for source in 0..variables {
            for target in 0..variables {
                if source == target {
                    continue;
                }
                causal_tests += 1;
                let pairwise = stable_evidence(&values, source, target, false);
                let conditional = stable_evidence(&values, source, target, true);
                conditional_tests += 3;
                conditional_scores.insert((source, target), conditional);
                if pairwise > 0.0 {
                    support_edges.insert((source, target));
                }
            }
        }
        let mut relations = Vec::new();
        let mut direct_certificates = Vec::new();
        let mut mediated_certificates = Vec::new();
        for source in 0..variables {
            for target in 0..variables {
                if source == target {
                    continue;
                }
                let score = conditional_scores[&(source, target)];
                let path = alternate_path(&support_edges, source, target, variables);
                let common = common_causes(&support_edges, source, target, variables);
                mediator_hypotheses += u64::from(path.is_some());
                direct_hypotheses += u64::from(direct_candidates.contains(&(source, target)));
                let class = if direct_candidates.contains(&(source, target)) {
                    if path.is_some() && score <= method.threshold() {
                        CausalRelationClass::Mediated
                    } else if !common.is_empty() && score <= method.threshold() {
                        CausalRelationClass::Confounded
                    } else {
                        CausalRelationClass::Direct
                    }
                } else if path.is_some() {
                    CausalRelationClass::Mediated
                } else if !common.is_empty() {
                    CausalRelationClass::Confounded
                } else {
                    CausalRelationClass::Unresolved
                };
                let uncertainty = 1.0 / (1.0 + score.abs());
                let relation = CausalRelation {
                    source,
                    target,
                    class,
                    lag: 0,
                    evidence_score: finite_or(score, 0.0),
                    uncertainty,
                };
                match class {
                    CausalRelationClass::Direct => {
                        direct_certificates.push(DirectPathCertificate {
                            case_id: descriptor.case_id.clone(),
                            source,
                            target,
                            supporting_evidence: format!(
                                "STABLE_CONDITIONAL_MDL={:.12}",
                                finite_or(score, 0.0)
                            ),
                            competing_mediator_paths: path.clone().into_iter().collect(),
                            competing_common_cause_hypotheses: common.clone(),
                            intervention_observation_evidence: "THREE_DISJOINT_TEMPORAL_FOLDS"
                                .to_string(),
                            uncertainty,
                            promotion_rationale:
                                "DIRECT_SUPPORT_SURVIVED_MEDIATOR_AND_COMMON_CAUSE_COMPETITION"
                                    .to_string(),
                        })
                    }
                    CausalRelationClass::Mediated => {
                        if let Some(path) = path.clone() {
                            mediated_certificates.push(MediatedPathCertificate {
                                case_id: descriptor.case_id.clone(),
                                source,
                                mediator_path: path[1..path.len() - 1].to_vec(),
                                target,
                                semantic_temporal_ordering: "SOURCE_BEFORE_MEDIATOR_BEFORE_TARGET"
                                    .to_string(),
                                evidence: "ALTERNATE_STABLE_CAUSAL_PATH".to_string(),
                                uncertainty,
                                counterfactual_implication:
                                    "BLOCKING_MEDIATOR_PATH_SHOULD_ATTENUATE_TOTAL_EFFECT"
                                        .to_string(),
                            });
                        }
                    }
                    _ => {}
                }
                relations.push(relation);
            }
        }
        let prediction = json!({
            "case_id": descriptor.case_id,
            "relations": relations,
            "causal_path_certificates": direct_certificates,
            "mediation_path_certificates": mediated_certificates,
            "total_effect_used_as_direct_edge_authority": false,
            "topology_template_to_causal_class_authority": false,
            "benchmark_id_to_causal_class_authority": false,
            "lag_used_as_mediator_authority": false
        });
        all_relations.extend(relations);
        all_direct_certificates.extend(direct_certificates);
        all_mediated_certificates.extend(mediated_certificates);
        predictions.push(prediction);
    }
    let commitment = prediction_commitment(&predictions)?;
    Ok(CausalBatch {
        method,
        predictions,
        prediction_commitment: commitment,
        relations: all_relations,
        direct_certificates: all_direct_certificates,
        mediated_certificates: all_mediated_certificates,
        causal_tests_performed: causal_tests,
        conditional_ablation_evaluations: conditional_tests,
        mediator_hypotheses_considered: mediator_hypotheses,
        direct_hypotheses_considered: direct_hypotheses,
        active_semantic_nodes: active_nodes,
        world_memory_full_scans: 0,
        causal_mechanism_full_scans: 0,
        temporal_memory_full_scans: 0,
    })
}

pub fn predict_transfer(
    cases: &[(R1CaseDescriptor, R1ExternalObservation)],
    policy: TransferPolicy,
) -> Result<TransferBatch, String> {
    let lane_b: Vec<_> = cases
        .iter()
        .filter(|(descriptor, _)| descriptor.lane == R1ExternalLane::B)
        .cloned()
        .collect();
    let no_change = r1_predict_batch(
        &lane_b,
        R1ExternalLane::B,
        TransferMethod::NoChange,
        TransferResearchMode::Full,
    )?;
    let candidate_methods = [
        TransferMethod::Persistence,
        TransferMethod::IndependentLinear,
        TransferMethod::SparseCoupledLinear,
        TransferMethod::DenseCoupledLinear,
        TransferMethod::InterventionRegression,
        TransferMethod::HybridMechanism,
    ];
    let method_batches: BTreeMap<TransferMethod, Vec<Value>> = candidate_methods
        .into_iter()
        .map(|method| {
            r1_predict_batch(
                &lane_b,
                R1ExternalLane::B,
                method,
                TransferResearchMode::Full,
            )
            .map(|batch| (method, batch.predictions))
        })
        .collect::<Result<_, _>>()?;
    let no_change_by_case: BTreeMap<_, _> = no_change
        .predictions
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
    let mut predictions = Vec::new();
    let mut promoted = 0;
    let mut abstained = 0;
    let mut rejected = 0;
    for ((descriptor, observation), index) in lane_b.iter().zip(0..) {
        let values = raw_series(observation)?;
        let gain = validation_gain(&values, policy.method);
        let margin = policy.margin_basis_points as f64 / 10_000.0;
        let decision = if gain > margin {
            TransferDecision::Promote
        } else if gain < -margin {
            TransferDecision::Reject
        } else {
            TransferDecision::Abstain
        };
        promoted += u64::from(decision == TransferDecision::Promote);
        abstained += u64::from(decision == TransferDecision::Abstain);
        rejected += u64::from(decision == TransferDecision::Reject);
        let no_change_prediction = &no_change_by_case[&descriptor.case_id];
        let selected = &method_batches[&policy.method][index];
        let selected_bits = if decision == TransferDecision::Promote {
            selected["predicted_y_ieee754_bits"].clone()
        } else {
            no_change_prediction["predicted_y_ieee754_bits"].clone()
        };
        let alternatives: Vec<Value> = candidate_methods
            .iter()
            .map(|method| {
                json!({
                    "method": method,
                    "predicted_y_ieee754_bits":
                        method_batches[method][index]["predicted_y_ieee754_bits"]
                })
            })
            .collect();
        let certificate = PromotionCertificate {
            case_id: descriptor.case_id.clone(),
            expected_benefit: finite_or(gain, 0.0),
            uncertainty: 1.0 / (1.0 + gain.abs() * 10.0),
            known_negative_evidence: if gain < 0.0 {
                "TARGET_HELD_OUT_RESIDUAL_WORSE_THAN_NO_CHANGE"
            } else {
                "NONE_OBSERVED_IN_TARGET_HELD_OUT_PREFIX"
            }
            .to_string(),
            applicability_conditions: "TARGET_PREFIX_VALIDATION_AND_LEGAL_INTERVENTION_CONTRACT"
                .to_string(),
            counterfactual_no_change_expectation: "EXPLICIT_NO_CHANGE_PREDICTION_PRESERVED"
                .to_string(),
            possible_downside: "INTERVENTION_RESPONSE_MAY_DIFFER_FROM_OBSERVATIONAL_DYNAMICS"
                .to_string(),
            reason_abstention_was_rejected: if decision == TransferDecision::Promote {
                "EXPECTED_GAIN_EXCEEDED_FROZEN_MARGIN"
            } else {
                "ABSTENTION_NOT_REJECTED"
            }
            .to_string(),
        };
        predictions.push(json!({
            "case_id": descriptor.case_id,
            "decision": decision,
            "selected_method": policy.method,
            "predicted_y_ieee754_bits": selected_bits,
            "alternatives": alternatives,
            "promotion_certificate": certificate,
            "similarity_only_authority": false,
            "benchmark_identity_authority": false,
            "no_change_baseline_preserved": true
        }));
    }
    let commitment = prediction_commitment(&predictions)?;
    Ok(TransferBatch {
        policy,
        predictions,
        prediction_commitment: commitment,
        transfer_promotion_evaluations: lane_b.len() as u64,
        promoted,
        abstained,
        rejected,
    })
}

pub fn no_change_predictions(
    cases: &[(R1CaseDescriptor, R1ExternalObservation)],
) -> Result<Vec<Value>, String> {
    Ok(r1_predict_batch(
        cases,
        R1ExternalLane::B,
        TransferMethod::NoChange,
        TransferResearchMode::Full,
    )?
    .predictions)
}

pub fn always_abstain_from(
    batch: &TransferBatch,
    no_change: &[Value],
) -> Result<TransferBatch, String> {
    let no_change_by_case: BTreeMap<_, _> = no_change
        .iter()
        .map(|prediction| {
            (
                prediction["case_id"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                prediction,
            )
        })
        .collect();
    let predictions: Vec<Value> = batch
        .predictions
        .iter()
        .map(|prediction| {
            let case_id = prediction["case_id"].as_str().unwrap_or_default();
            json!({
                "case_id": case_id,
                "decision": TransferDecision::Abstain,
                "selected_method": TransferMethod::NoChange,
                "predicted_y_ieee754_bits": no_change_by_case[case_id]["predicted_y_ieee754_bits"],
                "alternatives": prediction["alternatives"],
                "promotion_certificate": {
                    "case_id": case_id,
                    "expected_benefit": 0.0,
                    "uncertainty": 1.0,
                    "known_negative_evidence": "ALWAYS_ABSTAIN_CONTROL",
                    "applicability_conditions": "CONTROL_ONLY",
                    "counterfactual_no_change_expectation": "SELECTED",
                    "possible_downside": "MISSES_BENEFICIAL_TRANSFER",
                    "reason_abstention_was_rejected": "NOT_REJECTED"
                }
            })
        })
        .collect();
    Ok(TransferBatch {
        policy: TransferPolicy {
            method: TransferMethod::NoChange,
            margin_basis_points: u64::MAX,
        },
        prediction_commitment: prediction_commitment(&predictions)?,
        transfer_promotion_evaluations: predictions.len() as u64,
        promoted: 0,
        abstained: predictions.len() as u64,
        rejected: 0,
        predictions,
    })
}

pub fn always_promote_from(batch: &TransferBatch) -> Result<TransferBatch, String> {
    let method_name =
        serde_json::to_value(batch.policy.method).map_err(|error| error.to_string())?;
    let predictions: Vec<Value> = batch
        .predictions
        .iter()
        .map(|prediction| {
            let selected = prediction["alternatives"]
                .as_array()
                .and_then(|alternatives| {
                    alternatives
                        .iter()
                        .find(|alternative| alternative["method"] == method_name)
                })
                .map(|alternative| alternative["predicted_y_ieee754_bits"].clone())
                .unwrap_or_else(|| prediction["predicted_y_ieee754_bits"].clone());
            let mut promoted = prediction.clone();
            promoted["decision"] = json!(TransferDecision::Promote);
            promoted["predicted_y_ieee754_bits"] = selected;
            promoted
        })
        .collect();
    Ok(TransferBatch {
        policy: batch.policy,
        prediction_commitment: prediction_commitment(&predictions)?,
        transfer_promotion_evaluations: predictions.len() as u64,
        promoted: predictions.len() as u64,
        abstained: 0,
        rejected: 0,
        predictions,
    })
}

fn entity_series(
    observation: &R1ExternalObservation,
    entity_count: usize,
) -> Result<Vec<Vec<f64>>, String> {
    let raw = raw_series(observation)?;
    let mut by_entity = vec![Vec::new(); entity_count];
    for (column, binding) in observation.bindings.iter().enumerate() {
        if let Some(columns) = by_entity.get_mut(binding.entity as usize) {
            columns.push(column);
        }
    }
    Ok(raw
        .iter()
        .map(|row| {
            by_entity
                .iter()
                .map(|columns| {
                    if columns.is_empty() {
                        0.0
                    } else {
                        columns.iter().map(|column| row[*column]).sum::<f64>()
                            / columns.len() as f64
                    }
                })
                .collect()
        })
        .collect())
}

fn raw_series(observation: &R1ExternalObservation) -> Result<Vec<Vec<f64>>, String> {
    if observation.nonfinite_cells_have_numeric_authority
        || observation.missingness_transport != "EXPLICIT_NULL_NO_SENTINEL"
    {
        return Err("SEM37_R3_INVALID_NUMERIC_TRANSPORT".to_string());
    }
    let columns = observation.bindings.len();
    if columns == 0 || observation.values_ieee754_bits.len() < 12 {
        return Err("SEM37_R3_INSUFFICIENT_OBSERVATION".to_string());
    }
    let mut values = vec![vec![0.0; columns]; observation.values_ieee754_bits.len()];
    let mut last = vec![None; columns];
    for (time, row) in observation.values_ieee754_bits.iter().enumerate() {
        if row.len() != columns {
            return Err("SEM37_R3_OBSERVATION_ARITY_DRIFT".to_string());
        }
        for (column, cell) in row.iter().enumerate() {
            if let Some(bits) = cell {
                let value = f64::from_bits(*bits);
                if !value.is_finite() || value.to_bits() != *bits {
                    return Err("SEM37_R3_NUMERIC_AUTHORITY_VIOLATION".to_string());
                }
                values[time][column] = value;
                last[column] = Some(value);
            } else {
                values[time][column] = last[column].unwrap_or(0.0);
            }
        }
    }
    Ok(values)
}

fn stable_evidence(values: &[Vec<f64>], source: usize, target: usize, conditional: bool) -> f64 {
    let evidence = [
        mdl_evidence(values, source, target, conditional, 0),
        mdl_evidence(values, source, target, conditional, 1),
        mdl_evidence(values, source, target, conditional, 2),
    ];
    let mut sorted = evidence;
    sorted.sort_by(f64::total_cmp);
    if evidence.iter().filter(|value| **value > 0.0).count() >= 2 {
        sorted[1]
    } else {
        sorted[1].min(0.0)
    }
}

fn mdl_evidence(
    values: &[Vec<f64>],
    source: usize,
    target: usize,
    conditional: bool,
    fold: usize,
) -> f64 {
    let rows = values.len() - 1;
    let start = rows * fold / 3;
    let end = rows * (fold + 1) / 3;
    let mut full = Vec::new();
    let mut reduced = Vec::new();
    let mut outcomes = Vec::new();
    let mut validation = Vec::new();
    for time in 1..values.len() {
        let state = &values[time - 1];
        let reduced_sources: Vec<usize> = if conditional {
            (0..state.len())
                .filter(|candidate| *candidate != target && *candidate != source)
                .collect()
        } else {
            Vec::new()
        };
        let mut full_sources = reduced_sources.clone();
        full_sources.push(source);
        reduced.push(feature_row(state, target, &reduced_sources));
        full.push(feature_row(state, target, &full_sources));
        outcomes.push(values[time][target] - state[target]);
        validation.push(time > start && time - 1 < end);
    }
    let full_sse = held_out_sse(&full, &outcomes, &validation);
    let reduced_sse = held_out_sse(&reduced, &outcomes, &validation);
    let n = validation.iter().filter(|value| **value).count().max(2) as f64;
    n * ((reduced_sse + 1.0e-18) / (full_sse + 1.0e-18)).ln() - 4.0 * n.ln()
}

fn feature_row(state: &[f64], target: usize, sources: &[usize]) -> Vec<f64> {
    let target_value = state[target];
    let mut row = vec![
        1.0,
        target_value,
        target_value * target_value,
        target_value * target_value * target_value,
    ];
    for source in sources {
        let value = state[*source];
        row.extend([
            value,
            value * value,
            value * value * value,
            value * target_value,
        ]);
    }
    row
}

fn held_out_sse(features: &[Vec<f64>], outcomes: &[f64], validation: &[bool]) -> f64 {
    let train_x: Vec<Vec<f64>> = features
        .iter()
        .zip(validation)
        .filter(|(_, held_out)| !**held_out)
        .map(|(row, _)| row.clone())
        .collect();
    let train_y: Vec<f64> = outcomes
        .iter()
        .zip(validation)
        .filter(|(_, held_out)| !**held_out)
        .map(|(value, _)| *value)
        .collect();
    let coefficients = fit_linear(&train_x, &train_y, 1.0e-6);
    features
        .iter()
        .zip(outcomes)
        .zip(validation)
        .filter(|(_, held_out)| **held_out)
        .map(|((row, outcome), _)| {
            let residual = outcome - dot(row, &coefficients);
            residual * residual
        })
        .filter(|value| value.is_finite())
        .sum()
}

fn alternate_path(
    edges: &BTreeSet<(usize, usize)>,
    source: usize,
    target: usize,
    variables: usize,
) -> Option<Vec<usize>> {
    let mut frontier = vec![vec![source]];
    let mut best_depth = vec![usize::MAX; variables];
    best_depth[source] = 0;
    while let Some(path) = frontier.pop() {
        let node = *path.last()?;
        for (left, right) in edges {
            if *left != node || (*left == source && *right == target) || path.contains(right) {
                continue;
            }
            let mut next = path.clone();
            next.push(*right);
            if *right == target && next.len() >= 3 {
                return Some(next);
            }
            if next.len() < best_depth[*right] && next.len() <= variables {
                best_depth[*right] = next.len();
                frontier.push(next);
            }
        }
    }
    None
}

fn common_causes(
    edges: &BTreeSet<(usize, usize)>,
    source: usize,
    target: usize,
    variables: usize,
) -> Vec<usize> {
    (0..variables)
        .filter(|common| {
            *common != source
                && *common != target
                && edges.contains(&(*common, source))
                && edges.contains(&(*common, target))
        })
        .collect()
}

fn validation_gain(values: &[Vec<f64>], method: TransferMethod) -> f64 {
    if values.len() < 12 || values[0].is_empty() {
        return 0.0;
    }
    let split = values.len() * 2 / 3;
    let persistence = (split..values.len())
        .flat_map(|time| {
            values[time]
                .iter()
                .zip(&values[time - 1])
                .map(|(current, previous)| (current - previous).powi(2))
        })
        .sum::<f64>();
    let independent = method == TransferMethod::IndependentLinear;
    let mut model_sse = 0.0;
    for target in 0..values[0].len() {
        let features: Vec<Vec<f64>> = (1..split)
            .map(|time| {
                if independent {
                    vec![1.0, values[time - 1][target]]
                } else {
                    let mut row = vec![1.0];
                    row.extend_from_slice(&values[time - 1]);
                    row
                }
            })
            .collect();
        let outcomes: Vec<f64> = (1..split).map(|time| values[time][target]).collect();
        let coefficients = fit_linear(&features, &outcomes, 1.0e-4);
        for time in split..values.len() {
            let row = if independent {
                vec![1.0, values[time - 1][target]]
            } else {
                let mut row = vec![1.0];
                row.extend_from_slice(&values[time - 1]);
                row
            };
            let residual = values[time][target] - dot(&row, &coefficients);
            model_sse += residual * residual;
        }
    }
    let complexity_penalty = match method {
        TransferMethod::DenseCoupledLinear => 0.03,
        TransferMethod::InterventionRegression => 0.05,
        TransferMethod::HybridMechanism => 0.02,
        _ => 0.0,
    };
    ((persistence - model_sse) / persistence.max(1.0e-18)) - complexity_penalty
}

fn fit_linear(features: &[Vec<f64>], outcomes: &[f64], ridge: f64) -> Vec<f64> {
    let dimensions = features.first().map_or(0, Vec::len);
    if dimensions == 0 || features.len() != outcomes.len() {
        return vec![0.0; dimensions];
    }
    let mut normal = vec![vec![0.0; dimensions]; dimensions];
    let mut rhs = vec![0.0; dimensions];
    for (row, outcome) in features.iter().zip(outcomes) {
        for left in 0..dimensions {
            rhs[left] += row[left] * outcome;
            for right in 0..dimensions {
                normal[left][right] += row[left] * row[right];
            }
        }
    }
    for (index, diagonal) in normal.iter_mut().enumerate().skip(1) {
        diagonal[index] += ridge;
    }
    solve_linear(normal, rhs).unwrap_or_else(|| vec![0.0; dimensions])
}

#[allow(clippy::needless_range_loop)]
fn solve_linear(mut matrix: Vec<Vec<f64>>, mut rhs: Vec<f64>) -> Option<Vec<f64>> {
    let size = rhs.len();
    for pivot in 0..size {
        let best = (pivot..size).max_by(|left, right| {
            matrix[*left][pivot]
                .abs()
                .total_cmp(&matrix[*right][pivot].abs())
        })?;
        if matrix[best][pivot].abs() < 1.0e-12 {
            return None;
        }
        matrix.swap(pivot, best);
        rhs.swap(pivot, best);
        let scale = matrix[pivot][pivot];
        for column in pivot..size {
            matrix[pivot][column] /= scale;
        }
        rhs[pivot] /= scale;
        for row in 0..size {
            if row == pivot {
                continue;
            }
            let factor = matrix[row][pivot];
            for column in pivot..size {
                matrix[row][column] -= factor * matrix[pivot][column];
            }
            rhs[row] -= factor * rhs[pivot];
        }
    }
    rhs.iter().all(|value| value.is_finite()).then_some(rhs)
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

fn finite_or(value: f64, fallback: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alternate_path_is_not_the_direct_edge_itself() {
        let edges = BTreeSet::from([(0, 1), (1, 2), (0, 2)]);
        assert_eq!(alternate_path(&edges, 0, 2, 3), Some(vec![0, 1, 2]));
    }

    #[test]
    fn policy_bank_is_multi_objective_and_bounded() {
        assert_eq!(TransferPolicy::candidates().len(), 25);
        assert!(TransferPolicy::candidates().len() < 4096);
    }
}
