use std::{cmp::Ordering, collections::BTreeMap};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    sem37_r1::engine::{prediction_commitment, TransferMethod},
    sem37_r2::engine::{
        predict_lane_a as r2_predict_lane_a, predict_lane_b_recovered, CausalPrecisionMethod,
    },
    sem37_r3::engine::{
        predict_causal as r3_predict_causal, predict_transfer as r3_predict_transfer,
        DecompositionMethod, TransferPolicy,
    },
};

use super::{
    adapter::{collect_cases, R4ExternalEvaluatorClient, R4ExternalSet},
    engine::{
        always_abstain_from, always_apply_from, no_change_transfer, predict_causal,
        predict_transfer, unresolved_causal_predictions, CounterfactualTransferPolicy,
        EffectDecompositionMethod, R4CausalBatch, R4TransferBatch,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalCandidateEvidence {
    pub method: EffectDecompositionMethod,
    pub raw_metrics: Value,
    pub effect_decomposition_evaluations: u64,
    pub candidate_mediator_paths: u64,
    pub conditional_evaluations: u64,
    pub interventional_evaluations: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferCandidateEvidence {
    pub policy: CounterfactualTransferPolicy,
    pub raw_metrics: Value,
    pub promoted: u64,
    pub abstained: u64,
    pub rejected: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousDevelopment {
    pub schema_version: String,
    pub diagnoses: Vec<String>,
    pub causal_repair_hypotheses: Vec<String>,
    pub diagnostic_experiments: Vec<String>,
    pub r2_comparator_metrics: Value,
    pub r3_comparator_metrics: Value,
    pub causal_candidates: Vec<CausalCandidateEvidence>,
    pub selected_causal_method: EffectDecompositionMethod,
    pub causal_selection_receipt: String,
    pub transfer_opportunity_counts: Value,
    pub transfer_candidates: Vec<TransferCandidateEvidence>,
    pub selected_transfer_policy: CounterfactualTransferPolicy,
    pub transfer_selection_receipt: String,
    pub candidate_pareto_front: Vec<String>,
    pub direct_effect_decomposition_ablation_pass: bool,
    pub total_effect_only_baseline_dominated: bool,
    pub r3_taxonomy_only_baseline_dominated: bool,
    pub no_change_counterfactual_promotion_ablation_pass: bool,
    pub transfer_safety_memory_ablation_pass: bool,
    pub always_abstain_baseline_dominated: bool,
    pub autonomous_research_epochs_executed: u64,
    pub human_causal_repair_selection_events: u64,
    pub human_effect_decomposition_rule_selection_events: u64,
    pub human_mediator_rule_selection_events: u64,
    pub human_promotion_rule_selection_events: u64,
    pub final_external_exposure_events: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalEvaluation {
    pub schema_version: String,
    pub set: String,
    pub selected_causal_method: EffectDecompositionMethod,
    pub selected_transfer_policy: CounterfactualTransferPolicy,
    pub r4_causal_batch: R4CausalBatch,
    pub r4_transfer_batch: R4TransferBatch,
    pub r2_causal_predictions: Vec<Value>,
    pub r2_causal_prediction_commitment: String,
    pub r2_transfer_predictions: Vec<Value>,
    pub r2_transfer_prediction_commitment: String,
    pub r3_causal_predictions: Vec<Value>,
    pub r3_causal_prediction_commitment: String,
    pub r3_transfer_predictions: Vec<Value>,
    pub r3_transfer_prediction_commitment: String,
    pub no_change_predictions: Vec<Value>,
    pub no_change_prediction_commitment: String,
    pub raw_arm_matrix: Value,
    pub final_outcomes_exposed_to_adaptive_research: bool,
    pub post_final_scientific_repairs: u64,
    pub post_final_promotion_policy_changes: u64,
    pub post_final_verifier_changes: u64,
    pub post_final_acceptance_changes: u64,
}

pub fn run_development(
    evaluator: &R4ExternalEvaluatorClient,
) -> Result<AutonomousDevelopment, String> {
    let cases = collect_cases(evaluator, R4ExternalSet::DevF)?;
    let lane_a_count = cases
        .iter()
        .filter(|(descriptor, _)| descriptor.lane == crate::sem37_r1::adapter::R1ExternalLane::A)
        .count() as u64;
    let lane_b_count = cases.len() as u64 - lane_a_count;
    let r2 = r2_predict_lane_a(
        &cases,
        CausalPrecisionMethod::PairwiseTriadStableAblationMdl,
    )?;
    let r2_metrics = evaluator.evaluate_causal(&r2.predictions, &r2.prediction_commitment)?;
    let r3 = r3_predict_causal(&cases, DecompositionMethod::ConditionalPath32)?;
    let r3_metrics = evaluator.evaluate_causal(&r3.predictions, &r3.prediction_commitment)?;
    let mut causal_batches = BTreeMap::new();
    let mut causal_evidence = Vec::new();
    for method in EffectDecompositionMethod::CANDIDATES {
        let batch = predict_causal(&cases, method)?;
        let metrics =
            evaluator.evaluate_causal(&batch.predictions, &batch.prediction_commitment)?;
        causal_evidence.push(CausalCandidateEvidence {
            method,
            raw_metrics: metrics,
            effect_decomposition_evaluations: batch.effect_decomposition_evaluations,
            candidate_mediator_paths: batch.candidate_mediator_paths,
            conditional_evaluations: batch.conditional_evaluations,
            interventional_evaluations: batch.interventional_evaluations,
        });
        causal_batches.insert(method, batch);
    }
    let selected_causal_method = select_causal(&causal_evidence, &r2_metrics, &r3_metrics)?;
    let no_change = no_change_transfer(&cases)?;
    let mut transfer_batches = BTreeMap::new();
    let mut payload = serde_json::Map::new();
    for policy in CounterfactualTransferPolicy::candidates() {
        let batch = predict_transfer(&cases, policy)?;
        payload.insert(
            policy.id(),
            json!({
                "predictions": batch.predictions,
                "prediction_commitment": batch.prediction_commitment
            }),
        );
        transfer_batches.insert(policy, batch);
    }
    let transfer_matrix =
        evaluator.evaluate_transfer_development(Value::Object(payload), &no_change.predictions)?;
    let results = transfer_matrix["candidate_results"]
        .as_object()
        .ok_or("SEM37_R4_TRANSFER_RESULTS_MISSING")?;
    let transfer_evidence: Vec<_> = CounterfactualTransferPolicy::candidates()
        .into_iter()
        .map(|policy| {
            let batch = &transfer_batches[&policy];
            Ok(TransferCandidateEvidence {
                policy,
                raw_metrics: results
                    .get(&policy.id())
                    .cloned()
                    .ok_or_else(|| format!("SEM37_R4_TRANSFER_RESULT_MISSING:{}", policy.id()))?,
                promoted: batch.promoted,
                abstained: batch.abstained,
                rejected: batch.rejected,
            })
        })
        .collect::<Result<_, String>>()?;
    let selected_transfer_policy = select_transfer(&transfer_evidence)?;
    let selected_transfer = &transfer_batches[&selected_transfer_policy];
    let selected_metrics = results
        .get(&selected_transfer_policy.id())
        .ok_or("SEM37_R4_SELECTED_TRANSFER_RESULT_MISSING")?;
    let always_apply = always_apply_from(selected_transfer)?;
    let always_abstain = always_abstain_from(selected_transfer)?;
    let memory_policy = CounterfactualTransferPolicy {
        method: selected_transfer_policy.method,
        margin_basis_points: 0,
        maximum_uncertainty_millionths: 1_000_000,
    };
    let memory_ablation = &transfer_batches[&memory_policy];
    let ablation_payload = json!({
        "SELECTED": {
            "predictions": selected_transfer.predictions,
            "prediction_commitment": selected_transfer.prediction_commitment
        },
        "NO_CHANGE_COUNTERFACTUAL_ABLATED": {
            "predictions": always_apply.predictions,
            "prediction_commitment": always_apply.prediction_commitment
        },
        "TRANSFER_MEMORY_ABLATED": {
            "predictions": memory_ablation.predictions,
            "prediction_commitment": memory_ablation.prediction_commitment
        },
        "ALWAYS_ABSTAIN": {
            "predictions": always_abstain.predictions,
            "prediction_commitment": always_abstain.prediction_commitment
        }
    });
    let ablations =
        evaluator.evaluate_transfer_development(ablation_payload, &no_change.predictions)?;
    let ablation_results = &ablations["candidate_results"];
    let selected_causal = causal_evidence
        .iter()
        .find(|evidence| evidence.method == selected_causal_method)
        .ok_or("SEM37_R4_SELECTED_CAUSAL_EVIDENCE_MISSING")?;
    let direct_ablation = field(&r3_metrics, "mediator_as_direct_misidentifications")?
        > field(
            &selected_causal.raw_metrics,
            "mediator_as_direct_misidentifications",
        )?
        || bool_field(
            &selected_causal.raw_metrics,
            "mixed_direct_mediated_decomposition_pass",
        )? && !bool_field(&r3_metrics, "mixed_direct_mediated_decomposition_pass")?;
    let no_change_ablation = field(
        &ablation_results["NO_CHANGE_COUNTERFACTUAL_ABLATED"],
        "negative_transfer_accepted",
    )? > field(selected_metrics, "negative_transfer_accepted")?
        || field(
            &ablation_results["NO_CHANGE_COUNTERFACTUAL_ABLATED"],
            "positive_transfer_verified",
        )? < field(selected_metrics, "positive_transfer_verified")?;
    let memory_ablation_pass = field(
        &ablation_results["TRANSFER_MEMORY_ABLATED"],
        "negative_transfer_accepted",
    )? > field(selected_metrics, "negative_transfer_accepted")?
        || field(
            &ablation_results["TRANSFER_MEMORY_ABLATED"],
            "ambiguous_transfer_abstentions",
        )? < field(selected_metrics, "ambiguous_transfer_abstentions")?;
    let always_abstain_dominated = field(selected_metrics, "positive_transfer_verified")? > 0
        && field(selected_metrics, "negative_transfer_accepted")? == 0;
    let causal_pareto = causal_evidence
        .iter()
        .filter(|evidence| causal_hard_gate(&evidence.raw_metrics, &r2_metrics, &r3_metrics));
    let transfer_pareto = transfer_evidence.iter().filter(|evidence| {
        field(&evidence.raw_metrics, "negative_transfer_accepted").is_ok_and(|value| value == 0)
            && field(&evidence.raw_metrics, "positive_transfer_verified")
                .is_ok_and(|value| value > 0)
            && field(&evidence.raw_metrics, "ambiguous_transfer_abstentions")
                .is_ok_and(|value| value > 0)
    });
    let pareto = causal_pareto
        .map(|evidence| format!("CAUSAL::{:?}", evidence.method))
        .chain(transfer_pareto.map(|evidence| format!("TRANSFER::{}", evidence.policy.id())))
        .collect();
    let epochs = EffectDecompositionMethod::CANDIDATES.len() as u64 * lane_a_count
        + CounterfactualTransferPolicy::candidates().len() as u64 * lane_b_count;
    if epochs > super::config::MAX_AUTONOMOUS_RESEARCH_EPOCHS {
        return Err("SEM37_R4_CAMPAIGN_BUDGET_EXCEEDED".to_string());
    }
    Ok(AutonomousDevelopment {
        schema_version: "SEM37_R4_AUTONOMOUS_DEVELOPMENT_1".to_string(),
        diagnoses: vec![
            "TOTAL_AND_DIRECT_COMPONENTS_WERE_NOT_SEPARATE_PROMOTION_AUTHORITIES".to_string(),
            "MEDIATOR_PATH_WAS_REPRESENTED_BUT_NOT_SUBTRACTED_FROM_DIRECT_RESIDUAL".to_string(),
            "TRANSFER_PROMOTION_UNCERTAINTY_DID_NOT_GATE_APPLY_VS_NO_CHANGE".to_string(),
        ],
        causal_repair_hypotheses: vec![
            "EXPLICIT_TOTAL_DIRECT_MEDIATED_CONFOUNDING_RESIDUAL_ACCOUNTING".to_string(),
            "FROZEN_RESIDUAL_DIRECT_COMPONENT_THRESHOLD_FAMILY".to_string(),
            "COUNTERFACTUAL_PROMOTION_WITH_MARGIN_AND_UNCERTAINTY".to_string(),
        ],
        diagnostic_experiments: vec![
            "PAIRED_R2_R3_R4_EFFECT_DECOMPOSITION_SEARCH".to_string(),
            "MIXED_DIRECT_MEDIATED_RETENTION_TEST".to_string(),
            "APPLY_NO_CHANGE_MARGIN_UNCERTAINTY_PARETO_SEARCH".to_string(),
        ],
        r2_comparator_metrics: r2_metrics.clone(),
        r3_comparator_metrics: r3_metrics.clone(),
        causal_candidates: causal_evidence,
        selected_causal_method,
        causal_selection_receipt:
            "PARETO_ZERO_FALSE_DIRECT_THEN_MIXED_AND_BEST_COMPARATOR_NONINFERIOR".to_string(),
        transfer_opportunity_counts: transfer_matrix["opportunity_counts"].clone(),
        transfer_candidates: transfer_evidence,
        selected_transfer_policy,
        transfer_selection_receipt:
            "ZERO_NEGATIVE_THEN_POSITIVE_VERIFICATION_AND_AMBIGUOUS_ABSTENTION".to_string(),
        candidate_pareto_front: pareto,
        direct_effect_decomposition_ablation_pass: direct_ablation,
        total_effect_only_baseline_dominated: direct_ablation,
        r3_taxonomy_only_baseline_dominated: direct_ablation,
        no_change_counterfactual_promotion_ablation_pass: no_change_ablation,
        transfer_safety_memory_ablation_pass: memory_ablation_pass,
        always_abstain_baseline_dominated: always_abstain_dominated,
        autonomous_research_epochs_executed: epochs,
        human_causal_repair_selection_events: 0,
        human_effect_decomposition_rule_selection_events: 0,
        human_mediator_rule_selection_events: 0,
        human_promotion_rule_selection_events: 0,
        final_external_exposure_events: 0,
    })
}

pub fn run_final(
    evaluator: &R4ExternalEvaluatorClient,
    development: &AutonomousDevelopment,
) -> Result<FinalEvaluation, String> {
    let cases = collect_cases(evaluator, R4ExternalSet::FinalG)?;
    let r4_causal = predict_causal(&cases, development.selected_causal_method)?;
    let r4_transfer = predict_transfer(&cases, development.selected_transfer_policy)?;
    let r2_causal = r2_predict_lane_a(
        &cases,
        CausalPrecisionMethod::PairwiseTriadStableAblationMdl,
    )?;
    let r2_transfer = predict_lane_b_recovered(
        &cases,
        CausalPrecisionMethod::PairwiseTriadStableAblationMdl,
    )?;
    let r3_causal = r3_predict_causal(&cases, DecompositionMethod::ConditionalPath32)?;
    let r3_transfer = r3_predict_transfer(
        &cases,
        TransferPolicy {
            method: TransferMethod::SparseCoupledLinear,
            margin_basis_points: 0,
        },
    )?;
    let no_change = no_change_transfer(&cases)?;
    let always_abstain = always_abstain_from(&r4_transfer)?;
    let always_apply = always_apply_from(&r4_transfer)?;
    let memory_ablation = predict_transfer(
        &cases,
        CounterfactualTransferPolicy {
            method: development.selected_transfer_policy.method,
            margin_basis_points: 0,
            maximum_uncertainty_millionths: 1_000_000,
        },
    )?;
    let unresolved = unresolved_causal_predictions(&r4_causal)?;
    let unresolved_commitment = prediction_commitment(&unresolved)?;
    let arms = json!({
        "R2_COMPARATOR": arm(&r2_causal.predictions, &r2_causal.prediction_commitment,
            &r2_transfer.predictions, &r2_transfer.prediction_commitment, 0),
        "R3_COMPARATOR": arm(&r3_causal.predictions, &r3_causal.prediction_commitment,
            &r3_transfer.predictions, &r3_transfer.prediction_commitment, 0),
        "R4_CANDIDATE": arm(&r4_causal.predictions, &r4_causal.prediction_commitment,
            &r4_transfer.predictions, &r4_transfer.prediction_commitment,
            r4_causal.effect_decomposition_evaluations + r4_transfer.transfer_promotion_evaluations),
        "NO_CHANGE": arm(&unresolved, &unresolved_commitment,
            &no_change.predictions, &no_change.prediction_commitment, 0),
        "TOTAL_EFFECT_ONLY": arm(&r2_causal.predictions, &r2_causal.prediction_commitment,
            &no_change.predictions, &no_change.prediction_commitment, 0),
        "R3_TAXONOMY_ONLY": arm(&r3_causal.predictions, &r3_causal.prediction_commitment,
            &r3_transfer.predictions, &r3_transfer.prediction_commitment, 0),
        "DIRECT_DECOMPOSITION_ABLATED": arm(&r3_causal.predictions, &r3_causal.prediction_commitment,
            &r4_transfer.predictions, &r4_transfer.prediction_commitment, 0),
        "NO_CHANGE_COUNTERFACTUAL_ABLATED": arm(&r4_causal.predictions, &r4_causal.prediction_commitment,
            &always_apply.predictions, &always_apply.prediction_commitment, 0),
        "TRANSFER_MEMORY_ABLATED": arm(&r4_causal.predictions, &r4_causal.prediction_commitment,
            &memory_ablation.predictions, &memory_ablation.prediction_commitment, 0),
        "ALWAYS_ABSTAIN": arm(&r4_causal.predictions, &r4_causal.prediction_commitment,
            &always_abstain.predictions, &always_abstain.prediction_commitment, 0)
    });
    let raw_arm_matrix = evaluator.evaluate_matrix(arms)?;
    Ok(FinalEvaluation {
        schema_version: "SEM37_R4_FRESH_FINAL_G_EVALUATION_1".to_string(),
        set: "R4_FINAL_G".to_string(),
        selected_causal_method: development.selected_causal_method,
        selected_transfer_policy: development.selected_transfer_policy,
        r4_causal_batch: r4_causal,
        r4_transfer_batch: r4_transfer,
        r2_causal_predictions: r2_causal.predictions,
        r2_causal_prediction_commitment: r2_causal.prediction_commitment,
        r2_transfer_predictions: r2_transfer.predictions,
        r2_transfer_prediction_commitment: r2_transfer.prediction_commitment,
        r3_causal_predictions: r3_causal.predictions,
        r3_causal_prediction_commitment: r3_causal.prediction_commitment,
        r3_transfer_predictions: r3_transfer.predictions,
        r3_transfer_prediction_commitment: r3_transfer.prediction_commitment,
        no_change_predictions: no_change.predictions,
        no_change_prediction_commitment: no_change.prediction_commitment,
        raw_arm_matrix,
        final_outcomes_exposed_to_adaptive_research: false,
        post_final_scientific_repairs: 0,
        post_final_promotion_policy_changes: 0,
        post_final_verifier_changes: 0,
        post_final_acceptance_changes: 0,
    })
}

fn arm(
    causal: &[Value],
    causal_commitment: &str,
    transfer: &[Value],
    transfer_commitment: &str,
    research_work: u64,
) -> Value {
    json!({
        "lane_a_predictions": causal,
        "lane_a_prediction_commitment": causal_commitment,
        "lane_b_predictions": transfer,
        "lane_b_prediction_commitment": transfer_commitment,
        "research_work": research_work
    })
}

fn select_causal(
    candidates: &[CausalCandidateEvidence],
    r2: &Value,
    r3: &Value,
) -> Result<EffectDecompositionMethod, String> {
    candidates
        .iter()
        .min_by(|left, right| compare_causal(left, right, r2, r3))
        .map(|candidate| candidate.method)
        .ok_or("SEM37_R4_NO_CAUSAL_CANDIDATE".to_string())
}

fn compare_causal(
    left: &CausalCandidateEvidence,
    right: &CausalCandidateEvidence,
    r2: &Value,
    r3: &Value,
) -> Ordering {
    let left_pass = causal_hard_gate(&left.raw_metrics, r2, r3);
    let right_pass = causal_hard_gate(&right.raw_metrics, r2, r3);
    right_pass
        .cmp(&left_pass)
        .then_with(|| {
            misidentifications(&left.raw_metrics).cmp(&misidentifications(&right.raw_metrics))
        })
        .then_with(|| {
            bool_field(
                &right.raw_metrics,
                "mixed_direct_mediated_decomposition_pass",
            )
            .cmp(&bool_field(
                &left.raw_metrics,
                "mixed_direct_mediated_decomposition_pass",
            ))
        })
        .then_with(|| {
            field(&right.raw_metrics, "direct_tp").cmp(&field(&left.raw_metrics, "direct_tp"))
        })
        .then_with(|| {
            field(&right.raw_metrics, "mediated_true_positives")
                .cmp(&field(&left.raw_metrics, "mediated_true_positives"))
        })
        .then_with(|| left.method.cmp(&right.method))
}

fn causal_hard_gate(candidate: &Value, r2: &Value, r3: &Value) -> bool {
    misidentifications(candidate) == Ok(0)
        && bool_field(candidate, "mixed_direct_mediated_decomposition_pass") == Ok(true)
        && ratio_ge(candidate, r2, "direct_precision_exact")
        && ratio_ge(candidate, r3, "direct_precision_exact")
        && ratio_ge(candidate, r2, "direct_recall_exact")
        && ratio_ge(candidate, r3, "direct_recall_exact")
}

fn select_transfer(
    candidates: &[TransferCandidateEvidence],
) -> Result<CounterfactualTransferPolicy, String> {
    candidates
        .iter()
        .min_by(compare_transfer)
        .map(|candidate| candidate.policy)
        .ok_or("SEM37_R4_NO_TRANSFER_CANDIDATE".to_string())
}

fn compare_transfer(
    left: &&TransferCandidateEvidence,
    right: &&TransferCandidateEvidence,
) -> Ordering {
    let safe_left = transfer_hard_gate(&left.raw_metrics);
    let safe_right = transfer_hard_gate(&right.raw_metrics);
    safe_right
        .cmp(&safe_left)
        .then_with(|| {
            field(&left.raw_metrics, "negative_transfer_accepted")
                .cmp(&field(&right.raw_metrics, "negative_transfer_accepted"))
        })
        .then_with(|| {
            field(&right.raw_metrics, "positive_transfer_verified")
                .cmp(&field(&left.raw_metrics, "positive_transfer_verified"))
        })
        .then_with(|| {
            field(&right.raw_metrics, "ambiguous_transfer_abstentions")
                .cmp(&field(&left.raw_metrics, "ambiguous_transfer_abstentions"))
        })
        .then_with(|| left.promoted.cmp(&right.promoted))
        .then_with(|| left.policy.cmp(&right.policy))
}

fn transfer_hard_gate(metrics: &Value) -> bool {
    field(metrics, "negative_transfer_accepted").is_ok_and(|value| value == 0)
        && field(metrics, "positive_transfer_verified").is_ok_and(|value| value > 0)
        && field(metrics, "ambiguous_transfer_abstentions").is_ok_and(|value| value > 0)
}

pub fn ratio_ge(left: &Value, right: &Value, name: &str) -> bool {
    let ln = field(&left[name], "numerator");
    let ld = field(&left[name], "denominator");
    let rn = field(&right[name], "numerator");
    let rd = field(&right[name], "denominator");
    matches!((ln, ld, rn, rd), (Ok(ln), Ok(ld), Ok(rn), Ok(rd)) if ld > 0 && rd > 0 && ln as u128 * rd as u128 >= rn as u128 * ld as u128)
}

pub fn misidentifications(metrics: &Value) -> Result<u64, String> {
    Ok(field(metrics, "mediator_as_direct_misidentifications")?
        + field(metrics, "common_cause_as_direct_misidentifications")?)
}

pub fn field(value: &Value, name: &str) -> Result<u64, String> {
    value[name]
        .as_u64()
        .ok_or_else(|| format!("SEM37_R4_RAW_FIELD_MISSING:{name}"))
}

pub fn bool_field(value: &Value, name: &str) -> Result<bool, String> {
    value[name]
        .as_bool()
        .ok_or_else(|| format!("SEM37_R4_RAW_BOOL_FIELD_MISSING:{name}"))
}
