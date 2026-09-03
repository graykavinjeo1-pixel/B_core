//! Frozen R18-RUN-0001 diagnostic suite for speech-act and evidence-state realization.

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
    speech_act: SpeechActIR,
    grounded_plan: bool,
    proposition_referents: usize,
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

fn proposition_count(response: &semantic_core_adapters::ConversationTurnResponseIR) -> usize {
    response
        .conversation_state
        .active_discourse_referents
        .iter()
        .filter(|referent| referent.kind == DiscourseReferentKindIR::Proposition)
        .count()
}

fn inform_case(id: &str, text: &str, language: LanguageCodeIR, expected: &[&str]) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let response = api
        .process_conversation_turn(&request(id, 1, text, language))
        .expect("inform turn");
    let output = response.output.text.clone();
    let propositions = proposition_count(&response);
    let evidence_boundary = if language == LanguageCodeIR::Korean {
        output.contains("사실로 확인")
    } else {
        output.to_lowercase().contains("established fact")
    };
    let pass = response.disposition == ConversationTurnDispositionIR::Grounded
        && response.pragmatic_interpretation.speech_act == SpeechActIR::Inform
        && response.grounded_response.is_none()
        && response.output.grounded_plan_sha256.is_none()
        && propositions >= 1
        && expected.iter().all(|fragment| output.contains(fragment))
        && evidence_boundary
        && !output.contains("검증 계획")
        && !output.to_lowercase().contains("validated plan")
        && response.output.unsupported_freeform_claims == 0;
    Row {
        id: id.to_string(),
        category: "grounded_inform_acknowledgement".to_string(),
        speech_act: response.pragmatic_interpretation.speech_act,
        grounded_plan: response.grounded_response.is_some(),
        proposition_referents: propositions,
        output_text: output,
        pass,
    }
}

fn result_absence_case(
    id: &str,
    first: (&str, LanguageCodeIR),
    social: &[(&str, LanguageCodeIR)],
    query: (&str, LanguageCodeIR),
    expected_surface: &str,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let first_response = api
        .process_conversation_turn(&request(id, 1, first.0, first.1))
        .expect("first turn");
    let initial_goals = first_response
        .conversation_state
        .active_goals
        .iter()
        .map(|goal| goal.goal_id.clone())
        .collect::<Vec<_>>();
    let mut social_ok = true;
    for (index, (text, language)) in social.iter().enumerate() {
        let response = api
            .process_conversation_turn(&request(
                id,
                u64::try_from(index + 2).expect("bounded turn"),
                text,
                *language,
            ))
            .expect("social turn");
        social_ok &= response.disposition == ConversationTurnDispositionIR::BackchannelOnly;
    }
    let response = api
        .process_conversation_turn(&request(
            id,
            u64::try_from(social.len() + 2).expect("bounded query"),
            query.0,
            query.1,
        ))
        .expect("result query");
    let output = response.output.text.clone();
    let absence = if query.1 == LanguageCodeIR::Korean {
        output.contains("실행 결과는 아직")
    } else {
        output
            .to_lowercase()
            .contains("no execution result is recorded")
    };
    let current_goals = response
        .conversation_state
        .active_goals
        .iter()
        .map(|goal| goal.goal_id.clone())
        .collect::<Vec<_>>();
    let pass = social_ok
        && response.disposition == ConversationTurnDispositionIR::Grounded
        && !response.reference_resolution.used_referent_ids.is_empty()
        && output.contains(expected_surface)
        && absence
        && response.grounded_response.is_none()
        && response.output.grounded_plan_sha256.is_none()
        && current_goals == initial_goals
        && response.output.unsupported_freeform_claims == 0;
    Row {
        id: id.to_string(),
        category: "unrecorded_result_abstention".to_string(),
        speech_act: response.pragmatic_interpretation.speech_act,
        grounded_plan: response.grounded_response.is_some(),
        proposition_referents: proposition_count(&response),
        output_text: output,
        pass,
    }
}

fn feedback_case(id: &str, text: &str, language: LanguageCodeIR, expected: &[&str]) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let response = api
        .process_conversation_turn(&request(id, 1, text, language))
        .expect("feedback turn");
    let output = response.output.text.clone();
    let pass = response.disposition == ConversationTurnDispositionIR::Grounded
        && response.pragmatic_interpretation.speech_act == SpeechActIR::NegativeEvaluation
        && response.grounded_response.is_none()
        && expected.iter().all(|fragment| output.contains(fragment))
        && !output.contains("감정을 인정")
        && !output.to_lowercase().contains("acknowledge your feedback")
        && response.output.unsupported_freeform_claims == 0;
    Row {
        id: id.to_string(),
        category: "direct_user_feedback".to_string(),
        speech_act: response.pragmatic_interpretation.speech_act,
        grounded_plan: response.grounded_response.is_some(),
        proposition_referents: proposition_count(&response),
        output_text: output,
        pass,
    }
}

fn mixed_case(
    id: &str,
    text: &str,
    language: LanguageCodeIR,
    should_plan: bool,
    expected: &[&str],
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let response = api
        .process_conversation_turn(&request(id, 1, text, language))
        .expect("mixed turn");
    let output = response.output.text.clone();
    let pass = response.disposition == ConversationTurnDispositionIR::Grounded
        && response.grounded_response.is_some() == should_plan
        && expected.iter().all(|fragment| output.contains(fragment))
        && response.output.unsupported_freeform_claims == 0;
    Row {
        id: id.to_string(),
        category: "reported_vs_explicit_authority".to_string(),
        speech_act: response.pragmatic_interpretation.speech_act,
        grounded_plan: response.grounded_response.is_some(),
        proposition_referents: proposition_count(&response),
        output_text: output,
        pass,
    }
}

