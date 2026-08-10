use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::sem37_r1::{
    adapter::{R1CaseDescriptor, R1ExternalLane, R1ExternalObservation},
    engine::{
        predict_batch as r1_predict_batch, prediction_commitment, PredictionBatch, TransferMethod,
        TransferResearchMode,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CausalPrecisionMethod {
    R1DenseCandidate,
    PairwiseMdl,
    ConditionalLinearMdl,
    ConditionalNonlinearMdl,
    StableConditionalMdl,
    TransitiveReducedNonlinearMdl,
    TransitiveReducedStableMdl,
    GlobalSubsetMdl,
    GlobalSubsetTransitiveMdl,
    PairwiseForkAblationMdl,
    PairwiseChainAblationMdl,
    PairwiseTriadAblationMdl,
    PairwiseTriadStableAblationMdl,
}

impl CausalPrecisionMethod {
    pub const CANDIDATES: [Self; 13] = [
        Self::R1DenseCandidate,
        Self::PairwiseMdl,
        Self::ConditionalLinearMdl,
        Self::ConditionalNonlinearMdl,
        Self::StableConditionalMdl,
        Self::TransitiveReducedNonlinearMdl,
        Self::TransitiveReducedStableMdl,
        Self::GlobalSubsetMdl,
        Self::GlobalSubsetTransitiveMdl,
        Self::PairwiseForkAblationMdl,
        Self::PairwiseChainAblationMdl,
        Self::PairwiseTriadAblationMdl,
        Self::PairwiseTriadStableAblationMdl,
    ];
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalPrecisionBatch {
    pub method: CausalPrecisionMethod,
    pub lane: R1ExternalLane,
    pub predictions: Vec<Value>,
    pub prediction_commitment: String,
    pub causal_hypotheses_considered: u64,
    pub causal_hypotheses_retained: u64,
    pub manual_precision_threshold_repair_events: u64,
    pub recovered_shift_aware_component_used: bool,
}

pub fn predict_lane_a(
    cases: &[(R1CaseDescriptor, R1ExternalObservation)],
    method: CausalPrecisionMethod,
) -> Result<CausalPrecisionBatch, String> {
    let baseline = r1_predict_batch(
        cases,
        R1ExternalLane::A,
        TransferMethod::ShiftAwareTransfer,
        TransferResearchMode::Full,
    )?;
    let mut predictions = baseline.predictions;
    let mut considered = 0_u64;
    let mut retained = 0_u64;
    for ((descriptor, observation), prediction) in cases
        .iter()
        .filter(|(descriptor, _)| descriptor.lane == R1ExternalLane::A)
        .zip(&mut predictions)
    {
        let values = entity_series(observation, descriptor.entity_count as usize)?;
        let edges = discover_direct_edges(&values, method);
        considered += (descriptor.entity_count * descriptor.entity_count.saturating_sub(1)) as u64;
        retained += edges.len() as u64;
        prediction["edges"] = json!(edges
            .into_iter()
            .map(|(source, target)| vec![source as u64, target as u64, 0_u64])
            .collect::<Vec<_>>());
    }
    let commitment = prediction_commitment(&predictions)?;
    Ok(CausalPrecisionBatch {
        method,
        lane: R1ExternalLane::A,
        predictions,
        prediction_commitment: commitment,
        causal_hypotheses_considered: considered,
        causal_hypotheses_retained: retained,
        manual_precision_threshold_repair_events: 0,
        recovered_shift_aware_component_used: false,
    })
}

pub fn predict_lane_b_recovered(
    cases: &[(R1CaseDescriptor, R1ExternalObservation)],
    method: CausalPrecisionMethod,
) -> Result<CausalPrecisionBatch, String> {
    let batch: PredictionBatch = r1_predict_batch(
        cases,
        R1ExternalLane::B,
        TransferMethod::ShiftAwareTransfer,
        TransferResearchMode::Full,
    )?;
    Ok(CausalPrecisionBatch {
        method,
        lane: R1ExternalLane::B,
        predictions: batch.predictions,
        prediction_commitment: batch.prediction_commitment,
        causal_hypotheses_considered: batch
            .case_receipts
            .iter()
            .map(|receipt| receipt.hypotheses_generated)
            .sum(),
        causal_hypotheses_retained: batch
            .case_receipts
            .iter()
            .map(|receipt| receipt.hypotheses_retained)
            .sum(),
        manual_precision_threshold_repair_events: 0,
        recovered_shift_aware_component_used: true,
    })
}

fn entity_series(
    observation: &R1ExternalObservation,
    entity_count: usize,
) -> Result<Vec<Vec<f64>>, String> {
    if observation.nonfinite_cells_have_numeric_authority
        || observation.missingness_transport != "EXPLICIT_NULL_NO_SENTINEL"
    {
        return Err("SEM37_R2_INVALID_NUMERIC_TRANSPORT".to_string());
    }
    let columns = observation.bindings.len();
    if columns == 0 || observation.values_ieee754_bits.len() < 12 {
        return Err("SEM37_R2_INSUFFICIENT_OBSERVATION".to_string());
    }
    let mut raw = vec![vec![0.0; columns]; observation.values_ieee754_bits.len()];
    let mut sums = vec![0.0; columns];
    let mut counts = vec![0_u64; columns];
    for (time, row) in observation.values_ieee754_bits.iter().enumerate() {
        if row.len() != columns {
            return Err("SEM37_R2_OBSERVATION_ARITY_DRIFT".to_string());
        }
        for (column, cell) in row.iter().enumerate() {
            if let Some(bits) = cell {
                let value = f64::from_bits(*bits);
                if !value.is_finite() {
                    return Err("SEM37_R2_NONFINITE_NUMERIC_AUTHORITY".to_string());
                }
                raw[time][column] = value;
                sums[column] += value;
                counts[column] += 1;
            }
        }
    }
    for column in 0..columns {
        let mean = if counts[column] == 0 {
            0.0
        } else {
            sums[column] / counts[column] as f64
        };
        for (time, row) in observation.values_ieee754_bits.iter().enumerate() {
            if row[column].is_none() {
                raw[time][column] = if time > 0 {
                    raw[time - 1][column]
                } else {
                    mean
                };
            }
        }
    }
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

fn discover_direct_edges(
    values: &[Vec<f64>],
    method: CausalPrecisionMethod,
) -> Vec<(usize, usize)> {
    if values.len() < 12 || values[0].len() < 2 {
        return Vec::new();
    }
    let variables = values[0].len();
    if method == CausalPrecisionMethod::R1DenseCandidate {
        return (0..variables)
            .flat_map(|source| {
                (0..variables)
                    .filter(move |target| *target != source)
                    .map(move |target| (source, target))
            })
            .collect();
    }
    if matches!(
        method,
        CausalPrecisionMethod::GlobalSubsetMdl | CausalPrecisionMethod::GlobalSubsetTransitiveMdl
    ) {
        let mut edges = global_subset_edges(values);
        if method == CausalPrecisionMethod::GlobalSubsetTransitiveMdl {
            edges = remove_transitive_adjacencies(&edges, variables);
        }
        return edges;
    }
    if matches!(
        method,
        CausalPrecisionMethod::PairwiseForkAblationMdl
            | CausalPrecisionMethod::PairwiseChainAblationMdl
            | CausalPrecisionMethod::PairwiseTriadAblationMdl
            | CausalPrecisionMethod::PairwiseTriadStableAblationMdl
    ) {
        return pairwise_triad_ablation_edges(values, method);
    }
    let nonlinear = matches!(
        method,
        CausalPrecisionMethod::ConditionalNonlinearMdl
            | CausalPrecisionMethod::StableConditionalMdl
            | CausalPrecisionMethod::TransitiveReducedNonlinearMdl
            | CausalPrecisionMethod::TransitiveReducedStableMdl
    );
    let conditional = !matches!(method, CausalPrecisionMethod::PairwiseMdl);
    let stable = matches!(
        method,
        CausalPrecisionMethod::StableConditionalMdl
            | CausalPrecisionMethod::TransitiveReducedStableMdl
    );
    let transitive_reduction = matches!(
        method,
        CausalPrecisionMethod::TransitiveReducedNonlinearMdl
            | CausalPrecisionMethod::TransitiveReducedStableMdl
    );
    let mut edges = Vec::new();
    for target in 0..variables {
        for source in 0..variables {
            if source == target {
                continue;
            }
            let supported = if stable {
                let evidence: Vec<f64> = (0..3)
                    .map(|fold| {
                        mdl_evidence(values, source, target, conditional, nonlinear, Some(fold))
                    })
                    .collect();
                evidence.iter().filter(|score| **score > 0.0).count() >= 2
                    && median(&evidence) > 0.0
            } else {
                mdl_evidence(values, source, target, conditional, nonlinear, None) > 0.0
            };
            if supported {
                edges.push((source, target));
            }
        }
    }
    if transitive_reduction {
        remove_transitive_adjacencies(&edges, variables)
    } else {
        edges
    }
}

fn pairwise_triad_ablation_edges(
    values: &[Vec<f64>],
    method: CausalPrecisionMethod,
) -> Vec<(usize, usize)> {
    let variables = values[0].len();
    let pairwise: Vec<(usize, usize)> = (0..variables)
        .flat_map(|source| {
            (0..variables)
                .filter(move |target| *target != source)
                .filter(move |target| {
                    mdl_evidence(values, source, *target, false, false, None) > 0.0
                })
                .map(move |target| (source, target))
        })
        .collect();
    pairwise
        .iter()
        .copied()
        .filter(|(source, target)| {
            let fork = (0..variables).any(|common| {
                common != *source
                    && common != *target
                    && pairwise.contains(&(common, *source))
                    && pairwise.contains(&(common, *target))
            });
            let chain = (0..variables).any(|mediator| {
                mediator != *source
                    && mediator != *target
                    && pairwise.contains(&(*source, mediator))
                    && pairwise.contains(&(mediator, *target))
            });
            let requires_ablation = match method {
                CausalPrecisionMethod::PairwiseForkAblationMdl => fork,
                CausalPrecisionMethod::PairwiseChainAblationMdl => chain,
                CausalPrecisionMethod::PairwiseTriadAblationMdl => fork || chain,
                CausalPrecisionMethod::PairwiseTriadStableAblationMdl => fork || chain,
                _ => false,
            };
            if !requires_ablation {
                return true;
            }
            if method == CausalPrecisionMethod::PairwiseTriadStableAblationMdl {
                let evidence: Vec<f64> = (0..3)
                    .map(|fold| mdl_evidence(values, *source, *target, true, true, Some(fold)))
                    .collect();
                evidence.iter().filter(|score| **score > 0.0).count() >= 2
                    && median(&evidence) > 0.0
            } else {
                mdl_evidence(values, *source, *target, true, true, None) > 0.0
            }
        })
        .collect()
}

fn global_subset_edges(values: &[Vec<f64>]) -> Vec<(usize, usize)> {
    let variables = values[0].len();
    let mut edges = Vec::new();
    for target in 0..variables {
        let candidates: Vec<usize> = (0..variables).filter(|source| *source != target).collect();
        if candidates.len() >= usize::BITS as usize {
            continue;
        }
        let mut best_score = f64::INFINITY;
        let mut best_sources = Vec::new();
        for mask in 0..(1_usize << candidates.len()) {
            let sources: Vec<usize> = candidates
                .iter()
                .enumerate()
                .filter(|(bit, _)| mask & (1 << bit) != 0)
                .map(|(_, source)| *source)
                .collect();
            let score = subset_description_length(values, target, &sources);
            if score < best_score
                || (score.total_cmp(&best_score).is_eq() && sources.len() < best_sources.len())
            {
                best_score = score;
                best_sources = sources;
            }
        }
        edges.extend(best_sources.into_iter().map(|source| (source, target)));
    }
    edges
}

fn subset_description_length(values: &[Vec<f64>], target: usize, sources: &[usize]) -> f64 {
    let mut features = Vec::new();
    let mut outcomes = Vec::new();
    for time in 1..values.len() {
        features.push(subset_feature_row(&values[time - 1], target, sources));
        outcomes.push(values[time][target] - values[time - 1][target]);
    }
    let rows = features.len();
    let mut sse = 0.0;
    for fold in 0..3 {
        let start = rows * fold / 3;
        let end = rows * (fold + 1) / 3;
        let validation: Vec<bool> = (0..rows).map(|row| row >= start && row < end).collect();
        sse += held_out_sse(&features, &outcomes, &validation);
    }
    let observations = rows.max(2) as f64;
    let parameters = features.first().map_or(0, Vec::len).max(1) as f64;
    observations * ((sse + 1.0e-18) / observations).ln() + parameters * observations.ln()
}

fn subset_feature_row(state: &[f64], target: usize, sources: &[usize]) -> Vec<f64> {
    let mut variables = vec![target];
    variables.extend_from_slice(sources);
    variables.sort_unstable();
    variables.dedup();
    let mut row = vec![1.0];
    for variable in &variables {
        let value = state[*variable];
        row.push(value);
        row.push(value * value);
        row.push(value * value * value);
    }
    for left in 0..variables.len() {
        for right in (left + 1)..variables.len() {
            row.push(state[variables[left]] * state[variables[right]]);
        }
    }
    row
}

fn remove_transitive_adjacencies(
    edges: &[(usize, usize)],
    variables: usize,
) -> Vec<(usize, usize)> {
    edges
        .iter()
        .copied()
        .filter(|edge| !has_alternate_directed_path(edges, *edge, variables))
        .collect()
}

fn has_alternate_directed_path(
    edges: &[(usize, usize)],
    excluded: (usize, usize),
    variables: usize,
) -> bool {
    let mut frontier = vec![excluded.0];
    let mut visited = vec![false; variables];
    visited[excluded.0] = true;
    while let Some(node) = frontier.pop() {
        for (source, target) in edges {
            if (*source, *target) == excluded || *source != node || visited[*target] {
                continue;
            }
            if *target == excluded.1 {
                return true;
            }
            visited[*target] = true;
            frontier.push(*target);
        }
    }
    false
}

fn mdl_evidence(
    values: &[Vec<f64>],
    source: usize,
    target: usize,
    conditional: bool,
    nonlinear: bool,
    held_out_fold: Option<usize>,
) -> f64 {
    let rows = values.len() - 1;
    let fold = held_out_fold.unwrap_or(2).min(2);
    let validation_start = rows * fold / 3;
    let validation_end = rows * (fold + 1) / 3;
    let mut full_features = Vec::new();
    let mut reduced_features = Vec::new();
    let mut outcomes = Vec::new();
    let mut validation = Vec::new();
    for time in 1..values.len() {
        let state = &values[time - 1];
        let mut reduced_sources: Vec<usize> = if conditional {
            (0..state.len())
                .filter(|candidate| *candidate != target && *candidate != source)
                .collect()
        } else {
            Vec::new()
        };
        reduced_sources.sort_unstable();
        let mut full_sources = reduced_sources.clone();
        full_sources.push(source);
        full_sources.sort_unstable();
        reduced_features.push(feature_row(state, target, &reduced_sources, nonlinear));
        full_features.push(feature_row(state, target, &full_sources, nonlinear));
        outcomes.push(values[time][target] - state[target]);
        let row_index = time - 1;
        validation.push(row_index >= validation_start && row_index < validation_end);
    }
    let full_sse = held_out_sse(&full_features, &outcomes, &validation);
    let reduced_sse = held_out_sse(&reduced_features, &outcomes, &validation);
    let validation_rows = validation.iter().filter(|value| **value).count().max(2) as f64;
    let added_parameters = if nonlinear { 4.0 } else { 1.0 };
    let epsilon = 1.0e-18;
    validation_rows * ((reduced_sse + epsilon) / (full_sse + epsilon)).ln()
        - added_parameters * validation_rows.ln()
}

fn feature_row(state: &[f64], target: usize, sources: &[usize], nonlinear: bool) -> Vec<f64> {
    let target_value = state[target];
    let mut row = vec![1.0, target_value];
    if nonlinear {
        row.push(target_value * target_value);
        row.push(target_value * target_value * target_value);
    }
    for source in sources {
        let value = state[*source];
        row.push(value);
        if nonlinear {
            row.push(value * value);
            row.push(value * value * value);
            row.push(value * target_value);
        }
    }
    row
}

fn held_out_sse(features: &[Vec<f64>], outcomes: &[f64], validation: &[bool]) -> f64 {
    let training_features: Vec<Vec<f64>> = features
        .iter()
        .zip(validation)
        .filter(|(_, is_validation)| !**is_validation)
        .map(|(row, _)| row.clone())
        .collect();
    let training_outcomes: Vec<f64> = outcomes
        .iter()
        .zip(validation)
        .filter(|(_, is_validation)| !**is_validation)
        .map(|(value, _)| *value)
        .collect();
    let normalization = Normalization::fit(&training_features);
    let normalized_training: Vec<Vec<f64>> = training_features
        .iter()
        .map(|row| normalization.apply(row))
        .collect();
    let coefficients = fit_linear(&normalized_training, &training_outcomes, 1.0e-6);
    features
        .iter()
        .zip(outcomes)
        .zip(validation)
        .filter(|(_, is_validation)| **is_validation)
        .map(|((row, outcome), _)| {
            let predicted = dot(&normalization.apply(row), &coefficients);
            let residual = outcome - predicted;
            residual * residual
        })
        .filter(|value| value.is_finite())
        .sum::<f64>()
        .max(0.0)
}

struct Normalization {
    means: Vec<f64>,
    scales: Vec<f64>,
}

impl Normalization {
    fn fit(rows: &[Vec<f64>]) -> Self {
        let dimensions = rows.first().map_or(0, Vec::len);
        let mut means = vec![0.0; dimensions];
        let mut scales = vec![1.0; dimensions];
        for column in 1..dimensions {
            means[column] =
                rows.iter().map(|row| row[column]).sum::<f64>() / rows.len().max(1) as f64;
            let variance = rows
                .iter()
                .map(|row| (row[column] - means[column]).powi(2))
                .sum::<f64>()
                / rows.len().max(1) as f64;
            scales[column] = variance.sqrt().max(1.0e-12);
        }
        Self { means, scales }
    }

    fn apply(&self, row: &[f64]) -> Vec<f64> {
        row.iter()
            .enumerate()
            .map(|(column, value)| {
                if column == 0 {
                    *value
                } else {
                    (value - self.means[column]) / self.scales[column]
                }
            })
            .collect()
    }
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
    for index in 1..dimensions {
        normal[index][index] += ridge;
    }
    solve_linear(normal, rhs).unwrap_or_else(|| vec![0.0; dimensions])
}

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

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted
        .get(sorted.len() / 2)
        .copied()
        .unwrap_or(f64::NEG_INFINITY)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conditional_mdl_does_not_promote_transitive_chain_edge() {
        let mut values = vec![vec![0.2, 0.0, 0.0]];
        for time in 1..180 {
            let prior = values[time - 1].clone();
            values.push(vec![
                0.83 * prior[0] + (time as f64 * 0.17).sin() * 0.02,
                0.61 * prior[1] + 0.54 * prior[0],
                0.57 * prior[2] + 0.63 * prior[1],
            ]);
        }
        let edges =
            discover_direct_edges(&values, CausalPrecisionMethod::GlobalSubsetTransitiveMdl);
        assert!(
            !edges.contains(&(0, 2)),
            "transitive influence became direct adjacency"
        );
    }

    #[test]
    fn conditional_mdl_does_not_promote_observed_common_cause_edge() {
        let mut values = vec![vec![0.1, 0.0, 0.0]];
        for time in 1..180 {
            let prior = values[time - 1].clone();
            values.push(vec![
                0.79 * prior[0] + (time as f64 * 0.11).cos() * 0.03,
                0.48 * prior[1] + 0.72 * prior[0],
                0.44 * prior[2] - 0.67 * prior[0],
            ]);
        }
        let edges = discover_direct_edges(&values, CausalPrecisionMethod::GlobalSubsetMdl);
        assert!(!edges.contains(&(1, 2)) && !edges.contains(&(2, 1)));
    }
}
