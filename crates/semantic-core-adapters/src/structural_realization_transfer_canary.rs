//! Frozen R22-RUN-0002 held-out transfer for structural realization.

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

fn clean_and_planned(text: &str, language: LanguageCodeIR) -> bool {
    let lower = text.to_lowercase();
    let clean = !lower.contains("compositional_goal_graph")
        && !lower.contains("investigate:")
        && !lower.contains("repair:")
        && !lower.contains("execute:")
        && !lower.contains("local_ordinal")
        && !lower.contains("topic-");
    let planned = if language == LanguageCodeIR::Korean {
        lower.contains("계획") && (lower.contains("아직 실행 결과") || lower.contains("실행 전"))
    } else {
        lower.contains("plan")
            && (lower.contains("not completed")
                || lower.contains("no execution result")
                || lower.contains("before execution"))
    };
    clean && planned
}

struct CrossHistoryCase<'a> {
    id: &'a str,
    setup: [(&'a str, LanguageCodeIR); 3],
    shifts: [(&'a str, LanguageCodeIR); 3],
    indexed: (&'a str, LanguageCodeIR),
    action: (&'a str, LanguageCodeIR),
    target: &'a str,
    target_concept: &'a str,
    rejected: [&'a str; 2],
}

fn cross_history_case(case: CrossHistoryCase<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    for (offset, (text, language)) in case.setup.into_iter().chain(case.shifts).enumerate() {
        api.process_conversation_turn(&request(
            case.id,
            u64::try_from(offset + 1).expect("bounded turn"),
            text,
            language,
        ))
        .expect("cross history setup");
    }
    let shift = api
        .process_conversation_turn(&request(case.id, 7, case.indexed.0, case.indexed.1))
        .expect("cross indexed shift");
    let action = api
        .process_conversation_turn(&request(case.id, 8, case.action.0, case.action.1))
        .expect("cross indexed action");
    let resolved = action
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    Row {
        id: case.id.to_string(),
        category: "cross_language_indexed_topic_history".to_string(),
        trace: vec![shift.output.text, resolved.clone(), action.output.text],
        pass: shift
            .conversation_state
            .active_topics
            .first()
            .is_some_and(|topic| topic.concept_id_hint.as_deref() == Some(case.target_concept))
            && action.grounded_response.is_some()
            && resolved.contains(case.target)
            && case.rejected.iter().all(|term| !resolved.contains(term)),
    }
}

struct SpacedOrdinalCase<'a> {
    id: &'a str,
    text: &'a str,
    required: [&'a str; 2],
    language: LanguageCodeIR,
}

fn spaced_ordinal_case(case: SpacedOrdinalCase<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let response = api
        .process_conversation_turn(&request(case.id, 1, case.text, case.language))
        .expect("spaced ordinal");
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
        category: "spaced_local_ordinal_transfer".to_string(),
        trace: vec![
            response.reference_resolution.resolved_semantic_text,
            format!("subjects={subjects:?}"),
            response.output.text,
        ],
        pass: bound
            && subjects.len() == 2
            && case
                .required
                .iter()
                .all(|term| subjects.iter().any(|subject| subject.contains(term))),
    }
}

struct NestedCase<'a> {
    id: &'a str,
    text: &'a str,
    required: [&'a str; 3],
    language: LanguageCodeIR,
}

fn nested_case(case: NestedCase<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let response = api
        .process_conversation_turn(&request(case.id, 1, case.text, case.language))
        .expect("nested transfer");
    let output = response.output.text.to_lowercase();
    Row {
        id: case.id.to_string(),
        category: "nested_structure_grounded_realization".to_string(),
        trace: vec![
            output.clone(),
            format!("goals={}", response.conversation_state.active_goals.len()),
        ],
        pass: response.conversation_state.active_goals.len() >= 3
            && case.required.iter().all(|term| output.contains(term))
            && clean_and_planned(&output, case.language)
            && response.output.unsupported_freeform_claims == 0,
    }
}

struct OutOfRangeCase<'a> {
    id: &'a str,
    text: &'a str,
    language: LanguageCodeIR,
}

fn out_of_range_case(case: OutOfRangeCase<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let response = api
        .process_conversation_turn(&request(case.id, 1, case.text, case.language))
        .expect("ordinal ambiguity");
    Row {
        id: case.id.to_string(),
        category: "ordinal_out_of_range_fails_closed".to_string(),
        trace: vec![
            response.reference_resolution.resolved_semantic_text,
            format!(
                "ambiguity={:?}",
                response.reference_resolution.ambiguous_reference_surfaces
            ),
            response.output.text,
        ],
        pass: response.disposition == ConversationTurnDispositionIR::ClarificationRequired
            && response.grounded_response.is_none()
            && response.output.grounded_plan_sha256.is_none()
            && !response
                .reference_resolution
                .ambiguous_reference_surfaces
                .is_empty(),
    }
}

fn main() {
    let rows = vec![
        cross_history_case(CrossHistoryCase { id: "R22_TRANSFER_HISTORY_1", setup: [("캐시를 확인해", LanguageCodeIR::Korean), ("inspect the queue", LanguageCodeIR::English), ("로그를 확인해", LanguageCodeIR::Korean)], shifts: [("return to the cache", LanguageCodeIR::English), ("큐 얘기로 돌아가자", LanguageCodeIR::Korean), ("return to the log", LanguageCodeIR::English)], indexed: ("두 주제 전으로 돌아가자", LanguageCodeIR::Korean), action: ("repair it", LanguageCodeIR::English), target: "cache", target_concept: "TOPIC_CACHE", rejected: ["queue", "log"] }),
        cross_history_case(CrossHistoryCase { id: "R22_TRANSFER_HISTORY_2", setup: [("inspect the file", LanguageCodeIR::English), ("폴더를 확인해", LanguageCodeIR::Korean), ("inspect the report", LanguageCodeIR::English)], shifts: [("파일 얘기로 돌아가자", LanguageCodeIR::Korean), ("return to the folder", LanguageCodeIR::English), ("보고서 얘기로 돌아가자", LanguageCodeIR::Korean)], indexed: ("go back two topics", LanguageCodeIR::English), action: ("그거 수리해", LanguageCodeIR::Korean), target: "파일", target_concept: "C_OBJECT_FILE", rejected: ["폴더", "보고서"] }),
        cross_history_case(CrossHistoryCase { id: "R22_TRANSFER_HISTORY_3", setup: [("서버를 확인해", LanguageCodeIR::Korean), ("inspect the worker", LanguageCodeIR::English), ("백업을 확인해", LanguageCodeIR::Korean)], shifts: [("return to the server", LanguageCodeIR::English), ("워커 이야기로 돌아가자", LanguageCodeIR::Korean), ("return to the backup", LanguageCodeIR::English)], indexed: ("return to the topic from two turns ago", LanguageCodeIR::English), action: ("그것을 분석해", LanguageCodeIR::Korean), target: "서버", target_concept: "TOPIC_SERVER", rejected: ["워커", "백업"] }),
        cross_history_case(CrossHistoryCase { id: "R22_TRANSFER_HISTORY_4", setup: [("inspect the cache", LanguageCodeIR::English), ("큐를 확인해", LanguageCodeIR::Korean), ("inspect the backup", LanguageCodeIR::English)], shifts: [("캐시 얘기로 돌아가자", LanguageCodeIR::Korean), ("return to the queue", LanguageCodeIR::English), ("백업 이야기로 돌아가자", LanguageCodeIR::Korean)], indexed: ("두 단계 전 화제로 돌아가자", LanguageCodeIR::Korean), action: ("repair it", LanguageCodeIR::English), target: "cache", target_concept: "TOPIC_CACHE", rejected: ["queue", "backup"] }),
        spaced_ordinal_case(SpacedOrdinalCase { id: "R22_TRANSFER_ORDINAL_1", text: "파일은 오래됐고 폴더는 비었고 보고서는 낡았다. 첫 번째를 확인하고 세 번째를 수리해", required: ["파일", "보고서"], language: LanguageCodeIR::Korean }),
        spaced_ordinal_case(SpacedOrdinalCase { id: "R22_TRANSFER_ORDINAL_2", text: "the cache is stale, the queue is blocked, and the log is empty. repair the second and analyze the third", required: ["queue", "log"], language: LanguageCodeIR::English }),
        spaced_ordinal_case(SpacedOrdinalCase { id: "R22_TRANSFER_ORDINAL_3", text: "서버는 느리고 워커는 막혔고 백업은 낡았다. 두 번째를 분석하고 세 번째를 확인해", required: ["워커", "백업"], language: LanguageCodeIR::Korean }),
        spaced_ordinal_case(SpacedOrdinalCase { id: "R22_TRANSFER_ORDINAL_4", text: "the document is stale, the code is incomplete, and the report is empty. inspect the first and repair the second", required: ["document", "code"], language: LanguageCodeIR::English }),
        nested_case(NestedCase { id: "R22_TRANSFER_NESTED_1", text: "문서를 읽고 코드를 분석한 다음 보고서를 작성해", required: ["문서", "코드", "보고서"], language: LanguageCodeIR::Korean }),
        nested_case(NestedCase { id: "R22_TRANSFER_NESTED_2", text: "open the file, inspect the code, then create the report", required: ["file", "code", "report"], language: LanguageCodeIR::English }),
        nested_case(NestedCase { id: "R22_TRANSFER_NESTED_3", text: "로그를 분석하고 서버를 수리한 뒤 백업을 저장해", required: ["로그", "서버", "백업"], language: LanguageCodeIR::Korean }),
        nested_case(NestedCase { id: "R22_TRANSFER_NESTED_4", text: "inspect the queue, repair the worker, and save the document", required: ["queue", "worker", "document"], language: LanguageCodeIR::English }),
        out_of_range_case(OutOfRangeCase { id: "R22_TRANSFER_RANGE_1", text: "파일은 오래됐고 폴더는 비었다. 세 번째를 수리해", language: LanguageCodeIR::Korean }),
        out_of_range_case(OutOfRangeCase { id: "R22_TRANSFER_RANGE_2", text: "the cache is stale and the queue is blocked. repair the third", language: LanguageCodeIR::English }),
        out_of_range_case(OutOfRangeCase { id: "R22_TRANSFER_RANGE_3", text: "서버는 느리고 워커는 막혔다. 셋째를 분석해", language: LanguageCodeIR::Korean }),
        out_of_range_case(OutOfRangeCase { id: "R22_TRANSFER_RANGE_4", text: "the document is stale and the report is empty. inspect the third", language: LanguageCodeIR::English }),
    ];
    let passed = rows.iter().filter(|row| row.pass).count();
    let payload = serde_json::json!({
        "suite": "R22-RUN-0002",
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
