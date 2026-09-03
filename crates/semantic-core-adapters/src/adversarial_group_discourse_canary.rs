//! Frozen R33-RUN-0001 adversarial discourse-group diagnostic.
//!
//! The suite combines plural action reference, reported-versus-verified state,
//! multi-speaker proposition groups, correction, explicit focus bridging, and
//! fail-closed ambiguity through the public conversational API. No case text
//! or expected answer may enter product code.

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
struct GroupActionCase {
    id: &'static str,
    setup: &'static str,
    follow: &'static str,
    language: LanguageCodeIR,
}

#[derive(Clone, Copy)]
struct SpeakerCase {
    id: &'static str,
    first: &'static str,
    second: &'static str,
    query: &'static str,
    first_source: &'static str,
    second_source: &'static str,
    language: LanguageCodeIR,
}

#[derive(Clone, Copy)]
struct CorrectionCase {
    id: &'static str,
    first: &'static str,
    correction: &'static str,
    second: &'static str,
    query: &'static str,
    retained: &'static str,
    rejected: &'static str,
    language: LanguageCodeIR,
}

#[derive(Clone, Copy)]
struct BridgeCase {
    id: &'static str,
    target: &'static str,
    distractor: &'static str,
    shift: &'static str,
    follow: &'static str,
    expected: &'static str,
    rejected: &'static str,
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

fn group_binding(value: &Value, kind: &str, expected_size: usize) -> bool {
    value
        .pointer("/reference_resolution/discourse_bindings")
        .and_then(Value::as_array)
        .is_some_and(|bindings| {
            bindings.iter().any(|binding| {
                binding["kind"] == kind
                    && binding["referent_ids"]
                        .as_array()
                        .is_some_and(|ids| ids.len() == expected_size)
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

fn realization_safe(response: &semantic_core_adapters::ConversationTurnResponseIR) -> bool {
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
        receipt_id: format!("R33-{phase}-{compact}"),
        conversation_id: conversation_id.to_string(),
        action_id: action_id.to_string(),
        execution_id: format!("EXEC-{compact}"),
        status,
        evidence_digest: format!("{:064x}", compact.len() * 43 + phase.len()),
        verifier_receipt_sha256: String::new(),
    };
    receipt.verifier_receipt_sha256 = action_evidence_receipt_sha256(&receipt);
    let command = json!({"operation":"SUBMIT_ACTION_EVIDENCE", "request":receipt});
    api.execute_command_json(&command.to_string())
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .is_some_and(|response| response["ok"] == true)
}

fn group_query(case: GroupActionCase) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let setup = api
        .process_conversation_turn(&request(case.id, 1, case.setup, case.language))
        .expect("group setup");
    let ids = action_ids(&value(&setup));
    let response = api
        .process_conversation_turn(&request(case.id, 2, case.follow, case.language))
        .expect("group query");
    let observed = value(&response);
    Row {
        id: case.id.to_string(),
        category: "plural_action_status_query".to_string(),
        pass: ids.len() == 2
            && targets(&observed) == ids
            && group_binding(&observed, "PLURAL_EVENT_REFERENCE", 2)
            && claims_cover(&observed, &ids, "PLAN_STATUS", false)
            && claims_cover(&observed, &ids, "EVIDENCE_ABSENCE", false)
            && response.disposition == ConversationTurnDispositionIR::Grounded
            && realization_safe(&response),
        trace: vec![observed.to_string()],
    }
}

fn group_report(case: GroupActionCase) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let setup = api
        .process_conversation_turn(&request(case.id, 1, case.setup, case.language))
        .expect("group setup");
    let ids = action_ids(&value(&setup));
    let response = api
        .process_conversation_turn(&request(case.id, 2, case.follow, case.language))
        .expect("group report");
    let observed = value(&response);
    let report_count = observed
        .pointer("/action_state_analysis/detected_reports")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let ledger_reports = observed
        .pointer("/conversation_state/action_state_ledger/records")
        .and_then(Value::as_array)
        .is_some_and(|records| {
            records.len() == 2
                && records
                    .iter()
                    .all(|record| record["reported_status"] == "SUCCESS_CLAIMED")
        });
    Row {
        id: case.id.to_string(),
        category: "plural_action_language_report".to_string(),
        pass: ids.len() == 2
            && targets(&observed) == ids
            && report_count == 2
            && ledger_reports
            && claims_cover(&observed, &ids, "LANGUAGE_REPORT", false)
            && !claims_cover(&observed, &ids, "VERIFIED_EXECUTION", true)
            && realization_safe(&response),
        trace: vec![observed.to_string()],
    }
}

fn group_verified(case: GroupActionCase) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let setup = api
        .process_conversation_turn(&request(case.id, 1, case.setup, case.language))
        .expect("group setup");
    let ids = action_ids(&value(&setup));
    let receipts = ids.iter().all(|id| {
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
    });
    let response = api
        .process_conversation_turn(&request(case.id, 2, case.follow, case.language))
        .expect("group verified query");
    let observed = value(&response);
    Row {
        id: case.id.to_string(),
        category: "plural_action_verified_execution".to_string(),
        pass: ids.len() == 2
            && receipts
            && targets(&observed) == ids
            && claims_cover(&observed, &ids, "VERIFIED_EXECUTION", true)
            && realization_safe(&response),
        trace: vec![observed.to_string()],
    }
}

fn speaker_group(case: SpeakerCase) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(case.id, 1, case.first, case.language))
        .expect("first speaker");
    api.process_conversation_turn(&request(case.id, 2, case.second, case.language))
        .expect("second speaker");
    let response = api
        .process_conversation_turn(&request(case.id, 3, case.query, case.language))
        .expect("speaker group query");
    let observed = value(&response);
    let resolved = response
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    Row {
        id: case.id.to_string(),
        category: "plural_multi_speaker_proposition_reference".to_string(),
        pass: group_binding(&observed, "PLURAL_PROPOSITION_REFERENCE", 2)
            && resolved.contains(&case.first_source.to_lowercase())
            && resolved.contains(&case.second_source.to_lowercase())
            && response.grounded_response.is_some()
            && response.disposition == ConversationTurnDispositionIR::Grounded
            && realization_safe(&response),
        trace: vec![observed.to_string()],
    }
}

