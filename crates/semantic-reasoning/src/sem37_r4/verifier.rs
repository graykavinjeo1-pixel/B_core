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
    final_manifest: &Value,
) -> Result<Value, String> {
    let decision = acceptance::evaluate(
        final_evaluation,
        development,
        p0,
        final_freeze,
        final_manifest,
    )?;
    Ok(json!({
        "schema_version": "SEM37_R4_INDEPENDENT_SECONDARY_ACCEPTANCE_1",
        "status": decision.status,
        "disposition": decision.disposition,
        "r4_direct_tp": decision.r4_direct_tp,
        "r4_direct_fp": decision.r4_direct_fp,
        "r4_direct_fn": decision.r4_direct_fn,
        "r4_mediator_as_direct": decision.r4_mediator_as_direct,
        "negative_transfer_accepted": decision.negative_transfer_accepted,
        "positive_transfer_verified": decision.positive_transfer_verified,
        "ambiguous_transfer_abstentions": decision.ambiguous_transfer_abstentions,
        "levels": [
            decision.level_a_pass, decision.level_b_pass, decision.level_c_pass,
            decision.level_d_pass, decision.level_e_pass, decision.level_f_pass,
            decision.level_g_pass, decision.level_h_pass
        ],
        "bc_core_self_asserted_causal_success_events": 0,
        "bc_core_self_asserted_transfer_success_events": 0
    }))
}
