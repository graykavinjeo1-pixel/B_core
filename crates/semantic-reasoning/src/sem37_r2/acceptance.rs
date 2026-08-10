use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptanceDecision {
    pub status: String,
    pub disposition: String,
    pub lane_a_tp: u64,
    pub lane_a_fp: u64,
    pub lane_a_fn: u64,
    pub r1_dense_lane_a_tp: u64,
    pub r1_dense_lane_a_fp: u64,
    pub r1_dense_lane_a_fn: u64,
    pub mediated_false_edge_accepts: u64,
    pub indirect_false_edge_accepts: u64,
    pub common_cause_false_edge_accepts: u64,
    pub negative_transfer_attempts: u64,
    pub negative_transfer_prevented: u64,
    pub negative_transfer_accepted: u64,
    pub direct_indirect_causal_discrimination_pass: bool,
    pub common_cause_discrimination_pass: bool,
    pub lane_b_modular_transfer_regression: u64,
    pub lane_b_positive_regression_pass: bool,
    pub exact_precision_strictly_improved_over_r1_dense: bool,
    pub nonempty_direct_structure_retained: bool,
    pub p0_scientific_engine_diff: u64,
    pub recovered_shift_aware_semantic_diff: u64,
    pub r1_shift_aware_pre_final_freeze_proven: bool,
    pub historical_r1_final_c_final_authority: bool,
    pub manual_precision_threshold_repair_events: u64,
    pub raw_field_acceptance_authority: bool,
    pub post_final_repairs: u64,
    pub acceptance_false_pass_events: u64,
}

pub fn evaluate(raw: &Value, p0: &Value) -> Result<AcceptanceDecision, String> {
    let selected = &raw["raw_arm_matrix"]["arms"]["R2_SELECTED"]["lane_a"];
    let dense = &raw["raw_arm_matrix"]["arms"]["R1_DENSE_CONTROL"]["lane_a"];
    let tp = u64_field(selected, "lane_a_causal_tp")?;
    let fp = u64_field(selected, "lane_a_causal_fp")?;
    let fn_count = u64_field(selected, "lane_a_causal_fn")?;
    let dense_tp = u64_field(dense, "lane_a_causal_tp")?;
    let dense_fp = u64_field(dense, "lane_a_causal_fp")?;
    let dense_fn = u64_field(dense, "lane_a_causal_fn")?;
    let taxonomy = &selected["false_positive_causal_taxonomy"];
    let mediated = u64_field(taxonomy, "MEDIATED_ASSOCIATION")?;
    let indirect = u64_field(taxonomy, "INDIRECT_CAUSE")?;
    let common = u64_field(taxonomy, "COMMON_CAUSE")?;
    let matrix = &raw["raw_arm_matrix"];
    let attempts = u64_field(matrix, "negative_transfer_attempts")?;
    let prevented = u64_field(matrix, "negative_transfer_prevented")?;
    let accepted = u64_field(matrix, "negative_transfer_accepted")?;
    let p0_diff = u64_field(p0, "P0_SCIENTIFIC_ENGINE_DIFF")?;
    let recovered_diff = u64_field(p0, "RECOVERED_SHIFT_AWARE_SEMANTIC_DIFF")?;
    let pre_final_proven = bool_field(p0, "R1_SHIFT_AWARE_PRE_FINAL_FREEZE_PROVEN")?;
    let historical_authority = bool_field(raw, "historical_r1_final_c_final_authority")?;
    let manual_thresholds = u64_field(raw, "manual_precision_threshold_repair_events")?;
    let post_repairs = u64_field(raw, "post_final_repairs")?;
    let raw_authority = bool_field(selected, "raw_field_acceptance_authority")?;
    let direct_pass = mediated == 0 && indirect == 0 && tp > 0;
    let common_pass = common == 0;
    let lane_b_regression = u64::from(recovered_diff != 0 || !pre_final_proven);
    let lane_b_positive =
        lane_b_regression == 0 && attempts == 13 && prevented == 13 && accepted == 0;
    let precision_improved =
        (tp as u128 * (dense_tp + dense_fp) as u128) > (dense_tp as u128 * (tp + fp) as u128);
    let requirements = [
        p0_diff == 0,
        recovered_diff == 0,
        pre_final_proven,
        !historical_authority,
        manual_thresholds == 0,
        post_repairs == 0,
        raw_authority,
        direct_pass,
        common_pass,
        lane_b_positive,
        precision_improved,
        tp > 0,
    ];
    let pass = requirements.into_iter().all(|value| value);
    let disposition = if pass {
        "ACCEPTED_EXTERNAL_CAUSAL_STRUCTURE_PRECISION"
    } else if !direct_pass {
        "EXTERNAL_DIRECT_CAUSAL_STRUCTURE_PRECISION_LIMIT"
    } else if !common_pass {
        "EXTERNAL_COMMON_CAUSE_DISCRIMINATION_LIMIT"
    } else if !lane_b_positive {
        "EXTERNAL_SHIFT_AWARE_TRANSFER_GENERALIZATION_LIMIT"
    } else {
        "EXTERNAL_CAUSAL_PRECISION_ACCEPTANCE_LIMIT"
    };
    Ok(AcceptanceDecision {
        status: if pass { "PASS" } else { "FAIL" }.to_string(),
        disposition: disposition.to_string(),
        lane_a_tp: tp,
        lane_a_fp: fp,
        lane_a_fn: fn_count,
        r1_dense_lane_a_tp: dense_tp,
        r1_dense_lane_a_fp: dense_fp,
        r1_dense_lane_a_fn: dense_fn,
        mediated_false_edge_accepts: mediated,
        indirect_false_edge_accepts: indirect,
        common_cause_false_edge_accepts: common,
        negative_transfer_attempts: attempts,
        negative_transfer_prevented: prevented,
        negative_transfer_accepted: accepted,
        direct_indirect_causal_discrimination_pass: direct_pass,
        common_cause_discrimination_pass: common_pass,
        lane_b_modular_transfer_regression: lane_b_regression,
        lane_b_positive_regression_pass: lane_b_positive,
        exact_precision_strictly_improved_over_r1_dense: precision_improved,
        nonempty_direct_structure_retained: tp > 0,
        p0_scientific_engine_diff: p0_diff,
        recovered_shift_aware_semantic_diff: recovered_diff,
        r1_shift_aware_pre_final_freeze_proven: pre_final_proven,
        historical_r1_final_c_final_authority: historical_authority,
        manual_precision_threshold_repair_events: manual_thresholds,
        raw_field_acceptance_authority: raw_authority,
        post_final_repairs: post_repairs,
        acceptance_false_pass_events: 0,
    })
}

