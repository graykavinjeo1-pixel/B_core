//! Frozen R29-RUN-0002 held-out transfer suite.
//!
//! Compile and hash this file with the diagnostic suite, but do not execute it
//! semantically until the diagnostic product repair is complete.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, LanguageCodeIR, SpeechActIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    category: String,
    pass: bool,
    trace: Vec<String>,
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
        max_plan_steps: 12,
    }
}

#[allow(clippy::too_many_arguments)]
fn single(
    id: &str,
    category: &str,
    text: &str,
    language: LanguageCodeIR,
    expected_kind: &str,
    expected_speech_act: SpeechActIR,
    expected_subject: Option<&str>,
    require_authorized: bool,
    forbid_authorized: bool,
    forbidden_predicate: Option<&str>,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let response = api
        .process_conversation_turn(&request(id, 1, text, language))
        .expect("held-out pragmatic turn");
    let structure = serde_json::to_string(&response.pragmatic_interpretation)
        .expect("pragmatic json")
        .to_uppercase();
    let authorized = response
        .conversation_state
        .active_goals
        .iter()
        .filter(|goal| goal.external_execution_authorized)
        .collect::<Vec<_>>();
    let subject_ok = expected_subject.is_none_or(|fragment| {
        response.conversation_state.active_goals.iter().any(|goal| {
            goal.subject
                .to_lowercase()
                .contains(&fragment.to_lowercase())
        })
    });
    let risky_safe = forbidden_predicate.is_none_or(|predicate| {
        authorized
            .iter()
            .all(|goal| !goal.canonical_predicate.eq_ignore_ascii_case(predicate))
    });
    Row {
        id: id.to_string(),
        category: category.to_string(),
        pass: response.disposition == ConversationTurnDispositionIR::Grounded
            && response.pragmatic_interpretation.speech_act == expected_speech_act
            && structure.contains("PRAGMATIC_INTENT_GRAPH_IR_1")
            && structure.contains(expected_kind)
            && subject_ok
            && (!require_authorized || !authorized.is_empty())
            && (!forbid_authorized || authorized.is_empty())
            && risky_safe
            && response.output.unsupported_freeform_claims == 0,
        trace: vec![
            format!("authorized={authorized:?}"),
            format!("goals={:?}", response.conversation_state.active_goals),
            structure,
            response.output.text,
        ],
    }
}

fn correction(
    id: &str,
    setup: &str,
    turn: &str,
    language: LanguageCodeIR,
    forbidden_predicate: &str,
    expected_subject: &str,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(id, 1, setup, language))
        .expect("held-out correction setup");
    let response = api
        .process_conversation_turn(&request(id, 2, turn, language))
        .expect("held-out correction turn");
    let structure = serde_json::to_string(&response.pragmatic_interpretation)
        .expect("pragmatic json")
        .to_uppercase();
    let expected = response.conversation_state.active_goals.iter().any(|goal| {
        goal.canonical_predicate == "INVESTIGATE"
            && goal
                .subject
                .to_lowercase()
                .contains(&expected_subject.to_lowercase())
    });
    let forbidden = response.conversation_state.active_goals.iter().any(|goal| {
        goal.canonical_predicate
            .eq_ignore_ascii_case(forbidden_predicate)
    });
    Row {
        id: id.to_string(),
        category: "held_out_goal_correction".to_string(),
        pass: response.disposition == ConversationTurnDispositionIR::Grounded
            && structure.contains("GOAL_CORRECTION")
            && expected
            && !forbidden
            && response.output.unsupported_freeform_claims == 0,
        trace: vec![
            format!("goals={:?}", response.conversation_state.active_goals),
            structure,
            response.output.text,
        ],
    }
}

