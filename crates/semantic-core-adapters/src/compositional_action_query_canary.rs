//! Frozen R36-RUN-0001 diagnostic for typed compositional action-set queries.
//!
//! Frozen before the R36 product mechanism exists. The public conversation API
//! must compose a resolved discourse group with inclusion, union, difference,
//! complement, quantification, and an action-state predicate. Language alone
//! cannot create execution evidence or semantic authority.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, LanguageCodeIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    category: String,
    pass: bool,
    trace: Vec<String>,
}

#[derive(Clone, Copy)]
struct Case {
    id: &'static str,
    language: LanguageCodeIR,
    follow: &'static str,
    report_queue: bool,
    expected_subjects: &'static [&'static str],
    rejected_subjects: &'static [&'static str],
    operator: &'static str,
    quantifier: Option<&'static str>,
    predicate: Option<&'static str>,
    truth: Option<&'static str>,
    clarify: bool,
    category: &'static str,
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

fn setup(language: LanguageCodeIR) -> &'static str {
    match language {
        LanguageCodeIR::Korean => "캐시를 확인하고 큐를 수리하고 워커를 분석해",
        _ => "inspect the cache, repair the queue, and analyze the worker",
    }
}

fn queue_report(language: LanguageCodeIR) -> &'static str {
    match language {
        LanguageCodeIR::Korean => "큐 작업은 끝냈어",
        _ => "I completed the queue action",
    }
}

fn selected_subjects(observed: &Value) -> Vec<String> {
    let selected = observed
        .pointer("/action_state_analysis/target_action_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    let mut subjects = observed
        .pointer("/conversation_state/action_state_ledger/records")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|record| {
            record["action_id"]
                .as_str()
                .is_some_and(|id| selected.contains(&id))
        })
        .filter_map(|record| record["subject"].as_str().map(str::to_lowercase))
        .collect::<Vec<_>>();
    subjects.sort();
    subjects
}

fn contains_terms(subjects: &[String], required: &[&str], rejected: &[&str]) -> bool {
    required.iter().all(|term| {
        subjects
            .iter()
            .any(|subject| subject.contains(&term.to_lowercase()))
    }) && rejected.iter().all(|term| {
        subjects
            .iter()
            .all(|subject| !subject.contains(&term.to_lowercase()))
    })
}

fn safe(observed: &Value) -> bool {
    observed
        .pointer("/grounded_realization/unsupported_claims")
        .and_then(Value::as_u64)
        == Some(0)
        && observed
            .pointer("/grounded_realization/semantic_authority")
            .and_then(Value::as_bool)
            == Some(false)
        && observed
            .pointer("/grounded_realization/external_action_executed")
            .and_then(Value::as_bool)
            == Some(false)
        && observed
            .pointer("/action_state_analysis/set_query/semantic_authority")
            .and_then(Value::as_bool)
            == Some(false)
        && observed
            .pointer("/action_state_analysis/set_query/external_action_executed")
            .and_then(Value::as_bool)
            == Some(false)
}

fn run(case: Case) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(case.id, 1, setup(case.language), case.language))
        .expect("three-action setup");
    let mut turn = 2;
    if case.report_queue {
        api.process_conversation_turn(&request(
            case.id,
            turn,
            queue_report(case.language),
            case.language,
        ))
        .expect("queue language report");
        turn += 1;
    }
    let response = api
        .process_conversation_turn(&request(case.id, turn, case.follow, case.language))
        .expect("compositional action query");
    let observed = serde_json::to_value(&response).expect("response json");
    let query = &observed["action_state_analysis"]["set_query"];
    let subjects = selected_subjects(&observed);
    let operator_ok = query["operator_trace"].as_array().is_some_and(|items| {
        items
            .iter()
            .any(|item| item.as_str() == Some(case.operator))
    });
    let quantifier_ok = case
        .quantifier
        .is_none_or(|expected| query["quantifier"].as_str() == Some(expected));
    let predicate_ok = case
        .predicate
        .is_none_or(|expected| query["predicate"].as_str() == Some(expected));
    let truth_ok = case
        .truth
        .is_none_or(|expected| query["truth"].as_str() == Some(expected));
    let has_evaluation_claim = case.predicate.is_none()
        || observed
            .pointer("/grounded_realization/claims")
            .and_then(Value::as_array)
            .is_some_and(|claims| {
                claims
                    .iter()
                    .any(|claim| claim["kind"] == "ACTION_SET_EVALUATION")
            });
    let disposition_ok = if case.clarify {
        response.disposition == ConversationTurnDispositionIR::ClarificationRequired
            && subjects.is_empty()
            && query["unresolved_terms"]
                .as_array()
                .is_some_and(|items| !items.is_empty())
    } else {
        response.disposition == ConversationTurnDispositionIR::Grounded
            && contains_terms(&subjects, case.expected_subjects, case.rejected_subjects)
            && query["source_action_ids"]
                .as_array()
                .is_some_and(|ids| ids.len() == 3)
            && query["selected_action_ids"]
                .as_array()
                .is_some_and(|ids| ids.len() == case.expected_subjects.len())
    };
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        pass: query["schema"] == "B_CORE_ACTION_SET_QUERY_IR_1"
            && operator_ok
            && quantifier_ok
            && predicate_ok
            && truth_ok
            && has_evaluation_claim
            && disposition_ok
            && safe(&observed),
        trace: vec![format!("subjects={subjects:?}"), observed.to_string()],
    }
}

