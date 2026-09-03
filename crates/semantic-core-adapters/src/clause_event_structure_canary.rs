//! Frozen R23-RUN-0001 relative-clause, nested-quantifier, and event-chain diagnostic.

use dockable_semantic_core::PlanIntentIR;
use semantic_core_adapters::{
    CandidateDispositionIR, CognitiveApi, CompositionalSemanticAnalyzer,
    ConversationInputModalityIR, ConversationTurnDispositionIR, ConversationTurnRequestIR,
    LanguageCodeIR, QuantifierKindIR, SemanticRoleGraphIR, CONVERSATION_TURN_REQUEST_SCHEMA,
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

fn graph_json(graph: &SemanticRoleGraphIR) -> String {
    serde_json::to_string(graph)
        .expect("semantic role graph json")
        .to_lowercase()
}

fn has_scope(
    graph: &SemanticRoleGraphIR,
    target: &str,
    kind: QuantifierKindIR,
    cardinality: Option<u64>,
) -> bool {
    graph.quantifier_scopes.iter().any(|scope| {
        scope.quantifier == kind
            && scope.cardinality == cardinality
            && graph.nodes.iter().any(|node| {
                node.node_id == scope.target_node_id
                    && node.normalized_label.to_lowercase().contains(target)
            })
    })
}

struct RelativeCase<'a> {
    id: &'a str,
    text: &'a str,
    head: &'a str,
    dependent: &'a str,
}

fn relative_case(case: RelativeCase<'_>) -> Row {
    let analysis = CompositionalSemanticAnalyzer.analyze(case.text);
    let selected = analysis.selected_candidates();
    let serialized = graph_json(&analysis.semantic_role_graph);
    Row {
        id: case.id.to_string(),
        category: "relative_clause_attachment".to_string(),
        trace: vec![
            format!(
                "selected={:?}",
                selected
                    .iter()
                    .map(|item| &item.subject)
                    .collect::<Vec<_>>()
            ),
            serialized.clone(),
        ],
        pass: selected.len() == 1
            && selected[0].subject.to_lowercase() == case.head
            && serialized.contains("relative_clause_attachments")
            && serialized.contains(case.head)
            && serialized.contains(case.dependent),
    }
}

struct QuantifierCase<'a> {
    id: &'a str,
    text: &'a str,
    head: &'a str,
    head_kind: QuantifierKindIR,
    head_cardinality: Option<u64>,
    dependent: &'a str,
    dependent_kind: QuantifierKindIR,
    dependent_cardinality: Option<u64>,
}

fn quantifier_case(case: QuantifierCase<'_>) -> Row {
    let analysis = CompositionalSemanticAnalyzer.analyze(case.text);
    let graph = &analysis.semantic_role_graph;
    Row {
        id: case.id.to_string(),
        category: "nested_quantifier_scope".to_string(),
        trace: vec![serde_json::to_string(graph).expect("graph json")],
        pass: has_scope(graph, case.head, case.head_kind, case.head_cardinality)
            && has_scope(
                graph,
                case.dependent,
                case.dependent_kind,
                case.dependent_cardinality,
            ),
    }
}

struct EventOrdinalCase<'a> {
    id: &'a str,
    setup: &'a str,
    query: &'a str,
    setup_language: LanguageCodeIR,
    query_language: LanguageCodeIR,
    target: &'a str,
    rejected: &'a [&'a str],
}

fn event_ordinal_case(case: EventOrdinalCase<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let setup = api
        .process_conversation_turn(&request(case.id, 1, case.setup, case.setup_language))
        .expect("event setup");
    let response = api
        .process_conversation_turn(&request(case.id, 2, case.query, case.query_language))
        .expect("event ordinal query");
    let resolved = response
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    let ordinal_binding = response
        .reference_resolution
        .discourse_bindings
        .iter()
        .any(|binding| format!("{:?}", binding.kind) == "EventOrdinalReference");
    Row {
        id: case.id.to_string(),
        category: "cross_turn_event_ordinal".to_string(),
        trace: vec![
            format!(
                "setup_goals={}",
                setup.conversation_state.active_goals.len()
            ),
            resolved.clone(),
            response.output.text,
        ],
        pass: setup.conversation_state.active_goals.len() == 3
            && response.disposition == ConversationTurnDispositionIR::Grounded
            && ordinal_binding
            && resolved.contains(case.target)
            && case.rejected.iter().all(|term| !resolved.contains(term)),
    }
}

