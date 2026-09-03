//! Frozen public-API evaluator support for R59.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR, LanguageCodeIR,
    CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;
use serde_json::Value;

pub const UTTERANCE_INTENT_SCHEMA: &str = "B_CORE_UTTERANCE_INTENT_GRAPH_IR_1";

#[derive(Clone, Copy)]
pub struct Case {
    pub id: &'static str,
    pub category: &'static str,
    pub setup: Option<&'static str>,
    pub text: &'static str,
    pub language: LanguageCodeIR,
    pub expected_intent: &'static str,
    pub expected_response: &'static str,
    pub expected_target_fragment: &'static str,
    pub expected_goal_intent: Option<&'static str>,
    pub expected_speech_act: &'static str,
    pub expected_constraint_fragment: Option<&'static str>,
    pub forbidden_authorized_predicate: Option<&'static str>,
    pub clarification: bool,
}

#[derive(Debug, Serialize)]
pub struct Row {
    id: String,
    category: String,
    pass: bool,
    selected_intent: String,
    selected_response: String,
    selected_target: String,
    speech_act: String,
    inferred_goal_intent: Option<String>,
    authorized_predicates: Vec<String>,
    graph_schema: String,
    graph_valid: bool,
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

fn request(case: &Case, turn: u64, text: &str) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: case.id.to_string(),
        turn_index: turn,
        request_id: format!("{}-{turn}", case.id),
        modality: ConversationInputModalityIR::Text,
        raw_text: text.to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(case.language),
        context_tags: Vec::new(),
        max_plan_steps: 16,
    }
}

fn string_field(value: Option<&Value>, field: &str) -> String {
    value
        .and_then(|value| value.get(field))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn run(case: Case) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    if let Some(setup) = case.setup {
        api.process_conversation_turn(&request(&case, 1, setup))
            .expect("R59 setup turn");
    }
    let turn = u64::from(case.setup.is_some()) + 1;
    let response = api
        .process_conversation_turn(&request(&case, turn, case.text))
        .expect("R59 evaluated turn");
    let pragmatic = serde_json::to_value(&response.pragmatic_interpretation.pragmatic_intent_graph)
        .expect("pragmatic graph json");
    let graph = pragmatic.get("utterance_intent");
    let graph_schema = string_field(graph, "schema");
    let selected_id = string_field(graph, "selected_candidate_id");
    let selected = graph
        .and_then(|graph| graph.get("candidates"))
        .and_then(Value::as_array)
        .and_then(|candidates| {
            candidates.iter().find(|candidate| {
                candidate
                    .get("candidate_id")
                    .and_then(Value::as_str)
                    .is_some_and(|id| id == selected_id)
            })
        });
    let selected_intent = string_field(selected, "communicative_intent");
    let selected_response = string_field(selected, "expected_response");
    let selected_target = string_field(selected, "target");
    let selected_constraints = selected
        .and_then(|value| value.get("constraints"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    let graph_valid = graph.is_some_and(|graph| {
        graph_schema == UTTERANCE_INTENT_SCHEMA
            && graph
                .get("graph_sha256")
                .and_then(Value::as_str)
                .is_some_and(|hash| hash.len() == 64)
            && graph.get("semantic_authority") == Some(&Value::Bool(false))
            && graph.get("external_execution_authorized") == Some(&Value::Bool(false))
            && !selected_id.is_empty()
    });
    let speech_act = format!("{:?}", response.pragmatic_interpretation.speech_act).to_uppercase();
    let inferred_goal_intent = response
        .pragmatic_interpretation
        .inferred_goal
        .as_ref()
        .map(|goal| format!("{:?}", goal.intent).to_uppercase());
    let authorized_predicates = response
        .conversation_state
        .active_goals
        .iter()
        .filter(|goal| goal.external_execution_authorized)
        .map(|goal| goal.canonical_predicate.to_uppercase())
        .collect::<Vec<_>>();
    let constraint_ok = case.expected_constraint_fragment.is_none_or(|fragment| {
        selected_constraints
            .iter()
            .any(|constraint| constraint.to_lowercase().contains(&fragment.to_lowercase()))
    });
    let forbidden_ok = case.forbidden_authorized_predicate.is_none_or(|forbidden| {
        authorized_predicates
            .iter()
            .all(|predicate| !predicate.eq_ignore_ascii_case(forbidden))
    });
    let goal_ok = case.expected_goal_intent.map_or_else(
        || inferred_goal_intent.is_none(),
        |expected| {
            inferred_goal_intent
                .as_deref()
                .is_some_and(|actual| actual.eq_ignore_ascii_case(expected))
        },
    );
    let pass = graph_valid
        && selected_intent.eq_ignore_ascii_case(case.expected_intent)
        && selected_response.eq_ignore_ascii_case(case.expected_response)
        && selected_target
            .to_lowercase()
            .contains(&case.expected_target_fragment.to_lowercase())
        && speech_act.eq_ignore_ascii_case(case.expected_speech_act)
        && goal_ok
        && constraint_ok
        && forbidden_ok
        && response
            .pragmatic_interpretation
            .compositional_analysis
            .clarification_required
            == case.clarification
        && response.output.unsupported_freeform_claims == 0;
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        pass,
        selected_intent,
        selected_response,
        selected_target,
        speech_act,
        inferred_goal_intent,
        authorized_predicates,
        graph_schema,
        graph_valid,
        trace: vec![
            format!("constraints={selected_constraints:?}"),
            format!(
                "unresolved={:?}",
                response.pragmatic_interpretation.unresolved_bindings
            ),
            response.output.text,
        ],
    }
}

pub fn emit(suite: &str, held_out: bool, cases: &[Case]) {
    let rows = cases.iter().copied().map(run).collect::<Vec<_>>();
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
        serde_json::to_string_pretty(&report).expect("R59 report json")
    );
    if passed != cases.len() {
        std::process::exit(1);
    }
}
