//! Frozen R33-TRANSFER-0001 held-out group-discourse suite.
//!
//! Do not semantically execute this binary until R33-RUN-0001 passes.

use semantic_core_adapters::{
    action_evidence_receipt_sha256, ActionEvidenceRequestIR, ActionEvidenceStatusIR, CognitiveApi,
    ConversationInputModalityIR, ConversationTurnDispositionIR, ConversationTurnRequestIR,
    LanguageCodeIR, ACTION_EVIDENCE_REQUEST_SCHEMA, CONVERSATION_TURN_REQUEST_SCHEMA,
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
struct ActionCase {
    id: &'static str,
    setup: &'static str,
    follow: &'static str,
    language: LanguageCodeIR,
}

#[derive(Clone, Copy)]
struct SpeakerCase {
    id: &'static str,
    turns: &'static [&'static str],
    query: &'static str,
    required: &'static [&'static str],
    rejected: Option<&'static str>,
    language: LanguageCodeIR,
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

fn value(response: &semantic_core_adapters::ConversationTurnResponseIR) -> Value {
    serde_json::to_value(response).expect("response json")
}

fn action_ids(value: &Value) -> Vec<String> {
    value
        .pointer("/conversation_state/action_state_ledger/records")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|record| record["action_id"].as_str().map(str::to_string))
        .collect()
}

fn targets(value: &Value) -> Vec<String> {
    value
        .pointer("/action_state_analysis/target_action_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.as_str().map(str::to_string))
        .collect()
}

fn binding(value: &Value, kind: &str, size: usize) -> bool {
    value
        .pointer("/reference_resolution/discourse_bindings")
        .and_then(Value::as_array)
        .is_some_and(|bindings| {
            bindings.iter().any(|item| {
                item["kind"] == kind
                    && item["referent_ids"]
                        .as_array()
                        .is_some_and(|ids| ids.len() == size)
            })
        })
}

fn claims_cover(value: &Value, ids: &[String], kind: &str, verified: bool) -> bool {
    value
        .pointer("/grounded_realization/claims")
        .and_then(Value::as_array)
        .is_some_and(|claims| {
            ids.iter().all(|id| {
                claims.iter().any(|claim| {
                    claim["kind"] == kind
                        && claim["verified"] == verified
                        && claim["evidence_refs"]
                            .as_array()
                            .is_some_and(|refs| refs.iter().any(|item| item == id))
                })
            })
        })
}

fn safe(response: &semantic_core_adapters::ConversationTurnResponseIR) -> bool {
    response.grounded_realization.validate()
        && response.grounded_realization.realized_text == response.output.text
        && response.grounded_realization.unsupported_claims == 0
        && !response.grounded_realization.semantic_authority
        && !response.grounded_realization.external_action_executed
}

fn submit(
    api: &mut CognitiveApi,
    conversation_id: &str,
    action_id: &str,
    phase: &str,
    status: ActionEvidenceStatusIR,
) -> bool {
    let compact = action_id.replace('-', "");
    let mut receipt = ActionEvidenceRequestIR {
        schema: ACTION_EVIDENCE_REQUEST_SCHEMA.to_string(),
        receipt_id: format!("R33X-{phase}-{compact}"),
        conversation_id: conversation_id.to_string(),
        action_id: action_id.to_string(),
        execution_id: format!("XEXEC-{compact}"),
        status,
        evidence_digest: format!("{:064x}", compact.len() * 59 + phase.len()),
        verifier_receipt_sha256: String::new(),
    };
    receipt.verifier_receipt_sha256 = action_evidence_receipt_sha256(&receipt);
    let command = json!({"operation":"SUBMIT_ACTION_EVIDENCE", "request":receipt});
    api.execute_command_json(&command.to_string())
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .is_some_and(|response| response["ok"] == true)
}

fn action_case(case: ActionCase, mode: &str) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let setup = api
        .process_conversation_turn(&request(case.id, 1, case.setup, case.language))
        .expect("action setup");
    let ids = action_ids(&value(&setup));
    let receipts = if mode == "verified" {
        ids.iter().all(|id| {
            submit(
                &mut api,
                case.id,
                id,
                "START",
                ActionEvidenceStatusIR::ExecutionStarted,
            ) && submit(
                &mut api,
                case.id,
                id,
                "END",
                ActionEvidenceStatusIR::Succeeded,
            )
        })
    } else {
        true
    };
    let response = api
        .process_conversation_turn(&request(case.id, 2, case.follow, case.language))
        .expect("group follow-up");
    let observed = value(&response);
    let pass = ids.len() == 2
        && receipts
        && targets(&observed) == ids
        && binding(&observed, "PLURAL_EVENT_REFERENCE", 2)
        && match mode {
            "query" => {
                claims_cover(&observed, &ids, "PLAN_STATUS", false)
                    && claims_cover(&observed, &ids, "EVIDENCE_ABSENCE", false)
            }
            "report" => {
                observed
                    .pointer("/action_state_analysis/detected_reports")
                    .and_then(Value::as_array)
                    .is_some_and(|reports| reports.len() == 2)
                    && claims_cover(&observed, &ids, "LANGUAGE_REPORT", false)
                    && !claims_cover(&observed, &ids, "VERIFIED_EXECUTION", true)
            }
            _ => claims_cover(&observed, &ids, "VERIFIED_EXECUTION", true),
        }
        && response.disposition == ConversationTurnDispositionIR::Grounded
        && safe(&response);
    Row {
        id: case.id.to_string(),
        category: format!("heldout_plural_action_{mode}"),
        pass,
        trace: vec![observed.to_string()],
    }
}

