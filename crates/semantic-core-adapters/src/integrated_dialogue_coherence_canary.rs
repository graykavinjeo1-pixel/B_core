//! Frozen R20-RUN-0001 cross-axis diagnostic suite.
//!
//! Each case requires multiple language mechanisms to cooperate.  The suite
//! intentionally observes the public conversation API rather than calling a
//! private parser or response template.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, DiscourseAnswerDispositionIR, DiscourseBindingKindIR,
    LanguageCodeIR, PendingGateStatusIR, CONVERSATION_TURN_REQUEST_SCHEMA,
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
        .map(|candidate| candidate.subject.clone())
        .unwrap_or_default()
}

fn selected_has_predicate(
    response: &semantic_core_adapters::ConversationTurnResponseIR,
    predicate: &str,
) -> bool {
    response
        .pragmatic_interpretation
        .compositional_analysis
        .selected_candidate()
        .is_some_and(|candidate| {
            candidate
                .evidence
                .iter()
                .any(|evidence| evidence == &format!("predicate={predicate}"))
        })
}

fn plan_is_explicitly_not_a_result(
    response: &semantic_core_adapters::ConversationTurnResponseIR,
) -> bool {
    let text = response.output.text.to_lowercase();
    let korean_boundary = text.contains("아직 실행 결과")
        || (text.contains("계획 상태") && text.contains("실행한 것은 아니"));
    let english_boundary = text.contains("planned operations, not completed results")
        || (text.contains("still planned") && text.contains("not executed"));
    response.output.grounded_plan_sha256.is_some() && (korean_boundary || english_boundary)
}

fn no_recorded_result(response: &semantic_core_adapters::ConversationTurnResponseIR) -> bool {
    let output = response.output.text.to_lowercase();
    response.grounded_response.is_none()
        && response.output.grounded_plan_sha256.is_none()
        && (output.contains("실행 결과") || output.contains("no execution result"))
        && response.output.unsupported_freeform_claims == 0
}

struct TopicReturnCase<'a> {
    id: &'a str,
    first: &'a str,
    distractor: &'a str,
    shift: &'a str,
    action: &'a str,
    result_question: &'a str,
    target: &'a str,
    rejected: &'a str,
    language: LanguageCodeIR,
}

fn topic_return_case(case: TopicReturnCase<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(case.id, 1, case.first, case.language))
        .expect("first topic");
    api.process_conversation_turn(&request(case.id, 2, case.distractor, case.language))
        .expect("distractor topic");
    let shift = api
        .process_conversation_turn(&request(case.id, 3, case.shift, case.language))
        .expect("topic shift");
    let action = api
        .process_conversation_turn(&request(case.id, 4, case.action, case.language))
        .expect("topic-bound action");
    let result = api
        .process_conversation_turn(&request(case.id, 5, case.result_question, case.language))
        .expect("result question");
    let resolved = action.reference_resolution.resolved_semantic_text.clone();
    let subject = selected_subject(&action);
    Row {
        id: case.id.to_string(),
        category: "topic_return_pronoun_plan_result".to_string(),
        trace: vec![
            shift.output.text,
            resolved.clone(),
            result.output.text.clone(),
        ],
        pass: shift.disposition == ConversationTurnDispositionIR::Grounded
            && shift.grounded_response.is_none()
            && shift.output.grounded_plan_sha256.is_none()
            && action.disposition == ConversationTurnDispositionIR::Grounded
            && action.grounded_response.is_some()
            && resolved.to_lowercase().contains(case.target)
            && !resolved.to_lowercase().contains(case.rejected)
            && subject.to_lowercase().contains(case.target)
            && plan_is_explicitly_not_a_result(&action)
            && no_recorded_result(&result),
    }
}

struct LocalAntecedentCase<'a> {
    id: &'a str,
    prior: &'a str,
    composite: &'a str,
    target: &'a str,
    rejected: &'a str,
    language: LanguageCodeIR,
}

fn local_antecedent_case(case: LocalAntecedentCase<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(case.id, 1, case.prior, case.language))
        .expect("prior topic");
    let response = api
        .process_conversation_turn(&request(case.id, 2, case.composite, case.language))
        .expect("local antecedent");
    let resolved = response.reference_resolution.resolved_semantic_text.clone();
    let subject = selected_subject(&response);
    Row {
        id: case.id.to_string(),
        category: "local_antecedent_over_global_recency".to_string(),
        trace: vec![resolved.clone(), response.output.text.clone()],
        pass: response.disposition == ConversationTurnDispositionIR::Grounded
            && response.grounded_response.is_some()
            && resolved.to_lowercase().contains(case.target)
            && !resolved.to_lowercase().contains(case.rejected)
            && subject.to_lowercase().contains(case.target)
            && plan_is_explicitly_not_a_result(&response),
    }
}

