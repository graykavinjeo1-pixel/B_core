//! Frozen R20-RUN-0002 held-out cross-axis transfer and authority suite.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, DiscourseBindingKindIR, LanguageCodeIR, PendingGateStatusIR,
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
        max_plan_steps: 16,
    }
}

fn selected_subject(response: &semantic_core_adapters::ConversationTurnResponseIR) -> String {
    response
        .pragmatic_interpretation
        .compositional_analysis
        .selected_candidate()
        .map(|candidate| candidate.subject.to_lowercase())
        .unwrap_or_default()
}

struct CrossTopicCase<'a> {
    id: &'a str,
    first: (&'a str, LanguageCodeIR),
    distractor: (&'a str, LanguageCodeIR),
    shift: (&'a str, LanguageCodeIR),
    social: (&'a str, LanguageCodeIR),
    action: (&'a str, LanguageCodeIR),
    target: &'a str,
    rejected: &'a str,
}

fn cross_topic_case(case: CrossTopicCase<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(case.id, 1, case.first.0, case.first.1))
        .expect("first topic");
    api.process_conversation_turn(&request(case.id, 2, case.distractor.0, case.distractor.1))
        .expect("distractor");
    let shift = api
        .process_conversation_turn(&request(case.id, 3, case.shift.0, case.shift.1))
        .expect("cross-language shift");
    let social = api
        .process_conversation_turn(&request(case.id, 4, case.social.0, case.social.1))
        .expect("social turn");
    let action = api
        .process_conversation_turn(&request(case.id, 5, case.action.0, case.action.1))
        .expect("resumed action");
    let resolved = action
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    Row {
        id: case.id.to_string(),
        category: "cross_language_topic_survives_social_turn".to_string(),
        trace: vec![shift.output.text, social.output.text, resolved.clone()],
        pass: shift.grounded_response.is_none()
            && shift.output.grounded_plan_sha256.is_none()
            && matches!(
                social.disposition,
                ConversationTurnDispositionIR::HoldFloor
                    | ConversationTurnDispositionIR::BackchannelOnly
            )
            && action.grounded_response.is_some()
            && resolved.contains(case.target)
            && !resolved.contains(case.rejected)
            && selected_subject(&action).contains(case.target),
    }
}

struct NearestLocalCase<'a> {
    id: &'a str,
    prior: &'a str,
    local: &'a str,
    target: &'a str,
    rejected: &'a str,
    language: LanguageCodeIR,
}

fn nearest_local_case(case: NearestLocalCase<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(case.id, 1, case.prior, case.language))
        .expect("prior");
    let response = api
        .process_conversation_turn(&request(case.id, 2, case.local, case.language))
        .expect("local contrast");
    let resolved = response
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    Row {
        id: case.id.to_string(),
        category: "nearest_compatible_local_antecedent".to_string(),
        trace: vec![resolved.clone(), response.output.text.clone()],
        pass: response.grounded_response.is_some()
            && resolved.contains(case.target)
            && !resolved.contains(case.rejected)
            && selected_subject(&response).contains(case.target),
    }
}

struct GateAttackCase<'a> {
    id: &'a str,
    gate: &'a str,
    non_authoritative_evidence: &'a str,
    question: &'a str,
    benefit: &'a str,
    language: LanguageCodeIR,
}

fn gate_attack_case(case: GateAttackCase<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(case.id, 1, case.gate, case.language))
        .expect("gate");
    let evidence = api
        .process_conversation_turn(&request(
            case.id,
            2,
            case.non_authoritative_evidence,
            case.language,
        ))
        .expect("non-authoritative evidence");
    let decision = api
        .process_conversation_turn(&request(case.id, 3, case.question, case.language))
        .expect("decision");
    Row {
        id: case.id.to_string(),
        category: "reported_uncertain_or_proxy_evidence_cannot_unlock_gate".to_string(),
        trace: vec![evidence.output.text, decision.output.text.clone()],
        pass: decision.grounded_response.is_none()
            && decision.output.grounded_plan_sha256.is_none()
            && decision.output.text.to_lowercase().contains(case.benefit)
            && decision
                .pragmatic_state
                .pending_continuation_gate
                .as_ref()
                .is_some_and(|gate| gate.status == PendingGateStatusIR::AwaitingEvidence)
            && decision.output.unsupported_freeform_claims == 0,
    }
}