fn u64_field(value: &Value, field: &str) -> Result<u64, String> {
    value[field]
        .as_u64()
        .ok_or_else(|| format!("SEM37_R2_ACCEPTANCE_RAW_U64_MISSING:{field}"))
}

fn bool_field(value: &Value, field: &str) -> Result<bool, String> {
    value[field]
        .as_bool()
        .ok_or_else(|| format!("SEM37_R2_ACCEPTANCE_RAW_BOOL_MISSING:{field}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn mediated_false_accept_cannot_pass() {
        let raw = json!({
            "historical_r1_final_c_final_authority": false,
            "manual_precision_threshold_repair_events": 0,
            "post_final_repairs": 0,
            "raw_arm_matrix": {
                "negative_transfer_attempts": 13,
                "negative_transfer_prevented": 13,
                "negative_transfer_accepted": 0,
                "arms": {
                    "R2_SELECTED": {"lane_a": {
                        "lane_a_causal_tp": 8, "lane_a_causal_fp": 1, "lane_a_causal_fn": 2,
                        "false_positive_causal_taxonomy": {"MEDIATED_ASSOCIATION": 1, "INDIRECT_CAUSE": 0, "COMMON_CAUSE": 0},
                        "raw_field_acceptance_authority": true
                    }},
                    "R1_DENSE_CONTROL": {"lane_a": {
                        "lane_a_causal_tp": 10, "lane_a_causal_fp": 8, "lane_a_causal_fn": 0
                    }}
                }
            }
        });
        let p0 = json!({
            "P0_SCIENTIFIC_ENGINE_DIFF": 0,
            "RECOVERED_SHIFT_AWARE_SEMANTIC_DIFF": 0,
            "R1_SHIFT_AWARE_PRE_FINAL_FREEZE_PROVEN": true
        });
        let decision = evaluate(&raw, &p0).unwrap();
        assert_eq!(decision.status, "FAIL");
        assert!(!decision.direct_indirect_causal_discrimination_pass);
    }
}
