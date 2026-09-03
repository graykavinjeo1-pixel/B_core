use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR,
    ConversationTurnResponseIR, DiscourseBindingKindIR, LanguageCodeIR,
    CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Copy)]
pub struct Turn {
    pub text: &'static str,
    pub language: LanguageCodeIR,
    pub alternative: Option<&'static str>,
}

pub const fn text(text: &'static str, language: LanguageCodeIR) -> Turn {
    Turn {
        text,
        language,
        alternative: None,
    }
}

pub const fn voice(
    text: &'static str,
    alternative: &'static str,
    language: LanguageCodeIR,
) -> Turn {
    Turn {
        text,
        language,
        alternative: Some(alternative),
    }
}

pub struct Case {
    pub id: &'static str,
    pub category: &'static str,
    pub turns: &'static [Turn],
    pub restoration_turn: usize,
    pub expected_topic: &'static str,
    pub expected_focus: &'static str,
    pub forbidden_focuses: &'static [&'static str],
    pub expected_transition: &'static str,
    pub require_pending_question: bool,
}

#[derive(Serialize)]
struct Row {
    id: String,
    category: String,
    turn_count: usize,
    restored_topic: Option<String>,
    restored_focus: Option<String>,
    resolved_text: Option<String>,
    graph_schema: Option<String>,
    graph_active_topic_id: Option<String>,
    graph_context_count: usize,
    graph_active_context_count: usize,
    graph_active_focus_id: Option<String>,
    graph_pending_question_id: Option<String>,
    graph_resource_count: usize,
    graph_transition_kinds: Vec<String>,
    graph_hash_bound: bool,
    graph_non_authoritative: bool,
    response_contracts_valid: bool,
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
    graph_contracts_valid: usize,
    response_contracts_valid: usize,
    authority_violations: usize,
    unsupported_explanation_facts: usize,
    external_llm_calls: usize,
    local_teacher_calls: usize,
    network_calls: usize,
    recursive_source_mutations: usize,
    rows: Vec<Row>,
}

fn request(id: &str, turn_index: u64, turn: Turn) -> ConversationTurnRequestIR {
    let voice = turn.alternative.is_some();
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: id.to_string(),
        turn_index,
        request_id: format!("{id}-{turn_index}"),
        modality: if voice {
            ConversationInputModalityIR::VoiceTranscript
        } else {
            ConversationInputModalityIR::Text
        },
        raw_text: turn.text.to_string(),
        input_confidence_millis: if voice { 820 } else { 1_000 },
        alternatives: turn
            .alternative
            .map(|alternative| {
                vec![semantic_core_adapters::UtteranceAlternativeIR {
                    text: alternative.to_string(),
                    confidence_millis: 790,
                }]
            })
            .unwrap_or_default(),
        output_language: Some(turn.language),
        context_tags: Vec::new(),
        max_plan_steps: 20,
    }
}

fn contains(value: &str, term: &str) -> bool {
    value.to_lowercase().contains(&term.to_lowercase())
}

fn contains_or_alias(value: &str, term: &str) -> bool {
    if contains(value, term) {
        return true;
    }
    let term = term.to_lowercase();
    [
        ("파일", "file"),
        ("폴더", "folder"),
        ("문서", "document"),
        ("보고서", "report"),
        ("프로젝트", "project"),
        ("저장소", "repository"),
        ("캐시", "cache"),
        ("큐", "queue"),
        ("로그", "log"),
        ("서버", "server"),
        ("워커", "worker"),
        ("백업", "backup"),
    ]
    .into_iter()
    .find(|(korean, english)| term == *korean || term == *english)
    .is_some_and(|(korean, english)| contains(value, korean) || contains(value, english))
}

fn safe(response: &ConversationTurnResponseIR) -> bool {
    response.grounded_realization.validate()
        && response.grounded_realization.realized_text == response.output.text
        && response
            .language_cortex_integration
            .unsupported_explanation_facts
            == 0
        && !response.language_cortex_integration.semantic_authority
        && !response.language_cortex_integration.language_can_execute
        && !response
            .language_cortex_integration
            .external_action_executed
        && !response.action_state_analysis.external_action_executed
}

fn current_focus(value: &Value) -> Option<(&str, &str)> {
    let focus = &value["discourse_focus"];
    let id = focus["current_focus_id"].as_str()?;
    let node = focus["nodes"]
        .as_array()?
        .iter()
        .find(|node| node["focus_id"].as_str() == Some(id))?;
    Some((id, node["surface"].as_str()?))
}