struct DelayedResultCase<'a> {
    id: &'a str,
    first: &'a str,
    correction: &'a str,
    social: &'a str,
    result_question: &'a str,
    language: LanguageCodeIR,
}

fn delayed_result_case(case: DelayedResultCase<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(case.id, 1, case.first, case.language))
        .expect("plan");
    let correction = api
        .process_conversation_turn(&request(case.id, 2, case.correction, case.language))
        .expect("correction");
    api.process_conversation_turn(&request(case.id, 3, case.social, case.language))
        .expect("social");
    let result = api
        .process_conversation_turn(&request(case.id, 4, case.result_question, case.language))
        .expect("result question");
    let result_output = result.output.text.to_lowercase();
    Row {
        id: case.id.to_string(),
        category: "corrected_plan_is_not_delayed_execution_result".to_string(),
        trace: vec![correction.output.text, result.output.text.clone()],
        pass: correction
            .reference_resolution
            .discourse_bindings
            .iter()
            .any(|binding| binding.kind == DiscourseBindingKindIR::CorrectedArgument)
            && result.grounded_response.is_none()
            && result.output.grounded_plan_sha256.is_none()
            && (result_output.contains("실행 결과")
                || result_output.contains("no execution result"))
            && result.output.unsupported_freeform_claims == 0,
    }
}

