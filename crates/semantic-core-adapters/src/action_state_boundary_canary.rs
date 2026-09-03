//! Frozen R30 diagnostic suite.
//!
//! The suite observes only public API JSON. It is frozen before the typed
//! action-state product path exists and separates a plan, a language report,
//! and a verifier-bound host execution receipt.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR, LanguageCodeIR,
    CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const ACTION_EVIDENCE_SCHEMA: &str = "B_CORE_ACTION_EVIDENCE_REQUEST_1";

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    category: String,
    pass: bool,
    trace: Vec<String>,
}

#[derive(Clone, Copy)]
struct PlanCase {
    id: &'static str,
    text: &'static str,
    language: LanguageCodeIR,
    predicate: &'static str,
    subject: &'static str,
}

#[derive(Clone, Copy)]
struct ReportCase {
    id: &'static str,
    setup: &'static str,
    report: &'static str,
    language: LanguageCodeIR,
    expected_report: &'static str,
    category: &'static str,
}

#[derive(Clone, Copy)]
struct HostCase {
    id: &'static str,
    setup: &'static str,
    query: &'static str,
    language: LanguageCodeIR,
    terminal_status: &'static str,
    expected_execution: &'static str,
}

#[derive(Clone, Copy)]
enum RejectionKind {
    TextSpoof,
    InvalidHash,
    UnknownAction,
    TerminalWithoutStart,
}

#[derive(Clone, Copy)]
struct RejectionCase {
    id: &'static str,
    setup: &'static str,
    language: LanguageCodeIR,
    kind: RejectionKind,
}

#[derive(Clone, Copy)]
enum QueryPrior {
    PlanOnly,
    SuccessClaim,
    VerifiedRunning,
    VerifiedSuccess,
}

#[derive(Clone, Copy)]
struct QueryCase {
    id: &'static str,
    setup: &'static str,
    follow_up: &'static str,
    query: &'static str,
    language: LanguageCodeIR,
    prior: QueryPrior,
    expected_execution: &'static str,
    expected_report: Option<&'static str>,
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

fn response_json(response: &semantic_core_adapters::ConversationTurnResponseIR) -> Value {
    serde_json::to_value(response).expect("response json")
}

fn latest_record(value: &Value) -> Option<&Value> {
    value
        .pointer("/conversation_state/action_state_ledger/records")?
        .as_array()?
        .last()
}

fn action_id(value: &Value) -> Option<String> {
    latest_record(value)?["action_id"]
        .as_str()
        .map(str::to_string)
}

fn record_has(
    value: &Value,
    plan_status: &str,
    execution_status: &str,
    report_status: Option<&str>,
) -> bool {
    let Some(record) = latest_record(value) else {
        return false;
    };
    record["plan_status"] == plan_status
        && record["execution_status"] == execution_status
        && match report_status {
            Some(expected) => record["reported_status"] == expected,
            None => record.get("reported_status").is_none() || record["reported_status"].is_null(),
        }
}

fn analysis_safe(value: &Value) -> bool {
    value.pointer("/action_state_analysis/schema")
        == Some(&Value::String(
            "B_CORE_ACTION_STATE_ANALYSIS_IR_1".to_string(),
        ))
        && value.pointer("/action_state_analysis/semantic_authority") == Some(&Value::Bool(false))
        && value.pointer("/action_state_analysis/external_action_executed")
            == Some(&Value::Bool(false))
}

fn receipt_hash(
    receipt_id: &str,
    conversation_id: &str,
    action_id: &str,
    execution_id: &str,
    status: &str,
    evidence_digest: &str,
) -> String {
    let bytes = serde_json::to_vec(&(
        ACTION_EVIDENCE_SCHEMA,
        receipt_id,
        conversation_id,
        action_id,
        execution_id,
        status,
        evidence_digest,
    ))
    .expect("receipt hash payload");
    format!("{:x}", Sha256::digest(bytes))
}

fn submit_receipt(
    api: &mut CognitiveApi,
    conversation_id: &str,
    action_id: &str,
    suffix: &str,
    status: &str,
    valid_hash: bool,
) -> bool {
    let receipt_id = format!("{conversation_id}-RECEIPT-{suffix}");
    let execution_id = format!("{conversation_id}-EXECUTION-1");
    let evidence_digest = format!("{:064x}", suffix.len() + status.len());
    let verifier_receipt_sha256 = if valid_hash {
        receipt_hash(
            &receipt_id,
            conversation_id,
            action_id,
            &execution_id,
            status,
            &evidence_digest,
        )
    } else {
        "0".repeat(64)
    };
    let command = json!({
        "operation": "SUBMIT_ACTION_EVIDENCE",
        "request": {
            "schema": ACTION_EVIDENCE_SCHEMA,
            "receipt_id": receipt_id,
            "conversation_id": conversation_id,
            "action_id": action_id,
            "execution_id": execution_id,
            "status": status,
            "evidence_digest": evidence_digest,
            "verifier_receipt_sha256": verifier_receipt_sha256
        }
    });
    api.execute_command_json(&command.to_string())
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .is_some_and(|response| response["ok"] == true)
}

fn plan_case(case: PlanCase) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let response = api
        .process_conversation_turn(&request(case.id, 1, case.text, case.language))
        .expect("plan turn");
    let value = response_json(&response);
    let record = latest_record(&value);
    Row {
        id: case.id.to_string(),
        category: "plan_is_not_result".to_string(),
        pass: record_has(&value, "ACTIVE", "NOT_OBSERVED", None)
            && record.is_some_and(|record| {
                record["canonical_predicate"] == case.predicate
                    && record["subject"]
                        .as_str()
                        .is_some_and(|subject| subject.to_lowercase().contains(case.subject))
                    && record["verified_outcome"] == false
                    && record["execution_evidence_ids"]
                        .as_array()
                        .is_some_and(Vec::is_empty)
            })
            && analysis_safe(&value)
            && response.output.unsupported_freeform_claims == 0,
        trace: vec![value.to_string(), response.output.text],
    }
}

