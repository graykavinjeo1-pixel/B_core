//! Frozen R19-RUN-0001 diagnostic suite for clarification-answer adjacency.

use dockable_semantic_core::PlanIntentIR;
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
    first_clarified: bool,
    answer_grounded: bool,
    qud_bound: bool,
    resolved_text: String,
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

fn has_qud_binding(response: &semantic_core_adapters::ConversationTurnResponseIR) -> bool {
    response
        .reference_resolution
        .discourse_bindings
        .iter()
        .flat_map(|binding| binding.evidence.iter())
        .any(|evidence| evidence.starts_with("PENDING_QUD:"))
}

fn row(
    id: &str,
    category: &str,
    first: &semantic_core_adapters::ConversationTurnResponseIR,
    answer: &semantic_core_adapters::ConversationTurnResponseIR,
    selected: &str,
    rejected: &str,
    expected_intent: PlanIntentIR,
) -> Row {
    let first_clarified = first.disposition == ConversationTurnDispositionIR::ClarificationRequired
        && first.grounded_response.is_none();
    let answer_grounded = answer.disposition == ConversationTurnDispositionIR::Grounded
        && answer.grounded_response.is_some();
    let qud_bound = has_qud_binding(answer);
    let grounded_intent = answer
        .grounded_response
        .as_ref()
        .map(|grounded| grounded.understanding.intent);
    let pass = first_clarified
        && answer_grounded
        && qud_bound
        && answer
            .reference_resolution
            .resolved_semantic_text
            .contains(selected)
        && !answer
            .reference_resolution
            .resolved_semantic_text
            .contains(rejected)
        && grounded_intent == Some(expected_intent)
        && answer.output.unsupported_freeform_claims == 0;
    Row {
        id: id.to_string(),
        category: category.to_string(),
        first_clarified,
        answer_grounded,
        qud_bound,
        resolved_text: answer.reference_resolution.resolved_semantic_text.clone(),
        output_text: answer.output.text.clone(),
        pass,
    }
}

#[allow(clippy::too_many_arguments)]
fn voice_case(
    id: &str,
    primary: &str,
    alternative: &str,
    answer_text: &str,
    selected: &str,
    rejected: &str,
    expected_intent: PlanIntentIR,
    language: LanguageCodeIR,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let mut first_request = request(id, 1, primary, language);
    first_request.modality = ConversationInputModalityIR::VoiceTranscript;
    first_request.input_confidence_millis = 800;
    first_request.alternatives = vec![UtteranceAlternativeIR {
        text: alternative.to_string(),
        confidence_millis: 770,
    }];
    let first = api
        .process_conversation_turn(&first_request)
        .expect("ambiguous voice turn");
    let answer = api
        .process_conversation_turn(&request(id, 2, answer_text, language))
        .expect("voice clarification answer");
    row(
        id,
        "voice_alternative_answer",
        &first,
        &answer,
        selected,
        rejected,
        expected_intent,
    )
}

fn competition_case(
    id: &str,
    first_text: &str,
    answer_text: &str,
    selected: &str,
    rejected: &str,
    expected_intent: PlanIntentIR,
    language: LanguageCodeIR,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let first = api
        .process_conversation_turn(&request(id, 1, first_text, language))
        .expect("competing request turn");
    let answer = api
        .process_conversation_turn(&request(id, 2, answer_text, language))
        .expect("competition answer");
    row(
        id,
        "competing_goal_answer",
        &first,
        &answer,
        selected,
        rejected,
        expected_intent,
    )
}

#[allow(clippy::too_many_arguments)]
fn repeat_case(
    id: &str,
    first_text: &str,
    repeat_text: &str,
    answer_text: &str,
    selected: &str,
    rejected: &str,
    expected_intent: PlanIntentIR,
    language: LanguageCodeIR,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(id, 1, first_text, language))
        .expect("multi-goal source");
    let clarification = api
        .process_conversation_turn(&request(id, 2, repeat_text, language))
        .expect("ambiguous repeat");
    let answer = api
        .process_conversation_turn(&request(id, 3, answer_text, language))
        .expect("repeat answer");
    row(
        id,
        "multi_goal_repeat_answer",
        &clarification,
        &answer,
        selected,
        rejected,
        expected_intent,
    )
}

fn proposition_case(
    id: &str,
    facts: &str,
    question: &str,
    answer_text: &str,
    selected: &str,
    rejected: &str,
    language: LanguageCodeIR,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(id, 1, facts, language))
        .expect("proposition source");
    let clarification = api
        .process_conversation_turn(&request(id, 2, question, language))
        .expect("ambiguous proposition request");
    let answer = api
        .process_conversation_turn(&request(id, 3, answer_text, language))
        .expect("proposition answer");
    row(
        id,
        "proposition_reference_answer",
        &clarification,
        &answer,
        selected,
        rejected,
        PlanIntentIR::Explain,
    )
}

