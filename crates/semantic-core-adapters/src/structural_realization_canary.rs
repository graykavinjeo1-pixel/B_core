//! Frozen R22-RUN-0001 structural composition, history, and realization diagnostic.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, LanguageCodeIR, CONVERSATION_TURN_REQUEST_SCHEMA,
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
        max_plan_steps: 24,
    }
}

fn clean_realization(text: &str) -> bool {
    let lower = text.to_lowercase();
    !lower.contains("compositional_goal_graph")
        && !lower.contains("investigate:")
        && !lower.contains("repair:")
        && !lower.contains("execute:")
        && !lower.contains("local_ordered")
        && !lower.contains("topic-")
}

fn plan_fidelity(text: &str, language: LanguageCodeIR) -> bool {
    let lower = text.to_lowercase();
    let planned = if language == LanguageCodeIR::Korean {
        lower.contains("계획") && (lower.contains("아직 실행 결과") || lower.contains("실행 전"))
    } else {
        lower.contains("plan")
            && (lower.contains("not completed")
                || lower.contains("no execution result")
                || lower.contains("before execution"))
    };
    planned
        && !lower.contains("완료했")
        && !lower.contains("성공했")
        && !lower.contains("completed successfully")
        && !lower.contains("has been executed")
}

struct RealizationCase<'a> {
    id: &'a str,
    text: &'a str,
    required: &'a [&'a str],
    min_goals: usize,
    language: LanguageCodeIR,
    category: &'a str,
}

fn realization_case(case: RealizationCase<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let response = api
        .process_conversation_turn(&request(case.id, 1, case.text, case.language))
        .expect("structured realization");
    let output = response.output.text.to_lowercase();
    let goals = response
        .conversation_state
        .active_goals
        .iter()
        .map(|goal| format!("{:?}:{}", goal.intent, goal.subject))
        .collect::<Vec<_>>();
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        trace: vec![output.clone(), format!("goals={goals:?}")],
        pass: response.disposition == ConversationTurnDispositionIR::Grounded
            && response.grounded_response.is_some()
            && response.output.grounded_plan_sha256.is_some()
            && response.conversation_state.active_goals.len() >= case.min_goals
            && case.required.iter().all(|term| output.contains(term))
            && clean_realization(&output)
            && plan_fidelity(&output, case.language)
            && response.output.unsupported_freeform_claims == 0,
    }
}

