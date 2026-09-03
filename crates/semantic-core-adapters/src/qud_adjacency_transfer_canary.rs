//! Frozen R19-RUN-0002 held-out transfer and authority suite.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, LanguageCodeIR, UtteranceAlternativeIR,
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

fn qud_bound(response: &semantic_core_adapters::ConversationTurnResponseIR) -> bool {
    response
        .reference_resolution
        .discourse_bindings
        .iter()
        .flat_map(|binding| binding.evidence.iter())
        .any(|evidence| evidence.starts_with("PENDING_QUD:"))
}

fn cross_language_voice(
    id: &str,
    primary: (&str, LanguageCodeIR),
    alternative: &str,
    answer: (&str, LanguageCodeIR),
    selected: &str,
    rejected: &str,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let mut first = request(id, 1, primary.0, primary.1);
    first.modality = ConversationInputModalityIR::VoiceTranscript;
    first.input_confidence_millis = 820;
    first.alternatives = vec![UtteranceAlternativeIR {
        text: alternative.to_string(),
        confidence_millis: 790,
    }];
    let clarification = api.process_conversation_turn(&first).expect("voice turn");
    let response = api
        .process_conversation_turn(&request(id, 2, answer.0, answer.1))
        .expect("cross-language answer");
    Row {
        id: id.to_string(),
        category: "cross_language_choice".to_string(),
        output_text: response.output.text.clone(),
        pass: clarification.disposition == ConversationTurnDispositionIR::ClarificationRequired
            && response.disposition == ConversationTurnDispositionIR::Grounded
            && response.grounded_response.is_some()
            && qud_bound(&response)
            && response
                .reference_resolution
                .resolved_semantic_text
                .contains(selected)
            && !response
                .reference_resolution
                .resolved_semantic_text
                .contains(rejected),
    }
}

fn social_preservation(
    id: &str,
    first_text: &str,
    social: (&str, LanguageCodeIR),
    answer: (&str, LanguageCodeIR),
    selected: &str,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let clarification = api
        .process_conversation_turn(&request(id, 1, first_text, LanguageCodeIR::Korean))
        .expect("competition");
    let social_response = api
        .process_conversation_turn(&request(id, 2, social.0, social.1))
        .expect("social turn");
    let response = api
        .process_conversation_turn(&request(id, 3, answer.0, answer.1))
        .expect("delayed answer");
    Row {
        id: id.to_string(),
        category: "social_turn_preserves_qud".to_string(),
        output_text: response.output.text.clone(),
        pass: clarification.disposition == ConversationTurnDispositionIR::ClarificationRequired
            && matches!(
                social_response.disposition,
                ConversationTurnDispositionIR::HoldFloor
                    | ConversationTurnDispositionIR::BackchannelOnly
            )
            && response.disposition == ConversationTurnDispositionIR::Grounded
            && response.grounded_response.is_some()
            && qud_bound(&response)
            && response
                .reference_resolution
                .resolved_semantic_text
                .contains(selected),
    }
}

fn replacement_case(
    id: &str,
    first_text: &str,
    replacement: &str,
    expected: &str,
    forbidden: &str,
    language: LanguageCodeIR,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let clarification = api
        .process_conversation_turn(&request(id, 1, first_text, language))
        .expect("competition");
    let response = api
        .process_conversation_turn(&request(id, 2, replacement, language))
        .expect("replacement request");
    Row {
        id: id.to_string(),
        category: "explicit_new_request_cancels_qud".to_string(),
        output_text: response.output.text.clone(),
        pass: clarification.disposition == ConversationTurnDispositionIR::ClarificationRequired
            && response.disposition == ConversationTurnDispositionIR::Grounded
            && response.grounded_response.is_some()
            && !qud_bound(&response)
            && response
                .reference_resolution
                .resolved_semantic_text
                .contains(expected)
            && !response
                .reference_resolution
                .resolved_semantic_text
                .contains(forbidden),
    }
}

fn fail_closed_case(
    id: &str,
    first_text: &str,
    invalid_answer: &str,
    language: LanguageCodeIR,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let clarification = api
        .process_conversation_turn(&request(id, 1, first_text, language))
        .expect("competition");
    let response = api
        .process_conversation_turn(&request(id, 2, invalid_answer, language))
        .expect("invalid answer");
    Row {
        id: id.to_string(),
        category: "non_authoritative_or_invalid_answer".to_string(),
        output_text: response.output.text.clone(),
        pass: clarification.disposition == ConversationTurnDispositionIR::ClarificationRequired
            && response.disposition == ConversationTurnDispositionIR::ClarificationRequired
            && response.grounded_response.is_none()
            && !qud_bound(&response),
    }
}

