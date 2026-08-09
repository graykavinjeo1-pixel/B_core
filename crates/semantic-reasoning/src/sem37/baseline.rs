use serde::{Deserialize, Serialize};

use super::adapter::{
    transport_to_sealed_sem36, ExternalCatalog, ExternalEvaluatorClient, ExternalLane, ExternalSet,
    Sem36ExternalTransportDisposition, Sem36ExternalTransportReceipt,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sem36ExternalTransferBaseline {
    pub external_baseline_worlds: u64,
    pub lane_a_worlds: u64,
    pub lane_b_worlds: u64,
    pub external_baseline_frontiers_detected: u64,
    pub external_baseline_hypotheses: u64,
    pub external_baseline_discoveries: u64,
    pub external_baseline_novel_predictions: u64,
    pub external_baseline_predictions_verified: u64,
    pub external_baseline_counterfactual_accuracy_sampled: bool,
    pub external_baseline_counterfactual_verified: u64,
    pub external_repair_required: bool,
    pub measured_disposition: String,
    pub transport_receipts: Vec<Sem36ExternalTransportReceipt>,
    pub human_research_question_selection_events: u64,
    pub human_hypothesis_selection_events: u64,
    pub human_experiment_selection_events: u64,
    pub benchmark_specific_causal_hint_branches: u64,
    pub external_generator_source_reads_by_bcore: u64,
    pub external_ground_truth_graph_reads: u64,
    pub external_ground_truth_equation_reads: u64,
    pub expected_external_result_lookups: u64,
    pub network_reads_during_baseline: u64,
    pub network_writes_during_baseline: u64,
}

pub fn run_sem36_external_transfer_baseline(
    evaluator: &ExternalEvaluatorClient,
) -> Result<Sem36ExternalTransferBaseline, String> {
    let catalog: ExternalCatalog = evaluator.catalog(ExternalSet::A)?;
    let mut receipts = Vec::with_capacity(catalog.cases.len());
    for case in &catalog.cases {
        let observation = evaluator.observe(&case.case_id, 160)?;
        if observation.outcome_revealed
            || observation.ground_truth_revealed
            || observation.generator_source_revealed
        {
            return Err("SEM37_BASELINE_EXTERNAL_LEAKAGE".to_string());
        }
        receipts.push(transport_to_sealed_sem36(&observation));
    }
    let transportable = receipts
        .iter()
        .filter(|receipt| receipt.disposition == Sem36ExternalTransportDisposition::Transportable)
        .count() as u64;
    let frontiers = receipts.len() as u64 - transportable;
    let repair_required = frontiers > 0;
    Ok(Sem36ExternalTransferBaseline {
        external_baseline_worlds: catalog.cases.len() as u64,
        lane_a_worlds: catalog
            .cases
            .iter()
            .filter(|case| case.lane == ExternalLane::A)
            .count() as u64,
        lane_b_worlds: catalog
            .cases
            .iter()
            .filter(|case| case.lane == ExternalLane::B)
            .count() as u64,
        external_baseline_frontiers_detected: frontiers,
        external_baseline_hypotheses: 0,
        external_baseline_discoveries: 0,
        external_baseline_novel_predictions: 0,
        external_baseline_predictions_verified: 0,
        external_baseline_counterfactual_accuracy_sampled: false,
        external_baseline_counterfactual_verified: 0,
        external_repair_required: repair_required,
        measured_disposition: if repair_required {
            "EXTERNAL_GROUNDING_LIMIT"
        } else {
            "ZERO_SHOT_EXTERNAL_TRANSFER_SUFFICIENT"
        }
        .to_string(),
        transport_receipts: receipts,
        human_research_question_selection_events: 0,
        human_hypothesis_selection_events: 0,
        human_experiment_selection_events: 0,
        benchmark_specific_causal_hint_branches: 0,
        external_generator_source_reads_by_bcore: catalog.external_generator_source_reads_by_bcore,
        external_ground_truth_graph_reads: catalog.external_ground_truth_graph_reads,
        external_ground_truth_equation_reads: catalog.external_ground_truth_equation_reads,
        expected_external_result_lookups: catalog.expected_external_result_lookups,
        network_reads_during_baseline: 0,
        network_writes_during_baseline: 0,
    })
}
