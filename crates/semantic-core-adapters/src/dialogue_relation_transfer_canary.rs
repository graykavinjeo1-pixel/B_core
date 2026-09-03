//! Frozen R15-RUN-0002 relation transfer and adversarial boundary suite.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, DialogueRelationAnswerDispositionIR, DialogueRelationKindIR,
    LanguageCodeIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Serialize)]
struct Row {
    id: String,
    category: String,
    pass: bool,
    disposition: ConversationTurnDispositionIR,
    relation_count: usize,
    unresolved: Vec<String>,
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

fn transfer_case(
    id: &str,
    source: &str,
    relation: &str,
    query: &str,
    kind: DialogueRelationKindIR,
    languages: [LanguageCodeIR; 3],
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(id, 1, source, languages[0]))
        .expect("source");
    let linked = api
        .process_conversation_turn(&request(id, 2, relation, languages[1]))
        .expect("link");
    let answered = api
        .process_conversation_turn(&request(id, 3, query, languages[2]))
        .expect("query");
    let graph = &linked.conversation_state.dialogue_relation_graph;
    let pass = linked.disposition == ConversationTurnDispositionIR::Grounded
        && graph.validate(2)
        && graph.relations.len() == 1
        && graph.relations[0].kind == kind
        && graph.relations.iter().all(|edge| {
            edge.dialogue_claim_only
                && !edge.causal_truth_established
                && !edge.semantic_authority
                && !edge.external_execution_authorized
        })
        && answered
            .dialogue_relation_answer
            .as_ref()
            .is_some_and(|answer| {
                answer.validate()
                    && answer.disposition
                        == DialogueRelationAnswerDispositionIR::AnsweredFromDialogueRelation
            });
    Row {
        id: id.to_string(),
        category: "cross_language_transfer".to_string(),
        pass,
        disposition: linked.disposition,
        relation_count: graph.relations.len(),
        unresolved: linked.reference_resolution.ambiguous_reference_surfaces,
    }
}

fn adversarial_case(id: &str, turns: &[(&str, LanguageCodeIR)], expect_clarify: bool) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let mut last = None;
    for (index, (text, language)) in turns.iter().enumerate() {
        last = Some(
            api.process_conversation_turn(&request(
                id,
                u64::try_from(index + 1).expect("bounded turn"),
                text,
                *language,
            ))
            .expect("turn"),
        );
    }
    let last = last.expect("at least one turn");
    let relation_count = last
        .conversation_state
        .dialogue_relation_graph
        .relations
        .len();
    let pass = if expect_clarify {
        last.disposition == ConversationTurnDispositionIR::ClarificationRequired
            && relation_count == 0
            && last
                .reference_resolution
                .ambiguous_reference_surfaces
                .iter()
                .any(|item| item.contains("DISCOURSE_RELATION_ANTECEDENT"))
    } else {
        relation_count == 0
            && last
                .conversation_state
                .dialogue_relation_graph
                .validate(last.turn_index)
    };
    Row {
        id: id.to_string(),
        category: if expect_clarify {
            "fail_closed".to_string()
        } else {
            "non_trigger".to_string()
        },
        pass,
        disposition: last.disposition,
        relation_count,
        unresolved: last.reference_resolution.ambiguous_reference_surfaces,
    }
}