fn report_case(case: ReportCase) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let first = api
        .process_conversation_turn(&request(case.id, 1, case.setup, case.language))
        .expect("report setup");
    let before = response_json(&first);
    let before_action = action_id(&before);
    let response = api
        .process_conversation_turn(&request(case.id, 2, case.report, case.language))
        .expect("report turn");
    let value = response_json(&response);
    let after_action = action_id(&value);
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        pass: before_action.is_some()
            && before_action == after_action
            && record_has(&value, "ACTIVE", "NOT_OBSERVED", Some(case.expected_report))
            && value.pointer("/action_state_analysis/detected_report/reported_status")
                == Some(&Value::String(case.expected_report.to_string()))
            && latest_record(&value).is_some_and(|record| {
                record["verified_outcome"] == false
                    && record["execution_evidence_ids"]
                        .as_array()
                        .is_some_and(Vec::is_empty)
            })
            && analysis_safe(&value)
            && response.grounded_response.is_none()
            && response.output.unsupported_freeform_claims == 0,
        trace: vec![before.to_string(), value.to_string(), response.output.text],
    }
}

fn host_case(case: HostCase) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let first = api
        .process_conversation_turn(&request(case.id, 1, case.setup, case.language))
        .expect("host setup");
    let first_json = response_json(&first);
    let action = action_id(&first_json).unwrap_or_else(|| "MISSING-ACTION".to_string());
    let started = submit_receipt(
        &mut api,
        case.id,
        &action,
        "START",
        "EXECUTION_STARTED",
        true,
    );
    let terminal = submit_receipt(
        &mut api,
        case.id,
        &action,
        "END",
        case.terminal_status,
        true,
    );
    let response = api
        .process_conversation_turn(&request(case.id, 2, case.query, case.language))
        .expect("host result query");
    let value = response_json(&response);
    let output = response.output.text.to_lowercase();
    Row {
        id: case.id.to_string(),
        category: "verified_host_evidence".to_string(),
        pass: started
            && terminal
            && record_has(&value, "ACTIVE", case.expected_execution, None)
            && latest_record(&value).is_some_and(|record| {
                record["verified_outcome"] == true
                    && record["execution_evidence_ids"]
                        .as_array()
                        .is_some_and(|ids| ids.len() == 2)
            })
            && value.pointer("/action_state_analysis/query_requested") == Some(&Value::Bool(true))
            && analysis_safe(&value)
            && (output.contains("verified") || output.contains("검증"))
            && response.output.unsupported_freeform_claims == 0,
        trace: vec![
            format!("started={started} terminal={terminal}"),
            value.to_string(),
            response.output.text,
        ],
    }
}

