//! Frozen R23-RUN-0002 held-out transfer for clause and event structure.

use dockable_semantic_core::PlanIntentIR;
use semantic_core_adapters::{
    CandidateDispositionIR, CognitiveApi, CompositionalSemanticAnalyzer,
    ConversationInputModalityIR, ConversationTurnDispositionIR, ConversationTurnRequestIR,
    LanguageCodeIR, QuantifierKindIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    category: String,
    trace: Vec<String>,
    pass: bool,
}

fn request(id: &str, turn: u64, text: &str, language: LanguageCodeIR) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: id.to_string(),
        turn_index: turn,
        request_id: format!("{id}-{turn}"),
        modality: ConversationInputModalityIR::Text,
        raw_text: text.to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(language),
        context_tags: Vec::new(),
        max_plan_steps: 24,
    }
}

fn relative_transfer(
    id: &str,
    text: &str,
    language: LanguageCodeIR,
    head: &str,
    dependent: &str,
    head_quantifier: QuantifierKindIR,
    dependent_quantifier: QuantifierKindIR,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let response = api
        .process_conversation_turn(&request(id, 1, text, language))
        .expect("relative transfer");
    let graph = &response
        .pragmatic_interpretation
        .compositional_analysis
        .semantic_role_graph;
    let serialized = serde_json::to_string(graph)
        .expect("graph json")
        .to_lowercase();
    let scoped = |target: &str, kind: QuantifierKindIR| {
        graph.quantifier_scopes.iter().any(|scope| {
            scope.quantifier == kind
                && graph.nodes.iter().any(|node| {
                    node.node_id == scope.target_node_id
                        && node.normalized_label.to_lowercase().contains(target)
                })
        })
    };
    let output = response.output.text.to_lowercase();
    Row {
        id: id.to_string(),
        category: "open_vocabulary_relative_quantifier_transfer".to_string(),
        trace: vec![serialized.clone(), output.clone()],
        pass: response.disposition == ConversationTurnDispositionIR::Grounded
            && response.conversation_state.active_goals.len() == 1
            && response.conversation_state.active_goals[0]
                .subject
                .to_lowercase()
                == head
            && serialized.contains("relative_clause_attachments")
            && serialized.contains(dependent)
            && scoped(head, head_quantifier)
            && scoped(dependent, dependent_quantifier)
            && output.contains(head)
            && output.contains(dependent)
            && response.output.unsupported_freeform_claims == 0,
    }
}

fn cross_event_transfer(
    id: &str,
    setup: &str,
    query: &str,
    setup_language: LanguageCodeIR,
    query_language: LanguageCodeIR,
    target: &str,
    rejected: &[&str],
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(id, 1, setup, setup_language))
        .expect("event setup");
    let response = api
        .process_conversation_turn(&request(id, 2, query, query_language))
        .expect("event query");
    let resolved = response
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    let bound = response
        .reference_resolution
        .discourse_bindings
        .iter()
        .any(|binding| {
            format!("{:?}", binding.kind) == "EventOrdinalReference"
                && binding
                    .evidence
                    .iter()
                    .any(|item| item.contains("EVENT_SEQUENCE_POSITION"))
        });
    Row {
        id: id.to_string(),
        category: "cross_language_event_sequence_transfer".to_string(),
        trace: vec![resolved.clone(), response.output.text],
        pass: response.disposition == ConversationTurnDispositionIR::Grounded
            && bound
            && resolved.contains(target)
            && rejected.iter().all(|term| !resolved.contains(term)),
    }
}

fn authority_transfer(id: &str, text: &str, head: &str, embedded_intent: PlanIntentIR) -> Row {
    let analysis = CompositionalSemanticAnalyzer.analyze(text);
    let selected = analysis.selected_candidates();
    let embedded_safe = analysis.candidates.iter().any(|candidate| {
        candidate.intent == embedded_intent
            && candidate.disposition == CandidateDispositionIR::NonAuthoritativeMention
            && !candidate.external_execution_authorized
    });
    Row {
        id: id.to_string(),
        category: "relative_action_authority_attack".to_string(),
        trace: vec![format!("candidates={:?}", analysis.candidates)],
        pass: selected.len() == 1
            && selected[0].intent == PlanIntentIR::Investigate
            && selected[0].subject.to_lowercase() == head
            && embedded_safe,
    }
}

fn event_range_transfer(id: &str, setup: &str, query: &str, language: LanguageCodeIR) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(id, 1, setup, language))
        .expect("range setup");
    let response = api
        .process_conversation_turn(&request(id, 2, query, language))
        .expect("range query");
    Row {
        id: id.to_string(),
        category: "event_sequence_range_attack".to_string(),
        trace: vec![
            format!(
                "ambiguity={:?}",
                response.reference_resolution.ambiguous_reference_surfaces
            ),
            response.output.text,
        ],
        pass: response.disposition == ConversationTurnDispositionIR::ClarificationRequired
            && response
                .reference_resolution
                .ambiguous_reference_surfaces
                .iter()
                .any(|item| item == "EVENT_SEQUENCE_ORDINAL"),
    }
}

