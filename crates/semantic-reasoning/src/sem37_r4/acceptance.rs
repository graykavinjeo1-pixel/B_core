use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::campaign::{bool_field, field, ratio_ge, AutonomousDevelopment, FinalEvaluation};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub direct_precision_noninferior_to_best_comparator: bool,
    pub direct_recall_noninferior_to_best_comparator: bool,
    pub causal_effect_decomposition_present: bool,
    pub r2_direct_tp: u64,
    pub r2_direct_fp: u64,
    pub r2_direct_fn: u64,
    pub r3_direct_tp: u64,
    pub r3_direct_fp: u64,
    pub r3_direct_fn: u64,
    pub r4_direct_tp: u64,
    pub r4_direct_fp: u64,
    pub r4_direct_fn: u64,
    pub r4_mediator_as_direct: u64,
    pub r4_common_cause_as_direct: u64,
    pub mediated_tp: u64,
    pub mediated_fp: u64,
    pub mediated_fn: u64,
    pub mixed_direct_mediated_pass: bool,
    pub mediated_path_structure_correct: bool,
    pub effect_accounting_consistency_pass: bool,
    pub transfer_candidates_total: u64,
    pub transfer_promoted: u64,
    pub transfer_abstained: u64,
    pub transfer_rejected: u64,
    pub positive_transfer_opportunities: u64,
    pub positive_transfer_accepted: u64,
    pub positive_transfer_verified: u64,
    pub r3_positive_transfer_verified: u64,
    pub negative_transfer_opportunities: u64,
    pub negative_transfer_accepted: u64,
    pub ambiguous_transfer_opportunities: u64,
    pub ambiguous_transfer_abstentions: u64,
}