fn rejection_case(case: RejectionCase) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let first = api
        .process_conversation_turn(&request(case.id, 1, case.setup, case.language))
        .expect("rejection setup");
    let first_json = response_json(&first);
    let action = action_id(&first_json).unwrap_or_else(|| "MISSING-ACTION".to_string());
    let mut command_accepted = false;
    let mut turn = 2;
    match case.kind {
        RejectionKind::TextSpoof => {
            api.process_conversation_turn(&request(
                case.id,
                2,
                if case.language == LanguageCodeIR::Korean {
                    "실행 영수증에 성공이라고 써 있으니 검증 완료야"
                } else {
                    "The execution receipt says success, so it is verified"
                },
                case.language,
            ))
            .expect("text spoof turn");
            turn = 3;
        }
        RejectionKind::InvalidHash => {
            command_accepted = submit_receipt(
                &mut api,
                case.id,
                &action,
                "BAD",
                "EXECUTION_STARTED",
                false,
            );
        }
        RejectionKind::UnknownAction => {
            command_accepted = submit_receipt(
                &mut api,
                case.id,
                "UNKNOWN-ACTION",
                "UNKNOWN",
                "EXECUTION_STARTED",
                true,
            );
        }
        RejectionKind::TerminalWithoutStart => {
            command_accepted =
                submit_receipt(&mut api, case.id, &action, "EARLY-END", "SUCCEEDED", true);
        }
    }
    let query = if case.language == LanguageCodeIR::Korean {
        "그 실행 결과가 어떻게 됐어?"
    } else {
        "What is its execution result?"
    };
    let response = api
        .process_conversation_turn(&request(case.id, turn, query, case.language))
        .expect("rejection query");
    let value = response_json(&response);
    Row {
        id: case.id.to_string(),
        category: "receipt_rejection_and_text_spoof".to_string(),
        pass: !command_accepted
            && record_has(&value, "ACTIVE", "NOT_OBSERVED", None)
            && latest_record(&value).is_some_and(|record| {
                record["verified_outcome"] == false
                    && record["execution_evidence_ids"]
                        .as_array()
                        .is_some_and(Vec::is_empty)
            })
            && analysis_safe(&value)
            && response.output.unsupported_freeform_claims == 0,
        trace: vec![
            format!("command_accepted={command_accepted}"),
            value.to_string(),
            response.output.text,
        ],
    }
}

fn query_case(case: QueryCase) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let first = api
        .process_conversation_turn(&request(case.id, 1, case.setup, case.language))
        .expect("query setup");
    let first_json = response_json(&first);
    let action = action_id(&first_json).unwrap_or_else(|| "MISSING-ACTION".to_string());
    let mut next_turn = 2;
    match case.prior {
        QueryPrior::PlanOnly => {}
        QueryPrior::SuccessClaim => {
            api.process_conversation_turn(&request(case.id, 2, case.follow_up, case.language))
                .expect("claim turn");
            next_turn = 3;
        }
        QueryPrior::VerifiedRunning => {
            submit_receipt(
                &mut api,
                case.id,
                &action,
                "START",
                "EXECUTION_STARTED",
                true,
            );
        }
        QueryPrior::VerifiedSuccess => {
            submit_receipt(
                &mut api,
                case.id,
                &action,
                "START",
                "EXECUTION_STARTED",
                true,
            );
            submit_receipt(&mut api, case.id, &action, "END", "SUCCEEDED", true);
        }
    }
    let response = api
        .process_conversation_turn(&request(case.id, next_turn, case.query, case.language))
        .expect("state query");
    let value = response_json(&response);
    let output = response.output.text.to_lowercase();
    Row {
        id: case.id.to_string(),
        category: "typed_status_query".to_string(),
        pass: record_has(
            &value,
            "ACTIVE",
            case.expected_execution,
            case.expected_report,
        ) && value.pointer("/action_state_analysis/query_requested")
            == Some(&Value::Bool(true))
            && analysis_safe(&value)
            && (output.contains("plan")
                || output.contains("계획")
                || output.contains("reported")
                || output.contains("보고")
                || output.contains("verified")
                || output.contains("검증"))
            && response.output.unsupported_freeform_claims == 0,
        trace: vec![value.to_string(), response.output.text],
    }
}

