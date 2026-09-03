//! Frozen R43 diagnostic suite.
//!
//! Every case crosses multiple language axes and observes only the public
//! conversation response.  The suite was frozen before the R43 product
//! contract existed.

use std::collections::BTreeSet;

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR, LanguageCodeIR,
    CONVERSATION_TURN_REQUEST_SCHEMA, CONVERSATION_TURN_RESPONSE_SCHEMA,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    category: String,
    pass: bool,
    trace: Vec<String>,
}

#[derive(Clone, Copy)]
enum Expectation {
    PlannedPredicate(&'static str),
    Reference {
        target: &'static str,
        rejected: &'static str,
    },
    Ambiguous,
    ReportOnly,
    UntrustedEvidenceRejected,
    EvidenceConflict,
}

struct Case {
    id: &'static str,
    category: &'static str,
    turns: Vec<(&'static str, LanguageCodeIR)>,
    expectation: Expectation,
}

fn request(
    conversation_id: &str,
    turn_index: u64,
    text: &str,
    language: LanguageCodeIR,
) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: conversation_id.to_string(),
        turn_index,
        request_id: format!("{conversation_id}-{turn_index}"),
        modality: ConversationInputModalityIR::Text,
        raw_text: text.to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(language),
        context_tags: Vec::new(),
        max_plan_steps: 16,
    }
}

fn integration_contract(value: &Value) -> bool {
    let expected_axes = BTreeSet::from([
        "GRAMMATICAL_COMPOSITION",
        "DISCOURSE_TOPIC_STATE",
        "DEIXIS_ELLIPSIS",
        "PRAGMATIC_INTENT",
        "PLAN_RESULT_BOUNDARY",
        "EVIDENCE_GROUNDED_REALIZATION",
    ]);
    let Some(axes) = value
        .pointer("/six_axis_integration/axes")
        .and_then(Value::as_array)
    else {
        return false;
    };
    let actual_axes = axes
        .iter()
        .filter_map(|axis| axis["axis"].as_str())
        .collect::<BTreeSet<_>>();
    let invariants = value
        .pointer("/six_axis_integration/cross_axis_invariants")
        .and_then(Value::as_array);
    value["schema"] == CONVERSATION_TURN_RESPONSE_SCHEMA
        && value.pointer("/six_axis_integration/schema")
            == Some(&Value::String(
                "B_CORE_SIX_AXIS_INTEGRATION_IR_2".to_string(),
            ))
        && value.pointer("/six_axis_integration/complete") == Some(&Value::Bool(true))
        && value.pointer("/six_axis_integration/semantic_authority") == Some(&Value::Bool(false))
        && value.pointer("/six_axis_integration/language_can_execute") == Some(&Value::Bool(false))
        && value
            .pointer("/six_axis_integration/violations")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
        && value
            .pointer("/six_axis_integration/integration_sha256")
            .and_then(Value::as_str)
            .is_some_and(|hash| hash.len() == 64)
        && actual_axes == expected_axes
        && axes.iter().all(|axis| {
            axis["status"] == "PASS"
                && axis["semantic_authority"] == false
                && axis["external_action_executed"] == false
                && axis["component_sha256"]
                    .as_str()
                    .is_some_and(|hash| hash.len() == 64)
        })
        && invariants.is_some_and(|items| {
            items.len() >= 6 && items.iter().all(|item| item["satisfied"] == true)
        })
        && value.pointer("/six_axis_integration/package_boundary/schema")
            == Some(&Value::String(
                "B_CORE_LANGUAGE_CORTEX_PACKAGE_BOUNDARY_IR_1".to_string(),
            ))
        && value.pointer("/six_axis_integration/package_boundary/valid") == Some(&Value::Bool(true))
        && value.pointer("/six_axis_integration/package_boundary/semantic_authority")
            == Some(&Value::Bool(false))
        && value.pointer("/six_axis_integration/package_boundary/raw_language_reaches_core")
            == Some(&Value::Bool(false))
        && value.pointer("/six_axis_integration/package_boundary/adapter_owns_semantic_state")
            == Some(&Value::Bool(false))
        && value
            .pointer("/six_axis_integration/package_boundary/boundary_sha256")
            .and_then(Value::as_str)
            .is_some_and(|hash| hash.len() == 64)
}

