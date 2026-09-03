//! Frozen R21-RUN-0001 long-horizon, topic-stack, and local-order diagnostic.
//!
//! The public conversation API is the only system under test.  No expected
//! answer or sentence-specific repair data enters the reasoner.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, DiscourseAnswerDispositionIR, LanguageCodeIR,
    CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    category: String,
    trace: Vec<String>,
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
        max_plan_steps: 20,
    }
}

fn has_local_order_binding(response: &semantic_core_adapters::ConversationTurnResponseIR) -> bool {
    response
        .reference_resolution
        .discourse_bindings
        .iter()
        .any(|binding| {
            binding
                .evidence
                .iter()
                .any(|item| item == "SYNTACTIC_PRIORITY:LOCAL_ORDERED_ANTECEDENTS")
        })
}

fn goal_subjects(response: &semantic_core_adapters::ConversationTurnResponseIR) -> Vec<String> {
    response
        .conversation_state
        .active_goals
        .iter()
        .map(|goal| goal.subject.to_lowercase())
        .collect()
}

struct LocalOrderedCase<'a> {
    id: &'a str,
    text: &'a str,
    first_action: &'a str,
    second_action: &'a str,
    first_subject: &'a str,
    second_subject: &'a str,
    language: LanguageCodeIR,
}

fn local_ordered_case(case: LocalOrderedCase<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let response = api
        .process_conversation_turn(&request(case.id, 1, case.text, case.language))
        .expect("local ordered turn");
    let resolved = response
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    let goals = goal_subjects(&response);
    Row {
        id: case.id.to_string(),
        category: "same_turn_local_ordered_reference".to_string(),
        trace: vec![
            resolved.clone(),
            format!("goals={goals:?}"),
            response.output.text.clone(),
        ],
        pass: response.disposition == ConversationTurnDispositionIR::Grounded
            && response.grounded_response.is_some()
            && has_local_order_binding(&response)
            && resolved.contains(case.first_action)
            && resolved.contains(case.second_action)
            && goals.len() == 2
            && goals
                .iter()
                .any(|subject| subject.contains(case.first_subject))
            && goals
                .iter()
                .any(|subject| subject.contains(case.second_subject))
            && response.output.unsupported_freeform_claims == 0,
    }
}

struct PreviousTopicCase<'a> {
    id: &'a str,
    first: &'a str,
    second: &'a str,
    first_shift: &'a str,
    second_shift: &'a str,
    previous_shift: &'a str,
    action: &'a str,
    target: &'a str,
    rejected: &'a str,
    language: LanguageCodeIR,
}

fn previous_topic_case(case: PreviousTopicCase<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    for (turn, text) in [case.first, case.second, case.first_shift, case.second_shift]
        .into_iter()
        .enumerate()
    {
        api.process_conversation_turn(&request(
            case.id,
            u64::try_from(turn + 1).expect("bounded turn"),
            text,
            case.language,
        ))
        .expect("topic setup");
    }
    let shift = api
        .process_conversation_turn(&request(case.id, 5, case.previous_shift, case.language))
        .expect("previous topic shift");
    let action = api
        .process_conversation_turn(&request(case.id, 6, case.action, case.language))
        .expect("previous topic action");
    let resolved = action
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    Row {
        id: case.id.to_string(),
        category: "previous_topic_stack_navigation".to_string(),
        trace: vec![
            shift.output.text,
            resolved.clone(),
            action.output.text.clone(),
        ],
        pass: shift.grounded_response.is_none()
            && shift.output.grounded_plan_sha256.is_none()
            && shift
                .conversation_state
                .active_topics
                .first()
                .is_some_and(|topic| {
                    topic.explicitly_activated && topic.surface.to_lowercase().contains(case.target)
                })
            && action.grounded_response.is_some()
            && resolved.contains(case.target)
            && !resolved.contains(case.rejected)
            && action.output.unsupported_freeform_claims == 0,
    }
}

struct LongFocusCase<'a> {
    id: &'a str,
    target_request: &'a str,
    distractor_request: &'a str,
    shift: &'a str,
    action: &'a str,
    target: &'a str,
    rejected: &'a str,
    language: LanguageCodeIR,
}

