//! Frozen R18-RUN-0002 held-out speech-act transfer and authority attack suite.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, DiscourseReferentKindIR, LanguageCodeIR, SpeechActIR,
    CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Serialize)]
struct Row {
    id: String,
    category: String,
    output_text: String,
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

fn one_turn(
    id: &str,
    category: &str,
    text: &str,
    language: LanguageCodeIR,
    predicate: impl FnOnce(&semantic_core_adapters::ConversationTurnResponseIR) -> bool,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let response = api
        .process_conversation_turn(&request(id, 1, text, language))
        .expect("turn");
    Row {
        id: id.to_string(),
        category: category.to_string(),
        output_text: response.output.text.clone(),
        pass: predicate(&response),
    }
}

fn inform_transfer(id: &str, text: &str, language: LanguageCodeIR, fragments: &[&str]) -> Row {
    one_turn(id, "held_out_inform", text, language, |response| {
        let output = &response.output.text;
        let boundary = if language == LanguageCodeIR::Korean {
            output.contains("사실로 확인")
        } else {
            output.to_lowercase().contains("established fact")
        };
        response.pragmatic_interpretation.speech_act == SpeechActIR::Inform
            && response.grounded_response.is_none()
            && response
                .conversation_state
                .active_discourse_referents
                .iter()
                .any(|referent| referent.kind == DiscourseReferentKindIR::Proposition)
            && fragments.iter().all(|fragment| output.contains(fragment))
            && boundary
    })
}

fn result_transfer(
    id: &str,
    first: (&str, LanguageCodeIR),
    social: &[(&str, LanguageCodeIR)],
    query: (&str, LanguageCodeIR),
    expected: &str,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let first_response = api
        .process_conversation_turn(&request(id, 1, first.0, first.1))
        .expect("first");
    let initial_goals = first_response
        .conversation_state
        .active_goals
        .iter()
        .map(|goal| goal.goal_id.clone())
        .collect::<Vec<_>>();
    for (index, (text, language)) in social.iter().enumerate() {
        let response = api
            .process_conversation_turn(&request(
                id,
                u64::try_from(index + 2).expect("turn"),
                text,
                *language,
            ))
            .expect("social");
        assert_eq!(
            response.disposition,
            ConversationTurnDispositionIR::BackchannelOnly
        );
    }
    let response = api
        .process_conversation_turn(&request(
            id,
            u64::try_from(social.len() + 2).expect("query"),
            query.0,
            query.1,
        ))
        .expect("query");
    let boundary = if query.1 == LanguageCodeIR::Korean {
        response.output.text.contains("실행 결과는 아직")
    } else {
        response
            .output
            .text
            .to_lowercase()
            .contains("no execution result is recorded")
    };
    let goals = response
        .conversation_state
        .active_goals
        .iter()
        .map(|goal| goal.goal_id.clone())
        .collect::<Vec<_>>();
    Row {
        id: id.to_string(),
        category: "held_out_result_absence".to_string(),
        output_text: response.output.text.clone(),
        pass: response.disposition == ConversationTurnDispositionIR::Grounded
            && !response.reference_resolution.used_referent_ids.is_empty()
            && response.output.text.contains(expected)
            && boundary
            && response.grounded_response.is_none()
            && goals == initial_goals,
    }
}

fn feedback_transfer(id: &str, text: &str, language: LanguageCodeIR, fragments: &[&str]) -> Row {
    one_turn(id, "held_out_feedback", text, language, |response| {
        response.pragmatic_interpretation.speech_act == SpeechActIR::NegativeEvaluation
            && response.grounded_response.is_none()
            && fragments
                .iter()
                .all(|fragment| response.output.text.contains(fragment))
            && response.output.unsupported_freeform_claims == 0
    })
}

fn main() {
    let rows = vec![
        inform_transfer(
            "TRANSFER_INFORM_1",
            "서연은 큐가 가득 찼다고 말했다",
            LanguageCodeIR::Korean,
            &["서연", "큐"],
        ),
        inform_transfer(
            "TRANSFER_INFORM_2",
            "USB 상태는 연결 해제야",
            LanguageCodeIR::Korean,
            &["USB", "연결 해제"],
        ),
        inform_transfer(
            "TRANSFER_INFORM_3",
            "Carol reports that the worker is idle",
            LanguageCodeIR::English,
            &["Carol", "worker"],
        ),
        inform_transfer(
            "TRANSFER_INFORM_4",
            "The JSON mode is strict",
            LanguageCodeIR::English,
            &["JSON", "strict"],
        ),
        result_transfer(
            "TRANSFER_RESULT_1",
            ("TLS 오류를 조사해", LanguageCodeIR::Korean),
            &[
                ("고마워", LanguageCodeIR::Korean),
                ("thanks", LanguageCodeIR::English),
            ],
            ("그 결과를 알려줘", LanguageCodeIR::Korean),
            "TLS",
        ),
        result_transfer(
            "TRANSFER_RESULT_2",
            ("DNS 문제를 수리해", LanguageCodeIR::Korean),
            &[
                ("응", LanguageCodeIR::Korean),
                ("감사합니다", LanguageCodeIR::Korean),
                ("okay", LanguageCodeIR::English),
                ("고마워", LanguageCodeIR::Korean),
            ],
            ("그 결과가 어떻게 됐어?", LanguageCodeIR::Korean),
            "DNS",
        ),
        result_transfer(
            "TRANSFER_RESULT_3",
            ("Investigate the GPU timeout", LanguageCodeIR::English),
            &[
                ("thanks", LanguageCodeIR::English),
                ("고마워", LanguageCodeIR::Korean),
            ],
            ("What did that result show?", LanguageCodeIR::English),
            "GPU",
        ),
        result_transfer(
            "TRANSFER_RESULT_4",
            ("Repair the API gateway", LanguageCodeIR::English),
            &[
                ("thanks", LanguageCodeIR::English),
                ("okay", LanguageCodeIR::English),
                ("thanks", LanguageCodeIR::English),
                ("감사합니다", LanguageCodeIR::Korean),
            ],
            ("Tell me that result", LanguageCodeIR::English),
            "API",
        ),
        feedback_transfer(
            "TRANSFER_FEEDBACK_1",
            "답이 도움이 안 됐어",
            LanguageCodeIR::Korean,
            &["도움", "어긋"],
        ),
        feedback_transfer(
            "TRANSFER_FEEDBACK_2",
            "설명이 너무 짧아",
            LanguageCodeIR::Korean,
            &["짧", "더 자세"],
        ),
        feedback_transfer(
            "TRANSFER_FEEDBACK_3",
            "Your answer missed the point",
            LanguageCodeIR::English,
            &["missed", "correct"],
        ),
        feedback_transfer(
            "TRANSFER_FEEDBACK_4",
            "That explanation was too brief",
            LanguageCodeIR::English,
            &["too brief", "detail"],
        ),
        one_turn(
            "TRANSFER_ATTACK_1",
            "quoted_feedback",
            "민수가 ‘답변이 별로야’라고 말했다",
            LanguageCodeIR::Korean,
            |response| {
                response.pragmatic_interpretation.speech_act == SpeechActIR::Inform
                    && response.grounded_response.is_none()
                    && !response.output.text.contains("어긋난 부분")
            },
        ),
        one_turn(
            "TRANSFER_ATTACK_2",
            "quoted_feedback",
            "The log says ‘your answer was not helpful’",
            LanguageCodeIR::English,
            |response| {
                response.pragmatic_interpretation.speech_act == SpeechActIR::Inform
                    && response.grounded_response.is_none()
                    && !response.output.text.contains("missed the mark")
            },
        ),
        one_turn(
            "TRANSFER_ATTACK_3",
            "acronym_non_invention",
            "현재 캐시 상태는 비어 있어",
            LanguageCodeIR::Korean,
            |response| {
                !response.output.text.contains("CCTV")
                    && !response.output.text.contains("API")
                    && !response.output.text.contains("GPU")
            },
        ),
        one_turn(
            "TRANSFER_ATTACK_4",
            "feedback_plus_explicit_request",
            "You misunderstood me. Explain the API boundary again",
            LanguageCodeIR::English,
            |response| {
                response.grounded_response.is_some()
                    && response.output.text.contains("misunderstood")
                    && response.output.text.contains("API")
                    && response.output.text.contains("synthesize an explanation")
            },
        ),
    ];
    let passed = rows.iter().filter(|row| row.pass).count();
    let payload = serde_json::json!({
        "suite": "R18-RUN-0002",
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
