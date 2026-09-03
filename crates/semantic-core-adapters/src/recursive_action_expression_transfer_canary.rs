//! Frozen R37-TRANSFER-0001 held-out recursive-expression transfer suite.

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
    report_folder: bool,
    expected: &'static [&'static str],
    rejected: &'static [&'static str],
    root: &'static str,
    kinds: &'static [&'static str],
    depth: usize,
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
        LanguageCodeIR::Korean => "파일을 확인하고 폴더를 수리하고 코드를 분석해",
        _ => "inspect the file, repair the folder, and analyze the code",
    }
}
fn report(language: LanguageCodeIR) -> &'static str {
    match language {
        LanguageCodeIR::Korean => "폴더 작업은 완료했어",
        _ => "I finished the folder action",
    }
}

fn subjects(value: &Value) -> Vec<String> {
    let ids = value
        .pointer("/action_state_analysis/target_action_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    let mut values = value
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
    values.sort();
    values
}
fn walk(value: &Value, depth: usize, max_depth: &mut usize, kinds: &mut Vec<String>) {
    if let Some(kind) = value.get("kind").and_then(Value::as_str) {
        kinds.push(kind.to_string());
        *max_depth = (*max_depth).max(depth);
    }
    for field in ["left", "right", "source", "excluded"] {
        if let Some(child) = value.get(field) {
            walk(child, depth + 1, max_depth, kinds);
        }
    }
}
fn run(case: Case) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("core");
    api.process_conversation_turn(&request(case.id, 1, setup(case.language), case.language))
        .expect("setup");
    let mut turn = 2;
    if case.report_folder {
        api.process_conversation_turn(&request(
            case.id,
            turn,
            report(case.language),
            case.language,
        ))
        .expect("report");
        turn += 1;
    }
    let response = api
        .process_conversation_turn(&request(case.id, turn, case.follow, case.language))
        .expect("query");
    let value = serde_json::to_value(&response).expect("json");
    let query = &value["action_state_analysis"]["set_query"];
    let expression = &query["expression"];
    let selected = subjects(&value);
    let mut max_depth = 0;
    let mut actual_kinds = Vec::new();
    walk(expression, 1, &mut max_depth, &mut actual_kinds);
    let terms = case.expected.iter().all(|term| {
        selected
            .iter()
            .any(|subject| subject.contains(&term.to_lowercase()))
    }) && case.rejected.iter().all(|term| {
        selected
            .iter()
            .all(|subject| !subject.contains(&term.to_lowercase()))
    });
    let pass = response.disposition == ConversationTurnDispositionIR::Grounded
        && query["schema"] == "B_CORE_ACTION_SET_QUERY_IR_1"
        && query["source_action_ids"]
            .as_array()
            .is_some_and(|ids| ids.len() == 3)
        && query["selected_action_ids"]
            .as_array()
            .is_some_and(|ids| ids.len() == case.expected.len())
        && expression["kind"] == case.root
        && max_depth >= case.depth
        && case
            .kinds
            .iter()
            .all(|kind| actual_kinds.iter().any(|actual| actual == kind))
        && terms
        && value
            .pointer("/grounded_realization/unsupported_claims")
            .and_then(Value::as_u64)
            == Some(0)
        && query["semantic_authority"] == false
        && query["external_action_executed"] == false;
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        pass,
        trace: vec![
            format!("subjects={selected:?}; kinds={actual_kinds:?}"),
            value.to_string(),
        ],
    }
}