fn correction_group(case: CorrectionCase) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(case.id, 1, case.first, case.language))
        .expect("first claim");
    api.process_conversation_turn(&request(case.id, 2, case.correction, case.language))
        .expect("correction");
    api.process_conversation_turn(&request(case.id, 3, case.second, case.language))
        .expect("second speaker");
    let response = api
        .process_conversation_turn(&request(case.id, 4, case.query, case.language))
        .expect("correction group query");
    let observed = value(&response);
    let resolved = response
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    Row {
        id: case.id.to_string(),
        category: "correction_aware_speaker_group".to_string(),
        pass: group_binding(&observed, "PLURAL_PROPOSITION_REFERENCE", 2)
            && resolved.contains(&case.retained.to_lowercase())
            && !resolved.contains(&case.rejected.to_lowercase())
            && response.grounded_response.is_some()
            && realization_safe(&response),
        trace: vec![observed.to_string()],
    }
}

fn focus_bridge(case: BridgeCase) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(case.id, 1, case.target, case.language))
        .expect("bridge target");
    api.process_conversation_turn(&request(case.id, 2, case.distractor, case.language))
        .expect("bridge distractor");
    api.process_conversation_turn(&request(case.id, 3, case.shift, case.language))
        .expect("focus return");
    let response = api
        .process_conversation_turn(&request(case.id, 4, case.follow, case.language))
        .expect("possessive bridge");
    let observed = value(&response);
    let resolved = response
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    Row {
        id: case.id.to_string(),
        category: "explicit_focus_possessive_bridge".to_string(),
        pass: resolved.contains(&case.expected.to_lowercase())
            && !resolved.contains(&case.rejected.to_lowercase())
            && observed
                .pointer("/reference_resolution/discourse_bindings")
                .and_then(Value::as_array)
                .is_some_and(|bindings| {
                    bindings
                        .iter()
                        .any(|binding| binding["kind"] == "POSSESSIVE_FOCUS_REFERENCE")
                })
            && response.grounded_response.is_some()
            && realization_safe(&response),
        trace: vec![observed.to_string()],
    }
}

