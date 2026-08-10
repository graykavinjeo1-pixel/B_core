use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::adapter::{
    ExternalCaseDescriptor, ExternalInterventionContract, ExternalLane, ExternalObservation,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExternalMethod {
    Persistence,
    IndependentLinear,
    SparseCoupledLinear,
    DenseCoupledLinear,
    InterventionRegression,
    HybridMechanism,
}

impl ExternalMethod {
    pub const CANDIDATES: [Self; 6] = [
        Self::Persistence,
        Self::IndependentLinear,
        Self::SparseCoupledLinear,
        Self::DenseCoupledLinear,
        Self::InterventionRegression,
        Self::HybridMechanism,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExternalResearchMode {
    Full,
    FrontierSelectionOff,
    DiscoveredMemoryOff,
    InterventionOff,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NumericTransportReceipt {
    pub finite_cells: u64,
    pub explicit_missing_cells: u64,
    pub finite_ieee754_roundtrip_mismatches: u64,
    pub nonfinite_cells_with_numeric_authority: u64,
    pub numeric_value_as_new_primitive_events: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseResearchReceipt {
    pub case_id: String,
    pub lane: ExternalLane,
    pub method: ExternalMethod,
    pub mode: ExternalResearchMode,
    pub self_detected_frontier: bool,
    pub hypotheses_generated: u64,
    pub hypotheses_retained: u64,
    pub hypotheses_rejected: u64,
    pub proposed_edges: u64,
    pub future_predictions_frozen: u64,
    pub interventions_proposed: u64,
    pub predictions_frozen_before_intervention: bool,
    pub active_variables: u64,
    pub active_causal_mechanisms: u64,
    pub active_temporal_processes: u64,
    pub termination: String,
    pub numeric_transport: NumericTransportReceipt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PredictionBatch {
    pub lane: ExternalLane,
    pub method: ExternalMethod,
    pub mode: ExternalResearchMode,
    pub predictions: Vec<Value>,
    pub prediction_commitment: String,
    pub case_receipts: Vec<CaseResearchReceipt>,
}

#[derive(Debug, Clone)]
struct PreparedObservation {
    values: Vec<Vec<f64>>,
    present: Vec<Vec<bool>>,
    receipt: NumericTransportReceipt,
}

pub fn predict_batch(
    cases: &[(ExternalCaseDescriptor, ExternalObservation)],
    lane: ExternalLane,
    method: ExternalMethod,
    mode: ExternalResearchMode,
) -> Result<PredictionBatch, String> {
    let mut predictions = Vec::new();
    let mut case_receipts = Vec::new();
    for (descriptor, observation) in cases
        .iter()
        .filter(|(descriptor, _)| descriptor.lane == lane)
    {
        let prepared = prepare_observation(observation)?;
        let (prediction, receipt) = match lane {
            ExternalLane::A => predict_passive(descriptor, observation, &prepared, method, mode)?,
            ExternalLane::B => {
                predict_interventional(descriptor, observation, &prepared, method, mode)?
            }
        };
        predictions.push(prediction);
        case_receipts.push(receipt);
    }
    let prediction_commitment = prediction_commitment(&predictions)?;
    Ok(PredictionBatch {
        lane,
        method,
        mode,
        predictions,
        prediction_commitment,
        case_receipts,
    })
}

fn prepare_observation(observation: &ExternalObservation) -> Result<PreparedObservation, String> {
    if observation.nonfinite_cells_have_numeric_authority
        || observation.missingness_transport != "EXPLICIT_NULL_NO_SENTINEL"
    {
        return Err("SEM37_INVALID_GENERIC_MISSINGNESS_CONTRACT".to_string());
    }
    let columns = observation.bindings.len();
    if columns == 0 || observation.values_ieee754_bits.len() < 3 {
        return Err("SEM37_INSUFFICIENT_EXTERNAL_OBSERVATION".to_string());
    }
    let mut values = vec![vec![0.0; columns]; observation.values_ieee754_bits.len()];
    let mut present = vec![vec![false; columns]; observation.values_ieee754_bits.len()];
    let mut sums = vec![0.0; columns];
    let mut counts = vec![0_u64; columns];
    let mut finite_cells = 0_u64;
    let mut missing_cells = 0_u64;
    let mut roundtrip_mismatches = 0_u64;
    for (time, row) in observation.values_ieee754_bits.iter().enumerate() {
        if row.len() != columns {
            return Err("SEM37_EXTERNAL_OBSERVATION_ARITY_DRIFT".to_string());
        }
        for (column, cell) in row.iter().enumerate() {
            if let Some(bits) = cell {
                let value = f64::from_bits(*bits);
                if !value.is_finite() {
                    return Err("SEM37_NONFINITE_VALUE_ACQUIRED_NUMERIC_AUTHORITY".to_string());
                }
                if value.to_bits() != *bits {
                    roundtrip_mismatches += 1;
                }
                values[time][column] = value;
                present[time][column] = true;
                sums[column] += value;
                counts[column] += 1;
                finite_cells += 1;
            } else {
                missing_cells += 1;
            }
        }
    }
    for column in 0..columns {
        let mean = if counts[column] == 0 {
            0.0
        } else {
            sums[column] / counts[column] as f64
        };
        let mut last = None;
        for time in 0..values.len() {
            if present[time][column] {
                last = Some(values[time][column]);
            } else {
                values[time][column] = last.unwrap_or(mean);
            }
        }
        let mut next = None;
        for time in (0..values.len()).rev() {
            if present[time][column] {
                next = Some(values[time][column]);
            } else if next.is_some() && values[time][column] == mean {
                values[time][column] = next.unwrap_or(mean);
            }
        }
    }
    Ok(PreparedObservation {
        values,
        present,
        receipt: NumericTransportReceipt {
            finite_cells,
            explicit_missing_cells: missing_cells,
            finite_ieee754_roundtrip_mismatches: roundtrip_mismatches,
            nonfinite_cells_with_numeric_authority: 0,
            numeric_value_as_new_primitive_events: 0,
        },
    })
}

fn predict_passive(
    descriptor: &ExternalCaseDescriptor,
    observation: &ExternalObservation,
    prepared: &PreparedObservation,
    method: ExternalMethod,
    mode: ExternalResearchMode,
) -> Result<(Value, CaseResearchReceipt), String> {
    let entity_values = entity_series(observation, prepared, descriptor.entity_count as usize);
    let frontier = unexplained_dynamic_variance(&entity_values) > 1.0e-12;
    let allow_selection = mode != ExternalResearchMode::FrontierSelectionOff;
    let allow_memory = mode != ExternalResearchMode::DiscoveredMemoryOff;
    let effective_method = if !allow_selection || !allow_memory {
        ExternalMethod::Persistence
    } else {
        method
    };
    let edges = discover_edges(&entity_values, effective_method);
    let predicted = forecast_next(&prepared.values, effective_method);
    let last_time = prepared.values.len() - 1;
    let prediction_time = observation.time_end_exclusive;
    let mut future_predictions = Vec::new();
    if effective_method != ExternalMethod::Persistence || frontier {
        for (column, binding) in observation.bindings.iter().enumerate() {
            if prepared.present[last_time][column] {
                future_predictions.push(json!({
                    "time": prediction_time,
                    "entity": binding.entity,
                    "channel": binding.channel,
                    "value_ieee754_bits": finite_bits(predicted[column]),
                    "previous_value_ieee754_bits": finite_bits(prepared.values[last_time][column])
                }));
            }
        }
    }
    let hypothesis_count = if allow_selection && frontier {
        ExternalMethod::CANDIDATES.len() as u64
    } else {
        0
    };
    let termination = if !frontier {
        "INSUFFICIENT_EVIDENCE"
    } else if edges.is_empty() {
        "PARTIALLY_IDENTIFIED"
    } else {
        "DISCOVERED"
    };
    let prediction = json!({
        "case_id": observation.case_id,
        "edges": edges.iter().map(|(source, target)| vec![*source, *target, 0_u64]).collect::<Vec<_>>(),
        "future_predictions": future_predictions
    });
    let receipt = CaseResearchReceipt {
        case_id: observation.case_id.clone(),
        lane: ExternalLane::A,
        method: effective_method,
        mode,
        self_detected_frontier: frontier,
        hypotheses_generated: hypothesis_count,
        hypotheses_retained: u64::from(!edges.is_empty()),
        hypotheses_rejected: hypothesis_count.saturating_sub(u64::from(!edges.is_empty())),
        proposed_edges: edges.len() as u64,
        future_predictions_frozen: prediction["future_predictions"]
            .as_array()
            .map_or(0, |items| items.len() as u64),
        interventions_proposed: 0,
        predictions_frozen_before_intervention: true,
        active_variables: observation.bindings.len() as u64,
        active_causal_mechanisms: edges.len() as u64,
        active_temporal_processes: u64::from(frontier),
        termination: termination.to_string(),
        numeric_transport: prepared.receipt.clone(),
    };
    Ok((prediction, receipt))
}

fn predict_interventional(
    descriptor: &ExternalCaseDescriptor,
    observation: &ExternalObservation,
    prepared: &PreparedObservation,
    method: ExternalMethod,
    mode: ExternalResearchMode,
) -> Result<(Value, CaseResearchReceipt), String> {
    let contract = observation
        .legal_interventions
        .first()
        .ok_or("SEM37_LEGAL_INTERVENTION_CONTRACT_MISSING")?;
    let frontier = intervention_frontier(contract, observation.bindings.len());
    let allow_selection = mode != ExternalResearchMode::FrontierSelectionOff;
    let allow_memory = mode != ExternalResearchMode::DiscoveredMemoryOff;
    let allow_intervention = mode != ExternalResearchMode::InterventionOff;
    let effective_method = if !allow_selection || !allow_memory {
        ExternalMethod::Persistence
    } else {
        method
    };
    let predicted = predict_query_values(
        &prepared.values,
        contract,
        descriptor.time_steps as usize,
        effective_method,
        allow_intervention,
    );
    let prediction = json!({
        "case_id": observation.case_id,
        "predicted_y_ieee754_bits": predicted.into_iter().map(finite_bits).collect::<Vec<_>>()
    });
    let hypotheses = if allow_selection && frontier {
        ExternalMethod::CANDIDATES.len() as u64
    } else {
        0
    };
    let receipt = CaseResearchReceipt {
        case_id: observation.case_id.clone(),
        lane: ExternalLane::B,
        method: effective_method,
        mode,
        self_detected_frontier: frontier,
        hypotheses_generated: hypotheses,
        hypotheses_retained: u64::from(frontier && allow_selection),
        hypotheses_rejected: hypotheses.saturating_sub(u64::from(frontier && allow_selection)),
        proposed_edges: 0,
        future_predictions_frozen: contract.query_target.len() as u64,
        interventions_proposed: u64::from(allow_intervention && frontier),
        predictions_frozen_before_intervention: true,
        active_variables: observation.bindings.len() as u64,
        active_causal_mechanisms: if effective_method == ExternalMethod::Persistence {
            0
        } else {
            observation.bindings.len() as u64
        },
        active_temporal_processes: 1,
        termination: if frontier {
            "DISCOVERED"
        } else {
            "INSUFFICIENT_EVIDENCE"
        }
        .to_string(),
        numeric_transport: prepared.receipt.clone(),
    };
    Ok((prediction, receipt))
}

fn entity_series(
    observation: &ExternalObservation,
    prepared: &PreparedObservation,
    entity_count: usize,
) -> Vec<Vec<f64>> {
    let mut indices = vec![Vec::new(); entity_count];
    for (column, binding) in observation.bindings.iter().enumerate() {
        if let Some(entity) = indices.get_mut(binding.entity as usize) {
            entity.push(column);
        }
    }
    prepared
        .values
        .iter()
        .map(|row| {
            indices
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
        .collect()
}

#[allow(clippy::needless_range_loop)]
fn discover_edges(values: &[Vec<f64>], method: ExternalMethod) -> Vec<(u64, u64)> {
    if values.len() < 8 || values[0].len() < 2 || method == ExternalMethod::Persistence {
        return Vec::new();
    }
    let variables = values[0].len();
    if method == ExternalMethod::DenseCoupledLinear {
        return (0..variables)
            .flat_map(|source| {
                (0..variables)
                    .filter(move |target| *target != source)
                    .map(move |target| (source as u64, target as u64))
            })
            .collect();
    }
    let threshold = match method {
        ExternalMethod::IndependentLinear => 0.08,
        ExternalMethod::SparseCoupledLinear => 0.025,
        ExternalMethod::InterventionRegression => 0.015,
        ExternalMethod::HybridMechanism => 0.02,
        _ => 1.0,
    };
    let mut scores = vec![vec![0.0; variables]; variables];
    for target in 0..variables {
        let baseline = regression_sse(values, target, &[], false);
        for source in 0..variables {
            if source == target {
                continue;
            }
            let lagged = regression_sse(values, target, &[source], false);
            let same_step = regression_sse(values, target, &[source], true);
            let best = lagged.min(same_step);
            scores[source][target] = if baseline <= 1.0e-18 {
                0.0
            } else {
                ((baseline - best) / baseline).max(0.0)
            };
        }
    }
    let mut edges = Vec::new();
    for source in 0..variables {
        for target in 0..variables {
            if source == target {
                continue;
            }
            let score = scores[source][target];
            let reverse = scores[target][source];
            if score >= threshold && (score >= reverse * 0.85 || score >= threshold * 3.0) {
                edges.push((source as u64, target as u64));
            }
        }
    }
    edges
}

fn regression_sse(values: &[Vec<f64>], target: usize, sources: &[usize], same_step: bool) -> f64 {
    let mut features = Vec::new();
    let mut outcomes = Vec::new();
    for time in 1..values.len() {
        let mut row = vec![1.0, values[time - 1][target]];
        for source in sources {
            row.push(values[if same_step { time } else { time - 1 }][*source]);
        }
        features.push(row);
        outcomes.push(values[time][target]);
    }
    let coefficients = fit_linear(&features, &outcomes, 1.0e-6);
    features
        .iter()
        .zip(outcomes)
        .map(|(row, outcome)| {
            let residual = outcome - dot(row, &coefficients);
            residual * residual
        })
        .sum()
}

fn forecast_next(values: &[Vec<f64>], method: ExternalMethod) -> Vec<f64> {
    let last = values.last().cloned().unwrap_or_default();
    match method {
        ExternalMethod::Persistence => last,
        ExternalMethod::IndependentLinear => independent_forecast(values),
        _ => coupled_forecast(values),
    }
}

fn independent_forecast(values: &[Vec<f64>]) -> Vec<f64> {
    let variables = values[0].len();
    (0..variables)
        .map(|target| {
            let (features, outcomes): (Vec<_>, Vec<_>) = (1..values.len())
                .map(|time| (vec![1.0, values[time - 1][target]], values[time][target]))
                .unzip();
            let coefficients = fit_linear(&features, &outcomes, 1.0e-6);
            bounded_prediction(
                dot(&[1.0, values[values.len() - 1][target]], &coefficients),
                values,
                target,
            )
        })
        .collect()
}

fn coupled_forecast(values: &[Vec<f64>]) -> Vec<f64> {
    let coefficients = fit_var(values);
    forecast_with_var(values.last().unwrap_or(&Vec::new()), &coefficients, values)
}

fn fit_var(values: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let variables = values[0].len();
    let features: Vec<Vec<f64>> = (1..values.len())
        .map(|time| {
            let mut row = vec![1.0];
            row.extend_from_slice(&values[time - 1]);
            row
        })
        .collect();
    (0..variables)
        .map(|target| {
            let outcomes: Vec<f64> = (1..values.len()).map(|time| values[time][target]).collect();
            fit_linear(&features, &outcomes, 1.0e-4)
        })
        .collect()
}

fn forecast_with_var(
    previous: &[f64],
    coefficients: &[Vec<f64>],
    history: &[Vec<f64>],
) -> Vec<f64> {
    let mut features = vec![1.0];
    features.extend_from_slice(previous);
    coefficients
        .iter()
        .enumerate()
        .map(|(target, model)| bounded_prediction(dot(&features, model), history, target))
        .collect()
}

fn intervention_frontier(contract: &ExternalInterventionContract, variables: usize) -> bool {
    !contract.targets.is_empty()
        && !contract.query_target.is_empty()
        && contract
            .targets
            .iter()
            .chain(&contract.query_target)
            .all(|index| (*index as usize) < variables)
}

fn predict_query_values(
    values: &[Vec<f64>],
    contract: &ExternalInterventionContract,
    total_steps: usize,
    method: ExternalMethod,
    apply_contract: bool,
) -> Vec<f64> {
    let query_times: Vec<usize> = contract
        .query_time
        .iter()
        .map(|time| resolve_time(*time, total_steps))
        .collect();
    let max_time = query_times.iter().copied().max().unwrap_or(1);
    let coefficients = fit_var(values);
    let mut simulated = Vec::with_capacity(max_time + 1);
    simulated.push(values[0].clone());
    for time in 1..=max_time {
        let previous = simulated
            .last()
            .cloned()
            .unwrap_or_else(|| values[0].clone());
        let mut next = match method {
            ExternalMethod::Persistence => previous,
            ExternalMethod::IndependentLinear => independent_prefix_forecast(values, &previous),
            _ => forecast_with_var(&previous, &coefficients, values),
        };
        if apply_contract {
            apply_legal_intervention(&mut next, contract, time);
        }
        simulated.push(next);
    }
    contract
        .query_target
        .iter()
        .zip(query_times)
        .map(|(target, query_time)| {
            let target = *target as usize;
            let dynamic = simulated
                .get(query_time)
                .and_then(|row| row.get(target))
                .copied()
                .unwrap_or_else(|| values.last().map_or(0.0, |row| row[target]));
            if matches!(
                method,
                ExternalMethod::InterventionRegression | ExternalMethod::HybridMechanism
            ) && apply_contract
            {
                let regression = intervention_regression(values, contract, target, query_time);
                if method == ExternalMethod::HybridMechanism {
                    finite_or((dynamic + regression) * 0.5, dynamic)
                } else {
                    finite_or(regression, dynamic)
                }
            } else {
                finite_or(dynamic, values.last().map_or(0.0, |row| row[target]))
            }
        })
        .collect()
}

fn independent_prefix_forecast(values: &[Vec<f64>], previous: &[f64]) -> Vec<f64> {
    let variables = values[0].len();
    (0..variables)
        .map(|target| {
            let (features, outcomes): (Vec<_>, Vec<_>) = (1..values.len())
                .map(|time| (vec![1.0, values[time - 1][target]], values[time][target]))
                .unzip();
            let coefficients = fit_linear(&features, &outcomes, 1.0e-6);
            bounded_prediction(dot(&[1.0, previous[target]], &coefficients), values, target)
        })
        .collect()
}

fn resolve_time(raw: f64, total_steps: usize) -> usize {
    if raw.is_finite() && (0.0..=1.0).contains(&raw) {
        (raw * (total_steps.saturating_sub(1)) as f64).round() as usize
    } else if raw.is_finite() && raw > 1.0 {
        raw.round() as usize
    } else {
        1
    }
    .min(total_steps.saturating_sub(1))
    .max(1)
}

fn intervention_scalar(contract: &ExternalInterventionContract) -> Option<f64> {
    match contract.values.get("kind").and_then(Value::as_str) {
        Some("scalar") => contract.values.get("data").and_then(Value::as_f64),
        _ => None,
    }
}

fn apply_legal_intervention(
    state: &mut [f64],
    contract: &ExternalInterventionContract,
    time: usize,
) {
    if !contract
        .times
        .iter()
        .any(|candidate| *candidate as usize == time)
    {
        return;
    }
    let Some(value) = intervention_scalar(contract) else {
        return;
    };
    for target in &contract.targets {
        if let Some(cell) = state.get_mut(*target as usize) {
            if contract.intervention_type.eq_ignore_ascii_case("soft") {
                *cell = finite_or(*cell + value, *cell);
            } else {
                *cell = value;
            }
        }
    }
}

fn intervention_regression(
    values: &[Vec<f64>],
    contract: &ExternalInterventionContract,
    query_target: usize,
    query_time: usize,
) -> f64 {
    let baseline_index = query_time.min(values.len() - 1);
    let mut prediction = values[baseline_index][query_target];
    let Some(intervention_value) = intervention_scalar(contract) else {
        return prediction;
    };
    for source in &contract.targets {
        let source = *source as usize;
        if source >= values[0].len() {
            continue;
        }
        if source == query_target {
            prediction = intervention_value;
            continue;
        }
        let mut features = Vec::new();
        let mut outcomes = Vec::new();
        for time in 1..values.len() {
            features.push(vec![
                1.0,
                values[time - 1][query_target],
                values[time][source],
            ]);
            outcomes.push(values[time][query_target]);
        }
        let coefficients = fit_linear(&features, &outcomes, 1.0e-4);
        let effect = coefficients.get(2).copied().unwrap_or(0.0);
        let reference_time = contract
            .times
            .first()
            .copied()
            .unwrap_or(0)
            .min((values.len() - 1) as u64) as usize;
        prediction += effect * (intervention_value - values[reference_time][source]);
    }
    bounded_prediction(prediction, values, query_target)
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

fn bounded_prediction(value: f64, history: &[Vec<f64>], target: usize) -> f64 {
    let (minimum, maximum) = history.iter().map(|row| row[target]).fold(
        (f64::INFINITY, f64::NEG_INFINITY),
        |(minimum, maximum), value| (minimum.min(value), maximum.max(value)),
    );
    let span = (maximum - minimum).abs().max(1.0);
    finite_or(value, history.last().map_or(0.0, |row| row[target]))
        .clamp(minimum - span * 4.0, maximum + span * 4.0)
}

fn finite_or(value: f64, fallback: f64) -> f64 {
    if value.is_finite() {
        value
    } else if fallback.is_finite() {
        fallback
    } else {
        0.0
    }
}

fn finite_bits(value: f64) -> u64 {
    finite_or(value, 0.0).to_bits()
}

fn unexplained_dynamic_variance(values: &[Vec<f64>]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let mut residuals = Vec::new();
    for time in 1..values.len() {
        residuals.extend(
            values[time]
                .iter()
                .zip(&values[time - 1])
                .map(|(current, previous)| current - previous),
        );
    }
    let mean = residuals.iter().sum::<f64>() / residuals.len().max(1) as f64;
    residuals
        .iter()
        .map(|residual| (residual - mean).powi(2))
        .sum::<f64>()
        / residuals.len().max(1) as f64
}

pub fn prediction_commitment(predictions: &[Value]) -> Result<String, String> {
    let canonical: Vec<Value> = predictions.iter().map(canonicalize).collect();
    let bytes = serde_json::to_vec(&canonical).map_err(|error| error.to_string())?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.iter().map(canonicalize).collect()),
        Value::Object(values) => {
            let sorted: BTreeMap<String, Value> = values
                .iter()
                .map(|(key, value)| (key.clone(), canonicalize(value)))
                .collect();
            serde_json::to_value(sorted).unwrap_or(Value::Null)
        }
        _ => value.clone(),
    }
}

pub fn active_percentile(receipts: &[CaseResearchReceipt], percentile: f64, field: &str) -> u64 {
    let mut values: Vec<u64> = receipts
        .iter()
        .map(|receipt| match field {
            "variables" => receipt.active_variables,
            "mechanisms" => receipt.active_causal_mechanisms,
            "processes" => receipt.active_temporal_processes,
            _ => 0,
        })
        .collect();
    values.sort_unstable();
    if values.is_empty() {
        return 0;
    }
    let index = ((values.len() - 1) as f64 * percentile).ceil() as usize;
    values[index]
}

pub fn selected_frontier_ids(batch: &PredictionBatch) -> BTreeSet<String> {
    batch
        .case_receipts
        .iter()
        .filter(|receipt| receipt.self_detected_frontier)
        .map(|receipt| format!("EXTERNAL-FRONTIER-{}", receipt.case_id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_commitment_is_object_key_order_independent() {
        let left = vec![json!({"b": 2, "a": 1})];
        let right = vec![json!({"a": 1, "b": 2})];
        assert_eq!(
            prediction_commitment(&left).unwrap(),
            prediction_commitment(&right).unwrap()
        );
    }

    #[test]
    fn exact_finite_bits_are_never_changed_by_missingness_transport() {
        let value = -13.625_f64;
        assert_eq!(
            f64::from_bits(finite_bits(value)).to_bits(),
            value.to_bits()
        );
    }

    #[test]
    fn normalized_query_times_use_the_full_temporal_extent() {
        assert_eq!(resolve_time(0.5, 200), 100);
        assert_eq!(resolve_time(199.0, 200), 199);
    }
}