fn main() {
    let rows = vec![
        run(Case {
            id: "R37X_PREC_EN_1",
            language: LanguageCodeIR::English,
            follow: "From all tasks list (file or folder) apart from file.",
            report_folder: false,
            expected: &["folder"],
            rejected: &["file", "code"],
            root: "DIFFERENCE",
            kinds: &["UNION", "SUBJECT_TERM"],
            depth: 3,
            category: "heldout_precedence_scope",
        }),
        run(Case {
            id: "R37X_PREC_EN_2",
            language: LanguageCodeIR::English,
            follow: "From all tasks list file or (folder apart from folder).",
            report_folder: false,
            expected: &["file"],
            rejected: &["folder", "code"],
            root: "UNION",
            kinds: &["DIFFERENCE", "SUBJECT_TERM"],
            depth: 3,
            category: "heldout_precedence_scope",
        }),
        run(Case {
            id: "R37X_PREC_KO_1",
            language: LanguageCodeIR::Korean,
            follow: "모든 작업에서 (파일 또는 폴더) 중 파일 말고 남은 현황을 알려줘",
            report_folder: false,
            expected: &["폴더"],
            rejected: &["파일", "코드"],
            root: "DIFFERENCE",
            kinds: &["UNION", "SUBJECT_TERM"],
            depth: 3,
            category: "heldout_precedence_scope",
        }),
        run(Case {
            id: "R37X_PREC_KO_2",
            language: LanguageCodeIR::Korean,
            follow: "파일 또는 (폴더 중 폴더 말고 남은 것)을 모든 작업에서 골라줘",
            report_folder: false,
            expected: &["파일"],
            rejected: &["폴더", "코드"],
            root: "UNION",
            kinds: &["DIFFERENCE", "SUBJECT_TERM"],
            depth: 3,
            category: "heldout_precedence_scope",
        }),
        run(Case {
            id: "R37X_COMP_EN_1",
            language: LanguageCodeIR::English,
            follow: "From all tasks give the status of not (file or folder).",
            report_folder: false,
            expected: &["code"],
            rejected: &["file", "folder"],
            root: "COMPLEMENT",
            kinds: &["UNION", "SOURCE_SET"],
            depth: 3,
            category: "heldout_complement_scope",
        }),
        run(Case {
            id: "R37X_COMP_EN_2",
            language: LanguageCodeIR::English,
            follow: "Give the status of (not file) or folder from all tasks.",
            report_folder: false,
            expected: &["folder", "code"],
            rejected: &["file"],
            root: "UNION",
            kinds: &["COMPLEMENT", "SOURCE_SET"],
            depth: 3,
            category: "heldout_complement_scope",
        }),
        run(Case {
            id: "R37X_COMP_KO_1",
            language: LanguageCodeIR::Korean,
            follow: "모든 작업에서 (파일 또는 폴더)를 제외한 나머지 상태는?",
            report_folder: false,
            expected: &["코드"],
            rejected: &["파일", "폴더"],
            root: "COMPLEMENT",
            kinds: &["UNION", "SOURCE_SET"],
            depth: 3,
            category: "heldout_complement_scope",
        }),
        run(Case {
            id: "R37X_COMP_KO_2",
            language: LanguageCodeIR::Korean,
            follow: "(모든 작업에서 파일을 제외한 것) 또는 파일의 상태는?",
            report_folder: false,
            expected: &["파일", "폴더", "코드"],
            rejected: &[],
            root: "UNION",
            kinds: &["DIFFERENCE", "SOURCE_SET"],
            depth: 3,
            category: "heldout_complement_scope",
        }),
        run(Case {
            id: "R37X_REL_EN_1",
            language: LanguageCodeIR::English,
            follow: "Among all tasks list those having a completion report.",
            report_folder: true,
            expected: &["folder"],
            rejected: &["file", "code"],
            root: "INTERSECTION",
            kinds: &["STATE_PREDICATE", "SOURCE_SET"],
            depth: 2,
            category: "heldout_relative_filters",
        }),
        run(Case {
            id: "R37X_REL_EN_2",
            language: LanguageCodeIR::English,
            follow: "Among all tasks list those lacking a completion report.",
            report_folder: true,
            expected: &["file", "code"],
            rejected: &["folder"],
            root: "INTERSECTION",
            kinds: &["STATE_PREDICATE", "SOURCE_SET"],
            depth: 2,
            category: "heldout_relative_filters",
        }),
        run(Case {
            id: "R37X_REL_KO_1",
            language: LanguageCodeIR::Korean,
            follow: "모든 작업 가운데 완료 보고를 가진 작업만 알려줘",
            report_folder: true,
            expected: &["폴더"],
            rejected: &["파일", "코드"],
            root: "INTERSECTION",
            kinds: &["STATE_PREDICATE", "SOURCE_SET"],
            depth: 2,
            category: "heldout_relative_filters",
        }),
        run(Case {
            id: "R37X_REL_KO_2",
            language: LanguageCodeIR::Korean,
            follow: "모든 작업 가운데 완료 보고가 빠진 작업만 알려줘",
            report_folder: true,
            expected: &["파일", "코드"],
            rejected: &["폴더"],
            root: "INTERSECTION",
            kinds: &["STATE_PREDICATE", "SOURCE_SET"],
            depth: 2,
            category: "heldout_relative_filters",
        }),
        run(Case {
            id: "R37X_POR_EN_1",
            language: LanguageCodeIR::English,
            follow: "From all tasks show (those having a completion report) or file.",
            report_folder: true,
            expected: &["folder", "file"],
            rejected: &["code"],
            root: "UNION",
            kinds: &["INTERSECTION", "STATE_PREDICATE"],
            depth: 3,
            category: "heldout_predicate_composition",
        }),
        run(Case {
            id: "R37X_POR_EN_2",
            language: LanguageCodeIR::English,
            follow: "Show (file or code) that lack a completion report from all tasks.",
            report_folder: true,
            expected: &["file", "code"],
            rejected: &["folder"],
            root: "INTERSECTION",
            kinds: &["UNION", "STATE_PREDICATE"],
            depth: 3,
            category: "heldout_predicate_composition",
        }),
        run(Case {
            id: "R37X_POR_KO_1",
            language: LanguageCodeIR::Korean,
            follow: "모든 작업에서 (완료 보고를 가진 작업) 또는 파일을 보여줘",
            report_folder: true,
            expected: &["폴더", "파일"],
            rejected: &["코드"],
            root: "UNION",
            kinds: &["INTERSECTION", "STATE_PREDICATE"],
            depth: 3,
            category: "heldout_predicate_composition",
        }),
        run(Case {
            id: "R37X_POR_KO_2",
            language: LanguageCodeIR::Korean,
            follow: "(파일 또는 코드) 중 완료 보고가 빠진 작업만 모든 작업에서 보여줘",
            report_folder: true,
            expected: &["파일", "코드"],
            rejected: &["폴더"],
            root: "INTERSECTION",
            kinds: &["UNION", "STATE_PREDICATE"],
            depth: 3,
            category: "heldout_predicate_composition",
        }),
        run(Case {
            id: "R37X_ORDER_EN_1",
            language: LanguageCodeIR::English,
            follow: "From all tasks show code or (all tasks with a completion report).",
            report_folder: true,
            expected: &["code", "folder"],
            rejected: &["file"],
            root: "UNION",
            kinds: &["INTERSECTION", "STATE_PREDICATE"],
            depth: 3,
            category: "heldout_surface_order",
        }),
        run(Case {
            id: "R37X_ORDER_EN_2",
            language: LanguageCodeIR::English,
            follow: "From all tasks show (not folder) or folder.",
            report_folder: false,
            expected: &["file", "folder", "code"],
            rejected: &[],
            root: "UNION",
            kinds: &["COMPLEMENT", "SOURCE_SET"],
            depth: 3,
            category: "heldout_surface_order",
        }),
        run(Case {
            id: "R37X_ORDER_KO_1",
            language: LanguageCodeIR::Korean,
            follow: "코드 또는 (완료 보고를 가진 모든 작업)을 보여줘",
            report_folder: true,
            expected: &["코드", "폴더"],
            rejected: &["파일"],
            root: "UNION",
            kinds: &["INTERSECTION", "STATE_PREDICATE"],
            depth: 3,
            category: "heldout_surface_order",
        }),
        run(Case {
            id: "R37X_ORDER_KO_2",
            language: LanguageCodeIR::Korean,
            follow: "모든 작업에서 (폴더가 아닌 것) 또는 폴더 상태를 알려줘",
            report_folder: false,
            expected: &["파일", "폴더", "코드"],
            rejected: &[],
            root: "UNION",
            kinds: &["COMPLEMENT", "SOURCE_SET"],
            depth: 3,
            category: "heldout_surface_order",
        }),
    ];
    let passed = rows.iter().filter(|row| row.pass).count();
    let report = json!({ "suite": "R37-TRANSFER-0001", "held_out_until_after_diagnostic_pass": true,
        "total": rows.len(), "passed": passed, "failed": rows.len() - passed,
        "external_llm_calls": 0, "local_teacher_calls": 0, "network_calls": 0,
        "recursive_source_mutations": 0, "rows": rows });
    println!("{}", serde_json::to_string_pretty(&report).expect("report"));
    if passed != report["total"].as_u64().unwrap_or_default() as usize {
        std::process::exit(1);
    }
}
