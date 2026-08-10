use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::campaign::{AutonomousDevelopment, FinalEvaluation};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptanceDecision {
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
    pub direct_precision_noninferior_to_r2: bool,
    pub direct_recall_noninferior_to_r2: bool,
    pub r2_direct_tp: u64,
    pub r2_direct_fp: u64,
    pub r2_direct_fn: u64,
    pub r3_direct_tp: u64,
    pub r3_direct_fp: u64,
    pub r3_direct_fn: u64,
    pub r2_mediator_as_direct: u64,
    pub r2_common_cause_as_direct: u64,
    pub r3_mediator_as_direct: u64,
    pub r3_common_cause_as_direct: u64,
    pub mediated_true_positives: u64,
    pub mediated_false_positives: u64,
    pub mediated_false_negatives: u64,
    pub transfer_candidates_total: u64,
    pub transfer_promoted: u64,
    pub transfer_abstained: u64,
    pub transfer_rejected: u64,
    pub positive_transfer_opportunities: u64,
    pub positive_transfer_accepted: u64,
    pub positive_transfer_verified: u64,
    pub negative_transfer_opportunities: u64,
    pub negative_transfer_accepted: u64,
    pub ambiguous_transfer_cases: u64,
    pub ambiguous_transfer_abstentions: u64,
}