fn main() {
    let rows = vec![
        voice_case(
            "QUD_VOICE_KO_1",
            "캐시를 지워",
            "가시를 지워",
            "첫 번째",
            "캐시",
            "가시",
            PlanIntentIR::Execute,
            LanguageCodeIR::Korean,
        ),
        voice_case(
            "QUD_VOICE_KO_2",
            "파일을 열어",
            "폴더를 열어",
            "두 번째",
            "폴더",
            "파일",
            PlanIntentIR::Execute,
            LanguageCodeIR::Korean,
        ),
        voice_case(
            "QUD_VOICE_KO_3",
            "배포를 확인해",
            "백업을 확인해",
            "백업 쪽",
            "백업",
            "배포",
            PlanIntentIR::Investigate,
            LanguageCodeIR::Korean,
        ),
        voice_case(
            "QUD_VOICE_EN_1",
            "repair the cache",
            "repair the cash",
            "the first one",
            "cache",
            "cash",
            PlanIntentIR::Repair,
            LanguageCodeIR::English,
        ),
        voice_case(
            "QUD_VOICE_EN_2",
            "open the file",
            "open the folder",
            "the second one",
            "folder",
            "file",
            PlanIntentIR::Execute,
            LanguageCodeIR::English,
        ),
        voice_case(
            "QUD_VOICE_EN_3",
            "inspect the queue",
            "inspect the cache",
            "the cache option",
            "cache",
            "queue",
            PlanIntentIR::Investigate,
            LanguageCodeIR::English,
        ),
        competition_case(
            "QUD_COMPETE_KO_1",
            "파일을 분석해; 코드를 수정해",
            "코드 수정 쪽",
            "코드",
            "파일",
            PlanIntentIR::Repair,
            LanguageCodeIR::Korean,
        ),
        competition_case(
            "QUD_COMPETE_KO_2",
            "API를 조사해; 캐시를 삭제해",
            "API 조사 쪽",
            "API",
            "캐시",
            PlanIntentIR::Investigate,
            LanguageCodeIR::Korean,
        ),
        competition_case(
            "QUD_COMPETE_KO_3",
            "로그를 요약해; 문서를 작성해",
            "문서 작성 쪽",
            "문서",
            "로그",
            PlanIntentIR::Create,
            LanguageCodeIR::Korean,
        ),
        competition_case(
            "QUD_COMPETE_EN_1",
            "analyze the file; repair the code",
            "the code repair option",
            "code",
            "file",
            PlanIntentIR::Repair,
            LanguageCodeIR::English,
        ),
        competition_case(
            "QUD_COMPETE_EN_2",
            "investigate the API; delete the cache",
            "the API investigation",
            "API",
            "cache",
            PlanIntentIR::Investigate,
            LanguageCodeIR::English,
        ),
        competition_case(
            "QUD_COMPETE_EN_3",
            "summarize the log; create the document",
            "the document creation",
            "document",
            "log",
            PlanIntentIR::Create,
            LanguageCodeIR::English,
        ),
        repeat_case(
            "QUD_REPEAT_KO_1",
            "파일을 읽고 저장해",
            "그대로 해",
            "저장하는 쪽",
            "저장",
            "읽",
            PlanIntentIR::Execute,
            LanguageCodeIR::Korean,
        ),
        repeat_case(
            "QUD_REPEAT_KO_2",
            "로그를 읽고 분석해",
            "똑같이 해",
            "분석하는 쪽",
            "분석",
            "읽",
            PlanIntentIR::Investigate,
            LanguageCodeIR::Korean,
        ),
        repeat_case(
            "QUD_REPEAT_KO_3",
            "캐시를 확인하고 삭제해",
            "그대로 해",
            "확인하는 쪽",
            "확인",
            "삭제",
            PlanIntentIR::Investigate,
            LanguageCodeIR::Korean,
        ),
        repeat_case(
            "QUD_REPEAT_EN_1",
            "read and save the file",
            "do that again",
            "the save action",
            "save",
            "read",
            PlanIntentIR::Execute,
            LanguageCodeIR::English,
        ),
        repeat_case(
            "QUD_REPEAT_EN_2",
            "read and analyze the log",
            "repeat that",
            "the analyze action",
            "analyze",
            "read",
            PlanIntentIR::Investigate,
            LanguageCodeIR::English,
        ),
        repeat_case(
            "QUD_REPEAT_EN_3",
            "inspect and delete the cache",
            "do that again",
            "the inspect action",
            "inspect",
            "delete",
            PlanIntentIR::Investigate,
            LanguageCodeIR::English,
        ),
        proposition_case(
            "QUD_PROP_KO_1",
            "빌드가 실패했다. 로그가 비었다.",
            "그 사실을 설명해",
            "로그가 비었다는 쪽",
            "로그",
            "빌드",
            LanguageCodeIR::Korean,
        ),
        proposition_case(
            "QUD_PROP_KO_2",
            "큐가 가득 찼다. 워커가 멈췄다.",
            "그 사실을 설명해",
            "워커가 멈췄다는 사실",
            "워커",
            "큐",
            LanguageCodeIR::Korean,
        ),
        proposition_case(
            "QUD_PROP_KO_3",
            "배포가 끝났다. 테스트가 실패했다.",
            "그 사실을 설명해",
            "테스트 실패 쪽",
            "테스트",
            "배포",
            LanguageCodeIR::Korean,
        ),
        proposition_case(
            "QUD_PROP_EN_1",
            "The build failed. The log is empty.",
            "Explain that fact",
            "the empty log fact",
            "log",
            "build",
            LanguageCodeIR::English,
        ),
        proposition_case(
            "QUD_PROP_EN_2",
            "The queue is full. The worker stopped.",
            "Explain that fact",
            "the stopped worker fact",
            "worker",
            "queue",
            LanguageCodeIR::English,
        ),
        proposition_case(
            "QUD_PROP_EN_3",
            "The rollout finished. The test failed.",
            "Explain that fact",
            "the failed test fact",
            "test",
            "rollout",
            LanguageCodeIR::English,
        ),
    ];
    let passed = rows.iter().filter(|row| row.pass).count();
    let payload = serde_json::json!({
        "suite": "R19-RUN-0001",
        "frozen_before_first_execution": true,
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
