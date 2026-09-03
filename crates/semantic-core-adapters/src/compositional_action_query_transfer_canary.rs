//! Frozen R36-TRANSFER-0001 held-out transfer suite.
//!
//! Do not inspect or change case expectations in response to first exposure.

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
    operator: &'static str,
    quantifier: Option<&'static str>,
    predicate: Option<&'static str>,
    truth: Option<&'static str>,
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

fn run(case: Case) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
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
        .expect("transfer query");
    let value = serde_json::to_value(&response).expect("json");
    let query = &value["action_state_analysis"]["set_query"];
    let subjects = selected_subjects(&value);
    let terms = case.expected.iter().all(|term| {
        subjects
            .iter()
            .any(|subject| subject.contains(&term.to_lowercase()))
    }) && case.rejected.iter().all(|term| {
        subjects
            .iter()
            .all(|subject| !subject.contains(&term.to_lowercase()))
    });
    let evaluation_claim = case.predicate.is_none()
        || value
            .pointer("/grounded_realization/claims")
            .and_then(Value::as_array)
            .is_some_and(|claims| {
                claims
                    .iter()
                    .any(|claim| claim["kind"] == "ACTION_SET_EVALUATION")
            });
    let pass = response.disposition == ConversationTurnDispositionIR::Grounded
        && query["schema"] == "B_CORE_ACTION_SET_QUERY_IR_1"
        && query["source_action_ids"]
            .as_array()
            .is_some_and(|ids| ids.len() == 3)
        && query["selected_action_ids"]
            .as_array()
            .is_some_and(|ids| ids.len() == case.expected.len())
        && query["operator_trace"]
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item == case.operator))
        && case
            .quantifier
            .is_none_or(|item| query["quantifier"] == item)
        && case.predicate.is_none_or(|item| query["predicate"] == item)
        && case.truth.is_none_or(|item| query["truth"] == item)
        && terms
        && evaluation_claim
        && value
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
        pass,
        trace: vec![format!("subjects={subjects:?}"), value.to_string()],
    }
}

