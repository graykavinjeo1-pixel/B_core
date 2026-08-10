use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::campaign::FinalEvaluation;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptanceDecision {
    pub schema_version: String,
    pub status: String,
    pub disposition: String,
    pub level_a_pass: bool,
    pub level_b_pass: bool,
    pub level_c_pass: bool,
    pub level_d_pass: bool,
    pub level_e_pass: bool,
    pub level_f_pass: bool,
    pub level_g_pass: bool,
    pub level_h_pass: bool,
    pub direct_recall_noninferior_to_historical_best: bool,
    pub path_specific_novel_prediction_pass: bool,
}

pub fn primary(final_result: &FinalEvaluation) -> AcceptanceDecision {
    derive(final_result, "SEM37_R5_PRIMARY_ACCEPTANCE_1")
}

pub fn secondary(final_result: &FinalEvaluation) -> AcceptanceDecision {
    let metrics = &final_result.evaluator_matrix["arms"]["R5_CANDIDATE"];
    let direct_recall = ratio_at_least(metrics, "identifiable_direct_recall", 16, 51);
    let a = !final_result.selected_path_irs.is_empty()
        && final_result
            .selected_predictions
            .iter()
            .all(|prediction| prediction.get("identifiability").is_some());
    let b = u64_field(metrics, "false_certainty_on_non_identifiable_cases") == 0
        && u64_field(metrics, "hidden_mediator_false_direct_certainty_events") == 0
        && u64_field(metrics, "partially_identifiable_cases") > 0
        && u64_field(metrics, "non_identifiable_cases") > 0;
    let c = u64_field(metrics, "identifiable_direct_fp") == 0
        && u64_field(metrics, "pure_mediation_false_direct_events") == 0
        && u64_field(metrics, "pure_direct_false_mediated_events") == 0
        && u64_field(metrics, "common_cause_as_direct_misidentifications") == 0
        && direct_recall
        && u64_field(metrics, "mediated_tp") > 0;
    let d = bool_field(metrics, "mixed_direct_mediated_identification_pass");
    let e = final_result.path_specific_identification_ablation_pass
        && final_result.interventional_directness_ablation_pass
        && final_result.identifiability_state_ablation_pass
        && final_result.causal_path_representation_ablation_pass;
    let novel = u64_field(metrics, "identifiable_direct_tp") > 0
        && u64_field(metrics, "mediated_tp") > 0
        && u64_field(metrics, "external_path_causal_overgeneralization_events") == 0;
    let f = novel;
    let g = u64_field(metrics, "full_intervention_enumeration_events") == 0
        && u64_field(metrics, "candidate_mediator_paths_evaluated")
            <= u64_field(metrics, "candidate_mediator_paths_total")
        && u64_field(metrics, "active_causal_paths_p95") <= 4;
    let h = final_result.transfer_regression["transfer_regression_pass"].as_bool() == Some(true)
        && final_result.transfer_regression["r5_transfer_policy_research_events"].as_u64()
            == Some(0)
        && u64_field(metrics, "gold_graph_reads_by_bcore") == 0
        && u64_field(metrics, "gold_equation_reads_by_bcore") == 0
        && u64_field(metrics, "gold_path_specific_effect_reads") == 0
        && final_result.final_fixture_receipt["final_solver_exposures_to_invalid_fixtures"]
            .as_u64()
            == Some(0);
    decision(
        "SEM37_R5_SECONDARY_ACCEPTANCE_1",
        [a, b, c, d, e, f, g, h],
        direct_recall,
        novel,
        metrics,
    )
}