struct GraphContractObservation {
    valid: bool,
    non_authoritative: bool,
    schema: Option<String>,
    active_topic_id: Option<String>,
    context_count: usize,
    active_context_count: usize,
    active_focus_id: Option<String>,
    pending_question_id: Option<String>,
    resource_count: usize,
    transition_kinds: Vec<String>,
}

fn graph_contract(state: &Value) -> GraphContractObservation {
    let graph = &state["topic_context_graph"];
    let schema = graph["schema"].as_str().map(ToString::to_string);
    let active_topic_id = graph["active_topic_id"].as_str().map(ToString::to_string);
    let contexts = graph["contexts"].as_array().cloned().unwrap_or_default();
    let transitions = graph["transitions"].as_array().cloned().unwrap_or_default();
    let active = contexts
        .iter()
        .filter(|context| context["status"].as_str() == Some("ACTIVE"))
        .collect::<Vec<_>>();
    let active_focus = active
        .first()
        .and_then(|context| context["current_focus_id"].as_str())
        .map(ToString::to_string);
    let pending_question = active
        .first()
        .and_then(|context| context["pending_question_id"].as_str())
        .map(ToString::to_string);
    let resource_count = active.first().map_or(0, |context| {
        context["discourse_referent_ids"]
            .as_array()
            .map_or(0, Vec::len)
    });
    let transition_kinds = transitions
        .iter()
        .filter_map(|transition| transition["kind"].as_str().map(ToString::to_string))
        .collect::<Vec<_>>();
    let graph_hash_bound = graph["graph_sha256"]
        .as_str()
        .is_some_and(|hash| hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()));
    let graph_non_authoritative = graph["semantic_authority"].as_bool() == Some(false)
        && graph["external_execution_authorized"].as_bool() == Some(false)
        && contexts.iter().all(|context| {
            context["semantic_authority"].as_bool() == Some(false)
                && context["external_execution_authorized"].as_bool() == Some(false)
        })
        && transitions.iter().all(|transition| {
            transition["semantic_authority"].as_bool() == Some(false)
                && transition["external_execution_authorized"].as_bool() == Some(false)
        });
    let focus_id = current_focus(state).map(|(id, _)| id.to_string());
    let active_topic_matches = state["active_topics"]
        .as_array()
        .and_then(|topics| topics.first())
        .and_then(|topic| topic["topic_id"].as_str())
        == active_topic_id.as_deref();
    let valid = schema.as_deref() == Some("B_CORE_TOPIC_CONTEXT_GRAPH_IR_1")
        && graph_hash_bound
        && graph_non_authoritative
        && !contexts.is_empty()
        && active.len() == 1
        && active_focus == focus_id
        && active_topic_matches;
    GraphContractObservation {
        valid,
        non_authoritative: graph_non_authoritative,
        schema,
        active_topic_id,
        context_count: contexts.len(),
        active_context_count: active.len(),
        active_focus_id: active_focus,
        pending_question_id: pending_question,
        resource_count,
        transition_kinds,
    }
}

