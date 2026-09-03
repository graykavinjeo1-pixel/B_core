use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, LanguageCodeIR, CONVERSATION_TURN_REQUEST_SCHEMA,
    CONVERSATION_TURN_RESPONSE_SCHEMA, LANGUAGE_CORTEX_RESPONSE_INTEGRATION_SCHEMA,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Copy)]
pub struct Turn {
    pub text: &'static str,
    pub language: LanguageCodeIR,
}

pub struct Case {
    pub id: &'static str,
    pub category: &'static str,
    pub turns: &'static [Turn],
    pub expected_disposition: ConversationTurnDispositionIR,
}

#[derive(Serialize)]
struct Row {
    id: String,
    category: String,
    response_schema: String,
    integration_schema: String,
    turn_count: usize,
    state_aligned: bool,
    embedded_hashes_bound: bool,
    complete: bool,
    authority_violation: bool,
    unsupported_explanation_facts: usize,
    pass: bool,
}

#[derive(Serialize)]
struct Summary {
    schema: &'static str,
    suite: &'static str,
    total: usize,
    passed: usize,
    failed: usize,
    external_llm_calls: usize,
    local_teacher_calls: usize,
    network_calls: usize,
    recursive_source_mutations: usize,
    rows: Vec<Row>,
}

fn request(id: &str, turn: u64, spec: Turn) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: id.to_string(),
        turn_index: turn,
        request_id: format!("{id}-{turn}"),
        modality: ConversationInputModalityIR::Text,
        raw_text: spec.text.to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(spec.language),
        context_tags: Vec::new(),
        max_plan_steps: 16,
    }
}