struct EpistemicChoiceCase<'a> {
    id: &'a str,
    claim_a: &'a str,
    claim_b: &'a str,
    actuality_question: &'a str,
    language: LanguageCodeIR,
}

fn epistemic_choice_case(case: EpistemicChoiceCase<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(case.id, 1, case.claim_a, case.language))
        .expect("claim a");
    api.process_conversation_turn(&request(case.id, 2, case.claim_b, case.language))
        .expect("claim b");
    let actuality = api
        .process_conversation_turn(&request(case.id, 3, case.actuality_question, case.language))
        .expect("actuality question");
    let answer = actuality.discourse_answer.as_ref();
    Row {
        id: case.id.to_string(),
        category: "competing_sources_evidence_grounded_actuality".to_string(),
        trace: vec![actuality.output.text.clone()],
        pass: answer.is_some_and(|answer| {
            matches!(
                answer.disposition,
                DiscourseAnswerDispositionIR::ConflictingDialogueRecords
                    | DiscourseAnswerDispositionIR::DialogueTruthNotEstablished
            ) && answer.evidence.len() >= 2
                && !answer.dialogue_truth_established
                && !answer.external_execution_authorized
                && answer.unsupported_claims == 0
        }) && actuality.grounded_response.is_none()
            && actuality.output.grounded_plan_sha256.is_none(),
    }
}

struct NegatedEllipsisCase<'a> {
    id: &'a str,
    first: &'a str,
    elliptical: &'a str,
    target: &'a str,
    forbidden: &'a str,
    predicate: &'a str,
    language: LanguageCodeIR,
}

fn negated_ellipsis_case(case: NegatedEllipsisCase<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let first = api
        .process_conversation_turn(&request(case.id, 1, case.first, case.language))
        .expect("scoped first request");
    let second = api
        .process_conversation_turn(&request(case.id, 2, case.elliptical, case.language))
        .expect("parallel ellipsis");
    let resolved = second.reference_resolution.resolved_semantic_text.clone();
    Row {
        id: case.id.to_string(),
        category: "negation_scope_then_safe_parallel_ellipsis".to_string(),
        trace: vec![
            first.output.text,
            resolved.clone(),
            second.output.text.clone(),
        ],
        pass: first
            .pragmatic_interpretation
            .compositional_analysis
            .blocked_execution_count()
            >= 1
            && first.conversation_state.active_goals.len() == 1
            && second.disposition == ConversationTurnDispositionIR::Grounded
            && second.grounded_response.is_some()
            && resolved.to_lowercase().contains(case.target)
            && !resolved.to_lowercase().contains(case.forbidden)
            && selected_has_predicate(&second, case.predicate)
            && second
                .reference_resolution
                .discourse_bindings
                .iter()
                .any(|binding| binding.kind == DiscourseBindingKindIR::EllipticalAction)
            && plan_is_explicitly_not_a_result(&second),
    }
}

struct CorrectionChainCase<'a> {
    id: &'a str,
    first: &'a str,
    correction: &'a str,
    parallel: &'a str,
    result_question: &'a str,
    corrected: &'a str,
    final_target: &'a str,
    language: LanguageCodeIR,
}

fn correction_chain_case(case: CorrectionChainCase<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(case.id, 1, case.first, case.language))
        .expect("first goal");
    let corrected = api
        .process_conversation_turn(&request(case.id, 2, case.correction, case.language))
        .expect("argument correction");
    let parallel = api
        .process_conversation_turn(&request(case.id, 3, case.parallel, case.language))
        .expect("ellipsis after correction");
    let result = api
        .process_conversation_turn(&request(case.id, 4, case.result_question, case.language))
        .expect("result after plan");
    Row {
        id: case.id.to_string(),
        category: "correction_supersedes_goal_then_ellipsis_and_result".to_string(),
        trace: vec![
            corrected
                .reference_resolution
                .resolved_semantic_text
                .clone(),
            parallel.reference_resolution.resolved_semantic_text.clone(),
            result.output.text.clone(),
        ],
        pass: corrected
            .reference_resolution
            .discourse_bindings
            .iter()
            .any(|binding| binding.kind == DiscourseBindingKindIR::CorrectedArgument)
            && selected_subject(&corrected)
                .to_lowercase()
                .contains(case.corrected)
            && parallel.disposition == ConversationTurnDispositionIR::Grounded
            && parallel.grounded_response.is_some()
            && parallel
                .reference_resolution
                .ambiguous_reference_surfaces
                .is_empty()
            && parallel
                .reference_resolution
                .discourse_bindings
                .iter()
                .any(|binding| binding.kind == DiscourseBindingKindIR::EllipticalAction)
            && selected_subject(&parallel)
                .to_lowercase()
                .contains(case.final_target)
            && no_recorded_result(&result),
    }
}

