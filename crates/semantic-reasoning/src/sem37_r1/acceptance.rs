use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptanceDecision {
    pub status: &'static str,
    pub disposition: &'static str,
    pub levels: [bool; 8],
    pub mechanism_modularity_ablation_pass: bool,
    pub target_rebinding_ablation_pass: bool,
    pub transfer_gating_ablation_pass: bool,
    pub transfer_negative_memory_ablation_pass: bool,
    pub source_mechanism_transfer_ablation_pass: bool,
    pub shift_aware_counterfactual_transfer_pass: bool,
    pub modular_causal_transfer_observed: bool,
    pub violations: Vec<&'static str>,
}

pub fn evaluate_primary(
    development: &Value,
    policy_search: &Value,
    final_raw: &Value,
    internal_control_pass: bool,
) -> Result<AcceptanceDecision, String> {
    let arms = &final_raw["raw_arm_matrix"]["arms"];
    let no_change = &arms["NO_CHANGE"];
    let scratch = &arms["SCRATCH"];
    let naive = &arms["NAIVE_TRANSFER"];
    let shifted = &arms["SHIFT_AWARE_TRANSFER"];
    let shift_sse = sse(shifted)?;
    let no_change_sse = sse(no_change)?;
    let scratch_sse = sse(scratch)?;
    let naive_sse = sse(naive)?;
    let negative_accepted = raw_u64(&final_raw["raw_arm_matrix"], "negative_transfer_accepted")?;
    let negative_attempts = raw_u64(&final_raw["raw_arm_matrix"], "negative_transfer_attempts")?;
    let negative_prevented = raw_u64(&final_raw["raw_arm_matrix"], "negative_transfer_prevented")?;
    let promoted = raw_u64(final_raw, "lane_b_worlds")?.saturating_sub(negative_attempts);
    let structural_repair = f1_strictly_greater(&shifted["lane_a"], &naive["lane_a"])?
        && raw_u64(&shifted["lane_a"], "lane_a_causal_tp")? > 0;
    let work_advantage =
        raw_u64(shifted, "adaptation_work")? < raw_u64(scratch, "adaptation_work")?;
    let target_validity = shift_sse < naive_sse && shift_sse < scratch_sse;
    let conservative_gate = shift_sse < no_change_sse
        && negative_accepted == 0
        && raw_u64(
            &final_raw["raw_arm_matrix"],
            "promoted_target_transfer_worse_than_no_change_events",
        )? == 0;
    let diagnosis_pass = development["historical_failure_diagnosis"]
        .as_array()
        .is_some_and(|items| items.len() >= 2)
        && policy_search["ground_truth_edge_reads"].as_u64() == Some(0);
    let components = development["transfer_contract"]["components"]
        .as_array()
        .ok_or("SEM37_R1_TRANSFER_COMPONENTS_MISSING")?;
    let discrimination_pass = components
        .iter()
        .any(|item| item["state"].as_str() == Some("TRANSFERABLE"))
        && components
            .iter()
            .any(|item| item["state"].as_str() == Some("SHIFTED"));
    let modularity = shift_sse < naive_sse;
    let rebinding = shift_sse < naive_sse;
    let gating = conservative_gate && shift_sse < naive_sse;
    let negative_memory =
        negative_attempts > 0 && negative_prevented == negative_attempts && negative_accepted == 0;
    let source_ablation = target_validity && work_advantage;
    let levels = [
        diagnosis_pass,
        discrimination_pass,
        promoted > 0 && conservative_gate,
        conservative_gate,
        target_validity && work_advantage,
        target_validity && structural_repair,
        promoted > 0 && conservative_gate,
        modularity && rebinding && gating && negative_memory && source_ablation,
    ];
    let mut violations = Vec::new();
    if !structural_repair {
        violations.push("FRESH_LANE_A_STRUCTURE_PRECISION_NOT_IMPROVED_OVER_NAIVE_TRANSFER");
    }
    if !internal_control_pass {
        violations.push("INTERNAL_WORLD_CAPABILITY_REGRESSION");
    }
    if !conservative_gate {
        violations.push("NEGATIVE_TRANSFER_PROMOTION_GATE_FAILED");
    }
    let pass = levels.into_iter().all(|value| value) && internal_control_pass;
    Ok(AcceptanceDecision {
        status: if pass { "PASS" } else { "FAIL" },
        disposition: if pass {
            "SHIFT_AWARE_EXTERNAL_MECHANISM_TRANSFER"
        } else if !structural_repair {
            "EXTERNAL_CAUSAL_STRUCTURE_PRECISION_LIMIT"
        } else {
            "OTHER"
        },
        levels,
        mechanism_modularity_ablation_pass: modularity,
        target_rebinding_ablation_pass: rebinding,
        transfer_gating_ablation_pass: gating,
        transfer_negative_memory_ablation_pass: negative_memory,
        source_mechanism_transfer_ablation_pass: source_ablation,
        shift_aware_counterfactual_transfer_pass: promoted > 0 && conservative_gate,
        modular_causal_transfer_observed: promoted > 0
            && target_validity
            && work_advantage
            && conservative_gate,
        violations,
    })
}