fn speaker_case(case: SpeakerCase) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    for (index, text) in case.turns.iter().enumerate() {
        api.process_conversation_turn(&request(
            case.id,
            u64::try_from(index + 1).expect("bounded turn"),
            text,
            case.language,
        ))
        .expect("speaker turn");
    }
    let query_turn = u64::try_from(case.turns.len() + 1).expect("bounded turn");
    let response = api
        .process_conversation_turn(&request(case.id, query_turn, case.query, case.language))
        .expect("speaker query");
    let observed = value(&response);
    let resolved = response
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    Row {
        id: case.id.to_string(),
        category: "heldout_multi_speaker_group".to_string(),
        pass: binding(&observed, "PLURAL_PROPOSITION_REFERENCE", 2)
            && case
                .required
                .iter()
                .all(|term| resolved.contains(&term.to_lowercase()))
            && case
                .rejected
                .is_none_or(|term| !resolved.contains(&term.to_lowercase()))
            && response.grounded_response.is_some()
            && response.disposition == ConversationTurnDispositionIR::Grounded
            && safe(&response),
        trace: vec![observed.to_string()],
    }
}

fn fail_closed(case: SpeakerCase) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    for (index, text) in case.turns.iter().enumerate() {
        api.process_conversation_turn(&request(
            case.id,
            u64::try_from(index + 1).expect("bounded turn"),
            text,
            case.language,
        ))
        .expect("ambiguity setup");
    }
    let query_turn = u64::try_from(case.turns.len() + 1).expect("bounded turn");
    let response = api
        .process_conversation_turn(&request(case.id, query_turn, case.query, case.language))
        .expect("ambiguous group");
    let observed = value(&response);
    Row {
        id: case.id.to_string(),
        category: "heldout_group_ambiguity".to_string(),
        pass: response.disposition == ConversationTurnDispositionIR::ClarificationRequired
            && !binding(&observed, "PLURAL_PROPOSITION_REFERENCE", 2)
            && targets(&observed).is_empty()
            && safe(&response),
        trace: vec![observed.to_string()],
    }
}