fn selected_predicates(value: &Value) -> BTreeSet<&str> {
    let selected = value
        .pointer("/pragmatic_interpretation/pragmatic_intent_graph/composition/selected_node_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    value
        .pointer("/pragmatic_interpretation/pragmatic_intent_graph/composition/nodes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|node| {
            node["node_id"]
                .as_str()
                .is_some_and(|id| selected.contains(id))
        })
        .filter_map(|node| node["canonical_predicate"].as_str())
        .collect()
}

fn has_verified_result(value: &Value) -> bool {
    value
        .pointer("/interaction_provenance/nodes")
        .and_then(Value::as_array)
        .is_some_and(|nodes| nodes.iter().any(|node| node["kind"] == "VERIFIED_RESULT"))
}

fn expectation_holds(value: &Value, expectation: Expectation) -> bool {
    match expectation {
        Expectation::PlannedPredicate(predicate) => {
            value
                .get("grounded_response")
                .is_some_and(|item| !item.is_null())
                && value
                    .pointer("/output/grounded_plan_sha256")
                    .and_then(Value::as_str)
                    .is_some_and(|hash| hash.len() == 64)
                && selected_predicates(value).contains(predicate)
                && !has_verified_result(value)
        }
        Expectation::Reference { target, rejected } => {
            value
                .pointer("/reference_resolution/resolved_semantic_text")
                .and_then(Value::as_str)
                .is_some_and(|text| {
                    let text = text.to_lowercase();
                    text.contains(target) && !text.contains(rejected)
                })
                && value
                    .get("grounded_response")
                    .is_some_and(|item| !item.is_null())
        }
        Expectation::Ambiguous => {
            value.get("grounded_response").is_none_or(Value::is_null)
                && value.pointer(
                    "/pragmatic_interpretation/compositional_analysis/clarification_required",
                ) == Some(&Value::Bool(true))
                && value
                    .pointer("/conversation_state/action_state_ledger/records")
                    .and_then(Value::as_array)
                    .is_some_and(Vec::is_empty)
        }
        Expectation::ReportOnly => {
            value
                .pointer("/conversation_state/action_state_ledger/language_report_history")
                .and_then(Value::as_array)
                .is_some_and(|reports| !reports.is_empty())
                && !has_verified_result(value)
                && value
                    .pointer("/grounded_realization/claims")
                    .and_then(Value::as_array)
                    .is_some_and(|claims| {
                        claims.iter().all(|claim| {
                            claim["kind"] != "VERIFIED_EXECUTION" || claim["verified"] != true
                        })
                    })
        }
        Expectation::UntrustedEvidenceRejected => {
            value
                .pointer("/conversation_state/action_state_ledger/language_report_history")
                .and_then(Value::as_array)
                .is_some_and(Vec::is_empty)
                && value
                    .pointer("/conversation_state/action_state_ledger/evidence_audit_history")
                    .and_then(Value::as_array)
                    .is_some_and(Vec::is_empty)
                && !has_verified_result(value)
                && value
                    .pointer("/grounded_realization/claims")
                    .and_then(Value::as_array)
                    .is_some_and(|claims| claims.iter().all(|claim| claim["verified"] != true))
        }
        Expectation::EvidenceConflict => {
            value
                .get("discourse_answer")
                .is_some_and(|answer| !answer.is_null())
                && value.pointer("/discourse_answer/dialogue_truth_established")
                    == Some(&Value::Bool(false))
                && value.get("grounded_response").is_none_or(Value::is_null)
                && !has_verified_result(value)
        }
    }
}

fn run(case: Case) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let mut response = None;
    for (offset, (text, language)) in case.turns.iter().enumerate() {
        response = Some(
            api.process_conversation_turn(&request(
                case.id,
                u64::try_from(offset + 1).expect("bounded turn"),
                text,
                *language,
            ))
            .expect("conversation turn"),
        );
    }
    let response = response.expect("non-empty case");
    let value = serde_json::to_value(&response).expect("response json");
    let product_invariants = response.grounded_realization.validate()
        && response.interaction_provenance.validate_against(
            &response.grounded_realization,
            &response.conversation_state.action_state_ledger,
        )
        && response.output.unsupported_freeform_claims == 0;
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        pass: integration_contract(&value)
            && product_invariants
            && expectation_holds(&value, case.expectation),
        trace: vec![response.output.text, value.to_string()],
    }
}