/// Independent recomputation: intentionally does not call the primary path.
pub fn evaluate_secondary(
    development: &Value,
    policy_search: &Value,
    final_raw: &Value,
    internal_control_pass: bool,
) -> Result<Value, String> {
    let matrix = &final_raw["raw_arm_matrix"];
    let arms = &matrix["arms"];
    let shift = &arms["SHIFT_AWARE_TRANSFER"];
    let baseline = &arms["NO_CHANGE"];
    let scratch = &arms["SCRATCH"];
    let indivisible = &arms["NAIVE_TRANSFER"];
    let shift_error = sse(shift)?;
    let baseline_error = sse(baseline)?;
    let scratch_error = sse(scratch)?;
    let indivisible_error = sse(indivisible)?;
    let attempts = raw_u64(matrix, "negative_transfer_attempts")?;
    let prevented = raw_u64(matrix, "negative_transfer_prevented")?;
    let accepted = raw_u64(matrix, "negative_transfer_accepted")?;
    let promotions = raw_u64(final_raw, "lane_b_worlds")?.saturating_sub(attempts);
    let lane_a_gain = f1_strictly_greater(&shift["lane_a"], &indivisible["lane_a"])?;
    let context_states = development["transfer_contract"]["components"]
        .as_array()
        .ok_or("SEM37_R1_SECONDARY_COMPONENTS_MISSING")?;
    let levels = vec![
        development["historical_failure_diagnosis"]
            .as_array()
            .is_some_and(|items| items.len() >= 2)
            && policy_search["generator_source_reads"].as_u64() == Some(0),
        context_states
            .iter()
            .any(|item| item["state"].as_str() == Some("TRANSFERABLE"))
            && context_states
                .iter()
                .any(|item| item["state"].as_str() == Some("SHIFTED")),
        promotions > 0 && shift_error < baseline_error && accepted == 0,
        shift_error < baseline_error && accepted == 0,
        shift_error < scratch_error
            && raw_u64(shift, "adaptation_work")? < raw_u64(scratch, "adaptation_work")?,
        shift_error < scratch_error && shift_error < indivisible_error && lane_a_gain,
        promotions > 0 && shift_error < baseline_error && accepted == 0,
        shift_error < indivisible_error
            && shift_error < scratch_error
            && attempts > 0
            && attempts == prevented
            && accepted == 0,
    ];
    let status = if levels.iter().all(|value| *value) && internal_control_pass {
        "PASS"
    } else {
        "FAIL"
    };
    Ok(json!({
        "schema_version": "SEM37_R1_SECONDARY_ACCEPTANCE_1",
        "sem37_r1_status": status,
        "disposition": if lane_a_gain {"OTHER"} else {"EXTERNAL_CAUSAL_STRUCTURE_PRECISION_LIMIT"},
        "levels": levels,
        "raw_field_acceptance_authority": true,
        "derived_ratio_float_is_acceptance_authority": false,
        "global_float_epsilon_acceptance_rule": false
    }))
}

fn sse(arm: &Value) -> Result<f64, String> {
    let bits = raw_u64(&arm["lane_b"], "prediction_sse_ieee754_bits")?;
    let value = f64::from_bits(bits);
    value
        .is_finite()
        .then_some(value)
        .ok_or("SEM37_R1_NONFINITE_SSE".to_string())
}

fn f1_strictly_greater(left: &Value, right: &Value) -> Result<bool, String> {
    let left_tp = raw_u64(left, "lane_a_causal_tp")? as u128;
    let left_denominator = 2 * left_tp
        + raw_u64(left, "lane_a_causal_fp")? as u128
        + raw_u64(left, "lane_a_causal_fn")? as u128;
    let right_tp = raw_u64(right, "lane_a_causal_tp")? as u128;
    let right_denominator = 2 * right_tp
        + raw_u64(right, "lane_a_causal_fp")? as u128
        + raw_u64(right, "lane_a_causal_fn")? as u128;
    Ok(2 * left_tp * right_denominator > 2 * right_tp * left_denominator)
}

fn raw_u64(value: &Value, field: &str) -> Result<u64, String> {
    value[field]
        .as_u64()
        .ok_or_else(|| format!("SEM37_R1_RAW_FIELD_MISSING:{field}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_f1_comparison_does_not_use_float_epsilon() {
        let equal_left =
            json!({"lane_a_causal_tp": 38, "lane_a_causal_fp": 104, "lane_a_causal_fn": 0});
        let equal_right =
            json!({"lane_a_causal_tp": 38, "lane_a_causal_fp": 104, "lane_a_causal_fn": 0});
        assert!(!f1_strictly_greater(&equal_left, &equal_right).unwrap());
    }
}
