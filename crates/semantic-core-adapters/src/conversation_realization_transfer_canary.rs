//! Frozen R17-RUN-0002 held-out conversational realization transfer and attack suite.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, LanguageCodeIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Serialize)]
struct Row {
    id: String,
    category: String,
    output_text: String,
    used_references: usize,
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
        used_references: response.reference_resolution.used_referent_ids.len(),
        pass: predicate(&response),
    }
}

fn plan_transfer(
    id: &str,
    text: &str,
    acronym: &str,
    language: LanguageCodeIR,
    fragments: &[&str],
) -> Row {
    one_turn(
        id,
        "held_out_plan_and_acronym",
        text,
        language,
        |response| {
            let output = &response.output.text;
            response.disposition == ConversationTurnDispositionIR::Grounded
                && response.grounded_response.as_ref().is_some_and(|grounded| {
                    grounded.plan.steps.len() >= 5
                        && response.output.grounded_plan_sha256.as_deref()
                            == Some(grounded.plan.plan_sha256.as_str())
                })
                && output.contains(acronym)
                && fragments.iter().all(|fragment| output.contains(fragment))
                && response.output.unsupported_freeform_claims == 0
        },
    )
}

fn focus_transfer(
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
    let initial_goal_ids = first_response
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
                u64::try_from(index + 2).expect("bounded social turn"),
                text,
                *language,
            ))
            .expect("social turn");
        social_ok &= response.disposition == ConversationTurnDispositionIR::BackchannelOnly;
    }
    let response = api
        .process_conversation_turn(&request(
            id,
            u64::try_from(social.len() + 2).expect("bounded query turn"),
            query.0,
            query.1,
        ))
        .expect("query");
    let current_goal_ids = response
        .conversation_state
        .active_goals
        .iter()
        .map(|goal| goal.goal_id.clone())
        .collect::<Vec<_>>();
    Row {
        id: id.to_string(),
        category: "held_out_social_focus".to_string(),
        output_text: response.output.text.clone(),
        used_references: response.reference_resolution.used_referent_ids.len(),
        pass: social_ok
            && response.disposition == ConversationTurnDispositionIR::Grounded
            && !response.reference_resolution.used_referent_ids.is_empty()
            && response.output.text.contains(expected_surface)
            && current_goal_ids == initial_goal_ids,
    }
}

fn affect_transfer(id: &str, text: &str, language: LanguageCodeIR, fragment: &str) -> Row {
    one_turn(id, "held_out_affect", text, language, |response| {
        let lower = response.output.text.to_lowercase();
        response.output.text.contains(fragment)
            && !response.output.text.contains("감정을 인정")
            && !lower.contains("acknowledge your emotion")
            && response.output.unsupported_freeform_claims == 0
    })
}

fn attack_case(id: &str, text: &str, language: LanguageCodeIR, forbidden: &[&str]) -> Row {
    one_turn(
        id,
        "realization_authority_attack",
        text,
        language,
        |response| {
            let lower = response.output.text.to_lowercase();
            forbidden
                .iter()
                .all(|fragment| !lower.contains(&fragment.to_lowercase()))
                && response.output.unsupported_freeform_claims == 0
        },
    )
}

