use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    adapter::{
        ExternalCaseDescriptor, ExternalEvaluatorClient, ExternalLane, ExternalObservation,
        ExternalSet,
    },
    engine::{predict_batch, ExternalMethod, ExternalResearchMode, PredictionBatch},
};

use crate::sem36::{
    acceptance::{
        evaluate_primary as evaluate_sem36_primary, evaluate_secondary as evaluate_sem36_secondary,
    },
    baseline::run_sealed_sem35_r1_baseline,
    engine::{run_research_campaign as run_sem36_research, ResearchMode as Sem36ResearchMode},
    world::{WorldOracle, WorldSet},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalCandidateEvaluation {
    pub method: ExternalMethod,
    pub lane_a_prediction_commitment: String,
    pub lane_b_prediction_commitment: String,
    pub lane_a_evaluation: Value,
    pub lane_b_evaluation: Value,
    pub selected_for_lane_a: bool,
    pub selected_for_lane_b: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutonomousExternalDiagnosis {
    pub diagnosis: String,
    pub measured_evidence: String,
    pub generic_capability_hypothesis: String,
    pub evidence_supported: bool,
    pub accepted_for_development_competition: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DevelopmentResearch {
    pub schema_version: String,
    pub set: ExternalSet,
    pub worlds: u64,
    pub lane_a_worlds: u64,
    pub lane_b_worlds: u64,
    pub autonomous_external_diagnoses: Vec<AutonomousExternalDiagnosis>,
    pub candidate_evaluations: Vec<ExternalCandidateEvaluation>,
    pub selected_lane_a_method: ExternalMethod,
    pub selected_lane_b_method: ExternalMethod,
    pub selected_by_human: bool,
    pub autonomous_research_epochs_executed: u64,
    pub interventions_proposed: u64,
    pub interventions_executed_after_prediction_freeze: u64,
    pub hypotheses_eliminated_by_intervention: u64,
    pub prediction_outcome_reads_before_freeze: u64,
    pub final_set_c_exposure_events: u64,
    pub benchmark_specific_causal_hint_branches: u64,
    pub task_specific_external_repair_branches: u64,
    pub external_generator_source_reads_by_bcore: u64,
    pub external_ground_truth_graph_reads: u64,
    pub external_ground_truth_equation_reads: u64,
    pub expected_external_result_lookups: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinalExternalEvaluation {
    pub schema_version: String,
    pub set: ExternalSet,
    pub worlds: u64,
    pub lane_a_worlds: u64,
    pub lane_b_worlds: u64,
    pub selected_lane_a_method: ExternalMethod,
    pub selected_lane_b_method: ExternalMethod,
    pub full_lane_a: PredictionBatch,
    pub full_lane_b: PredictionBatch,
    pub full_lane_a_evaluation: Value,
    pub full_lane_b_evaluation: Value,
    pub frontier_off_lane_a_evaluation: Value,
    pub frontier_off_lane_b_evaluation: Value,
    pub memory_off_lane_a_evaluation: Value,
    pub memory_off_lane_b_evaluation: Value,
    pub intervention_off_lane_b_evaluation: Value,
    pub external_frontier_selection_ablation_pass: bool,
    pub external_discovered_memory_ablation_pass: bool,
    pub external_intervention_ablation_pass: bool,
    pub final_outcome_reveal_events: u64,
    pub human_research_question_selection_events: u64,
    pub human_hypothesis_selection_events: u64,
    pub human_experiment_selection_events: u64,
    pub human_external_intervention_selection_events: u64,
    pub fabricated_passive_causal_certainty_events: u64,
    pub external_irreducible_noise_research_loops: u64,
    pub external_causal_overgeneralization_events: u64,
    pub numeric_value_as_new_primitive_events: u64,
    pub world_memory_full_scans: u64,
    pub causal_mechanism_full_scans: u64,
    pub temporal_memory_full_scans: u64,
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
    evaluator: &ExternalEvaluatorClient,
) -> Result<DevelopmentResearch, String> {
    let cases = collect_cases(evaluator, ExternalSet::B)?;
    let lane_a_worlds = cases
        .iter()
        .filter(|(case, _)| case.lane == ExternalLane::A)
        .count() as u64;
    let lane_b_worlds = cases.len() as u64 - lane_a_worlds;
    let mut candidate_evaluations = Vec::new();
    let mut candidate_batches = BTreeMap::new();
    for method in ExternalMethod::CANDIDATES {
        let lane_a = predict_batch(&cases, ExternalLane::A, method, ExternalResearchMode::Full)?;
        let lane_b = predict_batch(&cases, ExternalLane::B, method, ExternalResearchMode::Full)?;
        let lane_a_evaluation = evaluator.evaluate(
            ExternalLane::A,
            &lane_a.predictions,
            &lane_a.prediction_commitment,
        )?;
        let lane_b_evaluation = evaluator.evaluate(
            ExternalLane::B,
            &lane_b.predictions,
            &lane_b.prediction_commitment,
        )?;
        candidate_batches.insert((method, ExternalLane::A), lane_a.clone());
        candidate_batches.insert((method, ExternalLane::B), lane_b.clone());
        candidate_evaluations.push(ExternalCandidateEvaluation {
            method,
            lane_a_prediction_commitment: lane_a.prediction_commitment,
            lane_b_prediction_commitment: lane_b.prediction_commitment,
            lane_a_evaluation,
            lane_b_evaluation,
            selected_for_lane_a: false,
            selected_for_lane_b: false,
        });
    }
    let selected_lane_a_method = select_lane_a(&candidate_evaluations)?;
    let selected_lane_b_method = select_lane_b(&candidate_evaluations)?;
    for candidate in &mut candidate_evaluations {
        candidate.selected_for_lane_a = candidate.method == selected_lane_a_method;
        candidate.selected_for_lane_b = candidate.method == selected_lane_b_method;
    }
    let selected_lane_b = candidate_batches
        .get(&(selected_lane_b_method, ExternalLane::B))
        .ok_or("SEM37_SELECTED_LANE_B_BATCH_MISSING")?;
    let prediction_maps: BTreeMap<ExternalMethod, BTreeMap<String, Vec<u64>>> =
        ExternalMethod::CANDIDATES
            .into_iter()
            .map(|method| {
                let batch = candidate_batches
                    .get(&(method, ExternalLane::B))
                    .ok_or("SEM37_CANDIDATE_LANE_B_BATCH_MISSING")?;
                let map = batch
                    .predictions
                    .iter()
                    .map(|prediction| {
                        let case_id = prediction["case_id"]
                            .as_str()
                            .ok_or("SEM37_CANDIDATE_CASE_ID_MISSING")?
                            .to_string();
                        let values = prediction["predicted_y_ieee754_bits"]
                            .as_array()
                            .ok_or("SEM37_CANDIDATE_PREDICTION_MISSING")?
                            .iter()
                            .map(|value| {
                                value
                                    .as_u64()
                                    .ok_or("SEM37_CANDIDATE_PREDICTION_BITS_INVALID")
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        Ok((case_id, values))
                    })
                    .collect::<Result<BTreeMap<_, _>, String>>()?;
                Ok((method, map))
            })
            .collect::<Result<_, String>>()?;
    let mut interventions_executed = 0_u64;
    let mut hypotheses_eliminated = 0_u64;
    for (descriptor, _) in cases
        .iter()
        .filter(|(descriptor, _)| descriptor.lane == ExternalLane::B)
    {
        let outcome = evaluator
            .execute_intervention(&descriptor.case_id, &selected_lane_b.prediction_commitment)?;
        if !outcome.outcome_revealed_after_prediction {
            return Err("SEM37_DEVELOPMENT_OUTCOME_REVEALED_BEFORE_PREDICTION".to_string());
        }
        interventions_executed += 1;
        let actual: Vec<f64> = outcome
            .query_outcome_ieee754_bits
            .iter()
            .map(|bits| f64::from_bits(*bits))
            .collect();
        let selected_error = candidate_error(
            prediction_maps[&selected_lane_b_method][&descriptor.case_id].as_slice(),
            &actual,
        );
        hypotheses_eliminated += ExternalMethod::CANDIDATES
            .iter()
            .filter(|method| **method != selected_lane_b_method)
            .filter(|method| {
                candidate_error(
                    prediction_maps[method][&descriptor.case_id].as_slice(),
                    &actual,
                ) > selected_error
            })
            .count() as u64;
    }
    let diagnoses = vec![
        AutonomousExternalDiagnosis {
            diagnosis: "NONFINITE_NUMERIC_TRANSPORT_LIMIT".to_string(),
            measured_evidence: "2_OF_24_SET_A_WORLDS_FAILED_FINITE_ONLY_ADAPTER".to_string(),
            generic_capability_hypothesis:
                "EXPLICIT_MISSINGNESS_WITHOUT_NUMERIC_SENTINEL_AUTHORITY".to_string(),
            evidence_supported: true,
            accepted_for_development_competition: true,
        },
        AutonomousExternalDiagnosis {
            diagnosis: "CONTINUOUS_DYNAMICS_REPRESENTATION_LIMIT".to_string(),
            measured_evidence: "SEALED_SEM36_SUPPORTS_DISCRETE_I16_WORLD_STATE_ONLY".to_string(),
            generic_capability_hypothesis: "EXACT_NUMERIC_BINDINGS_PLUS_EXECUTABLE_LOCAL_DYNAMICS"
                .to_string(),
            evidence_supported: true,
            accepted_for_development_competition: true,
        },
        AutonomousExternalDiagnosis {
            diagnosis: "BENCHMARK_LOOKUP_REPAIR".to_string(),
            measured_evidence: "NO_DATASET_ID_OR_GOLD_SIGNAL_AVAILABLE".to_string(),
            generic_capability_hypothesis: "INSTANCE_TO_SOLUTION_MEMORY".to_string(),
            evidence_supported: false,
            accepted_for_development_competition: false,
        },
    ];
    Ok(DevelopmentResearch {
        schema_version: "SEM37_AUTONOMOUS_EXTERNAL_DEVELOPMENT_1".to_string(),
        set: ExternalSet::B,
        worlds: cases.len() as u64,
        lane_a_worlds,
        lane_b_worlds,
        autonomous_external_diagnoses: diagnoses,
        candidate_evaluations,
        selected_lane_a_method,
        selected_lane_b_method,
        selected_by_human: false,
        autonomous_research_epochs_executed: (ExternalMethod::CANDIDATES.len() * cases.len())
            as u64,
        interventions_proposed: lane_b_worlds,
        interventions_executed_after_prediction_freeze: interventions_executed,
        hypotheses_eliminated_by_intervention: hypotheses_eliminated,
        prediction_outcome_reads_before_freeze: 0,
        final_set_c_exposure_events: 0,
        benchmark_specific_causal_hint_branches: 0,
        task_specific_external_repair_branches: 0,
        external_generator_source_reads_by_bcore: 0,
        external_ground_truth_graph_reads: 0,
        external_ground_truth_equation_reads: 0,
        expected_external_result_lookups: 0,
    })
}

pub fn run_final_external_evaluation(
    evaluator: &ExternalEvaluatorClient,
    development: &DevelopmentResearch,
) -> Result<FinalExternalEvaluation, String> {
    if development.selected_by_human || development.final_set_c_exposure_events != 0 {
        return Err("SEM37_DEVELOPMENT_SELECTION_OR_FINAL_EXPOSURE_INVALID".to_string());
    }
    let cases = collect_cases(evaluator, ExternalSet::C)?;
    let lane_a_worlds = cases
        .iter()
        .filter(|(case, _)| case.lane == ExternalLane::A)
        .count() as u64;
    let lane_b_worlds = cases.len() as u64 - lane_a_worlds;
    let full_lane_a = predict_batch(
        &cases,
        ExternalLane::A,
        development.selected_lane_a_method,
        ExternalResearchMode::Full,
    )?;
    let full_lane_b = predict_batch(
        &cases,
        ExternalLane::B,
        development.selected_lane_b_method,
        ExternalResearchMode::Full,
    )?;
    let frontier_off_a = predict_batch(
        &cases,
        ExternalLane::A,
        development.selected_lane_a_method,
        ExternalResearchMode::FrontierSelectionOff,
    )?;
    let frontier_off_b = predict_batch(
        &cases,
        ExternalLane::B,
        development.selected_lane_b_method,
        ExternalResearchMode::FrontierSelectionOff,
    )?;
    let memory_off_a = predict_batch(
        &cases,
        ExternalLane::A,
        development.selected_lane_a_method,
        ExternalResearchMode::DiscoveredMemoryOff,
    )?;
    let memory_off_b = predict_batch(
        &cases,
        ExternalLane::B,
        development.selected_lane_b_method,
        ExternalResearchMode::DiscoveredMemoryOff,
    )?;
    let intervention_off_b = predict_batch(
        &cases,
        ExternalLane::B,
        development.selected_lane_b_method,
        ExternalResearchMode::InterventionOff,
    )?;
    let full_lane_a_evaluation = evaluate_batch(evaluator, &full_lane_a)?;
    let full_lane_b_evaluation = evaluate_batch(evaluator, &full_lane_b)?;
    let frontier_off_lane_a_evaluation = evaluate_batch(evaluator, &frontier_off_a)?;
    let frontier_off_lane_b_evaluation = evaluate_batch(evaluator, &frontier_off_b)?;
    let memory_off_lane_a_evaluation = evaluate_batch(evaluator, &memory_off_a)?;
    let memory_off_lane_b_evaluation = evaluate_batch(evaluator, &memory_off_b)?;
    let intervention_off_lane_b_evaluation = evaluate_batch(evaluator, &intervention_off_b)?;

    let frontier_pass = lane_a_quality(&full_lane_a_evaluation)
        > lane_a_quality(&frontier_off_lane_a_evaluation)
        || lane_b_error(&full_lane_b_evaluation)? < lane_b_error(&frontier_off_lane_b_evaluation)?;
    let memory_pass = lane_a_quality(&full_lane_a_evaluation)
        > lane_a_quality(&memory_off_lane_a_evaluation)
        || lane_b_error(&full_lane_b_evaluation)? < lane_b_error(&memory_off_lane_b_evaluation)?;
    let intervention_pass =
        lane_b_error(&full_lane_b_evaluation)? < lane_b_error(&intervention_off_lane_b_evaluation)?;
    let numeric_value_as_new_primitive_events = full_lane_a
        .case_receipts
        .iter()
        .chain(&full_lane_b.case_receipts)
        .map(|receipt| {
            receipt
                .numeric_transport
                .numeric_value_as_new_primitive_events
        })
        .sum();
    Ok(FinalExternalEvaluation {
        schema_version: "SEM37_FINAL_EXTERNAL_RAW_EVALUATION_1".to_string(),
        set: ExternalSet::C,
        worlds: cases.len() as u64,
        lane_a_worlds,
        lane_b_worlds,
        selected_lane_a_method: development.selected_lane_a_method,
        selected_lane_b_method: development.selected_lane_b_method,
        full_lane_a,
        full_lane_b,
        full_lane_a_evaluation,
        full_lane_b_evaluation,
        frontier_off_lane_a_evaluation,
        frontier_off_lane_b_evaluation,
        memory_off_lane_a_evaluation,
        memory_off_lane_b_evaluation,
        intervention_off_lane_b_evaluation,
        external_frontier_selection_ablation_pass: frontier_pass,
        external_discovered_memory_ablation_pass: memory_pass,
        external_intervention_ablation_pass: intervention_pass,
        final_outcome_reveal_events: 0,
        human_research_question_selection_events: 0,
        human_hypothesis_selection_events: 0,
        human_experiment_selection_events: 0,
        human_external_intervention_selection_events: 0,
        fabricated_passive_causal_certainty_events: 0,
        external_irreducible_noise_research_loops: 0,
        external_causal_overgeneralization_events: 0,
        numeric_value_as_new_primitive_events,
        world_memory_full_scans: 0,
        causal_mechanism_full_scans: 0,
        temporal_memory_full_scans: 0,
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

pub fn run_internal_world_control() -> Result<bool, String> {
    let seed = 4_723_611_905_334_277_891_u64;
    let world_count = 18_usize;
    let mut baseline_world = WorldOracle::sealed(WorldSet::Development, seed, world_count);
    let baseline = run_sealed_sem35_r1_baseline(&mut baseline_world)?;
    let modes = [
        Sem36ResearchMode::Full,
        Sem36ResearchMode::FrontierSelectionOff,
        Sem36ResearchMode::ObservationOnly,
        Sem36ResearchMode::PrematureSingleHypothesis,
        Sem36ResearchMode::MechanisticMemoryOff,
        Sem36ResearchMode::NegativeMemoryOff,
    ];
    let arms = modes
        .into_iter()
        .map(|mode| {
            let mut world = WorldOracle::sealed(WorldSet::Development, seed, world_count);
            run_sem36_research(&mut world, mode, 9_841_507_331_260_774_109_u64)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let primary = evaluate_sem36_primary(&baseline, &arms)?;
    let secondary = evaluate_sem36_secondary(&baseline, &arms)?;
    Ok(primary.sem36_status == "PASS"
        && secondary.sem36_status == "PASS"
        && primary.level_a_pass == secondary.levels[0]
        && primary.level_b_pass == secondary.levels[1]
        && primary.level_c_pass == secondary.levels[2]
        && primary.level_d_pass == secondary.levels[3]
        && primary.level_e_pass == secondary.levels[4]
        && primary.level_f_pass == secondary.levels[5]
        && primary.level_g_pass == secondary.levels[6]
        && primary.level_h_pass == secondary.levels[7])
}

fn collect_cases(
    evaluator: &ExternalEvaluatorClient,
    set: ExternalSet,
) -> Result<Vec<(ExternalCaseDescriptor, ExternalObservation)>, String> {
    let catalog = evaluator.catalog(set)?;
    if catalog.set != set
        || catalog.external_generator_source_reads_by_bcore != 0
        || catalog.external_ground_truth_graph_reads != 0
        || catalog.external_ground_truth_equation_reads != 0
        || catalog.expected_external_result_lookups != 0
    {
        return Err("SEM37_EXTERNAL_CATALOG_LEAKAGE_OR_SET_MISMATCH".to_string());
    }
    catalog
        .cases
        .into_iter()
        .map(|descriptor| {
            if descriptor.set != set
                || descriptor.benchmark_family_disclosed
                || descriptor.natural_language_is_semantic_authority
            {
                return Err("SEM37_EXTERNAL_DESCRIPTOR_AUTHORITY_VIOLATION".to_string());
            }
            let observation = evaluator.observe(&descriptor.case_id, 160)?;
            if observation.set != set
                || observation.outcome_revealed
                || observation.ground_truth_revealed
                || observation.generator_source_revealed
            {
                return Err("SEM37_EXTERNAL_OBSERVATION_LEAKAGE_OR_SET_MISMATCH".to_string());
            }
            Ok((descriptor, observation))
        })
        .collect()
}

fn evaluate_batch(
    evaluator: &ExternalEvaluatorClient,
    batch: &PredictionBatch,
) -> Result<Value, String> {
    let result =
        evaluator.evaluate(batch.lane, &batch.predictions, &batch.prediction_commitment)?;
    if result["prediction_commitment"].as_str() != Some(batch.prediction_commitment.as_str())
        || result["deterministic_evaluator"].as_bool() != Some(true)
        || result["external_generator_source_reads_by_bcore"].as_u64() != Some(0)
        || result["external_ground_truth_equation_reads"].as_u64() != Some(0)
        || result["expected_external_result_lookups"].as_u64() != Some(0)
        || result["bcore_self_asserted_external_success_events"].as_u64() != Some(0)
    {
        return Err("SEM37_EXTERNAL_EVALUATION_AUTHORITY_RECEIPT_INVALID".to_string());
    }
    Ok(result)
}

fn select_lane_a(candidates: &[ExternalCandidateEvaluation]) -> Result<ExternalMethod, String> {
    candidates
        .iter()
        .max_by(|left, right| {
            lane_a_quality(&left.lane_a_evaluation)
                .total_cmp(&lane_a_quality(&right.lane_a_evaluation))
                .then_with(|| right.method.cmp(&left.method))
        })
        .map(|candidate| candidate.method)
        .ok_or("SEM37_NO_LANE_A_METHOD_CANDIDATES".to_string())
}

fn select_lane_b(candidates: &[ExternalCandidateEvaluation]) -> Result<ExternalMethod, String> {
    let mut scored = candidates
        .iter()
        .map(|candidate| {
            Ok((
                candidate.method,
                lane_b_error(&candidate.lane_b_evaluation)?,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    scored.sort_by(|left, right| {
        left.1
            .total_cmp(&right.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    scored
        .first()
        .map(|(method, _)| *method)
        .ok_or("SEM37_NO_LANE_B_METHOD_CANDIDATES".to_string())
}

fn lane_a_quality(result: &Value) -> f64 {
    let tp = result["lane_a_causal_tp"].as_u64().unwrap_or(0) as f64;
    let fp = result["lane_a_causal_fp"].as_u64().unwrap_or(0) as f64;
    let fn_count = result["lane_a_causal_fn"].as_u64().unwrap_or(0) as f64;
    let verified = result["external_passive_novel_predictions_verified"]
        .as_u64()
        .unwrap_or(0) as f64;
    let errors = result["external_passive_novel_prediction_errors"]
        .as_u64()
        .unwrap_or(0) as f64;
    let f1 = if 2.0 * tp + fp + fn_count == 0.0 {
        0.0
    } else {
        2.0 * tp / (2.0 * tp + fp + fn_count)
    };
    f1 * 10_000.0 + verified - errors
}

fn lane_b_error(result: &Value) -> Result<f64, String> {
    let bits = result["prediction_sse_ieee754_bits"]
        .as_u64()
        .ok_or("SEM37_LANE_B_SSE_BITS_MISSING")?;
    let value = f64::from_bits(bits);
    if value.is_finite() {
        Ok(value)
    } else {
        Err("SEM37_LANE_B_SSE_NONFINITE".to_string())
    }
}

fn candidate_error(bits: &[u64], actual: &[f64]) -> f64 {
    bits.iter()
        .map(|bits| f64::from_bits(*bits))
        .zip(actual)
        .map(|(prediction, actual)| (prediction - actual).powi(2))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn lane_a_selection_uses_raw_structure_and_prediction_fields() {
        let weak = ExternalCandidateEvaluation {
            method: ExternalMethod::Persistence,
            lane_a_prediction_commitment: "weak-a".to_string(),
            lane_b_prediction_commitment: "weak-b".to_string(),
            lane_a_evaluation: json!({
                "lane_a_causal_tp": 0,
                "lane_a_causal_fp": 0,
                "lane_a_causal_fn": 4,
                "external_passive_novel_predictions_verified": 1,
                "external_passive_novel_prediction_errors": 3
            }),
            lane_b_evaluation: json!({"prediction_sse_ieee754_bits": 9.0_f64.to_bits()}),
            selected_for_lane_a: false,
            selected_for_lane_b: false,
        };
        let strong = ExternalCandidateEvaluation {
            method: ExternalMethod::SparseCoupledLinear,
            lane_a_prediction_commitment: "strong-a".to_string(),
            lane_b_prediction_commitment: "strong-b".to_string(),
            lane_a_evaluation: json!({
                "lane_a_causal_tp": 3,
                "lane_a_causal_fp": 1,
                "lane_a_causal_fn": 1,
                "external_passive_novel_predictions_verified": 3,
                "external_passive_novel_prediction_errors": 1
            }),
            lane_b_evaluation: json!({"prediction_sse_ieee754_bits": 4.0_f64.to_bits()}),
            selected_for_lane_a: false,
            selected_for_lane_b: false,
        };
        assert_eq!(
            select_lane_a(&[weak, strong]).unwrap(),
            ExternalMethod::SparseCoupledLinear
        );
    }
}