struct IndexedTopicCase<'a> {
    id: &'a str,
    setup: [&'a str; 3],
    shifts: [&'a str; 3],
    indexed_shift: &'a str,
    action: &'a str,
    target: &'a str,
    rejected: [&'a str; 2],
    language: LanguageCodeIR,
}

fn indexed_topic_case(case: IndexedTopicCase<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    for (offset, text) in case.setup.into_iter().chain(case.shifts).enumerate() {
        api.process_conversation_turn(&request(
            case.id,
            u64::try_from(offset + 1).expect("bounded turn"),
            text,
            case.language,
        ))
        .expect("topic setup");
    }
    let shift = api
        .process_conversation_turn(&request(case.id, 7, case.indexed_shift, case.language))
        .expect("indexed shift");
    let action = api
        .process_conversation_turn(&request(case.id, 8, case.action, case.language))
        .expect("indexed action");
    let resolved = action
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    Row {
        id: case.id.to_string(),
        category: "indexed_topic_history".to_string(),
        trace: vec![
            shift.output.text,
            resolved.clone(),
            action.output.text.clone(),
        ],
        pass: shift
            .conversation_state
            .active_topics
            .first()
            .is_some_and(|topic| topic.surface.to_lowercase().contains(case.target))
            && action.grounded_response.is_some()
            && resolved.contains(case.target)
            && case.rejected.iter().all(|term| !resolved.contains(term))
            && action.output.unsupported_freeform_claims == 0,
    }
}

struct OrdinalCase<'a> {
    id: &'a str,
    text: &'a str,
    required_subjects: [&'a str; 2],
    language: LanguageCodeIR,
}

fn ordinal_case(case: OrdinalCase<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let response = api
        .process_conversation_turn(&request(case.id, 1, case.text, case.language))
        .expect("ordinal request");
    let resolved = response
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    let subjects = response
        .conversation_state
        .active_goals
        .iter()
        .map(|goal| goal.subject.to_lowercase())
        .collect::<Vec<_>>();
    let bound = response
        .reference_resolution
        .discourse_bindings
        .iter()
        .any(|binding| {
            binding
                .evidence
                .iter()
                .any(|item| item == "SYNTACTIC_PRIORITY:LOCAL_ORDINAL_ANTECEDENTS")
        });
    Row {
        id: case.id.to_string(),
        category: "local_ordinal_binding".to_string(),
        trace: vec![
            resolved,
            format!("subjects={subjects:?}"),
            response.output.text,
        ],
        pass: response.disposition == ConversationTurnDispositionIR::Grounded
            && bound
            && subjects.len() == 2
            && case
                .required_subjects
                .iter()
                .all(|required| subjects.iter().any(|subject| subject.contains(required)))
            && response.output.unsupported_freeform_claims == 0,
    }
}

fn main() {
    let rows = vec![
        realization_case(RealizationCase { id: "R22_REALIZE_1", text: "파일을 분석하고 폴더를 수리해", required: &["파일", "분석", "폴더", "수리"], min_goals: 2, language: LanguageCodeIR::Korean, category: "typed_multi_goal_realization" }),
        realization_case(RealizationCase { id: "R22_REALIZE_2", text: "analyze the cache and repair the queue", required: &["cache", "analy", "queue", "repair"], min_goals: 2, language: LanguageCodeIR::English, category: "typed_multi_goal_realization" }),
        realization_case(RealizationCase { id: "R22_REALIZE_3", text: "서버를 확인한 뒤 로그를 수리해", required: &["서버", "확인", "로그", "수리"], min_goals: 2, language: LanguageCodeIR::Korean, category: "typed_multi_goal_realization" }),
        realization_case(RealizationCase { id: "R22_REALIZE_4", text: "inspect the worker, then repair the backup", required: &["worker", "inspect", "backup", "repair"], min_goals: 2, language: LanguageCodeIR::English, category: "typed_multi_goal_realization" }),
        realization_case(RealizationCase { id: "R22_BLOCKED_1", text: "캐시를 분석하되 큐는 삭제하지 마", required: &["캐시", "분석", "큐", "제외"], min_goals: 1, language: LanguageCodeIR::Korean, category: "prohibition_aware_realization" }),
        realization_case(RealizationCase { id: "R22_BLOCKED_2", text: "analyze the server but do not delete the log", required: &["server", "analy", "log", "excluded"], min_goals: 1, language: LanguageCodeIR::English, category: "prohibition_aware_realization" }),
        realization_case(RealizationCase { id: "R22_BLOCKED_3", text: "파일을 확인하고 보고서는 지우지 마", required: &["파일", "확인", "보고서", "제외"], min_goals: 1, language: LanguageCodeIR::Korean, category: "prohibition_aware_realization" }),
        realization_case(RealizationCase { id: "R22_BLOCKED_4", text: "repair the folder but never delete the backup", required: &["folder", "repair", "backup", "excluded"], min_goals: 1, language: LanguageCodeIR::English, category: "prohibition_aware_realization" }),
        indexed_topic_case(IndexedTopicCase { id: "R22_HISTORY_1", setup: ["캐시를 확인해", "큐를 확인해", "로그를 확인해"], shifts: ["캐시 얘기로 돌아가자", "큐 얘기로 돌아가자", "로그 얘기로 돌아가자"], indexed_shift: "두 주제 전으로 돌아가자", action: "그거 수리해", target: "캐시", rejected: ["큐", "로그"], language: LanguageCodeIR::Korean }),
        indexed_topic_case(IndexedTopicCase { id: "R22_HISTORY_2", setup: ["inspect the file", "inspect the folder", "inspect the report"], shifts: ["return to the file", "return to the folder", "return to the report"], indexed_shift: "go back two topics", action: "repair it", target: "file", rejected: ["folder", "report"], language: LanguageCodeIR::English }),
        indexed_topic_case(IndexedTopicCase { id: "R22_HISTORY_3", setup: ["서버를 확인해", "워커를 확인해", "백업을 확인해"], shifts: ["서버 얘기로 돌아가자", "워커 얘기로 돌아가자", "백업 얘기로 돌아가자"], indexed_shift: "두 단계 전 화제로 돌아가자", action: "그것을 분석해", target: "서버", rejected: ["워커", "백업"], language: LanguageCodeIR::Korean }),
        indexed_topic_case(IndexedTopicCase { id: "R22_HISTORY_4", setup: ["inspect the cache", "inspect the queue", "inspect the log"], shifts: ["return to the cache", "return to the queue", "return to the log"], indexed_shift: "return to the topic from two turns ago", action: "repair it", target: "cache", rejected: ["queue", "log"], language: LanguageCodeIR::English }),
        ordinal_case(OrdinalCase { id: "R22_ORDINAL_1", text: "파일은 오래됐고 폴더는 비었고 보고서는 낡았다. 첫째를 분석하고 셋째를 수리해", required_subjects: ["파일", "보고서"], language: LanguageCodeIR::Korean }),
        ordinal_case(OrdinalCase { id: "R22_ORDINAL_2", text: "the file is stale, the folder is empty, and the report is old. analyze the first and repair the third", required_subjects: ["file", "report"], language: LanguageCodeIR::English }),
        ordinal_case(OrdinalCase { id: "R22_ORDINAL_3", text: "캐시는 오래됐고 큐는 막혔고 로그는 비었다. 둘째를 수리하고 셋째를 분석해", required_subjects: ["큐", "로그"], language: LanguageCodeIR::Korean }),
        ordinal_case(OrdinalCase { id: "R22_ORDINAL_4", text: "the server is slow, the worker is blocked, and the backup is stale. inspect the second and repair the third", required_subjects: ["worker", "backup"], language: LanguageCodeIR::English }),
        realization_case(RealizationCase { id: "R22_NESTED_1", text: "파일을 분석한 뒤 폴더를 수리하고 보고서를 저장해", required: &["파일", "폴더", "보고서"], min_goals: 3, language: LanguageCodeIR::Korean, category: "nested_composition_realization" }),
        realization_case(RealizationCase { id: "R22_NESTED_2", text: "inspect the file, then repair the folder, and save the report", required: &["file", "folder", "report"], min_goals: 3, language: LanguageCodeIR::English, category: "nested_composition_realization" }),
        realization_case(RealizationCase { id: "R22_NESTED_3", text: "캐시를 확인하고 큐를 수리한 뒤 로그를 저장해", required: &["캐시", "큐", "로그"], min_goals: 3, language: LanguageCodeIR::Korean, category: "nested_composition_realization" }),
        realization_case(RealizationCase { id: "R22_NESTED_4", text: "analyze the server, repair the worker, then save the backup", required: &["server", "worker", "backup"], min_goals: 3, language: LanguageCodeIR::English, category: "nested_composition_realization" }),
        realization_case(RealizationCase { id: "R22_STATUS_1", text: "문서를 분석하고 보고서를 작성해", required: &["문서", "보고서"], min_goals: 2, language: LanguageCodeIR::Korean, category: "composed_plan_result_fidelity" }),
        realization_case(RealizationCase { id: "R22_STATUS_2", text: "inspect the code and create the report", required: &["code", "report"], min_goals: 2, language: LanguageCodeIR::English, category: "composed_plan_result_fidelity" }),
        realization_case(RealizationCase { id: "R22_STATUS_3", text: "폴더를 확인하고 파일을 저장해", required: &["폴더", "파일"], min_goals: 2, language: LanguageCodeIR::Korean, category: "composed_plan_result_fidelity" }),
        realization_case(RealizationCase { id: "R22_STATUS_4", text: "analyze the log and repair the server", required: &["log", "server"], min_goals: 2, language: LanguageCodeIR::English, category: "composed_plan_result_fidelity" }),
    ];
    let passed = rows.iter().filter(|row| row.pass).count();
    let payload = serde_json::json!({
        "suite": "R22-RUN-0001",
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