fn ambiguity_case(case: GroupActionCase, three_speakers: bool) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    if three_speakers {
        api.process_conversation_turn(&request(case.id, 1, case.setup, case.language))
            .expect("speaker one");
        let second = if case.language == LanguageCodeIR::English {
            "Bob says that the worker is blocked."
        } else {
            "지수는 워커가 막혔다고 말했다."
        };
        let third = if case.language == LanguageCodeIR::English {
            "Kai says that the queue is stale."
        } else {
            "수아는 큐가 오래됐다고 말했다."
        };
        api.process_conversation_turn(&request(case.id, 2, second, case.language))
            .expect("speaker two");
        api.process_conversation_turn(&request(case.id, 3, third, case.language))
            .expect("speaker three");
        let response = api
            .process_conversation_turn(&request(case.id, 4, case.follow, case.language))
            .expect("ambiguous speakers");
        let observed = value(&response);
        return Row {
            id: case.id.to_string(),
            category: "group_reference_ambiguity_fails_closed".to_string(),
            pass: response.disposition == ConversationTurnDispositionIR::ClarificationRequired
                && !group_binding(&observed, "PLURAL_PROPOSITION_REFERENCE", 2)
                && response.grounded_response.is_none()
                && realization_safe(&response),
            trace: vec![observed.to_string()],
        };
    }
    api.process_conversation_turn(&request(case.id, 1, case.setup, case.language))
        .expect("three action setup");
    let response = api
        .process_conversation_turn(&request(case.id, 2, case.follow, case.language))
        .expect("ambiguous action group");
    let observed = value(&response);
    let no_reports = observed
        .pointer("/conversation_state/action_state_ledger/records")
        .and_then(Value::as_array)
        .is_some_and(|records| {
            records.iter().all(|record| {
                record.get("reported_status").is_none() || record["reported_status"].is_null()
            })
        });
    Row {
        id: case.id.to_string(),
        category: "group_reference_ambiguity_fails_closed".to_string(),
        pass: response.disposition == ConversationTurnDispositionIR::ClarificationRequired
            && targets(&observed).is_empty()
            && no_reports
            && realization_safe(&response),
        trace: vec![observed.to_string()],
    }
}