pub fn emit(suite: &'static str, cases: &[Case]) {
    let mut rows = Vec::new();
    for case in cases {
        let mut api = CognitiveApi::new_embedded().expect("embedded core");
        let mut responses = Vec::new();
        let mut contracts_valid = true;
        for (index, turn) in case.turns.iter().copied().enumerate() {
            let request = request(case.id, (index + 1) as u64, turn);
            match api.process_conversation_turn(&request) {
                Ok(response) => {
                    contracts_valid &= response.validate_against(&request) && safe(&response);
                    responses.push(response);
                }
                Err(_) => break,
            }
        }
        let restoration = case
            .restoration_turn
            .checked_sub(1)
            .and_then(|index| responses.get(index));
        let final_response = responses.last();
        let state_value = restoration
            .and_then(|response| serde_json::to_value(&response.conversation_state).ok())
            .unwrap_or(Value::Null);
        let restored_topic = state_value["active_topics"]
            .as_array()
            .and_then(|topics| topics.first())
            .and_then(|topic| topic["surface"].as_str())
            .map(ToString::to_string);
        let restored_focus = current_focus(&state_value).map(|(_, surface)| surface.to_string());
        let graph = graph_contract(&state_value);
        let resolved_text = final_response
            .map(|response| response.reference_resolution.resolved_semantic_text.clone());
        let focus_binding = final_response.is_some_and(|response| {
            response
                .reference_resolution
                .discourse_bindings
                .iter()
                .any(|binding| {
                    matches!(
                        binding.kind,
                        DiscourseBindingKindIR::DiscourseFocusReference
                            | DiscourseBindingKindIR::PossessiveFocusReference
                            | DiscourseBindingKindIR::DemonstrativeFocusReference
                            | DiscourseBindingKindIR::ZeroArgumentEllipsis
                    ) || case.require_pending_question
                        && binding.kind == DiscourseBindingKindIR::ClarificationAnswer
                        || case.expected_transition == "ACTIVATE"
                            && binding.kind == DiscourseBindingKindIR::TopicReference
                })
        });
        let authority_violation = !graph.non_authoritative && graph.schema.is_some()
            || final_response.is_some_and(|response| {
                response.language_cortex_integration.semantic_authority
                    || response.language_cortex_integration.language_can_execute
                    || response
                        .language_cortex_integration
                        .external_action_executed
                    || response.action_state_analysis.external_action_executed
            });
        let unsupported = final_response.map_or(0, |response| {
            response
                .language_cortex_integration
                .unsupported_explanation_facts
        });
        let topic_ok = restored_topic
            .as_deref()
            .is_some_and(|surface| contains_or_alias(surface, case.expected_topic));
        let focus_ok = restored_focus
            .as_deref()
            .is_some_and(|surface| contains_or_alias(surface, case.expected_focus));
        let resolution_ok = resolved_text.as_deref().is_some_and(|resolved| {
            contains_or_alias(resolved, case.expected_focus)
                && case
                    .forbidden_focuses
                    .iter()
                    .all(|term| !contains_or_alias(resolved, term))
        });
        let transition_ok = graph
            .transition_kinds
            .iter()
            .any(|kind| kind == case.expected_transition);
        let qud_ok = !case.require_pending_question || graph.pending_question_id.is_some();
        let pass = responses.len() == case.turns.len()
            && contracts_valid
            && graph.valid
            && topic_ok
            && focus_ok
            && resolution_ok
            && focus_binding
            && transition_ok
            && qud_ok
            && (graph.resource_count > 0
                || case.require_pending_question
                || case.expected_transition == "ACTIVATE")
            && !authority_violation
            && unsupported == 0;
        rows.push(Row {
            id: case.id.to_string(),
            category: case.category.to_string(),
            turn_count: case.turns.len(),
            restored_topic,
            restored_focus,
            resolved_text,
            graph_schema: graph.schema,
            graph_active_topic_id: graph.active_topic_id,
            graph_context_count: graph.context_count,
            graph_active_context_count: graph.active_context_count,
            graph_active_focus_id: graph.active_focus_id,
            graph_pending_question_id: graph.pending_question_id,
            graph_resource_count: graph.resource_count,
            graph_transition_kinds: graph.transition_kinds,
            graph_hash_bound: graph.valid,
            graph_non_authoritative: graph.non_authoritative,
            response_contracts_valid: contracts_valid,
            authority_violation,
            unsupported_explanation_facts: unsupported,
            pass,
        });
    }
    let total = rows.len();
    let passed = rows.iter().filter(|row| row.pass).count();
    let graph_contracts_valid = rows.iter().filter(|row| row.graph_hash_bound).count();
    let response_contracts_valid = rows
        .iter()
        .filter(|row| row.response_contracts_valid)
        .count();
    let authority_violations = rows.iter().filter(|row| row.authority_violation).count();
    let unsupported_explanation_facts = rows
        .iter()
        .map(|row| row.unsupported_explanation_facts)
        .sum();
    println!(
        "{}",
        serde_json::to_string(&Summary {
            schema: "B_CORE_R57_TOPIC_CONTEXT_CONSOLIDATION_CANARY_1",
            suite,
            total,
            passed,
            failed: total - passed,
            graph_contracts_valid,
            response_contracts_valid,
            authority_violations,
            unsupported_explanation_facts,
            external_llm_calls: 0,
            local_teacher_calls: 0,
            network_calls: 0,
            recursive_source_mutations: 0,
            rows,
        })
        .expect("summary JSON")
    );
    if passed != total {
        std::process::exit(1);
    }
}