fn main() {
    let rows = vec![
        run(Case { id: "R36_ONLY_EN_1", language: LanguageCodeIR::English, follow: "Among all tasks, show only the cache action's status.", report_queue: false, expected_subjects: &["cache"], rejected_subjects: &["queue", "worker"], operator: "INTERSECTION", quantifier: None, predicate: None, truth: None, clarify: false, category: "group_subject_intersection" }),
        run(Case { id: "R36_ONLY_EN_2", language: LanguageCodeIR::English, follow: "Show the status of just the worker action among all tasks.", report_queue: false, expected_subjects: &["worker"], rejected_subjects: &["cache", "queue"], operator: "INTERSECTION", quantifier: None, predicate: None, truth: None, clarify: false, category: "group_subject_intersection" }),
        run(Case { id: "R36_ONLY_KO_1", language: LanguageCodeIR::Korean, follow: "모든 작업 중 캐시 작업만 상태를 알려줘", report_queue: false, expected_subjects: &["캐시"], rejected_subjects: &["큐", "워커"], operator: "INTERSECTION", quantifier: None, predicate: None, truth: None, clarify: false, category: "group_subject_intersection" }),
        run(Case { id: "R36_ONLY_KO_2", language: LanguageCodeIR::Korean, follow: "워커 작업만 골라서 모든 작업의 상태 중 보여줘", report_queue: false, expected_subjects: &["워커"], rejected_subjects: &["캐시", "큐"], operator: "INTERSECTION", quantifier: None, predicate: None, truth: None, clarify: false, category: "group_subject_intersection" }),

        run(Case { id: "R36_DIFF_EN_1", language: LanguageCodeIR::English, follow: "For all tasks except the queue action, show the status.", report_queue: false, expected_subjects: &["cache", "worker"], rejected_subjects: &["queue"], operator: "DIFFERENCE", quantifier: None, predicate: None, truth: None, clarify: false, category: "group_difference" }),
        run(Case { id: "R36_DIFF_EN_2", language: LanguageCodeIR::English, follow: "List all tasks excluding the cache action and give their status.", report_queue: false, expected_subjects: &["queue", "worker"], rejected_subjects: &["cache"], operator: "DIFFERENCE", quantifier: None, predicate: None, truth: None, clarify: false, category: "group_difference" }),
        run(Case { id: "R36_DIFF_KO_1", language: LanguageCodeIR::Korean, follow: "모든 작업에서 큐 작업은 빼고 상태를 알려줘", report_queue: false, expected_subjects: &["캐시", "워커"], rejected_subjects: &["큐"], operator: "DIFFERENCE", quantifier: None, predicate: None, truth: None, clarify: false, category: "group_difference" }),
        run(Case { id: "R36_DIFF_KO_2", language: LanguageCodeIR::Korean, follow: "캐시 작업을 제외한 모든 작업의 현황을 보여줘", report_queue: false, expected_subjects: &["큐", "워커"], rejected_subjects: &["캐시"], operator: "DIFFERENCE", quantifier: None, predicate: None, truth: None, clarify: false, category: "group_difference" }),

        run(Case { id: "R36_UNION_EN_1", language: LanguageCodeIR::English, follow: "Among all tasks, show only the cache or worker actions' status.", report_queue: false, expected_subjects: &["cache", "worker"], rejected_subjects: &["queue"], operator: "UNION", quantifier: None, predicate: None, truth: None, clarify: false, category: "subject_union" }),
        run(Case { id: "R36_UNION_EN_2", language: LanguageCodeIR::English, follow: "Give the status of either the worker or queue action from all tasks.", report_queue: false, expected_subjects: &["worker", "queue"], rejected_subjects: &["cache"], operator: "UNION", quantifier: None, predicate: None, truth: None, clarify: false, category: "subject_union" }),
        run(Case { id: "R36_UNION_KO_1", language: LanguageCodeIR::Korean, follow: "모든 작업 중 캐시 또는 워커 작업만 상태를 보여줘", report_queue: false, expected_subjects: &["캐시", "워커"], rejected_subjects: &["큐"], operator: "UNION", quantifier: None, predicate: None, truth: None, clarify: false, category: "subject_union" }),
        run(Case { id: "R36_UNION_KO_2", language: LanguageCodeIR::Korean, follow: "큐나 워커 작업의 상태만 모든 작업에서 골라 알려줘", report_queue: false, expected_subjects: &["큐", "워커"], rejected_subjects: &["캐시"], operator: "UNION", quantifier: None, predicate: None, truth: None, clarify: false, category: "subject_union" }),

        run(Case { id: "R36_COMP_EN_1", language: LanguageCodeIR::English, follow: "For all tasks, exclude both the cache and queue actions and show what remains.", report_queue: false, expected_subjects: &["worker"], rejected_subjects: &["cache", "queue"], operator: "COMPLEMENT", quantifier: None, predicate: None, truth: None, clarify: false, category: "scoped_complement" }),
        run(Case { id: "R36_COMP_EN_2", language: LanguageCodeIR::English, follow: "From all tasks show neither the queue nor worker action, only what remains, with status.", report_queue: false, expected_subjects: &["cache"], rejected_subjects: &["queue", "worker"], operator: "COMPLEMENT", quantifier: None, predicate: None, truth: None, clarify: false, category: "scoped_complement" }),
        run(Case { id: "R36_COMP_KO_1", language: LanguageCodeIR::Korean, follow: "모든 작업에서 캐시와 큐 작업을 둘 다 빼고 남은 상태를 보여줘", report_queue: false, expected_subjects: &["워커"], rejected_subjects: &["캐시", "큐"], operator: "COMPLEMENT", quantifier: None, predicate: None, truth: None, clarify: false, category: "scoped_complement" }),
        run(Case { id: "R36_COMP_KO_2", language: LanguageCodeIR::Korean, follow: "큐도 워커도 아닌 작업만 모든 작업 중 상태를 알려줘", report_queue: false, expected_subjects: &["캐시"], rejected_subjects: &["큐", "워커"], operator: "COMPLEMENT", quantifier: None, predicate: None, truth: None, clarify: false, category: "scoped_complement" }),

        run(Case { id: "R36_ALL_EN_1", language: LanguageCodeIR::English, follow: "Do all tasks still lack verified execution results?", report_queue: false, expected_subjects: &["cache", "queue", "worker"], rejected_subjects: &[], operator: "IDENTITY", quantifier: Some("ALL"), predicate: Some("UNVERIFIED_EXECUTION"), truth: Some("TRUE"), clarify: false, category: "universal_state_predicate" }),
        run(Case { id: "R36_ALL_EN_2", language: LanguageCodeIR::English, follow: "Is every task still an active plan without a verified result?", report_queue: false, expected_subjects: &["cache", "queue", "worker"], rejected_subjects: &[], operator: "IDENTITY", quantifier: Some("ALL"), predicate: Some("UNVERIFIED_EXECUTION"), truth: Some("TRUE"), clarify: false, category: "universal_state_predicate" }),
        run(Case { id: "R36_ALL_KO_1", language: LanguageCodeIR::Korean, follow: "모든 작업에 아직 검증된 실행 결과가 없는 거야?", report_queue: false, expected_subjects: &["캐시", "큐", "워커"], rejected_subjects: &[], operator: "IDENTITY", quantifier: Some("ALL"), predicate: Some("UNVERIFIED_EXECUTION"), truth: Some("TRUE"), clarify: false, category: "universal_state_predicate" }),
        run(Case { id: "R36_ALL_KO_2", language: LanguageCodeIR::Korean, follow: "작업 전부가 검증 결과 없는 활성 계획 상태야?", report_queue: false, expected_subjects: &["캐시", "큐", "워커"], rejected_subjects: &[], operator: "IDENTITY", quantifier: Some("ALL"), predicate: Some("UNVERIFIED_EXECUTION"), truth: Some("TRUE"), clarify: false, category: "universal_state_predicate" }),

        run(Case { id: "R36_ANY_EN_1", language: LanguageCodeIR::English, follow: "Do any of all tasks have a reported completion status?", report_queue: true, expected_subjects: &["cache", "queue", "worker"], rejected_subjects: &[], operator: "IDENTITY", quantifier: Some("ANY"), predicate: Some("REPORTED_COMPLETION"), truth: Some("TRUE"), clarify: false, category: "existential_report_predicate" }),
        run(Case { id: "R36_ANY_EN_2", language: LanguageCodeIR::English, follow: "Has at least one of all tasks been reported complete?", report_queue: true, expected_subjects: &["cache", "queue", "worker"], rejected_subjects: &[], operator: "IDENTITY", quantifier: Some("ANY"), predicate: Some("REPORTED_COMPLETION"), truth: Some("TRUE"), clarify: false, category: "existential_report_predicate" }),
        run(Case { id: "R36_ANY_KO_1", language: LanguageCodeIR::Korean, follow: "모든 작업 중 하나라도 완료 보고 상태가 있어?", report_queue: true, expected_subjects: &["캐시", "큐", "워커"], rejected_subjects: &[], operator: "IDENTITY", quantifier: Some("ANY"), predicate: Some("REPORTED_COMPLETION"), truth: Some("TRUE"), clarify: false, category: "existential_report_predicate" }),
        run(Case { id: "R36_ANY_KO_2", language: LanguageCodeIR::Korean, follow: "작업 전부 중 적어도 하나는 완료됐다고 보고됐어?", report_queue: true, expected_subjects: &["캐시", "큐", "워커"], rejected_subjects: &[], operator: "IDENTITY", quantifier: Some("ANY"), predicate: Some("REPORTED_COMPLETION"), truth: Some("TRUE"), clarify: false, category: "existential_report_predicate" }),

        run(Case { id: "R36_EMPTY_EN_1", language: LanguageCodeIR::English, follow: "For all tasks except cache, queue, and worker, show the status.", report_queue: false, expected_subjects: &[], rejected_subjects: &[], operator: "COMPLEMENT", quantifier: None, predicate: None, truth: None, clarify: true, category: "empty_or_unknown_fails_closed" }),
        run(Case { id: "R36_EMPTY_EN_2", language: LanguageCodeIR::English, follow: "Among all tasks show only the database action's status.", report_queue: false, expected_subjects: &[], rejected_subjects: &[], operator: "INTERSECTION", quantifier: None, predicate: None, truth: None, clarify: true, category: "empty_or_unknown_fails_closed" }),
        run(Case { id: "R36_EMPTY_KO_1", language: LanguageCodeIR::Korean, follow: "모든 작업에서 캐시와 큐와 워커를 빼고 상태를 알려줘", report_queue: false, expected_subjects: &[], rejected_subjects: &[], operator: "COMPLEMENT", quantifier: None, predicate: None, truth: None, clarify: true, category: "empty_or_unknown_fails_closed" }),
        run(Case { id: "R36_EMPTY_KO_2", language: LanguageCodeIR::Korean, follow: "모든 작업 중 데이터베이스 작업만 상태를 보여줘", report_queue: false, expected_subjects: &[], rejected_subjects: &[], operator: "INTERSECTION", quantifier: None, predicate: None, truth: None, clarify: true, category: "empty_or_unknown_fails_closed" }),
    ];
    let passed = rows.iter().filter(|row| row.pass).count();
    let report = json!({
        "suite": "R36-RUN-0001",
        "frozen_before_product_changes": true,
        "total": rows.len(),
        "passed": passed,
        "failed": rows.len() - passed,
        "external_llm_calls": 0,
        "local_teacher_calls": 0,
        "network_calls": 0,
        "recursive_source_mutations": 0,
        "rows": rows,
    });
    println!("{}", serde_json::to_string_pretty(&report).expect("report"));
    if passed != report["total"].as_u64().unwrap_or_default() as usize {
        std::process::exit(1);
    }
}