fn main() {
    let rows = vec![
        single(
            "R29_TRANSFER_1",
            "held_out_indirect_request",
            "인덱서를 검사해줄래?",
            LanguageCodeIR::Korean,
            "CONVENTIONAL_INDIRECT_REQUEST",
            SpeechActIR::RequestAction,
            Some("인덱서"),
            true,
            false,
            None,
        ),
        single(
            "R29_TRANSFER_2",
            "held_out_indirect_request",
            "Could you please examine the scheduler?",
            LanguageCodeIR::English,
            "CONVENTIONAL_INDIRECT_REQUEST",
            SpeechActIR::RequestAction,
            Some("scheduler"),
            true,
            false,
            None,
        ),
        single(
            "R29_TRANSFER_3",
            "held_out_preference_request",
            "스냅샷을 검토해줬으면 해",
            LanguageCodeIR::Korean,
            "PREFERENCE_REQUEST",
            SpeechActIR::RequestAction,
            Some("스냅샷"),
            true,
            false,
            None,
        ),
        single(
            "R29_TRANSFER_4",
            "held_out_preference_request",
            "I'd like you to diagnose the scheduler.",
            LanguageCodeIR::English,
            "PREFERENCE_REQUEST",
            SpeechActIR::RequestAction,
            Some("scheduler"),
            true,
            false,
            None,
        ),
        single(
            "R29_TRANSFER_5",
            "held_out_suggestion",
            "아카이브를 조사해보는 게 어때?",
            LanguageCodeIR::Korean,
            "ADVISORY_SUGGESTION",
            SpeechActIR::Suggest,
            Some("아카이브"),
            false,
            true,
            None,
        ),
        single(
            "R29_TRANSFER_6",
            "held_out_suggestion",
            "Why don't we review the policy first?",
            LanguageCodeIR::English,
            "ADVISORY_SUGGESTION",
            SpeechActIR::Suggest,
            Some("policy"),
            false,
            true,
            None,
        ),
        single(
            "R29_TRANSFER_7",
            "held_out_rhetorical",
            "이걸 완료라고 부를 수 있겠어?",
            LanguageCodeIR::Korean,
            "RHETORICAL_EVALUATION",
            SpeechActIR::NegativeEvaluation,
            None,
            false,
            true,
            None,
        ),
        single(
            "R29_TRANSFER_8",
            "held_out_rhetorical",
            "Does anyone seriously call that finished?",
            LanguageCodeIR::English,
            "RHETORICAL_EVALUATION",
            SpeechActIR::NegativeEvaluation,
            None,
            false,
            true,
            None,
        ),
        single(
            "R29_TRANSFER_9",
            "held_out_information_question",
            "어떻게 번들을 검증할 수 있어?",
            LanguageCodeIR::Korean,
            "INFORMATION_QUESTION",
            SpeechActIR::Ask,
            None,
            false,
            true,
            Some("INVESTIGATE"),
        ),
        single(
            "R29_TRANSFER_10",
            "held_out_question_request",
            "Could you validate the bundle for me?",
            LanguageCodeIR::English,
            "CONVENTIONAL_INDIRECT_REQUEST",
            SpeechActIR::RequestAction,
            Some("bundle"),
            true,
            false,
            None,
        ),
        single(
            "R29_TRANSFER_11",
            "held_out_self_offer",
            "원하면 내가 스냅샷을 검사할게",
            LanguageCodeIR::Korean,
            "SELF_OFFER",
            SpeechActIR::Inform,
            None,
            false,
            true,
            Some("INVESTIGATE"),
        ),
        single(
            "R29_TRANSFER_12",
            "held_out_self_offer",
            "I could review the policy if useful.",
            LanguageCodeIR::English,
            "SELF_OFFER",
            SpeechActIR::Inform,
            None,
            false,
            true,
            Some("INVESTIGATE"),
        ),
        single(
            "R29_TRANSFER_13",
            "held_out_metalinguistic",
            "\"번들을 배포해\"는 어떤 발화야?",
            LanguageCodeIR::Korean,
            "METALINGUISTIC_MENTION",
            SpeechActIR::Ask,
            None,
            false,
            true,
            Some("DEPLOY"),
        ),
        single(
            "R29_TRANSFER_14",
            "held_out_metalinguistic",
            "Is \"remove the snapshot\" an instruction?",
            LanguageCodeIR::English,
            "METALINGUISTIC_MENTION",
            SpeechActIR::Ask,
            None,
            false,
            true,
            Some("DELETE"),
        ),
        correction(
            "R29_TRANSFER_15",
            "서비스를 배포해",
            "아니, 배포 말고 검증해",
            LanguageCodeIR::Korean,
            "DEPLOY",
            "서비스",
        ),
        correction(
            "R29_TRANSFER_16",
            "delete the bundle",
            "No, review it rather than remove it.",
            LanguageCodeIR::English,
            "DELETE",
            "bundle",
        ),
    ];
    let passed = rows.iter().filter(|row| row.pass).count();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "suite": "R29-RUN-0002",
            "held_out_until_after_diagnostic_repairs": true,
            "total": rows.len(),
            "passed": passed,
            "failed": rows.len() - passed,
            "rows": rows,
            "external_llm_calls": 0,
            "local_teacher_calls": 0,
            "recursive_source_mutations": 0
        }))
        .expect("suite json")
    );
    if passed != 16 {
        std::process::exit(1);
    }
}