pub fn evaluate(
    final_evaluation: &FinalEvaluation,
    development: &AutonomousDevelopment,
    p0: &Value,
    final_freeze: &Value,
) -> Result<AcceptanceDecision, String> {
    let matrix = &final_evaluation.raw_arm_matrix;
    let r2_causal = &matrix["arms"]["R2_COMPARATOR"]["lane_a"];
    let r3_causal = &matrix["arms"]["R3_CANDIDATE"]["lane_a"];
    let r3_transfer = &matrix["arms"]["R3_CANDIDATE"]["lane_b"];
    let r2_direct_tp = field(r2_causal, "direct_tp")?;
    let r2_direct_fp = field(r2_causal, "direct_fp")?;
    let r2_direct_fn = field(r2_causal, "direct_fn")?;
    let r3_direct_tp = field(r3_causal, "direct_tp")?;
    let r3_direct_fp = field(r3_causal, "direct_fp")?;
    let r3_direct_fn = field(r3_causal, "direct_fn")?;
    let precision_noninferior = ratio_ge(
        r3_direct_tp,
        r3_direct_tp + r3_direct_fp,
        r2_direct_tp,
        r2_direct_tp + r2_direct_fp,
    );
    let recall_noninferior = ratio_ge(
        r3_direct_tp,
        r3_direct_tp + r3_direct_fn,
        r2_direct_tp,
        r2_direct_tp + r2_direct_fn,
    );
    let r2_mediator = field(r2_causal, "mediator_as_direct_misidentifications")?;
    let r2_common = field(r2_causal, "common_cause_as_direct_misidentifications")?;
    let r3_mediator = field(r3_causal, "mediator_as_direct_misidentifications")?;
    let r3_common = field(r3_causal, "common_cause_as_direct_misidentifications")?;
    let relation_counts = r3_causal["predicted_relation_counts"]
        .as_object()
        .ok_or("SEM37_R3_RELATION_COUNTS_MISSING")?;
    let level_a = ["DIRECT", "MEDIATED", "CONFOUNDED", "UNRESOLVED"]
        .iter()
        .all(|class| relation_counts.contains_key(*class));
    let level_b = r3_mediator == 0 && r3_direct_tp > 0;
    let level_c = r3_common == 0;
    let level_d = precision_noninferior && recall_noninferior;
    let positive_accepted = field(r3_transfer, "positive_transfer_accepted")?;
    let positive_verified = field(r3_transfer, "positive_transfer_verified")?;
    let negative_accepted = field(r3_transfer, "negative_transfer_accepted")?;
    let level_e = negative_accepted == 0 && positive_accepted > 0 && positive_verified > 0;
    let level_f = field(r3_causal, "direct_tp")? > 0
        && field(r3_causal, "mediated_true_positives")? > 0
        && matrix["positive_transfer_opportunities_present"].as_bool() == Some(true)
        && matrix["negative_transfer_traps_present"].as_bool() == Some(true)
        && matrix["ambiguous_transfer_cases_present"].as_bool() == Some(true);
    let level_g = development.direct_mediated_decomposition_ablation_pass
        && development.transfer_promotion_safety_ablation_pass
        && development.transfer_safety_memory_ablation_pass
        && development.always_abstain_baseline_dominated;
    let level_h = p0["p0_semantic_behavior_diff"].as_u64() == Some(0)
        && p0["p0_causal_behavior_diff"].as_u64() == Some(0)
        && p0["p0_scientific_loop_diff"].as_u64() == Some(0)
        && p0["workspace_tests_pass"].as_bool() == Some(true)
        && final_freeze["FINAL_FREEZE_COMPLETE"].as_bool() == Some(true)
        && matrix["raw_field_acceptance_authority"].as_bool() == Some(true)
        && final_evaluation.post_final_scientific_repairs == 0
        && final_evaluation.post_final_policy_changes == 0
        && final_evaluation.r3_causal_batch.world_memory_full_scans == 0
        && final_evaluation.r3_causal_batch.causal_mechanism_full_scans == 0
        && final_evaluation.r3_causal_batch.temporal_memory_full_scans == 0;
    let levels = [
        level_a, level_b, level_c, level_d, level_e, level_f, level_g, level_h,
    ];
    let status = if levels.into_iter().all(|pass| pass) {
        "PASS"
    } else {
        "FAIL"
    };
    let disposition = if status == "PASS" {
        "DIRECT_MEDIATED_DECOMPOSITION_AND_SAFE_TRANSFER_PROMOTION"
    } else if r3_mediator > 0 {
        "MEDIATION_IDENTIFICATION_LIMIT"
    } else if !recall_noninferior {
        "DIRECT_RECALL_LIMIT"
    } else if negative_accepted > 0 {
        "TRANSFER_PROMOTION_SAFETY_LIMIT"
    } else if positive_verified == 0 {
        "TRANSFER_USEFULNESS_LIMIT"
    } else if r3_common > 0 {
        "CONFOUNDING_LIMIT"
    } else if !level_f {
        "EXTERNAL_GENERALIZATION_LIMIT"
    } else {
        "OTHER"
    };
    Ok(AcceptanceDecision {
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
        direct_precision_noninferior_to_r2: precision_noninferior,
        direct_recall_noninferior_to_r2: recall_noninferior,
        r2_direct_tp,
        r2_direct_fp,
        r2_direct_fn,
        r3_direct_tp,
        r3_direct_fp,
        r3_direct_fn,
        r2_mediator_as_direct: r2_mediator,
        r2_common_cause_as_direct: r2_common,
        r3_mediator_as_direct: r3_mediator,
        r3_common_cause_as_direct: r3_common,
        mediated_true_positives: field(r3_causal, "mediated_true_positives")?,
        mediated_false_positives: field(r3_causal, "mediated_false_positives")?,
        mediated_false_negatives: field(r3_causal, "mediated_false_negatives")?,
        transfer_candidates_total: field(r3_transfer, "transfer_candidates_total")?,
        transfer_promoted: field(r3_transfer, "transfer_promoted")?,
        transfer_abstained: field(r3_transfer, "transfer_abstained")?,
        transfer_rejected: field(r3_transfer, "transfer_rejected")?,
        positive_transfer_opportunities: field(r3_transfer, "positive_transfer_opportunities")?,
        positive_transfer_accepted: positive_accepted,
        positive_transfer_verified: positive_verified,
        negative_transfer_opportunities: field(r3_transfer, "negative_transfer_opportunities")?,
        negative_transfer_accepted: negative_accepted,
        ambiguous_transfer_cases: field(r3_transfer, "ambiguous_transfer_cases")?,
        ambiguous_transfer_abstentions: field(r3_transfer, "ambiguous_transfer_abstentions")?,
    })
}

fn ratio_ge(left_n: u64, left_d: u64, right_n: u64, right_d: u64) -> bool {
    left_d > 0
        && right_d > 0
        && left_n as u128 * right_d as u128 >= right_n as u128 * left_d as u128
}

fn field(value: &Value, name: &str) -> Result<u64, String> {
    value[name]
        .as_u64()
        .ok_or_else(|| format!("SEM37_R3_ACCEPTANCE_FIELD_MISSING:{name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_ratio_comparison_does_not_use_float_tolerance() {
        assert!(ratio_ge(12, 16, 41, 60));
        assert!(!ratio_ge(10, 16, 41, 60));
    }
}