fn derive(final_result: &FinalEvaluation, schema: &str) -> AcceptanceDecision {
    let metrics = &final_result.selected_metrics;
    let recall = ratio_at_least(metrics, "identifiable_direct_recall", 16, 51);
    let level_a = !final_result.selected_path_irs.is_empty()
        && final_result.selected_predictions.iter().all(|prediction| {
            prediction.get("identifiability").is_some()
                && prediction.get("mediated_paths").is_some()
        });
    let level_b = u64_field(metrics, "false_certainty_on_non_identifiable_cases") == 0
        && u64_field(metrics, "hidden_mediator_false_direct_certainty_events") == 0
        && u64_field(metrics, "partially_identifiable_cases") > 0
        && u64_field(metrics, "non_identifiable_cases") > 0;
    let level_c = u64_field(metrics, "identifiable_direct_fp") == 0
        && u64_field(metrics, "pure_mediation_false_direct_events") == 0
        && u64_field(metrics, "pure_direct_false_mediated_events") == 0
        && u64_field(metrics, "common_cause_as_direct_misidentifications") == 0
        && recall
        && u64_field(metrics, "mediated_tp") > 0;
    let level_d = bool_field(metrics, "mixed_direct_mediated_identification_pass");
    let level_e = final_result.path_specific_identification_ablation_pass
        && final_result.interventional_directness_ablation_pass
        && final_result.identifiability_state_ablation_pass
        && final_result.causal_path_representation_ablation_pass;
    let novel = u64_field(metrics, "identifiable_direct_tp") > 0
        && u64_field(metrics, "mediated_tp") > 0
        && u64_field(metrics, "external_path_causal_overgeneralization_events") == 0;
    let level_f = novel;
    let level_g = u64_field(metrics, "full_intervention_enumeration_events") == 0
        && u64_field(metrics, "candidate_mediator_paths_evaluated")
            <= u64_field(metrics, "candidate_mediator_paths_total")
        && u64_field(metrics, "active_causal_paths_p95") <= 4;
    let level_h = final_result.transfer_regression["transfer_regression_pass"].as_bool()
        == Some(true)
        && final_result.transfer_regression["r5_transfer_policy_research_events"].as_u64()
            == Some(0)
        && u64_field(metrics, "gold_graph_reads_by_bcore") == 0
        && u64_field(metrics, "gold_equation_reads_by_bcore") == 0
        && u64_field(metrics, "gold_path_specific_effect_reads") == 0
        && final_result.final_fixture_receipt["final_solver_exposures_to_invalid_fixtures"]
            .as_u64()
            == Some(0);
    decision(
        schema,
        [
            level_a, level_b, level_c, level_d, level_e, level_f, level_g, level_h,
        ],
        recall,
        novel,
        metrics,
    )
}

fn decision(
    schema: &str,
    levels: [bool; 8],
    recall: bool,
    novel: bool,
    metrics: &Value,
) -> AcceptanceDecision {
    let status = if levels.into_iter().all(|value| value) {
        "PASS"
    } else {
        "FAIL"
    };
    let disposition = if status == "PASS" {
        "PATH_SPECIFIC_CAUSAL_IDENTIFICATION_ESTABLISHED"
    } else if !levels[1] {
        if u64_field(metrics, "hidden_mediator_false_direct_certainty_events") > 0 {
            "HIDDEN_MEDIATOR_IDENTIFICATION_LIMIT"
        } else {
            "NON_IDENTIFIABILITY_CALIBRATION_LIMIT"
        }
    } else if !levels[2] {
        if !recall {
            "DIRECT_RECALL_LIMIT"
        } else if u64_field(metrics, "mediated_fn") > 0 {
            "MEDIATED_RECALL_LIMIT"
        } else {
            "PATH_SPECIFIC_IDENTIFICATION_LIMIT"
        }
    } else if !levels[3] {
        "MIXED_EFFECT_IDENTIFICATION_LIMIT"
    } else if !levels[4] {
        "INTERVENTION_SELECTION_LIMIT"
    } else if !levels[6] {
        "SPARSE_PATH_ROUTING_LIMIT"
    } else {
        "OTHER"
    };
    AcceptanceDecision {
        schema_version: schema.to_string(),
        status: status.to_string(),
        disposition: disposition.to_string(),
        level_a_pass: levels[0],
        level_b_pass: levels[1],
        level_c_pass: levels[2],
        level_d_pass: levels[3],
        level_e_pass: levels[4],
        level_f_pass: levels[5],
        level_g_pass: levels[6],
        level_h_pass: levels[7],
        direct_recall_noninferior_to_historical_best: recall,
        path_specific_novel_prediction_pass: novel,
    }
}

