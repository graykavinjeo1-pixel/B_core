//! Frozen R37-RUN-0001 diagnostic for recursive action-set expressions.
//!
//! Frozen before the R37 product mechanism exists. The suite requires a typed
//! recursive AST, not merely the correct final action IDs.

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
    expected: &'static [&'static str],
    rejected: &'static [&'static str],
    root_kind: &'static str,
    required_kinds: &'static [&'static str],
    min_depth: usize,
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

fn report(language: LanguageCodeIR) -> &'static str {
    match language {
        LanguageCodeIR::Korean => "큐 작업은 완료했어",
        _ => "I completed the queue action",
    }
}

fn selected_subjects(value: &Value) -> Vec<String> {
    let ids = value
        .pointer("/action_state_analysis/target_action_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    let mut subjects = value
        .pointer("/conversation_state/action_state_ledger/records")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|record| {
            record["action_id"]
                .as_str()
                .is_some_and(|id| ids.contains(&id))
        })
        .filter_map(|record| record["subject"].as_str().map(str::to_lowercase))
        .collect::<Vec<_>>();
    subjects.sort();
    subjects
}

fn expression_depth(value: &Value) -> usize {
    let child_depth = ["left", "right", "source", "excluded"]
        .into_iter()
        .filter_map(|field| value.get(field))
        .map(expression_depth)
        .max()
        .unwrap_or(0);
    usize::from(value.get("kind").is_some()) + child_depth
}

fn expression_kinds(value: &Value, kinds: &mut Vec<String>) {
    if let Some(kind) = value.get("kind").and_then(Value::as_str) {
        kinds.push(kind.to_string());
    }
    for field in ["left", "right", "source", "excluded"] {
        if let Some(child) = value.get(field) {
            expression_kinds(child, kinds);
        }
    }
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
            report(case.language),
            case.language,
        ))
        .expect("queue report");
        turn += 1;
    }
    let response = api
        .process_conversation_turn(&request(case.id, turn, case.follow, case.language))
        .expect("recursive query");
    let value = serde_json::to_value(&response).expect("response json");
    let query = &value["action_state_analysis"]["set_query"];
    let expression = &query["expression"];
    let subjects = selected_subjects(&value);
    let mut kinds = Vec::new();
    expression_kinds(expression, &mut kinds);
    let selection_ok = case.expected.iter().all(|term| {
        subjects
            .iter()
            .any(|subject| subject.contains(&term.to_lowercase()))
    }) && case.rejected.iter().all(|term| {
        subjects
            .iter()
            .all(|subject| !subject.contains(&term.to_lowercase()))
    });
    let disposition_ok = if case.clarify {
        response.disposition == ConversationTurnDispositionIR::ClarificationRequired
            && subjects.is_empty()
            && query["unresolved_terms"]
                .as_array()
                .is_some_and(|items| !items.is_empty())
    } else {
        response.disposition == ConversationTurnDispositionIR::Grounded
            && selection_ok
            && query["selected_action_ids"]
                .as_array()
                .is_some_and(|ids| ids.len() == case.expected.len())
    };
    let ast_ok = if case.clarify {
        expression.is_null()
    } else {
        expression["kind"] == case.root_kind
            && expression_depth(expression) >= case.min_depth
            && case
                .required_kinds
                .iter()
                .all(|kind| kinds.iter().any(|actual| actual == kind))
    };
    let safe = value
        .pointer("/grounded_realization/unsupported_claims")
        .and_then(Value::as_u64)
        == Some(0)
        && value
            .pointer("/grounded_realization/semantic_authority")
            .and_then(Value::as_bool)
            == Some(false)
        && value
            .pointer("/grounded_realization/external_action_executed")
            .and_then(Value::as_bool)
            == Some(false)
        && query["semantic_authority"] == false
        && query["external_action_executed"] == false;
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        pass: query["schema"] == "B_CORE_ACTION_SET_QUERY_IR_1"
            && query["source_action_ids"]
                .as_array()
                .is_some_and(|ids| ids.len() == 3)
            && disposition_ok
            && ast_ok
            && safe,
        trace: vec![
            format!("subjects={subjects:?}; kinds={kinds:?}"),
            value.to_string(),
        ],
    }
}

