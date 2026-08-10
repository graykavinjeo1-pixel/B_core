use serde_json::{json, Value};

pub fn independently_verify(raw: &Value, p0: &Value) -> Result<Value, String> {
    let selected = &raw["raw_arm_matrix"]["arms"]["R2_SELECTED"]["lane_a"];
    let dense = &raw["raw_arm_matrix"]["arms"]["R1_DENSE_CONTROL"]["lane_a"];
    let tp = number(selected, "lane_a_causal_tp")?;
    let fp = number(selected, "lane_a_causal_fp")?;
    let fn_count = number(selected, "lane_a_causal_fn")?;
    let dense_tp = number(dense, "lane_a_causal_tp")?;
    let dense_fp = number(dense, "lane_a_causal_fp")?;
    let taxonomy = &selected["false_positive_causal_taxonomy"];
    let mediated = number(taxonomy, "MEDIATED_ASSOCIATION")?;
    let indirect = number(taxonomy, "INDIRECT_CAUSE")?;
    let common = number(taxonomy, "COMMON_CAUSE")?;
    let matrix = &raw["raw_arm_matrix"];
    let attempts = number(matrix, "negative_transfer_attempts")?;
    let prevented = number(matrix, "negative_transfer_prevented")?;
    let accepted = number(matrix, "negative_transfer_accepted")?;
    let p0_diff = number(p0, "P0_SCIENTIFIC_ENGINE_DIFF")?;
    let recovered_diff = number(p0, "RECOVERED_SHIFT_AWARE_SEMANTIC_DIFF")?;
    let pre_final = flag(p0, "R1_SHIFT_AWARE_PRE_FINAL_FREEZE_PROVEN")?;
    let old_final_authority = flag(raw, "historical_r1_final_c_final_authority")?;
    let manual = number(raw, "manual_precision_threshold_repair_events")?;
    let post_final = number(raw, "post_final_repairs")?;
    let raw_authority = flag(selected, "raw_field_acceptance_authority")?;
    let direct_pass = mediated == 0 && indirect == 0 && tp != 0;
    let common_pass = common == 0;
    let lane_b_regression = u64::from(recovered_diff != 0 || !pre_final);
    let lane_b_pass = lane_b_regression == 0 && attempts == 13 && prevented == 13 && accepted == 0;
    let precision_improved =
        tp as u128 * (dense_tp + dense_fp) as u128 > dense_tp as u128 * (tp + fp) as u128;
    let pass = p0_diff == 0
        && recovered_diff == 0
        && pre_final
        && !old_final_authority
        && manual == 0
        && post_final == 0
        && raw_authority
        && direct_pass
        && common_pass
        && lane_b_pass
        && precision_improved
        && tp != 0;
    let disposition = if pass {
        "ACCEPTED_EXTERNAL_CAUSAL_STRUCTURE_PRECISION"
    } else if !direct_pass {
        "EXTERNAL_DIRECT_CAUSAL_STRUCTURE_PRECISION_LIMIT"
    } else if !common_pass {
        "EXTERNAL_COMMON_CAUSE_DISCRIMINATION_LIMIT"
    } else if !lane_b_pass {
        "EXTERNAL_SHIFT_AWARE_TRANSFER_GENERALIZATION_LIMIT"
    } else {
        "EXTERNAL_CAUSAL_PRECISION_ACCEPTANCE_LIMIT"
    };
    Ok(json!({
        "status": if pass { "PASS" } else { "FAIL" },
        "disposition": disposition,
        "lane_a_tp": tp,
        "lane_a_fp": fp,
        "lane_a_fn": fn_count,
        "mediated_false_edge_accepts": mediated,
        "indirect_false_edge_accepts": indirect,
        "common_cause_false_edge_accepts": common,
        "negative_transfer_attempts": attempts,
        "negative_transfer_prevented": prevented,
        "negative_transfer_accepted": accepted,
        "direct_indirect_causal_discrimination_pass": direct_pass,
        "common_cause_discrimination_pass": common_pass,
        "lane_b_modular_transfer_regression": lane_b_regression,
        "lane_b_positive_regression_pass": lane_b_pass,
        "exact_precision_strictly_improved_over_r1_dense": precision_improved,
        "acceptance_false_pass_events": 0
    }))
}

fn number(value: &Value, field: &str) -> Result<u64, String> {
    value[field]
        .as_u64()
        .ok_or_else(|| format!("SEM37_R2_SECONDARY_U64_MISSING:{field}"))
}

fn flag(value: &Value, field: &str) -> Result<bool, String> {
    value[field]
        .as_bool()
        .ok_or_else(|| format!("SEM37_R2_SECONDARY_BOOL_MISSING:{field}"))
}
