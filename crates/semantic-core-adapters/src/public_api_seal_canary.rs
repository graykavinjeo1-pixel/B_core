//! Frozen R63 public API, serialization, and tamper-resistance canary.

use semantic_core_adapters::{
    language_cortex_package_boundary, CognitiveApi, ConversationInputModalityIR,
    ConversationTurnRequestIR, ConversationTurnResponseIR, LanguageCodeIR,
    CONVERSATION_TURN_REQUEST_SCHEMA, CONVERSATION_TURN_RESPONSE_SCHEMA,
    LANGUAGE_CORTEX_RESPONSE_INTEGRATION_SCHEMA, SIX_AXIS_INTEGRATION_SCHEMA,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
struct ApiSealReport {
    suite: &'static str,
    schemas_exact: bool,
    public_request_response_round_trip: bool,
    live_response_validation: bool,
    output_tamper_rejected: bool,
    integration_hash_tamper_rejected: bool,
    package_boundary_valid: bool,
    rust_only_default: bool,
    passed: usize,
    failed: usize,
    external_llm_calls: usize,
    local_teacher_calls: usize,
    network_calls: usize,
    python_calls: usize,
    recursive_source_mutations: usize,
}

fn main() {
    let request = ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: "R63-PUBLIC-API-SEAL".to_string(),
        turn_index: 1,
        request_id: "R63-PUBLIC-API-SEAL-1".to_string(),
        modality: ConversationInputModalityIR::Text,
        raw_text: "Inspect the Marigold cache, but do not delete the Navy log".to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(LanguageCodeIR::English),
        context_tags: Vec::new(),
        max_plan_steps: 16,
    };
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let response = api
        .process_conversation_turn(&request)
        .expect("public conversation API");
    let encoded = serde_json::to_vec(&response).expect("serialize public response");
    let decoded: ConversationTurnResponseIR =
        serde_json::from_slice(&encoded).expect("deserialize public response");
    let round_trip = decoded == response;
    let live_valid = decoded.validate_against(&request);

    let mut output_tamper: Value = serde_json::from_slice(&encoded).expect("response value");
    output_tamper["output"]["text"] = Value::String("The deletion completed.".to_string());
    let output_tamper_rejected =
        serde_json::from_value::<ConversationTurnResponseIR>(output_tamper)
            .is_ok_and(|candidate| !candidate.validate_against(&request));

    let mut hash_tamper: Value = serde_json::from_slice(&encoded).expect("response value");
    hash_tamper["six_axis_integration"]["integration_sha256"] = Value::String("0".repeat(64));
    let integration_hash_tamper_rejected =
        serde_json::from_value::<ConversationTurnResponseIR>(hash_tamper)
            .is_ok_and(|candidate| !candidate.validate_against(&request));

    let boundary = language_cortex_package_boundary();
    let schemas_exact = CONVERSATION_TURN_RESPONSE_SCHEMA == "B_CORE_CONVERSATION_TURN_RESPONSE_18"
        && LANGUAGE_CORTEX_RESPONSE_INTEGRATION_SCHEMA
            == "B_CORE_LANGUAGE_CORTEX_RESPONSE_INTEGRATION_IR_5"
        && SIX_AXIS_INTEGRATION_SCHEMA == "B_CORE_SIX_AXIS_INTEGRATION_IR_2";
    let boundary_valid = boundary.validate()
        && boundary.dependency_direction == "LANGUAGE_ADAPTER_TO_SEMANTIC_CORE_ONLY"
        && !boundary.raw_language_reaches_core
        && !boundary.adapter_owns_semantic_state
        && !boundary.semantic_authority
        && !boundary.external_action_execution_authority;
    let rust_only = boundary.external_llm_calls == 0
        && boundary.local_teacher_calls == 0
        && boundary.network_calls == 0
        && boundary.recursive_source_mutations == 0;
    let checks = [
        schemas_exact,
        round_trip,
        live_valid,
        output_tamper_rejected,
        integration_hash_tamper_rejected,
        boundary_valid,
        rust_only,
    ];
    let passed = checks.iter().filter(|value| **value).count();
    let report = ApiSealReport {
        suite: "R63-PUBLIC-API-SEAL",
        schemas_exact,
        public_request_response_round_trip: round_trip,
        live_response_validation: live_valid,
        output_tamper_rejected,
        integration_hash_tamper_rejected,
        package_boundary_valid: boundary_valid,
        rust_only_default: rust_only,
        passed,
        failed: checks.len() - passed,
        external_llm_calls: 0,
        local_teacher_calls: 0,
        network_calls: 0,
        python_calls: 0,
        recursive_source_mutations: 0,
    };
    println!("{}", serde_json::to_string_pretty(&report).expect("report"));
    if report.failed != 0 {
        std::process::exit(1);
    }
}
