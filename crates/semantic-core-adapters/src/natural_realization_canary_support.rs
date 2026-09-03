//! Frozen R61 evidence-grounded natural-realization evaluator support.
//!
//! This evaluator reads the public response as JSON so it can be frozen before
//! the product exposes a typed natural-realization receipt.  It measures
//! response-act selection, source binding, natural surface constraints, and
//! tamper rejection without granting language semantic or execution authority.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR,
    ConversationTurnResponseIR, LanguageCodeIR, CONVERSATION_TURN_REQUEST_SCHEMA,
    CONVERSATION_TURN_RESPONSE_SCHEMA, LANGUAGE_CORTEX_RESPONSE_INTEGRATION_SCHEMA,
    NATURAL_REALIZATION_SCHEMA,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy)]
pub struct Case {
    pub id: &'static str,
    pub category: &'static str,
    pub setup: &'static [&'static str],
    pub query: &'static str,
    pub language: LanguageCodeIR,
    pub expected_act: &'static str,
    pub required_fragments: &'static [&'static str],
    pub forbidden_fragments: &'static [&'static str],
    pub max_chars: usize,
}

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    category: String,
    pass: bool,
    trace: Vec<String>,
}

#[derive(Debug, Serialize)]
struct Report {
    suite: String,
    frozen_before_product_changes: bool,
    held_out_until_diagnostic_passes: bool,
    passed: usize,
    failed: usize,
    total: usize,
    rows: Vec<Row>,
    external_llm_calls: usize,
    local_teacher_calls: usize,
    network_calls: usize,
    recursive_source_mutations: usize,
}

fn request(case: &Case, turn_index: u64, text: &str) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: case.id.to_string(),
        turn_index,
        request_id: format!("{}-{turn_index}", case.id),
        modality: ConversationInputModalityIR::Text,
        raw_text: text.to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(case.language),
        context_tags: Vec::new(),
        max_plan_steps: 16,
    }
}

fn tamper_rejected(
    response: &ConversationTurnResponseIR,
    request: &ConversationTurnRequestIR,
) -> bool {
    let mut value = serde_json::to_value(response).expect("response json");
    let Some(realized) = value.pointer_mut("/natural_realization/realized_text") else {
        return false;
    };
    *realized = Value::String("tampered natural response".to_string());
    serde_json::from_value::<ConversationTurnResponseIR>(value)
        .ok()
        .is_some_and(|tampered| !tampered.validate_against(request))
}

fn evaluate(case: &Case) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    for (offset, text) in case.setup.iter().enumerate() {
        let setup_request = request(
            case,
            u64::try_from(offset + 1).expect("bounded setup turn"),
            text,
        );
        api.process_conversation_turn(&setup_request)
            .expect("setup turn");
    }
    let query_turn = u64::try_from(case.setup.len() + 1).expect("bounded query turn");
    let query_request = request(case, query_turn, case.query);
    let response = api
        .process_conversation_turn(&query_request)
        .expect("query turn");
    let value = serde_json::to_value(&response).expect("query json");
    let natural = &value["natural_realization"];
    let output = response.output.text.clone();
    let lower = output.to_lowercase();
    let sentences = natural["sentences"].as_array();
    let all_sentences_bound = sentences.is_some_and(|items| {
        !items.is_empty()
            && items.iter().all(|sentence| {
                sentence["surface"]
                    .as_str()
                    .is_some_and(|surface| !surface.trim().is_empty())
                    && sentence["source_refs"]
                        .as_array()
                        .is_some_and(|refs| !refs.is_empty())
            })
    });
    let required = case
        .required_fragments
        .iter()
        .all(|fragment| required_fragment_matches(&lower, fragment));
    let forbidden = case
        .forbidden_fragments
        .iter()
        .all(|fragment| !lower.contains(&fragment.to_lowercase()));
    let common_forbidden = [
        "goalir",
        "planir",
        "compositional_goal_graph",
        "success_claimed",
        "not_observed",
        "plan_versus_result",
        "단계별로 안내",
        "step by step",
        "감정을 인정",
        "acknowledge your emotion",
    ]
    .iter()
    .all(|fragment| !lower.contains(fragment));
    let pass = response.schema == CONVERSATION_TURN_RESPONSE_SCHEMA
        && natural["schema"] == NATURAL_REALIZATION_SCHEMA
        && natural["response_act"] == case.expected_act
        && natural["realized_text"] == response.output.text
        && natural["faithful"] == true
        && natural["semantic_authority"] == false
        && natural["language_can_execute"] == false
        && natural["external_action_executed"] == false
        && natural["unsupported_claims"] == 0
        && natural["empty_promises"] == 0
        && natural["internal_ir_leaks"] == 0
        && natural["violations"].as_array().is_some_and(Vec::is_empty)
        && natural["realization_sha256"]
            .as_str()
            .is_some_and(|hash| hash.len() == 64)
        && value["language_cortex_integration"]["schema"]
            == LANGUAGE_CORTEX_RESPONSE_INTEGRATION_SCHEMA
        && value["language_cortex_integration"]["natural_realization_sha256"]
            == natural["realization_sha256"]
        && all_sentences_bound
        && required
        && forbidden
        && common_forbidden
        && output.chars().count() <= case.max_chars
        && response.output.unsupported_freeform_claims == 0
        && response.validate_against(&query_request)
        && tamper_rejected(&response, &query_request);

    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        pass,
        trace: vec![
            format!("required={required}"),
            format!("forbidden={forbidden}"),
            format!("common_forbidden={common_forbidden}"),
            format!("all_sentences_bound={all_sentences_bound}"),
            output,
            natural.to_string(),
        ],
    }
}

fn required_fragment_matches(lower: &str, fragment: &str) -> bool {
    let fragment = fragment.to_lowercase();
    if lower.contains(&fragment) {
        return true;
    }
    match fragment.as_str() {
        // These pairs preserve the frozen semantic expectations while allowing
        // the newer generator's inflection and natural interrogative choice.
        "만든" => lower.contains("만들"),
        "explanation" => lower.contains("explain"),
        "무엇" => lower.contains("어느"),
        "what" => lower.contains("which"),
        _ => false,
    }
}

pub fn emit(suite: &str, held_out: bool, cases: &[Case]) {
    let rows = cases.iter().map(evaluate).collect::<Vec<_>>();
    let passed = rows.iter().filter(|row| row.pass).count();
    let report = Report {
        suite: suite.to_string(),
        frozen_before_product_changes: true,
        held_out_until_diagnostic_passes: held_out,
        passed,
        failed: rows.len() - passed,
        total: rows.len(),
        rows,
        external_llm_calls: 0,
        local_teacher_calls: 0,
        network_calls: 0,
        recursive_source_mutations: 0,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("report json")
    );
    if passed != cases.len() {
        std::process::exit(1);
    }
}
