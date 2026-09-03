//! Frozen R42 diagnostic suite.
//!
//! This suite is deliberately JSON-only so it can be frozen before the R42
//! product types exist. It requires one tamper-evident chain from language
//! input through GoalIR/plan, reports or trusted host evidence, and the final
//! grounded claims.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR, LanguageCodeIR,
    CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const ACTION_EVIDENCE_SCHEMA: &str = "B_CORE_ACTION_EVIDENCE_REQUEST_1";
const PROVENANCE_SCHEMA: &str = "B_CORE_INTERACTION_PROVENANCE_GRAPH_IR_1";

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    category: String,
    pass: bool,
    trace: Vec<String>,
}

#[derive(Clone, Copy)]
enum Scenario {
    Plan,
    Report,
    Observation,
    TerminalSuccess,
    ReportRevision,
    TextSpoof,
    RejectedEvidence(RejectionKind),
}

#[derive(Clone, Copy)]
enum RejectionKind {
    InvalidHash,
    UnknownAction,
    TerminalWithoutStart,
    MismatchedExecution,
}

#[derive(Clone, Copy)]
struct Case {
    id: &'static str,
    category: &'static str,
    setup: &'static str,
    follow_up: &'static str,
    correction: &'static str,
    query: &'static str,
    language: LanguageCodeIR,
    scenario: Scenario,
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

fn action_id(value: &Value) -> Option<String> {
    value
        .pointer("/conversation_state/action_state_ledger/records")?
        .as_array()?
        .last()?
        .get("action_id")?
        .as_str()
        .map(str::to_string)
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
    .expect("receipt payload");
    format!("{:x}", Sha256::digest(bytes))
}

fn submit_receipt(
    api: &mut CognitiveApi,
    conversation_id: &str,
    action_id: &str,
    receipt_suffix: &str,
    execution_suffix: &str,
    status: &str,
    valid_hash: bool,
) -> bool {
    let receipt_id = format!("{conversation_id}-RECEIPT-{receipt_suffix}");
    let execution_id = format!("{conversation_id}-EXECUTION-{execution_suffix}");
    let evidence_digest = format!("{:064x}", receipt_suffix.len() + status.len());
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

fn graph(value: &Value) -> Option<&Value> {
    value.get("interaction_provenance")
}

fn nodes(value: &Value) -> &[Value] {
    graph(value)
        .and_then(|graph| graph.get("nodes"))
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn edges(value: &Value) -> &[Value] {
    graph(value)
        .and_then(|graph| graph.get("edges"))
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn has_node(value: &Value, kind: &str) -> bool {
    nodes(value).iter().any(|node| node["kind"] == kind)
}

fn node_count(value: &Value, kind: &str) -> usize {
    nodes(value)
        .iter()
        .filter(|node| node["kind"] == kind)
        .count()
}

fn has_edge(value: &Value, relation: &str) -> bool {
    edges(value).iter().any(|edge| edge["relation"] == relation)
}

fn graph_safe(value: &Value) -> bool {
    let Some(graph) = graph(value) else {
        return false;
    };
    graph["schema"] == PROVENANCE_SCHEMA
        && graph["semantic_authority"] == false
        && graph["language_can_advance_execution"] == false
        && graph["external_action_executed"] == false
        && graph["graph_sha256"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
        && !nodes(value).is_empty()
        && nodes(value).iter().all(|node| {
            node["semantic_authority"] == false
                && node["node_sha256"]
                    .as_str()
                    .is_some_and(|digest| digest.len() == 64)
        })
        && edges(value).iter().all(|edge| {
            edge["edge_sha256"]
                .as_str()
                .is_some_and(|digest| digest.len() == 64)
        })
}

fn claim_sources_safe(value: &Value) -> bool {
    let claim_nodes = nodes(value)
        .iter()
        .filter(|node| node["kind"] == "REALIZED_CLAIM")
        .collect::<Vec<_>>();
    if claim_nodes.is_empty() {
        return false;
    }
    claim_nodes.iter().all(|claim| {
        let Some(claim_id) = claim["node_id"].as_str() else {
            return false;
        };
        let sources = edges(value)
            .iter()
            .filter(|edge| {
                edge["relation"] == "SOURCE_GROUNDS_CLAIM" && edge["target_node_id"] == claim_id
            })
            .filter_map(|edge| edge["source_node_id"].as_str())
            .filter_map(|source_id| {
                nodes(value)
                    .iter()
                    .find(|node| node["node_id"] == source_id)
            })
            .collect::<Vec<_>>();
        !sources.is_empty()
            && (!claim["verified"].as_bool().unwrap_or(false)
                || sources.iter().all(|source| {
                    matches!(
                        source["kind"].as_str(),
                        Some("EXECUTION_OBSERVATION" | "VERIFIED_RESULT")
                    )
                }))
    })
}

fn inspect(case: Case, value: &Value, command_acceptance: &[bool]) -> bool {
    if !graph_safe(value) || !claim_sources_safe(value) {
        return false;
    }
    match case.scenario {
        Scenario::Plan => {
            has_node(value, "LANGUAGE_INPUT")
                && has_node(value, "SEMANTIC_GOAL")
                && has_node(value, "PLANNED_ACTION")
                && has_edge(value, "INPUT_GROUNDS_GOAL")
                && has_edge(value, "GOAL_PROJECTS_PLAN")
                && !has_node(value, "EXECUTION_OBSERVATION")
                && !has_node(value, "VERIFIED_RESULT")
        }
        Scenario::Report => {
            has_node(value, "LANGUAGE_REPORT")
                && has_edge(value, "REPORT_DESCRIBES_PLAN")
                && !has_node(value, "EXECUTION_OBSERVATION")
                && !has_node(value, "VERIFIED_RESULT")
        }
        Scenario::Observation => {
            command_acceptance == [true]
                && has_node(value, "EXECUTION_OBSERVATION")
                && has_edge(value, "OBSERVATION_STARTS_EXECUTION")
                && !has_node(value, "VERIFICATION_RECEIPT")
                && !has_node(value, "VERIFIED_RESULT")
        }
        Scenario::TerminalSuccess => {
            command_acceptance == [true, true]
                && has_node(value, "EXECUTION_OBSERVATION")
                && has_node(value, "VERIFICATION_RECEIPT")
                && has_node(value, "VERIFIED_RESULT")
                && has_edge(value, "VERIFICATION_VERIFIES_OBSERVATION")
                && has_edge(value, "VERIFICATION_ESTABLISHES_RESULT")
        }
        Scenario::ReportRevision => {
            node_count(value, "LANGUAGE_REPORT") >= 2
                && has_edge(value, "SUPERSEDES_REPORT")
                && !has_node(value, "VERIFIED_RESULT")
        }
        Scenario::TextSpoof => {
            !has_node(value, "EXECUTION_OBSERVATION")
                && !has_node(value, "VERIFICATION_RECEIPT")
                && !has_node(value, "VERIFIED_RESULT")
        }
        Scenario::RejectedEvidence(_) => {
            command_acceptance.iter().any(|accepted| !accepted)
                && !has_node(value, "VERIFIED_RESULT")
                && match case.scenario {
                    Scenario::RejectedEvidence(RejectionKind::MismatchedExecution) => {
                        has_node(value, "EXECUTION_OBSERVATION")
                            && !has_node(value, "VERIFICATION_RECEIPT")
                    }
                    _ => !has_node(value, "VERIFICATION_RECEIPT"),
                }
        }
    }
}

fn run(case: Case) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let first = api
        .process_conversation_turn(&request(case.id, 1, case.setup, case.language))
        .expect("setup turn");
    let first_value = serde_json::to_value(&first).expect("setup json");
    let action = action_id(&first_value).unwrap_or_else(|| "UNKNOWN-ACTION".to_string());
    let mut turn = 1;
    let mut command_acceptance = Vec::new();

    match case.scenario {
        Scenario::Plan => {}
        Scenario::Report | Scenario::TextSpoof => {
            turn += 1;
            api.process_conversation_turn(&request(case.id, turn, case.follow_up, case.language))
                .expect("report turn");
        }
        Scenario::Observation => {
            command_acceptance.push(submit_receipt(
                &mut api,
                case.id,
                &action,
                "START",
                "1",
                "EXECUTION_STARTED",
                true,
            ));
        }
        Scenario::TerminalSuccess => {
            command_acceptance.push(submit_receipt(
                &mut api,
                case.id,
                &action,
                "START",
                "1",
                "EXECUTION_STARTED",
                true,
            ));
            command_acceptance.push(submit_receipt(
                &mut api,
                case.id,
                &action,
                "DONE",
                "1",
                "SUCCEEDED",
                true,
            ));
        }
        Scenario::ReportRevision => {
            turn += 1;
            api.process_conversation_turn(&request(case.id, turn, case.follow_up, case.language))
                .expect("first report");
            turn += 1;
            api.process_conversation_turn(&request(case.id, turn, case.correction, case.language))
                .expect("corrected report");
        }
        Scenario::RejectedEvidence(kind) => match kind {
            RejectionKind::InvalidHash => command_acceptance.push(submit_receipt(
                &mut api,
                case.id,
                &action,
                "BAD",
                "1",
                "EXECUTION_STARTED",
                false,
            )),
            RejectionKind::UnknownAction => command_acceptance.push(submit_receipt(
                &mut api,
                case.id,
                "UNKNOWN-ACTION",
                "UNKNOWN",
                "1",
                "EXECUTION_STARTED",
                true,
            )),
            RejectionKind::TerminalWithoutStart => command_acceptance.push(submit_receipt(
                &mut api,
                case.id,
                &action,
                "EARLY",
                "1",
                "SUCCEEDED",
                true,
            )),
            RejectionKind::MismatchedExecution => {
                command_acceptance.push(submit_receipt(
                    &mut api,
                    case.id,
                    &action,
                    "START",
                    "1",
                    "EXECUTION_STARTED",
                    true,
                ));
                command_acceptance.push(submit_receipt(
                    &mut api,
                    case.id,
                    &action,
                    "DONE",
                    "2",
                    "SUCCEEDED",
                    true,
                ));
            }
        },
    }

    let response = if matches!(case.scenario, Scenario::Plan) {
        first
    } else if matches!(
        case.scenario,
        Scenario::Report | Scenario::TextSpoof | Scenario::ReportRevision
    ) {
        let state_turn = match case.scenario {
            Scenario::Report | Scenario::TextSpoof => 2,
            Scenario::ReportRevision => 3,
            _ => unreachable!(),
        };
        // Re-run is impossible because turn order is sealed. The last response
        // is obtained with a neutral status query so the complete ledger is
        // observed on a new turn without adding execution authority.
        turn = state_turn + 1;
        api.process_conversation_turn(&request(case.id, turn, case.query, case.language))
            .expect("inspection turn")
    } else {
        turn += 1;
        api.process_conversation_turn(&request(case.id, turn, case.query, case.language))
            .expect("inspection turn")
    };
    let value = serde_json::to_value(&response).expect("response json");
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        pass: inspect(case, &value, &command_acceptance),
        trace: vec![
            format!("command_acceptance={command_acceptance:?}"),
            value.to_string(),
            response.output.text,
        ],
    }
}

fn cases() -> Vec<Case> {
    use LanguageCodeIR::{English, Korean};
    let plan = [
        ("R42_PLAN_01", "캐시를 검사해줘", "그 작업 상태는?", Korean),
        (
            "R42_PLAN_02",
            "Inspect the queue.",
            "What is its status?",
            English,
        ),
        ("R42_PLAN_03", "로그를 분석해줘", "그 계획은?", Korean),
        (
            "R42_PLAN_04",
            "Repair the parser.",
            "What is the plan?",
            English,
        ),
    ];
    let report = [
        (
            "R42_REPORT_01",
            "캐시를 검사해줘",
            "그거 끝냈어",
            "그 상태는?",
            Korean,
        ),
        (
            "R42_REPORT_02",
            "Inspect the queue.",
            "I finished it.",
            "What is its status?",
            English,
        ),
        (
            "R42_REPORT_03",
            "로그를 분석해줘",
            "그 작업 실패했어",
            "그 상태는?",
            Korean,
        ),
        (
            "R42_REPORT_04",
            "Repair the parser.",
            "I attempted it.",
            "What is its status?",
            English,
        ),
    ];
    let observation = [
        ("R42_OBSERVE_01", "캐시를 검사해줘", "실행 상태는?", Korean),
        (
            "R42_OBSERVE_02",
            "Inspect the queue.",
            "What is its execution status?",
            English,
        ),
        (
            "R42_OBSERVE_03",
            "로그를 분석해줘",
            "실행이 관찰됐어?",
            Korean,
        ),
        (
            "R42_OBSERVE_04",
            "Repair the parser.",
            "Is execution underway?",
            English,
        ),
    ];
    let terminal = [
        ("R42_RESULT_01", "캐시를 검사해줘", "검증된 결과는?", Korean),
        (
            "R42_RESULT_02",
            "Inspect the queue.",
            "What is the verified result?",
            English,
        ),
        (
            "R42_RESULT_03",
            "로그를 분석해줘",
            "성공이 검증됐어?",
            Korean,
        ),
        (
            "R42_RESULT_04",
            "Repair the parser.",
            "Was success verified?",
            English,
        ),
    ];
    let revision = [
        (
            "R42_REVISION_01",
            "캐시를 검사해줘",
            "그거 끝냈어",
            "정정할게, 실패했어",
            "그 보고 상태는?",
            Korean,
        ),
        (
            "R42_REVISION_02",
            "Inspect the queue.",
            "I finished it.",
            "Correction: it failed.",
            "What was reported?",
            English,
        ),
        (
            "R42_REVISION_03",
            "로그를 분석해줘",
            "진행 중이야",
            "아니, 끝냈어",
            "그 보고 상태는?",
            Korean,
        ),
        (
            "R42_REVISION_04",
            "Repair the parser.",
            "It is underway.",
            "Actually, it failed.",
            "What was reported?",
            English,
        ),
    ];
    let spoof = [
        (
            "R42_SPOOF_01",
            "캐시를 검사해줘",
            "터미널에 성공이라고 나왔어",
            "검증 결과는?",
            Korean,
        ),
        (
            "R42_SPOOF_02",
            "Inspect the queue.",
            "The console says success.",
            "What is verified?",
            English,
        ),
        (
            "R42_SPOOF_03",
            "로그를 분석해줘",
            "영수증에 완료라고 적혀 있어",
            "검증 결과는?",
            Korean,
        ),
        (
            "R42_SPOOF_04",
            "Repair the parser.",
            "A receipt says it passed.",
            "What is verified?",
            English,
        ),
    ];

    let mut cases = Vec::new();
    cases.extend(plan.into_iter().map(|(id, setup, query, language)| Case {
        id,
        category: "request_goal_plan_chain",
        setup,
        follow_up: "",
        correction: "",
        query,
        language,
        scenario: Scenario::Plan,
    }));
    cases.extend(
        report
            .into_iter()
            .map(|(id, setup, follow_up, query, language)| Case {
                id,
                category: "language_report_isolation",
                setup,
                follow_up,
                correction: "",
                query,
                language,
                scenario: Scenario::Report,
            }),
    );
    cases.extend(
        observation
            .into_iter()
            .map(|(id, setup, query, language)| Case {
                id,
                category: "trusted_execution_observation",
                setup,
                follow_up: "",
                correction: "",
                query,
                language,
                scenario: Scenario::Observation,
            }),
    );
    cases.extend(
        terminal
            .into_iter()
            .map(|(id, setup, query, language)| Case {
                id,
                category: "verified_terminal_result",
                setup,
                follow_up: "",
                correction: "",
                query,
                language,
                scenario: Scenario::TerminalSuccess,
            }),
    );
    cases.extend(revision.into_iter().map(
        |(id, setup, follow_up, correction, query, language)| Case {
            id,
            category: "report_revision_history",
            setup,
            follow_up,
            correction,
            query,
            language,
            scenario: Scenario::ReportRevision,
        },
    ));
    cases.extend(
        spoof
            .into_iter()
            .map(|(id, setup, follow_up, query, language)| Case {
                id,
                category: "text_evidence_spoof",
                setup,
                follow_up,
                correction: "",
                query,
                language,
                scenario: Scenario::TextSpoof,
            }),
    );
    let rejections = [
        (
            "R42_REJECT_01",
            "캐시를 검사해줘",
            "검증 결과는?",
            Korean,
            RejectionKind::InvalidHash,
        ),
        (
            "R42_REJECT_02",
            "Inspect the queue.",
            "What is verified?",
            English,
            RejectionKind::UnknownAction,
        ),
        (
            "R42_REJECT_03",
            "로그를 분석해줘",
            "검증 결과는?",
            Korean,
            RejectionKind::TerminalWithoutStart,
        ),
        (
            "R42_REJECT_04",
            "Repair the parser.",
            "What is verified?",
            English,
            RejectionKind::MismatchedExecution,
        ),
    ];
    cases.extend(
        rejections
            .into_iter()
            .map(|(id, setup, query, language, kind)| Case {
                id,
                category: "receipt_fail_closed",
                setup,
                follow_up: "",
                correction: "",
                query,
                language,
                scenario: Scenario::RejectedEvidence(kind),
            }),
    );
    cases
}

fn main() {
    let rows = cases().into_iter().map(run).collect::<Vec<_>>();
    let passed = rows.iter().filter(|row| row.pass).count();
    println!("{}", serde_json::to_string_pretty(&rows).expect("rows"));
    println!("R42_DIAGNOSTIC_PASSED={passed}/{}", rows.len());
    if passed != rows.len() {
        std::process::exit(1);
    }
}