fn event_range_case(id: &str, setup: &str, query: &str, language: LanguageCodeIR) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(id, 1, setup, language))
        .expect("event range setup");
    let response = api
        .process_conversation_turn(&request(id, 2, query, language))
        .expect("event range query");
    Row {
        id: id.to_string(),
        category: "event_ordinal_range_fails_closed".to_string(),
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

struct RelativeAuthorityCase<'a> {
    id: &'a str,
    text: &'a str,
    head: &'a str,
    main_intent: PlanIntentIR,
    embedded_intent: PlanIntentIR,
}

fn relative_authority_case(case: RelativeAuthorityCase<'_>) -> Row {
    let analysis = CompositionalSemanticAnalyzer.analyze(case.text);
    let selected = analysis.selected_candidates();
    let embedded_is_descriptive = analysis.candidates.iter().any(|candidate| {
        candidate.intent == case.embedded_intent
            && candidate.disposition == CandidateDispositionIR::NonAuthoritativeMention
            && !candidate.external_execution_authorized
    });
    Row {
        id: case.id.to_string(),
        category: "relative_predicate_has_no_goal_authority".to_string(),
        trace: vec![
            format!("frames={:?}", analysis.frames),
            format!("candidates={:?}", analysis.candidates),
        ],
        pass: selected.len() == 1
            && selected[0].intent == case.main_intent
            && selected[0].subject.to_lowercase() == case.head
            && embedded_is_descriptive
            && graph_json(&analysis.semantic_role_graph).contains("relative_clause_attachments"),
    }
}

fn realization_case(
    id: &str,
    text: &str,
    language: LanguageCodeIR,
    head: &str,
    dependent: &str,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let response = api
        .process_conversation_turn(&request(id, 1, text, language))
        .expect("relative realization");
    let output = response.output.text.to_lowercase();
    let planned = if language == LanguageCodeIR::Korean {
        output.contains("계획") && output.contains("아직 실행 결과")
    } else {
        output.contains("plan") && output.contains("not completed")
    };
    Row {
        id: id.to_string(),
        category: "grounded_relative_realization".to_string(),
        trace: vec![output.clone()],
        pass: response.disposition == ConversationTurnDispositionIR::Grounded
            && response.conversation_state.active_goals.len() == 1
            && output.contains(head)
            && output.contains(dependent)
            && planned
            && !output.contains("relative_clause")
            && response.output.unsupported_freeform_claims == 0,
    }
}