fn main() {
    let en_actions = "repair the cache and inspect the queue";
    let ko_actions = "캐시를 수리하고 큐를 확인해";
    let mut rows = vec![
        group_query(GroupActionCase {
            id: "R33_GROUP_QUERY_EN_1",
            setup: en_actions,
            follow: "What are the statuses of both actions?",
            language: LanguageCodeIR::English,
        }),
        group_query(GroupActionCase {
            id: "R33_GROUP_QUERY_EN_2",
            setup: "inspect the parser and repair the worker",
            follow: "Give me the results of all actions",
            language: LanguageCodeIR::English,
        }),
        group_query(GroupActionCase {
            id: "R33_GROUP_QUERY_KO_1",
            setup: ko_actions,
            follow: "두 작업 모두의 상태는?",
            language: LanguageCodeIR::Korean,
        }),
        group_query(GroupActionCase {
            id: "R33_GROUP_QUERY_KO_2",
            setup: "파서를 확인하고 워커를 수리해",
            follow: "모든 작업의 결과를 알려줘",
            language: LanguageCodeIR::Korean,
        }),
        group_report(GroupActionCase {
            id: "R33_GROUP_REPORT_EN_1",
            setup: en_actions,
            follow: "Both actions finished",
            language: LanguageCodeIR::English,
        }),
        group_report(GroupActionCase {
            id: "R33_GROUP_REPORT_EN_2",
            setup: "repair the parser and repair the worker",
            follow: "I completed all actions",
            language: LanguageCodeIR::English,
        }),
        group_report(GroupActionCase {
            id: "R33_GROUP_REPORT_KO_1",
            setup: ko_actions,
            follow: "두 작업 모두 완료했어",
            language: LanguageCodeIR::Korean,
        }),
        group_report(GroupActionCase {
            id: "R33_GROUP_REPORT_KO_2",
            setup: "파서를 수리하고 워커를 수리해",
            follow: "작업 전부 끝났어",
            language: LanguageCodeIR::Korean,
        }),
        group_verified(GroupActionCase {
            id: "R33_GROUP_VERIFY_EN_1",
            setup: en_actions,
            follow: "What are the verified results of both actions?",
            language: LanguageCodeIR::English,
        }),
        group_verified(GroupActionCase {
            id: "R33_GROUP_VERIFY_EN_2",
            setup: "inspect the parser and repair the worker",
            follow: "Did all actions succeed?",
            language: LanguageCodeIR::English,
        }),
        group_verified(GroupActionCase {
            id: "R33_GROUP_VERIFY_KO_1",
            setup: ko_actions,
            follow: "두 작업 모두의 검증된 결과는?",
            language: LanguageCodeIR::Korean,
        }),
        group_verified(GroupActionCase {
            id: "R33_GROUP_VERIFY_KO_2",
            setup: "파서를 확인하고 워커를 수리해",
            follow: "모든 작업이 성공했어?",
            language: LanguageCodeIR::Korean,
        }),
        speaker_group(SpeakerCase {
            id: "R33_SPEAKER_EN_1",
            first: "Alice says that the cache is stale.",
            second: "Bob says that the worker is blocked.",
            query: "Compare their claims",
            first_source: "alice",
            second_source: "bob",
            language: LanguageCodeIR::English,
        }),
        speaker_group(SpeakerCase {
            id: "R33_SPEAKER_EN_2",
            first: "Nora believes that the parser might stall.",
            second: "Omar reports that the queue is growing.",
            query: "Analyze their statements",
            first_source: "nora",
            second_source: "omar",
            language: LanguageCodeIR::English,
        }),
        speaker_group(SpeakerCase {
            id: "R33_SPEAKER_KO_1",
            first: "민수는 캐시가 오래됐다고 말했다.",
            second: "지수는 워커가 막혔다고 말했다.",
            query: "그들의 주장을 비교해",
            first_source: "민수",
            second_source: "지수",
            language: LanguageCodeIR::Korean,
        }),
        speaker_group(SpeakerCase {
            id: "R33_SPEAKER_KO_2",
            first: "수아는 파서가 멈출 수 있다고 믿는다.",
            second: "준은 큐가 늘고 있다고 보고했다.",
            query: "그들의 말을 분석해",
            first_source: "수아",
            second_source: "준",
            language: LanguageCodeIR::Korean,
        }),
        correction_group(CorrectionCase {
            id: "R33_CORRECT_EN_1",
            first: "Alice says that the cache is stale.",
            correction: "Actually, Alice says that the cache is healthy.",
            second: "Bob says that the worker is blocked.",
            query: "Compare their claims",
            retained: "healthy",
            rejected: "stale",
            language: LanguageCodeIR::English,
        }),
        correction_group(CorrectionCase {
            id: "R33_CORRECT_EN_2",
            first: "Nora reports that the build failed.",
            correction: "Correction: Nora reports that the build succeeded.",
            second: "Omar says that the queue is empty.",
            query: "Analyze their statements",
            retained: "succeeded",
            rejected: "failed",
            language: LanguageCodeIR::English,
        }),
        correction_group(CorrectionCase {
            id: "R33_CORRECT_KO_1",
            first: "민수는 캐시가 오래됐다고 말했다.",
            correction: "정정하면 민수는 캐시가 정상이라고 말했다.",
            second: "지수는 워커가 막혔다고 말했다.",
            query: "그들의 주장을 비교해",
            retained: "정상",
            rejected: "오래됐",
            language: LanguageCodeIR::Korean,
        }),
        correction_group(CorrectionCase {
            id: "R33_CORRECT_KO_2",
            first: "수아는 빌드가 실패했다고 보고했다.",
            correction: "아니, 수아는 빌드가 성공했다고 보고했다.",
            second: "준은 큐가 비었다고 말했다.",
            query: "그들의 말을 분석해",
            retained: "성공",
            rejected: "실패",
            language: LanguageCodeIR::Korean,
        }),
        focus_bridge(BridgeCase {
            id: "R33_BRIDGE_EN_1",
            target: "inspect the zircon repository",
            distractor: "inspect the ember queue",
            shift: "return to the zircon repository",
            follow: "inspect its manifest",
            expected: "zircon repository's manifest",
            rejected: "ember",
            language: LanguageCodeIR::English,
        }),
        focus_bridge(BridgeCase {
            id: "R33_BRIDGE_EN_2",
            target: "inspect the cobalt service",
            distractor: "inspect the amber cache",
            shift: "back to the cobalt service",
            follow: "analyze its configuration",
            expected: "cobalt service's configuration",
            rejected: "amber",
            language: LanguageCodeIR::English,
        }),
        focus_bridge(BridgeCase {
            id: "R33_BRIDGE_KO_1",
            target: "지르콘을 확인해",
            distractor: "엠버를 확인해",
            shift: "지르콘 이야기로 돌아가자",
            follow: "그것의 매니페스트를 확인해",
            expected: "지르콘의 매니페스트",
            rejected: "엠버",
            language: LanguageCodeIR::Korean,
        }),
        focus_bridge(BridgeCase {
            id: "R33_BRIDGE_KO_2",
            target: "코발트를 확인해",
            distractor: "앰버를 확인해",
            shift: "코발트 이야기로 돌아가자",
            follow: "그것의 설정을 분석해",
            expected: "코발트의 설정",
            rejected: "앰버",
            language: LanguageCodeIR::Korean,
        }),
    ];
    rows.extend([
        ambiguity_case(
            GroupActionCase {
                id: "R33_AMBIG_ACTION_EN",
                setup: "repair the cache, inspect the queue, and analyze the worker",
                follow: "Both actions finished",
                language: LanguageCodeIR::English,
            },
            false,
        ),
        ambiguity_case(
            GroupActionCase {
                id: "R33_AMBIG_ACTION_KO",
                setup: "캐시를 수리하고 큐를 확인한 뒤 워커를 분석해",
                follow: "두 작업 모두 끝났어",
                language: LanguageCodeIR::Korean,
            },
            false,
        ),
        ambiguity_case(
            GroupActionCase {
                id: "R33_AMBIG_SPEAKER_EN",
                setup: "Alice says that the cache is stale.",
                follow: "Compare their claims",
                language: LanguageCodeIR::English,
            },
            true,
        ),
        ambiguity_case(
            GroupActionCase {
                id: "R33_AMBIG_SPEAKER_KO",
                setup: "민수는 캐시가 오래됐다고 말했다.",
                follow: "그들의 주장을 비교해",
                language: LanguageCodeIR::Korean,
            },
            true,
        ),
    ]);
    let passed = rows.iter().filter(|row| row.pass).count();
    let total = rows.len();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "suite":"R33-RUN-0001",
            "frozen_before_product_changes":true,
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