fn main() {
    let rows = vec![
        cross_language_voice(
            "QUD_TRANSFER_XLANG_1",
            ("파일을 열어", LanguageCodeIR::Korean),
            "폴더를 열어",
            ("the second one", LanguageCodeIR::English),
            "folder",
            "file",
        ),
        cross_language_voice(
            "QUD_TRANSFER_XLANG_2",
            ("repair the cache", LanguageCodeIR::English),
            "repair the queue",
            ("첫 번째", LanguageCodeIR::Korean),
            "cache",
            "queue",
        ),
        cross_language_voice(
            "QUD_TRANSFER_XLANG_3",
            ("로그를 확인해", LanguageCodeIR::Korean),
            "백업을 확인해",
            ("the backup option", LanguageCodeIR::English),
            "backup",
            "log",
        ),
        cross_language_voice(
            "QUD_TRANSFER_XLANG_4",
            ("inspect the worker", LanguageCodeIR::English),
            "inspect the server",
            ("두 번째", LanguageCodeIR::Korean),
            "server",
            "worker",
        ),
        social_preservation(
            "QUD_TRANSFER_SOCIAL_1",
            "파일을 분석해; 코드를 수정해",
            ("음...", LanguageCodeIR::Korean),
            ("코드 수정 쪽", LanguageCodeIR::Korean),
            "코드",
        ),
        social_preservation(
            "QUD_TRANSFER_SOCIAL_2",
            "API를 조사해; 캐시를 삭제해",
            ("잠깐", LanguageCodeIR::Korean),
            ("API 조사 쪽", LanguageCodeIR::Korean),
            "API",
        ),
        social_preservation(
            "QUD_TRANSFER_SOCIAL_3",
            "로그를 요약해; 문서를 작성해",
            ("thanks", LanguageCodeIR::English),
            ("문서 작성 쪽", LanguageCodeIR::Korean),
            "문서",
        ),
        social_preservation(
            "QUD_TRANSFER_SOCIAL_4",
            "큐를 확인해; 서버를 수리해",
            ("어...", LanguageCodeIR::Korean),
            ("서버 수리 쪽", LanguageCodeIR::Korean),
            "서버",
        ),
        replacement_case(
            "QUD_TRANSFER_REPLACE_1",
            "파일을 분석해; 코드를 수정해",
            "새로 백업을 검사해",
            "백업",
            "코드",
            LanguageCodeIR::Korean,
        ),
        replacement_case(
            "QUD_TRANSFER_REPLACE_2",
            "analyze the file; repair the code",
            "inspect the new queue",
            "queue",
            "code",
            LanguageCodeIR::English,
        ),
        replacement_case(
            "QUD_TRANSFER_REPLACE_3",
            "API를 조사해; 캐시를 삭제해",
            "로그를 읽어",
            "로그",
            "캐시",
            LanguageCodeIR::Korean,
        ),
        replacement_case(
            "QUD_TRANSFER_REPLACE_4",
            "summarize the log; create the document",
            "repair the worker",
            "worker",
            "document",
            LanguageCodeIR::English,
        ),
        fail_closed_case(
            "QUD_TRANSFER_SAFE_1",
            "파일을 분석해; 코드를 수정해",
            "세 번째",
            LanguageCodeIR::Korean,
        ),
        fail_closed_case(
            "QUD_TRANSFER_SAFE_2",
            "analyze the file; repair the code",
            "maybe the second one",
            LanguageCodeIR::English,
        ),
        fail_closed_case(
            "QUD_TRANSFER_SAFE_3",
            "API를 조사해; 캐시를 삭제해",
            "민수는 첫 번째라고 말했다",
            LanguageCodeIR::Korean,
        ),
        fail_closed_case(
            "QUD_TRANSFER_SAFE_4",
            "summarize the log; create the document",
            "Alice said the first one",
            LanguageCodeIR::English,
        ),
    ];
    let passed = rows.iter().filter(|row| row.pass).count();
    let payload = serde_json::json!({
        "suite": "R19-RUN-0002",
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