fn long_focus_case(case: LongFocusCase<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(case.id, 1, case.target_request, case.language))
        .expect("target");
    api.process_conversation_turn(&request(case.id, 2, case.distractor_request, case.language))
        .expect("distractor");
    let shift = api
        .process_conversation_turn(&request(case.id, 3, case.shift, case.language))
        .expect("focus");
    let social = if case.language == LanguageCodeIR::Korean {
        ["음...", "고마워", "잠깐", "알겠어", "그래", "어..."]
    } else {
        ["uh...", "thanks", "one moment", "okay", "right", "hmm..."]
    };
    for (offset, text) in social.into_iter().enumerate() {
        api.process_conversation_turn(&request(
            case.id,
            u64::try_from(offset + 4).expect("bounded turn"),
            text,
            case.language,
        ))
        .expect("social delay");
    }
    let action = api
        .process_conversation_turn(&request(case.id, 10, case.action, case.language))
        .expect("long-horizon action");
    let resolved = action
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    Row {
        id: case.id.to_string(),
        category: "explicit_focus_survives_long_social_delay".to_string(),
        trace: vec![
            shift.output.text,
            resolved.clone(),
            action.output.text.clone(),
        ],
        pass: action.grounded_response.is_some()
            && resolved.contains(case.target)
            && !resolved.contains(case.rejected)
            && action.output.unsupported_freeform_claims == 0,
    }
}

struct OpenTopicCase<'a> {
    id: &'a str,
    target_request: &'a str,
    distractor_request: &'a str,
    shift: &'a str,
    action: &'a str,
    target: &'a str,
    rejected: &'a str,
    language: LanguageCodeIR,
}

fn open_topic_case(case: OpenTopicCase<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(case.id, 1, case.target_request, case.language))
        .expect("open target");
    api.process_conversation_turn(&request(case.id, 2, case.distractor_request, case.language))
        .expect("distractor");
    let shift = api
        .process_conversation_turn(&request(case.id, 3, case.shift, case.language))
        .expect("open topic shift");
    let action = api
        .process_conversation_turn(&request(case.id, 4, case.action, case.language))
        .expect("open topic action");
    let resolved = action
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    Row {
        id: case.id.to_string(),
        category: "open_vocabulary_topic_binding".to_string(),
        trace: vec![
            shift.output.text,
            resolved.clone(),
            action.output.text.clone(),
        ],
        pass: shift
            .conversation_state
            .active_topics
            .first()
            .is_some_and(|topic| topic.concept_id_hint.is_none() && topic.explicitly_activated)
            && action.grounded_response.is_some()
            && resolved.contains(case.target)
            && !resolved.contains(case.rejected),
    }
}

struct ScopedOrderedCase<'a> {
    id: &'a str,
    text: &'a str,
    authorized_subject: &'a str,
    prohibited_subject: &'a str,
    language: LanguageCodeIR,
}

fn scoped_ordered_case(case: ScopedOrderedCase<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let response = api
        .process_conversation_turn(&request(case.id, 1, case.text, case.language))
        .expect("scoped ordered turn");
    let goals = goal_subjects(&response);
    let resolved = response
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    Row {
        id: case.id.to_string(),
        category: "local_order_with_scoped_prohibition".to_string(),
        trace: vec![
            resolved.clone(),
            format!("goals={goals:?}"),
            response.output.text.clone(),
        ],
        pass: has_local_order_binding(&response)
            && response
                .pragmatic_interpretation
                .compositional_analysis
                .blocked_execution_count()
                >= 1
            && goals.len() == 1
            && goals[0].contains(case.authorized_subject)
            && !goals[0].contains(case.prohibited_subject)
            && response.output.unsupported_freeform_claims == 0,
    }
}

struct DelayedEvidenceCase<'a> {
    id: &'a str,
    first_claim: &'a str,
    second_claim: &'a str,
    question: &'a str,
    language: LanguageCodeIR,
}

