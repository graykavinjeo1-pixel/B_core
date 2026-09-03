//! Frozen R15-RUN-0001 cross-turn causal/concessive diagnostic suite.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, DialogueRelationAnswerDispositionIR, DialogueRelationKindIR,
    LanguageCodeIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Serialize)]
struct Row {
    id: String,
    source: String,
    relation_turn: String,
    query: String,
    expected_kind: DialogueRelationKindIR,
    relation_count: usize,
    answer_disposition: Option<DialogueRelationAnswerDispositionIR>,
    causal_truth_established: bool,
    semantic_authority: bool,
    external_execution_authorized: bool,
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
        max_plan_steps: 16,
    }
}

fn run_case(
    id: String,
    source: String,
    relation_turn: String,
    query: String,
    expected_kind: DialogueRelationKindIR,
    language: LanguageCodeIR,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(&id, 1, &source, language))
        .expect("source turn");
    let linked = api
        .process_conversation_turn(&request(&id, 2, &relation_turn, language))
        .expect("relation turn");
    let edges = &linked.conversation_state.dialogue_relation_graph.relations;
    let edge_guard = edges.first().map_or((true, true, true), |edge| {
        (
            edge.causal_truth_established,
            edge.semantic_authority,
            edge.external_execution_authorized,
        )
    });
    let answered = api
        .process_conversation_turn(&request(&id, 3, &query, language))
        .expect("query turn");
    let answer_disposition = answered
        .dialogue_relation_answer
        .as_ref()
        .map(|answer| answer.disposition);
    let answer_guard = answered
        .dialogue_relation_answer
        .as_ref()
        .is_some_and(|answer| {
            answer.validate()
                && !answer.dialogue_truth_established
                && !answer.external_execution_authorized
                && answer.unsupported_claims == 0
                && answer.evidence.iter().all(|item| {
                    !item.causal_truth_established
                        && !item.semantic_authority
                        && !item.external_execution_authorized
                })
        });
    let pass = linked.disposition == ConversationTurnDispositionIR::Grounded
        && edges.len() == 1
        && edges[0].kind == expected_kind
        && !edge_guard.0
        && !edge_guard.1
        && !edge_guard.2
        && answer_disposition
            == Some(DialogueRelationAnswerDispositionIR::AnsweredFromDialogueRelation)
        && answer_guard
        && answered.grounded_response.is_none()
        && answered.output.grounded_plan_sha256.is_none();
    Row {
        id,
        source,
        relation_turn,
        query,
        expected_kind,
        relation_count: edges.len(),
        answer_disposition,
        causal_truth_established: edge_guard.0,
        semantic_authority: edge_guard.1,
        external_execution_authorized: edge_guard.2,
        pass,
    }
}

fn main() {
    let english = ["Atlas", "Birch", "Cinder", "Delta", "Ember", "Fjord"];
    let korean = ["가온", "나래", "다온", "라온", "마루", "보람"];
    let mut rows = Vec::new();

    for (index, name) in english.iter().enumerate() {
        let connector = if index % 2 == 0 {
            "Because of that"
        } else {
            "For that reason"
        };
        rows.push(run_case(
            format!("EN_CAUSE_{name}"),
            format!("{name} cache integrity failed"),
            format!("{connector}, {name} service latency is high"),
            format!("Why is {name} service latency high?"),
            DialogueRelationKindIR::Cause,
            LanguageCodeIR::English,
        ));
    }
    for (index, name) in korean.iter().enumerate() {
        let connector = if index % 2 == 0 {
            "그 때문에"
        } else {
            "그런 이유로"
        };
        rows.push(run_case(
            format!("KO_CAUSE_{index:02}"),
            format!("{name} 캐시 무결성 실패"),
            format!("{connector}, {name} 서비스 지연 발생"),
            format!("왜 {name} 서비스 지연 발생?"),
            DialogueRelationKindIR::Cause,
            LanguageCodeIR::Korean,
        ));
    }
    for (index, name) in english.iter().enumerate() {
        let connector = ["Therefore", "Consequently", "As a result"][index % 3];
        rows.push(run_case(
            format!("EN_RESULT_{name}"),
            format!("{name} cache failure"),
            format!("{connector}, {name} service entered degraded mode"),
            format!("What resulted from {name} cache failure?"),
            DialogueRelationKindIR::Consequence,
            LanguageCodeIR::English,
        ));
    }
    for (index, name) in korean.iter().enumerate() {
        let connector = ["그래서", "따라서", "그 결과"][index % 3];
        rows.push(run_case(
            format!("KO_RESULT_{index:02}"),
            format!("{name} 캐시 장애"),
            format!("{connector}, {name} 서비스 성능 저하"),
            format!("{name} 캐시 장애의 결과는?"),
            DialogueRelationKindIR::Consequence,
            LanguageCodeIR::Korean,
        ));
    }
    for (index, name) in english.iter().enumerate() {
        let connector = ["Even so", "Nevertheless", "Despite that"][index % 3];
        rows.push(run_case(
            format!("EN_CONCESSION_{name}"),
            format!("{name} migration high cost"),
            format!("{connector}, {name} team continued rollout"),
            format!("What happened despite {name} migration high cost?"),
            DialogueRelationKindIR::Concession,
            LanguageCodeIR::English,
        ));
    }
    for (index, name) in korean.iter().enumerate() {
        let connector = ["그럼에도", "그런데도", "그래도"][index % 3];
        rows.push(run_case(
            format!("KO_CONCESSION_{index:02}"),
            format!("{name} 마이그레이션 고비용"),
            format!("{connector}, {name} 팀 배포 계속"),
            format!("{name} 마이그레이션 고비용에도 불구하고 결과?"),
            DialogueRelationKindIR::Concession,
            LanguageCodeIR::Korean,
        ));
    }

    let passed = rows.iter().filter(|row| row.pass).count();
    let payload = serde_json::json!({
        "suite": "R15-RUN-0001",
        "frozen_before_first_suite_execution": true,
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