fn main() {
    let rows = vec![
        relative_case(RelativeCase {
            id: "R23_REL_1",
            text: "모든 오류가 있는 각 파일을 분석해",
            head: "파일",
            dependent: "오류",
        }),
        relative_case(RelativeCase {
            id: "R23_REL_2",
            text: "inspect each file that contains some error",
            head: "file",
            dependent: "error",
        }),
        relative_case(RelativeCase {
            id: "R23_REL_3",
            text: "일부 로그가 있는 모든 폴더를 확인해",
            head: "폴더",
            dependent: "로그",
        }),
        relative_case(RelativeCase {
            id: "R23_REL_4",
            text: "repair every queue that has some blocked worker",
            head: "queue",
            dependent: "worker",
        }),
        quantifier_case(QuantifierCase {
            id: "R23_QUANT_1",
            text: "정확히 2개의 오류가 있는 모든 파일을 분석해",
            head: "파일",
            head_kind: QuantifierKindIR::All,
            head_cardinality: None,
            dependent: "오류",
            dependent_kind: QuantifierKindIR::Exactly,
            dependent_cardinality: Some(2),
        }),
        quantifier_case(QuantifierCase {
            id: "R23_QUANT_2",
            text: "inspect at least 2 files that contain exactly 1 error",
            head: "file",
            head_kind: QuantifierKindIR::AtLeast,
            head_cardinality: Some(2),
            dependent: "error",
            dependent_kind: QuantifierKindIR::Exactly,
            dependent_cardinality: Some(1),
        }),
        quantifier_case(QuantifierCase {
            id: "R23_QUANT_3",
            text: "일부 로그가 있는 각 폴더를 확인해",
            head: "폴더",
            head_kind: QuantifierKindIR::Each,
            head_cardinality: None,
            dependent: "로그",
            dependent_kind: QuantifierKindIR::Some,
            dependent_cardinality: None,
        }),
        quantifier_case(QuantifierCase {
            id: "R23_QUANT_4",
            text: "repair every queue that has some worker",
            head: "queue",
            head_kind: QuantifierKindIR::All,
            head_cardinality: None,
            dependent: "worker",
            dependent_kind: QuantifierKindIR::Some,
            dependent_cardinality: None,
        }),
        event_ordinal_case(EventOrdinalCase {
            id: "R23_EVENT_1",
            setup: "파일을 분석하고 폴더를 수리한 뒤 보고서를 저장해",
            query: "두 번째 작업을 설명해",
            setup_language: LanguageCodeIR::Korean,
            query_language: LanguageCodeIR::Korean,
            target: "폴더",
            rejected: &["파일", "보고서"],
        }),
        event_ordinal_case(EventOrdinalCase {
            id: "R23_EVENT_2",
            setup: "inspect the file, repair the folder, then save the report",
            query: "explain the third action",
            setup_language: LanguageCodeIR::English,
            query_language: LanguageCodeIR::English,
            target: "report",
            rejected: &["file", "folder"],
        }),
        event_ordinal_case(EventOrdinalCase {
            id: "R23_EVENT_3",
            setup: "파일을 분석하고 폴더를 수리한 뒤 보고서를 저장해",
            query: "explain the first action",
            setup_language: LanguageCodeIR::Korean,
            query_language: LanguageCodeIR::English,
            target: "file",
            rejected: &["folder", "report"],
        }),
        event_ordinal_case(EventOrdinalCase {
            id: "R23_EVENT_4",
            setup: "inspect the file, repair the folder, then save the report",
            query: "세 번째 작업을 설명해",
            setup_language: LanguageCodeIR::English,
            query_language: LanguageCodeIR::Korean,
            target: "보고서",
            rejected: &["파일", "폴더"],
        }),
        event_range_case(
            "R23_RANGE_1",
            "파일을 분석하고 폴더를 수리해",
            "세 번째 작업을 설명해",
            LanguageCodeIR::Korean,
        ),
        event_range_case(
            "R23_RANGE_2",
            "inspect the file and repair the folder",
            "explain the third action",
            LanguageCodeIR::English,
        ),
        event_range_case(
            "R23_RANGE_3",
            "캐시를 확인하고 큐를 수리해",
            "네 번째 작업을 설명해",
            LanguageCodeIR::Korean,
        ),
        event_range_case(
            "R23_RANGE_4",
            "inspect the server and repair the worker",
            "explain the fourth action",
            LanguageCodeIR::English,
        ),
        relative_authority_case(RelativeAuthorityCase {
            id: "R23_AUTH_1",
            text: "파서가 수리한 모든 파일을 분석해",
            head: "파일",
            main_intent: PlanIntentIR::Investigate,
            embedded_intent: PlanIntentIR::Repair,
        }),
        relative_authority_case(RelativeAuthorityCase {
            id: "R23_AUTH_2",
            text: "inspect every file that the parser repaired",
            head: "file",
            main_intent: PlanIntentIR::Investigate,
            embedded_intent: PlanIntentIR::Repair,
        }),
        relative_authority_case(RelativeAuthorityCase {
            id: "R23_AUTH_3",
            text: "워커가 수정한 각 보고서를 확인해",
            head: "보고서",
            main_intent: PlanIntentIR::Investigate,
            embedded_intent: PlanIntentIR::Repair,
        }),
        relative_authority_case(RelativeAuthorityCase {
            id: "R23_AUTH_4",
            text: "analyze each report that the worker created",
            head: "report",
            main_intent: PlanIntentIR::Investigate,
            embedded_intent: PlanIntentIR::Create,
        }),
        realization_case(
            "R23_REALIZE_1",
            "일부 오류가 있는 모든 파일을 분석해",
            LanguageCodeIR::Korean,
            "파일",
            "오류",
        ),
        realization_case(
            "R23_REALIZE_2",
            "inspect every file that contains some error",
            LanguageCodeIR::English,
            "file",
            "error",
        ),
        realization_case(
            "R23_REALIZE_3",
            "일부 로그가 있는 각 폴더를 확인해",
            LanguageCodeIR::Korean,
            "폴더",
            "로그",
        ),
        realization_case(
            "R23_REALIZE_4",
            "repair each queue that has some worker",
            LanguageCodeIR::English,
            "queue",
            "worker",
        ),
    ];
    let passed = rows.iter().filter(|row| row.pass).count();
    let payload = serde_json::json!({
        "suite": "R23-RUN-0001",
        "frozen_before_product_changes": true,
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