fn delayed_evidence_case(case: DelayedEvidenceCase<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(case.id, 1, case.first_claim, case.language))
        .expect("first claim");
    api.process_conversation_turn(&request(case.id, 2, case.second_claim, case.language))
        .expect("second claim");
    let social = if case.language == LanguageCodeIR::Korean {
        ["음...", "고마워", "잠깐", "알겠어", "그래"]
    } else {
        ["uh...", "thanks", "one moment", "okay", "right"]
    };
    for (offset, text) in social.into_iter().enumerate() {
        api.process_conversation_turn(&request(
            case.id,
            u64::try_from(offset + 3).expect("bounded turn"),
            text,
            case.language,
        ))
        .expect("evidence delay");
    }
    let answer = api
        .process_conversation_turn(&request(case.id, 8, case.question, case.language))
        .expect("delayed actuality");
    let typed = answer.discourse_answer.as_ref();
    Row {
        id: case.id.to_string(),
        category: "evidence_grounding_survives_long_delay".to_string(),
        trace: vec![answer.output.text.clone()],
        pass: typed.is_some_and(|typed| {
            matches!(
                typed.disposition,
                DiscourseAnswerDispositionIR::ConflictingDialogueRecords
                    | DiscourseAnswerDispositionIR::DialogueTruthNotEstablished
            ) && typed.evidence.len() >= 2
                && !typed.dialogue_truth_established
                && !typed.external_execution_authorized
                && typed.unsupported_claims == 0
        }) && answer.grounded_response.is_none()
            && answer.output.grounded_plan_sha256.is_none(),
    }
}

