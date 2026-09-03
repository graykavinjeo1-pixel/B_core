//! Frozen R29-RUN-0001 diagnostic suite.
//!
//! This public-API suite is frozen before R29 product changes. It measures
//! pragmatic force independently of surface sentence mood and forbids
//! quoted, rhetorical, informational, or self-directed language from
//! authorizing an assistant action.

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
    require_authorized_goal: bool,
    forbid_authorized_goal: bool,
    forbidden_predicate: Option<&str>,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let response = api
        .process_conversation_turn(&request(id, 1, text, language))
        .expect("pragmatic diagnostic turn");
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
        response.conversation_state.active_goals.iter().all(|goal| {
            !goal.canonical_predicate.eq_ignore_ascii_case(predicate)
                || !goal.external_execution_authorized
        })
    });
    Row {
        id: id.to_string(),
        category: category.to_string(),
        pass: response.disposition == ConversationTurnDispositionIR::Grounded
            && response.pragmatic_interpretation.speech_act == expected_speech_act
            && structure.contains("PRAGMATIC_INTENT_GRAPH_IR_1")
            && structure.contains(expected_kind)
            && subject_ok
            && (!require_authorized_goal || !authorized.is_empty())
            && (!forbid_authorized_goal || authorized.is_empty())
            && risky_safe
            && response.output.unsupported_freeform_claims == 0,
        trace: vec![
            format!(
                "speech_act={:?}",
                response.pragmatic_interpretation.speech_act
            ),
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
    correction_text: &str,
    language: LanguageCodeIR,
    expected_predicate: &str,
    forbidden_predicate: &str,
    expected_subject: &str,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let before = api
        .process_conversation_turn(&request(id, 1, setup, language))
        .expect("correction setup");
    let response = api
        .process_conversation_turn(&request(id, 2, correction_text, language))
        .expect("correction turn");
    let structure = serde_json::to_string(&response.pragmatic_interpretation)
        .expect("pragmatic json")
        .to_uppercase();
    let expected = response.conversation_state.active_goals.iter().any(|goal| {
        goal.canonical_predicate
            .eq_ignore_ascii_case(expected_predicate)
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
        category: "goal_correction".to_string(),
        pass: !before.conversation_state.active_goals.is_empty()
            && response.disposition == ConversationTurnDispositionIR::Grounded
            && response.pragmatic_interpretation.speech_act == SpeechActIR::RequestAction
            && structure.contains("PRAGMATIC_INTENT_GRAPH_IR_1")
            && structure.contains("GOAL_CORRECTION")
            && expected
            && !forbidden
            && response.output.unsupported_freeform_claims == 0,
        trace: vec![
            format!("before={:?}", before.conversation_state.active_goals),
            format!("after={:?}", response.conversation_state.active_goals),
            structure,
            response.output.text,
        ],
    }
}

fn main() {
    let rows = vec![
        single(
            "R29_INDIRECT_1",
            "conventional_indirect_request",
            "파서를 고쳐줄래?",
            LanguageCodeIR::Korean,
            "CONVENTIONAL_INDIRECT_REQUEST",
            SpeechActIR::RequestAction,
            Some("파서"),
            true,
            false,
            None,
        ),
        single(
            "R29_INDIRECT_2",
            "conventional_indirect_request",
            "Would you mind inspecting the queue?",
            LanguageCodeIR::English,
            "CONVENTIONAL_INDIRECT_REQUEST",
            SpeechActIR::RequestAction,
            Some("queue"),
            true,
            false,
            None,
        ),
        single(
            "R29_INDIRECT_3",
            "conventional_indirect_request",
            "캐시를 분석해주면 안 될까?",
            LanguageCodeIR::Korean,
            "CONVENTIONAL_INDIRECT_REQUEST",
            SpeechActIR::RequestAction,
            Some("캐시"),
            true,
            false,
            None,
        ),
        single(
            "R29_INDIRECT_4",
            "conventional_indirect_request",
            "Shouldn't you repair the worker?",
            LanguageCodeIR::English,
            "CONVENTIONAL_INDIRECT_REQUEST",
            SpeechActIR::RequestAction,
            Some("worker"),
            true,
            false,
            None,
        ),
        single(
            "R29_PREFERENCE_1",
            "preference_request",
            "로그를 먼저 확인해줬으면 좋겠어",
            LanguageCodeIR::Korean,
            "PREFERENCE_REQUEST",
            SpeechActIR::RequestAction,
            Some("로그"),
            true,
            false,
            None,
        ),
        single(
            "R29_PREFERENCE_2",
            "preference_request",
            "I would like you to inspect the manifest.",
            LanguageCodeIR::English,
            "PREFERENCE_REQUEST",
            SpeechActIR::RequestAction,
            Some("manifest"),
            true,
            false,
            None,
        ),
        single(
            "R29_PREFERENCE_3",
            "preference_request",
            "가능하면 설정을 분석해줬으면 해",
            LanguageCodeIR::Korean,
            "PREFERENCE_REQUEST",
            SpeechActIR::RequestAction,
            Some("설정"),
            true,
            false,
            None,
        ),
        single(
            "R29_PREFERENCE_4",
            "preference_request",
            "I'd prefer you to repair the dispatcher.",
            LanguageCodeIR::English,
            "PREFERENCE_REQUEST",
            SpeechActIR::RequestAction,
            Some("dispatcher"),
            true,
            false,
            None,
        ),
        single(
            "R29_SUGGEST_1",
            "advisory_suggestion",
            "우선 로그를 확인하는 게 어때?",
            LanguageCodeIR::Korean,
            "ADVISORY_SUGGESTION",
            SpeechActIR::Suggest,
            Some("로그"),
            false,
            true,
            None,
        ),
        single(
            "R29_SUGGEST_2",
            "advisory_suggestion",
            "How about inspecting the cache first?",
            LanguageCodeIR::English,
            "ADVISORY_SUGGESTION",
            SpeechActIR::Suggest,
            Some("cache"),
            false,
            true,
            None,
        ),
        single(
            "R29_SUGGEST_3",
            "advisory_suggestion",
            "설정을 분석해보는 건 어떨까?",
            LanguageCodeIR::Korean,
            "ADVISORY_SUGGESTION",
            SpeechActIR::Suggest,
            Some("설정"),
            false,
            true,
            None,
        ),
        single(
            "R29_SUGGEST_4",
            "advisory_suggestion",
            "Maybe we should repair the parser.",
            LanguageCodeIR::English,
            "ADVISORY_SUGGESTION",
            SpeechActIR::Suggest,
            Some("parser"),
            false,
            true,
            None,
        ),
        single(
            "R29_RHETORICAL_1",
            "rhetorical_evaluation",
            "누가 이 결과를 성공이라고 하겠어?",
            LanguageCodeIR::Korean,
            "RHETORICAL_EVALUATION",
            SpeechActIR::NegativeEvaluation,
            None,
            false,
            true,
            None,
        ),
        single(
            "R29_RHETORICAL_2",
            "rhetorical_evaluation",
            "Who would call this a success?",
            LanguageCodeIR::English,
            "RHETORICAL_EVALUATION",
            SpeechActIR::NegativeEvaluation,
            None,
            false,
            true,
            None,
        ),
        single(
            "R29_RHETORICAL_3",
            "rhetorical_evaluation",
            "이게 제대로 고친 거라고?",
            LanguageCodeIR::Korean,
            "RHETORICAL_EVALUATION",
            SpeechActIR::NegativeEvaluation,
            None,
            false,
            true,
            Some("REPAIR"),
        ),
        single(
            "R29_RHETORICAL_4",
            "rhetorical_evaluation",
            "You call this a repair?",
            LanguageCodeIR::English,
            "RHETORICAL_EVALUATION",
            SpeechActIR::NegativeEvaluation,
            None,
            false,
            true,
            Some("REPAIR"),
        ),
        single(
            "R29_QUESTION_1",
            "information_question",
            "로그를 어디서 확인할 수 있어?",
            LanguageCodeIR::Korean,
            "INFORMATION_QUESTION",
            SpeechActIR::Ask,
            None,
            false,
            true,
            Some("INVESTIGATE"),
        ),
        single(
            "R29_QUESTION_2",
            "information_question",
            "Where can I inspect the manifest?",
            LanguageCodeIR::English,
            "INFORMATION_QUESTION",
            SpeechActIR::Ask,
            None,
            false,
            true,
            Some("INVESTIGATE"),
        ),
        single(
            "R29_QUESTION_3",
            "question_actor_boundary",
            "로그를 확인해줄래?",
            LanguageCodeIR::Korean,
            "CONVENTIONAL_INDIRECT_REQUEST",
            SpeechActIR::RequestAction,
            Some("로그"),
            true,
            false,
            None,
        ),
        single(
            "R29_QUESTION_4",
            "question_actor_boundary",
            "Would you inspect the manifest for me?",
            LanguageCodeIR::English,
            "CONVENTIONAL_INDIRECT_REQUEST",
            SpeechActIR::RequestAction,
            Some("manifest"),
            true,
            false,
            None,
        ),
        single(
            "R29_OFFER_1",
            "self_offer",
            "필요하면 내가 로그를 확인할게",
            LanguageCodeIR::Korean,
            "SELF_OFFER",
            SpeechActIR::Inform,
            None,
            false,
            true,
            Some("INVESTIGATE"),
        ),
        single(
            "R29_OFFER_2",
            "self_offer",
            "I can inspect the cache if needed.",
            LanguageCodeIR::English,
            "SELF_OFFER",
            SpeechActIR::Inform,
            None,
            false,
            true,
            Some("INVESTIGATE"),
        ),
        single(
            "R29_OFFER_3",
            "self_offer",
            "내가 파서를 고칠까?",
            LanguageCodeIR::Korean,
            "SELF_OFFER",
            SpeechActIR::Inform,
            None,
            false,
            true,
            Some("REPAIR"),
        ),
        single(
            "R29_OFFER_4",
            "self_offer",
            "Shall I repair the worker?",
            LanguageCodeIR::English,
            "SELF_OFFER",
            SpeechActIR::Inform,
            None,
            false,
            true,
            Some("REPAIR"),
        ),
        single(
            "R29_META_1",
            "metalinguistic_mention",
            "\"캐시를 삭제해\"라는 문장은 무슨 뜻이야?",
            LanguageCodeIR::Korean,
            "METALINGUISTIC_MENTION",
            SpeechActIR::Ask,
            None,
            false,
            true,
            Some("DELETE"),
        ),
        single(
            "R29_META_2",
            "metalinguistic_mention",
            "What does \"delete the archive\" mean?",
            LanguageCodeIR::English,
            "METALINGUISTIC_MENTION",
            SpeechActIR::Ask,
            None,
            false,
            true,
            Some("DELETE"),
        ),
        single(
            "R29_META_3",
            "metalinguistic_mention",
            "\"파서를 고쳐\"는 요청이야?",
            LanguageCodeIR::Korean,
            "METALINGUISTIC_MENTION",
            SpeechActIR::Ask,
            None,
            false,
            true,
            Some("REPAIR"),
        ),
        single(
            "R29_META_4",
            "metalinguistic_mention",
            "Is \"repair the worker\" a command?",
            LanguageCodeIR::English,
            "METALINGUISTIC_MENTION",
            SpeechActIR::Ask,
            None,
            false,
            true,
            Some("REPAIR"),
        ),
        correction(
            "R29_CORRECT_1",
            "캐시를 삭제해",
            "아니, 삭제가 아니라 분석이야",
            LanguageCodeIR::Korean,
            "INVESTIGATE",
            "DELETE",
            "캐시",
        ),
        correction(
            "R29_CORRECT_2",
            "보고서를 저장해",
            "아니, 저장 말고 검증해",
            LanguageCodeIR::Korean,
            "INVESTIGATE",
            "EXECUTE",
            "보고서",
        ),
        correction(
            "R29_CORRECT_3",
            "delete the archive",
            "No, inspect it instead of deleting it.",
            LanguageCodeIR::English,
            "INVESTIGATE",
            "DELETE",
            "archive",
        ),
        correction(
            "R29_CORRECT_4",
            "repair the parser",
            "Actually, analyze it instead.",
            LanguageCodeIR::English,
            "INVESTIGATE",
            "REPAIR",
            "parser",
        ),
    ];
    let passed = rows.iter().filter(|row| row.pass).count();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "suite": "R29-RUN-0001",
            "frozen_before_product_changes": true,
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
    if passed != 32 {
        std::process::exit(1);
    }
}