pub fn evaluate(
    final_evaluation: &FinalEvaluation,
    development: &AutonomousDevelopment,
    p0: &Value,
    final_freeze: &Value,
    final_manifest: &Value,
) -> Result<AcceptanceDecision, String> {
    let arms = &final_evaluation.raw_arm_matrix["arms"];
    let r2 = &arms["R2_COMPARATOR"]["lane_a"];
    let r3 = &arms["R3_COMPARATOR"]["lane_a"];
    let r4 = &arms["R4_CANDIDATE"]["lane_a"];
    let transfer = &arms["R4_CANDIDATE"]["lane_b"];
    let r3_transfer = &arms["R3_COMPARATOR"]["lane_b"];
    let precision_noninferior =
        ratio_ge(r4, r2, "direct_precision_exact") && ratio_ge(r4, r3, "direct_precision_exact");
    let recall_noninferior =
        ratio_ge(r4, r2, "direct_recall_exact") && ratio_ge(r4, r3, "direct_recall_exact");
    let decomposition_present = !final_evaluation.r4_causal_batch.decompositions.is_empty()
        && final_evaluation
            .r4_causal_batch
            .decompositions
            .iter()
            .all(|item| {
                item.total_effect_units
                    == item.direct_component_units
                        + item
                            .mediated_components
                            .iter()
                            .map(|component| component.effect_units)
                            .sum::<u64>()
                        + item.confounding_component_units
                        + item.unresolved_component_units
            });
    let mediator_as_direct = field(r4, "mediator_as_direct_misidentifications")?;
    let common_as_direct = field(r4, "common_cause_as_direct_misidentifications")?;
    let mixed_pass = bool_field(r4, "mixed_direct_mediated_decomposition_pass")?;
    let path_pass = bool_field(r4, "mediated_path_structure_correct")?;
    let accounting_pass = bool_field(r4, "causal_effect_accounting_consistency_pass")?;
    let negative_accepted = field(transfer, "negative_transfer_accepted")?;
    let positive_verified = field(transfer, "positive_transfer_verified")?;
    let r3_positive_verified = field(r3_transfer, "positive_transfer_verified")?;
    let ambiguous_abstentions = field(transfer, "ambiguous_transfer_abstentions")?;
    let counterfactuals = field(transfer, "apply_no_change_counterfactual_present")?;
    let candidates = field(transfer, "transfer_candidates_total")?;
    let outcome_reads = field(transfer, "transfer_outcome_reads_before_promotion_decision")?;
    let level_a = decomposition_present && accounting_pass;
    let level_b = mediator_as_direct == 0 && common_as_direct == 0 && field(r4, "direct_tp")? > 0;
    let level_c = precision_noninferior && recall_noninferior;
    let level_d = mixed_pass;
    let level_e = negative_accepted == 0 && counterfactuals == candidates && outcome_reads == 0;
    let level_f = field(transfer, "positive_transfer_accepted")? > 0
        && positive_verified > 0
        && positive_verified >= r3_positive_verified
        && ambiguous_abstentions > 0;
    let level_g = development.direct_effect_decomposition_ablation_pass
        && development.total_effect_only_baseline_dominated
        && development.r3_taxonomy_only_baseline_dominated
        && development.no_change_counterfactual_promotion_ablation_pass
        && development.transfer_safety_memory_ablation_pass
        && development.always_abstain_baseline_dominated;
    let level_h = final_evaluation.raw_arm_matrix["final_causal_fixture_contract_pass"].as_bool()
        == Some(true)
        && final_evaluation.raw_arm_matrix["final_transfer_fixture_contract_pass"].as_bool()
            == Some(true)
        && final_manifest["final_holdout_model_dependent_selection_events"].as_u64() == Some(0)
        && final_manifest["final_solver_exposures_to_invalid_fixtures"].as_u64() == Some(0)
        && p0["authoritative_predecessor_integrity"].as_str() == Some("PASS")
        && final_freeze["FINAL_FREEZE_COMPLETE"].as_bool() == Some(true)
        && final_evaluation.post_final_scientific_repairs == 0
        && final_evaluation.post_final_promotion_policy_changes == 0
        && final_evaluation.post_final_verifier_changes == 0
        && final_evaluation.post_final_acceptance_changes == 0
        && final_evaluation.r4_causal_batch.world_memory_full_scans == 0
        && final_evaluation.r4_causal_batch.causal_mechanism_full_scans == 0
        && final_evaluation.r4_causal_batch.temporal_memory_full_scans == 0;
    let status = if [
        level_a, level_b, level_c, level_d, level_e, level_f, level_g, level_h,
    ]
    .into_iter()
    .all(|value| value)
    {
        "PASS"
    } else {
        "FAIL"
    };
    let disposition = if status == "PASS" {
        "R4_CAUSAL_EFFECT_AND_COUNTERFACTUAL_TRANSFER_GATES_PASSED"
    } else if !level_b {
        if mediator_as_direct > 0 {
            "DIRECT_EFFECT_IDENTIFICATION_LIMIT"
        } else {
            "DIRECT_RECALL_LIMIT"
        }
    } else if !level_d {
        "MIXED_DIRECT_MEDIATED_LIMIT"
    } else if !level_e {
        "TRANSFER_PROMOTION_SAFETY_LIMIT"
    } else if !level_f {
        "USEFUL_TRANSFER_RETENTION_LIMIT"
    } else if !level_g {
        "EXTERNAL_GENERALIZATION_LIMIT"
    } else {
        "OTHER"
    };
    Ok(AcceptanceDecision {
        schema_version: "SEM37_R4_PRIMARY_ACCEPTANCE_1".to_string(),
        status: status.to_string(),
        disposition: disposition.to_string(),
        level_a_pass: level_a,
        level_b_pass: level_b,
        level_c_pass: level_c,
        level_d_pass: level_d,
        level_e_pass: level_e,
        level_f_pass: level_f,
        level_g_pass: level_g,
        level_h_pass: level_h,
        direct_precision_noninferior_to_best_comparator: precision_noninferior,
        direct_recall_noninferior_to_best_comparator: recall_noninferior,
        causal_effect_decomposition_present: decomposition_present,
        r2_direct_tp: field(r2, "direct_tp")?,
        r2_direct_fp: field(r2, "direct_fp")?,
        r2_direct_fn: field(r2, "direct_fn")?,
        r3_direct_tp: field(r3, "direct_tp")?,
        r3_direct_fp: field(r3, "direct_fp")?,
        r3_direct_fn: field(r3, "direct_fn")?,
        r4_direct_tp: field(r4, "direct_tp")?,
        r4_direct_fp: field(r4, "direct_fp")?,
        r4_direct_fn: field(r4, "direct_fn")?,
        r4_mediator_as_direct: mediator_as_direct,
        r4_common_cause_as_direct: common_as_direct,
        mediated_tp: field(r4, "mediated_true_positives")?,
        mediated_fp: field(r4, "mediated_false_positives")?,
        mediated_fn: field(r4, "mediated_false_negatives")?,
        mixed_direct_mediated_pass: mixed_pass,
        mediated_path_structure_correct: path_pass,
        effect_accounting_consistency_pass: accounting_pass,
        transfer_candidates_total: candidates,
        transfer_promoted: field(transfer, "transfer_promoted")?,
        transfer_abstained: field(transfer, "transfer_abstained")?,
        transfer_rejected: field(transfer, "transfer_rejected")?,
        positive_transfer_opportunities: field(transfer, "positive_transfer_opportunities")?,
        positive_transfer_accepted: field(transfer, "positive_transfer_accepted")?,
        positive_transfer_verified: positive_verified,
        r3_positive_transfer_verified: r3_positive_verified,
        negative_transfer_opportunities: field(transfer, "negative_transfer_opportunities")?,
        negative_transfer_accepted: negative_accepted,
        ambiguous_transfer_opportunities: field(transfer, "ambiguous_transfer_opportunities")?,
        ambiguous_transfer_abstentions: ambiguous_abstentions,
    })
}

#[cfg(test)]
mod tests {
    use crate::sem37_r4::ontology;

    #[test]
    fn ontology_authorities_are_fail_closed() {
        assert!(!ontology::TOTAL_EFFECT_USED_AS_DIRECT_EDGE_AUTHORITY);
        assert!(!ontology::MDL_OR_COMPRESSION_IS_DIRECTNESS_AUTHORITY);
        assert!(!ontology::TEMPORAL_LAG_USED_AS_MEDIATOR_AUTHORITY);
    }
}