fn main() {
    let rows = vec![
        local_ordered_case(LocalOrderedCase { id: "R21_LOCAL_ORDER_1", text: "파일은 오래됐고 폴더는 비었다. 전자를 분석하고 후자를 수리해", first_action: "파일을 분석", second_action: "폴더를 수리", first_subject: "파일", second_subject: "폴더", language: LanguageCodeIR::Korean }),
        local_ordered_case(LocalOrderedCase { id: "R21_LOCAL_ORDER_2", text: "the file is stale and the folder is empty. analyze the former and repair the latter", first_action: "analyze the file", second_action: "repair the folder", first_subject: "file", second_subject: "folder", language: LanguageCodeIR::English }),
        local_ordered_case(LocalOrderedCase { id: "R21_LOCAL_ORDER_3", text: "캐시는 낡았고 큐는 막혔다. 전자를 수리하고 후자를 분석해", first_action: "캐시를 수리", second_action: "큐를 분석", first_subject: "캐시", second_subject: "큐", language: LanguageCodeIR::Korean }),
        local_ordered_case(LocalOrderedCase { id: "R21_LOCAL_ORDER_4", text: "the server is slow but the log is incomplete. repair the former and analyze the latter", first_action: "repair the server", second_action: "analyze the log", first_subject: "server", second_subject: "log", language: LanguageCodeIR::English }),
        previous_topic_case(PreviousTopicCase { id: "R21_PREVIOUS_1", first: "캐시를 확인해", second: "큐를 분석해", first_shift: "캐시 얘기로 돌아가자", second_shift: "큐 얘기로 돌아가자", previous_shift: "이전 주제로 돌아가자", action: "그거 수리해", target: "캐시", rejected: "큐", language: LanguageCodeIR::Korean }),
        previous_topic_case(PreviousTopicCase { id: "R21_PREVIOUS_2", first: "inspect the log", second: "inspect the server", first_shift: "return to the log", second_shift: "return to the server", previous_shift: "go back to the previous topic", action: "repair it", target: "log", rejected: "server", language: LanguageCodeIR::English }),
        previous_topic_case(PreviousTopicCase { id: "R21_PREVIOUS_3", first: "백업을 확인해", second: "워커를 분석해", first_shift: "백업 이야기로 돌아가자", second_shift: "워커 이야기로 돌아가자", previous_shift: "아까 주제로 돌아가자", action: "그것을 분석해", target: "백업", rejected: "워커", language: LanguageCodeIR::Korean }),
        previous_topic_case(PreviousTopicCase { id: "R21_PREVIOUS_4", first: "inspect the file", second: "inspect the folder", first_shift: "go back to the file topic", second_shift: "go back to the folder topic", previous_shift: "return to the prior topic", action: "repair it", target: "file", rejected: "folder", language: LanguageCodeIR::English }),
        long_focus_case(LongFocusCase { id: "R21_LONG_1", target_request: "캐시를 확인해", distractor_request: "워커를 분석해", shift: "캐시 얘기로 돌아가자", action: "그거 수리해", target: "캐시", rejected: "워커", language: LanguageCodeIR::Korean }),
        long_focus_case(LongFocusCase { id: "R21_LONG_2", target_request: "inspect the log", distractor_request: "inspect the server", shift: "return to the log", action: "repair it", target: "log", rejected: "server", language: LanguageCodeIR::English }),
        long_focus_case(LongFocusCase { id: "R21_LONG_3", target_request: "큐를 확인해", distractor_request: "백업을 분석해", shift: "큐 이야기로 돌아가자", action: "그것을 수리해", target: "큐", rejected: "백업", language: LanguageCodeIR::Korean }),
        long_focus_case(LongFocusCase { id: "R21_LONG_4", target_request: "inspect the file", distractor_request: "inspect the folder", shift: "go back to the file topic", action: "repair it", target: "file", rejected: "folder", language: LanguageCodeIR::English }),
        open_topic_case(OpenTopicCase { id: "R21_OPEN_1", target_request: "데이터베이스를 확인해", distractor_request: "캐시를 분석해", shift: "데이터베이스 얘기로 돌아가자", action: "그거 수리해", target: "데이터베이스", rejected: "캐시", language: LanguageCodeIR::Korean }),
        open_topic_case(OpenTopicCase { id: "R21_OPEN_2", target_request: "inspect the scheduler", distractor_request: "inspect the log", shift: "return to the scheduler", action: "repair it", target: "scheduler", rejected: "log", language: LanguageCodeIR::English }),
        open_topic_case(OpenTopicCase { id: "R21_OPEN_3", target_request: "파이프라인을 확인해", distractor_request: "큐를 분석해", shift: "파이프라인 이야기로 돌아가자", action: "그것을 수리해", target: "파이프라인", rejected: "큐", language: LanguageCodeIR::Korean }),
        open_topic_case(OpenTopicCase { id: "R21_OPEN_4", target_request: "inspect the coordinator", distractor_request: "inspect the server", shift: "go back to the coordinator topic", action: "repair it", target: "coordinator", rejected: "server", language: LanguageCodeIR::English }),
        scoped_ordered_case(ScopedOrderedCase { id: "R21_SCOPE_ORDER_1", text: "캐시는 오래됐고 큐는 막혔다. 전자는 분석하되 후자는 삭제하지 마", authorized_subject: "캐시", prohibited_subject: "큐", language: LanguageCodeIR::Korean }),
        scoped_ordered_case(ScopedOrderedCase { id: "R21_SCOPE_ORDER_2", text: "the cache is stale and the queue is blocked. analyze the former but do not delete the latter", authorized_subject: "cache", prohibited_subject: "queue", language: LanguageCodeIR::English }),
        scoped_ordered_case(ScopedOrderedCase { id: "R21_SCOPE_ORDER_3", text: "로그는 비었고 백업은 온전하다. 전자는 확인하되 후자는 지우지 마", authorized_subject: "로그", prohibited_subject: "백업", language: LanguageCodeIR::Korean }),
        scoped_ordered_case(ScopedOrderedCase { id: "R21_SCOPE_ORDER_4", text: "the server is slow and the worker is healthy. inspect the former but never delete the latter", authorized_subject: "server", prohibited_subject: "worker", language: LanguageCodeIR::English }),
        delayed_evidence_case(DelayedEvidenceCase { id: "R21_EVIDENCE_1", first_claim: "민아는 캐시가 손상됐다고 말했다", second_claim: "준은 캐시가 정상이라고 말했다", question: "캐시가 실제로 손상됐어?", language: LanguageCodeIR::Korean }),
        delayed_evidence_case(DelayedEvidenceCase { id: "R21_EVIDENCE_2", first_claim: "Mina says the worker is blocked", second_claim: "Jules says the worker is healthy", question: "is the worker actually blocked?", language: LanguageCodeIR::English }),
        delayed_evidence_case(DelayedEvidenceCase { id: "R21_EVIDENCE_3", first_claim: "서윤은 서버가 느리다고 말했다", second_claim: "하준은 서버가 빠르다고 말했다", question: "서버가 실제로 느려?", language: LanguageCodeIR::Korean }),
        delayed_evidence_case(DelayedEvidenceCase { id: "R21_EVIDENCE_4", first_claim: "Avery says the queue is stale", second_claim: "Rowan says the queue is fresh", question: "is the queue actually stale?", language: LanguageCodeIR::English }),
    ];
    let passed = rows.iter().filter(|row| row.pass).count();
    let payload = serde_json::json!({
        "suite": "R21-RUN-0001",
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