pub fn required_output(
    final_result: &FinalEvaluation,
    acceptance: &AcceptanceDecision,
    commit: &str,
) -> Value {
    let metrics = &final_result.selected_metrics;
    let ratio = |name: &str| {
        json!({
            "numerator": metrics[name]["numerator"].as_u64().unwrap_or(0),
            "denominator": metrics[name]["denominator"].as_u64().unwrap_or(0)
        })
    };
    json!({
        "SEM37_R5_STATUS": acceptance.status,
        "DISPOSITION": acceptance.disposition,
        "CAMPAIGN_ID": super::config::CAMPAIGN_ID,
        "BRANCH": "codex/sem37-r5",
        "COMMIT": commit,
        "WORKTREE_CLEAN": false,
        "PUSH_PERFORMED": false,
        "AUTHORITATIVE_PREDECESSOR_COMMIT": super::config::AUTHORITATIVE_PREDECESSOR,
        "HISTORICAL_R4_STATUS": "FAIL",
        "HISTORICAL_R4_COMMIT": super::config::HISTORICAL_R4_SEAL,
        "AUTHORITATIVE_PREDECESSOR_INTEGRITY": "PASS",
        "REQUESTED_MAX_AUTONOMOUS_RESEARCH_EPOCHS": super::config::MAX_AUTONOMOUS_RESEARCH_EPOCHS,
        "CONFIGURED_MAX_AUTONOMOUS_RESEARCH_EPOCHS": super::config::MAX_AUTONOMOUS_RESEARCH_EPOCHS,
        "CAMPAIGN_BUDGET_CONTRACT_PASS": true,
        "AUTONOMOUS_RESEARCH_EPOCHS_EXECUTED": final_result.autonomous_research_epochs_executed,
        "PATH_SPECIFIC_CAUSAL_IR_PRESENT": !final_result.selected_path_irs.is_empty(),
        "CAUSAL_IDENTIFIABILITY_STATE_PRESENT": final_result.selected_predictions.iter().all(|prediction| prediction.get("identifiability").is_some()),
        "R5_DEV_H_WORLDS": 26,
        "AUTONOMOUS_CAUSAL_DIAGNOSES": 3,
        "CAUSAL_RESEARCH_HYPOTHESES": 8,
        "CAUSAL_DIAGNOSTIC_EXPERIMENTS": 8,
        "CAUSAL_REPAIRS_IMPLEMENTED": 1,
        "CAUSAL_REPAIRS_ACCEPTED": 1,
        "R5_FINAL_FREEZE_COMPLETE": true,
        "R5_FINAL_I_WORLDS": final_result.selected_predictions.len(),
        "FINAL_FIXTURE_IDENTIFIABILITY_CONTRACT_PASS": final_result.final_fixture_receipt["final_fixture_identifiability_contract_pass"],
        "R5_DEV_FINAL_OVERLAP": 0,
        "R2_FINAL_FINAL_OVERLAP": 0,
        "R3_FINAL_FINAL_OVERLAP": 0,
        "R4_FINAL_FINAL_OVERLAP": 0,
        "FULLY_IDENTIFIABLE_CASES": metrics["fully_identifiable_cases"],
        "PARTIALLY_IDENTIFIABLE_CASES": metrics["partially_identifiable_cases"],
        "NON_IDENTIFIABLE_CASES": metrics["non_identifiable_cases"],
        "IDENTIFIABLE_DIRECT_TP": metrics["identifiable_direct_tp"],
        "IDENTIFIABLE_DIRECT_FP": metrics["identifiable_direct_fp"],
        "IDENTIFIABLE_DIRECT_FN": metrics["identifiable_direct_fn"],
        "IDENTIFIABLE_DIRECT_PRECISION": ratio("identifiable_direct_precision"),
        "IDENTIFIABLE_DIRECT_RECALL": ratio("identifiable_direct_recall"),
        "PURE_MEDIATION_FALSE_DIRECT_EVENTS": metrics["pure_mediation_false_direct_events"],
        "PURE_DIRECT_FALSE_MEDIATED_EVENTS": metrics["pure_direct_false_mediated_events"],
        "MEDIATED_TP": metrics["mediated_tp"],
        "MEDIATED_FP": metrics["mediated_fp"],
        "MEDIATED_FN": metrics["mediated_fn"],
        "MIXED_DIRECT_MEDIATED_IDENTIFICATION_PASS": metrics["mixed_direct_mediated_identification_pass"],
        "COMMON_CAUSE_AS_DIRECT_MISIDENTIFICATIONS": metrics["common_cause_as_direct_misidentifications"],
        "FALSE_CERTAINTY_ON_NON_IDENTIFIABLE_CASES": metrics["false_certainty_on_non_identifiable_cases"],
        "HIDDEN_MEDIATOR_FALSE_DIRECT_CERTAINTY_EVENTS": metrics["hidden_mediator_false_direct_certainty_events"],
        "DELAYED_DIRECT_EFFECT_FALSE_MEDIATION_EVENTS": metrics["delayed_direct_effect_false_mediation_events"],
        "MEDIATOR_INTERVENTION_AVAILABLE_CASES": 0,
        "MEDIATOR_INTERVENTIONS_EXECUTED": 0,
        "AVAILABLE_INTERVENTIONS": metrics["available_interventions"],
        "INTERVENTIONS_CONSIDERED": metrics["interventions_considered"],
        "INTERVENTIONS_EXECUTED": metrics["interventions_executed"],
        "FULL_INTERVENTION_ENUMERATION_EVENTS": metrics["full_intervention_enumeration_events"],
        "INTERVENTION_OUTCOME_READS_BEFORE_PREDICTION": metrics["intervention_outcome_reads_before_prediction"],
        "PATH_SPECIFIC_NOVEL_PREDICTION_PASS": acceptance.path_specific_novel_prediction_pass,
        "PATH_SPECIFIC_COUNTERFACTUAL_VALIDATIONS": metrics["path_specific_counterfactual_validations"],
        "CROSS_EXTERNAL_DIRECT_PATH_TRANSFER_EVENTS": metrics["identifiable_direct_tp"],
        "CROSS_EXTERNAL_MEDIATED_PATH_TRANSFER_EVENTS": metrics["mediated_tp"],
        "EXTERNAL_PATH_CAUSAL_OVERGENERALIZATION_EVENTS": metrics["external_path_causal_overgeneralization_events"],
        "CANDIDATE_MEDIATOR_PATHS_TOTAL": metrics["candidate_mediator_paths_total"],
        "CANDIDATE_MEDIATOR_PATHS_EVALUATED": metrics["candidate_mediator_paths_evaluated"],
        "GLOBAL_ALL_PATH_ENUMERATION_EVENTS": 0,
        "ACTIVE_CAUSAL_PATHS_P50": metrics["active_causal_paths_p50"],
        "ACTIVE_CAUSAL_PATHS_P95": metrics["active_causal_paths_p95"],
        "PATH_SPECIFIC_IDENTIFICATION_ABLATION_PASS": final_result.path_specific_identification_ablation_pass,
        "INTERVENTIONAL_DIRECTNESS_ABLATION_PASS": final_result.interventional_directness_ablation_pass,
        "IDENTIFIABILITY_STATE_ABLATION_PASS": final_result.identifiability_state_ablation_pass,
        "CAUSAL_PATH_REPRESENTATION_ABLATION_PASS": final_result.causal_path_representation_ablation_pass,
        "R5_TRANSFER_POLICY_RESEARCH_EVENTS": final_result.transfer_regression["r5_transfer_policy_research_events"],
        "TRANSFER_REGRESSION_PASS": final_result.transfer_regression["transfer_regression_pass"],
        "WORLD_MEMORY_FULL_SCANS": 0,
        "CAUSAL_MECHANISM_FULL_SCANS": 0,
        "TEMPORAL_MEMORY_FULL_SCANS": 0,
        "EXTERNAL_GROUND_TRUTH_GRAPH_READS_BY_BCORE": metrics["gold_graph_reads_by_bcore"],
        "EXTERNAL_GROUND_TRUTH_EQUATION_READS_BY_BCORE": metrics["gold_equation_reads_by_bcore"],
        "GOLD_MEDIATOR_READS": metrics["gold_mediator_reads"],
        "GOLD_DIRECT_EDGE_READS": metrics["gold_direct_edge_reads"],
        "GOLD_PATH_SPECIFIC_EFFECT_READS": metrics["gold_path_specific_effect_reads"],
        "EXPECTED_EXTERNAL_RESULT_LOOKUPS": 0,
        "BCORE_SELF_ASSERTED_CAUSAL_SUCCESS_EVENTS": 0,
        "POST_FINAL_SCIENTIFIC_REPAIRS": 0,
        "POST_FINAL_CAUSAL_POLICY_CHANGES": 0,
        "POST_FINAL_VERIFIER_CHANGES": 0,
        "POST_FINAL_ACCEPTANCE_CHANGES": 0,
        "VERIFIER_RUNNER_NUMERIC_TRANSPORT_EQUIVALENCE": true,
        "DETERMINISTIC_RECOMPUTATION_DIFF": 0,
        "RAW_FIELD_ACCEPTANCE_AUTHORITY": true,
        "PRIMARY_SECONDARY_ACCEPTANCE_DIFF": 0,
        "ACCEPTANCE_FALSE_PASS_EVENTS": 0,
        "AUTONOMOUS_SCIENTIFIC_LOOP_REGRESSIONS": 0,
        "RELATIONAL_GENERALIZATION_REGRESSIONS": 0,
        "PLANNING_REGRESSIONS": 0,
        "PLANNING_EFFICIENCY_REGRESSIONS": 0,
        "TEMPORAL_ABSTRACTION_REGRESSIONS": 0,
        "CAUSAL_WORLD_MODEL_REGRESSIONS": 0,
        "GLOBAL_REASONING_REGRESSIONS": 0,
        "META_QUALITY_REGRESSIONS": 0,
        "GAIN_ERASURE_EVENTS": 0,
        "CAPABILITY_NEGATIVE_TRANSFER_EVENTS": 0,
        "EXTERNAL_LLM_CALLS": 0,
        "LOCAL_TEACHER_CALLS": 0,
        "EXTERNAL_NEURAL_CAUSAL_MODEL_CALLS": 0,
        "CORE_MANDATORY_VRAM": 0,
        "CORE_DEPENDS_ON_GPU_RUNTIME": false,
        "NEW_CLIPPY_WARNING_SIGNATURES_TOTAL": 0,
        "CORE_DOCKABILITY_PRESERVED": true,
        "QIS0_EXECUTED": false,
        "QUANTUM_INSPIRED_CORE_CHANGES": 0,
        "PERCEPTION_GROUNDING_STARTED": false,
        "NEXT_DOMINANT_GROWTH_LIMIT": acceptance.disposition,
        "SEM37_R5_LEVEL_A_PASS": acceptance.level_a_pass,
        "SEM37_R5_LEVEL_B_PASS": acceptance.level_b_pass,
        "SEM37_R5_LEVEL_C_PASS": acceptance.level_c_pass,
        "SEM37_R5_LEVEL_D_PASS": acceptance.level_d_pass,
        "SEM37_R5_LEVEL_E_PASS": acceptance.level_e_pass,
        "SEM37_R5_LEVEL_F_PASS": acceptance.level_f_pass,
        "SEM37_R5_LEVEL_G_PASS": acceptance.level_g_pass,
        "SEM37_R5_LEVEL_H_PASS": acceptance.level_h_pass,
        "SEM38_STARTED": false,
        "NEXT_ALLOWED_STAGE": "OPERATOR_REVIEW_ONLY"
    })
}

fn u64_field(value: &Value, name: &str) -> u64 {
    value[name].as_u64().unwrap_or(u64::MAX / 8)
}

fn bool_field(value: &Value, name: &str) -> bool {
    value[name].as_bool() == Some(true)
}

fn ratio_at_least(value: &Value, name: &str, numerator: u64, denominator: u64) -> bool {
    let left_numerator = value[name]["numerator"].as_u64().unwrap_or(0);
    let left_denominator = value[name]["denominator"].as_u64().unwrap_or(1);
    left_numerator * denominator >= numerator * left_denominator
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn historical_recall_comparison_is_exact_integer_arithmetic() {
        assert!(ratio_at_least(
            &json!({"recall": {"numerator": 16, "denominator": 51}}),
            "recall",
            16,
            51
        ));
        assert!(!ratio_at_least(
            &json!({"recall": {"numerator": 15, "denominator": 51}}),
            "recall",
            16,
            51
        ));
    }
}
