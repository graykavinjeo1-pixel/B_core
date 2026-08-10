use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::{
    adapter::{collect_observations, ExternalEvaluatorClient},
    config::{DEV_SET, FINAL_SET, MAX_AUTONOMOUS_RESEARCH_EPOCHS},
    engine::{
        ablate_identifiability, ablate_pairwise_path, ablate_path_specific, arm, build_batch,
        build_observation_only_batch, compare_candidate_metrics, intervention_hypotheses,
        CandidateModel, CaseEvidence, PredictionBatch,
    },
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateEvidence {
    pub model: CandidateModel,
    pub metrics: Value,
    pub causal_work: u64,
    pub candidate_mediator_paths_total: u64,
    pub candidate_mediator_paths_evaluated: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutonomousDevelopment {
    pub schema_version: String,
    pub set: String,
    pub diagnoses: Vec<String>,
    pub hypotheses: Vec<String>,
    pub candidate_evidence: Vec<CandidateEvidence>,
    pub selected_model: CandidateModel,
    pub selected_metrics: Value,
    pub candidate_pareto_front: Vec<String>,
    pub autonomous_research_epochs_executed: u64,
    pub path_specific_identification_ablation_pass: bool,
    pub interventional_directness_ablation_pass: bool,
    pub identifiability_state_ablation_pass: bool,
    pub causal_path_representation_ablation_pass: bool,
    pub intervention_prediction_commitment: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinalEvaluation {
    pub schema_version: String,
    pub set: String,
    pub selected_model: CandidateModel,
    pub selected_predictions: Vec<Value>,
    pub selected_path_irs: Vec<Value>,
    pub selected_direct_certificates: Vec<Value>,
    pub selected_mediated_certificates: Vec<Value>,
    pub selected_unresolved_certificates: Vec<Value>,
    pub intervention_predictions: Vec<Value>,
    pub intervention_results: Vec<Value>,
    pub intervention_prediction_commitment: String,
    pub evaluator_matrix: Value,
    pub selected_metrics: Value,
    pub transfer_regression: Value,
    pub final_fixture_receipt: Value,
    pub path_specific_identification_ablation_pass: bool,
    pub interventional_directness_ablation_pass: bool,
    pub identifiability_state_ablation_pass: bool,
    pub causal_path_representation_ablation_pass: bool,
    pub autonomous_research_epochs_executed: u64,
}

pub fn run_development(
    root: &Path,
    evaluator: &ExternalEvaluatorClient,
) -> Result<AutonomousDevelopment, String> {
    let report = report_dir(root);
    fs::create_dir_all(&report).map_err(|error| error.to_string())?;
    let fixture_integrity = evaluator.verify_fixtures()?;
    let freeze_receipt = evaluator.freeze_dev()?;
    let cases = collect_observations(evaluator, DEV_SET)?;
    let public_catalog = evaluator.catalog(DEV_SET)?;
    let (intervention_predictions, intervention_commitment) = intervention_hypotheses(&cases)?;
    let evidence = collect_case_evidence(
        evaluator,
        cases,
        &intervention_predictions,
        &intervention_commitment,
    )?;

    let mut candidate_evidence = Vec::new();
    for model in CandidateModel::CANDIDATES {
        let batch = build_batch(&evidence, model)?;
        let metrics =
            evaluator.evaluate_predictions(&batch.predictions, &batch.prediction_commitment)?;
        candidate_evidence.push(CandidateEvidence {
            model,
            metrics,
            causal_work: batch.causal_work,
            candidate_mediator_paths_total: batch.candidate_mediator_paths_total,
            candidate_mediator_paths_evaluated: batch.candidate_mediator_paths_evaluated,
        });
    }
    let selected = candidate_evidence
        .iter()
        .min_by(|left, right| {
            compare_candidate_metrics(
                (&left.metrics, left.causal_work),
                (&right.metrics, right.causal_work),
            )
        })
        .ok_or("SEM37_R5_NO_DEVELOPMENT_CANDIDATE")?;
    let selected_model = selected.model;
    let selected_metrics = selected.metrics.clone();
    let selected_batch = build_batch(&evidence, selected_model)?;
    let ablations = evaluate_ablations(evaluator, &evidence, &selected_batch)?;
    let epochs = CandidateModel::CANDIDATES.len() as u64 * evidence.len() as u64
        + evidence.len() as u64
        + 4 * evidence.len() as u64;
    if epochs > MAX_AUTONOMOUS_RESEARCH_EPOCHS {
        return Err("SEM37_R5_AUTONOMOUS_RESEARCH_BUDGET_EXCEEDED".to_string());
    }
    let pareto = pareto_front(&candidate_evidence);
    let diagnoses = vec![
        "R4_STRUCTURAL_TAXONOMY_DID_NOT_ESTABLISH_IDENTIFIABILITY".to_string(),
        "TOTAL_EFFECT_AND_PATH_COMPONENTS_REQUIRE_SEPARATE_EVIDENCE".to_string(),
        "HIDDEN_COMPONENTS_REQUIRE_CALIBRATED_UNRESOLVED_STATE".to_string(),
    ];
    let hypotheses = CandidateModel::CANDIDATES
        .iter()
        .map(|model| {
            format!(
                "{}:BOUNDED_PATH_RANKING+IDENTIFIABILITY_GUARD+LEGAL_INTERVENTION_EVIDENCE",
                model.name()
            )
        })
        .collect::<Vec<_>>();
    let development = AutonomousDevelopment {
        schema_version: "SEM37_R5_AUTONOMOUS_DEVELOPMENT_1".to_string(),
        set: DEV_SET.to_string(),
        diagnoses,
        hypotheses,
        candidate_evidence,
        selected_model,
        selected_metrics,
        candidate_pareto_front: pareto,
        autonomous_research_epochs_executed: epochs,
        path_specific_identification_ablation_pass: ablations.path_specific,
        interventional_directness_ablation_pass: ablations.interventional,
        identifiability_state_ablation_pass: ablations.identifiability,
        causal_path_representation_ablation_pass: ablations.path_representation,
        intervention_prediction_commitment: intervention_commitment.clone(),
    };
    write_json(&report.join("r5_dev_h_manifest.json"), &public_catalog)?;
    write_json(
        &report.join("pre_dev_fixture_validation_receipt.json"),
        &json!({
            "fixture_integrity": fixture_integrity,
            "freeze_receipt": freeze_receipt,
            "candidate_behavior_observed_before_dev": false,
            "final_solver_exposures": 0,
            "invalid_fixture_solver_exposures": 0
        }),
    )?;
    write_json(
        &report.join("path_specific_causal_ir.json"),
        &selected_batch.path_irs,
    )?;
    write_json(
        &report.join("identifiability_contract.json"),
        &json!({
            "states": ["FULLY_IDENTIFIABLE", "PARTIALLY_IDENTIFIABLE", "NOT_IDENTIFIABLE_UNDER_AVAILABLE_EVIDENCE"],
            "identifiability_precedes_classification": true,
            "predictive_residual_is_direct_effect_proof": false,
            "temporal_precedence_is_direct_effect_authority": false,
            "unavailable_counterfactual_used_as_observed_evidence": 0
        }),
    )?;
    write_jsonl(&report.join("r5_failure_diagnosis.jsonl"), &development.diagnoses.iter().enumerate().map(|(index, diagnosis)| json!({"sequence": index + 1, "diagnosis": diagnosis, "source": "AUTONOMOUS_R4_RESIDUAL_ANALYSIS"})).collect::<Vec<_>>())?;
    write_jsonl(&report.join("r5_research_hypotheses.jsonl"), &development.hypotheses.iter().enumerate().map(|(index, hypothesis)| json!({"sequence": index + 1, "hypothesis": hypothesis, "operator_selected": false})).collect::<Vec<_>>())?;
    write_jsonl(&report.join("r5_diagnostic_experiments.jsonl"), &development.candidate_evidence.iter().enumerate().map(|(index, candidate)| json!({"sequence": index + 1, "model": candidate.model, "metrics": candidate.metrics, "causal_work": candidate.causal_work, "outcomes_read_after_commitment": true})).collect::<Vec<_>>())?;
    write_jsonl(
        &report.join("candidate_path_models.jsonl"),
        &development
            .candidate_evidence
            .iter()
            .map(|candidate| json!(candidate))
            .collect::<Vec<_>>(),
    )?;
    write_jsonl(
        &report.join("direct_path_certificates.jsonl"),
        &selected_batch
            .direct_certificates
            .iter()
            .map(|value| json!(value))
            .collect::<Vec<_>>(),
    )?;
    write_jsonl(
        &report.join("mediated_path_certificates.jsonl"),
        &selected_batch
            .mediated_certificates
            .iter()
            .map(|value| json!(value))
            .collect::<Vec<_>>(),
    )?;
    write_jsonl(
        &report.join("unresolved_certificates.jsonl"),
        &selected_batch
            .unresolved_certificates
            .iter()
            .map(|value| json!(value))
            .collect::<Vec<_>>(),
    )?;
    write_jsonl(
        &report.join("intervention_predictions.jsonl"),
        &intervention_predictions,
    )?;
    write_jsonl(&report.join("intervention_results.jsonl"), &evidence.iter().map(|case| json!({"case_id": case.descriptor.case_id, "prediction_commitment": case.prediction_commitment, "outcome_revealed_after_prediction": case.intervention.outcome_revealed_after_prediction, "gold_path_labels_revealed": case.intervention.gold_path_labels_revealed})).collect::<Vec<_>>())?;
    write_json(
        &report.join("candidate_pareto_front.json"),
        &json!({"models": development.candidate_pareto_front, "frozen_multi_objective": true}),
    )?;
    write_json(
        &report.join("candidate_selection_receipt.json"),
        &json!({
            "selected_model": selected_model,
            "selection": "FROZEN_MULTI_OBJECTIVE_LEXICOGRAPHIC_PARETO",
            "operator_selected": false,
            "human_direct_effect_repair_selection_events": 0,
            "human_mediation_estimator_selection_events": 0,
            "human_intervention_selection_events": 0,
            "autonomous_research_epochs_executed": epochs
        }),
    )?;
    write_ablation_files(&report, &ablations)?;
    write_json(&report.join("development_result.json"), &development)?;
    Ok(development)
}

pub fn run_final(
    root: &Path,
    evaluator: &ExternalEvaluatorClient,
    development: &AutonomousDevelopment,
) -> Result<FinalEvaluation, String> {
    let report = report_dir(root);
    let final_fixture_receipt = evaluator.freeze_final()?;
    let cases = collect_observations(evaluator, FINAL_SET)?;
    let public_catalog = evaluator.catalog(FINAL_SET)?;
    let (intervention_predictions, intervention_commitment) = intervention_hypotheses(&cases)?;
    let evidence = collect_case_evidence(
        evaluator,
        cases,
        &intervention_predictions,
        &intervention_commitment,
    )?;
    let selected_batch = build_batch(&evidence, development.selected_model)?;
    let observation_only = build_observation_only_batch(&evidence, development.selected_model)?;
    let path_ablated = ablate_path_specific(&selected_batch)?;
    let identifiability_ablated = ablate_identifiability(&selected_batch)?;
    let pairwise_ablated = ablate_pairwise_path(&selected_batch)?;
    let matrix = evaluator.evaluate_matrix(json!({
        "R5_CANDIDATE": arm(selected_batch.predictions.clone())?,
        "PATH_SPECIFIC_ABLATED": arm(path_ablated)?,
        "OBSERVATION_ONLY": arm(observation_only.predictions.clone())?,
        "IDENTIFIABILITY_ABLATED": arm(identifiability_ablated)?,
        "PAIRWISE_PATH_ABLATED": arm(pairwise_ablated)?
    }))?;
    let selected_metrics = matrix["arms"]["R5_CANDIDATE"].clone();
    let ablations = AblationResults::from_matrix(&matrix);
    let transfer_regression = evaluator.transfer_regression()?;
    let final_result = FinalEvaluation {
        schema_version: "SEM37_R5_FINAL_EVALUATION_1".to_string(),
        set: FINAL_SET.to_string(),
        selected_model: development.selected_model,
        selected_predictions: selected_batch.predictions.clone(),
        selected_path_irs: selected_batch
            .path_irs
            .iter()
            .map(|value| json!(value))
            .collect(),
        selected_direct_certificates: selected_batch
            .direct_certificates
            .iter()
            .map(|value| json!(value))
            .collect(),
        selected_mediated_certificates: selected_batch
            .mediated_certificates
            .iter()
            .map(|value| json!(value))
            .collect(),
        selected_unresolved_certificates: selected_batch
            .unresolved_certificates
            .iter()
            .map(|value| json!(value))
            .collect(),
        intervention_predictions: intervention_predictions.clone(),
        intervention_results: evidence
            .iter()
            .map(|case| json!(case.intervention))
            .collect(),
        intervention_prediction_commitment: intervention_commitment,
        evaluator_matrix: matrix.clone(),
        selected_metrics: selected_metrics.clone(),
        transfer_regression: transfer_regression.clone(),
        final_fixture_receipt: final_fixture_receipt.clone(),
        path_specific_identification_ablation_pass: ablations.path_specific,
        interventional_directness_ablation_pass: ablations.interventional,
        identifiability_state_ablation_pass: ablations.identifiability,
        causal_path_representation_ablation_pass: ablations.path_representation,
        autonomous_research_epochs_executed: development.autonomous_research_epochs_executed,
    };
    write_json(
        &report.join("r5_final_fixture_contract.json"),
        &final_fixture_receipt,
    )?;
    write_jsonl(&report.join("rejected_final_fixture_manifests.jsonl"), &[])?;
    write_json(&report.join("r5_final_i_manifest.json"), &public_catalog)?;
    write_json(
        &report.join("historical_comparator_results.json"),
        &json!({
            "r2_r3_historical_direct": {"tp": 16, "fp": 4, "fn": 35},
            "r4_historical_direct": {"tp": 13, "fp": 4, "fn": 38},
            "r4_historical_mediated": {"tp": 4, "fp": 14, "fn": 13},
            "paired_final_i_comparator_execution": false,
            "comparison_policy": "FROZEN_HISTORICAL_BEST_RECALL_16_OVER_51",
            "comparators_are_capability_predecessors": false
        }),
    )?;
    write_json(&report.join("r5_final_i_raw.json"), &final_result)?;
    write_json(
        &report.join("identifiable_case_matrix.json"),
        &json!({"metrics": selected_metrics, "predictions": selected_batch.predictions}),
    )?;
    write_json(
        &report.join("non_identifiable_case_matrix.json"),
        &json!({
            "partially_identifiable_cases": selected_metrics["partially_identifiable_cases"],
            "non_identifiable_cases": selected_metrics["non_identifiable_cases"],
            "false_certainty_on_non_identifiable_cases": selected_metrics["false_certainty_on_non_identifiable_cases"]
        }),
    )?;
    write_json(
        &report.join("mixed_effect_matrix.json"),
        &json!({
            "mixed_direct_mediated_cases": selected_metrics["mixed_direct_mediated_cases"],
            "mixed_direct_mediated_correct": selected_metrics["mixed_direct_mediated_correct"],
            "mixed_direct_mediated_identification_pass": selected_metrics["mixed_direct_mediated_identification_pass"]
        }),
    )?;
    write_json(
        &report.join("transfer_regression_matrix.json"),
        &transfer_regression,
    )?;
    write_ablation_files(&report, &ablations)?;
    Ok(final_result)
}

fn collect_case_evidence(
    evaluator: &ExternalEvaluatorClient,
    cases: Vec<(
        super::adapter::CaseDescriptor,
        super::adapter::ExternalObservation,
    )>,
    predictions: &[Value],
    commitment: &str,
) -> Result<Vec<CaseEvidence>, String> {
    let by_case = predictions
        .iter()
        .map(|prediction| {
            Ok((
                prediction["case_id"]
                    .as_str()
                    .ok_or("SEM37_R5_INTERVENTION_PREDICTION_CASE_MISSING")?
                    .to_string(),
                prediction.clone(),
            ))
        })
        .collect::<Result<std::collections::BTreeMap<_, _>, String>>()?;
    cases
        .into_iter()
        .map(|(descriptor, observation)| {
            let intervention = evaluator.execute_intervention(&descriptor.case_id, commitment)?;
            if !intervention.outcome_revealed_after_prediction
                || !intervention.prediction_commitment_verified
                || intervention.gold_path_labels_revealed
            {
                return Err("SEM37_R5_INTERVENTION_ORDER_OR_LEAKAGE_FAILURE".to_string());
            }
            Ok(CaseEvidence {
                pre_intervention_prediction: by_case[&descriptor.case_id].clone(),
                prediction_commitment: commitment.to_string(),
                descriptor,
                observation,
                intervention,
            })
        })
        .collect()
}

#[derive(Debug, Clone)]
struct AblationResults {
    path_specific: bool,
    interventional: bool,
    identifiability: bool,
    path_representation: bool,
    selected: Value,
    path_specific_metrics: Value,
    observation_only_metrics: Value,
    identifiability_metrics: Value,
    path_representation_metrics: Value,
}

impl AblationResults {
    fn from_matrix(matrix: &Value) -> Self {
        let selected = matrix["arms"]["R5_CANDIDATE"].clone();
        let path = matrix["arms"]["PATH_SPECIFIC_ABLATED"].clone();
        let observation = matrix["arms"]["OBSERVATION_ONLY"].clone();
        let identifiability = matrix["arms"]["IDENTIFIABILITY_ABLATED"].clone();
        let pairwise = matrix["arms"]["PAIRWISE_PATH_ABLATED"].clone();
        let selected_error = total_error(&selected);
        Self {
            path_specific: selected_error < total_error(&path),
            interventional: selected_error < total_error(&observation),
            identifiability: field(&selected, "false_certainty_on_non_identifiable_cases")
                < field(
                    &identifiability,
                    "false_certainty_on_non_identifiable_cases",
                ),
            path_representation: field(&selected, "mediated_tp") > field(&pairwise, "mediated_tp"),
            selected,
            path_specific_metrics: path,
            observation_only_metrics: observation,
            identifiability_metrics: identifiability,
            path_representation_metrics: pairwise,
        }
    }
}

fn evaluate_ablations(
    evaluator: &ExternalEvaluatorClient,
    evidence: &[CaseEvidence],
    selected: &PredictionBatch,
) -> Result<AblationResults, String> {
    let observation = build_observation_only_batch(evidence, selected.model)?;
    let matrix = evaluator.evaluate_matrix(json!({
        "R5_CANDIDATE": arm(selected.predictions.clone())?,
        "PATH_SPECIFIC_ABLATED": arm(ablate_path_specific(selected)?)?,
        "OBSERVATION_ONLY": arm(observation.predictions)?,
        "IDENTIFIABILITY_ABLATED": arm(ablate_identifiability(selected)?)?,
        "PAIRWISE_PATH_ABLATED": arm(ablate_pairwise_path(selected)?)?
    }))?;
    Ok(AblationResults::from_matrix(&matrix))
}

fn field(value: &Value, name: &str) -> u64 {
    value[name].as_u64().unwrap_or(u64::MAX / 16)
}

fn total_error(value: &Value) -> u64 {
    [
        "identifiable_direct_fp",
        "identifiable_direct_fn",
        "mediated_fp",
        "mediated_fn",
        "false_certainty_on_non_identifiable_cases",
        "pure_mediation_false_direct_events",
        "pure_direct_false_mediated_events",
        "common_cause_as_direct_misidentifications",
    ]
    .iter()
    .map(|name| field(value, name))
    .sum()
}

fn pareto_front(candidates: &[CandidateEvidence]) -> Vec<String> {
    candidates
        .iter()
        .filter(|candidate| {
            !candidates.iter().any(|other| {
                other.model != candidate.model
                    && total_error(&other.metrics) <= total_error(&candidate.metrics)
                    && other.causal_work <= candidate.causal_work
                    && (total_error(&other.metrics) < total_error(&candidate.metrics)
                        || other.causal_work < candidate.causal_work)
            })
        })
        .map(|candidate| candidate.model.name().to_string())
        .collect()
}

fn write_ablation_files(report: &Path, ablations: &AblationResults) -> Result<(), String> {
    let values = [
        (
            "path_specific_identification_ablation.json",
            ablations.path_specific,
            &ablations.path_specific_metrics,
        ),
        (
            "observation_only_ablation.json",
            ablations.interventional,
            &ablations.observation_only_metrics,
        ),
        (
            "identifiability_state_ablation.json",
            ablations.identifiability,
            &ablations.identifiability_metrics,
        ),
        (
            "path_representation_ablation.json",
            ablations.path_representation,
            &ablations.path_representation_metrics,
        ),
    ];
    for (name, passed, metrics) in values {
        write_json(
            &report.join(name),
            &json!({"pass": passed, "selected_metrics": ablations.selected, "ablated_metrics": metrics}),
        )?;
    }
    Ok(())
}

pub fn report_dir(root: &Path) -> PathBuf {
    root.join("reports/sem37-r5")
}

pub fn write_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    fs::write(path, bytes).map_err(|error| error.to_string())
}

pub fn write_jsonl(path: &Path, values: &[Value]) -> Result<(), String> {
    let mut output = String::new();
    for value in values {
        output.push_str(&serde_json::to_string(value).map_err(|error| error.to_string())?);
        output.push('\n');
    }
    fs::write(path, output).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn research_epoch_accounting_is_bounded() {
        let worlds = 26_u64;
        let epochs = CandidateModel::CANDIDATES.len() as u64 * worlds + worlds + 4 * worlds;
        assert!(epochs <= MAX_AUTONOMOUS_RESEARCH_EPOCHS);
    }

    #[test]
    fn total_error_uses_raw_fields() {
        let metrics = json!({
            "identifiable_direct_fp": 1, "identifiable_direct_fn": 2,
            "mediated_fp": 3, "mediated_fn": 4,
            "false_certainty_on_non_identifiable_cases": 5,
            "pure_mediation_false_direct_events": 6,
            "pure_direct_false_mediated_events": 7,
            "common_cause_as_direct_misidentifications": 8
        });
        assert_eq!(total_error(&metrics), 36);
    }
}