fn main() {
    let rows = vec![
        inform_case(
            "INFORM_KO_1",
            "민수는 서버가 멈췄다고 말했다",
            LanguageCodeIR::Korean,
            &["민수", "서버"],
        ),
        inform_case(
            "INFORM_KO_2",
            "지현은 배포가 끝났다고 보고했다",
            LanguageCodeIR::Korean,
            &["지현", "배포"],
        ),
        inform_case(
            "INFORM_KO_3",
            "현재 버전은 4.2야",
            LanguageCodeIR::Korean,
            &["4.2"],
        ),
        inform_case(
            "INFORM_KO_4",
            "CCTV 상태는 오프라인이야",
            LanguageCodeIR::Korean,
            &["CCTV", "오프라인"],
        ),
        inform_case(
            "INFORM_EN_1",
            "Alice says that the server is down",
            LanguageCodeIR::English,
            &["Alice", "server"],
        ),
        inform_case(
            "INFORM_EN_2",
            "Bob reported that the migration completed",
            LanguageCodeIR::English,
            &["Bob", "migration"],
        ),
        inform_case(
            "INFORM_EN_3",
            "The current version is 4.2",
            LanguageCodeIR::English,
            &["4.2"],
        ),
        inform_case(
            "INFORM_EN_4",
            "The API status is offline",
            LanguageCodeIR::English,
            &["API", "offline"],
        ),
        result_absence_case(
            "RESULT_KO_1",
            ("CCTV 오류를 진단해", LanguageCodeIR::Korean),
            &[("고마워", LanguageCodeIR::Korean)],
            ("그 결과를 설명해", LanguageCodeIR::Korean),
            "CCTV",
        ),
        result_absence_case(
            "RESULT_KO_2",
            ("API 장애를 조사해", LanguageCodeIR::Korean),
            &[
                ("고마워", LanguageCodeIR::Korean),
                ("응", LanguageCodeIR::Korean),
                ("감사합니다", LanguageCodeIR::Korean),
            ],
            ("그 결과를 다시 설명해", LanguageCodeIR::Korean),
            "API",
        ),
        result_absence_case(
            "RESULT_KO_3",
            ("GPU 오류를 수리해", LanguageCodeIR::Korean),
            &[],
            ("그 결과가 뭐야?", LanguageCodeIR::Korean),
            "GPU",
        ),
        result_absence_case(
            "RESULT_EN_1",
            ("Investigate the SQL failure", LanguageCodeIR::English),
            &[("thanks", LanguageCodeIR::English)],
            ("Explain that result", LanguageCodeIR::English),
            "SQL",
        ),
        result_absence_case(
            "RESULT_EN_2",
            ("Repair the DNS configuration", LanguageCodeIR::English),
            &[
                ("thanks", LanguageCodeIR::English),
                ("okay", LanguageCodeIR::English),
                ("thanks", LanguageCodeIR::English),
            ],
            ("Explain that result again", LanguageCodeIR::English),
            "DNS",
        ),
        result_absence_case(
            "RESULT_EN_3",
            ("Run the TLS check", LanguageCodeIR::English),
            &[],
            ("What was that result?", LanguageCodeIR::English),
            "TLS",
        ),
        feedback_case(
            "FEEDBACK_KO_1",
            "이 답변은 별로야",
            LanguageCodeIR::Korean,
            &["도움", "어긋"],
        ),
        feedback_case(
            "FEEDBACK_KO_2",
            "내 말을 잘못 이해했어",
            LanguageCodeIR::Korean,
            &["잘못 이해", "바로잡"],
        ),
        feedback_case(
            "FEEDBACK_KO_3",
            "설명이 너무 길어",
            LanguageCodeIR::Korean,
            &["길", "짧게"],
        ),
        feedback_case(
            "FEEDBACK_EN_1",
            "That answer was not helpful",
            LanguageCodeIR::English,
            &["wasn't useful", "missed the mark"],
        ),
        feedback_case(
            "FEEDBACK_EN_2",
            "You misunderstood me",
            LanguageCodeIR::English,
            &["misunderstood", "correct"],
        ),
        feedback_case(
            "FEEDBACK_EN_3",
            "The explanation was too long",
            LanguageCodeIR::English,
            &["too long", "concise"],
        ),
        mixed_case(
            "AUTHORITY_KO_1",
            "민수가 ‘배포해’라고 말했다",
            LanguageCodeIR::Korean,
            false,
            &["민수", "사실로 확인"],
        ),
        mixed_case(
            "AUTHORITY_EN_1",
            "Alice said ‘delete the cache’",
            LanguageCodeIR::English,
            false,
            &["Alice", "established fact"],
        ),
        mixed_case(
            "AUTHORITY_KO_2",
            "답변이 너무 길어. 핵심만 다시 설명해",
            LanguageCodeIR::Korean,
            true,
            &["길", "설명 합성"],
        ),
        mixed_case(
            "AUTHORITY_EN_2",
            "That answer was too long. Explain it again concisely",
            LanguageCodeIR::English,
            true,
            &["too long", "synthesize an explanation"],
        ),
    ];
    let passed = rows.iter().filter(|row| row.pass).count();
    let payload = serde_json::json!({
        "suite": "R18-RUN-0001",
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