fn main() {
    let rows = vec![
        run(Case {
            id: "R37_PREC_EN_1",
            language: LanguageCodeIR::English,
            follow: "From all tasks, show the status of (cache or queue) except cache.",
            report_queue: false,
            expected: &["queue"],
            rejected: &["cache", "worker"],
            root_kind: "DIFFERENCE",
            required_kinds: &["UNION", "SUBJECT_TERM"],
            min_depth: 3,
            clarify: false,
            category: "parenthesized_precedence",
        }),
        run(Case {
            id: "R37_PREC_EN_2",
            language: LanguageCodeIR::English,
            follow: "From all tasks, show the status of cache or (queue except queue).",
            report_queue: false,
            expected: &["cache"],
            rejected: &["queue", "worker"],
            root_kind: "UNION",
            required_kinds: &["DIFFERENCE", "SUBJECT_TERM"],
            min_depth: 3,
            clarify: false,
            category: "parenthesized_precedence",
        }),
        run(Case {
            id: "R37_PREC_KO_1",
            language: LanguageCodeIR::Korean,
            follow: "모든 작업에서 (캐시 또는 큐) 중 캐시를 빼고 남은 상태를 보여줘",
            report_queue: false,
            expected: &["큐"],
            rejected: &["캐시", "워커"],
            root_kind: "DIFFERENCE",
            required_kinds: &["UNION", "SUBJECT_TERM"],
            min_depth: 3,
            clarify: false,
            category: "parenthesized_precedence",
        }),
        run(Case {
            id: "R37_PREC_KO_2",
            language: LanguageCodeIR::Korean,
            follow: "모든 작업에서 캐시 또는 (큐 중 큐를 빼고 남은 것)의 상태를 보여줘",
            report_queue: false,
            expected: &["캐시"],
            rejected: &["큐", "워커"],
            root_kind: "UNION",
            required_kinds: &["DIFFERENCE", "SUBJECT_TERM"],
            min_depth: 3,
            clarify: false,
            category: "parenthesized_precedence",
        }),
        run(Case {
            id: "R37_SCOPE_EN_1",
            language: LanguageCodeIR::English,
            follow: "From all tasks show the status of not (cache or queue).",
            report_queue: false,
            expected: &["worker"],
            rejected: &["cache", "queue"],
            root_kind: "COMPLEMENT",
            required_kinds: &["UNION", "SOURCE_SET"],
            min_depth: 3,
            clarify: false,
            category: "complement_scope",
        }),
        run(Case {
            id: "R37_SCOPE_EN_2",
            language: LanguageCodeIR::English,
            follow: "From all tasks show the status of (not cache) or queue.",
            report_queue: false,
            expected: &["queue", "worker"],
            rejected: &["cache"],
            root_kind: "UNION",
            required_kinds: &["COMPLEMENT", "SOURCE_SET"],
            min_depth: 3,
            clarify: false,
            category: "complement_scope",
        }),
        run(Case {
            id: "R37_SCOPE_KO_1",
            language: LanguageCodeIR::Korean,
            follow: "모든 작업에서 (캐시 또는 큐)를 제외한 나머지 상태를 보여줘",
            report_queue: false,
            expected: &["워커"],
            rejected: &["캐시", "큐"],
            root_kind: "COMPLEMENT",
            required_kinds: &["UNION", "SOURCE_SET"],
            min_depth: 3,
            clarify: false,
            category: "complement_scope",
        }),
        run(Case {
            id: "R37_SCOPE_KO_2",
            language: LanguageCodeIR::Korean,
            follow: "(모든 작업에서 캐시를 뺀 것) 또는 캐시 작업의 상태를 보여줘",
            report_queue: false,
            expected: &["캐시", "큐", "워커"],
            rejected: &[],
            root_kind: "UNION",
            required_kinds: &["DIFFERENCE", "SOURCE_SET"],
            min_depth: 3,
            clarify: false,
            category: "complement_scope",
        }),
        run(Case {
            id: "R37_REL_EN_1",
            language: LanguageCodeIR::English,
            follow: "Among all tasks, show the actions with a reported completion.",
            report_queue: true,
            expected: &["queue"],
            rejected: &["cache", "worker"],
            root_kind: "INTERSECTION",
            required_kinds: &["SOURCE_SET", "STATE_PREDICATE"],
            min_depth: 2,
            clarify: false,
            category: "reported_relative_filter",
        }),
        run(Case {
            id: "R37_REL_EN_2",
            language: LanguageCodeIR::English,
            follow: "List all tasks that were reported complete.",
            report_queue: true,
            expected: &["queue"],
            rejected: &["cache", "worker"],
            root_kind: "INTERSECTION",
            required_kinds: &["SOURCE_SET", "STATE_PREDICATE"],
            min_depth: 2,
            clarify: false,
            category: "reported_relative_filter",
        }),
        run(Case {
            id: "R37_REL_KO_1",
            language: LanguageCodeIR::Korean,
            follow: "모든 작업 중 완료 보고가 있는 작업만 상태를 보여줘",
            report_queue: true,
            expected: &["큐"],
            rejected: &["캐시", "워커"],
            root_kind: "INTERSECTION",
            required_kinds: &["SOURCE_SET", "STATE_PREDICATE"],
            min_depth: 2,
            clarify: false,
            category: "reported_relative_filter",
        }),
        run(Case {
            id: "R37_REL_KO_2",
            language: LanguageCodeIR::Korean,
            follow: "완료됐다고 보고된 작업만 모든 작업에서 골라 현황을 알려줘",
            report_queue: true,
            expected: &["큐"],
            rejected: &["캐시", "워커"],
            root_kind: "INTERSECTION",
            required_kinds: &["SOURCE_SET", "STATE_PREDICATE"],
            min_depth: 2,
            clarify: false,
            category: "reported_relative_filter",
        }),
        run(Case {
            id: "R37_NEGREL_EN_1",
            language: LanguageCodeIR::English,
            follow: "Among all tasks show the actions without a completion report.",
            report_queue: true,
            expected: &["cache", "worker"],
            rejected: &["queue"],
            root_kind: "INTERSECTION",
            required_kinds: &["SOURCE_SET", "STATE_PREDICATE"],
            min_depth: 2,
            clarify: false,
            category: "negated_report_filter",
        }),
        run(Case {
            id: "R37_NEGREL_EN_2",
            language: LanguageCodeIR::English,
            follow: "List all tasks that were not reported complete.",
            report_queue: true,
            expected: &["cache", "worker"],
            rejected: &["queue"],
            root_kind: "INTERSECTION",
            required_kinds: &["SOURCE_SET", "STATE_PREDICATE"],
            min_depth: 2,
            clarify: false,
            category: "negated_report_filter",
        }),
        run(Case {
            id: "R37_NEGREL_KO_1",
            language: LanguageCodeIR::Korean,
            follow: "모든 작업 중 완료 보고가 없는 작업의 상태를 보여줘",
            report_queue: true,
            expected: &["캐시", "워커"],
            rejected: &["큐"],
            root_kind: "INTERSECTION",
            required_kinds: &["SOURCE_SET", "STATE_PREDICATE"],
            min_depth: 2,
            clarify: false,
            category: "negated_report_filter",
        }),
        run(Case {
            id: "R37_NEGREL_KO_2",
            language: LanguageCodeIR::Korean,
            follow: "완료됐다고 보고되지 않은 작업만 모든 작업에서 골라줘",
            report_queue: true,
            expected: &["캐시", "워커"],
            rejected: &["큐"],
            root_kind: "INTERSECTION",
            required_kinds: &["SOURCE_SET", "STATE_PREDICATE"],
            min_depth: 2,
            clarify: false,
            category: "negated_report_filter",
        }),
        run(Case {
            id: "R37_POR_EN_1",
            language: LanguageCodeIR::English,
            follow: "From all tasks show (tasks with a completion report) or cache.",
            report_queue: true,
            expected: &["queue", "cache"],
            rejected: &["worker"],
            root_kind: "UNION",
            required_kinds: &["INTERSECTION", "STATE_PREDICATE", "SUBJECT_TERM"],
            min_depth: 3,
            clarify: false,
            category: "predicate_union",
        }),
        run(Case {
            id: "R37_POR_EN_2",
            language: LanguageCodeIR::English,
            follow: "Show cache or (all tasks reported complete) from all tasks.",
            report_queue: true,
            expected: &["cache", "queue"],
            rejected: &["worker"],
            root_kind: "UNION",
            required_kinds: &["INTERSECTION", "STATE_PREDICATE", "SUBJECT_TERM"],
            min_depth: 3,
            clarify: false,
            category: "predicate_union",
        }),
        run(Case {
            id: "R37_POR_KO_1",
            language: LanguageCodeIR::Korean,
            follow: "모든 작업에서 (완료 보고가 있는 작업) 또는 캐시 작업의 상태를 보여줘",
            report_queue: true,
            expected: &["큐", "캐시"],
            rejected: &["워커"],
            root_kind: "UNION",
            required_kinds: &["INTERSECTION", "STATE_PREDICATE", "SUBJECT_TERM"],
            min_depth: 3,
            clarify: false,
            category: "predicate_union",
        }),
        run(Case {
            id: "R37_POR_KO_2",
            language: LanguageCodeIR::Korean,
            follow: "캐시 작업 또는 (모든 작업 중 완료됐다고 보고된 작업)의 현황을 알려줘",
            report_queue: true,
            expected: &["캐시", "큐"],
            rejected: &["워커"],
            root_kind: "UNION",
            required_kinds: &["INTERSECTION", "STATE_PREDICATE", "SUBJECT_TERM"],
            min_depth: 3,
            clarify: false,
            category: "predicate_union",
        }),
        run(Case {
            id: "R37_INT_EN_1",
            language: LanguageCodeIR::English,
            follow: "From all tasks show (cache or worker) that were not reported complete.",
            report_queue: true,
            expected: &["cache", "worker"],
            rejected: &["queue"],
            root_kind: "INTERSECTION",
            required_kinds: &["UNION", "STATE_PREDICATE"],
            min_depth: 3,
            clarify: false,
            category: "nested_relative_intersection",
        }),
        run(Case {
            id: "R37_INT_EN_2",
            language: LanguageCodeIR::English,
            follow: "Show (worker or queue) with a reported completion from all tasks.",
            report_queue: true,
            expected: &["queue"],
            rejected: &["cache", "worker"],
            root_kind: "INTERSECTION",
            required_kinds: &["UNION", "STATE_PREDICATE"],
            min_depth: 3,
            clarify: false,
            category: "nested_relative_intersection",
        }),
        run(Case {
            id: "R37_INT_KO_1",
            language: LanguageCodeIR::Korean,
            follow: "모든 작업에서 (캐시 또는 워커) 중 완료 보고가 없는 작업을 보여줘",
            report_queue: true,
            expected: &["캐시", "워커"],
            rejected: &["큐"],
            root_kind: "INTERSECTION",
            required_kinds: &["UNION", "STATE_PREDICATE"],
            min_depth: 3,
            clarify: false,
            category: "nested_relative_intersection",
        }),
        run(Case {
            id: "R37_INT_KO_2",
            language: LanguageCodeIR::Korean,
            follow: "(워커 또는 큐) 중 완료 보고가 있는 작업만 모든 작업에서 골라줘",
            report_queue: true,
            expected: &["큐"],
            rejected: &["캐시", "워커"],
            root_kind: "INTERSECTION",
            required_kinds: &["UNION", "STATE_PREDICATE"],
            min_depth: 3,
            clarify: false,
            category: "nested_relative_intersection",
        }),
        run(Case {
            id: "R37_BAD_EN_1",
            language: LanguageCodeIR::English,
            follow: "From all tasks show (cache or queue status.",
            report_queue: false,
            expected: &[],
            rejected: &[],
            root_kind: "",
            required_kinds: &[],
            min_depth: 0,
            clarify: true,
            category: "malformed_or_unknown_fails_closed",
        }),
        run(Case {
            id: "R37_BAD_EN_2",
            language: LanguageCodeIR::English,
            follow: "From all tasks show cache or (database except queue).",
            report_queue: false,
            expected: &[],
            rejected: &[],
            root_kind: "",
            required_kinds: &[],
            min_depth: 0,
            clarify: true,
            category: "malformed_or_unknown_fails_closed",
        }),
        run(Case {
            id: "R37_BAD_KO_1",
            language: LanguageCodeIR::Korean,
            follow: "모든 작업에서 (캐시 또는 큐 상태를 보여줘",
            report_queue: false,
            expected: &[],
            rejected: &[],
            root_kind: "",
            required_kinds: &[],
            min_depth: 0,
            clarify: true,
            category: "malformed_or_unknown_fails_closed",
        }),
        run(Case {
            id: "R37_BAD_KO_2",
            language: LanguageCodeIR::Korean,
            follow: "모든 작업에서 캐시 또는 (데이터베이스 중 큐를 뺀 것)을 보여줘",
            report_queue: false,
            expected: &[],
            rejected: &[],
            root_kind: "",
            required_kinds: &[],
            min_depth: 0,
            clarify: true,
            category: "malformed_or_unknown_fails_closed",
        }),
    ];
    let passed = rows.iter().filter(|row| row.pass).count();
    let report = json!({
        "suite": "R37-RUN-0001", "frozen_before_product_changes": true,
        "total": rows.len(), "passed": passed, "failed": rows.len() - passed,
        "external_llm_calls": 0, "local_teacher_calls": 0, "network_calls": 0,
        "recursive_source_mutations": 0, "rows": rows,
    });
    println!("{}", serde_json::to_string_pretty(&report).expect("report"));
    if passed != report["total"].as_u64().unwrap_or_default() as usize {
        std::process::exit(1);
    }
}
