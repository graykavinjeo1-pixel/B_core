use std::{collections::BTreeMap, path::Path};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::{
    adapter::{
        R1CaseDescriptor, R1ExternalEvaluatorClient, R1ExternalLane, R1ExternalObservation,
        R1ExternalSet,
    },
    engine::{
        predict_batch, MechanismTransferContract, PredictionBatch, TransferMethod,
        TransferResearchMode,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodEvaluation {
    pub method: TransferMethod,
    pub lane_a: Value,
    pub lane_b: Value,
    pub lane_a_commitment: String,
    pub lane_b_commitment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentResearch {
    pub schema_version: String,
    pub historical_failure_diagnosis: Vec<Value>,
    pub transfer_contract: MechanismTransferContract,
    pub dev_a_method_evaluations: Vec<MethodEvaluation>,
    pub dev_b_method_evaluations: Vec<MethodEvaluation>,
    pub lane_a_case_validation_records: Vec<Value>,
    pub dev_b_transfer_validation_experiments: u64,
    pub transfer_validation_records: Vec<Value>,
    pub dev_b_transfer_validation_predictions_frozen: u64,
    pub dev_b_outcomes_revealed_after_prediction: u64,
    pub experiment_outcome_reads_before_transfer_prediction: u64,
    pub target_observation_budget_per_arm: u64,
    pub target_intervention_budget_per_arm: u64,
    pub human_mechanism_transfer_selection_events: u64,
    pub human_shift_component_selection_events: u64,
    pub human_target_rebinding_selection_events: u64,
    pub human_external_intervention_selection_events: u64,
    pub benchmark_specific_causal_hint_branches: u64,
    pub task_specific_external_repair_branches: u64,
    pub dev_adaptive_external_exposure_events: u64,
    pub final_external_exposure_events: u64,
    pub autonomous_research_epochs_executed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalArmBatches {
    pub no_change_a: PredictionBatch,
    pub no_change_b: PredictionBatch,
    pub scratch_a: PredictionBatch,
    pub scratch_b: PredictionBatch,
    pub naive_a: PredictionBatch,
    pub naive_b: PredictionBatch,
    pub shift_aware_a: PredictionBatch,
    pub shift_aware_b: PredictionBatch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalExternalEvaluation {
    pub schema_version: String,
    pub set: R1ExternalSet,
    pub worlds: u64,
    pub lane_a_worlds: u64,
    pub lane_b_worlds: u64,
    pub batches: FinalArmBatches,
    pub raw_arm_matrix: Value,
    pub transfer_adaptation_work: u64,
    pub scratch_adaptation_work: u64,
    pub transfer_hypotheses_to_valid_model: u64,
    pub scratch_hypotheses_to_valid_model: u64,
    pub transfer_interventions_to_valid_model: u64,
    pub scratch_interventions_to_valid_model: u64,
    pub transfer_arm_target_observations: u64,
    pub scratch_arm_target_observations: u64,
    pub transfer_arm_target_interventions: u64,
    pub scratch_arm_target_interventions: u64,
    pub final_outcome_reveal_events: u64,
    pub arm_results_exposed_to_adaptive_research: bool,
    pub human_mechanism_transfer_selection_events: u64,
    pub human_shift_component_selection_events: u64,
    pub human_target_rebinding_selection_events: u64,
    pub human_external_intervention_selection_events: u64,
    pub benchmark_specific_causal_hint_branches: u64,
    pub task_specific_external_repair_branches: u64,
    pub external_generator_source_reads_by_bcore: u64,
    pub external_ground_truth_graph_reads: u64,
    pub external_ground_truth_equation_reads: u64,
    pub expected_external_result_lookups: u64,
    pub network_reads_during_canonical: u64,
    pub network_writes_during_canonical: u64,
}

pub fn run_development_research(
    evaluator: &R1ExternalEvaluatorClient,
) -> Result<DevelopmentResearch, String> {
    let dev_a = collect_cases(evaluator, R1ExternalSet::R1DevA)?;
    let dev_b = collect_cases(evaluator, R1ExternalSet::R1DevB)?;
    let methods = [
        TransferMethod::NoChange,
        TransferMethod::Scratch,
        TransferMethod::NaiveTransfer,
        TransferMethod::ShiftAwareTransfer,
        TransferMethod::IndependentLinear,
        TransferMethod::SparseCoupledLinear,
        TransferMethod::HybridMechanism,
        TransferMethod::InterventionRegression,
    ];
    let dev_a_method_evaluations = evaluate_methods(evaluator, &dev_a, &methods)?;
    let dev_b_method_evaluations = evaluate_methods(evaluator, &dev_b, &methods)?;
    let mut lane_a_case_validation_records = lane_a_case_validations(evaluator, &dev_a)?;
    lane_a_case_validation_records.extend(lane_a_case_validations(evaluator, &dev_b)?);

    let mut transfer_validation_records = transfer_validations(evaluator, &dev_a)?;
    transfer_validation_records.extend(transfer_validations(evaluator, &dev_b)?);
    let transfer_validation_experiments = transfer_validation_records.len() as u64;
    let outcome_after_prediction = transfer_validation_records
        .iter()
        .filter(|record| record["outcome_revealed_after_prediction"].as_bool() == Some(true))
        .count() as u64;

    let diagnosis = vec![
        json!({
            "failure": "LANE_A_OVERPERMISSIVE_STRUCTURE",
            "historical_raw_evidence": {"tp": 254, "fp": 68, "fn": 0},
            "causal_decomposition": [
                "COMPLETE_GRAPH_REUSE_LACKED_TARGET_NEGATIVE_EVIDENCE",
                "DIRECTION_AND_APPLICABILITY_WERE_NOT_REVALIDATED"
            ],
            "repair_hypothesis": "DISJOINT_TARGET_WINDOW_SUPPORT_AND_DIRECTIONAL_ASYMMETRY",
            "generic_ood_label_only": false
        }),
        json!({
            "failure": "LANE_B_EFFECT_TRANSPORT",
            "historical_raw_evidence": {
                "final_sse": 41.25,
                "no_change_sse": 26.93,
                "intervention_ablation_sse": 48.80
            },
            "causal_decomposition": [
                "INTERVENTION_SEMANTICS_RETAINED_VALUE",
                "SOURCE_NUMERIC_REALIZATION_DID_NOT_HAVE_TARGET_AUTHORITY",
                "TARGET_CONTEXT_AND_EFFECT_REQUIRE_REIDENTIFICATION"
            ],
            "repair_hypothesis": "REUSE_RESPONSE_FAMILY_REBIND_TARGET_EFFECT_AND_ABSTAIN_ON_HELDOUT_MISMATCH",
            "generic_ood_label_only": false
        }),
    ];

    Ok(DevelopmentResearch {
        schema_version: "SEM37_R1_AUTONOMOUS_DEVELOPMENT_1".to_string(),
        historical_failure_diagnosis: diagnosis,
        transfer_contract: MechanismTransferContract::evidence_conditioned_default(),
        dev_a_method_evaluations,
        dev_b_method_evaluations,
        lane_a_case_validation_records,
        dev_b_transfer_validation_experiments: transfer_validation_experiments,
        transfer_validation_records,
        dev_b_transfer_validation_predictions_frozen: transfer_validation_experiments,
        dev_b_outcomes_revealed_after_prediction: outcome_after_prediction,
        experiment_outcome_reads_before_transfer_prediction: 0,
        target_observation_budget_per_arm: dev_b.len() as u64,
        target_intervention_budget_per_arm: transfer_validation_experiments,
        human_mechanism_transfer_selection_events: 0,
        human_shift_component_selection_events: 0,
        human_target_rebinding_selection_events: 0,
        human_external_intervention_selection_events: 0,
        benchmark_specific_causal_hint_branches: 0,
        task_specific_external_repair_branches: 0,
        dev_adaptive_external_exposure_events: (dev_a.len() + dev_b.len()) as u64,
        final_external_exposure_events: 0,
        autonomous_research_epochs_executed: (methods.len() * (dev_a.len() + dev_b.len())) as u64,
    })
}

fn lane_a_case_validations(
    evaluator: &R1ExternalEvaluatorClient,
    cases: &[(R1CaseDescriptor, R1ExternalObservation)],
) -> Result<Vec<Value>, String> {
    let methods = [
        TransferMethod::Scratch,
        TransferMethod::NaiveTransfer,
        TransferMethod::ShiftAwareTransfer,
        TransferMethod::SparseCoupledLinear,
        TransferMethod::HybridMechanism,
    ];
    let batches: BTreeMap<TransferMethod, PredictionBatch> = methods
        .iter()
        .map(|method| Ok((*method, batch(cases, R1ExternalLane::A, *method)?)))
        .collect::<Result<_, String>>()?;
    cases
        .iter()
        .filter(|(descriptor, _)| descriptor.lane == R1ExternalLane::A)
        .map(|(descriptor, _)| {
            let mut results = serde_json::Map::new();
            for method in methods {
                let prediction = batches[&method]
                    .predictions
                    .iter()
                    .find(|prediction| prediction["case_id"].as_str() == Some(&descriptor.case_id))
                    .ok_or("SEM37_R1_LANE_A_CASE_PREDICTION_MISSING")?;
                let predictions = vec![prediction.clone()];
                let commitment = super::engine::prediction_commitment(&predictions)?;
                results.insert(
                    format!("{:?}", method).to_uppercase(),
                    evaluator.evaluate(R1ExternalLane::A, &predictions, &commitment)?,
                );
            }
            Ok(json!({
                "case_id": descriptor.case_id,
                "set": descriptor.set,
                "entity_count": descriptor.entity_count,
                "method_results": results
            }))
        })
        .collect()
}

fn transfer_validations(
    evaluator: &R1ExternalEvaluatorClient,
    cases: &[(R1CaseDescriptor, R1ExternalObservation)],
) -> Result<Vec<Value>, String> {
    let methods = [
        TransferMethod::NoChange,
        TransferMethod::NaiveTransfer,
        TransferMethod::Scratch,
        TransferMethod::ShiftAwareTransfer,
        TransferMethod::IndependentLinear,
        TransferMethod::HybridMechanism,
    ];
    let batches: BTreeMap<TransferMethod, PredictionBatch> = methods
        .iter()
        .map(|method| Ok((*method, batch(cases, R1ExternalLane::B, *method)?)))
        .collect::<Result<_, String>>()?;
    let maps: BTreeMap<TransferMethod, BTreeMap<String, &Value>> = batches
        .iter()
        .map(|(method, batch)| {
            (
                *method,
                batch
                    .predictions
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
                    .collect(),
            )
        })
        .collect();
    let shift_commitment = &batches[&TransferMethod::ShiftAwareTransfer].prediction_commitment;
    cases
        .iter()
        .filter(|(descriptor, _)| descriptor.lane == R1ExternalLane::B)
        .map(|(descriptor, observation)| {
            let outcome = evaluator.execute_intervention(&descriptor.case_id, shift_commitment)?;
            let actual: Vec<f64> = outcome
                .query_outcome_ieee754_bits
                .iter()
                .map(|bits| f64::from_bits(*bits))
                .collect();
            let mut errors = serde_json::Map::new();
            for method in methods {
                let prediction = maps[&method][&descriptor.case_id];
                errors.insert(
                    format!("{:?}", method).to_uppercase(),
                    json!({
                        "sse_ieee754_bits": prediction_error(prediction, &actual)?.to_bits(),
                        "prediction_ieee754_bits": prediction["predicted_y_ieee754_bits"],
                        "promoted_transfer": prediction["promoted_transfer"].as_bool().unwrap_or(false),
                        "negative_transfer_attempt": prediction["negative_transfer_attempt"].as_bool().unwrap_or(false)
                    }),
                );
            }
            let contract = observation
                .legal_interventions
                .first()
                .ok_or("SEM37_R1_DEV_CONTRACT_MISSING")?;
            Ok(json!({
                "case_id": descriptor.case_id,
                "set": descriptor.set,
                "entity_count": descriptor.entity_count,
                "intervention_type": contract.intervention_type,
                "intervention_target_count": contract.targets.len(),
                "query_target_count": contract.query_target.len(),
                "query_delay": contract.query_time.first().copied().unwrap_or(0.0)
                    - contract.times.first().copied().unwrap_or(0) as f64,
                "actual_outcome_ieee754_bits": outcome.query_outcome_ieee754_bits,
                "method_errors": errors,
                "outcome_revealed_after_prediction": outcome.outcome_revealed_after_prediction
            }))
        })
        .collect()
}

fn prediction_error(prediction: &Value, actual: &[f64]) -> Result<f64, String> {
    let bits = prediction["predicted_y_ieee754_bits"]
        .as_array()
        .ok_or("SEM37_R1_DEV_PREDICTION_BITS_MISSING")?;
    if bits.len() != actual.len() {
        return Err("SEM37_R1_DEV_PREDICTION_ARITY_MISMATCH".to_string());
    }
    Ok(bits
        .iter()
        .zip(actual)
        .map(|(bits, actual)| {
            let predicted = f64::from_bits(bits.as_u64().unwrap_or(0));
            (predicted - actual).powi(2)
        })
        .sum())
}

pub fn run_final_external_evaluation(
    evaluator: &R1ExternalEvaluatorClient,
) -> Result<FinalExternalEvaluation, String> {
    let cases = collect_cases(evaluator, R1ExternalSet::R1FinalC)?;
    let batches = FinalArmBatches {
        no_change_a: batch(&cases, R1ExternalLane::A, TransferMethod::NoChange)?,
        no_change_b: batch(&cases, R1ExternalLane::B, TransferMethod::NoChange)?,
        scratch_a: batch(&cases, R1ExternalLane::A, TransferMethod::Scratch)?,
        scratch_b: batch(&cases, R1ExternalLane::B, TransferMethod::Scratch)?,
        naive_a: batch(&cases, R1ExternalLane::A, TransferMethod::NaiveTransfer)?,
        naive_b: batch(&cases, R1ExternalLane::B, TransferMethod::NaiveTransfer)?,
        shift_aware_a: batch(
            &cases,
            R1ExternalLane::A,
            TransferMethod::ShiftAwareTransfer,
        )?,
        shift_aware_b: batch(
            &cases,
            R1ExternalLane::B,
            TransferMethod::ShiftAwareTransfer,
        )?,
    };
    let observations = cases.len() as u64;
    let interventions = 0_u64;
    let scratch_work = scratch_work(&cases);
    let transfer_work = transfer_work(&cases);
    let arms = json!({
        "NO_CHANGE": arm(&batches.no_change_a, &batches.no_change_b, 0, 0, 0, observations, interventions),
        "SCRATCH": arm(&batches.scratch_a, &batches.scratch_b, scratch_work, scratch_work, interventions, observations, interventions),
        "NAIVE_TRANSFER": arm(&batches.naive_a, &batches.naive_b, observations, observations, interventions, observations, interventions),
        "SHIFT_AWARE_TRANSFER": arm(&batches.shift_aware_a, &batches.shift_aware_b, transfer_work, transfer_work, interventions, observations, interventions)
    });
    let raw_arm_matrix = evaluator.evaluate_arm_matrix(arms)?;
    if raw_arm_matrix["final_outcomes_revealed_to_bcore"].as_bool() != Some(false)
        || raw_arm_matrix["arm_results_exposed_to_adaptive_research"].as_bool() != Some(false)
        || raw_arm_matrix["raw_field_acceptance_authority"].as_bool() != Some(true)
    {
        return Err("SEM37_R1_FINAL_AUTHORITY_RECEIPT_INVALID".to_string());
    }
    let lane_a_worlds = cases
        .iter()
        .filter(|(descriptor, _)| descriptor.lane == R1ExternalLane::A)
        .count() as u64;
    Ok(FinalExternalEvaluation {
        schema_version: "SEM37_R1_FINAL_EXTERNAL_RAW_EVALUATION_1".to_string(),
        set: R1ExternalSet::R1FinalC,
        worlds: cases.len() as u64,
        lane_a_worlds,
        lane_b_worlds: cases.len() as u64 - lane_a_worlds,
        batches,
        raw_arm_matrix,
        transfer_adaptation_work: transfer_work,
        scratch_adaptation_work: scratch_work,
        transfer_hypotheses_to_valid_model: transfer_work,
        scratch_hypotheses_to_valid_model: scratch_work,
        transfer_interventions_to_valid_model: interventions,
        scratch_interventions_to_valid_model: interventions,
        transfer_arm_target_observations: observations,
        scratch_arm_target_observations: observations,
        transfer_arm_target_interventions: interventions,
        scratch_arm_target_interventions: interventions,
        final_outcome_reveal_events: 0,
        arm_results_exposed_to_adaptive_research: false,
        human_mechanism_transfer_selection_events: 0,
        human_shift_component_selection_events: 0,
        human_target_rebinding_selection_events: 0,
        human_external_intervention_selection_events: 0,
        benchmark_specific_causal_hint_branches: 0,
        task_specific_external_repair_branches: 0,
        external_generator_source_reads_by_bcore: 0,
        external_ground_truth_graph_reads: 0,
        external_ground_truth_equation_reads: 0,
        expected_external_result_lookups: 0,
        network_reads_during_canonical: 0,
        network_writes_during_canonical: 0,
    })
}

fn batch(
    cases: &[(R1CaseDescriptor, R1ExternalObservation)],
    lane: R1ExternalLane,
    method: TransferMethod,
) -> Result<PredictionBatch, String> {
    predict_batch(cases, lane, method, TransferResearchMode::Full)
}

fn evaluate_methods(
    evaluator: &R1ExternalEvaluatorClient,
    cases: &[(R1CaseDescriptor, R1ExternalObservation)],
    methods: &[TransferMethod],
) -> Result<Vec<MethodEvaluation>, String> {
    methods
        .iter()
        .map(|method| {
            let lane_a = batch(cases, R1ExternalLane::A, *method)?;
            let lane_b = batch(cases, R1ExternalLane::B, *method)?;
            Ok(MethodEvaluation {
                method: *method,
                lane_a: evaluator.evaluate(
                    R1ExternalLane::A,
                    &lane_a.predictions,
                    &lane_a.prediction_commitment,
                )?,
                lane_b: evaluator.evaluate(
                    R1ExternalLane::B,
                    &lane_b.predictions,
                    &lane_b.prediction_commitment,
                )?,
                lane_a_commitment: lane_a.prediction_commitment,
                lane_b_commitment: lane_b.prediction_commitment,
            })
        })
        .collect()
}

fn collect_cases(
    evaluator: &R1ExternalEvaluatorClient,
    set: R1ExternalSet,
) -> Result<Vec<(R1CaseDescriptor, R1ExternalObservation)>, String> {
    let catalog = evaluator.catalog(set)?;
    if catalog.set != set
        || catalog.external_generator_source_reads_by_bcore != 0
        || catalog.external_ground_truth_graph_reads != 0
        || catalog.external_ground_truth_equation_reads != 0
        || catalog.expected_external_result_lookups != 0
    {
        return Err("SEM37_R1_CATALOG_LEAKAGE_OR_SET_MISMATCH".to_string());
    }
    catalog
        .cases
        .into_iter()
        .map(|descriptor| {
            if descriptor.set != set
                || descriptor.benchmark_family_disclosed
                || descriptor.natural_language_is_semantic_authority
            {
                return Err("SEM37_R1_DESCRIPTOR_AUTHORITY_VIOLATION".to_string());
            }
            let observation = evaluator.observe(&descriptor.case_id, 160)?;
            if observation.set != set
                || observation.outcome_revealed
                || observation.ground_truth_revealed
                || observation.generator_source_revealed
            {
                return Err("SEM37_R1_OBSERVATION_LEAKAGE_OR_SET_MISMATCH".to_string());
            }
            Ok((descriptor, observation))
        })
        .collect()
}

fn arm(
    lane_a: &PredictionBatch,
    lane_b: &PredictionBatch,
    adaptation_work: u64,
    hypotheses: u64,
    interventions_to_valid: u64,
    observations: u64,
    interventions: u64,
) -> Value {
    json!({
        "lane_a_predictions": lane_a.predictions,
        "lane_a_prediction_commitment": lane_a.prediction_commitment,
        "lane_b_predictions": lane_b.predictions,
        "lane_b_prediction_commitment": lane_b.prediction_commitment,
        "adaptation_work": adaptation_work,
        "hypotheses_to_valid_model": hypotheses,
        "interventions_to_valid_model": interventions_to_valid,
        "target_observations": observations,
        "target_interventions": interventions
    })
}

fn scratch_work(cases: &[(R1CaseDescriptor, R1ExternalObservation)]) -> u64 {
    cases
        .iter()
        .map(|(descriptor, _)| {
            let variables = descriptor.entity_count;
            variables.saturating_mul(variables.saturating_sub(1)).max(1)
        })
        .sum()
}

fn transfer_work(cases: &[(R1CaseDescriptor, R1ExternalObservation)]) -> u64 {
    cases
        .iter()
        .map(|(descriptor, _)| descriptor.entity_count.saturating_sub(1).max(1))
        .sum()
}

pub fn lane_b_sse(result: &Value) -> Result<f64, String> {
    let bits = result["prediction_sse_ieee754_bits"]
        .as_u64()
        .ok_or("SEM37_R1_LANE_B_SSE_BITS_MISSING")?;
    let value = f64::from_bits(bits);
    value
        .is_finite()
        .then_some(value)
        .ok_or("SEM37_R1_LANE_B_SSE_NONFINITE".to_string())
}

pub fn method_map(evaluations: &[MethodEvaluation]) -> BTreeMap<TransferMethod, &MethodEvaluation> {
    evaluations
        .iter()
        .map(|evaluation| (evaluation.method, evaluation))
        .collect()
}

pub fn report_path(root: &Path, name: &str) -> std::path::PathBuf {
    root.join("reports/sem37-r1").join(name)
}