fn cases() -> Vec<Case> {
    use LanguageCodeIR::{English, Korean};
    let mut cases = vec![
        Case {
            id: "R43_COMPOSE_01",
            category: "composition_intent",
            turns: vec![
                ("파서를 수리해줘", Korean),
                ("아니, 파서를 수리하지 말고 검사해줘", Korean),
            ],
            expectation: Expectation::PlannedPredicate("INVESTIGATE"),
        },
        Case {
            id: "R43_COMPOSE_02",
            category: "composition_intent",
            turns: vec![
                ("Repair the parser.", English),
                ("No, do not repair it; inspect it instead.", English),
            ],
            expectation: Expectation::PlannedPredicate("INVESTIGATE"),
        },
        Case {
            id: "R43_COMPOSE_03",
            category: "composition_intent",
            turns: vec![
                ("워커를 삭제해줘", Korean),
                ("아니, 삭제하지 말고 워커를 분석해줘", Korean),
            ],
            expectation: Expectation::PlannedPredicate("INVESTIGATE"),
        },
        Case {
            id: "R43_COMPOSE_04",
            category: "composition_intent",
            turns: vec![
                ("Delete the cache.", English),
                (
                    "Actually, don't delete it; inspect the cache instead.",
                    English,
                ),
            ],
            expectation: Expectation::PlannedPredicate("INVESTIGATE"),
        },
        Case {
            id: "R43_TOPIC_01",
            category: "topic_deixis",
            turns: vec![
                ("캐시를 확인해", Korean),
                ("워커를 조사해", Korean),
                ("캐시 얘기로 돌아가자", Korean),
                ("그것을 수리해", Korean),
            ],
            expectation: Expectation::Reference {
                target: "캐시",
                rejected: "워커",
            },
        },
        Case {
            id: "R43_TOPIC_02",
            category: "topic_deixis",
            turns: vec![
                ("inspect the log", English),
                ("inspect the server", English),
                ("let's return to the log", English),
                ("repair it", English),
            ],
            expectation: Expectation::Reference {
                target: "log",
                rejected: "server",
            },
        },
        Case {
            id: "R43_TOPIC_03",
            category: "topic_deixis",
            turns: vec![
                ("큐를 확인해", Korean),
                ("백업을 분석해", Korean),
                ("큐 이야기로 돌아가자", Korean),
                ("그거 고쳐", Korean),
            ],
            expectation: Expectation::Reference {
                target: "큐",
                rejected: "백업",
            },
        },
        Case {
            id: "R43_TOPIC_04",
            category: "topic_deixis",
            turns: vec![
                ("inspect the file", English),
                ("analyze the folder", English),
                ("go back to the file topic", English),
                ("repair it", English),
            ],
            expectation: Expectation::Reference {
                target: "file",
                rejected: "folder",
            },
        },
        Case {
            id: "R43_LOCAL_01",
            category: "local_antecedent",
            turns: vec![
                ("워커를 확인해", Korean),
                ("캐시는 오래됐다. 그것을 분석해", Korean),
            ],
            expectation: Expectation::Reference {
                target: "캐시",
                rejected: "워커",
            },
        },
        Case {
            id: "R43_LOCAL_02",
            category: "local_antecedent",
            turns: vec![
                ("inspect the server", English),
                ("the queue is stale. analyze it", English),
            ],
            expectation: Expectation::Reference {
                target: "queue",
                rejected: "server",
            },
        },
        Case {
            id: "R43_LOCAL_03",
            category: "local_antecedent",
            turns: vec![
                ("백업을 확인해", Korean),
                ("로그는 비어 있다. 그것을 분석해", Korean),
            ],
            expectation: Expectation::Reference {
                target: "로그",
                rejected: "백업",
            },
        },
        Case {
            id: "R43_LOCAL_04",
            category: "local_antecedent",
            turns: vec![
                ("inspect the folder", English),
                ("the file is stale. analyze it", English),
            ],
            expectation: Expectation::Reference {
                target: "file",
                rejected: "folder",
            },
        },
        Case {
            id: "R43_AMBIG_01",
            category: "ambiguity_fail_closed",
            turns: vec![("큐를 검사하거나 워커를 수리해줘", Korean)],
            expectation: Expectation::Ambiguous,
        },
        Case {
            id: "R43_AMBIG_02",
            category: "ambiguity_fail_closed",
            turns: vec![("Inspect the queue or repair the worker.", English)],
            expectation: Expectation::Ambiguous,
        },
        Case {
            id: "R43_AMBIG_03",
            category: "ambiguity_fail_closed",
            turns: vec![("캐시를 삭제할지 로그를 분석할지 해줘", Korean)],
            expectation: Expectation::Ambiguous,
        },
        Case {
            id: "R43_AMBIG_04",
            category: "ambiguity_fail_closed",
            turns: vec![("Either delete the cache or inspect the log.", English)],
            expectation: Expectation::Ambiguous,
        },
        Case {
            id: "R43_REPORT_01",
            category: "report_not_result",
            turns: vec![
                ("캐시를 검사해줘", Korean),
                ("그거 끝냈어", Korean),
                ("그 상태는?", Korean),
            ],
            expectation: Expectation::ReportOnly,
        },
        Case {
            id: "R43_REPORT_02",
            category: "report_not_result",
            turns: vec![
                ("Inspect the queue.", English),
                ("I finished it.", English),
                ("What is its status?", English),
            ],
            expectation: Expectation::ReportOnly,
        },
        Case {
            id: "R43_REPORT_03",
            category: "report_not_result",
            turns: vec![
                ("로그를 분석해줘", Korean),
                ("그 작업 실패했어", Korean),
                ("그 상태는?", Korean),
            ],
            expectation: Expectation::ReportOnly,
        },
        Case {
            id: "R43_REPORT_04",
            category: "report_not_result",
            turns: vec![
                ("Repair the parser.", English),
                ("I attempted it.", English),
                ("What is its status?", English),
            ],
            expectation: Expectation::ReportOnly,
        },
        Case {
            id: "R43_SPOOF_01",
            category: "language_evidence_spoof",
            turns: vec![
                ("캐시를 검사해줘", Korean),
                ("터미널에 성공이라고 나왔어", Korean),
                ("검증 결과는?", Korean),
            ],
            expectation: Expectation::UntrustedEvidenceRejected,
        },
        Case {
            id: "R43_SPOOF_02",
            category: "language_evidence_spoof",
            turns: vec![
                ("Inspect the queue.", English),
                ("The console says success.", English),
                ("What is verified?", English),
            ],
            expectation: Expectation::UntrustedEvidenceRejected,
        },
        Case {
            id: "R43_SPOOF_03",
            category: "language_evidence_spoof",
            turns: vec![
                ("로그를 분석해줘", Korean),
                ("영수증에 완료라고 적혀 있어", Korean),
                ("검증 결과는?", Korean),
            ],
            expectation: Expectation::UntrustedEvidenceRejected,
        },
        Case {
            id: "R43_SPOOF_04",
            category: "language_evidence_spoof",
            turns: vec![
                ("Repair the parser.", English),
                ("A receipt says it passed.", English),
                ("What is verified?", English),
            ],
            expectation: Expectation::UntrustedEvidenceRejected,
        },
    ];
    let conflicts = [
        (
            "R43_LONG_01",
            Korean,
            "민아는 캐시가 손상됐다고 말했다",
            "준은 캐시가 정상이라고 말했다",
            "캐시가 실제로 손상됐어?",
        ),
        (
            "R43_LONG_02",
            English,
            "Mina says the worker is blocked",
            "Jules says the worker is healthy",
            "is the worker actually blocked?",
        ),
        (
            "R43_LONG_03",
            Korean,
            "서윤은 서버가 느리다고 말했다",
            "하준은 서버가 빠르다고 말했다",
            "서버가 실제로 느려?",
        ),
        (
            "R43_LONG_04",
            English,
            "Avery says the queue is stale",
            "Rowan says the queue is fresh",
            "is the queue actually stale?",
        ),
    ];
    for (id, language, first, second, question) in conflicts {
        let social = if language == Korean {
            vec![
                ("음...", Korean),
                ("고마워", Korean),
                ("잠깐", Korean),
                ("알겠어", Korean),
                ("그래", Korean),
            ]
        } else {
            vec![
                ("uh...", English),
                ("thanks", English),
                ("one moment", English),
                ("okay", English),
                ("right", English),
            ]
        };
        let mut turns = vec![(first, language), (second, language)];
        turns.extend(social);
        turns.push((question, language));
        cases.push(Case {
            id,
            category: "long_horizon_evidence",
            turns,
            expectation: Expectation::EvidenceConflict,
        });
    }
    cases
}

fn main() {
    let rows = cases().into_iter().map(run).collect::<Vec<_>>();
    let passed = rows.iter().filter(|row| row.pass).count();
    println!("{}", serde_json::to_string_pretty(&rows).expect("rows"));
    println!("R43_DIAGNOSTIC_PASSED={passed}/{}", rows.len());
    if passed != rows.len() {
        std::process::exit(1);
    }
}
