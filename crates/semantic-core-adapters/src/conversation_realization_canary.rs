//! Frozen R17-RUN-0001 diagnostic suite for grounded conversational realization.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, LanguageCodeIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Serialize)]
struct Row {
    id: String,
    category: String,
    disposition: ConversationTurnDispositionIR,
    plan_steps: usize,
    used_references: usize,
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

fn plan_case(id: &str, text: &str, language: LanguageCodeIR, required_fragments: &[&str]) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let response = api
        .process_conversation_turn(&request(id, 1, text, language))
        .expect("plan turn");
    let plan_steps = response
        .grounded_response
        .as_ref()
        .map_or(0, |grounded| grounded.plan.steps.len());
    let output = response.output.text.clone();
    let plan_hash_matches = response.grounded_response.as_ref().is_some_and(|grounded| {
        response.output.grounded_plan_sha256.as_deref() == Some(grounded.plan.plan_sha256.as_str())
    });
    let pass = response.disposition == ConversationTurnDispositionIR::Grounded
        && plan_steps >= 5
        && plan_hash_matches
        && required_fragments
            .iter()
            .all(|fragment| output.contains(fragment))
        && !output.contains("단계별로 안내")
        && !output.contains("step by step")
        && response.output.unsupported_freeform_claims == 0;
    Row {
        id: id.to_string(),
        category: "grounded_plan_realization".to_string(),
        disposition: response.disposition,
        plan_steps,
        used_references: response.reference_resolution.used_referent_ids.len(),
        output_text: output,
        pass,
    }
}

fn acronym_case(id: &str, text: &str, acronym: &str, language: LanguageCodeIR) -> Row {
    let mut row = plan_case(
        id,
        text,
        language,
        if language == LanguageCodeIR::Korean {
            &["현재", "검증"]
        } else {
            &["current", "verify"]
        },
    );
    row.category = "input_grounded_acronym".to_string();
    row.pass &= row.output_text.contains(acronym);
    row
}

fn continuity_case(
    id: &str,
    first: &str,
    social_turns: &[&str],
    query: &str,
    expected_surface: &str,
    language: LanguageCodeIR,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let first_response = api
        .process_conversation_turn(&request(id, 1, first, language))
        .expect("first turn");
    let initial_goals = first_response.conversation_state.active_goals.len();
    let mut all_social = true;
    for (index, text) in social_turns.iter().enumerate() {
        let response = api
            .process_conversation_turn(&request(
                id,
                u64::try_from(index + 2).expect("bounded turn"),
                text,
                language,
            ))
            .expect("social turn");
        all_social &= response.disposition == ConversationTurnDispositionIR::BackchannelOnly;
    }
    let query_turn = u64::try_from(social_turns.len() + 2).expect("bounded query turn");
    let response = api
        .process_conversation_turn(&request(id, query_turn, query, language))
        .expect("reference turn");
    let output = response.output.text.clone();
    let pass = all_social
        && response.disposition == ConversationTurnDispositionIR::Grounded
        && !response.reference_resolution.used_referent_ids.is_empty()
        && response.conversation_state.active_goals.len() == initial_goals
        && output.contains(expected_surface)
        && !output.contains("‘의 결과’")
        && !output.contains("‘the result’");
    Row {
        id: id.to_string(),
        category: "social_turn_focus_continuity".to_string(),
        disposition: response.disposition,
        plan_steps: response
            .grounded_response
            .as_ref()
            .map_or(0, |grounded| grounded.plan.steps.len()),
        used_references: response.reference_resolution.used_referent_ids.len(),
        output_text: output,
        pass,
    }
}

fn affect_case(id: &str, text: &str, language: LanguageCodeIR, natural_fragment: &str) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let response = api
        .process_conversation_turn(&request(id, 1, text, language))
        .expect("affect turn");
    let output = response.output.text.clone();
    let lower = output.to_lowercase();
    let pass = output.contains(natural_fragment)
        && !output.contains("감정을 인정")
        && !output.contains("감정을 인식")
        && !lower.contains("acknowledge your emotion")
        && !lower.contains("recognize your emotion")
        && response.output.unsupported_freeform_claims == 0;
    Row {
        id: id.to_string(),
        category: "direct_affect_realization".to_string(),
        disposition: response.disposition,
        plan_steps: response
            .grounded_response
            .as_ref()
            .map_or(0, |grounded| grounded.plan.steps.len()),
        used_references: response.reference_resolution.used_referent_ids.len(),
        output_text: output,
        pass,
    }
}