fn main() {
    let rows = vec![
        cross_topic_case(CrossTopicCase {
            id: "R20_TRANSFER_TOPIC_1",
            first: ("캐시를 확인해", LanguageCodeIR::Korean),
            distractor: ("워커를 조사해", LanguageCodeIR::Korean),
            shift: ("let's return to the cache", LanguageCodeIR::English),
            social: ("음...", LanguageCodeIR::Korean),
            action: ("그거 수리해", LanguageCodeIR::Korean),
            target: "캐시",
            rejected: "워커",
        }),
        cross_topic_case(CrossTopicCase {
            id: "R20_TRANSFER_TOPIC_2",
            first: ("inspect the log", LanguageCodeIR::English),
            distractor: ("inspect the server", LanguageCodeIR::English),
            shift: ("로그 얘기로 돌아가자", LanguageCodeIR::Korean),
            social: ("thanks", LanguageCodeIR::English),
            action: ("repair it", LanguageCodeIR::English),
            target: "log",
            rejected: "server",
        }),
        cross_topic_case(CrossTopicCase {
            id: "R20_TRANSFER_TOPIC_3",
            first: ("큐를 분석해", LanguageCodeIR::Korean),
            distractor: ("백업을 확인해", LanguageCodeIR::Korean),
            shift: ("go back to the queue topic", LanguageCodeIR::English),
            social: ("잠깐", LanguageCodeIR::Korean),
            action: ("그것을 수리해", LanguageCodeIR::Korean),
            target: "큐",
            rejected: "백업",
        }),
        cross_topic_case(CrossTopicCase {
            id: "R20_TRANSFER_TOPIC_4",
            first: ("inspect the file", LanguageCodeIR::English),
            distractor: ("analyze the folder", LanguageCodeIR::English),
            shift: ("파일 이야기로 돌아가자", LanguageCodeIR::Korean),
            social: ("uh...", LanguageCodeIR::English),
            action: ("repair it", LanguageCodeIR::English),
            target: "file",
            rejected: "folder",
        }),
        nearest_local_case(NearestLocalCase {
            id: "R20_TRANSFER_LOCAL_1",
            prior: "서버를 확인해",
            local: "서버는 정상이고 캐시는 오래됐다. 그것을 분석해",
            target: "캐시",
            rejected: "서버를 분석",
            language: LanguageCodeIR::Korean,
        }),
        nearest_local_case(NearestLocalCase {
            id: "R20_TRANSFER_LOCAL_2",
            prior: "inspect the worker",
            local: "although the worker is healthy, the queue is stale. analyze it",
            target: "queue",
            rejected: "analyze the worker",
            language: LanguageCodeIR::English,
        }),
        nearest_local_case(NearestLocalCase {
            id: "R20_TRANSFER_LOCAL_3",
            prior: "백업을 확인해",
            local: "백업은 온전하지만 로그는 비었다. 그것을 분석해",
            target: "로그",
            rejected: "백업을 분석",
            language: LanguageCodeIR::Korean,
        }),
        nearest_local_case(NearestLocalCase {
            id: "R20_TRANSFER_LOCAL_4",
            prior: "inspect the folder",
            local: "the folder is intact but the file is stale. analyze it",
            target: "file",
            rejected: "analyze the folder",
            language: LanguageCodeIR::English,
        }),
        gate_attack_case(GateAttackCase {
            id: "R20_TRANSFER_GATE_1",
            gate: "통합으로 실제 커버리지가 늘어야 계속할 수 있다.",
            non_authoritative_evidence: "민수가 커버리지가 늘었다고 말했다",
            question: "그러면 계속해도 돼?",
            benefit: "커버리지",
            language: LanguageCodeIR::Korean,
        }),
        gate_attack_case(GateAttackCase {
            id: "R20_TRANSFER_GATE_2",
            gate: "We should continue migration only if real failures decrease.",
            non_authoritative_evidence: "Avery said the failures decreased",
            question: "Should we continue now?",
            benefit: "failure",
            language: LanguageCodeIR::English,
        }),
        gate_attack_case(GateAttackCase {
            id: "R20_TRANSFER_GATE_3",
            gate: "리팩터링은 실제 장애가 줄어야 계속할 만하다.",
            non_authoritative_evidence: "아마 장애가 줄었을 거야",
            question: "이제 계속 진행해도 돼?",
            benefit: "장애",
            language: LanguageCodeIR::Korean,
        }),
        gate_attack_case(GateAttackCase {
            id: "R20_TRANSFER_GATE_4",
            gate: "Integration is worth continuing only if actual coverage expands.",
            non_authoritative_evidence: "The benchmark score expanded",
            question: "Can we keep going?",
            benefit: "coverage",
            language: LanguageCodeIR::English,
        }),
        delayed_result_case(DelayedResultCase {
            id: "R20_TRANSFER_RESULT_1",
            first: "파일을 열어",
            correction: "그거 말고 폴더로",
            social: "고마워",
            result_question: "그 결과는?",
            language: LanguageCodeIR::Korean,
        }),
        delayed_result_case(DelayedResultCase {
            id: "R20_TRANSFER_RESULT_2",
            first: "inspect the cache",
            correction: "not that, the queue instead",
            social: "thanks",
            result_question: "what did that result show?",
            language: LanguageCodeIR::English,
        }),
        delayed_result_case(DelayedResultCase {
            id: "R20_TRANSFER_RESULT_3",
            first: "로그를 분석해",
            correction: "그거 말고 백업으로",
            social: "음...",
            result_question: "그 출력은?",
            language: LanguageCodeIR::Korean,
        }),
        delayed_result_case(DelayedResultCase {
            id: "R20_TRANSFER_RESULT_4",
            first: "repair the server",
            correction: "not that, the worker instead",
            social: "okay",
            result_question: "tell me that result",
            language: LanguageCodeIR::English,
        }),
    ];
    let passed = rows.iter().filter(|row| row.pass).count();
    let payload = serde_json::json!({
        "suite": "R20-RUN-0002",
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