fn main() {
    let mut rows = vec![
        action_case(
            ActionCase {
                id: "R33X_QUERY_EN_1",
                setup: "inspect the relay and repair the broker",
                follow: "Show the state of both tasks",
                language: LanguageCodeIR::English,
            },
            "query",
        ),
        action_case(
            ActionCase {
                id: "R33X_QUERY_EN_2",
                setup: "repair the conduit and analyze the journal",
                follow: "What happened with all tasks?",
                language: LanguageCodeIR::English,
            },
            "query",
        ),
        action_case(
            ActionCase {
                id: "R33X_QUERY_KO_1",
                setup: "릴레이를 확인하고 브로커를 수리해",
                follow: "두 작업의 상태를 보여줘",
                language: LanguageCodeIR::Korean,
            },
            "query",
        ),
        action_case(
            ActionCase {
                id: "R33X_QUERY_KO_2",
                setup: "도관을 수리하고 저널을 분석해",
                follow: "작업 전체의 결과는?",
                language: LanguageCodeIR::Korean,
            },
            "query",
        ),
        action_case(
            ActionCase {
                id: "R33X_REPORT_EN_1",
                setup: "inspect the relay and repair the broker",
                follow: "Both tasks are done",
                language: LanguageCodeIR::English,
            },
            "report",
        ),
        action_case(
            ActionCase {
                id: "R33X_REPORT_EN_2",
                setup: "repair the conduit and analyze the journal",
                follow: "Every action completed",
                language: LanguageCodeIR::English,
            },
            "report",
        ),
        action_case(
            ActionCase {
                id: "R33X_REPORT_KO_1",
                setup: "릴레이를 확인하고 브로커를 수리해",
                follow: "두 작업 다 완료됐어",
                language: LanguageCodeIR::Korean,
            },
            "report",
        ),
        action_case(
            ActionCase {
                id: "R33X_REPORT_KO_2",
                setup: "도관을 수리하고 저널을 분석해",
                follow: "전체 작업을 끝냈어",
                language: LanguageCodeIR::Korean,
            },
            "report",
        ),
        action_case(
            ActionCase {
                id: "R33X_VERIFY_EN_1",
                setup: "inspect the relay and repair the broker",
                follow: "Were both tasks verified successful?",
                language: LanguageCodeIR::English,
            },
            "verified",
        ),
        action_case(
            ActionCase {
                id: "R33X_VERIFY_EN_2",
                setup: "repair the conduit and analyze the journal",
                follow: "Give the verified outcomes for every action",
                language: LanguageCodeIR::English,
            },
            "verified",
        ),
        action_case(
            ActionCase {
                id: "R33X_VERIFY_KO_1",
                setup: "릴레이를 확인하고 브로커를 수리해",
                follow: "두 작업 다 검증된 성공이야?",
                language: LanguageCodeIR::Korean,
            },
            "verified",
        ),
        action_case(
            ActionCase {
                id: "R33X_VERIFY_KO_2",
                setup: "도관을 수리하고 저널을 분석해",
                follow: "전체 작업의 검증 결과를 말해줘",
                language: LanguageCodeIR::Korean,
            },
            "verified",
        ),
        speaker_case(SpeakerCase {
            id: "R33X_SPEAKER_EN_1",
            turns: &[
                "Iris says that the relay is unstable.",
                "Jules reports that the broker is saturated.",
            ],
            query: "Review their reports",
            required: &["iris", "jules"],
            rejected: None,
            language: LanguageCodeIR::English,
        }),
        speaker_case(SpeakerCase {
            id: "R33X_SPEAKER_KO_1",
            turns: &[
                "하람은 릴레이가 불안정하다고 말했다.",
                "누리는 브로커가 포화됐다고 보고했다.",
            ],
            query: "그들의 보고를 검토해",
            required: &["하람", "누리"],
            rejected: None,
            language: LanguageCodeIR::Korean,
        }),
        speaker_case(SpeakerCase {
            id: "R33X_CORRECT_EN_1",
            turns: &[
                "Iris says that the relay is unstable.",
                "Actually, Iris says that the relay is stable.",
                "Jules reports that the broker is saturated.",
            ],
            query: "Compare their reports",
            required: &["stable", "saturated"],
            rejected: Some("unstable"),
            language: LanguageCodeIR::English,
        }),
        speaker_case(SpeakerCase {
            id: "R33X_CORRECT_KO_1",
            turns: &[
                "하람은 릴레이가 불안정하다고 말했다.",
                "정정하면 하람은 릴레이가 안정적이라고 말했다.",
                "누리는 브로커가 포화됐다고 보고했다.",
            ],
            query: "그들의 보고를 비교해",
            required: &["안정", "포화"],
            rejected: Some("불안정"),
            language: LanguageCodeIR::Korean,
        }),
    ];
    rows.extend([
        fail_closed(SpeakerCase {
            id: "R33X_AMBIG_EN_1",
            turns: &[
                "Iris says that the relay is unstable.",
                "Jules says that the broker is saturated.",
                "Mara says that the journal is incomplete.",
            ],
            query: "Compare their reports",
            required: &[],
            rejected: None,
            language: LanguageCodeIR::English,
        }),
        fail_closed(SpeakerCase {
            id: "R33X_AMBIG_KO_1",
            turns: &[
                "하람은 릴레이가 불안정하다고 말했다.",
                "누리는 브로커가 포화됐다고 말했다.",
                "마루는 저널이 불완전하다고 말했다.",
            ],
            query: "그들의 보고를 비교해",
            required: &[],
            rejected: None,
            language: LanguageCodeIR::Korean,
        }),
        fail_closed(SpeakerCase {
            id: "R33X_UNBOUND_EN",
            turns: &[],
            query: "Show the results of both tasks",
            required: &[],
            rejected: None,
            language: LanguageCodeIR::English,
        }),
        fail_closed(SpeakerCase {
            id: "R33X_UNBOUND_KO",
            turns: &[],
            query: "두 작업의 결과를 보여줘",
            required: &[],
            rejected: None,
            language: LanguageCodeIR::Korean,
        }),
    ]);
    let passed = rows.iter().filter(|row| row.pass).count();
    let total = rows.len();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "suite":"R33-TRANSFER-0001",
            "held_out_until_after_diagnostic_pass":true,
            "external_llm_calls":0,
            "local_teacher_calls":0,
            "network_calls":0,
            "recursive_source_mutations":0,
            "total":total,
            "passed":passed,
            "failed":total-passed,
            "rows":rows
        }))
        .expect("suite json")
    );
    if passed != total {
        std::process::exit(1);
    }
}
