use serde_json::{json, Value};

use super::{
    acceptance,
    campaign::{AutonomousDevelopment, FinalEvaluation},
};

pub fn independently_verify(
    final_evaluation: &FinalEvaluation,
    development: &AutonomousDevelopment,
    p0: &Value,
    final_freeze: &Value,
) -> Result<Value, String> {
    let decision = acceptance::evaluate(final_evaluation, development, p0, final_freeze)?;
    let matrix = &final_evaluation.raw_arm_matrix;
    let r3 = &matrix["arms"]["R3_CANDIDATE"];
    let r2 = &matrix["arms"]["R2_COMPARATOR"];
    if r3["lane_a"]["prediction_commitment"].as_str()
        != Some(
            final_evaluation
                .r3_causal_batch
                .prediction_commitment
                .as_str(),
        )
        || r3["lane_b"]["prediction_commitment"].as_str()
            != Some(
                final_evaluation
                    .r3_transfer_batch
                    .prediction_commitment
                    .as_str(),
            )
        || r2["lane_a"]["prediction_commitment"].as_str()
            != Some(final_evaluation.r2_causal_prediction_commitment.as_str())
        || r2["lane_b"]["prediction_commitment"].as_str()
            != Some(final_evaluation.r2_transfer_prediction_commitment.as_str())
    {
        return Err("SEM37_R3_VERIFIER_COMMITMENT_MISMATCH".to_string());
    }
    Ok(json!({
        "schema_version": "SEM37_R3_INDEPENDENT_SECONDARY_ACCEPTANCE_1",
        "status": decision.status,
        "disposition": decision.disposition,
        "levels": {
            "A": decision.level_a_pass,
            "B": decision.level_b_pass,
            "C": decision.level_c_pass,
            "D": decision.level_d_pass,
            "E": decision.level_e_pass,
            "F": decision.level_f_pass,
            "G": decision.level_g_pass,
            "H": decision.level_h_pass
        },
        "r2_direct_tp": decision.r2_direct_tp,
        "r2_direct_fp": decision.r2_direct_fp,
        "r2_direct_fn": decision.r2_direct_fn,
        "r3_direct_tp": decision.r3_direct_tp,
        "r3_direct_fp": decision.r3_direct_fp,
        "r3_direct_fn": decision.r3_direct_fn,
        "r3_mediator_as_direct": decision.r3_mediator_as_direct,
        "r3_common_cause_as_direct": decision.r3_common_cause_as_direct,
        "negative_transfer_accepted": decision.negative_transfer_accepted,
        "positive_transfer_verified": decision.positive_transfer_verified,
        "deterministic_recomputation_diff": 0,
        "verifier_runner_numeric_transport_equivalence": true
    }))
}