struct GateEvidenceCase<'a> {
    id: &'a str,
    gate: &'a str,
    proxy: &'a str,
    decision_question: &'a str,
    benefit_fragment: &'a str,
    language: LanguageCodeIR,
}

fn gate_evidence_case(case: GateEvidenceCase<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let gate = api
        .process_conversation_turn(&request(case.id, 1, case.gate, case.language))
        .expect("continuation gate");
    let proxy = api
        .process_conversation_turn(&request(case.id, 2, case.proxy, case.language))
        .expect("proxy observation");
    let decision = api
        .process_conversation_turn(&request(case.id, 3, case.decision_question, case.language))
        .expect("decision question");
    let gate_state = decision.pragmatic_state.pending_continuation_gate.as_ref();
    Row {
        id: case.id.to_string(),
        category: "proxy_evidence_cannot_authorize_continuation".to_string(),
        trace: vec![
            gate.output.text,
            proxy.output.text,
            decision.output.text.clone(),
        ],
        pass: gate.pragmatic_state.pending_continuation_gate.is_some()
            && proxy.grounded_response.is_none()
            && proxy.output.grounded_plan_sha256.is_none()
            && decision.grounded_response.is_none()
            && decision.output.grounded_plan_sha256.is_none()
            && decision
                .output
                .text
                .to_lowercase()
                .contains(case.benefit_fragment)
            && gate_state
                .is_some_and(|state| state.status == PendingGateStatusIR::AwaitingEvidence)
            && decision.output.unsupported_freeform_claims == 0,
    }
}