fn digest(value: Option<&Value>) -> Option<&str> {
    value
        .and_then(Value::as_str)
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn same_digest(integration: &Value, response: &Value, binding: &str, path: &[&str]) -> bool {
    let mut value = response;
    for segment in path {
        let Some(next) = value.get(*segment) else {
            return false;
        };
        value = next;
    }
    digest(integration.get(binding)) == digest(Some(value))
}

pub fn emit(suite: &'static str, cases: &[Case]) {
    let mut rows = Vec::new();
    for case in cases {
        let mut api = CognitiveApi::new_embedded().expect("embedded core");
        let mut response_value = None;
        let mut response_disposition = ConversationTurnDispositionIR::Grounded;
        for (index, turn) in case.turns.iter().copied().enumerate() {
            match api.process_conversation_turn(&request(
                case.id,
                u64::try_from(index + 1).expect("bounded turn"),
                turn,
            )) {
                Ok(response) => {
                    response_disposition = response.disposition;
                    response_value = Some(serde_json::to_value(response).expect("response json"));
                }
                Err(_) => {
                    response_value = None;
                    break;
                }
            }
        }
        let Some(response) = response_value else {
            rows.push(Row {
                id: case.id.to_string(),
                category: case.category.to_string(),
                response_schema: "MISSING".to_string(),
                integration_schema: "MISSING".to_string(),
                turn_count: case.turns.len(),
                state_aligned: false,
                embedded_hashes_bound: false,
                complete: false,
                authority_violation: false,
                unsupported_explanation_facts: 0,
                pass: false,
            });
            continue;
        };
        let integration = response
            .get("language_cortex_integration")
            .cloned()
            .unwrap_or(Value::Null);
        let response_schema = response
            .get("schema")
            .and_then(Value::as_str)
            .unwrap_or("MISSING")
            .to_string();
        let integration_schema = integration
            .get("schema")
            .and_then(Value::as_str)
            .unwrap_or("MISSING")
            .to_string();
        let final_turn = u64::try_from(case.turns.len()).expect("bounded turn count");
        let state_aligned = integration.get("conversation_id").and_then(Value::as_str)
            == Some(case.id)
            && integration.get("request_id").and_then(Value::as_str)
                == Some(format!("{}-{final_turn}", case.id).as_str())
            && integration.get("turn_index").and_then(Value::as_u64) == Some(final_turn)
            && response
                .get("conversation_state")
                .and_then(|state| state.get("completed_turns"))
                .and_then(Value::as_u64)
                == Some(final_turn)
            && response
                .get("pragmatic_state")
                .and_then(|state| state.get("completed_turns"))
                .and_then(Value::as_u64)
                == Some(final_turn);
        let embedded_hashes_bound = same_digest(
            &integration,
            &response,
            "definition_grounding_sha256",
            &["definition_grounding", "grounding_sha256"],
        ) && same_digest(
            &integration,
            &response,
            "pragmatic_state_sha256",
            &["pragmatic_state", "state_sha256"],
        ) && same_digest(
            &integration,
            &response,
            "conversation_state_sha256",
            &["conversation_state", "state_sha256"],
        ) && same_digest(
            &integration,
            &response,
            "grounded_realization_sha256",
            &["grounded_realization", "realization_sha256"],
        ) && same_digest(
            &integration,
            &response,
            "interaction_provenance_sha256",
            &["interaction_provenance", "graph_sha256"],
        ) && same_digest(
            &integration,
            &response,
            "six_axis_integration_sha256",
            &["six_axis_integration", "integration_sha256"],
        ) && digest(integration.get("request_sha256")).is_some()
            && digest(integration.get("normalization_sha256")).is_some()
            && digest(integration.get("reference_resolution_sha256")).is_some()
            && digest(integration.get("pragmatic_interpretation_sha256")).is_some()
            && digest(integration.get("action_state_analysis_sha256")).is_some()
            && digest(integration.get("discourse_outputs_sha256")).is_some()
            && digest(integration.get("output_sha256")).is_some()
            && digest(integration.get("response_payload_sha256")).is_some()
            && digest(integration.get("integration_sha256")).is_some();
        let complete = integration
            .get("complete")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let authority_violation = integration
            .get("semantic_authority")
            .and_then(Value::as_bool)
            .unwrap_or(true)
            || integration
                .get("language_can_execute")
                .and_then(Value::as_bool)
                .unwrap_or(true)
            || integration
                .get("external_action_executed")
                .and_then(Value::as_bool)
                .unwrap_or(true);
        let unsupported_explanation_facts = integration
            .get("unsupported_explanation_facts")
            .and_then(Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(usize::MAX);
        let zero_dependencies = [
            "external_llm_calls",
            "local_teacher_calls",
            "network_calls",
            "recursive_source_mutations",
        ]
        .iter()
        .all(|field| integration.get(*field).and_then(Value::as_u64) == Some(0));
        let pass = response_schema == CONVERSATION_TURN_RESPONSE_SCHEMA
            && integration_schema == LANGUAGE_CORTEX_RESPONSE_INTEGRATION_SCHEMA
            && response_disposition == case.expected_disposition
            && state_aligned
            && embedded_hashes_bound
            && complete
            && !authority_violation
            && unsupported_explanation_facts == 0
            && zero_dependencies;
        rows.push(Row {
            id: case.id.to_string(),
            category: case.category.to_string(),
            response_schema,
            integration_schema,
            turn_count: case.turns.len(),
            state_aligned,
            embedded_hashes_bound,
            complete,
            authority_violation,
            unsupported_explanation_facts,
            pass,
        });
    }
    let passed = rows.iter().filter(|row| row.pass).count();
    let total = rows.len();
    println!(
        "{}",
        serde_json::to_string(&Summary {
            schema: "B_CORE_R50_LANGUAGE_CORTEX_INTEGRATION_CANARY_1",
            suite,
            total,
            passed,
            failed: total - passed,
            external_llm_calls: 0,
            local_teacher_calls: 0,
            network_calls: 0,
            recursive_source_mutations: 0,
            rows,
        })
        .expect("summary json")
    );
    if passed != total {
        std::process::exit(1);
    }
}
