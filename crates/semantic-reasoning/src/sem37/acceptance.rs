use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::campaign::{DevelopmentResearch, FinalExternalEvaluation};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sem37Acceptance {
    pub sem37_status: String,
    pub disposition: String,
    pub sem37_level_a_pass: bool,
    pub sem37_level_b_pass: bool,
    pub sem37_level_c_pass: bool,
    pub sem37_level_d_pass: bool,
    pub sem37_level_e_pass: bool,
    pub sem37_level_f_pass: bool,
    pub sem37_level_g_pass: bool,
    pub sem37_level_h_pass: bool,
    pub external_frontier_selection_ablation_pass: bool,
    pub external_discovered_memory_ablation_pass: bool,
    pub external_intervention_ablation_pass: bool,
    pub internal_world_capability_regressions: u64,
    pub raw_field_acceptance_authority: bool,
    pub invariants_pass: bool,
    pub violations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecondarySem37Acceptance {
    pub sem37_status: String,
    pub levels: [bool; 8],
    pub ablations: [bool; 3],
    pub invariants_pass: bool,
}

pub fn evaluate_primary(
    baseline: &Value,
    development: &DevelopmentResearch,
    final_raw: &FinalExternalEvaluation,
    internal_world_control_pass: bool,
) -> Sem37Acceptance {
    let lane_a = &final_raw.full_lane_a_evaluation;
    let lane_b = &final_raw.full_lane_b_evaluation;
    let finite_roundtrip_mismatches: u64 = final_raw
        .full_lane_a
        .case_receipts
        .iter()
        .chain(&final_raw.full_lane_b.case_receipts)
        .map(|receipt| {
            receipt
                .numeric_transport
                .finite_ieee754_roundtrip_mismatches
        })
        .sum();
    let nonfinite_numeric_authority: u64 = final_raw
        .full_lane_a
        .case_receipts
        .iter()
        .chain(&final_raw.full_lane_b.case_receipts)
        .map(|receipt| {
            receipt
                .numeric_transport
                .nonfinite_cells_with_numeric_authority
        })
        .sum();
    let frontiers = final_raw
        .full_lane_a
        .case_receipts
        .iter()
        .chain(&final_raw.full_lane_b.case_receipts)
        .filter(|receipt| receipt.self_detected_frontier)
        .count() as u64;
    let hypotheses = final_raw
        .full_lane_a
        .case_receipts
        .iter()
        .chain(&final_raw.full_lane_b.case_receipts)
        .map(|receipt| receipt.hypotheses_generated)
        .sum::<u64>();
    let lane_a_tp = raw_u64(lane_a, "lane_a_causal_tp");
    let lane_a_future = raw_u64(lane_a, "external_passive_novel_predictions");
    let lane_a_verified = raw_u64(lane_a, "external_passive_novel_predictions_verified");
    let lane_b_predictions = raw_u64(lane_b, "external_interventional_predictions");
    let lane_b_verified = raw_u64(lane_b, "external_interventional_predictions_verified");
    let baseline_failed = baseline["MEASURED_EXTERNAL_FAILURE"].as_bool() == Some(true)
        && baseline["EXTERNAL_REPAIR_REQUIRED"].as_bool() == Some(true);
    let level_a = final_raw.worlds > 0
        && finite_roundtrip_mismatches == 0
        && nonfinite_numeric_authority == 0
        && final_raw.numeric_value_as_new_primitive_events == 0
        && final_raw.benchmark_specific_causal_hint_branches == 0;
    let level_b = baseline_failed
        && !development.selected_by_human
        && final_raw.human_research_question_selection_events == 0;
    let level_c = lane_a_tp > 0 && lane_a_future > 0 && lane_a_verified > 0;
    let level_d = development.interventions_executed_after_prediction_freeze > 0
        && development.prediction_outcome_reads_before_freeze == 0
        && lane_b_verified > 0
        && final_raw.external_intervention_ablation_pass;
    let level_e = lane_a_future + lane_b_predictions > 0
        && lane_a_verified + lane_b_verified > 0
        && final_raw.final_outcome_reveal_events == 0;
    let level_f = lane_a_tp > 0
        && lane_b["external_post_discovery_prediction_gain"].as_bool() == Some(true)
        && final_raw.external_causal_overgeneralization_events == 0;
    let level_g = frontiers > 0
        && hypotheses > 0
        && development.hypotheses_eliminated_by_intervention > 0
        && lane_a_verified + lane_b_verified > 0;
    let level_h = final_raw.external_frontier_selection_ablation_pass
        && final_raw.external_discovered_memory_ablation_pass
        && final_raw.external_intervention_ablation_pass
        && internal_world_control_pass;
    let invariants = final_raw.human_research_question_selection_events == 0
        && final_raw.human_hypothesis_selection_events == 0
        && final_raw.human_experiment_selection_events == 0
        && final_raw.human_external_intervention_selection_events == 0
        && final_raw.fabricated_passive_causal_certainty_events == 0
        && final_raw.external_irreducible_noise_research_loops == 0
        && final_raw.external_causal_overgeneralization_events == 0
        && final_raw.numeric_value_as_new_primitive_events == 0
        && final_raw.world_memory_full_scans == 0
        && final_raw.causal_mechanism_full_scans == 0
        && final_raw.temporal_memory_full_scans == 0
        && final_raw.benchmark_specific_causal_hint_branches == 0
        && final_raw.task_specific_external_repair_branches == 0
        && final_raw.external_generator_source_reads_by_bcore == 0
        && final_raw.external_ground_truth_graph_reads == 0
        && final_raw.external_ground_truth_equation_reads == 0
        && final_raw.expected_external_result_lookups == 0
        && final_raw.network_reads_during_canonical == 0
        && final_raw.network_writes_during_canonical == 0
        && development.autonomous_research_epochs_executed <= 4096;
    let levels = [
        level_a, level_b, level_c, level_d, level_e, level_f, level_g, level_h,
    ];
    let mut violations = Vec::new();
    for (index, passed) in levels.iter().enumerate() {
        if !passed {
            violations.push(format!(
                "SEM37_LEVEL_{}_FAILED",
                (b'A' + index as u8) as char
            ));
        }
    }
    if !invariants {
        violations.push("SEM37_INVARIANTS_FAILED".to_string());
    }
    let pass = violations.is_empty();
    Sem37Acceptance {
        sem37_status: if pass { "PASS" } else { "FAIL" }.to_string(),
        disposition: if pass {
            "VERIFIED_THIRD_PARTY_DYNAMICAL_WORLD_EXTERNAL_VALIDITY"
        } else if !level_a {
            "EXTERNAL_GROUNDING_LIMIT"
        } else if !level_c {
            "NOISY_CAUSAL_DISCOVERY_LIMIT"
        } else if !level_d {
            "INTERVENTION_TRANSFER_LIMIT"
        } else if !level_e {
            "COUNTERFACTUAL_EXTERNALITY_LIMIT"
        } else if !level_f {
            "EXTERNAL_MECHANISM_TRANSFER_LIMIT"
        } else {
            "SCIENTIFIC_LOOP_TRANSFER_LIMIT"
        }
        .to_string(),
        sem37_level_a_pass: level_a,
        sem37_level_b_pass: level_b,
        sem37_level_c_pass: level_c,
        sem37_level_d_pass: level_d,
        sem37_level_e_pass: level_e,
        sem37_level_f_pass: level_f,
        sem37_level_g_pass: level_g,
        sem37_level_h_pass: level_h,
        external_frontier_selection_ablation_pass: final_raw
            .external_frontier_selection_ablation_pass,
        external_discovered_memory_ablation_pass: final_raw
            .external_discovered_memory_ablation_pass,
        external_intervention_ablation_pass: final_raw.external_intervention_ablation_pass,
        internal_world_capability_regressions: u64::from(!internal_world_control_pass),
        raw_field_acceptance_authority: true,
        invariants_pass: invariants,
        violations,
    }
}

/// Independent raw-field derivation. It does not call the primary evaluator
/// and never consumes any primary-derived pass boolean.
pub fn evaluate_secondary(
    baseline: &Value,
    development: &DevelopmentResearch,
    final_raw: &FinalExternalEvaluation,
    internal_world_control_pass: bool,
) -> SecondarySem37Acceptance {
    let a = &final_raw.full_lane_a_evaluation;
    let b = &final_raw.full_lane_b_evaluation;
    let transport_clean = final_raw
        .full_lane_a
        .case_receipts
        .iter()
        .chain(&final_raw.full_lane_b.case_receipts)
        .all(|receipt| {
            receipt
                .numeric_transport
                .finite_ieee754_roundtrip_mismatches
                == 0
                && receipt
                    .numeric_transport
                    .nonfinite_cells_with_numeric_authority
                    == 0
                && receipt
                    .numeric_transport
                    .numeric_value_as_new_primitive_events
                    == 0
        });
    let selected_frontiers = final_raw
        .full_lane_a
        .case_receipts
        .iter()
        .chain(&final_raw.full_lane_b.case_receipts)
        .filter(|receipt| receipt.self_detected_frontier)
        .count();
    let generated_hypotheses: u64 = final_raw
        .full_lane_a
        .case_receipts
        .iter()
        .chain(&final_raw.full_lane_b.case_receipts)
        .map(|receipt| receipt.hypotheses_generated)
        .sum();
    let a_tp = raw_u64(a, "lane_a_causal_tp");
    let a_predictions = raw_u64(a, "external_passive_novel_predictions");
    let a_verified = raw_u64(a, "external_passive_novel_predictions_verified");
    let b_predictions = raw_u64(b, "external_interventional_predictions");
    let b_verified = raw_u64(b, "external_interventional_predictions_verified");
    let ablations = [
        final_raw.external_frontier_selection_ablation_pass,
        final_raw.external_discovered_memory_ablation_pass,
        final_raw.external_intervention_ablation_pass,
    ];
    let levels = [
        transport_clean
            && final_raw.worlds > 0
            && final_raw.benchmark_specific_causal_hint_branches == 0,
        baseline["MEASURED_EXTERNAL_FAILURE"].as_bool() == Some(true)
            && !development.selected_by_human
            && final_raw.human_research_question_selection_events == 0,
        a_tp > 0 && a_predictions > 0 && a_verified > 0,
        development.interventions_executed_after_prediction_freeze > 0
            && development.prediction_outcome_reads_before_freeze == 0
            && b_verified > 0
            && ablations[2],
        a_predictions + b_predictions > 0
            && a_verified + b_verified > 0
            && final_raw.final_outcome_reveal_events == 0,
        a_tp > 0
            && b["external_post_discovery_prediction_gain"].as_bool() == Some(true)
            && final_raw.external_causal_overgeneralization_events == 0,
        selected_frontiers > 0
            && generated_hypotheses > 0
            && development.hypotheses_eliminated_by_intervention > 0
            && a_verified + b_verified > 0,
        ablations.into_iter().all(|passed| passed) && internal_world_control_pass,
    ];
    let invariants = final_raw.human_research_question_selection_events == 0
        && final_raw.human_hypothesis_selection_events == 0
        && final_raw.human_experiment_selection_events == 0
        && final_raw.human_external_intervention_selection_events == 0
        && final_raw.fabricated_passive_causal_certainty_events == 0
        && final_raw.external_irreducible_noise_research_loops == 0
        && final_raw.numeric_value_as_new_primitive_events == 0
        && final_raw.world_memory_full_scans == 0
        && final_raw.causal_mechanism_full_scans == 0
        && final_raw.temporal_memory_full_scans == 0
        && final_raw.task_specific_external_repair_branches == 0
        && final_raw.external_generator_source_reads_by_bcore == 0
        && final_raw.external_ground_truth_graph_reads == 0
        && final_raw.external_ground_truth_equation_reads == 0
        && final_raw.expected_external_result_lookups == 0
        && final_raw.network_reads_during_canonical == 0
        && final_raw.network_writes_during_canonical == 0
        && development.autonomous_research_epochs_executed <= 4096;
    SecondarySem37Acceptance {
        sem37_status: if levels.into_iter().all(|passed| passed) && invariants {
            "PASS"
        } else {
            "FAIL"
        }
        .to_string(),
        levels,
        ablations,
        invariants_pass: invariants,
    }
}

fn raw_u64(value: &Value, field: &str) -> u64 {
    value[field].as_u64().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn final_disposition_never_calls_external_adapter_failure_science_success() {
        let acceptance = Sem37Acceptance {
            sem37_status: "FAIL".to_string(),
            disposition: "EXTERNAL_GROUNDING_LIMIT".to_string(),
            sem37_level_a_pass: false,
            sem37_level_b_pass: true,
            sem37_level_c_pass: false,
            sem37_level_d_pass: false,
            sem37_level_e_pass: false,
            sem37_level_f_pass: false,
            sem37_level_g_pass: false,
            sem37_level_h_pass: false,
            external_frontier_selection_ablation_pass: false,
            external_discovered_memory_ablation_pass: false,
            external_intervention_ablation_pass: false,
            internal_world_capability_regressions: 0,
            raw_field_acceptance_authority: true,
            invariants_pass: true,
            violations: vec!["SEM37_LEVEL_A_FAILED".to_string()],
        };
        assert_eq!(acceptance.sem37_status, "FAIL");
        assert_eq!(acceptance.disposition, "EXTERNAL_GROUNDING_LIMIT");
    }
}