fn main() {
    let mut rows = vec![
        plan_case(PlanCase {
            id: "R30_PLAN_1",
            text: "캐시를 수리해",
            language: LanguageCodeIR::Korean,
            predicate: "REPAIR",
            subject: "캐시",
        }),
        plan_case(PlanCase {
            id: "R30_PLAN_2",
            text: "로그를 분석해",
            language: LanguageCodeIR::Korean,
            predicate: "INVESTIGATE",
            subject: "로그",
        }),
        plan_case(PlanCase {
            id: "R30_PLAN_3",
            text: "repair the worker",
            language: LanguageCodeIR::English,
            predicate: "REPAIR",
            subject: "worker",
        }),
        plan_case(PlanCase {
            id: "R30_PLAN_4",
            text: "inspect the archive",
            language: LanguageCodeIR::English,
            predicate: "INVESTIGATE",
            subject: "archive",
        }),
    ];
    rows.extend([
        report_case(ReportCase {
            id: "R30_ATTEMPT_1",
            setup: "파서를 수리해",
            report: "그 작업을 실행해 보긴 했어",
            language: LanguageCodeIR::Korean,
            expected_report: "ATTEMPTED",
            category: "language_attempt_report",
        }),
        report_case(ReportCase {
            id: "R30_ATTEMPT_2",
            setup: "큐를 확인해",
            report: "일단 시도는 해봤어",
            language: LanguageCodeIR::Korean,
            expected_report: "ATTEMPTED",
            category: "language_attempt_report",
        }),
        report_case(ReportCase {
            id: "R30_ATTEMPT_3",
            setup: "repair the parser",
            report: "I tried running it",
            language: LanguageCodeIR::English,
            expected_report: "ATTEMPTED",
            category: "language_attempt_report",
        }),
        report_case(ReportCase {
            id: "R30_ATTEMPT_4",
            setup: "inspect the queue",
            report: "I made an attempt",
            language: LanguageCodeIR::English,
            expected_report: "ATTEMPTED",
            category: "language_attempt_report",
        }),
        report_case(ReportCase {
            id: "R30_RUNNING_1",
            setup: "워커를 수리해",
            report: "지금 그 작업을 실행 중이야",
            language: LanguageCodeIR::Korean,
            expected_report: "IN_PROGRESS_CLAIMED",
            category: "language_in_progress_report",
        }),
        report_case(ReportCase {
            id: "R30_RUNNING_2",
            setup: "아카이브를 검사해",
            report: "아직 처리하고 있어",
            language: LanguageCodeIR::Korean,
            expected_report: "IN_PROGRESS_CLAIMED",
            category: "language_in_progress_report",
        }),
        report_case(ReportCase {
            id: "R30_RUNNING_3",
            setup: "repair the worker",
            report: "It is running now",
            language: LanguageCodeIR::English,
            expected_report: "IN_PROGRESS_CLAIMED",
            category: "language_in_progress_report",
        }),
        report_case(ReportCase {
            id: "R30_RUNNING_4",
            setup: "inspect the archive",
            report: "I am still working on it",
            language: LanguageCodeIR::English,
            expected_report: "IN_PROGRESS_CLAIMED",
            category: "language_in_progress_report",
        }),
        report_case(ReportCase {
            id: "R30_SUCCESS_1",
            setup: "캐시를 수리해",
            report: "그 작업은 끝났어",
            language: LanguageCodeIR::Korean,
            expected_report: "SUCCESS_CLAIMED",
            category: "language_success_claim",
        }),
        report_case(ReportCase {
            id: "R30_SUCCESS_2",
            setup: "로그를 분석해",
            report: "분석이 성공했어",
            language: LanguageCodeIR::Korean,
            expected_report: "SUCCESS_CLAIMED",
            category: "language_success_claim",
        }),
        report_case(ReportCase {
            id: "R30_SUCCESS_3",
            setup: "repair the cache",
            report: "I completed it",
            language: LanguageCodeIR::English,
            expected_report: "SUCCESS_CLAIMED",
            category: "language_success_claim",
        }),
        report_case(ReportCase {
            id: "R30_SUCCESS_4",
            setup: "inspect the log",
            report: "The inspection succeeded",
            language: LanguageCodeIR::English,
            expected_report: "SUCCESS_CLAIMED",
            category: "language_success_claim",
        }),
        report_case(ReportCase {
            id: "R30_FAILURE_1",
            setup: "파서를 수리해",
            report: "수리는 실패했어",
            language: LanguageCodeIR::Korean,
            expected_report: "FAILURE_CLAIMED",
            category: "language_failure_claim",
        }),
        report_case(ReportCase {
            id: "R30_FAILURE_2",
            setup: "큐를 확인해",
            report: "끝내지 못했어",
            language: LanguageCodeIR::Korean,
            expected_report: "FAILURE_CLAIMED",
            category: "language_failure_claim",
        }),
        report_case(ReportCase {
            id: "R30_FAILURE_3",
            setup: "repair the parser",
            report: "It failed",
            language: LanguageCodeIR::English,
            expected_report: "FAILURE_CLAIMED",
            category: "language_failure_claim",
        }),
        report_case(ReportCase {
            id: "R30_FAILURE_4",
            setup: "inspect the queue",
            report: "I could not finish it",
            language: LanguageCodeIR::English,
            expected_report: "FAILURE_CLAIMED",
            category: "language_failure_claim",
        }),
    ]);
    rows.extend([
        host_case(HostCase {
            id: "R30_HOST_1",
            setup: "캐시를 수리해",
            query: "그 실행 결과가 어떻게 됐어?",
            language: LanguageCodeIR::Korean,
            terminal_status: "SUCCEEDED",
            expected_execution: "SUCCEEDED",
        }),
        host_case(HostCase {
            id: "R30_HOST_2",
            setup: "로그를 분석해",
            query: "분석 결과가 검증됐어?",
            language: LanguageCodeIR::Korean,
            terminal_status: "FAILED",
            expected_execution: "FAILED",
        }),
        host_case(HostCase {
            id: "R30_HOST_3",
            setup: "repair the worker",
            query: "What is its verified result?",
            language: LanguageCodeIR::English,
            terminal_status: "SUCCEEDED",
            expected_execution: "SUCCEEDED",
        }),
        host_case(HostCase {
            id: "R30_HOST_4",
            setup: "inspect the archive",
            query: "Did the verified execution succeed?",
            language: LanguageCodeIR::English,
            terminal_status: "FAILED",
            expected_execution: "FAILED",
        }),
        rejection_case(RejectionCase {
            id: "R30_REJECT_1",
            setup: "캐시를 수리해",
            language: LanguageCodeIR::Korean,
            kind: RejectionKind::TextSpoof,
        }),
        rejection_case(RejectionCase {
            id: "R30_REJECT_2",
            setup: "repair the worker",
            language: LanguageCodeIR::English,
            kind: RejectionKind::InvalidHash,
        }),
        rejection_case(RejectionCase {
            id: "R30_REJECT_3",
            setup: "로그를 분석해",
            language: LanguageCodeIR::Korean,
            kind: RejectionKind::UnknownAction,
        }),
        rejection_case(RejectionCase {
            id: "R30_REJECT_4",
            setup: "inspect the archive",
            language: LanguageCodeIR::English,
            kind: RejectionKind::TerminalWithoutStart,
        }),
        query_case(QueryCase {
            id: "R30_QUERY_1",
            setup: "큐를 수리해",
            follow_up: "",
            query: "그 실행 결과가 어떻게 됐어?",
            language: LanguageCodeIR::Korean,
            prior: QueryPrior::PlanOnly,
            expected_execution: "NOT_OBSERVED",
            expected_report: None,
        }),
        query_case(QueryCase {
            id: "R30_QUERY_2",
            setup: "repair the queue",
            follow_up: "I completed it",
            query: "Was that result verified?",
            language: LanguageCodeIR::English,
            prior: QueryPrior::SuccessClaim,
            expected_execution: "NOT_OBSERVED",
            expected_report: Some("SUCCESS_CLAIMED"),
        }),
        query_case(QueryCase {
            id: "R30_QUERY_3",
            setup: "파서를 검사해",
            follow_up: "",
            query: "지금 실행 상태가 뭐야?",
            language: LanguageCodeIR::Korean,
            prior: QueryPrior::VerifiedRunning,
            expected_execution: "IN_PROGRESS",
            expected_report: None,
        }),
        query_case(QueryCase {
            id: "R30_QUERY_4",
            setup: "repair the archive",
            follow_up: "",
            query: "What is the execution status?",
            language: LanguageCodeIR::English,
            prior: QueryPrior::VerifiedSuccess,
            expected_execution: "SUCCEEDED",
            expected_report: None,
        }),
    ]);
    let passed = rows.iter().filter(|row| row.pass).count();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "suite": "R30-RUN-0001",
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