fn main() {
    let rows = vec![
        relative_transfer(
            "R23_TRANSFER_REL_1",
            "일부 결함이 있는 모든 아카이브를 분석해",
            LanguageCodeIR::Korean,
            "아카이브",
            "결함",
            QuantifierKindIR::All,
            QuantifierKindIR::Some,
        ),
        relative_transfer(
            "R23_TRANSFER_REL_2",
            "inspect each bundle that contains some anomaly",
            LanguageCodeIR::English,
            "bundle",
            "anomaly",
            QuantifierKindIR::Each,
            QuantifierKindIR::Some,
        ),
        relative_transfer(
            "R23_TRANSFER_REL_3",
            "일부 경고가 있는 각 스냅샷을 확인해",
            LanguageCodeIR::Korean,
            "스냅샷",
            "경고",
            QuantifierKindIR::Each,
            QuantifierKindIR::Some,
        ),
        relative_transfer(
            "R23_TRANSFER_REL_4",
            "repair every channel that has some stalled consumer",
            LanguageCodeIR::English,
            "channel",
            "consumer",
            QuantifierKindIR::All,
            QuantifierKindIR::Some,
        ),
        cross_event_transfer(
            "R23_TRANSFER_EVENT_1",
            "파일을 분석하고 폴더를 수리한 뒤 보고서를 저장해",
            "explain the second action",
            LanguageCodeIR::Korean,
            LanguageCodeIR::English,
            "folder",
            &["file", "report"],
        ),
        cross_event_transfer(
            "R23_TRANSFER_EVENT_2",
            "inspect the cache, repair the queue, then save the log",
            "세 번째 작업을 설명해",
            LanguageCodeIR::English,
            LanguageCodeIR::Korean,
            "로그",
            &["캐시", "큐"],
        ),
        cross_event_transfer(
            "R23_TRANSFER_EVENT_3",
            "서버를 확인하고 워커를 수리한 뒤 백업을 저장해",
            "explain the last action",
            LanguageCodeIR::Korean,
            LanguageCodeIR::English,
            "backup",
            &["server", "worker"],
        ),
        cross_event_transfer(
            "R23_TRANSFER_EVENT_4",
            "inspect the document, repair the code, then save the report",
            "첫 번째 작업을 설명해",
            LanguageCodeIR::English,
            LanguageCodeIR::Korean,
            "문서",
            &["코드", "보고서"],
        ),
        authority_transfer(
            "R23_TRANSFER_AUTH_1",
            "검증기가 수정한 모든 산출물을 분석해",
            "산출물",
            PlanIntentIR::Repair,
        ),
        authority_transfer(
            "R23_TRANSFER_AUTH_2",
            "inspect every artifact that the verifier repaired",
            "artifact",
            PlanIntentIR::Repair,
        ),
        authority_transfer(
            "R23_TRANSFER_AUTH_3",
            "워커가 생성한 각 스냅샷을 확인해",
            "스냅샷",
            PlanIntentIR::Create,
        ),
        authority_transfer(
            "R23_TRANSFER_AUTH_4",
            "analyze each bundle that the worker created",
            "bundle",
            PlanIntentIR::Create,
        ),
        event_range_transfer(
            "R23_TRANSFER_RANGE_1",
            "문서를 분석하고 코드를 수리해",
            "마지막 다음 작업을 설명해",
            LanguageCodeIR::Korean,
        ),
        event_range_transfer(
            "R23_TRANSFER_RANGE_2",
            "inspect the document and repair the code",
            "explain the fourth action",
            LanguageCodeIR::English,
        ),
        event_range_transfer(
            "R23_TRANSFER_RANGE_3",
            "캐시를 확인하고 큐를 수리해",
            "세 번째 작업을 설명해",
            LanguageCodeIR::Korean,
        ),
        event_range_transfer(
            "R23_TRANSFER_RANGE_4",
            "inspect the server and repair the worker",
            "explain the action after the last",
            LanguageCodeIR::English,
        ),
    ];
    let passed = rows.iter().filter(|row| row.pass).count();
    let payload = serde_json::json!({
        "suite": "R23-RUN-0002",
        "held_out_until_after_diagnostic_repairs": true,
        "external_llm_calls": 0,
        "local_teacher_calls": 0,
        "recursive_source_mutations": 0,
        "total": rows.len(),
        "passed": passed,
        "failed": rows.len() - passed,
        "rows": rows,
    });
    println!("{}", serde_json::to_string_pretty(&payload).expect("json"));
    if passed != payload["total"].as_u64().unwrap_or_default() as usize {
        std::process::exit(1);
    }
}
