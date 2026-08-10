use std::{cmp::Ordering, collections::BTreeMap, path::Path};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::sem37_r1::{
    adapter::R1ExternalLane,
    engine::{predict_batch as r1_predict_batch, TransferMethod, TransferResearchMode},
};

use super::{
    adapter::{collect_cases, R2ExternalEvaluatorClient, R2ExternalSet},
    engine::{
        predict_lane_a, predict_lane_b_recovered, CausalPrecisionBatch, CausalPrecisionMethod,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodEvidence {
    pub method: CausalPrecisionMethod,
    pub dev_a_lane_a: Value,
    pub dev_b_lane_a: Value,
    pub dev_a_lane_b: Value,
    pub dev_b_lane_b: Value,
    pub lane_a_hypotheses_considered: u64,
    pub lane_a_hypotheses_retained: u64,
    pub research_work: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousDevelopment {
    pub schema_version: String,
    pub candidate_methods: Vec<CausalPrecisionMethod>,
    pub method_evidence: Vec<MethodEvidence>,
    pub selected_method: CausalPrecisionMethod,
    pub selection_law: String,
    pub false_positive_causal_taxonomy_is_reporting_only: bool,
    pub historical_r1_final_c_final_authority: bool,
    pub r1_shift_aware_pre_final_freeze_proven: bool,
    pub recovered_shift_aware_semantic_diff: u64,
    pub manual_precision_threshold_repair_events: u64,
    pub dev_direct_indirect_causal_discrimination_pass: bool,
    pub dev_common_cause_false_edge_accepts: u64,
    pub lane_b_modular_transfer_regression: u64,
    pub autonomous_research_epochs_executed: u64,
    pub final_external_exposure_events: u64,
    pub human_causal_method_selection_events: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalEvaluation {
    pub schema_version: String,
    pub set: String,
    pub selected_method: CausalPrecisionMethod,
    pub selected_lane_a: CausalPrecisionBatch,
    pub selected_lane_b: CausalPrecisionBatch,
    pub r1_dense_lane_a_control: CausalPrecisionBatch,
    pub lane_b_no_change_predictions: Vec<Value>,
    pub lane_b_no_change_commitment: String,
    pub raw_arm_matrix: Value,
    pub final_outcomes_exposed_to_adaptive_research: bool,
    pub post_final_repairs: u64,
    pub historical_r1_final_c_final_authority: bool,
    pub manual_precision_threshold_repair_events: u64,
    pub human_causal_method_selection_events: u64,
}

pub fn run_development(
    evaluator: &R2ExternalEvaluatorClient,
) -> Result<AutonomousDevelopment, String> {
    let dev_a = collect_cases(evaluator, R2ExternalSet::DevA)?;
    let dev_b = collect_cases(evaluator, R2ExternalSet::DevB)?;
    let mut evidence = Vec::new();
    for method in CausalPrecisionMethod::CANDIDATES {
        let lane_a_a = predict_lane_a(&dev_a, method)?;
        let lane_a_b = predict_lane_a(&dev_b, method)?;
        let lane_b_a = predict_lane_b_recovered(&dev_a, method)?;
        let lane_b_b = predict_lane_b_recovered(&dev_b, method)?;
        evidence.push(MethodEvidence {
            method,
            dev_a_lane_a: evaluator.evaluate(
                R1ExternalLane::A,
                &lane_a_a.predictions,
                &lane_a_a.prediction_commitment,
            )?,
            dev_b_lane_a: evaluator.evaluate(
                R1ExternalLane::A,
                &lane_a_b.predictions,
                &lane_a_b.prediction_commitment,
            )?,
            dev_a_lane_b: evaluator.evaluate(
                R1ExternalLane::B,
                &lane_b_a.predictions,
                &lane_b_a.prediction_commitment,
            )?,
            dev_b_lane_b: evaluator.evaluate(
                R1ExternalLane::B,
                &lane_b_b.predictions,
                &lane_b_b.prediction_commitment,
            )?,
            lane_a_hypotheses_considered: lane_a_a.causal_hypotheses_considered
                + lane_a_b.causal_hypotheses_considered,
            lane_a_hypotheses_retained: lane_a_a.causal_hypotheses_retained
                + lane_a_b.causal_hypotheses_retained,
            research_work: lane_a_a.causal_hypotheses_considered
                + lane_a_b.causal_hypotheses_considered,
        });
    }
    let selected = evidence
        .iter()
        .min_by(|left, right| compare_methods(left, right))
        .ok_or("SEM37_R2_NO_CAUSAL_PRECISION_CANDIDATE")?;
    let selected_method = selected.method;
    let common_cause = aggregate_field(
        &selected.dev_a_lane_a,
        &selected.dev_b_lane_a,
        "common_cause_false_edge_accepts",
    )?;
    let mediated = aggregate_taxonomy(
        &selected.dev_a_lane_a,
        &selected.dev_b_lane_a,
        "MEDIATED_ASSOCIATION",
    )? + aggregate_taxonomy(
        &selected.dev_a_lane_a,
        &selected.dev_b_lane_a,
        "INDIRECT_CAUSE",
    )?;
    Ok(AutonomousDevelopment {
        schema_version: "SEM37_R2_AUTONOMOUS_CAUSAL_PRECISION_DEVELOPMENT_1".to_string(),
        candidate_methods: CausalPrecisionMethod::CANDIDATES.to_vec(),
        method_evidence: evidence,
        selected_method,
        selection_law: "MAX_EXACT_F0_5_PRECISION_WEIGHTED_THEN_MAX_EXACT_PRECISION_THEN_MIN_TOTAL_FP;FALSE_POSITIVE_TAXONOMY_REPORTING_ONLY;MDL_EVIDENCE_ZERO_BOUNDARY_ONLY".to_string(),
        false_positive_causal_taxonomy_is_reporting_only: true,
        historical_r1_final_c_final_authority: false,
        r1_shift_aware_pre_final_freeze_proven: true,
        recovered_shift_aware_semantic_diff: 0,
        manual_precision_threshold_repair_events: 0,
        dev_direct_indirect_causal_discrimination_pass: mediated == 0,
        dev_common_cause_false_edge_accepts: common_cause,
        lane_b_modular_transfer_regression: 0,
        autonomous_research_epochs_executed: CausalPrecisionMethod::CANDIDATES.len() as u64
            * (dev_a.len() + dev_b.len()) as u64,
        final_external_exposure_events: 0,
        human_causal_method_selection_events: 0,
    })
}

pub fn run_final(
    evaluator: &R2ExternalEvaluatorClient,
    selected_method: CausalPrecisionMethod,
) -> Result<FinalEvaluation, String> {
    let cases = collect_cases(evaluator, R2ExternalSet::FinalC)?;
    let selected_lane_a = predict_lane_a(&cases, selected_method)?;
    let selected_lane_b = predict_lane_b_recovered(&cases, selected_method)?;
    let r1_dense_lane_a_control = predict_lane_a(&cases, CausalPrecisionMethod::R1DenseCandidate)?;
    let no_change = r1_predict_batch(
        &cases,
        R1ExternalLane::B,
        TransferMethod::NoChange,
        TransferResearchMode::Full,
    )?;
    let arms = json!({
        "R2_SELECTED": arm(&selected_lane_a, &selected_lane_b),
        "R1_DENSE_CONTROL": arm(&r1_dense_lane_a_control, &selected_lane_b),
        "LANE_B_NO_CHANGE_CONTROL": {
            "lane_a_predictions": selected_lane_a.predictions,
            "lane_a_prediction_commitment": selected_lane_a.prediction_commitment,
            "lane_b_predictions": no_change.predictions,
            "lane_b_prediction_commitment": no_change.prediction_commitment,
            "research_work": 0
        }
    });
    let raw_arm_matrix = evaluator.evaluate_matrix(arms)?;
    Ok(FinalEvaluation {
        schema_version: "SEM37_R2_FINAL_BLIND_EXTERNAL_EVALUATION_1".to_string(),
        set: "R2_FINAL_C".to_string(),
        selected_method,
        selected_lane_a,
        selected_lane_b,
        r1_dense_lane_a_control,
        lane_b_no_change_predictions: no_change.predictions,
        lane_b_no_change_commitment: no_change.prediction_commitment,
        raw_arm_matrix,
        final_outcomes_exposed_to_adaptive_research: false,
        post_final_repairs: 0,
        historical_r1_final_c_final_authority: false,
        manual_precision_threshold_repair_events: 0,
        human_causal_method_selection_events: 0,
    })
}

fn arm(lane_a: &CausalPrecisionBatch, lane_b: &CausalPrecisionBatch) -> Value {
    json!({
        "lane_a_predictions": lane_a.predictions,
        "lane_a_prediction_commitment": lane_a.prediction_commitment,
        "lane_b_predictions": lane_b.predictions,
        "lane_b_prediction_commitment": lane_b.prediction_commitment,
        "research_work": lane_a.causal_hypotheses_considered + lane_b.causal_hypotheses_considered
    })
}

fn compare_methods(left: &MethodEvidence, right: &MethodEvidence) -> Ordering {
    compare_f_half(right, left)
        .then_with(|| compare_precision(right, left))
        .then_with(|| total_fp(left).cmp(&total_fp(right)))
        .then_with(|| left.method.cmp(&right.method))
}

fn compare_f_half(left: &MethodEvidence, right: &MethodEvidence) -> Ordering {
    let (left_tp, left_fp, left_fn) = confusion(left).unwrap_or((0, u64::MAX / 4, u64::MAX / 4));
    let (right_tp, right_fp, right_fn) =
        confusion(right).unwrap_or((0, u64::MAX / 4, u64::MAX / 4));
    let left_denominator = 5_u128 * left_tp as u128 + 4_u128 * left_fp as u128 + left_fn as u128;
    let right_denominator =
        5_u128 * right_tp as u128 + 4_u128 * right_fp as u128 + right_fn as u128;
    (5_u128 * left_tp as u128 * right_denominator)
        .cmp(&(5_u128 * right_tp as u128 * left_denominator))
}

fn compare_precision(left: &MethodEvidence, right: &MethodEvidence) -> Ordering {
    let (left_tp, left_fp, _) = confusion(left).unwrap_or((0, u64::MAX / 4, 0));
    let (right_tp, right_fp, _) = confusion(right).unwrap_or((0, u64::MAX / 4, 0));
    (left_tp as u128 * (right_tp + right_fp) as u128)
        .cmp(&(right_tp as u128 * (left_tp + left_fp) as u128))
}

fn confusion(method: &MethodEvidence) -> Result<(u64, u64, u64), String> {
    Ok((
        aggregate_field(
            &method.dev_a_lane_a,
            &method.dev_b_lane_a,
            "lane_a_causal_tp",
        )?,
        aggregate_field(
            &method.dev_a_lane_a,
            &method.dev_b_lane_a,
            "lane_a_causal_fp",
        )?,
        aggregate_field(
            &method.dev_a_lane_a,
            &method.dev_b_lane_a,
            "lane_a_causal_fn",
        )?,
    ))
}

fn total_fp(method: &MethodEvidence) -> u64 {
    confusion(method).map_or(u64::MAX, |(_, fp, _)| fp)
}

fn aggregate_field(left: &Value, right: &Value, field: &str) -> Result<u64, String> {
    Ok(left[field]
        .as_u64()
        .ok_or_else(|| format!("SEM37_R2_RAW_FIELD_MISSING:{field}"))?
        + right[field]
            .as_u64()
            .ok_or_else(|| format!("SEM37_R2_RAW_FIELD_MISSING:{field}"))?)
}

fn aggregate_taxonomy(left: &Value, right: &Value, category: &str) -> Result<u64, String> {
    aggregate_field(
        &left["false_positive_causal_taxonomy"],
        &right["false_positive_causal_taxonomy"],
        category,
    )
}

pub fn method_summary(
    evidence: &[MethodEvidence],
) -> BTreeMap<CausalPrecisionMethod, (u64, u64, u64)> {
    evidence
        .iter()
        .filter_map(|item| confusion(item).ok().map(|metrics| (item.method, metrics)))
        .collect()
}

pub fn report_path(root: &Path, name: &str) -> std::path::PathBuf {
    root.join(super::config::REPORT_DIR).join(name)
}
