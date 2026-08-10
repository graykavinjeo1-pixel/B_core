use std::{cmp::Ordering, collections::BTreeMap};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    sem37_r1::{adapter::R1ExternalLane, engine::prediction_commitment},
    sem37_r2::engine::{
        predict_lane_a as r2_predict_lane_a, predict_lane_b_recovered, CausalPrecisionMethod,
    },
};

use super::{
    adapter::{collect_cases, R3ExternalEvaluatorClient, R3ExternalSet},
    engine::{
        always_abstain_from, always_promote_from, no_change_predictions, predict_causal,
        predict_transfer, CausalBatch, DecompositionMethod, TransferBatch, TransferPolicy,
    },
    ontology::CausalRelationClass,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalCandidateEvidence {
    pub method: DecompositionMethod,
    pub raw_metrics: Value,
    pub causal_tests_performed: u64,
    pub conditional_ablation_evaluations: u64,
    pub mediator_hypotheses_considered: u64,
    pub direct_hypotheses_considered: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferCandidateEvidence {
    pub policy: TransferPolicy,
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
    pub causal_candidates: Vec<CausalCandidateEvidence>,
    pub selected_causal_method: DecompositionMethod,
    pub causal_selection_receipt: String,
    pub transfer_opportunity_counts: Value,
    pub transfer_candidates: Vec<TransferCandidateEvidence>,
    pub selected_transfer_policy: TransferPolicy,
    pub transfer_selection_receipt: String,
    pub candidate_pareto_front: Vec<String>,
    pub direct_mediated_decomposition_ablation_pass: bool,
    pub transfer_promotion_safety_ablation_pass: bool,
    pub transfer_safety_memory_ablation_pass: bool,
    pub always_abstain_baseline_dominated: bool,
    pub autonomous_research_epochs_executed: u64,
    pub human_causal_repair_selection_events: u64,
    pub human_mediator_rule_selection_events: u64,
    pub human_promotion_rule_selection_events: u64,
    pub final_external_exposure_events: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalEvaluation {
    pub schema_version: String,
    pub set: String,
    pub selected_causal_method: DecompositionMethod,
    pub selected_transfer_policy: TransferPolicy,
    pub r3_causal_batch: CausalBatch,
    pub r3_transfer_batch: TransferBatch,
    pub r2_causal_predictions: Vec<Value>,
    pub r2_causal_prediction_commitment: String,
    pub r2_transfer_predictions: Vec<Value>,
    pub r2_transfer_prediction_commitment: String,
    pub no_change_predictions: Vec<Value>,
    pub no_change_prediction_commitment: String,
    pub always_abstain_predictions: Vec<Value>,
    pub always_abstain_prediction_commitment: String,
    pub raw_arm_matrix: Value,
    pub final_outcomes_exposed_to_adaptive_research: bool,
    pub post_final_scientific_repairs: u64,
    pub post_final_policy_changes: u64,
}

pub fn run_development(
    evaluator: &R3ExternalEvaluatorClient,
) -> Result<AutonomousDevelopment, String> {
    let cases = collect_cases(evaluator, R3ExternalSet::DevD)?;
    let lane_a_count = cases
        .iter()
        .filter(|(descriptor, _)| descriptor.lane == R1ExternalLane::A)
        .count() as u64;
    let lane_b_count = cases
        .iter()
        .filter(|(descriptor, _)| descriptor.lane == R1ExternalLane::B)
        .count() as u64;
    let r2_batch = r2_predict_lane_a(
        &cases,
        CausalPrecisionMethod::PairwiseTriadStableAblationMdl,
    )?;
    let r2_metrics =
        evaluator.evaluate_causal(&r2_batch.predictions, &r2_batch.prediction_commitment)?;
    let mut causal_batches = BTreeMap::new();
    let mut causal_evidence = Vec::new();
    for method in DecompositionMethod::CANDIDATES {
        let batch = predict_causal(&cases, method)?;
        let metrics =
            evaluator.evaluate_causal(&batch.predictions, &batch.prediction_commitment)?;
        causal_evidence.push(CausalCandidateEvidence {
            method,
            raw_metrics: metrics,
            causal_tests_performed: batch.causal_tests_performed,
            conditional_ablation_evaluations: batch.conditional_ablation_evaluations,
            mediator_hypotheses_considered: batch.mediator_hypotheses_considered,
            direct_hypotheses_considered: batch.direct_hypotheses_considered,
        });
        causal_batches.insert(method, batch);
    }
    let selected_causal_method = select_causal(&causal_evidence, &r2_metrics)?;
    let no_change = no_change_predictions(&cases)?;
    let mut transfer_batches = BTreeMap::new();
    let mut candidate_payload = serde_json::Map::new();
    for policy in TransferPolicy::candidates() {
        let batch = predict_transfer(&cases, policy)?;
        candidate_payload.insert(
            policy.id(),
            json!({
                "predictions": batch.predictions,
                "prediction_commitment": batch.prediction_commitment
            }),
        );
        transfer_batches.insert(policy, batch);
    }
    let reference_policy = TransferPolicy::candidates()
        .into_iter()
        .next()
        .ok_or("SEM37_R3_NO_TRANSFER_POLICY")?;
    let always_promote = always_promote_from(&transfer_batches[&reference_policy])?;
    let always_abstain = always_abstain_from(&transfer_batches[&reference_policy], &no_change)?;
    candidate_payload.insert(
        "ABLATION_ALWAYS_PROMOTE".to_string(),
        json!({
            "predictions": always_promote.predictions,
            "prediction_commitment": always_promote.prediction_commitment
        }),
    );
    candidate_payload.insert(
        "ALWAYS_ABSTAIN".to_string(),
        json!({
            "predictions": always_abstain.predictions,
            "prediction_commitment": always_abstain.prediction_commitment
        }),
    );
    let transfer_matrix =
        evaluator.evaluate_transfer_development(Value::Object(candidate_payload), &no_change)?;
    let transfer_results = transfer_matrix["candidate_results"]
        .as_object()
        .ok_or("SEM37_R3_TRANSFER_RESULTS_MISSING")?;
    let transfer_evidence: Vec<_> = TransferPolicy::candidates()
        .into_iter()
        .map(|policy| {
            let batch = &transfer_batches[&policy];
            Ok(TransferCandidateEvidence {
                policy,
                raw_metrics: transfer_results
                    .get(&policy.id())
                    .cloned()
                    .ok_or_else(|| format!("SEM37_R3_TRANSFER_RESULT_MISSING:{}", policy.id()))?,
                promoted: batch.promoted,
                abstained: batch.abstained,
                rejected: batch.rejected,
            })
        })
        .collect::<Result<_, String>>()?;
    let selected_transfer_policy = select_transfer(&transfer_evidence)?;
    let selected_transfer = &transfer_batches[&selected_transfer_policy];
    let selected_transfer_metrics = transfer_results
        .get(&selected_transfer_policy.id())
        .ok_or("SEM37_R3_SELECTED_TRANSFER_RESULT_MISSING")?;
    let selected_always_promote = always_promote_from(selected_transfer)?;
    let ablation_payload = json!({
        "SELECTED": {
            "predictions": selected_transfer.predictions,
            "prediction_commitment": selected_transfer.prediction_commitment
        },
        "SELECTED_ALWAYS_PROMOTE": {
            "predictions": selected_always_promote.predictions,
            "prediction_commitment": selected_always_promote.prediction_commitment
        },
        "ALWAYS_ABSTAIN": {
            "predictions": always_abstain.predictions,
            "prediction_commitment": always_abstain.prediction_commitment
        }
    });
    let ablation_results = evaluator.evaluate_transfer_development(ablation_payload, &no_change)?;
    let ablations = &ablation_results["candidate_results"];
    let safety_ablation_pass = field(
        &ablations["SELECTED_ALWAYS_PROMOTE"],
        "negative_transfer_accepted",
    )? > field(selected_transfer_metrics, "negative_transfer_accepted")?
        || field(
            &ablations["SELECTED_ALWAYS_PROMOTE"],
            "positive_transfer_verified",
        )? < field(selected_transfer_metrics, "positive_transfer_verified")?;
    let zero_margin_policy = TransferPolicy {
        method: selected_transfer_policy.method,
        margin_basis_points: 0,
    };
    let memory_ablation_metrics = transfer_results
        .get(&zero_margin_policy.id())
        .ok_or("SEM37_R3_MEMORY_ABLATION_RESULT_MISSING")?;
    let memory_ablation_pass = selected_transfer_policy.margin_basis_points == 0
        || field(memory_ablation_metrics, "negative_transfer_accepted")?
            > field(selected_transfer_metrics, "negative_transfer_accepted")?
        || field(memory_ablation_metrics, "positive_transfer_verified")?
            < field(selected_transfer_metrics, "positive_transfer_verified")?;
    let decomposition_ablation_pass = field(&r2_metrics, "mediator_as_direct_misidentifications")?
        > field(
            &causal_evidence
                .iter()
                .find(|evidence| evidence.method == selected_causal_method)
                .ok_or("SEM37_R3_SELECTED_CAUSAL_EVIDENCE_MISSING")?
                .raw_metrics,
            "mediator_as_direct_misidentifications",
        )?;
    let always_abstain_dominated = field(selected_transfer_metrics, "positive_transfer_verified")?
        > 0
        && field(selected_transfer_metrics, "negative_transfer_accepted")? == 0
        && field(
            &causal_evidence
                .iter()
                .find(|evidence| evidence.method == selected_causal_method)
                .ok_or("SEM37_R3_SELECTED_CAUSAL_EVIDENCE_MISSING")?
                .raw_metrics,
            "direct_tp",
        )? > 0;
    let pareto = causal_evidence
        .iter()
        .filter(|evidence| causal_noninferior(&evidence.raw_metrics, &r2_metrics))
        .map(|evidence| format!("CAUSAL::{:?}", evidence.method))
        .chain(
            transfer_evidence
                .iter()
                .filter(|evidence| {
                    field(&evidence.raw_metrics, "negative_transfer_accepted")
                        .is_ok_and(|value| value == 0)
                        && field(&evidence.raw_metrics, "positive_transfer_verified")
                            .is_ok_and(|value| value > 0)
                })
                .map(|evidence| format!("TRANSFER::{}", evidence.policy.id())),
        )
        .collect();
    let epochs = DecompositionMethod::CANDIDATES.len() as u64 * lane_a_count
        + TransferPolicy::candidates().len() as u64 * lane_b_count;
    if epochs > super::config::MAX_AUTONOMOUS_RESEARCH_EPOCHS {
        return Err("SEM37_R3_CAMPAIGN_BUDGET_EXCEEDED".to_string());
    }
    Ok(AutonomousDevelopment {
        schema_version: "SEM37_R3_AUTONOMOUS_DEVELOPMENT_1".to_string(),
        diagnoses: vec![
            "MEDIATOR_ROLE_NOT_FIRST_CLASS".to_string(),
            "PATH_EVIDENCE_COLLAPSED_INTO_PAIRWISE_DIRECT_EDGE".to_string(),
            "TRANSFER_PROMOTION_LACKED_EXPLICIT_NO_CHANGE_CAUSAL_GATE".to_string(),
        ],
        causal_repair_hypotheses: vec![
            "FIRST_CLASS_DIRECT_MEDIATED_CONFOUNDED_UNRESOLVED_ONTOLOGY".to_string(),
            "STABLE_CONDITIONAL_EVIDENCE_COMPETES_WITH_MEDIATOR_PATHS".to_string(),
            "TARGET_VALIDATED_PROMOTION_WITH_EXPLICIT_NO_CHANGE".to_string(),
        ],
        diagnostic_experiments: vec![
            "PAIRED_R2_VS_CONDITIONAL_PATH_DECOMPOSITION".to_string(),
            "MULTI_THRESHOLD_PARETO_CAUSAL_SEARCH".to_string(),
            "TRANSFER_MARGIN_POLICY_MATRIX_WITH_ALWAYS_PROMOTE_AND_ABSTAIN".to_string(),
        ],
        r2_comparator_metrics: r2_metrics,
        causal_candidates: causal_evidence,
        selected_causal_method,
        causal_selection_receipt:
            "PARETO_ZERO_MEDIATOR_AND_COMMON_FALSE_DIRECT_THEN_R2_NONINFERIOR_PRECISION_RECALL"
                .to_string(),
        transfer_opportunity_counts: transfer_matrix["opportunity_counts"].clone(),
        transfer_candidates: transfer_evidence,
        selected_transfer_policy,
        transfer_selection_receipt:
            "ZERO_NEGATIVE_ACCEPTANCE_WITH_POSITIVE_VERIFICATION_THEN_COVERAGE_AND_ABSTENTION"
                .to_string(),
        candidate_pareto_front: pareto,
        direct_mediated_decomposition_ablation_pass: decomposition_ablation_pass,
        transfer_promotion_safety_ablation_pass: safety_ablation_pass,
        transfer_safety_memory_ablation_pass: memory_ablation_pass,
        always_abstain_baseline_dominated: always_abstain_dominated,
        autonomous_research_epochs_executed: epochs,
        human_causal_repair_selection_events: 0,
        human_mediator_rule_selection_events: 0,
        human_promotion_rule_selection_events: 0,
        final_external_exposure_events: 0,
    })
}

pub fn run_final(
    evaluator: &R3ExternalEvaluatorClient,
    development: &AutonomousDevelopment,
) -> Result<FinalEvaluation, String> {
    let cases = collect_cases(evaluator, R3ExternalSet::FinalE)?;
    let r3_causal = predict_causal(&cases, development.selected_causal_method)?;
    let r3_transfer = predict_transfer(&cases, development.selected_transfer_policy)?;
    let r2_causal = r2_predict_lane_a(
        &cases,
        CausalPrecisionMethod::PairwiseTriadStableAblationMdl,
    )?;
    let r2_transfer = predict_lane_b_recovered(
        &cases,
        CausalPrecisionMethod::PairwiseTriadStableAblationMdl,
    )?;
    let no_change = no_change_predictions(&cases)?;
    let no_change_commitment = prediction_commitment(&no_change)?;
    let always_abstain = always_abstain_from(&r3_transfer, &no_change)?;
    let unresolved = unresolved_predictions(&r3_causal.predictions)?;
    let unresolved_commitment = prediction_commitment(&unresolved)?;
    let arms = json!({
        "R2_COMPARATOR": {
            "lane_a_predictions": r2_causal.predictions,
            "lane_a_prediction_commitment": r2_causal.prediction_commitment,
            "lane_b_predictions": r2_transfer.predictions,
            "lane_b_prediction_commitment": r2_transfer.prediction_commitment,
            "research_work": r2_causal.causal_hypotheses_considered
                + r2_transfer.causal_hypotheses_considered
        },
        "R3_CANDIDATE": {
            "lane_a_predictions": r3_causal.predictions,
            "lane_a_prediction_commitment": r3_causal.prediction_commitment,
            "lane_b_predictions": r3_transfer.predictions,
            "lane_b_prediction_commitment": r3_transfer.prediction_commitment,
            "research_work": r3_causal.causal_tests_performed
                + r3_transfer.transfer_promotion_evaluations
        },
        "NO_CHANGE": {
            "lane_a_predictions": unresolved,
            "lane_a_prediction_commitment": unresolved_commitment,
            "lane_b_predictions": no_change,
            "lane_b_prediction_commitment": no_change_commitment,
            "research_work": 0
        },
        "ALWAYS_ABSTAIN": {
            "lane_a_predictions": unresolved,
            "lane_a_prediction_commitment": unresolved_commitment,
            "lane_b_predictions": always_abstain.predictions,
            "lane_b_prediction_commitment": always_abstain.prediction_commitment,
            "research_work": 0
        }
    });
    let raw_arm_matrix = evaluator.evaluate_matrix(arms)?;
    Ok(FinalEvaluation {
        schema_version: "SEM37_R3_FRESH_FINAL_E_EVALUATION_1".to_string(),
        set: "R3_FINAL_E".to_string(),
        selected_causal_method: development.selected_causal_method,
        selected_transfer_policy: development.selected_transfer_policy,
        r3_causal_batch: r3_causal,
        r3_transfer_batch: r3_transfer,
        r2_causal_predictions: r2_causal.predictions,
        r2_causal_prediction_commitment: r2_causal.prediction_commitment,
        r2_transfer_predictions: r2_transfer.predictions,
        r2_transfer_prediction_commitment: r2_transfer.prediction_commitment,
        no_change_predictions: no_change,
        no_change_prediction_commitment: no_change_commitment,
        always_abstain_predictions: always_abstain.predictions,
        always_abstain_prediction_commitment: always_abstain.prediction_commitment,
        raw_arm_matrix,
        final_outcomes_exposed_to_adaptive_research: false,
        post_final_scientific_repairs: 0,
        post_final_policy_changes: 0,
    })
}

fn select_causal(
    candidates: &[CausalCandidateEvidence],
    r2: &Value,
) -> Result<DecompositionMethod, String> {
    candidates
        .iter()
        .min_by(|left, right| compare_causal(left, right, r2))
        .map(|candidate| candidate.method)
        .ok_or("SEM37_R3_NO_CAUSAL_CANDIDATE".to_string())
}

fn compare_causal(
    left: &CausalCandidateEvidence,
    right: &CausalCandidateEvidence,
    r2: &Value,
) -> Ordering {
    let left_pass = causal_noninferior(&left.raw_metrics, r2);
    let right_pass = causal_noninferior(&right.raw_metrics, r2);
    right_pass
        .cmp(&left_pass)
        .then_with(|| {
            misidentifications(&left.raw_metrics).cmp(&misidentifications(&right.raw_metrics))
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

fn causal_noninferior(candidate: &Value, r2: &Value) -> bool {
    misidentifications(candidate) == Ok(0)
        && ratio_ge(candidate, r2, "direct_precision_exact")
        && ratio_ge(candidate, r2, "direct_recall_exact")
}

fn ratio_ge(left: &Value, right: &Value, field_name: &str) -> bool {
    let ln = field(&left[field_name], "numerator");
    let ld = field(&left[field_name], "denominator");
    let rn = field(&right[field_name], "numerator");
    let rd = field(&right[field_name], "denominator");
    match (ln, ld, rn, rd) {
        (Ok(ln), Ok(ld), Ok(rn), Ok(rd)) if ld > 0 && rd > 0 => {
            ln as u128 * rd as u128 >= rn as u128 * ld as u128
        }
        _ => false,
    }
}

fn misidentifications(metrics: &Value) -> Result<u64, String> {
    Ok(field(metrics, "mediator_as_direct_misidentifications")?
        + field(metrics, "common_cause_as_direct_misidentifications")?)
}

fn select_transfer(candidates: &[TransferCandidateEvidence]) -> Result<TransferPolicy, String> {
    candidates
        .iter()
        .min_by(compare_transfer)
        .map(|candidate| candidate.policy)
        .ok_or("SEM37_R3_NO_TRANSFER_CANDIDATE".to_string())
}

fn compare_transfer(
    left: &&TransferCandidateEvidence,
    right: &&TransferCandidateEvidence,
) -> Ordering {
    let safe_left = field(&left.raw_metrics, "negative_transfer_accepted")
        .is_ok_and(|value| value == 0)
        && field(&left.raw_metrics, "positive_transfer_verified").is_ok_and(|value| value > 0);
    let safe_right = field(&right.raw_metrics, "negative_transfer_accepted")
        .is_ok_and(|value| value == 0)
        && field(&right.raw_metrics, "positive_transfer_verified").is_ok_and(|value| value > 0);
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
        .then_with(|| left.policy.cmp(&right.policy))
}

fn unresolved_predictions(predictions: &[Value]) -> Result<Vec<Value>, String> {
    predictions
        .iter()
        .map(|prediction| {
            let relations = prediction["relations"]
                .as_array()
                .ok_or("SEM37_R3_RELATIONS_MISSING")?
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
                "causal_path_certificates": [],
                "mediation_path_certificates": []
            }))
        })
        .collect()
}

fn field(value: &Value, name: &str) -> Result<u64, String> {
    value[name]
        .as_u64()
        .ok_or_else(|| format!("SEM37_R3_RAW_FIELD_MISSING:{name}"))
}
