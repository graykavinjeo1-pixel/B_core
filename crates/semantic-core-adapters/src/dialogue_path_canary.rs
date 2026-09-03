//! Frozen R16-RUN-0001 multi-hop dialogue-path diagnostic suite.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR,
    DialogueRelationAnswerDispositionIR, DialogueRelationStatusIR, LanguageCodeIR,
    CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Serialize)]
struct Row {
    id: String,
    category: String,
    edge_count: usize,
    active_edges: usize,
    path_count: usize,
    hop_count: usize,
    disposition: Option<DialogueRelationAnswerDispositionIR>,
    nonactual: bool,
    pass: bool,
}

struct ChainCase {
    id: String,
    turns: Vec<(String, LanguageCodeIR)>,
    query: String,
    query_language: LanguageCodeIR,
    expected_hops: usize,
    expect_nonactual: bool,
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

fn run_chain(case: ChainCase) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let mut state = None;
    for (index, (text, language)) in case.turns.iter().enumerate() {
        let turn = u64::try_from(index + 1).expect("bounded turn");
        state = Some(
            api.process_conversation_turn(&request(&case.id, turn, text, *language))
                .unwrap_or_else(|error| {
                    panic!(
                        "chain turn failed: case={}, turn={turn}, text={text:?}, error={error:?}",
                        case.id
                    )
                })
                .conversation_state,
        );
    }
    let state = state.expect("non-empty chain");
    let query_turn = u64::try_from(case.turns.len() + 1).expect("bounded query turn");
    let response = api
        .process_conversation_turn(&request(
            &case.id,
            query_turn,
            &case.query,
            case.query_language,
        ))
        .expect("path query");
    let answer = response.dialogue_relation_answer.as_ref();
    let path_count = answer.map_or(0, |answer| answer.paths.len());
    let hop_count = answer
        .and_then(|answer| answer.paths.first())
        .map_or(0, |path| path.hop_count);
    let nonactual = answer
        .and_then(|answer| answer.paths.first())
        .is_some_and(|path| path.contains_nonactual_world);
    let disposition = answer.map(|answer| answer.disposition);
    let active_edges = state
        .dialogue_relation_graph
        .relations
        .iter()
        .filter(|edge| edge.status == DialogueRelationStatusIR::Active)
        .count();
    let pass = state
        .dialogue_relation_graph
        .validate_with_ledger(state.completed_turns, &state.epistemic_ledger)
        && active_edges == case.expected_hops
        && disposition == Some(DialogueRelationAnswerDispositionIR::AnsweredFromDialoguePath)
        && path_count == 1
        && hop_count == case.expected_hops
        && nonactual == case.expect_nonactual
        && answer.is_some_and(|answer| {
            answer.validate()
                && answer.evidence.len() == case.expected_hops
                && answer.paths.iter().all(|path| {
                    path.dialogue_claim_only
                        && !path.causal_truth_established
                        && !path.semantic_authority
                        && !path.external_execution_authorized
                })
        })
        && response.grounded_response.is_none()
        && response.output.grounded_plan_sha256.is_none();
    Row {
        id: case.id,
        category: if case.expect_nonactual {
            "nonactual_path".to_string()
        } else {
            "multi_hop_path".to_string()
        },
        edge_count: state.dialogue_relation_graph.relations.len(),
        active_edges,
        path_count,
        hop_count,
        disposition,
        nonactual,
        pass,
    }
}

fn run_retraction(id: &str, language: LanguageCodeIR, turns: [&str; 3], query: &str) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    for (index, text) in turns.iter().enumerate() {
        api.process_conversation_turn(&request(
            id,
            u64::try_from(index + 1).expect("bounded turn"),
            text,
            language,
        ))
        .expect("chain turn");
    }
    let retraction = if language == LanguageCodeIR::Korean {
        "그 주장을 철회해"
    } else {
        "Retract that claim"
    };
    let retracted = api
        .process_conversation_turn(&request(id, 4, retraction, language))
        .expect("retraction");
    let response = api
        .process_conversation_turn(&request(id, 5, query, language))
        .expect("query after retraction");
    let answer = response.dialogue_relation_answer.as_ref();
    let inactive = retracted
        .conversation_state
        .dialogue_relation_graph
        .relations
        .iter()
        .filter(|edge| edge.status != DialogueRelationStatusIR::Active)
        .count();
    let pass = retracted.reference_resolution.resolved_reference_count == 1
        && inactive == 1
        && retracted
            .conversation_state
            .dialogue_relation_graph
            .validate_with_ledger(
                retracted.conversation_state.completed_turns,
                &retracted.conversation_state.epistemic_ledger,
            )
        && answer.is_some_and(|answer| {
            answer.disposition == DialogueRelationAnswerDispositionIR::NoMatchingDialogueRelation
                && answer.evidence.is_empty()
                && answer.paths.is_empty()
        });
    Row {
        id: id.to_string(),
        category: "retraction_truth_maintenance".to_string(),
        edge_count: retracted
            .conversation_state
            .dialogue_relation_graph
            .relations
            .len(),
        active_edges: retracted
            .conversation_state
            .dialogue_relation_graph
            .relations
            .len()
            - inactive,
        path_count: answer.map_or(0, |answer| answer.paths.len()),
        hop_count: 0,
        disposition: answer.map(|answer| answer.disposition),
        nonactual: false,
        pass,
    }
}