fn main() {
    let rows = vec![
        plan_case(
            "PLAN_KO_1",
            "Nimbus 오류를 수리해",
            LanguageCodeIR::Korean,
            &["현재 상태", "선택 행동", "결과 검증"],
        ),
        plan_case(
            "PLAN_KO_2",
            "Atlas 장애 원인을 조사해",
            LanguageCodeIR::Korean,
            &["현재 상태", "진단 실행", "결과 검증"],
        ),
        plan_case(
            "PLAN_KO_3",
            "Beryl 설정 파일을 만들어",
            LanguageCodeIR::Korean,
            &["완료 조건", "선택 행동", "결과 검증"],
        ),
        plan_case(
            "PLAN_KO_4",
            "Cinder 작업을 실행해",
            LanguageCodeIR::Korean,
            &["현재 상태", "선택 행동", "결과 검증"],
        ),
        plan_case(
            "PLAN_KO_5",
            "Delta API 사용법을 학습해",
            LanguageCodeIR::Korean,
            &["지식 공백", "교훈", "결과 검증"],
        ),
        plan_case(
            "PLAN_KO_6",
            "Ember 캐시 구조를 설명해",
            LanguageCodeIR::Korean,
            &["현재 상태", "설명 합성", "결과 전달"],
        ),
        plan_case(
            "PLAN_EN_1",
            "Repair the Nimbus cache fault",
            LanguageCodeIR::English,
            &["current state", "selected action", "verify"],
        ),
        plan_case(
            "PLAN_EN_2",
            "Investigate the Atlas service failure",
            LanguageCodeIR::English,
            &["current state", "diagnostic", "verify"],
        ),
        plan_case(
            "PLAN_EN_3",
            "Create the Beryl configuration file",
            LanguageCodeIR::English,
            &["completion conditions", "selected action", "verify"],
        ),
        plan_case(
            "PLAN_EN_4",
            "Execute the Cinder migration",
            LanguageCodeIR::English,
            &["current state", "selected action", "verify"],
        ),
        plan_case(
            "PLAN_EN_5",
            "Learn how the Delta API works",
            LanguageCodeIR::English,
            &["knowledge gap", "lesson", "verify"],
        ),
        plan_case(
            "PLAN_EN_6",
            "Explain the Ember cache architecture",
            LanguageCodeIR::English,
            &["current state", "explanation", "result"],
        ),
        acronym_case(
            "ACRONYM_1",
            "CCTV 오류를 진단해",
            "CCTV",
            LanguageCodeIR::Korean,
        ),
        acronym_case(
            "ACRONYM_2",
            "API 응답 지연을 조사해",
            "API",
            LanguageCodeIR::Korean,
        ),
        acronym_case(
            "ACRONYM_3",
            "Diagnose the GPU timeout",
            "GPU",
            LanguageCodeIR::English,
        ),
        acronym_case(
            "ACRONYM_4",
            "Investigate the SQL connection failure",
            "SQL",
            LanguageCodeIR::English,
        ),
        continuity_case(
            "FOCUS_1",
            "CCTV 오류를 진단해",
            &["고마워"],
            "그 결과를 다시 설명해",
            "CCTV",
            LanguageCodeIR::Korean,
        ),
        continuity_case(
            "FOCUS_2",
            "API 장애를 조사해",
            &["고마워", "응", "감사합니다", "응", "고마워"],
            "그 결과를 다시 설명해",
            "API",
            LanguageCodeIR::Korean,
        ),
        continuity_case(
            "FOCUS_3",
            "Investigate the GPU timeout",
            &["thanks"],
            "Explain that result again",
            "GPU",
            LanguageCodeIR::English,
        ),
        continuity_case(
            "FOCUS_4",
            "Investigate the SQL failure",
            &["thanks", "okay", "thanks", "okay", "thanks"],
            "Explain that result again",
            "SQL",
            LanguageCodeIR::English,
        ),
        affect_case(
            "AFFECT_1",
            "CCTV가 계속 끊겨서 정말 답답해",
            LanguageCodeIR::Korean,
            "답답할",
        ),
        affect_case(
            "AFFECT_2",
            "배포가 또 실패해서 화나",
            LanguageCodeIR::Korean,
            "화날",
        ),
        affect_case(
            "AFFECT_3",
            "This API keeps timing out and it is frustrating",
            LanguageCodeIR::English,
            "frustrating",
        ),
        affect_case(
            "AFFECT_4",
            "The build failed again and I am worried",
            LanguageCodeIR::English,
            "worry",
        ),
    ];

    let passed = rows.iter().filter(|row| row.pass).count();
    let payload = serde_json::json!({
        "suite": "R17-RUN-0001",
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