fn main() {
    let rows = vec![
        run(Case {
            id: "R36X_ONLY_EN_1",
            language: LanguageCodeIR::English,
            follow: "Out of all tasks, give me only the file action's status.",
            report_folder: false,
            expected: &["file"],
            rejected: &["folder", "code"],
            operator: "INTERSECTION",
            quantifier: None,
            predicate: None,
            truth: None,
            category: "heldout_intersection_difference",
        }),
        run(Case {
            id: "R36X_ONLY_EN_2",
            language: LanguageCodeIR::English,
            follow: "Give all tasks' status apart from the code action.",
            report_folder: false,
            expected: &["file", "folder"],
            rejected: &["code"],
            operator: "DIFFERENCE",
            quantifier: None,
            predicate: None,
            truth: None,
            category: "heldout_intersection_difference",
        }),
        run(Case {
            id: "R36X_ONLY_KO_1",
            language: LanguageCodeIR::Korean,
            follow: "모든 작업 가운데 파일 작업만 현황을 말해줘",
            report_folder: false,
            expected: &["파일"],
            rejected: &["폴더", "코드"],
            operator: "INTERSECTION",
            quantifier: None,
            predicate: None,
            truth: None,
            category: "heldout_intersection_difference",
        }),
        run(Case {
            id: "R36X_ONLY_KO_2",
            language: LanguageCodeIR::Korean,
            follow: "모든 작업에서 코드 작업 말고 나머지 상태를 알려줘",
            report_folder: false,
            expected: &["파일", "폴더"],
            rejected: &["코드"],
            operator: "DIFFERENCE",
            quantifier: None,
            predicate: None,
            truth: None,
            category: "heldout_intersection_difference",
        }),
        run(Case {
            id: "R36X_SET_EN_1",
            language: LanguageCodeIR::English,
            follow: "From all tasks list just the file or code action states.",
            report_folder: false,
            expected: &["file", "code"],
            rejected: &["folder"],
            operator: "UNION",
            quantifier: None,
            predicate: None,
            truth: None,
            category: "heldout_union_complement",
        }),
        run(Case {
            id: "R36X_SET_EN_2",
            language: LanguageCodeIR::English,
            follow: "For all tasks leave out both file and folder, then show the remaining status.",
            report_folder: false,
            expected: &["code"],
            rejected: &["file", "folder"],
            operator: "COMPLEMENT",
            quantifier: None,
            predicate: None,
            truth: None,
            category: "heldout_union_complement",
        }),
        run(Case {
            id: "R36X_SET_KO_1",
            language: LanguageCodeIR::Korean,
            follow: "모든 작업에서 파일이나 코드 작업만 골라 상태를 보여줘",
            report_folder: false,
            expected: &["파일", "코드"],
            rejected: &["폴더"],
            operator: "UNION",
            quantifier: None,
            predicate: None,
            truth: None,
            category: "heldout_union_complement",
        }),
        run(Case {
            id: "R36X_SET_KO_2",
            language: LanguageCodeIR::Korean,
            follow: "파일과 폴더는 모두 제외하고 모든 작업 중 남은 현황을 알려줘",
            report_folder: false,
            expected: &["코드"],
            rejected: &["파일", "폴더"],
            operator: "COMPLEMENT",
            quantifier: None,
            predicate: None,
            truth: None,
            category: "heldout_union_complement",
        }),
        run(Case {
            id: "R36X_ALL_EN_1",
            language: LanguageCodeIR::English,
            follow: "Does every one of all tasks remain without verified execution evidence?",
            report_folder: false,
            expected: &["file", "folder", "code"],
            rejected: &[],
            operator: "IDENTITY",
            quantifier: Some("ALL"),
            predicate: Some("UNVERIFIED_EXECUTION"),
            truth: Some("TRUE"),
            category: "heldout_universal",
        }),
        run(Case {
            id: "R36X_ALL_EN_2",
            language: LanguageCodeIR::English,
            follow: "Are all tasks unverified plans still?",
            report_folder: false,
            expected: &["file", "folder", "code"],
            rejected: &[],
            operator: "IDENTITY",
            quantifier: Some("ALL"),
            predicate: Some("UNVERIFIED_EXECUTION"),
            truth: Some("TRUE"),
            category: "heldout_universal",
        }),
        run(Case {
            id: "R36X_ALL_KO_1",
            language: LanguageCodeIR::Korean,
            follow: "모든 작업이 여전히 실행 검증 증거가 없는 상태야?",
            report_folder: false,
            expected: &["파일", "폴더", "코드"],
            rejected: &[],
            operator: "IDENTITY",
            quantifier: Some("ALL"),
            predicate: Some("UNVERIFIED_EXECUTION"),
            truth: Some("TRUE"),
            category: "heldout_universal",
        }),
        run(Case {
            id: "R36X_ALL_KO_2",
            language: LanguageCodeIR::Korean,
            follow: "작업 전부 아직 미검증 계획이야?",
            report_folder: false,
            expected: &["파일", "폴더", "코드"],
            rejected: &[],
            operator: "IDENTITY",
            quantifier: Some("ALL"),
            predicate: Some("UNVERIFIED_EXECUTION"),
            truth: Some("TRUE"),
            category: "heldout_universal",
        }),
        run(Case {
            id: "R36X_ANY_EN_1",
            language: LanguageCodeIR::English,
            follow: "Is there any task among all tasks with a reported completion?",
            report_folder: true,
            expected: &["file", "folder", "code"],
            rejected: &[],
            operator: "IDENTITY",
            quantifier: Some("ANY"),
            predicate: Some("REPORTED_COMPLETION"),
            truth: Some("TRUE"),
            category: "heldout_existential_report",
        }),
        run(Case {
            id: "R36X_ANY_EN_2",
            language: LanguageCodeIR::English,
            follow: "Was at least one of all tasks reported as finished?",
            report_folder: true,
            expected: &["file", "folder", "code"],
            rejected: &[],
            operator: "IDENTITY",
            quantifier: Some("ANY"),
            predicate: Some("REPORTED_COMPLETION"),
            truth: Some("TRUE"),
            category: "heldout_existential_report",
        }),
        run(Case {
            id: "R36X_ANY_KO_1",
            language: LanguageCodeIR::Korean,
            follow: "모든 작업 가운데 완료 보고가 있는 게 하나라도 있어?",
            report_folder: true,
            expected: &["파일", "폴더", "코드"],
            rejected: &[],
            operator: "IDENTITY",
            quantifier: Some("ANY"),
            predicate: Some("REPORTED_COMPLETION"),
            truth: Some("TRUE"),
            category: "heldout_existential_report",
        }),
        run(Case {
            id: "R36X_ANY_KO_2",
            language: LanguageCodeIR::Korean,
            follow: "작업 전부 중 최소 하나가 끝났다고 보고됐어?",
            report_folder: true,
            expected: &["파일", "폴더", "코드"],
            rejected: &[],
            operator: "IDENTITY",
            quantifier: Some("ANY"),
            predicate: Some("REPORTED_COMPLETION"),
            truth: Some("TRUE"),
            category: "heldout_existential_report",
        }),
        run(Case {
            id: "R36X_ORDER_EN_1",
            language: LanguageCodeIR::English,
            follow: "Only the code or file actions: show their status from all tasks.",
            report_folder: false,
            expected: &["code", "file"],
            rejected: &["folder"],
            operator: "UNION",
            quantifier: None,
            predicate: None,
            truth: None,
            category: "heldout_surface_order",
        }),
        run(Case {
            id: "R36X_ORDER_EN_2",
            language: LanguageCodeIR::English,
            follow: "Exclude the folder action from all tasks and tell me the remaining states.",
            report_folder: false,
            expected: &["file", "code"],
            rejected: &["folder"],
            operator: "DIFFERENCE",
            quantifier: None,
            predicate: None,
            truth: None,
            category: "heldout_surface_order",
        }),
        run(Case {
            id: "R36X_ORDER_KO_1",
            language: LanguageCodeIR::Korean,
            follow: "코드 아니면 파일 작업만, 모든 작업에서 상태를 보여줘",
            report_folder: false,
            expected: &["코드", "파일"],
            rejected: &["폴더"],
            operator: "UNION",
            quantifier: None,
            predicate: None,
            truth: None,
            category: "heldout_surface_order",
        }),
        run(Case {
            id: "R36X_ORDER_KO_2",
            language: LanguageCodeIR::Korean,
            follow: "폴더 작업을 빼고 모든 작업의 나머지 현황을 말해줘",
            report_folder: false,
            expected: &["파일", "코드"],
            rejected: &["폴더"],
            operator: "DIFFERENCE",
            quantifier: None,
            predicate: None,
            truth: None,
            category: "heldout_surface_order",
        }),
    ];
    let passed = rows.iter().filter(|row| row.pass).count();
    let report = json!({ "suite": "R36-TRANSFER-0001", "held_out_until_after_diagnostic_pass": true,
        "total": rows.len(), "passed": passed, "failed": rows.len() - passed,
        "external_llm_calls": 0, "local_teacher_calls": 0, "network_calls": 0,
        "recursive_source_mutations": 0, "rows": rows });
    println!("{}", serde_json::to_string_pretty(&report).expect("report"));
    if passed != report["total"].as_u64().unwrap_or_default() as usize {
        std::process::exit(1);
    }
}