fn main() {
    let english = ["Atlas", "Birch", "Cinder", "Delta", "Ember", "Fjord"];
    let korean = ["가온", "나래", "다온", "라온", "마루", "보람"];
    let mut rows = Vec::new();

    for name in english {
        rows.push(run_chain(ChainCase {
            id: format!("EN_BACKWARD_{name}"),
            turns: vec![
                (format!("{name} cache failure"), LanguageCodeIR::English),
                (
                    format!("Because of that, {name} service latency increase"),
                    LanguageCodeIR::English,
                ),
                (
                    format!("Therefore, {name} request queue growth"),
                    LanguageCodeIR::English,
                ),
            ],
            query: format!("Why {name} request queue growth?"),
            query_language: LanguageCodeIR::English,
            expected_hops: 2,
            expect_nonactual: false,
        }));
    }
    for (index, name) in korean.into_iter().enumerate() {
        rows.push(run_chain(ChainCase {
            id: format!("KO_BACKWARD_{}", index + 1),
            turns: vec![
                (format!("{name} 캐시 장애"), LanguageCodeIR::Korean),
                (
                    format!("그 때문에, {name} 서비스 지연"),
                    LanguageCodeIR::Korean,
                ),
                (
                    format!("따라서, {name} 요청 대기열 증가"),
                    LanguageCodeIR::Korean,
                ),
            ],
            query: format!("왜 {name} 요청 대기열 증가?"),
            query_language: LanguageCodeIR::Korean,
            expected_hops: 2,
            expect_nonactual: false,
        }));
    }
    for name in ["Grove", "Harbor", "Ivory", "Juniper"] {
        rows.push(run_chain(ChainCase {
            id: format!("EN_FORWARD_{name}"),
            turns: vec![
                (format!("{name} storage fault"), LanguageCodeIR::English),
                (
                    format!("As a result, {name} worker retry growth"),
                    LanguageCodeIR::English,
                ),
                (
                    format!("Consequently, {name} throughput decline"),
                    LanguageCodeIR::English,
                ),
            ],
            query: format!("What resulted from {name} storage fault?"),
            query_language: LanguageCodeIR::English,
            expected_hops: 2,
            expect_nonactual: false,
        }));
    }
    for (index, name) in ["사랑", "아람", "자람", "초롱"].into_iter().enumerate() {
        rows.push(run_chain(ChainCase {
            id: format!("KO_FORWARD_{}", index + 1),
            turns: vec![
                (format!("{name} 저장소 결함"), LanguageCodeIR::Korean),
                (
                    format!("그 결과, {name} 작업자 재시도 증가"),
                    LanguageCodeIR::Korean,
                ),
                (
                    format!("그래서, {name} 처리량 감소"),
                    LanguageCodeIR::Korean,
                ),
            ],
            query: format!("{name} 저장소 결함의 결과는?"),
            query_language: LanguageCodeIR::Korean,
            expected_hops: 2,
            expect_nonactual: false,
        }));
    }
    for (index, name) in ["Kite", "Lumen", "Mica", "Nimbus"].iter().enumerate() {
        rows.push(run_chain(ChainCase {
            id: format!("CROSS_{name}"),
            turns: vec![
                (format!("{name} cache failure"), LanguageCodeIR::English),
                (
                    format!("그 때문에, {name} 서비스 지연"),
                    LanguageCodeIR::Korean,
                ),
                (
                    format!("Therefore, {name} queue growth"),
                    LanguageCodeIR::English,
                ),
            ],
            query: if index % 2 == 0 {
                format!("왜 {name} queue growth?")
            } else {
                format!("Why {name} queue growth?")
            },
            query_language: if index % 2 == 0 {
                LanguageCodeIR::Korean
            } else {
                LanguageCodeIR::English
            },
            expected_hops: 2,
            expect_nonactual: false,
        }));
    }
    rows.push(run_retraction(
        "RET_EN_1",
        LanguageCodeIR::English,
        [
            "Opal cache failure",
            "Because of that, Opal service latency increase",
            "Therefore, Opal queue growth",
        ],
        "Why Opal queue growth?",
    ));
    rows.push(run_retraction(
        "RET_EN_2",
        LanguageCodeIR::English,
        [
            "Pine storage fault",
            "As a result, Pine retry growth",
            "Consequently, Pine throughput decline",
        ],
        "Why Pine throughput decline?",
    ));
    rows.push(run_retraction(
        "RET_KO_1",
        LanguageCodeIR::Korean,
        [
            "하람 캐시 장애",
            "그 때문에, 하람 서비스 지연",
            "따라서, 하람 대기열 증가",
        ],
        "왜 하람 대기열 증가?",
    ));
    for name in ["Quartz", "Ripple", "Sable"] {
        rows.push(run_chain(ChainCase {
            id: format!("MODAL_{name}"),
            turns: vec![
                (
                    format!("Maybe {name} cache failure"),
                    LanguageCodeIR::English,
                ),
                (
                    format!("Because of that, {name} service latency increase"),
                    LanguageCodeIR::English,
                ),
                (
                    format!("Therefore, {name} queue growth"),
                    LanguageCodeIR::English,
                ),
            ],
            query: format!("Why {name} queue growth?"),
            query_language: LanguageCodeIR::English,
            expected_hops: 2,
            expect_nonactual: true,
        }));
    }

    let passed = rows.iter().filter(|row| row.pass).count();
    let payload = serde_json::json!({
        "suite": "R16-RUN-0001",
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