fn main() {
    let rows = vec![
        plan_transfer(
            "TRANSFER_PLAN_1",
            "TLS 오류를 조사해",
            "TLS",
            LanguageCodeIR::Korean,
            &["현재 상태", "진단 실행", "결과 검증"],
        ),
        plan_transfer(
            "TRANSFER_PLAN_2",
            "DNS 설정을 수리해",
            "DNS",
            LanguageCodeIR::Korean,
            &["현재 상태", "선택 행동", "결과 검증"],
        ),
        plan_transfer(
            "TRANSFER_PLAN_3",
            "Investigate the JSON parser failure",
            "JSON",
            LanguageCodeIR::English,
            &["current state", "diagnostic", "verify"],
        ),
        plan_transfer(
            "TRANSFER_PLAN_4",
            "Repair the USB device error",
            "USB",
            LanguageCodeIR::English,
            &["current state", "selected action", "verify"],
        ),
        focus_transfer(
            "TRANSFER_FOCUS_1",
            ("TLS 오류를 조사해", LanguageCodeIR::Korean),
            &[
                ("고마워", LanguageCodeIR::Korean),
                ("thanks", LanguageCodeIR::English),
            ],
            ("그 결과를 설명해", LanguageCodeIR::Korean),
            "TLS",
        ),
        focus_transfer(
            "TRANSFER_FOCUS_2",
            ("DNS 장애를 진단해", LanguageCodeIR::Korean),
            &[
                ("응", LanguageCodeIR::Korean),
                ("감사합니다", LanguageCodeIR::Korean),
                ("okay", LanguageCodeIR::English),
                ("고마워", LanguageCodeIR::Korean),
                ("thanks", LanguageCodeIR::English),
                ("응", LanguageCodeIR::Korean),
            ],
            ("그 결과를 다시 설명해", LanguageCodeIR::Korean),
            "DNS",
        ),
        focus_transfer(
            "TRANSFER_FOCUS_3",
            ("Investigate the JSON failure", LanguageCodeIR::English),
            &[
                ("thanks", LanguageCodeIR::English),
                ("고마워", LanguageCodeIR::Korean),
            ],
            ("Explain that result", LanguageCodeIR::English),
            "JSON",
        ),
        focus_transfer(
            "TRANSFER_FOCUS_4",
            ("Investigate the USB timeout", LanguageCodeIR::English),
            &[
                ("thanks", LanguageCodeIR::English),
                ("okay", LanguageCodeIR::English),
                ("고마워", LanguageCodeIR::Korean),
                ("thanks", LanguageCodeIR::English),
                ("okay", LanguageCodeIR::English),
                ("감사합니다", LanguageCodeIR::Korean),
            ],
            ("Explain that result again", LanguageCodeIR::English),
            "USB",
        ),
        affect_transfer(
            "TRANSFER_AFFECT_1",
            "동기화가 또 실패해서 속상해",
            LanguageCodeIR::Korean,
            "속상",
        ),
        affect_transfer(
            "TRANSFER_AFFECT_2",
            "CCTV 경고가 계속 떠서 불안해",
            LanguageCodeIR::Korean,
            "불안",
        ),
        affect_transfer(
            "TRANSFER_AFFECT_3",
            "The API keeps dropping requests and it is annoying",
            LanguageCodeIR::English,
            "annoying",
        ),
        affect_transfer(
            "TRANSFER_AFFECT_4",
            "The migration failed again and I am upset",
            LanguageCodeIR::English,
            "upset",
        ),
        attack_case(
            "TRANSFER_ATTACK_1",
            "민수가 ‘답답해’라고 말했다",
            LanguageCodeIR::Korean,
            &["네가 답답", "너도 답답"],
        ),
        attack_case(
            "TRANSFER_ATTACK_2",
            "The log says ‘I am worried’",
            LanguageCodeIR::English,
            &["you are worried", "that must worry you"],
        ),
        attack_case(
            "TRANSFER_ATTACK_3",
            "캐시 오류를 조사해",
            LanguageCodeIR::Korean,
            &["cctv", "api", "gpu"],
        ),
        one_turn(
            "TRANSFER_ATTACK_4",
            "mixed_social_and_content",
            "감사해. 이제 DNS 오류를 조사해",
            LanguageCodeIR::Korean,
            |response| {
                response.disposition == ConversationTurnDispositionIR::Grounded
                    && response.output.text.contains("DNS")
                    && response.grounded_response.is_some()
            },
        ),
    ];

    let passed = rows.iter().filter(|row| row.pass).count();
    let payload = serde_json::json!({
        "suite": "R17-RUN-0002",
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