fn main() {
    let mut rows = vec![
        transfer_case(
            "XFER_EN_KO_CAUSE",
            "Solace cache integrity failed",
            "그 때문에, Solace 서비스 지연 발생",
            "Why is Solace 서비스 지연 발생?",
            DialogueRelationKindIR::Cause,
            [
                LanguageCodeIR::English,
                LanguageCodeIR::Korean,
                LanguageCodeIR::English,
            ],
        ),
        transfer_case(
            "XFER_KO_EN_CAUSE",
            "누리 캐시 무결성 실패",
            "Because of that, 누리 service latency high",
            "왜 누리 service latency high?",
            DialogueRelationKindIR::Cause,
            [
                LanguageCodeIR::Korean,
                LanguageCodeIR::English,
                LanguageCodeIR::Korean,
            ],
        ),
        transfer_case(
            "XFER_EN_KO_RESULT",
            "Verdant cache failure",
            "따라서, Verdant 서비스 성능 저하",
            "Verdant cache failure의 결과는?",
            DialogueRelationKindIR::Consequence,
            [
                LanguageCodeIR::English,
                LanguageCodeIR::Korean,
                LanguageCodeIR::Korean,
            ],
        ),
        transfer_case(
            "XFER_KO_EN_RESULT",
            "보라 캐시 장애",
            "As a result, 보라 service degraded",
            "What resulted from 보라 캐시 장애?",
            DialogueRelationKindIR::Consequence,
            [
                LanguageCodeIR::Korean,
                LanguageCodeIR::English,
                LanguageCodeIR::English,
            ],
        ),
        transfer_case(
            "XFER_EN_KO_CONCESSION",
            "Tundra migration high cost",
            "그럼에도, Tundra 팀 배포 계속",
            "Tundra migration high cost에도 불구하고 결과?",
            DialogueRelationKindIR::Concession,
            [
                LanguageCodeIR::English,
                LanguageCodeIR::Korean,
                LanguageCodeIR::Korean,
            ],
        ),
        transfer_case(
            "XFER_KO_EN_CONCESSION",
            "이음 마이그레이션 고비용",
            "Nevertheless, 이음 team continued rollout",
            "What happened despite 이음 마이그레이션 고비용?",
            DialogueRelationKindIR::Concession,
            [
                LanguageCodeIR::Korean,
                LanguageCodeIR::English,
                LanguageCodeIR::English,
            ],
        ),
    ];

    rows.push(adversarial_case(
        "UNBOUND_EN",
        &[(
            "Because of that, the service is slow",
            LanguageCodeIR::English,
        )],
        true,
    ));
    rows.push(adversarial_case(
        "UNBOUND_KO",
        &[("그 때문에, 서비스 지연 발생", LanguageCodeIR::Korean)],
        true,
    ));
    rows.push(adversarial_case(
        "AMBIGUOUS_EN",
        &[
            (
                "Atlas cache failed. Atlas network failed.",
                LanguageCodeIR::English,
            ),
            ("Therefore, Atlas latency rose", LanguageCodeIR::English),
        ],
        true,
    ));
    rows.push(adversarial_case(
        "AMBIGUOUS_KO",
        &[
            (
                "가온 캐시 장애. 가온 네트워크 장애.",
                LanguageCodeIR::Korean,
            ),
            ("그래서, 가온 지연 증가", LanguageCodeIR::Korean),
        ],
        true,
    ));
    rows.push(adversarial_case(
        "QUOTED_EN",
        &[
            ("Atlas cache failed", LanguageCodeIR::English),
            (
                "‘Because of that, Atlas latency rose’ is a quotation",
                LanguageCodeIR::English,
            ),
        ],
        false,
    ));
    rows.push(adversarial_case(
        "QUOTED_KO",
        &[
            ("가온 캐시 장애", LanguageCodeIR::Korean),
            (
                "‘그래서 가온 지연 증가’는 인용문이다",
                LanguageCodeIR::Korean,
            ),
        ],
        false,
    ));
    rows.push(adversarial_case(
        "NON_LEADING_EN",
        &[
            ("Atlas cache failed", LanguageCodeIR::English),
            (
                "The phrase because of that appears later",
                LanguageCodeIR::English,
            ),
        ],
        false,
    ));
    rows.push(adversarial_case(
        "NON_LEADING_KO",
        &[
            ("가온 캐시 장애", LanguageCodeIR::Korean),
            ("문장 뒤에 그래서라는 표현이 있다", LanguageCodeIR::Korean),
        ],
        false,
    ));
    let mut distant_turns = vec![("Atlas cache failed", LanguageCodeIR::English)];
    distant_turns.extend(std::iter::repeat_n(("okay", LanguageCodeIR::English), 9));
    distant_turns.push((
        "Because of that, Atlas latency rose",
        LanguageCodeIR::English,
    ));
    rows.push(adversarial_case("OUT_OF_WINDOW_EN", &distant_turns, true));

    let passed = rows.iter().filter(|row| row.pass).count();
    let payload = serde_json::json!({
        "suite": "R15-RUN-0002",
        "frozen_transfer_suite": true,
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
