//! Frozen R40-RUN-0002 held-out transfer suite for topic-anchored reference.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, LanguageCodeIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Clone, Copy)]
struct Step<'a> {
    text: &'a str,
    language: LanguageCodeIR,
}

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    category: String,
    pass: bool,
    trace: Vec<String>,
}

fn request(id: &str, turn: u64, step: Step<'_>) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: id.to_string(),
        turn_index: turn,
        request_id: format!("{id}-{turn}"),
        modality: ConversationInputModalityIR::Text,
        raw_text: step.text.to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(step.language),
        context_tags: Vec::new(),
        max_plan_steps: 20,
    }
}

fn execute(id: &str, steps: &[Step<'_>]) -> semantic_core_adapters::ConversationTurnResponseIR {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let mut response = None;
    for (index, step) in steps.iter().enumerate() {
        response = Some(
            api.process_conversation_turn(&request(id, index as u64 + 1, *step))
                .expect("turn"),
        );
    }
    response.expect("steps")
}

fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(|item| item.to_lowercase())
        .collect()
}

fn applied_case(
    id: &str,
    category: &str,
    steps: &[Step<'_>],
    kind: &str,
    selector: &str,
    expected: &[&str],
) -> Row {
    let response = execute(id, steps);
    let observed = serde_json::to_value(&response).expect("json");
    let anchor = observed.pointer("/reference_resolution/topic_anchored_resolution");
    let selected = anchor
        .map(|item| strings(item.get("selected_member_keys")))
        .unwrap_or_default();
    let expected = expected
        .iter()
        .map(|item| item.to_lowercase())
        .collect::<Vec<_>>();
    let group_match = anchor
        .and_then(|item| item["group_id"].as_str())
        .is_some_and(|group_id| {
            observed
                .pointer("/conversation_state/active_discourse_groups")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .find(|group| group["group_id"] == group_id)
                .is_some_and(|group| {
                    anchor.is_some_and(|item| {
                        item["group_revision"] == group["revision"]
                            && item["membership_sha256"] == group["membership_sha256"]
                            && item["member_keys"] == group["member_keys"]
                    })
                })
        });
    Row {
        id: id.to_string(),
        category: category.to_string(),
        pass: response.disposition == ConversationTurnDispositionIR::Grounded
            && anchor.is_some_and(|item| {
                item["applied"] == true
                    && item["kind"] == kind
                    && item["selector"] == selector
                    && item["semantic_authority"] == false
                    && item["external_execution_authorized"] == false
                    && item["resolution_sha256"]
                        .as_str()
                        .is_some_and(|hash| hash.len() == 64)
            })
            && group_match
            && selected == expected
            && response.grounded_realization.unsupported_claims == 0,
        trace: vec![observed.to_string()],
    }
}

fn unresolved_case(id: &str, steps: &[Step<'_>], selector: &str, term: &str) -> Row {
    let response = execute(id, steps);
    let observed = serde_json::to_value(&response).expect("json");
    let anchor = observed.pointer("/reference_resolution/topic_anchored_resolution");
    Row {
        id: id.to_string(),
        category: "transfer_safety_boundary".to_string(),
        pass: response.disposition == ConversationTurnDispositionIR::ClarificationRequired
            && response.grounded_response.is_none()
            && response.reference_resolution.original_semantic_text
                == response.reference_resolution.resolved_semantic_text
            && anchor.is_some_and(|item| {
                item["applied"] == false
                    && item["kind"] == "UNRESOLVED"
                    && item["selector"] == selector
                    && strings(item.get("unresolved_terms")).contains(&term.to_lowercase())
                    && item["semantic_authority"] == false
                    && item["external_execution_authorized"] == false
            }),
        trace: vec![observed.to_string()],
    }
}

fn neutral(language: LanguageCodeIR) -> Vec<Step<'static>> {
    let text = if language == LanguageCodeIR::Korean {
        "응, 알겠어"
    } else {
        "Okay, noted."
    };
    (0..18).map(|_| Step { text, language }).collect()
}

#[allow(clippy::vec_init_then_push)]
fn main() {
    let en = LanguageCodeIR::English;
    let ko = LanguageCodeIR::Korean;
    let mut rows = Vec::new();

    rows.push(applied_case(
        "R40_XFER_XL_01",
        "cross_language_anchor",
        &[
            Step {
                text: "inspect parser and repair index",
                language: en,
            },
            Step {
                text: "그 작업 묶음을 주제로 기억해 둬",
                language: ko,
            },
            Step {
                text: "Switch to the release topic.",
                language: en,
            },
            Step {
                text: "직전 주제로 복귀하자",
                language: ko,
            },
            Step {
                text: "Check the second one again.",
                language: en,
            },
        ],
        "ACTION_MEMBER",
        "ORDINAL",
        &["GOAL-000001-02"],
    ));
    rows.push(applied_case(
        "R40_XFER_XL_02",
        "cross_language_anchor",
        &[
            Step {
                text: "파서를 확인하고 인덱스를 수리해",
                language: ko,
            },
            Step {
                text: "Pin that task group as the topic.",
                language: en,
            },
            Step {
                text: "릴리스 주제로 전환하자",
                language: ko,
            },
            Step {
                text: "Resume the prior topic.",
                language: en,
            },
            Step {
                text: "두 번째 것을 다시 검사해",
                language: ko,
            },
        ],
        "ACTION_MEMBER",
        "ORDINAL",
        &["GOAL-000001-02"],
    ));
    rows.push(applied_case(
        "R40_XFER_XL_03",
        "cross_language_anchor",
        &[
            Step {
                text: "Dana says the parser is stable.",
                language: en,
            },
            Step {
                text: "Eli says the index is corrupt.",
                language: en,
            },
            Step {
                text: "What did Dana and Eli say?",
                language: en,
            },
            Step {
                text: "그 화자 묶음을 주제로 기억해 둬",
                language: ko,
            },
            Step {
                text: "Switch to the release topic.",
                language: en,
            },
            Step {
                text: "직전 주제로 복귀하자",
                language: ko,
            },
            Step {
                text: "What did the second one report?",
                language: en,
            },
        ],
        "PROPOSITION_MEMBER",
        "ORDINAL",
        &["eli"],
    ));
    rows.push(applied_case(
        "R40_XFER_XL_04",
        "cross_language_anchor",
        &[
            Step {
                text: "다나는 파서가 안정적이라고 말했다.",
                language: ko,
            },
            Step {
                text: "엘리는 인덱스가 손상됐다고 말했다.",
                language: ko,
            },
            Step {
                text: "다나와 엘리는 뭐라고 말했어?",
                language: ko,
            },
            Step {
                text: "Pin that speaker group as the topic.",
                language: en,
            },
            Step {
                text: "릴리스 주제로 전환하자",
                language: ko,
            },
            Step {
                text: "Resume the prior topic.",
                language: en,
            },
            Step {
                text: "두 번째 사람은 뭐라고 보고했어?",
                language: ko,
            },
        ],
        "PROPOSITION_MEMBER",
        "ORDINAL",
        &["엘리"],
    ));

    rows.push(applied_case(
        "R40_XFER_COMP_01",
        "composite_and_revised_anchor",
        &[
            Step {
                text: "inspect parser and repair index",
                language: en,
            },
            Step {
                text: "analyze deployer and check release",
                language: en,
            },
            Step {
                text: "Combine the first and second task groups.",
                language: en,
            },
            Step {
                text: "Make the combined task group the topic.",
                language: en,
            },
            Step {
                text: "Switch to the backup topic.",
                language: en,
            },
            Step {
                text: "Resume the prior topic.",
                language: en,
            },
            Step {
                text: "Inspect the fourth one again.",
                language: en,
            },
        ],
        "ACTION_MEMBER",
        "ORDINAL",
        &["GOAL-000002-02"],
    ));
    rows.push(applied_case(
        "R40_XFER_COMP_02",
        "composite_and_revised_anchor",
        &[
            Step {
                text: "파서를 확인하고 인덱스를 수리해",
                language: ko,
            },
            Step {
                text: "배포기를 분석하고 릴리스를 확인해",
                language: ko,
            },
            Step {
                text: "첫 작업 묶음과 둘째 작업 묶음을 결합해",
                language: ko,
            },
            Step {
                text: "결합된 작업 묶음을 주제로 기억해 둬",
                language: ko,
            },
            Step {
                text: "백업 주제로 전환하자",
                language: ko,
            },
            Step {
                text: "직전 주제로 복귀하자",
                language: ko,
            },
            Step {
                text: "네 번째 것을 다시 검사해",
                language: ko,
            },
        ],
        "ACTION_MEMBER",
        "ORDINAL",
        &["GOAL-000002-02"],
    ));
    rows.push(applied_case(
        "R40_XFER_COMP_03",
        "composite_and_revised_anchor",
        &[
            Step {
                text: "inspect parser and repair index",
                language: en,
            },
            Step {
                text: "analyze deployer",
                language: en,
            },
            Step {
                text: "Make the first task group the topic.",
                language: en,
            },
            Step {
                text: "Switch to the release topic.",
                language: en,
            },
            Step {
                text: "Attach deployer to the first task group.",
                language: en,
            },
            Step {
                text: "Resume the prior topic.",
                language: en,
            },
            Step {
                text: "Check all of them again.",
                language: en,
            },
        ],
        "ACTION_GROUP",
        "PLURAL",
        &["GOAL-000001-01", "GOAL-000001-02", "GOAL-000002-01"],
    ));
    rows.push(applied_case(
        "R40_XFER_COMP_04",
        "composite_and_revised_anchor",
        &[
            Step {
                text: "파서를 확인하고 인덱스를 수리해",
                language: ko,
            },
            Step {
                text: "배포기를 분석해",
                language: ko,
            },
            Step {
                text: "첫 작업 묶음을 주제로 기억해 둬",
                language: ko,
            },
            Step {
                text: "릴리스 주제로 전환하자",
                language: ko,
            },
            Step {
                text: "첫 작업 묶음에 배포기를 추가해",
                language: ko,
            },
            Step {
                text: "직전 주제로 복귀하자",
                language: ko,
            },
            Step {
                text: "그것들 전부 다시 확인해",
                language: ko,
            },
        ],
        "ACTION_GROUP",
        "PLURAL",
        &["GOAL-000001-01", "GOAL-000001-02", "GOAL-000002-01"],
    ));

    let mut long_en = vec![
        Step {
            text: "inspect parser and repair index",
            language: en,
        },
        Step {
            text: "Pin that task group as the topic.",
            language: en,
        },
        Step {
            text: "Switch to the release topic.",
            language: en,
        },
    ];
    long_en.extend(neutral(en));
    long_en.push(Step {
        text: "Resume the prior topic.",
        language: en,
    });
    long_en.push(Step {
        text: "Analyze the repair task.",
        language: en,
    });
    rows.push(applied_case(
        "R40_XFER_LONG_01",
        "long_horizon_anchor",
        &long_en,
        "ACTION_MEMBER",
        "PREDICATE_ROLE",
        &["GOAL-000001-02"],
    ));
    let mut long_ko = vec![
        Step {
            text: "파서를 확인하고 인덱스를 수리해",
            language: ko,
        },
        Step {
            text: "그 작업 묶음을 주제로 기억해 둬",
            language: ko,
        },
        Step {
            text: "릴리스 주제로 전환하자",
            language: ko,
        },
    ];
    long_ko.extend(neutral(ko));
    long_ko.push(Step {
        text: "직전 주제로 복귀하자",
        language: ko,
    });
    long_ko.push(Step {
        text: "수리 작업을 분석해",
        language: ko,
    });
    rows.push(applied_case(
        "R40_XFER_LONG_02",
        "long_horizon_anchor",
        &long_ko,
        "ACTION_MEMBER",
        "PREDICATE_ROLE",
        &["GOAL-000001-02"],
    ));
    let mut long_sp_en = vec![
        Step {
            text: "Dana says the parser is stable.",
            language: en,
        },
        Step {
            text: "Eli says the index is corrupt.",
            language: en,
        },
        Step {
            text: "What did Dana and Eli say?",
            language: en,
        },
        Step {
            text: "Pin that speaker group as the topic.",
            language: en,
        },
        Step {
            text: "Switch to the release topic.",
            language: en,
        },
    ];
    long_sp_en.extend(neutral(en));
    long_sp_en.push(Step {
        text: "Resume the prior topic.",
        language: en,
    });
    long_sp_en.push(Step {
        text: "Summarize their reports.",
        language: en,
    });
    rows.push(applied_case(
        "R40_XFER_LONG_03",
        "long_horizon_anchor",
        &long_sp_en,
        "PROPOSITION_GROUP",
        "PLURAL",
        &["dana", "eli"],
    ));
    let mut long_sp_ko = vec![
        Step {
            text: "다나는 파서가 안정적이라고 말했다.",
            language: ko,
        },
        Step {
            text: "엘리는 인덱스가 손상됐다고 말했다.",
            language: ko,
        },
        Step {
            text: "다나와 엘리는 뭐라고 말했어?",
            language: ko,
        },
        Step {
            text: "그 화자 묶음을 주제로 기억해 둬",
            language: ko,
        },
        Step {
            text: "릴리스 주제로 전환하자",
            language: ko,
        },
    ];
    long_sp_ko.extend(neutral(ko));
    long_sp_ko.push(Step {
        text: "직전 주제로 복귀하자",
        language: ko,
    });
    long_sp_ko.push(Step {
        text: "그들의 보고를 요약해",
        language: ko,
    });
    rows.push(applied_case(
        "R40_XFER_LONG_04",
        "long_horizon_anchor",
        &long_sp_ko,
        "PROPOSITION_GROUP",
        "PLURAL",
        &["다나", "엘리"],
    ));

    rows.push(applied_case(
        "R40_XFER_VAR_01",
        "surface_generalization",
        &[
            Step {
                text: "inspect parser and repair index",
                language: en,
            },
            Step {
                text: "Pin that task group as the topic.",
                language: en,
            },
            Step {
                text: "Switch to release.",
                language: en,
            },
            Step {
                text: "Resume the prior topic.",
                language: en,
            },
            Step {
                text: "Recheck whichever task was for repair.",
                language: en,
            },
        ],
        "ACTION_MEMBER",
        "PREDICATE_ROLE",
        &["GOAL-000001-02"],
    ));
    rows.push(applied_case(
        "R40_XFER_VAR_02",
        "surface_generalization",
        &[
            Step {
                text: "파서를 확인하고 인덱스를 수리해",
                language: ko,
            },
            Step {
                text: "그 작업 묶음을 주제로 기억해 둬",
                language: ko,
            },
            Step {
                text: "릴리스 주제로 전환하자",
                language: ko,
            },
            Step {
                text: "직전 주제로 복귀하자",
                language: ko,
            },
            Step {
                text: "수리하던 쪽을 다시 검사해",
                language: ko,
            },
        ],
        "ACTION_MEMBER",
        "PREDICATE_ROLE",
        &["GOAL-000001-02"],
    ));
    rows.push(applied_case(
        "R40_XFER_VAR_03",
        "surface_generalization",
        &[
            Step {
                text: "Dana says the parser is stable.",
                language: en,
            },
            Step {
                text: "Eli says the index is corrupt.",
                language: en,
            },
            Step {
                text: "What did Dana and Eli say?",
                language: en,
            },
            Step {
                text: "Pin that speaker group as the topic.",
                language: en,
            },
            Step {
                text: "Switch to release.",
                language: en,
            },
            Step {
                text: "Resume the prior topic.",
                language: en,
            },
            Step {
                text: "What was reported by the latter speaker?",
                language: en,
            },
        ],
        "PROPOSITION_MEMBER",
        "ORDINAL",
        &["eli"],
    ));
    rows.push(applied_case(
        "R40_XFER_VAR_04",
        "surface_generalization",
        &[
            Step {
                text: "다나는 파서가 안정적이라고 말했다.",
                language: ko,
            },
            Step {
                text: "엘리는 인덱스가 손상됐다고 말했다.",
                language: ko,
            },
            Step {
                text: "다나와 엘리는 뭐라고 말했어?",
                language: ko,
            },
            Step {
                text: "그 화자 묶음을 주제로 기억해 둬",
                language: ko,
            },
            Step {
                text: "릴리스 주제로 전환하자",
                language: ko,
            },
            Step {
                text: "직전 주제로 복귀하자",
                language: ko,
            },
            Step {
                text: "뒤 사람이 보고한 내용은 뭐야?",
                language: ko,
            },
        ],
        "PROPOSITION_MEMBER",
        "ORDINAL",
        &["엘리"],
    ));

    rows.push(unresolved_case(
        "R40_XFER_SAFE_01",
        &[
            Step {
                text: "inspect parser and repair index",
                language: en,
            },
            Step {
                text: "Pin that task group as the topic.",
                language: en,
            },
            Step {
                text: "Inspect the third one again.",
                language: en,
            },
        ],
        "ORDINAL",
        "ORDINAL_OUT_OF_RANGE",
    ));
    rows.push(unresolved_case(
        "R40_XFER_SAFE_02",
        &[
            Step {
                text: "파서를 확인하고 인덱스를 수리해",
                language: ko,
            },
            Step {
                text: "그 작업 묶음을 주제로 기억해 둬",
                language: ko,
            },
            Step {
                text: "그것을 다시 검사해",
                language: ko,
            },
        ],
        "GENERIC_SINGULAR",
        "AMBIGUOUS_GROUP_MEMBER",
    ));
    rows.push(unresolved_case(
        "R40_XFER_SAFE_03",
        &[
            Step {
                text: "Dana says the parser is stable.",
                language: en,
            },
            Step {
                text: "Eli says the index is corrupt.",
                language: en,
            },
            Step {
                text: "What did Dana and Eli say?",
                language: en,
            },
            Step {
                text: "Pin that speaker group as the topic.",
                language: en,
            },
            Step {
                text: "Run them again.",
                language: en,
            },
        ],
        "TYPE_MISMATCH",
        "ANCHOR_KIND_MISMATCH",
    ));
    rows.push(unresolved_case(
        "R40_XFER_SAFE_04",
        &[
            Step {
                text: "파서를 확인하고 인덱스를 수리해",
                language: ko,
            },
            Step {
                text: "그 작업 묶음을 주제로 기억해 둬",
                language: ko,
            },
            Step {
                text: "그들은 뭐라고 말했어?",
                language: ko,
            },
        ],
        "TYPE_MISMATCH",
        "ANCHOR_KIND_MISMATCH",
    ));

    let passed = rows.iter().filter(|row| row.pass).count();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "suite":"R40-RUN-0002", "frozen_before_product_changes":true,
            "held_out_until_diagnostic_passes":true, "total":rows.len(), "passed":passed,
            "failed":rows.len()-passed, "external_llm_calls":0, "local_teacher_calls":0,
            "network_calls":0, "recursive_source_mutations":0, "rows":rows,
        }))
        .expect("suite json")
    );
}