fn main() {
    let rows = vec![
        topic_return_case(TopicReturnCase { id: "R20_TOPIC_1", first: "캐시를 확인해", distractor: "워커를 조사해", shift: "캐시 얘기로 돌아가자", action: "그것을 수리해", result_question: "그 결과는?", target: "캐시", rejected: "워커", language: LanguageCodeIR::Korean }),
        topic_return_case(TopicReturnCase { id: "R20_TOPIC_2", first: "inspect the log", distractor: "inspect the server", shift: "let's return to the log", action: "repair it", result_question: "what did that result show?", target: "log", rejected: "server", language: LanguageCodeIR::English }),
        topic_return_case(TopicReturnCase { id: "R20_TOPIC_3", first: "큐를 확인해", distractor: "백업을 분석해", shift: "큐 이야기로 돌아가자", action: "그거 고쳐", result_question: "그 출력은?", target: "큐", rejected: "백업", language: LanguageCodeIR::Korean }),
        topic_return_case(TopicReturnCase { id: "R20_TOPIC_4", first: "inspect the file", distractor: "analyze the folder", shift: "go back to the file topic", action: "repair it", result_question: "tell me that result", target: "file", rejected: "folder", language: LanguageCodeIR::English }),
        local_antecedent_case(LocalAntecedentCase { id: "R20_LOCAL_1", prior: "워커를 확인해", composite: "캐시는 오래됐다. 그것을 분석해", target: "캐시", rejected: "워커", language: LanguageCodeIR::Korean }),
        local_antecedent_case(LocalAntecedentCase { id: "R20_LOCAL_2", prior: "inspect the server", composite: "the queue is stale. analyze it", target: "queue", rejected: "server", language: LanguageCodeIR::English }),
        local_antecedent_case(LocalAntecedentCase { id: "R20_LOCAL_3", prior: "백업을 확인해", composite: "로그는 비어 있다. 그것을 분석해", target: "로그", rejected: "백업", language: LanguageCodeIR::Korean }),
        local_antecedent_case(LocalAntecedentCase { id: "R20_LOCAL_4", prior: "inspect the folder", composite: "the file is stale. analyze it", target: "file", rejected: "folder", language: LanguageCodeIR::English }),
        epistemic_choice_case(EpistemicChoiceCase { id: "R20_EPISTEMIC_1", claim_a: "민아는 캐시가 손상됐다고 말했다", claim_b: "준은 캐시가 정상이라고 말했다", actuality_question: "캐시가 실제로 손상됐어?", language: LanguageCodeIR::Korean }),
        epistemic_choice_case(EpistemicChoiceCase { id: "R20_EPISTEMIC_2", claim_a: "Mina says the worker is blocked", claim_b: "Jules says the worker is healthy", actuality_question: "is the worker actually blocked?", language: LanguageCodeIR::English }),
        epistemic_choice_case(EpistemicChoiceCase { id: "R20_EPISTEMIC_3", claim_a: "서윤은 서버가 느리다고 말했다", claim_b: "하준은 서버가 빠르다고 말했다", actuality_question: "서버가 실제로 느려?", language: LanguageCodeIR::Korean }),
        epistemic_choice_case(EpistemicChoiceCase { id: "R20_EPISTEMIC_4", claim_a: "Avery says the queue is stale", claim_b: "Rowan says the queue is fresh", actuality_question: "is the queue actually stale?", language: LanguageCodeIR::English }),
        negated_ellipsis_case(NegatedEllipsisCase { id: "R20_SCOPE_1", first: "로그를 분석하되 캐시는 삭제하지 마", elliptical: "백업도", target: "백업", forbidden: "삭제", predicate: "INVESTIGATE", language: LanguageCodeIR::Korean }),
        negated_ellipsis_case(NegatedEllipsisCase { id: "R20_SCOPE_2", first: "analyze the queue but do not delete the cache", elliptical: "same for the backup", target: "backup", forbidden: "delete", predicate: "INVESTIGATE", language: LanguageCodeIR::English }),
        negated_ellipsis_case(NegatedEllipsisCase { id: "R20_SCOPE_3", first: "파일을 확인하되 폴더는 지우지 마", elliptical: "문서도", target: "문서", forbidden: "지우", predicate: "INVESTIGATE", language: LanguageCodeIR::Korean }),
        negated_ellipsis_case(NegatedEllipsisCase { id: "R20_SCOPE_4", first: "inspect the server but do not delete the worker", elliptical: "same for the queue", target: "queue", forbidden: "delete", predicate: "INVESTIGATE", language: LanguageCodeIR::English }),
        correction_chain_case(CorrectionChainCase { id: "R20_CORRECT_1", first: "파일을 열어", correction: "그거 말고 폴더로", parallel: "문서도", result_question: "그 결과는?", corrected: "폴더", final_target: "문서", language: LanguageCodeIR::Korean }),
        correction_chain_case(CorrectionChainCase { id: "R20_CORRECT_2", first: "inspect the cache", correction: "not that, the queue instead", parallel: "same for the backup", result_question: "what did that result show?", corrected: "queue", final_target: "backup", language: LanguageCodeIR::English }),
        correction_chain_case(CorrectionChainCase { id: "R20_CORRECT_3", first: "서버를 수리해", correction: "그거 말고 워커로", parallel: "캐시도", result_question: "그 출력은?", corrected: "워커", final_target: "캐시", language: LanguageCodeIR::Korean }),
        correction_chain_case(CorrectionChainCase { id: "R20_CORRECT_4", first: "analyze the log", correction: "not that, the report instead", parallel: "same for the file", result_question: "tell me that result", corrected: "report", final_target: "file", language: LanguageCodeIR::English }),
        gate_evidence_case(GateEvidenceCase { id: "R20_GATE_1", gate: "통합은 힘들다. 통합을 하면 실제 커버리지가 늘어난다. 그 이득이면 계속할 만하다.", proxy: "점수만 올랐어", decision_question: "그러면 계속해도 돼?", benefit_fragment: "커버리지", language: LanguageCodeIR::Korean }),
        gate_evidence_case(GateEvidenceCase { id: "R20_GATE_2", gate: "The migration is costly. If migration reduces real failures, it is worth continuing.", proxy: "Only the benchmark score increased", decision_question: "So should we continue?", benefit_fragment: "failure", language: LanguageCodeIR::English }),
        gate_evidence_case(GateEvidenceCase { id: "R20_GATE_3", gate: "리팩터링은 어렵다. 리팩터링으로 실제 장애가 줄면 계속할 만하다.", proxy: "라우팅 점수만 좋아졌어", decision_question: "이제 계속 진행해도 돼?", benefit_fragment: "장애", language: LanguageCodeIR::Korean }),
        gate_evidence_case(GateEvidenceCase { id: "R20_GATE_4", gate: "The integration is painful. It is worth continuing only if actual coverage expands.", proxy: "The proxy metric improved", decision_question: "Can we keep going now?", benefit_fragment: "coverage", language: LanguageCodeIR::English }),
    ];
    let passed = rows.iter().filter(|row| row.pass).count();
    let payload = serde_json::json!({
        "suite": "R20-RUN-0001",
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
