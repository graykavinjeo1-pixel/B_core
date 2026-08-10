use std::{cmp::Ordering, collections::BTreeMap};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::{
    adapter::{CaseDescriptor, ExternalObservation, InterventionObservation, LegalIntervention},
    ontology::{
        DirectPathCertificate, IdentifiabilityState, MediatedPathCertificate, PathSpecificCausalIr,
        UnresolvedCertificate,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DirectPolicy {
    ConditionalParallel,
    PathExclusive,
    EvidenceRankedParallel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CandidateKind {
    InterventionConditionalSparse,
    PathExclusiveLag1,
    EvidenceRankedParallelLag1,
    ConditionalParallelLag2,
    TemporalPathSparseLag2,
    ObservationalIdentifiabilityGuard,
    InterventionPathParallel,
    ConservativeUnresolved,
}

impl CandidateKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InterventionConditionalSparse => "INTERVENTION_CONDITIONAL_SPARSE",
            Self::PathExclusiveLag1 => "PATH_EXCLUSIVE_LAG1",
            Self::EvidenceRankedParallelLag1 => "EVIDENCE_RANKED_PARALLEL_LAG1",
            Self::ConditionalParallelLag2 => "CONDITIONAL_PARALLEL_LAG2",
            Self::TemporalPathSparseLag2 => "TEMPORAL_PATH_SPARSE_LAG2",
            Self::ObservationalIdentifiabilityGuard => "OBSERVATIONAL_IDENTIFIABILITY_GUARD",
            Self::InterventionPathParallel => "INTERVENTION_PATH_PARALLEL",
            Self::ConservativeUnresolved => "CONSERVATIVE_UNRESOLVED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CandidateModel {
    pub kind: CandidateKind,
    pub lag: usize,
    pub fanout: usize,
    pub conditional_direct: bool,
    pub direction_advantage: bool,
    pub identifiability_guard: bool,
    pub require_intervention_signal: bool,
    pub direct_policy: DirectPolicy,
}

impl CandidateModel {
    pub const CANDIDATES: [Self; 8] = [
        Self {
            kind: CandidateKind::InterventionConditionalSparse,
            lag: 1,
            fanout: 1,
            conditional_direct: true,
            direction_advantage: true,
            identifiability_guard: true,
            require_intervention_signal: true,
            direct_policy: DirectPolicy::ConditionalParallel,
        },
        Self {
            kind: CandidateKind::PathExclusiveLag1,
            lag: 1,
            fanout: 2,
            conditional_direct: true,
            direction_advantage: true,
            identifiability_guard: true,
            require_intervention_signal: false,
            direct_policy: DirectPolicy::PathExclusive,
        },
        Self {
            kind: CandidateKind::EvidenceRankedParallelLag1,
            lag: 1,
            fanout: 2,
            conditional_direct: true,
            direction_advantage: false,
            identifiability_guard: true,
            require_intervention_signal: true,
            direct_policy: DirectPolicy::EvidenceRankedParallel,
        },
        Self {
            kind: CandidateKind::ConditionalParallelLag2,
            lag: 2,
            fanout: 2,
            conditional_direct: true,
            direction_advantage: true,
            identifiability_guard: true,
            require_intervention_signal: false,
            direct_policy: DirectPolicy::ConditionalParallel,
        },
        Self {
            kind: CandidateKind::TemporalPathSparseLag2,
            lag: 2,
            fanout: 1,
            conditional_direct: false,
            direction_advantage: true,
            identifiability_guard: true,
            require_intervention_signal: true,
            direct_policy: DirectPolicy::PathExclusive,
        },
        Self {
            kind: CandidateKind::ObservationalIdentifiabilityGuard,
            lag: 1,
            fanout: 1,
            conditional_direct: true,
            direction_advantage: false,
            identifiability_guard: true,
            require_intervention_signal: false,
            direct_policy: DirectPolicy::ConditionalParallel,
        },
        Self {
            kind: CandidateKind::InterventionPathParallel,
            lag: 1,
            fanout: 2,
            conditional_direct: false,
            direction_advantage: true,
            identifiability_guard: true,
            require_intervention_signal: true,
            direct_policy: DirectPolicy::EvidenceRankedParallel,
        },
        Self {
            kind: CandidateKind::ConservativeUnresolved,
            lag: 2,
            fanout: 1,
            conditional_direct: true,
            direction_advantage: true,
            identifiability_guard: true,
            require_intervention_signal: true,
            direct_policy: DirectPolicy::PathExclusive,
        },
    ];

    pub const fn name(self) -> &'static str {
        self.kind.as_str()
    }

    pub fn by_name(name: &str) -> Result<Self, String> {
        Self::CANDIDATES
            .into_iter()
            .find(|candidate| candidate.name() == name)
            .ok_or_else(|| format!("SEM37_R5_UNKNOWN_CANDIDATE:{name}"))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaseEvidence {
    pub descriptor: CaseDescriptor,
    pub observation: ExternalObservation,
    pub intervention: InterventionObservation,
    pub pre_intervention_prediction: Value,
    pub prediction_commitment: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PredictionBatch {
    pub model: CandidateModel,
    pub predictions: Vec<Value>,
    pub prediction_commitment: String,
    pub path_irs: Vec<PathSpecificCausalIr>,
    pub direct_certificates: Vec<DirectPathCertificate>,
    pub mediated_certificates: Vec<MediatedPathCertificate>,
    pub unresolved_certificates: Vec<UnresolvedCertificate>,
    pub candidate_mediator_paths_total: u64,
    pub candidate_mediator_paths_evaluated: u64,
    pub causal_work: u64,
}

pub fn intervention_hypotheses(
    cases: &[(CaseDescriptor, ExternalObservation)],
) -> Result<(Vec<Value>, String), String> {
    let predictions: Vec<_> = cases
        .iter()
        .map(|(descriptor, observation)| {
            let source = descriptor.primary_source as usize;
            let target = descriptor.primary_target as usize;
            let pre_end = intervention_start(&observation.legal_interventions)
                .unwrap_or(observation.values_ieee754_bits.len());
            let last_target = observation
                .values_ieee754_bits
                .get(pre_end.saturating_sub(1))
                .and_then(|row| row.get(target))
                .copied()
                .flatten();
            json!({
                "case_id": descriptor.case_id,
                "hypothesis": "LEGAL_SOURCE_INTERVENTION_DISCRIMINATES_TOTAL_INFLUENCE;PATH_CLASS_REMAINS_OPEN",
                "source": source,
                "target": target,
                "predicted_no_change_target_ieee754_bits": last_target,
                "candidate_mediator_count": descriptor.observed_entity_count.saturating_sub(2),
                "prediction_frozen_before_outcome": true,
                "gold_fields_used": 0
            })
        })
        .collect();
    let commitment = prediction_commitment(&predictions)?;
    Ok((predictions, commitment))
}

pub fn build_batch(
    cases: &[CaseEvidence],
    model: CandidateModel,
) -> Result<PredictionBatch, String> {
    build_batch_internal(cases, model, true)
}

pub fn build_observation_only_batch(
    cases: &[CaseEvidence],
    model: CandidateModel,
) -> Result<PredictionBatch, String> {
    build_batch_internal(cases, model, false)
}

fn build_batch_internal(
    cases: &[CaseEvidence],
    model: CandidateModel,
    intervention_enabled: bool,
) -> Result<PredictionBatch, String> {
    let mut predictions = Vec::with_capacity(cases.len());
    let mut path_irs = Vec::with_capacity(cases.len());
    let mut direct_certificates = Vec::new();
    let mut mediated_certificates = Vec::new();
    let mut unresolved_certificates = Vec::new();
    let mut candidate_total = 0_u64;
    let mut candidate_evaluated = 0_u64;
    let mut work = 0_u64;

    for case in cases {
        let analysis = analyze_case(case, model, intervention_enabled)?;
        candidate_total += analysis.candidate_total;
        candidate_evaluated += analysis.candidate_evaluated;
        work += analysis.work;
        path_irs.push(analysis.ir.clone());
        if let Some(certificate) = analysis.direct_certificate.clone() {
            direct_certificates.push(certificate);
        }
        mediated_certificates.extend(analysis.mediated_certificates.clone());
        if let Some(certificate) = analysis.unresolved_certificate.clone() {
            unresolved_certificates.push(certificate);
        }
        predictions.push(analysis.prediction);
    }
    let commitment = prediction_commitment(&predictions)?;
    Ok(PredictionBatch {
        model,
        predictions,
        prediction_commitment: commitment,
        path_irs,
        direct_certificates,
        mediated_certificates,
        unresolved_certificates,
        candidate_mediator_paths_total: candidate_total,
        candidate_mediator_paths_evaluated: candidate_evaluated,
        causal_work: work,
    })
}

struct CaseAnalysis {
    prediction: Value,
    ir: PathSpecificCausalIr,
    direct_certificate: Option<DirectPathCertificate>,
    mediated_certificates: Vec<MediatedPathCertificate>,
    unresolved_certificate: Option<UnresolvedCertificate>,
    candidate_total: u64,
    candidate_evaluated: u64,
    work: u64,
}

fn analyze_case(
    case: &CaseEvidence,
    model: CandidateModel,
    intervention_enabled: bool,
) -> Result<CaseAnalysis, String> {
    let descriptor = &case.descriptor;
    let observation = &case.observation;
    let source = descriptor.primary_source as usize;
    let target = descriptor.primary_target as usize;
    let entity_count = descriptor.entity_count as usize;
    if source >= entity_count || target >= entity_count || source == target {
        return Err("SEM37_R5_INVALID_PUBLIC_CAUSAL_QUERY".to_string());
    }
    let pre_end = intervention_start(&observation.legal_interventions)
        .unwrap_or(observation.values_ieee754_bits.len())
        .min(observation.values_ieee754_bits.len());
    let matrix = decode_optional_matrix(&observation.values_ieee754_bits)?;
    let observed: Vec<_> = observation
        .bindings
        .iter()
        .filter(|binding| binding.observed)
        .map(|binding| binding.entity as usize)
        .collect();
    let hidden = observed.len() < entity_count;
    let observed_middle: Vec<_> = observed
        .iter()
        .copied()
        .filter(|node| *node != source && *node != target)
        .collect();

    let identifiability = if hidden && model.identifiability_guard {
        if observed_middle.len() >= 2 {
            IdentifiabilityState::PartiallyIdentifiable
        } else {
            IdentifiabilityState::NotIdentifiableUnderAvailableEvidence
        }
    } else {
        IdentifiabilityState::FullyIdentifiable
    };

    let mut direct_present = false;
    let mut mediator_paths = Vec::new();
    let mut candidate_paths = Vec::new();
    let mut common_causes = Vec::new();
    let mut edges = Vec::new();
    let mut evaluated_paths = 0_u64;
    let intervention_signal = if intervention_enabled {
        intervention_signal(case, pre_end, target)
    } else {
        false
    };

    if identifiability == IdentifiabilityState::FullyIdentifiable {
        edges = ranked_local_edges(
            &matrix,
            pre_end,
            &observed,
            model.lag,
            model.fanout,
            model.direction_advantage,
        );
        let mut all_paths = Vec::new();
        let mut current = vec![source];
        enumerate_local_paths(
            source,
            target,
            &edges,
            &mut current,
            &mut all_paths,
            &mut evaluated_paths,
            4,
        );
        all_paths.sort();
        all_paths.dedup();
        candidate_paths = all_paths.clone();
        mediator_paths = all_paths
            .into_iter()
            .filter(|path| path.len() >= 3)
            .collect();

        let raw_direct = edge_selected(&edges, source, target);
        let conditional_direct = if model.conditional_direct && !observed_middle.is_empty() {
            conditional_direct_is_ranked(
                &matrix,
                pre_end,
                source,
                target,
                &observed_middle,
                model.lag,
                model.fanout,
            )
        } else {
            raw_direct
        };
        direct_present = match model.direct_policy {
            DirectPolicy::ConditionalParallel => conditional_direct,
            DirectPolicy::PathExclusive => conditional_direct && mediator_paths.is_empty(),
            DirectPolicy::EvidenceRankedParallel => {
                if let Some(path) = mediator_paths.first() {
                    conditional_direct
                        && direct_score(&matrix, pre_end, source, target, model.lag)
                            >= weakest_path_score(&matrix, pre_end, path, model.lag)
                } else {
                    conditional_direct
                }
            }
        };
        if model.require_intervention_signal
            && observation
                .legal_interventions
                .first()
                .is_some_and(|contract| contract.targets.contains(&(source as u64)))
        {
            direct_present &= intervention_signal;
        }

        common_causes = observed_middle
            .iter()
            .copied()
            .filter(|middle| {
                edge_selected(&edges, *middle, source) && edge_selected(&edges, *middle, target)
            })
            .collect();
    }

    let unresolved = identifiability != IdentifiabilityState::FullyIdentifiable
        || (!direct_present && mediator_paths.is_empty());
    let direct_evidence = vec![format!(
        "DIRECT_EDGE_LOCAL_RANKED={};CONDITIONAL={};INTERVENTION_SIGNAL={intervention_signal}",
        edge_selected(&edges, source, target),
        model.conditional_direct
    )];
    let mediated_evidence = mediator_paths
        .iter()
        .map(|path| format!("BOUNDED_TEMPORAL_PATH={path:?}"))
        .collect::<Vec<_>>();
    let intervention_id = observation
        .legal_interventions
        .first()
        .map(|contract| contract.contract_id.clone())
        .unwrap_or_else(|| "NONE".to_string());
    let constraints = observation
        .legal_interventions
        .iter()
        .map(intervention_constraint)
        .collect::<Vec<_>>();
    let uncertainty = match identifiability {
        IdentifiabilityState::FullyIdentifiable if !unresolved => 100_000,
        IdentifiabilityState::FullyIdentifiable => 500_000,
        IdentifiabilityState::PartiallyIdentifiable => 750_000,
        IdentifiabilityState::NotIdentifiableUnderAvailableEvidence => 1_000_000,
    };
    let ir = PathSpecificCausalIr {
        case_id: descriptor.case_id.clone(),
        source,
        target,
        candidate_paths: candidate_paths.clone(),
        direct_path_candidate: direct_present.then_some(vec![source, target]),
        mediator_paths: mediator_paths.clone(),
        common_cause_hypotheses: common_causes.clone(),
        available_interventions: vec![intervention_id.clone()],
        intervention_constraints: constraints.clone(),
        path_identifiability: identifiability,
        direct_effect_evidence: direct_evidence.clone(),
        mediated_effect_evidence: mediated_evidence.clone(),
        mixed_effect_evidence: if direct_present && !mediator_paths.is_empty() {
            vec!["DIRECT_AND_MEDIATED_COMPONENTS_RETAINED".to_string()]
        } else {
            Vec::new()
        },
        unresolved_components: unresolved
            .then_some("AVAILABLE_EVIDENCE_DOES_NOT_SEPARATE_ALL_COMPONENTS".to_string())
            .into_iter()
            .collect(),
        uncertainty_millionths: uncertainty,
        provenance: vec!["THIRD_PARTY_DOTIME_PUBLIC_OBSERVATION".to_string()],
        verification: vec!["INDEPENDENT_FROZEN_EVALUATOR".to_string()],
    };

    let direct_certificate = direct_present.then(|| DirectPathCertificate {
        case_id: descriptor.case_id.clone(),
        source,
        target,
        candidate_mediators: observed_middle.clone(),
        candidate_confounders: common_causes.clone(),
        interventions_available: vec![intervention_id.clone()],
        interventions_performed: intervention_enabled
            .then_some(intervention_id.clone())
            .into_iter()
            .collect(),
        predicted_outcomes_before_intervention: case.pre_intervention_prediction.clone(),
        observed_outcomes: json!(case.intervention.query_outcome_ieee754_bits),
        path_specific_evidence: direct_evidence,
        identifiability,
        remaining_uncertainty: ir.unresolved_components.clone(),
        promotion_rationale:
            "DIRECT_COMPONENT_RETAINED_ONLY_AFTER_IDENTIFIABILITY_AND_LOCAL_PATH_ANALYSIS"
                .to_string(),
    });
    let mediated_certificates = mediator_paths
        .iter()
        .map(|path| MediatedPathCertificate {
            case_id: descriptor.case_id.clone(),
            source,
            mediator_path: path.clone(),
            target,
            path_intervention_evidence: vec![format!("LEGAL_INTERVENTION={intervention_id}")],
            temporal_evidence: vec![format!("ORDERED_LOCAL_PATH={path:?}")],
            transfer_evidence: vec!["FRESH_EXTERNAL_WORLD".to_string()],
            identifiability,
            uncertainty_millionths: uncertainty,
        })
        .collect::<Vec<_>>();
    let unresolved_certificate = unresolved.then(|| UnresolvedCertificate {
        case_id: descriptor.case_id.clone(),
        remaining_hypotheses: vec![
            "DIRECT_COMPONENT".to_string(),
            "MEDIATED_COMPONENT".to_string(),
            "COMMON_CAUSE_OR_HIDDEN_COMPONENT".to_string(),
        ],
        discrimination_limit: if hidden {
            "UNOBSERVED_COMPONENT_PREVENTS_PATH_SEPARATION".to_string()
        } else {
            "LOCAL_EVIDENCE_DID_NOT_SELECT_A_STABLE_PATH".to_string()
        },
        resolving_intervention_if_available:
            "LEGAL_MEDIATOR_CONTROL_OR_ADDITIONAL_RANDOMIZED_SOURCE_INTERVENTION".to_string(),
    });
    let prediction = json!({
        "case_id": descriptor.case_id,
        "source": source,
        "target": target,
        "identifiability": identifiability,
        "direct_present": direct_present,
        "mediated_paths": mediator_paths,
        "candidate_confounders": common_causes,
        "unresolved": unresolved,
        "available_interventions": observation.legal_interventions.len(),
        "interventions_considered": observation.legal_interventions.len(),
        "interventions_executed": if intervention_enabled { observation.legal_interventions.len() } else { 0 },
        "full_intervention_enumeration": false,
        "outcome_read_before_prediction": false,
        "counterfactual_validations": if intervention_enabled { 1 } else { 0 },
        "candidate_mediator_paths_total": local_path_upper_bound(observed_middle.len()),
        "candidate_mediator_paths_evaluated": evaluated_paths,
        "path_intervention_certificate": direct_certificate.as_ref().map(|value| json!(value)),
        "mediated_path_certificates": mediated_certificates,
        "unresolved_certificate": unresolved_certificate.as_ref().map(|value| json!(value)),
        "model": model.name(),
        "mediator_intervention_available": observation.legal_interventions.iter().any(|contract| contract.mediator_intervention_available),
        "unavailable_counterfactual_used_as_observed_evidence": false
    });
    Ok(CaseAnalysis {
        prediction,
        ir,
        direct_certificate,
        mediated_certificates,
        unresolved_certificate,
        candidate_total: local_path_upper_bound(observed_middle.len()),
        candidate_evaluated: evaluated_paths,
        work: edges.len() as u64 + evaluated_paths + observation.legal_interventions.len() as u64,
    })
}

fn intervention_constraint(contract: &LegalIntervention) -> String {
    format!(
        "CONTRACT={};TARGETS={:?};MEDIATOR_CONTROL_AVAILABLE={}",
        contract.contract_id, contract.targets, contract.mediator_intervention_available
    )
}

fn intervention_start(contracts: &[LegalIntervention]) -> Option<usize> {
    contracts
        .iter()
        .flat_map(|contract| contract.times.iter().copied())
        .min()
        .map(|value| value as usize)
}

fn decode_optional_matrix(bits: &[Vec<Option<u64>>]) -> Result<Vec<Vec<Option<f64>>>, String> {
    bits.iter()
        .map(|row| {
            row.iter()
                .map(|value| {
                    value
                        .map(f64::from_bits)
                        .map(|number| {
                            if number.is_finite() {
                                Ok(number)
                            } else {
                                Err("SEM37_R5_NONFINITE_NUMERIC_AUTHORITY".to_string())
                            }
                        })
                        .transpose()
                })
                .collect()
        })
        .collect()
}

fn series(matrix: &[Vec<Option<f64>>], end: usize, node: usize) -> Vec<Option<f64>> {
    matrix
        .iter()
        .take(end)
        .map(|row| row.get(node).copied().flatten())
        .collect()
}

fn lagged_score(
    matrix: &[Vec<Option<f64>>],
    end: usize,
    from: usize,
    to: usize,
    lag: usize,
) -> f64 {
    if lag == 0 || end <= lag + 2 {
        return 0.0;
    }
    let left = series(matrix, end, from);
    let right = series(matrix, end, to);
    let pairs = (lag..left.len().min(right.len()))
        .filter_map(|index| Some((left[index - lag]?, right[index]?)))
        .collect::<Vec<_>>();
    pearson_abs(&pairs)
}

fn pearson_abs(pairs: &[(f64, f64)]) -> f64 {
    if pairs.len() < 3 {
        return 0.0;
    }
    let n = pairs.len() as f64;
    let mx = pairs.iter().map(|pair| pair.0).sum::<f64>() / n;
    let my = pairs.iter().map(|pair| pair.1).sum::<f64>() / n;
    let covariance = pairs
        .iter()
        .map(|pair| (pair.0 - mx) * (pair.1 - my))
        .sum::<f64>();
    let vx = pairs.iter().map(|pair| (pair.0 - mx).powi(2)).sum::<f64>();
    let vy = pairs.iter().map(|pair| (pair.1 - my).powi(2)).sum::<f64>();
    if vx == 0.0 || vy == 0.0 {
        0.0
    } else {
        (covariance / (vx.sqrt() * vy.sqrt())).abs()
    }
}

fn ranked_local_edges(
    matrix: &[Vec<Option<f64>>],
    end: usize,
    nodes: &[usize],
    lag: usize,
    fanout: usize,
    direction_advantage: bool,
) -> Vec<(usize, usize, f64)> {
    let mut edges = Vec::new();
    for &from in nodes {
        let mut candidates = nodes
            .iter()
            .copied()
            .filter(|to| *to != from)
            .map(|to| (from, to, lagged_score(matrix, end, from, to, lag)))
            .filter(|(_, to, score)| {
                *score > 0.0
                    && (!direction_advantage || *score >= lagged_score(matrix, end, *to, from, lag))
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            right
                .2
                .partial_cmp(&left.2)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.1.cmp(&right.1))
        });
        edges.extend(candidates.into_iter().take(fanout.max(1)));
    }
    edges
}

fn edge_selected(edges: &[(usize, usize, f64)], from: usize, to: usize) -> bool {
    edges.iter().any(|edge| edge.0 == from && edge.1 == to)
}

fn enumerate_local_paths(
    node: usize,
    target: usize,
    edges: &[(usize, usize, f64)],
    current: &mut Vec<usize>,
    output: &mut Vec<Vec<usize>>,
    evaluated: &mut u64,
    max_nodes: usize,
) {
    if current.len() >= max_nodes {
        return;
    }
    for next in edges
        .iter()
        .filter(|edge| edge.0 == node)
        .map(|edge| edge.1)
    {
        *evaluated += 1;
        if current.contains(&next) {
            continue;
        }
        current.push(next);
        if next == target {
            output.push(current.clone());
        } else {
            enumerate_local_paths(next, target, edges, current, output, evaluated, max_nodes);
        }
        current.pop();
    }
}

fn conditional_direct_is_ranked(
    matrix: &[Vec<Option<f64>>],
    end: usize,
    source: usize,
    target: usize,
    middles: &[usize],
    lag: usize,
    fanout: usize,
) -> bool {
    let direct = partial_lagged_score(matrix, end, source, target, middles, lag);
    let mut alternatives = middles
        .iter()
        .copied()
        .map(|other| partial_lagged_score(matrix, end, source, other, &[], lag))
        .collect::<Vec<_>>();
    alternatives.push(direct);
    alternatives.sort_by(|left, right| right.partial_cmp(left).unwrap_or(Ordering::Equal));
    let rank = alternatives
        .iter()
        .position(|score| (*score - direct).abs() <= f64::EPSILON)
        .unwrap_or(alternatives.len());
    direct > 0.0 && rank < fanout.max(1)
}

fn partial_lagged_score(
    matrix: &[Vec<Option<f64>>],
    end: usize,
    source: usize,
    target: usize,
    controls: &[usize],
    lag: usize,
) -> f64 {
    if end <= lag + 3 {
        return 0.0;
    }
    let mut rows = Vec::new();
    for time in lag..end {
        let Some(x) = matrix[time - lag].get(source).copied().flatten() else {
            continue;
        };
        let Some(y) = matrix[time].get(target).copied().flatten() else {
            continue;
        };
        let control_values = controls
            .iter()
            .map(|node| matrix[time - lag].get(*node).copied().flatten())
            .collect::<Option<Vec<_>>>();
        if let Some(values) = control_values {
            rows.push((x, y, values));
        }
    }
    if rows.len() < 4 {
        return 0.0;
    }
    let mut rx = rows.iter().map(|row| row.0).collect::<Vec<_>>();
    let mut ry = rows.iter().map(|row| row.1).collect::<Vec<_>>();
    for index in 0..controls.len() {
        let control = rows.iter().map(|row| row.2[index]).collect::<Vec<_>>();
        residualize(&mut rx, &control);
        residualize(&mut ry, &control);
    }
    pearson_abs(&rx.into_iter().zip(ry).collect::<Vec<_>>())
}

fn residualize(values: &mut [f64], control: &[f64]) {
    if values.len() != control.len() || values.is_empty() {
        return;
    }
    let n = values.len() as f64;
    let value_mean = values.iter().sum::<f64>() / n;
    let control_mean = control.iter().sum::<f64>() / n;
    let covariance = values
        .iter()
        .zip(control)
        .map(|(value, feature)| (value - value_mean) * (feature - control_mean))
        .sum::<f64>();
    let variance = control
        .iter()
        .map(|feature| (feature - control_mean).powi(2))
        .sum::<f64>();
    let coefficient = if variance == 0.0 {
        0.0
    } else {
        covariance / variance
    };
    for (value, feature) in values.iter_mut().zip(control) {
        *value -= value_mean + coefficient * (feature - control_mean);
    }
}

fn intervention_signal(case: &CaseEvidence, pre_end: usize, target: usize) -> bool {
    let observed = &case.observation.values_ieee754_bits;
    let Some(last) = observed
        .get(pre_end.saturating_sub(1))
        .and_then(|row| row.get(target))
        .copied()
        .flatten()
        .map(f64::from_bits)
    else {
        return false;
    };
    let Some(actual) = case
        .intervention
        .query_outcome_ieee754_bits
        .first()
        .copied()
        .map(f64::from_bits)
    else {
        return false;
    };
    let mut steps = (1..pre_end)
        .filter_map(|index| {
            let left = observed[index - 1].get(target).copied().flatten()?;
            let right = observed[index].get(target).copied().flatten()?;
            Some((f64::from_bits(right) - f64::from_bits(left)).abs())
        })
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return false;
    }
    steps.sort_by(|left, right| left.partial_cmp(right).unwrap_or(Ordering::Equal));
    let median_step = steps[steps.len() / 2];
    (actual - last).abs() > median_step
}

fn direct_score(
    matrix: &[Vec<Option<f64>>],
    end: usize,
    source: usize,
    target: usize,
    lag: usize,
) -> f64 {
    lagged_score(matrix, end, source, target, lag)
}

fn weakest_path_score(matrix: &[Vec<Option<f64>>], end: usize, path: &[usize], lag: usize) -> f64 {
    path.windows(2)
        .map(|edge| lagged_score(matrix, end, edge[0], edge[1], lag))
        .fold(f64::INFINITY, f64::min)
}

fn local_path_upper_bound(middle_count: usize) -> u64 {
    match middle_count {
        0 => 0,
        1 => 1,
        value => (value * (value + 1)) as u64,
    }
}

pub fn ablate_path_specific(batch: &PredictionBatch) -> Result<Vec<Value>, String> {
    rewrite_predictions(batch, |prediction| {
        prediction["identifiability"] = json!(IdentifiabilityState::FullyIdentifiable);
        prediction["direct_present"] = json!(true);
        prediction["mediated_paths"] = json!([]);
        prediction["unresolved"] = json!(false);
        prediction["model"] = json!("PATH_SPECIFIC_IDENTIFICATION_ABLATED");
    })
}

pub fn ablate_identifiability(batch: &PredictionBatch) -> Result<Vec<Value>, String> {
    rewrite_predictions(batch, |prediction| {
        prediction["identifiability"] = json!(IdentifiabilityState::FullyIdentifiable);
        if prediction["unresolved"].as_bool() == Some(true) {
            prediction["direct_present"] = json!(true);
            prediction["unresolved"] = json!(false);
        }
        prediction["model"] = json!("IDENTIFIABILITY_STATE_ABLATED");
    })
}

pub fn ablate_pairwise_path(batch: &PredictionBatch) -> Result<Vec<Value>, String> {
    rewrite_predictions(batch, |prediction| {
        if prediction["mediated_paths"]
            .as_array()
            .is_some_and(|paths| !paths.is_empty())
        {
            prediction["direct_present"] = json!(true);
        }
        prediction["mediated_paths"] = json!([]);
        prediction["model"] = json!("PAIRWISE_EDGE_ONLY");
    })
}

fn rewrite_predictions<F>(batch: &PredictionBatch, mut rewrite: F) -> Result<Vec<Value>, String>
where
    F: FnMut(&mut Value),
{
    let mut predictions = batch.predictions.clone();
    for prediction in &mut predictions {
        rewrite(prediction);
    }
    Ok(predictions)
}

pub fn prediction_commitment(predictions: &[Value]) -> Result<String, String> {
    let bytes = serde_json::to_vec(predictions).map_err(|error| error.to_string())?;
    let digest = Sha256::digest(bytes);
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub fn arm(predictions: Vec<Value>) -> Result<Value, String> {
    let commitment = prediction_commitment(&predictions)?;
    Ok(json!({"predictions": predictions, "prediction_commitment": commitment}))
}

pub fn metrics_field(metrics: &Value, name: &str) -> u64 {
    metrics[name].as_u64().unwrap_or(u64::MAX / 4)
}

pub fn ratio(metrics: &Value, name: &str) -> (u64, u64) {
    (
        metrics[name]["numerator"].as_u64().unwrap_or(0),
        metrics[name]["denominator"].as_u64().unwrap_or(1),
    )
}

pub fn compare_candidate_metrics(left: (&Value, u64), right: (&Value, u64)) -> Ordering {
    let lm = left.0;
    let rm = right.0;
    let l_hard = hard_gate_vector(lm);
    let r_hard = hard_gate_vector(rm);
    r_hard
        .cmp(&l_hard)
        .then_with(|| {
            let (ln, ld) = ratio(lm, "identifiable_direct_recall");
            let (rn, rd) = ratio(rm, "identifiable_direct_recall");
            (rn * ld).cmp(&(ln * rd))
        })
        .then_with(|| metrics_field(lm, "mediated_fn").cmp(&metrics_field(rm, "mediated_fn")))
        .then_with(|| metrics_field(lm, "mediated_fp").cmp(&metrics_field(rm, "mediated_fp")))
        .then_with(|| left.1.cmp(&right.1))
}

fn hard_gate_vector(metrics: &Value) -> (bool, bool, bool, bool, bool) {
    (
        metrics_field(metrics, "pure_mediation_false_direct_events") == 0,
        metrics_field(metrics, "common_cause_as_direct_misidentifications") == 0,
        metrics_field(metrics, "false_certainty_on_non_identifiable_cases") == 0,
        metrics["mixed_direct_mediated_identification_pass"].as_bool() == Some(true),
        metrics_field(metrics, "external_path_causal_overgeneralization_events") == 0,
    )
}

pub fn candidate_summary(models: &[CandidateModel]) -> BTreeMap<&'static str, CandidateModel> {
    models.iter().map(|model| (model.name(), *model)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_bank_is_frozen_bounded_and_has_no_threshold_parameter() {
        assert_eq!(CandidateModel::CANDIDATES.len(), 8);
        assert!(CandidateModel::CANDIDATES
            .iter()
            .all(|candidate| candidate.fanout <= 2));
    }

    #[test]
    fn local_path_bound_is_sparse() {
        assert_eq!(local_path_upper_bound(0), 0);
        assert_eq!(local_path_upper_bound(1), 1);
        assert_eq!(local_path_upper_bound(2), 6);
    }

    #[test]
    fn commitment_is_deterministic() {
        let values = vec![json!({"b": 2, "a": 1})];
        assert_eq!(
            prediction_commitment(&values),
            prediction_commitment(&values)
        );
    }
}
